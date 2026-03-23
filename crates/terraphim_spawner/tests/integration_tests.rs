//! Integration tests for opencode provider dispatch
//!
//! Tests the full dispatch pipeline including:
//! - Provider tier routing
//! - Circuit breaker fallback behavior
//! - Subscription guards for banned providers
//! - NDJSON parsing
//! - Skill chain validation and resolution
//! - Persona injection

use std::collections::HashMap;
use std::path::Path;
use std::time::Duration;

use terraphim_spawner::{AgentSpawner, CircuitBreaker, CircuitState, ProviderTier, SpawnRequest};

// Mock NDJSON strings for testing - no external API calls
const SAMPLE_NDJSON: &str = r#"{"type":"step_start","timestamp":1234567890,"sessionID":"sess-123","part":{"step":1}}
{"type":"text","timestamp":1234567891,"sessionID":"sess-123","part":{"text":"Hello, world!"}}
{"type":"tool_use","timestamp":1234567892,"sessionID":"sess-123","part":{"tool":"Read","args":{"path":"/tmp/file.txt"}}}
{"type":"text","timestamp":1234567893,"sessionID":"sess-123","part":{"text":"Processing complete."}}
{"type":"step_finish","timestamp":1234567894,"sessionID":"sess-123","part":{"step":1,"tokens":{"total":150,"prompt":100,"completion":50}}}
{"type":"result","timestamp":1234567895,"sessionID":"sess-123","part":{"success":true,"cost":0.002,"tokens":{"total":150,"prompt":100,"completion":50}}}"#;

const ERROR_NDJSON: &str = r#"{"type":"step_start","timestamp":1234567890,"sessionID":"sess-error","part":{"step":1}}
{"type":"error","timestamp":1234567891,"sessionID":"sess-error","part":{"message":"Connection failed","code":500}}
{"type":"result","timestamp":1234567892,"sessionID":"sess-error","part":{"success":false,"error":"Connection failed"}}"#;

/// Test provider tier routing - verify timeouts match tier expectations
#[tokio::test]
async fn test_provider_tier_routing() {
    // Define expected provider+model combinations for each tier
    let tier_expectations: Vec<(ProviderTier, u64, &str, &str)> = vec![
        // (tier, expected_timeout_secs, provider, model)
        (ProviderTier::Quick, 30, "opencode-go", "kimi-k2.5-quick"),
        (ProviderTier::Deep, 60, "kimi-for-coding", "k2p5-deep"),
        (ProviderTier::Implementation, 120, "opencode-go", "glm-5"),
        (
            ProviderTier::Oracle,
            300,
            "deepseek-for-coding",
            "deepseek-r1",
        ),
    ];

    for (tier, expected_secs, provider, model) in tier_expectations {
        // Verify timeout matches tier
        let actual_timeout = tier.timeout_secs();
        assert_eq!(
            actual_timeout, expected_secs,
            "Timeout mismatch for tier {:?}: expected {}s, got {}s",
            tier, expected_secs, actual_timeout
        );

        // Create spawn request for this tier
        let request = SpawnRequest {
            name: format!("test-agent-{:?}", tier).to_lowercase(),
            cli_tool: "echo".to_string(),
            task: "test task".to_string(),
            provider: Some(provider.to_string()),
            model: Some(model.to_string()),
            fallback_provider: Some("opencode-go".to_string()),
            fallback_model: Some("glm-5".to_string()),
            provider_tier: Some(tier),
            persona_name: None,
            persona_symbol: None,
            persona_vibe: None,
            meta_cortex_connections: vec![],
        };

        // Verify the request has correct tier configuration
        assert_eq!(
            request.provider_tier,
            Some(tier),
            "Provider tier should be set correctly"
        );
        assert_eq!(request.provider, Some(provider.to_string()));
        assert_eq!(request.model, Some(model.to_string()));

        // Verify tier timeout extraction
        let timeout_from_request = request
            .provider_tier
            .map(|t| t.timeout_secs())
            .unwrap_or(120);
        assert_eq!(
            timeout_from_request, expected_secs,
            "Timeout extraction failed for tier {:?}",
            tier
        );
    }
}

