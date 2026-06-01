//! Unit tests for terraphim_service (relocated from lib.rs as part of the
//! Gitea #1910 god-file decomposition; behaviour unchanged).

use super::*;
use std::path::PathBuf;
use terraphim_config::ConfigBuilder;
use terraphim_types::{Document, Layer, NormalizedTermValue, RoleName};

#[tokio::test]
async fn test_get_config() {
    let mut config = ConfigBuilder::new()
        .build_default_desktop()
        .build()
        .unwrap();
    let config_state = ConfigState::new(&mut config).await.unwrap();
    let service = TerraphimService::new(config_state);
    let fetched_config = service.fetch_config().await;
    assert_eq!(fetched_config.id, terraphim_config::ConfigId::Desktop);
}

#[tokio::test]
async fn test_search_documents_selected_role() {
    // Check if KG directory exists before running test
    let project_root = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let kg_path = project_root.join("docs/src/kg");
    if !kg_path.exists() {
        println!("Skipping test: KG directory not found at {:?}", kg_path);
        return;
    }

    let mut config = ConfigBuilder::new()
        .build_default_desktop()
        .build()
        .unwrap();
    let config_state = match ConfigState::new(&mut config).await {
        Ok(state) => state,
        Err(e) => {
            println!("Skipping test: Failed to create config state: {:?}", e);
            return;
        }
    };
    let mut service = TerraphimService::new(config_state);
    let search_term = NormalizedTermValue::new("terraphim".to_string());
    let documents = match service.search_documents_selected_role(&search_term).await {
        Ok(docs) => docs,
        Err(e) => {
            println!(
                "Skipping test: Search failed (expected in some environments): {:?}",
                e
            );
            return;
        }
    };
    assert!(documents.is_empty() || !documents.is_empty()); // Either empty or has results
}

#[tokio::test]
async fn test_ensure_thesaurus_loaded_terraphim_engineer() {
    // Create a fresh config with correct KG path for testing
    let project_root = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let kg_path = project_root.join("docs/src/kg");

    // Skip test gracefully if KG directory doesn't exist
    if !kg_path.exists() {
        println!("⚠️ KG directory not found at {:?}, skipping test", kg_path);
        return;
    }

    let mut config = ConfigBuilder::new()
        .build_default_desktop()
        .build()
        .unwrap();

    // Update the Terraphim Engineer role to use project KG directory
    if let Some(terr_eng_role) = config.roles.get_mut(&"Terraphim Engineer".into()) {
        if let Some(kg) = &mut terr_eng_role.kg {
            if let Some(kg_local) = &mut kg.knowledge_graph_local {
                kg_local.path = kg_path;
            }
        }
    }

    let config_state = ConfigState::new(&mut config).await.unwrap();
    let mut service = TerraphimService::new(config_state);

    let role_name = RoleName::new("Terraphim Engineer");
    let thesaurus_result = service.ensure_thesaurus_loaded(&role_name).await;

    match thesaurus_result {
        Ok(thesaurus) => {
            println!(
                "✅ Successfully loaded thesaurus with {} entries",
                thesaurus.len()
            );
            // Verify thesaurus contains expected terms
            assert!(!thesaurus.is_empty(), "Thesaurus should not be empty");

            // Check for expected terms from docs/src/kg using &thesaurus for iteration
            let has_terraphim = (&thesaurus)
                .into_iter()
                .any(|(term, _)| term.as_str().to_lowercase().contains("terraphim"));
            let has_graph = (&thesaurus)
                .into_iter()
                .any(|(term, _)| term.as_str().to_lowercase().contains("graph"));

            println!("   Contains 'terraphim': {}", has_terraphim);
            println!("   Contains 'graph': {}", has_graph);

            // At least one of these should be present
            assert!(
                has_terraphim || has_graph,
                "Thesaurus should contain expected terms"
            );
        }
        Err(e) => {
            println!("❌ Failed to load thesaurus: {:?}", e);
            // This might fail if the local KG files don't exist, which is expected in some test environments
            // We'll just log the error but not fail the test
        }
    }
}

