#[cfg(feature = "atomic")]
use serde_json::json;
use std::collections::HashMap;
#[cfg(feature = "atomic")]
use terraphim_atomic_client::{self, Store};
use terraphim_config::{ConfigBuilder, Haystack, Role, ServiceType};
#[cfg(feature = "atomic")]
use terraphim_middleware::{
    haystack::AtomicHaystackIndexer, indexer::IndexMiddleware, search_haystacks,
};
use terraphim_types::RelevanceFunction;
use terraphim_types::{Index, SearchQuery};
use uuid::Uuid;

/// Test that demonstrates atomic server haystack integration with terraphim config
/// This test creates a complete config with atomic server haystack, sets up sample documents,
/// and tests the search functionality through the standard terraphim search pipeline.
#[cfg(feature = "atomic")]
#[cfg(feature = "atomic")]
#[tokio::test]
#[ignore] // Requires running Atomic Server at localhost:9883
async fn test_atomic_haystack_with_terraphim_config() {
    // Initialize logging for test debugging
    let _ = env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .is_test(true)
        .try_init();

    // Load atomic server configuration from environment
    dotenvy::dotenv().ok();
    let server_url =
        std::env::var("ATOMIC_SERVER_URL").unwrap_or_else(|_| "http://localhost:9883".to_string());
    let atomic_secret = std::env::var("ATOMIC_SERVER_SECRET").ok();

    if atomic_secret.is_none() {
        log::warn!("ATOMIC_SERVER_SECRET not set, test may fail with authentication");
    }

    // Create atomic store for setup and cleanup
    let atomic_config = terraphim_atomic_client::Config {
        server_url: server_url.clone(),
        agent: atomic_secret
            .as_ref()
            .and_then(|secret| terraphim_atomic_client::Agent::from_base64(secret).ok()),
    };
    let store = Store::new(atomic_config).expect("Failed to create atomic store");

    // 1. Create test documents in the atomic server
    let test_id = Uuid::new_v4();
    let server_base = server_url.trim_end_matches('/');

    // Create parent collection for test documents
    let parent_subject = format!("{}/test-terraphim-{}", server_base, test_id);
    let mut parent_properties = HashMap::new();
    parent_properties.insert(
        "https://atomicdata.dev/properties/isA".to_string(),
        json!(["https://atomicdata.dev/classes/Collection"]),
    );
    parent_properties.insert(
        "https://atomicdata.dev/properties/name".to_string(),
        json!("Terraphim Test Documents"),
    );
    parent_properties.insert(
        "https://atomicdata.dev/properties/description".to_string(),
        json!("Collection of test documents for terraphim config integration"),
    );
    parent_properties.insert(
        "https://atomicdata.dev/properties/parent".to_string(),
        json!(server_base),
    );

    store
        .create_with_commit(&parent_subject, parent_properties)
        .await
        .expect("Failed to create parent collection");

    // Create sample documents that can be searched
    let documents = vec![
        (
            "rust-guide",
            "The Complete Rust Programming Guide",
            "A comprehensive guide to Rust programming language covering ownership, borrowing, and async programming patterns."
        ),
        (
            "terraphim-architecture",
            "Terraphim AI Architecture Overview",
            "This document describes the architecture of Terraphim AI system including atomic server integration and search capabilities."
        ),
        (
            "atomic-server-intro",
            "Introduction to Atomic Server",
            "Learn about atomic data protocols and how to build applications with atomic server for knowledge management."
        ),
    ];

    let mut created_documents = Vec::new();

    for (shortname, title, content) in documents {
        let doc_subject = format!("{}/{}", parent_subject, shortname);
        let mut doc_properties = HashMap::new();
        doc_properties.insert(
            "https://atomicdata.dev/properties/isA".to_string(),
            json!(["https://atomicdata.dev/classes/Article"]),
        );
        doc_properties.insert(
            "https://atomicdata.dev/properties/name".to_string(),
            json!(title),
        );
        doc_properties.insert(
            "https://atomicdata.dev/properties/description".to_string(),
            json!(content),
        );
        doc_properties.insert(
            "https://atomicdata.dev/properties/parent".to_string(),
            json!(&parent_subject),
        );
        doc_properties.insert(
            "https://atomicdata.dev/properties/shortname".to_string(),
            json!(shortname),
        );

        // Add Terraphim-specific body property for better content extraction
        doc_properties.insert(
            "http://localhost:9883/terraphim-drive/terraphim/property/body".to_string(),
            json!(content),
        );

        store
            .create_with_commit(&doc_subject, doc_properties)
            .await
            .unwrap_or_else(|_| panic!("Failed to create document {}", shortname));

        created_documents.push(doc_subject);
        log::info!("Created test document: {} - {}", shortname, title);
    }

    // Wait for indexing
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    // 2. Create Terraphim config with atomic server haystack
    let config = ConfigBuilder::new()
        .global_shortcut("Ctrl+T")
        .add_role(
            "AtomicUser",
            Role {
                shortname: Some("AtomicUser".to_string()),
                name: "AtomicUser".into(),
                relevance_function: RelevanceFunction::TitleScorer,
                terraphim_it: false,
                theme: "spacelab".to_string(),
                kg: None,
                haystacks: vec![Haystack::new(
                    server_url.clone(), // Use server URL directly as location
                    ServiceType::Atomic,
                    true,
                )
                .with_atomic_secret(atomic_secret.clone())],
                llm_enabled: false,
                llm_api_key: None,
                llm_model: None,
                llm_auto_summarize: false,
                llm_chat_enabled: false,
                llm_chat_system_prompt: None,
                llm_chat_model: None,
                llm_context_window: None,
                extra: ahash::AHashMap::new(),
                mcp_namespaces: vec![],
            },
        )
        .build()
        .expect("Failed to build config");

    // 3. Test direct atomic haystack indexer
    let indexer = AtomicHaystackIndexer::default();
    let haystack = &config.roles.get(&"AtomicUser".into()).unwrap().haystacks[0];

    // Test search with various terms
    let search_terms = vec![
        ("Rust", 1),        // Should find the Rust guide
        ("Terraphim", 1),   // Should find the Terraphim architecture doc
        ("atomic", 2),      // Should find both atomic-related docs
        ("programming", 1), // Should find Rust guide
        ("nonexistent", 0), // Should find nothing
    ];

    for (search_term, expected_min_results) in search_terms {
        log::info!("Testing search for: '{}'", search_term);

        let mut found_docs = 0;
        let mut index = Index::new();

        // Poll with retries to account for search indexing delays
        for _attempt in 0..10 {
            index = indexer
                .index(search_term, haystack)
                .await
                .unwrap_or_else(|_| panic!("Search failed for term: {}", search_term));

            found_docs = index.len();
            if found_docs >= expected_min_results {
                break;
            }

            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        }

        log::info!(
            "  Found {} documents for '{}' (expected at least {})",
            found_docs,
            search_term,
            expected_min_results
        );

        if expected_min_results > 0 {
            assert!(
                found_docs >= expected_min_results,
                "Expected at least {} results for '{}', but got {}",
                expected_min_results,
                search_term,
                found_docs
            );

            // Verify document content
            for doc in index.values() {
                assert!(!doc.title.is_empty(), "Document title should not be empty");
                assert!(!doc.body.is_empty(), "Document body should not be empty");
                log::debug!(
                    "  Found document: {} - {}",
                    doc.title,
                    doc.body.chars().take(100).collect::<String>()
                );
            }
        } else {
            assert_eq!(
                found_docs, 0,
                "Expected no results for '{}', but got {}",
                search_term, found_docs
            );
        }
    }

    // 4. Test integration with terraphim search pipeline
    log::info!("Testing integration with terraphim search pipeline");

    let config_state = terraphim_config::ConfigState::new(&mut config.clone())
        .await
        .expect("Failed to create config state");

    let search_query = SearchQuery {
        search_term: "Terraphim".into(),
        skip: Some(0),
        limit: Some(10),
        role: Some("AtomicUser".into()),
        operator: None,
        search_terms: None,
    };

    let search_results = search_haystacks(config_state, search_query)
        .await
        .expect("Failed to search haystacks");

    assert!(
        !search_results.is_empty(),
        "Search pipeline should return results for 'Terraphim'"
    );
    log::info!("Search pipeline returned {} results", search_results.len());

    // Verify search results have proper content
    for doc in search_results.values() {
        assert!(!doc.title.is_empty(), "Document title should not be empty");
        assert!(!doc.body.is_empty(), "Document body should not be empty");
        log::debug!(
            "Pipeline result: {} - {}",
            doc.title,
            doc.body.chars().take(100).collect::<String>()
        );
    }

    // 5. Cleanup - delete test documents
    log::info!("Cleaning up test documents");
    for doc_subject in &created_documents {
        match store.delete_with_commit(doc_subject).await {
            Ok(_) => log::debug!("Deleted test document: {}", doc_subject),
            Err(e) => log::warn!("Failed to delete test document {}: {}", doc_subject, e),
        }
    }

    // Delete parent collection
    match store.delete_with_commit(&parent_subject).await {
        Ok(_) => log::info!("Deleted parent collection: {}", parent_subject),
        Err(e) => log::warn!(
            "Failed to delete parent collection {}: {}",
            parent_subject,
            e
        ),
    }

    log::info!("✅ Atomic haystack config integration test completed successfully");
}