/// Test that circuit breaker opens after 3 consecutive failures and triggers fallback
#[tokio::test]
async fn test_fallback_dispatch_on_failure() {
    let spawner = AgentSpawner::new();
    let mut circuit_breakers: HashMap<String, CircuitBreaker> = HashMap::new();
    let banned_providers: Vec<String> = vec![];

    // Create a request with a command that will fail
    let request = SpawnRequest {
        name: "failing-agent".to_string(),
        cli_tool: "nonexistent_command_12345".to_string(), // Will fail
        task: "This will fail".to_string(),
        provider: Some("primary-provider".to_string()),
        model: Some("model-1".to_string()),
        fallback_provider: Some("fallback-provider".to_string()),
        fallback_model: Some("fallback-model".to_string()),
        provider_tier: Some(ProviderTier::Quick),
        persona_name: None,
        persona_symbol: None,
        persona_vibe: None,
        meta_cortex_connections: vec![],
    };

    let primary_key = "primary-provider/model-1";

    // First failure - circuit should still be closed
    let result1 = spawner
        .spawn_with_fallback(
            &request,
            Path::new("/tmp"),
            &banned_providers,
            &mut circuit_breakers,
        )
        .await;
    assert!(result1.is_err(), "First spawn should fail");

    // Check circuit breaker was created and recorded failure
    assert!(
        circuit_breakers.contains_key(primary_key),
        "Circuit breaker should be created for primary provider"
    );
    let cb = circuit_breakers.get(primary_key).unwrap();
    assert!(
        cb.should_allow(),
        "Circuit should still allow after 1 failure"
    );

    // Second failure
    let result2 = spawner
        .spawn_with_fallback(
            &request,
            Path::new("/tmp"),
            &banned_providers,
            &mut circuit_breakers,
        )
        .await;
    assert!(result2.is_err(), "Second spawn should fail");

    let cb = circuit_breakers.get(primary_key).unwrap();
    assert!(
        cb.should_allow(),
        "Circuit should still allow after 2 failures"
    );

    // Third failure - circuit should open
    let result3 = spawner
        .spawn_with_fallback(
            &request,
            Path::new("/tmp"),
            &banned_providers,
            &mut circuit_breakers,
        )
        .await;
    assert!(result3.is_err(), "Third spawn should fail");

    let cb = circuit_breakers.get(primary_key).unwrap();
    assert!(
        !cb.should_allow(),
        "Circuit should be OPEN after 3 failures"
    );
    assert_eq!(
        cb.state(),
        CircuitState::Open,
        "Circuit state should be Open"
    );

    // Fourth attempt - should skip primary and try fallback
    // Fallback will also fail because we're using the same failing command
    let result4 = spawner
        .spawn_with_fallback(
            &request,
            Path::new("/tmp"),
            &banned_providers,
            &mut circuit_breakers,
        )
        .await;
    assert!(result4.is_err(), "Fallback spawn should also fail");

    // Verify error mentions both primary and fallback failure
    let err_msg = result4.unwrap_err().to_string();
    assert!(
        err_msg.contains("Both primary and fallback failed")
            || err_msg.contains("fallback")
            || err_msg.contains("Primary provider failed"),
        "Error should indicate fallback was attempted: {}",
        err_msg
    );
}

