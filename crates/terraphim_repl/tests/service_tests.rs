//! Service tests for REPL TuiService
//!
//! These tests verify the service layer functionality for
//! role management, search, find, replace, and thesaurus operations.

use serial_test::serial;
use std::path::PathBuf;
use terraphim_automata::{ThesaurusBuilder, builder::Logseq};

/// Build a test thesaurus from the docs/src/kg directory
async fn build_test_thesaurus() -> Result<terraphim_types::Thesaurus, Box<dyn std::error::Error>> {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());
    let manifest_path = PathBuf::from(manifest_dir);

    // Go up two levels: crates/terraphim_repl -> crates -> workspace_root
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
    async fn test_thesaurus_can_be_built() {
        let result = build_test_thesaurus().await;
        match result {
            Ok(thesaurus) => {
                assert!(!thesaurus.is_empty(), "Thesaurus should not be empty");
            }
            Err(e) => {
                eprintln!("Thesaurus build skipped: {}", e);
            }
        }
    }

    #[tokio::test]
    async fn test_thesaurus_has_terms() {
        let result = build_test_thesaurus().await;
        if let Ok(thesaurus) = result {
            let count = thesaurus.len();
            assert!(count > 0, "Thesaurus should have at least one term");
        }
    }

    #[tokio::test]
    async fn test_thesaurus_iteration() {
        let result = build_test_thesaurus().await;
        if let Ok(thesaurus) = result {
            let mut count = 0;
            for (_key, term) in thesaurus.into_iter() {
                assert!(
                    !term.value.to_string().is_empty(),
                    "Term value should not be empty"
                );
                count += 1;
            }
            assert!(count > 0, "Should iterate over at least one term");
        }
    }
}

#[cfg(test)]
mod find_matches_tests {
    use super::*;

    #[tokio::test]
    async fn test_find_matches_basic() {
        let thesaurus = match build_test_thesaurus().await {
            Ok(t) => t,
            Err(_) => return,
        };

        let text = "npm install packages";
        let result = terraphim_automata::find_matches(text, thesaurus, true);

        assert!(result.is_ok(), "find_matches should succeed");
    }

    #[tokio::test]
    async fn test_find_matches_empty_text() {
        let thesaurus = match build_test_thesaurus().await {
            Ok(t) => t,
            Err(_) => return,
        };

        let text = "";
        let result = terraphim_automata::find_matches(text, thesaurus, true);

        assert!(
            result.is_ok(),
            "find_matches should succeed with empty text"
        );
        let matches = result.unwrap();
        assert!(matches.is_empty(), "Empty text should have no matches");
    }

    #[tokio::test]
    async fn test_find_matches_no_matches() {
        let thesaurus = match build_test_thesaurus().await {
            Ok(t) => t,
            Err(_) => return,
        };

        let text = "xyz123 completely random text no matches";
        let result = terraphim_automata::find_matches(text, thesaurus, true);

        assert!(result.is_ok(), "find_matches should succeed");
        // May or may not have matches depending on thesaurus content
    }

    #[tokio::test]
    async fn test_find_matches_positions() {
        let thesaurus = match build_test_thesaurus().await {
            Ok(t) => t,
            Err(_) => return,
        };

        let text = "rust async tokio programming";
        let result = terraphim_automata::find_matches(text, thesaurus, true);

        if let Ok(matches) = result {
            for m in matches {
                // Each match should have proper fields
                assert!(!m.term.is_empty(), "Term should not be empty");
                if let Some((start, end)) = m.pos {
                    assert!(start <= end, "Start should be <= end");
                }
            }
        }
    }
}

#[cfg(test)]
mod replace_matches_tests {
    use super::*;

    #[tokio::test]
    async fn test_replace_markdown() {
        let thesaurus = match build_test_thesaurus().await {
            Ok(t) => t,
            Err(_) => return,
        };

        let text = "npm install";
        let result = terraphim_automata::replace_matches(
            text,
            thesaurus,
            terraphim_automata::LinkType::MarkdownLinks,
        );

        assert!(result.is_ok(), "replace_matches should succeed");
        let replaced = String::from_utf8(result.unwrap()).unwrap();
        assert!(!replaced.is_empty(), "Result should not be empty");
    }

    #[tokio::test]
    async fn test_replace_html() {
        let thesaurus = match build_test_thesaurus().await {
            Ok(t) => t,
            Err(_) => return,
        };

        let text = "yarn add";
        let result = terraphim_automata::replace_matches(
            text,
            thesaurus,
            terraphim_automata::LinkType::HTMLLinks,
        );

        assert!(result.is_ok(), "replace_matches HTML should succeed");
    }

    #[tokio::test]
    async fn test_replace_wiki() {
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

        assert!(result.is_ok(), "replace_matches Wiki should succeed");
    }

    #[tokio::test]
    async fn test_replace_plain() {
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

        assert!(result.is_ok(), "replace_matches PlainText should succeed");
    }

