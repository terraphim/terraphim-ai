//! Simple demonstration of intelligent routing
//!
//! This example shows how the proxy routes requests to different providers
//! based on the content and requirements of the request.

use std::sync::Arc;
use terraphim_llm_proxy::{
    analyzer::RequestAnalyzer,
    config::{
        ManagementSettings, OAuthSettings, Provider, ProxyConfig, ProxySettings, RouterSettings,
        SecuritySettings,
    },
    router::RouterAgent,
    routing::RoutingStrategy,
    token_counter::{ChatRequest, Message, MessageContent, TokenCounter},
    webhooks::WebhookSettings,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    // Create test providers
    let providers = vec![
        Provider {
            name: "openai".to_string(),
            api_base_url: "https://api.openai.com/v1".to_string(),
            api_key: "test-key".to_string(),
            models: vec!["gpt-4o".to_string(), "gpt-4o-mini".to_string()],
            transformers: vec![],
        },
        Provider {
            name: "anthropic".to_string(),
            api_base_url: "https://api.anthropic.com".to_string(),
            api_key: "test-key".to_string(),
            models: vec!["claude-3-5-sonnet".to_string()],
            transformers: vec![],
        },
        Provider {
            name: "deepseek".to_string(),
            api_base_url: "https://api.deepseek.com".to_string(),
            api_key: "test-key".to_string(),
            models: vec!["deepseek-reasoner".to_string()],
            transformers: vec![],
        },
    ];

    // Create configuration
    let config = ProxyConfig {
        proxy: ProxySettings {
            host: "localhost".to_string(),
            port: 8080,
            api_key: "test-key".to_string(),
            timeout_ms: 30000,
        },
        router: RouterSettings {
            default: "openai:gpt-4o".to_string(),
            background: Some("openai:gpt-4o-mini".to_string()),
            think: Some("deepseek:deepseek-reasoner".to_string()),
            plan_implementation: None,
            long_context: Some("anthropic:claude-3-5-sonnet".to_string()),
            long_context_threshold: 50000,
            web_search: Some("anthropic:claude-3-5-sonnet".to_string()),
            image: Some("openai:gpt-4o".to_string()),
            model_mappings: vec![],
            model_exclusions: vec![],
            strategy: RoutingStrategy::default(),
        },
        providers,
        security: SecuritySettings::default(),
        oauth: OAuthSettings::default(),
        management: ManagementSettings::default(),
        webhooks: WebhookSettings::default(),
    };

    // Create router
    let router = RouterAgent::new(Arc::new(config));

    // Create token counter and analyzer
    let token_counter = Arc::new(TokenCounter::new()?);
    let analyzer = RequestAnalyzer::new(token_counter);

    println!("=== Intelligent Routing Demo ===\n");

    // Test different scenarios
    let scenarios = vec![
        (
            "Image Generation",
            vec![Message {
                role: "user".to_string(),
                content: MessageContent::Text(
                    "Generate an image of a sunset over mountains".to_string(),
                ),
                ..Default::default()
            }],
        ),
        (
            "Reasoning Request",
            vec![Message {
                role: "user".to_string(),
                content: MessageContent::Text(
                    "Think step by step to solve this complex math problem: What is 47 * 89?"
                        .to_string(),
                ),
                ..Default::default()
            }],
        ),
        (
            "Simple Question",
            vec![Message {
                role: "user".to_string(),
                content: MessageContent::Text("What is the capital of France?".to_string()),
                ..Default::default()
            }],
        ),
        (
            "Long Context",
            vec![Message {
                role: "user".to_string(),
                content: MessageContent::Text(
                    "Analyze this very long document. ".to_string() + &"x".repeat(50000),
                ),
                ..Default::default()
            }],
        ),
        (
            "Web Search",
            vec![Message {
                role: "user".to_string(),
                content: MessageContent::Text(
                    "Search for the latest news about artificial intelligence".to_string(),
                ),
                ..Default::default()
            }],
        ),
    ];

    for (name, messages) in scenarios {
        println!("--- {} ---", name);

        let request = ChatRequest {
            model: "auto".to_string(),
            messages: messages.clone(),
            system: None,
            tools: None,
            max_tokens: Some(1000),
            temperature: Some(0.7),
            stream: Some(false),
            thinking: None,
            ..Default::default()
        };

        // Analyze the request
        let hints = analyzer.analyze(&request)?;
        println!("Analysis hints: {:?}", hints);

        // Route the request
        match router.route(&request, &hints).await {
            Ok(decision) => {
                println!(
                    "✅ Routed to: {} ({})",
                    decision.provider.name, decision.model
                );
                println!("   Scenario: {:?}", decision.scenario);
            }
            Err(e) => {
                println!("❌ Routing failed: {}", e);
            }
        }
        println!();
    }

    println!("=== Demo Complete ===");
    Ok(())
}
