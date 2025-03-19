use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::Path;
use terraphim_config::{ConfigState, ServiceType};
use terraphim_types::{Index, SearchQuery};

use crate::{Error, Result};

mod ripgrep;

pub use ripgrep::RipgrepIndexer;

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
    // Note: use of `async fn` in public traits is discouraged as auto trait bounds cannot be specified
    fn index(
        &self,
        needle: &str,
        haystack: &Path,
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
    let mut full_index = Index::new();

    let role = config
        .roles
        .get(&search_query_role)
        .ok_or_else(|| Error::RoleNotFound(search_query_role.to_string()))?;

    for haystack in &role.haystacks {
        log::info!("Finding documents in haystack: {:#?}", haystack);

        let index = match haystack.service {
            ServiceType::Ripgrep => {
                // If the search term is empty, use "." to match all files
                let search_pattern = if needle.is_empty() { "." } else { needle };
                ripgrep.index(search_pattern, &haystack.path).await?
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
