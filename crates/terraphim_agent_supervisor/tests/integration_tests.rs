//! Integration tests for the supervision system

use std::sync::Arc;
use std::time::Duration;

use serde_json::json;
use tokio::time::sleep;

use terraphim_agent_supervisor::{
    AgentSpec, AgentSupervisor, ExitReason, RestartIntensity, RestartPolicy, RestartStrategy,
    SupervisorConfig, SupervisorStatus, TestAgentFactory,
};

/// Regression test for #252: RestForOne must restart only agents started after the failed one.
/// Previously sort_by_key used agent_id (UUID) which gave arbitrary order; now uses start_time.
#[tokio::test]
async fn rest_for_one_respects_start_order() {
    env_logger::try_init().ok();

    let mut config = SupervisorConfig::default();
    config.restart_policy.strategy = RestartStrategy::RestForOne;

    let factory = Arc::new(TestAgentFactory);
    let mut supervisor = AgentSupervisor::new(config, factory);
    supervisor.start().await.unwrap();

    // Spawn three agents in sequence; start_time ordering: A < B < C.
    let agent_a = supervisor
        .spawn_agent(AgentSpec::new("test".to_string(), json!({})))
        .await
        .unwrap();
    // Brief sleep so system clock advances between spawns.
    sleep(Duration::from_millis(10)).await;
    let agent_b = supervisor
        .spawn_agent(AgentSpec::new("test".to_string(), json!({})))
        .await
        .unwrap();
    sleep(Duration::from_millis(10)).await;
    let agent_c = supervisor
        .spawn_agent(AgentSpec::new("test".to_string(), json!({})))
        .await
        .unwrap();

    // Capture start_times before the failure so we can compare afterwards.
    let start_times_before: std::collections::HashMap<_, _> = supervisor
        .get_children()
        .await
        .into_iter()
        .map(|(pid, info)| (pid, info.start_time))
        .collect();
    assert_eq!(start_times_before.len(), 3);

    // Agent B fails. RestForOne must restart B and C (started after A) but leave A untouched.
    supervisor
        .handle_agent_exit(agent_b.clone(), ExitReason::Error("b failed".to_string()))
        .await
        .unwrap();

    sleep(Duration::from_millis(100)).await;

    let children_after = supervisor.get_children().await;
    assert_eq!(children_after.len(), 3, "all three agents must still exist");

    // A must not have been restarted: its start_time must be unchanged.
    let a_start_after = children_after
        .get(&agent_a)
        .expect("agent A must still exist")
        .start_time;
    assert_eq!(
        a_start_after,
        *start_times_before.get(&agent_a).unwrap(),
        "agent A (started before B) must not be restarted by RestForOne"
    );

    // B and C must have been re-spawned: their start_times must be >= their original values.
    // (spawn_agent creates a new SupervisedAgentInfo with Utc::now() as start_time)
    let b_start_after = children_after
        .get(&agent_b)
        .expect("agent B must still exist")
        .start_time;
    assert!(
        b_start_after >= *start_times_before.get(&agent_b).unwrap(),
        "agent B must have been re-spawned"
    );

    let c_start_after = children_after
        .get(&agent_c)
        .expect("agent C must still exist")
        .start_time;
    assert!(
        c_start_after >= *start_times_before.get(&agent_c).unwrap(),
        "agent C must have been re-spawned"
    );

    supervisor.stop().await.unwrap();
}

/// Regression test for #255: supervisor must permanently set `escalated = true` when max
/// restarts are exceeded, and all subsequent restart attempts must be rejected.
#[tokio::test]
async fn escalated_flag_prevents_further_restarts() {
    env_logger::try_init().ok();

    // Allow only 1 restart within a long window.
    let config = SupervisorConfig {
        restart_policy: RestartPolicy::new(
            RestartStrategy::OneForOne,
            RestartIntensity::new(1, Duration::from_secs(120)),
        ),
        ..Default::default()
    };

    let factory = Arc::new(TestAgentFactory);
    let mut supervisor = AgentSupervisor::new(config, factory);
    supervisor.start().await.unwrap();

    assert!(
        !supervisor.is_escalated(),
        "supervisor must start un-escalated"
    );

    let spec = AgentSpec::new("test".to_string(), json!({}));
    let agent_id = supervisor.spawn_agent(spec).await.unwrap();

    // First failure: restart_count goes from 0 -> 1, which equals max_restarts.
    // The *next* failure should trigger escalation.
    supervisor
        .handle_agent_exit(agent_id.clone(), ExitReason::Error("first".to_string()))
        .await
        .unwrap();
    sleep(Duration::from_millis(50)).await;
    assert!(
        !supervisor.is_escalated(),
        "not yet escalated after first failure"
    );

    // Second failure: restart_count is now 1, which exceeds max_restarts (1) -> escalate.
    let result = supervisor
        .handle_agent_exit(agent_id.clone(), ExitReason::Error("second".to_string()))
        .await;
    assert!(result.is_err(), "second failure must be rejected");
    assert!(
        supervisor.is_escalated(),
        "supervisor must be escalated now"
    );

    // Third attempt on a completely different agent must also be rejected.
    let spec2 = AgentSpec::new("test".to_string(), json!({}));
    let agent_id2 = supervisor.spawn_agent(spec2).await.unwrap();
    let result2 = supervisor
        .handle_agent_exit(agent_id2, ExitReason::Error("third".to_string()))
        .await;
    assert!(
        result2.is_err(),
        "escalated supervisor must reject all restarts"
    );

    supervisor.stop().await.unwrap();
}

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
    let config = SupervisorConfig {
        restart_policy: RestartPolicy::lenient_one_for_one(),
        ..Default::default()
    };

    let factory = Arc::new(TestAgentFactory);
    let mut supervisor = AgentSupervisor::new(config, factory);

    supervisor.start().await.unwrap();

    // Spawn an agent
    let spec = AgentSpec::new("test".to_string(), json!({}));
    let agent_id = supervisor.spawn_agent(spec).await.unwrap();

    // Verify agent is running
    let children = supervisor.get_children().await;
    assert_eq!(children.len(), 1);
    let _original_start_time = children.get(&agent_id).unwrap().start_time;

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

    let _agent_id1 = supervisor.spawn_agent(spec1).await.unwrap();
    let agent_id2 = supervisor.spawn_agent(spec2).await.unwrap();
    let _agent_id3 = supervisor.spawn_agent(spec3).await.unwrap();

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
    let config = SupervisorConfig {
        restart_policy: RestartPolicy::new(
            RestartStrategy::OneForOne,
            RestartIntensity::new(2, Duration::from_secs(60)), // Only 2 restarts allowed
        ),
        ..Default::default()
    };

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

    let config = SupervisorConfig {
        max_children: 2,
        agent_timeout: Duration::from_secs(5),
        health_check_interval: Duration::from_secs(30),
        ..Default::default()
    };

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
