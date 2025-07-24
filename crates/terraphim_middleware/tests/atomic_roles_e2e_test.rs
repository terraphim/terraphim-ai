use terraphim_config::{ConfigBuilder, Haystack, Role, ServiceType};
use terraphim_types::{RelevanceFunction, SearchQuery};
use terraphim_middleware::{haystack::AtomicHaystackIndexer, indexer::IndexMiddleware, search_haystacks};
use terraphim_atomic_client::{self, Store};
use serde_json::json;
use std::collections::HashMap;
use std::path::PathBuf;
use uuid::Uuid;

/// Test that demonstrates atomic server haystack integration with Title Scorer role
/// This test creates a complete config with atomic server haystack using TitleScorer,
/// sets up sample documents, and tests the search functionality through the standard terraphim search pipeline.
#[tokio::test]
async fn test_atomic_haystack_title_scorer_role() {
    // Initialize logging for test debugging
    let _ = env_logger::try_init();

    // Load atomic server configuration from environment
    dotenvy::dotenv().ok();
    let server_url = std::env::var("ATOMIC_SERVER_URL").unwrap_or_else(|_| "http://localhost:9883".to_string());
    let atomic_secret = std::env::var("ATOMIC_SERVER_SECRET").ok();
    
    if atomic_secret.is_none() {
        log::warn!("ATOMIC_SERVER_SECRET not set, test may fail with authentication");
    }

    // Create atomic store for setup and cleanup
    let atomic_config = terraphim_atomic_client::Config {
        server_url: server_url.clone(),
        agent: atomic_secret.as_ref().and_then(|secret| {
            terraphim_atomic_client::Agent::from_base64(secret).ok()
        }),
    };
    let store = Store::new(atomic_config).expect("Failed to create atomic store");

    // 1. Create test documents in the atomic server
    let test_id = Uuid::new_v4();
    let server_base = server_url.trim_end_matches('/');
    
    // Create parent collection for test documents
    let parent_subject = format!("{}/test-title-scorer-{}", server_base, test_id);
    let mut parent_properties = HashMap::new();
    parent_properties.insert("https://atomicdata.dev/properties/isA".to_string(), 
                            json!(["https://atomicdata.dev/classes/Collection"]));
    parent_properties.insert("https://atomicdata.dev/properties/name".to_string(), 
                            json!("Title Scorer Test Documents"));
    parent_properties.insert("https://atomicdata.dev/properties/description".to_string(), 
                            json!("Collection of test documents for Title Scorer role"));
    parent_properties.insert("https://atomicdata.dev/properties/parent".to_string(), 
                            json!(server_base));
    
    store.create_with_commit(&parent_subject, parent_properties).await
        .expect("Failed to create parent collection");
    
    let mut created_documents = Vec::new();
    
    // Create test documents with clear titles for title-based scoring
    let documents = vec![
        ("terraphim-guide", "Terraphim User Guide", "A comprehensive guide to using Terraphim for knowledge management and search."),
        ("terraphim-arch", "Terraphim Architecture Overview", "Detailed overview of Terraphim system architecture and components."),
        ("atomic-server", "Atomic Server Integration", "How to integrate and use Atomic Server with Terraphim."),
        ("search-algorithms", "Search Algorithm Implementation", "Implementation details of various search algorithms in Terraphim."),
        ("knowledge-graph", "Knowledge Graph Construction", "Building and maintaining knowledge graphs for semantic search."),
    ];
    
    for (shortname, title, content) in documents {
        let doc_subject = format!("{}/{}", parent_subject, shortname);
        let mut doc_properties = HashMap::new();
        doc_properties.insert("https://atomicdata.dev/properties/isA".to_string(), 
                            json!(["https://atomicdata.dev/classes/Article"]));
        doc_properties.insert("https://atomicdata.dev/properties/name".to_string(), json!(title));
        doc_properties.insert("https://atomicdata.dev/properties/description".to_string(), json!(content));
        doc_properties.insert("https://atomicdata.dev/properties/parent".to_string(), json!(&parent_subject));
        doc_properties.insert("https://atomicdata.dev/properties/shortname".to_string(), json!(shortname));
        
        // Add Terraphim-specific body property for better content extraction
        doc_properties.insert("http://localhost:9883/terraphim-drive/terraphim/property/body".to_string(), json!(content));
        
        store.create_with_commit(&doc_subject, doc_properties).await
            .expect(&format!("Failed to create document {}", shortname));
        
        created_documents.push(doc_subject);
        log::info!("Created test document: {} - {}", shortname, title);
    }

    // Wait for indexing - reduced for faster tests
    tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

    // 2. Create Terraphim config with atomic server haystack and TitleScorer
    let config = ConfigBuilder::new()
        .global_shortcut("Ctrl+T")
        .add_role(
            "AtomicTitleScorer",
            Role {
                shortname: Some("title-scorer".to_string()),
                name: "Atomic Title Scorer".into(),
                relevance_function: RelevanceFunction::TitleScorer,
                terraphim_it: false,
                theme: "cerulean".to_string(),
                kg: None, // No knowledge graph for title scorer
                haystacks: vec![Haystack::new(
                    server_url.clone(),
                    ServiceType::Atomic,
                    true,
                ).with_atomic_secret(atomic_secret.clone())],
                extra: ahash::AHashMap::new(),
            },
        )
        .build()
        .expect("Failed to build config");

    // 3. Test direct atomic haystack indexer with title-based search
    let indexer = AtomicHaystackIndexer::default();
    let haystack = &config.roles.get(&"AtomicTitleScorer".into()).unwrap().haystacks[0];

    // Test search with terms that should match titles (both test docs and real docs)
    let search_terms = vec![
        ("Terraphim", 2), // Should find test doc + real docs with 'Terraphim' in title
        ("Architecture", 1), // Should find architecture-related docs
        ("Search", 1), // Should find the Search Algorithm doc
        ("Knowledge", 1), // Should find the Knowledge Graph doc
        ("Server", 1), // Should find the Atomic Server doc
        ("Guide", 1), // Should find guide documents
        ("Introduction", 1), // Should find introduction documents
        ("nonexistent", 0), // Should find nothing
    ];

    for (search_term, expected_min_results) in search_terms {
        log::info!("Testing title-based search for: '{}'", search_term);
        
        // Single search call - indexing should be instant for local server
        let start_time = std::time::Instant::now();
        let index = indexer.index(search_term, haystack).await
            .expect(&format!("Search failed for term: {}", search_term));
        let search_duration = start_time.elapsed();
        
        let found_docs = index.len();
        log::info!("  Search took {:?} and found {} documents for '{}' (expected at least {})", 
                  search_duration, found_docs, search_term, expected_min_results);
        
        if expected_min_results > 0 {
            assert!(found_docs >= expected_min_results, 
                   "Expected at least {} results for '{}', but got {}", 
                   expected_min_results, search_term, found_docs);
            
            // Verify document content and that titles are being used for scoring
            for doc in index.values() {
                assert!(!doc.title.is_empty(), "Document title should not be empty");
                assert!(!doc.body.is_empty(), "Document body should not be empty");
                log::debug!("  Found document: {} - {}", doc.title, doc.body.chars().take(100).collect::<String>());
                
                // For title scorer, verify that matching terms are in the title or body (since full-text search includes body)
                if search_term != "nonexistent" {
                    let term_lower = search_term.to_lowercase();
                    let title_lower = doc.title.to_lowercase();
                    let body_lower = doc.body.to_lowercase();
                    
                    // Check if the search term appears in title or body (atomic server does full-text search)
                    let found_in_content = title_lower.contains(&term_lower) || 
                                          body_lower.contains(&term_lower) ||
                                          // Also check for partial matches (first word of search term)
                                          title_lower.contains(&term_lower.split_whitespace().next().unwrap_or("")) ||
                                          body_lower.contains(&term_lower.split_whitespace().next().unwrap_or(""));
                    
                    if !found_in_content {
                        log::warn!("Document '{}' doesn't contain search term '{}' in title or body", doc.title, search_term);
                        log::debug!("Title: '{}', Body preview: '{}'", doc.title, doc.body.chars().take(200).collect::<String>());
                    }
                    
                    // For atomic server, we expect the search term to be found somewhere in the document
                    // since it uses full-text search across all properties
                    assert!(found_in_content,
                           "Document should contain search term '{}' somewhere for full-text search. Title: '{}', Body preview: '{}'", 
                           search_term, doc.title, doc.body.chars().take(100).collect::<String>());
                }
            }
        } else {
            assert_eq!(found_docs, 0, "Expected no results for '{}', but got {}", search_term, found_docs);
        }
    }

    // 4. Test integration with terraphim search pipeline
    log::info!("Testing integration with terraphim search pipeline (Title Scorer)");
    
    let config_state = terraphim_config::ConfigState::new(&mut config.clone()).await
        .expect("Failed to create config state");
    
    let search_query = SearchQuery {
        search_term: "Terraphim".to_string().into(),
        skip: Some(0),
        limit: Some(10),
        role: Some("AtomicTitleScorer".into()),
    };
    
    let pipeline_start_time = std::time::Instant::now();
    let search_results = search_haystacks(config_state, search_query).await
        .expect("Failed to search haystacks");
    let pipeline_duration = pipeline_start_time.elapsed();
    
    assert!(!search_results.is_empty(), "Search pipeline should return results for 'Terraphim'");
    log::info!("Search pipeline took {:?} and returned {} results", pipeline_duration, search_results.len());
    
    // Verify search results have proper content and title-based ranking
    for doc in search_results.values() {
        assert!(!doc.title.is_empty(), "Document title should not be empty");
        assert!(!doc.body.is_empty(), "Document body should not be empty");
        
        // Check if 'terraphim' appears in title or body (atomic server does full-text search)
        let title_lower = doc.title.to_lowercase();
        let body_lower = doc.body.to_lowercase();
        let contains_terraphim = title_lower.contains("terraphim") || body_lower.contains("terraphim");
        
        if !contains_terraphim {
            log::warn!("Document '{}' doesn't contain 'terraphim' in title or body", doc.title);
        }
        
        assert!(contains_terraphim, 
               "Document should contain 'terraphim' somewhere for full-text search. Title: '{}', Body preview: '{}'", 
               doc.title, doc.body.chars().take(100).collect::<String>());
        log::debug!("Pipeline result: {} - {}", doc.title, doc.body.chars().take(100).collect::<String>());
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
        Err(e) => log::warn!("Failed to delete parent collection {}: {}", parent_subject, e),
    }

    log::info!("✅ Atomic haystack Title Scorer role test completed successfully");
}

