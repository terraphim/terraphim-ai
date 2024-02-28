use ahash::AHashMap;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::Path;
use terraphim_config::{ConfigState, ServiceType};
use terraphim_types::{Index, SearchQuery};

use crate::{Error, Result};

mod ripgrep;

pub use ripgrep::RipgrepIndexer;

fn calculate_hash<T: Hash>(t: &T) -> String {
    let mut s = DefaultHasher::new();
    t.hash(&mut s);
    format!("{:x}", s.finish())
}

/// A Middleware is a service that creates an index of articles from
/// a haystack.
///
/// Every middleware receives a needle and a haystack and returns
/// a HashMap of Articles.
pub trait IndexMiddleware {
    /// Index the haystack and return a HashMap of Articles
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

/// Use Middleware to search through haystacks and return an index of articles
/// that match the search query.
pub async fn search_haystacks(
    mut config_state: ConfigState,
    search_query: SearchQuery,
) -> Result<Index> {
    let config = config_state.config.lock().await.clone();

    let search_query_role = search_query
        .role
        .unwrap_or(config.default_role)
        .to_lowercase();

    let role_config = config
        .roles
        .get(&search_query_role)
        .ok_or_else(|| Error::RoleNotFound(search_query_role.to_string()))?;

    // Define middleware to be used for searching.
    let ripgrep = RipgrepIndexer::default();

    let mut all_new_articles: Index = AHashMap::new();

    for haystack in &role_config.haystacks {
        log::info!("Finding articles in haystack: {:#?}", haystack);
        let needle = &search_query.search_term;

        let new_articles = match haystack.service {
            ServiceType::Ripgrep => {
                // Search through articles using ripgrep
                // This spins up ripgrep the service and indexes into the
                // `TerraphimGraph` and caches the articles
                ripgrep.index(needle, &haystack.path).await?
            }
        };

        for new_article in new_articles.values() {
            if let Err(e) = config_state.index_article(new_article).await {
                log::warn!(
                    "Failed to index article `{}` ({}): {e:?}",
                    new_article.title,
                    new_article.url
                );
            }
        }

        all_new_articles.extend(new_articles);
    }
    Ok(all_new_articles)
}
