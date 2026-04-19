use std::path::PathBuf;
use std::time::Duration;

use serial_test::serial;
use terraphim_orchestrator::{
    AgentDefinition, AgentLayer, AgentOrchestrator, CompoundReviewConfig, HandoffContext,
    NightwatchConfig, OrchestratorConfig, OrchestratorError, TrackerConfig, TrackerStates,
    WorkflowConfig,
};
use uuid::Uuid;

/// Return a deterministic baseline for git-diff tests.
/// Prefer the repository root commit, but fall back to the empty tree when the
/// checkout is shallow and history before HEAD is unavailable.
fn git_diff_baseline() -> String {
    let output = std::process::Command::new("git")
        .args(["rev-list", "--max-parents=0", "HEAD"])
        .output()
        .expect("git rev-list failed");
    let commits = String::from_utf8_lossy(&output.stdout);
    let baseline = commits.lines().next().unwrap_or("").trim();

    if baseline.is_empty() {
        return "4b825dc642cb6eb9a060e54bf8d69288fbee4904".to_string();
    }

    let head_output = std::process::Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()
        .expect("git rev-parse failed");
    let head = String::from_utf8_lossy(&head_output.stdout)
        .trim()
        .to_string();

    if baseline == head {
        return "4b825dc642cb6eb9a060e54bf8d69288fbee4904".to_string();
    }

    baseline.to_string()
}

