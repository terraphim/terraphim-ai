#[allow(unused_imports)]
use ahash::AHashMap;
use serial_test::serial;
use std::path::PathBuf;
use terraphim_config::{Config, Haystack, KnowledgeGraph, KnowledgeGraphLocal, Role, ServiceType};
use terraphim_persistence::{DeviceStorage, Persistable};
use terraphim_service::TerraphimService;
use terraphim_types::{Document, NormalizedTermValue, RelevanceFunction, RoleName, SearchQuery};

/// Comprehensive test for TerraphimGraph search issue
///
/// This test validates:
/// 1. Documents are properly saved to persistence with fallback
/// 2. RoleGraph is correctly populated with nodes and edges
/// 3. TerraphimGraph search returns results
/// 4. Document content is preserved through the indexing pipeline
#[tokio::test]
#[serial]
async fn test_terraphim_graph_search_comprehensive() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging for debugging
    env_logger::try_init().ok();

    // Initialize memory-only persistence for testing
    DeviceStorage::init_memory_only().await?;
    log::info!("🔧 Initialized memory-only persistence for testing");

    // Create test documents with rich content that should match thesaurus terms
    let test_documents = vec![
        Document {
            id: "haystack_doc".to_string(),
            title: "haystack.md".to_string(),
            body: "This is a haystack document that provides service functionality. It contains information about terraphim-graph and knowledge graph systems.".to_string(),
            url: "docs/src/kg/haystack.md".to_string(),
            description: Some("A haystack is a service that provides data indexing".to_string()),
            summarization: None,
            stub: None,
            tags: Some(vec!["haystack".to_string(), "service".to_string()]),
            rank: None,
            source_haystack: None,
        },
        Document {
            id: "service_doc".to_string(),
            title: "service.md".to_string(),
            body: "Service architecture documentation. This service provides haystack functionality and integrates with the knowledge graph system.".to_string(),
            url: "docs/src/kg/service.md".to_string(),
            description: Some("Service layer documentation".to_string()),
            summarization: None,
            stub: None,
            tags: Some(vec!["service".to_string()]),
            rank: None,
            source_haystack: None,
        },
        Document {
            id: "terraphim_graph_doc".to_string(),
            title: "terraphim-graph.md".to_string(),
            body: "Terraphim-graph is the knowledge graph system that provides semantic search. It uses haystacks as data sources and services for processing.".to_string(),
            url: "docs/src/kg/terraphim-graph.md".to_string(),
            description: Some("Terraphim graph documentation".to_string()),
            summarization: None,
            stub: None,
            tags: Some(vec!["terraphim-graph".to_string(), "knowledge-graph".to_string()]),
            rank: None,
            source_haystack: None,
        },
    ];

    // Step 1: Save test documents to persistence (test multiple backends)
    log::info!("💾 Saving test documents to persistence");
    for doc in &test_documents {
        doc.save().await?;
        log::info!("✅ Saved document '{}' to persistence", doc.id);

        // Verify document can be loaded back
        let mut loaded_doc = Document::new(doc.id.clone());
        loaded_doc = loaded_doc.load().await?;
        assert_eq!(
            loaded_doc.title, doc.title,
            "Document title should be preserved"
        );
        assert_eq!(
            loaded_doc.body, doc.body,
            "Document body should be preserved"
        );
        assert!(
            !loaded_doc.body.is_empty(),
            "Document body should not be empty"
        );
        log::info!(
            "✅ Verified document '{}' can be loaded with content",
            doc.id
        );
    }

    // Step 2: Create TerraphimGraph role configuration
    let role_name = RoleName::new("Test Terraphim Engineer");
    let mut config = Config::default();

    let role = Role {
        shortname: Some("test-terraphim".to_string()),
        name: role_name.clone(),
        haystacks: vec![Haystack {
            location: "docs/src".to_string(),
            service: ServiceType::Ripgrep,
            read_only: true,
            atomic_server_secret: None,
            extra_parameters: std::collections::HashMap::new(),
            fetch_content: false,
        }],
        kg: Some(KnowledgeGraph {
            automata_path: None,
            knowledge_graph_local: Some(KnowledgeGraphLocal {
                input_type: terraphim_types::KnowledgeGraphInputType::Markdown,
                path: PathBuf::from("docs/src/kg"),
            }),
            public: true,
            publish: true,
        }),
        terraphim_it: true,
        theme: "lumen".to_string(),
        relevance_function: RelevanceFunction::TerraphimGraph,
        ..Default::default()
    };

    config.roles.insert(role_name.clone(), role);
    config.default_role = role_name.clone();
    config.selected_role = role_name.clone();

    // Step 3: Initialize service and build rolegraph
    log::info!("🔧 Creating TerraphimService and building rolegraph");
    let mut config_state = terraphim_config::ConfigState::new(&mut config).await?;
    let mut terraphim_service = TerraphimService::new(config_state.clone());

    // Ensure thesaurus is loaded for the role
    let _thesaurus = terraphim_service
        .ensure_thesaurus_loaded(&role_name)
        .await?;
    log::info!("✅ Thesaurus loaded for role '{}'", role_name);

    // Step 4: Test document indexing and rolegraph population
    log::info!("📊 Testing rolegraph population");

    // Add test documents to rolegraphs manually to simulate proper indexing
    for doc in &test_documents {
        config_state.add_to_roles(doc).await?;
        log::info!("✅ Added document '{}' to rolegraph", doc.id);
    }

    // Step 5: Verify rolegraph is properly populated
    log::info!("🔍 Verifying rolegraph population");
    if let Some(rolegraph_sync) = config_state.roles.get(&role_name) {
        let rolegraph = rolegraph_sync.lock().await;
        let node_count = rolegraph.get_node_count();
        let edge_count = rolegraph.get_edge_count();
        let document_count = rolegraph.get_document_count();

        log::info!("📈 RoleGraph statistics:");
        log::info!("  - Nodes: {}", node_count);
        log::info!("  - Edges: {}", edge_count);
        log::info!("  - Documents: {}", document_count);

        assert!(
            node_count > 0,
            "RoleGraph should have nodes after document indexing"
        );
        assert!(
            edge_count > 0,
            "RoleGraph should have edges after document indexing"
        );
        assert_eq!(
            document_count,
            test_documents.len(),
            "RoleGraph should contain all test documents"
        );

        log::info!("✅ RoleGraph is properly populated");
    } else {
        panic!("RoleGraph not found for role '{}'", role_name);
    }

    // Step 6: Test TerraphimGraph search functionality
    log::info!("🔍 Testing TerraphimGraph search");

    let test_queries = vec![
        ("haystack", "Should find haystack documents"),
        ("service", "Should find service documents"),
        ("terraphim-graph", "Should find terraphim-graph documents"),
        (
            "knowledge graph",
            "Should find knowledge graph related documents",
        ),
    ];

    for (query, description) in test_queries {
        log::info!("🔍 Testing query: '{}' - {}", query, description);

        let search_query = SearchQuery {
            search_term: NormalizedTermValue::new(query.to_string()),
            search_terms: None,
            operator: None,
            role: Some(role_name.clone()),
            skip: None,
            limit: Some(10),
        };

        let results = terraphim_service.search(&search_query).await?;

        log::info!("  📊 Query '{}' returned {} results", query, results.len());

        // Log result details for debugging
        for (i, result) in results.iter().enumerate() {
            log::info!(
                "    {}. '{}' (ID: {}, Rank: {:?})",
                i + 1,
                result.title,
                result.id,
                result.rank
            );
        }

        // The search should return results for queries that match our test documents
        // This is the core assertion that validates the fix
        if query == "haystack" || query == "service" || query == "terraphim-graph" {
            assert!(
                !results.is_empty(),
                "Query '{}' should return results - this indicates TerraphimGraph is working",
                query
            );
            log::info!(
                "  ✅ Query '{}' successfully returned {} results",
                query,
                results.len()
            );
        }
    }

    // Step 7: Test persistence fallback by attempting to load documents
    log::info!("🔄 Testing persistence fallback functionality");

    for doc in &test_documents {
        let mut test_doc = Document::new(doc.id.clone());
        let loaded_doc = test_doc.load().await?;

        assert_eq!(loaded_doc.id, doc.id, "Loaded document ID should match");
        assert!(
            !loaded_doc.body.is_empty(),
            "Loaded document should have body content"
        );

        log::info!(
            "✅ Successfully loaded document '{}' with content via persistence",
            doc.id
        );
    }

    log::info!("🎉 All TerraphimGraph search tests completed successfully!");
    log::info!("✅ Persistence layer: Working with fallback");
    log::info!("✅ Document content loading: Working");
    log::info!("✅ RoleGraph population: Working");
    log::info!("✅ TerraphimGraph search: Working");

    Ok(())
}