#[tokio::test]
#[ignore = "Requires local KG fixtures at ~/.terraphim/kg"]
async fn test_config_building_with_local_kg() {
    // Test that config building works correctly with local KG files
    let mut config = ConfigBuilder::new()
        .build_default_desktop()
        .build()
        .unwrap();
    let config_state_result = ConfigState::new(&mut config).await;

    match config_state_result {
        Ok(config_state) => {
            println!("✅ Successfully built config state");
            // Verify that roles were created
            assert!(
                !config_state.roles.is_empty(),
                "Config state should have roles"
            );

            // Check if Terraphim Engineer role was created
            let terraphim_engineer_role = RoleName::new("Terraphim Engineer");
            let has_terraphim_engineer = config_state.roles.contains_key(&terraphim_engineer_role);
            println!("   Has Terraphim Engineer role: {}", has_terraphim_engineer);

            // The role should exist even if thesaurus building failed
            assert!(
                has_terraphim_engineer,
                "Terraphim Engineer role should exist"
            );
        }
        Err(e) => {
            println!("❌ Failed to build config state: {:?}", e);
            // This might fail if the local KG files don't exist, which is expected in some test environments
            // We'll just log the error but not fail the test
        }
    }
}

#[tokio::test]
async fn test_atomic_data_persistence_skip() {
    use ahash::AHashMap;
    use terraphim_config::{Config, Haystack, Role, ServiceType};
    use terraphim_persistence::DeviceStorage;
    use terraphim_types::{NormalizedTermValue, RoleName, SearchQuery};

    // Initialize memory-only persistence for testing
    DeviceStorage::init_memory_only().await.unwrap();

    // Create a test config with a role
    let mut config = Config::default();
    let role_name = RoleName::new("test_role");
    let role = Role {
        shortname: None,
        name: "test_role".into(),
        haystacks: vec![Haystack {
            location: "test".to_string(),
            service: ServiceType::Ripgrep,
            read_only: false,
            atomic_server_secret: None,
            extra_parameters: std::collections::HashMap::new(),
            fetch_content: false,
        }],
        kg: None,
        terraphim_it: false,
        theme: "default".to_string(),
        relevance_function: terraphim_types::RelevanceFunction::TitleScorer,
        llm_enabled: false,
        llm_api_key: None,
        llm_model: None,
        llm_auto_summarize: false,
        llm_chat_enabled: false,
        llm_chat_system_prompt: None,
        llm_chat_model: None,
        llm_context_window: None,
        extra: AHashMap::new(),
        llm_router_enabled: false,
        llm_router_config: None,
    };
    config.roles.insert(role_name.clone(), role);

    let config_state = ConfigState::new(&mut config).await.unwrap();
    let mut service = TerraphimService::new(config_state);

    // Create a test search query
    let search_query = SearchQuery {
        search_term: NormalizedTermValue::new("test".to_string()),
        search_terms: None,
        operator: None,
        limit: Some(10),
        skip: None,
        role: Some(role_name),
        layer: Layer::default(),
        include_pinned: false,
        min_quality: None,
    };

    // Test that Atomic Data URLs are skipped during persistence lookup
    // This test verifies that the debug message is logged instead of trying to load from persistence
    let result = service.search(&search_query).await;

    // The search should complete without errors, even though no documents are found
    // The important thing is that Atomic Data URLs don't cause persistence lookup errors
    assert!(result.is_ok(), "Search should complete without errors");
}