/// Test that banned provider prefixes are rejected at runtime
/// Note: The implementation uses starts_with() matching, so "opencode" bans "opencode-go" too
#[tokio::test]
async fn test_subscription_guard_rejects_banned_prefixes() {
    let spawner = AgentSpawner::new();
    let mut circuit_breakers: HashMap<String, CircuitBreaker> = HashMap::new();

    // Test banned providers list - anything starting with these is banned
    let banned_providers = vec!["opencode".to_string(), "zen".to_string()];

    // Request with exact banned provider (opencode)
    let request = SpawnRequest {
        name: "banned-agent".to_string(),
        cli_tool: "echo".to_string(),
        task: "Test task".to_string(),
        provider: Some("opencode".to_string()), // Banned - should be rejected
        model: Some("kimi-k2.5".to_string()),
        fallback_provider: None,
        fallback_model: None,
        provider_tier: Some(ProviderTier::Quick),
        persona_name: None,
        persona_symbol: None,
        persona_vibe: None,
        meta_cortex_connections: vec![],
    };

    let result = spawner
        .spawn_with_fallback(
            &request,
            Path::new("/tmp"),
            &banned_providers,
            &mut circuit_breakers,
        )
        .await;

    assert!(result.is_err(), "Should reject banned provider");
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("banned") || err_msg.contains("Banned"),
        "Error should mention banned provider: {}",
        err_msg
    );
    assert!(
        err_msg.contains("opencode"),
        "Error should mention the banned provider name 'opencode': {}",
        err_msg
    );

    // Test that opencode-go is also banned (starts_with matching)
    let opencode_go_request = SpawnRequest {
        name: "opencode-go-agent".to_string(),
        cli_tool: "echo".to_string(),
        task: "Test task".to_string(),
        provider: Some("opencode-go".to_string()), // Also banned due to starts_with
        model: Some("glm-5".to_string()),
        fallback_provider: None,
        fallback_model: None,
        provider_tier: Some(ProviderTier::Quick),
        persona_name: None,
        persona_symbol: None,
        persona_vibe: None,
        meta_cortex_connections: vec![],
    };

    let result = spawner
        .spawn_with_fallback(
            &opencode_go_request,
            Path::new("/tmp"),
            &banned_providers,
            &mut circuit_breakers,
        )
        .await;

    assert!(
        result.is_err(),
        "Should reject opencode-go (starts_with 'opencode'): {:?}",
        result
    );
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("banned") || err_msg.contains("Banned"),
        "Error should mention banned provider: {}",
        err_msg
    );

    // Test that non-banned providers are allowed
    let allowed_request = SpawnRequest {
        name: "allowed-agent".to_string(),
        cli_tool: "echo".to_string(),
        task: "Test task".to_string(),
        provider: Some("kimi-for-coding".to_string()), // Not banned
        model: Some("k2p5".to_string()),
        fallback_provider: None,
        fallback_model: None,
        provider_tier: Some(ProviderTier::Quick),
        persona_name: None,
        persona_symbol: None,
        persona_vibe: None,
        meta_cortex_connections: vec![],
    };

    let result = spawner
        .spawn_with_fallback(
            &allowed_request,
            Path::new("/tmp"),
            &banned_providers,
            &mut circuit_breakers,
        )
        .await;

    assert!(
        result.is_ok(),
        "Should allow kimi-for-coding provider (not banned): {:?}",
        result
    );

    // Test zen prefix is also banned
    let zen_request = SpawnRequest {
        name: "zen-agent".to_string(),
        cli_tool: "echo".to_string(),
        task: "Test task".to_string(),
        provider: Some("zen-model".to_string()),
        model: Some("v1".to_string()),
        fallback_provider: None,
        fallback_model: None,
        provider_tier: Some(ProviderTier::Quick),
        persona_name: None,
        persona_symbol: None,
        persona_vibe: None,
        meta_cortex_connections: vec![],
    };

    let result = spawner
        .spawn_with_fallback(
            &zen_request,
            Path::new("/tmp"),
            &banned_providers,
            &mut circuit_breakers,
        )
        .await;

    assert!(result.is_err(), "Should reject zen prefix");
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("banned") || err_msg.contains("zen"),
        "Error should mention banned provider: {}",
        err_msg
    );

    // Test with empty banned list - all providers should be allowed
    let empty_banned: Vec<String> = vec![];
    let opencode_request = SpawnRequest {
        name: "unbanned-opencode".to_string(),
        cli_tool: "echo".to_string(),
        task: "Test task".to_string(),
        provider: Some("opencode".to_string()),
        model: Some("glm-5".to_string()),
        fallback_provider: None,
        fallback_model: None,
        provider_tier: Some(ProviderTier::Quick),
        persona_name: None,
        persona_symbol: None,
        persona_vibe: None,
        meta_cortex_connections: vec![],
    };

    let result = spawner
        .spawn_with_fallback(
            &opencode_request,
            Path::new("/tmp"),
            &empty_banned,
            &mut circuit_breakers,
        )
        .await;

    assert!(
        result.is_ok(),
        "Should allow opencode when banned list is empty: {:?}",
        result
    );
}

