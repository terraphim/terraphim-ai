//! # Terraphim Graph: Knowledge Graph Ranking in Action
//!
//! This example demonstrates how Terraphim's knowledge graph ranking system works
//! and how adding new terms to the knowledge graph changes document ranking and
//! improves retrieval quality.
//!
//! ## What this example shows:
//!
//! 1. Building a thesaurus (vocabulary of domain concepts)
//! 2. Creating a RoleGraph (knowledge graph for a specific role)
//! 3. Indexing documents into the graph
//! 4. Searching with graph-based ranking
//! 5. Adding new knowledge graph terms and observing rank changes
//! 6. Comparing before/after retrieval quality
//!
//! Run with:
//! ```sh
//! cargo run -p terraphim_rolegraph --example knowledge_graph_ranking
//! ```

use terraphim_rolegraph::RoleGraph;
use terraphim_types::{Document, DocumentType, NormalizedTerm, NormalizedTermValue, Thesaurus};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Print a separator line for readability.
fn separator(title: &str) {
    println!("\n{}", "=".repeat(72));
    println!("  {}", title);
    println!("{}\n", "=".repeat(72));
}

/// Print search results with ranks and matched tags.
fn print_results(results: &[(String, terraphim_types::IndexedDocument)]) {
    if results.is_empty() {
        println!("  (no results)");
        return;
    }
    for (i, (_doc_id, doc)) in results.iter().enumerate() {
        println!(
            "  #{}: id={:<24} rank={:<6} tags={:?}",
            i + 1,
            doc.id,
            doc.rank,
            doc.tags
        );
    }
}

/// Print graph statistics.
fn print_graph_stats(rg: &RoleGraph) {
    let stats = rg.get_graph_stats();
    println!(
        "  Graph: {} nodes, {} edges, {} documents, {} thesaurus terms",
        stats.node_count, stats.edge_count, stats.document_count, stats.thesaurus_size
    );
}

// ---------------------------------------------------------------------------
// Thesaurus construction
// ---------------------------------------------------------------------------

/// Build a small thesaurus about AI agents.
///
/// Each entry maps a search term (synonym) to a normalized concept.
/// Multiple synonyms can point to the same concept ID,
/// which is how the knowledge graph discovers relationships.
fn build_agent_thesaurus() -> Thesaurus {
    let mut thesaurus = Thesaurus::new("AI Agent Domain".to_string());

    // Concept 1: "agent" -- the core concept
    // Synonyms: "agent", "ai agent", "autonomous agent"
    let agent_id = 1;
    let agent_nterm = NormalizedTermValue::from("agent");
    thesaurus.insert(
        NormalizedTermValue::from("agent"),
        NormalizedTerm::new(agent_id, agent_nterm.clone()),
    );
    thesaurus.insert(
        NormalizedTermValue::from("ai agent"),
        NormalizedTerm::new(agent_id, agent_nterm.clone()),
    );
    thesaurus.insert(
        NormalizedTermValue::from("autonomous agent"),
        NormalizedTerm::new(agent_id, agent_nterm.clone()),
    );

    // Concept 2: "planning"
    let planning_id = 2;
    let planning_nterm = NormalizedTermValue::from("planning");
    thesaurus.insert(
        NormalizedTermValue::from("planning"),
        NormalizedTerm::new(planning_id, planning_nterm.clone()),
    );
    thesaurus.insert(
        NormalizedTermValue::from("plan"),
        NormalizedTerm::new(planning_id, planning_nterm.clone()),
    );

    // Concept 3: "reasoning"
    let reasoning_id = 3;
    let reasoning_nterm = NormalizedTermValue::from("reasoning");
    thesaurus.insert(
        NormalizedTermValue::from("reasoning"),
        NormalizedTerm::new(reasoning_id, reasoning_nterm.clone()),
    );
    thesaurus.insert(
        NormalizedTermValue::from("inference"),
        NormalizedTerm::new(reasoning_id, reasoning_nterm.clone()),
    );

    // Concept 4: "tool use"
    let tool_use_id = 4;
    let tool_use_nterm = NormalizedTermValue::from("tool use");
    thesaurus.insert(
        NormalizedTermValue::from("tool use"),
        NormalizedTerm::new(tool_use_id, tool_use_nterm.clone()),
    );
    thesaurus.insert(
        NormalizedTermValue::from("function calling"),
        NormalizedTerm::new(tool_use_id, tool_use_nterm.clone()),
    );

    // Concept 5: "memory"
    let memory_id = 5;
    let memory_nterm = NormalizedTermValue::from("memory");
    thesaurus.insert(
        NormalizedTermValue::from("memory"),
        NormalizedTerm::new(memory_id, memory_nterm.clone()),
    );
    thesaurus.insert(
        NormalizedTermValue::from("context window"),
        NormalizedTerm::new(memory_id, memory_nterm.clone()),
    );

    thesaurus
}

