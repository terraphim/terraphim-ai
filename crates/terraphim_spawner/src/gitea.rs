//! Gitea authentication for agents
//!
//! This module provides Gitea-based authentication for agents using
//! Personal Access Tokens (PATs). Agents authenticate to Gitea and
//! log their actions as issues or repository events.

use std::collections::HashMap;
use std::time::Duration;

use serde::{Deserialize, Serialize};

/// Gitea agent authentication
#[derive(Debug, Clone)]
pub struct GiteaAgentAuth {
    /// Agent identifier
    pub agent_id: String,
    /// Human sponsor who owns this agent
    pub sponsor: String,
    /// Gitea instance URL
    pub gitea_url: String,
    /// Personal Access Token
    token: String,
    /// Default repository for logging
    pub log_repo: String,
}

impl GiteaAgentAuth {
    /// Create new Gitea agent authentication
    pub fn new(
        agent_id: impl Into<String>,
        sponsor: impl Into<String>,
        gitea_url: impl Into<String>,
        token: impl Into<String>,
        log_repo: impl Into<String>,
    ) -> Self {
        Self {
            agent_id: agent_id.into(),
            sponsor: sponsor.into(),
            gitea_url: gitea_url.into(),
            token: token.into(),
            log_repo: log_repo.into(),
        }
    }

    /// Create from environment variables
    pub fn from_env(agent_id: impl Into<String>) -> Result<Self, GiteaAuthError> {
        let agent_id = agent_id.into();
        let sponsor = std::env::var("GITEA_SPONSOR")
            .map_err(|_| GiteaAuthError::MissingEnv("GITEA_SPONSOR".to_string()))?;
        let gitea_url = std::env::var("GITEA_URL")
            .map_err(|_| GiteaAuthError::MissingEnv("GITEA_URL".to_string()))?;
        let token = std::env::var("GITEA_TOKEN")
            .map_err(|_| GiteaAuthError::MissingEnv("GITEA_TOKEN".to_string()))?;
        let log_repo = std::env::var("GITEA_LOG_REPO")
            .unwrap_or_else(|_| format!("{}/agent-logs", sponsor));

        Ok(Self {
            agent_id,
            sponsor,
            gitea_url,
            token,
            log_repo,
        })
    }

    /// Get HTTP client with authentication
    fn client(&self) -> reqwest::Client {
        reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("Failed to build HTTP client")
    }

    /// Verify token is valid
    pub async fn verify(&self) -> Result<GiteaUser, GiteaAuthError> {
        let url = format!("{}/api/v1/user", self.gitea_url);
        
        let response = self.client()
            .get(&url)
            .header("Authorization", format!("token {}", self.token))
            .send()
            .await
            .map_err(|e| GiteaAuthError::RequestFailed(e.to_string()))?;

        if !response.status().is_success() {
            return Err(GiteaAuthError::InvalidToken);
        }

        let user: GiteaUser = response.json().await
            .map_err(|e| GiteaAuthError::ParseError(e.to_string()))?;

        Ok(user)
    }

