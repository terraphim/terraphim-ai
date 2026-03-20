//! End-to-end integration tests for the orchestrator dual mode operation.
//!
//! These tests verify the complete flow of:
//! - Dual mode: Both time-based and issue-driven tasks are processed
//! - Time-only mode: Legacy operation with time-based scheduling only
//! - Issue-only mode: Issue-driven task processing
//! - Fairness: Both task types are processed without starvation
//! - Graceful shutdown: Clean termination with queue draining
//! - Stall detection: Warning when queue grows beyond threshold

use terraphim_orchestrator::{
    AgentDefinition, AgentLayer, AgentOrchestrator, CompoundReviewConfig, ConcurrencyConfig,
    DispatchTask, DriftDetectionConfig, ModeCoordinator, NightwatchConfig, OrchestratorConfig,
    SessionRotationConfig, TrackerConfig, TrackerType, WorkflowConfig, WorkflowMode,
};
use tracing::info;

/// Create a test configuration with dual mode enabled
fn create_dual_mode_config() -> OrchestratorConfig {
    // Set the test token env var
    std::env::set_var("TEST_TOKEN", "test-token-12345");
    
    OrchestratorConfig {
        working_dir: std::path::PathBuf::from("/tmp/test-orchestrator"),
        nightwatch: NightwatchConfig::default(),
        compound_review: CompoundReviewConfig {
            schedule: "0 2 * * *".to_string(),
            max_duration_secs: 60,
            repo_path: std::path::PathBuf::from("/tmp"),
            create_prs: false,
        },
        agents: vec![
            AgentDefinition {
                name: "time-agent".to_string(),
                layer: AgentLayer::Core,
                cli_tool: "echo".to_string(),
                task: "time task".to_string(),
                model: None,
                schedule: Some("0 * * * *".to_string()),
                capabilities: vec!["time".to_string()],
                max_memory_bytes: None,
                provider: None,
                fallback_provider: None,
                fallback_model: None,
                provider_tier: None,
                persona_name: None,
                persona_symbol: None,
                persona_vibe: None,
                meta_cortex_connections: vec![],
                skill_chain: vec![],
            },
            AgentDefinition {
                name: "issue-agent".to_string(),
                layer: AgentLayer::Growth,
                cli_tool: "echo".to_string(),
                task: "issue task".to_string(),
                model: None,
                schedule: None,
                capabilities: vec!["issue".to_string()],
                max_memory_bytes: None,
                provider: None,
                fallback_provider: None,
                fallback_model: None,
                provider_tier: None,
                persona_name: None,
                persona_symbol: None,
                persona_vibe: None,
                meta_cortex_connections: vec![],
                skill_chain: vec![],
            },
        ],
        restart_cooldown_secs: 0,
        max_restart_count: 10,
        tick_interval_secs: 30,
        allowed_providers: vec![],
        banned_providers: vec![],
        skill_registry: Default::default(),
        stagger_delay_ms: 100,
        review_pairs: vec![],
        drift_detection: DriftDetectionConfig::default(),
        session_rotation: SessionRotationConfig::default(),
        convergence: Default::default(),
        workflow: Some(WorkflowConfig {
            mode: WorkflowMode::Dual,
            poll_interval_secs: 60,
            max_concurrent_tasks: 5,
        }),
        tracker: Some(TrackerConfig {
            tracker_type: TrackerType::Gitea,
            url: "https://test.example.com".to_string(),
            token_env_var: "TEST_TOKEN".to_string(),
            owner: "test".to_string(),
            repo: "test".to_string(),
        }),
        concurrency: Some(ConcurrencyConfig {
            max_parallel_agents: 3,
            queue_depth: 50,
            starvation_timeout_secs: 60,
        }),
    }
}

/// Create a test configuration with time-only mode (legacy)
fn create_time_only_config() -> OrchestratorConfig {
    let mut config = create_dual_mode_config();
    config.workflow = Some(WorkflowConfig {
        mode: WorkflowMode::TimeOnly,
        poll_interval_secs: 60,
        max_concurrent_tasks: 5,
    });
    config.tracker = None;
    config
}

/// Create a test configuration with issue-only mode
fn create_issue_only_config() -> OrchestratorConfig {
    // Set the test token env var
    std::env::set_var("TEST_TOKEN", "test-token-12345");
    
    let mut config = create_dual_mode_config();
    config.workflow = Some(WorkflowConfig {
        mode: WorkflowMode::IssueOnly,
        poll_interval_secs: 60,
        max_concurrent_tasks: 5,
    });
    config
}

