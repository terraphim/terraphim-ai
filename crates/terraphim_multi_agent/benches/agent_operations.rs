use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use std::sync::Arc;
use tokio::runtime::Runtime;

use terraphim_multi_agent::{
    test_utils::create_test_agent_simple, AgentRegistry, CommandInput, CommandType,
};

/// Benchmark agent creation time
fn bench_agent_creation(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    c.bench_function("agent_creation", |b| {
        b.iter(|| {
            rt.block_on(async {
                let agent = create_test_agent_simple().await;
                black_box(agent)
            })
        })
    });
}

/// Benchmark agent initialization
fn bench_agent_initialization(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    c.bench_function("agent_initialization", |b| {
        b.iter(|| {
            rt.block_on(async {
                let agent = create_test_agent_simple().await.unwrap();
                let result = agent.initialize().await;
                black_box(result)
            })
        })
    });
}

/// Benchmark command processing for different command types
fn bench_command_processing(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let command_types = vec![
        CommandType::Generate,
        CommandType::Answer,
        CommandType::Analyze,
        CommandType::Create,
        CommandType::Review,
    ];

    for command_type in command_types {
        let group_name = format!("command_processing_{:?}", command_type);
        c.bench_with_input(
            BenchmarkId::new(&group_name, "standard"),
            &command_type,
            |b, cmd_type| {
                b.iter(|| {
                    rt.block_on(async {
                        let agent = create_test_agent_simple().await.unwrap();
                        agent.initialize().await.unwrap();

                        let input = CommandInput {
                            command_type: cmd_type.clone(),
                            text: "Test command for benchmarking".to_string(),
                            parameters: std::collections::HashMap::new(),
                            source: terraphim_multi_agent::CommandSource::User,
                            priority: terraphim_multi_agent::CommandPriority::Normal,
                            timeout_ms: None,
                        };

                        let result = agent.process_command(black_box(input)).await;
                        black_box(result)
                    })
                })
            },
        );
    }
}

/// Benchmark agent registry operations
fn bench_registry_operations(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    c.bench_function("registry_register_agent", |b| {
        b.iter(|| {
            rt.block_on(async {
                let registry = AgentRegistry::new();
                let agent = create_test_agent_simple().await.unwrap();
                agent.initialize().await.unwrap();

                let result = registry.register_agent(Arc::new(agent)).await;
                black_box(result)
            })
        })
    });

    c.bench_function("registry_find_by_capability", |b| {
        b.iter(|| {
            rt.block_on(async {
                let registry = AgentRegistry::new();

                // Pre-populate with test agents
                for _i in 0..10 {
                    let agent = create_test_agent_simple().await.unwrap();
                    agent.initialize().await.unwrap();
                    registry.register_agent(Arc::new(agent)).await.unwrap();
                }

                let result = registry.find_agents_by_capability("test_capability").await;
                black_box(result)
            })
        })
    });
}

/// Benchmark memory operations
fn bench_memory_operations(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    c.bench_function("memory_context_enrichment", |b| {
        b.iter(|| {
            rt.block_on(async {
                let agent = create_test_agent_simple().await.unwrap();
                agent.initialize().await.unwrap();

                // Simulate context enrichment operation
                let query = "test query for context enrichment";
                let result = agent.get_enriched_context_for_query(query).await;
                black_box(result)
            })
        })
    });

    c.bench_function("memory_save_state", |b| {
        b.iter(|| {
            rt.block_on(async {
                let agent = create_test_agent_simple().await.unwrap();
                agent.initialize().await.unwrap();

                let result = agent.save_state().await;
                black_box(result)
            })
        })
    });
}

/// Benchmark batch operations
fn bench_batch_operations(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let batch_sizes = vec![1, 5, 10, 20, 50];

    for batch_size in batch_sizes {
        c.bench_with_input(
            BenchmarkId::new("batch_command_processing", batch_size),
            &batch_size,
            |b, &size| {
                b.iter(|| {
                    rt.block_on(async {
                        let agent = create_test_agent_simple().await.unwrap();
                        agent.initialize().await.unwrap();

                        let mut results = Vec::new();

                        for i in 0..size {
                            let input = CommandInput {
                                command_type: CommandType::Generate,
                                text: format!("Batch command {}", i),
                                parameters: std::collections::HashMap::new(),
                                source: terraphim_multi_agent::CommandSource::User,
                                priority: terraphim_multi_agent::CommandPriority::Normal,
                                timeout_ms: None,
                            };

                            let result = agent.process_command(input).await;
                            results.push(result);
                        }

                        black_box(results)
                    })
                })
            },
        );
    }
}

