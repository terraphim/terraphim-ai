//! Gitea REST issue tracker client.
//!
//! Fetches issues from a Gitea instance using the REST API, normalising
//! them to the common [`Issue`](super::Issue) model.

use super::{Issue, IssueTracker};
use crate::config::ServiceConfig;
use crate::error::{Result, SymphonyError};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use reqwest::Client;
use serde::Deserialize;
use tracing::debug;

/// Gitea REST API client.
pub struct GiteaTracker {
    client: Client,
    base_url: String,
    token: String,
    owner: String,
    repo: String,
    active_states: Vec<String>,
    terminal_states: Vec<String>,
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
    #[serde(default)]
    ref_field: Option<String>,
}

/// Gitea label.
#[derive(Debug, Deserialize)]
struct GiteaLabel {
    name: String,
}

impl GiteaTracker {
    /// Create a new Gitea tracker from the service configuration.
    pub fn from_config(config: &ServiceConfig) -> Result<Self> {
        let token = config
            .tracker_api_key()
            .or_else(|| {
                std::env::var("GITEA_TOKEN")
                    .ok()
                    .filter(|v| !v.is_empty())
            })
            .ok_or_else(|| SymphonyError::AuthenticationMissing {
                service: "gitea".into(),
            })?;

        let owner =
            config
                .tracker_gitea_owner()
                .ok_or_else(|| SymphonyError::ValidationFailed {
                    checks: vec!["tracker.owner is required for gitea".into()],
                })?;

        let repo =
            config
                .tracker_gitea_repo()
                .ok_or_else(|| SymphonyError::ValidationFailed {
                    checks: vec!["tracker.repo is required for gitea".into()],
                })?;

        let base_url = config.tracker_endpoint();

        Ok(Self {
            client: Client::new(),
            base_url,
            token,
            owner,
            repo,
            active_states: config.active_states(),
            terminal_states: config.terminal_states(),
        })
    }

    /// Build the full API URL for an issues endpoint.
    fn issues_url(&self) -> String {
        format!(
            "{}/api/v1/repos/{}/{}/issues",
            self.base_url.trim_end_matches('/'),
            self.owner,
            self.repo
        )
    }

    /// Convert a Gitea API issue to the common Issue model.
    fn normalise_issue(&self, gitea: &GiteaIssue) -> Issue {
        let identifier = format!("{}/{}#{}", self.owner, self.repo, gitea.number);

        let labels: Vec<String> = gitea
            .labels
            .as_ref()
            .map(|ls| ls.iter().map(|l| l.name.to_lowercase()).collect())
            .unwrap_or_default();

        // Derive state from labels if present, otherwise use Gitea's open/closed
        let state = self.derive_state(&labels, &gitea.state);

        // Extract priority from labels (e.g. "priority/P1-high" -> 1)
        let priority = self.extract_priority(&labels);

        let created_at = gitea
            .created_at
            .as_deref()
            .and_then(|s| s.parse::<DateTime<Utc>>().ok());
        let updated_at = gitea
            .updated_at
            .as_deref()
            .and_then(|s| s.parse::<DateTime<Utc>>().ok());

        Issue {
            id: gitea.id.to_string(),
            identifier,
            title: gitea.title.clone(),
            description: gitea.body.clone(),
            priority,
            state,
            branch_name: gitea.ref_field.clone(),
            url: gitea.html_url.clone(),
            labels,
            blocked_by: vec![], // Gitea dependencies fetched separately if needed
            created_at,
            updated_at,
        }
    }

    /// Derive the issue state from labels or Gitea status.
    ///
    /// Looks for labels matching configured active/terminal states (case-insensitive).
    /// Falls back to mapping Gitea's "open"/"closed" to the first active/terminal state.
    fn derive_state(&self, labels: &[String], gitea_state: &str) -> String {
        // Check labels for known state names
        for label in labels {
            // Strip prefix patterns like "status/" or "state/"
            let cleaned = label
                .strip_prefix("status/")
                .or_else(|| label.strip_prefix("state/"))
                .unwrap_or(label);

            for active in &self.active_states {
                if active.eq_ignore_ascii_case(cleaned) {
                    return active.clone();
                }
            }
            for terminal in &self.terminal_states {
                if terminal.eq_ignore_ascii_case(cleaned) {
                    return terminal.clone();
                }
            }
        }

        // Fall back to Gitea's open/closed mapping
        match gitea_state {
            "open" => self
                .active_states
                .first()
                .cloned()
                .unwrap_or_else(|| "Todo".into()),
            "closed" => self
                .terminal_states
                .first()
                .cloned()
                .unwrap_or_else(|| "Done".into()),
            other => other.to_string(),
        }
    }

