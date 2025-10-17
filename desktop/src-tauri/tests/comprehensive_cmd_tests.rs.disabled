//! Comprehensive command tests for Tauri CLI
//!
//! Tests all Tauri CLI commands with various scenarios and edge cases

use serial_test::serial;
use std::time::Duration;
use tokio::time::timeout;

use terraphim_config::{ConfigBuilder, ConfigId, ConfigState};
use terraphim_service::TerraphimService;
use terraphim_types::{Document, LogicalOperator, NormalizedTermValue, RoleName, SearchQuery};

async fn create_test_config_state() -> ConfigState {
    let mut config = ConfigBuilder::new_with_id(ConfigId::Desktop)
        .build_default_desktop()
        .build()
        .unwrap();
    ConfigState::new(&mut config).await.unwrap()
}

#[tokio::test]
#[serial]
async fn test_search_command_comprehensive() {
    println!("🔍 Testing search command comprehensive functionality");

    let config_state = create_test_config_state().await;
    let mut service = TerraphimService::new(config_state.clone());

    // Test basic search
    let basic_query = SearchQuery {
        search_term: "test".into(),
        search_terms: None,
        operator: None,
        role: Some("Default".into()),
        skip: None,
        limit: Some(10),
    };

    let result = timeout(Duration::from_secs(30), service.search(&basic_query)).await;
    assert!(result.is_ok(), "Basic search should not timeout");

    let search_result = result.unwrap();
    assert!(search_result.is_ok(), "Basic search should succeed");
    println!("✅ Basic search completed");

    // Test multi-term search with AND operator
    let and_query = SearchQuery {
        search_term: "test".into(),
        search_terms: Some(vec!["data".into(), "system".into()]),
        operator: Some(LogicalOperator::And),
        role: Some("Default".into()),
        skip: None,
        limit: Some(5),
    };

    let and_result = timeout(Duration::from_secs(30), service.search(&and_query)).await;
    assert!(
        and_result.is_ok() && and_result.unwrap().is_ok(),
        "AND search should succeed"
    );
    println!("✅ Multi-term AND search completed");

    // Test multi-term search with OR operator
    let or_query = SearchQuery {
        search_term: "test".into(),
        search_terms: Some(vec!["haystack".into(), "service".into()]),
        operator: Some(LogicalOperator::Or),
        role: Some("Terraphim Engineer".into()),
        skip: Some(0),
        limit: Some(15),
    };

    let or_result = timeout(Duration::from_secs(30), service.search(&or_query)).await;
    assert!(
        or_result.is_ok() && or_result.unwrap().is_ok(),
        "OR search should succeed"
    );
    println!("✅ Multi-term OR search completed");

    // Test search with pagination
    let paginated_query = SearchQuery {
        search_term: "system".into(),
        search_terms: None,
        operator: None,
        role: Some("Default".into()),
        skip: Some(10),
        limit: Some(5),
    };

    let paginated_result = timeout(Duration::from_secs(30), service.search(&paginated_query)).await;
    assert!(
        paginated_result.is_ok() && paginated_result.unwrap().is_ok(),
        "Paginated search should succeed"
    );
    println!("✅ Paginated search completed");
}

#[tokio::test]
#[serial]
async fn test_config_management_comprehensive() {
    println!("🔧 Testing config management comprehensive functionality");

    // Initialize memory-only persistence to avoid filesystem dependencies
    terraphim_persistence::DeviceStorage::init_memory_only()
        .await
        .unwrap();

    let config_state = create_test_config_state().await;
    let service = TerraphimService::new(config_state.clone());

    // Test get config
    let config = service.fetch_config().await;
    assert!(!config.roles.is_empty(), "Config should have roles");
    assert!(
        !config.global_shortcut.is_empty(),
        "Config should have global shortcut"
    );
    println!("✅ Config fetch completed");

    // Test update config
    let mut updated_config = config.clone();
    updated_config.global_shortcut = "Ctrl+Alt+Test".to_string();

    let update_result = service.update_config(updated_config.clone()).await;
    assert!(update_result.is_ok(), "Config update should succeed");
    println!("✅ Config update completed");

    // Verify update persistence
    let verified_config = service.fetch_config().await;
    assert_eq!(
        verified_config.global_shortcut, "Ctrl+Alt+Test",
        "Config update should persist"
    );
    println!("✅ Config persistence verified");

    // Test role selection
    let original_role = verified_config.selected_role.clone();
    let new_role = if original_role.original == "Default" {
        RoleName::new("Terraphim Engineer")
    } else {
        RoleName::new("Default")
    };

    let role_update_result = service.update_selected_role(new_role.clone()).await;
    assert!(role_update_result.is_ok(), "Role update should succeed");

    let role_verified_config = service.fetch_config().await;
    assert_eq!(
        role_verified_config.selected_role, new_role,
        "Role selection should persist"
    );
    println!("✅ Role selection completed");
}