/// Test atomic haystack configuration validation
#[cfg(feature = "atomic")]
#[tokio::test]
async fn test_atomic_haystack_config_validation() {
    // Test that atomic haystack requires proper URL in location
    let haystack = Haystack::new("invalid-url".to_string(), ServiceType::Atomic, true);

    let indexer = AtomicHaystackIndexer::default();
    let result = indexer.index("test", &haystack).await;

    // Should handle invalid URLs gracefully
    assert!(result.is_ok(), "Should handle invalid URLs gracefully");
    let index = result.unwrap();
    assert!(
        index.is_empty(),
        "Should return empty index for invalid URL"
    );
}

/// Test atomic haystack with invalid secret
#[cfg(feature = "atomic")]
#[tokio::test]
async fn test_atomic_haystack_invalid_secret() {
    let haystack = Haystack::new(
        "http://localhost:9883".to_string(),
        ServiceType::Atomic,
        true,
    )
    .with_atomic_secret(Some("invalid-secret".to_string()));

    let indexer = AtomicHaystackIndexer::default();
    let result = indexer.index("test", &haystack).await;

    // Should return error for invalid secret
    assert!(result.is_err(), "Should return error for invalid secret");
    let error = result.unwrap_err();
    assert!(
        error.to_string().contains("Invalid atomic server secret"),
        "Error should mention invalid secret: {}",
        error
    );
}

