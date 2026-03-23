//! Gitea issue tracker implementation
//!
//! Provides integration with Gitea REST API v1 and gitea-robot for PageRank.

use crate::{
    IssueState, IssueTracker, ListIssuesParams, Result, TrackedIssue, TrackerConfig, TrackerError,
};
use async_trait::async_trait;
use reqwest::{Client, Method, StatusCode};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{debug, info, warn};

/// Gitea API issue response
#[derive(Debug, Deserialize)]
struct GiteaIssue {
    number: u64,
    title: String,
    state: String,
    #[serde(default)]
    labels: Vec<GiteaLabel>,
    #[serde(default)]
    assignees: Vec<GiteaUser>,
    body: Option<String>,
    #[serde(default)]
    created_at: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(default)]
    updated_at: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(default)]
    closed_at: Option<chrono::DateTime<chrono::Utc>>,
    url: Option<String>,
    #[serde(flatten)]
    extra: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Deserialize)]
struct GiteaLabel {
    name: String,
}

#[derive(Debug, Deserialize)]
struct GiteaUser {
    login: String,
}

/// Gitea robot PageRank response
#[derive(Debug, Deserialize)]
struct RobotRankResponse {
    #[serde(default)]
    issues: Vec<RobotIssueRank>,
}

#[derive(Debug, Deserialize)]
struct RobotIssueRank {
    number: u64,
    score: f64,
}

/// Request body for creating/updating issues
#[derive(Debug, Serialize)]
struct CreateIssueRequest {
    title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    body: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    labels: Option<Vec<String>>,
}

/// Request body for updating issue state
#[derive(Debug, Serialize)]
struct UpdateIssueStateRequest {
    state: String,
}

/// Request body for assigning issue
#[derive(Debug, Serialize)]
struct AssignIssueRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    assignees: Option<Vec<String>>,
}

/// Gitea issue tracker implementation
pub struct GiteaTracker {
    config: TrackerConfig,
    client: Client,
    base_url: String,
}

impl GiteaTracker {
    /// Create a new Gitea tracker
    pub fn new(config: TrackerConfig) -> Result<Self> {
        config.validate()?;

        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(TrackerError::NetworkError)?;

        // Normalize base URL (remove trailing slash)
        let base_url = config.url.trim_end_matches('/').to_string();

        Ok(Self {
            config,
            client,
            base_url,
        })
    }

    /// Get the tracker configuration
    pub fn config(&self) -> &TrackerConfig {
        &self.config
    }

    /// Build API URL for a path
    fn api_url(&self, path: &str) -> String {
        format!("{}/api/v1{}", self.base_url, path)
    }

    /// Build request with authentication
    fn build_request(&self, method: Method, path: &str) -> reqwest::RequestBuilder {
        self.client
            .request(method, self.api_url(path))
            .header("Authorization", format!("token {}", self.config.token))
            .header("Content-Type", "application/json")
    }

    /// Convert Gitea issue to TrackedIssue
    fn convert_issue(&self, issue: GiteaIssue) -> TrackedIssue {
        let state = match issue.state.as_str() {
            "closed" | "CLOSED" => IssueState::Closed,
            _ => IssueState::Open,
        };

        TrackedIssue {
            id: issue.number,
            title: issue.title,
            state,
            labels: issue.labels.into_iter().map(|l| l.name).collect(),
            assignees: issue.assignees.into_iter().map(|u| u.login).collect(),
            priority: None,        // Gitea doesn't have built-in priority field
            page_rank_score: None, // Will be populated separately
            body: issue.body,
            created_at: issue.created_at,
            updated_at: issue.updated_at,
            closed_at: issue.closed_at,
            url: issue.url,
            extra: issue.extra,
        }
    }