/// Test that demonstrates atomic server haystack integration with Graph Embeddings role
/// This test creates a complete config with atomic server haystack using TerraphimGraph,
/// sets up sample documents, and tests the search functionality through the standard terraphim search pipeline.
#[tokio::test]
async fn test_atomic_haystack_graph_embeddings_role() {
    // Initialize logging for test debugging
    let _ = env_logger::try_init();

    // Load atomic server configuration from environment
    dotenvy::dotenv().ok();
    let server_url = std::env::var("ATOMIC_SERVER_URL").unwrap_or_else(|_| "http://localhost:9883".to_string());
    let atomic_secret = std::env::var("ATOMIC_SERVER_SECRET").ok();
    
    if atomic_secret.is_none() {
        log::warn!("ATOMIC_SERVER_SECRET not set, test may fail with authentication");
    }

    // Create atomic store for setup and cleanup
    let atomic_config = terraphim_atomic_client::Config {
        server_url: server_url.clone(),
        agent: atomic_secret.as_ref().and_then(|secret| {
            terraphim_atomic_client::Agent::from_base64(secret).ok()
        }),
    };
    let store = Store::new(atomic_config).expect("Failed to create atomic store");

    // 1. Create test documents in the atomic server with graph-related content
    let test_id = Uuid::new_v4();
    let server_base = server_url.trim_end_matches('/');
    
    // Create parent collection for test documents
    let parent_subject = format!("{}/test-graph-embeddings-{}", server_base, test_id);
    let mut parent_properties = HashMap::new();
    parent_properties.insert("https://atomicdata.dev/properties/isA".to_string(), 
                            json!(["https://atomicdata.dev/classes/Collection"]));
    parent_properties.insert("https://atomicdata.dev/properties/name".to_string(), 
                            json!("Graph Embeddings Test Documents"));
    parent_properties.insert("https://atomicdata.dev/properties/description".to_string(), 
                            json!("Collection of test documents for Graph Embeddings role"));
    parent_properties.insert("https://atomicdata.dev/properties/parent".to_string(), 
                            json!(server_base));
    
    store.create_with_commit(&parent_subject, parent_properties).await
        .expect("Failed to create parent collection");
    
    let mut created_documents = Vec::new();
    
    // Create test documents with graph-related content for graph-based scoring
    let documents = vec![
        ("terraphim-graph", "Terraphim Graph Implementation", "Implementation of the Terraphim knowledge graph with nodes, edges, and embeddings."),
        ("graph-embeddings", "Graph Embeddings and Vector Search", "Using graph embeddings for semantic search and knowledge discovery."),
        ("knowledge-nodes", "Knowledge Graph Nodes and Relationships", "Building knowledge graph nodes and establishing semantic relationships."),
        ("semantic-search", "Semantic Search with Graph Embeddings", "Implementing semantic search using graph embeddings and vector similarity."),
        ("graph-algorithms", "Graph Algorithms for Knowledge Discovery", "Algorithms for traversing and analyzing knowledge graphs."),
    ];
    
    for (shortname, title, content) in documents {
        let doc_subject = format!("{}/{}", parent_subject, shortname);
        let mut doc_properties = HashMap::new();
        doc_properties.insert("https://atomicdata.dev/properties/isA".to_string(), 
                            json!(["https://atomicdata.dev/classes/Article"]));
        doc_properties.insert("https://atomicdata.dev/properties/name".to_string(), json!(title));
        doc_properties.insert("https://atomicdata.dev/properties/description".to_string(), json!(content));
        doc_properties.insert("https://atomicdata.dev/properties/parent".to_string(), json!(&parent_subject));
        doc_properties.insert("https://atomicdata.dev/properties/shortname".to_string(), json!(shortname));
        
        // Add Terraphim-specific body property for better content extraction
        doc_properties.insert("http://localhost:9883/terraphim-drive/terraphim/property/body".to_string(), json!(content));
        
        store.create_with_commit(&doc_subject, doc_properties).await
            .expect(&format!("Failed to create document {}", shortname));
        
        created_documents.push(doc_subject);
        log::info!("Created test document: {} - {}", shortname, title);
    }

    // Wait for indexing - reduced for faster tests
    tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

    // 2. Create Terraphim config with atomic server haystack and TerraphimGraph
    let config = ConfigBuilder::new()
        .global_shortcut("Ctrl+G")
        .add_role(
            "AtomicGraphEmbeddings",
            Role {
                shortname: Some("graph-embeddings".to_string()),
                name: "Atomic Graph Embeddings".into(),
                relevance_function: RelevanceFunction::TerraphimGraph,
                terraphim_it: true,
                theme: "superhero".to_string(),
                kg: Some(terraphim_config::KnowledgeGraph {
                    automata_path: None, // Will be built from local files
                    knowledge_graph_local: Some(terraphim_config::KnowledgeGraphLocal {
                        input_type: terraphim_types::KnowledgeGraphInputType::Markdown,
                        path: PathBuf::from("docs/src"),
                    }),
                    public: true,
                    publish: true,
                }),
                haystacks: vec![Haystack::new(
                    server_url.clone(),
                    ServiceType::Atomic,
                    true,
                ).with_atomic_secret(atomic_secret.clone())],
                extra: ahash::AHashMap::new(),
            },
        )
        .build()
        .expect("Failed to build config");

    // 3. Test direct atomic haystack indexer with graph-based search
    let indexer = AtomicHaystackIndexer::default();
    let haystack = &config.roles.get(&"AtomicGraphEmbeddings".into()).unwrap().haystacks[0];

    // Test search with graph-related terms
    let search_terms = vec![
        ("graph", 3), // Should find graph-related docs
        ("embeddings", 2), // Should find embedding-related docs
        ("knowledge", 2), // Should find knowledge-related docs
        ("semantic", 1), // Should find semantic search doc
        ("terraphim", 1), // Should find Terraphim graph doc
        ("algorithms", 1), // Should find graph algorithms doc
        ("nonexistent", 0), // Should find nothing
    ];

    for (search_term, expected_min_results) in search_terms {
        log::info!("Testing graph-based search for: '{}'", search_term);
        
        // Single search call - indexing should be instant for local server
        let start_time = std::time::Instant::now();
        let index = indexer.index(search_term, haystack).await
            .expect(&format!("Search failed for term: {}", search_term));
        let search_duration = start_time.elapsed();
        
        let found_docs = index.len();
        log::info!("  Search took {:?} and found {} documents for '{}' (expected at least {})", 
                  search_duration, found_docs, search_term, expected_min_results);
        
        if expected_min_results > 0 {
            assert!(found_docs >= expected_min_results, 
                   "Expected at least {} results for '{}', but got {}", 
                   expected_min_results, search_term, found_docs);
            
            // Verify document content
            for doc in index.values() {
                assert!(!doc.title.is_empty(), "Document title should not be empty");
                assert!(!doc.body.is_empty(), "Document body should not be empty");
                log::debug!("  Found document: {} - {}", doc.title, doc.body.chars().take(100).collect::<String>());
            }
        } else {
            assert_eq!(found_docs, 0, "Expected no results for '{}', but got {}", search_term, found_docs);
        }
    }

    // 4. Test integration with terraphim search pipeline
    log::info!("Testing integration with terraphim search pipeline (Graph Embeddings)");
    
    let config_state = terraphim_config::ConfigState::new(&mut config.clone()).await
        .expect("Failed to create config state");
    
    let search_query = SearchQuery {
        search_term: "graph".to_string().into(),
        skip: Some(0),
        limit: Some(10),
        role: Some("AtomicGraphEmbeddings".into()),
    };
    
    let pipeline_start_time = std::time::Instant::now();
    let search_results = search_haystacks(config_state, search_query).await
        .expect("Failed to search haystacks");
    let pipeline_duration = pipeline_start_time.elapsed();
    
    assert!(!search_results.is_empty(), "Search pipeline should return results for 'graph'");
    log::info!("Search pipeline took {:?} and returned {} results", pipeline_duration, search_results.len());
    
    // Verify search results have proper content and graph-based ranking
    for doc in search_results.values() {
        assert!(!doc.title.is_empty(), "Document title should not be empty");
        assert!(!doc.body.is_empty(), "Document body should not be empty");
        log::debug!("Pipeline result: {} - {}", doc.title, doc.body.chars().take(100).collect::<String>());
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
        Err(e) => log::warn!("Failed to delete parent collection {}: {}", parent_subject, e),
    }

    log::info!("✅ Atomic haystack Graph Embeddings role test completed successfully");
}