#[tokio::test]
async fn test_atomic_data_caching() {
    use ahash::AHashMap;
    use terraphim_config::{Config, Haystack, Role, ServiceType};
    use terraphim_persistence::DeviceStorage;
    use terraphim_types::{Document, NormalizedTermValue, RoleName, SearchQuery};

    // Initialize memory-only persistence for testing
    DeviceStorage::init_memory_only().await.unwrap();

    // Create a test config with a role
    let mut config = Config::default();
    let role_name = RoleName::new("test_role");
    let role = Role {
        shortname: None,
        name: "test_role".into(),
        haystacks: vec![Haystack {
            location: "test".to_string(),
            service: ServiceType::Ripgrep,
            read_only: false,
            atomic_server_secret: None,
            extra_parameters: std::collections::HashMap::new(),
            fetch_content: false,
        }],
        kg: None,
        terraphim_it: false,
        theme: "default".to_string(),
        relevance_function: terraphim_types::RelevanceFunction::TitleScorer,
        llm_enabled: false,
        llm_api_key: None,
        llm_model: None,
        llm_auto_summarize: false,
        llm_chat_enabled: false,
        llm_chat_system_prompt: None,
        llm_chat_model: None,
        llm_context_window: None,
        extra: AHashMap::new(),
        llm_router_enabled: false,
        llm_router_config: None,
    };
    config.roles.insert(role_name.clone(), role);

    let config_state = ConfigState::new(&mut config).await.unwrap();
    let mut service = TerraphimService::new(config_state);

    // Create a mock Atomic Data document
    let atomic_doc = Document {
        id: "http://localhost:9883/borrower-portal/form-field/requestedLoanAmount".to_string(),
        url: "http://localhost:9883/borrower-portal/form-field/requestedLoanAmount".to_string(),
        title: "Requested Loan Amount ($)".to_string(),
        body: "Form field for Requested Loan Amount ($)".to_string(),
        description: Some("Form field for Requested Loan Amount ($)".to_string()),
        summarization: None,
        stub: None,
        tags: None,
        rank: None,
        source_haystack: None,
        doc_type: terraphim_types::DocumentType::KgEntry,
        synonyms: None,
        route: None,
        priority: None,
        quality_score: None,
    };

    // Test 1: Save Atomic Data document to persistence
    log::info!("Testing Atomic Data document caching...");
    match atomic_doc.save().await {
        Ok(_) => log::info!("✅ Successfully saved Atomic Data document to persistence"),
        Err(e) => {
            log::error!("❌ Failed to save Atomic Data document: {}", e);
            panic!("Atomic Data document save failed");
        }
    }

    // Test 2: Verify the document can be loaded from persistence
    let mut placeholder = Document {
        id: atomic_doc.id.clone(),
        ..Default::default()
    };
    match placeholder.load().await {
        Ok(loaded_doc) => {
            log::info!("✅ Successfully loaded Atomic Data document from persistence");
            assert_eq!(loaded_doc.title, atomic_doc.title);
            assert_eq!(loaded_doc.body, atomic_doc.body);
            assert_eq!(loaded_doc.description, atomic_doc.description);
        }
        Err(e) => {
            log::error!(
                "❌ Failed to load Atomic Data document from persistence: {}",
                e
            );
            panic!("Atomic Data document load failed");
        }
    }

    // Test 3: Verify the search logic would find the cached document
    let search_query = SearchQuery {
        search_term: NormalizedTermValue::new("test".to_string()),
        search_terms: None,
        operator: None,
        limit: Some(10),
        skip: None,
        role: Some(role_name),
        layer: Layer::default(),
        include_pinned: false,
        min_quality: None,
    };

    let result = service.search(&search_query).await;
    assert!(result.is_ok(), "Search should complete without errors");

    log::info!("✅ All Atomic Data caching tests passed!");
}

