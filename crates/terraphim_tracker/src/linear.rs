//! Linear GraphQL issue tracker client.
//!
//! Fetches issues from Linear using the GraphQL API, normalising them
//! to the common [`Issue`] model.

use crate::{BlockerRef, Issue, IssueTracker, Result, TrackerError};
use async_trait::async_trait;
use reqwest::Client;
use tracing::debug;

/// Configuration for Linear tracker.
#[derive(Debug, Clone)]
pub struct LinearConfig {
    /// GraphQL endpoint URL (typically https://api.linear.app/graphql).
    pub endpoint: String,
    /// API key for authentication (LINEAR_API_KEY).
    pub api_key: String,
    /// Project slug identifier (e.g., "ABC" for project ABC).
    pub project_slug: String,
    /// Active states that trigger agent dispatch.
    pub active_states: Vec<String>,
    /// Terminal states that trigger workspace cleanup.
    pub terminal_states: Vec<String>,
}

/// Linear GraphQL client.
pub struct LinearTracker {
    client: Client,
    /// Configuration for the tracker (exposed for testing).
    #[doc(hidden)]
    pub config: LinearConfig,
}

impl LinearTracker {
    /// Create a new Linear tracker from configuration.
    pub fn new(config: LinearConfig) -> Result<Self> {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| TrackerError::Api {
                message: format!("Failed to create HTTP client: {e}"),
            })?;