    /// Extract priority from labels (e.g. "priority/p1-high" -> 1).
    fn extract_priority(&self, labels: &[String]) -> Option<i32> {
        for label in labels {
            let cleaned = label
                .strip_prefix("priority/")
                .unwrap_or(label);

            // Match patterns like "p1", "p2", "p1-high"
            if let Some(rest) = cleaned.strip_prefix("p") {
                if let Some(digit) = rest.chars().next() {
                    if let Some(n) = digit.to_digit(10) {
                        return Some(n as i32);
                    }
                }
            }
        }
        None
    }
}

#[async_trait]
impl IssueTracker for GiteaTracker {
    async fn fetch_candidate_issues(&self) -> Result<Vec<Issue>> {
        let url = self.issues_url();

        let mut all_issues = Vec::new();
        let mut page = 1u32;

        loop {
            let resp = self
                .client
                .get(&url)
                .header("Authorization", format!("token {}", self.token))
                .query(&[
                    ("state", "open"),
                    ("type", "issues"),
                    ("limit", "50"),
                    ("page", &page.to_string()),
                ])
                .send()
                .await
                .map_err(|e| SymphonyError::Tracker {
                    kind: "gitea".into(),
                    message: format!("request failed: {e}"),
                })?;

            let status = resp.status();
            if !status.is_success() {
                let text = resp.text().await.unwrap_or_default();
                return Err(SymphonyError::Tracker {
                    kind: "gitea".into(),
                    message: format!("HTTP {status}: {text}"),
                });
            }

            let issues: Vec<GiteaIssue> =
                resp.json().await.map_err(|e| SymphonyError::Tracker {
                    kind: "gitea".into(),
                    message: format!("response parse error: {e}"),
                })?;

            if issues.is_empty() {
                break;
            }

            for gi in &issues {
                let issue = self.normalise_issue(gi);
                // Only include issues in active states
                let is_active = self
                    .active_states
                    .iter()
                    .any(|s| s.eq_ignore_ascii_case(&issue.state));
                if is_active {
                    all_issues.push(issue);
                }
            }

            // Gitea returns fewer than limit when at the last page
            if issues.len() < 50 {
                break;
            }
            page += 1;
        }

        debug!(count = all_issues.len(), "fetched issues from Gitea");
        Ok(all_issues)
    }

    async fn fetch_issue_states_by_ids(&self, ids: &[String]) -> Result<Vec<Issue>> {
        if ids.is_empty() {
            return Ok(vec![]);
        }

        // Gitea doesn't support bulk ID lookup easily, so we fetch individual issues.
        // For small numbers of running issues this is acceptable.
        let mut results = Vec::new();

        // Gitea doesn't support bulk ID lookup directly; fetch all open issues
        // and filter by ID. For small numbers of running issues this is acceptable.
        let all = self.fetch_candidate_issues().await?;
        for issue in all {
            if ids.contains(&issue.id) {
                results.push(issue);
            }
        }

        Ok(results)
    }