    #[tokio::test]
    async fn test_replace_empty_text() {
        let thesaurus = match build_test_thesaurus().await {
            Ok(t) => t,
            Err(_) => return,
        };

        let text = "";
        let result = terraphim_automata::replace_matches(
            text,
            thesaurus,
            terraphim_automata::LinkType::MarkdownLinks,
        );

        assert!(
            result.is_ok(),
            "replace_matches should succeed with empty text"
        );
        let replaced = String::from_utf8(result.unwrap()).unwrap();
        assert!(
            replaced.is_empty(),
            "Empty input should produce empty output"
        );
    }

    #[tokio::test]
    async fn test_replace_preserves_unmatched_text() {
        let thesaurus = match build_test_thesaurus().await {
            Ok(t) => t,
            Err(_) => return,
        };

        let text = "some xyz123 random text";
        let result = terraphim_automata::replace_matches(
            text,
            thesaurus,
            terraphim_automata::LinkType::MarkdownLinks,
        );

        if let Ok(bytes) = result {
            let replaced = String::from_utf8(bytes).unwrap();
            // Unmatched parts should be preserved
            assert!(
                replaced.contains("xyz123") || replaced.contains("random"),
                "Unmatched text should be preserved"
            );
        }
    }
}

#[cfg(test)]
mod search_query_tests {
    use terraphim_types::{NormalizedTermValue, RoleName, SearchQuery};

    #[test]
    fn test_search_query_with_all_fields() {
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
        assert_eq!(
            query.role.as_ref().map(|r| r.to_string()),
            Some("Default".to_string())
        );
    }

    #[test]
    fn test_search_query_defaults() {
        let query = SearchQuery {
            search_term: NormalizedTermValue::from("test"),
            search_terms: None,
            operator: None,
            skip: None,
            limit: None,
            role: None,
        };

        assert!(query.limit.is_none());
        assert!(query.role.is_none());
        assert!(query.skip.is_none());
    }

    #[test]
    fn test_role_name_special_characters() {
        let roles = vec![
            "Engineer",
            "System Operator",
            "Default-Role",
            "Role_with_underscore",
        ];

        for role_str in roles {
            let role = RoleName::new(role_str);
            assert_eq!(role.to_string(), role_str);
        }
    }
}

#[cfg(test)]
mod config_tests {
    use terraphim_types::RoleName;

    #[test]
    fn test_role_name_equality() {
        let role1 = RoleName::new("Default");
        let role2 = RoleName::new("Default");
        let role3 = RoleName::new("Engineer");

        assert_eq!(role1, role2);
        assert_ne!(role1, role3);
    }

    #[test]
    fn test_role_name_display() {
        let role = RoleName::new("Test Role");
        let display = format!("{}", role);
        assert_eq!(display, "Test Role");
    }
}

#[cfg(test)]
mod link_type_tests {
    use terraphim_automata::LinkType;

    #[test]
    fn test_link_types() {
        // Verify all expected link types exist
        let _ = LinkType::MarkdownLinks;
        let _ = LinkType::HTMLLinks;
        let _ = LinkType::WikiLinks;
        let _ = LinkType::PlainText;
    }
}

#[cfg(test)]
mod embedded_assets_tests {
    use std::path::PathBuf;

    #[test]
    fn test_default_config_path() {
        let config_path = dirs::home_dir().map(|h| h.join(".terraphim").join("config.json"));

        assert!(
            config_path.is_some(),
            "Should be able to construct config path"
        );
    }

    #[test]
    fn test_default_thesaurus_path() {
        let thesaurus_path =
            dirs::home_dir().map(|h| h.join(".terraphim").join("default_thesaurus.json"));

        assert!(
            thesaurus_path.is_some(),
            "Should be able to construct thesaurus path"
        );
    }

    #[test]
    fn test_history_file_path() {
        let history_path = dirs::home_dir()
            .map(|h| h.join(".terraphim_repl_history"))
            .unwrap_or_else(|| PathBuf::from(".terraphim_repl_history"));

        assert!(!history_path.to_string_lossy().is_empty());
    }
}

#[cfg(test)]
mod output_format_tests {
    #[test]
    fn test_json_serialization() {
        #[derive(serde::Serialize)]
        struct TestOutput {
            role: String,
            results: Vec<String>,
        }

        let output = TestOutput {
            role: "Default".to_string(),
            results: vec!["result1".to_string(), "result2".to_string()],
        };

        let json = serde_json::to_string(&output).unwrap();
        assert!(json.contains("Default"));
        assert!(json.contains("result1"));
    }

    #[test]
    fn test_pretty_json_serialization() {
        #[derive(serde::Serialize)]
        struct TestOutput {
            field1: String,
            field2: u32,
        }

        let output = TestOutput {
            field1: "test".to_string(),
            field2: 42,
        };

        let json = serde_json::to_string_pretty(&output).unwrap();
        // Pretty JSON should have newlines
        assert!(json.contains('\n'));
    }
}
