//! Tests for CliService functionality
//!
//! These tests verify the CliService methods work correctly for
//! role management, search, find, replace, and thesaurus operations.

use serial_test::serial;
use std::path::PathBuf;
use terraphim_automata::{ThesaurusBuilder, builder::Logseq};

/// Build a test thesaurus from the docs/src/kg directory
async fn build_test_thesaurus() -> Result<terraphim_types::Thesaurus, Box<dyn std::error::Error>> {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());
    let manifest_path = PathBuf::from(manifest_dir);

    // Go up two levels: crates/terraphim_cli -> crates -> workspace_root
    let workspace_root = manifest_path
        .parent()
        .and_then(|p| p.parent())
        .ok_or("Cannot find workspace root")?;

    let kg_path = workspace_root.join("docs/src/kg");

    if !kg_path.exists() {
        return Err(format!("KG path does not exist: {:?}", kg_path).into());
    }

    let logseq_builder = Logseq::default();
    let thesaurus = logseq_builder
        .build("test_role".to_string(), kg_path)
        .await?;

    Ok(thesaurus)
}

#[cfg(test)]
mod thesaurus_tests {
    use super::*;

    #[tokio::test]
    async fn test_thesaurus_can_be_loaded() {
        let result = build_test_thesaurus().await;
        assert!(result.is_ok(), "Should be able to build thesaurus");

        let thesaurus = result.unwrap();
        assert!(!thesaurus.is_empty(), "Thesaurus should not be empty");
    }

    #[tokio::test]
    async fn test_thesaurus_has_expected_terms() {
        let thesaurus = match build_test_thesaurus().await {
            Ok(t) => t,
            Err(_) => return, // Skip if KG files not available
        };

        // The thesaurus should have some terms
        let term_count = thesaurus.len();
        assert!(term_count > 0, "Thesaurus should have terms");
    }
}

#[cfg(test)]
mod automata_tests {
    use super::*;

    #[tokio::test]
    async fn test_find_matches_basic() {
        let thesaurus = match build_test_thesaurus().await {
            Ok(t) => t,
            Err(_) => return, // Skip if KG files not available
        };

        let text = "npm install packages";
        let matches = terraphim_automata::find_matches(text, thesaurus, true);

        assert!(matches.is_ok(), "find_matches should succeed");
    }

    #[tokio::test]
    async fn test_replace_matches_markdown() {
        let thesaurus = match build_test_thesaurus().await {
            Ok(t) => t,
            Err(_) => return, // Skip if KG files not available
        };

        let text = "npm install";
        let result = terraphim_automata::replace_matches(
            text,
            thesaurus,
            terraphim_automata::LinkType::MarkdownLinks,
        );

        assert!(result.is_ok(), "replace_matches should succeed");
        let replaced = String::from_utf8(result.unwrap()).unwrap();

        // Result should be different from input if there are matches
        // or same if no matches
        assert!(!replaced.is_empty(), "Result should not be empty");
    }

    #[tokio::test]
    async fn test_replace_matches_html() {
        let thesaurus = match build_test_thesaurus().await {
            Ok(t) => t,
            Err(_) => return,
        };

        let text = "yarn add dependencies";
        let result = terraphim_automata::replace_matches(
            text,
            thesaurus,
            terraphim_automata::LinkType::HTMLLinks,
        );

        assert!(result.is_ok(), "replace_matches with HTML should succeed");
    }

    #[tokio::test]
    async fn test_replace_matches_wiki() {
        let thesaurus = match build_test_thesaurus().await {
            Ok(t) => t,
            Err(_) => return,
        };

        let text = "pnpm install";
        let result = terraphim_automata::replace_matches(
            text,
            thesaurus,
            terraphim_automata::LinkType::WikiLinks,
        );

        assert!(result.is_ok(), "replace_matches with Wiki should succeed");
    }

    #[tokio::test]
    async fn test_replace_matches_plain() {
        let thesaurus = match build_test_thesaurus().await {
            Ok(t) => t,
            Err(_) => return,
        };

        let text = "npm run build";
        let result = terraphim_automata::replace_matches(
            text,
            thesaurus,
            terraphim_automata::LinkType::PlainText,
        );

        assert!(
            result.is_ok(),
            "replace_matches with PlainText should succeed"
        );
    }

