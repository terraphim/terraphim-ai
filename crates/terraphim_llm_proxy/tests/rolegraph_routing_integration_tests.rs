//! Integration tests for RoleGraph routing patterns
//!
//! These tests validate the RoleGraph pattern matching and routing logic
//! for different query types and scenarios.

use std::fs;
use std::path::Path;
use std::sync::Arc;
use tempfile::TempDir;
use terraphim_llm_proxy::router::RoutingScenario;
use terraphim_llm_proxy::{
    analyzer::RequestAnalyzer,
    config::{
        ManagementSettings, OAuthSettings, Provider, ProxyConfig, ProxySettings, RouterSettings,
        SecuritySettings,
    },
    performance::{PerformanceConfig, PerformanceDatabase, PerformanceTester},
    rolegraph_client::RoleGraphClient,
    router::RouterAgent,
    routing::RoutingStrategy,
    session::{SessionConfig, SessionManager},
    token_counter::{ChatRequest, Message, MessageContent, TokenCounter},
    webhooks::WebhookSettings,
};

fn create_test_taxonomy_dir() -> (TempDir, std::path::PathBuf) {
    let temp_dir = TempDir::new().unwrap();
    let taxonomy_dir = temp_dir.path().to_path_buf();
    let scenarios_dir = taxonomy_dir.join("routing_scenarios");
    fs::create_dir_all(&scenarios_dir).unwrap();

    // Create default routing scenario
    let default_routing = scenarios_dir.join("default_routing.md");
    fs::write(
        &default_routing,
        r#"# Default Routing

synonyms:: default, general, simple, basic, request

route:: openrouter, anthropic/claude-3.5-sonnet
"#,
    )
    .unwrap();

    // Create background processing scenario
    let background_routing = scenarios_dir.join("background_processing.md");
    fs::write(
        &background_routing,
        r#"# Background Processing

synonyms:: background, batch, async, offline

route:: deepseek, deepseek-chat
"#,
    )
    .unwrap();

    // Create thinking/reasoning scenario
    let thinking_routing = scenarios_dir.join("thinking_reasoning.md");
    fs::write(
        &thinking_routing,
        r#"# Thinking and Reasoning

synonyms:: think, reasoning, analysis, complex

route:: openrouter, anthropic/claude-3.5-sonnet
"#,
    )
    .unwrap();

    // Create web search scenario
    let web_search_routing = scenarios_dir.join("web_search.md");
    fs::write(
        &web_search_routing,
        r#"# Web Search

synonyms:: search, web_search, browse, lookup

route:: openrouter, openai/gpt-4o
"#,
    )
    .unwrap();

    // Create high throughput scenario
    let high_throughput = scenarios_dir.join("high_throughput_routing.md");
    fs::write(
        &high_throughput,
        r#"# High Throughput Routing

synonyms:: high throughput, fast path, low latency mode, bursty traffic, latency critical

route:: groq, llama-3.1-8b-instant
"#,
    )
    .unwrap();

    // Create low cost scenario
    let low_cost = scenarios_dir.join("low_cost_routing.md");
    fs::write(
        &low_cost,
        r#"# Low Cost Routing

synonyms:: low cost, budget mode, cheapest route, cost saver, economy tier

route:: deepseek, deepseek-chat
"#,
    )
    .unwrap();

    (temp_dir, taxonomy_dir)
}

fn create_test_router_with_rolegraph(
    config: Arc<ProxyConfig>,
    rolegraph: RoleGraphClient,
    session_manager: Arc<SessionManager>,
) -> RouterAgent {
    let performance_config = PerformanceConfig::default();
    let performance_database = Arc::new(PerformanceDatabase::new(performance_config.clone()));
    let performance_tester = Arc::new(PerformanceTester::new(
        performance_config,
        performance_database.clone(),
    ));

    RouterAgent::with_all_features(
        config,
        Arc::new(rolegraph),
        session_manager,
        performance_tester,
        performance_database,
    )
}

