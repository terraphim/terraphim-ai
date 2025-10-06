use std::path::PathBuf;
use terraphim_automata::AutomataPath;
use terraphim_config::{KnowledgeGraph, KnowledgeGraphLocal, Role};
use terraphim_service::{ConfigState, TerraphimService};
use terraphim_types::{Document, KnowledgeGraphInputType, RoleName};

/// Test to verify that KG preprocessing happens exactly once per document-role combination
/// This is a comprehensive test that focuses on the HashMap tracking functionality
#[tokio::test]
async fn test_kg_preprocessing_prevention_with_hashmap() {
    // Initialize logging for test visibility
    let _ = env_logger::builder()
        .filter_level(log::LevelFilter::Debug)
        .is_test(true)
        .try_init();

    // Create a test document
    let test_document = Document {
        id: "test-document-id".to_string(),
        title: "Test Document".to_string(),
        body: "This is a test document body for KG preprocessing testing.".to_string(),
        description: Some("Test document".to_string()),
        ..Default::default()
    };

    // Create a simple role with terraphim_it enabled
    let test_role = Role {
        name: RoleName::new("Test Role"),
        terraphim_it: true,
        kg: Some(KnowledgeGraph {
            automata_path: Some(AutomataPath::Remote("test://kg".to_string())),
            knowledge_graph_local: Some(KnowledgeGraphLocal {
                input_type: KnowledgeGraphInputType::Markdown,
                path: PathBuf::from("test/kg"),
            }),
            public: false,
            publish: false,
        }),
        ..Default::default()
    };

    // Create configuration
    let mut config = terraphim_config::Config::default();
    config
        .roles
        .insert(test_role.name.clone(), test_role.clone());
    config.selected_role = test_role.name.clone();

    // Create service
    let config_state = ConfigState::new(&mut config)
        .await
        .expect("Failed to create config state");
    let mut service = TerraphimService::new(config_state);

    println!("🧪 Testing KG preprocessing HashMap tracking...");

    // Test 1: Check that processed documents map is initially empty
    assert_eq!(
        service.processed_documents.len(),
        0,
        "Initial processed documents map should be empty"
    );

    // Test 2: First processing attempt - should update the HashMap
    println!("🔄 First processing attempt...");
    let result1 = service
        .apply_kg_preprocessing_if_needed(test_document.clone())
        .await;
    assert!(result1.is_ok(), "First processing should succeed");

    // Check that the document was added to processed map
    // The key format should be "document_id::role_name"
    let expected_key = format!("{}::{}", test_document.id, test_role.name);
    assert!(
        service.processed_documents.contains_key(&expected_key),
        "Document should be added to processed map after first processing"
    );
    assert_eq!(
        service.processed_documents.len(),
        1,
        "Should have exactly one processed document"
    );

    // Test 3: Second processing attempt - should be skipped
    println!("🔄 Second processing attempt (should be skipped)...");
    let result2 = service
        .apply_kg_preprocessing_if_needed(test_document.clone())
        .await;
    assert!(
        result2.is_ok(),
        "Second processing should succeed but be skipped"
    );

    // HashMap should still have only one entry (not duplicated)
    assert_eq!(
        service.processed_documents.len(),
        1,
        "Should still have exactly one processed document"
    );
    assert!(
        service.processed_documents.contains_key(&expected_key),
        "Same document key should still exist"
    );

    // Test 4: Different document should be processed separately
    let different_document = Document {
        id: "different-document-id".to_string(),
        title: "Different Document".to_string(),
        body: "This is a different test document.".to_string(),
        ..Default::default()
    };

    println!("🔄 Processing different document...");
    let result3 = service
        .apply_kg_preprocessing_if_needed(different_document.clone())
        .await;
    assert!(
        result3.is_ok(),
        "Different document processing should succeed"
    );

    // Should now have two entries in the HashMap
    let different_key = format!("{}::{}", different_document.id, test_role.name);
    assert_eq!(
        service.processed_documents.len(),
        2,
        "Should have two processed documents"
    );
    assert!(
        service.processed_documents.contains_key(&different_key),
        "Different document should be added to processed map"
    );

    // Test 5: Clear cache and verify reprocessing
    println!("🧹 Testing cache clearing...");
    service.clear_processed_documents_cache();
    assert_eq!(
        service.processed_documents.len(),
        0,
        "Cache should be empty after clearing"
    );

    // Reprocess original document - should work again
    let result4 = service
        .apply_kg_preprocessing_if_needed(test_document.clone())
        .await;
    assert!(
        result4.is_ok(),
        "Processing after cache clear should succeed"
    );
    assert_eq!(
        service.processed_documents.len(),
        1,
        "Should have one document after reprocessing"
    );

    println!("✅ All HashMap tracking tests passed!");
    println!("✅ Documents are tracked correctly to prevent duplicates");
    println!("✅ Cache clearing enables reprocessing when needed");
}