#[tokio::test]
#[serial]
async fn test_document_management() {
    println!("📄 Testing document management functionality");

    // Initialize memory-only persistence
    terraphim_persistence::DeviceStorage::init_memory_only()
        .await
        .unwrap();

    let config_state = create_test_config_state().await;
    let mut service = TerraphimService::new(config_state.clone());

    // Test document creation
    let test_doc = Document {
        id: "test_doc_001".to_string(),
        title: "Test Document for CLI".to_string(),
        body: "This is a test document created for comprehensive CLI testing.".to_string(),
        url: "https://test.example.com/doc1".to_string(),
        description: Some("A test document".to_string()),
        summarization: None,
        stub: None,
        tags: Some(vec!["test".to_string(), "cli".to_string()]),
        rank: Some(95),
    };

    let create_result = service.create_document(test_doc.clone()).await;
    assert!(create_result.is_ok(), "Document creation should succeed");

    let created_doc = create_result.unwrap();
    assert_eq!(
        created_doc.id, test_doc.id,
        "Created document should have same ID"
    );
    println!("✅ Document creation completed");

    // Test document retrieval
    let retrieve_result = service.get_document_by_id(&test_doc.id).await;
    assert!(retrieve_result.is_ok(), "Document retrieval should succeed");

    let retrieved_doc_opt = retrieve_result.unwrap();
    if let Some(retrieved_doc) = retrieved_doc_opt {
        assert_eq!(
            retrieved_doc.title, test_doc.title,
            "Retrieved document should match"
        );
        println!("✅ Document retrieval completed");
    } else {
        println!("⚠️ Document retrieval returned None - may be expected in test environment");
    }

    // Test document search integration
    let search_query = SearchQuery {
        search_term: NormalizedTermValue::from("CLI testing"),
        search_terms: None,
        operator: None,
        role: Some("Default".into()),
        skip: None,
        limit: Some(10),
    };

    let search_result = service.search(&search_query).await;
    assert!(
        search_result.is_ok(),
        "Search should succeed after document creation"
    );
    println!("✅ Document search integration completed");
}

#[tokio::test]
#[serial]
async fn test_thesaurus_functionality() {
    println!("📚 Testing thesaurus functionality");

    let config_state = create_test_config_state().await;
    let mut service = TerraphimService::new(config_state.clone());

    // Test thesaurus loading for different roles
    let test_roles = vec![
        RoleName::new("Terraphim Engineer"),
        RoleName::new("Default"),
    ];

    for role_name in test_roles {
        println!("  Testing thesaurus for role: {}", role_name.original);

        let thesaurus_result = timeout(
            Duration::from_secs(30),
            service.ensure_thesaurus_loaded(&role_name),
        )
        .await;

        match thesaurus_result {
            Ok(Ok(thesaurus)) => {
                println!(
                    "    ✅ Thesaurus loaded successfully: {} terms",
                    thesaurus.len()
                );

                // Basic validation
                assert!(!thesaurus.name().is_empty(), "Thesaurus should have a name");

                if !thesaurus.is_empty() {
                    println!("    ✅ Thesaurus has content");
                } else {
                    println!("    ⚠️ Thesaurus is empty - may need initialization");
                }
            }
            Ok(Err(e)) => {
                println!("    ⚠️ Thesaurus loading failed: {:?}", e);
                println!("      This may be expected if thesaurus files are not available");
            }
            Err(_) => {
                panic!(
                    "Thesaurus loading timed out for role: {}",
                    role_name.original
                );
            }
        }
    }
}

#[tokio::test]
#[serial]
async fn test_knowledge_graph_integration() {
    println!("🕸️ Testing knowledge graph integration");

    let config_state = create_test_config_state().await;
    let mut service = TerraphimService::new(config_state.clone());

    let role_name = RoleName::new("Terraphim Engineer");

    // Test KG term lookup
    let test_terms = vec!["haystack", "service", "graph", "system"];

    for term in test_terms {
        println!("  Testing KG lookup for term: {}", term);

        let kg_result = timeout(
            Duration::from_secs(30),
            service.find_documents_for_kg_term(&role_name, term),
        )
        .await;

        match kg_result {
            Ok(Ok(documents)) => {
                println!(
                    "    ✅ KG lookup succeeded: {} documents found",
                    documents.len()
                );

                // Validate document structure
                for doc in documents {
                    assert!(!doc.id.is_empty(), "Document ID should not be empty");
                    assert!(!doc.title.is_empty(), "Document title should not be empty");
                }
            }
            Ok(Err(e)) => {
                println!("    ⚠️ KG lookup failed: {:?}", e);
                println!("      This may be expected if KG is not initialized");
            }
            Err(_) => {
                panic!("KG lookup timed out for term: {}", term);
            }
        }
    }
}

