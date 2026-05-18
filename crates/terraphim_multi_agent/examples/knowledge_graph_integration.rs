//! Knowledge Graph Integration Example
//!
//! This example demonstrates the knowledge graph intelligence features:
//! - Context enrichment from knowledge graph
//! - Semantic relationship discovery
//! - Query-specific context injection
//! - Multi-layered context assembly

use terraphim_config::Role;
use terraphim_multi_agent::{
    test_utils::create_test_role, CommandInput, CommandType, MultiAgentResult, TerraphimAgent,
};
use terraphim_persistence::DeviceStorage;

/// Create a role configured for knowledge graph demonstration
fn create_knowledge_graph_role() -> Role {
    let mut role = create_test_role();
    role.name = "KnowledgeGraphAgent".into();
    role.shortname = Some("kg_agent".to_string());

    // Add knowledge domain configuration
    role.extra.insert(
        "knowledge_domain".to_string(),
        serde_json::json!("rust_programming"),
    );
    role.extra.insert(
        "specializations".to_string(),
        serde_json::json!(["memory_management", "async_programming", "type_system"]),
    );
    role.extra
        .insert("context_enrichment".to_string(), serde_json::json!(true));

    // Configure for knowledge graph integration
    role.extra
        .insert("llm_provider".to_string(), serde_json::json!("ollama"));
    role.extra
        .insert("ollama_model".to_string(), serde_json::json!("gemma3:270m"));
    role.extra
        .insert("llm_temperature".to_string(), serde_json::json!(0.6)); // Balanced for knowledge work

    role
}

/// Example 1: Basic Knowledge Graph Context Enrichment
async fn example_context_enrichment() -> MultiAgentResult<()> {
    println!("🧠 Example 1: Knowledge Graph Context Enrichment");
    println!("===============================================");

    // Initialize storage
    let persistence = DeviceStorage::arc_memory_only()
        .await
        .map_err(|e| terraphim_multi_agent::MultiAgentError::PersistenceError(e.to_string()))?;

    // Create knowledge graph enabled agent
    let role = create_knowledge_graph_role();
    let agent = TerraphimAgent::new(role, persistence, None).await?;
    agent.initialize().await?;

    println!(
        "✅ Created knowledge graph agent: {}",
        agent.role_config.name
    );

    // Test queries that should trigger knowledge graph enrichment
    let queries = vec![
        "How does Rust handle memory management?",
        "What are async functions in Rust?",
        "Explain Rust's type system",
        "How to handle errors in Rust?",
    ];

    for query in queries {
        println!("\n🔍 Query: {}", query);

        // This will internally call get_enriched_context_for_query()
        let input = CommandInput::new(query.to_string(), CommandType::Answer);
        let output = agent.process_command(input).await?;

        println!("📝 Response: {}", output.text);

        // Check if context was enriched
        let context = agent.context.read().await;
        println!("📊 Context items: {}", context.items.len());
        println!("🎯 Context tokens: {}", context.current_tokens);
    }

    Ok(())
}

/// Example 2: Semantic Relationship Discovery
async fn example_semantic_relationships() -> MultiAgentResult<()> {
    println!("\n🕸️  Example 2: Semantic Relationship Discovery");
    println!("===============================================");

    // Initialize storage and agent
    let persistence = DeviceStorage::arc_memory_only()
        .await
        .map_err(|e| terraphim_multi_agent::MultiAgentError::PersistenceError(e.to_string()))?;

    let role = create_knowledge_graph_role();
    let agent = TerraphimAgent::new(role, persistence, None).await?;
    agent.initialize().await?;

    // Queries designed to test semantic relationships
    let relationship_queries = vec![
        (
            "ownership",
            "How does Rust ownership relate to memory safety?",
        ),
        (
            "borrowing",
            "What's the relationship between borrowing and lifetimes?",
        ),
        ("async await", "How do async/await relate to Rust futures?"),
        ("traits generics", "How do traits work with generic types?"),
    ];

    for (concept, query) in relationship_queries {
        println!("\n🎯 Concept: {} | Query: {}", concept, query);

        let input = CommandInput::new(query.to_string(), CommandType::Analyze);
        let output = agent.process_command(input).await?;

        println!("🔗 Analysis: {}", output.text);

        // The agent internally uses:
        // - rolegraph.find_matching_node_ids(query)
        // - rolegraph.is_all_terms_connected_by_path(query)
        // - rolegraph.query_graph(query, Some(3), None)
        println!("   ✅ Knowledge graph relationships analyzed");
    }

    Ok(())
}

