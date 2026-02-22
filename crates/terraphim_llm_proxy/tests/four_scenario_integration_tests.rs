//! Integration Tests for 4 Inference Scenarios
//!
//! This test suite validates all 4 routing scenarios:
//! 1. Fast & Expensive - High throughput, premium models
//! 2. Intelligent - Keyword-based routing with think/plan detection
//! 3. Balanced - Cost/performance balance
//! 4. Slow & Cheap - Background tasks, cost-optimized

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
    router::RouterAgent,
    routing::RoutingStrategy,
    session::{SessionConfig, SessionManager},
    token_counter::{ChatRequest, Message, MessageContent, TokenCounter},
    webhooks::WebhookSettings,
};

fn create_four_scenario_taxonomy() -> (TempDir, std::path::PathBuf) {
    let temp_dir = TempDir::new().unwrap();
    let taxonomy_dir = temp_dir.path().to_path_buf();
    let scenarios_dir = taxonomy_dir.join("routing_scenarios");
    fs::create_dir_all(&scenarios_dir).unwrap();

    // Fast & Expensive routing scenario
    let fast_expensive_routing = scenarios_dir.join("fast_expensive_routing.md");
    fs::write(
        &fast_expensive_routing,
        r#"# Fast & Expensive Routing

Fast & Expensive routing is used for high-throughput, premium model requests where speed and quality are prioritized over cost. This scenario automatically routes to the highest performing models available.

route:: openrouter, anthropic/claude-sonnet-4.5

synonyms:: premium, fast, expensive, high performance, top tier, best quality, speed, urgent, critical, production, realtime, low latency, maximum quality, fastest, premium tier, enterprise, professional
"#,
    )
    .unwrap();

    // Intelligent routing scenario (already working)
    let intelligent_routing = scenarios_dir.join("intelligent_routing.md");
    fs::write(
        &intelligent_routing,
        r#"# Intelligent Routing

Intelligent routing is used for complex problem-solving tasks that require deep reasoning, step-by-step analysis, or extended chain-of-thought processing. This routing scenario activates when keywords indicate reasoning requirements.

route:: openrouter, deepseek/deepseek-v3.1-terminus

synonyms:: think, plan, reason, analyze, break, down, step, by, through, consider, carefully, work, design, systematic, logical, critical, problem, solving, strategic, planning, chain, thought, deep, routing, mode, model, step by step, think through, reason through, consider carefully, work through, design thinking, systematic thinking, logical reasoning, critical thinking, problem solving, strategic planning, chain-of-thought, deep reasoning, thinking routing, reasoning routing, reasoning mode, plan mode, think model
"#,
    )
    .unwrap();

    // Balanced routing scenario
    let balanced_routing = scenarios_dir.join("balanced_routing.md");
    fs::write(
        &balanced_routing,
        r#"# Balanced Routing

Balanced routing provides an optimal mix of cost and performance for general-purpose tasks. This scenario is ideal for everyday use where neither maximum speed nor minimum cost is critical.

route:: openrouter, anthropic/claude-haiku-3.5

synonyms:: balanced, standard, general, everyday, normal, moderate, typical, common, regular, default, standard tier, mid-range, cost-effective, reasonable, practical, efficient, good value, mainstream
"#,
    )
    .unwrap();

    // Slow & Cheap routing scenario
    let slow_cheap_routing = scenarios_dir.join("slow_cheap_routing.md");
    fs::write(
        &slow_cheap_routing,
        r#"# Slow & Cheap Routing

Slow & Cheap routing is used for background tasks, batch processing, and cost-optimized operations where speed is not critical. This scenario prioritizes cost efficiency over response time.

route:: openrouter, google/gemini-2.5-flash-preview-09-2025

synonyms:: cheap, slow, background, batch, cost-optimized, economy, budget, low-cost, affordable, inexpensive, offline, async, deferred, bulk, processing, heavy, intensive, resource-intensive, non-urgent, low priority
"#,
    )
    .unwrap();

    (temp_dir, taxonomy_dir)
}