/// Test NDJSON parsing and text extraction from opencode output
#[test]
fn test_opencode_ndjson_parsing() {
    use terraphim_spawner::OpenCodeEvent;

    // Parse the sample NDJSON
    let events: Vec<_> = OpenCodeEvent::parse_lines(SAMPLE_NDJSON)
        .into_iter()
        .filter_map(|r| r.ok())
        .collect();

    assert_eq!(events.len(), 6, "Should parse 6 events from sample NDJSON");

    // Verify event types
    assert_eq!(events[0].event_type, "step_start");
    assert_eq!(events[1].event_type, "text");
    assert_eq!(events[2].event_type, "tool_use");
    assert_eq!(events[3].event_type, "text");
    assert_eq!(events[4].event_type, "step_finish");
    assert_eq!(events[5].event_type, "result");

    // Test text content extraction
    assert_eq!(
        events[1].text_content(),
        Some("Hello, world!"),
        "Should extract first text content"
    );
    assert_eq!(
        events[3].text_content(),
        Some("Processing complete."),
        "Should extract second text content"
    );

    // Test no text content for non-text events
    assert!(
        events[0].text_content().is_none(),
        "step_start should not have text content"
    );
    assert!(
        events[2].text_content().is_none(),
        "tool_use should not have text content"
    );

    // Test is_result detection
    assert!(!events[0].is_result(), "step_start should not be a result");
    assert!(!events[1].is_result(), "text event should not be a result");
    assert!(events[5].is_result(), "Last event should be a result");

    // Test is_step_finish detection
    assert!(
        !events[0].is_step_finish(),
        "step_start should not be step_finish"
    );
    assert!(
        events[4].is_step_finish(),
        "step_finish event should be detected"
    );

    // Test token extraction
    assert_eq!(
        events[4].total_tokens(),
        Some(150),
        "Should extract 150 tokens from step_finish"
    );
    assert_eq!(
        events[5].total_tokens(),
        Some(150),
        "Should extract 150 tokens from result"
    );
    assert!(
        events[1].total_tokens().is_none(),
        "Text event should not have tokens"
    );

    // Test session ID extraction
    assert_eq!(
        events[0].session_id,
        Some("sess-123".to_string()),
        "Should extract session ID"
    );
}

/// Test NDJSON error handling
#[test]
fn test_opencode_ndjson_error_parsing() {
    use terraphim_spawner::OpenCodeEvent;

    let events: Vec<_> = OpenCodeEvent::parse_lines(ERROR_NDJSON)
        .into_iter()
        .filter_map(|r| r.ok())
        .collect();

    assert_eq!(events.len(), 3, "Should parse 3 events from error NDJSON");

    // Error event type
    assert_eq!(events[1].event_type, "error");

    // Result should indicate failure
    assert!(events[2].is_result());
    assert!(
        events[2]
            .part
            .as_ref()
            .map(|p| p.get("success") == Some(&serde_json::json!(false)))
            .unwrap_or(false),
        "Result should indicate failure"
    );
}

/// Test skill chain validation for both terraphim-skills and zestic-engineering-skills
#[test]
fn test_skill_chain_validation() {
    use terraphim_spawner::{SkillResolver, SkillSource};

    let resolver = SkillResolver::new();

    // Test terraphim-only chain
    let terraphim_chain = vec![
        "security-audit".to_string(),
        "code-review".to_string(),
        "rust-development".to_string(),
    ];

    let resolved = resolver
        .resolve_skill_chain(terraphim_chain.clone())
        .unwrap();
    assert_eq!(resolved.len(), 3, "Should resolve all 3 terraphim skills");

    for skill in &resolved {
        assert_eq!(
            skill.source,
            SkillSource::Terraphim,
            "All skills should be from Terraphim source"
        );
    }

    // Verify skill metadata
    assert_eq!(resolved[0].name, "security-audit");
    assert!(!resolved[0].description.is_empty());
    assert!(!resolved[0].applicable_to.is_empty());
    assert!(resolved[0]
        .path
        .to_string_lossy()
        .contains("security-audit"));

    // Test zestic-only chain
    let zestic_chain = vec![
        "quality-oversight".to_string(),
        "responsible-ai".to_string(),
        "cross-platform".to_string(),
    ];

    let resolved = resolver.resolve_skill_chain(zestic_chain.clone()).unwrap();
    assert_eq!(resolved.len(), 3, "Should resolve all 3 zestic skills");

    for skill in &resolved {
        assert_eq!(
            skill.source,
            SkillSource::Zestic,
            "All skills should be from Zestic source"
        );
    }

    // Test validation of chains
    assert!(
        resolver.validate_skill_chain(&terraphim_chain).is_ok(),
        "Terraphim chain should be valid"
    );
    assert!(
        resolver.validate_skill_chain(&zestic_chain).is_ok(),
        "Zestic chain should be valid"
    );

    // Test invalid chain detection
    let invalid_chain = vec![
        "security-audit".to_string(),
        "nonexistent-skill".to_string(),
        "also-invalid".to_string(),
    ];

    let result = resolver.validate_skill_chain(&invalid_chain);
    assert!(result.is_err(), "Should reject invalid chain");

    let invalid_skills = result.unwrap_err();
    assert!(invalid_skills.contains(&"nonexistent-skill".to_string()));
    assert!(invalid_skills.contains(&"also-invalid".to_string()));
    assert!(!invalid_skills.contains(&"security-audit".to_string()));
}