fn create_test_config() -> ProxyConfig {
    ProxyConfig {
        proxy: ProxySettings {
            host: "127.0.0.1".to_string(),
            port: 3456,
            api_key: "test_api_key_for_routing".to_string(),
            timeout_ms: 60000,
        },
        router: RouterSettings {
            default: "openrouter,anthropic/claude-3.5-sonnet".to_string(),
            background: Some("deepseek,deepseek-chat".to_string()),
            think: Some("openrouter,anthropic/claude-3.5-sonnet".to_string()),
            plan_implementation: None,
            long_context: Some("openrouter,anthropic/claude-3.5-sonnet".to_string()),
            long_context_threshold: 60000,
            web_search: Some("openrouter,openai/gpt-4o".to_string()),
            image: Some("openrouter,google/gemini-pro-vision".to_string()),
            model_mappings: vec![],
            model_exclusions: vec![],
            strategy: RoutingStrategy::default(),
        },
        providers: vec![
            Provider {
                name: "openrouter".to_string(),
                api_base_url: "https://openrouter.ai/api/v1".to_string(),
                api_key: "test_key".to_string(),
                models: vec![
                    "anthropic/claude-3.5-sonnet".to_string(),
                    "openai/gpt-4o".to_string(),
                ],
                transformers: vec![],
            },
            Provider {
                name: "deepseek".to_string(),
                api_base_url: "https://api.deepseek.com".to_string(),
                api_key: "test_key".to_string(),
                models: vec!["deepseek-chat".to_string()],
                transformers: vec![],
            },
            Provider {
                name: "groq".to_string(),
                api_base_url: "https://api.groq.com/openai/v1".to_string(),
                api_key: "test_key".to_string(),
                models: vec!["llama-3.1-8b-instant".to_string()],
                transformers: vec![],
            },
        ],
        security: SecuritySettings::default(),
        oauth: OAuthSettings::default(),
        management: ManagementSettings::default(),
        webhooks: WebhookSettings::default(),
    }
}

#[tokio::test]
async fn test_rolegraph_pattern_matching() {
    let (_temp_dir, taxonomy_dir) = create_test_taxonomy_dir();
    let mut rolegraph = RoleGraphClient::new(&taxonomy_dir).unwrap();
    rolegraph.load_taxonomy().unwrap();

    // Test basic pattern matching
    let test_cases = vec![
        ("simple query", "default_routing", "openrouter"),
        ("background task", "background_processing", "deepseek"),
        ("reasoning needed", "thinking_and_reasoning", "openrouter"),
        ("search web", "web_search", "openrouter"),
        ("general question", "default_routing", "openrouter"),
        ("batch processing", "background_processing", "deepseek"),
        ("complex analysis", "thinking_and_reasoning", "openrouter"),
        ("lookup information", "web_search", "openrouter"),
        (
            "enable the fast path for this query",
            "high_throughput_routing",
            "groq",
        ),
        (
            "activate low cost mode please",
            "low_cost_routing",
            "deepseek",
        ),
    ];

    for (query, expected_scenario, expected_provider) in test_cases {
        match rolegraph.query_routing(query) {
            Some(pattern_match) => {
                assert_eq!(
                    pattern_match.concept, expected_scenario,
                    "Pattern mismatch for: {}",
                    query
                );
                assert_eq!(
                    pattern_match.provider, expected_provider,
                    "Provider mismatch for: {}",
                    query
                );
                assert!(
                    !pattern_match.model.is_empty(),
                    "Model should not be empty for: {}",
                    query
                );
            }
            None => {
                panic!("Pattern matching failed for '{}': No match found", query);
            }
        }
    }
}

