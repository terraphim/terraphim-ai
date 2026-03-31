//! TuiBackend: Unified interface for local (offline) and remote (server) TUI operations.
//!
//! This module provides an enum-based abstraction that allows the TUI to work with either
//! a local TuiService (offline, no server required) or a remote ApiClient (server-backed).
//! Both variants are always compiled; the choice is made at runtime based on CLI flags.

use anyhow::Result;
use terraphim_config::Config;
use terraphim_types::{Document, RoleName, SearchQuery};

use crate::client::ApiClient;
use crate::service::TuiService;

/// Backend for TUI operations, supporting both local (offline) and remote (server) modes.
#[derive(Clone)]
pub enum TuiBackend {
    /// Local/offline backend using TuiService directly.
    Local(TuiService),
    /// Remote/server backend using HTTP API client.
    Remote(ApiClient),
}

impl std::fmt::Debug for TuiBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Local(_) => f.debug_tuple("Local").field(&"TuiService").finish(),
            Self::Remote(api) => f.debug_tuple("Remote").field(api).finish(),
        }
    }
}

impl TuiBackend {
    /// Execute a search query and return matching documents.
    ///
    /// # Arguments
    /// * `query` - The search query containing search term, role, limits, etc.
    ///
    /// # Returns
    /// A vector of Document objects matching the query.
    pub async fn search(&self, query: &SearchQuery) -> Result<Vec<Document>> {
        match self {
            Self::Local(svc) => {
                let results = svc.search_with_query(query).await?;
                Ok(results)
            }
            Self::Remote(api) => {
                let resp = api.search(query).await?;
                Ok(resp.results)
            }
        }
    }

    /// Get the current configuration.
    ///
    /// # Returns
    /// The current terraphim_config::Config.
    pub async fn get_config(&self) -> Result<Config> {
        match self {
            Self::Local(svc) => {
                let config = svc.get_config().await;
                Ok(config)
            }
            Self::Remote(api) => {
                let resp = api.get_config().await?;
                Ok(resp.config)
            }
        }
    }

    /// Get the top terms from a role's knowledge graph.
    ///
    /// # Arguments
    /// * `role` - The role name to get terms for.
    ///
    /// # Returns
    /// A vector of term strings from the rolegraph.
    pub async fn get_rolegraph_terms(&self, role: &str) -> Result<Vec<String>> {
        match self {
            Self::Local(svc) => {
                let role_name = RoleName::new(role);
                let terms = svc.get_role_graph_top_k(&role_name, 50).await?;
                Ok(terms)
            }
            Self::Remote(api) => {
                let resp = api.rolegraph(Some(role)).await?;
                let labels: Vec<String> = resp.nodes.into_iter().map(|n| n.label).collect();
                Ok(labels)
            }
        }
    }

    /// Get autocomplete suggestions for a partial query.
    ///
    /// # Arguments
    /// * `role` - The role context for autocomplete.
    /// * `query` - The partial query string to complete.
    ///
    /// # Returns
    /// A vector of suggestion strings.
    pub async fn autocomplete(&self, role: &str, query: &str) -> Result<Vec<String>> {
        match self {
            Self::Local(svc) => {
                let role_name = RoleName::new(role);
                let results = svc.autocomplete(&role_name, query, Some(5)).await?;
                let suggestions: Vec<String> = results.into_iter().map(|r| r.term).collect();
                Ok(suggestions)
            }
            Self::Remote(api) => {
                let resp = api.get_autocomplete(role, query).await?;
                let suggestions: Vec<String> =
                    resp.suggestions.into_iter().map(|s| s.text).collect();
                Ok(suggestions)
            }
        }
    }

    /// Summarize a document using the configured AI/LLM.
    ///
    /// # Arguments
    /// * `document` - The document to summarize.
    /// * `role` - Optional role context for the summary.
    ///
    /// # Returns
    /// An optional summary string (None if summarization is unavailable).
    pub async fn summarize(
        &self,
        document: &Document,
        role: Option<&str>,
    ) -> Result<Option<String>> {
        match self {
            Self::Local(svc) => {
                let role_name = RoleName::new(role.unwrap_or("Terraphim Engineer"));
                let summary = svc.summarize(&role_name, &document.body).await?;
                Ok(Some(summary))
            }
            Self::Remote(api) => {
                let resp = api.summarize_document(document, role).await?;
                Ok(resp.summary)
            }
        }
    }

    /// Switch to a different role and return the updated config.
    ///
    /// # Arguments
    /// * `role` - The role name to switch to.
    ///
    /// # Returns
    /// The updated configuration after switching roles.
    #[allow(dead_code)]
    pub async fn switch_role(&self, role: &str) -> Result<Config> {
        match self {
            Self::Local(svc) => {
                let role_name = RoleName::new(role);
                let config = svc.update_selected_role(role_name).await?;
                Ok(config)
            }
            Self::Remote(api) => {
                let resp = api.update_selected_role(role).await?;
                Ok(resp.config)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test that TuiBackend::Local variant can be constructed.
    /// Full method testing requires a running service or mocked dependencies.
    #[tokio::test]
    async fn test_tuibackend_local_variant_exists() {
        // This test verifies the enum structure compiles correctly.
        // Actual TuiService::new() requires filesystem/config access.
        // We just verify the type signatures are correct.
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<TuiBackend>();
    }

    /// Test that TuiBackend::Remote variant can be constructed.
    #[test]
    fn test_tuibackend_remote_variant_exists() {
        let api = ApiClient::new("http://localhost:8000".to_string());
        let backend = TuiBackend::Remote(api);

        // Verify we can match on the variant
        match backend {
            TuiBackend::Remote(_) => (), // Expected
            TuiBackend::Local(_) => panic!("Expected Remote variant"),
        }
    }

    /// Test that the backend enum is Clone.
    #[test]
    fn test_tuibackend_is_clone() {
        let api = ApiClient::new("http://localhost:8000".to_string());
        let backend = TuiBackend::Remote(api);
        let _cloned = backend.clone();
        // If this compiles, Clone is implemented correctly.
    }

    /// Test that the backend enum is Debug.
    #[test]
    fn test_tuibackend_is_debug() {
        let api = ApiClient::new("http://localhost:8000".to_string());
        let backend = TuiBackend::Remote(api);
        let _debug_str = format!("{:?}", backend);
        // If this compiles, Debug is implemented correctly.
    }
}
