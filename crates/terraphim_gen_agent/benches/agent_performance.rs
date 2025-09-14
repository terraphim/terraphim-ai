//! Performance benchmarks for GenAgent framework

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use std::time::Duration;
use tokio::runtime::Runtime;

use terraphim_agent_messaging::{AgentMailbox, AgentMessage, MailboxConfig};
use terraphim_gen_agent::{
    state::ExampleState, AgentPid, ExampleGenAgent, ExampleMessage, ExampleReply, GenAgentRuntime,
    InMemoryStateManager, InitArgs, SupervisorId,
};

fn create_runtime() -> Runtime {
    Runtime::new().unwrap()
}

fn bench_agent_initialization(c: &mut Criterion) {
    let rt = create_runtime();

    c.bench_function("agent_initialization", |b| {
        b.to_async(&rt).iter(|| async {
            let agent = ExampleGenAgent::new();
            let agent_id = AgentPid::new();
            let mailbox = AgentMailbox::new(agent_id.clone(), MailboxConfig::default());
            let runtime = GenAgentRuntime::new(agent, mailbox, None);

            let args = InitArgs {
                agent_id: agent_id.clone(),
                supervisor_id: SupervisorId::new(),
                config: serde_json::json!({"name": "bench_agent"}),
            };

            black_box(runtime.init(args).await.unwrap());
        });
    });
}

fn bench_message_handling(c: &mut Criterion) {
    let rt = create_runtime();

    let mut group = c.benchmark_group("message_handling");

    for message_count in [10, 100, 1000].iter() {
        group.bench_with_input(
            BenchmarkId::new("cast_messages", message_count),
            message_count,
            |b, &message_count| {
                b.to_async(&rt).iter(|| async {
                    let agent = ExampleGenAgent::new();
                    let agent_id = AgentPid::new();
                    let mailbox = AgentMailbox::new(agent_id.clone(), MailboxConfig::default());
                    let runtime = GenAgentRuntime::new(agent, mailbox, None);

                    // Initialize
                    let args = InitArgs {
                        agent_id: agent_id.clone(),
                        supervisor_id: SupervisorId::new(),
                        config: serde_json::json!({"name": "bench_agent"}),
                    };
                    runtime.init(args).await.unwrap();

                    // Send messages
                    for _ in 0..message_count {
                        let cast_msg =
                            AgentMessage::cast(agent_id.clone(), ExampleMessage::Increment);
                        black_box(runtime.handle_message(cast_msg).await.unwrap());
                    }
                });
            },
        );
    }

    group.finish();
}

fn bench_state_transitions(c: &mut Criterion) {
    let rt = create_runtime();

    c.bench_function("state_transitions", |b| {
        b.to_async(&rt).iter(|| async {
            let agent = ExampleGenAgent::new();
            let agent_id = AgentPid::new();
            let mailbox = AgentMailbox::new(agent_id.clone(), MailboxConfig::default());
            let runtime = GenAgentRuntime::new(agent, mailbox, None);

            // Initialize
            let args = InitArgs {
                agent_id: agent_id.clone(),
                supervisor_id: SupervisorId::new(),
                config: serde_json::json!({"name": "bench_agent"}),
            };
            runtime.init(args).await.unwrap();

            // Perform state transitions
            for _ in 0..100 {
                let cast_msg = AgentMessage::cast(agent_id.clone(), ExampleMessage::Increment);
                black_box(runtime.handle_message(cast_msg).await.unwrap());
            }
        });
    });
}

fn bench_state_persistence(c: &mut Criterion) {
    let rt = create_runtime();

    let mut group = c.benchmark_group("state_persistence");

    // Benchmark with persistence disabled
    group.bench_function("no_persistence", |b| {
        b.to_async(&rt).iter(|| async {
            let mut agent = ExampleGenAgent::new();
            let config = terraphim_gen_agent::GenAgentConfig {
                enable_persistence: false,
                ..Default::default()
            };
            agent = agent.with_config(config);

            let agent_id = AgentPid::new();
            let mailbox = AgentMailbox::new(agent_id.clone(), MailboxConfig::default());
            let runtime = GenAgentRuntime::new(agent, mailbox, None);

            let args = InitArgs {
                agent_id: agent_id.clone(),
                supervisor_id: SupervisorId::new(),
                config: serde_json::json!({"name": "bench_agent"}),
            };
            runtime.init(args).await.unwrap();

            // Perform operations
            for _ in 0..50 {
                let cast_msg = AgentMessage::cast(agent_id.clone(), ExampleMessage::Increment);
                black_box(runtime.handle_message(cast_msg).await.unwrap());
            }
        });
    });

    // Benchmark with persistence enabled
    group.bench_function("with_persistence", |b| {
        b.to_async(&rt).iter(|| async {
            let mut agent = ExampleGenAgent::new();
            let config = terraphim_gen_agent::GenAgentConfig {
                enable_persistence: true,
                ..Default::default()
            };
            agent = agent.with_config(config);

            let agent_id = AgentPid::new();
            let mailbox = AgentMailbox::new(agent_id.clone(), MailboxConfig::default());
            let state_manager = std::sync::Arc::new(InMemoryStateManager::new());
            let runtime = GenAgentRuntime::new(agent, mailbox, Some(state_manager));

            let args = InitArgs {
                agent_id: agent_id.clone(),
                supervisor_id: SupervisorId::new(),
                config: serde_json::json!({"name": "bench_agent"}),
            };
            runtime.init(args).await.unwrap();

            // Perform operations
            for _ in 0..50 {
                let cast_msg = AgentMessage::cast(agent_id.clone(), ExampleMessage::Increment);
                black_box(runtime.handle_message(cast_msg).await.unwrap());
            }
        });
    });

    group.finish();
}

fn bench_concurrent_agents(c: &mut Criterion) {
    let rt = create_runtime();

    let mut group = c.benchmark_group("concurrent_agents");

    for agent_count in [10, 50, 100].iter() {
        group.bench_with_input(
            BenchmarkId::new("concurrent_message_processing", agent_count),
            agent_count,
            |b, &agent_count| {
                b.to_async(&rt).iter(|| async {
                    let mut runtimes = Vec::new();

                    // Create multiple agents
                    for i in 0..agent_count {
                        let agent = ExampleGenAgent::new();
                        let agent_id = AgentPid::new();
                        let mailbox = AgentMailbox::new(agent_id.clone(), MailboxConfig::default());
                        let runtime = GenAgentRuntime::new(agent, mailbox, None);

                        let args = InitArgs {
                            agent_id: agent_id.clone(),
                            supervisor_id: SupervisorId::new(),
                            config: serde_json::json!({"name": format!("bench_agent_{}", i)}),
                        };
                        runtime.init(args).await.unwrap();

                        runtimes.push(runtime);
                    }

                    // Send messages to all agents concurrently
                    let mut handles = Vec::new();
                    for runtime in &runtimes {
                        let runtime_clone = runtime.clone_for_task();
                        let handle = tokio::spawn(async move {
                            for _ in 0..10 {
                                let cast_msg = AgentMessage::cast(
                                    runtime_clone.mailbox.agent_id().clone(),
                                    ExampleMessage::Increment,
                                );
                                runtime_clone.handle_message(cast_msg).await.unwrap();
                            }
                        });
                        handles.push(handle);
                    }

                    // Wait for all to complete
                    for handle in handles {
                        black_box(handle.await.unwrap());
                    }
                });
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_agent_initialization,
    bench_message_handling,
    bench_state_transitions,
    bench_state_persistence,
    bench_concurrent_agents
);
criterion_main!(benches);