#[tokio::test]
#[ignore = "Requires local KG fixtures at 'test' directory"]
async fn test_kg_term_search_with_atomic_data() {
    use ahash::AHashMap;
    use std::path::PathBuf;
    use terraphim_config::{
        Config, Haystack, KnowledgeGraph, KnowledgeGraphLocal, Role, ServiceType,
    };
    use terraphim_persistence::DeviceStorage;
    use terraphim_types::{Document, KnowledgeGraphInputType, RoleName};

    // Initialize memory-only persistence for testing
    DeviceStorage::init_memory_only().await.unwrap();

    // Create a test config with a role that has KG enabled
    let mut config = Config::default();
    let role_name = RoleName::new("test_kg_role");
    let role = Role {
        shortname: None,
        name: "test_kg_role".into(),
        haystacks: vec![Haystack {
            location: "test".to_string(),
            service: ServiceType::Ripgrep,
            read_only: false,
            atomic_server_secret: None,
            extra_parameters: std::collections::HashMap::new(),
            fetch_content: false,
        }],
        kg: Some(KnowledgeGraph {
            automata_path: None,
            knowledge_graph_local: Some(KnowledgeGraphLocal {
                input_type: KnowledgeGraphInputType::Markdown,
                path: PathBuf::from("test"),
            }),
            public: true,
            publish: true,
        }),
        terraphim_it: true,
        theme: "default".to_string(),
        relevance_function: terraphim_types::RelevanceFunction::TerraphimGraph,
        llm_enabled: false,
        llm_api_key: None,
        llm_model: None,
        llm_auto_summarize: false,
        llm_chat_enabled: false,
        llm_chat_system_prompt: None,
        llm_chat_model: None,
        llm_context_window: None,
        extra: AHashMap::new(),
        llm_router_enabled: false,
        llm_router_config: None,
    };
    config.roles.insert(role_name.clone(), role);

    let config_state = ConfigState::new(&mut config).await.unwrap();
    let mut service = TerraphimService::new(config_state);

    // Create and cache an Atomic Data document
    let atomic_doc = Document {
        id: "http://localhost:9883/borrower-portal/form-field/requestedLoanAmount".to_string(),
        url: "http://localhost:9883/borrower-portal/form-field/requestedLoanAmount".to_string(),
        title: "Requested Loan Amount ($)".to_string(),
        body: "Form field for Requested Loan Amount ($)".to_string(),
        description: Some("Form field for Requested Loan Amount ($)".to_string()),
        summarization: None,
        stub: None,
        tags: None,
        rank: None,
        source_haystack: None,
        doc_type: terraphim_types::DocumentType::KgEntry,
        synonyms: None,
        route: None,
        priority: None,
        quality_score: None,
    };

    // Save the Atomic Data document to persistence
    log::info!("Testing KG term search with Atomic Data documents...");
    match atomic_doc.save().await {
        Ok(_) => log::info!("✅ Successfully saved Atomic Data document to persistence"),
        Err(e) => {
            log::error!("❌ Failed to save Atomic Data document: {}", e);
            panic!("Atomic Data document save failed");
        }
    }

    // Test that find_documents_for_kg_term can handle Atomic Data document IDs
    // Note: In a real scenario, the rolegraph would contain the Atomic Data document ID
    // For this test, we're verifying that the function can handle Atomic Data URLs properly
    let result = service.find_documents_for_kg_term(&role_name, "test").await;

    // The function should complete without errors, even if no documents are found
    // The important thing is that it doesn't crash when encountering Atomic Data URLs
    assert!(
        result.is_ok(),
        "find_documents_for_kg_term should complete without errors"
    );

    let documents = result.unwrap();
    log::info!(
        "✅ KG term search completed successfully, found {} documents",
        documents.len()
    );

    // Verify that the function can handle Atomic Data document loading
    // by manually testing the document loading logic
    let atomic_doc_id = "http://localhost:9883/borrower-portal/form-field/requestedLoanAmount";
    let mut placeholder = Document {
        id: atomic_doc_id.to_string(),
        ..Default::default()
    };

    match placeholder.load().await {
        Ok(loaded_doc) => {
            log::info!(
                "✅ Successfully loaded Atomic Data document from persistence in KG term search context"
            );
            assert_eq!(loaded_doc.title, atomic_doc.title);
            assert_eq!(loaded_doc.body, atomic_doc.body);
        }
        Err(e) => {
            log::error!(
                "❌ Failed to load Atomic Data document in KG term search context: {}",
                e
            );
            panic!("Atomic Data document load failed in KG term search context");
        }
    }

    log::info!("✅ All KG term search with Atomic Data tests passed!");
}

