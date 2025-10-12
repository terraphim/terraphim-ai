use ahash::AHashMap;
use terraphim_config::{Config, ConfigBuilder, Haystack, Role, ServiceType};
use terraphim_types::RelevanceFunction;

// Multi-agent system demonstration requires the crate to be added as dependency
// #[cfg(feature = "multi_agent")]
#[allow(unused_imports)]
use std::sync::Arc;
use terraphim_multi_agent::{CommandInput, CommandType, TerraphimAgent};
use terraphim_persistence::DeviceStorage;

/// Enhanced Atomic Server Configuration with Multi-Agent Intelligence
///
/// This example demonstrates the evolution from traditional Role configurations
/// to intelligent autonomous agents that can utilize atomic servers for enhanced search.
/// Shows both classic configuration patterns and new multi-agent enhancements.
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    env_logger::init();

    // Example 1: Traditional atomic server configuration
    println!("📋 Example 1: Traditional Atomic Server Configuration");
    println!("============================================");

    let basic_config = ConfigBuilder::new()
        .global_shortcut("Ctrl+T")
        .add_role(
            "AtomicUser",
            Role {
                terraphim_it: true,
                shortname: Some("AtomicUser".to_string()),
                name: "AtomicUser".into(),
                relevance_function: RelevanceFunction::TitleScorer,
                theme: "spacelab".to_string(),
                kg: None,
                haystacks: vec![Haystack::new(
                    "http://localhost:9883".to_string(), // Atomic server URL
                    ServiceType::Atomic,
                    true,
                )
                .with_atomic_secret(Some("your-base64-secret-here".to_string()))],
                llm_enabled: false,
                llm_api_key: None,
                llm_model: None,
                llm_auto_summarize: false,
                llm_chat_enabled: false,
                llm_chat_system_prompt: None,
                llm_chat_model: None,
                llm_context_window: Some(32768),
                extra: {
                    let mut extra = AHashMap::new();

                    // Multi-agent system configuration - enables intelligent capabilities
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

                    // LLM integration for AI-enhanced search
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
            },
        )
        .build()
        .expect("Failed to build basic config");

    println!("✅ Traditional atomic server config created successfully");
    println!("   Server URL: http://localhost:9883");
    println!("   Authentication: Required (secret provided)");
    println!("   Read-only: true");
    println!("   Multi-agent capabilities: Enabled (agent_capabilities, agent_goals)");
    println!("   LLM integration: Ollama with gemma3:270m");
    println!("   Context enrichment: Enabled (knowledge graph integration)");

    // Example 2: Hybrid configuration with both ripgrep and atomic server
    println!("\n📋 Example 2: Hybrid Ripgrep + Atomic Server Configuration");

    let _hybrid_config = ConfigBuilder::new()
        .global_shortcut("Ctrl+Shift+T")
        .add_role(
            "LocalResearcher",
            Role {
                terraphim_it: true,
                shortname: Some("LocalResearcher".to_string()),
                name: "LocalResearcher".into(),
                relevance_function: RelevanceFunction::TitleScorer,
                theme: "lumen".to_string(),
                kg: None,
                haystacks: vec![
                    // Local filesystem haystack using ripgrep
                    Haystack::new(
                        "./docs".to_string(), // Local filesystem path
                        ServiceType::Ripgrep,
                        false,
                    ),
                    // Remote atomic server haystack
                    Haystack::new(
                        "http://localhost:9883".to_string(), // Atomic server URL
                        ServiceType::Atomic,
                        true,
                    )
                    .with_atomic_secret(Some("your-base64-secret-here".to_string())),
                ],
                extra: ahash::AHashMap::new(),
                llm_enabled: false,
                llm_api_key: None,
                llm_model: None,
                llm_auto_summarize: false,
                llm_chat_enabled: false,
                llm_chat_system_prompt: None,
                llm_chat_model: None,
                llm_context_window: Some(32768),
            },
        )
        .add_role(
            "RemoteResearcher",
            Role {
                terraphim_it: true,
                shortname: Some("RemoteResearcher".to_string()),
                name: "RemoteResearcher".into(),
                relevance_function: RelevanceFunction::TitleScorer,
                theme: "superhero".to_string(),
                kg: None,
                haystacks: vec![
                    // Multiple atomic server instances
                    Haystack::new(
                        "http://localhost:9883".to_string(),
                        ServiceType::Atomic,
                        true,
                    )
                    .with_atomic_secret(Some("secret-for-server-1".to_string())),
                    Haystack::new(
                        "https://example.com/atomic".to_string(),
                        ServiceType::Atomic,
                        true,
                    )
                    .with_atomic_secret(Some("secret-for-server-2".to_string())),
                ],
                extra: ahash::AHashMap::new(),
                llm_enabled: false,
                llm_api_key: None,
                llm_model: None,
                llm_auto_summarize: false,
                llm_chat_enabled: false,
                llm_chat_system_prompt: None,
                llm_chat_model: None,
                llm_context_window: Some(32768),
            },
        )
        .build()
        .expect("Failed to build hybrid config");

    println!("✅ Hybrid config created successfully");
    println!("   LocalResearcher role:");
    println!("     - Local docs via ripgrep: ./docs");
    println!("     - Remote atomic server: http://localhost:9883");
    println!("   RemoteResearcher role:");
    println!("     - Two atomic servers with different secrets");

    // Example 3: Anonymous access to atomic server
    println!("\n📋 Example 3: Anonymous Access to Atomic Server");

    let _anonymous_config = ConfigBuilder::new()
        .global_shortcut("Ctrl+Alt+T")
        .add_role(
            "AnonymousUser",
            Role {
                terraphim_it: false,
                shortname: Some("AnonymousUser".to_string()),
                name: "AnonymousUser".into(),
                relevance_function: RelevanceFunction::TitleScorer,
                theme: "spacelab".to_string(),
                kg: None,
                haystacks: vec![Haystack::new(
                    "http://localhost:9883".to_string(),
                    ServiceType::Atomic,
                    true,
                    // No authentication (atomic_server_secret: None is default)
                )],
                extra: ahash::AHashMap::new(),
                llm_enabled: false,
                llm_api_key: None,
                llm_model: None,
                llm_auto_summarize: false,
                llm_chat_enabled: false,
                llm_chat_system_prompt: None,
                llm_chat_model: None,
                llm_context_window: Some(32768),
            },
        )
        .build()
        .expect("Failed to build anonymous config");

    println!("✅ Anonymous access config created successfully");
    println!("   No authentication required");
    println!("   May have limited access to public resources only");

    // Example 4: Public document server configuration
    println!("\n📋 Example 4: Public Document Server Configuration");

    let _public_docs_config = ConfigBuilder::new()
        .global_shortcut("Ctrl+P")
        .add_role(
            "PublicReader",
            Role {
                terraphim_it: false,
                shortname: Some("PublicReader".to_string()),
                name: "PublicReader".into(),
                relevance_function: RelevanceFunction::TitleScorer,
                theme: "flatly".to_string(),
                kg: None,
                haystacks: vec![
                    // Public documentation server
                    Haystack::new(
                        "https://docs.example.com".to_string(),
                        ServiceType::Atomic,
                        true,
                        // Public access to documentation (atomic_server_secret: None is default)
                    ),
                    // Public knowledge base
                    Haystack::new(
                        "https://kb.company.com".to_string(),
                        ServiceType::Atomic,
                        true,
                        // Public company knowledge base (atomic_server_secret: None is default)
                    ),
                ],
                extra: ahash::AHashMap::new(),
                llm_enabled: false,
                llm_api_key: None,
                llm_model: None,
                llm_auto_summarize: false,
                llm_chat_enabled: false,
                llm_chat_system_prompt: None,
                llm_chat_model: None,
                llm_context_window: Some(32768),
            },
        )
        .build()
        .expect("Failed to build public docs config");

    println!("✅ Public document server config created successfully");
    println!("   Multiple public atomic servers configured");
    println!("   No authentication required for any haystack");
    println!("   Read-only access to public documentation and knowledge bases");

    // Example 5: Configuration serialization
    println!("\n📋 Example 5: Configuration Serialization");

    let json_output = serde_json::to_string_pretty(&basic_config)?;
    println!("JSON configuration:");
    println!("{}", json_output);

    let toml_output = toml::to_string_pretty(&basic_config)?;
    println!("\nTOML configuration:");
    println!("{}", toml_output);

    // Example 5: Key differences between service types
    println!("\n📋 Example 5: Service Type Comparison");
    println!("🔍 Ripgrep Haystacks:");
    println!("   - location: filesystem path (e.g., './docs', '/home/user/notes')");
    println!("   - service: ServiceType::Ripgrep");
    println!("   - atomic_server_secret: None (not used)");
    println!("   - Searches local markdown files");

    println!("\n🌐 Atomic Server Haystacks:");
    println!("   - location: URL (e.g., 'http://localhost:9883', 'https://my-server.com/atomic')");
    println!("   - service: ServiceType::Atomic");
    println!("   - atomic_server_secret: Optional authentication");
    println!("     • None = Anonymous/Public access (no authentication required)");
    println!("     • Some(base64_secret) = Authenticated access (private resources)");
    println!("   - Searches remote atomic data with configurable access level");

    println!("\n🎯 Best Practices:");
    println!("   - Use read_only: true for shared/remote atomic servers");
    println!("   - Use read_only: false for local filesystems you want to edit");
    println!("   - Combine both service types for comprehensive search coverage");
    println!("   - Store secrets securely (environment variables, secure vaults)");
    println!("   - Use atomic_server_secret: None for public document servers");
    println!("   - Use atomic_server_secret: Some(secret) for private/authenticated servers");

    println!("\n🔒 Access Level Examples:");
    println!("   Public Access (atomic_server_secret: None):");
    println!("     ✓ Public documentation sites");
    println!("     ✓ Open knowledge bases");
    println!("     ✓ Community wikis");
    println!("     ✓ Educational content");

    println!("\n   Authenticated Access (atomic_server_secret: Some(secret)):");
    println!("     ✓ Private company documents");
    println!("     ✓ Personal notes and archives");
    println!("     ✓ Confidential knowledge bases");
    println!("     ✓ Team-specific resources");

    // Example 6: Multi-Agent System Integration (if feature enabled)
    // Commented out - requires terraphim_multi_agent dependency
    // #[cfg(feature = "multi_agent")]
    if false {
        println!("\n🤖 Example 6: Multi-Agent System Integration");
        println!("============================================");

        // This would create an intelligent agent from the role
        let persistence = create_test_storage().await?;
        let role = basic_config.roles.values().next().unwrap();
        let agent = TerraphimAgent::new(role.clone(), persistence, None).await?;
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

        // Show tracking information
        let token_tracker = agent.token_tracker.read().await;
        let cost_tracker = agent.cost_tracker.read().await;

        println!("\n📊 Tracking Information:");
        println!(
            "   Tokens: {} in / {} out",
            token_tracker.total_input_tokens, token_tracker.total_output_tokens
        );
        println!("   Cost: ${:.6}", cost_tracker.current_month_spending);
    }

    // Multi-agent system information
    {
        println!("\n💡 Multi-Agent System Available");
        println!("====================================");
        println!("To see the multi-agent system in action:");
        println!("   1. Add 'multi_agent' feature flag");
        println!("   2. The role configuration automatically becomes an intelligent agent");
        println!("   3. All queries become AI-powered with context enrichment");
        println!("   4. Performance tracking and learning capabilities are enabled");
        println!("\n   Example: cargo run --features multi_agent --example atomic_server_config");
    }

    println!("\n🎉 Configuration Evolution Complete!");
    println!("\n✅ Key Benefits:");
    println!("   • Seamless evolution from traditional roles to intelligent agents");
    println!("   • AI-powered query understanding and response generation");
    println!("   • Context-aware processing with knowledge graph integration");
    println!("   • Goal-aligned behavior for better user experiences");
    println!("   • Performance tracking and continuous optimization");
    println!("   • Backward compatibility with existing configurations");

    Ok(())
}

