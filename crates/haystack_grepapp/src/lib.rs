use anyhow::Result;
use haystack_core::HaystackProvider;
use terraphim_types::{Document, SearchQuery};

mod client;
mod models;

pub use client::{GrepAppClient, SearchParams};
pub use models::{Hit, SearchResponse};

/// Grep.app haystack provider for searching code across GitHub repositories
pub struct GrepAppHaystack {
    client: GrepAppClient,
    /// Optional default language filter
    pub default_language: Option<String>,
    /// Optional default repository filter
    pub default_repo: Option<String>,
    /// Optional default path filter
    pub default_path: Option<String>,
}

impl GrepAppHaystack {
    /// Create a new GrepAppHaystack with default settings
    pub fn new() -> Result<Self> {
        Ok(Self {
            client: GrepAppClient::new()?,
            default_language: None,
            default_repo: None,
            default_path: None,
        })
    }

    /// Create a new GrepAppHaystack with filters
    pub fn with_filters(
        language: Option<String>,
        repo: Option<String>,
        path: Option<String>,
    ) -> Result<Self> {
        Ok(Self {
            client: GrepAppClient::new()?,
            default_language: language,
            default_repo: repo,
            default_path: path,
        })
    }

    /// Convert a Hit to a Document
    fn hit_to_document(&self, hit: &Hit) -> Document {
        let repo = &hit.source.repo.raw;
        let path = &hit.source.path.raw;
        let branch = &hit.source.branch.raw;

        // Construct GitHub URL
        let url = format!(
            "https://github.com/{}/blob/{}/{}",
            repo, branch, path
        );

        // Extract plain text from HTML snippet (remove <mark> tags but keep content)
        let snippet = hit.source.content.snippet
            .replace("<mark>", "")
            .replace("</mark>", "");

        // Create title from repo and file name
        let file_name = path.rsplit('/').next().unwrap_or(path);
        let title = format!("{} - {}", repo, file_name);

        // Create a unique ID from repo, path, and branch
        let id = format!("{}:{}:{}", repo, branch, path);

        Document {
            id,
            url,
            title,
            body: snippet.clone(),
            description: Some(format!("Code from {} in {}", path, repo)),
            tags: Some(vec![repo.to_string(), file_name.to_string()]),
            ..Default::default()
        }
    }
}

impl Default for GrepAppHaystack {
    fn default() -> Self {
        Self::new().expect("Failed to create default GrepAppHaystack")
    }
}

impl HaystackProvider for GrepAppHaystack {
    type Error = anyhow::Error;

    async fn search(&self, query: &SearchQuery) -> Result<Vec<Document>, Self::Error> {
        let search_term = query.search_term.to_string();

        // Build search parameters with defaults
        let params = SearchParams {
            query: search_term,
            language: self.default_language.clone(),
            repo: self.default_repo.clone(),
            path: self.default_path.clone(),
        };

        tracing::info!(
            "Searching grep.app with query: '{}', language: {:?}, repo: {:?}, path: {:?}",
            params.query,
            params.language,
            params.repo,
            params.path
        );

        let hits = self.client.search(&params).await?;

        let documents: Vec<Document> = hits
            .iter()
            .map(|hit| self.hit_to_document(hit))
            .collect();

        tracing::info!("Found {} documents from grep.app", documents.len());

        Ok(documents)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_haystack_creation() {
        let haystack = GrepAppHaystack::new();
        assert!(haystack.is_ok());

        let haystack = GrepAppHaystack::with_filters(
            Some("Rust".to_string()),
            Some("terraphim/terraphim-ai".to_string()),
            None,
        );
        assert!(haystack.is_ok());
        assert_eq!(
            haystack.unwrap().default_language,
            Some("Rust".to_string())
        );
    }

    #[test]
    fn test_hit_to_document() {
        let haystack = GrepAppHaystack::new().unwrap();

        let hit = Hit {
            source: models::HitSource {
                repo: models::RepoField {
                    raw: "terraphim/terraphim-ai".to_string(),
                },
                path: models::PathField {
                    raw: "src/main.rs".to_string(),
                },
                branch: models::BranchField {
                    raw: "main".to_string(),
                },
                content: models::ContentField {
                    snippet: "async fn <mark>search</mark>() { }".to_string(),
                },
            },
        };

        let doc = haystack.hit_to_document(&hit);

        assert_eq!(doc.url, "https://github.com/terraphim/terraphim-ai/blob/main/src/main.rs");
        assert_eq!(doc.title, "terraphim/terraphim-ai - main.rs");
        assert_eq!(doc.body, "async fn search() { }");
        assert!(doc.tags.is_some());
        assert_eq!(doc.tags.unwrap(), vec!["terraphim/terraphim-ai", "main.rs"]);
    }
}