    /// Fetch PageRank scores from gitea-robot
    async fn fetch_page_ranks(&self) -> Result<HashMap<u64, f64>> {
        let robot_url = match &self.config.robot_url {
            Some(url) => url,
            None => return Ok(HashMap::new()), // No robot configured, return empty
        };

        let url = format!(
            "{}/triage?owner={}&repo={}",
            robot_url.trim_end_matches('/'),
            self.config.owner,
            self.config.repo
        );

        debug!(url = %url, "Fetching PageRank scores from robot");

        let response = self
            .client
            .get(&url)
            .timeout(std::time::Duration::from_secs(10))
            .send()
            .await;

        match response {
            Ok(resp) if resp.status().is_success() => {
                let data: RobotRankResponse = resp.json().await?;
                let ranks: HashMap<u64, f64> = data
                    .issues
                    .into_iter()
                    .map(|r| (r.number, r.score))
                    .collect();
                info!(count = ranks.len(), "Fetched PageRank scores");
                Ok(ranks)
            }
            Ok(resp) => {
                warn!(status = %resp.status(), "Robot API returned non-success status");
                Ok(HashMap::new())
            }
            Err(e) => {
                warn!(error = %e, "Failed to fetch PageRank scores");
                Ok(HashMap::new()) // Non-fatal, return empty
            }
        }
    }

    /// Enrich issues with PageRank scores
    async fn enrich_with_page_ranks(
        &self,
        mut issues: Vec<TrackedIssue>,
    ) -> Result<Vec<TrackedIssue>> {
        let ranks = self.fetch_page_ranks().await?;

        for issue in &mut issues {
            if let Some(score) = ranks.get(&issue.id) {
                issue.page_rank_score = Some(*score);
            }
        }

        Ok(issues)
    }

    /// Get repository issues path
    fn repo_issues_path(&self) -> String {
        format!("/repos/{}/{}/issues", self.config.owner, self.config.repo)
    }

    /// Get single issue path
    fn issue_path(&self, id: u64) -> String {
        format!(
            "/repos/{}/{}/issues/{}",
            self.config.owner, self.config.repo, id
        )
    }

    /// Get issue labels path
    fn issue_labels_path(&self, id: u64) -> String {
        format!(
            "/repos/{}/{}/issues/{}/labels",
            self.config.owner, self.config.repo, id
        )
    }
}

#[async_trait]
impl IssueTracker for GiteaTracker {
    async fn list_issues(&self, params: ListIssuesParams) -> Result<Vec<TrackedIssue>> {
        let mut query = Vec::new();

        if let Some(state) = params.state {
            query.push(("state", state.to_string()));
        }

        if let Some(labels) = params.labels {
            query.push(("labels", labels.join(",")));
        }

        if let Some(assignee) = params.assignee {
            query.push(("assignee", assignee));
        }

        if let Some(limit) = params.limit {
            query.push(("limit", limit.to_string()));
        }

        if let Some(page) = params.page {
            query.push(("page", page.to_string()));
        }

        let path = self.repo_issues_path();
        let request = self.build_request(Method::GET, &path).query(&query);

        debug!(path = %path, "Listing issues");

        let response = request.send().await?;

        match response.status() {
            StatusCode::OK => {
                let gitea_issues: Vec<GiteaIssue> = response.json().await?;
                let issues: Vec<TrackedIssue> = gitea_issues
                    .into_iter()
                    .map(|i| self.convert_issue(i))
                    .collect();

                // Enrich with PageRank scores
                self.enrich_with_page_ranks(issues).await
            }
            StatusCode::UNAUTHORIZED => Err(TrackerError::AuthenticationError(
                "Invalid token".to_string(),
            )),
            StatusCode::NOT_FOUND => Err(TrackerError::NotFound(format!(
                "Repository {}/{} not found",
                self.config.owner, self.config.repo
            ))),
            status => {
                let text = response.text().await.unwrap_or_default();
                Err(TrackerError::ApiError(format!(
                    "Unexpected status {}: {}",
                    status, text
                )))
            }
        }
    }

