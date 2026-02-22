//! Comprehensive routing demonstration
//!
//! This example demonstrates all routing scenarios with detailed output

use std::sync::Arc;
use terraphim_llm_proxy::{
    analyzer::RequestAnalyzer,
    config::{
        ManagementSettings, OAuthSettings, Provider, ProxyConfig, ProxySettings, RouterSettings,
        SecuritySettings,
    },
    router::RouterAgent,
    routing::RoutingStrategy,
    token_counter::{ChatRequest, FunctionTool, Message, MessageContent, Tool},
    webhooks::WebhookSettings,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    println!("=== Terraphim LLM Proxy Routing Demo ===\n");

    // Create configuration
    let config = ProxyConfig {
        proxy: ProxySettings {
            host: "localhost".to_string(),
            port: 8080,
            api_key: "demo-key".to_string(),
            timeout_ms: 30000,
        },
        router: RouterSettings {
            default: "openai,gpt-4o".to_string(),
            background: Some("groq,llama-3.1-8b".to_string()),
            think: Some("deepseek,deepseek-reasoner".to_string()),
            plan_implementation: None,
            long_context: Some("anthropic,claude-3-5-sonnet".to_string()),
            long_context_threshold: 50000,
            web_search: Some("perplexity,sonar-reasoning".to_string()),
            image: Some("openai,dall-e-3".to_string()),
            model_mappings: vec![],
            model_exclusions: vec![],
            strategy: RoutingStrategy::default(),
        },
        providers: vec![
            Provider {
                name: "openai".to_string(),
                api_base_url: "https://api.openai.com/v1".to_string(),
                api_key: "demo-key".to_string(),
                models: vec![
                    "gpt-4o".to_string(),
                    "gpt-4o-mini".to_string(),
                    "dall-e-3".to_string(),
                ],
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
            Provider {
                name: "groq".to_string(),
                api_base_url: "https://api.groq.com/openai/v1".to_string(),
                api_key: "demo-key".to_string(),
                models: vec!["llama-3.1-8b".to_string()],
                transformers: vec![],
            },
            Provider {
                name: "perplexity".to_string(),
                api_base_url: "https://api.perplexity.ai".to_string(),
                api_key: "demo-key".to_string(),
                models: vec!["sonar-reasoning".to_string()],
                transformers: vec![],
            },
        ],
        security: SecuritySettings::default(),
        oauth: OAuthSettings::default(),
        management: ManagementSettings::default(),
        webhooks: WebhookSettings::default(),
    };

    // Create router and analyzer
    let router = RouterAgent::new(Arc::new(config));
    let token_counter = Arc::new(terraphim_llm_proxy::token_counter::TokenCounter::new()?);
    let analyzer = RequestAnalyzer::new(token_counter);

    // Define test scenarios
    let scenarios = [
        ("Simple Question", "What is the capital of France?", None),
        ("Image Generation", "Generate an image of a sunset over mountains", None),
        ("Web Search", "Search for the latest news about artificial intelligence", Some(vec![
            Tool {
                tool_type: Some("function".to_string()),
                function: Some(FunctionTool {
                    name: "web_search".to_string(),
                    description: Some("Search the web for current information".to_string()),
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
            }
        ])),
        ("Complex Reasoning", "Think step by step to solve: A company has 150 employees. If 20% work in engineering, 30% in sales, and the rest in operations, how many work in operations?", None),
        ("Code Generation", "Write a Python function that implements binary search with proper error handling", None),
        ("Creative Writing", "Write a short story about a time traveler who changes history", None),
        ("Background Task", "What's 2+2?", None),
    ];

    println!("--- Testing Routing Scenarios ---\n");

    for (i, (name, content, tools)) in scenarios.iter().enumerate() {
        println!("{}. {}", i + 1, name);
        println!("   Request: {}", content);

        let request = ChatRequest {
            model: "auto".to_string(),
            messages: vec![Message {
                role: "user".to_string(),
                content: MessageContent::Text(content.to_string()),
                ..Default::default()
            }],
            system: None,
            tools: tools.clone(),
            max_tokens: Some(1000),
            temperature: Some(0.7),
            stream: Some(false),
            thinking: if name.contains("Reasoning") {
                Some(serde_json::json!({}))
            } else {
                None
            },
            ..Default::default()
        };

        // Analyze the request
        match analyzer.analyze(&request) {
            Ok(hints) => {
                println!("   Analysis:");
                println!("     - Tokens: {}", hints.token_count);
                println!("     - Background: {}", hints.is_background);
                println!("     - Thinking: {}", hints.has_thinking);
                println!("     - Web Search: {}", hints.has_web_search);
                println!("     - Images: {}", hints.has_images);

                // Route the request
                match router.route(&request, &hints).await {
                    Ok(decision) => {
                        println!(
                            "   ✅ Route: {} -> {}",
                            decision.provider.name, decision.model
                        );
                        println!("     Scenario: {:?}", decision.scenario);
                    }
                    Err(e) => {
                        println!("   ❌ Routing failed: {}", e);
                    }
                }
            }
            Err(e) => {
                println!("   ❌ Analysis failed: {}", e);
            }
        }
        println!();
    }

    // Test long context scenario
    println!("{}. Long Context", scenarios.len() + 1);
    println!("   Request: Long document analysis (60,000+ tokens)");

    let long_content = "Analyze this research paper. ".to_string() + &"x".repeat(60000);
    let long_request = ChatRequest {
        model: "auto".to_string(),
        messages: vec![Message {
            role: "user".to_string(),
            content: MessageContent::Text(long_content),
            ..Default::default()
        }],
        system: None,
        tools: None,
        max_tokens: Some(2000),
        temperature: Some(0.3),
        stream: Some(false),
        thinking: None,
        ..Default::default()
    };

    match analyzer.analyze(&long_request) {
        Ok(hints) => {
            println!("   Analysis:");
            println!("     - Tokens: {}", hints.token_count);
            println!("     - Long Context: {}", hints.token_count > 50000);

            match router.route(&long_request, &hints).await {
                Ok(decision) => {
                    println!(
                        "   ✅ Route: {} -> {}",
                        decision.provider.name, decision.model
                    );
                    println!("     Scenario: {:?}", decision.scenario);
                }
                Err(e) => {
                    println!("   ❌ Routing failed: {}", e);
                }
            }
        }
        Err(e) => {
            println!("   ❌ Analysis failed: {}", e);
        }
    }

    println!("\n=== Routing Summary ===");
    println!("✅ Rule-based routing: Simple questions, background tasks, long context");
    println!("✅ Tool-based routing: Web search with tools");
    println!("✅ Feature-based routing: Thinking field for reasoning");
    println!("✅ Pattern matching: Available with RoleGraph integration");
    println!("✅ Multi-provider support: OpenAI, Anthropic, DeepSeek, Groq, Perplexity");

    println!("\n=== Demo Complete ===");
    Ok(())
}
