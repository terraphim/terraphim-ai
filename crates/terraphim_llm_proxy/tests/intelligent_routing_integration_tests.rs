//! Integration tests for Intelligent Routing (Scenario 2)
//!
//! These tests specifically validate the think_routing pattern matching
//! and intelligent routing functionality that was fixed to properly detect
//! "think", "plan", and reasoning keywords in message content.

use std::fs;
use std::sync::Arc;
use tempfile::TempDir;
use terraphim_llm_proxy::{
    analyzer::RequestAnalyzer,
    config::{
        ManagementSettings, OAuthSettings, Provider, ProxyConfig, ProxySettings, RouterSettings,
        SecuritySettings,
    },
    performance::{PerformanceConfig, PerformanceDatabase, PerformanceTester},
    rolegraph_client::RoleGraphClient,
    router::{RouterAgent, RoutingScenario},
    routing::RoutingStrategy,
    session::{SessionConfig, SessionManager},
    token_counter::{ChatRequest, Message, MessageContent, TokenCounter},
    webhooks::WebhookSettings,
};

fn create_intelligent_routing_taxonomy() -> (TempDir, std::path::PathBuf) {
    let temp_dir = TempDir::new().unwrap();
    let taxonomy_dir = temp_dir.path().to_path_buf();
    let scenarios_dir = taxonomy_dir.join("routing_scenarios");
    fs::create_dir_all(&scenarios_dir).unwrap();

    // Create think_routing scenario with enhanced keywords (the actual fix)
    let think_routing = scenarios_dir.join("think_routing.md");
    fs::write(
        &think_routing,
        r#"# Think Routing

Think routing (also known as reasoning mode) is used for complex problem-solving tasks that require deep reasoning, step-by-step analysis, or extended chain-of-thought processing. This routing scenario activates when Claude Code enters "Plan Mode" or when explicit thinking is required.

route:: openrouter, deepseek/deepseek-v3.1-terminus

synonyms:: think, plan, reason, analyze, break, down, step, by, through, consider, carefully, work, design, systematic, logical, critical, problem, solving, strategic, planning, chain, thought, deep, routing, mode, model, step by step, think through, reason through, consider carefully, work through, design thinking, systematic thinking, logical reasoning, critical thinking, problem solving, strategic planning, chain-of-thought, deep reasoning, thinking routing, reasoning routing, reasoning mode, plan mode, think model
"#,
    )
    .unwrap();

    // Create default routing for fallback
    let default_routing = scenarios_dir.join("default_routing.md");
    fs::write(
        &default_routing,
        r#"# Default Routing

synonyms:: default, general, simple, basic, request

route:: openrouter, anthropic/claude-3.5-sonnet
"#,
    )
    .unwrap();

    (temp_dir, taxonomy_dir)
}

fn create_test_config_with_intelligent_routing() -> ProxyConfig {
    ProxyConfig {
        proxy: ProxySettings {
            host: "127.0.0.1".to_string(),
            port: 3456,
            api_key: "test_api_key".to_string(),
            timeout_ms: 60000,
        },
        router: RouterSettings {
            default: "openrouter,anthropic/claude-3.5-sonnet".to_string(),
            background: Some("deepseek,deepseek-chat".to_string()),
            think: Some("openrouter,deepseek/deepseek-v3.1-terminus".to_string()),
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
                    "deepseek/deepseek-v3.1-terminus".to_string(),
                ],
                transformers: vec![],
            },
            Provider {
                name: "deepseek".to_string(),
                api_base_url: "https://api.deepseek.com".to_string(),
                api_key: "test_key".to_string(),
                models: vec!["deepseek-reasoner".to_string(), "deepseek-chat".to_string()],
                transformers: vec![],
            },
        ],
        security: SecuritySettings::default(),
        oauth: OAuthSettings::default(),
        management: ManagementSettings::default(),
        webhooks: WebhookSettings::default(),
    }
}