    async fn fetch_issues_by_states(&self, states: &[String]) -> Result<Vec<Issue>> {
        if states.is_empty() {
            return Ok(vec![]);
        }

        // Determine if we need open, closed, or both
        let need_open = states.iter().any(|s| {
            self.active_states
                .iter()
                .any(|a| a.eq_ignore_ascii_case(s))
        });
        let need_closed = states.iter().any(|s| {
            self.terminal_states
                .iter()
                .any(|t| t.eq_ignore_ascii_case(s))
        });

        let mut all_issues = Vec::new();

        let gitea_states: Vec<&str> = match (need_open, need_closed) {
            (true, true) => vec!["open", "closed"],
            (true, false) => vec!["open"],
            (false, true) => vec!["closed"],
            (false, false) => return Ok(vec![]),
        };

        for gitea_state in gitea_states {
            let mut page = 1u32;
            loop {
                let resp = self
                    .client
                    .get(self.issues_url())
                    .header("Authorization", format!("token {}", self.token))
                    .query(&[
                        ("state", gitea_state),
                        ("type", "issues"),
                        ("limit", "50"),
                        ("page", &page.to_string()),
                    ])
                    .send()
                    .await
                    .map_err(|e| SymphonyError::Tracker {
                        kind: "gitea".into(),
                        message: format!("request failed: {e}"),
                    })?;

                let status = resp.status();
                if !status.is_success() {
                    let text = resp.text().await.unwrap_or_default();
                    return Err(SymphonyError::Tracker {
                        kind: "gitea".into(),
                        message: format!("HTTP {status}: {text}"),
                    });
                }

                let issues: Vec<GiteaIssue> =
                    resp.json().await.map_err(|e| SymphonyError::Tracker {
                        kind: "gitea".into(),
                        message: format!("response parse error: {e}"),
                    })?;

                if issues.is_empty() {
                    break;
                }

                for gi in &issues {
                    let issue = self.normalise_issue(gi);
                    let matches = states
                        .iter()
                        .any(|s| s.eq_ignore_ascii_case(&issue.state));
                    if matches {
                        all_issues.push(issue);
                    }
                }

                if issues.len() < 50 {
                    break;
                }
                page += 1;
            }
        }

        debug!(
            count = all_issues.len(),
            "fetched issues by states from Gitea"
        );
        Ok(all_issues)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_tracker() -> GiteaTracker {
        GiteaTracker {
            client: Client::new(),
            base_url: "https://git.example.com".into(),
            token: "test-token".into(),
            owner: "testowner".into(),
            repo: "testrepo".into(),
            active_states: vec!["Todo".into(), "In Progress".into()],
            terminal_states: vec!["Done".into(), "Closed".into()],
        }
    }

    #[test]
    fn normalise_open_issue() {
        let tracker = make_tracker();
        let gi = GiteaIssue {
            id: 42,
            number: 7,
            title: "Fix something".into(),
            body: Some("Detailed description".into()),
            state: "open".into(),
            html_url: Some("https://git.example.com/testowner/testrepo/issues/7".into()),
            created_at: Some("2025-01-15T10:00:00Z".into()),
            updated_at: Some("2025-01-16T12:00:00Z".into()),
            labels: Some(vec![GiteaLabel {
                name: "priority/P2-medium".into(),
            }]),
            ref_field: None,
        };

        let issue = tracker.normalise_issue(&gi);
        assert_eq!(issue.id, "42");
        assert_eq!(issue.identifier, "testowner/testrepo#7");
        assert_eq!(issue.title, "Fix something");
        assert_eq!(issue.state, "Todo"); // open maps to first active state
        assert_eq!(issue.priority, Some(2));
        assert!(issue.created_at.is_some());
    }

    #[test]
    fn normalise_closed_issue() {
        let tracker = make_tracker();
        let gi = GiteaIssue {
            id: 43,
            number: 8,
            title: "Done issue".into(),
            body: None,
            state: "closed".into(),
            html_url: None,
            created_at: None,
            updated_at: None,
            labels: None,
            ref_field: None,
        };

        let issue = tracker.normalise_issue(&gi);
        assert_eq!(issue.state, "Done"); // closed maps to first terminal state
    }

    #[test]
    fn derive_state_from_label() {
        let tracker = make_tracker();
        let labels = vec!["status/in progress".into(), "bug".into()];
        assert_eq!(
            tracker.derive_state(&labels, "open"),
            "In Progress"
        );
    }

    #[test]
    fn derive_state_fallback_to_gitea() {
        let tracker = make_tracker();
        let labels = vec!["bug".into()];
        assert_eq!(tracker.derive_state(&labels, "open"), "Todo");
        assert_eq!(tracker.derive_state(&labels, "closed"), "Done");
    }

    #[test]
    fn extract_priority_from_labels() {
        let tracker = make_tracker();
        assert_eq!(
            tracker.extract_priority(&["priority/p1-high".into()]),
            Some(1)
        );
        assert_eq!(
            tracker.extract_priority(&["priority/p3-low".into()]),
            Some(3)
        );
        assert_eq!(tracker.extract_priority(&["bug".into()]), None);
    }

    #[test]
    fn issues_url_format() {
        let tracker = make_tracker();
        assert_eq!(
            tracker.issues_url(),
            "https://git.example.com/api/v1/repos/testowner/testrepo/issues"
        );
    }
}
