//! TuiBackend: Unified interface for local (offline) and remote (server) TUI operations.
//!
//! This module provides an enum-based abstraction that allows the TUI to work with either
//! a local TuiService (offline, no server required) or a remote ApiClient (server-backed).
//!
//! When the `server` feature is disabled, only the `Local` variant is available.

use anyhow::Result;
use terraphim_config::Config;
use terraphim_types::{Document, SearchQuery};

use crate::service::TuiService;

#[cfg(feature = "server")]
use crate::client::ApiClient;

/// Backend for TUI operations, supporting both local (offline) and remote (server) modes.
#[derive(Clone)]
pub enum TuiBackend {
    /// Local/offline backend using TuiService directly.
    #[allow(dead_code)]
    Local(TuiService),
    /// Remote/server backend using HTTP API client.
    #[cfg(feature = "server")]
    Remote(ApiClient),
}

impl std::fmt::Debug for TuiBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Local(_) => f.debug_tuple("Local").field(&"TuiService").finish(),
            #[cfg(feature = "server")]
            Self::Remote(api) => f.debug_tuple("Remote").field(api).finish(),
        }
    }
}

impl TuiBackend {
    /// Execute a search query and return matching documents.
    pub async fn search(&self, query: &SearchQuery) -> Result<Vec<Document>> {
        match self {
            Self::Local(svc) => {
                let results = svc.search_with_query(query).await?;
                Ok(results)
            }
            #[cfg(feature = "server")]
            Self::Remote(api) => {
                let resp = api.search(query).await?;
                Ok(resp.results)
            }
        }
    }

    /// Get the current configuration.
    pub async fn get_config(&self) -> Result<Config> {
        match self {
            Self::Local(svc) => {
                let config = svc.get_config().await;
                Ok(config)
            }
            #[cfg(feature = "server")]
            Self::Remote(api) => {
                let resp = api.get_config().await?;
                Ok(resp.config)
            }
        }
    }

    /// Get the top terms from a role's knowledge graph.
    pub async fn get_rolegraph_terms(&self, role: &str) -> Result<Vec<String>> {
        use terraphim_types::RoleName;
        match self {
            Self::Local(svc) => {
                let role_name = RoleName::new(role);
                let terms = svc.get_role_graph_top_k(&role_name, 50).await?;
                Ok(terms)
            }
            #[cfg(feature = "server")]
            Self::Remote(api) => {
                let resp = api.rolegraph(Some(role)).await?;
                let labels: Vec<String> = resp.nodes.into_iter().map(|n| n.label).collect();
                Ok(labels)
            }
        }
    }

    /// Get autocomplete suggestions for a partial query.
    pub async fn autocomplete(&self, role: &str, query: &str) -> Result<Vec<String>> {
        use terraphim_types::RoleName;
        match self {
            Self::Local(svc) => {
                let role_name = RoleName::new(role);
                let results = svc.autocomplete(&role_name, query, Some(5)).await?;
                let suggestions: Vec<String> = results.into_iter().map(|r| r.term).collect();
                Ok(suggestions)
            }
            #[cfg(feature = "server")]
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
    /// Returns None if summarization is unavailable (llm feature disabled).
    #[cfg(feature = "llm")]
    pub async fn summarize(
        &self,
        document: &Document,
        role: Option<&str>,
    ) -> Result<Option<String>> {
        use terraphim_types::RoleName;
        match self {
            Self::Local(svc) => {
                let role_name = RoleName::new(role.unwrap_or("Terraphim Engineer"));
                let summary = svc.summarize(&role_name, &document.body).await?;
                Ok(Some(summary))
            }
            #[cfg(feature = "server")]
            Self::Remote(api) => {
                let resp = api.summarize_document(document, role).await?;
                Ok(resp.summary)
            }
        }
    }

    /// Switch to a different role and return the updated config.
    #[allow(dead_code)]
    pub async fn switch_role(&self, role: &str) -> Result<Config> {
        use terraphim_types::RoleName;
        match self {
            Self::Local(svc) => {
                let role_name = RoleName::new(role);
                let config = svc.update_selected_role(role_name).await?;
                Ok(config)
            }
            #[cfg(feature = "server")]
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

    /// Test that TuiBackend::Local variant type signature is correct.
    #[tokio::test]
    async fn test_tuibackend_local_variant_exists() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<TuiBackend>();
    }

    /// Test that the backend enum is Clone and Debug (with Local variant).
    #[test]
    fn test_tuibackend_is_clone_and_debug() {
        // We can only test with Local variant without a real TuiService
        fn assert_clone<T: Clone>() {}
        fn assert_debug<T: std::fmt::Debug>() {}
        assert_clone::<TuiBackend>();
        assert_debug::<TuiBackend>();
    }

    #[cfg(feature = "server")]
    #[test]
    fn test_tuibackend_remote_variant_exists() {
        let api = ApiClient::new("http://localhost:8000".to_string());
        let backend = TuiBackend::Remote(api);

        match backend {
            TuiBackend::Remote(_) => (),
            TuiBackend::Local(_) => panic!("Expected Remote variant"),
        }
    }
}