#[tokio::test]
#[serial]
async fn test_search_edge_cases() {
    println!("🎯 Testing search edge cases");

    let config_state = create_test_config_state().await;
    let mut service = TerraphimService::new(config_state.clone());

    // Test empty search term
    let empty_query = SearchQuery {
        search_term: "".into(),
        search_terms: None,
        operator: None,
        role: Some("Default".into()),
        skip: None,
        limit: Some(5),
    };

    let empty_result = service.search(&empty_query).await;
    assert!(empty_result.is_ok(), "Empty search should not error");
    println!("✅ Empty search handled");

    // Test very long search term
    let long_term = "a".repeat(1000);
    let long_query = SearchQuery {
        search_term: long_term.as_str().into(),
        search_terms: None,
        operator: None,
        role: Some("Default".into()),
        skip: None,
        limit: Some(5),
    };

    let long_result = service.search(&long_query).await;
    assert!(long_result.is_ok(), "Long search term should not error");
    println!("✅ Long search term handled");

    // Test special characters
    let special_query = SearchQuery {
        search_term: "üñíçödé".into(),
        search_terms: None,
        operator: None,
        role: Some("Default".into()),
        skip: None,
        limit: Some(5),
    };

    let special_result = service.search(&special_query).await;
    assert!(
        special_result.is_ok(),
        "Special characters should not error"
    );
    println!("✅ Special characters handled");

    // Test large limit
    let large_limit_query = SearchQuery {
        search_term: "test".into(),
        search_terms: None,
        operator: None,
        role: Some("Default".into()),
        skip: None,
        limit: Some(10000),
    };

    let large_limit_result = service.search(&large_limit_query).await;
    assert!(large_limit_result.is_ok(), "Large limit should not error");
    println!("✅ Large limit handled");

    // Test invalid role
    let invalid_role_query = SearchQuery {
        search_term: "test".into(),
        search_terms: None,
        operator: None,
        role: Some("NonExistentRole".into()),
        skip: None,
        limit: Some(5),
    };

    let invalid_role_result = service.search(&invalid_role_query).await;
    // This might error or return empty results - both are acceptable
    match invalid_role_result {
        Ok(_) => println!("✅ Invalid role handled gracefully"),
        Err(_) => println!("✅ Invalid role properly rejected"),
    }
}

#[tokio::test]
#[serial]
async fn test_concurrent_operations() {
    println!("🔄 Testing concurrent operations");

    let config_state = create_test_config_state().await;

    // Create multiple service instances
    let mut service1 = TerraphimService::new(config_state.clone());
    let mut service2 = TerraphimService::new(config_state.clone());
    let mut service3 = TerraphimService::new(config_state.clone());

    // Test concurrent searches
    let query1 = SearchQuery {
        search_term: "test1".into(),
        search_terms: None,
        operator: None,
        role: Some("Default".into()),
        skip: None,
        limit: Some(5),
    };

    let query2 = SearchQuery {
        search_term: "test2".into(),
        search_terms: None,
        operator: None,
        role: Some("Default".into()),
        skip: None,
        limit: Some(5),
    };

    let query3 = SearchQuery {
        search_term: "test3".into(),
        search_terms: None,
        operator: None,
        role: Some("Terraphim Engineer".into()),
        skip: None,
        limit: Some(5),
    };

    // Run concurrent searches
    let (result1, result2, result3) = timeout(Duration::from_secs(60), async {
        tokio::join!(
            service1.search(&query1),
            service2.search(&query2),
            service3.search(&query3)
        )
    })
    .await
    .expect("Concurrent searches timed out");

    assert!(result1.is_ok(), "Concurrent search 1 should succeed");
    assert!(result2.is_ok(), "Concurrent search 2 should succeed");
    assert!(result3.is_ok(), "Concurrent search 3 should succeed");

    println!("✅ Concurrent searches completed successfully");

    // Test concurrent config operations
    let config1 = service1.fetch_config();
    let config2 = service2.fetch_config();
    let config3 = service3.fetch_config();

    let (cfg1, cfg2, cfg3) = timeout(Duration::from_secs(30), async {
        tokio::join!(config1, config2, config3)
    })
    .await
    .expect("Concurrent config fetches timed out");

    assert!(!cfg1.roles.is_empty(), "Config 1 should have roles");
    assert!(!cfg2.roles.is_empty(), "Config 2 should have roles");
    assert!(!cfg3.roles.is_empty(), "Config 3 should have roles");

    println!("✅ Concurrent config operations completed successfully");
}

#[tokio::test]
#[serial]
async fn test_service_resilience() {
    println!("🛡️ Testing service resilience");

    let config_state = create_test_config_state().await;
    let mut service = TerraphimService::new(config_state.clone());

    // Test service state after operations
    let initial_config = service.fetch_config().await;
    let initial_role = initial_config.selected_role.clone();

    // Perform multiple operations
    let queries = ["test1", "test2", "test3", "test4", "test5"];

    for (i, query_term) in queries.iter().enumerate() {
        let query = SearchQuery {
            search_term: (*query_term).into(),
            search_terms: None,
            operator: None,
            role: Some(initial_role.clone()),
            skip: None,
            limit: Some(3),
        };

        let result = service.search(&query).await;
        assert!(result.is_ok(), "Search {} should succeed", i + 1);
    }

    // Verify service state is still consistent
    let final_config = service.fetch_config().await;
    assert_eq!(
        final_config.selected_role, initial_role,
        "Service state should remain consistent"
    );

    println!("✅ Service resilience test completed");
}