    async fn get_issue(&self, id: u64) -> Result<TrackedIssue> {
        let path = self.issue_path(id);
        let request = self.build_request(Method::GET, &path);

        debug!(issue_id = id, "Getting issue");

        let response = request.send().await?;

        match response.status() {
            StatusCode::OK => {
                let gitea_issue: GiteaIssue = response.json().await?;
                let mut issue = self.convert_issue(gitea_issue);

                // Try to enrich with PageRank
                let ranks = self.fetch_page_ranks().await?;
                if let Some(score) = ranks.get(&issue.id) {
                    issue.page_rank_score = Some(*score);
                }

                Ok(issue)
            }
            StatusCode::UNAUTHORIZED => Err(TrackerError::AuthenticationError(
                "Invalid token".to_string(),
            )),
            StatusCode::NOT_FOUND => Err(TrackerError::NotFound(format!("Issue {} not found", id))),
            status => {
                let text = response.text().await.unwrap_or_default();
                Err(TrackerError::ApiError(format!(
                    "Unexpected status {}: {}",
                    status, text
                )))
            }
        }
    }

    async fn create_issue(
        &self,
        title: &str,
        body: Option<&str>,
        labels: Option<Vec<String>>,
    ) -> Result<TrackedIssue> {
        let path = self.repo_issues_path();
        let request_body = CreateIssueRequest {
            title: title.to_string(),
            body: body.map(|s| s.to_string()),
            labels,
        };

        let request = self.build_request(Method::POST, &path).json(&request_body);

        info!(title = %title, "Creating issue");

        let response = request.send().await?;

        match response.status() {
            StatusCode::CREATED => {
                let gitea_issue: GiteaIssue = response.json().await?;
                Ok(self.convert_issue(gitea_issue))
            }
            StatusCode::UNAUTHORIZED => Err(TrackerError::AuthenticationError(
                "Invalid token".to_string(),
            )),
            status => {
                let text = response.text().await.unwrap_or_default();
                Err(TrackerError::ApiError(format!(
                    "Unexpected status {}: {}",
                    status, text
                )))
            }
        }
    }

    async fn update_issue(
        &self,
        id: u64,
        title: Option<&str>,
        body: Option<&str>,
        labels: Option<Vec<String>>,
    ) -> Result<TrackedIssue> {
        let path = self.issue_path(id);
        let request_body = CreateIssueRequest {
            title: title.map(|s| s.to_string()).unwrap_or_default(),
            body: body.map(|s| s.to_string()),
            labels,
        };

        let request = self.build_request(Method::PATCH, &path).json(&request_body);

        debug!(issue_id = id, "Updating issue");

        let response = request.send().await?;

        match response.status() {
            StatusCode::OK => {
                let gitea_issue: GiteaIssue = response.json().await?;
                Ok(self.convert_issue(gitea_issue))
            }
            StatusCode::UNAUTHORIZED => Err(TrackerError::AuthenticationError(
                "Invalid token".to_string(),
            )),
            StatusCode::NOT_FOUND => Err(TrackerError::NotFound(format!("Issue {} not found", id))),
            status => {
                let text = response.text().await.unwrap_or_default();
                Err(TrackerError::ApiError(format!(
                    "Unexpected status {}: {}",
                    status, text
                )))
            }
        }
    }

    async fn close_issue(&self, id: u64) -> Result<TrackedIssue> {
        let path = self.issue_path(id);
        let request_body = UpdateIssueStateRequest {
            state: "closed".to_string(),
        };

        let request = self.build_request(Method::PATCH, &path).json(&request_body);

        info!(issue_id = id, "Closing issue");

        let response = request.send().await?;

        match response.status() {
            StatusCode::OK => {
                let gitea_issue: GiteaIssue = response.json().await?;
                Ok(self.convert_issue(gitea_issue))
            }
            StatusCode::UNAUTHORIZED => Err(TrackerError::AuthenticationError(
                "Invalid token".to_string(),
            )),
            StatusCode::NOT_FOUND => Err(TrackerError::NotFound(format!("Issue {} not found", id))),
            status => {
                let text = response.text().await.unwrap_or_default();
                Err(TrackerError::ApiError(format!(
                    "Unexpected status {}: {}",
                    status, text
                )))
            }
        }
    }