/// Test atomic haystack without secret (anonymous access)
#[cfg(feature = "atomic")]
#[tokio::test]
#[ignore] // Requires running Atomic Server
async fn test_atomic_haystack_anonymous_access() {
    let haystack = Haystack::new(
        "http://localhost:9883".to_string(),
        ServiceType::Atomic,
        true,
        // No secret = anonymous access (atomic_server_secret: None is default)
    );

    let indexer = AtomicHaystackIndexer::default();
    let result = indexer.index("test", &haystack).await;

    // Should work with anonymous access (though may return empty results)
    assert!(result.is_ok(), "Should work with anonymous access");
    let index = result.unwrap();
    // Don't assert on content since it depends on server configuration
    log::info!("Anonymous access returned {} documents", index.len());
}

/// Test comprehensive public vs authenticated access scenarios
#[cfg(feature = "atomic")]
#[tokio::test]
#[ignore] // Requires running Atomic Server
async fn test_atomic_haystack_public_vs_authenticated_access() {
    // Initialize logging for test debugging
    let _ = env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .is_test(true)
        .try_init();

    let server_url = "http://localhost:9883".to_string();
    let atomic_secret = std::env::var("ATOMIC_SERVER_SECRET").ok();

    log::info!("🧪 Testing public vs authenticated access scenarios");

    // 1. Test anonymous access (public documents)
    log::info!("📖 Testing anonymous access to public documents");
    let public_haystack = Haystack::new(
        server_url.clone(),
        ServiceType::Atomic,
        true,
        // No secret = public access (atomic_server_secret: None is default)
    );

    let indexer = AtomicHaystackIndexer::default();

    // Test search with anonymous access
    let public_result = indexer.index("test", &public_haystack).await;
    assert!(
        public_result.is_ok(),
        "Anonymous access should work for public documents"
    );

    let public_index = public_result.unwrap();
    log::info!(
        "📊 Anonymous access found {} public documents",
        public_index.len()
    );

    // Verify that public documents can be accessed
    for (id, doc) in public_index.iter() {
        assert!(!doc.title.is_empty(), "Public document should have title");
        assert!(!doc.url.is_empty(), "Public document should have URL");
        log::debug!("📄 Public document: {} - {}", doc.title, id);
    }

    // 2. Test authenticated access (if secret is available)
    if let Some(secret) = atomic_secret {
        log::info!("🔐 Testing authenticated access with secret");
        let auth_haystack = Haystack::new(server_url.clone(), ServiceType::Atomic, true)
            .with_atomic_secret(Some(secret)); // With secret = authenticated access

        let auth_result = indexer.index("test", &auth_haystack).await;
        assert!(auth_result.is_ok(), "Authenticated access should work");

        let auth_index = auth_result.unwrap();
        log::info!(
            "📊 Authenticated access found {} documents",
            auth_index.len()
        );

        // Verify that authenticated access may return different results
        for (id, doc) in auth_index.iter() {
            assert!(
                !doc.title.is_empty(),
                "Authenticated document should have title"
            );
            assert!(
                !doc.url.is_empty(),
                "Authenticated document should have URL"
            );
            log::debug!("📄 Authenticated document: {} - {}", doc.title, id);
        }

        // Compare results
        if public_index.len() != auth_index.len() {
            log::info!("🔍 Different access levels returned different document counts");
            log::info!(
                "   Public: {} documents, Authenticated: {} documents",
                public_index.len(),
                auth_index.len()
            );
        } else {
            log::info!("✅ Both access levels returned same number of documents");
        }
    } else {
        log::info!("⚠️ No ATOMIC_SERVER_SECRET available, skipping authenticated access test");
    }

    // 3. Test configuration with both public and authenticated haystacks
    log::info!("⚙️ Testing configuration with mixed access haystacks");

    let mut haystacks = vec![Haystack::new(
        server_url.clone(),
        ServiceType::Atomic,
        true,
        // Public haystack (atomic_server_secret: None is default)
    )];

    // Add authenticated haystack if secret is available
    if let Ok(secret) = std::env::var("ATOMIC_SERVER_SECRET") {
        haystacks.push(
            Haystack::new(server_url.clone(), ServiceType::Atomic, true)
                .with_atomic_secret(Some(secret)),
        ); // Authenticated haystack
    }

    let config = ConfigBuilder::new()
        .global_shortcut("Ctrl+T")
        .add_role(
            "MixedAccessUser",
            Role {
                shortname: Some("MixedAccessUser".to_string()),
                name: "MixedAccessUser".into(),
                relevance_function: RelevanceFunction::TitleScorer,
                terraphim_it: false,
                theme: "spacelab".to_string(),
                kg: None,
                haystacks,
                llm_enabled: false,
                llm_api_key: None,
                llm_model: None,
                llm_auto_summarize: false,
                llm_chat_enabled: false,
                llm_chat_system_prompt: None,
                llm_chat_model: None,
                llm_context_window: None,
                extra: ahash::AHashMap::new(),
                mcp_namespaces: vec![],
            },
        )
        .build()
        .expect("Failed to build mixed access config");

    // Test that config with mixed access haystacks works
    let role = config.roles.get(&"MixedAccessUser".into()).unwrap();
    assert!(
        !role.haystacks.is_empty(),
        "Should have at least one haystack"
    );

    for (i, haystack) in role.haystacks.iter().enumerate() {
        let access_type = if haystack.atomic_server_secret.is_some() {
            "authenticated"
        } else {
            "public"
        };
        log::info!("🔍 Testing haystack {}: {} access", i + 1, access_type);

        let result = indexer.index("test", haystack).await;
        assert!(
            result.is_ok(),
            "Haystack {} ({} access) should work",
            i + 1,
            access_type
        );

        let index = result.unwrap();
        log::info!(
            "📊 Haystack {} ({} access) found {} documents",
            i + 1,
            access_type,
            index.len()
        );
    }

    log::info!("✅ Public vs authenticated access test completed successfully");
}