/// Test persistence fallback behavior specifically
#[tokio::test]
#[serial]
async fn test_persistence_fallback_behavior() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::try_init().ok();
    DeviceStorage::init_memory_only().await?;

    log::info!("🔧 Testing persistence fallback behavior");

    // Create a test document
    let test_doc = Document {
        id: "fallback_test_doc".to_string(),
        title: "Fallback Test Document".to_string(),
        body: "This document tests persistence fallback functionality.".to_string(),
        url: "test://fallback".to_string(),
        description: Some("Fallback test document".to_string()),
        summarization: None,
        stub: None,
        tags: Some(vec!["fallback".to_string(), "test".to_string()]),
        rank: None,
        source_haystack: None,
    };

    // Save to all profiles
    test_doc.save().await?;
    log::info!("✅ Saved test document to all persistence profiles");

    // Load document - this should succeed via fallback mechanism
    let mut loaded_doc = Document::new(test_doc.id.clone());
    loaded_doc = loaded_doc.load().await?;

    // Verify content is preserved
    assert_eq!(
        loaded_doc.title, test_doc.title,
        "Title should be preserved"
    );
    assert_eq!(loaded_doc.body, test_doc.body, "Body should be preserved");
    assert!(!loaded_doc.body.is_empty(), "Body should not be empty");

    log::info!("✅ Persistence fallback test completed successfully");
    Ok(())
}