        Ok(Self { client, config })
    }

    /// Execute a GraphQL query against the Linear API.
    async fn graphql(
        &self,
        query: &str,
        variables: serde_json::Value,
    ) -> Result<serde_json::Value> {
        let body = serde_json::json!({
            "query": query,
            "variables": variables,
        });

        let resp = self
            .client
            .post(&self.config.endpoint)
            .header("Authorization", format!("Bearer {}", &self.config.api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| TrackerError::Api {
                message: format!("request failed: {e}"),
            })?;

        let status = resp.status();
        if !status.is_success() {
            let text = resp.text().await.unwrap_or_default();
            return Err(TrackerError::Api {
                message: format!("HTTP {status}: {text}"),
            });
        }

        let json: serde_json::Value = resp.json().await.map_err(|e| TrackerError::Api {
            message: format!("response parse error: {e}"),
        })?;

        // Check for GraphQL errors
        if let Some(errors) = json.get("errors") {
            if let Some(arr) = errors.as_array() {
                if !arr.is_empty() {
                    let messages: Vec<String> = arr
                        .iter()
                        .filter_map(|e| e.get("message").and_then(|m| m.as_str()))
                        .map(String::from)
                        .collect();
                    return Err(TrackerError::GraphQLError {
                        message: messages.join("; "),
                    });
                }
            }
        }

        json.get("data").cloned().ok_or_else(|| TrackerError::Api {
            message: "missing data field in response".into(),
        })
    }

    /// Build the state filter expression for a GraphQL query.
    fn state_filter(&self, states: &[String]) -> String {
        let quoted: Vec<String> = states.iter().map(|s| format!("\"{}\"", s)).collect();
        format!("[{}]", quoted.join(", "))
    }

    /// Parse a Linear issue node into the common Issue model.
    fn parse_issue(node: &serde_json::Value) -> Option<Issue> {
        let id = node.get("id")?.as_str()?.to_string();
        let identifier = node.get("identifier")?.as_str()?.to_string();
        let title = node.get("title")?.as_str()?.to_string();
        let description = node
            .get("description")
            .and_then(|d| d.as_str())
            .map(String::from);
        let priority = node
            .get("priority")
            .and_then(|p| p.as_i64())
            .map(|p| p as i32);
        let state = node
            .pointer("/state/name")
            .and_then(|s| s.as_str())
            .unwrap_or("Unknown")
            .to_string();
        let branch_name = node
            .get("branchName")
            .and_then(|b| b.as_str())
            .map(String::from);
        let url = node.get("url").and_then(|u| u.as_str()).map(String::from);

        let labels: Vec<String> = node
            .pointer("/labels/nodes")
            .and_then(|l| l.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|l| l.get("name").and_then(|n| n.as_str()))
                    .map(|s| s.to_lowercase())
                    .collect()
            })
            .unwrap_or_default();

        // Parse blockers from inverse relations
        let blocked_by: Vec<BlockerRef> = node
            .pointer("/relations/nodes")
            .and_then(|r| r.as_array())
            .map(|arr| {
                arr.iter()
                    .filter(|r| {
                        r.get("type")
                            .and_then(|t| t.as_str())
                            .is_some_and(|t| t == "blocks")
                    })
                    .filter_map(|r| {
                        let related = r.get("relatedIssue")?;
                        Some(BlockerRef {
                            id: related.get("id").and_then(|i| i.as_str()).map(String::from),
                            identifier: related
                                .get("identifier")
                                .and_then(|i| i.as_str())
                                .map(String::from),
                            state: related
                                .pointer("/state/name")
                                .and_then(|s| s.as_str())
                                .map(String::from),
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();

        let created_at = node
            .get("createdAt")
            .and_then(|c| c.as_str())
            .and_then(|s| s.parse::<jiff::Timestamp>().ok())
            .map(|ts| ts.to_zoned(jiff::tz::TimeZone::UTC));
        let updated_at = node
            .get("updatedAt")
            .and_then(|u| u.as_str())
            .and_then(|s| s.parse::<jiff::Timestamp>().ok())
            .map(|ts| ts.to_zoned(jiff::tz::TimeZone::UTC));

        Some(Issue {
            id,
            identifier,
            title,
            description,
            priority,
            state,
            branch_name,
            url,
            labels,
            blocked_by,
            pagerank_score: None,
            created_at,
            updated_at,
        })
    }

    /// Fetch issues with pagination.
    async fn fetch_paginated(
        &self,
        query: &str,
        variables: serde_json::Value,
        data_path: &str,
    ) -> Result<Vec<Issue>> {
        let mut all_issues = Vec::new();
        let mut cursor: Option<String> = None;

        loop {
            let mut vars = variables.clone();
            if let Some(ref c) = cursor {
                vars["after"] = serde_json::Value::String(c.clone());
            }

            let data = self.graphql(query, vars).await?;

            let connection = data_path
                .split('.')
                .try_fold(&data, |acc, key| acc.get(key))
                .ok_or_else(|| TrackerError::Api {
                    message: format!("missing path '{}' in response", data_path),
                })?;

            // Try "nodes" first (real Linear API), fallback to "edges" (twin)
            let items: Vec<&serde_json::Value> = connection
                .get("nodes")
                .and_then(|n| n.as_array())
                .map(|arr| arr.iter().collect())
                .or_else(|| {
                    connection
                        .get("edges")
                        .and_then(|e| e.as_array())
                        .map(|arr| arr.iter().filter_map(|edge| edge.get("node")).collect())
                })
                .ok_or_else(|| TrackerError::Api {
                    message: "missing nodes or edges array in response".into(),
                })?;

            for item in items {
                if let Some(issue) = Self::parse_issue(item) {
                    all_issues.push(issue);
                }
            }

            let has_next = connection
                .pointer("/pageInfo/hasNextPage")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);

            if !has_next {
                break;
            }

            cursor = connection
                .pointer("/pageInfo/endCursor")
                .and_then(|v| v.as_str())
                .map(String::from);

            if cursor.is_none() {
                return Err(TrackerError::Api {
                    message: "missing endCursor for pagination".into(),
                });
            }
        }

        debug!(count = all_issues.len(), "fetched issues from Linear");
        Ok(all_issues)
    }
}

#[async_trait]
impl IssueTracker for LinearTracker {
    async fn fetch_candidate_issues(&self) -> Result<Vec<Issue>> {
        let state_names = self.state_filter(&self.config.active_states);

        let query = format!(
            r#"
            query($projectSlug: String!, $after: String) {{
                issues(
                    filter: {{
                        project: {{ slugId: {{ eq: $projectSlug }} }}
                        state: {{ name: {{ in: {state_names} }} }}
                    }}
                    first: 50
                    after: $after
                    orderBy: "updatedAt"
                ) {{
                    nodes {{
                        id
                        identifier
                        title
                        description
                        priority
                        branchName
                        url
                        createdAt
                        updatedAt
                        state {{ name }}
                        labels(first: 20) {{ nodes {{ name }} }}
                        relations(first: 20) {{
                            nodes {{
                                type
                                relatedIssue {{
                                    id
                                    identifier
                                    state {{ name }}
                                }}
                            }}
                        }}
                    }}
                    pageInfo {{
                        hasNextPage
                        endCursor
                    }}
                }}
            }}
            "#
        );

        let variables = serde_json::json!({
            "projectSlug": self.config.project_slug,
        });

        self.fetch_paginated(&query, variables, "issues").await
    }

    async fn fetch_issue_states_by_ids(&self, ids: &[String]) -> Result<Vec<Issue>> {
        if ids.is_empty() {
            return Ok(vec![]);
        }

        // Linear's filter supports `id: { in: [...] }`
        let query = r#"
            query($ids: [ID!]!) {
                issues(filter: { id: { in: $ids } }, first: 50) {
                    nodes {
                        id
                        identifier
                        title
                        state { name }
                        createdAt
                    }
                    pageInfo {
                        hasNextPage
                        endCursor
                    }
                }
            }
        "#;

        let variables = serde_json::json!({
            "ids": ids,
        });

        self.fetch_paginated(query, variables, "issues").await
    }

    async fn fetch_issues_by_states(&self, states: &[String]) -> Result<Vec<Issue>> {
        if states.is_empty() {
            return Ok(vec![]);
        }

        let state_names = self.state_filter(states);

        let query = format!(
            r#"
            query($projectSlug: String!, $after: String) {{
                issues(
                    filter: {{
                        project: {{ slugId: {{ eq: $projectSlug }} }}
                        state: {{ name: {{ in: {state_names} }} }}
                    }}
                    first: 50
                    after: $after
                ) {{
                    nodes {{
                        id
                        identifier
                        title
                        state {{ name }}
                        createdAt
                    }}
                    pageInfo {{
                        hasNextPage
                        endCursor
                    }}
                }}
            }}
            "#
        );

        let variables = serde_json::json!({
            "projectSlug": self.config.project_slug,
        });

        self.fetch_paginated(&query, variables, "issues").await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> LinearConfig {
        LinearConfig {
            endpoint: "https://api.linear.app/graphql".into(),
            api_key: "test-key".into(),
            project_slug: "TEST".into(),
            active_states: vec!["Todo".into(), "In Progress".into()],
            terminal_states: vec!["Done".into(), "Closed".into()],
        }
    }

    #[test]
    fn linear_tracker_new_succeeds_with_valid_config() {
        let config = test_config();
        let tracker = LinearTracker::new(config);
        assert!(tracker.is_ok());
    }

    #[test]
    fn parse_linear_issue_node() {
        let node = serde_json::json!({
            "id": "abc123",
            "identifier": "PRJ-42",
            "title": "Fix the bug",
            "description": "Something is broken",
            "priority": 2,
            "branchName": "fix/prj-42",
            "url": "https://linear.app/prj/issue/PRJ-42",
            "createdAt": "2025-01-15T10:00:00Z",
            "updatedAt": "2025-01-16T12:00:00Z",
            "state": { "name": "In Progress" },
            "labels": { "nodes": [{ "name": "Bug" }, { "name": "P1" }] },
            "relations": { "nodes": [
                {
                    "type": "blocks",
                    "relatedIssue": {
                        "id": "def456",
                        "identifier": "PRJ-10",
                        "state": { "name": "Done" }
                    }
                }
            ]}
        });

        let issue = LinearTracker::parse_issue(&node).unwrap();
        assert_eq!(issue.id, "abc123");
        assert_eq!(issue.identifier, "PRJ-42");
        assert_eq!(issue.title, "Fix the bug");
        assert_eq!(issue.state, "In Progress");
        assert_eq!(issue.priority, Some(2));
        assert_eq!(issue.labels, vec!["bug", "p1"]); // lowercase
        assert_eq!(issue.blocked_by.len(), 1);
        assert_eq!(issue.blocked_by[0].identifier.as_deref(), Some("PRJ-10"));
        assert_eq!(issue.blocked_by[0].state.as_deref(), Some("Done"));
        assert!(issue.created_at.is_some());
        assert!(issue.updated_at.is_some());
    }

    #[test]
    fn parse_minimal_linear_node() {
        let node = serde_json::json!({
            "id": "min1",
            "identifier": "PRJ-1",
            "title": "Minimal",
            "state": { "name": "Todo" },
        });

        let issue = LinearTracker::parse_issue(&node).unwrap();
        assert_eq!(issue.id, "min1");
        assert!(issue.description.is_none());
        assert!(issue.priority.is_none());
        assert!(issue.labels.is_empty());
        assert!(issue.blocked_by.is_empty());
    }

    #[test]
    fn parse_node_missing_required_fields() {
        let node = serde_json::json!({
            "id": "x",
        });
        assert!(LinearTracker::parse_issue(&node).is_none());
    }

    #[test]
    fn state_filter_builds_correctly() {
        let config = test_config();
        let tracker = LinearTracker::new(config).unwrap();
        let states = vec!["Todo".to_string(), "In Progress".to_string()];
        let filter = tracker.state_filter(&states);
        assert_eq!(filter, "[\"Todo\", \"In Progress\"]");
    }
}