/// Create configuration from environment variables
#[allow(dead_code)]
fn create_config_from_environment() -> Result<Config, Box<dyn std::error::Error>> {
    let server_url =
        std::env::var("ATOMIC_SERVER_URL").unwrap_or_else(|_| "http://localhost:9883".to_string());

    let secret = std::env::var("ATOMIC_SERVER_SECRET").ok();

    let read_only = std::env::var("ATOMIC_READ_ONLY")
        .unwrap_or_else(|_| "true".to_string())
        .parse::<bool>()
        .unwrap_or(true);

    let config = ConfigBuilder::new()
        .global_shortcut("Ctrl+E")
        .add_role(
            "EnvironmentUser",
            Role {
                terraphim_it: false,
                shortname: Some("EnvironmentUser".to_string()),
                name: "EnvironmentUser".into(),
                relevance_function: RelevanceFunction::TitleScorer,
                theme: "default".to_string(),
                kg: None,
                haystacks: vec![Haystack::new(server_url, ServiceType::Atomic, read_only)
                    .with_atomic_secret(secret)],
                extra: ahash::AHashMap::new(),
                llm_enabled: false,
                llm_api_key: None,
                llm_model: None,
                llm_auto_summarize: false,
                llm_chat_enabled: false,
                llm_chat_system_prompt: None,
                llm_chat_model: None,
                llm_context_window: Some(32768),
            },
        )
        .build()?;

    Ok(config)
}

/// Helper function to create test storage (would be imported from multi-agent crate)
// Commented out - requires terraphim_multi_agent dependency
#[allow(dead_code)]
async fn create_test_storage(
) -> Result<std::sync::Arc<terraphim_persistence::DeviceStorage>, Box<dyn std::error::Error>> {
    // Use the safe Arc method instead of unsafe ptr::read
    let storage = DeviceStorage::arc_memory_only().await?;
    Ok(storage)
}
