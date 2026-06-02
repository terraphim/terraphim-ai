//! Single-writer commit-status helper (Gitea REST `statuses` API).
//!
//! Posts at most one status per `(sha, context)` transition, so the native lane
//! (`terraphim-native/<job>`) and the interim ADF lane (`adf/build`) never write
//! the same context. Used for both the native status and the optional legacy
//! `adf/build` mirror during migration.

use crate::{Result, RunnerError};
use std::collections::HashMap;
use std::sync::Mutex;

/// Commit-status state strings accepted by Gitea.
#[derive(Debug, Clone, Copy)]
pub enum StatusState {
    Pending,
    Success,
    Failure,
    Error,
}

impl StatusState {
    fn as_str(self) -> &'static str {
        match self {
            StatusState::Pending => "pending",
            StatusState::Success => "success",
            StatusState::Failure => "failure",
            StatusState::Error => "error",
        }
    }
}

/// Posts commit statuses, deduplicating identical `(sha, context, state)` posts.
pub struct SingleStatusWriter {
    base_url: String,
    token: String,
    http: reqwest::Client,
    seen: Mutex<HashMap<(String, String), String>>,
}

impl SingleStatusWriter {
    /// Create a writer for the given Gitea instance + API token.
    pub fn new(instance_url: impl Into<String>, token: impl Into<String>) -> Self {
        Self {
            base_url: instance_url.into().trim_end_matches('/').to_string(),
            token: token.into(),
            http: reqwest::Client::new(),
            seen: Mutex::new(HashMap::new()),
        }
    }

    /// Post a commit status. No-ops if the same `(sha, context)` was already set
    /// to the same `state`.
    pub async fn post(
        &self,
        owner: &str,
        repo: &str,
        sha: &str,
        state: StatusState,
        context: &str,
        description: &str,
    ) -> Result<()> {
        {
            let mut seen = self.seen.lock().unwrap();
            let key = (sha.to_string(), context.to_string());
            if seen.get(&key).map(|s| s.as_str()) == Some(state.as_str()) {
                return Ok(());
            }
            seen.insert(key, state.as_str().to_string());
        }
        let url = format!(
            "{}/api/v1/repos/{}/{}/statuses/{}",
            self.base_url, owner, repo, sha
        );
        let body = serde_json::json!({
            "state": state.as_str(),
            "context": context,
            "description": description.chars().take(140).collect::<String>(),
        });
        let resp = self
            .http
            .post(&url)
            .header("authorization", format!("token {}", self.token))
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| RunnerError::Protocol(format!("set status: {e}")))?;
        if !resp.status().is_success() {
            return Err(RunnerError::Protocol(format!(
                "set status: HTTP {}",
                resp.status()
            )));
        }
        Ok(())
    }
}