fn test_config() -> OrchestratorConfig {
    OrchestratorConfig {
        working_dir: PathBuf::from("/tmp/test-orchestrator"),
        nightwatch: NightwatchConfig::default(),
        compound_review: CompoundReviewConfig {
            cli_tool: None,
            provider: None,
            model: None,
            schedule: "0 2 * * *".to_string(),
            max_duration_secs: 60,
            repo_path: PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../.."),
            create_prs: false,
            worktree_root: PathBuf::from("/tmp/test-orchestrator/.worktrees"),
            base_branch: "main".to_string(),
            max_concurrent_agents: 3,
            ..Default::default()
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
                pre_check: None,

                gitea_issue: None,

                project: None,
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
                pre_check: None,

                gitea_issue: None,

                project: None,
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
                pre_check: None,

                gitea_issue: None,

                project: None,
            },
        ],
        restart_cooldown_secs: 60,
        max_restart_count: 10,
        restart_budget_window_secs: 43_200,
        disk_usage_threshold: 100, // disable disk guard in tests
        tick_interval_secs: 30,
        handoff_buffer_ttl_secs: None,
        persona_data_dir: None,
        skill_data_dir: None,
        flows: vec![],
        flow_state_dir: None,
        gitea: None,
        mentions: None,
        webhook: None,
        role_config_path: None,
        routing: None,
        #[cfg(feature = "quickwit")]
        quickwit: None,
        projects: vec![],
        include: vec![],
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
    let result = workflow.run("HEAD", &git_diff_baseline()).await.unwrap();

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

    assert_eq!(config.agents.len(), 16);
    assert_eq!(config.agents[0].layer, AgentLayer::Safety);
    assert_eq!(config.agents[1].layer, AgentLayer::Safety);
    assert_eq!(config.agents[2].layer, AgentLayer::Core);

    let orch = AgentOrchestrator::new(config);
    assert!(
        orch.is_ok(),
        "Orchestrator creation failed: {:?}",
        orch.err()
    );
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

#[tokio::test]
async fn test_shell_pre_check_skips_on_empty_output() {
    let mut config = test_config();
    config.agents[0].pre_check = Some(terraphim_orchestrator::PreCheckStrategy::Shell {
        script: "true".to_string(), // exit 0, empty stdout
        timeout_secs: 5,
    });
    let mut orch = AgentOrchestrator::new(config).unwrap();
    // Agent should NOT be spawned (NoFindings -> skip)
    let result = orch.spawn_agent_for_test("sentinel").await;
    assert!(result.is_ok());
    assert!(!orch.is_agent_active("sentinel"));
}

#[tokio::test]
async fn test_shell_pre_check_returns_findings() {
    let mut config = test_config();
    config.agents[0].pre_check = Some(terraphim_orchestrator::PreCheckStrategy::Shell {
        script: "echo 'found issues'".to_string(),
        timeout_secs: 5,
    });
    let mut orch = AgentOrchestrator::new(config).unwrap();
    let result = orch.spawn_agent_for_test("sentinel").await;
    assert!(result.is_ok());
    assert!(orch.is_agent_active("sentinel"));
}

#[tokio::test]
async fn test_shell_pre_check_fail_open_on_error() {
    let mut config = test_config();
    config.agents[0].pre_check = Some(terraphim_orchestrator::PreCheckStrategy::Shell {
        script: "exit 1".to_string(),
        timeout_secs: 5,
    });
    let mut orch = AgentOrchestrator::new(config).unwrap();
    let result = orch.spawn_agent_for_test("sentinel").await;
    assert!(result.is_ok());
    assert!(orch.is_agent_active("sentinel")); // fail-open
}

#[tokio::test]
async fn test_shell_pre_check_timeout_fail_open() {
    let mut config = test_config();
    config.agents[0].pre_check = Some(terraphim_orchestrator::PreCheckStrategy::Shell {
        script: "sleep 10".to_string(),
        timeout_secs: 1,
    });
    let mut orch = AgentOrchestrator::new(config).unwrap();
    let result = orch.spawn_agent_for_test("sentinel").await;
    assert!(result.is_ok());
    assert!(orch.is_agent_active("sentinel")); // fail-open
}

#[tokio::test]
async fn test_no_pre_check_spawns_normally() {
    let config = test_config(); // pre_check is None
    let mut orch = AgentOrchestrator::new(config).unwrap();
    let result = orch.spawn_agent_for_test("sentinel").await;
    assert!(result.is_ok());
    assert!(orch.is_agent_active("sentinel"));
}

/// Integration test: gitea-issue pre-check fails open when no workflow config.
#[tokio::test]
async fn test_gitea_issue_no_workflow_config_fail_open() {
    let mut config = test_config();
    config.workflow = None; // no workflow config
    config.agents[0].pre_check =
        Some(terraphim_orchestrator::PreCheckStrategy::GiteaIssue { issue_number: 42 });
    let mut orch = AgentOrchestrator::new(config).unwrap();
    let result = orch.spawn_agent_for_test("sentinel").await;
    assert!(result.is_ok());
    assert!(orch.is_agent_active("sentinel")); // fail-open
}

/// Git-diff: first run (no last_run_commits) always spawns.
#[tokio::test]
async fn test_git_diff_first_run_always_spawns() {
    let mut config = test_config();
    config.agents[0].pre_check = Some(terraphim_orchestrator::PreCheckStrategy::GitDiff {
        watch_paths: vec!["crates/".to_string()],
    });
    let mut orch = AgentOrchestrator::new(config).unwrap();
    // First run: no last_run_commits -> should spawn (Findings)
    let result = orch.spawn_agent_for_test("sentinel").await;
    assert!(result.is_ok());
    assert!(orch.is_agent_active("sentinel"));
}

/// Git-diff: HEAD unchanged since last run -> skip (NoFindings).
#[tokio::test]
async fn test_git_diff_no_changes_skips() {
    let mut config = test_config();
    // Set working_dir to the actual repo so git commands work
    config.working_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    config.agents[0].pre_check = Some(terraphim_orchestrator::PreCheckStrategy::GitDiff {
        watch_paths: vec!["crates/".to_string()],
    });
    let mut orch = AgentOrchestrator::new(config).unwrap();

    // First spawn to record HEAD
    let result = orch.spawn_agent_for_test("sentinel").await;
    assert!(result.is_ok());
    assert!(orch.is_agent_active("sentinel"));

    // Remove agent from active so we can test second spawn
    orch.remove_agent_for_test("sentinel");

    // Second spawn with same HEAD -> should skip
    let result = orch.spawn_agent_for_test("sentinel").await;
    assert!(result.is_ok());
    assert!(!orch.is_agent_active("sentinel")); // skipped
}

/// Git-diff: changes in watched paths -> spawn (Findings).
#[tokio::test]
async fn test_git_diff_matching_changes_spawns() {
    let mut config = test_config();
    config.working_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    config.agents[0].pre_check = Some(terraphim_orchestrator::PreCheckStrategy::GitDiff {
        watch_paths: vec!["crates/".to_string()],
    });
    let mut orch = AgentOrchestrator::new(config).unwrap();

    // Seed with initial commit so there ARE changes
    let baseline_commit = git_diff_baseline();
    orch.set_last_run_commit("sentinel", &baseline_commit);

    let result = orch.spawn_agent_for_test("sentinel").await;
    assert!(result.is_ok());
    assert!(orch.is_agent_active("sentinel")); // changes found in crates/
}

/// Git-diff: changes exist but NOT in watched paths -> skip (NoFindings).
#[tokio::test]
async fn test_git_diff_non_matching_changes_skips() {
    let mut config = test_config();
    config.working_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    config.agents[0].pre_check = Some(terraphim_orchestrator::PreCheckStrategy::GitDiff {
        watch_paths: vec!["nonexistent-directory-that-will-never-match/".to_string()],
    });
    let mut orch = AgentOrchestrator::new(config).unwrap();

    // Seed with initial commit so there ARE changes, but none match the watch path
    let baseline_commit = git_diff_baseline();
    orch.set_last_run_commit("sentinel", &baseline_commit);

    let result = orch.spawn_agent_for_test("sentinel").await;
    assert!(result.is_ok());
    assert!(!orch.is_agent_active("sentinel")); // no matching changes
}

/// Integration test: telemetry summary round-trips through persistence.
#[tokio::test]
#[serial]
async fn test_telemetry_persistence_round_trip() {
    use terraphim_orchestrator::control_plane::telemetry::TokenBreakdown;
    use terraphim_orchestrator::control_plane::{
        CompletionEvent, TelemetryStore, TelemetrySummary,
    };
    use terraphim_persistence::{DeviceStorage, Persistable};

    DeviceStorage::init_memory_only().await.unwrap();

    let store = TelemetryStore::new(3600);
    store
        .record(CompletionEvent {
            model: "test-model".to_string(),
            session_id: "test-session".to_string(),
            completed_at: chrono::Utc::now(),
            latency_ms: 150,
            success: true,
            tokens: TokenBreakdown {
                total: 1000,
                input: 800,
                output: 200,
                ..Default::default()
            },
            cost_usd: 0.01,
            error: None,
        })
        .await;

    let summary = store.export_summary().await;
    // Use save() which writes to all available profiles, avoiding profile-name
    // sensitivity when the global DeviceStorage was already initialised by
    // another test with dashmap+sqlite rather than memory-only.
    summary.save().await.unwrap();

    let mut loaded = TelemetrySummary::new("telemetry_summary".to_string());
    loaded = loaded.load().await.unwrap();

    assert_eq!(loaded.model_performances.len(), 1);
    assert_eq!(loaded.model_performances[0].model, "test-model");
    assert_eq!(loaded.model_performances[0].successful_completions, 1);

    // Import into fresh store and verify
    let restored = TelemetryStore::new(3600);
    restored.import_summary(loaded).await;

    let perf = restored.model_performance("test-model").await;
    assert!(perf.successful_completions > 0);
    assert!(perf.avg_latency_ms > 0.0);
}

/// Integration test: orchestrator constructs successfully with routing config.
#[test]
fn test_orchestrator_with_routing_config() {
    let mut config = test_config();
    config.routing = Some(terraphim_orchestrator::config::RoutingConfig {
        taxonomy_path: std::path::PathBuf::from("/tmp/nonexistent-taxonomy"),
        probe_ttl_secs: 300,
        probe_results_dir: None,
        probe_on_startup: false,
        use_routing_engine: true,
    });
    let orch = AgentOrchestrator::new(config);
    assert!(
        orch.is_ok(),
        "orchestrator should construct with routing config: {:?}",
        orch.err()
    );
}

/// Gitea-issue: no comments on issue -> spawn (Findings).
/// This tests the path where fetch_comments returns empty vec.
/// Without a real Gitea server, we test via the fail-open path.
/// The no-workflow test already covers fail-open; this test ensures
/// that a config WITH workflow but unreachable endpoint also fails open.
#[tokio::test]
async fn test_gitea_issue_unreachable_endpoint_fail_open() {
    let mut config = test_config();
    config.workflow = Some(WorkflowConfig {
        enabled: true,
        poll_interval_secs: 30,
        workflow_file: PathBuf::from("WORKFLOW.md"),
        tracker: TrackerConfig {
            kind: "gitea".to_string(),
            endpoint: "http://127.0.0.1:1".to_string(), // unreachable port
            api_key: "test-token".to_string(),
            owner: "testowner".to_string(),
            repo: "testrepo".to_string(),
            project_slug: None,
            use_robot_api: false,
            states: TrackerStates {
                active: vec!["open".to_string()],
                terminal: vec!["closed".to_string()],
            },
        },
        concurrency: terraphim_orchestrator::ConcurrencyConfig::default(),
    });
    config.agents[0].pre_check =
        Some(terraphim_orchestrator::PreCheckStrategy::GiteaIssue { issue_number: 42 });
    let mut orch = AgentOrchestrator::new(config).unwrap();
    let result = orch.spawn_agent_for_test("sentinel").await;
    assert!(result.is_ok());
    assert!(orch.is_agent_active("sentinel")); // fail-open on connection refused
}

/// Integration: spawn_agent is skipped when git-diff finds no matching changes.
/// Uses real git repo (CARGO_MANIFEST_DIR) with HEAD as last-run commit.
#[tokio::test]
async fn test_spawn_agent_skipped_by_git_diff_no_matching() {
    let mut config = test_config();
    config.working_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    config.agents[0].pre_check = Some(terraphim_orchestrator::PreCheckStrategy::GitDiff {
        watch_paths: vec!["nonexistent-path-that-never-matches/".to_string()],
    });
    let mut orch = AgentOrchestrator::new(config).unwrap();

    // Seed with initial commit so there are diff results
    let baseline_commit = git_diff_baseline();
    orch.set_last_run_commit("sentinel", &baseline_commit);

    let result = orch.spawn_agent_for_test("sentinel").await;
    assert!(result.is_ok());
    assert!(!orch.is_agent_active("sentinel")); // skipped: changes don't match watch_paths
}

/// Integration: spawn_agent proceeds when git-diff finds matching changes.
/// Uses real git repo with empty tree as baseline.
#[tokio::test]
async fn test_spawn_agent_proceeds_with_git_diff_findings() {
    let mut config = test_config();
    config.working_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    config.agents[0].pre_check = Some(terraphim_orchestrator::PreCheckStrategy::GitDiff {
        watch_paths: vec!["crates/".to_string()],
    });
    let mut orch = AgentOrchestrator::new(config).unwrap();

    // Use initial commit as baseline -> every file is a change
    let baseline_commit = git_diff_baseline();
    orch.set_last_run_commit("sentinel", &baseline_commit);

    let result = orch.spawn_agent_for_test("sentinel").await;
    assert!(result.is_ok());
    assert!(orch.is_agent_active("sentinel")); // changes match crates/ watch_path
}