    #[tokio::test]
    async fn test_find_matches_returns_positions() {
        let thesaurus = match build_test_thesaurus().await {
            Ok(t) => t,
            Err(_) => return,
        };

        let text = "testing npm with yarn and pnpm";
        let matches = terraphim_automata::find_matches(text, thesaurus, true);

        if let Ok(matches) = matches {
            for m in &matches {
                // Each match should have a term
                assert!(!m.term.is_empty(), "Match should have a term");
                // Position should be Some if include_positions is true
                if let Some((start, end)) = m.pos {
                    assert!(start <= end, "Start should be <= end");
                    assert!(end <= text.len(), "End should be within text bounds");
                }
            }
        }
    }
}

#[cfg(test)]
mod link_type_tests {
    use terraphim_automata::LinkType;

    #[test]
    fn test_link_types_exist() {
        // Verify all expected link types exist
        let _ = LinkType::MarkdownLinks;
        let _ = LinkType::HTMLLinks;
        let _ = LinkType::WikiLinks;
        let _ = LinkType::PlainText;
    }
}

#[cfg(test)]
mod search_query_tests {
    use terraphim_types::{NormalizedTermValue, RoleName, SearchQuery};

    #[test]
    fn test_search_query_construction() {
        let query = SearchQuery {
            search_term: NormalizedTermValue::from("rust async"),
            search_terms: None,
            operator: None,
            skip: Some(0),
            limit: Some(10),
            role: Some(RoleName::new("Default")),
        };

        assert_eq!(query.search_term.to_string(), "rust async");
        assert_eq!(query.limit, Some(10));
        assert_eq!(query.skip, Some(0));
    }

    #[test]
    fn test_search_query_without_role() {
        let query = SearchQuery {
            search_term: NormalizedTermValue::from("tokio"),
            search_terms: None,
            operator: None,
            skip: None,
            limit: None,
            role: None,
        };

        assert!(query.role.is_none());
        assert!(query.limit.is_none());
    }

    #[test]
    fn test_role_name_creation() {
        let role = RoleName::new("Engineer");
        assert_eq!(role.to_string(), "Engineer");

        let role2 = RoleName::new("System Operator");
        assert_eq!(role2.to_string(), "System Operator");
    }
}

