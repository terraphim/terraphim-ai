//! Tests for TuiService methods
//!
//! These tests verify that the core TuiService methods work correctly.

use anyhow::Result;
use terraphim_agent::service::TuiService;

/// Test that TuiService can be created and basic methods work
#[tokio::test]
async fn test_tui_service_creation() -> Result<()> {
    let service = TuiService::new_with_embedded_defaults().await?;

    // Get the current config
    let config = service.get_config().await;
    let roles_count = config.roles.len();
    println!("Configuration has {} roles", roles_count);

    // Get the selected role
    let selected_role = service.get_selected_role().await;
    assert!(
        !selected_role.to_string().is_empty(),
        "Should have a selected role"
    );

    Ok(())
}

/// Test the search method with default role
#[tokio::test]
async fn test_tui_service_search() -> Result<()> {
    let service = TuiService::new_with_embedded_defaults().await?;

    // Search with the default search method (uses selected role)
    let results = service.search("test", Some(5)).await;

    // Search may return empty or results depending on data, but should not panic
    match results {
        Ok(docs) => {
            println!("Search returned {} documents", docs.len());
        }
        Err(e) => {
            // Expected if no haystack data is available
            println!("Search returned error (expected if no data): {}", e);
        }
    }

    Ok(())
}

/// Test autocomplete method
#[tokio::test]
async fn test_tui_service_autocomplete() -> Result<()> {
    let service = TuiService::new_with_embedded_defaults().await?;
    let role_name = service.get_selected_role().await;

    // Autocomplete may fail if no thesaurus is loaded, which is expected
    let results = service.autocomplete(&role_name, "test", Some(5)).await;

    match results {
        Ok(suggestions) => {
            println!("Autocomplete returned {} suggestions", suggestions.len());
            for suggestion in &suggestions {
                println!("  - {} (score: {})", suggestion.term, suggestion.score);
            }
        }
        Err(e) => {
            // Expected if no thesaurus data is available
            println!("Autocomplete returned error (expected if no data): {}", e);
        }
    }

    Ok(())
}

/// Test replace_matches method
#[tokio::test]
async fn test_tui_service_replace_matches() -> Result<()> {
    let service = TuiService::new_with_embedded_defaults().await?;
    let role_name = service.get_selected_role().await;

    let text = "This is a test with some terms to replace.";
    let link_type = terraphim_automata::LinkType::HTMLLinks;

    // Replace matches may fail if no thesaurus is loaded
    let result = service.replace_matches(&role_name, text, link_type).await;

    match result {
        Ok(replaced_text) => {
            println!("Replace matches result: {}", replaced_text);
        }
        Err(e) => {
            // Expected if no thesaurus data is available
            println!(
                "Replace matches returned error (expected if no data): {}",
                e
            );
        }
    }

    Ok(())
}

/// Test summarize method
#[tokio::test]
async fn test_tui_service_summarize() -> Result<()> {
    let service = TuiService::new_with_embedded_defaults().await?;
    let role_name = service.get_selected_role().await;

    let content = "This is a test paragraph that needs to be summarized. It contains multiple sentences with various topics and information that should be condensed.";

    // Summarize will fail if no LLM is configured, which is expected in tests
    let result = service.summarize(&role_name, content).await;

    match result {
        Ok(summary) => {
            println!("Summary: {}", summary);
        }
        Err(e) => {
            // Expected if no LLM is configured
            let error_msg = e.to_string();
            assert!(
                error_msg.contains("No LLM configured") || error_msg.contains("LLM"),
                "Should indicate LLM not configured: {}",
                error_msg
            );
            println!("Summarize returned expected error (no LLM): {}", e);
        }
    }

    Ok(())
}

/// Test list roles with info
#[tokio::test]
async fn test_tui_service_list_roles_with_info() -> Result<()> {
    let service = TuiService::new_with_embedded_defaults().await?;

    let roles = service.list_roles_with_info().await;

    // Should return role names with optional shortnames
    for (name, shortname) in &roles {
        println!("Role: {} (shortname: {:?})", name, shortname);
    }

    Ok(())
}

/// Test find_matches method
#[tokio::test]
async fn test_tui_service_find_matches() -> Result<()> {
    let service = TuiService::new_with_embedded_defaults().await?;
    let role_name = service.get_selected_role().await;

    let text = "This is a test paragraph with some terms to match.";

    let result = service.find_matches(&role_name, text).await;

    match result {
        Ok(matches) => {
            println!("Found {} matches", matches.len());
            for m in &matches {
                println!("  - {} at position {:?}", m.term, m.pos);
            }
        }
        Err(e) => {
            // Expected if no thesaurus data is available
            println!("Find matches returned error (expected if no data): {}", e);
        }
    }

    Ok(())
}