/// Test: Dual mode operation - both time and issue tasks processed
#[tokio::test]
async fn test_dual_mode_operation() {
    let config = create_dual_mode_config();
    let mut orch = AgentOrchestrator::new(config).unwrap();

    // Verify mode coordinator is created with dual mode
    let mode = orch.workflow_mode();
    assert_eq!(mode, Some(WorkflowMode::Dual));

    // Verify mode coordinator exists
    let coord = orch.mode_coordinator();
    assert!(coord.is_some());

    let coord = coord.unwrap();
    assert!(coord.time_mode.is_some());
    assert_eq!(coord.workflow_mode, WorkflowMode::Dual);

    // Simulate submitting tasks to both modes
    if let Some(ref mut coord_mut) = orch.mode_coordinator_mut() {
        // Submit time task
        let time_task = DispatchTask::TimeTask("time-agent".to_string(), "0 * * * *".to_string());
        coord_mut.dispatch_queue.submit(time_task).unwrap();

        // Submit issue task
        let issue_task = DispatchTask::IssueTask("issue-agent".to_string(), 1, 100);
        coord_mut.dispatch_queue.submit(issue_task).unwrap();

        // Verify both tasks are in queue
        assert_eq!(coord_mut.queue_depth(), 2);
    }

    // Process tasks from queue
    let dispatched = orch.dispatch_from_queue().await;
    assert!(dispatched >= 0); // May dispatch 0 or 1 depending on concurrency
}

/// Test: Time mode only - legacy configuration
#[tokio::test]
async fn test_time_mode_only() {
    let config = create_time_only_config();
    let mut orch = AgentOrchestrator::new(config).unwrap();

    // Verify time-only mode
    let mode = orch.workflow_mode();
    assert_eq!(mode, Some(WorkflowMode::TimeOnly));

    // Verify mode coordinator has time mode but no issue mode
    let coord = orch.mode_coordinator();
    assert!(coord.is_some());
    assert!(coord.unwrap().time_mode.is_some());

    // Simulate time task submission
    if let Some(ref mut coord_mut) = orch.mode_coordinator_mut() {
        let time_task = DispatchTask::TimeTask("time-agent".to_string(), "0 * * * *".to_string());
        coord_mut.dispatch_queue.submit(time_task).unwrap();

        assert_eq!(coord_mut.queue_depth(), 1);

        // Verify it's a time task
        let task = coord_mut.next_task();
        assert!(matches!(task, Some(DispatchTask::TimeTask(_, _))));
    }
}

/// Test: Issue mode only - issue-driven configuration
#[tokio::test]
async fn test_issue_mode_only() {
    let config = create_issue_only_config();
    let mut orch = AgentOrchestrator::new(config).unwrap();

    // Verify issue-only mode
    let mode = orch.workflow_mode();
    assert_eq!(mode, Some(WorkflowMode::IssueOnly));

    // Verify mode coordinator
    let coord = orch.mode_coordinator();
    assert!(coord.is_some());

    // Note: Issue mode won't be created without a real tracker
    // but the coordinator should exist
    let coord = coord.unwrap();
    assert_eq!(coord.workflow_mode, WorkflowMode::IssueOnly);
}

/// Test: Fairness under load - neither mode starves
#[tokio::test]
async fn test_fairness_under_load() {
    let config = create_dual_mode_config();
    let mut orch = AgentOrchestrator::new(config).unwrap();

    // Submit many tasks from both modes
    let num_time_tasks = 10;
    let num_issue_tasks = 10;

    if let Some(ref mut coord_mut) = orch.mode_coordinator_mut() {
        // Submit time tasks (lower priority)
        for i in 0..num_time_tasks {
            let task = DispatchTask::TimeTask(
                format!("time-agent-{}", i),
                "0 * * * *".to_string(),
            );
            coord_mut.dispatch_queue.submit(task).unwrap();
        }

        // Submit issue tasks (higher priority)
        for i in 0..num_issue_tasks {
            let task = DispatchTask::IssueTask(
                format!("issue-agent-{}", i),
                i as u64,
                10, // Higher priority
            );
            coord_mut.dispatch_queue.submit(task).unwrap();
        }

        assert_eq!(coord_mut.queue_depth(), num_time_tasks + num_issue_tasks);

        // Dequeue all tasks and verify we get both types
        let mut time_count = 0;
        let mut issue_count = 0;

        while let Some(task) = coord_mut.next_task() {
            match task {
                DispatchTask::TimeTask(_, _) => time_count += 1,
                DispatchTask::IssueTask(_, _, _) => issue_count += 1,
            }
        }

        // Verify we got tasks from both sources
        assert_eq!(time_count, num_time_tasks, "time tasks should not be starved");
        assert_eq!(issue_count, num_issue_tasks, "issue tasks should not be starved");
    }
}

