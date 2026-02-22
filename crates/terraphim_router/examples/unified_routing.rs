//! Example: Unified routing for LLMs and Agents
//!
//! This example demonstrates how to use terraphim_router to route tasks
//! to either LLM providers or spawned agents based on capability matching.

use std::path::PathBuf;
use terraphim_router::{Router, RoutingContext};
use terraphim_spawner::AgentSpawner;
use terraphim_types::capability::{Capability, CostLevel, Latency, Provider, ProviderType};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create a router
    let mut router = Router::new();

    // Register an LLM provider for deep thinking
    let llm_provider = Provider::new(
        "claude-opus",
        "Claude Opus",
        ProviderType::Llm {
            model_id: "claude-3-opus-20240229".to_string(),
            api_endpoint: "https://api.anthropic.com/v1".to_string(),
        },
        vec![Capability::DeepThinking, Capability::CodeGeneration],
    )
    .with_cost(CostLevel::Expensive)
    .with_latency(Latency::Slow)
    .with_keywords(vec!["think".to_string(), "reason".to_string()]);

    router.add_provider(llm_provider);

    // Register an agent provider for fast coding
    let agent_provider = Provider::new(
        "@codex",
        "Codex Agent",
        ProviderType::Agent {
            agent_id: "@codex".to_string(),
            cli_command: "opencode".to_string(),
            working_dir: PathBuf::from("/workspace"),
        },
        vec![Capability::CodeGeneration, Capability::CodeReview],
    )
    .with_cost(CostLevel::Cheap)
    .with_latency(Latency::Fast)
    .with_keywords(vec!["implement".to_string(), "code".to_string()]);

    router.add_provider(agent_provider);

    // Create an agent spawner
    let spawner = AgentSpawner::new()
        .with_working_dir("/workspace")
        .with_auto_restart(true);

    // Example 1: Route a thinking task (should go to LLM)
    println!("=== Example 1: Deep Thinking Task ===");
    let task1 = "Think deeply about the architecture of a distributed system";
    let decision1 = router.route(task1, &RoutingContext::default())?;

    println!("Task: {}", task1);
    println!(
        "Routed to: {} ({})",
        decision1.provider.name, decision1.provider.id
    );
    println!("Matched capabilities: {:?}", decision1.matched_capabilities);
    println!("Confidence: {:.2}", decision1.confidence);

    match &decision1.provider.provider_type {
        ProviderType::Llm { model_id, .. } => {
            println!("Action: Call LLM API with model {}", model_id);
        }
        ProviderType::Agent { .. } => {
            println!("Action: Spawn agent process");
        }
    }

    // Example 2: Route a coding task (could go to either)
    println!("\n=== Example 2: Coding Task ===");
    let task2 = "Implement a function to parse JSON in Rust";
    let decision2 = router.route(task2, &RoutingContext::default())?;

    println!("Task: {}", task2);
    println!(
        "Routed to: {} ({})",
        decision2.provider.name, decision2.provider.id
    );
    println!("Matched capabilities: {:?}", decision2.matched_capabilities);

    match &decision2.provider.provider_type {
        ProviderType::Llm { model_id, .. } => {
            println!("Action: Call LLM API with model {}", model_id);
        }
        ProviderType::Agent { cli_command, .. } => {
            println!("Action: Spawn agent using CLI: {}", cli_command);
            // In a real scenario, you would spawn the agent:
            // let handle = spawner.spawn(&decision2.provider, task2).await?;
            // println!("Agent spawned with PID: {}", handle.process_id());
        }
    }

    // Example 3: Route with cost optimization strategy
    println!("\n=== Example 3: Cost-Optimized Routing ===");
    use terraphim_router::strategy::CostOptimized;

    let mut router_cost = Router::new();
    router_cost.add_provider(
        Provider::new(
            "claude-opus",
            "Claude Opus",
            ProviderType::Llm {
                model_id: "claude-3-opus-20240229".to_string(),
                api_endpoint: "https://api.anthropic.com/v1".to_string(),
            },
            vec![Capability::DeepThinking, Capability::CodeGeneration],
        )
        .with_cost(CostLevel::Expensive),
    );
    router_cost.add_provider(
        Provider::new(
            "@codex",
            "Codex Agent",
            ProviderType::Agent {
                agent_id: "@codex".to_string(),
                cli_command: "opencode".to_string(),
                working_dir: PathBuf::from("/workspace"),
            },
            vec![Capability::CodeGeneration],
        )
        .with_cost(CostLevel::Cheap),
    );
    let router_cost = router_cost.with_strategy(Box::new(CostOptimized::default()));

    let decision3 = router_cost.route(task2, &RoutingContext::default())?;
    println!(
        "Cost-optimized route: {} (cost: {:?})",
        decision3.provider.name, decision3.provider.cost_level
    );

    println!("\n=== All Examples Complete ===");
    Ok(())
}
