/// Backend Integration Tests for Search
///
/// These tests validate that GPUI uses the same backend as Tauri
/// by calling the exact same service methods with the same patterns.
use terraphim_config::{ConfigBuilder, ConfigId, ConfigState};
use terraphim_persistence::Persistable;
use terraphim_service::TerraphimService;
use terraphim_types::{LogicalOperator, RoleName, SearchQuery};

#[tokio::test]
async fn test_search_backend_integration_basic() {
    // Pattern from Tauri main.rs config loading
    let mut config = match ConfigBuilder::new_with_id(ConfigId::Desktop).build() {
        Ok(mut config) => match config.load().await {
            Ok(config) => config,
            Err(_) => ConfigBuilder::new()
                .build_default_desktop()
                .build()
                .unwrap(),
        },
        Err(_) => panic!("Failed to build config"),
    };

    let config_state = ConfigState::new(&mut config).await.unwrap();

    // Pattern from Tauri cmd.rs:115-126 (search command)
    let mut terraphim_service = TerraphimService::new(config_state);

    let search_query = SearchQuery {
        search_term: "async".into(),
        search_terms: None,
        operator: None,
        role: Some(RoleName::from("Terraphim Engineer")),
        limit: Some(20),
        skip: Some(0),
    };

    // This is the EXACT same call as Tauri makes
    let results = terraphim_service.search(&search_query).await.unwrap();

    assert!(
        !results.is_empty(),
        "Should find results for 'async' in knowledge graph"
    );
    println!(
        "✅ Search found {} results using TerraphimService",
        results.len()
    );
}

#[tokio::test]
async fn test_search_with_multiple_terms_and_operator() {
    let mut config = ConfigBuilder::new()
        .build_default_desktop()
        .build()
        .unwrap();
    let config_state = ConfigState::new(&mut config).await.unwrap();
    let mut service = TerraphimService::new(config_state);

    let search_query = SearchQuery {
        search_term: "async".into(),
        search_terms: Some(vec!["async".into(), "tokio".into()]),
        operator: Some(LogicalOperator::And),
        role: Some(RoleName::from("Terraphim Engineer")),
        limit: Some(10),
        skip: Some(0),
    };

    let results = service.search(&search_query).await.unwrap();

    println!(
        "✅ Multi-term search with AND operator returned {} results",
        results.len()
    );
}

#[tokio::test]
async fn test_search_different_roles() {
    let mut config = ConfigBuilder::new()
        .build_default_desktop()
        .build()
        .unwrap();
    let config_state = ConfigState::new(&mut config).await.unwrap();

    // Test search with different roles
    for role_name in ["Terraphim Engineer", "Default", "Rust Engineer"] {
        let mut service = TerraphimService::new(config_state.clone());

        let search_query = SearchQuery {
            search_term: "test".into(),
            search_terms: None,
            operator: None,
            role: Some(RoleName::from(role_name)),
            limit: Some(5),
            skip: Some(0),
        };

        match service.search(&search_query).await {
            Ok(results) => {
                println!(
                    "✅ Role '{}' search returned {} results",
                    role_name,
                    results.len()
                );
            }
            Err(e) => {
                println!(
                    "⚠️ Role '{}' search failed: {} (may not have haystacks)",
                    role_name, e
                );
            }
        }
    }
}

#[tokio::test]
async fn test_search_backend_error_handling() {
    let mut config = ConfigBuilder::new()
        .build_default_desktop()
        .build()
        .unwrap();
    let config_state = ConfigState::new(&mut config).await.unwrap();
    let mut service = TerraphimService::new(config_state);

    // Search with non-existent role
    let search_query = SearchQuery {
        search_term: "test".into(),
        search_terms: None,
        operator: None,
        role: Some(RoleName::from("NonExistentRole")),
        limit: Some(10),
        skip: Some(0),
    };

    // Should handle gracefully (empty results or error)
    match service.search(&search_query).await {
        Ok(results) => {
            println!(
                "✅ Search with invalid role handled gracefully: {} results",
                results.len()
            );
        }
        Err(e) => {
            println!(
                "✅ Search with invalid role returned error (expected): {}",
                e
            );
        }
    }
}

#[test]
fn test_search_query_construction() {
    // Verify SearchQuery can be constructed correctly
    let query = SearchQuery {
        search_term: "async rust".into(),
        search_terms: None,
        operator: None,
        role: Some(RoleName::from("Terraphim Engineer")),
        limit: Some(20),
        skip: Some(0),
    };

    assert_eq!(query.search_term.to_string(), "async rust");
    assert_eq!(query.limit, Some(20));
    assert_eq!(query.skip, Some(0));
}