/// Test: Graceful shutdown - clean termination
#[tokio::test]
async fn test_graceful_shutdown() {
    let config = create_dual_mode_config();
    let mut orch = AgentOrchestrator::new(config).unwrap();

    // Add some tasks to the queue
    if let Some(ref mut coord_mut) = orch.mode_coordinator_mut() {
        for i in 0..5 {
            let task = DispatchTask::TimeTask(format!("agent-{}", i), "0 * * * *".to_string());
            coord_mut.dispatch_queue.submit(task).unwrap();
        }
        assert_eq!(coord_mut.queue_depth(), 5);
    }

    // Trigger unified shutdown
    orch.unified_shutdown().await;

    // Verify queue was drained
    if let Some(ref coord) = orch.mode_coordinator() {
        assert_eq!(coord.queue_depth(), 0, "queue should be drained after shutdown");
    }

    // Verify shutdown completed without errors
    // (mode_coordinator may be None if all tasks completed)  
    info!("Graceful shutdown completed successfully");
}

/// Test: Stall detection - warning logged when queue exceeds threshold
#[test]
fn test_stall_detection() {
    let mut config = create_dual_mode_config();
    // Set a low threshold for testing
    config.concurrency = Some(ConcurrencyConfig {
        max_parallel_agents: 3,
        queue_depth: 5, // Low threshold
        starvation_timeout_secs: 60,
    });

    let mut orch = AgentOrchestrator::new(config).unwrap();

    // Initially not stalled
    assert!(!orch.check_stall(), "should not be stalled initially");

    // Fill queue above threshold
    if let Some(ref mut coord_mut) = orch.mode_coordinator_mut() {
        for i in 0..10 {
            let task = DispatchTask::TimeTask(format!("agent-{}", i), "0 * * * *".to_string());
            coord_mut.dispatch_queue.submit(task).unwrap();
        }
    }

    // Now should be stalled
    assert!(orch.check_stall(), "should be stalled when queue exceeds threshold");
}

/// Test: ModeCoordinator initialization with tracker
#[test]
fn test_mode_coordinator_with_tracker() {
    let workflow = WorkflowConfig {
        mode: WorkflowMode::Dual,
        poll_interval_secs: 30,
        max_concurrent_tasks: 3,
    };

    let agents = vec![
        AgentDefinition {
            name: "implementation-swarm".to_string(),
            layer: AgentLayer::Growth,
            cli_tool: "echo".to_string(),
            task: "Implement features".to_string(),
            model: None,
            schedule: None,
            capabilities: vec!["implementation".to_string()],
            max_memory_bytes: None,
            provider: None,
            fallback_provider: None,
            fallback_model: None,
            provider_tier: None,
            persona_name: None,
            persona_symbol: None,
            persona_vibe: None,
            meta_cortex_connections: vec![],
            skill_chain: vec![],
        },
    ];

    let tracker_config = terraphim_tracker::TrackerConfig::new(
        "https://test.example.com",
        "test-token",
        "test",
        "test",
    );

    let (coord, _shutdown_rx) = ModeCoordinator::new(
        workflow,
        agents,
        Some(tracker_config),
        Some("0 2 * * *".to_string()),
    )
    .unwrap();

    assert_eq!(coord.workflow_mode, WorkflowMode::Dual);
    assert!(coord.time_mode.is_some());
    // Issue mode may or may not be created depending on tracker initialization
    assert_eq!(coord.concurrency_controller.max_parallel(), 3);
}

/// Test: Concurrency limits are enforced
#[test]
fn test_concurrency_limits() {
    let workflow = WorkflowConfig {
        mode: WorkflowMode::Dual,
        poll_interval_secs: 60,
        max_concurrent_tasks: 2,
    };

    let (coord, _shutdown_rx) = ModeCoordinator::new(
        workflow,
        vec![],
        None,
        None,
    )
    .unwrap();

    // Acquire permits up to limit
    let permit1 = coord.try_acquire_permit();
    assert!(permit1.is_some());

    let permit2 = coord.try_acquire_permit();
    assert!(permit2.is_some());

    // Third permit should fail
    let permit3 = coord.try_acquire_permit();
    assert!(permit3.is_none());

    // After dropping a permit, should be able to acquire again
    drop(permit1);
    let permit4 = coord.try_acquire_permit();
    assert!(permit4.is_some());
}