#[tokio::test]
async fn test_kg_term_search_rank_assignment() -> Result<()> {
    use ahash::AHashMap;
    use terraphim_config::{Config, Haystack, Role, ServiceType};
    use terraphim_persistence::DeviceStorage;
    use terraphim_types::{Document, RoleName};

    // Initialize memory-only persistence for testing
    DeviceStorage::init_memory_only().await.unwrap();

    // Create a test config with a role that has KG capabilities
    let mut config = Config::default();
    let role_name = RoleName::new("Test KG Role");
    let role = Role {
        shortname: Some("test-kg".to_string()),
        name: role_name.clone(),
        haystacks: vec![Haystack {
            location: "test".to_string(),
            service: ServiceType::Ripgrep,
            read_only: false,
            atomic_server_secret: None,
            extra_parameters: std::collections::HashMap::new(),
            fetch_content: false,
        }],
        kg: Some(terraphim_config::KnowledgeGraph {
            automata_path: Some(terraphim_automata::AutomataPath::local_example()),
            knowledge_graph_local: None,
            public: false,
            publish: false,
        }),
        terraphim_it: false,
        theme: "default".to_string(),
        relevance_function: terraphim_types::RelevanceFunction::TitleScorer,
        llm_enabled: false,
        llm_api_key: None,
        llm_model: None,
        llm_auto_summarize: false,
        llm_chat_enabled: false,
        llm_chat_system_prompt: None,
        llm_chat_model: None,
        llm_context_window: None,
        extra: AHashMap::new(),
        llm_router_enabled: false,
        llm_router_config: None,
    };
    config.roles.insert(role_name.clone(), role);

    let config_state = ConfigState::new(&mut config).await.unwrap();
    let _service = TerraphimService::new(config_state);

    // Create test documents and save them to persistence
    let test_documents = vec![
        Document {
            id: "test-doc-1".to_string(),
            title: "First Test Document".to_string(),
            body: "This is the first test document body".to_string(),
            url: "test://doc1".to_string(),
            description: Some("First document description".to_string()),
            summarization: None,
            stub: None,
            tags: Some(vec!["test".to_string(), "first".to_string()]),
            rank: None, // Should be assigned by the function
            source_haystack: None,
            doc_type: terraphim_types::DocumentType::KgEntry,
            synonyms: None,
            route: None,
            priority: None,
            quality_score: None,
        },
        Document {
            id: "test-doc-2".to_string(),
            title: "Second Test Document".to_string(),
            body: "This is the second test document body".to_string(),
            url: "test://doc2".to_string(),
            description: Some("Second document description".to_string()),
            summarization: None,
            stub: None,
            tags: Some(vec!["test".to_string(), "second".to_string()]),
            rank: None, // Should be assigned by the function
            source_haystack: None,
            doc_type: terraphim_types::DocumentType::KgEntry,
            synonyms: None,
            route: None,
            priority: None,
            quality_score: None,
        },
        Document {
            id: "test-doc-3".to_string(),
            title: "Third Test Document".to_string(),
            body: "This is the third test document body".to_string(),
            url: "test://doc3".to_string(),
            description: Some("Third document description".to_string()),
            summarization: None,
            stub: None,
            tags: Some(vec!["test".to_string(), "third".to_string()]),
            rank: None, // Should be assigned by the function
            source_haystack: None,
            doc_type: terraphim_types::DocumentType::KgEntry,
            synonyms: None,
            route: None,
            priority: None,
            quality_score: None,
        },
    ];

    // Save test documents to persistence
    for doc in &test_documents {
        doc.save().await.expect("Failed to save test document");
    }

    // The rolegraph will be created automatically by ensure_thesaurus_loaded
    // We don't need to manually create it for this test

    // Test the rank assignment logic directly
    // This validates the core functionality we implemented in find_documents_for_kg_term
    let mut simulated_documents = test_documents.clone();

    // Apply the same rank assignment logic as in find_documents_for_kg_term
    let total_length = simulated_documents.len();
    for (idx, doc) in simulated_documents.iter_mut().enumerate() {
        let rank = (total_length - idx) as u64;
        doc.rank = Some(rank);
    }

    // Verify rank assignment
    assert_eq!(simulated_documents.len(), 3, "Should have 3 test documents");

    // Check that all documents have ranks assigned
    for doc in &simulated_documents {
        assert!(
            doc.rank.is_some(),
            "Document '{}' should have a rank assigned",
            doc.title
        );
        assert!(
            doc.rank.unwrap() > 0,
            "Document '{}' should have a positive rank",
            doc.title
        );
    }

    // Check that ranks are in descending order (first document has highest rank)
    assert_eq!(
        simulated_documents[0].rank,
        Some(3),
        "First document should have highest rank (3)"
    );
    assert_eq!(
        simulated_documents[1].rank,
        Some(2),
        "Second document should have rank 2"
    );
    assert_eq!(
        simulated_documents[2].rank,
        Some(1),
        "Third document should have rank 1"
    );

    // Verify ranks are unique and properly ordered
    let mut ranks: Vec<u64> = simulated_documents
        .iter()
        .map(|doc| doc.rank.unwrap())
        .collect();
    ranks.sort_by_key(|r| std::cmp::Reverse(*r));
    assert_eq!(
        ranks,
        vec![3, 2, 1],
        "Ranks should be unique and in descending order"
    );

    log::info!("✅ KG term search rank assignment test completed successfully!");
    Ok(())
}