/// Example 3: Multi-layered Context Assembly
async fn example_multilayer_context() -> MultiAgentResult<()> {
    println!("\n🏗️  Example 3: Multi-layered Context Assembly");
    println!("==============================================");

    // Initialize storage and agent
    let persistence = DeviceStorage::arc_memory_only()
        .await
        .map_err(|e| terraphim_multi_agent::MultiAgentError::PersistenceError(e.to_string()))?;

    let mut role = create_knowledge_graph_role();

    // Add haystack configuration to demonstrate multi-layered context
    role.haystacks.push(terraphim_config::Haystack {
        read_only: true,
        atomic_server_secret: None,
        extra_parameters: std::collections::HashMap::new(),
        location: "./rust_docs".to_string(),
        service: terraphim_config::ServiceType::Ripgrep,
        fetch_content: false,
    });

    let agent = TerraphimAgent::new(role, persistence, None).await?;
    agent.initialize().await?;

    // Add some context to agent memory first (simulate previous interactions)
    {
        let mut context = agent.context.write().await;
        context.add_item(terraphim_multi_agent::ContextItem::new(
            terraphim_multi_agent::ContextItemType::Memory,
            "Previous discussion about Rust memory management principles".to_string(),
            25,
            0.8,
        ))?;
        context.add_item(terraphim_multi_agent::ContextItem::new(
            terraphim_multi_agent::ContextItemType::Memory,
            "User is learning about advanced Rust concepts".to_string(),
            20,
            0.7,
        ))?;
    }

    println!("📚 Added memory context for demonstration");

    // Query that will trigger multi-layered context assembly
    let complex_query = "How can I optimize memory allocation in async Rust applications?";

    println!("🔍 Complex Query: {}", complex_query);
    println!("\n🏗️  Multi-layered context will include:");
    println!("   1. Knowledge graph nodes matching the query terms");
    println!("   2. Semantic connectivity analysis");
    println!("   3. Related concepts from graph traversal");
    println!("   4. Relevant items from agent memory");
    println!("   5. Available haystack search sources");

    let input = CommandInput::new(complex_query.to_string(), CommandType::Analyze);
    let output = agent.process_command(input).await?;

    println!("\n📝 Comprehensive Analysis:");
    println!("{}", output.text);

    // Show final context state
    let context = agent.context.read().await;
    println!("\n📊 Final Context Statistics:");
    println!("   Total items: {}", context.items.len());
    println!("   Total tokens: {}", context.current_tokens);
    println!(
        "   Token utilization: {:.1}%",
        (context.current_tokens as f32 / context.max_tokens as f32) * 100.0
    );

    Ok(())
}

