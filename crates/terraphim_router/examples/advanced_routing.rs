//! Example: Advanced routing with fallback, metrics, and knowledge graph
//!
//! This example demonstrates:
//! - Fallback routing when providers fail
//! - Metrics collection
//! - Knowledge graph integration for term expansion

use std::path::PathBuf;
use terraphim_router::{
    FallbackRouter, FallbackStrategy, KnowledgeGraphRouter,
    Router, RouterMetrics, RoutingContext, Timer,
};
use terraphim_types::capability::{
    Capability, CostLevel, Latency, Provider, ProviderType,
};
use terraphim_types::{NormalizedTerm, NormalizedTermValue, RoleName, Thesaurus};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Advanced Unified Routing Example ===\n");

    // Create metrics collector
    let metrics = RouterMetrics::new();

    // Create base router
    let mut router = Router::new();

    // Register providers
    router.add_provider(Provider::new(
        "gpt-4",
        "GPT-4",
        ProviderType::Llm {
            model_id: "gpt-4".to_string(),
            api_endpoint: "https://api.openai.com".to_string(),
        },
        vec![Capability::DeepThinking, Capability::CodeGeneration],
    ).with_cost(CostLevel::Expensive));

    router.add_provider(Provider::new(
        "@codex",
        "Codex Agent",
        ProviderType::Agent {
            agent_id: "@codex".to_string(),
            cli_command: "opencode".to_string(),
            working_dir: PathBuf::from("/workspace"),
        },
        vec![Capability::CodeGeneration],
    ).with_cost(CostLevel::Cheap));

    // Example 1: Fallback routing
    println!("=== Example 1: Fallback Routing ===");
    let fallback_router = FallbackRouter::new(router.clone())
        .with_strategy(FallbackStrategy::NextBestProvider)
        .with_max_fallbacks(3);

    let timer = Timer::start();
    let result = fallback_router
        .route_with_fallback(
            "Implement a function",
            &RoutingContext::default(),
            |provider| async move {
                println!("  Trying provider: {}", provider.id);
                // Simulate failure for first provider
                if provider.id == "gpt-4" {
                    Err("API rate limited".to_string())
                } else {
                    Ok(())
                }
            },
        )
        .await;

    match result {
        Ok(decision) => {
            println!("  ✓ Successfully routed to: {}", decision.provider.id);
            metrics.record_routing_request(&decision.provider,
                timer.elapsed_ms()
            );
        }
        Err(e) => {
            println!("  ✗ Routing failed: {}", e);
            metrics.record_routing_failure("all_providers_failed");
        }
    }

    // Example 2: Knowledge graph term expansion
    println!("\n=== Example 2: Knowledge Graph Integration ===");
    let mut kg_router = KnowledgeGraphRouter::new()
        .with_default_role(RoleName::new("engineer"));

    // Create a thesaurus with synonyms
    let mut thesaurus = Thesaurus::new("programming".to_string());
    let mut term = NormalizedTerm::new(
        1,
        NormalizedTermValue::from("rust"),
    );
    term.synonyms.push(NormalizedTermValue::from("rustlang"));
    term.synonyms.push(NormalizedTermValue::from("rust-lang"));
    thesaurus.insert(
        NormalizedTermValue::from("rust"),
        term,
    );

    kg_router.add_thesaurus(RoleName::new("engineer"), thesaurus);

    let terms = vec![NormalizedTermValue::from("rust")];
    let expanded = kg_router.expand_terms(
        &terms,
        Some(&RoleName::new("engineer")),
    );

    println!("  Original terms: {:?}", terms);
    println!("  Expanded terms: {:?}", expanded);

    // Example 3: LLM fallback when agent fails
    println!("\n=== Example 3: LLM Fallback Strategy ===");
    let llm_fallback_router = FallbackRouter::new(router.clone())
        .with_strategy(FallbackStrategy::LlmFallback);

    let result = llm_fallback_router
        .route_with_fallback(
            "Think deeply about system design",
            &RoutingContext::default(),
            |provider| async move {
                match &provider.provider_type {
                    ProviderType::Agent { .. } => {
                        println!("  Agent {} failed to spawn", provider.id);
                        Err("Spawn failed".to_string())
                    }
                    ProviderType::Llm { model_id, .. } => {
                        println!("  LLM {} succeeded", model_id);
                        Ok(())
                    }
                }
            },
        )
        .await;

    if let Ok(decision) = result {
        println!("  ✓ Fallback to LLM: {}", decision.provider.id);
    }

    // Print metrics summary
    println!("\n=== Metrics Summary ===");
    println!("{}", metrics);

    println!("\n=== All Examples Complete ===");
    Ok(())
}