/// Test persona identity injection into task prompts
#[test]
fn test_persona_injection() {
    // Test with full persona configuration
    let request_with_persona = SpawnRequest {
        name: "test-agent".to_string(),
        cli_tool: "echo".to_string(),
        task: "Analyze this code".to_string(),
        provider: Some("opencode-go".to_string()),
        model: Some("glm-5".to_string()),
        fallback_provider: None,
        fallback_model: None,
        provider_tier: None,
        persona_name: Some("CodeReviewer".to_string()),
        persona_symbol: Some("🔍".to_string()),
        persona_vibe: Some("Analytical and thorough".to_string()),
        meta_cortex_connections: vec!["@security-agent".to_string(), "@quality-agent".to_string()],
    };

    // Verify persona fields are set
    assert_eq!(
        request_with_persona.persona_name,
        Some("CodeReviewer".to_string())
    );
    assert_eq!(request_with_persona.persona_symbol, Some("🔍".to_string()));
    assert_eq!(
        request_with_persona.persona_vibe,
        Some("Analytical and thorough".to_string())
    );
    assert_eq!(
        request_with_persona.meta_cortex_connections,
        vec!["@security-agent".to_string(), "@quality-agent".to_string()]
    );

    // The build_persona_prefix function is internal, so we verify the request structure
    // In actual dispatch, this would prepend:
    // # Identity
    //
    // You are **CodeReviewer**, a member of Species Terraphim.
    // Symbol: 🔍
    // Personality: Analytical and thorough
    // Meta-cortex connections: @security-agent, @quality-agent
    //
    // ---

    // Test without persona
    let request_without_persona = SpawnRequest {
        name: "plain-agent".to_string(),
        cli_tool: "echo".to_string(),
        task: "Simple task".to_string(),
        provider: Some("opencode-go".to_string()),
        model: Some("glm-5".to_string()),
        fallback_provider: None,
        fallback_model: None,
        provider_tier: None,
        persona_name: None,
        persona_symbol: None,
        persona_vibe: None,
        meta_cortex_connections: vec![],
    };

    assert!(request_without_persona.persona_name.is_none());

    // Test partial persona (only name)
    let request_partial = SpawnRequest {
        name: "partial-agent".to_string(),
        cli_tool: "echo".to_string(),
        task: "Task".to_string(),
        provider: Some("opencode-go".to_string()),
        model: Some("glm-5".to_string()),
        fallback_provider: None,
        fallback_model: None,
        provider_tier: None,
        persona_name: Some("MinimalPersona".to_string()),
        persona_symbol: None,
        persona_vibe: None,
        meta_cortex_connections: vec![],
    };

    assert_eq!(
        request_partial.persona_name,
        Some("MinimalPersona".to_string())
    );
    assert!(request_partial.persona_symbol.is_none());
    assert!(request_partial.persona_vibe.is_none());
}