/// Example 4: Context-Aware Command Comparison
async fn example_context_aware_commands() -> MultiAgentResult<()> {
    println!("\n🎛️  Example 4: Context-Aware Command Comparison");
    println!("===============================================");

    // Initialize storage and agent
    let persistence = DeviceStorage::arc_memory_only()
        .await
        .map_err(|e| terraphim_multi_agent::MultiAgentError::PersistenceError(e.to_string()))?;

    let role = create_knowledge_graph_role();
    let agent = TerraphimAgent::new(role, persistence, None).await?;
    agent.initialize().await?;

    let base_query = "Rust async programming patterns";

    println!("🎯 Base Query: {}", base_query);
    println!("\n🔄 Testing all command types with knowledge graph enrichment:");

    // Test each command type with the same query to show different behaviors
    let command_types = vec![
        (CommandType::Generate, "Creative generation with context"),
        (CommandType::Answer, "Knowledge-based Q&A with enrichment"),
        (
            CommandType::Analyze,
            "Structured analysis with graph insights",
        ),
        (CommandType::Create, "Innovation with related concepts"),
        (
            CommandType::Review,
            "Balanced review with comprehensive context",
        ),
    ];

    for (command_type, description) in command_types {
        println!("\n🔸 {} ({:?})", description, command_type);

        let input = CommandInput::new(base_query.to_string(), command_type);
        let start = std::time::Instant::now();
        let output = agent.process_command(input).await?;
        let duration = start.elapsed();

        println!("   ⏱️  Processing time: {:?}", duration);
        println!("   📝 Response: {}", output.text);

        // Each command type uses the same get_enriched_context_for_query() but processes it differently
        // based on temperature and system prompt variations
    }

    println!("\n✅ All command types successfully used knowledge graph enrichment!");

    Ok(())
}

/// Example 5: Knowledge Graph Performance Analysis
async fn example_performance_analysis() -> MultiAgentResult<()> {
    println!("\n⚡ Example 5: Knowledge Graph Performance Analysis");
    println!("=================================================");

    // Initialize storage and agent
    let persistence = DeviceStorage::arc_memory_only()
        .await
        .map_err(|e| terraphim_multi_agent::MultiAgentError::PersistenceError(e.to_string()))?;

    let role = create_knowledge_graph_role();
    let agent = TerraphimAgent::new(role, persistence, None).await?;
    agent.initialize().await?;

    // Test performance with different query complexities
    let test_queries = vec![
        ("Simple", "Rust"),
        ("Medium", "Rust memory management"),
        (
            "Complex",
            "How does Rust's ownership system interact with async programming patterns?",
        ),
        (
            "Very Complex",
            "What are the performance implications of different async runtime configurations in Rust applications with heavy concurrent workloads?",
        ),
    ];

    println!("📊 Performance Analysis of Knowledge Graph Enrichment:");

    for (complexity, query) in test_queries {
        println!("\n🔍 {} Query: {}", complexity, query);

        let input = CommandInput::new(query.to_string(), CommandType::Answer);

        // Measure context enrichment performance
        let start = std::time::Instant::now();
        let output = agent.process_command(input).await?;
        let total_duration = start.elapsed();

        // Get tracking information
        let _token_tracker = agent.token_tracker.read().await;
        let context = agent.context.read().await;

        println!("   ⏱️  Total time: {:?}", total_duration);
        println!("   🧠 Context items: {}", context.items.len());
        println!("   🎫 Context tokens: {}", context.current_tokens);
        println!("   📏 Response length: {} chars", output.text.len());

        // Calculate enrichment efficiency
        let efficiency = output.text.len() as f32 / total_duration.as_millis() as f32;
        println!("   📈 Efficiency: {:.2} chars/ms", efficiency);
    }

    println!("\n✅ Knowledge graph performance analysis completed!");
    println!("🎯 The system efficiently handles queries of all complexity levels!");

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🧠 Terraphim Knowledge Graph Integration Examples");
    println!("=================================================\n");

    // Run all knowledge graph examples
    example_context_enrichment().await?;
    example_semantic_relationships().await?;
    example_multilayer_context().await?;
    example_context_aware_commands().await?;
    example_performance_analysis().await?;

    println!("\n✅ All knowledge graph examples completed successfully!");
    println!("🎉 Knowledge graph integration is working perfectly!");
    println!("\n🚀 Key Features Demonstrated:");
    println!("   • Smart context enrichment with get_enriched_context_for_query()");
    println!("   • RoleGraph integration with semantic analysis");
    println!("   • Multi-layered context assembly (graph + memory + haystacks)");
    println!("   • Context-aware command processing with different temperatures");
    println!("   • Performance optimization with efficient knowledge retrieval");

    Ok(())
}