/// Test to verify that documents with terraphim_it disabled are not tracked
#[tokio::test]
async fn test_disabled_terraphim_it_not_tracked() {
    let _ = env_logger::builder()
        .filter_level(log::LevelFilter::Debug)
        .is_test(true)
        .try_init();

    let test_document = Document {
        id: "untracked-doc".to_string(),
        title: "Untracked Document".to_string(),
        body: "This document should not be tracked.".to_string(),
        ..Default::default()
    };

    // Create role with terraphim_it disabled
    let disabled_role = Role {
        name: RoleName::new("Disabled Role"),
        terraphim_it: false, // Disabled
        kg: None,
        ..Default::default()
    };

    let mut config = terraphim_config::Config::default();
    config
        .roles
        .insert(disabled_role.name.clone(), disabled_role.clone());
    config.selected_role = disabled_role.name.clone();

    let config_state = ConfigState::new(&mut config)
        .await
        .expect("Failed to create config state");
    let mut service = TerraphimService::new(config_state);

    println!("🧪 Testing disabled terraphim_it behavior...");

    // Process document with disabled role
    let result = service
        .apply_kg_preprocessing_if_needed(test_document.clone())
        .await;
    assert!(
        result.is_ok(),
        "Processing with disabled role should succeed"
    );

    // HashMap should remain empty because terraphim_it is disabled
    assert_eq!(
        service.processed_documents.len(),
        0,
        "Disabled role should not add documents to processed map"
    );

    println!("✅ Disabled terraphim_it correctly bypasses tracking");
}

/// Test to verify that existing KG links prevent processing and tracking
#[tokio::test]
async fn test_existing_kg_links_not_tracked() {
    let _ = env_logger::builder()
        .filter_level(log::LevelFilter::Debug)
        .is_test(true)
        .try_init();

    // Document with existing KG links
    let doc_with_links = Document {
        id: "linked-doc".to_string(),
        title: "Document with KG Links".to_string(),
        body: "This document already has [some link](kg:test) in it.".to_string(),
        ..Default::default()
    };

    let test_role = Role {
        name: RoleName::new("Test Role"),
        terraphim_it: true,
        kg: Some(KnowledgeGraph {
            automata_path: Some(AutomataPath::Remote("test://kg".to_string())),
            knowledge_graph_local: Some(KnowledgeGraphLocal {
                input_type: KnowledgeGraphInputType::Markdown,
                path: PathBuf::from("test/kg"),
            }),
            public: false,
            publish: false,
        }),
        ..Default::default()
    };

    let mut config = terraphim_config::Config::default();
    config
        .roles
        .insert(test_role.name.clone(), test_role.clone());
    config.selected_role = test_role.name.clone();

    let config_state = ConfigState::new(&mut config)
        .await
        .expect("Failed to create config state");
    let mut service = TerraphimService::new(config_state);

    println!("🧪 Testing existing KG links behavior...");

    // Process document with existing KG links
    let result = service
        .apply_kg_preprocessing_if_needed(doc_with_links.clone())
        .await;
    assert!(
        result.is_ok(),
        "Processing document with existing KG links should succeed"
    );

    // HashMap should remain empty because document already has KG links
    assert_eq!(
        service.processed_documents.len(),
        0,
        "Documents with existing KG links should not be tracked"
    );

    println!("✅ Existing KG links correctly prevent processing and tracking");
}

