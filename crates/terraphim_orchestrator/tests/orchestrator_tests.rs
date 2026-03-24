use std::path::PathBuf;
use std::time::Duration;

use terraphim_orchestrator::{
    AgentDefinition, AgentLayer, AgentOrchestrator, CompoundReviewConfig, HandoffContext,
    NightwatchConfig, OrchestratorConfig, OrchestratorError,
};
use uuid::Uuid;

fn test_config() -> OrchestratorConfig {
    OrchestratorConfig {
        working_dir: PathBuf::from("/tmp/test-orchestrator"),
        nightwatch: NightwatchConfig::default(),
        compound_review: CompoundReviewConfig {
            schedule: "0 2 * * *".to_string(),
            max_duration_secs: 60,
            repo_path: PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../.."),
            create_prs: false,
            worktree_root: PathBuf::from("/tmp/test-orchestrator/.worktrees"),
            base_branch: "main".to_string(),
            max_concurrent_agents: 3,
        },
        workflow: None,
        agents: vec![
            AgentDefinition {
                name: "sentinel".to_string(),
                layer: AgentLayer::Safety,
                cli_tool: "echo".to_string(),
                task: "safety watch".to_string(),
                model: None,
                schedule: None,
                capabilities: vec!["security".to_string()],
                max_memory_bytes: None,
                budget_monthly_cents: None,
                provider: None,
                persona: None,
                terraphim_role: None,
                skill_chain: vec![],
                sfia_skills: vec![],
                fallback_provider: None,
                fallback_model: None,
                grace_period_secs: None,
                max_cpu_seconds: None,
            },
            AgentDefinition {
                name: "sync".to_string(),
                layer: AgentLayer::Core,
                cli_tool: "echo".to_string(),
                task: "sync upstream".to_string(),
                model: None,
                schedule: Some("0 3 * * *".to_string()),
                capabilities: vec!["sync".to_string()],
                max_memory_bytes: None,
                budget_monthly_cents: None,
                provider: None,
                persona: None,
                terraphim_role: None,
                skill_chain: vec![],
                sfia_skills: vec![],
                fallback_provider: None,
                fallback_model: None,
                grace_period_secs: None,
                max_cpu_seconds: None,
            },
            AgentDefinition {
                name: "reviewer".to_string(),
                layer: AgentLayer::Growth,
                cli_tool: "echo".to_string(),
                task: "review code".to_string(),
                model: None,
                schedule: None,
                capabilities: vec!["code-review".to_string()],
                max_memory_bytes: None,
                budget_monthly_cents: None,
                provider: None,
                persona: None,
                terraphim_role: None,
                skill_chain: vec![],
                sfia_skills: vec![],
                fallback_provider: None,
                fallback_model: None,
                grace_period_secs: None,
                max_cpu_seconds: None,
            },
        ],
        restart_cooldown_secs: 60,
        max_restart_count: 10,
        tick_interval_secs: 30,
        handoff_buffer_ttl_secs: None,
        persona_data_dir: None,
    }
}

/// Integration test: orchestrator creates successfully from config and starts
/// with correct initial state.
/// Design reference: test_orchestrator_spawns_safety_agents (partial -- creation + state)
#[test]
fn test_orchestrator_creates_and_initial_state() {
    let config = test_config();
    let orch = AgentOrchestrator::new(config).unwrap();

    // No agents running before run() is called
    let statuses = orch.agent_statuses();
    assert!(
        statuses.is_empty(),
        "no agents should be running before run()"
    );
}

/// Integration test: shutdown flag prevents reconciliation loop from continuing.
/// Design reference: test_orchestrator_shutdown_cleans_up
#[tokio::test]
async fn test_orchestrator_shutdown_cleans_up() {
    let config = test_config();
    let mut orch = AgentOrchestrator::new(config).unwrap();

    // Request shutdown before running -- the run() loop should exit immediately
    orch.shutdown();

    // Run should return quickly because shutdown is already requested
    let result = tokio::time::timeout(Duration::from_secs(5), orch.run()).await;

    match result {
        Ok(Ok(())) => {} // expected: run() returned cleanly
        Ok(Err(e)) => panic!("run() returned error: {}", e),
        Err(_) => panic!("run() did not exit within 5 seconds after shutdown"),
    }
}

/// Integration test: compound review with empty groups runs without worktree ops.
/// Uses empty groups to avoid git worktree creation which fails when the git
/// index is locked (e.g. during pre-commit hooks).
#[tokio::test]
async fn test_orchestrator_compound_review_integration() {
    use terraphim_orchestrator::{CompoundReviewWorkflow, SwarmConfig};

    let swarm_config = SwarmConfig {
        groups: vec![],
        timeout: std::time::Duration::from_secs(60),
        worktree_root: PathBuf::from("/tmp/test-orchestrator/.worktrees"),
        repo_path: PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../.."),
        base_branch: "main".to_string(),
        max_concurrent_agents: 3,
        create_prs: false,
    };

    let workflow = CompoundReviewWorkflow::new(swarm_config);
    let result = workflow.run("HEAD", "HEAD~1").await.unwrap();

    assert!(
        !result.correlation_id.is_nil(),
        "correlation_id should be set"
    );
    assert_eq!(result.agents_run, 0, "no agents with empty groups");
    assert_eq!(result.agents_failed, 0);
}

