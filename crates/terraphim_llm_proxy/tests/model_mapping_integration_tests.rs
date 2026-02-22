//! Integration tests for configurable model mappings
//!
//! Tests verify that model name mappings work correctly through the request pipeline.

use std::sync::Arc;
use terraphim_llm_proxy::{
    analyzer::RequestAnalyzer,
    config::{
        ManagementSettings, OAuthSettings, Provider, ProxyConfig, ProxySettings, RouterSettings,
        SecuritySettings,
    },
    router::RouterAgent,
    routing::{ModelMapping, RoutingStrategy},
    token_counter::{ChatRequest, Message, MessageContent, TokenCounter},
    webhooks::WebhookSettings,
};

/// Create a test configuration with model mappings
fn create_config_with_mappings() -> ProxyConfig {
    ProxyConfig {
        proxy: ProxySettings {
            host: "127.0.0.1".to_string(),
            port: 3456,
            api_key: "test_api_key".to_string(),
            timeout_ms: 60000,
        },
        router: RouterSettings {
            default: "openrouter,anthropic/claude-3.5-sonnet".to_string(),
            background: None,
            think: None,
            plan_implementation: None,
            long_context: None,
            long_context_threshold: 60000,
            web_search: None,
            image: None,
            model_mappings: vec![
                // Exact match mapping
                ModelMapping::new(
                    "claude-3-5-sonnet-20241022",
                    "openrouter,anthropic/claude-3.5-sonnet",
                ),
                // Glob pattern mapping
                ModelMapping::new("claude-3-opus-*", "openrouter,anthropic/claude-3-opus"),
                // Bidirectional mapping
                ModelMapping::with_bidirectional(
                    "my-fast-model",
                    "openrouter,anthropic/claude-3.5-sonnet",
                ),
            ],
            model_exclusions: vec![],
            strategy: RoutingStrategy::default(),
        },
        providers: vec![Provider {
            name: "openrouter".to_string(),
            api_base_url: "https://openrouter.ai/api/v1".to_string(),
            api_key: "test".to_string(),
            models: vec![
                "anthropic/claude-3.5-sonnet".to_string(),
                "anthropic/claude-3-opus".to_string(),
            ],
            transformers: vec!["openrouter".to_string()],
        }],
        security: SecuritySettings::default(),
        oauth: OAuthSettings::default(),
        management: ManagementSettings::default(),
        webhooks: WebhookSettings::default(),
    }
}

#[test]
fn test_model_mapping_exact_match() {
    let config = create_config_with_mappings();

    // Create a request with the aliased model name
    let request = ChatRequest {
        model: "claude-3-5-sonnet-20241022".to_string(),
        messages: vec![Message {
            role: "user".to_string(),
            content: MessageContent::Text("Hello".to_string()),
            ..Default::default()
        }],
        ..Default::default()
    };

    // Verify the mapping exists and would match
    let (resolved, matched) =
        terraphim_llm_proxy::routing::resolve_model(&request.model, &config.router.model_mappings);

    assert!(matched.is_some(), "Mapping should match");
    assert_eq!(
        resolved, "openrouter,anthropic/claude-3.5-sonnet",
        "Model should resolve to OpenRouter format"
    );
}

#[test]
fn test_model_mapping_glob_pattern() {
    let config = create_config_with_mappings();

    // Test with glob pattern match
    let request = ChatRequest {
        model: "claude-3-opus-20240229".to_string(),
        messages: vec![Message {
            role: "user".to_string(),
            content: MessageContent::Text("Hello".to_string()),
            ..Default::default()
        }],
        ..Default::default()
    };

    let (resolved, matched) =
        terraphim_llm_proxy::routing::resolve_model(&request.model, &config.router.model_mappings);

    assert!(matched.is_some(), "Glob pattern should match");
    assert_eq!(
        resolved, "openrouter,anthropic/claude-3-opus",
        "Model should resolve via glob pattern"
    );
}

#[test]
fn test_model_mapping_no_match_passthrough() {
    let config = create_config_with_mappings();

    // Test with a model that doesn't match any mapping
    let request = ChatRequest {
        model: "gpt-4o".to_string(),
        messages: vec![Message {
            role: "user".to_string(),
            content: MessageContent::Text("Hello".to_string()),
            ..Default::default()
        }],
        ..Default::default()
    };

    let (resolved, matched) =
        terraphim_llm_proxy::routing::resolve_model(&request.model, &config.router.model_mappings);

    assert!(matched.is_none(), "Should not match any mapping");
    assert_eq!(resolved, "gpt-4o", "Model should pass through unchanged");
}

#[test]
fn test_model_mapping_case_insensitive() {
    let config = create_config_with_mappings();

    // Test case-insensitive matching
    let (resolved, matched) = terraphim_llm_proxy::routing::resolve_model(
        "CLAUDE-3-5-SONNET-20241022",
        &config.router.model_mappings,
    );

    assert!(matched.is_some(), "Case-insensitive match should work");
    assert_eq!(resolved, "openrouter,anthropic/claude-3.5-sonnet");
}

