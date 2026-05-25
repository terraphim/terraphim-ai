use std::sync::Arc;

use terraphim_config::{ConfigState, ServiceType};
use terraphim_file_search::kg_scorer::KgPathScorer;
use terraphim_types::{Index, RelevanceFunction, RoleName, SearchQuery};

use crate::{Error, Result};

mod fff;
mod ripgrep;

#[cfg(feature = "ai-assistant")]
use crate::haystack::AiAssistantHaystackIndexer;
#[cfg(feature = "grepapp")]
use crate::haystack::GrepAppHaystackIndexer;
#[cfg(feature = "jmap")]
use crate::haystack::JmapHaystackIndexer;
use crate::haystack::{
    ClickUpHaystackIndexer, McpHaystackIndexer, PerplexityHaystackIndexer, QueryRsHaystackIndexer,
    QuickwitHaystackIndexer,
};
pub use fff::FffIndexer;
pub use ripgrep::RipgrepIndexer;

async fn kg_scorer_for_role(
    config_state: &ConfigState,
    role_name: &RoleName,
    role: &terraphim_config::Role,
) -> Option<Arc<KgPathScorer>> {
    if role.relevance_function != RelevanceFunction::TerraphimGraph {
        return None;
    }
    let rg_sync = config_state.roles.get(role_name)?;
    let thesaurus = {
        let rg = rg_sync.lock().await;
        if rg.thesaurus.is_empty() {
            return None;
        }
        rg.thesaurus.clone()
    };
    Some(Arc::new(KgPathScorer::new(thesaurus)))
}

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

    let role = config
        .roles
        .get(&search_query_role)
        .ok_or_else(|| Error::RoleNotFound(search_query_role.to_string()))?;

    let kg_scorer = kg_scorer_for_role(&config_state, &search_query_role, role).await;

    let mut fff = FffIndexer::default();
    if let Some(scorer) = kg_scorer {
        fff = fff.with_kg_scorer(scorer);
    }
    let query_rs = QueryRsHaystackIndexer::default();
    let clickup = ClickUpHaystackIndexer::default();
    let mut full_index = Index::new();

    for haystack in &role.haystacks {
        log::info!("Finding documents in haystack: {:#?}", haystack);

        let index = match haystack.service {
            ServiceType::Ripgrep => {
                // Search through documents using fff-search
                // This indexes the haystack using the fff-search middleware
                fff.index(needle, haystack).await?
            }
            ServiceType::Atomic => {
                log::warn!(
                    "Atomic haystack support not enabled. Skipping haystack: {}",
                    haystack.location
                );
                Index::new()
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
            ServiceType::GrepApp => {
                #[cfg(feature = "grepapp")]
                {
                    // Search using grep.app for code across GitHub repositories
                    let grep_app = GrepAppHaystackIndexer::default();
                    grep_app.index(needle, haystack).await?
                }
                #[cfg(not(feature = "grepapp"))]
                {
                    log::warn!(
                        "GrepApp haystack support not enabled. Skipping haystack: {}",
                        haystack.location
                    );
                    Index::new()
                }
            }
            ServiceType::AiAssistant => {
                #[cfg(feature = "ai-assistant")]
                {
                    // Search through AI coding assistant session logs
                    let ai_assistant = AiAssistantHaystackIndexer;
                    ai_assistant.index(needle, haystack).await?
                }
                #[cfg(not(feature = "ai-assistant"))]
                {
                    log::warn!(
                        "AI assistant haystack support not enabled. Skipping haystack: {}",
                        haystack.location
                    );
                    Index::new()
                }
            }
            ServiceType::Quickwit => {
                // Search using Quickwit search engine for log and observability data
                let quickwit = QuickwitHaystackIndexer::default();
                quickwit.index(needle, haystack).await?
            }
            ServiceType::Jmap => {
                #[cfg(feature = "jmap")]
                {
                    // Search emails via JMAP protocol
                    let jmap = JmapHaystackIndexer;
                    jmap.index(needle, haystack).await?
                }
                #[cfg(not(feature = "jmap"))]
                {
                    log::warn!(
                        "JMAP haystack support not enabled. Skipping haystack: {}",
                        haystack.location
                    );
                    Index::new()
                }
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
