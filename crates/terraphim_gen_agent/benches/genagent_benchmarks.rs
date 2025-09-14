//! Benchmarks for the GenAgent framework

use std::sync::Arc;
use std::time::Duration;

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use tokio::runtime::Runtime;

use terraphim_gen_agent::{
    AgentPid, AgentState, BehaviorSpec, CallContext, CastContext, GenAgent, GenAgentFactory,
    GenAgentInitArgs, GenAgentResult, InfoContext, RuntimeConfig, StateContainer, StateManager,
    SupervisorId,
};

// Benchmark agent state
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
struct BenchmarkState {
    counter: u64,
    data: Vec<u8>,
    metadata: std::collections::HashMap<String, String>,
}

impl AgentState for BenchmarkState {
    fn serialize(&self) -> GenAgentResult<String> {
        serde_json::to_string(self).map_err(|e| {
            terraphim_gen_agent::GenAgentError::StateSerialization(AgentPid::new(), e.to_string())
        })
    }

    fn deserialize(data: &str) -> GenAgentResult<Self> {
        serde_json::from_str(data).map_err(|e| {
            terraphim_gen_agent::GenAgentError::StateDeserialization(AgentPid::new(), e.to_string())
        })
    }

    fn validate(&self) -> GenAgentResult<()> {
        Ok(())
    }
}

impl BenchmarkState {
    fn new(data_size: usize) -> Self {
        let mut metadata = std::collections::HashMap::new();
        metadata.insert("benchmark".to_string(), "true".to_string());
        metadata.insert("version".to_string(), "1.0.0".to_string());

        Self {
            counter: 0,
            data: vec![0u8; data_size],
            metadata,
        }
    }
}

// Benchmark agent implementation
struct BenchmarkAgent {
    name: String,
}

impl BenchmarkAgent {
    fn new(name: String) -> Self {
        Self { name }
    }
}

#[derive(Debug, Clone)]
struct BenchmarkMessage {
    payload: Vec<u8>,
    timestamp: std::time::Instant,
}

#[derive(Debug, Clone)]
struct BenchmarkReply {
    processed_size: usize,
    processing_time: Duration,
}

#[async_trait::async_trait]
impl GenAgent<BenchmarkState> for BenchmarkAgent {
    type CallMessage = BenchmarkMessage;
    type CallReply = BenchmarkReply;
    type CastMessage = BenchmarkMessage;
    type InfoMessage = BenchmarkMessage;

    async fn init(&mut self, args: GenAgentInitArgs) -> GenAgentResult<BenchmarkState> {
        Ok(BenchmarkState::new(1024)) // 1KB initial state
    }

    async fn handle_call(
        &mut self,
        message: Self::CallMessage,
        context: CallContext,
        mut state: BenchmarkState,
    ) -> GenAgentResult<(Self::CallReply, BenchmarkState)> {
        let start = std::time::Instant::now();

        state.counter += 1;

        // Simulate some processing work
        let checksum: u32 = message.payload.iter().map(|&b| b as u32).sum();
        state.data[0] = (checksum % 256) as u8;

        let processing_time = start.elapsed();

        let reply = BenchmarkReply {
            processed_size: message.payload.len(),
            processing_time,
        };

        Ok((reply, state))
    }

    async fn handle_cast(
        &mut self,
        message: Self::CastMessage,
        context: CastContext,
        mut state: BenchmarkState,
    ) -> GenAgentResult<BenchmarkState> {
        state.counter += 1;

        // Simulate processing
        let checksum: u32 = message.payload.iter().map(|&b| b as u32).sum();
        if !state.data.is_empty() {
            state.data[0] = (checksum % 256) as u8;
        }

        Ok(state)
    }

    async fn handle_info(
        &mut self,
        message: Self::InfoMessage,
        context: InfoContext,
        state: BenchmarkState,
    ) -> GenAgentResult<BenchmarkState> {
        // Info messages don't modify state in this benchmark
        Ok(state)
    }
}