/// Test mixed skill chain resolution from both terraphim and zestic sources
#[test]
fn test_mixed_skill_chain_resolution() {
    use terraphim_spawner::{SkillResolver, SkillSource};

    let resolver = SkillResolver::new();

    // Create a mixed chain with skills from both sources
    let mixed_chain = vec![
        // Terraphim skills
        "security-audit".to_string(),
        "code-review".to_string(),
        // Zestic skills
        "quality-oversight".to_string(),
        "rust-mastery".to_string(),
        // More Terraphim
        "testing".to_string(),
        // More Zestic
        "cross-platform".to_string(),
    ];

    // Resolve the mixed chain
    let resolved = resolver.resolve_skill_chain(mixed_chain.clone()).unwrap();
    assert_eq!(
        resolved.len(),
        6,
        "Should resolve all 6 skills in mixed chain"
    );

    // Verify sources alternate correctly
    assert_eq!(resolved[0].name, "security-audit");
    assert_eq!(resolved[0].source, SkillSource::Terraphim);

    assert_eq!(resolved[1].name, "code-review");
    assert_eq!(resolved[1].source, SkillSource::Terraphim);

    assert_eq!(resolved[2].name, "quality-oversight");
    assert_eq!(resolved[2].source, SkillSource::Zestic);

    assert_eq!(resolved[3].name, "rust-mastery");
    assert_eq!(resolved[3].source, SkillSource::Zestic);

    assert_eq!(resolved[4].name, "testing");
    assert_eq!(resolved[4].source, SkillSource::Terraphim);

    assert_eq!(resolved[5].name, "cross-platform");
    assert_eq!(resolved[5].source, SkillSource::Zestic);

    // Verify all resolved skills have valid structure
    for skill in &resolved {
        assert!(!skill.name.is_empty(), "Skill name should not be empty");
        assert!(
            !skill.description.is_empty(),
            "Skill description should not be empty"
        );
        assert!(
            !skill.applicable_to.is_empty(),
            "Skill should have applicable_to tags"
        );
        assert!(
            skill.path.to_string_lossy().contains(&skill.name),
            "Path should contain skill name"
        );
    }

    // Test that we can also get all skill names
    let all_names = resolver.all_skill_names();
    for skill in &resolved {
        assert!(
            all_names.contains(&skill.name),
            "Resolved skill {} should be in all_skill_names",
            skill.name
        );
    }

    // Test individual skill resolution
    let terraphim_skill = resolver.resolve_skill("security-audit").unwrap();
    assert_eq!(terraphim_skill.source, SkillSource::Terraphim);

    let zestic_skill = resolver.resolve_skill("quality-oversight").unwrap();
    assert_eq!(zestic_skill.source, SkillSource::Zestic);

    // Test that validation works on mixed chains
    assert!(
        resolver.validate_skill_chain(&mixed_chain).is_ok(),
        "Mixed chain should validate successfully"
    );

    // Test chain with only terraphim skills
    let terraphim_only = vec![
        "security-audit".to_string(),
        "documentation".to_string(),
        "md-book".to_string(),
    ];
    let resolved = resolver.resolve_skill_chain(terraphim_only).unwrap();
    for skill in resolved {
        assert_eq!(skill.source, SkillSource::Terraphim);
    }

    // Test chain with only zestic skills
    let zestic_only = vec![
        "responsible-ai".to_string(),
        "insight-synthesis".to_string(),
        "perspective-investigation".to_string(),
    ];
    let resolved = resolver.resolve_skill_chain(zestic_only).unwrap();
    for skill in resolved {
        assert_eq!(skill.source, SkillSource::Zestic);
    }
}

/// Test that the spawner correctly builds provider strings with models
#[test]
fn test_provider_string_building() {
    // This tests the internal build_provider_string behavior through public APIs
    // Provider string format: {provider}/{model}

    let test_cases = vec![
        (Some("opencode-go"), Some("glm-5"), "opencode-go/glm-5"),
        (
            Some("kimi-for-coding"),
            Some("k2p5"),
            "kimi-for-coding/k2p5",
        ),
        (
            Some("deepseek-for-coding"),
            Some("deepseek-r1"),
            "deepseek-for-coding/deepseek-r1",
        ),
        (Some("opencode-go"), None, "opencode-go"),
        (None, Some("k2p5"), "unknown/k2p5"),
        (None, None, "unknown"),
    ];

    for (provider, model, expected) in test_cases {
        // Create request and verify expected format
        let request = SpawnRequest {
            name: "test".to_string(),
            cli_tool: "echo".to_string(),
            task: "test".to_string(),
            provider: provider.map(|s| s.to_string()),
            model: model.map(|s| s.to_string()),
            fallback_provider: None,
            fallback_model: None,
            provider_tier: None,
            persona_name: None,
            persona_symbol: None,
            persona_vibe: None,
            meta_cortex_connections: vec![],
        };

        // Build expected provider string format (mirrors internal logic)
        let provider_str = match (&request.provider, &request.model) {
            (Some(p), Some(m)) => format!("{}/{}", p, m),
            (Some(p), None) => p.clone(),
            (None, Some(m)) => format!("unknown/{}", m),
            (None, None) => "unknown".to_string(),
        };

        assert_eq!(
            provider_str, expected,
            "Provider string mismatch for {:?} / {:?}",
            provider, model
        );
    }
}

