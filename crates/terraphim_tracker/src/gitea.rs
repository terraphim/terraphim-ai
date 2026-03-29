//! Gitea REST issue tracker client.

use crate::{Issue, IssueTracker, Result, TrackerError};
use async_trait::async_trait;
use jiff::Zoned;
use reqwest::Client;
use serde::Deserialize;

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
    pub(crate) config: GiteaConfig,
}

/// Gitea API issue response.
#[derive(Debug, Deserialize)]
struct GiteaIssue {
    id: u64,
    number: u64,
    title: String,
    body: Option<String>,
    state: String,
    html_url: Option<String>,
    created_at: Option<String>,
    updated_at: Option<String>,
    labels: Option<Vec<GiteaLabel>>,
}

/// Gitea label.
#[derive(Debug, Deserialize)]
struct GiteaLabel {
    name: String,
}

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
    pub(crate) fn build_request(&self, method: reqwest::Method, url: &str) -> reqwest::RequestBuilder {
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

/// Parse ISO 8601 datetime string.
fn parse_datetime(s: &str) -> Option<Zoned> {
    s.parse::<jiff::Timestamp>()
        .ok()
        .map(|ts| ts.to_zoned(jiff::tz::TimeZone::UTC))
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
