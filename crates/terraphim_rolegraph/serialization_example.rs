//! Example demonstrating RoleGraph serialization capabilities
//!
//! This example shows how to:
//! - Create a RoleGraph
//! - Add documents to it
//! - Serialize it to JSON
//! - Deserialize it back to a RoleGraph
//! - Use RoleGraphSync with serialization

use terraphim_rolegraph::{RoleGraph, RoleGraphSync, SerializableRoleGraph};
use terraphim_types::{Document, RoleName};
use ulid::Ulid;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    env_logger::init();

    // Create a simple thesaurus for demonstration
    let mut thesaurus = terraphim_types::Thesaurus::new("example".to_string());

    // Add some sample terms to the thesaurus
    let life_cycle_term = terraphim_types::NormalizedTerm::new(
        1,
        terraphim_types::NormalizedTermValue::new("life cycle".to_string())
    );
    let project_term = terraphim_types::NormalizedTerm::new(
        2,
        terraphim_types::NormalizedTermValue::new("project".to_string())
    );
    let planning_term = terraphim_types::NormalizedTerm::new(
        3,
        terraphim_types::NormalizedTermValue::new("planning".to_string())
    );

    thesaurus.insert(
        terraphim_types::NormalizedTermValue::new("life cycle".to_string()),
        life_cycle_term
    );
    thesaurus.insert(
        terraphim_types::NormalizedTermValue::new("project".to_string()),
        project_term
    );
    thesaurus.insert(
        terraphim_types::NormalizedTermValue::new("planning".to_string()),
        planning_term
    );

    println!("🚀 Creating RoleGraph with thesaurus containing {} terms", thesaurus.len());

    // Create a RoleGraph
    let role = RoleName::new("example");
    let mut rolegraph = RoleGraph::new(role, thesaurus).await?;

    // Add some documents
    let document_id = Ulid::new().to_string();
    let document = Document {
        id: document_id.clone(),
        title: "Example Document".to_string(),
        body: "This document discusses life cycle management and project planning processes.".to_string(),
        url: "/example/document".to_string(),
        description: Some("An example document for serialization testing".to_string()),
        tags: Some(vec!["example".to_string(), "serialization".to_string()]),
        rank: Some(1),
        stub: None,
        summarization: None,
        source_haystack: None,
    };

    rolegraph.insert_document(&document_id, document);
    println!("📝 Added document to RoleGraph");

    // Get graph statistics
    let stats = rolegraph.get_graph_stats();
    println!("📊 Graph Statistics:");
    println!("  - Nodes: {}", stats.node_count);
    println!("  - Edges: {}", stats.edge_count);
    println!("  - Documents: {}", stats.document_count);
    println!("  - Thesaurus size: {}", stats.thesaurus_size);

    // Demonstrate basic RoleGraph serialization
    println!("\n🔄 Serializing RoleGraph...");
    let serializable = rolegraph.to_serializable();
    let json_str = serializable.to_json()?;
    println!("✅ Serialized to JSON ({} bytes)", json_str.len());

    // Show a sample of the JSON
    let json_preview = if json_str.len() > 200 {
        format!("{}...", &json_str[..200])
    } else {
        json_str.clone()
    };
    println!("📄 JSON Preview: {}", json_preview);

    // Deserialize back to RoleGraph
    println!("\n🔄 Deserializing from JSON...");
    let deserialized = SerializableRoleGraph::from_json(&json_str)?;
    let restored_rolegraph = RoleGraph::from_serializable(deserialized).await?;
    println!("✅ Successfully restored RoleGraph");

    // Verify the restoration
    let restored_stats = restored_rolegraph.get_graph_stats();
    println!("📊 Restored Graph Statistics:");
    println!("  - Nodes: {}", restored_stats.node_count);
    println!("  - Edges: {}", restored_stats.edge_count);
    println!("  - Documents: {}", restored_stats.document_count);
    println!("  - Thesaurus size: {}", restored_stats.thesaurus_size);

    // Demonstrate RoleGraphSync serialization
    println!("\n🔄 Demonstrating RoleGraphSync serialization...");
    let rolegraph_sync = RoleGraphSync::from(rolegraph);
    let sync_json = rolegraph_sync.to_json().await?;
    println!("✅ RoleGraphSync serialized to JSON ({} bytes)", sync_json.len());

    // Restore from RoleGraphSync
    let restored_sync = RoleGraphSync::from_json(&sync_json).await?;
    let _guard = restored_sync.lock().await;
    println!("✅ RoleGraphSync successfully restored");

    // Test search functionality
    println!("\n🔍 Testing search functionality...");
    let search_results = restored_rolegraph.query_graph("life cycle", None, Some(10))?;
    println!("📊 Search results for 'life cycle': {} documents found", search_results.len());

    let automata_matches = restored_rolegraph.find_matching_node_ids("project planning");
    println!("🔤 Aho-Corasick matches for 'project planning': {} terms found", automata_matches.len());

    println!("\n🎉 Serialization example completed successfully!");

    Ok(())
}
