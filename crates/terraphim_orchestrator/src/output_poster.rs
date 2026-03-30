//! Post agent output as Gitea issue comments.
//!
//! The OutputPoster formats agent results (ReviewAgentOutput or plain text)
//! into markdown-formatted Gitea comments, enabling human and agent visibility
//! of results directly in the issue thread.

use terraphim_tracker::gitea::GiteaTracker;
use terraphim_tracker::TrackerError;
use terraphim_symphony::runner::protocol::ReviewAgentOutput;

/// Maximum comment body length (Gitea default limit is 65535 chars).
const MAX_COMMENT_LENGTH: usize = 60_000;

/// Posts agent output as Gitea issue comments.
pub struct OutputPoster {
    tracker: GiteaTracker,
}

impl OutputPoster {
    pub fn new(tracker: GiteaTracker) -> Self {
        Self { tracker }
    }

    /// Format and post a ReviewAgentOutput as a markdown comment.
    /// Returns the posted comment ID.
    pub async fn post_agent_output(
        &self,
        issue_number: u64,
        agent_name: &str,
        output: &ReviewAgentOutput,
    ) -> Result<u64, TrackerError> {
        let mut body = format!(
            "## Agent Report: {}\n\n**Verdict**: {}\n\n**Summary**: {}\n",
            agent_name,
            if output.pass { "PASS" } else { "FAIL" },
            output.summary,
        );

        if !output.findings.is_empty() {
            body.push_str("\n### Findings\n\n");
            for finding in &output.findings {
                body.push_str(&format!(
                    "- **[{:?}]** {:?}: {}\n",
                    finding.severity, finding.category, finding.finding
                ));
                if !finding.file.is_empty() {
                    body.push_str(&format!("  - File: `{}`", finding.file));
                    if finding.line > 0 {
                        body.push_str(&format!(":{}", finding.line));
                    }
                    body.push('\n');
                }
                if let Some(ref suggestion) = finding.suggestion {
                    body.push_str(&format!("  - Suggestion: {}\n", suggestion));
                }
            }
        }

        // Truncate if too long
        if body.len() > MAX_COMMENT_LENGTH {
            body.truncate(MAX_COMMENT_LENGTH - 50);
            body.push_str("\n\n---\n*[Output truncated]*");
        }

        let comment = self.tracker.post_comment(issue_number, &body).await?;
        tracing::info!(
            issue = issue_number,
            agent = agent_name,
            comment_id = comment.id,
            "posted agent output to Gitea issue"
        );
        Ok(comment.id)
    }

    /// Post a plain text message attributed to an agent.
    /// Returns the posted comment ID.
    pub async fn post_agent_message(
        &self,
        issue_number: u64,
        agent_name: &str,
        message: &str,
    ) -> Result<u64, TrackerError> {
        let body = format!(
            "## Agent Message: {}\n\n{}\n",
            agent_name, message
        );

        let truncated = if body.len() > MAX_COMMENT_LENGTH {
            let mut b = body[..MAX_COMMENT_LENGTH - 50].to_string();
            b.push_str("\n\n---\n*[Output truncated]*");
            b
        } else {
            body
        };

        let comment = self.tracker.post_comment(issue_number, &truncated).await?;
        tracing::info!(
            issue = issue_number,
            agent = agent_name,
            comment_id = comment.id,
            "posted agent message to Gitea issue"
        );
        Ok(comment.id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use terraphim_tracker::gitea::GiteaConfig;
    use terraphim_symphony::runner::protocol::{FindingCategory, FindingSeverity, ReviewFinding};
    use wiremock::{Mock, MockServer, ResponseTemplate};
    use wiremock::matchers::{method, path, body_string_contains};

    async fn setup() -> (MockServer, OutputPoster) {
        let server = MockServer::start().await;
        let config = GiteaConfig {
            base_url: server.uri(),
            token: "test-token".into(),
            owner: "testowner".into(),
            repo: "testrepo".into(),
            active_states: vec!["open".into()],
            terminal_states: vec!["closed".into()],
            use_robot_api: false,
        };
        let tracker = GiteaTracker::new(config).unwrap();
        let poster = OutputPoster::new(tracker);
        (server, poster)
    }

    fn mock_comment_response(id: u64) -> ResponseTemplate {
        ResponseTemplate::new(201).set_body_json(serde_json::json!({
            "id": id,
            "body": "test",
            "user": {"login": "bot", "id": 1},
            "created_at": "2026-03-30T00:00:00Z",
            "updated_at": "2026-03-30T00:00:00Z"
        }))
    }

    #[tokio::test]
    async fn test_post_agent_output_pass() {
        let (server, poster) = setup().await;

        Mock::given(method("POST"))
            .and(path("/api/v1/repos/testowner/testrepo/issues/42/comments"))
            .and(body_string_contains("PASS"))
            .respond_with(mock_comment_response(201))
            .mount(&server)
            .await;

        let output = ReviewAgentOutput {
            agent: "security-sentinel".into(),
            findings: vec![],
            summary: "No issues found".into(),
            pass: true,
        };

        let id = poster.post_agent_output(42, "security-sentinel", &output).await.unwrap();
        assert_eq!(id, 201);
    }

    #[tokio::test]
    async fn test_post_agent_output_with_findings() {
        let (server, poster) = setup().await;

        Mock::given(method("POST"))
            .and(path("/api/v1/repos/testowner/testrepo/issues/42/comments"))
            .and(body_string_contains("FAIL"))
            .respond_with(mock_comment_response(202))
            .mount(&server)
            .await;

        let output = ReviewAgentOutput {
            agent: "security-sentinel".into(),
            findings: vec![
                ReviewFinding {
                    category: FindingCategory::Security,
                    severity: FindingSeverity::High,
                    finding: "SQL injection vulnerability".into(),
                    file: "src/db.rs".into(),
                    line: 42,
                    suggestion: Some("Use parameterised queries".into()),
                    confidence: 0.95,
                },
            ],
            summary: "Critical security issue found".into(),
            pass: false,
        };

        let id = poster.post_agent_output(42, "security-sentinel", &output).await.unwrap();
        assert_eq!(id, 202);
    }

    #[tokio::test]
    async fn test_post_agent_message() {
        let (server, poster) = setup().await;

        Mock::given(method("POST"))
            .and(path("/api/v1/repos/testowner/testrepo/issues/10/comments"))
            .respond_with(mock_comment_response(303))
            .mount(&server)
            .await;

        let id = poster.post_agent_message(10, "meta-coordinator", "All checks passed").await.unwrap();
        assert_eq!(id, 303);
    }

    #[tokio::test]
    async fn test_output_truncation() {
        let (server, poster) = setup().await;

        Mock::given(method("POST"))
            .and(path("/api/v1/repos/testowner/testrepo/issues/1/comments"))
            .respond_with(mock_comment_response(404))
            .mount(&server)
            .await;

        let long_message = "x".repeat(70_000);
        let id = poster.post_agent_message(1, "test-agent", &long_message).await.unwrap();
        assert_eq!(id, 404);
    }
}