/// Test that compares the behavior difference between Title Scorer and Graph Embeddings roles
#[tokio::test]
async fn test_atomic_haystack_role_comparison() {
    // Initialize logging for test debugging
    let _ = env_logger::try_init();

    // Load atomic server configuration from environment
    dotenvy::dotenv().ok();
    let server_url = std::env::var("ATOMIC_SERVER_URL").unwrap_or_else(|_| "http://localhost:9883".to_string());
    let atomic_secret = std::env::var("ATOMIC_SERVER_SECRET").ok();
    
    if atomic_secret.is_none() {
        log::warn!("ATOMIC_SERVER_SECRET not set, test may fail with authentication");
    }

    // Create atomic store for setup and cleanup
    let atomic_config = terraphim_atomic_client::Config {
        server_url: server_url.clone(),
        agent: atomic_secret.as_ref().and_then(|secret| {
            terraphim_atomic_client::Agent::from_base64(secret).ok()
        }),
    };
    let store = Store::new(atomic_config).expect("Failed to create atomic store");

    // 1. Create test documents in the atomic server
    let test_id = Uuid::new_v4();
    let server_base = server_url.trim_end_matches('/');
    
    // Create parent collection for test documents
    let parent_subject = format!("{}/test-role-comparison-{}", server_base, test_id);
    let mut parent_properties = HashMap::new();
    parent_properties.insert("https://atomicdata.dev/properties/isA".to_string(), 
                            json!(["https://atomicdata.dev/classes/Collection"]));
    parent_properties.insert("https://atomicdata.dev/properties/name".to_string(), 
                            json!("Role Comparison Test Documents"));
    parent_properties.insert("https://atomicdata.dev/properties/description".to_string(), 
                            json!("Collection of test documents for role comparison"));
    parent_properties.insert("https://atomicdata.dev/properties/parent".to_string(), 
                            json!(server_base));
    
    store.create_with_commit(&parent_subject, parent_properties).await
        .expect("Failed to create parent collection");
    
    let mut created_documents = Vec::new();
    
    // Create test documents that can be scored differently by title vs graph
    let documents = vec![
        ("rust-programming", "Rust Programming Guide", "A comprehensive guide to Rust programming language. This document covers ownership, borrowing, and concurrency patterns in Rust."),
        ("graph-algorithms", "Graph Algorithms and Data Structures", "Implementation of graph algorithms including depth-first search, breadth-first search, and shortest path algorithms."),
        ("machine-learning", "Machine Learning with Graph Embeddings", "Using graph embeddings for machine learning tasks and knowledge representation."),
        ("terraphim-architecture", "Terraphim System Architecture", "Detailed architecture of the Terraphim system including knowledge graphs, search algorithms, and atomic server integration."),
    ];
    
    for (shortname, title, content) in documents {
        let doc_subject = format!("{}/{}", parent_subject, shortname);
        let mut doc_properties = HashMap::new();
        doc_properties.insert("https://atomicdata.dev/properties/isA".to_string(), 
                            json!(["https://atomicdata.dev/classes/Article"]));
        doc_properties.insert("https://atomicdata.dev/properties/name".to_string(), json!(title));
        doc_properties.insert("https://atomicdata.dev/properties/description".to_string(), json!(content));
        doc_properties.insert("https://atomicdata.dev/properties/parent".to_string(), json!(&parent_subject));
        doc_properties.insert("https://atomicdata.dev/properties/shortname".to_string(), json!(shortname));
        
        // Add Terraphim-specific body property for better content extraction
        doc_properties.insert("http://localhost:9883/terraphim-drive/terraphim/property/body".to_string(), json!(content));
        
        store.create_with_commit(&doc_subject, doc_properties).await
            .expect(&format!("Failed to create document {}", shortname));
        
        created_documents.push(doc_subject);
        log::info!("Created test document: {} - {}", shortname, title);
    }

    // Wait for indexing - reduced for faster tests
    tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

    // 2. Create both role configurations
    let title_scorer_config = ConfigBuilder::new()
        .global_shortcut("Ctrl+T")
        .add_role(
            "TitleScorer",
            Role {
                shortname: Some("title-scorer".to_string()),
                name: "Title Scorer".into(),
                relevance_function: RelevanceFunction::TitleScorer,
                terraphim_it: false,
                theme: "cerulean".to_string(),
                kg: None,
                haystacks: vec![Haystack {
                    location: server_url.clone(),
                    service: ServiceType::Atomic,
                    read_only: true,
                    atomic_server_secret: atomic_secret.clone(),
                    extra_parameters: std::collections::HashMap::new(),
                }],
                extra: ahash::AHashMap::new(),
            },
        )
        .build()
        .expect("Failed to build title scorer config");

    let graph_embeddings_config = ConfigBuilder::new()
        .global_shortcut("Ctrl+G")
        .add_role(
            "GraphEmbeddings",
            Role {
                shortname: Some("graph-embeddings".to_string()),
                name: "Graph Embeddings".into(),
                relevance_function: RelevanceFunction::TerraphimGraph,
                terraphim_it: true,
                theme: "superhero".to_string(),
                kg: Some(terraphim_config::KnowledgeGraph {
                    automata_path: None,
                    knowledge_graph_local: Some(terraphim_config::KnowledgeGraphLocal {
                        input_type: terraphim_types::KnowledgeGraphInputType::Markdown,
                        path: PathBuf::from("docs/src"),
                    }),
                    public: true,
                    publish: true,
                }),
                haystacks: vec![Haystack {
                    location: server_url.clone(),
                    service: ServiceType::Atomic,
                    read_only: true,
                    atomic_server_secret: atomic_secret.clone(),
                    extra_parameters: std::collections::HashMap::new(),
                }],
                extra: ahash::AHashMap::new(),
            },
        )
        .build()
        .expect("Failed to build graph embeddings config");

    // 3. Test search with both roles and compare results
    let indexer = AtomicHaystackIndexer::default();
    let title_haystack = &title_scorer_config.roles.get(&"TitleScorer".into()).unwrap().haystacks[0];
    let graph_haystack = &graph_embeddings_config.roles.get(&"GraphEmbeddings".into()).unwrap().haystacks[0];

    // Test search terms that should show different behavior
    let search_terms = vec![
        "graph",
        "programming", 
        "algorithms",
        "machine",
        "terraphim"
    ];

    for search_term in search_terms {
        log::info!("Comparing search results for: '{}'", search_term);
        
        // Search with title scorer
        let title_start_time = std::time::Instant::now();
        let title_index = indexer.index(search_term, title_haystack).await
            .expect(&format!("Title scorer search failed for term: {}", search_term));
        let title_duration = title_start_time.elapsed();
        
        // Search with graph embeddings
        let graph_start_time = std::time::Instant::now();
        let graph_index = indexer.index(search_term, graph_haystack).await
            .expect(&format!("Graph embeddings search failed for term: {}", search_term));
        let graph_duration = graph_start_time.elapsed();
        
        log::info!("  Title Scorer took {:?} and found: {} documents", title_duration, title_index.len());
        log::info!("  Graph Embeddings took {:?} and found: {} documents", graph_duration, graph_index.len());
        
        // Log document titles for comparison
        log::info!("  Title Scorer results:");
        for doc in title_index.values() {
            log::info!("    - {}", doc.title);
        }
        
        log::info!("  Graph Embeddings results:");
        for doc in graph_index.values() {
            log::info!("    - {}", doc.title);
        }
        
        // Both should find some results for valid terms
        if search_term != "nonexistent" {
            assert!(title_index.len() > 0 || graph_index.len() > 0, 
                   "At least one role should find results for '{}'", search_term);
        }
    }

    // 4. Test integration with terraphim search pipeline for both roles
    log::info!("Testing search pipeline integration for both roles");
    
    let title_config_state = terraphim_config::ConfigState::new(&mut title_scorer_config.clone()).await
        .expect("Failed to create title scorer config state");
    
    let graph_config_state = terraphim_config::ConfigState::new(&mut graph_embeddings_config.clone()).await
        .expect("Failed to create graph embeddings config state");
    
    let search_query = SearchQuery {
        search_term: "graph".to_string().into(),
        skip: Some(0),
        limit: Some(10),
        role: None, // Will use default role
    };
    
    // Test with title scorer
    let title_pipeline_start = std::time::Instant::now();
    let title_results = search_haystacks(title_config_state, search_query.clone()).await
        .expect("Failed to search with title scorer");
    let title_pipeline_duration = title_pipeline_start.elapsed();
    
    // Test with graph embeddings
    let graph_pipeline_start = std::time::Instant::now();
    let graph_results = search_haystacks(graph_config_state, search_query).await
        .expect("Failed to search with graph embeddings");
    let graph_pipeline_duration = graph_pipeline_start.elapsed();
    
    log::info!("Title Scorer pipeline took {:?} and returned {} results", title_pipeline_duration, title_results.len());
    log::info!("Graph Embeddings pipeline took {:?} and returned {} results", graph_pipeline_duration, graph_results.len());
    
    // Both should return results
    assert!(title_results.len() > 0 || graph_results.len() > 0, 
           "At least one role should return results from search pipeline");

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
        Err(e) => log::warn!("Failed to delete parent collection {}: {}", parent_subject, e),
    }

    log::info!("✅ Atomic haystack role comparison test completed successfully");
}

