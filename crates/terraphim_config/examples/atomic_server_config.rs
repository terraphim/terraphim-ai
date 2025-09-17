use terraphim_config::{Config, ConfigBuilder, Haystack, Role, ServiceType};
use terraphim_types::RelevanceFunction;

/// Example demonstrating how to configure Terraphim with atomic server haystacks
///
/// This example shows how to create a complete Terraphim configuration that includes
/// both traditional ripgrep haystacks and atomic server haystacks for hybrid search.
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    env_logger::init();

    // Example 1: Basic atomic server haystack configuration
    println!("📋 Example 1: Basic Atomic Server Haystack Configuration");

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
                extra: ahash::AHashMap::new(),
                #[cfg(feature = "openrouter")]
                openrouter_enabled: false,
                #[cfg(feature = "openrouter")]
                openrouter_api_key: None,
                #[cfg(feature = "openrouter")]
                openrouter_model: None,
                #[cfg(feature = "openrouter")]
                openrouter_auto_summarize: false,
                #[cfg(feature = "openrouter")]
                openrouter_chat_enabled: false,
                #[cfg(feature = "openrouter")]
                openrouter_chat_system_prompt: None,
                #[cfg(feature = "openrouter")]
                openrouter_chat_model: None,
                llm_system_prompt: None,
            },
        )
        .build()
        .expect("Failed to build basic config");

    println!("✅ Basic atomic server config created successfully");
    println!("   Server URL: http://localhost:9883");
    println!("   Authentication: Required (secret provided)");
    println!("   Read-only: true");

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
                #[cfg(feature = "openrouter")]
                openrouter_enabled: false,
                #[cfg(feature = "openrouter")]
                openrouter_api_key: None,
                #[cfg(feature = "openrouter")]
                openrouter_model: None,
                #[cfg(feature = "openrouter")]
                openrouter_auto_summarize: false,
                #[cfg(feature = "openrouter")]
                openrouter_chat_enabled: false,
                #[cfg(feature = "openrouter")]
                openrouter_chat_system_prompt: None,
                #[cfg(feature = "openrouter")]
                openrouter_chat_model: None,
                llm_system_prompt: None,
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
                #[cfg(feature = "openrouter")]
                openrouter_enabled: false,
                #[cfg(feature = "openrouter")]
                openrouter_api_key: None,
                #[cfg(feature = "openrouter")]
                openrouter_model: None,
                #[cfg(feature = "openrouter")]
                openrouter_auto_summarize: false,
                #[cfg(feature = "openrouter")]
                openrouter_chat_enabled: false,
                #[cfg(feature = "openrouter")]
                openrouter_chat_system_prompt: None,
                #[cfg(feature = "openrouter")]
                openrouter_chat_model: None,
                llm_system_prompt: None,
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
                #[cfg(feature = "openrouter")]
                openrouter_enabled: false,
                #[cfg(feature = "openrouter")]
                openrouter_api_key: None,
                #[cfg(feature = "openrouter")]
                openrouter_model: None,
                #[cfg(feature = "openrouter")]
                openrouter_auto_summarize: false,
                #[cfg(feature = "openrouter")]
                openrouter_chat_enabled: false,
                #[cfg(feature = "openrouter")]
                openrouter_chat_system_prompt: None,
                #[cfg(feature = "openrouter")]
                openrouter_chat_model: None,
                llm_system_prompt: None,
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
                #[cfg(feature = "openrouter")]
                openrouter_enabled: false,
                #[cfg(feature = "openrouter")]
                openrouter_api_key: None,
                #[cfg(feature = "openrouter")]
                openrouter_model: None,
                #[cfg(feature = "openrouter")]
                openrouter_auto_summarize: false,
                #[cfg(feature = "openrouter")]
                openrouter_chat_enabled: false,
                #[cfg(feature = "openrouter")]
                openrouter_chat_system_prompt: None,
                #[cfg(feature = "openrouter")]
                openrouter_chat_model: None,
                llm_system_prompt: None,
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
                #[cfg(feature = "openrouter")]
                openrouter_enabled: false,
                #[cfg(feature = "openrouter")]
                openrouter_api_key: None,
                #[cfg(feature = "openrouter")]
                openrouter_model: None,
                #[cfg(feature = "openrouter")]
                openrouter_auto_summarize: false,
                #[cfg(feature = "openrouter")]
                openrouter_chat_enabled: false,
                #[cfg(feature = "openrouter")]
                openrouter_chat_system_prompt: None,
                #[cfg(feature = "openrouter")]
                openrouter_chat_model: None,
                llm_system_prompt: None,
            },
        )
        .build()?;

    Ok(config)
}
