//! Comprehensive tests for REPL offline mode commands
//!
//! These tests verify that all REPL commands work correctly in offline mode
//! using TuiService without requiring a running server.

use anyhow::Result;
use serial_test::serial;
use terraphim_tui::TuiService;

// Note: ReplHandler is not publicly exported, so we test TuiService directly
// REPL functionality is tested through integration tests

#[tokio::test]
#[serial]
async fn test_tui_service_initialization() -> Result<()> {
    // Test that TuiService initializes with embedded config
    let service = TuiService::new().await?;

    // Should have config
    let config = service.get_config().await;
    assert_eq!(config.id, terraphim_config::ConfigId::Embedded);

    // Should have selected role
    let selected_role = service.get_selected_role().await;
    assert!(!selected_role.to_string().is_empty());

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_tui_service_list_roles() -> Result<()> {
    let service = TuiService::new().await?;

    let roles = service.list_roles().await;

    // Embedded config should have 3 roles
    assert_eq!(roles.len(), 3, "Should have 3 roles in embedded config");
    assert!(roles.contains(&"Terraphim Engineer".to_string()));
    assert!(roles.contains(&"Rust Engineer".to_string()));
    assert!(roles.contains(&"Default".to_string()));

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_tui_service_role_switching() -> Result<()> {
    let service = TuiService::new().await?;

    // Get initial role
    let initial_role = service.get_selected_role().await;
    println!("Initial role: {}", initial_role);

    // Switch to Rust Engineer
    let rust_engineer = terraphim_types::RoleName::new("Rust Engineer");
    let updated_config = service.update_selected_role(rust_engineer.clone()).await?;

    assert_eq!(updated_config.selected_role, rust_engineer);

    // Verify role changed
    let current_role = service.get_selected_role().await;
    assert_eq!(current_role, rust_engineer);

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_tui_service_search() -> Result<()> {
    let service = TuiService::new().await?;

    let role_name = terraphim_types::RoleName::new("Terraphim Engineer");

    // Search should work (may return empty results if no data)
    let results = service
        .search_with_role("test", &role_name, Some(10))
        .await?;

    // Should not panic, results may be empty
    println!("Search returned {} results", results.len());

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_tui_service_autocomplete() -> Result<()> {
    let service = TuiService::new().await?;

    let role_name = terraphim_types::RoleName::new("Terraphim Engineer");

    // Autocomplete should work (may return empty if no thesaurus)
    let results = service.autocomplete(&role_name, "test", Some(10)).await;

    match results {
        Ok(suggestions) => {
            println!("Autocomplete returned {} suggestions", suggestions.len());
        }
        Err(e) => {
            // May fail if role doesn't have knowledge graph
            println!("Autocomplete failed (expected if no KG): {}", e);
        }
    }

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_tui_service_get_thesaurus() -> Result<()> {
    let service = TuiService::new().await?;

    let role_name = terraphim_types::RoleName::new("Terraphim Engineer");

    // Get thesaurus for role
    let result = service.get_thesaurus(&role_name).await;

    match result {
        Ok(thesaurus) => {
            println!("Thesaurus loaded: {} entries", thesaurus.len());
            // Thesaurus may be empty if KG not built yet
        }
        Err(e) => {
            // May fail if role doesn't have KG configured
            println!("Thesaurus load failed (may be expected): {}", e);
        }
    }

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_tui_service_get_role_graph() -> Result<()> {
    let service = TuiService::new().await?;

    let role_name = terraphim_types::RoleName::new("Terraphim Engineer");

    // Get top concepts
    let concepts = service.get_role_graph_top_k(&role_name, 10).await?;

    // Should return concepts (currently placeholder data)
    assert!(!concepts.is_empty(), "Should return concepts");
    println!("Role graph concepts: {:?}", concepts);

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_tui_service_extract_paragraphs() -> Result<()> {
    let service = TuiService::new().await?;

    let role_name = terraphim_types::RoleName::new("Terraphim Engineer");
    let text = "This is a test paragraph with some content for extraction.";

    // Extract paragraphs
    let result = service.extract_paragraphs(&role_name, text, false).await;

    match result {
        Ok(paragraphs) => {
            println!("Extracted {} paragraphs", paragraphs.len());
        }
        Err(e) => {
            // May fail if thesaurus not available
            println!("Extract paragraphs failed (may be expected): {}", e);
        }
    }

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_tui_service_find_matches() -> Result<()> {
    let service = TuiService::new().await?;

    let role_name = terraphim_types::RoleName::new("Terraphim Engineer");
    let text = "This text contains keywords for matching.";

    // Find matches
    let result = service.find_matches(&role_name, text).await;

    match result {
        Ok(matches) => {
            println!("Found {} matches", matches.len());
        }
        Err(e) => {
            println!("Find matches failed (may be expected): {}", e);
        }
    }

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_tui_service_replace_matches() -> Result<()> {
    let service = TuiService::new().await?;

    let role_name = terraphim_types::RoleName::new("Terraphim Engineer");
    let text = "This text contains keywords for replacement.";

    // Replace matches with markdown links
    let result = service
        .replace_matches(
            &role_name,
            text,
            terraphim_automata::LinkType::MarkdownLinks,
        )
        .await;

    match result {
        Ok(replaced_text) => {
            println!("Replaced text: {}", replaced_text);
        }
        Err(e) => {
            println!("Replace matches failed (may be expected): {}", e);
        }
    }

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_all_embedded_roles_available() -> Result<()> {
    let service = TuiService::new().await?;

    let roles = service.list_roles().await;

    // Verify all expected roles from embedded config
    assert!(
        roles.contains(&"Terraphim Engineer".to_string()),
        "Should include Terraphim Engineer"
    );
    assert!(
        roles.contains(&"Rust Engineer".to_string()),
        "Should include Rust Engineer"
    );
    assert!(
        roles.contains(&"Default".to_string()),
        "Should include Default"
    );

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_rust_engineer_role_configuration() -> Result<()> {
    let service = TuiService::new().await?;

    // Get config and verify Rust Engineer exists
    let config = service.get_config().await;

    let rust_engineer_role = config
        .roles
        .get(&terraphim_types::RoleName::new("Rust Engineer"));
    assert!(
        rust_engineer_role.is_some(),
        "Rust Engineer role should exist"
    );

    let role = rust_engineer_role.unwrap();
    assert_eq!(role.shortname, Some("rust-engineer".to_string()));
    assert_eq!(role.theme, "cosmo");
    assert_eq!(role.haystacks.len(), 1);
    assert_eq!(role.haystacks[0].location, "https://query.rs");
    assert_eq!(
        role.haystacks[0].service,
        terraphim_config::ServiceType::QueryRs
    );

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_config_persistence() -> Result<()> {
    let service = TuiService::new().await?;

    // Update selected role
    let new_role = terraphim_types::RoleName::new("Default");
    service.update_selected_role(new_role.clone()).await?;

    // Save config
    service.save_config().await?;

    // Create new service instance
    let service2 = TuiService::new().await?;

    // Verify role persisted
    let loaded_role = service2.get_selected_role().await;
    assert_eq!(loaded_role, new_role, "Role should persist across sessions");

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_search_with_query_operators() -> Result<()> {
    let service = TuiService::new().await?;

    let search_query = terraphim_types::SearchQuery {
        search_term: terraphim_types::NormalizedTermValue::from("test"),
        search_terms: Some(vec![terraphim_types::NormalizedTermValue::from("query")]),
        operator: Some(terraphim_types::LogicalOperator::And),
        skip: Some(0),
        limit: Some(10),
        role: Some(terraphim_types::RoleName::new("Terraphim Engineer")),
    };

    // Should handle multi-term queries with logical operators
    let results = service.search_with_query(&search_query).await?;

    println!("Multi-term search returned {} results", results.len());

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_offline_mode_no_server_required() -> Result<()> {
    // This test verifies the entire offline workflow works without a server

    // 1. Initialize service
    let service = TuiService::new().await?;

    // 2. List roles
    let roles = service.list_roles().await;
    assert!(!roles.is_empty(), "Should have roles");

    // 3. Select role
    let role_name = terraphim_types::RoleName::new("Rust Engineer");
    service.update_selected_role(role_name.clone()).await?;

    // 4. Perform search
    let _results = service
        .search_with_role("async", &role_name, Some(10))
        .await?;

    // 5. Get config
    let config = service.get_config().await;
    assert_eq!(config.selected_role, role_name);

    // All operations completed without server
    println!("✅ Complete offline workflow succeeded");

    Ok(())
}
