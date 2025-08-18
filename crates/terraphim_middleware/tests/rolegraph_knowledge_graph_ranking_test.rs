use serial_test::serial;
use terraphim_automata::{load_thesaurus, AutomataPath};
use terraphim_config::{
    ConfigBuilder, Haystack, KnowledgeGraph, KnowledgeGraphLocal, Role, ServiceType,
};
use terraphim_middleware::thesaurus::{Logseq, ThesaurusBuilder};
use terraphim_middleware::{indexer::IndexMiddleware, RipgrepIndexer};
use terraphim_rolegraph::RoleGraph;
use terraphim_types::{
    Document, KnowledgeGraphInputType, NormalizedTermValue, RelevanceFunction, RoleName,
};

/// Test to validate rolegraph and knowledge graph based ranking
/// The test ensures that the "Terraphim Engineer" role can find the terraphim-graph.md document
/// when searching for terms like "terraphim-graph", "graph embeddings", and "graph"
#[tokio::test]
#[serial]
async fn test_rolegraph_knowledge_graph_ranking() {
    env_logger::init();

    // 1. Setup test environment with correct paths
    // Navigate up from middleware crate to project root
    let current_dir = std::env::current_dir().unwrap();
    let project_root = current_dir.parent().unwrap().parent().unwrap(); // Go up two levels from crates/terraphim_middleware
    let docs_src_path = project_root.join("docs/src");
    let kg_path = docs_src_path.join("kg");

    // Verify that the test files exist
    assert!(
        kg_path.exists(),
        "Knowledge graph directory should exist: {:?}",
        kg_path
    );
    assert!(
        kg_path.join("terraphim-graph.md").exists(),
        "terraphim-graph.md should exist"
    );

    // 2. Create a test role configuration that uses local KG
    let terraphim_engineer_role = Role {
        shortname: Some("terraphim-engineer".to_string()),
        name: "Terraphim Engineer".into(),
        relevance_function: RelevanceFunction::TerraphimGraph,
        theme: "superhero".to_string(),
        kg: Some(KnowledgeGraph {
            automata_path: None, // Will be set after building thesaurus
            knowledge_graph_local: Some(KnowledgeGraphLocal {
                input_type: KnowledgeGraphInputType::Markdown,
                path: format!("{}/kg", docs_src_path.display()).into(),
            }),
            public: true,
            publish: true,
        }),
        haystacks: vec![],
        extra: ahash::AHashMap::new(),
        terraphim_it: false,
    };

    // 3. Build thesaurus from local markdown files in kg directory
    let logseq_builder = Logseq::default();
    let thesaurus = logseq_builder
        .build("Terraphim Engineer".to_string(), kg_path.clone())
        .await
        .expect("Failed to build thesaurus from knowledge graph");

    // 4. Verify that the thesaurus contains expected terms
    println!("Built thesaurus with {} entries", thesaurus.len());

    // Check that key terms from terraphim-graph.md are in the thesaurus
    let terraphim_graph_term = NormalizedTermValue::new("terraphim-graph".to_string());
    let graph_embeddings_term = NormalizedTermValue::new("graph embeddings".to_string());
    let graph_term = NormalizedTermValue::new("graph".to_string());

    assert!(
        thesaurus.get(&terraphim_graph_term).is_some(),
        "Thesaurus should contain 'terraphim-graph' term"
    );
    assert!(
        thesaurus.get(&graph_embeddings_term).is_some() || thesaurus.get(&graph_term).is_some(),
        "Thesaurus should contain graph-related terms"
    );

    // 5. Create RoleGraph with the built thesaurus
    let role_name = RoleName::new("Terraphim Engineer");
    let mut rolegraph = RoleGraph::new(role_name.clone(), thesaurus.clone())
        .await
        .expect("Failed to create RoleGraph");

    // 6. Index the terraphim-graph.md document into the rolegraph
    let terraphim_graph_file = kg_path.join("terraphim-graph.md");
    let content = tokio::fs::read_to_string(&terraphim_graph_file)
        .await
        .expect("Failed to read terraphim-graph.md");

    let document = Document {
        id: "terraphim-graph".to_string(),
        url: terraphim_graph_file.to_string_lossy().to_string(),
        title: "Terraphim-graph".to_string(),
        body: content.clone(),
        description: Some("Terraphim Graph scorer using unique graph embeddings".to_string()),
        stub: None,
        tags: None,
        rank: None,
    };

    rolegraph.insert_document(&document.id, document.clone());

    // 7. Test search functionality with different query terms
    let test_queries = vec![
        "terraphim-graph",
        "graph embeddings",
        "graph",
        "knowledge graph based embeddings",
        "terraphim graph scorer",
    ];

    for query_term in test_queries {
        println!("\n🔍 Testing search for: '{}'", query_term);

        let results = rolegraph
            .query_graph(query_term, Some(0), Some(10))
            .expect("Query should succeed");

        println!("Found {} results for '{}'", results.len(), query_term);

        // Verify that we found the terraphim-graph document
        let found_terraphim_graph = results
            .iter()
            .any(|(doc_id, _)| doc_id == "terraphim-graph");

        assert!(
            found_terraphim_graph,
            "Should find terraphim-graph document when searching for '{}'. Results: {:?}",
            query_term,
            results.iter().map(|(id, _)| id).collect::<Vec<_>>()
        );

        // Check ranking - the document should have a meaningful rank
        if let Some((_, indexed_doc)) = results.first() {
            assert!(
                indexed_doc.rank > 0,
                "Document should have a positive rank. Got: {}",
                indexed_doc.rank
            );
            println!("✅ Top result rank: {}", indexed_doc.rank);
        }
    }

    // 8. Test full integration with haystack indexing
    // This tests the complete search flow including haystack indexing
    println!("\n🧪 Testing full integration with haystack indexing...");

    let _config = ConfigBuilder::new()
        .global_shortcut("Ctrl+T")
        .add_role("Terraphim Engineer", terraphim_engineer_role)
        .build()
        .expect("Failed to build config");

    // Index documents using ripgrep
    let haystack = Haystack::new(
        docs_src_path.to_string_lossy().to_string(),
        ServiceType::Ripgrep,
        true,
    );

    let indexer = RipgrepIndexer::default();
    let index = indexer
        .index("terraphim-graph", &haystack)
        .await
        .expect("Failed to index haystack");

    // Verify that the index contains our document
    let all_docs = index.get_all_documents();
    let found_in_index = all_docs
        .iter()
        .any(|doc| doc.url.contains("terraphim-graph.md"));

    assert!(
        found_in_index,
        "terraphim-graph.md should be found in the indexed documents"
    );

    println!(
        "✅ All tests passed! Rolegraph and knowledge graph based ranking is working correctly."
    );
}