/// Comprehensive test for multiple roles with same document
#[tokio::test]
async fn test_multiple_roles_same_document() {
    let _ = env_logger::builder()
        .filter_level(log::LevelFilter::Debug)
        .is_test(true)
        .try_init();

    let test_document = Document {
        id: "multi-role-doc".to_string(),
        title: "Multi-Role Document".to_string(),
        body: "This document will be processed by multiple roles.".to_string(),
        ..Default::default()
    };

    // Create multiple roles
    let role1 = Role {
        name: RoleName::new("Role One"),
        terraphim_it: true,
        kg: Some(KnowledgeGraph {
            automata_path: Some(AutomataPath::Remote("test://kg1".to_string())),
            knowledge_graph_local: Some(KnowledgeGraphLocal {
                input_type: KnowledgeGraphInputType::Markdown,
                path: PathBuf::from("test/kg1"),
            }),
            public: false,
            publish: false,
        }),
        ..Default::default()
    };

    let role2 = Role {
        name: RoleName::new("Role Two"),
        terraphim_it: true,
        kg: Some(KnowledgeGraph {
            automata_path: Some(AutomataPath::Remote("test://kg2".to_string())),
            knowledge_graph_local: Some(KnowledgeGraphLocal {
                input_type: KnowledgeGraphInputType::Markdown,
                path: PathBuf::from("test/kg2"),
            }),
            public: false,
            publish: false,
        }),
        ..Default::default()
    };

    let mut config = terraphim_config::Config::default();
    config.roles.insert(role1.name.clone(), role1.clone());
    config.roles.insert(role2.name.clone(), role2.clone());
    config.selected_role = role1.name.clone();

    let config_state = ConfigState::new(&mut config)
        .await
        .expect("Failed to create config state");
    let mut service = TerraphimService::new(config_state);

    println!("🧪 Testing multiple roles with same document...");

    // Process with role1
    let result1 = service
        .apply_kg_preprocessing_if_needed(test_document.clone())
        .await;
    assert!(result1.is_ok(), "Processing with role1 should succeed");

    let key1 = format!("{}::{}", test_document.id, role1.name);
    assert!(
        service.processed_documents.contains_key(&key1),
        "Role1 should be tracked"
    );
    assert_eq!(
        service.processed_documents.len(),
        1,
        "Should have one tracked document"
    );

    // Switch to role2 and process same document
    {
        let mut config_guard = service.config_state.config.lock().await;
        config_guard.selected_role = role2.name.clone();
    }

    let result2 = service
        .apply_kg_preprocessing_if_needed(test_document.clone())
        .await;
    assert!(result2.is_ok(), "Processing with role2 should succeed");

    let key2 = format!("{}::{}", test_document.id, role2.name);
    assert!(
        service.processed_documents.contains_key(&key1),
        "Role1 should still be tracked"
    );
    assert!(
        service.processed_documents.contains_key(&key2),
        "Role2 should also be tracked"
    );
    assert_eq!(
        service.processed_documents.len(),
        2,
        "Should have two tracked documents"
    );

    // Try processing with role1 again - should be skipped
    {
        let mut config_guard = service.config_state.config.lock().await;
        config_guard.selected_role = role1.name.clone();
    }

    let result3 = service
        .apply_kg_preprocessing_if_needed(test_document.clone())
        .await;
    assert!(
        result3.is_ok(),
        "Duplicate processing with role1 should be skipped"
    );
    assert_eq!(
        service.processed_documents.len(),
        2,
        "Should still have two tracked documents"
    );

    println!("✅ Multiple roles correctly track same document separately");
}

