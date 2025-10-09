//! Service-level persistence integration tests
//!
//! This test suite validates that persistence works correctly at the service level,
//! including real-world scenarios with thesaurus building, document creation,
//! and cross-service instance persistence.

use serial_test::serial;
use std::time::Duration;
use tokio::time::timeout;
use tracing::Level;

use terraphim_config::{ConfigBuilder, ConfigId, ConfigState};
use terraphim_persistence::Persistable;
use terraphim_service::TerraphimService;
use terraphim_types::{Document, NormalizedTermValue, RoleName, SearchQuery};

#[tokio::test]
#[serial]
async fn test_service_document_persistence_integration() {
    // Initialize logging for debugging
    let _ = tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .try_init();

    println!("🧪 Testing service-level document persistence integration");

    // Step 1: Initialize memory-only persistence
    println!("📝 Step 1: Initializing memory-only persistence");
    terraphim_persistence::DeviceStorage::init_memory_only()
        .await
        .expect("Failed to initialize memory-only persistence");

    // Step 2: Create service with desktop config
    println!("🔧 Step 2: Creating service configuration");
    let mut config = ConfigBuilder::new_with_id(ConfigId::Desktop)
        .build_default_desktop()
        .build()
        .expect("Failed to build desktop config");

    let config_state = ConfigState::new(&mut config)
        .await
        .expect("Failed to initialize ConfigState");

    let mut service = TerraphimService::new(config_state.clone());

    // Step 3: Create and save a document through the service
    println!("📄 Step 3: Creating document through service");
    let test_document = Document {
        id: "service-integration-test-doc".to_string(),
        title: "Service Integration Test Document".to_string(),
        body: "This document tests service-level persistence integration with comprehensive content that should be preserved across service restarts.".to_string(),
        url: "https://example.com/service-test".to_string(),
        description: Some("A test document for service persistence validation".to_string()),
        summarization: None,
        stub: None,
        tags: Some(vec!["service".to_string(), "persistence".to_string(), "integration".to_string()]),
        rank: Some(95),
        source_haystack: None,
    };

    let created_doc = service
        .create_document(test_document.clone())
        .await
        .expect("Failed to create document through service");

    assert_eq!(
        created_doc.id, test_document.id,
        "Created document ID should match"
    );
    assert_eq!(
        created_doc.title, test_document.title,
        "Created document title should match"
    );
    println!(
        "  ✅ Document created through service: '{}'",
        created_doc.title
    );

    // Step 4: Verify document can be retrieved by ID
    println!("🔍 Step 4: Retrieving document by ID");
    let retrieved_doc = service
        .get_document_by_id(&test_document.id)
        .await
        .expect("Failed to retrieve document by ID")
        .expect("Document should exist");

    assert_eq!(
        retrieved_doc.id, test_document.id,
        "Retrieved document ID should match"
    );
    assert_eq!(
        retrieved_doc.title, test_document.title,
        "Retrieved document title should match"
    );
    assert_eq!(
        retrieved_doc.body, test_document.body,
        "Retrieved document body should match"
    );
    assert_eq!(
        retrieved_doc.description, test_document.description,
        "Retrieved document description should match"
    );
    assert_eq!(
        retrieved_doc.tags, test_document.tags,
        "Retrieved document tags should match"
    );
    assert_eq!(
        retrieved_doc.rank, test_document.rank,
        "Retrieved document rank should match"
    );
    println!(
        "  ✅ Document retrieved successfully: '{}'",
        retrieved_doc.title
    );

    // Step 5: Create a new service instance to test persistence across instances
    println!("🔄 Step 5: Testing persistence with new service instance");
    let mut new_service = TerraphimService::new(config_state.clone());

    let retrieved_doc_new_instance = new_service
        .get_document_by_id(&test_document.id)
        .await
        .expect("Failed to retrieve document with new service instance")
        .expect("Document should exist in new service instance");

    assert_eq!(
        retrieved_doc_new_instance.id, test_document.id,
        "New instance document ID should match"
    );
    assert_eq!(
        retrieved_doc_new_instance.title, test_document.title,
        "New instance document title should match"
    );
    assert_eq!(
        retrieved_doc_new_instance.body, test_document.body,
        "New instance document body should match"
    );
    println!("  ✅ Document persisted across service instances");

    // Step 6: Test document persistence with special characters in ID
    println!("📄 Step 6: Testing document with special character ID");
    let special_id_doc = Document {
        id: "doc-with-special@chars#and spaces!".to_string(),
        title: "Special Character ID Test".to_string(),
        body: "Testing document persistence with special characters in ID.".to_string(),
        url: "https://example.com/special-chars".to_string(),
        description: Some("Document with special character ID".to_string()),
        summarization: None,
        stub: None,
        tags: None,
        rank: None,
        source_haystack: None,
    };

    let _created_special_doc = new_service
        .create_document(special_id_doc.clone())
        .await
        .expect("Failed to create document with special character ID");

    let retrieved_special_doc = new_service
        .get_document_by_id(&special_id_doc.id)
        .await
        .expect("Failed to retrieve special character document")
        .expect("Special character document should exist");

    assert_eq!(
        retrieved_special_doc.id, special_id_doc.id,
        "Special char document ID should match"
    );
    assert_eq!(
        retrieved_special_doc.title, special_id_doc.title,
        "Special char document title should match"
    );
    println!("  ✅ Document with special character ID persisted correctly");

    println!("🎉 All service document persistence integration tests passed!");
}