#[test]
fn test_model_mapping_bidirectional() {
    let config = create_config_with_mappings();

    // Test bidirectional mapping resolution
    let (resolved, matched) =
        terraphim_llm_proxy::routing::resolve_model("my-fast-model", &config.router.model_mappings);

    assert!(matched.is_some(), "Bidirectional mapping should match");
    assert!(
        matched.unwrap().bidirectional,
        "Mapping should be bidirectional"
    );
    assert_eq!(resolved, "openrouter,anthropic/claude-3.5-sonnet");

    // Test reverse resolution
    let reversed = terraphim_llm_proxy::routing::reverse_resolve(
        "anthropic/claude-3.5-sonnet",
        &config.router.model_mappings,
    );

    assert_eq!(
        reversed,
        Some("my-fast-model"),
        "Reverse resolution should return alias"
    );
}

#[tokio::test]
async fn test_model_mapping_in_routing_pipeline() {
    let config = Arc::new(create_config_with_mappings());
    let router = RouterAgent::new(config.clone());
    let token_counter = Arc::new(TokenCounter::new().unwrap());
    let analyzer = RequestAnalyzer::new(token_counter);

    // Create request with aliased model
    let mut request = ChatRequest {
        model: "claude-3-5-sonnet-20241022".to_string(),
        messages: vec![Message {
            role: "user".to_string(),
            content: MessageContent::Text("Hello, world!".to_string()),
            ..Default::default()
        }],
        ..Default::default()
    };

    // Apply model mappings (simulating what server.rs does)
    let (resolved, matched) =
        terraphim_llm_proxy::routing::resolve_model(&request.model, &config.router.model_mappings);
    if matched.is_some() {
        // Keep the full "provider,model" format for explicit routing
        request.model = resolved;
    }

    // Verify model was transformed to full provider,model format
    assert_eq!(
        request.model, "openrouter,anthropic/claude-3.5-sonnet",
        "Model should be resolved to full provider,model format"
    );

    // Analyze and route
    let hints = analyzer.analyze(&request).unwrap();
    let decision = router.route(&request, &hints).await.unwrap();

    assert_eq!(
        decision.provider.name, "openrouter",
        "Should route to openrouter provider"
    );
}

#[test]
fn test_model_mapping_first_match_wins() {
    // Create config with overlapping mappings
    let config = ProxyConfig {
        proxy: ProxySettings {
            host: "127.0.0.1".to_string(),
            port: 3456,
            api_key: "test".to_string(),
            timeout_ms: 60000,
        },
        router: RouterSettings {
            default: "default,model".to_string(),
            background: None,
            think: None,
            plan_implementation: None,
            long_context: None,
            long_context_threshold: 60000,
            web_search: None,
            image: None,
            model_mappings: vec![
                // More specific mapping first
                ModelMapping::new("claude-3-opus-20240229", "provider1,specific-opus"),
                // Less specific glob pattern second
                ModelMapping::new("claude-3-*", "provider2,generic-claude3"),
            ],
            model_exclusions: vec![],
            strategy: RoutingStrategy::default(),
        },
        providers: vec![],
        security: SecuritySettings::default(),
        oauth: OAuthSettings::default(),
        management: ManagementSettings::default(),
        webhooks: WebhookSettings::default(),
    };

    // Exact match should win over glob
    let (resolved, _) = terraphim_llm_proxy::routing::resolve_model(
        "claude-3-opus-20240229",
        &config.router.model_mappings,
    );
    assert_eq!(
        resolved, "provider1,specific-opus",
        "First exact match should win"
    );

    // Other models should match glob
    let (resolved, _) = terraphim_llm_proxy::routing::resolve_model(
        "claude-3-sonnet",
        &config.router.model_mappings,
    );
    assert_eq!(
        resolved, "provider2,generic-claude3",
        "Glob pattern should match other models"
    );
}

#[test]
fn test_empty_model_mappings() {
    let config = ProxyConfig {
        proxy: ProxySettings {
            host: "127.0.0.1".to_string(),
            port: 3456,
            api_key: "test".to_string(),
            timeout_ms: 60000,
        },
        router: RouterSettings {
            default: "default,model".to_string(),
            background: None,
            think: None,
            plan_implementation: None,
            long_context: None,
            long_context_threshold: 60000,
            web_search: None,
            image: None,
            model_mappings: vec![], // No mappings
            model_exclusions: vec![],
            strategy: RoutingStrategy::default(),
        },
        providers: vec![],
        security: SecuritySettings::default(),
        oauth: OAuthSettings::default(),
        management: ManagementSettings::default(),
        webhooks: WebhookSettings::default(),
    };

    // All models should pass through unchanged
    let (resolved, matched) =
        terraphim_llm_proxy::routing::resolve_model("any-model", &config.router.model_mappings);

    assert!(matched.is_none(), "No mapping should match");
    assert_eq!(resolved, "any-model", "Model should pass through unchanged");
}
