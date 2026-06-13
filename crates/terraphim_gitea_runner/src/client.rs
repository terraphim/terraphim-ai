//! Connect-JSON client for the Gitea `RunnerService`.
//!
//! Connect-RPC methods are plain HTTP POSTs to
//! `{instance_url}/api/actions/runner.v1.RunnerService/{Method}` with
//! `Content-Type: application/json`. All methods except `Register` authenticate
//! with `x-runner-uuid` + `x-runner-token` headers.

use crate::state::RunnerState;
use crate::types::*;
use crate::{Result, RunnerError};
use async_trait::async_trait;
use std::time::Duration;

const SERVICE: &str = "api/actions/runner.v1.RunnerService";

/// The Gitea runner protocol surface needed for Milestone 1.
#[async_trait]
pub trait GiteaRunnerClient: Send + Sync {
    /// Register a new runner using a registration token (no auth headers).
    async fn register(&self, req: RegisterRequest) -> Result<RunnerInfo>;
    /// Declare labels/version on startup.
    async fn declare(&self, state: &RunnerState, req: DeclareRequest) -> Result<DeclareResponse>;
    /// Poll for an available task.
    async fn fetch_task(
        &self,
        state: &RunnerState,
        tasks_version: i64,
    ) -> Result<FetchTaskResponse>;
    /// Report task state/outputs.
    async fn update_task(
        &self,
        state: &RunnerState,
        req: UpdateTaskRequest,
    ) -> Result<UpdateTaskResponse>;
    /// Stream a batch of log rows.
    async fn update_log(
        &self,
        state: &RunnerState,
        req: UpdateLogRequest,
    ) -> Result<UpdateLogResponse>;
}

/// reqwest-backed [`GiteaRunnerClient`].
pub struct ReqwestRunnerClient {
    base_url: String,
    http: reqwest::Client,
}

impl ReqwestRunnerClient {
    /// Create a client with a 30-second per-request timeout.
    pub fn new(instance_url: impl Into<String>) -> Self {
        Self::new_with_timeout(instance_url, Duration::from_secs(30))
    }

    /// Create a client with an explicit per-request timeout.
    ///
    /// Pass the value from [`crate::config::RunnerConfig::http_request_timeout`].
    /// A hung `FetchTask` call returns an error within the timeout window rather
    /// than blocking the poll loop indefinitely.
    pub fn new_with_timeout(instance_url: impl Into<String>, timeout: Duration) -> Self {
        let http = reqwest::Client::builder()
            .timeout(timeout)
            .build()
            .expect("failed to build reqwest client");
        Self {
            base_url: instance_url.into().trim_end_matches('/').to_string(),
            http,
        }
    }

    /// Use a pre-built reqwest client (e.g. with custom timeouts) -- for tests
    /// pointed at a fake server.
    pub fn with_http(instance_url: impl Into<String>, http: reqwest::Client) -> Self {
        Self {
            base_url: instance_url.into().trim_end_matches('/').to_string(),
            http,
        }
    }

    fn url(&self, method: &str) -> String {
        format!("{}/{}/{}", self.base_url, SERVICE, method)
    }

    async fn post<Req: serde::Serialize, Resp: serde::de::DeserializeOwned>(
        &self,
        method: &str,
        auth: Option<&RunnerState>,
        body: &Req,
    ) -> Result<Resp> {
        let mut rb = self
            .http
            .post(self.url(method))
            .header("content-type", "application/json");
        if let Some(state) = auth {
            rb = rb
                .header("x-runner-uuid", &state.uuid)
                .header("x-runner-token", &state.token);
        }
        let resp = rb
            .json(body)
            .send()
            .await
            .map_err(|e| RunnerError::Protocol(format!("{method}: request failed: {e}")))?;
        let status = resp.status();
        let text = resp
            .text()
            .await
            .map_err(|e| RunnerError::Protocol(format!("{method}: read body failed: {e}")))?;
        if !status.is_success() {
            return Err(RunnerError::Protocol(format!(
                "{method}: HTTP {status}: {text}"
            )));
        }
        serde_json::from_str(&text).map_err(|e| {
            RunnerError::Protocol(format!("{method}: decode response failed: {e}: {text}"))
        })
    }
}

#[async_trait]
impl GiteaRunnerClient for ReqwestRunnerClient {
    async fn register(&self, req: RegisterRequest) -> Result<RunnerInfo> {
        let resp: RegisterResponse = self.post("Register", None, &req).await?;
        Ok(resp.runner)
    }

    async fn declare(&self, state: &RunnerState, req: DeclareRequest) -> Result<DeclareResponse> {
        self.post("Declare", Some(state), &req).await
    }

    async fn fetch_task(
        &self,
        state: &RunnerState,
        tasks_version: i64,
    ) -> Result<FetchTaskResponse> {
        self.post(
            "FetchTask",
            Some(state),
            &FetchTaskRequest { tasks_version },
        )
        .await
    }

    async fn update_task(
        &self,
        state: &RunnerState,
        req: UpdateTaskRequest,
    ) -> Result<UpdateTaskResponse> {
        self.post("UpdateTask", Some(state), &req).await
    }

    async fn update_log(
        &self,
        state: &RunnerState,
        req: UpdateLogRequest,
    ) -> Result<UpdateLogResponse> {
        self.post("UpdateLog", Some(state), &req).await
    }
}
