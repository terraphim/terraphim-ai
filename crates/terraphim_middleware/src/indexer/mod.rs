use terraphim_config::{ConfigState, ServiceType};
use terraphim_types::{Index, SearchQuery};

use crate::{Error, Result};

mod ripgrep;

#[cfg(feature = "atomic")]
use crate::haystack::AtomicHaystackIndexer;
use crate::haystack::{
    ClickUpHaystackIndexer, McpHaystackIndexer, PerplexityHaystackIndexer, QueryRsHaystackIndexer,
};
pub use ripgrep::RipgrepIndexer;

/// A Middleware is a service that creates an index of documents from
/// a haystack.
///
/// Every middleware receives a needle and a haystack and returns
/// a HashMap of Documents.
pub trait IndexMiddleware {
    /// Index the haystack and return a HashMap of Documents
    ///
    /// # Errors
    ///
    /// Returns an error if the middleware fails to index the haystack
    fn index(
        &self,
        needle: &str,
        haystack: &terraphim_config::Haystack,
    ) -> impl std::future::Future<Output = Result<Index>> + Send;
}

/// Use Middleware to search through haystacks and return an index of documents
/// that match the search query.
pub async fn search_haystacks(
    mut config_state: ConfigState,
    search_query: SearchQuery,
) -> Result<Index> {
    let config = config_state.config.lock().await.clone();
    let search_query_role = search_query.role.unwrap_or(config.default_role);
    let needle = search_query.search_term.as_str();

    let ripgrep = RipgrepIndexer::default();
    #[cfg(feature = "atomic")]
    let atomic = AtomicHaystackIndexer::default();
    let query_rs = QueryRsHaystackIndexer::default();
    let clickup = ClickUpHaystackIndexer::default();
    let mut full_index = Index::new();

    let role = config
        .roles
        .get(&search_query_role)
        .ok_or_else(|| Error::RoleNotFound(search_query_role.to_string()))?;

    for haystack in &role.haystacks {
        log::info!("Finding documents in haystack: {:#?}", haystack);

        let index = match haystack.service {
            ServiceType::Ripgrep => {
                // Search through documents using ripgrep
                // This indexes the haystack using the ripgrep middleware
                ripgrep.index(needle, haystack).await?
            }
            ServiceType::Atomic => {
                #[cfg(feature = "atomic")]
                {
                    // Search through documents using atomic-server
                    atomic.index(needle, haystack).await?
                }
                #[cfg(not(feature = "atomic"))]
                {
                    log::warn!(
                        "Atomic haystack support not enabled. Skipping haystack: {}",
                        haystack.location
                    );
                    Index::new()
                }
            }
            ServiceType::QueryRs => {
                // Search through documents using query.rs
                query_rs.index(needle, haystack).await?
            }
            ServiceType::ClickUp => {
                // Search through documents using ClickUp
                clickup.index(needle, haystack).await?
            }
            ServiceType::Mcp => {
                // Search via MCP client
                let mcp = McpHaystackIndexer;
                mcp.index(needle, haystack).await?
            }
            ServiceType::Perplexity => {
                // Search using Perplexity AI-powered web search
                let perplexity = match PerplexityHaystackIndexer::from_haystack_config(haystack) {
                    Ok(indexer) => indexer,
                    Err(e) => {
                        log::error!("Failed to create Perplexity indexer: {}", e);
                        // Return empty index to allow graceful degradation
                        return Ok(Index::new());
                    }
                };
                perplexity.index(needle, haystack).await?
            }
        };

        // Tag all documents from this haystack with their source
        let mut tagged_index = Index::new();
        for (doc_id, mut document) in index {
            // Set the source haystack for this document
            document.source_haystack = Some(haystack.location.clone());
            tagged_index.insert(doc_id, document);
        }

        for indexed_doc in tagged_index.values() {
            if let Err(e) = config_state.add_to_roles(indexed_doc).await {
                log::warn!(
                    "Failed to insert document `{}` ({}): {e:?}",
                    indexed_doc.title,
                    indexed_doc.url
                );
            }
        }

        full_index.extend(tagged_index);
    }
    Ok(full_index)
}