    /// Log agent action as issue
    pub async fn log_action(
        &self,
        action: &str,
        details: &str,
        metadata: Option<HashMap<String, String>>,
    ) -> Result<u64, GiteaAuthError> {
        let url = format!(
            "{}/api/v1/repos/{}/issues",
            self.gitea_url,
            self.log_repo
        );

        let title = format!("[{}] {}", self.agent_id, action);
        
        let mut body = format!(
            "## Agent Action Log\n\n\
             **Agent:** `{}`\n\
             **Sponsor:** @{}\n\
             **Action:** {}\n\
             **Time:** {}\n\n\
             ### Details\n\n\
             {}\n",
            self.agent_id,
            self.sponsor,
            action,
            chrono::Utc::now().to_rfc3339(),
            details
        );

        // Add metadata if provided
        if let Some(meta) = metadata {
            body.push_str("\n### Metadata\n\n");
            for (key, value) in meta {
                body.push_str(&format!("- **{}:** {}\n", key, value));
            }
        }

        let issue = CreateIssueRequest { title, body };

        let response = self.client()
            .post(&url)
            .header("Authorization", format!("token {}", self.token))
            .json(&issue)
            .send()
            .await
            .map_err(|e| GiteaAuthError::RequestFailed(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(GiteaAuthError::ApiError {
                status: status.as_u16(),
                message: text,
            });
        }

        let created: GiteaIssue = response.json().await
            .map_err(|e| GiteaAuthError::ParseError(e.to_string()))?;

        Ok(created.number)
    }

    /// Update agent status in repository
    pub async fn update_status(
        &self,
        status: AgentStatus,
        message: &str,
    ) -> Result<(), GiteaAuthError> {
        let url = format!(
            "{}/api/v1/repos/{}/contents/agent-status/{}.md",
            self.gitea_url,
            self.log_repo,
            self.agent_id
        );

        let content = format!(
            "# Agent Status: {}\n\n\
             **Status:** {:?}\n\
             **Message:** {}\n\
             **Updated:** {}\n",
            self.agent_id,
            status,
            message,
            chrono::Utc::now().to_rfc3339()
        );

        let encoded = base64::encode(&content);

        let request = UpdateFileRequest {
            content: encoded,
            message: format!("Agent {} status: {:?}", self.agent_id, status),
        };

        let response = self.client()
            .post(&url)
            .header("Authorization", format!("token {}", self.token))
            .json(&request)
            .send()
            .await
            .map_err(|e| GiteaAuthError::RequestFailed(e.to_string()))?;

        if !response.status().is_success() {
            // File might not exist, try creating
            return self.create_status_file(request).await;
        }

        Ok(())
    }

    async fn create_status_file(
        &self,
        request: UpdateFileRequest,
    ) -> Result<(), GiteaAuthError> {
        let url = format!(
            "{}/api/v1/repos/{}/contents/agent-status/{}.md",
            self.gitea_url,
            self.log_repo,
            self.agent_id
        );

        let response = self.client()
            .post(&url)
            .header("Authorization", format!("token {}", self.token))
            .json(&request)
            .send()
            .await
            .map_err(|e| GiteaAuthError::RequestFailed(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(GiteaAuthError::ApiError {
                status: status.as_u16(),
                message: text,
            });
        }

        Ok(())
    }
}

/// Agent status for Gitea logging
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentStatus {
    Starting,
    Running,
    Idle,
    Error,
    Stopped,
}

/// Gitea user info
#[derive(Debug, Clone, Deserialize)]
pub struct GiteaUser {
    pub id: u64,
    pub login: String,
    pub email: String,
}

/// Gitea issue
#[derive(Debug, Clone, Deserialize)]
pub struct GiteaIssue {
    pub number: u64,
    pub title: String,
    pub state: String,
}

/// Create issue request
#[derive(Debug, Clone, Serialize)]
struct CreateIssueRequest {
    title: String,
    body: String,
}

/// Update file request
#[derive(Debug, Clone, Serialize)]
struct UpdateFileRequest {
    content: String,
    message: String,
}

/// Gitea authentication errors
#[derive(thiserror::Error, Debug)]
pub enum GiteaAuthError {
    #[error("Missing environment variable: {0}")]
    MissingEnv(String),
    
    #[error("Invalid or expired token")]
    InvalidToken,
    
    #[error("Request failed: {0}")]
    RequestFailed(String),
    
    #[error("Parse error: {0}")]
    ParseError(String),
    
    #[error("API error {status}: {message}")]
    ApiError { status: u16, message: String },
}

/// Simple base64 encoding (since we don't have the base64 crate)
mod base64 {
    pub fn encode(input: &str) -> String {
        use std::io::Write;
        
        const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        
        let bytes = input.as_bytes();
        let mut result = Vec::with_capacity((bytes.len() + 2) / 3 * 4);
        
        for chunk in bytes.chunks(3) {
            let b = match chunk.len() {
                1 => [chunk[0], 0, 0],
                2 => [chunk[0], chunk[1], 0],
                3 => [chunk[0], chunk[1], chunk[2]],
                _ => unreachable!(),
            };
            
            let n = (b[0] as usize) << 16 | (b[1] as usize) << 8 | (b[2] as usize);
            
            result.push(ALPHABET[(n >> 18) & 0x3F]);
            result.push(ALPHABET[(n >> 12) & 0x3F]);
            result.push(if chunk.len() > 1 { ALPHABET[(n >> 6) & 0x3F] } else { b'=' });
            result.push(if chunk.len() > 2 { ALPHABET[n & 0x3F] } else { b'=' });
        }
        
        String::from_utf8(result).unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gitea_auth_creation() {
        let auth = GiteaAgentAuth::new(
            "agent-123",
            "alex",
            "https://git.terraphim.cloud",
            "test-token",
            "alex/agent-logs",
        );

        assert_eq!(auth.agent_id, "agent-123");
        assert_eq!(auth.sponsor, "alex");
        assert_eq!(auth.gitea_url, "https://git.terraphim.cloud");
    }

    #[test]
    fn test_base64_encode() {
        assert_eq!(base64::encode("hello"), "aGVsbG8=");
        assert_eq!(base64::encode("test"), "dGVzdA==");
    }
}