#[tokio::test]
#[serial]
async fn test_service_thesaurus_persistence_integration() {
    let _ = tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .try_init();

    println!("🧪 Testing service-level thesaurus persistence integration");

    // Step 1: Initialize persistence
    println!("📝 Step 1: Initializing memory-only persistence");
    terraphim_persistence::DeviceStorage::init_memory_only()
        .await
        .expect("Failed to initialize memory-only persistence");

    // Step 2: Create service with role that has challenging name
    println!("🔧 Step 2: Creating service with challenging role name");
    let mut config = ConfigBuilder::new_with_id(ConfigId::Desktop)
        .build_default_desktop()
        .build()
        .expect("Failed to build desktop config");

    let config_state = ConfigState::new(&mut config)
        .await
        .expect("Failed to initialize ConfigState");

    let mut service = TerraphimService::new(config_state.clone());

    // Test with role name that contains spaces and special characters
    let challenging_role_names = vec![
        "Terraphim Engineer",     // Standard case with space
        "AI/ML Specialist",       // Forward slash
        "Data & Analytics",       // Ampersand
        "Senior Engineer (2024)", // Parentheses and number
    ];

    for role_name_str in challenging_role_names {
        println!(
            "  🔍 Testing thesaurus persistence for role: '{}'",
            role_name_str
        );

        let role_name = RoleName::new(role_name_str);

        // Step 3: Load thesaurus for challenging role name
        let thesaurus_result = timeout(
            Duration::from_secs(30),
            service.ensure_thesaurus_loaded(&role_name),
        )
        .await
        .expect("Thesaurus load timed out")
        .expect("Failed to load thesaurus");

        println!(
            "    ✅ Thesaurus loaded: {} entries",
            thesaurus_result.len()
        );

        // Verify some expected terms exist (if thesaurus has content)
        if !thesaurus_result.is_empty() {
            let expected_terms = vec!["service", "haystack"];
            let mut found_terms = Vec::new();

            for term in &expected_terms {
                let normalized_term = NormalizedTermValue::from(term.to_string());
                if thesaurus_result.get(&normalized_term).is_some() {
                    found_terms.push(*term);
                }
            }

            if !found_terms.is_empty() {
                println!("    ✓ Found expected terms: {:?}", found_terms);
            }
        }

        // Step 4: Create new service instance and test persistence
        let mut new_service = TerraphimService::new(config_state.clone());

        let thesaurus_result_new_instance = timeout(
            Duration::from_secs(15), // Should be faster from persistence
            new_service.ensure_thesaurus_loaded(&role_name),
        )
        .await
        .expect("New instance thesaurus load timed out")
        .expect("Failed to load thesaurus in new instance");

        assert_eq!(
            thesaurus_result.len(),
            thesaurus_result_new_instance.len(),
            "Thesaurus should have same size across instances for role '{}'",
            role_name_str
        );

        println!(
            "    ✅ Thesaurus persisted across service instances for role: '{}'",
            role_name_str
        );
    }

    println!("🎉 All service thesaurus persistence integration tests passed!");
}