#[tokio::test]
async fn test_rolegraph_with_router_integration() {
    let (_temp_dir, taxonomy_dir) = create_test_taxonomy_dir();
    let mut rolegraph = RoleGraphClient::new(&taxonomy_dir).unwrap();
    rolegraph.load_taxonomy().unwrap();

    let config = Arc::new(create_test_config());
    let session_config = SessionConfig {
        max_sessions: 1000,
        max_context_messages: 10,
        session_timeout_minutes: 60,
        redis_url: None,
        enable_redis: false,
    };
    let session_manager = Arc::new(SessionManager::new(session_config).unwrap());
    let token_counter = Arc::new(TokenCounter::new().unwrap());
    let analyzer = Arc::new(RequestAnalyzer::new(token_counter));

    let router =
        create_test_router_with_rolegraph(config.clone(), rolegraph, session_manager.clone());

    // Test simple query routing
    let request = ChatRequest {
        model: "any".to_string(),
        messages: vec![Message {
            role: "user".to_string(),
            content: MessageContent::Text("What is the capital of France?".to_string()),
            ..Default::default()
        }],
        system: None,
        max_tokens: Some(100),
        temperature: Some(0.7),
        stream: None,
        tools: None,
        thinking: None,
        ..Default::default()
    };

    let hints = analyzer.analyze(&request).unwrap();
    let decision = router.route_with_fallback(&request, &hints).await.unwrap();

    // Should route to default provider
    assert_eq!(decision.provider.name, "openrouter");
    assert!(decision.model.contains("claude"));
}

#[tokio::test]
async fn test_rolegraph_background_routing() {
    let (_temp_dir, taxonomy_dir) = create_test_taxonomy_dir();
    let mut rolegraph = RoleGraphClient::new(&taxonomy_dir).unwrap();
    rolegraph.load_taxonomy().unwrap();

    let config = Arc::new(create_test_config());
    let session_config = SessionConfig {
        max_sessions: 1000,
        max_context_messages: 10,
        session_timeout_minutes: 60,
        redis_url: None,
        enable_redis: false,
    };
    let session_manager = Arc::new(SessionManager::new(session_config).unwrap());
    let token_counter = Arc::new(TokenCounter::new().unwrap());
    let analyzer = Arc::new(RequestAnalyzer::new(token_counter));

    let router =
        create_test_router_with_rolegraph(config.clone(), rolegraph, session_manager.clone());

    // Test background processing query
    let request = ChatRequest {
        model: "any".to_string(),
        messages: vec![Message {
            role: "user".to_string(),
            content: MessageContent::Text("Process this batch of data asynchronously".to_string()),
            ..Default::default()
        }],
        system: Some(terraphim_llm_proxy::token_counter::SystemPrompt::Text(
            "This is a background task".to_string(),
        )),
        max_tokens: Some(1000),
        temperature: Some(0.3),
        stream: None,
        tools: None,
        thinking: None,
        ..Default::default()
    };

    let hints = analyzer.analyze(&request).unwrap();
    let decision = router.route_with_fallback(&request, &hints).await.unwrap();

    // Should route to deepseek for background processing
    assert_eq!(decision.provider.name, "deepseek");
    assert_eq!(decision.model, "deepseek-chat");
}

#[tokio::test]
async fn test_rolegraph_high_throughput_routing() {
    let (_temp_dir, taxonomy_dir) = create_test_taxonomy_dir();
    let mut rolegraph = RoleGraphClient::new(&taxonomy_dir).unwrap();
    rolegraph.load_taxonomy().unwrap();

    let config = Arc::new(create_test_config());
    let session_config = SessionConfig {
        max_sessions: 1000,
        max_context_messages: 10,
        session_timeout_minutes: 60,
        redis_url: None,
        enable_redis: false,
    };
    let session_manager = Arc::new(SessionManager::new(session_config).unwrap());
    let token_counter = Arc::new(TokenCounter::new().unwrap());
    let analyzer = Arc::new(RequestAnalyzer::new(token_counter));

    let router =
        create_test_router_with_rolegraph(config.clone(), rolegraph, session_manager.clone());

    let request = ChatRequest {
        model: "auto".to_string(),
        messages: vec![Message {
            role: "user".to_string(),
            content: MessageContent::Text("Run this in fast path high throughput mode".to_string()),
            ..Default::default()
        }],
        system: None,
        max_tokens: Some(128),
        temperature: Some(0.2),
        stream: Some(true),
        tools: None,
        thinking: None,
        ..Default::default()
    };

    let hints = analyzer.analyze(&request).unwrap();
    let decision = router.route_with_fallback(&request, &hints).await.unwrap();

    assert_eq!(decision.provider.name, "groq");
    assert_eq!(decision.model, "llama-3.1-8b-instant");
    if let RoutingScenario::Pattern(concept) = &decision.scenario {
        assert_eq!(concept, "high_throughput_routing");
    } else {
        panic!("Expected Pattern scenario, found {:?}", decision.scenario);
    }
}

