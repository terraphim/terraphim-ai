//! RL training status (partial Hermes parity).
//!
//! Hermes `rl_training_tool.py` (1380 lines) orchestrates a full veRL training
//! stack: it scans Pydantic environment configs, spawns rollout/model/trainer
//! processes, integrates wandb, and supports checkpoint/recover. That is deeply
//! coupled to Python/ray/veRL and is a **deliberate non-goal** for the Rust
//! port. This module exposes the one portable, monitorable piece: polling a
//! rollout server's status endpoint (`rl_check_status`).

use crate::config::RlConfig;
use crate::tools::{Tool, ToolError};
use async_trait::async_trait;
use std::time::Duration;

/// HTTP client for a rollout server status endpoint.
pub struct RlClient {
    http: reqwest::Client,
    server_url: String,
}

impl RlClient {
    pub fn from_config(cfg: &RlConfig) -> Self {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .unwrap_or_default();
        Self {
            http,
            server_url: cfg.rollout_server_url.trim_end_matches('/').to_string(),
        }
    }

    /// Poll status for a training run.
    pub async fn check_status(&self, run_id: &str) -> Result<serde_json::Value, String> {
        let url = format!("{}/status/{}", self.server_url, run_id);
        let resp = self
            .http
            .get(&url)
            .send()
            .await
            .map_err(|e| e.to_string())?;
        if !resp.status().is_success() {
            return Err(format!(
                "rollout server status failed: HTTP {}",
                resp.status()
            ));
        }
        resp.json::<serde_json::Value>()
            .await
            .map_err(|e| e.to_string())
    }
}

/// The `rl_check_status` tool.
pub struct RlCheckStatusTool {
    client: RlClient,
}

impl RlCheckStatusTool {
    pub fn from_config(cfg: &RlConfig) -> Self {
        Self {
            client: RlClient::from_config(cfg),
        }
    }
}

#[async_trait]
impl Tool for RlCheckStatusTool {
    fn name(&self) -> &str {
        "rl_check_status"
    }
    fn description(&self) -> &str {
        "Check the status and metrics of an RL training run by polling the \
         rollout server status endpoint. Full training orchestration is not \
         available in this build."
    }
    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "run_id": { "type": "string", "description": "The training run id" }
            },
            "required": ["run_id"]
        })
    }
    async fn execute(&self, args: serde_json::Value) -> Result<String, ToolError> {
        let run_id = args["run_id"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArguments {
                tool: "rl_check_status".to_string(),
                message: "run_id is required".to_string(),
            })?;
        let status =
            self.client
                .check_status(run_id)
                .await
                .map_err(|e| ToolError::ExecutionFailed {
                    tool: "rl_check_status".to_string(),
                    message: e,
                })?;
        Ok(status.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_available() {
        let cfg = RlConfig {
            enabled: true,
            ..Default::default()
        };
        assert!(cfg.available());
        let cfg2 = RlConfig::default();
        assert!(!cfg2.available());
    }

    #[tokio::test]
    async fn check_status_unreachable_server_errors() {
        // Port 1 is essentially never a live HTTP server.
        let cfg = RlConfig {
            enabled: true,
            rollout_server_url: "http://127.0.0.1:1".into(),
        };
        let client = RlClient::from_config(&cfg);
        let result = client.check_status("run-1").await;
        assert!(result.is_err());
    }
}
