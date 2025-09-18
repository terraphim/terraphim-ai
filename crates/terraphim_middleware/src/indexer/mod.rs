use futures::stream::{self, StreamExt};
use terraphim_config::{ConfigState, ServiceType};
use terraphim_types::{Index, SearchQuery};

use crate::{Error, Result};

mod ripgrep;

use crate::haystack::{
    AtomicHaystackIndexer, ClickUpHaystackIndexer, McpHaystackIndexer, PerplexityHaystackIndexer,
    QueryRsHaystackIndexer,
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
                // Search through documents using atomic-server
                atomic.index(needle, haystack).await?
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

        for indexed_doc in index.values() {
            if let Err(e) = config_state.add_to_roles(indexed_doc).await {
                log::warn!(
                    "Failed to insert document `{}` ({}): {e:?}",
                    indexed_doc.title,
                    indexed_doc.url
                );
            }
        }

        full_index.extend(index);
    }
    Ok(full_index)
}

/// Parallel version of search_haystacks for improved performance
/// This version searches all haystacks concurrently with a maximum concurrency limit
pub async fn search_haystacks_parallel(
    mut config_state: ConfigState,
    search_query: SearchQuery,
    max_concurrency: usize,
) -> Result<Index> {
    let config = config_state.config.lock().await.clone();
    let search_query_role = search_query.role.unwrap_or(config.default_role);
    let needle = search_query.search_term.as_str().to_string();

    let role = config
        .roles
        .get(&search_query_role)
        .ok_or_else(|| Error::RoleNotFound(search_query_role.to_string()))?;

    let mut full_index = Index::new();

    // Create tasks for parallel execution
    let search_tasks = role.haystacks.iter().map(|haystack| {
        let needle = needle.clone();
        let haystack = haystack.clone();
        async move {
            log::info!("Finding documents in haystack: {:#?}", haystack);

            let index = match haystack.service {
                ServiceType::Ripgrep => {
                    let ripgrep = RipgrepIndexer::default();
                    ripgrep.index(needle.as_str(), &haystack).await
                }
                ServiceType::Atomic => {
                    let atomic = AtomicHaystackIndexer::default();
                    atomic.index(needle.as_str(), &haystack).await
                }
                ServiceType::QueryRs => {
                    let query_rs = QueryRsHaystackIndexer::default();
                    query_rs.index(needle.as_str(), &haystack).await
                }
                ServiceType::ClickUp => {
                    let clickup = ClickUpHaystackIndexer::default();
                    clickup.index(needle.as_str(), &haystack).await
                }
                ServiceType::Mcp => {
                    let mcp = McpHaystackIndexer;
                    mcp.index(needle.as_str(), &haystack).await
                }
                ServiceType::Perplexity => {
                    match PerplexityHaystackIndexer::from_haystack_config(&haystack) {
                        Ok(perplexity) => perplexity.index(needle.as_str(), &haystack).await,
                        Err(e) => {
                            log::error!("Failed to create Perplexity indexer: {}", e);
                            Ok(Index::new()) // Return empty index for graceful degradation
                        }
                    }
                }
            };

            (haystack.clone(), index)
        }
    });

    // Execute searches in parallel with concurrency limit
    let results = stream::iter(search_tasks)
        .buffer_unordered(max_concurrency)
        .collect::<Vec<_>>()
        .await;

    // Process results and merge indices
    for (haystack, index_result) in results {
        match index_result {
            Ok(index) => {
                // Add documents to config state
                for indexed_doc in index.values() {
                    if let Err(e) = config_state.add_to_roles(indexed_doc).await {
                        log::warn!(
                            "Failed to insert document `{}` ({}): {e:?}",
                            indexed_doc.title,
                            indexed_doc.url
                        );
                    }
                }
                full_index.extend(index);
            }
            Err(e) => {
                log::error!("Search failed for haystack {:?}: {}", haystack, e);
                // Continue with other results even if one fails
            }
        }
    }

    Ok(full_index)
}