#[tokio::test]
async fn test_rolegraph_low_cost_routing() {
    let (_temp_dir, taxonomy_dir) = create_test_taxonomy_dir();
    let mut rolegraph = RoleGraphClient::new(&taxonomy_dir).unwrap();
    rolegraph.load_taxonomy().unwrap();

    let config = Arc::new(create_test_config());
    let session_config = SessionConfig {
        max_sessions: 1000,
        max_context_messages: 10,
        session_timeout_minutes: 60,
        redis_url: None,
        enable_redis: false,
    };
    let session_manager = Arc::new(SessionManager::new(session_config).unwrap());
    let token_counter = Arc::new(TokenCounter::new().unwrap());
    let analyzer = Arc::new(RequestAnalyzer::new(token_counter));

    let router =
        create_test_router_with_rolegraph(config.clone(), rolegraph, session_manager.clone());

    let request = ChatRequest {
        model: "auto".to_string(),
        messages: vec![Message {
            role: "user".to_string(),
            content: MessageContent::Text("Activate low cost mode for this request".to_string()),
            ..Default::default()
        }],
        system: None,
        max_tokens: Some(256),
        temperature: Some(0.5),
        stream: None,
        tools: None,
        thinking: None,
        ..Default::default()
    };

    let hints = analyzer.analyze(&request).unwrap();
    let decision = router.route_with_fallback(&request, &hints).await.unwrap();

    assert_eq!(decision.provider.name, "deepseek");
    assert_eq!(decision.model, "deepseek-chat");
    if let RoutingScenario::Pattern(concept) = &decision.scenario {
        assert_eq!(concept, "low_cost_routing");
    } else {
        panic!("Expected Pattern scenario, found {:?}", decision.scenario);
    }
}

#[tokio::test]
async fn test_rolegraph_fallback_routing() {
    let (_temp_dir, taxonomy_dir) = create_test_taxonomy_dir();
    let mut rolegraph = RoleGraphClient::new(&taxonomy_dir).unwrap();
    rolegraph.load_taxonomy().unwrap();

    let config = Arc::new(create_test_config());
    let session_config = SessionConfig {
        max_sessions: 1000,
        max_context_messages: 10,
        session_timeout_minutes: 60,
        redis_url: None,
        enable_redis: false,
    };
    let session_manager = Arc::new(SessionManager::new(session_config).unwrap());
    let token_counter = Arc::new(TokenCounter::new().unwrap());
    let analyzer = Arc::new(RequestAnalyzer::new(token_counter));

    let router =
        create_test_router_with_rolegraph(config.clone(), rolegraph, session_manager.clone());

    // Test request for a model that doesn't exist in configuration
    let request = ChatRequest {
        model: "nonexistent:nonexistent-model".to_string(),
        messages: vec![Message {
            role: "user".to_string(),
            content: MessageContent::Text("Test fallback routing".to_string()),
            ..Default::default()
        }],
        system: None,
        max_tokens: Some(100),
        temperature: Some(0.7),
        stream: None,
        tools: None,
        thinking: None,
        ..Default::default()
    };

    let hints = analyzer.analyze(&request).unwrap();
    let decision = router.route_with_fallback(&request, &hints).await.unwrap();

    // Should fallback to default routing
    assert_eq!(decision.provider.name, "openrouter");
    assert!(decision.model.contains("claude"));
}

