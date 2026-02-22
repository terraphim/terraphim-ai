//! Integration tests for unified routing

use std::path::PathBuf;
use terraphim_router::{Router, RoutingContext};
use terraphim_types::capability::{Capability, CostLevel, Provider, ProviderType};

#[test]
fn test_route_to_llm_provider() {
    let mut router = Router::new();

    let llm = Provider::new(
        "gpt-4",
        "GPT-4",
        ProviderType::Llm {
            model_id: "gpt-4".to_string(),
            api_endpoint: "https://api.openai.com".to_string(),
        },
        vec![Capability::DeepThinking],
    );

    router.add_provider(llm);

    let decision = router
        .route(
            "Think deeply about system design",
            &RoutingContext::default(),
        )
        .unwrap();

    assert_eq!(decision.provider.id, "gpt-4");
    assert!(decision
        .matched_capabilities
        .contains(&Capability::DeepThinking));
}

#[test]
fn test_route_to_agent_provider() {
    let mut router = Router::new();

    let agent = Provider::new(
        "@codex",
        "Codex",
        ProviderType::Agent {
            agent_id: "@codex".to_string(),
            cli_command: "opencode".to_string(),
            working_dir: PathBuf::from("/tmp"),
        },
        vec![Capability::CodeGeneration],
    );

    router.add_provider(agent);

    let decision = router
        .route("Implement a function", &RoutingContext::default())
        .unwrap();

    assert_eq!(decision.provider.id, "@codex");
    assert!(matches!(
        decision.provider.provider_type,
        ProviderType::Agent { .. }
    ));
}

#[test]
fn test_capability_based_selection() {
    let mut router = Router::new();

    // Cheap agent for code generation
    let agent = Provider::new(
        "@codex",
        "Codex",
        ProviderType::Agent {
            agent_id: "@codex".to_string(),
            cli_command: "opencode".to_string(),
            working_dir: PathBuf::from("/tmp"),
        },
        vec![Capability::CodeGeneration],
    )
    .with_cost(CostLevel::Cheap);

    // Expensive LLM for deep thinking
    let llm = Provider::new(
        "claude-opus",
        "Claude Opus",
        ProviderType::Llm {
            model_id: "claude-3-opus".to_string(),
            api_endpoint: "https://api.anthropic.com".to_string(),
        },
        vec![Capability::DeepThinking, Capability::CodeGeneration],
    )
    .with_cost(CostLevel::Expensive);

    router.add_provider(agent);
    router.add_provider(llm);

    // Deep thinking task should route to LLM
    let decision = router
        .route(
            "Think deeply about architecture",
            &RoutingContext::default(),
        )
        .unwrap();
    assert_eq!(decision.provider.id, "claude-opus");

    // Code generation could go to either, but agent is cheaper
    let decision = router
        .route("Implement a function", &RoutingContext::default())
        .unwrap();
    // With CostOptimized strategy, should pick cheaper option
    assert_eq!(decision.provider.cost_level, CostLevel::Cheap);
}