/// Benchmark concurrent operations
fn bench_concurrent_operations(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let concurrency_levels = vec![1, 2, 4, 8];

    for concurrency in concurrency_levels {
        c.bench_with_input(
            BenchmarkId::new("concurrent_command_processing", concurrency),
            &concurrency,
            |b, &level| {
                b.iter(|| {
                    rt.block_on(async {
                        let mut tasks = Vec::new();

                        for i in 0..level {
                            let task = tokio::spawn(async move {
                                let agent = create_test_agent_simple().await.unwrap();
                                agent.initialize().await.unwrap();

                                let input = CommandInput {
                                    command_type: CommandType::Answer,
                                    text: format!("Concurrent command {}", i),
                                    parameters: std::collections::HashMap::new(),
                                    source: terraphim_multi_agent::CommandSource::User,
                                    priority: terraphim_multi_agent::CommandPriority::Normal,
                                    timeout_ms: None,
                                };

                                agent.process_command(input).await
                            });
                            tasks.push(task);
                        }

                        let results = futures::future::join_all(tasks).await;
                        black_box(results)
                    })
                })
            },
        );
    }
}

/// Benchmark knowledge graph operations
fn bench_knowledge_graph_operations(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    c.bench_function("rolegraph_query", |b| {
        b.iter(|| {
            rt.block_on(async {
                let agent = create_test_agent_simple().await.unwrap();
                agent.initialize().await.unwrap();

                // Test knowledge graph query operation
                let query = "test knowledge query";
                let result = agent.rolegraph.query_graph(query, Some(5), None);
                black_box(result)
            })
        })
    });

    c.bench_function("rolegraph_find_matching_nodes", |b| {
        b.iter(|| {
            rt.block_on(async {
                let agent = create_test_agent_simple().await.unwrap();
                agent.initialize().await.unwrap();

                let query = "test node matching";
                let result = agent.rolegraph.find_matching_node_ids(query);
                black_box(result)
            })
        })
    });

    c.bench_function("rolegraph_path_connectivity", |b| {
        b.iter(|| {
            rt.block_on(async {
                let agent = create_test_agent_simple().await.unwrap();
                agent.initialize().await.unwrap();

                let query = "test path connectivity";
                let result = agent.rolegraph.is_all_terms_connected_by_path(query);
                black_box(result)
            })
        })
    });
}

/// Benchmark automata operations
fn bench_automata_operations(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    c.bench_function("automata_autocomplete", |b| {
        b.iter(|| {
            rt.block_on(async {
                let agent = create_test_agent_simple().await.unwrap();
                agent.initialize().await.unwrap();

                // Test autocomplete operation
                let prefix = "test";
                let result =
                    terraphim_automata::autocomplete_search(&agent.automata, prefix, Some(10));
                black_box(result)
            })
        })
    });

    c.bench_function("automata_find_matches", |b| {
        b.iter(|| {
            rt.block_on(async {
                let agent = create_test_agent_simple().await.unwrap();
                agent.initialize().await.unwrap();

                let _text = "test text for finding matches";
                // Note: AutocompleteIndex doesn't have find_matches method
                // This benchmark needs to be updated to use correct API
                let result = ();
                black_box(result)
            })
        })
    });
}

/// Benchmark LLM client operations
fn bench_llm_operations(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    c.bench_function("llm_simple_generation", |b| {
        b.iter(|| {
            rt.block_on(async {
                let agent = create_test_agent_simple().await.unwrap();
                agent.initialize().await.unwrap();

                let messages = vec![terraphim_multi_agent::LlmMessage::user(
                    "Test message for benchmarking".to_string(),
                )];

                let request = terraphim_multi_agent::LlmRequest::new(messages);
                let result = agent.llm_client.generate(request).await;
                black_box(result)
            })
        })
    });
}

/// Benchmark tracking operations
fn bench_tracking_operations(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    c.bench_function("token_usage_tracking", |b| {
        b.iter(|| {
            rt.block_on(async {
                let agent = create_test_agent_simple().await.unwrap();
                agent.initialize().await.unwrap();

                let record = terraphim_multi_agent::TokenUsageRecord::new(
                    agent.agent_id,
                    "test-model".to_string(),
                    100,
                    50,
                    0.01,
                    1000,
                );

                let mut tracker = agent.token_tracker.write().await;
                tracker.record_usage(black_box(record));

                let stats = tracker.get_today_usage();
                black_box(stats)
            })
        })
    });

    c.bench_function("cost_tracking", |b| {
        b.iter(|| {
            rt.block_on(async {
                let agent = create_test_agent_simple().await.unwrap();
                agent.initialize().await.unwrap();

                let mut cost_tracker = agent.cost_tracker.write().await;
                cost_tracker.record_spending(agent.agent_id, black_box(0.01));

                let result = cost_tracker.check_budget_limits(agent.agent_id, 0.001);
                black_box(result)
            })
        })
    });
}

criterion_group!(
    benches,
    bench_agent_creation,
    bench_agent_initialization,
    bench_command_processing,
    bench_registry_operations,
    bench_memory_operations,
    bench_batch_operations,
    bench_concurrent_operations,
    bench_knowledge_graph_operations,
    bench_automata_operations,
    bench_llm_operations,
    bench_tracking_operations
);

criterion_main!(benches);
