//! Haystack integration for Atlassian products (Confluence, Jira).
//!
//! Implements [`HaystackProvider`] over the Confluence REST API, enabling
//! full-text search of Confluence spaces as a Terraphim haystack source.
use anyhow::Result;
use haystack_core::HaystackProvider;
use terraphim_types::{Document, SearchQuery};

pub mod confluence;
pub mod jira;

/// HTTP client for the Confluence REST API.
pub struct ConfluenceClient {
    /// Base URL of the Confluence instance (e.g. `https://example.atlassian.net/wiki`).
    pub base_url: String,
    /// Atlassian account username (email address).
    pub username: String,
    /// Atlassian API token for authentication.
    pub token: String,
}

/// HTTP client for the Jira REST API.
pub struct JiraClient {
    /// Base URL of the Jira instance (e.g. `https://example.atlassian.net`).
    pub base_url: String,
    /// Atlassian account username (email address).
    pub username: String,
    /// Atlassian API token for authentication.
    pub token: String,
}

impl ConfluenceClient {
    /// Creates a new Confluence client authenticated with the given credentials.
    pub fn new(base_url: String, username: String, token: String) -> Self {
        Self {
            base_url,
            username,
            token,
        }
    }
}

impl JiraClient {
    /// Creates a new Jira client authenticated with the given credentials.
    pub fn new(base_url: String, username: String, token: String) -> Self {
        Self {
            base_url,
            username,
            token,
        }
    }
}

impl HaystackProvider for ConfluenceClient {
    type Error = anyhow::Error;

    async fn search(&self, query: &SearchQuery) -> Result<Vec<Document>, Self::Error> {
        let search_term = &query.search_term.to_string();
        let cql_query = format!("text ~ \"{}\"", search_term);

        let results =
            confluence::search(&self.base_url, &self.username, &self.token, &cql_query, 10).await?;

        let documents: Vec<Document> = results
            .into_iter()
            .map(|result| Document {
                id: result.id,
                url: result.links.web_ui,
                title: result.title,
                body: result.excerpt.unwrap_or_default(),
                description: Some(format!("Page from {} space", result.space.name)),
                tags: Some(vec![result.space.key, result.content_type]),
                ..Default::default()
            })
            .collect();

        Ok(documents)
    }
}

impl HaystackProvider for JiraClient {
    type Error = anyhow::Error;

    async fn search(&self, query: &SearchQuery) -> Result<Vec<Document>, Self::Error> {
        let search_term = &query.search_term.to_string();
        let jql_query = format!(
            "text ~ \"{}\" OR summary ~ \"{}\" OR description ~ \"{}\"",
            search_term, search_term, search_term
        );

        let issues = jira::search(
            &self.base_url,
            &self.username,
            &self.token,
            &jql_query,
            "key,summary,description,status,priority,issuetype,components",
            10,
        )
        .await?;

        let documents: Vec<Document> = issues
            .into_iter()
            .map(|issue| {
                let body = if let Some(desc) = &issue.fields.description {
                    format!(
                        "# {}\n\n## Description\n{}\n\n**Status:** {} | **Type:** {} | **Priority:** {}",
                        issue.fields.summary,
                        desc,
                        issue.fields.status.name,
                        issue.fields.issue_type.name,
                        issue.fields.priority.as_ref().map_or("None", |p| &p.name)
                    )
                } else {
                    format!(
                        "# {}\n\n**Status:** {} | **Type:** {} | **Priority:** {}",
                        issue.fields.summary,
                        issue.fields.status.name,
                        issue.fields.issue_type.name,
                        issue.fields.priority.as_ref().map_or("None", |p| &p.name)
                    )
                };

                let mut tags = vec![
                    issue.fields.issue_type.name.clone(),
                    issue.fields.status.name.clone(),
                ];

                if let Some(priority) = &issue.fields.priority {
                    tags.push(priority.name.clone());
                }

                if let Some(components) = &issue.fields.components {
                    for component in components {
                        tags.push(component.name.clone());
                    }
                }

                Document {
                    id: issue.key.clone(),
                    url: format!("{}/browse/{}", self.base_url.trim_end_matches('/'), issue.key),
                    title: format!("[{}] {}", issue.key, issue.fields.summary),
                    body,
                    description: Some(issue.fields.summary.clone()),
                    tags: Some(tags),
                    ..Default::default()
                }
            })
            .collect();

        Ok(documents)
    }
}

/// Legacy combined Atlassian client kept for backward compatibility.
///
/// Prefer [`ConfluenceClient`] or [`JiraClient`] for new integrations.
pub struct AtlassianClient;

impl HaystackProvider for AtlassianClient {
    type Error = anyhow::Error;

    async fn search(&self, _query: &SearchQuery) -> Result<Vec<Document>, Self::Error> {
        // Legacy placeholder - use ConfluenceClient or JiraClient instead
        Ok(vec![])
    }
}
