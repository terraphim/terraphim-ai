//! Enhanced Atomic Server Configuration with Multi-Agent System
//!
//! This example demonstrates how the new multi-agent system works with
//! atomic server configurations, showing the evolution from simple Role
//! configurations to intelligent autonomous agents.

use ahash::AHashMap;
use std::sync::Arc;
use terraphim_config::{ConfigBuilder, Haystack, Role, ServiceType};
use terraphim_multi_agent::{
    CommandInput, CommandType, MultiAgentError, MultiAgentResult, TerraphimAgent,
};
use terraphim_persistence::DeviceStorage;
use terraphim_types::RelevanceFunction;

/// Create an atomic server role that becomes a multi-agent
fn create_atomic_server_agent_role() -> Role {
    Role {
        terraphim_it: true,
        shortname: Some("AtomicAgent".to_string()),
        name: "AtomicServerAgent".into(),
        relevance_function: RelevanceFunction::TitleScorer,
        theme: "spacelab".to_string(),
        kg: None,
        llm_enabled: false,
        llm_api_key: None,
        llm_model: None,
        llm_auto_summarize: false,
        llm_chat_enabled: false,
        llm_chat_system_prompt: None,
        llm_chat_model: None,
        llm_context_window: Some(16000),
        llm_router_enabled: false,
        llm_router_config: None,
        haystacks: vec![
            Haystack::new(
                "http://localhost:9883".to_string(), // Atomic server URL
                ServiceType::Atomic,
                true, // read-only
            )
            .with_atomic_secret(Some("your-base64-secret-here".to_string())),
        ],
        extra: {
            let mut extra = AHashMap::new();
            // Multi-agent specific configuration
            extra.insert(
                "agent_type".to_string(),
                serde_json::json!("atomic_server_specialist"),
            );
            extra.insert(
                "capabilities".to_string(),
                serde_json::json!([
                    "atomic_data_search",
                    "knowledge_retrieval",
                    "semantic_analysis"
                ]),
            );
            extra.insert(
                "goals".to_string(),
                serde_json::json!([
                    "Access atomic data efficiently",
                    "Provide semantic search",
                    "Maintain data consistency"
                ]),
            );

            // LLM configuration
            extra.insert("llm_provider".to_string(), serde_json::json!("ollama"));
            extra.insert("ollama_model".to_string(), serde_json::json!("gemma3:270m"));
            extra.insert("llm_temperature".to_string(), serde_json::json!(0.4)); // Balanced for data retrieval

            // Context enrichment settings
            extra.insert("context_enrichment".to_string(), serde_json::json!(true));
            extra.insert("max_context_tokens".to_string(), serde_json::json!(16000));
            extra
        },
    }
}

/// Demonstrate atomic server configuration evolution
async fn demonstrate_config_evolution() -> MultiAgentResult<()> {
    println!("📋 Configuration Evolution: From Role to Intelligent Agent");
    println!("=========================================================");

    // Step 1: Traditional configuration
    println!("\n1️⃣ Step 1: Traditional Role Configuration");
    let _config = ConfigBuilder::new()
        .global_shortcut("Ctrl+T")
        .add_role("AtomicUser", create_atomic_server_agent_role())
        .build()
        .expect("Failed to build config");

    println!("✅ Traditional config created:");
    println!("   - Role: AtomicServerAgent");
    println!("   - Haystack: Atomic server (http://localhost:9883)");
    println!("   - Authentication: Base64 secret");
    println!("   - Read-only: true");

    // Step 2: Multi-agent evolution
    println!("\n2️⃣ Step 2: Multi-Agent System Evolution");

    // Initialize storage
    DeviceStorage::init_memory_only()
        .await
        .map_err(|e| MultiAgentError::PersistenceError(e.to_string()))?;

    let storage_ref = DeviceStorage::instance()
        .await
        .map_err(|e| MultiAgentError::PersistenceError(e.to_string()))?;

    use std::ptr;
    let storage_copy = unsafe { ptr::read(storage_ref) };
    let persistence = Arc::new(storage_copy);

    // Transform role into intelligent agent
    let role = create_atomic_server_agent_role();
    let agent = TerraphimAgent::new(role, persistence, None).await?;
    agent.initialize().await?;

    println!("✅ Role evolved into intelligent agent:");
    println!("   - Agent ID: {}", agent.agent_id);
    println!("   - Status: {:?}", agent.status);
    println!("   - Capabilities: {:?}", agent.get_capabilities());
    println!("   - Goals: {:?}", agent.goals.individual_goals);

    Ok(())
}

/// Demonstrate intelligent atomic data queries
async fn demonstrate_intelligent_queries() -> MultiAgentResult<()> {
    println!("\n🧠 Intelligent Atomic Data Queries");
    println!("==================================");

    // Initialize agent
    DeviceStorage::init_memory_only()
        .await
        .map_err(|e| MultiAgentError::PersistenceError(e.to_string()))?;

    let storage_ref = DeviceStorage::instance()
        .await
        .map_err(|e| MultiAgentError::PersistenceError(e.to_string()))?;

    use std::ptr;
    let storage_copy = unsafe { ptr::read(storage_ref) };
    let persistence = Arc::new(storage_copy);

    let role = create_atomic_server_agent_role();
    let agent = TerraphimAgent::new(role, persistence, None).await?;
    agent.initialize().await?;

    // Intelligent queries that leverage both atomic data and AI reasoning
    let queries = vec![
        (
            CommandType::Answer,
            "Find all resources related to data modeling in the atomic server",
        ),
        (
            CommandType::Analyze,
            "Analyze the relationships between atomic data properties",
        ),
        (
            CommandType::Generate,
            "Generate a summary of atomic server best practices",
        ),
        (
            CommandType::Review,
            "Review the data consistency in our atomic server",
        ),
    ];

    for (command_type, query_text) in queries {
        println!("\n🔍 Query Type: {:?}", command_type);
        println!("   Query: {}", query_text);

        let input = CommandInput::new(query_text.to_string(), command_type);
        let output = agent.process_command(input).await?;

        println!("   🤖 AI Response: {}", output.text);

        // Show tracking information
        let token_tracker = agent.token_tracker.read().await;
        let cost_tracker = agent.cost_tracker.read().await;

        println!(
            "   📊 Tokens: {} in / {} out",
            token_tracker.total_input_tokens, token_tracker.total_output_tokens
        );
        println!("   💰 Cost: ${:.6}", cost_tracker.current_month_spending);
    }

    Ok(())
}