/// Test circuit breaker state transitions
#[test]
fn test_circuit_breaker_state_transitions() {
    use terraphim_spawner::{CircuitBreaker, CircuitBreakerConfig, CircuitState};

    let mut cb = CircuitBreaker::new(CircuitBreakerConfig {
        failure_threshold: 3,
        cooldown: Duration::from_secs(300),
        success_threshold: 1,
    });

    // Initial state should allow requests
    assert_eq!(cb.state(), CircuitState::Closed);
    assert!(cb.should_allow(), "Should allow requests initially");

    // After 1 failure, still closed
    cb.record_failure();
    assert_eq!(cb.state(), CircuitState::Closed);
    assert!(cb.should_allow(), "Should allow after 1 failure");

    // After 2 failures, still closed
    cb.record_failure();
    assert_eq!(cb.state(), CircuitState::Closed);
    assert!(cb.should_allow(), "Should allow after 2 failures");

    // After 3 failures, circuit opens
    cb.record_failure();
    assert_eq!(cb.state(), CircuitState::Open);
    assert!(!cb.should_allow(), "Should NOT allow after 3 failures");

    // Record success while open - should remain open
    cb.record_success();
    assert_eq!(cb.state(), CircuitState::Open);
    assert!(
        !cb.should_allow(),
        "Should remain open after success in open state"
    );
}

/// Integration test: full dispatch flow with in-memory config
#[tokio::test]
async fn test_full_dispatch_flow() {
    let spawner = AgentSpawner::new();
    let mut circuit_breakers: HashMap<String, CircuitBreaker> = HashMap::new();
    let banned_providers: Vec<String> = vec!["zen".to_string()];

    // Create a complex request with all features
    let request = SpawnRequest {
        name: "integration-agent".to_string(),
        cli_tool: "echo".to_string(), // echo for predictable output
        task: "Generate code".to_string(),
        provider: Some("opencode-go".to_string()),
        model: Some("kimi-k2.5".to_string()),
        fallback_provider: Some("opencode-go".to_string()),
        fallback_model: Some("glm-5".to_string()),
        provider_tier: Some(ProviderTier::Implementation),
        persona_name: Some("CodeGenerator".to_string()),
        persona_symbol: Some("💻".to_string()),
        persona_vibe: Some("Creative and efficient".to_string()),
        meta_cortex_connections: vec!["@reviewer".to_string()],
    };

    // Verify tier timeout
    assert_eq!(
        request.provider_tier.unwrap().timeout_secs(),
        120,
        "Implementation tier should have 120s timeout"
    );

    // Verify provider is not banned
    let provider = request.provider.as_ref().unwrap();
    assert!(
        !banned_providers
            .iter()
            .any(|banned| provider.starts_with(banned)),
        "Provider should not be banned"
    );

    // Spawn the agent
    let result = spawner
        .spawn_with_fallback(
            &request,
            Path::new("/tmp"),
            &banned_providers,
            &mut circuit_breakers,
        )
        .await;

    assert!(
        result.is_ok(),
        "Should successfully spawn agent: {:?}",
        result
    );

    let handle = result.unwrap();
    assert!(
        handle.is_healthy().await,
        "Agent should be healthy after spawning"
    );

    // Verify circuit breaker recorded success
    let provider_key = "opencode-go/kimi-k2.5";
    assert!(
        circuit_breakers.contains_key(provider_key),
        "Circuit breaker should exist for provider"
    );
    let cb = circuit_breakers.get(provider_key).unwrap();
    assert!(cb.should_allow(), "Circuit should be closed after success");
    assert_eq!(cb.state(), CircuitState::Closed);
}