    async fn add_labels(&self, id: u64, labels: Vec<String>) -> Result<TrackedIssue> {
        let path = self.issue_labels_path(id);
        let request = self.build_request(Method::POST, &path).json(&labels);

        debug!(issue_id = id, labels = ?labels, "Adding labels");

        let response = request.send().await?;

        match response.status() {
            StatusCode::OK => {
                let gitea_issue: GiteaIssue = response.json().await?;
                Ok(self.convert_issue(gitea_issue))
            }
            StatusCode::UNAUTHORIZED => Err(TrackerError::AuthenticationError(
                "Invalid token".to_string(),
            )),
            StatusCode::NOT_FOUND => Err(TrackerError::NotFound(format!("Issue {} not found", id))),
            status => {
                let text = response.text().await.unwrap_or_default();
                Err(TrackerError::ApiError(format!(
                    "Unexpected status {}: {}",
                    status, text
                )))
            }
        }
    }

    async fn remove_labels(&self, id: u64, labels: Vec<String>) -> Result<TrackedIssue> {
        let path = self.issue_labels_path(id);
        let label_param = labels.join(",");
        let request = self
            .build_request(Method::DELETE, &path)
            .query(&[("labels", label_param)]);

        debug!(issue_id = id, labels = ?labels, "Removing labels");

        let response = request.send().await?;

        match response.status() {
            StatusCode::OK => {
                let gitea_issue: GiteaIssue = response.json().await?;
                Ok(self.convert_issue(gitea_issue))
            }
            StatusCode::UNAUTHORIZED => Err(TrackerError::AuthenticationError(
                "Invalid token".to_string(),
            )),
            StatusCode::NOT_FOUND => Err(TrackerError::NotFound(format!("Issue {} not found", id))),
            status => {
                let text = response.text().await.unwrap_or_default();
                Err(TrackerError::ApiError(format!(
                    "Unexpected status {}: {}",
                    status, text
                )))
            }
        }
    }