/// Test edge cases with unusual document content
#[tokio::test]
async fn test_edge_cases() {
    let _ = env_logger::builder()
        .filter_level(log::LevelFilter::Debug)
        .is_test(true)
        .try_init();

    let test_role = Role {
        name: RoleName::new("Edge Case Role"),
        terraphim_it: true,
        kg: Some(KnowledgeGraph {
            automata_path: Some(AutomataPath::Remote("test://kg".to_string())),
            knowledge_graph_local: Some(KnowledgeGraphLocal {
                input_type: KnowledgeGraphInputType::Markdown,
                path: PathBuf::from("test/kg"),
            }),
            public: false,
            publish: false,
        }),
        ..Default::default()
    };

    let mut config = terraphim_config::Config::default();
    config
        .roles
        .insert(test_role.name.clone(), test_role.clone());
    config.selected_role = test_role.name.clone();

    let config_state = ConfigState::new(&mut config)
        .await
        .expect("Failed to create config state");
    let mut service = TerraphimService::new(config_state);

    println!("🧪 Testing edge cases...");

    // Test 1: Empty document
    let empty_doc = Document {
        id: "empty-doc".to_string(),
        title: "Empty Document".to_string(),
        body: "".to_string(),
        ..Default::default()
    };

    let result1 = service
        .apply_kg_preprocessing_if_needed(empty_doc.clone())
        .await;
    assert!(result1.is_ok(), "Empty document processing should succeed");

    let empty_key = format!("{}::{}", empty_doc.id, test_role.name);
    assert!(
        service.processed_documents.contains_key(&empty_key),
        "Empty document should be tracked"
    );

    // Test 2: Very long document (simulate large content)
    let long_content = "word ".repeat(10000); // 50KB of text
    let long_doc = Document {
        id: "long-doc".to_string(),
        title: "Long Document".to_string(),
        body: long_content,
        ..Default::default()
    };

    let result2 = service
        .apply_kg_preprocessing_if_needed(long_doc.clone())
        .await;
    assert!(result2.is_ok(), "Long document processing should succeed");

    let long_key = format!("{}::{}", long_doc.id, test_role.name);
    assert!(
        service.processed_documents.contains_key(&long_key),
        "Long document should be tracked"
    );

    // Test 3: Document with special characters in ID
    let special_doc = Document {
        id: "special-doc@#$%^&*()".to_string(),
        title: "Special Characters Document".to_string(),
        body: "Document with special characters in ID.".to_string(),
        ..Default::default()
    };

    let result3 = service
        .apply_kg_preprocessing_if_needed(special_doc.clone())
        .await;
    assert!(
        result3.is_ok(),
        "Special characters document processing should succeed"
    );

    let special_key = format!("{}::{}", special_doc.id, test_role.name);
    assert!(
        service.processed_documents.contains_key(&special_key),
        "Special characters document should be tracked"
    );

    // Test 4: Document with Unicode content
    let unicode_doc = Document {
        id: "unicode-doc".to_string(),
        title: "Unicode Document 中文测试 🚀".to_string(),
        body: "This document contains Unicode: 中文测试 日本語 العربية русский 🚀🎉💡".to_string(),
        ..Default::default()
    };

    let result4 = service
        .apply_kg_preprocessing_if_needed(unicode_doc.clone())
        .await;
    assert!(
        result4.is_ok(),
        "Unicode document processing should succeed"
    );

    let unicode_key = format!("{}::{}", unicode_doc.id, test_role.name);
    assert!(
        service.processed_documents.contains_key(&unicode_key),
        "Unicode document should be tracked"
    );

    assert_eq!(
        service.processed_documents.len(),
        4,
        "Should have all 4 edge case documents tracked"
    );

    println!("✅ All edge cases handled correctly");
}