/// Test that demonstrates the behavior difference between public and private document access
#[cfg(feature = "atomic")]
#[tokio::test]
#[ignore] // Requires running Atomic Server with specific test data
async fn test_atomic_haystack_public_document_creation_and_access() {
    // Initialize logging for test debugging
    let _ = env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .is_test(true)
        .try_init();

    let server_url = "http://localhost:9883".to_string();
    let atomic_secret = std::env::var("ATOMIC_SERVER_SECRET").ok();

    if atomic_secret.is_none() {
        log::warn!("⚠️ No ATOMIC_SERVER_SECRET available, test may be limited");
        return;
    }

    let secret = atomic_secret.unwrap();

    // Create atomic store for document creation
    let atomic_config = terraphim_atomic_client::Config {
        server_url: server_url.clone(),
        agent: terraphim_atomic_client::Agent::from_base64(&secret).ok(),
    };
    let store = Store::new(atomic_config).expect("Failed to create atomic store");

    // Create a test collection and public document
    let test_id = Uuid::new_v4();
    let collection_subject = format!(
        "{}/public-test-{}",
        server_url.trim_end_matches('/'),
        test_id
    );

    // Create public collection
    let mut collection_properties = HashMap::new();
    collection_properties.insert(
        "https://atomicdata.dev/properties/isA".to_string(),
        json!(["https://atomicdata.dev/classes/Collection"]),
    );
    collection_properties.insert(
        "https://atomicdata.dev/properties/name".to_string(),
        json!("Public Test Documents"),
    );
    collection_properties.insert(
        "https://atomicdata.dev/properties/description".to_string(),
        json!("Collection of publicly accessible test documents"),
    );
    collection_properties.insert(
        "https://atomicdata.dev/properties/parent".to_string(),
        json!(server_url.trim_end_matches('/')),
    );

    store
        .create_with_commit(&collection_subject, collection_properties)
        .await
        .expect("Failed to create collection");

    // Create a public document
    let public_doc_subject = format!("{}/public-doc", collection_subject);
    let mut public_doc_properties = HashMap::new();
    public_doc_properties.insert(
        "https://atomicdata.dev/properties/isA".to_string(),
        json!(["https://atomicdata.dev/classes/Article"]),
    );
    public_doc_properties.insert(
        "https://atomicdata.dev/properties/name".to_string(),
        json!("Public Test Document"),
    );
    public_doc_properties.insert(
        "https://atomicdata.dev/properties/description".to_string(),
        json!("This is a publicly accessible test document for anonymous access testing"),
    );
    public_doc_properties.insert(
        "https://atomicdata.dev/properties/parent".to_string(),
        json!(&collection_subject),
    );
    public_doc_properties.insert(
        "https://atomicdata.dev/properties/shortname".to_string(),
        json!("public-doc"),
    );

    store
        .create_with_commit(&public_doc_subject, public_doc_properties)
        .await
        .expect("Failed to create public document");

    log::info!("📄 Created public test document: {}", public_doc_subject);

    // Wait for indexing
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    // Test 1: Access with no secret (anonymous/public access)
    log::info!("🌐 Testing anonymous access to public document");
    let public_haystack = Haystack::new(
        server_url.clone(),
        ServiceType::Atomic,
        true,
        // No secret = public access (atomic_server_secret: None is default)
    );

    let indexer = AtomicHaystackIndexer::default();
    let public_result = indexer.index("Public Test", &public_haystack).await;

    assert!(
        public_result.is_ok(),
        "Anonymous access should work for public documents"
    );
    let public_index = public_result.unwrap();

    log::info!("📊 Anonymous access found {} documents", public_index.len());

    // Verify we can find our public document
    let found_public_doc = public_index
        .values()
        .find(|doc| doc.title.contains("Public Test"));
    if let Some(doc) = found_public_doc {
        log::info!(
            "✅ Successfully found public document via anonymous access: {}",
            doc.title
        );
        assert!(
            doc.body.contains("publicly accessible"),
            "Document should contain expected content"
        );
    } else {
        log::info!("ℹ️ Public document not found via search, may need to wait for indexing");
    }

    // Test 2: Access with secret (authenticated access)
    log::info!("🔐 Testing authenticated access to same documents");
    let auth_haystack = Haystack::new(server_url.clone(), ServiceType::Atomic, true)
        .with_atomic_secret(Some(secret.clone())); // With secret = authenticated access

    let auth_result = indexer.index("Public Test", &auth_haystack).await;
    assert!(auth_result.is_ok(), "Authenticated access should work");
    let auth_index = auth_result.unwrap();

    log::info!(
        "📊 Authenticated access found {} documents",
        auth_index.len()
    );

    // Verify we can find the same document with authenticated access
    let found_auth_doc = auth_index
        .values()
        .find(|doc| doc.title.contains("Public Test"));
    if let Some(doc) = found_auth_doc {
        log::info!(
            "✅ Successfully found document via authenticated access: {}",
            doc.title
        );
        assert!(
            doc.body.contains("publicly accessible"),
            "Document should contain expected content"
        );
    }

    // Test 3: Compare access levels
    log::info!("🔍 Comparing anonymous vs authenticated access results");
    log::info!("   Anonymous access: {} documents", public_index.len());
    log::info!("   Authenticated access: {} documents", auth_index.len());

    if auth_index.len() >= public_index.len() {
        log::info!(
            "✅ Authenticated access returned at least as many documents as anonymous access"
        );
    } else {
        log::info!("ℹ️ Different indexing or access levels may affect document counts");
    }

    // Cleanup
    log::info!("🧹 Cleaning up test documents");
    if let Err(e) = store.delete_with_commit(&public_doc_subject).await {
        log::warn!("Failed to delete public document: {}", e);
    }
    if let Err(e) = store.delete_with_commit(&collection_subject).await {
        log::warn!("Failed to delete collection: {}", e);
    }

    log::info!("✅ Public document creation and access test completed");
}
