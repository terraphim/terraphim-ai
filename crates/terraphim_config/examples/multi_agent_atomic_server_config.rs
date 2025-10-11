//! Multi-Agent Enhanced Atomic Server Configuration Example
//!
//! This example shows how traditional atomic server configurations
//! can be enhanced with the new multi-agent system to create
//! intelligent, autonomous agents.

use ahash::AHashMap;
use std::sync::Arc;
use terraphim_config::{Config, ConfigBuilder, Haystack, Role, ServiceType};
use terraphim_types::RelevanceFunction;

// Multi-agent system demonstration requires the crate to be added as dependency
// #[cfg(feature = "multi_agent")]
// use terraphim_multi_agent::{CommandInput, CommandType, TerraphimAgent};

/// Enhanced atomic server configuration with multi-agent capabilities
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    env_logger::init();

    println!("🚀 Multi-Agent Enhanced Atomic Server Configuration");
    println!("=================================================\n");

    // Example 1: Enhanced Atomic Server Configuration
    println!("📋 Example 1: Enhanced Atomic Server Configuration");

    let enhanced_role = Role {
        terraphim_it: true,
        shortname: Some("EnhancedAtomic".to_string()),
        name: "EnhancedAtomicAgent".into(),
        relevance_function: RelevanceFunction::TitleScorer,
        theme: "spacelab".to_string(),
        kg: None,
        haystacks: vec![Haystack::new(
            "http://localhost:9883".to_string(),
            ServiceType::Atomic,
            true,
        )
        .with_atomic_secret(Some("your-base64-secret-here".to_string()))],
        extra: {
            let mut extra = AHashMap::new();

            // Multi-agent system configuration
            extra.insert(
                "agent_capabilities".to_string(),
                serde_json::json!([
                    "atomic_data_access",
                    "semantic_search",
                    "knowledge_retrieval"
                ]),
            );
            extra.insert(
                "agent_goals".to_string(),
                serde_json::json!([
                    "Efficient atomic data access",
                    "Maintain data consistency",
                    "Provide intelligent responses"
                ]),
            );

            // LLM integration for AI capabilities
            extra.insert("llm_provider".to_string(), serde_json::json!("ollama"));
            extra.insert(
                "ollama_base_url".to_string(),
                serde_json::json!("http://127.0.0.1:11434"),
            );
            extra.insert("ollama_model".to_string(), serde_json::json!("gemma3:270m"));
            extra.insert("llm_temperature".to_string(), serde_json::json!(0.4));

            // Context enrichment settings
            extra.insert(
                "context_enrichment_enabled".to_string(),
                serde_json::json!(true),
            );
            extra.insert("max_context_tokens".to_string(), serde_json::json!(16000));
            extra.insert(
                "knowledge_graph_integration".to_string(),
                serde_json::json!(true),
            );

            extra
        },
        llm_enabled: false,
        llm_api_key: None,
        llm_model: None,
        llm_auto_summarize: false,
        llm_chat_enabled: false,
        llm_chat_system_prompt: None,
        llm_chat_model: None,
        llm_context_window: Some(32768),
    };

    let config = ConfigBuilder::new()
        .global_shortcut("Ctrl+Alt+A")
        .add_role("EnhancedAtomicAgent", enhanced_role)
        .build()
        .expect("Failed to build enhanced config");

    println!("✅ Enhanced atomic server config created:");
    println!("   Server URL: http://localhost:9883");
    println!("   Authentication: Required (secret provided)");
    println!("   Multi-agent capabilities: Enabled");
    println!("   LLM integration: Ollama with gemma3:270m");
    println!("   Context enrichment: Enabled");
    println!("   Knowledge graph: Integrated");

    // Example 2: Configuration comparison
    println!("\n📊 Configuration Evolution Comparison");
    println!("=====================================");

    println!("🔴 Traditional Configuration:");
    println!("   • Static role with fixed capabilities");
    println!("   • Basic haystack search");
    println!("   • No AI integration");
    println!("   • Limited context awareness");
    println!("   • Manual query processing");

    println!("\n🟢 Multi-Agent Enhanced Configuration:");
    println!("   • Intelligent autonomous agent");
    println!("   • AI-powered query understanding");
    println!("   • Context-enriched responses");
    println!("   • Continuous learning from interactions");
    println!("   • Goal-aligned behavior");
    println!("   • Performance and cost tracking");

    // Example 3: Demonstrate multi-agent system integration (if available)
    #[cfg(feature = "multi_agent")]
    {
        println!("\n🤖 Multi-Agent System Integration Demo");
        println!("======================================");

        // This would create an intelligent agent from the role
        let persistence = create_test_storage().await?;
        let mut agent = TerraphimAgent::new(enhanced_role.clone(), persistence, None).await?;
        agent.initialize().await?;

        println!("✅ Intelligent agent created from configuration:");
        println!("   Agent ID: {}", agent.agent_id);
        println!("   Status: {:?}", agent.status);
        println!("   Capabilities: {:?}", agent.get_capabilities());

        // Demonstrate intelligent query processing
        let query = "Find resources about atomic data modeling best practices";
        let input = CommandInput::new(query.to_string(), CommandType::Answer);
        let output = agent.process_command(input).await?;

        println!("\n🔍 Intelligent Query Processing:");
        println!("   Query: {}", query);
        println!("   AI Response: {}", output.text);
        println!("   Metadata: {:?}", output.metadata);
    }

    #[cfg(not(feature = "multi_agent"))]
    {
        println!("\n💡 Multi-Agent Integration Available");
        println!("====================================");
        println!("To see the multi-agent system in action:");
        println!("   1. Add 'multi_agent' feature flag");
        println!("   2. The role configuration automatically becomes an intelligent agent");
        println!("   3. All queries become AI-powered with context enrichment");
        println!("   4. Performance tracking and learning capabilities are enabled");
    }

    // Example 4: Best practices for multi-agent configurations
    println!("\n📚 Multi-Agent Configuration Best Practices");
    println!("============================================");

    println!("🎯 Role Configuration:");
    println!("   • Add 'agent_capabilities' to define what the agent can do");
    println!("   • Specify 'agent_goals' for goal-aligned behavior");
    println!("   • Configure LLM settings for optimal performance");
    println!("   • Enable context enrichment for intelligent responses");

    println!("\n🔧 LLM Integration:");
    println!("   • Use 'llm_provider': 'ollama' for local models");
    println!("   • Set appropriate 'llm_temperature' (0.3-0.7 range)");
    println!("   • Configure model-specific settings in 'extra' parameters");
    println!("   • Enable knowledge graph integration for semantic understanding");

    println!("\n📊 Performance Optimization:");
    println!("   • Set 'max_context_tokens' based on model capabilities");
    println!("   • Enable tracking for cost and performance monitoring");
    println!("   • Use appropriate haystacks for data access");
    println!("   • Configure goals that align with use cases");

    // Example 5: Configuration serialization with multi-agent features
    println!("\n💾 Enhanced Configuration Serialization");
    println!("=======================================");

    let json_output = serde_json::to_string_pretty(&config)?;
    println!("📄 Enhanced JSON configuration with multi-agent features:");
    println!("{}", json_output);

    println!("\n🎉 Multi-Agent Enhanced Atomic Server Configuration Complete!");
    println!("\n✅ Key Benefits:");
    println!("   • Seamless evolution from traditional roles to intelligent agents");
    println!("   • AI-powered query understanding and response generation");
    println!("   • Context-aware processing with knowledge graph integration");
    println!("   • Goal-aligned behavior for better user experiences");
    println!("   • Performance tracking and continuous optimization");
    println!("   • Backward compatibility with existing configurations");

    Ok(())
}

/// Helper function to create test storage (would be imported from multi-agent crate)
#[cfg(feature = "multi_agent")]
async fn create_test_storage(
) -> Result<std::sync::Arc<terraphim_persistence::DeviceStorage>, Box<dyn std::error::Error>> {
    use std::sync::Arc;
    use terraphim_persistence::DeviceStorage;

    // Use the safe Arc method instead of unsafe ptr::read
    let storage = DeviceStorage::arc_memory_only().await?;
    Ok(storage)
}
