//! Gitea REST issue tracker client.

use crate::{Issue, IssueTracker, Result, TrackerError};
use async_trait::async_trait;
use jiff::Zoned;
use reqwest::Client;
use serde::{Deserialize, Serialize};

/// Configuration for Gitea tracker.
#[derive(Debug, Clone)]
pub struct GiteaConfig {
    pub base_url: String,
    pub token: String,
    pub owner: String,
    pub repo: String,
    pub active_states: Vec<String>,
    pub terminal_states: Vec<String>,
    pub use_robot_api: bool,
}

/// Gitea REST API client.
pub struct GiteaTracker {
    client: Client,
    config: GiteaConfig,
}

/// Gitea API issue response.
#[derive(Debug, Deserialize)]
pub struct GiteaIssue {
    pub id: u64,
    pub number: u64,
    pub title: String,
    pub body: Option<String>,
    pub state: String,
    pub html_url: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
    pub labels: Option<Vec<GiteaLabel>>,
}

/// Gitea label.
#[derive(Debug, Deserialize)]
struct GiteaLabel {
    name: String,
}

/// Gitea issue comment from the API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IssueComment {
    pub id: u64,
    /// Issue number (extracted from issue_url for repo-wide API, filled by caller for per-issue API).
    #[serde(default)]
    pub issue_number: u64,
    pub body: String,
    pub user: CommentUser,
    pub created_at: String,
    pub updated_at: String,
}

/// User who authored a comment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommentUser {
    pub login: String,
}

/// Backward-compatible alias.
pub type GiteaComment = IssueComment;
impl GiteaTracker {
    /// Create a new Gitea tracker from configuration.
    pub fn new(config: GiteaConfig) -> Result<Self> {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| TrackerError::Api {
                message: format!("Failed to create HTTP client: {e}"),
            })?;

        Ok(Self { client, config })
    }

    /// Build request with authentication.
    fn build_request(&self, method: reqwest::Method, url: &str) -> reqwest::RequestBuilder {
        self.client
            .request(method, url)
            .header("Authorization", format!("token {}", self.config.token))
            .header("Accept", "application/json")
    }

    /// Convert Gitea issue to normalised Issue.
    fn normalise_issue(&self, gi: GiteaIssue) -> Issue {
        let identifier = format!("{}/{}/{}", self.config.owner, self.config.repo, gi.number);
        let labels: Vec<String> = gi
            .labels
            .unwrap_or_default()
            .into_iter()
            .map(|l| l.name.to_lowercase())
            .collect();

        Issue {
            id: gi.id.to_string(),
            identifier,
            title: gi.title,
            description: gi.body,
            priority: None, // Gitea doesn't have native priority
            state: gi.state,
            branch_name: None,
            url: gi.html_url,
            labels,
            blocked_by: vec![], // Would need to fetch dependencies separately
            pagerank_score: None,
            created_at: gi.created_at.and_then(|s| parse_datetime(&s)),
            updated_at: gi.updated_at.and_then(|s| parse_datetime(&s)),
        }
    }
}

#[async_trait]
impl IssueTracker for GiteaTracker {
    async fn fetch_candidate_issues(&self) -> Result<Vec<Issue>> {
        let url = format!(
            "{}/api/v1/repos/{}/{}/issues?state=open&limit=100",
            self.config.base_url, self.config.owner, self.config.repo
        );

        let response = self
            .build_request(reqwest::Method::GET, &url)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(TrackerError::Api {
                message: format!("Gitea API error {}: {}", status, text),
            });
        }

        let gitea_issues: Vec<GiteaIssue> = response.json().await?;

        let issues: Vec<Issue> = gitea_issues
            .into_iter()
            .filter(|gi| {
                self.config
                    .active_states
                    .iter()
                    .any(|s| s.eq_ignore_ascii_case(&gi.state))
            })
            .map(|gi| self.normalise_issue(gi))
            .collect();

        tracing::info!(
            fetched = issues.len(),
            owner = %self.config.owner,
            repo = %self.config.repo,
            "fetched candidate issues from Gitea"
        );