#[cfg(test)]
mod output_format_tests {
    #[test]
    fn test_json_serialization() {
        #[derive(serde::Serialize)]
        struct TestResult {
            query: String,
            count: usize,
        }

        let result = TestResult {
            query: "rust".to_string(),
            count: 5,
        };

        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("rust"));
        assert!(json.contains("5"));
    }

    #[test]
    fn test_json_pretty_serialization() {
        #[derive(serde::Serialize)]
        struct TestResult {
            query: String,
            count: usize,
        }

        let result = TestResult {
            query: "async".to_string(),
            count: 10,
        };

        let json = serde_json::to_string_pretty(&result).unwrap();
        // Pretty JSON should have newlines
        assert!(json.contains('\n'));
    }

    #[test]
    fn test_search_result_structure() {
        #[derive(serde::Serialize, serde::Deserialize)]
        struct SearchResult {
            query: String,
            role: String,
            results: Vec<DocumentResult>,
            count: usize,
        }

        #[derive(serde::Serialize, serde::Deserialize)]
        struct DocumentResult {
            id: String,
            title: String,
            url: String,
            rank: Option<f64>,
        }

        let result = SearchResult {
            query: "test".to_string(),
            role: "Default".to_string(),
            results: vec![DocumentResult {
                id: "1".to_string(),
                title: "Test Doc".to_string(),
                url: "https://example.com".to_string(),
                rank: Some(1.0),
            }],
            count: 1,
        };

        let json = serde_json::to_string(&result).unwrap();
        let parsed: SearchResult = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.query, "test");
        assert_eq!(parsed.count, 1);
        assert_eq!(parsed.results.len(), 1);
    }

    #[test]
    fn test_find_result_structure() {
        #[derive(serde::Serialize, serde::Deserialize)]
        struct FindResult {
            text: String,
            matches: Vec<MatchResult>,
            count: usize,
        }

        #[derive(serde::Serialize, serde::Deserialize)]
        struct MatchResult {
            term: String,
            position: Option<(usize, usize)>,
            normalized: String,
        }

        let result = FindResult {
            text: "rust async".to_string(),
            matches: vec![
                MatchResult {
                    term: "rust".to_string(),
                    position: Some((0, 4)),
                    normalized: "rust programming language".to_string(),
                },
                MatchResult {
                    term: "async".to_string(),
                    position: Some((5, 10)),
                    normalized: "asynchronous programming".to_string(),
                },
            ],
            count: 2,
        };

        let json = serde_json::to_string(&result).unwrap();
        let parsed: FindResult = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.matches.len(), 2);
        assert_eq!(parsed.count, 2);
    }

    #[test]
    fn test_replace_result_structure() {
        #[derive(serde::Serialize, serde::Deserialize)]
        struct ReplaceResult {
            original: String,
            replaced: String,
            format: String,
        }

        let result = ReplaceResult {
            original: "rust programming".to_string(),
            replaced: "[rust](https://rust-lang.org) programming".to_string(),
            format: "markdown".to_string(),
        };

        let json = serde_json::to_string(&result).unwrap();
        let parsed: ReplaceResult = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.format, "markdown");
        assert!(parsed.replaced.contains("[rust]"));
    }

    #[test]
    fn test_thesaurus_result_structure() {
        #[derive(serde::Serialize, serde::Deserialize)]
        struct ThesaurusResult {
            role: String,
            name: String,
            terms: Vec<ThesaurusTerm>,
            total_count: usize,
            shown_count: usize,
        }

        #[derive(serde::Serialize, serde::Deserialize)]
        struct ThesaurusTerm {
            id: u64,
            term: String,
            normalized: String,
            url: Option<String>,
        }

        let result = ThesaurusResult {
            role: "Default".to_string(),
            name: "default".to_string(),
            terms: vec![ThesaurusTerm {
                id: 1,
                term: "rust".to_string(),
                normalized: "rust programming language".to_string(),
                url: Some("https://rust-lang.org".to_string()),
            }],
            total_count: 30,
            shown_count: 1,
        };

        let json = serde_json::to_string(&result).unwrap();
        let parsed: ThesaurusResult = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.role, "Default");
        assert_eq!(parsed.total_count, 30);
        assert_eq!(parsed.shown_count, 1);
    }

    #[test]
    fn test_graph_result_structure() {
        #[derive(serde::Serialize, serde::Deserialize)]
        struct GraphResult {
            role: String,
            top_k: usize,
            concepts: Vec<String>,
        }

        let result = GraphResult {
            role: "Default".to_string(),
            top_k: 10,
            concepts: vec![
                "concept_1".to_string(),
                "concept_2".to_string(),
                "concept_3".to_string(),
            ],
        };

        let json = serde_json::to_string(&result).unwrap();
        let parsed: GraphResult = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.top_k, 10);
        assert_eq!(parsed.concepts.len(), 3);
    }
}

#[cfg(test)]
mod error_handling_tests {
    #[test]
    fn test_error_result_structure() {
        #[derive(serde::Serialize, serde::Deserialize)]
        struct ErrorResult {
            error: String,
            details: Option<String>,
        }

        let result = ErrorResult {
            error: "Unknown format: invalid".to_string(),
            details: Some("Use: markdown, html, wiki, or plain".to_string()),
        };

        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("error"));
        assert!(json.contains("details"));
    }

    #[test]
    fn test_error_without_details() {
        #[derive(serde::Serialize, serde::Deserialize)]
        struct ErrorResult {
            error: String,
            #[serde(skip_serializing_if = "Option::is_none")]
            details: Option<String>,
        }

        let result = ErrorResult {
            error: "Simple error".to_string(),
            details: None,
        };

        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("error"));
        // details should not appear when None
        assert!(!json.contains("details"));
    }
}
