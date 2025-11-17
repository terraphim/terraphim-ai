//! GitHub Runner registration protocol implementation

use crate::{RunnerConfig, RunnerId, RunnerResult, RunnerScope, RunnerState, RunnerError};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Runner registration with GitHub
pub struct RunnerRegistration {
    /// Runner ID (assigned by GitHub after registration)
    pub runner_id: Option<u64>,
    /// Local runner identifier
    pub local_id: RunnerId,
    /// Configuration
    pub config: RunnerConfig,
    /// Current state
    state: Arc<RwLock<RunnerState>>,
    /// HTTP client
    client: Client,
    /// GitHub API base URL
    api_base_url: String,
    /// Access token for API calls
    access_token: Arc<RwLock<Option<String>>>,
}

/// Response from runner registration
#[derive(Debug, Deserialize)]
struct RegistrationResponse {
    id: u64,
    name: String,
    os: String,
    status: String,
    labels: Vec<Label>,
}

#[derive(Debug, Deserialize)]
struct Label {
    id: u64,
    name: String,
    #[serde(rename = "type")]
    label_type: String,
}

/// Request to register a new runner
#[derive(Debug, Serialize)]
struct RegistrationRequest {
    name: String,
    runner_group_id: Option<u64>,
    labels: Vec<String>,
    work_folder: String,
}

/// Runner credentials from GitHub
#[derive(Debug, Deserialize)]
pub struct RunnerCredentials {
    pub url: String,
    pub token: String,
}

impl RunnerRegistration {
    /// Create a new runner registration
    pub fn new(config: RunnerConfig) -> Self {
        Self {
            runner_id: None,
            local_id: RunnerId::new(),
            config,
            state: Arc::new(RwLock::new(RunnerState::Initializing)),
            client: Client::new(),
            api_base_url: "https://api.github.com".to_string(),
            access_token: Arc::new(RwLock::new(None)),
        }
    }

    /// Set custom API base URL (for GitHub Enterprise)
    pub fn with_api_url(mut self, url: &str) -> Self {
        self.api_base_url = url.to_string();
        self
    }

    /// Register the runner with GitHub
    pub async fn register(&mut self) -> RunnerResult<()> {
        log::info!("Registering runner '{}' with GitHub", self.config.name);

        // Exchange registration token for access token
        let access_token = self.exchange_token().await?;
        *self.access_token.write().await = Some(access_token.clone());

        // Build registration request
        let mut labels: Vec<String> = self.config.labels.builtin.clone();
        labels.extend(self.config.labels.custom.clone());

        let request = RegistrationRequest {
            name: self.config.name.clone(),
            runner_group_id: None,
            labels,
            work_folder: self.config.work_directory.clone(),
        };

        // Send registration request
        let url = self.config.scope.api_url(&self.api_base_url);
        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", access_token))
            .header("Accept", "application/vnd.github+json")
            .header("X-GitHub-Api-Version", "2022-11-28")
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(RunnerError::Registration(format!(
                "Failed to register runner: {}",
                error_text
            )));
        }

        let registration: RegistrationResponse = response.json().await?;
        self.runner_id = Some(registration.id);
        *self.state.write().await = RunnerState::Idle;

        log::info!(
            "Runner registered successfully with ID: {}",
            registration.id
        );

        Ok(())
    }

    /// Exchange registration token for access token
    async fn exchange_token(&self) -> RunnerResult<String> {
        // The registration token is used to get a JWT that allows
        // the runner to communicate with GitHub Actions service
        let url = match &self.config.scope {
            RunnerScope::Repository { owner, repo } => {
                format!(
                    "{}/repos/{}/{}/actions/runners/registration-token",
                    self.api_base_url, owner, repo
                )
            }
            RunnerScope::Organization { org } => {
                format!(
                    "{}/orgs/{}/actions/runners/registration-token",
                    self.api_base_url, org
                )
            }
            RunnerScope::Enterprise { enterprise } => {
                format!(
                    "{}/enterprises/{}/actions/runners/registration-token",
                    self.api_base_url, enterprise
                )
            }
        };

        // For now, we'll use the registration token directly
        // In a real implementation, this would involve JWT exchange
        Ok(self.config.registration_token.clone())
    }

    /// Unregister the runner from GitHub
    pub async fn unregister(&mut self) -> RunnerResult<()> {
        if let Some(runner_id) = self.runner_id {
            log::info!("Unregistering runner {}", runner_id);

            let url = format!(
                "{}/{}",
                self.config.scope.api_url(&self.api_base_url),
                runner_id
            );

            let access_token = self.access_token.read().await;
            let token = access_token
                .as_ref()
                .ok_or_else(|| RunnerError::Authentication("No access token".to_string()))?;

            let response = self
                .client
                .delete(&url)
                .header("Authorization", format!("Bearer {}", token))
                .header("Accept", "application/vnd.github+json")
                .header("X-GitHub-Api-Version", "2022-11-28")
                .send()
                .await?;

            if !response.status().is_success() {
                let error_text = response.text().await.unwrap_or_default();
                return Err(RunnerError::Registration(format!(
                    "Failed to unregister runner: {}",
                    error_text
                )));
            }

            self.runner_id = None;
            *self.state.write().await = RunnerState::Offline;

            log::info!("Runner unregistered successfully");
        }

        Ok(())
    }

    /// Get current runner state
    pub async fn state(&self) -> RunnerState {
        *self.state.read().await
    }

    /// Set runner state
    pub async fn set_state(&self, state: RunnerState) {
        *self.state.write().await = state;
    }

    /// Check if runner is registered
    pub fn is_registered(&self) -> bool {
        self.runner_id.is_some()
    }

    /// Get GitHub runner ID
    pub fn github_runner_id(&self) -> Option<u64> {
        self.runner_id
    }

    /// Send heartbeat to GitHub
    pub async fn heartbeat(&self) -> RunnerResult<()> {
        // In a real implementation, this would poll the GitHub Actions service
        // for new jobs and maintain the connection
        log::debug!("Runner heartbeat");
        Ok(())
    }
}