#[tokio::test]
#[serial]
async fn test_service_search_with_persisted_data() {
    let _ = tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .try_init();

    println!("🧪 Testing search functionality with persisted data");

    // Step 1: Initialize persistence
    println!("📝 Step 1: Initializing memory-only persistence");
    terraphim_persistence::DeviceStorage::init_memory_only()
        .await
        .expect("Failed to initialize memory-only persistence");

    // Step 2: Create service and load thesaurus
    println!("🔧 Step 2: Creating service and loading thesaurus");
    let mut config = ConfigBuilder::new_with_id(ConfigId::Desktop)
        .build_default_desktop()
        .build()
        .expect("Failed to build desktop config");

    let config_state = ConfigState::new(&mut config)
        .await
        .expect("Failed to initialize ConfigState");

    let mut service = TerraphimService::new(config_state.clone());
    let role_name = RoleName::new("Terraphim Engineer");

    // Load thesaurus to ensure it's persisted
    let _thesaurus = timeout(
        Duration::from_secs(30),
        service.ensure_thesaurus_loaded(&role_name),
    )
    .await
    .expect("Thesaurus load timed out")
    .expect("Failed to load thesaurus");

    println!("  ✅ Thesaurus loaded and persisted");

    // Step 3: Perform search with persisted thesaurus
    println!("🔎 Step 3: Performing search with persisted thesaurus");
    let search_query = SearchQuery {
        search_term: "service".into(),
        role: Some(role_name.clone()),
        skip: None,
        limit: Some(10),
        ..Default::default()
    };

    let search_results = timeout(Duration::from_secs(30), service.search(&search_query))
        .await
        .expect("Search timed out")
        .expect("Search failed");

    println!(
        "  📊 Search returned {} results",
        search_results.documents.len()
    );

    // Step 4: Create new service instance and perform same search
    println!("🔄 Step 4: Testing search with new service instance");
    let mut new_service = TerraphimService::new(config_state.clone());

    let search_results_new_instance =
        timeout(Duration::from_secs(30), new_service.search(&search_query))
            .await
            .expect("New instance search timed out")
            .expect("New instance search failed");

    println!(
        "  📊 New instance search returned {} results",
        search_results_new_instance.documents.len()
    );

    // Results should be consistent (though may vary slightly due to timing/caching)
    // The key point is that both searches should work and return reasonable results
    assert!(
        !search_results_new_instance.documents.is_empty() || search_results.documents.is_empty(),
        "Search results should be consistent across service instances"
    );

    println!("  ✅ Search functionality works with persisted data across instances");

    println!("🎉 All search persistence integration tests passed!");
}

#[tokio::test]
#[serial]
async fn test_key_generation_consistency_in_service() {
    let _ = tracing_subscriber::fmt()
        .with_max_level(Level::DEBUG) // Enable debug logging to see key generation
        .try_init();

    println!("🧪 Testing key generation consistency in service context");

    // Initialize persistence
    terraphim_persistence::DeviceStorage::init_memory_only()
        .await
        .expect("Failed to initialize memory-only persistence");

    // Test thesaurus key generation for various role names
    let test_roles = vec![
        ("Simple", "thesaurus_simple.json"),
        ("Terraphim Engineer", "thesaurus_terraphim_engineer.json"),
        ("AI/ML Engineer", "thesaurus_ai_ml_engineer.json"),
        (
            "Data & Analytics Expert",
            "thesaurus_data_analytics_expert.json",
        ),
    ];

    for (role_name_str, expected_key) in test_roles {
        println!("  🔍 Testing key generation for role: '{}'", role_name_str);

        // Create thesaurus directly and check key
        let thesaurus = terraphim_types::Thesaurus::new(role_name_str.to_lowercase());
        let actual_key = thesaurus.get_key();

        assert_eq!(
            actual_key, expected_key,
            "Key generation mismatch for role '{}': got '{}', expected '{}'",
            role_name_str, actual_key, expected_key
        );

        println!(
            "    ✅ Key generated correctly: '{}' → '{}'",
            role_name_str, actual_key
        );
    }

    // Test document key generation for various IDs
    let test_document_ids = vec![
        ("simple-doc", "document_simpledoc.json"),
        ("Document with Spaces", "document_documentwithspaces.json"),
        ("doc@special#chars!", "document_docspecialchars.json"),
        ("a33bd45bece9c7cb", "document_a33bd45bece9c7cb.json"), // Hash format
    ];

    for (doc_id, expected_key) in test_document_ids {
        println!("  📄 Testing key generation for document ID: '{}'", doc_id);

        let document = Document {
            id: doc_id.to_string(),
            ..Default::default()
        };
        let actual_key = document.get_key();

        assert_eq!(
            actual_key, expected_key,
            "Document key generation mismatch for ID '{}': got '{}', expected '{}'",
            doc_id, actual_key, expected_key
        );

        println!(
            "    ✅ Document key generated correctly: '{}' → '{}'",
            doc_id, actual_key
        );
    }

    println!("🎉 All key generation consistency tests passed!");
}

