//! Gitea issue comment read/write API.
//!
//! Extends GiteaTracker with methods to post and fetch comments
//! on Gitea issues. Used by the ADF orchestrator to enable
//! inter-agent communication via issue comments.

use crate::gitea::GiteaTracker;
use crate::{Result, TrackerError};
use serde::{Deserialize, Serialize};

/// A comment on a Gitea issue (from the Gitea REST API).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IssueComment {
    pub id: u64,
    pub body: String,
    pub user: CommentUser,
    pub created_at: String,
    pub updated_at: String,
}

/// The user who authored a comment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommentUser {
    pub login: String,
    pub id: u64,
}

impl GiteaTracker {
    /// Post a comment on a Gitea issue.
    ///
    /// Calls POST /api/v1/repos/{owner}/{repo}/issues/{index}/comments
    /// Returns the created comment.
    pub async fn post_comment(
        &self,
        issue_number: u64,
        body: &str,
    ) -> Result<IssueComment> {
        let url = format!(
            "{}/api/v1/repos/{}/{}/issues/{}/comments",
            self.config.base_url, self.config.owner, self.config.repo, issue_number
        );

        let response = self
            .build_request(reqwest::Method::POST, &url)
            .json(&serde_json::json!({"body": body}))
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(TrackerError::Api {
                message: format!("Gitea POST comment error {}: {}", status, text),
            });
        }

        let comment: IssueComment = response.json().await?;
        tracing::info!(
            issue = issue_number,
            comment_id = comment.id,
            "posted comment to Gitea issue"
        );
        Ok(comment)
    }

    /// Fetch comments on a Gitea issue.
    ///
    /// Calls GET /api/v1/repos/{owner}/{repo}/issues/{index}/comments
    /// If `since` is provided, only returns comments created after that ISO 8601 timestamp.
    /// Results are ordered by creation time ascending.
    pub async fn fetch_comments(
        &self,
        issue_number: u64,
        since: Option<&str>,
    ) -> Result<Vec<IssueComment>> {
        let mut url = format!(
            "{}/api/v1/repos/{}/{}/issues/{}/comments",
            self.config.base_url, self.config.owner, self.config.repo, issue_number
        );

        if let Some(ts) = since {
            url.push_str(&format!("?since={}", ts));
        }

        let response = self
            .build_request(reqwest::Method::GET, &url)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(TrackerError::Api {
                message: format!("Gitea GET comments error {}: {}", status, text),
            });
        }

        let comments: Vec<IssueComment> = response.json().await?;
        tracing::debug!(
            issue = issue_number,
            count = comments.len(),
            "fetched comments from Gitea issue"
        );
        Ok(comments)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gitea::GiteaConfig;
    use wiremock::matchers::{method, path, path_regex};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    async fn setup() -> (MockServer, GiteaTracker) {
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
        (server, tracker)
    }

    #[tokio::test]
    async fn test_post_comment_success() {
        let (server, tracker) = setup().await;

        Mock::given(method("POST"))
            .and(path("/api/v1/repos/testowner/testrepo/issues/42/comments"))
            .respond_with(ResponseTemplate::new(201).set_body_json(serde_json::json!({
                "id": 101,
                "body": "Hello from agent",
                "user": {"login": "bot", "id": 1},
                "created_at": "2026-03-30T00:00:00Z",
                "updated_at": "2026-03-30T00:00:00Z"
            })))
            .mount(&server)
            .await;

        let comment = tracker.post_comment(42, "Hello from agent").await.unwrap();
        assert_eq!(comment.id, 101);
        assert_eq!(comment.body, "Hello from agent");
        assert_eq!(comment.user.login, "bot");
    }

    #[tokio::test]
    async fn test_post_comment_auth_failure() {
        let (server, tracker) = setup().await;

        Mock::given(method("POST"))
            .and(path("/api/v1/repos/testowner/testrepo/issues/42/comments"))
            .respond_with(ResponseTemplate::new(403).set_body_string("forbidden"))
            .mount(&server)
            .await;

        let result = tracker.post_comment(42, "Hello").await;
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("403"), "error should contain status code: {}", err);
    }

    #[tokio::test]
    async fn test_fetch_comments_success() {
        let (server, tracker) = setup().await;

        Mock::given(method("GET"))
            .and(path("/api/v1/repos/testowner/testrepo/issues/42/comments"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([
                {
                    "id": 101,
                    "body": "First comment",
                    "user": {"login": "alice", "id": 2},
                    "created_at": "2026-03-30T00:00:00Z",
                    "updated_at": "2026-03-30T00:00:00Z"
                },
                {
                    "id": 102,
                    "body": "Second comment with @adf:vigil mention",
                    "user": {"login": "bob", "id": 3},
                    "created_at": "2026-03-30T01:00:00Z",
                    "updated_at": "2026-03-30T01:00:00Z"
                }
            ])))
            .mount(&server)
            .await;

        let comments = tracker.fetch_comments(42, None).await.unwrap();
        assert_eq!(comments.len(), 2);
        assert_eq!(comments[0].body, "First comment");
        assert_eq!(comments[1].body, "Second comment with @adf:vigil mention");
    }

    #[tokio::test]
    async fn test_fetch_comments_empty() {
        let (server, tracker) = setup().await;

        Mock::given(method("GET"))
            .and(path("/api/v1/repos/testowner/testrepo/issues/99/comments"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([])))
            .mount(&server)
            .await;

        let comments = tracker.fetch_comments(99, None).await.unwrap();
        assert!(comments.is_empty());
    }

    #[tokio::test]
    async fn test_fetch_comments_since_filter() {
        let (server, tracker) = setup().await;

        Mock::given(method("GET"))
            .and(path_regex(r"/api/v1/repos/testowner/testrepo/issues/42/comments.*"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([
                {
                    "id": 102,
                    "body": "New comment after timestamp",
                    "user": {"login": "bob", "id": 3},
                    "created_at": "2026-03-30T12:00:00Z",
                    "updated_at": "2026-03-30T12:00:00Z"
                }
            ])))
            .mount(&server)
            .await;

        let comments = tracker.fetch_comments(42, Some("2026-03-30T10:00:00Z")).await.unwrap();
        assert_eq!(comments.len(), 1);
        assert_eq!(comments[0].id, 102);
    }
}
