use std::path::PathBuf;
use terraphim_automata::AutomataPath;
use terraphim_config::{KnowledgeGraph, KnowledgeGraphLocal, Role};
use terraphim_service::{ConfigState, TerraphimService};
use terraphim_types::{Document, KnowledgeGraphInputType, RoleName};
use tokio;

/// Test to verify that KG preprocessing happens exactly once per document-role combination
/// This is a simplified test that focuses on the HashMap tracking functionality
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