        Ok(issues)
    }

    async fn fetch_issue_states_by_ids(&self, ids: &[String]) -> Result<Vec<Issue>> {
        let mut issues = Vec::new();

        for id in ids {
            let url = format!(
                "{}/api/v1/repos/{}/{}/issues/{}",
                self.config.base_url, self.config.owner, self.config.repo, id
            );

            let response = self
                .build_request(reqwest::Method::GET, &url)
                .send()
                .await?;

            if response.status().is_success() {
                let gi: GiteaIssue = response.json().await?;
                issues.push(self.normalise_issue(gi));
            }
        }

        Ok(issues)
    }

    async fn fetch_issues_by_states(&self, states: &[String]) -> Result<Vec<Issue>> {
        let url = format!(
            "{}/api/v1/repos/{}/{}/issues?state=open&limit=1000",
            self.config.base_url, self.config.owner, self.config.repo
        );

        let response = self
            .build_request(reqwest::Method::GET, &url)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(TrackerError::Api {
                message: format!("Gitea API error {}: {}", status, text),
            });
        }

        let gitea_issues: Vec<GiteaIssue> = response.json().await?;

        let issues: Vec<Issue> = gitea_issues
            .into_iter()
            .filter(|gi| states.iter().any(|s| s.eq_ignore_ascii_case(&gi.state)))
            .map(|gi| self.normalise_issue(gi))
            .collect();

        Ok(issues)
    }
}

impl GiteaTracker {
    /// Post a comment on a Gitea issue.
    pub async fn post_comment(&self, issue_number: u64, body: &str) -> Result<IssueComment> {
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
                message: format!(
                    "Gitea comment POST error {} on issue {}: {}",
                    status, issue_number, text
                ),
            });
        }
        response.json().await.map_err(TrackerError::Http)
    }

    /// Create a new Gitea issue.
    pub async fn create_issue(
        &self,
        title: &str,
        body: &str,
        labels: &[&str],
    ) -> Result<GiteaIssue> {
        let url = format!(
            "{}/api/v1/repos/{}/{}/issues",
            self.config.base_url, self.config.owner, self.config.repo
        );
        let response = self
            .build_request(reqwest::Method::POST, &url)
            .json(&serde_json::json!({
                "title": title,
                "body": body
                
            }))
            .send()
            .await?;
        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(TrackerError::Api {
                message: format!("Gitea create_issue error {}: {}", status, text),
            });
        }
        response.json().await.map_err(TrackerError::Http)
    }

    /// Search open issues by keyword in title.
    /// Returns issue numbers whose titles contain the given keyword.
    pub async fn search_issues_by_title(&self, keyword: &str) -> Result<Vec<u64>> {
        let url = format!(
            "{}/api/v1/repos/{}/{}/issues?state=open&q={}&type=issues",
            self.config.base_url,
            self.config.owner,
            self.config.repo,
            urlencoding::encode(keyword)
        );
        let response = self
            .build_request(reqwest::Method::GET, &url)
            .send()
            .await?;
        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(TrackerError::Api {
                message: format!("Gitea search issues error {}: {}", status, text),
            });
        }
        let issues: Vec<GiteaIssue> = response.json().await.map_err(TrackerError::Http)?;
        Ok(issues.into_iter().map(|i| i.number).collect())
    }

    /// Fetch comments on a Gitea issue, optionally filtering by `since` timestamp.
    pub async fn fetch_comments(
        &self,
        issue_number: u64,
        since: Option<&str>,
    ) -> Result<Vec<IssueComment>> {
        let mut url = format!(
            "{}/api/v1/repos/{}/{}/issues/{}/comments",
            self.config.base_url, self.config.owner, self.config.repo, issue_number
        );
        if let Some(since_ts) = since {
            url.push_str(&format!("?since={}", since_ts));
        }
        let response = self
            .build_request(reqwest::Method::GET, &url)
            .send()
            .await?;
        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(TrackerError::Api {
                message: format!(
                    "Gitea comments GET error {} on issue {}: {}",
                    status, issue_number, text
                ),
            });
        }
        response.json().await.map_err(TrackerError::Http)
    }

    /// Fetch comments across all issues in the repo, optionally filtering by `since` timestamp.
    ///
    /// This is the repo-wide endpoint that returns comments from all issues,
    /// which is more efficient than polling each issue individually.
    ///
    /// # API Endpoint
    ///
    /// `GET /api/v1/repos/{owner}/{repo}/issues/comments?since={since}&limit={limit}`
    ///
    /// # Response
    ///
    /// Each comment includes an `issue_url` field from which the issue number
    /// can be extracted.
    pub async fn fetch_repo_comments(
        &self,
        since: Option<&str>,
        limit: Option<u32>,
    ) -> Result<Vec<IssueComment>> {
        let mut url = format!(
            "{}/api/v1/repos/{}/{}/issues/comments",
            self.config.base_url, self.config.owner, self.config.repo
        );
        let mut params = Vec::new();
        if let Some(since_ts) = since {
            params.push(format!("since={}", since_ts));
        }
        if let Some(limit_val) = limit {
            params.push(format!("limit={}", limit_val));
        }
        if !params.is_empty() {
            url.push('?');
            url.push_str(&params.join("&"));
        }

        let response = self
            .build_request(reqwest::Method::GET, &url)
            .send()
            .await?;
        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(TrackerError::Api {
                message: format!("Gitea repo comments GET error {}: {}", status, text),
            });
        }

        // Parse response and extract issue numbers from issue_url
        let raw_comments: Vec<RepoComment> = response.json().await.map_err(TrackerError::Http)?;
        Ok(raw_comments.into_iter().map(|rc| rc.into()).collect())
    }
}