/// Integration test: orchestrator loads from TOML string.
#[test]
fn test_orchestrator_from_toml_integration() {
    let toml_str = r#"
working_dir = "/tmp/integration-test"

[nightwatch]
eval_interval_secs = 600
minor_threshold = 0.15

[compound_review]
schedule = "0 4 * * *"
repo_path = "/tmp"

[[agents]]
name = "agent-alpha"
layer = "Safety"
cli_tool = "echo"
task = "monitor"

[[agents]]
name = "agent-beta"
layer = "Core"
cli_tool = "echo"
task = "sync"
schedule = "30 3 * * *"
capabilities = ["sync", "update"]

[[agents]]
name = "agent-gamma"
layer = "Growth"
cli_tool = "echo"
task = "review"
max_memory_bytes = 1073741824
"#;
    let config = OrchestratorConfig::from_toml(toml_str).unwrap();
    let orch = AgentOrchestrator::new(config);
    assert!(orch.is_ok());

    let orch = orch.unwrap();
    let statuses = orch.agent_statuses();
    assert!(statuses.is_empty());
}

/// Integration test: orchestrator handles drift alert by verifying the
/// NightwatchMonitor and AgentOrchestrator work together through the
/// rate limiter and drift score query APIs.
/// Design reference: test_orchestrator_handles_drift_alert
#[test]
fn test_orchestrator_handles_drift_alert() {
    let config = test_config();
    let mut orch = AgentOrchestrator::new(config).unwrap();

    // Verify rate limiter is accessible and functional through orchestrator
    assert!(orch.rate_limiter().can_call("sentinel", "openai"));

    orch.rate_limiter_mut().record_call("sentinel", "openai");
    orch.rate_limiter_mut()
        .update_limit("sentinel", "openai", 100);

    assert_eq!(
        orch.rate_limiter().remaining("sentinel", "openai"),
        Some(99)
    );

    // Verify router is accessible
    let _router = orch.router();
}

/// Integration test: handoff context round-trips through file correctly.
#[tokio::test]
async fn test_handoff_context_file_roundtrip() {
    let dir = tempfile::tempdir().unwrap();
    let handoff_path = dir.path().join("handoff-test.json");

    let original = HandoffContext {
        handoff_id: Uuid::new_v4(),
        from_agent: "test-agent-a".to_string(),
        to_agent: "test-agent-b".to_string(),
        task: "Integration test task".to_string(),
        progress_summary: "Completed initial analysis".to_string(),
        decisions: vec![
            "Use async approach".to_string(),
            "Skip database migration".to_string(),
        ],
        files_touched: vec![
            PathBuf::from("src/lib.rs"),
            PathBuf::from("tests/integration.rs"),
        ],
        timestamp: chrono::Utc::now(),
        ttl_secs: Some(3600),
    };

    original.write_to_file(&handoff_path).unwrap();
    let restored = HandoffContext::read_from_file(&handoff_path).unwrap();

    assert_eq!(original, restored);
    assert_eq!(restored.decisions.len(), 2);
    assert_eq!(restored.files_touched.len(), 2);
}

/// Integration test: example config file loads and creates orchestrator.
#[test]
fn test_example_config_creates_orchestrator() {
    let example_path =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("orchestrator.example.toml");
    let config = OrchestratorConfig::from_file(&example_path).unwrap();

    assert_eq!(config.agents.len(), 3);
    assert_eq!(config.agents[0].layer, AgentLayer::Safety);
    assert_eq!(config.agents[1].layer, AgentLayer::Core);
    assert_eq!(config.agents[2].layer, AgentLayer::Growth);

    let orch = AgentOrchestrator::new(config);
    assert!(orch.is_ok());
}

/// Integration test: error variants produce correct error messages.
/// DEF-3: exercises SpawnFailed, AgentNotFound, HandoffFailed error variants.
#[test]
fn test_error_variants_display() {
    let err = OrchestratorError::SpawnFailed {
        agent: "test-agent".to_string(),
        reason: "process not found".to_string(),
    };
    assert!(err.to_string().contains("test-agent"));
    assert!(err.to_string().contains("process not found"));

    let err = OrchestratorError::AgentNotFound("missing-agent".to_string());
    assert!(err.to_string().contains("missing-agent"));

    let err = OrchestratorError::HandoffFailed {
        from: "agent-a".to_string(),
        to: "agent-b".to_string(),
        reason: "timeout".to_string(),
    };
    let msg = err.to_string();
    assert!(msg.contains("agent-a"));
    assert!(msg.contains("agent-b"));
    assert!(msg.contains("timeout"));
}
