use serde::{Deserialize, Serialize};

/// Response from grep.app search API
#[derive(Debug, Deserialize, Serialize)]
pub struct SearchResponse {
    /// Aggregated statistics for the search results
    pub facets: Option<Facets>,
    /// Array of search result hits
    pub hits: Hits,
}

/// Search result hits container
#[derive(Debug, Deserialize, Serialize)]
pub struct Hits {
    /// Array of individual search results
    #[serde(default)]
    pub hits: Vec<Hit>,
}

/// Aggregated statistics
#[derive(Debug, Deserialize, Serialize)]
pub struct Facets {
    /// Language breakdown
    pub lang: Option<FacetBucket>,
    /// Repository breakdown
    pub repo: Option<FacetBucket>,
}

/// Facet bucket containing counts
#[derive(Debug, Deserialize, Serialize)]
pub struct FacetBucket {
    /// Array of buckets with counts
    pub buckets: Vec<Bucket>,
}

/// Individual bucket with key and count
#[derive(Debug, Deserialize, Serialize)]
pub struct Bucket {
    /// Bucket key (e.g., language name or repo name)
    pub key: String,
    /// Number of results in this bucket
    #[serde(rename = "doc_count")]
    pub doc_count: u64,
}

/// Individual search result hit
#[derive(Debug, Deserialize, Serialize)]
pub struct Hit {
    /// Source data containing the actual result details
    #[serde(rename = "_source")]
    pub source: HitSource,
}

/// Source data for a search hit
#[derive(Debug, Deserialize, Serialize)]
pub struct HitSource {
    /// Repository information
    pub repo: RepoField,
    /// File path information
    pub path: PathField,
    /// Branch information
    pub branch: BranchField,
    /// Content snippet with matches
    pub content: ContentField,
}

/// Repository field
#[derive(Debug, Deserialize, Serialize)]
pub struct RepoField {
    /// Raw repository name (e.g., "owner/repo")
    pub raw: String,
}

/// Path field
#[derive(Debug, Deserialize, Serialize)]
pub struct PathField {
    /// Raw file path
    pub raw: String,
}

/// Branch field
#[derive(Debug, Deserialize, Serialize)]
pub struct BranchField {
    /// Raw branch name
    pub raw: String,
}

/// Content field with snippet
#[derive(Debug, Deserialize, Serialize)]
pub struct ContentField {
    /// HTML-formatted code snippet with highlighted matches
    pub snippet: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deserialize_search_response() {
        let json = r#"{
            "facets": {
                "lang": {
                    "buckets": [
                        {"key": "Rust", "doc_count": 42}
                    ]
                },
                "repo": {
                    "buckets": [
                        {"key": "terraphim/terraphim-ai", "doc_count": 10}
                    ]
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
        }"#;

        let response: SearchResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.hits.hits.len(), 1);
        assert_eq!(response.hits.hits[0].source.repo.raw, "terraphim/terraphim-ai");

        let facets = response.facets.unwrap();
        assert!(facets.lang.is_some());
        assert_eq!(facets.lang.unwrap().buckets[0].key, "Rust");
    }
}