/// Performance test with multiple documents processed rapidly
#[tokio::test]
async fn test_performance_multiple_documents() {
    let _ = env_logger::builder()
        .filter_level(log::LevelFilter::Info) // Reduce logging for performance test
        .is_test(true)
        .try_init();

    let test_role = Role {
        name: RoleName::new("Performance Role"),
        terraphim_it: true,
        kg: Some(KnowledgeGraph {
            automata_path: Some(AutomataPath::Remote("test://kg".to_string())),
            knowledge_graph_local: Some(KnowledgeGraphLocal {
                input_type: KnowledgeGraphInputType::Markdown,
                path: PathBuf::from("test/kg"),
            }),
            public: false,
            publish: false,
        }),
        ..Default::default()
    };

    let mut config = terraphim_config::Config::default();
    config
        .roles
        .insert(test_role.name.clone(), test_role.clone());
    config.selected_role = test_role.name.clone();

    let config_state = ConfigState::new(&mut config)
        .await
        .expect("Failed to create config state");
    let mut service = TerraphimService::new(config_state);

    println!("🧪 Testing performance with multiple documents...");

    let start_time = std::time::Instant::now();
    let num_documents = 100;

    // Create and process multiple documents
    for i in 0..num_documents {
        let doc = Document {
            id: format!("perf-doc-{}", i),
            title: format!("Performance Document {}", i),
            body: format!(
                "This is performance test document number {} with some content.",
                i
            ),
            ..Default::default()
        };

        let result = service.apply_kg_preprocessing_if_needed(doc).await;
        assert!(
            result.is_ok(),
            "Performance document {} should process successfully",
            i
        );
    }

    let processing_time = start_time.elapsed();
    println!(
        "⏱️  Processed {} documents in {:?}",
        num_documents, processing_time
    );

    // Verify all documents are tracked
    assert_eq!(
        service.processed_documents.len(),
        num_documents,
        "Should have tracked all {} documents",
        num_documents
    );

    // Test duplicate processing prevention - should be very fast
    let duplicate_start = std::time::Instant::now();

    for i in 0..num_documents {
        let doc = Document {
            id: format!("perf-doc-{}", i),
            title: format!("Performance Document {}", i),
            body: format!(
                "This is performance test document number {} with some content.",
                i
            ),
            ..Default::default()
        };

        let result = service.apply_kg_preprocessing_if_needed(doc).await;
        assert!(
            result.is_ok(),
            "Duplicate processing should succeed but be skipped"
        );
    }

    let duplicate_time = duplicate_start.elapsed();
    println!(
        "⏱️  Duplicate processing prevention took {:?}",
        duplicate_time
    );

    // Duplicate processing should be much faster than original processing
    assert!(
        duplicate_time < processing_time / 2,
        "Duplicate processing should be at least 2x faster due to caching"
    );

    // HashMap should still have the same number of entries
    assert_eq!(
        service.processed_documents.len(),
        num_documents,
        "Should still have exactly {} documents after duplicate processing",
        num_documents
    );

    println!(
        "✅ Performance test passed with {} documents",
        num_documents
    );
    println!("✅ Original processing: {:?}", processing_time);
    println!(
        "✅ Duplicate prevention: {:?} ({}x faster)",
        duplicate_time,
        processing_time.as_millis() / duplicate_time.as_millis().max(1)
    );
}

/// Test configuration changes during processing
#[tokio::test]
async fn test_configuration_changes() {
    let _ = env_logger::builder()
        .filter_level(log::LevelFilter::Debug)
        .is_test(true)
        .try_init();

    let role1 = Role {
        name: RoleName::new("Config Role 1"),
        terraphim_it: true,
        kg: Some(KnowledgeGraph {
            automata_path: Some(AutomataPath::Remote("test://kg1".to_string())),
            knowledge_graph_local: Some(KnowledgeGraphLocal {
                input_type: KnowledgeGraphInputType::Markdown,
                path: PathBuf::from("test/kg1"),
            }),
            public: false,
            publish: false,
        }),
        ..Default::default()
    };

    let mut config = terraphim_config::Config::default();
    config.roles.insert(role1.name.clone(), role1.clone());
    config.selected_role = role1.name.clone();

    let config_state = ConfigState::new(&mut config)
        .await
        .expect("Failed to create config state");
    let mut service = TerraphimService::new(config_state);

    let test_doc = Document {
        id: "config-test-doc".to_string(),
        title: "Configuration Test Document".to_string(),
        body: "This document tests configuration changes.".to_string(),
        ..Default::default()
    };

    println!("🧪 Testing configuration changes...");

    // Process with original role
    let result1 = service
        .apply_kg_preprocessing_if_needed(test_doc.clone())
        .await;
    assert!(result1.is_ok(), "Initial processing should succeed");
    assert_eq!(
        service.processed_documents.len(),
        1,
        "Should have one tracked document"
    );

    // Add a new role dynamically
    let role2 = Role {
        name: RoleName::new("Config Role 2"),
        terraphim_it: true,
        kg: Some(KnowledgeGraph {
            automata_path: Some(AutomataPath::Remote("test://kg2".to_string())),
            knowledge_graph_local: Some(KnowledgeGraphLocal {
                input_type: KnowledgeGraphInputType::Markdown,
                path: PathBuf::from("test/kg2"),
            }),
            public: false,
            publish: false,
        }),
        ..Default::default()
    };

    // Update configuration with new role
    {
        let mut config_guard = service.config_state.config.lock().await;
        config_guard.roles.insert(role2.name.clone(), role2.clone());
        config_guard.selected_role = role2.name.clone();
    }

    // Process same document with new role
    let result2 = service
        .apply_kg_preprocessing_if_needed(test_doc.clone())
        .await;
    assert!(result2.is_ok(), "Processing with new role should succeed");
    assert_eq!(
        service.processed_documents.len(),
        2,
        "Should now have two tracked documents"
    );

    // Disable terraphim_it on existing role and test
    {
        let mut config_guard = service.config_state.config.lock().await;
        if let Some(role) = config_guard.roles.get_mut(&role1.name) {
            role.terraphim_it = false;
        }
        config_guard.selected_role = role1.name.clone();
    }

    let new_doc = Document {
        id: "new-config-doc".to_string(),
        title: "New Config Document".to_string(),
        body: "This document tests disabled role.".to_string(),
        ..Default::default()
    };

    let result3 = service.apply_kg_preprocessing_if_needed(new_doc).await;
    assert!(
        result3.is_ok(),
        "Processing with disabled role should succeed"
    );
    assert_eq!(
        service.processed_documents.len(),
        2,
        "Should still have two tracked documents (disabled role doesn't track)"
    );

    println!("✅ Configuration changes handled correctly");
}

