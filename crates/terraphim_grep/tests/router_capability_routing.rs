//! L2 router integration test: prove that grep synthesis prompts produce sensible routing
//! decisions when fed to `terraphim_router::Router`.
//!
//! This test does not call any LLM. It builds a small in-memory provider registry, feeds
//! it grep-style synthesis prompts (constructed exactly the way `lib.rs` constructs them
//! for the LLM call), and asserts that the keyword router extracts the expected capability
//! and the cost-optimised strategy picks the correct provider.
//!
//! Why this matters: if grep's synthesis prompt happens to contain no keyword the router
//! recognises, the routing decision becomes accidental. This test pins down the contract
//! between grep prompts and the router so future prompt changes can't silently break it.
#![cfg(feature = "llm")]

use terraphim_grep::{KgConcept, RetrievedChunk, RlmContext};
use terraphim_router::{
    Capability, CostLevel, CostOptimized, Latency, Provider, ProviderType, Router, RoutingContext,
};

/// Build a synthesis prompt the same way `TerraphimGrep::search_with_rlm_fallback` does.
/// Keeping this helper here means the test will compile-fail if the upstream prompt shape
/// changes -- a useful canary.
fn grep_synthesis_prompt(query: &str, include_answer: bool) -> String {
    let chunks = vec![RetrievedChunk {
        content: format!("fn handle_{query}() {{}}"),
        source: "src/lib.rs".to_string(),
        line_start: Some(1),
        line_end: Some(1),
        relevance_score: 1.0,
        haystack: "code",
    }];
    let concepts = vec![KgConcept {
        id: 1,
        name: query.to_string(),
        display_value: None,
        score: 0.9,
    }];
    let body = RlmContext::new(query.to_string())
        .with_chunks(chunks)
        .with_concepts(concepts)
        .build_prompt();

    let tail = if include_answer {
        "Synthesise an answer."
    } else {
        "List the relevant findings."
    };
    format!("{body}\n\n{tail}\n\nProvide a brief answer based on the context above.")
}

fn llm_kind(model: &str) -> ProviderType {
    ProviderType::Llm {
        model_id: model.to_string(),
        api_endpoint: "https://openrouter.ai/api/v1".to_string(),
    }
}

fn fast_provider() -> Provider {
    Provider::new(
        "fast-summariser",
        "Fast Summariser",
        llm_kind("liquid/lfm-2.5-1.2b-instruct:free"),
        vec![Capability::Explanation, Capability::FastThinking],
    )
    .with_cost(CostLevel::Cheap)
    .with_latency(Latency::Fast)
}

fn code_provider() -> Provider {
    Provider::new(
        "code-specialist",
        "Code Specialist",
        llm_kind("qwen/qwen3-coder:free"),
        vec![
            Capability::CodeGeneration,
            Capability::Refactoring,
            Capability::Testing,
        ],
    )
    .with_cost(CostLevel::Moderate)
    .with_latency(Latency::Medium)
}

fn router_with_both_providers() -> Router {
    let mut router = Router::new().with_strategy(Box::new(CostOptimized));
    router.add_provider(fast_provider());
    router.add_provider(code_provider());
    router
}

#[test]
fn explanation_query_routes_to_fast_provider() {
    let prompt = grep_synthesis_prompt("explain how the retry policy works", false);
    let router = router_with_both_providers();

    let decision = router
        .route(&prompt, &RoutingContext::default())
        .expect("router should resolve a provider for an explanation prompt");

    assert!(
        decision.provider.has_capability(&Capability::Explanation),
        "expected provider with Explanation capability, got {} with capabilities {:?}",
        decision.provider.id,
        decision.provider.capabilities,
    );
    assert_eq!(
        decision.provider.id, "fast-summariser",
        "cost-optimised strategy should prefer the Free-tier provider for explanation queries"
    );
}

#[test]
fn implementation_query_routes_to_code_provider() {
    // The grep synthesis tail ("Provide a brief answer based on the context above") happens
    // to contain "answer" and "provide" -- both neutral for routing. The user's query is
    // the load-bearing signal. An "implement" query should trigger CodeGeneration in the
    // keyword router, which only `code-specialist` satisfies.
    let prompt = grep_synthesis_prompt(
        "implement a function that handles retry with exponential backoff",
        true,
    );
    // Use CapabilityFirst rather than CostOptimized here: the cost-optimised strategy
    // would pick the cheaper provider whenever both satisfy *any* extracted capability,
    // which is correct in production but obscures whether grep prompts route to the
    // *right* capability. CapabilityFirst makes the capability extraction the load-bearing
    // signal, which is what we want this test to pin down.
    let mut router =
        terraphim_router::Router::new().with_strategy(Box::new(terraphim_router::CapabilityFirst));
    router.add_provider(fast_provider());
    router.add_provider(code_provider());

    let decision = router
        .route(&prompt, &RoutingContext::default())
        .expect("router should resolve a provider for a code-implementation prompt");

    assert!(
        decision
            .provider
            .has_capability(&Capability::CodeGeneration),
        "expected CodeGeneration provider for an implementation prompt, got {} with caps {:?}",
        decision.provider.id,
        decision.provider.capabilities,
    );
    assert_eq!(decision.provider.id, "code-specialist");
}

#[test]
fn empty_extract_falls_back_without_crashing() {
    // A query with no router keywords. We do not assert which provider wins -- only that
    // the router degrades cleanly. This is the case grep currently lands in when the user's
    // query is a bare identifier ("parse_grep_query" etc.); the test pins down that the
    // pipeline still produces *some* decision rather than panicking.
    let prompt = grep_synthesis_prompt("parse_grep_query", false);
    let router = router_with_both_providers();

    let _ = router
        .route(&prompt, &RoutingContext::default())
        .expect("router must produce a decision even when keyword extraction is empty");
}
