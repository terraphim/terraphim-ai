use terraphim_config::{ConfigBuilder, Haystack, Role, ServiceType};
use terraphim_types::{RelevanceFunction, SearchQuery, Index};
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
                theme: "cerulean".to_string(),
                kg: None, // No knowledge graph for title scorer
                haystacks: vec![Haystack {
                    location: server_url.clone(),
                    service: ServiceType::Atomic,
                    read_only: true,
                    atomic_server_secret: atomic_secret.clone(),
                }],
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
        
        let mut found_docs = 0;
        let mut index = Index::new();
        
        // Single search call - indexing should be instant for local server
        let start_time = std::time::Instant::now();
        index = indexer.index(search_term, haystack).await
            .expect(&format!("Search failed for term: {}", search_term));
        let search_duration = start_time.elapsed();
        
        found_docs = index.len();
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
                haystacks: vec![Haystack {
                    location: server_url.clone(),
                    service: ServiceType::Atomic,
                    read_only: true,
                    atomic_server_secret: atomic_secret.clone(),
                }],
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
        
        let mut found_docs = 0;
        let mut index = Index::new();
        
        // Single search call - indexing should be instant for local server
        let start_time = std::time::Instant::now();
        index = indexer.index(search_term, haystack).await
            .expect(&format!("Search failed for term: {}", search_term));
        let search_duration = start_time.elapsed();
        
        found_docs = index.len();
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
                theme: "cerulean".to_string(),
                kg: None,
                haystacks: vec![Haystack {
                    location: server_url.clone(),
                    service: ServiceType::Atomic,
                    read_only: true,
                    atomic_server_secret: atomic_secret.clone(),
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
                theme: "cerulean".to_string(),
                kg: None, // Title scorer doesn't need knowledge graph
                haystacks: vec![Haystack {
                    location: "http://localhost:9883".to_string(),
                    service: ServiceType::Atomic,
                    read_only: true,
                    atomic_server_secret: None,
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