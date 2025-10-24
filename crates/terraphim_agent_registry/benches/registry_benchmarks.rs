//! Benchmarks for the agent registry

use criterion::{black_box, BenchmarkId, Criterion};
use tokio::runtime::Runtime;

use terraphim_agent_registry::{
    AgentDiscoveryQuery, AgentMetadata, AgentPid, AgentRegistry, AgentRole, RegistryBuilder,
    SupervisorId,
};

#[allow(dead_code)]
fn bench_agent_registration(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let mut group = c.benchmark_group("agent_registration");

    for num_agents in [10, 50, 100].iter() {
        group.bench_with_input(
            BenchmarkId::new("register_agents", num_agents),
            num_agents,
            |b, &num_agents| {
                b.iter(|| {
                    rt.block_on(async {
                        let registry = RegistryBuilder::new().build().unwrap();

                        for i in 0..num_agents {
                            let agent_id = AgentPid::new();
                            let supervisor_id = SupervisorId::new();

                            let role = AgentRole::new(
                                format!("agent_{}", i),
                                format!("Agent {}", i),
                                format!("Test agent {}", i),
                            );

                            let metadata = AgentMetadata::new(agent_id, supervisor_id, role);
                            registry.register_agent(metadata).await.unwrap();
                            black_box(());
                        }
                    })
                })
            },
        );
    }

    group.finish();
}

#[allow(dead_code)]
fn bench_agent_discovery(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let mut group = c.benchmark_group("agent_discovery");

    for num_agents in [10, 50, 100].iter() {
        group.bench_with_input(
            BenchmarkId::new("discover_agents", num_agents),
            num_agents,
            |b, &num_agents| {
                b.iter(|| {
                    rt.block_on(async {
                        let registry = RegistryBuilder::new().build().unwrap();

                        // Pre-populate registry
                        for i in 0..num_agents {
                            let agent_id = AgentPid::new();
                            let supervisor_id = SupervisorId::new();

                            let role = AgentRole::new(
                                format!("role_{}", i % 5), // 5 different roles
                                format!("Role {}", i % 5),
                                format!("Test role {}", i % 5),
                            );

                            let metadata = AgentMetadata::new(agent_id, supervisor_id, role);
                            registry.register_agent(metadata).await.unwrap();
                        }

                        // Perform discovery
                        let query = AgentDiscoveryQuery {
                            required_roles: vec!["role_0".to_string()],
                            required_capabilities: Vec::new(),
                            required_domains: Vec::new(),
                            task_description: None,
                            min_success_rate: None,
                            max_resource_usage: None,
                            preferred_tags: Vec::new(),
                        };

                        let _result = registry.discover_agents(query).await.unwrap();
                        black_box(());
                    })
                })
            },
        );
    }

    group.finish();
}

// Temporarily disabled due to API compatibility issues
// criterion_group!(benches, bench_agent_registration, bench_agent_discovery);
// criterion_main!(benches);