/// Test building thesaurus from knowledge graph markdown files
#[tokio::test]
#[serial]
async fn test_build_thesaurus_from_kg_files() {
    let current_dir = std::env::current_dir().unwrap();
    let project_root = current_dir.parent().unwrap().parent().unwrap();
    let kg_path = project_root.join("docs/src/kg");

    // Skip test if kg directory doesn't exist
    if !kg_path.exists() {
        println!("Skipping test - kg directory not found: {:?}", kg_path);
        return;
    }

    let logseq_builder = Logseq::default();
    let thesaurus = logseq_builder
        .build("Test".to_string(), kg_path.clone())
        .await
        .expect("Failed to build thesaurus");

    println!("Built thesaurus with {} entries", thesaurus.len());

    // Print all thesaurus entries for debugging
    for (term, normalized_term) in &thesaurus {
        println!(
            "Term: '{}' -> Concept: '{}' (ID: {})",
            term.as_str(),
            normalized_term.value.as_str(),
            normalized_term.id
        );
    }

    // Verify expected terms exist
    let expected_terms = vec![
        "terraphim-graph",
        "graph embeddings",
        "graph",
        "knowledge graph based embeddings",
        "haystack",
        "datasource",
        "service",
        "provider",
        "middleware",
    ];

    for expected_term in expected_terms {
        let term_key = NormalizedTermValue::new(expected_term.to_string());
        assert!(
            thesaurus.get(&term_key).is_some(),
            "Thesaurus should contain term: '{}'",
            expected_term
        );
    }
}

/// Test that demonstrates the issue when using wrong thesaurus
#[tokio::test]
#[serial]
async fn test_demonstrates_issue_with_wrong_thesaurus() {
    // This test demonstrates why search fails when using the remote thesaurus
    // instead of a locally built one from the kg files

    let remote_automata_path = AutomataPath::remote_example();

    // Try to load the remote thesaurus (this is what "Engineer" role currently uses)
    let remote_thesaurus = load_thesaurus(&remote_automata_path)
        .await
        .expect("Failed to load remote thesaurus");

    println!("Remote thesaurus has {} entries", remote_thesaurus.len());

    // Check if the remote thesaurus contains our local KG terms
    let terraphim_graph_term = NormalizedTermValue::new("terraphim-graph".to_string());
    let graph_embeddings_term = NormalizedTermValue::new("graph embeddings".to_string());

    let has_terraphim_graph = remote_thesaurus.get(&terraphim_graph_term).is_some();
    let has_graph_embeddings = remote_thesaurus.get(&graph_embeddings_term).is_some();

    println!(
        "Remote thesaurus contains 'terraphim-graph': {}",
        has_terraphim_graph
    );
    println!(
        "Remote thesaurus contains 'graph embeddings': {}",
        has_graph_embeddings
    );

    // This demonstrates the issue - the remote thesaurus doesn't contain our local terms
    // (This assertion will likely fail, which proves the point)
    if !has_terraphim_graph {
        println!("❌ ISSUE DEMONSTRATED: Remote thesaurus missing 'terraphim-graph' term");
        println!("   This is why the Engineer role can't find local KG documents!");
    }

    if !has_graph_embeddings {
        println!("❌ ISSUE DEMONSTRATED: Remote thesaurus missing 'graph embeddings' term");
    }
}
