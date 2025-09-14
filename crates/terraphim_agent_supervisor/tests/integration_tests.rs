//! Integration tests for the supervision system

use std::sync::Arc;
use std::time::Duration;

use serde_json::json;
use tokio::time::sleep;

use terraphim_agent_supervisor::{
    AgentSpec, AgentSupervisor, ExitReason, RestartIntensity, RestartPolicy, RestartStrategy,
    SupervisorConfig, SupervisorStatus, TestAgentFactory,
};

#[tokio::test]
async fn test_supervision_tree_basic_operations() {
    env_logger::try_init().ok();

    // Create supervisor with default configuration
    let config = SupervisorConfig::default();
    let factory = Arc::new(TestAgentFactory);
    let mut supervisor = AgentSupervisor::new(config, factory);

    // Start supervisor
    supervisor.start().await.unwrap();
    assert_eq!(supervisor.status(), SupervisorStatus::Running);

    // Spawn multiple agents
    let spec1 = AgentSpec::new("test".to_string(), json!({"name": "agent1"}))
        .with_name("test-agent-1".to_string());
    let spec2 = AgentSpec::new("test".to_string(), json!({"name": "agent2"}))
        .with_name("test-agent-2".to_string());

    let agent_id1 = supervisor.spawn_agent(spec1).await.unwrap();
    let agent_id2 = supervisor.spawn_agent(spec2).await.unwrap();

    // Verify agents are running
    let children = supervisor.get_children().await;
    assert_eq!(children.len(), 2);
    assert!(children.contains_key(&agent_id1));
    assert!(children.contains_key(&agent_id2));

    // Stop supervisor (should stop all agents)
    supervisor.stop().await.unwrap();
    assert_eq!(supervisor.status(), SupervisorStatus::Stopped);

    // Verify all agents are stopped
    let children = supervisor.get_children().await;
    assert_eq!(children.len(), 0);
}

#[tokio::test]
async fn test_agent_restart_on_failure() {
    env_logger::try_init().ok();

    // Create supervisor with lenient restart policy
    let mut config = SupervisorConfig::default();
    config.restart_policy = RestartPolicy::lenient_one_for_one();

    let factory = Arc::new(TestAgentFactory);
    let mut supervisor = AgentSupervisor::new(config, factory);

    supervisor.start().await.unwrap();

    // Spawn an agent
    let spec = AgentSpec::new("test".to_string(), json!({}));
    let agent_id = supervisor.spawn_agent(spec).await.unwrap();

    // Verify agent is running
    let children = supervisor.get_children().await;
    assert_eq!(children.len(), 1);
    let original_start_time = children.get(&agent_id).unwrap().start_time;

    // Simulate agent failure
    supervisor
        .handle_agent_exit(
            agent_id.clone(),
            ExitReason::Error("simulated failure".to_string()),
        )
        .await
        .unwrap();

    // Give some time for restart
    sleep(Duration::from_millis(100)).await;

    // Verify agent was restarted
    let children = supervisor.get_children().await;
    assert_eq!(children.len(), 1);

    let agent_info = children.get(&agent_id).unwrap();
    assert_eq!(agent_info.restart_count, 1);
    assert!(agent_info.last_restart.is_some());

    supervisor.stop().await.unwrap();
}

#[tokio::test]
async fn test_restart_strategy_one_for_all() {
    env_logger::try_init().ok();

    // Create supervisor with OneForAll restart strategy
    let mut config = SupervisorConfig::default();
    config.restart_policy.strategy = RestartStrategy::OneForAll;

    let factory = Arc::new(TestAgentFactory);
    let mut supervisor = AgentSupervisor::new(config, factory);

    supervisor.start().await.unwrap();

    // Spawn multiple agents
    let spec1 = AgentSpec::new("test".to_string(), json!({}));
    let spec2 = AgentSpec::new("test".to_string(), json!({}));
    let spec3 = AgentSpec::new("test".to_string(), json!({}));

    let agent_id1 = supervisor.spawn_agent(spec1).await.unwrap();
    let agent_id2 = supervisor.spawn_agent(spec2).await.unwrap();
    let agent_id3 = supervisor.spawn_agent(spec3).await.unwrap();

    // Verify all agents are running
    let children = supervisor.get_children().await;
    assert_eq!(children.len(), 3);

    // Simulate failure of one agent
    supervisor
        .handle_agent_exit(
            agent_id2.clone(),
            ExitReason::Error("simulated failure".to_string()),
        )
        .await
        .unwrap();

    // Give some time for restart
    sleep(Duration::from_millis(100)).await;

    // Verify all agents are still present (all should have been restarted)
    let children = supervisor.get_children().await;
    assert_eq!(children.len(), 3);

    supervisor.stop().await.unwrap();
}

#[tokio::test]
async fn test_restart_intensity_limits() {
    env_logger::try_init().ok();

    // Create supervisor with strict restart policy (max 3 restarts)
    let mut config = SupervisorConfig::default();
    config.restart_policy = RestartPolicy::new(
        RestartStrategy::OneForOne,
        RestartIntensity::new(2, Duration::from_secs(60)), // Only 2 restarts allowed
    );

    let factory = Arc::new(TestAgentFactory);
    let mut supervisor = AgentSupervisor::new(config, factory);

    supervisor.start().await.unwrap();

    // Spawn an agent
    let spec = AgentSpec::new("test".to_string(), json!({}));
    let agent_id = supervisor.spawn_agent(spec).await.unwrap();

    // Simulate multiple failures
    for i in 1..=3 {
        let result = supervisor
            .handle_agent_exit(
                agent_id.clone(),
                ExitReason::Error(format!("failure {}", i)),
            )
            .await;

        if i <= 2 {
            // First two failures should succeed
            assert!(result.is_ok(), "Restart {} should succeed", i);
        } else {
            // Third failure should exceed limits
            assert!(result.is_err(), "Restart {} should fail due to limits", i);
        }

        sleep(Duration::from_millis(50)).await;
    }

    supervisor.stop().await.unwrap();
}

#[tokio::test]
async fn test_supervisor_configuration() {
    env_logger::try_init().ok();

    let mut config = SupervisorConfig::default();
    config.max_children = 2;
    config.agent_timeout = Duration::from_secs(5);
    config.health_check_interval = Duration::from_secs(30);

    let factory = Arc::new(TestAgentFactory);
    let mut supervisor = AgentSupervisor::new(config, factory);

    supervisor.start().await.unwrap();

    // Spawn agents up to the limit
    let spec1 = AgentSpec::new("test".to_string(), json!({}));
    let spec2 = AgentSpec::new("test".to_string(), json!({}));
    let spec3 = AgentSpec::new("test".to_string(), json!({}));

    let _agent_id1 = supervisor.spawn_agent(spec1).await.unwrap();
    let _agent_id2 = supervisor.spawn_agent(spec2).await.unwrap();

    // Third agent should fail due to max_children limit
    let result = supervisor.spawn_agent(spec3).await;
    assert!(result.is_err());

    supervisor.stop().await.unwrap();
}
