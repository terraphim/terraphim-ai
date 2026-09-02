//! Linear HTTP client + GraphQL query for the haystack integration.
//!
//! Single round-trip with the full selection set: title, description, state,
//! labels, and top-3 comments. Measured at 265 ms for 25 issues on the live
//! ODITECH workspace. See Spike 2 in `.docs/spike-report-phase-1.5-linear.md`.
//!
//! No per-issue fan-out, no cache.

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::error::LinearError;

/// The subset of Linear issue data we care about. Subset of
/// `terraphim-linear-cli::backend::IssueDetail` — defined locally so the
/// haystack crate doesn't depend on the CLI binary (which is in a sibling
/// repo) or repeat the full CLI Issue shape.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinearIssue {
    pub id: String,
    pub identifier: String,
    pub title: String,
    #[serde(default)]
    pub description: Option<String>,
    pub url: String,
    pub priority: i32,
    #[serde(default)]
    pub state_name: Option<String>,
    #[serde(default)]
    pub state_type: Option<String>,
    #[serde(default)]
    pub assignee_name: Option<String>,
    #[serde(default)]
    pub team_key: Option<String>,
    #[serde(default)]
    pub team_name: Option<String>,
    #[serde(default)]
    pub labels: Vec<String>,
    #[serde(default)]
    pub comments: Vec<LinearComment>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinearComment {
    pub id: String,
    pub body: String,
    #[serde(default)]
    pub user_name: Option<String>,
    pub created_at: String,
}

/// Linear HTTP client. Cheap to clone (`reqwest::Client` is internally `Arc`).
#[derive(Debug, Clone)]
pub struct LinearClient {
    http: reqwest::Client,
    api_key: String,
    endpoint: String,
}

impl LinearClient {
    /// Construct a client pointing at the public Linear GraphQL endpoint.
    pub fn new(api_key: impl Into<String>) -> anyhow::Result<Self> {
        Self::with_endpoint(api_key, "https://api.linear.app/graphql")
    }

    /// Construct a client with a custom endpoint (used by tests).
    pub fn with_endpoint(
        api_key: impl Into<String>,
        endpoint: impl Into<String>,
    ) -> anyhow::Result<Self> {
        let http = reqwest::Client::builder()
            .user_agent(concat!("linear_haystack/", env!("CARGO_PKG_VERSION")))
            .build()
            .map_err(|e| LinearError::Http {
                status: 0,
                body: e.to_string(),
            })?;
        Ok(Self {
            http,
            api_key: api_key.into(),
            endpoint: endpoint.into(),
        })
    }

    /// Resolve the API key from `terraphim-linear-auth` (3 sources: file →
    /// env → `op read`) and construct a client.
    #[cfg(feature = "linear")]
    pub fn from_auth() -> anyhow::Result<Self> {
        let key = terraphim_linear_auth::load_key()?;
        Self::new(key.expose())
    }

    /// Fetch issues matching `searchable_content` (Linear's title+description
    /// filter, verified via API probe 2026-07-29). One round-trip with full
    /// selection set. No per-issue follow-up calls.
    ///
    /// Returns up to `limit` issues (default 25, hard-capped at 25 to keep
    /// the response under Linear's 216 KB / 4 %-of-complexity-budget
    /// threshold for this query).
    pub async fn search(
        &self,
        searchable_content: &str,
        limit: u32,
    ) -> Result<Vec<LinearIssue>, LinearError> {
        let first = limit.clamp(1, 25);
        let query = r#"
            query SearchIssues($first: Int!, $filter: IssueFilter!) {
              issues(filter: $filter, first: $first) {
                nodes {
                  id identifier title description url priority
                  state { name type }
                  assignee { name }
                  team { key name }
                  labels(first: 10) { nodes { name } }
                  comments(first: 3) {
                    nodes { id body createdAt user { name } }
                  }
                }
              }
            }
        "#;
        let variables = json!({
            "first": first,
            "filter": {
                "searchableContent": { "contains": searchable_content }
            }
        });
        let body = json!({ "query": query, "variables": variables });

        let resp = self
            .http
            .post(&self.endpoint)
            .header("Authorization", &self.api_key)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;

        let status = resp.status();
        if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
            let retry_after_secs = resp
                .headers()
                .get("retry-after")
                .and_then(|v| v.to_str().ok())
                .and_then(|s| s.parse::<u64>().ok())
                .unwrap_or(60);
            return Err(LinearError::RateLimited { retry_after_secs });
        }
        if !status.is_success() {
            let text = resp.text().await.unwrap_or_default();
            return Err(LinearError::Http {
                status: status.as_u16(),
                body: text,
            });
        }