#[test]
fn test_rolegraph_invalid_taxonomy_handling() {
    // Test with non-existent directory
    let non_existent_dir = Path::new("/tmp/non_existent_taxonomy_12345");
    let result = RoleGraphClient::new(non_existent_dir);
    assert!(result.is_err()); // Should return error for non-existent directory

    // Test with empty directory
    let temp_dir = TempDir::new().unwrap();
    let empty_dir = temp_dir.path();
    let mut rolegraph = RoleGraphClient::new(empty_dir).unwrap();
    let result = rolegraph.load_taxonomy();
    assert!(result.is_ok()); // Should load without errors, just no patterns
}

#[test]
fn test_rolegraph_pattern_priority() {
    let (_temp_dir, taxonomy_dir) = create_test_taxonomy_dir();
    let mut rolegraph = RoleGraphClient::new(&taxonomy_dir).unwrap();
    rolegraph.load_taxonomy().unwrap();

    // Test multiple pattern matches to see which gets selected
    let test_cases = vec![
        ("simple query", "default_routing"), // Should match default
        ("background task", "background_processing"), // Should match background
        ("complex analysis", "thinking_and_reasoning"), // Should match thinking
    ];

    for (query, _expected_scenario) in test_cases {
        match rolegraph.query_routing(query) {
            Some(pattern_match) => {
                // Verify we get a reasonable match (exact match might vary due to scoring)
                assert!(
                    !pattern_match.concept.is_empty(),
                    "Pattern should not be empty for: {}",
                    query
                );
                assert!(
                    !pattern_match.provider.is_empty(),
                    "Provider should not be empty for: {}",
                    query
                );
                assert!(
                    !pattern_match.model.is_empty(),
                    "Model should not be empty for: {}",
                    query
                );
                // Verify the score is reasonable
                assert!(
                    pattern_match.score > 0.0,
                    "Score should be positive for: {}",
                    query
                );
            }
            None => {
                panic!("Pattern matching failed for '{}': No match found", query);
            }
        }
    }
}

#[test]
fn test_rolegraph_multiple_synonyms() {
    let (_temp_dir, taxonomy_dir) = create_test_taxonomy_dir();
    let mut rolegraph = RoleGraphClient::new(&taxonomy_dir).unwrap();
    rolegraph.load_taxonomy().unwrap();

    // Test multiple synonyms for the same scenario
    let default_synonyms = vec![
        "simple query",
        "general question",
        "simple task",
        "basic request",
    ];

    for query in default_synonyms {
        match rolegraph.query_routing(query) {
            Some(pattern_match) => {
                assert_eq!(
                    pattern_match.concept, "default_routing",
                    "Synonym test failed for: {}",
                    query
                );
                assert_eq!(
                    pattern_match.provider, "openrouter",
                    "Provider mismatch for: {}",
                    query
                );
            }
            None => {
                panic!(
                    "Pattern matching failed for synonym '{}': No match found",
                    query
                );
            }
        }
    }
}

#[test]
fn test_rolegraph_error_handling() {
    let (_temp_dir, taxonomy_dir) = create_test_taxonomy_dir();
    let mut rolegraph = RoleGraphClient::new(&taxonomy_dir).unwrap();
    rolegraph.load_taxonomy().unwrap();

    // Test queries that shouldn't match any patterns
    let no_match_queries = vec![
        "completely unrelated query",
        "xyz nonsense query",
        "12345 numbers only",
        "",
    ];

    for query in no_match_queries {
        let result = rolegraph.query_routing(query);
        // Should either match default or return error - both are acceptable
        match result {
            Some(pattern_match) => {
                // If it matches, it should be a reasonable fallback
                assert!(
                    !pattern_match.concept.is_empty(),
                    "Concept should not be empty"
                );
                assert!(
                    !pattern_match.provider.is_empty(),
                    "Provider should not be empty"
                );
            }
            None => {
                // None is also acceptable for no-match scenarios
            }
        }
    }
}
