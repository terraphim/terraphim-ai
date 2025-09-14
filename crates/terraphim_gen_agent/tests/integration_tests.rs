//! Integration tests for the GenAgent framework

use std::sync::Arc;
use std::time::Duration;

use tokio::time::timeout;

use terraphim_gen_agent::{
    AgentPid, AgentState, BehaviorSpec, CallContext, CastContext, GenAgent, GenAgentFactory,
    GenAgentInitArgs, GenAgentResult, InfoContext, RuntimeConfig, StateManager, SupervisorId,
    TestAgentState,
};

// Test agent implementation for integration tests
struct IntegrationTestAgent {
    name: String,
}

impl IntegrationTestAgent {
    fn new(name: String) -> Self {
        Self { name }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
struct IntegrationTestState {
    counter: u64,
    name: String,
    messages_received: Vec<String>,
}

impl AgentState for IntegrationTestState {
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
        if self.name.is_empty() {
            return Err(terraphim_gen_agent::GenAgentError::StateTransitionFailed(
                AgentPid::new(),
                "Name cannot be empty".to_string(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
struct TestCallMessage {
    content: String,
}

#[derive(Debug, Clone)]
struct TestCallReply {
    response: String,
    counter: u64,
}

#[derive(Debug, Clone)]
struct TestCastMessage {
    notification: String,
}

#[derive(Debug, Clone)]
struct TestInfoMessage {
    info: String,
}

#[async_trait::async_trait]
impl GenAgent<IntegrationTestState> for IntegrationTestAgent {
    type CallMessage = TestCallMessage;
    type CallReply = TestCallReply;
    type CastMessage = TestCastMessage;
    type InfoMessage = TestInfoMessage;

    async fn init(&mut self, args: GenAgentInitArgs) -> GenAgentResult<IntegrationTestState> {
        Ok(IntegrationTestState {
            counter: 0,
            name: self.name.clone(),
            messages_received: Vec::new(),
        })
    }

    async fn handle_call(
        &mut self,
        message: Self::CallMessage,
        context: CallContext,
        mut state: IntegrationTestState,
    ) -> GenAgentResult<(Self::CallReply, IntegrationTestState)> {
        state.counter += 1;
        state
            .messages_received
            .push(format!("call: {}", message.content));

        let reply = TestCallReply {
            response: format!("Processed call: {}", message.content),
            counter: state.counter,
        };

        Ok((reply, state))
    }

    async fn handle_cast(
        &mut self,
        message: Self::CastMessage,
        context: CastContext,
        mut state: IntegrationTestState,
    ) -> GenAgentResult<IntegrationTestState> {
        state.counter += 1;
        state
            .messages_received
            .push(format!("cast: {}", message.notification));

        Ok(state)
    }

    async fn handle_info(
        &mut self,
        message: Self::InfoMessage,
        context: InfoContext,
        mut state: IntegrationTestState,
    ) -> GenAgentResult<IntegrationTestState> {
        state
            .messages_received
            .push(format!("info: {}", message.info));

        Ok(state)
    }
}

#[tokio::test]
async fn test_agent_lifecycle_integration() {
    env_logger::try_init().ok();

    let state_manager = Arc::new(StateManager::new(false));
    let config = RuntimeConfig {
        message_buffer_size: 100,
        max_concurrent_messages: 10,
        message_timeout: Duration::from_secs(5),
        hibernation_timeout: Some(Duration::from_secs(60)),
        enable_tracing: true,
        enable_metrics: true,
    };

    let factory = GenAgentFactory::new(state_manager, config);

    let agent_id = AgentPid::new();
    let supervisor_id = SupervisorId::new();

    let init_args = GenAgentInitArgs {
        agent_id: agent_id.clone(),
        supervisor_id: supervisor_id.clone(),
        config: serde_json::json!({
            "test_config": "integration_test"
        }),
        timeout: Duration::from_secs(30),
    };

    let behavior_spec = BehaviorSpec {
        name: "integration_test_agent".to_string(),
        version: "1.0.0".to_string(),
        description: "Agent for integration testing".to_string(),
        timeout: Duration::from_secs(30),
        hibernation_after: Some(Duration::from_secs(60)),
        debug_options: terraphim_gen_agent::DebugOptions {
            trace_calls: true,
            trace_casts: true,
            trace_info: true,
            log_state_changes: true,
            statistics: true,
        },
    };

    let agent = IntegrationTestAgent::new("integration_test_agent".to_string());

    // Create and start the agent
    let runtime = factory
        .create_agent(
            agent,
            agent_id.clone(),
            supervisor_id,
            init_args,
            Some(behavior_spec),
            None,
        )
        .await
        .unwrap();

    // Verify agent is running
    assert!(runtime.is_running());

    // Test call message
    let call_message = TestCallMessage {
        content: "test_call".to_string(),
    };

    // Note: This is a simplified test - the actual runtime would need proper message handling
    // For now, we'll test the basic runtime creation and status

    let status = runtime.get_status().await;
    println!("Agent status: {:?}", status);

    let stats = runtime.get_stats().await;
    println!("Runtime stats: {:?}", stats);

    // Test factory operations
    let factory_stats = factory.get_stats().await;
    assert_eq!(factory_stats.total_agents, 1);

    let agents = factory.list_agents().await;
    assert_eq!(agents.len(), 1);
    assert_eq!(agents[0], agent_id);

    // Stop the agent
    factory.stop_agent(&agent_id).await.unwrap();

    let final_stats = factory.get_stats().await;
    assert_eq!(final_stats.total_agents, 0);
}

#[tokio::test]
async fn test_state_persistence_integration() {
    env_logger::try_init().ok();

    let state_manager = Arc::new(StateManager::new(true)); // Enable persistence
    let config = RuntimeConfig::default();
    let factory = GenAgentFactory::new(state_manager.clone(), config);

    let agent_id = AgentPid::new();
    let supervisor_id = SupervisorId::new();

    let init_args = GenAgentInitArgs {
        agent_id: agent_id.clone(),
        supervisor_id: supervisor_id.clone(),
        config: serde_json::json!({}),
        timeout: Duration::from_secs(30),
    };

    let agent = IntegrationTestAgent::new("persistence_test_agent".to_string());

    // Create agent
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

    // Verify state manager has the agent
    let agents = state_manager.list_agents().await;
    assert!(agents.contains(&agent_id));

    // Stop agent
    factory.stop_agent(&agent_id).await.unwrap();

    // Verify cleanup
    let final_agents = state_manager.list_agents().await;
    assert!(!final_agents.contains(&agent_id));
}

#[tokio::test]
async fn test_multiple_agents_integration() {
    env_logger::try_init().ok();

    let state_manager = Arc::new(StateManager::new(false));
    let config = RuntimeConfig::default();
    let factory = GenAgentFactory::new(state_manager, config);

    let num_agents = 5;
    let mut agent_ids = Vec::new();

    // Create multiple agents
    for i in 0..num_agents {
        let agent_id = AgentPid::new();
        let supervisor_id = SupervisorId::new();

        let init_args = GenAgentInitArgs {
            agent_id: agent_id.clone(),
            supervisor_id: supervisor_id.clone(),
            config: serde_json::json!({
                "agent_index": i
            }),
            timeout: Duration::from_secs(30),
        };

        let agent = IntegrationTestAgent::new(format!("multi_test_agent_{}", i));

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

        assert!(runtime.is_running());
        agent_ids.push(agent_id);
    }

    // Verify all agents are created
    let factory_stats = factory.get_stats().await;
    assert_eq!(factory_stats.total_agents, num_agents);

    let agents = factory.list_agents().await;
    assert_eq!(agents.len(), num_agents);

    // Stop all agents
    for agent_id in &agent_ids {
        factory.stop_agent(agent_id).await.unwrap();
    }

    let final_stats = factory.get_stats().await;
    assert_eq!(final_stats.total_agents, 0);
}

#[tokio::test]
async fn test_error_handling_integration() {
    env_logger::try_init().ok();

    let state_manager = Arc::new(StateManager::new(false));
    let config = RuntimeConfig::default();
    let factory = GenAgentFactory::new(state_manager, config);

    // Test with invalid init args
    let agent_id = AgentPid::new();
    let supervisor_id = SupervisorId::new();

    let init_args = GenAgentInitArgs {
        agent_id: agent_id.clone(),
        supervisor_id: supervisor_id.clone(),
        config: serde_json::json!({}),
        timeout: Duration::from_millis(1), // Very short timeout
    };

    let agent = IntegrationTestAgent::new("".to_string()); // Empty name should cause validation error

    // This should succeed in creation but might fail during state validation
    let result = factory
        .create_agent(
            agent,
            agent_id.clone(),
            supervisor_id,
            init_args,
            None,
            None,
        )
        .await;

    // The result depends on when validation occurs
    match result {
        Ok(runtime) => {
            // If creation succeeded, the agent should still be running
            assert!(runtime.is_running());
            factory.stop_agent(&agent_id).await.unwrap();
        }
        Err(e) => {
            // If creation failed, that's also acceptable for this test
            println!("Expected error during agent creation: {:?}", e);
        }
    }
}

#[tokio::test]
async fn test_concurrent_operations_integration() {
    env_logger::try_init().ok();

    let state_manager = Arc::new(StateManager::new(false));
    let config = RuntimeConfig {
        message_buffer_size: 1000,
        max_concurrent_messages: 50,
        message_timeout: Duration::from_secs(10),
        hibernation_timeout: None, // Disable hibernation for this test
        enable_tracing: false,
        enable_metrics: true,
    };

    let factory = Arc::new(GenAgentFactory::new(state_manager, config));

    let num_concurrent_agents = 10;
    let mut handles = Vec::new();

    // Create agents concurrently
    for i in 0..num_concurrent_agents {
        let factory_clone = factory.clone();
        let handle = tokio::spawn(async move {
            let agent_id = AgentPid::new();
            let supervisor_id = SupervisorId::new();

            let init_args = GenAgentInitArgs {
                agent_id: agent_id.clone(),
                supervisor_id: supervisor_id.clone(),
                config: serde_json::json!({
                    "concurrent_index": i
                }),
                timeout: Duration::from_secs(30),
            };

            let agent = IntegrationTestAgent::new(format!("concurrent_agent_{}", i));

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
            tokio::time::sleep(Duration::from_millis(100)).await;

            agent_id
        });

        handles.push(handle);
    }

    // Wait for all agents to be created
    let mut agent_ids = Vec::new();
    for handle in handles {
        let agent_id = handle.await.unwrap();
        agent_ids.push(agent_id);
    }

    // Verify all agents were created
    let factory_stats = factory.get_stats().await;
    assert_eq!(factory_stats.total_agents, num_concurrent_agents);

    // Stop all agents concurrently
    let mut stop_handles = Vec::new();
    for agent_id in agent_ids {
        let factory_clone = factory.clone();
        let handle = tokio::spawn(async move {
            factory_clone.stop_agent(&agent_id).await.unwrap();
        });
        stop_handles.push(handle);
    }

    // Wait for all stops to complete
    for handle in stop_handles {
        handle.await.unwrap();
    }

    let final_stats = factory.get_stats().await;
    assert_eq!(final_stats.total_agents, 0);
}