fn bench_state_operations(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let mut group = c.benchmark_group("state_operations");

    for size in [1024, 4096, 16384, 65536].iter() {
        group.bench_with_input(BenchmarkId::new("serialize", size), size, |b, &size| {
            let state = BenchmarkState::new(size);
            b.iter(|| {
                black_box(state.serialize().unwrap());
            });
        });

        group.bench_with_input(BenchmarkId::new("deserialize", size), size, |b, &size| {
            let state = BenchmarkState::new(size);
            let serialized = state.serialize().unwrap();
            b.iter(|| {
                black_box(BenchmarkState::deserialize(&serialized).unwrap());
            });
        });

        group.bench_with_input(BenchmarkId::new("checksum", size), size, |b, &size| {
            let state = BenchmarkState::new(size);
            b.iter(|| {
                black_box(state.checksum());
            });
        });
    }

    group.finish();
}

fn bench_state_container_operations(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let mut group = c.benchmark_group("state_container");

    for size in [1024, 4096, 16384].iter() {
        group.bench_with_input(BenchmarkId::new("create", size), size, |b, &size| {
            b.iter(|| {
                let agent_id = AgentPid::new();
                let state = BenchmarkState::new(size);
                black_box(StateContainer::new(agent_id, state).unwrap());
            });
        });

        group.bench_with_input(BenchmarkId::new("update", size), size, |b, &size| {
            let agent_id = AgentPid::new();
            let initial_state = BenchmarkState::new(size);
            let mut container = StateContainer::new(agent_id, initial_state).unwrap();

            b.iter(|| {
                let new_state = BenchmarkState::new(size);
                black_box(container.update_state(new_state).unwrap());
            });
        });
    }

    group.finish();
}

fn bench_state_manager_operations(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let mut group = c.benchmark_group("state_manager");

    for num_agents in [10, 50, 100, 500].iter() {
        group.bench_with_input(
            BenchmarkId::new("store_retrieve", num_agents),
            num_agents,
            |b, &num_agents| {
                b.to_async(&rt).iter(|| async {
                    let manager = StateManager::new(false);

                    // Store multiple agent states
                    for i in 0..num_agents {
                        let agent_id = AgentPid::new();
                        let state = BenchmarkState::new(1024);
                        let container = StateContainer::new(agent_id.clone(), state).unwrap();
                        manager.store_state(agent_id, container).await.unwrap();
                    }

                    // Retrieve all states
                    let agents = manager.list_agents().await;
                    for agent_id in agents {
                        black_box(
                            manager
                                .get_state::<BenchmarkState>(&agent_id)
                                .await
                                .unwrap(),
                        );
                    }
                });
            },
        );
    }

    group.finish();
}

fn bench_agent_creation(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let mut group = c.benchmark_group("agent_creation");

    for num_agents in [1, 10, 50].iter() {
        group.bench_with_input(
            BenchmarkId::new("create_agents", num_agents),
            num_agents,
            |b, &num_agents| {
                b.to_async(&rt).iter(|| async {
                    let state_manager = Arc::new(StateManager::new(false));
                    let config = RuntimeConfig {
                        message_buffer_size: 100,
                        max_concurrent_messages: 10,
                        message_timeout: Duration::from_secs(1),
                        hibernation_timeout: None,
                        enable_tracing: false,
                        enable_metrics: false,
                    };
                    let factory = GenAgentFactory::new(state_manager, config);

                    let mut agent_ids = Vec::new();

                    for i in 0..num_agents {
                        let agent_id = AgentPid::new();
                        let supervisor_id = SupervisorId::new();

                        let init_args = GenAgentInitArgs {
                            agent_id: agent_id.clone(),
                            supervisor_id: supervisor_id.clone(),
                            config: serde_json::json!({}),
                            timeout: Duration::from_secs(1),
                        };

                        let agent = BenchmarkAgent::new(format!("bench_agent_{}", i));

                        let runtime = factory
                            .create_agent(
                                agent,
                                agent_id.clone(),
                                supervisor_id,
                                init_args,
                                None,
                                None,
                            )
                            .await
                            .unwrap();

                        agent_ids.push(agent_id);
                        black_box(runtime);
                    }

                    // Clean up
                    for agent_id in agent_ids {
                        factory.stop_agent(&agent_id).await.unwrap();
                    }
                });
            },
        );
    }

    group.finish();
}