        let data: Value = resp.json().await?;
        if let Some(errors) = data.get("errors").and_then(|v| v.as_array())
            && !errors.is_empty()
        {
            let msg = errors
                .iter()
                .filter_map(|e| e.get("message").and_then(|m| m.as_str()))
                .collect::<Vec<_>>()
                .join("; ");
            return Err(LinearError::GraphQL(msg));
        }
        let nodes = data
            .get("data")
            .and_then(|d| d.get("issues"))
            .and_then(|i| i.get("nodes"))
            .and_then(|n| n.as_array())
            .cloned()
            .unwrap_or_default();
        let issues: Vec<LinearIssue> = nodes.into_iter().map(LinearIssue::from_value).collect();
        Ok(issues)
    }
}

impl LinearIssue {
    /// Parse a `serde_json::Value` (one node from the GraphQL response) into
    /// a `LinearIssue`. Defensive: missing fields become `None` / empty.
    fn from_value(v: Value) -> Self {
        let labels = v
            .get("labels")
            .and_then(|l| l.get("nodes"))
            .and_then(|n| n.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|node| node.get("name").and_then(|n| n.as_str()))
                    .map(String::from)
                    .collect()
            })
            .unwrap_or_default();
        let comments = v
            .get("comments")
            .and_then(|c| c.get("nodes"))
            .and_then(|n| n.as_array())
            .map(|arr| {
                arr.iter()
                    .map(|c| LinearComment {
                        id: c
                            .get("id")
                            .and_then(|x| x.as_str())
                            .unwrap_or("")
                            .to_string(),
                        body: c
                            .get("body")
                            .and_then(|x| x.as_str())
                            .unwrap_or("")
                            .to_string(),
                        user_name: c
                            .get("user")
                            .and_then(|u| u.get("name"))
                            .and_then(|n| n.as_str())
                            .map(String::from),
                        created_at: c
                            .get("createdAt")
                            .and_then(|x| x.as_str())
                            .unwrap_or("")
                            .to_string(),
                    })
                    .collect()
            })
            .unwrap_or_default();
        Self {
            id: v.get("id").and_then(|x| x.as_str()).unwrap_or("").into(),
            identifier: v
                .get("identifier")
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .into(),
            title: v.get("title").and_then(|x| x.as_str()).unwrap_or("").into(),
            description: v
                .get("description")
                .and_then(|x| x.as_str())
                .map(String::from),
            url: v.get("url").and_then(|x| x.as_str()).unwrap_or("").into(),
            priority: v.get("priority").and_then(|x| x.as_i64()).unwrap_or(0) as i32,
            state_name: v
                .get("state")
                .and_then(|s| s.get("name"))
                .and_then(|n| n.as_str())
                .map(String::from),
            state_type: v
                .get("state")
                .and_then(|s| s.get("type"))
                .and_then(|n| n.as_str())
                .map(String::from),
            assignee_name: v
                .get("assignee")
                .and_then(|a| a.get("name"))
                .and_then(|n| n.as_str())
                .map(String::from),
            team_key: v
                .get("team")
                .and_then(|t| t.get("key"))
                .and_then(|k| k.as_str())
                .map(String::from),
            team_name: v
                .get("team")
                .and_then(|t| t.get("name"))
                .and_then(|n| n.as_str())
                .map(String::from),
            labels,
            comments,
        }
    }
}