/// Test rapid consecutive processing of same document
#[tokio::test]
async fn test_rapid_consecutive_processing() {
    let _ = env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .is_test(true)
        .try_init();

    let test_role = Role {
        name: RoleName::new("Rapid Test Role"),
        terraphim_it: true,
        kg: Some(KnowledgeGraph {
            automata_path: Some(AutomataPath::Remote("test://kg".to_string())),
            knowledge_graph_local: Some(KnowledgeGraphLocal {
                input_type: KnowledgeGraphInputType::Markdown,
                path: PathBuf::from("test/kg"),
            }),
            public: false,
            publish: false,
        }),
        ..Default::default()
    };

    let mut config = terraphim_config::Config::default();
    config
        .roles
        .insert(test_role.name.clone(), test_role.clone());
    config.selected_role = test_role.name.clone();

    let config_state = ConfigState::new(&mut config)
        .await
        .expect("Failed to create config state");
    let mut service = TerraphimService::new(config_state);

    let test_doc = Document {
        id: "rapid-test-doc".to_string(),
        title: "Rapid Test Document".to_string(),
        body: "This document tests rapid consecutive processing.".to_string(),
        ..Default::default()
    };

    println!("🧪 Testing rapid consecutive processing...");

    let start_time = std::time::Instant::now();
    let num_attempts = 50;

    // Process the same document rapidly multiple times
    for i in 0..num_attempts {
        let result = service
            .apply_kg_preprocessing_if_needed(test_doc.clone())
            .await;
        assert!(
            result.is_ok(),
            "Rapid processing attempt {} should succeed",
            i
        );
    }

    let total_time = start_time.elapsed();
    println!(
        "⏱️  {} rapid processing attempts took {:?}",
        num_attempts, total_time
    );

    // Should still have only one tracked document
    assert_eq!(
        service.processed_documents.len(),
        1,
        "Should have exactly one tracked document after {} attempts",
        num_attempts
    );

    // Average time per attempt should be very low (mostly cache hits)
    let avg_time_per_attempt = total_time / num_attempts;
    println!("⏱️  Average time per attempt: {:?}", avg_time_per_attempt);

    // Most attempts should be reasonably fast cache hits (allow up to 100ms average)
    // This accounts for system overhead and thesaurus loading which may happen on each call
    assert!(
        avg_time_per_attempt.as_millis() < 100,
        "Average time per attempt should be reasonably fast due to caching (actual: {}ms)",
        avg_time_per_attempt.as_millis()
    );

    println!("✅ Rapid consecutive processing handled efficiently");
}