fn bench_message_throughput(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let mut group = c.benchmark_group("message_throughput");
    group.sample_size(10); // Reduce sample size for expensive operations

    for payload_size in [64, 256, 1024].iter() {
        group.bench_with_input(
            BenchmarkId::new("cast_messages", payload_size),
            payload_size,
            |b, &payload_size| {
                b.to_async(&rt).iter(|| async {
                    let state_manager = Arc::new(StateManager::new(false));
                    let config = RuntimeConfig {
                        message_buffer_size: 1000,
                        max_concurrent_messages: 100,
                        message_timeout: Duration::from_secs(5),
                        hibernation_timeout: None,
                        enable_tracing: false,
                        enable_metrics: false,
                    };
                    let factory = GenAgentFactory::new(state_manager, config);

                    let agent_id = AgentPid::new();
                    let supervisor_id = SupervisorId::new();

                    let init_args = GenAgentInitArgs {
                        agent_id: agent_id.clone(),
                        supervisor_id: supervisor_id.clone(),
                        config: serde_json::json!({}),
                        timeout: Duration::from_secs(5),
                    };

                    let agent = BenchmarkAgent::new("throughput_test".to_string());

                    let runtime = factory
                        .create_agent(
                            agent,
                            agent_id.clone(),
                            supervisor_id,
                            init_args,
                            None,
                            None,
                        )
                        .await
                        .unwrap();

                    // Send multiple cast messages
                    let num_messages = 100;
                    for _ in 0..num_messages {
                        let message = BenchmarkMessage {
                            payload: vec![42u8; payload_size],
                            timestamp: std::time::Instant::now(),
                        };

                        // Note: This is a simplified benchmark - actual message sending
                        // would require proper runtime integration
                        black_box(message);
                    }

                    // Clean up
                    factory.stop_agent(&agent_id).await.unwrap();
                });
            },
        );
    }

    group.finish();
}

fn bench_concurrent_agents(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let mut group = c.benchmark_group("concurrent_agents");
    group.sample_size(10);

    for num_agents in [5, 10, 20].iter() {
        group.bench_with_input(
            BenchmarkId::new("concurrent_operations", num_agents),
            num_agents,
            |b, &num_agents| {
                b.to_async(&rt).iter(|| async {
                    let state_manager = Arc::new(StateManager::new(false));
                    let config = RuntimeConfig::default();
                    let factory = Arc::new(GenAgentFactory::new(state_manager, config));

                    let mut handles = Vec::new();

                    // Create agents concurrently
                    for i in 0..num_agents {
                        let factory_clone = factory.clone();
                        let handle = tokio::spawn(async move {
                            let agent_id = AgentPid::new();
                            let supervisor_id = SupervisorId::new();

                            let init_args = GenAgentInitArgs {
                                agent_id: agent_id.clone(),
                                supervisor_id: supervisor_id.clone(),
                                config: serde_json::json!({}),
                                timeout: Duration::from_secs(5),
                            };

                            let agent = BenchmarkAgent::new(format!("concurrent_{}", i));

                            let runtime = factory_clone
                                .create_agent(
                                    agent,
                                    agent_id.clone(),
                                    supervisor_id,
                                    init_args,
                                    None,
                                    None,
                                )
                                .await
                                .unwrap();

                            // Simulate some work
                            tokio::time::sleep(Duration::from_millis(10)).await;

                            agent_id
                        });

                        handles.push(handle);
                    }

                    // Wait for all agents
                    let mut agent_ids = Vec::new();
                    for handle in handles {
                        let agent_id = handle.await.unwrap();
                        agent_ids.push(agent_id);
                    }

                    black_box(&agent_ids);

                    // Clean up
                    for agent_id in agent_ids {
                        factory.stop_agent(&agent_id).await.unwrap();
                    }
                });
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_state_operations,
    bench_state_container_operations,
    bench_state_manager_operations,
    bench_agent_creation,
    bench_message_throughput,
    bench_concurrent_agents
);

criterion_main!(benches);
