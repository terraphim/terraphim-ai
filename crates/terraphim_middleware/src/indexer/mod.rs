use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use terraphim_config::{ConfigState, ServiceType};
use terraphim_types::{Index, SearchQuery};

use crate::{Error, Result};

mod ripgrep;

pub use ripgrep::RipgrepIndexer;
use crate::haystack::{AtomicHaystackIndexer, QueryRsHaystackIndexer, ClickUpHaystackIndexer};

fn hash_as_string<T: Hash>(t: &T) -> String {
    let mut s = DefaultHasher::new();
    t.hash(&mut s);
    format!("{:x}", s.finish())
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
