use crate::indexer::IndexMiddleware;
use crate::Result;
use grepapp_haystack::{GrepAppClient, SearchParams};
use terraphim_config::Haystack;
use terraphim_types::Index;

/// Middleware that uses grep.app as a haystack for searching code across GitHub repositories.
#[derive(Debug, Clone)]
pub struct GrepAppHaystackIndexer {
    client: GrepAppClient,
}

impl Default for GrepAppHaystackIndexer {
    fn default() -> Self {
        Self {
            client: GrepAppClient::new().expect("Failed to create GrepAppClient"),
        }
    }
}

impl IndexMiddleware for GrepAppHaystackIndexer {
    #[allow(clippy::manual_async_fn)]
    fn index(
        &self,
        needle: &str,
        haystack: &Haystack,
    ) -> impl std::future::Future<Output = Result<Index>> + Send {
        async move {
            log::info!("GrepApp: Starting search for query: '{}'", needle);

            // Extract optional filters from haystack extra_parameters
            let language = haystack.extra_parameters.get("language").and_then(|v| {
                if v.is_empty() {
                    None
                } else {
                    Some(v.clone())
                }
            });

            let repo = haystack.extra_parameters.get("repo").and_then(|v| {
                if v.is_empty() {
                    None
                } else {
                    Some(v.clone())
                }
            });

            let path = haystack.extra_parameters.get("path").and_then(|v| {
                if v.is_empty() {
                    None
                } else {
                    Some(v.clone())
                }
            });

            log::debug!(
                "GrepApp: Filters - language: {:?}, repo: {:?}, path: {:?}",
                language,
                repo,
                path
            );

            // Build search parameters
            let params = SearchParams {
                query: needle.to_string(),
                language,
                repo,
                path,
            };

            // Execute search
            let hits = match self.client.search(&params).await {
                Ok(hits) => hits,
                Err(e) => {
                    log::warn!("GrepApp search failed for '{}': {}", needle, e);
                    // Return empty index on error to allow graceful degradation
                    return Ok(Index::new());
                }
            };

            log::info!(
                "GrepApp: Found {} results for query: '{}'",
                hits.len(),
                needle
            );

            // Convert hits to documents and build index
            let mut index = Index::new();
            for hit in hits {
                let repo = &hit.source.repo.raw;
                let path = &hit.source.path.raw;
                let branch = &hit.source.branch.raw;

                // Construct GitHub URL
                let url = format!("https://github.com/{}/blob/{}/{}", repo, branch, path);

                // Extract plain text from HTML snippet (remove <mark> tags but keep content)
                let snippet = hit
                    .source
                    .content
                    .snippet
                    .replace("<mark>", "")
                    .replace("</mark>", "");

                // Create title from repo and file name
                let file_name = path.rsplit('/').next().unwrap_or(path);
                let title = format!("{} - {}", repo, file_name);

                // Create a unique ID from repo, path, and branch
                let id = format!("grepapp:{}:{}:{}", repo, branch, path)
                    .replace('/', "_")
                    .replace('.', "_")
                    .replace(':', "_");

                let document = terraphim_types::Document {
                    id: id.clone(),
                    url,
                    title,
                    body: snippet.clone(),
                    description: Some(format!("Code from {} in {}", path, repo)),
                    tags: Some(vec![
                        repo.to_string(),
                        file_name.to_string(),
                        "grep.app".to_string(),
                    ]),
                    source_haystack: Some("GrepApp".to_string()),
                    ..Default::default()
                };

                index.insert(id, document);
            }

            log::info!("GrepApp: Indexed {} documents", index.len());

            Ok(index)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use terraphim_config::ServiceType;

    fn create_test_haystack(extra_parameters: HashMap<String, String>) -> Haystack {
        Haystack {
            location: "https://grep.app".to_string(),
            service: ServiceType::GrepApp,
            read_only: false,
            fetch_content: false,
            atomic_server_secret: None,
            extra_parameters,
        }
    }

    #[tokio::test]
    async fn test_indexer_creation() {
        let indexer = GrepAppHaystackIndexer::default();
        let haystack = create_test_haystack(HashMap::new());

        // Test with empty query - should return empty index gracefully
        let result = indexer.index("", &haystack).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_filter_extraction() {
        let mut params = HashMap::new();
        params.insert("language".to_string(), "Rust".to_string());
        params.insert("repo".to_string(), "tokio-rs/tokio".to_string());
        params.insert("path".to_string(), "src/".to_string());

        let haystack = create_test_haystack(params);
        let indexer = GrepAppHaystackIndexer::default();

        // This would make a real API call, so we just test that it doesn't panic
        let result = indexer.index("tokio", &haystack).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_empty_filters() {
        let mut params = HashMap::new();
        params.insert("language".to_string(), "".to_string());
        params.insert("repo".to_string(), "".to_string());

        let haystack = create_test_haystack(params);
        let indexer = GrepAppHaystackIndexer::default();

        let result = indexer.index("test", &haystack).await;
        assert!(result.is_ok());
    }
}