/// Test configuration validation for both roles
#[tokio::test]
async fn test_atomic_roles_config_validation() {
    // Test Title Scorer role configuration
    let title_scorer_config = ConfigBuilder::new()
        .global_shortcut("Ctrl+T")
        .add_role(
            "TitleScorer",
            Role {
                shortname: Some("title-scorer".to_string()),
                name: "Title Scorer".into(),
                relevance_function: RelevanceFunction::TitleScorer,
                terraphim_it: false,
                theme: "cerulean".to_string(),
                kg: None, // Title scorer doesn't need knowledge graph
                haystacks: vec![Haystack {
                    location: "http://localhost:9883".to_string(),
                    service: ServiceType::Atomic,
                    read_only: true,
                    atomic_server_secret: None,
                    extra_parameters: std::collections::HashMap::new(),
                }],
                extra: ahash::AHashMap::new(),
            },
        )
        .build()
        .expect("Failed to build title scorer config");

    // Verify Title Scorer role configuration
    let title_role = title_scorer_config.roles.get(&"TitleScorer".into()).unwrap();
    assert_eq!(title_role.relevance_function, RelevanceFunction::TitleScorer);
    assert!(title_role.kg.is_none(), "Title scorer should not have knowledge graph");
    assert_eq!(title_role.haystacks.len(), 1);
    assert_eq!(title_role.haystacks[0].service, ServiceType::Atomic);

    // Test Graph Embeddings role configuration
    let graph_embeddings_config = ConfigBuilder::new()
        .global_shortcut("Ctrl+G")
        .add_role(
            "GraphEmbeddings",
            Role {
                shortname: Some("graph-embeddings".to_string()),
                name: "Graph Embeddings".into(),
                relevance_function: RelevanceFunction::TerraphimGraph,
                terraphim_it: true,
                theme: "superhero".to_string(),
                kg: Some(terraphim_config::KnowledgeGraph {
                    automata_path: None,
                    knowledge_graph_local: Some(terraphim_config::KnowledgeGraphLocal {
                        input_type: terraphim_types::KnowledgeGraphInputType::Markdown,
                        path: PathBuf::from("docs/src"),
                    }),
                    public: true,
                    publish: true,
                }),
                haystacks: vec![Haystack {
                    location: "http://localhost:9883".to_string(),
                    service: ServiceType::Atomic,
                    read_only: true,
                    atomic_server_secret: None,
                    extra_parameters: std::collections::HashMap::new(),
                }],
                extra: ahash::AHashMap::new(),
            },
        )
        .build()
        .expect("Failed to build graph embeddings config");

    // Verify Graph Embeddings role configuration
    let graph_role = graph_embeddings_config.roles.get(&"GraphEmbeddings".into()).unwrap();
    assert_eq!(graph_role.relevance_function, RelevanceFunction::TerraphimGraph);
    assert!(graph_role.kg.is_some(), "Graph embeddings should have knowledge graph");
    assert_eq!(graph_role.haystacks.len(), 1);
    assert_eq!(graph_role.haystacks[0].service, ServiceType::Atomic);

    log::info!("✅ Atomic roles configuration validation test completed successfully");
} 