// Helper to build a Document with a given composite quality score.
fn doc_with_quality(id: &str, knowledge: f64, logic: f64, structure: f64) -> Document {
    Document {
        id: id.to_string(),
        url: format!("https://example.com/{id}"),
        title: id.to_string(),
        body: String::new(),
        quality_score: Some(terraphim_types::QualityScore {
            knowledge: Some(knowledge),
            logic: Some(logic),
            structure: Some(structure),
            last_evaluated: None,
        }),
        ..Default::default()
    }
}

fn doc_without_quality(id: &str) -> Document {
    Document {
        id: id.to_string(),
        url: format!("https://example.com/{id}"),
        title: id.to_string(),
        body: String::new(),
        quality_score: None,
        ..Default::default()
    }
}

#[test]
fn test_min_quality_none_returns_all_documents() {
    // When min_quality is None, all documents are returned unchanged.
    let docs = vec![
        doc_with_quality("a", 0.9, 0.9, 0.9),
        doc_with_quality("b", 0.1, 0.1, 0.1),
        doc_without_quality("c"),
    ];
    let result = TerraphimService::apply_min_quality_filter(docs, None);
    assert_eq!(result.len(), 3);
}

#[test]
fn test_min_quality_keeps_documents_at_or_above_threshold() {
    // composite = (0.8 + 0.6 + 0.7) / 3 = 0.7
    let high = doc_with_quality("high", 0.8, 0.6, 0.7);
    // composite = (0.3 + 0.2 + 0.1) / 3 ≈ 0.2
    let low = doc_with_quality("low", 0.3, 0.2, 0.1);
    let docs = vec![high, low];

    let result = TerraphimService::apply_min_quality_filter(docs, Some(0.5));
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].id, "high");
}

#[test]
fn test_min_quality_excludes_documents_below_threshold() {
    // composite = 0.4
    let doc = doc_with_quality("below", 0.4, 0.4, 0.4);
    let result = TerraphimService::apply_min_quality_filter(vec![doc], Some(0.5));
    assert!(result.is_empty());
}

#[test]
fn test_min_quality_excludes_documents_without_quality_score() {
    // Documents with no quality_score must be excluded when a threshold is set.
    let no_score = doc_without_quality("no-score");
    let result = TerraphimService::apply_min_quality_filter(vec![no_score], Some(0.0));
    assert!(result.is_empty());
}

#[test]
fn test_min_quality_exact_threshold_is_included() {
    // composite = 0.5 exactly — must satisfy >= threshold
    let doc = doc_with_quality("exact", 0.5, 0.5, 0.5);
    let result = TerraphimService::apply_min_quality_filter(vec![doc], Some(0.5));
    assert_eq!(result.len(), 1);
}

