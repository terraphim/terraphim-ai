//! Basic Multi-Agent System Usage Example
//!
//! This example demonstrates how to create and use TerraphimAgent instances
//! with the new multi-agent architecture, including:
//! - Agent creation from Role configurations
//! - Command processing with different types
//! - Token and cost tracking
//! - Knowledge graph integration

use std::sync::Arc;
use terraphim_multi_agent::{
    test_utils::create_test_role, CommandInput, CommandType, MultiAgentResult, TerraphimAgent,
};
use terraphim_persistence::DeviceStorage;

/// Example 1: Basic Agent Creation and Initialization
async fn example_basic_agent_creation() -> MultiAgentResult<()> {
    println!("🤖 Example 1: Basic Agent Creation");
    println!("=====================================");

    // Create a test role configuration
    let role = create_test_role();
    println!("✅ Created role: {}", role.name);

    // Initialize storage
    DeviceStorage::init_memory_only()
        .await
        .map_err(|e| terraphim_multi_agent::MultiAgentError::PersistenceError(e.to_string()))?;
    let storage = DeviceStorage::instance()
        .await
        .map_err(|e| terraphim_multi_agent::MultiAgentError::PersistenceError(e.to_string()))?;

    // Create new owned storage to avoid lifetime issues
    use std::ptr;
    let storage_copy = unsafe { ptr::read(storage) };
    let persistence = Arc::new(storage_copy);

    // Create agent with default configuration
    let mut agent = TerraphimAgent::new(role, persistence, None).await?;
    println!("✅ Agent created with ID: {}", agent.agent_id);

    // Initialize the agent
    agent.initialize().await?;
    println!("✅ Agent initialized successfully");

    println!("📊 Agent Status:");
    println!("   - Status: {:?}", agent.status);
    println!("   - Capabilities: {:?}", agent.get_capabilities());

    Ok(())
}

/// Example 2: Command Processing with Different Types
async fn example_command_processing() -> MultiAgentResult<()> {
    println!("\n🎯 Example 2: Command Processing");
    println!("==================================");

    // Create and initialize agent
    let role = create_test_role();
    DeviceStorage::init_memory_only()
        .await
        .map_err(|e| terraphim_multi_agent::MultiAgentError::PersistenceError(e.to_string()))?;
    let storage = DeviceStorage::instance()
        .await
        .map_err(|e| terraphim_multi_agent::MultiAgentError::PersistenceError(e.to_string()))?;

    use std::ptr;
    let storage_copy = unsafe { ptr::read(storage) };
    let persistence = Arc::new(storage_copy);

    let mut agent = TerraphimAgent::new(role, persistence, None).await?;
    agent.initialize().await?;

    // Test different command types
    let commands = vec![
        (
            CommandType::Generate,
            "Write a hello world function in Rust",
        ),
        (CommandType::Answer, "What is the capital of France?"),
        (
            CommandType::Analyze,
            "Analyze the performance of this algorithm",
        ),
        (CommandType::Create, "Create a new REST API design"),
        (CommandType::Review, "Review this code for best practices"),
    ];

    for (command_type, text) in commands {
        println!("\n🔄 Processing {:?} command...", command_type);

        let input = CommandInput::new(text.to_string(), command_type.clone());
        let output = agent.process_command(input).await?;

        println!("   Input: {}", text);
        println!("   Output: {}", output.text);
        println!("   Metadata: {:?}", output.metadata);
    }

    Ok(())
}

/// Example 3: Token and Cost Tracking
async fn example_tracking() -> MultiAgentResult<()> {
    println!("\n💰 Example 3: Token and Cost Tracking");
    println!("=====================================");

    // Create and initialize agent
    let role = create_test_role();
    DeviceStorage::init_memory_only()
        .await
        .map_err(|e| terraphim_multi_agent::MultiAgentError::PersistenceError(e.to_string()))?;
    let storage = DeviceStorage::instance()
        .await
        .map_err(|e| terraphim_multi_agent::MultiAgentError::PersistenceError(e.to_string()))?;

    use std::ptr;
    let storage_copy = unsafe { ptr::read(storage) };
    let persistence = Arc::new(storage_copy);

    let mut agent = TerraphimAgent::new(role, persistence, None).await?;
    agent.initialize().await?;

    // Process multiple commands to accumulate tracking data
    for i in 1..=3 {
        let input = CommandInput::new(
            format!("Generate example code snippet number {}", i),
            CommandType::Generate,
        );

        println!("🔄 Processing command {}...", i);
        let _output = agent.process_command(input).await?;
    }

    // Display tracking information
    let token_tracker = agent.token_tracker.read().await;
    let cost_tracker = agent.cost_tracker.read().await;
    let command_history = agent.command_history.read().await;

    println!("📊 Tracking Results:");
    println!(
        "   Total Input Tokens: {}",
        token_tracker.total_input_tokens
    );
    println!(
        "   Total Output Tokens: {}",
        token_tracker.total_output_tokens
    );
    println!("   Total Requests: {}", token_tracker.total_requests);
    println!("   Total Cost: ${:.6}", cost_tracker.current_month_spending);
    println!("   Commands Processed: {}", command_history.records.len());

    Ok(())
}

