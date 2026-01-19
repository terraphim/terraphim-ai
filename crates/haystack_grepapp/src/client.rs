use anyhow::{Context, Result};
use reqwest::Client;
use url::Url;

use crate::models::{Hit, SearchResponse};

const GREP_APP_API_URL: &str = "https://grep.app/api/search";

/// Client for interacting with grep.app search API
#[derive(Debug, Clone)]
pub struct GrepAppClient {
    client: Client,
    api_url: Url,
}

/// Search parameters for grep.app
#[derive(Debug, Default)]
pub struct SearchParams {
    /// Search query (required)
    pub query: String,
    /// Language filter (optional, e.g., "Rust", "Python")
    pub language: Option<String>,
    /// Repository filter (optional, format: "owner/repo")
    pub repo: Option<String>,
    /// Path filter (optional, e.g., "src/")
    pub path: Option<String>,
}

impl GrepAppClient {
    /// Create a new GrepAppClient with default API URL
    pub fn new() -> Result<Self> {
        Self::with_url(GREP_APP_API_URL)
    }

    /// Create a new GrepAppClient with custom API URL (useful for testing)
    pub fn with_url(api_url: &str) -> Result<Self> {
        let api_url = Url::parse(api_url).context("Failed to parse API URL")?;
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .context("Failed to build HTTP client")?;

        Ok(Self { client, api_url })
    }

    /// Search for code using grep.app API
    pub async fn search(&self, params: &SearchParams) -> Result<Vec<Hit>> {
        if params.query.is_empty() {
            anyhow::bail!("Query cannot be empty");
        }

        if params.query.len() > 1000 {
            anyhow::bail!("Query too long (max 1000 characters)");
        }

        let mut url = self.api_url.clone();

        // Add query parameters
        {
            let mut query_pairs = url.query_pairs_mut();
            query_pairs.append_pair("q", &params.query);

            if let Some(ref lang) = params.language {
                query_pairs.append_pair("f.lang", lang);
            }

            if let Some(ref repo) = params.repo {
                query_pairs.append_pair("f.repo", repo);
            }

            if let Some(ref path) = params.path {
                query_pairs.append_pair("f.path", path);
            }
        }

        tracing::debug!("Making request to: {}", url);

        let response = self
            .client
            .get(url.as_str())
            .send()
            .await
            .context("Failed to send request to grep.app")?;

        let status = response.status();
        tracing::debug!("Response status: {}", status);

        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();

            if status.as_u16() == 429 {
                anyhow::bail!("Rate limit exceeded");
            } else if status.as_u16() == 404 {
                // No results found is not an error, return empty vec
                return Ok(vec![]);
            }

            anyhow::bail!("API request failed with status {}: {}", status, error_text);
        }

        let search_response: SearchResponse = response
            .json()
            .await
            .context("Failed to parse response JSON")?;

        tracing::debug!("Found {} hits", search_response.hits.hits.len());

        Ok(search_response.hits.hits)
    }
}

impl Default for GrepAppClient {
    fn default() -> Self {
        Self::new().expect("Failed to create default GrepAppClient")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::{
        Mock, MockServer, ResponseTemplate,
        matchers::{method, path, query_param},
    };

    #[tokio::test]
    async fn test_search_success() {
        let mock_server = MockServer::start().await;

        let mock_response = serde_json::json!({
            "facets": {
                "lang": {
                    "buckets": [{"key": "Rust", "doc_count": 42}]
                }
            },
            "hits": {
                "hits": [
                    {
                        "_source": {
                            "repo": {"raw": "terraphim/terraphim-ai"},
                            "path": {"raw": "src/main.rs"},
                            "branch": {"raw": "main"},
                            "content": {"snippet": "async fn <mark>search</mark>()"}
                        }
                    }
                ]
            }
        });

        Mock::given(method("GET"))
            .and(path("/api/search"))
            .and(query_param("q", "async search"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&mock_response))
            .mount(&mock_server)
            .await;

        let client = GrepAppClient::with_url(&format!("{}/api/search", mock_server.uri())).unwrap();

        let params = SearchParams {
            query: "async search".to_string(),
            ..Default::default()
        };

        let results = client.search(&params).await.unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].source.repo.raw, "terraphim/terraphim-ai");
        assert_eq!(results[0].source.path.raw, "src/main.rs");
    }

    #[tokio::test]
    async fn test_search_with_filters() {
        let mock_server = MockServer::start().await;

        let mock_response = serde_json::json!({
            "hits": {
                "hits": []
            }
        });

        Mock::given(method("GET"))
            .and(path("/api/search"))
            .and(query_param("q", "tokio spawn"))
            .and(query_param("f.lang", "Rust"))
            .and(query_param("f.repo", "tokio-rs/tokio"))
            .and(query_param("f.path", "tokio/src/"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&mock_response))
            .mount(&mock_server)
            .await;

        let client = GrepAppClient::with_url(&format!("{}/api/search", mock_server.uri())).unwrap();

        let params = SearchParams {
            query: "tokio spawn".to_string(),
            language: Some("Rust".to_string()),
            repo: Some("tokio-rs/tokio".to_string()),
            path: Some("tokio/src/".to_string()),
        };

        let results = client.search(&params).await.unwrap();
        assert_eq!(results.len(), 0);
    }

    #[tokio::test]
    async fn test_search_404_returns_empty() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/api/search"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&mock_server)
            .await;

        let client = GrepAppClient::with_url(&format!("{}/api/search", mock_server.uri())).unwrap();

        let params = SearchParams {
            query: "nonexistent".to_string(),
            ..Default::default()
        };

        let results = client.search(&params).await.unwrap();
        assert_eq!(results.len(), 0);
    }

    #[tokio::test]
    async fn test_search_rate_limit() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/api/search"))
            .respond_with(ResponseTemplate::new(429))
            .mount(&mock_server)
            .await;

        let client = GrepAppClient::with_url(&format!("{}/api/search", mock_server.uri())).unwrap();

        let params = SearchParams {
            query: "test".to_string(),
            ..Default::default()
        };

        let error = client.search(&params).await.unwrap_err();
        assert!(error.to_string().contains("Rate limit"));
    }

    #[test]
    fn test_empty_query_validation() {
        let client = GrepAppClient::new().unwrap();
        let params = SearchParams {
            query: "".to_string(),
            ..Default::default()
        };

        let result = tokio_test::block_on(client.search(&params));
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("empty"));
    }

    #[test]
    fn test_query_length_validation() {
        let client = GrepAppClient::new().unwrap();
        let params = SearchParams {
            query: "a".repeat(1001),
            ..Default::default()
        };

        let result = tokio_test::block_on(client.search(&params));
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("too long"));
    }
}