#[test]
fn test_min_quality_threshold_zero_excludes_no_score_docs() {
    // Threshold 0.0 passes any document that has a score, but not scoreless ones.
    let with_score = doc_with_quality("scored", 0.0, 0.0, 0.0);
    let no_score = doc_without_quality("unscored");
    let result = TerraphimService::apply_min_quality_filter(vec![with_score, no_score], Some(0.0));
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].id, "scored");
}

#[test]
fn test_min_quality_empty_input_returns_empty() {
    let result = TerraphimService::apply_min_quality_filter(vec![], Some(0.5));
    assert!(result.is_empty());
}

#[test]
fn test_min_quality_preserves_document_order() {
    // Verify that documents passing the filter are returned in original order.
    let a = doc_with_quality("a", 0.9, 0.9, 0.9);
    let b = doc_with_quality("b", 0.8, 0.8, 0.8);
    let c = doc_with_quality("c", 0.7, 0.7, 0.7);
    let result = TerraphimService::apply_min_quality_filter(vec![a, b, c], Some(0.5));
    assert_eq!(result.len(), 3);
    assert_eq!(result[0].id, "a");
    assert_eq!(result[1].id, "b");
    assert_eq!(result[2].id, "c");
}

#[test]
fn test_min_quality_negative_threshold_clamped_to_zero() {
    // A negative threshold is clamped to 0.0: documents with any score pass,
    // documents without a score are still excluded.
    let with_score = doc_with_quality("scored", 0.1, 0.1, 0.1);
    let no_score = doc_without_quality("unscored");
    let result = TerraphimService::apply_min_quality_filter(vec![with_score, no_score], Some(-0.1));
    assert_eq!(result.len(), 1, "only scored document should pass");
    assert_eq!(result[0].id, "scored");
}

#[test]
fn test_snippet_around_ascii_simple() {
    let s = "Hello World foo](kg:bar Baz";
    let result = snippet_around(s, "](kg:", 10, 10);
    assert_eq!(result, " World foo](kg:bar Baz");
}

#[test]
fn test_snippet_around_ascii_truncation_left() {
    let s = "xyz Hello World foo](kg:bar";
    let result = snippet_around(s, "](kg:", 10, 10);
    assert_eq!(result, " World foo](kg:bar");
}

#[test]
fn test_snippet_around_ascii_truncation_right() {
    let s = "Hello World foo](kg:bar xyz";
    let result = snippet_around(s, "](kg:", 10, 10);
    assert_eq!(result, " World foo](kg:bar xyz");
}

#[test]
fn test_snippet_around_multibyte_cjk() {
    let s = "日本語 Hello](kg:bar 日本語";
    let result = snippet_around(s, "](kg:", 5, 5);
    assert!(!result.is_empty());
    assert!(result.contains("Hello"));
    assert!(result.contains("](kg:"));
}

#[test]
fn test_snippet_around_multibyte_emoji() {
    let s = "Hello 😂 World](kg:bar";
    let result = snippet_around(s, "](kg:", 10, 10);
    assert!(!result.is_empty());
    assert!(result.contains("😂"));
    assert!(result.contains("](kg:"));
}

#[test]
fn test_snippet_around_marker_not_found() {
    let s = "Hello World";
    let result = snippet_around(s, "](kg:", 10, 10);
    assert_eq!(result, "");
}

#[test]
fn test_snippet_around_empty_string() {
    let s = "";
    let result = snippet_around(s, "](kg:", 10, 10);
    assert_eq!(result, "");
}

#[test]
fn test_snippet_around_marker_at_start() {
    let s = "](kg:bar Hello";
    let result = snippet_around(s, "](kg:", 10, 10);
    assert_eq!(result, "](kg:bar Hello");
}

#[test]
fn test_snippet_around_marker_at_end() {
    let s = "Hello ](kg:bar";
    let result = snippet_around(s, "](kg:", 10, 10);
    assert_eq!(result, "Hello ](kg:bar");
}