/// Example 4: Context Management and Knowledge Graph Integration
async fn example_context_and_knowledge_graph() -> MultiAgentResult<()> {
    println!("\n🧠 Example 4: Context Management & Knowledge Graph");
    println!("=================================================");

    // Create role with extra configuration for knowledge graph
    let mut role = create_test_role();
    role.extra.insert(
        "knowledge_domain".to_string(),
        serde_json::json!("programming"),
    );

    DeviceStorage::init_memory_only()
        .await
        .map_err(|e| terraphim_multi_agent::MultiAgentError::PersistenceError(e.to_string()))?;
    let storage = DeviceStorage::instance()
        .await
        .map_err(|e| terraphim_multi_agent::MultiAgentError::PersistenceError(e.to_string()))?;

    use std::ptr;
    let storage_copy = unsafe { ptr::read(storage) };
    let persistence = Arc::new(storage_copy);

    let mut agent = TerraphimAgent::new(role, persistence, None).await?;
    agent.initialize().await?;

    // Test context-aware processing
    let query = "How to implement async functions in Rust";

    println!("🔍 Query: {}", query);

    // The agent will automatically use get_enriched_context_for_query()
    let input = CommandInput::new(query.to_string(), CommandType::Answer);
    let output = agent.process_command(input).await?;

    println!("✅ Response: {}", output.text);

    // Check context information
    let context = agent.context.read().await;
    println!("📄 Context Items: {}", context.items.len());
    println!("🎯 Current Context Tokens: {}", context.current_tokens);

    Ok(())
}

/// Example 5: Agent Goals and Capabilities
async fn example_agent_goals() -> MultiAgentResult<()> {
    println!("\n🎯 Example 5: Agent Goals and Capabilities");
    println!("==========================================");

    // Create specialized role with capabilities
    let mut role = create_test_role();
    role.name = "RustExpert".into();
    role.extra.insert(
        "capabilities".to_string(),
        serde_json::json!([
            "rust_programming",
            "systems_design",
            "performance_optimization"
        ]),
    );
    role.extra.insert(
        "goals".to_string(),
        serde_json::json!([
            "Write efficient Rust code",
            "Ensure memory safety",
            "Optimize performance"
        ]),
    );

    DeviceStorage::init_memory_only()
        .await
        .map_err(|e| terraphim_multi_agent::MultiAgentError::PersistenceError(e.to_string()))?;
    let storage = DeviceStorage::instance()
        .await
        .map_err(|e| terraphim_multi_agent::MultiAgentError::PersistenceError(e.to_string()))?;

    use std::ptr;
    let storage_copy = unsafe { ptr::read(storage) };
    let persistence = Arc::new(storage_copy);

    let mut agent = TerraphimAgent::new(role, persistence, None).await?;
    agent.initialize().await?;

    println!("🤖 Agent: {}", agent.role_config.name);
    println!("💪 Capabilities: {:?}", agent.get_capabilities());
    println!("🎯 Goals:");
    println!("   Global: {}", agent.goals.global_goal);
    println!("   Individual: {:?}", agent.goals.individual_goals);
    println!("   Alignment Score: {:.2}", agent.goals.alignment_score);

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Terraphim Multi-Agent System Examples");
    println!("=========================================\n");

    // Run all examples
    example_basic_agent_creation().await?;
    example_command_processing().await?;
    example_tracking().await?;
    example_context_and_knowledge_graph().await?;
    example_agent_goals().await?;

    println!("\n✅ All examples completed successfully!");
    println!("🎉 The Terraphim Multi-Agent System is working perfectly!");

    Ok(())
}