fn create_router_with_intelligent_routing(
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

#[tokio::test]
async fn test_intelligent_think_routing_with_enhanced_keywords() {
    let (_temp_dir, taxonomy_dir) = create_intelligent_routing_taxonomy();
    let mut rolegraph = RoleGraphClient::new(&taxonomy_dir).unwrap();
    rolegraph.load_taxonomy().unwrap();

    let config = Arc::new(create_test_config_with_intelligent_routing());
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
        create_router_with_intelligent_routing(config.clone(), rolegraph, session_manager.clone());

    // Test cases for think routing with the enhanced keywords we added
    let think_routing_test_cases = vec![
        (
            "I need you to think through this step by step and plan out a solution",
            vec!["step by step"],
        ),
        (
            "Please analyze this problem carefully and break it down",
            vec!["analyze", "break down"],
        ),
        (
            "Let's reason through this systematically",
            vec!["reason through", "systematically"],
        ),
        (
            "I need you to think through the implications",
            vec!["think through"],
        ),
        (
            "Work through this design thinking exercise",
            vec!["work through", "design thinking"],
        ),
        (
            "Apply logical reasoning to solve this",
            vec!["logical reasoning"],
        ),
        (
            "Use critical thinking to evaluate",
            vec!["critical thinking"],
        ),
        ("Help me plan out this project strategy", vec!["plan"]),
        (
            "I need strategic planning for this initiative",
            vec!["strategic planning"],
        ),
        (
            "Let's do some problem solving here",
            vec!["problem solving"],
        ),
        (
            "Time for some deep reasoning on this topic",
            vec!["deep reasoning"],
        ),
        (
            "Can you chain-of-thought through this?",
            vec!["chain-of-thought"],
        ),
        ("Please consider this carefully", vec!["consider"]),
        (
            "This requires systematic thinking",
            vec!["systematic thinking"],
        ),
    ];

    for (query, keywords) in think_routing_test_cases {
        let request = ChatRequest {
            model: "auto".to_string(),
            messages: vec![Message {
                role: "user".to_string(),
                content: MessageContent::Text(query.to_string()),
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

        // Should route to openrouter with deepseek model for intelligent reasoning
        assert_eq!(
            decision.provider.name, "openrouter",
            "Failed to route think query to openrouter: '{}'",
            query
        );
        assert_eq!(
            decision.model, "deepseek/deepseek-v3.1-terminus",
            "Failed to select deepseek-v3.1-terminus model for query: '{}'",
            query
        );

        // Verify it's the think_routing scenario
        if let RoutingScenario::Pattern(concept) = &decision.scenario {
            assert_eq!(
                concept, "think_routing",
                "Expected think_routing scenario for query: '{}', got: {}",
                query, concept
            );
        } else {
            panic!(
                "Expected Pattern(think_routing) scenario for query: '{}', got: {:?}",
                query, decision.scenario
            );
        }

        // Verify the keywords would be detected (check if any keyword is in the query)
        let keyword_found = keywords.iter().any(|&keyword| query.contains(keyword));
        assert!(
            keyword_found,
            "Test case should contain the keyword it's testing: '{}', keywords: {:?}",
            query, keywords
        );
    }
}

#[tokio::test]
async fn test_intelligent_routing_content_based_detection() {
    let (_temp_dir, taxonomy_dir) = create_intelligent_routing_taxonomy();
    let mut rolegraph = RoleGraphClient::new(&taxonomy_dir).unwrap();
    rolegraph.load_taxonomy().unwrap();

    let config = Arc::new(create_test_config_with_intelligent_routing());
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
        create_router_with_intelligent_routing(config.clone(), rolegraph, session_manager.clone());

    // Test that content-based detection works even without explicit "thinking" field
    let request = ChatRequest {
        model: "auto".to_string(),
        messages: vec![Message {
            role: "user".to_string(),
            content: MessageContent::Text("I need to think through this architecture design and plan the implementation step by step".to_string()),
            ..Default::default()
        }],
        system: None,
        max_tokens: Some(200),
        temperature: Some(0.5),
        stream: None,
        tools: None,
        thinking: None, // Explicitly no thinking field - test content-based detection
        ..Default::default()
    };

    let hints = analyzer.analyze(&request).unwrap();

    // Verify hints don't have explicit thinking field
    assert!(
        !hints.has_thinking,
        "Should not have explicit thinking field"
    );

    let decision = router.route_with_fallback(&request, &hints).await.unwrap();

    // Should still route to openrouter with deepseek model based on content keywords
    assert_eq!(decision.provider.name, "openrouter");
    assert_eq!(decision.model, "deepseek/deepseek-v3.1-terminus");

    if let RoutingScenario::Pattern(concept) = &decision.scenario {
        assert_eq!(concept, "think_routing");
    } else {
        panic!("Expected Pattern(think_routing) scenario");
    }
}

#[tokio::test]
async fn test_intelligent_routing_vs_default_routing() {
    let (_temp_dir, taxonomy_dir) = create_intelligent_routing_taxonomy();
    let mut rolegraph = RoleGraphClient::new(&taxonomy_dir).unwrap();
    rolegraph.load_taxonomy().unwrap();

    let config = Arc::new(create_test_config_with_intelligent_routing());
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
        create_router_with_intelligent_routing(config.clone(), rolegraph, session_manager.clone());

    // Test cases that should NOT trigger think routing
    let default_routing_test_cases = vec![
        "What is the weather today?",
        "Tell me a simple fact",
        "Basic question about history",
        "General knowledge query",
        "Simple request for information",
        "Can you help me with something easy?",
        "I have a quick question",
        "Just need a simple answer",
    ];

    for query in default_routing_test_cases {
        let request = ChatRequest {
            model: "auto".to_string(),
            messages: vec![Message {
                role: "user".to_string(),
                content: MessageContent::Text(query.to_string()),
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

        // Should route to default provider (openrouter) for non-thinking queries
        assert_eq!(
            decision.provider.name, "openrouter",
            "Non-thinking query should route to default: '{}'",
            query
        );
        assert!(
            decision.model.contains("claude"),
            "Should use Claude model for default query: '{}'",
            query
        );

        // Should NOT be think_routing scenario
        match &decision.scenario {
            RoutingScenario::Pattern(concept) => {
                assert_ne!(
                    concept, "think_routing",
                    "Non-thinking query should not trigger think_routing: '{}'",
                    query
                );
            }
            RoutingScenario::Default => {
                // Default scenario is acceptable
            }
            _ => {
                // Other scenarios might be acceptable depending on implementation
            }
        }
    }
}

#[tokio::test]
async fn test_intelligent_routing_pattern_matching_score() {
    let (_temp_dir, taxonomy_dir) = create_intelligent_routing_taxonomy();
    let mut rolegraph = RoleGraphClient::new(&taxonomy_dir).unwrap();
    rolegraph.load_taxonomy().unwrap();

    // Test that think_routing patterns get reasonable scores
    let think_queries = vec![
        "I need to think through this problem",
        "Let me plan this out carefully",
        "We should analyze this step by step",
        "Can you reason through this solution?",
    ];

    for query in think_queries {
        println!("Testing query: '{}'", query);
        match rolegraph.query_routing(query) {
            Some(pattern_match) => {
                println!(
                    "  ✓ Matched: concept={}, provider={}, model={}, score={}",
                    pattern_match.concept,
                    pattern_match.provider,
                    pattern_match.model,
                    pattern_match.score
                );
                assert_eq!(pattern_match.concept, "think_routing");
                assert_eq!(pattern_match.provider, "openrouter");
                assert_eq!(pattern_match.model, "deepseek/deepseek-v3.1-terminus");
                assert!(
                    pattern_match.score > 0.0,
                    "Score should be positive for think query: '{}', got: {}",
                    query,
                    pattern_match.score
                );
                assert!(
                    pattern_match.priority.value() > 0,
                    "Priority should be positive for think query: '{}', got: {}",
                    query,
                    pattern_match.priority.value()
                );
            }
            None => {
                println!("  ✗ No match found");
                panic!(
                    "Think query should match think_routing pattern: '{}'",
                    query
                );
            }
        }
    }
}

#[tokio::test]
async fn test_intelligent_routing_multiple_keywords() {
    let (_temp_dir, taxonomy_dir) = create_intelligent_routing_taxonomy();
    let mut rolegraph = RoleGraphClient::new(&taxonomy_dir).unwrap();
    rolegraph.load_taxonomy().unwrap();

    let config = Arc::new(create_test_config_with_intelligent_routing());
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
        create_router_with_intelligent_routing(config.clone(), rolegraph, session_manager.clone());

    // Test queries with multiple think keywords
    let multi_keyword_queries = vec![
        "I need to think through this step by step and plan the implementation carefully",
        "Let's analyze this problem with systematic thinking and logical reasoning",
        "Use critical thinking to break down this complex problem solving task",
        "Work through the design thinking process and reason through each decision",
    ];

    for query in multi_keyword_queries {
        let request = ChatRequest {
            model: "auto".to_string(),
            messages: vec![Message {
                role: "user".to_string(),
                content: MessageContent::Text(query.to_string()),
                ..Default::default()
            }],
            system: None,
            max_tokens: Some(150),
            temperature: Some(0.6),
            stream: None,
            tools: None,
            thinking: None,
            ..Default::default()
        };

        let hints = analyzer.analyze(&request).unwrap();
        let decision = router.route_with_fallback(&request, &hints).await.unwrap();

        // Should still route to openrouter with deepseek model
        assert_eq!(
            decision.provider.name, "openrouter",
            "Multi-keyword query should route to openrouter: '{}'",
            query
        );
        assert_eq!(
            decision.model, "deepseek/deepseek-v3.1-terminus",
            "Multi-keyword query should use deepseek-v3.1-terminus model: '{}'",
            query
        );

        if let RoutingScenario::Pattern(concept) = &decision.scenario {
            assert_eq!(
                concept, "think_routing",
                "Multi-keyword query should be think_routing: '{}'",
                query
            );
        } else {
            panic!(
                "Expected Pattern(think_routing) for multi-keyword query: '{}'",
                query
            );
        }
    }
}

#[test]
fn test_rolegraph_think_routing_pattern_loading() {
    let (_temp_dir, taxonomy_dir) = create_intelligent_routing_taxonomy();
    let mut rolegraph = RoleGraphClient::new(&taxonomy_dir).unwrap();

    // Should successfully load the taxonomy
    let result = rolegraph.load_taxonomy();
    assert!(
        result.is_ok(),
        "Failed to load taxonomy with think_routing patterns"
    );

    // Should have loaded patterns
    let pattern_count = rolegraph.pattern_count();
    assert!(
        pattern_count > 0,
        "Should have loaded think_routing patterns"
    );

    // Test direct pattern matching
    let test_result = rolegraph.query_routing("I need to think through this");
    assert!(test_result.is_some(), "Should match think_routing pattern");

    let pattern_match = test_result.unwrap();
    assert_eq!(pattern_match.concept, "think_routing");
    assert_eq!(pattern_match.provider, "openrouter");
    assert_eq!(pattern_match.model, "deepseek/deepseek-v3.1-terminus");
}