    async fn assign_issue(&self, id: u64, assignees: Vec<String>) -> Result<TrackedIssue> {
        let path = self.issue_path(id);
        let request_body = AssignIssueRequest {
            assignees: Some(assignees),
        };

        let request = self.build_request(Method::PATCH, &path).json(&request_body);

        debug!(issue_id = id, "Assigning issue");

        let response = request.send().await?;

        match response.status() {
            StatusCode::OK => {
                let gitea_issue: GiteaIssue = response.json().await?;
                Ok(self.convert_issue(gitea_issue))
            }
            StatusCode::UNAUTHORIZED => Err(TrackerError::AuthenticationError(
                "Invalid token".to_string(),
            )),
            StatusCode::NOT_FOUND => Err(TrackerError::NotFound(format!("Issue {} not found", id))),
            status => {
                let text = response.text().await.unwrap_or_default();
                Err(TrackerError::ApiError(format!(
                    "Unexpected status {}: {}",
                    status, text
                )))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{body_json, header, method, path, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn create_test_config(server_url: &str) -> TrackerConfig {
        TrackerConfig::new(server_url, "test-token", "test-owner", "test-repo")
    }

    #[test]
    fn test_gitea_tracker_creation() {
        let config = TrackerConfig::new("https://git.example.com", "token123", "owner", "repo");

        let tracker = GiteaTracker::new(config);
        assert!(tracker.is_ok());
    }

    #[tokio::test]
    async fn test_list_issues_success() {
        let mock_server = MockServer::start().await;

        let mock_response = serde_json::json!([
            {
                "number": 1,
                "title": "Test Issue 1",
                "state": "open",
                "labels": [{"name": "bug"}],
                "assignees": [{"login": "alice"}],
                "body": "Issue body",
                "url": "https://git.example.com/issues/1"
            },
            {
                "number": 2,
                "title": "Test Issue 2",
                "state": "closed",
                "labels": [],
                "assignees": [],
                "body": null,
                "url": "https://git.example.com/issues/2"
            }
        ]);

        Mock::given(method("GET"))
            .and(path("/api/v1/repos/test-owner/test-repo/issues"))
            .and(header("Authorization", "token test-token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(mock_response))
            .mount(&mock_server)
            .await;

        let config = create_test_config(&mock_server.uri());
        let tracker = GiteaTracker::new(config).unwrap();

        let issues = tracker.list_issues(ListIssuesParams::new()).await.unwrap();

        assert_eq!(issues.len(), 2);
        assert_eq!(issues[0].id, 1);
        assert_eq!(issues[0].title, "Test Issue 1");
        assert!(issues[0].is_open());
        assert_eq!(issues[1].id, 2);
        assert!(issues[1].is_closed());
    }

    #[tokio::test]
    async fn test_get_issue_success() {
        let mock_server = MockServer::start().await;

        let mock_response = serde_json::json!({
            "number": 42,
            "title": "Specific Issue",
            "state": "open",
            "labels": [{"name": "feature"}],
            "assignees": [],
            "body": "Issue description",
            "url": "https://git.example.com/issues/42"
        });

        Mock::given(method("GET"))
            .and(path("/api/v1/repos/test-owner/test-repo/issues/42"))
            .and(header("Authorization", "token test-token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(mock_response))
            .mount(&mock_server)
            .await;

        let config = create_test_config(&mock_server.uri());
        let tracker = GiteaTracker::new(config).unwrap();

        let issue = tracker.get_issue(42).await.unwrap();

        assert_eq!(issue.id, 42);
        assert_eq!(issue.title, "Specific Issue");
        assert!(issue.has_label("feature"));
    }

    #[tokio::test]
    async fn test_create_issue_success() {
        let mock_server = MockServer::start().await;

        let mock_response = serde_json::json!({
            "number": 100,
            "title": "New Issue",
            "state": "open",
            "labels": [{"name": "bug"}],
            "assignees": [],
            "body": "New issue body",
            "url": "https://git.example.com/issues/100"
        });

        Mock::given(method("POST"))
            .and(path("/api/v1/repos/test-owner/test-repo/issues"))
            .and(header("Authorization", "token test-token"))
            .and(body_json(serde_json::json!({
                "title": "New Issue",
                "body": "New issue body",
                "labels": ["bug"]
            })))
            .respond_with(ResponseTemplate::new(201).set_body_json(mock_response))
            .mount(&mock_server)
            .await;

        let config = create_test_config(&mock_server.uri());
        let tracker = GiteaTracker::new(config).unwrap();

        let issue = tracker
            .create_issue(
                "New Issue",
                Some("New issue body"),
                Some(vec!["bug".to_string()]),
            )
            .await
            .unwrap();

        assert_eq!(issue.id, 100);
        assert_eq!(issue.title, "New Issue");
    }

    #[tokio::test]
    async fn test_close_issue_success() {
        let mock_server = MockServer::start().await;

        let mock_response = serde_json::json!({
            "number": 1,
            "title": "Issue to Close",
            "state": "closed",
            "labels": [],
            "assignees": [],
            "body": null,
            "url": "https://git.example.com/issues/1"
        });

        Mock::given(method("PATCH"))
            .and(path("/api/v1/repos/test-owner/test-repo/issues/1"))
            .and(header("Authorization", "token test-token"))
            .and(body_json(serde_json::json!({
                "state": "closed"
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(mock_response))
            .mount(&mock_server)
            .await;

        let config = create_test_config(&mock_server.uri());
        let tracker = GiteaTracker::new(config).unwrap();

        let issue = tracker.close_issue(1).await.unwrap();

        assert!(issue.is_closed());
    }

    #[tokio::test]
    async fn test_issue_not_found() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/api/v1/repos/test-owner/test-repo/issues/999"))
            .and(header("Authorization", "token test-token"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&mock_server)
            .await;

        let config = create_test_config(&mock_server.uri());
        let tracker = GiteaTracker::new(config).unwrap();

        let result = tracker.get_issue(999).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), TrackerError::NotFound(_)));
    }

    #[tokio::test]
    async fn test_authentication_error() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/api/v1/repos/test-owner/test-repo/issues"))
            .respond_with(ResponseTemplate::new(401))
            .mount(&mock_server)
            .await;

        let config = create_test_config(&mock_server.uri());
        let tracker = GiteaTracker::new(config).unwrap();

        let result = tracker.list_issues(ListIssuesParams::new()).await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            TrackerError::AuthenticationError(_)
        ));
    }

    #[tokio::test]
    async fn test_list_issues_with_params() {
        let mock_server = MockServer::start().await;

        let mock_response = serde_json::json!([
            {
                "number": 1,
                "title": "Bug Issue",
                "state": "open",
                "labels": [{"name": "bug"}],
                "assignees": [{"login": "alice"}],
                "body": null,
                "url": "https://git.example.com/issues/1"
            }
        ]);

        Mock::given(method("GET"))
            .and(path("/api/v1/repos/test-owner/test-repo/issues"))
            .and(query_param("state", "open"))
            .and(query_param("labels", "bug"))
            .and(query_param("assignee", "alice"))
            .and(query_param("limit", "10"))
            .respond_with(ResponseTemplate::new(200).set_body_json(mock_response))
            .mount(&mock_server)
            .await;

        let config = create_test_config(&mock_server.uri());
        let tracker = GiteaTracker::new(config).unwrap();

        let params = ListIssuesParams::new()
            .with_state(IssueState::Open)
            .with_labels(vec!["bug".to_string()])
            .with_assignee("alice")
            .with_limit(10);

        let issues = tracker.list_issues(params).await.unwrap();
        assert_eq!(issues.len(), 1);
        assert!(issues[0].has_label("bug"));
    }

    #[tokio::test]
    async fn test_page_rank_integration() {
        let mock_server = MockServer::start().await;
        let robot_server = MockServer::start().await;

        // Mock Gitea API response
        let mock_issues = serde_json::json!([
            {
                "number": 1,
                "title": "Issue 1",
                "state": "open",
                "labels": [],
                "assignees": [],
                "body": null
            },
            {
                "number": 2,
                "title": "Issue 2",
                "state": "open",
                "labels": [],
                "assignees": [],
                "body": null
            }
        ]);

        Mock::given(method("GET"))
            .and(path("/api/v1/repos/test-owner/test-repo/issues"))
            .respond_with(ResponseTemplate::new(200).set_body_json(mock_issues))
            .mount(&mock_server)
            .await;

        // Mock robot PageRank response
        let mock_ranks = serde_json::json!({
            "issues": [
                {"number": 1, "score": 0.95},
                {"number": 2, "score": 0.72}
            ]
        });

        Mock::given(method("GET"))
            .and(path("/triage"))
            .and(query_param("owner", "test-owner"))
            .and(query_param("repo", "test-repo"))
            .respond_with(ResponseTemplate::new(200).set_body_json(mock_ranks))
            .mount(&robot_server)
            .await;

        let mut config = create_test_config(&mock_server.uri());
        config.robot_url = Some(robot_server.uri());

        let tracker = GiteaTracker::new(config).unwrap();
        let issues = tracker.list_issues(ListIssuesParams::new()).await.unwrap();

        assert_eq!(issues.len(), 2);
        assert_eq!(issues[0].page_rank_score, Some(0.95));
        assert_eq!(issues[1].page_rank_score, Some(0.72));
    }
}