/// Demonstrate multi-layered context with atomic data
async fn demonstrate_context_integration() -> MultiAgentResult<()> {
    println!("\n🏗️ Multi-layered Context Integration");
    println!("===================================");

    // Initialize agent
    DeviceStorage::init_memory_only()
        .await
        .map_err(|e| MultiAgentError::PersistenceError(e.to_string()))?;

    let storage_ref = DeviceStorage::instance()
        .await
        .map_err(|e| MultiAgentError::PersistenceError(e.to_string()))?;

    use std::ptr;
    let storage_copy = unsafe { ptr::read(storage_ref) };
    let persistence = Arc::new(storage_copy);

    let role = create_atomic_server_agent_role();
    let agent = TerraphimAgent::new(role, persistence, None).await?;
    agent.initialize().await?;

    // Query that should trigger comprehensive context assembly
    let complex_query = "How can I optimize atomic data queries for better performance while maintaining consistency?";

    println!("🎯 Complex Query: {}", complex_query);
    println!("\n🔍 Context Sources Being Integrated:");
    println!("   1. 🌐 Atomic Server Data (via haystack)");
    println!("   2. 🧠 Knowledge Graph (semantic relationships)");
    println!("   3. 💭 Agent Memory (previous interactions)");
    println!("   4. 🎯 Role Goals (optimization & consistency)");
    println!("   5. ⚙️  Agent Capabilities (atomic_data_search, semantic_analysis)");

    let input = CommandInput::new(complex_query.to_string(), CommandType::Analyze);
    let output = agent.process_command(input).await?;

    println!("\n📝 Comprehensive Analysis:");
    println!("{}", output.text);

    // Show context utilization
    let context = agent.context.read().await;
    println!("\n📊 Context Utilization:");
    println!("   Context Items: {}", context.items.len());
    println!("   Context Tokens: {}", context.current_tokens);
    println!(
        "   Token Efficiency: {:.1}%",
        (context.current_tokens as f32 / context.max_tokens as f32) * 100.0
    );

    Ok(())
}

/// Compare traditional vs intelligent approach
async fn demonstrate_evolution_comparison() -> MultiAgentResult<()> {
    println!("\n⚖️ Evolution Comparison: Traditional vs Intelligent");
    println!("=================================================");

    println!("🔴 Traditional Approach:");
    println!("   • Static role configuration");
    println!("   • Manual query construction");
    println!("   • Basic haystack search");
    println!("   • No learning or adaptation");
    println!("   • Limited context awareness");

    println!("\n🟢 Multi-Agent Intelligence:");
    println!("   • Dynamic agent evolution");
    println!("   • AI-powered query understanding");
    println!("   • Context-enriched search");
    println!("   • Continuous learning from interactions");
    println!("   • Semantic relationship discovery");
    println!("   • Goal-aligned responses");
    println!("   • Cost and performance tracking");

    // Initialize intelligent agent
    DeviceStorage::init_memory_only()
        .await
        .map_err(|e| MultiAgentError::PersistenceError(e.to_string()))?;

    let storage_ref = DeviceStorage::instance()
        .await
        .map_err(|e| MultiAgentError::PersistenceError(e.to_string()))?;

    use std::ptr;
    let storage_copy = unsafe { ptr::read(storage_ref) };
    let persistence = Arc::new(storage_copy);

    let role = create_atomic_server_agent_role();
    let agent = TerraphimAgent::new(role, persistence, None).await?;
    agent.initialize().await?;

    // Demonstrate intelligent capabilities
    let test_query = "atomic data consistency";
    let input = CommandInput::new(test_query.to_string(), CommandType::Generate);
    let output = agent.process_command(input).await?;

    println!("\n🧪 Example: '{}'", test_query);
    println!("🤖 Intelligent Response: {}", output.text);

    // Show intelligence metrics
    let command_history = agent.command_history.read().await;
    println!("\n📈 Intelligence Metrics:");
    println!("   Commands Processed: {}", command_history.records.len());
    println!("   Agent Learning: Active");
    println!("   Context Enrichment: Enabled");
    println!("   Performance Tracking: Real-time");

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Enhanced Atomic Server Configuration with Multi-Agent System");
    println!("==============================================================\n");

    // Run all demonstrations
    demonstrate_config_evolution().await?;
    demonstrate_intelligent_queries().await?;
    demonstrate_context_integration().await?;
    demonstrate_evolution_comparison().await?;

    println!("\n🎉 All demonstrations completed successfully!");
    println!("\n✅ Key Achievements:");
    println!("   • Traditional Role configurations seamlessly evolve into intelligent agents");
    println!("   • Atomic server data becomes accessible through AI-powered interfaces");
    println!("   • Context enrichment provides comprehensive understanding");
    println!("   • Multi-layered intelligence enhances every query");
    println!("   • Performance tracking enables continuous optimization");

    println!(
        "\n🚀 The Multi-Agent System transforms static configurations into intelligent, adaptive agents!"
    );

    Ok(())
}