/// Raw comment from repo-wide API (includes issue_url instead of issue number).
#[derive(Debug, Clone, serde::Deserialize)]
struct RepoComment {
    id: u64,
    issue_url: String,
    user: CommentUser,
    body: String,
    created_at: String,
    updated_at: String,
}

impl From<RepoComment> for IssueComment {
    fn from(rc: RepoComment) -> Self {
        // Extract issue number from issue_url like "/api/v1/repos/owner/repo/issues/123"
        let issue_number = rc
            .issue_url
            .rsplit('/')
            .next()
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);

        IssueComment {
            id: rc.id,
            issue_number,
            body: rc.body,
            user: rc.user,
            created_at: rc.created_at,
            updated_at: rc.updated_at,
        }
    }
}

/// Parse ISO 8601 datetime string.
fn parse_datetime(s: &str) -> Option<Zoned> {
    s.parse::<jiff::Timestamp>()
        .ok()
        .map(|ts| ts.to_zoned(jiff::tz::TimeZone::UTC))
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{method, path, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn make_tracker(base_url: &str) -> GiteaTracker {
        let config = GiteaConfig {
            base_url: base_url.to_string(),
            token: "test-token".to_string(),
            owner: "testowner".to_string(),
            repo: "testrepo".to_string(),
            active_states: vec!["open".to_string()],
            terminal_states: vec!["closed".to_string()],
            use_robot_api: false,
        };
        GiteaTracker::new(config).unwrap()
    }

    fn test_config() -> GiteaConfig {
        GiteaConfig {
            base_url: "https://git.example.com".into(),
            token: "test-token".into(),
            owner: "testowner".into(),
            repo: "testrepo".into(),
            active_states: vec!["open".into(), "Todo".into()],
            terminal_states: vec!["closed".into(), "Done".into()],
            use_robot_api: false,
        }
    }

    #[test]
    fn normalise_issue_converts_fields() {
        let config = test_config();
        let tracker = GiteaTracker::new(config).unwrap();

        let gi = GiteaIssue {
            id: 42,
            number: 123,
            title: "Test Issue".into(),
            body: Some("Description".into()),
            state: "open".into(),
            html_url: Some("https://example.com/issue/123".into()),
            created_at: Some("2024-01-15T10:30:00Z".into()),
            updated_at: Some("2024-01-15T11:00:00Z".into()),
            labels: Some(vec![
                GiteaLabel { name: "bug".into() },
                GiteaLabel {
                    name: "Priority:High".into(),
                },
            ]),
        };

        let issue = tracker.normalise_issue(gi);

        assert_eq!(issue.id, "42");
        assert_eq!(issue.identifier, "testowner/testrepo/123");
        assert_eq!(issue.title, "Test Issue");
        assert_eq!(issue.description, Some("Description".into()));
        assert_eq!(issue.state, "open");
        assert!(issue.labels.contains(&"bug".to_string()));
        assert!(issue.labels.contains(&"priority:high".to_string()));
    }

    #[test]
    fn normalise_issue_lowercases_labels() {
        let config = test_config();
        let tracker = GiteaTracker::new(config).unwrap();

        let gi = GiteaIssue {
            id: 1,
            number: 1,
            title: "Test".into(),
            body: None,
            state: "open".into(),
            html_url: None,
            created_at: None,
            updated_at: None,
            labels: Some(vec![
                GiteaLabel { name: "BUG".into() },
                GiteaLabel {
                    name: "FEATURE".into(),
                },
            ]),
        };

        let issue = tracker.normalise_issue(gi);
        assert!(
            issue
                .labels
                .iter()
                .all(|l| l.chars().all(|c| !c.is_uppercase()))
        );
    }

    #[tokio::test]
    async fn test_post_comment_success() {
        let mock_server = MockServer::start().await;
        let comment_json = serde_json::json!({
            "id": 42,
            "body": "Test comment",
            "user": {"login": "testbot"},
            "created_at": "2026-03-31T12:00:00Z",
            "updated_at": "2026-03-31T12:00:00Z"
        });
        Mock::given(method("POST"))
            .and(path("/api/v1/repos/testowner/testrepo/issues/1/comments"))
            .respond_with(ResponseTemplate::new(201).set_body_json(&comment_json))
            .mount(&mock_server)
            .await;

        let tracker = make_tracker(&mock_server.uri());
        let result = tracker.post_comment(1, "Test comment").await;
        assert!(result.is_ok());
        let comment = result.unwrap();
        assert_eq!(comment.id, 42);
        assert_eq!(comment.body, "Test comment");
        assert_eq!(comment.user.login, "testbot");
    }

    #[tokio::test]
    async fn test_post_comment_error_returns_api_error() {
        let mock_server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/v1/repos/testowner/testrepo/issues/999/comments"))
            .respond_with(ResponseTemplate::new(403).set_body_string("forbidden"))
            .mount(&mock_server)
            .await;

        let tracker = make_tracker(&mock_server.uri());
        let result = tracker.post_comment(999, "body").await;
        assert!(result.is_err());
        let err_str = format!("{}", result.unwrap_err());
        assert!(
            err_str.contains("403"),
            "Expected 403 in error: {}",
            err_str
        );
    }

    #[tokio::test]
    async fn test_fetch_comments_without_since() {
        let mock_server = MockServer::start().await;
        let comments_json = serde_json::json!([
            {
                "id": 1,
                "body": "First",
                "user": {"login": "alice"},
                "created_at": "2026-03-31T10:00:00Z",
                "updated_at": "2026-03-31T10:00:00Z"
            },
            {
                "id": 2,
                "body": "Second",
                "user": {"login": "bob"},
                "created_at": "2026-03-31T11:00:00Z",
                "updated_at": "2026-03-31T11:00:00Z"
            }
        ]);
        Mock::given(method("GET"))
            .and(path("/api/v1/repos/testowner/testrepo/issues/5/comments"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&comments_json))
            .mount(&mock_server)
            .await;

        let tracker = make_tracker(&mock_server.uri());
        let result = tracker.fetch_comments(5, None).await;
        assert!(result.is_ok());
        let comments = result.unwrap();
        assert_eq!(comments.len(), 2);
        assert_eq!(comments[0].body, "First");
        assert_eq!(comments[1].user.login, "bob");
    }

    #[tokio::test]
    async fn test_fetch_comments_with_since() {
        let mock_server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v1/repos/testowner/testrepo/issues/5/comments"))
            .and(query_param("since", "2026-03-31T00:00:00Z"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([])))
            .mount(&mock_server)
            .await;

        let tracker = make_tracker(&mock_server.uri());
        let result = tracker
            .fetch_comments(5, Some("2026-03-31T00:00:00Z"))
            .await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 0);
    }

    #[tokio::test]
    async fn test_fetch_comments_error() {
        let mock_server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v1/repos/testowner/testrepo/issues/404/comments"))
            .respond_with(ResponseTemplate::new(404).set_body_string("not found"))
            .mount(&mock_server)
            .await;

        let tracker = make_tracker(&mock_server.uri());
        let result = tracker.fetch_comments(404, None).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_issue_comment_deserialisation() {
        let json = r#"{
            "id": 100,
            "body": "Hello @adf:security-sentinel",
            "user": {"login": "root"},
            "created_at": "2026-03-31T14:00:00+02:00",
            "updated_at": "2026-03-31T14:00:00+02:00"
        }"#;
        let comment: IssueComment = serde_json::from_str(json).unwrap();
        assert_eq!(comment.id, 100);
        assert!(comment.body.contains("@adf:security-sentinel"));
        assert_eq!(comment.user.login, "root");
    }
}