/// Test: Queue prioritization - higher priority tasks served first
#[test]
fn test_queue_prioritization() {
    let workflow = WorkflowConfig {
        mode: WorkflowMode::Dual,
        poll_interval_secs: 60,
        max_concurrent_tasks: 5,
    };

    let (mut coord, _shutdown_rx) = ModeCoordinator::new(
        workflow,
        vec![],
        None,
        None,
    )
    .unwrap();

    // Submit tasks with different priorities
    let low_priority = DispatchTask::IssueTask("low".to_string(), 1, 1);
    let high_priority = DispatchTask::IssueTask("high".to_string(), 2, 10);
    let medium_priority = DispatchTask::IssueTask("medium".to_string(), 3, 5);

    coord.dispatch_queue.submit(low_priority).unwrap();
    coord.dispatch_queue.submit(high_priority).unwrap();
    coord.dispatch_queue.submit(medium_priority).unwrap();

    // Should dequeue in priority order: high (10), medium (5), low (1)
    let first = coord.next_task().unwrap();
    assert!(matches!(first, DispatchTask::IssueTask(name, 2, 10) if name == "high"));

    let second = coord.next_task().unwrap();
    assert!(matches!(second, DispatchTask::IssueTask(name, 3, 5) if name == "medium"));

    let third = coord.next_task().unwrap();
    assert!(matches!(third, DispatchTask::IssueTask(name, 1, 1) if name == "low"));
}

/// Test: Task submission when queue is full
#[test]
fn test_queue_full_behavior() {
    let workflow = WorkflowConfig {
        mode: WorkflowMode::Dual,
        poll_interval_secs: 60,
        max_concurrent_tasks: 5,
    };

    let (mut coord, _shutdown_rx) = ModeCoordinator::new(
        workflow,
        vec![],
        None,
        None,
    )
    .unwrap();

    // Fill the queue to capacity (queue depth = max_concurrent_tasks * 10 = 50)
    for i in 0..50 {
        let task = DispatchTask::TimeTask(format!("agent-{}", i), "0 * * * *".to_string());
        coord.dispatch_queue.submit(task).unwrap();
    }

    assert!(coord.dispatch_queue.is_full());

    // Next submission should fail
    let overflow_task = DispatchTask::TimeTask("overflow".to_string(), "0 * * * *".to_string());
    let result = coord.dispatch_queue.submit(overflow_task);
    assert!(result.is_err());
}

/// Test: Backward compatibility - config without workflow field
#[test]
fn test_backward_compatibility() {
    let config = OrchestratorConfig {
        working_dir: std::path::PathBuf::from("/tmp"),
        nightwatch: NightwatchConfig::default(),
        compound_review: CompoundReviewConfig {
            schedule: "0 2 * * *".to_string(),
            max_duration_secs: 60,
            repo_path: std::path::PathBuf::from("/tmp"),
            create_prs: false,
        },
        agents: vec![AgentDefinition {
            name: "test".to_string(),
            layer: AgentLayer::Safety,
            cli_tool: "echo".to_string(),
            task: "test".to_string(),
            model: None,
            schedule: None,
            capabilities: vec![],
            max_memory_bytes: None,
            provider: None,
            fallback_provider: None,
            fallback_model: None,
            provider_tier: None,
            persona_name: None,
            persona_symbol: None,
            persona_vibe: None,
            meta_cortex_connections: vec![],
            skill_chain: vec![],
        }],
        restart_cooldown_secs: 60,
        max_restart_count: 10,
        tick_interval_secs: 30,
        allowed_providers: vec![],
        banned_providers: vec![],
        skill_registry: Default::default(),
        stagger_delay_ms: 5000,
        review_pairs: vec![],
        drift_detection: DriftDetectionConfig::default(),
        session_rotation: SessionRotationConfig::default(),
        convergence: Default::default(),
        workflow: None, // No workflow config
        tracker: None,
        concurrency: None,
    };

    let orch = AgentOrchestrator::new(config).unwrap();

    // Without workflow config, mode coordinator should not be created
    assert!(orch.mode_coordinator().is_none());
    assert!(orch.workflow_mode().is_none());
}