/// Build sample documents about AI agents.
fn build_documents() -> Vec<Document> {
    vec![
        Document {
            id: "doc-react-agent".to_string(),
            title: "ReAct: Reasoning and Acting in AI Agents".to_string(),
            body: "The ReAct framework combines reasoning and acting in AI agent systems. \
                   An agent observes the environment, performs reasoning about the next step, \
                   and then takes an action. This reasoning-action loop is fundamental to \
                   how autonomous agent systems operate. The agent uses planning to decompose \
                   complex tasks."
                .to_string(),
            url: "/docs/react-agent".to_string(),
            description: Some("ReAct framework for AI agents".to_string()),
            doc_type: DocumentType::KgEntry,
            ..Default::default()
        },
        Document {
            id: "doc-tool-agent".to_string(),
            title: "Tool-Augmented AI Agents".to_string(),
            body: "Modern AI agents use tool use and function calling to interact with \
                   external systems. An agent can call APIs, query databases, and execute \
                   code. Tool use extends what an autonomous agent can accomplish beyond \
                   pure text generation."
                .to_string(),
            url: "/docs/tool-agent".to_string(),
            description: Some("How agents use tools".to_string()),
            doc_type: DocumentType::KgEntry,
            ..Default::default()
        },
        Document {
            id: "doc-memory-systems".to_string(),
            title: "Memory Systems for AI Agents".to_string(),
            body: "AI agent memory is a critical component. Short-term memory uses the \
                   context window of the language model. Long-term memory requires external \
                   storage. Memory allows an agent to learn from past interactions and \
                   maintain state across sessions."
                .to_string(),
            url: "/docs/memory-systems".to_string(),
            description: Some("Memory and context in agents".to_string()),
            doc_type: DocumentType::KgEntry,
            ..Default::default()
        },
        Document {
            id: "doc-multi-agent".to_string(),
            title: "Multi-Agent Collaboration".to_string(),
            body: "Multiple AI agents can collaborate on complex tasks through planning \
                   and coordination. Each agent specializes in a different capability. \
                   One agent handles reasoning about the overall plan while another \
                   agent focuses on tool use and function calling to execute individual \
                   steps. Memory sharing between agents enables coherent collaboration."
                .to_string(),
            url: "/docs/multi-agent".to_string(),
            description: Some("Collaboration between multiple agents".to_string()),
            doc_type: DocumentType::KgEntry,
            ..Default::default()
        },
        Document {
            id: "doc-simple-chatbot".to_string(),
            title: "Building a Simple Chatbot".to_string(),
            body: "A chatbot responds to user messages using pattern matching or a language \
                   model. Unlike a full agent, a simple chatbot does not perform planning \
                   or use external tools. It operates in a single request-response turn."
                .to_string(),
            url: "/docs/simple-chatbot".to_string(),
            description: Some("Basic chatbot without agent capabilities".to_string()),
            doc_type: DocumentType::KgEntry,
            ..Default::default()
        },
    ]
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

#[tokio::main]
async fn main() {
    // -----------------------------------------------------------------------
    // PHASE 1: Build initial knowledge graph
    // -----------------------------------------------------------------------
    separator("PHASE 1: Building the Knowledge Graph");

    let thesaurus = build_agent_thesaurus();
    println!(
        "  Created thesaurus with {} terms (synonyms + concepts)",
        thesaurus.len()
    );

    let role_name = "AI Agent Researcher".to_string();
    let mut rolegraph = RoleGraph::new(role_name.into(), thesaurus)
        .await
        .expect("Failed to create RoleGraph");

    println!("  RoleGraph created for role: AI Agent Researcher");
    print_graph_stats(&rolegraph);

    // -----------------------------------------------------------------------
    // PHASE 2: Index documents
    // -----------------------------------------------------------------------
    separator("PHASE 2: Indexing Documents");

    let documents = build_documents();
    for doc in &documents {
        println!("  Indexing: {} ({})", doc.title, doc.id);
        rolegraph.insert_document(&doc.id, doc.clone());
    }

    print_graph_stats(&rolegraph);

    // Show warnings for documents that may not have matched any terms
    let warnings = rolegraph.validate_documents();
    if !warnings.is_empty() {
        println!("\n  Validation warnings:");
        for w in &warnings {
            println!("    - {}", w);
        }
    }

    // -----------------------------------------------------------------------
    // PHASE 3: Search BEFORE adding new terms
    // -----------------------------------------------------------------------
    separator("PHASE 3: Search Results BEFORE Knowledge Graph Expansion");

    // Query 1: "agent reasoning"
    let query1 = "agent reasoning";
    println!("  Query: \"{}\"", query1);
    let results_before_1 = rolegraph
        .query_graph(query1, Some(0), Some(10))
        .expect("query_graph failed");
    print_results(&results_before_1);

    // Query 2: "agent planning tool use"
    let query2 = "agent planning tool use";
    println!("\n  Query: \"{}\"", query2);
    let results_before_2 = rolegraph
        .query_graph(query2, Some(0), Some(10))
        .expect("query_graph failed");
    print_results(&results_before_2);

    // Query 3: "reflection" -- a term NOT yet in the thesaurus
    let query3 = "reflection";
    println!("\n  Query: \"{}\"", query3);
    let results_before_3 = rolegraph
        .query_graph(query3, Some(0), Some(10))
        .expect("query_graph failed");
    print_results(&results_before_3);
    println!("  --> \"reflection\" is not in the thesaurus, so no graph results.");

    // Query 4: "orchestration" -- not in thesaurus yet
    let query4 = "orchestration";
    println!("\n  Query: \"{}\"", query4);
    let results_before_4 = rolegraph
        .query_graph(query4, Some(0), Some(10))
        .expect("query_graph failed");
    print_results(&results_before_4);
    println!("  --> \"orchestration\" is not in the thesaurus either.");

    // Save rank snapshot for comparison
    let react_rank_before = results_before_2
        .iter()
        .find(|(id, _)| id == "doc-react-agent")
        .map(|(_, d)| d.rank)
        .unwrap_or(0);
    let multi_rank_before = results_before_2
        .iter()
        .find(|(id, _)| id == "doc-multi-agent")
        .map(|(_, d)| d.rank)
        .unwrap_or(0);

    // -----------------------------------------------------------------------
    // PHASE 4: Expand the knowledge graph with new terms
    // -----------------------------------------------------------------------
    separator("PHASE 4: Expanding the Knowledge Graph");

    println!("  Adding new concepts and synonyms to the thesaurus...\n");

    // To add new terms, we need to rebuild the RoleGraph with an expanded thesaurus.
    // This simulates what happens when a user adds new markdown KG files.
    let mut expanded_thesaurus = build_agent_thesaurus();

    // NEW Concept 6: "reflection" -- maps to "reasoning"
    // This means "reflection" in text will now match the "reasoning" concept.
    let reasoning_id = 3;
    let reasoning_nterm = NormalizedTermValue::from("reasoning");
    expanded_thesaurus.insert(
        NormalizedTermValue::from("reflection"),
        NormalizedTerm::new(reasoning_id, reasoning_nterm.clone()),
    );
    expanded_thesaurus.insert(
        NormalizedTermValue::from("self-reflection"),
        NormalizedTerm::new(reasoning_id, reasoning_nterm.clone()),
    );
    println!("  + Added \"reflection\" and \"self-reflection\" as synonyms of \"reasoning\"");

    // NEW Concept 7: "orchestration" -- maps to "planning"
    let planning_id = 2;
    let planning_nterm = NormalizedTermValue::from("planning");
    expanded_thesaurus.insert(
        NormalizedTermValue::from("orchestration"),
        NormalizedTerm::new(planning_id, planning_nterm.clone()),
    );
    expanded_thesaurus.insert(
        NormalizedTermValue::from("coordination"),
        NormalizedTerm::new(planning_id, planning_nterm.clone()),
    );
    println!("  + Added \"orchestration\" and \"coordination\" as synonyms of \"planning\"");

    // NEW Concept 8: "task decomposition" -- a new concept
    let decomp_id = 6;
    let decomp_nterm = NormalizedTermValue::from("task decomposition");
    expanded_thesaurus.insert(
        NormalizedTermValue::from("task decomposition"),
        NormalizedTerm::new(decomp_id, decomp_nterm.clone()),
    );
    expanded_thesaurus.insert(
        NormalizedTermValue::from("decompose"),
        NormalizedTerm::new(decomp_id, decomp_nterm.clone()),
    );
    println!("  + Added \"task decomposition\" and \"decompose\" as new concept");

    println!(
        "\n  Expanded thesaurus: {} terms (was {})",
        expanded_thesaurus.len(),
        build_agent_thesaurus().len()
    );

    // Rebuild the RoleGraph with the expanded thesaurus
    let mut rolegraph_expanded = RoleGraph::new("AI Agent Researcher".into(), expanded_thesaurus)
        .await
        .expect("Failed to create expanded RoleGraph");

    // Re-index the same documents plus one new document that uses the new terms
    for doc in &documents {
        rolegraph_expanded.insert_document(&doc.id, doc.clone());
    }

    // Add a new document that heavily uses the new concepts
    let new_doc = Document {
        id: "doc-reflection-agent".to_string(),
        title: "Reflexion: Self-Reflection in AI Agent Loops".to_string(),
        body: "Reflexion is an agent framework that uses self-reflection to improve \
               task performance. The agent attempts a task, performs reflection on the \
               outcome through reasoning, and refines its plan. This orchestration of \
               reflection, reasoning, and planning creates a powerful feedback loop. \
               The agent can decompose complex problems and coordinate multiple \
               sub-tasks. Memory of past reflections guides future reasoning."
            .to_string(),
        url: "/docs/reflexion-agent".to_string(),
        description: Some("Self-reflective agent framework".to_string()),
        doc_type: DocumentType::KgEntry,
        ..Default::default()
    };
    println!("\n  + Added new document: \"{}\"", new_doc.title);
    let new_doc_id = new_doc.id.clone();
    rolegraph_expanded.insert_document(&new_doc_id, new_doc);

    print_graph_stats(&rolegraph_expanded);

    // -----------------------------------------------------------------------
    // PHASE 5: Search AFTER adding new terms
    // -----------------------------------------------------------------------
    separator("PHASE 5: Search Results AFTER Knowledge Graph Expansion");

    // Query 1 again: "agent reasoning"
    println!("  Query: \"{}\"", query1);
    let results_after_1 = rolegraph_expanded
        .query_graph(query1, Some(0), Some(10))
        .expect("query_graph failed");
    print_results(&results_after_1);

    // Query 2 again: "agent planning tool use"
    println!("\n  Query: \"{}\"", query2);
    let results_after_2 = rolegraph_expanded
        .query_graph(query2, Some(0), Some(10))
        .expect("query_graph failed");
    print_results(&results_after_2);

    // Query 3 again: "reflection" -- NOW in the thesaurus!
    println!("\n  Query: \"{}\"", query3);
    let results_after_3 = rolegraph_expanded
        .query_graph(query3, Some(0), Some(10))
        .expect("query_graph failed");
    print_results(&results_after_3);
    if !results_after_3.is_empty() {
        println!("  --> \"reflection\" now maps to \"reasoning\" and returns results!");
    }

    // Query 4 again: "orchestration" -- NOW in the thesaurus!
    println!("\n  Query: \"{}\"", query4);
    let results_after_4 = rolegraph_expanded
        .query_graph(query4, Some(0), Some(10))
        .expect("query_graph failed");
    print_results(&results_after_4);
    if !results_after_4.is_empty() {
        println!("  --> \"orchestration\" now maps to \"planning\" and returns results!");
    }

    // Query 5: "task decomposition coordination"
    let query5 = "task decomposition coordination";
    println!("\n  Query: \"{}\"", query5);
    let results_after_5 = rolegraph_expanded
        .query_graph(query5, Some(0), Some(10))
        .expect("query_graph failed");
    print_results(&results_after_5);

    // -----------------------------------------------------------------------
    // PHASE 6: Compare rankings
    // -----------------------------------------------------------------------
    separator("PHASE 6: Ranking Comparison");

    let react_rank_after = results_after_2
        .iter()
        .find(|(id, _)| id == "doc-react-agent")
        .map(|(_, d)| d.rank)
        .unwrap_or(0);
    let multi_rank_after = results_after_2
        .iter()
        .find(|(id, _)| id == "doc-multi-agent")
        .map(|(_, d)| d.rank)
        .unwrap_or(0);

    println!("  For query \"{}\"\n", query2);
    println!("  Document                  | Rank Before | Rank After | Change");
    println!("  --------------------------+-------------+------------+-------");
    println!(
        "  doc-react-agent           | {:>11} | {:>10} | {:>+6}",
        react_rank_before,
        react_rank_after,
        react_rank_after as i64 - react_rank_before as i64
    );
    println!(
        "  doc-multi-agent           | {:>11} | {:>10} | {:>+6}",
        multi_rank_before,
        multi_rank_after,
        multi_rank_after as i64 - multi_rank_before as i64
    );

    // Show that previously-unfindable queries now return results
    println!("\n  Previously unfindable queries now returning results:");
    println!(
        "    \"reflection\"           : {} -> {} results",
        results_before_3.len(),
        results_after_3.len()
    );
    println!(
        "    \"orchestration\"        : {} -> {} results",
        results_before_4.len(),
        results_after_4.len()
    );

    // -----------------------------------------------------------------------
    // PHASE 7: Path connectivity
    // -----------------------------------------------------------------------
    separator("PHASE 7: Graph Connectivity Analysis");

    let connectivity_tests = [
        "agent reasoning planning",
        "agent tool use memory",
        "reflection orchestration",
        "task decomposition coordination memory",
    ];

    for text in connectivity_tests {
        let connected = rolegraph_expanded.is_all_terms_connected_by_path(text);
        let matched_ids = rolegraph_expanded.find_matching_node_ids(text);
        println!(
            "  \"{}\"  -->  {} matched terms, connected={}",
            text,
            matched_ids.len(),
            connected
        );
    }

    // -----------------------------------------------------------------------
    // PHASE 8: Serialization round-trip
    // -----------------------------------------------------------------------
    separator("PHASE 8: Serialization Round-Trip");

    let serializable = rolegraph_expanded.to_serializable();
    let json = serializable
        .to_json()
        .expect("Failed to serialize RoleGraph");
    println!("  Serialized RoleGraph to {} bytes of JSON", json.len());

    let restored = RoleGraph::from_serializable(
        terraphim_rolegraph::SerializableRoleGraph::from_json(&json)
            .expect("Failed to deserialize"),
    )
    .await
    .expect("Failed to restore RoleGraph");

    let restored_results = restored
        .query_graph(query1, Some(0), Some(10))
        .expect("query_graph failed on restored graph");
    println!(
        "  Restored graph query \"{}\" returned {} results (same as before: {})",
        query1,
        restored_results.len(),
        restored_results.len() == results_after_1.len()
    );

    // -----------------------------------------------------------------------
    // Summary
    // -----------------------------------------------------------------------
    separator("SUMMARY");

    println!("  The Terraphim knowledge graph ranking system works by:");
    println!("  1. Building a thesaurus of domain concepts and their synonyms");
    println!("  2. Using Aho-Corasick automata to find concept mentions in documents");
    println!("  3. Creating graph nodes (concepts) and edges (co-occurrences)");
    println!("  4. Ranking documents by: node.rank + edge.rank + document_frequency");
    println!("  5. Documents with more connected concepts rank higher");
    println!();
    println!("  Adding new synonyms to the knowledge graph:");
    println!("  - Enables retrieval of previously-unfindable content");
    println!("  - Increases ranks for documents rich in domain concepts");
    println!("  - Creates new edges between concepts, improving connectivity");
    println!("  - Documents mentioning multiple connected concepts rank highest");
    println!();
    println!("  This is what makes Terraphim Graph ranking different from BM25:");
    println!("  - BM25 treats each word independently (bag of words)");
    println!("  - Terraphim Graph understands concept relationships");
    println!("  - Synonyms like \"reflection\" -> \"reasoning\" expand recall");
    println!("  - Co-occurrence edges reward conceptual density");
    println!();
}
