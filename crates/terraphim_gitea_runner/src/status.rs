//! Single-writer commit-status helper (Gitea REST `statuses` API).
//!
//! Posts at most one status per `(sha, context)` transition, so the native lane
//! (`terraphim-native/<job>`) and the interim ADF lane (`adf/build`) never write
//! the same context. Used for both the native status and the optional legacy
//! `adf/build` mirror during migration.

use crate::config::DEFAULT_HTTP_REQUEST_TIMEOUT;
use crate::{Result, RunnerError};
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Duration;

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
    /// Create a writer with the runner's default per-request timeout.
    pub fn new(instance_url: impl Into<String>, token: impl Into<String>) -> Self {
        Self::new_with_timeout(instance_url, token, DEFAULT_HTTP_REQUEST_TIMEOUT)
    }

    /// Create a writer with an explicit per-request timeout.
    pub fn new_with_timeout(
        instance_url: impl Into<String>,
        token: impl Into<String>,
        timeout: Duration,
    ) -> Self {
        Self {
            base_url: instance_url.into().trim_end_matches('/').to_string(),
            token: token.into(),
            http: reqwest::Client::builder()
                .timeout(timeout)
                .build()
                .expect("failed to build status reqwest client"),
            seen: Mutex::new(HashMap::new()),
        }
    }

    /// Idempotency gate: record `(sha, context) -> state` and report whether a
    /// network post is warranted. Returns `false` (skip the post) when the same
    /// `(sha, context)` was already set to the same `state` -- the single-writer
    /// guarantee that keeps the native lane from re-posting an identical status.
    fn should_send(&self, sha: &str, context: &str, state: StatusState) -> bool {
        let mut seen = self.seen.lock().unwrap();
        let key = (sha.to_string(), context.to_string());
        if seen.get(&key).map(|s| s.as_str()) == Some(state.as_str()) {
            return false;
        }
        seen.insert(key, state.as_str().to_string());
        true
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
        if !self.should_send(sha, context, state) {
            return Ok(());
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn idempotent_on_sha_context_state() {
        let w = SingleStatusWriter::new("https://git.example", "tok");
        // First transition to pending warrants a post.
        assert!(w.should_send("deadbeef", "terraphim-native/build", StatusState::Pending));
        // Same (sha, context, state) is deduplicated -- single writer, no re-post.
        assert!(!w.should_send("deadbeef", "terraphim-native/build", StatusState::Pending));
        // A genuine state transition for the same (sha, context) is allowed.
        assert!(w.should_send("deadbeef", "terraphim-native/build", StatusState::Success));
        assert!(!w.should_send("deadbeef", "terraphim-native/build", StatusState::Success));
    }

    #[test]
    fn distinct_contexts_do_not_collide() {
        let w = SingleStatusWriter::new("https://git.example", "tok");
        // The native lane and the interim ADF lane write distinct contexts for the
        // same sha and must each be allowed (coexistence guard, AC-8).
        assert!(w.should_send("deadbeef", "terraphim-native/build", StatusState::Success));
        assert!(w.should_send("deadbeef", "adf/build", StatusState::Success));
        // Different sha, same context, also distinct.
        assert!(w.should_send("cafef00d", "terraphim-native/build", StatusState::Success));
    }

    #[tokio::test]
    async fn status_post_is_bounded_by_configured_http_timeout() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let stalled_server = tokio::spawn(async move {
            let (_socket, _) = listener.accept().await.unwrap();
            std::future::pending::<()>().await;
        });
        let writer = SingleStatusWriter::new_with_timeout(
            format!("http://{address}"),
            "tok",
            Duration::from_millis(20),
        );

        let _error = tokio::time::timeout(
            Duration::from_secs(1),
            writer.post(
                "owner",
                "repo",
                "deadbeef",
                StatusState::Success,
                "terraphim-native/build",
                "complete",
            ),
        )
        .await
        .expect("status writer must enforce its configured request timeout")
        .expect_err("stalled status endpoint must fail");

        stalled_server.abort();
    }
}