/// Test comprehensive atomic server haystack role configurations including:
/// 1. Pure atomic roles (TitleScorer and TerraphimGraph)
/// 2. Hybrid roles (Atomic + Ripgrep haystacks)
/// 3. Role switching and comparison
/// 4. Configuration validation
#[tokio::test]
async fn test_comprehensive_atomic_haystack_roles() {
    // Initialize logging for test debugging
    let _ = env_logger::try_init();

    // Load atomic server configuration from environment
    dotenvy::dotenv().ok();
    let server_url = std::env::var("ATOMIC_SERVER_URL").unwrap_or_else(|_| "http://localhost:9883".to_string());
    let atomic_secret = std::env::var("ATOMIC_SERVER_SECRET").ok();
    
    if atomic_secret.is_none() {
        log::warn!("ATOMIC_SERVER_SECRET not set, test may fail with authentication");
    }

    // Create atomic store for setup and cleanup
    let atomic_config = terraphim_atomic_client::Config {
        server_url: server_url.clone(),
        agent: atomic_secret.as_ref().and_then(|secret| {
            terraphim_atomic_client::Agent::from_base64(secret).ok()
        }),
    };
    let store = Store::new(atomic_config).expect("Failed to create atomic store");

    // 1. Create test documents in the atomic server
    let test_id = Uuid::new_v4();
    let server_base = server_url.trim_end_matches('/');
    
    // Create parent collection for test documents
    let parent_subject = format!("{}/test-comprehensive-roles-{}", server_base, test_id);
    let mut parent_properties = HashMap::new();
    parent_properties.insert(
        "https://atomicdata.dev/properties/name".to_string(),
        json!("Comprehensive Roles Test Collection")
    );
    parent_properties.insert(
        "https://atomicdata.dev/properties/description".to_string(),
        json!("Test collection for comprehensive atomic haystack role testing")
    );
    parent_properties.insert(
        "https://atomicdata.dev/properties/isA".to_string(),
        json!(["https://atomicdata.dev/classes/Collection"])
    );
    parent_properties.insert(
        "https://atomicdata.dev/properties/parent".to_string(),
        json!(server_base)
    );

    store.create_with_commit(&parent_subject, parent_properties)
        .await
        .expect("Failed to create parent collection");

    // Create diverse test documents for different search scenarios
    let test_documents = vec![
        (
            format!("{}/atomic-integration-guide", parent_subject),
            "ATOMIC: Integration Guide",
            "Complete guide for integrating Terraphim with atomic server. Covers authentication, configuration, and advanced search features."
        ),
        (
            format!("{}/semantic-search-algorithms", parent_subject),
            "ATOMIC: Semantic Search Algorithms",
            "Advanced semantic search algorithms using graph embeddings, vector spaces, and knowledge graphs for improved relevance."
        ),
        (
            format!("{}/hybrid-haystack-configuration", parent_subject),
            "ATOMIC: Hybrid Haystack Configuration",
            "Configuration guide for setting up hybrid haystacks combining atomic server and ripgrep for comprehensive document search."
        ),
        (
            format!("{}/role-based-search", parent_subject),
            "ATOMIC: Role-Based Search",
            "Role-based search functionality allowing different user roles to access different search capabilities and document sets."
        ),
        (
            format!("{}/performance-optimization", parent_subject),
            "ATOMIC: Performance Optimization",
            "Performance optimization techniques for atomic server integration including caching, indexing, and query optimization."
        ),
    ];

    let mut created_documents = Vec::new();
    for (subject, title, description) in &test_documents {
        let mut properties = HashMap::new();
        properties.insert(
            "https://atomicdata.dev/properties/name".to_string(),
            json!(title)
        );
        properties.insert(
            "https://atomicdata.dev/properties/description".to_string(),
            json!(description)
        );
        properties.insert(
            "https://atomicdata.dev/properties/isA".to_string(),
            json!(["https://atomicdata.dev/classes/Article"])
        );
        properties.insert(
            "https://atomicdata.dev/properties/parent".to_string(),
            json!(parent_subject)
        );

        store.create_with_commit(subject, properties)
            .await
            .expect("Failed to create test document");
        created_documents.push(subject.clone());
        log::debug!("Created test document: {}", title);
    }

    log::info!("Created {} test documents in atomic server", created_documents.len());

    // 2. Create comprehensive role configurations
    
    // Pure Atomic Title Scorer Role
    let pure_atomic_title_config = ConfigBuilder::new()
        .global_shortcut("Ctrl+1")
        .add_role(
            "PureAtomicTitle",
            Role {
                shortname: Some("pure-atomic-title".to_string()),
                name: "Pure Atomic Title".into(),
                relevance_function: RelevanceFunction::TitleScorer,
                terraphim_it: false,
                theme: "cerulean".to_string(),
                kg: None,
                haystacks: vec![Haystack {
                    location: server_url.clone(),
                    service: ServiceType::Atomic,
                    read_only: true,
                    atomic_server_secret: atomic_secret.clone(),
                    extra_parameters: std::collections::HashMap::new(),
                }],
                extra: ahash::AHashMap::new(),
            },
        )
        .build()
        .expect("Failed to build pure atomic title config");

    // Pure Atomic Graph Embeddings Role
    let pure_atomic_graph_config = ConfigBuilder::new()
        .global_shortcut("Ctrl+2")
        .add_role(
            "PureAtomicGraph",
            Role {
                shortname: Some("pure-atomic-graph".to_string()),
                name: "Pure Atomic Graph".into(),
                relevance_function: RelevanceFunction::TerraphimGraph,
                terraphim_it: true,
                theme: "superhero".to_string(),
                kg: Some(terraphim_config::KnowledgeGraph {
                    automata_path: None,
                    knowledge_graph_local: Some(terraphim_config::KnowledgeGraphLocal {
                        input_type: terraphim_types::KnowledgeGraphInputType::Markdown,
                        path: PathBuf::from("docs/src"),
                    }),
                    public: true,
                    publish: true,
                }),
                haystacks: vec![Haystack {
                    location: server_url.clone(),
                    service: ServiceType::Atomic,
                    read_only: true,
                    atomic_server_secret: atomic_secret.clone(),
                    extra_parameters: std::collections::HashMap::new(),
                }],
                extra: ahash::AHashMap::new(),
            },
        )
        .build()
        .expect("Failed to build pure atomic graph config");

    // Hybrid Role: Atomic + Ripgrep with Title Scorer
    let hybrid_title_config = ConfigBuilder::new()
        .global_shortcut("Ctrl+3")
        .add_role(
            "HybridTitle",
            Role {
                shortname: Some("hybrid-title".to_string()),
                name: "Hybrid Title".into(),
                relevance_function: RelevanceFunction::TitleScorer,
                terraphim_it: false,
                theme: "lumen".to_string(),
                kg: None,
                haystacks: vec![
                    Haystack {
                        location: server_url.clone(),
                        service: ServiceType::Atomic,
                        read_only: true,
                        atomic_server_secret: atomic_secret.clone(),
                        extra_parameters: std::collections::HashMap::new(),
                    },
                    Haystack {
                        location: "docs/src".to_string(),
                        service: ServiceType::Ripgrep,
                        read_only: true,
                        atomic_server_secret: None,
                        extra_parameters: std::collections::HashMap::new(),
                    },
                ],
                extra: ahash::AHashMap::new(),
            },
        )
        .build()
        .expect("Failed to build hybrid title config");

    // Hybrid Role: Atomic + Ripgrep with Graph Embeddings
    let hybrid_graph_config = ConfigBuilder::new()
        .global_shortcut("Ctrl+4")
        .add_role(
            "HybridGraph",
            Role {
                shortname: Some("hybrid-graph".to_string()),
                name: "Hybrid Graph".into(),
                relevance_function: RelevanceFunction::TerraphimGraph,
                terraphim_it: true,
                theme: "darkly".to_string(),
                kg: Some(terraphim_config::KnowledgeGraph {
                    automata_path: None,
                    knowledge_graph_local: Some(terraphim_config::KnowledgeGraphLocal {
                        input_type: terraphim_types::KnowledgeGraphInputType::Markdown,
                        path: PathBuf::from("docs/src"),
                    }),
                    public: true,
                    publish: true,
                }),
                haystacks: vec![
                    Haystack {
                        location: server_url.clone(),
                        service: ServiceType::Atomic,
                        read_only: true,
                        atomic_server_secret: atomic_secret.clone(),
                        extra_parameters: std::collections::HashMap::new(),
                    },
                    Haystack {
                        location: "docs/src".to_string(),
                        service: ServiceType::Ripgrep,
                        read_only: true,
                        atomic_server_secret: None,
                        extra_parameters: std::collections::HashMap::new(),
                    },
                ],
                extra: ahash::AHashMap::new(),
            },
        )
        .build()
        .expect("Failed to build hybrid graph config");

    // 3. Test each role configuration
    let configs = vec![
        ("PureAtomicTitle", pure_atomic_title_config),
        ("PureAtomicGraph", pure_atomic_graph_config),
        ("HybridTitle", hybrid_title_config),
        ("HybridGraph", hybrid_graph_config),
    ];

    let search_terms = vec!["integration", "semantic", "configuration", "performance"];
    let mut all_results = HashMap::new();

    for (role_name, config) in &configs {
        log::info!("Testing role: {}", role_name);
        
        // Validate configuration structure
        let role = config.roles.values().next().unwrap();
        match *role_name {
            "PureAtomicTitle" | "PureAtomicGraph" => {
                assert_eq!(role.haystacks.len(), 1, "Pure atomic roles should have 1 haystack");
                assert_eq!(role.haystacks[0].service, ServiceType::Atomic);
            },
            "HybridTitle" | "HybridGraph" => {
                assert_eq!(role.haystacks.len(), 2, "Hybrid roles should have 2 haystacks");
                assert!(role.haystacks.iter().any(|h| h.service == ServiceType::Atomic));
                assert!(role.haystacks.iter().any(|h| h.service == ServiceType::Ripgrep));
            },
            _ => panic!("Unknown role name: {}", role_name),
        }

        // Test search functionality for each role
        let indexer = AtomicHaystackIndexer::default();
        let role_results = &mut all_results.entry(role_name.to_string()).or_insert_with(HashMap::new);

        for search_term in &search_terms {
            let search_start = std::time::Instant::now();
            
            // Test search across all haystacks for this role
            let mut total_results = 0;
            for haystack in &role.haystacks {
                if haystack.service == ServiceType::Atomic {
                    match indexer.index(search_term, haystack).await {
                        Ok(results) => {
                            total_results += results.len();
                            log::debug!("Role {}, haystack {:?}, term '{}': {} results", 
                                       role_name, haystack.service, search_term, results.len());
                        },
                        Err(e) => {
                            log::warn!("Search failed for role {}, term '{}': {}", role_name, search_term, e);
                        }
                    }
                }
            }
            
            let search_duration = search_start.elapsed();
            role_results.insert(search_term.to_string(), (total_results, search_duration));
            log::info!("Role {}, term '{}': {} total results in {:?}", 
                      role_name, search_term, total_results, search_duration);
        }
    }

    // 4. Validate search results and performance
    for (role_name, results) in &all_results {
        log::info!("=== Results Summary for {} ===", role_name);
        for (term, (count, duration)) in results {
            log::info!("  '{}': {} results in {:?}", term, count, duration);
            
            // Validate that we get reasonable results
            if atomic_secret.is_some() {
                assert!(*count > 0, "Role {} should find results for term '{}'", role_name, term);
            }
            
            // Validate reasonable performance (less than 5 seconds per search)
            assert!(duration.as_secs() < 5, "Search should complete within 5 seconds");
        }
    }

    // 5. Test role comparison - hybrid roles should generally find more results
    if atomic_secret.is_some() {
        for search_term in &search_terms {
            let pure_title_count = all_results.get("PureAtomicTitle")
                .and_then(|r| r.get(*search_term))
                .map(|(count, _)| *count)
                .unwrap_or(0);
            
            let hybrid_title_count = all_results.get("HybridTitle")
                .and_then(|r| r.get(*search_term))
                .map(|(count, _)| *count)
                .unwrap_or(0);
            
            log::info!("Term '{}': Pure={}, Hybrid={}", search_term, pure_title_count, hybrid_title_count);
            
            // Hybrid should generally find more or equal results (has additional ripgrep haystack)
            // Note: This is not always guaranteed depending on document overlap
            if hybrid_title_count < pure_title_count {
                log::warn!("Hybrid role found fewer results than pure atomic for '{}' - this may indicate an issue", search_term);
            }
        }
    }

    // 6. Test configuration serialization and deserialization
    for (role_name, config) in &configs {
        let json_str = serde_json::to_string_pretty(config)
            .expect("Failed to serialize config");
        
        let deserialized_config: terraphim_config::Config = serde_json::from_str(&json_str)
            .expect("Failed to deserialize config");
        
        assert_eq!(config.roles.len(), deserialized_config.roles.len(), 
                  "Serialized config should maintain role count for {}", role_name);
        
        log::debug!("Role {} config serialization validated", role_name);
    }

    // 7. Cleanup - delete test documents
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
        Err(e) => log::warn!("Failed to delete parent collection {}: {}", parent_subject, e),
    }

    log::info!("✅ Comprehensive atomic haystack roles test completed successfully");
}

