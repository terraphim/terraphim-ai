use serial_test::serial;
use tempfile::TempDir;
use tokio::fs;

use terraphim_middleware::thesaurus::{Logseq, ThesaurusBuilder};
use terraphim_rolegraph::RoleGraph;
use terraphim_types::{Document, RoleName};

/// Test for knowledge graph ranking expansion that validates:
/// 1. KG construction from docs/src/kg
/// 2. Node and edge counting
/// 3. Adding new records with defined synonyms
/// 4. Verifying nodes/edges changed
/// 5. Validating "terraphim-graph" rank changed using Terraphim Engineer role
#[tokio::test]
#[serial]
async fn test_knowledge_graph_ranking_expansion() {
    env_logger::init();

    // 1. Setup test environment with correct paths
    let current_dir = std::env::current_dir().unwrap();
    let project_root = current_dir.parent().unwrap().parent().unwrap();
    let docs_src_path = project_root.join("docs/src");
    let original_kg_path = docs_src_path.join("kg");

    // Verify that the test files exist
    assert!(
        original_kg_path.exists(),
        "Knowledge graph directory should exist: {:?}",
        original_kg_path
    );
    assert!(
        original_kg_path.join("terraphim-graph.md").exists(),
        "terraphim-graph.md should exist"
    );

    // 2. Create temporary directory and copy existing KG files
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let temp_kg_path = temp_dir.path().join("kg");
    fs::create_dir_all(&temp_kg_path)
        .await
        .expect("Failed to create temp kg dir");

    // Copy existing KG files to temp directory
    let mut original_files = Vec::new();
    let mut entries = fs::read_dir(&original_kg_path)
        .await
        .expect("Failed to read kg directory");
    while let Some(entry) = entries.next_entry().await.expect("Failed to read entry") {
        if entry.path().extension().is_some_and(|ext| ext == "md") {
            let file_name = entry.file_name();
            let source_path = entry.path();
            let dest_path = temp_kg_path.join(&file_name);
            fs::copy(&source_path, &dest_path)
                .await
                .expect("Failed to copy file");
            original_files.push(file_name.to_string_lossy().to_string());
        }
    }

    println!(
        "📁 Copied {} original KG files to temp directory",
        original_files.len()
    );
    println!("   Original files: {:?}", original_files);

    // 3. Build initial knowledge graph and count nodes/edges
    println!("\n🔧 Building initial knowledge graph...");
    let logseq_builder = Logseq::default();
    let initial_thesaurus = logseq_builder
        .build("Terraphim Engineer".to_string(), &temp_kg_path)
        .await
        .expect("Failed to build initial thesaurus");

    let initial_thesaurus_size = initial_thesaurus.len();
    println!(
        "📊 Initial thesaurus contains {} terms",
        initial_thesaurus_size
    );

    // Print initial thesaurus contents for debugging
    println!("📋 Initial thesaurus terms:");
    for (term, normalized_term) in &initial_thesaurus {
        println!(
            "   '{}' -> '{}' (ID: {})",
            term.as_str(),
            normalized_term.value.as_str(),
            normalized_term.id
        );
    }

    // 4. Create initial RoleGraph and count nodes/edges
    let role_name = RoleName::new("Terraphim Engineer");
    let mut initial_rolegraph = RoleGraph::new(role_name.clone(), initial_thesaurus.clone())
        .await
        .expect("Failed to create initial RoleGraph");

    // Index initial documents into the rolegraph
    let mut initial_documents = Vec::new();
    let mut entries = fs::read_dir(&temp_kg_path)
        .await
        .expect("Failed to read temp kg directory");
    while let Some(entry) = entries.next_entry().await.expect("Failed to read entry") {
        if entry.path().extension().is_some_and(|ext| ext == "md") {
            let content = fs::read_to_string(&entry.path())
                .await
                .expect("Failed to read file");
            let file_stem = entry
                .path()
                .file_stem()
                .unwrap()
                .to_string_lossy()
                .to_string();

            let document = Document {
                id: file_stem.clone(),
                url: entry.path().to_string_lossy().to_string(),
                title: file_stem.clone(),
                body: content,
                description: None,
                summarization: None,
                stub: None,
                tags: None,
                rank: None,
                source_haystack: None,
            };

            initial_rolegraph.insert_document(&document.id, document.clone());
            initial_documents.push(document);
        }
    }

    let initial_nodes_count = initial_rolegraph.nodes_map().len();
    let initial_edges_count = initial_rolegraph.edges_map().len();

    println!("📊 Initial RoleGraph stats:");
    println!("   Nodes: {}", initial_nodes_count);
    println!("   Edges: {}", initial_edges_count);
    println!("   Documents: {}", initial_documents.len());

    // 5. Test initial ranking for "terraphim-graph"
    println!("\n🔍 Testing initial ranking for 'terraphim-graph'...");
    let initial_results = initial_rolegraph
        .query_graph("terraphim-graph", Some(0), Some(10))
        .expect("Initial query should succeed");

    let initial_rank = if let Some((_, indexed_doc)) = initial_results.first() {
        indexed_doc.rank
    } else {
        0 // No results found
    };

    println!("📈 Initial rank for 'terraphim-graph': {}", initial_rank);

    // 6. Add new record with defined synonyms to knowledge graph
    println!("\n➕ Adding new knowledge graph record with synonyms...");
    let new_kg_file_path = temp_kg_path.join("graph-analysis.md");
    let new_kg_content = r#"# Graph Analysis

## Advanced Graph Processing

Graph Analysis is a comprehensive approach to understanding complex data relationships and structures within knowledge graphs.

synonyms:: data analysis, network analysis, graph processing, relationship mapping, connectivity analysis, terraphim-graph, graph embeddings

This concept extends the capabilities of graph-based systems by providing deeper insights into data relationships and semantic connections.

## Key Features

- Advanced relationship detection
- Semantic connectivity mapping
- Dynamic graph structure analysis
- Knowledge pattern recognition

The Graph Analysis component works closely with existing graph processing systems to enhance overall system capabilities.
"#;

    fs::write(&new_kg_file_path, new_kg_content)
        .await
        .expect("Failed to write new KG file");

    println!("📝 Created new KG file: graph-analysis.md");
    println!("🔗 New synonyms: data analysis, network analysis, graph processing, relationship mapping, connectivity analysis, terraphim-graph, graph embeddings");

    // 7. Rebuild knowledge graph with new content
    println!("\n🔧 Rebuilding knowledge graph with new content...");
    let expanded_thesaurus = logseq_builder
        .build("Terraphim Engineer".to_string(), &temp_kg_path)
        .await
        .expect("Failed to build expanded thesaurus");

    let expanded_thesaurus_size = expanded_thesaurus.len();
    println!(
        "📊 Expanded thesaurus contains {} terms",
        expanded_thesaurus_size
    );

    // Print new thesaurus contents for comparison
    println!("📋 Expanded thesaurus terms:");
    for (term, normalized_term) in &expanded_thesaurus {
        println!(
            "   '{}' -> '{}' (ID: {})",
            term.as_str(),
            normalized_term.value.as_str(),
            normalized_term.id
        );
    }

    // 8. Create expanded RoleGraph and count nodes/edges
    let mut expanded_rolegraph = RoleGraph::new(role_name.clone(), expanded_thesaurus.clone())
        .await
        .expect("Failed to create expanded RoleGraph");

    // Index all documents (including new one) into the expanded rolegraph
    let mut expanded_documents = Vec::new();
    let mut entries = fs::read_dir(&temp_kg_path)
        .await
        .expect("Failed to read temp kg directory");
    while let Some(entry) = entries.next_entry().await.expect("Failed to read entry") {
        if entry.path().extension().is_some_and(|ext| ext == "md") {
            let content = fs::read_to_string(&entry.path())
                .await
                .expect("Failed to read file");
            let file_stem = entry
                .path()
                .file_stem()
                .unwrap()
                .to_string_lossy()
                .to_string();

            let document = Document {
                id: file_stem.clone(),
                url: entry.path().to_string_lossy().to_string(),
                title: file_stem.clone(),
                body: content,
                description: None,
                summarization: None,
                stub: None,
                tags: None,
                rank: None,
                source_haystack: None,
            };

            expanded_rolegraph.insert_document(&document.id, document.clone());
            expanded_documents.push(document);
        }
    }

    let expanded_nodes_count = expanded_rolegraph.nodes_map().len();
    let expanded_edges_count = expanded_rolegraph.edges_map().len();

    println!("📊 Expanded RoleGraph stats:");
    println!("   Nodes: {}", expanded_nodes_count);
    println!("   Edges: {}", expanded_edges_count);
    println!("   Documents: {}", expanded_documents.len());

    // 9. Test expanded ranking for "terraphim-graph"
    println!("\n🔍 Testing expanded ranking for 'terraphim-graph'...");
    let expanded_results = expanded_rolegraph
        .query_graph("terraphim-graph", Some(0), Some(10))
        .expect("Expanded query should succeed");

    let expanded_rank = if let Some((_, indexed_doc)) = expanded_results.first() {
        indexed_doc.rank
    } else {
        0 // No results found
    };

    println!("📈 Expanded rank for 'terraphim-graph': {}", expanded_rank);

    // 10. Validation assertions
    println!("\n✅ Validating knowledge graph expansion results...");

    // Verify thesaurus grew
    assert!(
        expanded_thesaurus_size > initial_thesaurus_size,
        "Thesaurus should have grown from {} to {} terms",
        initial_thesaurus_size,
        expanded_thesaurus_size
    );
    println!(
        "✅ Thesaurus grew: {} -> {} terms (+{})",
        initial_thesaurus_size,
        expanded_thesaurus_size,
        expanded_thesaurus_size - initial_thesaurus_size
    );

    // Verify nodes increased
    assert!(
        expanded_nodes_count > initial_nodes_count,
        "Nodes should have increased from {} to {}",
        initial_nodes_count,
        expanded_nodes_count
    );
    println!(
        "✅ Nodes increased: {} -> {} (+{})",
        initial_nodes_count,
        expanded_nodes_count,
        expanded_nodes_count - initial_nodes_count
    );

    // Verify edges increased
    assert!(
        expanded_edges_count > initial_edges_count,
        "Edges should have increased from {} to {}",
        initial_edges_count,
        expanded_edges_count
    );
    println!(
        "✅ Edges increased: {} -> {} (+{})",
        initial_edges_count,
        expanded_edges_count,
        expanded_edges_count - initial_edges_count
    );

    // Verify documents increased
    assert!(
        expanded_documents.len() > initial_documents.len(),
        "Documents should have increased from {} to {}",
        initial_documents.len(),
        expanded_documents.len()
    );
    println!(
        "✅ Documents increased: {} -> {} (+{})",
        initial_documents.len(),
        expanded_documents.len(),
        expanded_documents.len() - initial_documents.len()
    );

    // Verify rank changed (should increase due to more connections)
    assert_ne!(
        expanded_rank, initial_rank,
        "Rank should have changed from {} to {}",
        initial_rank, expanded_rank
    );
    println!(
        "✅ Rank changed: {} -> {} ({}{})",
        initial_rank,
        expanded_rank,
        if expanded_rank > initial_rank {
            "+"
        } else {
            ""
        },
        (expanded_rank as i64) - (initial_rank as i64)
    );

    // 11. Test that new synonyms are searchable
    println!("\n🔍 Testing new synonyms are searchable...");
    let new_synonym_tests = vec![
        "data analysis",
        "network analysis",
        "graph processing",
        "relationship mapping",
        "connectivity analysis",
        "graph embeddings",
    ];

    for synonym in &new_synonym_tests {
        let results = expanded_rolegraph
            .query_graph(synonym, Some(0), Some(5))
            .expect("New synonym query should succeed");

        assert!(
            !results.is_empty(),
            "Should find results for new synonym: '{}'",
            synonym
        );
        println!(
            "✅ Found {} results for synonym: '{}'",
            results.len(),
            synonym
        );
    }

    // 12. Verify Terraphim Engineer role configuration is used correctly
    assert_eq!(
        role_name.original.as_str(),
        "Terraphim Engineer",
        "Should be using Terraphim Engineer role"
    );
    println!("✅ Using correct role: {}", role_name.original.as_str());

    // 13. Final summary
    println!("\n🎉 Knowledge Graph Ranking Expansion Test Complete!");
    println!(
        "   📊 Initial State: {} terms, {} nodes, {} edges, rank {}",
        initial_thesaurus_size, initial_nodes_count, initial_edges_count, initial_rank
    );
    println!(
        "   📈 Expanded State: {} terms, {} nodes, {} edges, rank {}",
        expanded_thesaurus_size, expanded_nodes_count, expanded_edges_count, expanded_rank
    );
    println!(
        "   🚀 Growth: +{} terms, +{} nodes, +{} edges, rank change: {}",
        expanded_thesaurus_size - initial_thesaurus_size,
        expanded_nodes_count - initial_nodes_count,
        expanded_edges_count - initial_edges_count,
        (expanded_rank as i64) - (initial_rank as i64)
    );

    println!("✅ All validations passed!");
}