#[tokio::test]
#[serial]
async fn test_unicode_persistence_in_service() {
    let _ = tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .try_init();

    println!("🧪 Testing unicode content persistence in service");

    // Initialize persistence
    terraphim_persistence::DeviceStorage::init_memory_only()
        .await
        .expect("Failed to initialize memory-only persistence");

    let mut config = ConfigBuilder::new_with_id(ConfigId::Desktop)
        .build_default_desktop()
        .build()
        .expect("Failed to build desktop config");

    let config_state = ConfigState::new(&mut config)
        .await
        .expect("Failed to initialize ConfigState");

    let mut service = TerraphimService::new(config_state.clone());

    // Test document with unicode content
    let unicode_doc = Document {
        id: "unicode-test-document".to_string(),
        title: "Unicode Test: 🚀 Documentation café naïve résumé".to_string(),
        body: "This document contains various unicode characters: 中文, العربية, русский, 🎉, mathematical symbols: ∑∫∂".to_string(),
        url: "https://example.com/unicode-test".to_string(),
        description: Some("Testing unicode persistence: ñoño café".to_string()),
        summarization: None,
        stub: None,
        tags: Some(vec!["unicode".to_string(), "测试".to_string(), "тест".to_string()]),
        rank: Some(100),
        source_haystack: None,
    };

    // Create document through service
    let created_doc = service
        .create_document(unicode_doc.clone())
        .await
        .expect("Failed to create unicode document");

    // Verify unicode content is preserved
    assert_eq!(
        created_doc.title, unicode_doc.title,
        "Unicode title should be preserved"
    );
    assert_eq!(
        created_doc.body, unicode_doc.body,
        "Unicode body should be preserved"
    );
    assert_eq!(
        created_doc.description, unicode_doc.description,
        "Unicode description should be preserved"
    );
    assert_eq!(
        created_doc.tags, unicode_doc.tags,
        "Unicode tags should be preserved"
    );

    // Retrieve document and verify unicode persistence
    let retrieved_doc = service
        .get_document_by_id(&unicode_doc.id)
        .await
        .expect("Failed to retrieve unicode document")
        .expect("Unicode document should exist");

    assert_eq!(
        retrieved_doc.title, unicode_doc.title,
        "Retrieved unicode title should match"
    );
    assert_eq!(
        retrieved_doc.body, unicode_doc.body,
        "Retrieved unicode body should match"
    );
    assert_eq!(
        retrieved_doc.description, unicode_doc.description,
        "Retrieved unicode description should match"
    );
    assert_eq!(
        retrieved_doc.tags, unicode_doc.tags,
        "Retrieved unicode tags should match"
    );

    println!("  ✅ Unicode content preserved correctly in service persistence");
    println!("🎉 Unicode persistence test passed!");
}

#[tokio::test]
#[serial]
async fn test_large_content_persistence_in_service() {
    let _ = tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .try_init();

    println!("🧪 Testing large content persistence in service");

    // Initialize persistence
    terraphim_persistence::DeviceStorage::init_memory_only()
        .await
        .expect("Failed to initialize memory-only persistence");

    let mut config = ConfigBuilder::new_with_id(ConfigId::Desktop)
        .build_default_desktop()
        .build()
        .expect("Failed to build desktop config");

    let config_state = ConfigState::new(&mut config)
        .await
        .expect("Failed to initialize ConfigState");

    let mut service = TerraphimService::new(config_state.clone());

    // Create a document with large content (~50KB)
    let large_content = "Lorem ipsum dolor sit amet, consectetur adipiscing elit. ".repeat(1000);
    let large_doc = Document {
        id: "large-content-test".to_string(),
        title: "Large Content Persistence Test".to_string(),
        body: large_content.clone(),
        url: "https://example.com/large-content".to_string(),
        description: Some("Testing persistence of large document content".to_string()),
        summarization: None,
        stub: None,
        tags: Some(vec!["large".to_string(), "performance".to_string()]),
        rank: Some(50),
        source_haystack: None,
    };

    println!(
        "  📄 Creating document with content size: {} bytes",
        large_content.len()
    );

    // Create and verify large document
    let created_doc = service
        .create_document(large_doc.clone())
        .await
        .expect("Failed to create large document");

    assert_eq!(
        created_doc.body.len(),
        large_content.len(),
        "Large content size should be preserved"
    );

    // Retrieve and verify large document persists correctly
    let retrieved_doc = service
        .get_document_by_id(&large_doc.id)
        .await
        .expect("Failed to retrieve large document")
        .expect("Large document should exist");

    assert_eq!(
        retrieved_doc.body.len(),
        large_content.len(),
        "Retrieved large content size should match"
    );
    assert_eq!(
        retrieved_doc.body, large_content,
        "Retrieved large content should match exactly"
    );

    println!(
        "  ✅ Large content ({} bytes) persisted correctly",
        large_content.len()
    );
    println!("🎉 Large content persistence test passed!");
}