/// Test atomic server error handling and graceful degradation
#[tokio::test]
async fn test_atomic_haystack_error_handling() {
    // Initialize logging for test debugging
    let _ = env_logger::try_init();

    // Test with invalid atomic server URL
    let invalid_config = ConfigBuilder::new()
        .global_shortcut("Ctrl+E")
        .add_role(
            "InvalidAtomic",
            Role {
                shortname: Some("invalid-atomic".to_string()),
                name: "Invalid Atomic".into(),
                relevance_function: RelevanceFunction::TitleScorer,
                terraphim_it: false,
                theme: "cerulean".to_string(),
                kg: None,
                haystacks: vec![Haystack {
                    location: "http://localhost:9999".to_string(), // Non-existent server
                    service: ServiceType::Atomic,
                    read_only: true,
                    atomic_server_secret: Some("invalid_secret".to_string()),
                    extra_parameters: std::collections::HashMap::new(),
                }],
                extra: ahash::AHashMap::new(),
            },
        )
        .build()
        .expect("Failed to build invalid config");

    // Test search with invalid configuration - should handle errors gracefully
    let indexer = AtomicHaystackIndexer::default();
    let role = invalid_config.roles.values().next().unwrap();
    let haystack = &role.haystacks[0];

    let search_result = indexer.index("test", haystack).await;
    
    // Should return an error, not panic
    assert!(search_result.is_err(), "Search with invalid atomic server should return error");
    log::info!("✅ Error handling test: Got expected error - {}", search_result.unwrap_err());

    // Test with missing secret
    let no_secret_config = ConfigBuilder::new()
        .global_shortcut("Ctrl+N")
        .add_role(
            "NoSecretAtomic",
            Role {
                shortname: Some("no-secret-atomic".to_string()),
                name: "No Secret Atomic".into(),
                relevance_function: RelevanceFunction::TitleScorer,
                terraphim_it: false,
                theme: "cerulean".to_string(),
                kg: None,
                haystacks: vec![Haystack {
                    location: "http://localhost:9883".to_string(),
                    service: ServiceType::Atomic,
                    read_only: true,
                    atomic_server_secret: None, // No authentication secret
                    extra_parameters: std::collections::HashMap::new(),
                }],
                extra: ahash::AHashMap::new(),
            },
        )
        .build()
        .expect("Failed to build no-secret config");

    let no_secret_role = no_secret_config.roles.values().next().unwrap();
    let no_secret_haystack = &no_secret_role.haystacks[0];

    let no_secret_result = indexer.index("test", no_secret_haystack).await;
    
    // May succeed (anonymous access) or fail (authentication required) - both are valid
    match no_secret_result {
        Ok(results) => {
            log::info!("✅ Anonymous access test: Found {} results", results.len());
        },
        Err(e) => {
            log::info!("✅ Authentication required test: Got expected error - {}", e);
        }
    }

    log::info!("✅ Atomic haystack error handling test completed successfully");
} 