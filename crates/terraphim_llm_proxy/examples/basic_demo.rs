//! Basic demonstration of the Terraphim LLM Proxy
//!
//! This example shows how to create a simple request and test the token counter.

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

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    println!("=== Terraphim LLM Proxy Basic Demo ===\n");

    // Create a simple chat request
    let request = ChatRequest {
        model: "gpt-4o".to_string(),
        messages: vec![Message {
            role: "user".to_string(),
            content: MessageContent::Text("What is the capital of France?".to_string()),
            ..Default::default()
        }],
        system: None,
        tools: None,
        max_tokens: Some(1000),
        temperature: Some(0.7),
        stream: Some(false),
        thinking: None,
        ..Default::default()
    };

    // Count tokens
    let token_counter = TokenCounter::new()?;
    let token_count = token_counter.count_request(&request)?;
    println!("Token count for simple request: {}", token_count);

    // Create a more complex request with tools
    let complex_request = ChatRequest {
        model: "claude-3-5-sonnet".to_string(),
        messages: vec![Message {
            role: "user".to_string(),
            content: MessageContent::Text(
                "Search for the latest AI news and summarize it.".to_string(),
            ),
            ..Default::default()
        }],
        system: None,
        tools: Some(vec![terraphim_llm_proxy::token_counter::Tool {
            tool_type: Some("function".to_string()),
            function: Some(terraphim_llm_proxy::token_counter::FunctionTool {
                name: "web_search".to_string(),
                description: Some("Search the web for information".to_string()),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "query": {"type": "string"}
                    }
                }),
            }),
            name: None,
            description: None,
            input_schema: None,
        }]),
        max_tokens: Some(2000),
        temperature: Some(0.3),
        stream: Some(false),
        thinking: None,
        ..Default::default()
    };

    let complex_token_count = token_counter.count_request(&complex_request)?;
    println!(
        "Token count for complex request with tools: {}",
        complex_token_count
    );

    // Create configuration for routing demo
    let config = ProxyConfig {
        proxy: ProxySettings {
            host: "localhost".to_string(),
            port: 8080,
            api_key: "demo-key".to_string(),
            timeout_ms: 30000,
        },
        router: RouterSettings {
            default: "openai,gpt-4o".to_string(),
            background: Some("openai,gpt-4o-mini".to_string()),
            think: Some("deepseek,deepseek-reasoner".to_string()),
            plan_implementation: None,
            long_context: Some("anthropic,claude-3-5-sonnet".to_string()),
            long_context_threshold: 50000,
            web_search: Some("anthropic,claude-3-5-sonnet".to_string()),
            image: Some("openai,gpt-4o".to_string()),
            model_mappings: vec![],
            model_exclusions: vec![],
            strategy: RoutingStrategy::default(),
        },
        providers: vec![
            Provider {
                name: "openai".to_string(),
                api_base_url: "https://api.openai.com/v1".to_string(),
                api_key: "demo-key".to_string(),
                models: vec!["gpt-4o".to_string(), "gpt-4o-mini".to_string()],
                transformers: vec![],
            },
            Provider {
                name: "anthropic".to_string(),
                api_base_url: "https://api.anthropic.com".to_string(),
                api_key: "demo-key".to_string(),
                models: vec!["claude-3-5-sonnet".to_string()],
                transformers: vec![],
            },
            Provider {
                name: "deepseek".to_string(),
                api_base_url: "https://api.deepseek.com".to_string(),
                api_key: "demo-key".to_string(),
                models: vec!["deepseek-reasoner".to_string()],
                transformers: vec![],
            },
        ],
        security: SecuritySettings::default(),
        oauth: OAuthSettings::default(),
        management: ManagementSettings::default(),
        webhooks: WebhookSettings::default(),
    };

    // Create router
    let router = RouterAgent::new(Arc::new(config));
    let token_counter = Arc::new(TokenCounter::new()?);
    let analyzer = RequestAnalyzer::new(token_counter);

    println!("\n--- Routing Analysis ---");

    // Test different scenarios
    let scenarios: Vec<(&str, String)> = vec![
        (
            "Simple Question",
            "What is the capital of France?".to_string(),
        ),
        (
            "Reasoning Request",
            "Think step by step to solve: What is 47 * 89?".to_string(),
        ),
        (
            "Image Generation",
            "Generate an image of a sunset over mountains".to_string(),
        ),
        (
            "Web Search",
            "Search for the latest news about AI".to_string(),
        ),
        ("Long Context", {
            let mut s = "Analyze this document. ".to_string();
            s.push_str(&"x".repeat(60000));
            s
        }),
    ];

    for (name, content) in scenarios {
        println!("\n--- {} ---", name);

        let test_request = ChatRequest {
            model: "auto".to_string(),
            messages: vec![Message {
                role: "user".to_string(),
                content: MessageContent::Text(content.to_string()),
                ..Default::default()
            }],
            system: None,
            tools: if name.contains("Web Search") || name.contains("Search") {
                Some(vec![terraphim_llm_proxy::token_counter::Tool {
                    tool_type: Some("function".to_string()),
                    function: Some(terraphim_llm_proxy::token_counter::FunctionTool {
                        name: "web_search".to_string(),
                        description: Some("Search the web".to_string()),
                        parameters: serde_json::json!({"type": "object"}),
                    }),
                    name: None,
                    description: None,
                    input_schema: None,
                }])
            } else {
                None
            },
            max_tokens: Some(1000),
            temperature: Some(0.7),
            stream: Some(false),
            thinking: None,
            ..Default::default()
        };

        // Analyze
        match analyzer.analyze(&test_request) {
            Ok(hints) => {
                println!("Analysis: {:?}", hints);

                // Route (using tokio runtime for async)
                let rt = tokio::runtime::Runtime::new()?;
                match rt.block_on(router.route(&test_request, &hints)) {
                    Ok(decision) => {
                        println!("✅ Route: {} -> {}", decision.provider.name, decision.model);
                        println!("   Scenario: {:?}", decision.scenario);
                    }
                    Err(e) => {
                        println!("❌ Routing failed: {}", e);
                    }
                }
            }
            Err(e) => {
                println!("❌ Analysis failed: {}", e);
            }
        }
    }

    println!("\n=== Demo Complete ===");
    Ok(())
}