fn create_four_scenario_test_config() -> ProxyConfig {
    ProxyConfig {
        proxy: ProxySettings {
            host: "127.0.0.1".to_string(),
            port: 3456,
            api_key: "test_api_key".to_string(),
            timeout_ms: 60000,
        },
        router: RouterSettings {
            default: "openrouter,anthropic/claude-sonnet-4.5".to_string(), // Fast & Expensive
            background: Some("openrouter,anthropic/claude-haiku-3.5".to_string()), // Balanced
            think: Some("openrouter,deepseek/deepseek-v3.1-terminus".to_string()), // Intelligent
            plan_implementation: None,
            long_context: Some("openrouter,google/gemini-2.5-flash-preview-09-2025".to_string()), // Slow & Cheap
            long_context_threshold: 60000,
            web_search: Some("openrouter,perplexity/llama-3.1-sonar-large-128k-online".to_string()),
            image: Some("openrouter,anthropic/claude-sonnet-4.5".to_string()),
            model_mappings: vec![],
            model_exclusions: vec![],
            strategy: RoutingStrategy::default(),
        },
        providers: vec![Provider {
            name: "openrouter".to_string(),
            api_base_url: "https://openrouter.ai/api/v1".to_string(),
            api_key: "test_key".to_string(),
            models: vec![
                "anthropic/claude-sonnet-4.5".to_string(),
                "anthropic/claude-haiku-3.5".to_string(),
                "deepseek/deepseek-v3.1-terminus".to_string(),
                "google/gemini-2.5-flash-preview-09-2025".to_string(),
            ],
            transformers: vec![],
        }],
        security: SecuritySettings::default(),
        oauth: OAuthSettings::default(),
        management: ManagementSettings::default(),
        webhooks: WebhookSettings::default(),
    }
}