/// Test empty rolegraph behavior (should not crash)
#[tokio::test]
#[serial]
async fn test_empty_rolegraph_search() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::try_init().ok();
    DeviceStorage::init_memory_only().await?;

    log::info!("🔧 Testing empty rolegraph search behavior");

    // Create minimal role configuration without documents
    let role_name = RoleName::new("Empty Test Role");
    let mut config = Config::default();

    let role = Role {
        shortname: Some("empty-test".to_string()),
        name: role_name.clone(),
        haystacks: vec![],
        kg: Some(KnowledgeGraph {
            automata_path: None,
            knowledge_graph_local: Some(KnowledgeGraphLocal {
                input_type: terraphim_types::KnowledgeGraphInputType::Markdown,
                path: PathBuf::from("docs/src/kg"),
            }),
            public: true,
            publish: true,
        }),
        terraphim_it: true,
        theme: "lumen".to_string(),
        relevance_function: RelevanceFunction::TerraphimGraph,
        ..Default::default()
    };

    config.roles.insert(role_name.clone(), role);
    config.default_role = role_name.clone();

    let config_state = terraphim_config::ConfigState::new(&mut config).await?;
    let mut terraphim_service = TerraphimService::new(config_state);

    // Search on empty rolegraph should not crash
    let search_query = SearchQuery {
        search_term: NormalizedTermValue::new("test".to_string()),
        search_terms: None,
        operator: None,
        role: Some(role_name.clone()),
        skip: None,
        limit: Some(10),
    };

    let results = terraphim_service.search(&search_query).await?;

    // Should return empty results, not crash
    assert!(
        results.is_empty(),
        "Empty rolegraph should return empty results"
    );

    log::info!(
        "✅ Empty rolegraph search handled gracefully (returned {} results)",
        results.len()
    );
    Ok(())
}
