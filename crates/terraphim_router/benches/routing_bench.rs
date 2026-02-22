//! Benchmarks for the routing engine, strategy selection, and keyword extraction.
//!
//! To run all routing benchmarks:
//!
//! ```sh
//! cargo bench --bench routing_bench -p terraphim_router
//! ```
//!
//! To run a single group:
//!
//! ```sh
//! cargo bench --bench routing_bench -- keyword_extraction
//! ```

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};

use terraphim_router::{
    Capability, CapabilityFirst, CostLevel, CostOptimized, KeywordRouter, Latency,
    LatencyOptimized, Provider, ProviderType, RoundRobin, Router, RoutingContext, RoutingEngine,
    RoutingStrategy,
};

/// Create a test provider with the given id, cost, and latency.
fn make_provider(id: &str, cost: CostLevel, latency: Latency) -> Provider {
    Provider::new(
        id,
        format!("Provider {}", id),
        ProviderType::Llm {
            model_id: id.to_string(),
            api_endpoint: "https://api.test.com".to_string(),
        },
        Capability::all(),
    )
    .with_cost(cost)
    .with_latency(latency)
}

/// Generate N providers with rotating cost/latency combinations.
fn generate_providers(n: usize) -> Vec<Provider> {
    let costs = [CostLevel::Cheap, CostLevel::Moderate, CostLevel::Expensive];
    let latencies = [Latency::Fast, Latency::Medium, Latency::Slow];

    (0..n)
        .map(|i| {
            make_provider(
                &format!("provider-{}", i),
                costs[i % costs.len()],
                latencies[i % latencies.len()],
            )
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Keyword extraction benchmarks
// ---------------------------------------------------------------------------

fn bench_keyword_extraction(c: &mut Criterion) {
    let mut group = c.benchmark_group("keyword_extraction");
    let router = KeywordRouter::new();

    let prompts = [
        ("simple_greeting", "Hello, how are you today?"),
        (
            "code_generation",
            "Please implement a function to parse JSON and return a struct",
        ),
        (
            "security_audit",
            "Audit this authentication code for security vulnerabilities and threats",
        ),
        (
            "multi_capability",
            "Think carefully, implement a secure system, write tests, and optimize performance",
        ),
    ];

    for (name, prompt) in &prompts {
        group.bench_with_input(BenchmarkId::new("extract", *name), prompt, |b, &prompt| {
            b.iter(|| router.extract_capabilities(prompt))
        });
    }

    group.finish();
}

// ---------------------------------------------------------------------------
// Strategy selection benchmarks
// ---------------------------------------------------------------------------

fn bench_strategy_selection(c: &mut Criterion) {
    let mut group = c.benchmark_group("strategy_selection");

    let provider_counts: &[usize] = &[5, 10, 50, 100];

    let strategies: Vec<(&str, Box<dyn RoutingStrategy>)> = vec![
        ("cost_optimized", Box::new(CostOptimized)),
        ("latency_optimized", Box::new(LatencyOptimized)),
        ("capability_first", Box::new(CapabilityFirst)),
        ("round_robin", Box::new(RoundRobin::new())),
    ];

    for (strategy_name, strategy) in &strategies {
        for &count in provider_counts {
            let providers = generate_providers(count);
            let candidates: Vec<&Provider> = providers.iter().collect();

            group.bench_with_input(
                BenchmarkId::new(*strategy_name, count),
                &candidates,
                |b, candidates| b.iter(|| strategy.select_provider(candidates.clone())),
            );
        }
    }

    group.finish();
}

// ---------------------------------------------------------------------------
// Full routing benchmarks
// ---------------------------------------------------------------------------

fn bench_full_routing(c: &mut Criterion) {
    let mut group = c.benchmark_group("full_routing");

    let prompts = [
        ("simple", "Hello world"),
        (
            "code_task",
            "Implement a function to parse JSON and build a data structure",
        ),
        (
            "security_task",
            "Audit the authentication module for security vulnerabilities",
        ),
        (
            "complex_task",
            "Think carefully, design the architecture, implement it securely, and write comprehensive tests",
        ),
    ];

    let provider_counts: &[usize] = &[5, 50, 100];

    for &count in provider_counts {
        let providers = generate_providers(count);
        let context = RoutingContext::default();

        for (prompt_name, prompt) in &prompts {
            let mut engine = RoutingEngine::new();
            for p in &providers {
                engine.add_provider(p.clone());
            }
            let router = Router::from_engine(engine);

            group.bench_with_input(
                BenchmarkId::new(format!("{}_{}_providers", prompt_name, count), prompt.len()),
                prompt,
                |b, &prompt| {
                    b.iter(|| router.route(prompt, &context));
                },
            );
        }
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_keyword_extraction,
    bench_strategy_selection,
    bench_full_routing,
);
criterion_main!(benches);