fn create_four_scenario_router(
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
async fn test_fast_expensive_routing() {
    let (_temp_dir, taxonomy_dir) = create_four_scenario_taxonomy();
    let mut rolegraph = RoleGraphClient::new(&taxonomy_dir).unwrap();
    rolegraph.load_taxonomy().unwrap();

    let config = Arc::new(create_four_scenario_test_config());
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

    let router = create_four_scenario_router(config.clone(), rolegraph, session_manager.clone());

    // Test cases for fast & expensive routing
    let fast_expensive_test_cases = vec![
        (
            "I need a premium, urgent response for this critical production issue",
            vec!["premium", "urgent", "critical"],
        ),
        (
            "This is a high priority request that needs the fastest possible response",
            vec!["high priority", "fastest"],
        ),
        (
            "Enterprise-grade solution required with maximum quality and low latency",
            vec!["enterprise", "maximum quality", "low latency"],
        ),
        (
            "Professional, top-tier performance needed for this realtime application",
            vec!["professional", "top-tier", "realtime"],
        ),
    ];

    for (query, keywords) in fast_expensive_test_cases {
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

        // Should route to fast & expensive (Claude Sonnet 4.5)
        assert_eq!(
            decision.provider.name, "openrouter",
            "Failed to route fast query to openrouter: '{}'",
            query
        );
        assert_eq!(
            decision.model, "anthropic/claude-sonnet-4.5",
            "Failed to select fast model for query: '{}'",
            query
        );

        // Verify the keywords would be detected
        let keyword_found = keywords.iter().any(|&keyword| query.contains(keyword));
        assert!(
            keyword_found,
            "Test case should contain the keyword it's testing: '{}', keywords: {:?}",
            query, keywords
        );
    }
}

#[tokio::test]
async fn test_intelligent_routing_scenarios() {
    let (_temp_dir, taxonomy_dir) = create_four_scenario_taxonomy();
    let mut rolegraph = RoleGraphClient::new(&taxonomy_dir).unwrap();
    rolegraph.load_taxonomy().unwrap();

    let config = Arc::new(create_four_scenario_test_config());
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

    let router = create_four_scenario_router(config.clone(), rolegraph, session_manager.clone());

    // Test cases for intelligent routing
    let intelligent_test_cases = vec![
        (
            "I need to think through this complex problem step by step",
            vec!["think", "step by step"],
        ),
        (
            "Let me plan this architecture design carefully and reason through the implications",
            vec!["plan", "reason"],
        ),
        (
            "Analyze this system design and break down the components systematically",
            vec!["analyze", "break down", "systematically"],
        ),
        (
            "Work through this strategic planning exercise with critical thinking",
            vec!["work through", "strategic planning", "critical thinking"],
        ),
    ];

    for (query, keywords) in intelligent_test_cases {
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

        // Should route to intelligent (DeepSeek Reasoner)
        assert_eq!(
            decision.provider.name, "openrouter",
            "Failed to route intelligent query to openrouter: '{}'",
            query
        );
        assert_eq!(
            decision.model, "deepseek/deepseek-v3.1-terminus",
            "Failed to select intelligent model for query: '{}'",
            query
        );

        // Verify the keywords would be detected
        let keyword_found = keywords.iter().any(|&keyword| query.contains(keyword));
        assert!(
            keyword_found,
            "Test case should contain the keyword it's testing: '{}', keywords: {:?}",
            query, keywords
        );
    }
}

#[tokio::test]
async fn test_balanced_routing() {
    let (_temp_dir, taxonomy_dir) = create_four_scenario_taxonomy();
    let mut rolegraph = RoleGraphClient::new(&taxonomy_dir).unwrap();
    rolegraph.load_taxonomy().unwrap();

    let config = Arc::new(create_four_scenario_test_config());
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

    let router = create_four_scenario_router(config.clone(), rolegraph, session_manager.clone());

    // Test cases for balanced routing
    let balanced_test_cases: Vec<(&str, Vec<&str>)> = vec![
        (
            "I need a standard, everyday response for this typical question",
            vec!["standard", "everyday", "typical"],
        ),
        (
            "This is a general purpose request that requires a balanced approach",
            vec!["general", "balanced"],
        ),
        ("What's the weather like today?", vec![]),
        ("Tell me a joke", vec![]),
        ("Help me write a simple email", vec![]),
        (
            "Provide a cost-effective solution for this common problem",
            vec!["cost-effective", "common"],
        ),
    ];

    for (query, keywords) in balanced_test_cases {
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

        // Should route to balanced (Claude Haiku 3.5) or default (if no keywords)
        assert_eq!(
            decision.provider.name, "openrouter",
            "Failed to route balanced query to openrouter: '{}'",
            query
        );

        // Check if it's balanced or default routing
        let expected_models = [
            "anthropic/claude-haiku-3.5",  // balanced
            "anthropic/claude-sonnet-4.5", // default fallback
        ];
        assert!(
            expected_models.contains(&decision.model.as_str()),
            "Expected balanced or default model, got: '{}' for query: '{}'",
            decision.model,
            query
        );

        // Verify the keywords would be detected (if any)
        if !keywords.is_empty() {
            let keyword_found = keywords.iter().any(|&keyword| query.contains(keyword));
            assert!(
                keyword_found,
                "Test case should contain the keyword it's testing: '{}', keywords: {:?}",
                query, keywords
            );
        }
    }
}

#[tokio::test]
async fn test_slow_cheap_routing() {
    let (_temp_dir, taxonomy_dir) = create_four_scenario_taxonomy();
    let mut rolegraph = RoleGraphClient::new(&taxonomy_dir).unwrap();
    rolegraph.load_taxonomy().unwrap();

    let config = Arc::new(create_four_scenario_test_config());
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

    let router = create_four_scenario_router(config.clone(), rolegraph, session_manager.clone());

    // Test cases for slow & cheap routing
    let slow_cheap_test_cases = vec![
        (
            "I need a cheap, background processing solution for this batch job",
            vec!["cheap", "background", "batch"],
        ),
        (
            "Run this cost-optimized analysis when resources are available",
            vec!["cost-optimized"],
        ),
        (
            "Process this economy-grade data during off-peak hours",
            vec!["economy"],
        ),
        (
            "This is a low priority, budget-conscious task that can be deferred",
            vec!["low priority", "budget"],
        ),
    ];

    for (query, keywords) in slow_cheap_test_cases {
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

        // Should route to slow & cheap (Gemini Flash)
        assert_eq!(
            decision.provider.name, "openrouter",
            "Failed to route slow query to openrouter: '{}'",
            query
        );
        assert_eq!(
            decision.model, "google/gemini-2.5-flash-preview-09-2025",
            "Failed to select slow model for query: '{}'",
            query
        );

        // Verify the keywords would be detected
        let keyword_found = keywords.iter().any(|&keyword| query.contains(keyword));
        assert!(
            keyword_found,
            "Test case should contain the keyword it's testing: '{}', keywords: {:?}",
            query, keywords
        );
    }
}

#[tokio::test]
async fn test_four_scenario_coverage() {
    let (_temp_dir, taxonomy_dir) = create_four_scenario_taxonomy();
    let mut rolegraph = RoleGraphClient::new(&taxonomy_dir).unwrap();
    rolegraph.load_taxonomy().unwrap();

    // Test that all four scenarios are loaded in the RoleGraph
    let pattern_count = rolegraph.pattern_count();
    assert!(
        pattern_count >= 4,
        "Should have loaded at least 4 scenario patterns"
    );

    // Test direct pattern matching for each scenario
    let scenario_tests = vec![
        ("premium urgent service", "fast_&_expensive_routing"),
        ("think through this problem", "intelligent_routing"),
        ("standard everyday task", "balanced_routing"),
        ("cheap background processing", "slow_&_cheap_routing"),
    ];

    for (query, expected_concept) in scenario_tests {
        match rolegraph.query_routing(query) {
            Some(pattern_match) => {
                assert_eq!(
                    pattern_match.concept, expected_concept,
                    "Expected concept '{}' for query '{}', got: '{}'",
                    expected_concept, query, pattern_match.concept
                );
                assert!(
                    pattern_match.score > 0.0,
                    "Score should be positive for query '{}', got: {}",
                    query,
                    pattern_match.score
                );
            }
            None => {
                panic!("Query should match pattern: '{}'", query);
            }
        }
    }
}

#[test]
fn test_rolegraph_four_scenario_loading() {
    let (_temp_dir, taxonomy_dir) = create_four_scenario_taxonomy();
    let mut rolegraph = RoleGraphClient::new(&taxonomy_dir).unwrap();

    // Should successfully load the taxonomy with all four scenarios
    let result = rolegraph.load_taxonomy();
    assert!(
        result.is_ok(),
        "Failed to load taxonomy with four scenario patterns"
    );

    // Should have loaded patterns for all scenarios
    let pattern_count = rolegraph.pattern_count();
    assert!(
        pattern_count >= 4,
        "Should have loaded at least 4 scenario patterns, got: {}",
        pattern_count
    );

    // Test that each scenario pattern can be matched
    let test_queries = vec![
        ("premium urgent service", "fast_&_expensive_routing"),
        ("think through this", "intelligent_routing"),
        ("standard everyday", "balanced_routing"),
        ("cheap background", "slow_&_cheap_routing"),
    ];

    for (query, expected_concept) in test_queries {
        let test_result = rolegraph.query_routing(query);
        assert!(
            test_result.is_some(),
            "Should match {} pattern for query: '{}'",
            expected_concept,
            query
        );

        let pattern_match = test_result.unwrap();
        assert_eq!(pattern_match.concept, expected_concept);
        assert_eq!(pattern_match.provider, "openrouter");
    }
}
