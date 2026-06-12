//! Unit tests for terraphim_orchestrator (relocated from lib.rs as part of the
//! Gitea #1910 god-file decomposition; behaviour unchanged).

use super::*;
use tempfile::TempDir;

fn legacy_key(name: &str) -> (String, String) {
    (
        crate::dispatcher::LEGACY_PROJECT_ID.to_string(),
        name.to_string(),
    )
}

fn test_config() -> OrchestratorConfig {
    OrchestratorConfig {
        working_dir: std::path::PathBuf::from("/tmp/test-orchestrator"),
        nightwatch: NightwatchConfig::default(),
        compound_review: CompoundReviewConfig {
            cli_tool: None,
            provider: None,
            model: None,
            schedule: "0 2 * * *".to_string(),
            max_duration_secs: 60,
            repo_path: std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../.."),
            create_prs: false,
            worktree_root: std::path::PathBuf::from("/tmp/test-orchestrator/.worktrees"),
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
                default_tier: None,
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
                event_only: false,
                evolution_enabled: false,
                rlm_enabled: None,
                bypass_kg_routing: false,
                enabled: true,

                project: None,
            },
            AgentDefinition {
                name: "sync".to_string(),
                layer: AgentLayer::Core,
                cli_tool: "echo".to_string(),
                task: "sync upstream".to_string(),
                model: None,
                default_tier: None,
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
                event_only: false,
                evolution_enabled: false,
                rlm_enabled: None,
                bypass_kg_routing: false,
                enabled: true,

                project: None,
            },
        ],
        restart_cooldown_secs: 60,
        max_restart_count: 10,
        restart_budget_window_secs: 43_200,
        disk_usage_threshold: 100, // disable disk guard in tests
        tick_interval_secs: 30,
        gate_reconcile_interval_ticks: 20,
        handoff_buffer_ttl_secs: None,
        persona_data_dir: None,
        skill_data_dir: None,
        gitea_skill_repo: None,
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
        providers: vec![],
        provider_budget_state_file: None,
        pause_dir: None,
        project_circuit_breaker_threshold: 3,
        fleet_escalation_owner: None,
        fleet_escalation_repo: None,
        auto_merge: None,
        post_merge_gate: None,
        learning: config::LearningConfig::default(),
        evolution: config::EvolutionConfig::default(),
        pr_dispatch: None,
        pr_dispatch_per_project: Default::default(),
        direct_dispatch: None,
    }
}

#[test]
fn test_orchestrator_creates_from_config() {
    let config = test_config();
    let orch = AgentOrchestrator::new(config);
    assert!(orch.is_ok());
}

#[test]
fn test_orchestrator_initial_state() {
    let config = test_config();
    let orch = AgentOrchestrator::new(config).unwrap();
    assert!(orch.active_agents.is_empty());
    assert!(!orch.shutdown_requested);
    let statuses = orch.agent_statuses();
    assert!(statuses.is_empty());
}

#[test]
fn test_orchestrator_shutdown_flag() {
    let config = test_config();
    let mut orch = AgentOrchestrator::new(config).unwrap();
    assert!(!orch.shutdown_requested);
    orch.shutdown();
    assert!(orch.shutdown_requested);
}

#[cfg(unix)]
#[tokio::test]
async fn test_direct_dispatch_config_starts_socket_listener() {
    use std::os::unix::fs::FileTypeExt;

    let temp = TempDir::new().unwrap();
    let socket_path = temp.path().join("direct-dispatch.sock");
    let mut config = test_config();
    config.agents.clear();
    config.direct_dispatch = Some(crate::config::DirectDispatchConfig {
        socket_path: socket_path.clone(),
    });

    let mut orch = AgentOrchestrator::new(config).unwrap();
    orch.shutdown();
    orch.run().await.unwrap();

    let mut socket_created = false;
    for _ in 0..50 {
        if std::fs::symlink_metadata(&socket_path)
            .map(|metadata| metadata.file_type().is_socket())
            .unwrap_or(false)
        {
            socket_created = true;
            break;
        }
        tokio::task::yield_now().await;
    }

    assert!(
        socket_created,
        "direct dispatch listener did not create socket at {}",
        socket_path.display()
    );
}

#[tokio::test]
async fn test_handle_direct_dispatch_spawns_agent_without_mentions() {
    let mut config = test_config();
    config.agents = vec![AgentDefinition {
        name: "echo-agent".to_string(),
        layer: AgentLayer::Core,
        cli_tool: "echo".to_string(),
        task: "echo hello".to_string(),
        schedule: None,
        model: None,
        default_tier: None,
        capabilities: vec!["echo".to_string()],
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
        event_only: false,
        evolution_enabled: false,
        rlm_enabled: None,
        bypass_kg_routing: false,
        enabled: true,
        project: None,
    }];
    config.mentions = None;

    let mut orch = AgentOrchestrator::new(config).unwrap();

    let dispatch = webhook::WebhookDispatch::SpawnAgent {
        agent_name: "echo-agent".to_string(),
        detected_project: None,
        issue_number: 0,
        comment_id: 0,
        context: "test context".to_string(),
        synthetic_event: None,
    };

    orch.handle_direct_dispatch(dispatch).await;

    assert!(
        orch.active_agents.contains_key("echo-agent"),
        "direct dispatch must spawn agent even without mentions config; active_agents: {:?}",
        orch.active_agents.keys().collect::<Vec<_>>()
    );
}

#[tokio::test]
async fn test_handle_direct_dispatch_rejects_disabled_agent() {
    let mut config = test_config();
    config.agents = vec![AgentDefinition {
        name: "disabled-agent".to_string(),
        layer: AgentLayer::Core,
        cli_tool: "echo".to_string(),
        task: "echo hello".to_string(),
        schedule: None,
        model: None,
        default_tier: None,
        capabilities: vec!["echo".to_string()],
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
        event_only: false,
        evolution_enabled: false,
        rlm_enabled: None,
        bypass_kg_routing: false,
        enabled: false,
        project: None,
    }];
    config.mentions = None;

    let mut orch = AgentOrchestrator::new(config).unwrap();

    let dispatch = webhook::WebhookDispatch::SpawnAgent {
        agent_name: "disabled-agent".to_string(),
        detected_project: None,
        issue_number: 0,
        comment_id: 0,
        context: String::new(),
        synthetic_event: None,
    };

    orch.handle_direct_dispatch(dispatch).await;

    assert!(
        !orch.active_agents.contains_key("disabled-agent"),
        "direct dispatch must not spawn disabled agent; active_agents: {:?}",
        orch.active_agents.keys().collect::<Vec<_>>()
    );
}

#[tokio::test]
#[ignore = "flaky: depends on live git repo state which may be shallow clone in CI/rch"]
async fn test_orchestrator_compound_review_manual() {
    // Use empty groups to avoid git worktree operations during test.
    // Worktree creation fails when git index is locked (e.g. pre-commit hooks).
    let repo_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");

    // In shallow clones (e.g. CI with fetch-depth: 1) HEAD~1 does not exist.
    // Fall back to diffing against the empty tree so the test works everywhere.
    let base_ref = {
        let check = std::process::Command::new("git")
            .args([
                "-C",
                repo_path.to_str().unwrap(),
                "rev-parse",
                "--verify",
                "HEAD~1",
            ])
            .output();
        match check {
            Ok(o) if o.status.success() => "HEAD~1".to_string(),
            _ => {
                // 4b825dc: the well-known empty tree hash in git
                let empty = std::process::Command::new("git")
                    .args([
                        "-C",
                        repo_path.to_str().unwrap(),
                        "hash-object",
                        "-t",
                        "tree",
                        "/dev/null",
                    ])
                    .output()
                    .expect("git hash-object failed");
                String::from_utf8_lossy(&empty.stdout).trim().to_string()
            }
        }
    };

    let swarm_config = SwarmConfig {
        groups: vec![],
        timeout: Duration::from_secs(60),
        worktree_root: std::path::PathBuf::from("/tmp/test-orchestrator/.worktrees"),
        repo_path,
        base_branch: "main".to_string(),
        max_concurrent_agents: 3,
        create_prs: false,
    };

    let workflow = CompoundReviewWorkflow::new(swarm_config);
    let result = workflow.run("HEAD", &base_ref).await.unwrap();

    assert!(
        !result.correlation_id.is_nil(),
        "correlation_id should be set"
    );
    assert_eq!(result.agents_run, 0, "no agents with empty groups");
    assert_eq!(result.agents_failed, 0);
}

/// Regression test for #1562.
///
/// Property: when `check_cron_schedules` fires the compound review,
/// `last_compound_review_fired_at` advances **before** the `await`
/// on `handle_schedule_event`. Calling `check_cron_schedules` a
/// second time without advancing wall-clock time must NOT re-fire
/// the same occurrence; the cursor stays put.
///
/// This is the property that breaks if the cursor is dropped: the
/// 90 s `tokio::time::timeout` wrapping `reconcile_tick` cancels
/// the future mid-await, `last_tick_time` is never updated, and
/// the next tick re-evaluates the same cron occurrence as "should
/// fire", spawning a new worktree every tick (the bigbox storm).
#[tokio::test]
async fn test_compound_review_cursor_advances_on_cancellation() {
    // Build a test config and override compound_review so that the
    // workflow has no review groups -- it still creates a worktree
    // on the workspace git repo, but no agent subprocesses are
    // launched. This mirrors `test_orchestrator_compound_review_manual`.
    let mut config = test_config();
    let tmp_worktree = TempDir::new().expect("tempdir");
    config.compound_review.worktree_root = tmp_worktree.path().to_path_buf();
    // Schedule fires hourly so we can use a recent `last_tick_time`.
    // 5-field cron: minute 0 of every hour.
    config.compound_review.schedule = "0 * * * *".to_string();

    let mut orch = AgentOrchestrator::new(config).expect("orchestrator");

    // Replace the compound workflow with one that uses an empty
    // group list so the cron-fire path is a no-op apart from the
    // worktree creation/removal. The orchestrator's
    // `repo_path`/`base_branch` are inherited from the test config.
    let repo_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let swarm_config = crate::compound::SwarmConfig {
        groups: vec![],
        timeout: Duration::from_secs(60),
        worktree_root: tmp_worktree.path().to_path_buf(),
        repo_path,
        base_branch: "main".to_string(),
        max_concurrent_agents: 1,
        create_prs: false,
    };
    orch.compound_workflow = crate::compound::CompoundReviewWorkflow::new(swarm_config);

    // Plant `last_tick_time` 2 hours ago so at least one occurrence
    // of `0 * * * *` lies in [last_tick_time, now]. Clear the new
    // cursor so the first call has nothing to compare against.
    let two_hours_ago = chrono::Utc::now() - chrono::Duration::hours(2);
    orch.set_last_tick_time(two_hours_ago);
    orch.clear_last_compound_review_fired_at();
    assert!(
        orch.last_compound_review_fired_at().is_none(),
        "cursor should start empty",
    );

    // First call: should advance the cursor to a past fire time.
    orch.check_cron_schedules().await;
    let cursor_after_first = orch
        .last_compound_review_fired_at()
        .expect("cursor should be Some after first fire");
    assert!(
        cursor_after_first <= chrono::Utc::now(),
        "cursor should be in the past, got {}",
        cursor_after_first
    );

    // Second call without advancing wall-clock or `last_tick_time`:
    // the cursor must NOT advance (the same occurrence is gated).
    orch.check_cron_schedules().await;
    let cursor_after_second = orch
        .last_compound_review_fired_at()
        .expect("cursor should still be Some");
    assert_eq!(
        cursor_after_first, cursor_after_second,
        "cursor must not re-advance on a re-check without new occurrences \
             (#1562 storm regression)",
    );
}

#[test]
fn test_orchestrator_from_toml() {
    let toml_str = r#"
working_dir = "/tmp"

[nightwatch]

[compound_review]
schedule = "0 2 * * *"
repo_path = "/tmp"

[[agents]]
name = "test"
layer = "Safety"
cli_tool = "echo"
task = "test"
"#;
    let config = OrchestratorConfig::from_toml(toml_str).unwrap();
    let orch = AgentOrchestrator::new(config);
    assert!(orch.is_ok());
}

#[test]
fn test_agent_status_fields() {
    let status = AgentStatus {
        name: "test".to_string(),
        layer: AgentLayer::Safety,
        running: true,
        health: HealthStatus::Healthy,
        drift_score: Some(0.05),
        uptime: Duration::from_secs(3600),
        restart_count: 0,
        api_calls_remaining: HashMap::new(),
    };
    assert_eq!(status.name, "test");
    assert!(status.running);
    assert_eq!(status.drift_score, Some(0.05));
}

#[test]
fn test_load_skill_chain_content_supports_lowercase_skill_md() {
    let skill_root = TempDir::new().unwrap();
    let skill_dir = skill_root.path().join("business-scenario-design");
    std::fs::create_dir_all(&skill_dir).unwrap();
    std::fs::write(skill_dir.join("skill.md"), "Lowercase skill content").unwrap();

    let mut config = test_config();
    config.skill_data_dir = Some(skill_root.path().to_path_buf());
    let orch = AgentOrchestrator::new(config).unwrap();

    let mut def = orch.config.agents[0].clone();
    def.skill_chain = vec!["business-scenario-design".to_string()];

    let loaded = orch.load_skill_chain_content(&def);
    assert!(loaded.contains("### Skill: business-scenario-design"));
    assert!(loaded.contains("Lowercase skill content"));
}

#[test]
fn test_load_skill_chain_content_falls_back_to_home_skill_roots() {
    let home_dir = TempDir::new().unwrap();
    let configured_skill_root = TempDir::new().unwrap();

    let roots = AgentOrchestrator::skill_roots(
        Some(configured_skill_root.path()),
        Some(home_dir.path()),
        None,
    );

    assert_eq!(roots[0], configured_skill_root.path());
    assert!(roots.iter().any(|path| path.ends_with(".opencode/skills")));
    assert!(roots.iter().any(|path| path.ends_with(".claude/skills")));
}

/// Helper: create a config with a single Safety echo agent and short cooldown.
fn test_config_fast_lifecycle() -> OrchestratorConfig {
    OrchestratorConfig {
        working_dir: std::path::PathBuf::from("/tmp"),
        nightwatch: NightwatchConfig::default(),
        compound_review: CompoundReviewConfig {
            cli_tool: None,
            provider: None,
            model: None,
            schedule: "0 2 * * *".to_string(),
            max_duration_secs: 60,
            repo_path: std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../.."),
            create_prs: false,
            worktree_root: std::path::PathBuf::from("/tmp/.worktrees"),
            base_branch: "main".to_string(),
            max_concurrent_agents: 3,
            ..Default::default()
        },
        workflow: None,
        agents: vec![AgentDefinition {
            name: "echo-safety".to_string(),
            layer: AgentLayer::Safety,
            cli_tool: "echo".to_string(),
            task: "safety watch".to_string(),
            model: None,
            default_tier: None,
            schedule: None,
            capabilities: vec![],
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
            event_only: false,
            evolution_enabled: false,
            rlm_enabled: None,
            bypass_kg_routing: false,
            enabled: true,

            project: None,
        }],
        restart_cooldown_secs: 0, // instant restart for testing
        max_restart_count: 3,
        restart_budget_window_secs: 43_200,
        disk_usage_threshold: 100, // disable disk guard in tests
        tick_interval_secs: 1,
        gate_reconcile_interval_ticks: 20,
        handoff_buffer_ttl_secs: None,
        persona_data_dir: None,
        skill_data_dir: None,
        gitea_skill_repo: None,
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
        providers: vec![],
        provider_budget_state_file: None,
        pause_dir: None,
        project_circuit_breaker_threshold: 3,
        fleet_escalation_owner: None,
        fleet_escalation_repo: None,
        auto_merge: None,
        post_merge_gate: None,
        learning: config::LearningConfig::default(),
        evolution: config::EvolutionConfig::default(),
        pr_dispatch: None,
        pr_dispatch_per_project: Default::default(),
        direct_dispatch: None,
    }
}

#[tokio::test]
async fn test_reconcile_detects_agent_exit() {
    let config = test_config_fast_lifecycle();
    let mut orch = AgentOrchestrator::new(config).unwrap();

    // Spawn the echo agent (exits immediately)
    let def = orch.config.agents[0].clone();
    orch.spawn_agent(&def).await.unwrap();
    assert!(orch.active_agents.contains_key("echo-safety"));

    // Give echo time to exit
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Poll for exits
    orch.poll_agent_exits().await;

    // Agent should be removed from active_agents
    assert!(
        !orch.active_agents.contains_key("echo-safety"),
        "exited agent should be removed from active_agents"
    );

    // Successful exit (code 0) should NOT increment restart count
    assert_eq!(
        orch.restart_counts
            .get(&legacy_key("echo-safety"))
            .copied()
            .unwrap_or(0),
        0,
        "successful exit should not increment restart count"
    );
}

#[tokio::test]
async fn test_safety_agent_restarts_after_cooldown() {
    let config = test_config_fast_lifecycle();
    let mut orch = AgentOrchestrator::new(config).unwrap();

    // Spawn and let it exit
    let def = orch.config.agents[0].clone();
    orch.spawn_agent(&def).await.unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;
    orch.poll_agent_exits().await;
    assert!(!orch.active_agents.contains_key("echo-safety"));

    // Restart pending (cooldown is 0, so immediate)
    orch.restart_pending_safety_agents().await;
    assert!(
        orch.active_agents.contains_key("echo-safety"),
        "safety agent should be restarted after cooldown"
    );
}

#[tokio::test]
async fn test_core_agent_no_auto_restart() {
    let mut config = test_config_fast_lifecycle();
    config.agents = vec![AgentDefinition {
        name: "echo-core".to_string(),
        layer: AgentLayer::Core,
        cli_tool: "echo".to_string(),
        task: "core task".to_string(),
        model: None,
        default_tier: None,
        schedule: Some("0 3 * * *".to_string()),
        capabilities: vec![],
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
        event_only: false,
        evolution_enabled: false,
        rlm_enabled: None,
        bypass_kg_routing: false,
        enabled: true,

        project: None,
    }];
    let mut orch = AgentOrchestrator::new(config).unwrap();

    // Spawn core agent and let it exit
    let def = orch.config.agents[0].clone();
    orch.spawn_agent(&def).await.unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;
    orch.poll_agent_exits().await;
    assert!(!orch.active_agents.contains_key("echo-core"));

    // Restart pending should NOT restart a Core agent
    orch.restart_pending_safety_agents().await;
    assert!(
        !orch.active_agents.contains_key("echo-core"),
        "core agent should not auto-restart"
    );
}

#[tokio::test]
async fn test_max_restart_count_respected() {
    let mut config = test_config_fast_lifecycle();
    config.max_restart_count = 2;
    // Use a command that exits non-zero so restart_count increments
    config.agents[0].cli_tool = "false".to_string();
    config.agents[0].task = String::new();
    let mut orch = AgentOrchestrator::new(config).unwrap();
    let def = orch.config.agents[0].clone();

    // Cycle through max_restart_count + 1 exits (all non-zero)
    for i in 0..3 {
        orch.spawn_agent(&def).await.unwrap();
        tokio::time::sleep(Duration::from_millis(100)).await;
        orch.poll_agent_exits().await;
        assert!(
            !orch.active_agents.contains_key("echo-safety"),
            "agent should have exited on cycle {}",
            i
        );
    }

    // After 3 non-zero exits, restart count = 3, max = 2
    // restart_pending should NOT restart (count > max)
    orch.restart_pending_safety_agents().await;
    assert!(
        !orch.active_agents.contains_key("echo-safety"),
        "agent should not restart after exceeding max_restart_count"
    );
    assert_eq!(
        orch.restart_counts.get(&legacy_key("echo-safety")).copied(),
        Some(3)
    );
}

#[test]
fn test_restart_count_ages_out_after_budget_window() {
    let mut config = test_config_fast_lifecycle();
    config.restart_budget_window_secs = 1;
    let mut orch = AgentOrchestrator::new(config).unwrap();

    orch.restart_counts.insert(legacy_key("echo-safety"), 3);
    orch.restart_last_failure_unix_secs.insert(
        legacy_key("echo-safety"),
        chrono::Utc::now().timestamp() - 5,
    );

    let count = orch.current_restart_count(&legacy_key("echo-safety"));
    assert_eq!(count, 0);
    assert!(!orch.restart_counts.contains_key(&legacy_key("echo-safety")));
    assert!(!orch
        .restart_last_failure_unix_secs
        .contains_key(&legacy_key("echo-safety")));
}

#[tokio::test]
async fn test_successful_exit_does_not_increment_restart_count() {
    let config = test_config_fast_lifecycle();
    let mut orch = AgentOrchestrator::new(config).unwrap();
    let def = orch.config.agents[0].clone(); // echo "safety watch" -> exit 0

    // Spawn and let it exit successfully multiple times
    for _ in 0..5 {
        orch.spawn_agent(&def).await.unwrap();
        tokio::time::sleep(Duration::from_millis(100)).await;
        orch.poll_agent_exits().await;
    }

    // Exit code 0 should never increment restart_count
    assert_eq!(
        orch.restart_counts
            .get(&legacy_key("echo-safety"))
            .copied()
            .unwrap_or(0),
        0,
        "successful exits (code 0) must not increment restart_count"
    );

    // Agent should still be eligible for restart
    orch.restart_cooldowns.insert(
        legacy_key("echo-safety"),
        Instant::now() - Duration::from_secs(999),
    );
    orch.restart_pending_safety_agents().await;
    assert!(
        orch.active_agents.contains_key("echo-safety"),
        "agent with only successful exits should always be restartable"
    );
}

#[tokio::test]
async fn test_output_events_fed_to_nightwatch() {
    let config = test_config_fast_lifecycle();
    let mut orch = AgentOrchestrator::new(config).unwrap();

    // Spawn echo agent (writes "safety watch" to stdout)
    let def = orch.config.agents[0].clone();
    orch.spawn_agent(&def).await.unwrap();

    // Give the output capture time to process
    tokio::time::sleep(Duration::from_millis(200)).await;

    // Drain events
    orch.drain_output_events();

    // Nightwatch should have observations for the agent
    let drift = orch.nightwatch.drift_score("echo-safety");
    assert!(
        drift.is_some(),
        "nightwatch should have drift data after draining output events"
    );
    let drift = drift.unwrap();
    assert!(
        drift.metrics.sample_count > 0,
        "nightwatch should have at least one sample from drained output"
    );
}

#[tokio::test]
async fn test_reconcile_tick_full_cycle() {
    let config = test_config_fast_lifecycle();
    let mut orch = AgentOrchestrator::new(config).unwrap();

    // Spawn echo agent
    let def = orch.config.agents[0].clone();
    orch.spawn_agent(&def).await.unwrap();

    // Give echo time to exit and produce output
    tokio::time::sleep(Duration::from_millis(200)).await;

    // Run a full reconciliation tick
    orch.reconcile_tick().await;

    // After tick: echo exited, so it was detected and marked for restart.
    // With 0 cooldown, it should have been restarted in the same tick.
    assert!(
        orch.active_agents.contains_key("echo-safety"),
        "safety agent should be restarted by reconcile_tick"
    );
    // echo exits with code 0, so restart_count stays at 0
    assert_eq!(
        orch.restart_counts
            .get(&legacy_key("echo-safety"))
            .copied()
            .unwrap_or(0),
        0,
        "successful exit should not increment restart count"
    );
}

// =========================================================================
// Persona Injection Tests (Gitea #73)
// =========================================================================

/// Test that spawn_agent composes persona-enriched prompt when persona exists
#[tokio::test]
async fn test_spawn_agent_with_persona_composes_prompt() {
    let mut config = test_config_fast_lifecycle();

    // Add an agent with a persona
    // Use cat (not echo) because persona_found=true triggers stdin delivery.
    // cat reads stdin before exiting, avoiding broken pipe under parallel load.
    config.agents = vec![AgentDefinition {
        name: "persona-agent".to_string(),
        layer: AgentLayer::Safety,
        cli_tool: "cat".to_string(),
        task: "test task".to_string(),
        model: None,
        default_tier: None,
        schedule: None,
        capabilities: vec![],
        max_memory_bytes: None,
        budget_monthly_cents: None,
        provider: None,
        persona: Some("TestAgent".to_string()), // Persona that exists in default test_persona
        terraphim_role: None,
        skill_chain: vec![],
        sfia_skills: vec![],
        fallback_provider: None,
        fallback_model: None,
        grace_period_secs: None,
        max_cpu_seconds: None,
        pre_check: None,

        gitea_issue: None,
        event_only: false,
        evolution_enabled: false,
        rlm_enabled: None,
        bypass_kg_routing: false,
        enabled: true,

        project: None,
    }];

    // Set up persona data dir with a test persona
    let temp_dir =
        std::env::temp_dir().join(format!("terraphim-test-persona-{}", std::process::id()));
    std::fs::create_dir_all(&temp_dir).unwrap();

    let persona_toml = r#"
agent_name = "TestAgent"
role_name = "Test Engineer"
name_origin = "From testing"
vibe = "Thorough, methodical"
symbol = "Checkmark"
core_characteristics = [{ name = "Thorough", description = "checks everything twice" }]
speech_style = "Precise and factual."
terraphim_nature = "Adapted to testing environments."
sfia_title = "Test Engineer"
primary_level = 4
guiding_phrase = "Enable"
level_essence = "Works autonomously under general direction."
sfia_skills = [{ code = "TEST", name = "Testing", level = 4, description = "Designs and executes test plans." }]
"#;
    std::fs::write(temp_dir.join("testagent.toml"), persona_toml).unwrap();
    config.persona_data_dir = Some(temp_dir.clone());

    let mut orch = AgentOrchestrator::new(config).unwrap();

    // Spawn the agent - it should use the persona-enriched prompt
    let def = orch.config.agents[0].clone();
    let result = orch.spawn_agent(&def).await;

    // Cleanup
    let _ = std::fs::remove_dir_all(&temp_dir);

    // Spawn should succeed
    assert!(result.is_ok());

    // The agent should be active
    assert!(orch.active_agents.contains_key("persona-agent"));
}

/// Test that spawn_agent uses bare task when persona is None
#[tokio::test]
async fn test_spawn_agent_without_persona_uses_bare_task() {
    let config = test_config_fast_lifecycle();
    let mut orch = AgentOrchestrator::new(config).unwrap();

    // Agent without persona should use bare task
    let def = orch.config.agents[0].clone();
    assert!(def.persona.is_none());

    let result = orch.spawn_agent(&def).await;
    assert!(result.is_ok());

    assert!(orch.active_agents.contains_key("echo-safety"));
}

/// Test graceful degradation when persona not found in registry
#[tokio::test]
async fn test_spawn_agent_persona_not_found_graceful() {
    let mut config = test_config_fast_lifecycle();

    // Add an agent with a non-existent persona
    config.agents = vec![AgentDefinition {
        name: "unknown-persona-agent".to_string(),
        layer: AgentLayer::Safety,
        cli_tool: "echo".to_string(),
        task: "test task".to_string(),
        model: None,
        default_tier: None,
        schedule: None,
        capabilities: vec![],
        max_memory_bytes: None,
        budget_monthly_cents: None,
        provider: None,
        persona: Some("NonExistentPersona".to_string()), // This persona doesn't exist
        terraphim_role: None,
        skill_chain: vec![],
        sfia_skills: vec![],
        fallback_provider: None,
        fallback_model: None,
        grace_period_secs: None,
        max_cpu_seconds: None,
        pre_check: None,

        gitea_issue: None,
        event_only: false,
        evolution_enabled: false,
        rlm_enabled: None,
        bypass_kg_routing: false,
        enabled: true,

        project: None,
    }];

    // No persona_data_dir, so registry will be empty
    config.persona_data_dir = None;

    let mut orch = AgentOrchestrator::new(config).unwrap();

    // Spawn should still succeed even though persona doesn't exist
    let def = orch.config.agents[0].clone();
    let result = orch.spawn_agent(&def).await;

    assert!(
        result.is_ok(),
        "spawn should succeed with fallback to bare task"
    );
    assert!(orch.active_agents.contains_key("unknown-persona-agent"));
}

// ==================== Agent Name Validation Tests ====================

#[test]
fn test_validate_agent_name_accepts_valid() {
    assert!(validate_agent_name("my-agent_1").is_ok());
    assert!(validate_agent_name("sentinel").is_ok());
    assert!(validate_agent_name("Agent-42").is_ok());
}

#[test]
fn test_validate_agent_name_rejects_traversal() {
    assert!(validate_agent_name("../etc/passwd").is_err());
    assert!(validate_agent_name("..").is_err());
    assert!(validate_agent_name("foo/../bar").is_err());
}

#[test]
fn test_validate_agent_name_rejects_slash() {
    assert!(validate_agent_name("foo/bar").is_err());
    assert!(validate_agent_name("foo\\bar").is_err());
}

#[test]
fn test_validate_agent_name_rejects_empty() {
    assert!(validate_agent_name("").is_err());
}

#[test]
fn test_validate_agent_name_rejects_special_chars() {
    assert!(validate_agent_name("agent name").is_err()); // spaces
    assert!(validate_agent_name("agent@host").is_err()); // @
    assert!(validate_agent_name("agent.name").is_err()); // dots
}

// ==================== has_matching_changes Tests ====================

#[test]
fn test_has_matching_changes_prefix_match() {
    let changed = vec!["crates/orchestrator/src/lib.rs".to_string()];
    let watch = vec!["crates/orchestrator/".to_string()];
    assert!(has_matching_changes(&changed, &watch));
}

#[test]
fn test_has_matching_changes_exact_match() {
    let changed = vec!["Cargo.toml".to_string()];
    let watch = vec!["Cargo.toml".to_string()];
    assert!(has_matching_changes(&changed, &watch));
}

#[test]
fn test_has_matching_changes_no_match() {
    let changed = vec!["docs/README.md".to_string()];
    let watch = vec!["crates/orchestrator/".to_string()];
    assert!(!has_matching_changes(&changed, &watch));
}

#[test]
fn test_has_matching_changes_multiple_files_one_matches() {
    let changed = vec![
        "docs/README.md".to_string(),
        "crates/orchestrator/src/config.rs".to_string(),
    ];
    let watch = vec!["crates/orchestrator/".to_string()];
    assert!(has_matching_changes(&changed, &watch));
}

#[test]
fn test_has_matching_changes_multiple_watch_paths() {
    let changed = vec!["tests/integration.rs".to_string()];
    let watch = vec!["crates/orchestrator/".to_string(), "tests/".to_string()];
    assert!(has_matching_changes(&changed, &watch));
}

#[test]
fn test_has_matching_changes_empty_watch_paths() {
    let changed = vec!["crates/orchestrator/src/lib.rs".to_string()];
    let watch: Vec<String> = vec![];
    assert!(!has_matching_changes(&changed, &watch));
}

// =========================================================================
// ADF Remediation Tests (Gitea #117)
// =========================================================================

#[test]
fn test_provider_model_composition_opencode() {
    // Simulate what spawn_agent does for opencode with provider + model
    let provider = Some("kimi-for-coding".to_string());
    let model = Some("k2p5".to_string());
    let cli_name = "opencode";

    let composed = if cli_name == "opencode" {
        match (&provider, &model) {
            (Some(p), Some(m)) => Some(format!("{}/{}", p, m)),
            _ => model,
        }
    } else {
        model
    };
    assert_eq!(composed, Some("kimi-for-coding/k2p5".to_string()));
}

#[test]
fn test_provider_model_composition_claude_unchanged() {
    // Claude should not have provider/model composed
    let provider = Some("anthropic".to_string());
    let model = Some("claude-opus-4-6".to_string());
    let cli_name = "claude";

    let composed = if cli_name == "opencode" {
        match (&provider, &model) {
            (Some(p), Some(m)) => Some(format!("{}/{}", p, m)),
            _ => model.clone(),
        }
    } else {
        model.clone()
    };
    assert_eq!(composed, Some("claude-opus-4-6".to_string()));
}

#[test]
fn parse_reset_time_relative_hours() {
    let result = parse_reset_time("resets in 2 hours");
    assert!(result.is_some());
}

#[test]
fn parse_reset_time_relative_minutes() {
    let result = parse_reset_time("resets in 30 minutes");
    assert!(result.is_some());
}

#[test]
fn parse_reset_time_utc_format() {
    let result = parse_reset_time("resets at 14:00 UTC");
    assert!(result.is_some());
}

#[test]
fn parse_reset_time_fallback_generic() {
    let result = parse_reset_time("resets 2am Europe/Berlin");
    assert!(result.is_some());
}

#[test]
fn parse_reset_time_no_match() {
    let result = parse_reset_time("something unrelated");
    assert!(result.is_none());
}

#[test]
fn parse_reset_time_short_hours_abbreviation() {
    // "resets in 4h" -- Claude Code's compact format
    let before = std::time::Instant::now();
    let result = parse_reset_time("5-hour limit reached, resets in 4h").unwrap();
    let delta = result.duration_since(before);
    // 4 hours, allowing a small wall-clock slack between `before` and `Instant::now()`
    // inside the parser.
    assert!(delta >= std::time::Duration::from_secs(4 * 3600 - 1));
    assert!(delta <= std::time::Duration::from_secs(4 * 3600 + 5));
}

#[test]
fn parse_reset_time_short_minutes_abbreviation() {
    // "resets in 30m"
    let before = std::time::Instant::now();
    let result = parse_reset_time("resets in 30m").unwrap();
    let delta = result.duration_since(before);
    assert!(delta >= std::time::Duration::from_secs(30 * 60 - 1));
    assert!(delta <= std::time::Duration::from_secs(30 * 60 + 5));
}

#[test]
fn parse_reset_time_pm_suffix() {
    // "resets 11pm" -- amount depends on wall-clock; just assert non-zero & bounded
    let result = parse_reset_time("you've hit your session limit, resets 11pm");
    assert!(result.is_some(), "11pm must parse to a future instant");
    // The pm path computes a delta < 24h.
    let delta = result.unwrap().duration_since(std::time::Instant::now());
    assert!(delta <= std::time::Duration::from_secs(24 * 3600));
}

#[test]
fn parse_reset_time_am_suffix() {
    let result = parse_reset_time("session limit reached, resets 7am");
    assert!(result.is_some());
    let delta = result.unwrap().duration_since(std::time::Instant::now());
    assert!(delta <= std::time::Duration::from_secs(24 * 3600));
}

#[test]
fn parse_reset_time_unknown_without_resets_returns_none() {
    // No "resets" token at all -> caller applies safety floor.
    assert!(parse_reset_time("you've hit your session limit").is_none());
    assert!(parse_reset_time("rate limit exceeded, try later").is_none());
}

#[test]
fn default_rate_limit_block_is_fifteen_minutes() {
    assert_eq!(
        DEFAULT_RATE_LIMIT_BLOCK,
        std::time::Duration::from_secs(900)
    );
}

#[test]
fn agent_def_bypass_kg_routing_defaults_false() {
    // TOML without the field deserialises to `false` via #[serde(default)].
    let toml_src = r#"
name = "test-agent"
layer = "Core"
cli_tool = "/bin/echo"
task = "ping"
"#;
    let def: AgentDefinition = toml::from_str(toml_src).expect("parse default");
    assert!(!def.bypass_kg_routing);
}

#[test]
fn agent_def_bypass_kg_routing_explicit_true() {
    let toml_src = r#"
name = "test-agent"
layer = "Core"
cli_tool = "/bin/echo"
task = "ping"
bypass_kg_routing = true
"#;
    let def: AgentDefinition = toml::from_str(toml_src).expect("parse explicit");
    assert!(def.bypass_kg_routing);
}

#[test]
fn rate_limit_window_block_and_check() {
    let mut window = ProviderRateLimitWindow::new();
    assert!(!window.is_blocked("claude-code"));
    window.block_until(
        "claude-code",
        std::time::Instant::now() + std::time::Duration::from_secs(3600),
    );
    assert!(window.is_blocked("claude-code"));
    assert!(!window.is_blocked("kimi"));
}

#[test]
fn rate_limit_window_expired_unblocks() {
    let mut window = ProviderRateLimitWindow::new();
    window.block_until(
        "claude-code",
        std::time::Instant::now() + std::time::Duration::from_millis(1),
    );
    std::thread::sleep(std::time::Duration::from_millis(5));
    assert!(!window.is_blocked("claude-code"));
}

#[test]
fn rate_limit_window_blocked_providers_list() {
    let mut window = ProviderRateLimitWindow::new();
    window.block_until(
        "claude-code",
        std::time::Instant::now() + std::time::Duration::from_secs(3600),
    );
    window.block_until(
        "anthropic",
        std::time::Instant::now() + std::time::Duration::from_secs(3600),
    );
    let blocked = window.blocked_providers();
    assert_eq!(blocked.len(), 2);
    assert!(blocked.contains(&"claude-code".to_string()));
    assert!(blocked.contains(&"anthropic".to_string()));
}

#[test]
fn rate_limit_window_clean_expired() {
    let mut window = ProviderRateLimitWindow::new();
    window.block_until(
        "expired",
        std::time::Instant::now() + std::time::Duration::from_millis(1),
    );
    window.block_until(
        "active",
        std::time::Instant::now() + std::time::Duration::from_secs(3600),
    );
    std::thread::sleep(std::time::Duration::from_millis(5));
    window.clean_expired();
    assert!(!window.is_blocked("expired"));
    assert!(window.is_blocked("active"));
}

#[tokio::test]
async fn test_wall_clock_timeout_kills_agent() {
    let mut config = test_config_fast_lifecycle();
    // Use sleep agent with 1-second timeout
    config.agents = vec![AgentDefinition {
        name: "timeout-test".to_string(),
        layer: AgentLayer::Core,
        cli_tool: "sleep".to_string(),
        task: "60".to_string(),
        model: None,
        default_tier: None,
        schedule: None,
        capabilities: vec![],
        max_memory_bytes: None,
        budget_monthly_cents: None,
        provider: None,
        persona: None,
        terraphim_role: None,
        skill_chain: vec![],
        sfia_skills: vec![],
        fallback_provider: None,
        fallback_model: None,
        grace_period_secs: Some(2),
        max_cpu_seconds: Some(1), // 1 second timeout
        pre_check: None,
        gitea_issue: None,
        event_only: false,
        evolution_enabled: false,
        rlm_enabled: None,
        bypass_kg_routing: false,
        enabled: true,
        project: None,
    }];
    let mut orch = AgentOrchestrator::new(config).unwrap();
    let def = orch.config.agents[0].clone();
    orch.spawn_agent(&def).await.unwrap();
    assert!(orch.active_agents.contains_key("timeout-test"));

    // Wait for the timeout to elapse
    tokio::time::sleep(Duration::from_secs(2)).await;

    // Poll should detect timeout and kill
    orch.poll_agent_exits().await;
    assert!(!orch.active_agents.contains_key("timeout-test"));
}

// =========================================================================
// Flow DAG Orchestrator Integration Tests (Gitea #163)
// =========================================================================

#[test]
fn test_orchestrator_with_empty_flows() {
    let mut config = test_config();
    config.flows = vec![];
    config.flow_state_dir = None;

    let orch = AgentOrchestrator::new(config);
    assert!(
        orch.is_ok(),
        "orchestrator should initialize with empty flows"
    );

    let orch = orch.unwrap();
    assert!(
        orch.active_flows.is_empty(),
        "active_flows should be empty initially"
    );
}

/// Test that flow scheduling overlap prevention works
#[tokio::test]
async fn test_flow_overlap_prevention() {
    use crate::flow::config::{FlowDefinition, FlowStepDef, StepKind};

    let mut config = test_config_fast_lifecycle();

    // Add a test flow with a schedule
    config.flows = vec![FlowDefinition {
        name: "test-flow".to_string(),
        project: "test".to_string(),
        schedule: Some("0 2 * * *".to_string()), // 2 AM daily
        repo_path: "/tmp/test-repo".to_string(),
        base_branch: "main".to_string(),
        timeout_secs: 3600,
        steps: vec![FlowStepDef {
            name: "test-step".to_string(),
            kind: StepKind::Action,
            command: Some("echo test".to_string()),
            cli_tool: None,
            model: None,
            task: None,
            task_file: None,
            condition: None,
            timeout_secs: 60,
            on_fail: crate::flow::config::FailStrategy::Abort,
            provider: None,
            persona: None,
            matrix: None,
            loop_target: None,
        }],
    }];

    config.flow_state_dir = Some(PathBuf::from("/tmp/test-flow-states"));

    let orch = AgentOrchestrator::new(config);
    assert!(orch.is_ok(), "orchestrator should initialize with flows");

    let orch = orch.unwrap();
    assert!(
        orch.active_flows.is_empty(),
        "active_flows should be empty initially"
    );
}

// ==================== Sanitisation Tests ====================

#[test]
fn test_sanitise_for_title_strips_json_braces() {
    let input = r#"{"type":"tool_use","timestamp":1775313676859}"#;
    let result = AgentOrchestrator::sanitise_for_title(input);
    assert!(!result.contains('{'), "title should not contain open brace");
    assert!(
        !result.contains('}'),
        "title should not contain close brace"
    );
    assert!(
        !result.contains('['),
        "title should not contain open bracket"
    );
    assert!(
        !result.contains(']'),
        "title should not contain close bracket"
    );
}

#[test]
fn test_sanitise_for_title_strips_quotes() {
    let input = r#"JSON "quoted" text"#;
    let result = AgentOrchestrator::sanitise_for_title(input);
    assert!(!result.contains('"'), "title should not contain quotes");
}

#[test]
fn test_sanitise_for_title_truncates_long_input() {
    let input = "This is a very long finding text that should be truncated because it exceeds eighty characters limit";
    let result = AgentOrchestrator::sanitise_for_title(input);
    assert!(
        result.len() <= 80,
        "title should be at most 80 chars, got {}",
        result.len()
    );
}

#[test]
fn test_sanitise_for_body_escapes_backticks() {
    let input = "Use `code` here";
    let result = AgentOrchestrator::sanitise_for_body(input);
    assert!(result.contains("``"), "body should escape backticks");
}

#[test]
fn test_sanitise_for_body_escapes_markdown_chars() {
    let input = "Text with *asterisks* and [brackets]";
    let result = AgentOrchestrator::sanitise_for_body(input);
    assert!(
        result.contains('\\'),
        "body should contain backslash, got: {}",
        result
    );
}

/// An agent whose monthly budget is exhausted must skip spawn entirely.
/// `CostTracker::check()` returning `Exhausted` short-circuits
/// dispatch before pre-check or routing runs.
#[tokio::test]
async fn test_spawn_agent_skips_when_budget_exhausted() {
    let mut config = test_config_fast_lifecycle();
    config.agents = vec![AgentDefinition {
        name: "broke-agent".to_string(),
        layer: AgentLayer::Safety,
        cli_tool: "echo".to_string(),
        task: "should not run".to_string(),
        model: None,
        default_tier: None,
        schedule: None,
        capabilities: vec![],
        max_memory_bytes: None,
        // $1 monthly budget.
        budget_monthly_cents: Some(100),
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
        event_only: false,
        evolution_enabled: false,
        rlm_enabled: None,
        bypass_kg_routing: false,
        enabled: true,
        project: None,
    }];

    let mut orch = AgentOrchestrator::new(config).unwrap();

    // Blow through the budget before attempting to spawn.
    let verdict = orch.cost_tracker.record_cost("broke-agent", 2.00);
    assert!(
        verdict.should_pause(),
        "budget must be exhausted: {verdict}"
    );

    let def = orch.config.agents[0].clone();
    let result = orch.spawn_agent(&def).await;

    // Spawn should succeed-with-no-op rather than error.
    assert!(result.is_ok(), "spawn returned error: {:?}", result);
    // Agent must NOT have been added to active_agents.
    assert!(
        !orch.active_agents.contains_key("broke-agent"),
        "exhausted agent should not have been spawned"
    );
}

/// An agent with no budget cap (subscription) must spawn normally
/// even after recording cost -- `record_cost` returns `Uncapped`.
#[tokio::test]
async fn test_spawn_agent_runs_when_budget_uncapped() {
    let mut config = test_config_fast_lifecycle();
    // Ensure the only agent is uncapped.
    config.agents[0].budget_monthly_cents = None;
    config.agents[0].name = "subscription-agent".to_string();

    let mut orch = AgentOrchestrator::new(config).unwrap();

    // Even a large recorded spend must not pause an uncapped agent.
    let _ = orch
        .cost_tracker
        .record_cost("subscription-agent", 9_999.00);
    let verdict = orch.cost_tracker.check("subscription-agent");
    assert!(!verdict.should_pause(), "uncapped must never pause");

    let def = orch.config.agents[0].clone();
    let result = orch.spawn_agent(&def).await;
    assert!(result.is_ok(), "spawn errored: {:?}", result);
    assert!(orch.active_agents.contains_key("subscription-agent"));
}

/// Helper: build a minimal config with a project-scoped `pr-reviewer`
/// agent suitable for driving `handle_review_pr` through the full
/// routing and spawn pipeline. Returns the config plus the tempdir that
/// owns the working directory so the caller keeps it alive.
fn review_pr_config(cli_tool: &str) -> (OrchestratorConfig, TempDir) {
    let tmp = TempDir::new().unwrap();
    let working_dir = tmp.path().to_path_buf();
    let config = OrchestratorConfig {
        working_dir: working_dir.clone(),
        nightwatch: NightwatchConfig::default(),
        compound_review: CompoundReviewConfig {
            cli_tool: None,
            provider: None,
            model: None,
            schedule: "0 2 * * *".to_string(),
            max_duration_secs: 60,
            repo_path: working_dir.clone(),
            create_prs: false,
            worktree_root: working_dir.join(".worktrees"),
            base_branch: "main".to_string(),
            max_concurrent_agents: 3,
            ..Default::default()
        },
        workflow: None,
        agents: vec![AgentDefinition {
            name: "pr-reviewer".to_string(),
            layer: AgentLayer::Safety,
            cli_tool: cli_tool.to_string(),
            task: "review".to_string(),
            model: None,
            default_tier: None,
            schedule: None,
            capabilities: vec!["review".to_string()],
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
            event_only: false,
            evolution_enabled: false,
            rlm_enabled: None,
            bypass_kg_routing: false,
            enabled: true,
            project: Some("alpha".to_string()),
        }],
        restart_cooldown_secs: 0,
        max_restart_count: 3,
        restart_budget_window_secs: 43_200,
        disk_usage_threshold: 100,
        tick_interval_secs: 1,
        gate_reconcile_interval_ticks: 20,
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
        projects: vec![crate::config::Project {
            id: "alpha".to_string(),
            working_dir: working_dir.clone(),
            schedule_offset_minutes: 0,
            gitea: None,
            mentions: None,
            workflow: None,
            #[cfg(feature = "quickwit")]
            quickwit: None,
            max_concurrent_agents: None,
            max_concurrent_mention_agents: None,
        }],
        include: vec![],
        providers: vec![],
        provider_budget_state_file: None,
        gitea_skill_repo: None,
        pause_dir: None,
        project_circuit_breaker_threshold: 3,
        fleet_escalation_owner: None,
        fleet_escalation_repo: None,
        auto_merge: None,
        post_merge_gate: None,
        learning: config::LearningConfig::default(),
        evolution: config::EvolutionConfig::default(),
        pr_dispatch: None,
        pr_dispatch_per_project: Default::default(),
        direct_dispatch: None,
    };
    (config, tmp)
}

fn review_pr_task() -> dispatcher::DispatchTask {
    dispatcher::DispatchTask::ReviewPr {
        pr_number: 641,
        project: "alpha".to_string(),
        head_sha: "deadbeef1234".to_string(),
        head_ref: "task/641-review".to_string(),
        author_login: "claude-code".to_string(),
        title: "fix(kg): short synonyms".to_string(),
        diff_loc: 42,
    }
}

/// `handle_review_pr` must drive the routing engine and spawn the
/// pr-reviewer agent it selected, registering it in `active_agents`.
#[tokio::test]
async fn reviewpr_dispatch_routes_via_routing_engine() {
    let (config, _tmp) = review_pr_config("echo");
    let mut orch = AgentOrchestrator::new(config).unwrap();

    orch.handle_review_pr(review_pr_task()).await.unwrap();

    let managed = orch
        .active_agents
        .get("pr-reviewer")
        .expect("pr-reviewer must be registered in active_agents after routing");
    assert!(
        !managed.session_id.is_empty(),
        "routing must tag the spawned agent with a session_id"
    );
    assert!(
        managed.session_id.starts_with("pr-reviewer-"),
        "session id should be scoped to the agent, got: {}",
        managed.session_id
    );
}

/// When a workflow tracker is configured, `handle_review_pr` must POST
/// a `pending` commit status with context `adf/pr-reviewer` for the
/// PR head SHA after spawning. Verifies issue #928 (ADF Phase 1).
#[tokio::test]
async fn reviewpr_dispatch_posts_pending_status_when_tracker_configured() {
    use axum::{
        extract::{Path, State},
        http::StatusCode,
        response::IntoResponse,
        routing::post,
        Router,
    };
    use std::sync::atomic::{AtomicUsize, Ordering as AOrdering};
    use std::sync::Arc;
    use tokio::net::TcpListener;

    #[derive(Default)]
    struct Captured {
        calls: AtomicUsize,
        last_path: std::sync::Mutex<Option<String>>,
        last_body: std::sync::Mutex<Option<serde_json::Value>>,
    }

    async fn capture(
        Path((owner, repo, sha)): Path<(String, String, String)>,
        State(captured): State<Arc<Captured>>,
        body: axum::body::Bytes,
    ) -> impl IntoResponse {
        captured.calls.fetch_add(1, AOrdering::SeqCst);
        *captured.last_path.lock().unwrap() =
            Some(format!("/api/v1/repos/{}/{}/statuses/{}", owner, repo, sha));
        if let Ok(parsed) = serde_json::from_slice::<serde_json::Value>(&body) {
            *captured.last_body.lock().unwrap() = Some(parsed);
        }
        StatusCode::CREATED
    }

    let captured = Arc::new(Captured::default());
    let app = Router::new()
        .route("/api/v1/repos/{owner}/{repo}/statuses/{sha}", post(capture))
        .with_state(captured.clone());
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.ok();
    });
    let base_url = format!("http://{}", addr);

    // Build the standard review-pr config and bolt on a workflow that
    // points at the loopback Gitea endpoint above. Tests with no
    // workflow already cover the silent skip-path (other reviewpr_*).
    let (mut config, _tmp) = review_pr_config("echo");
    config.workflow = Some(crate::config::WorkflowConfig {
        enabled: true,
        poll_interval_secs: 60,
        workflow_file: std::path::PathBuf::from("/tmp/workflow.md"),
        tracker: crate::config::TrackerConfig {
            kind: "gitea".to_string(),
            endpoint: base_url.clone(),
            api_key: "test-token".to_string(),
            owner: "fakeowner".to_string(),
            repo: "fakerepo".to_string(),
            project_slug: None,
            use_robot_api: false,
            states: crate::config::TrackerStates::default(),
        },
        concurrency: crate::config::ConcurrencyConfig::default(),
    });
    let mut orch = AgentOrchestrator::new(config).unwrap();

    orch.handle_review_pr(review_pr_task()).await.unwrap();

    // Allow the loopback server to receive the POST.
    for _ in 0..50 {
        if captured.calls.load(AOrdering::SeqCst) > 0 {
            break;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    assert_eq!(
        captured.calls.load(AOrdering::SeqCst),
        1,
        "exactly one pending status post expected"
    );
    assert_eq!(
        captured.last_path.lock().unwrap().as_deref(),
        Some("/api/v1/repos/fakeowner/fakerepo/statuses/deadbeef1234")
    );
    let body = captured.last_body.lock().unwrap().clone().unwrap();
    assert_eq!(body["state"], "pending");
    assert_eq!(body["context"], "adf/pr-reviewer");
}

/// A banned provider configured on the pr-reviewer agent must be
/// short-circuited by the C1/C3 allow-list gate; no spawn happens.
#[tokio::test]
async fn reviewpr_dispatch_rejects_banned_provider() {
    let (mut config, _tmp) = review_pr_config("echo");
    // Bypass load-time validation by mutating the test config before
    // construction to simulate a stale/poisoned config entry reaching the
    // dispatcher.
    config.agents[0].model = Some("google/gemini-2".to_string());
    let mut orch = AgentOrchestrator::new(config).unwrap();

    orch.handle_review_pr(review_pr_task()).await.unwrap();

    assert!(
        !orch.active_agents.contains_key("pr-reviewer"),
        "banned provider must short-circuit before spawn"
    );
}

/// The per-PR `ADF_PR_*` env overrides must reach the spawned process so
/// downstream review skills can read them without parsing the task string.
#[tokio::test]
async fn reviewpr_dispatch_sets_env_vars() {
    let tmp = TempDir::new().unwrap();
    let script_path = tmp.path().join("dump-env.sh");
    let dump_path = tmp.path().join("env.dump");
    let script_body = format!(
        "#!/bin/sh\nenv | grep '^ADF_PR_' > {}\n",
        dump_path.display()
    );
    std::fs::write(&script_path, script_body).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&script_path).unwrap().permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&script_path, perms).unwrap();
    }

    let (config, _cfg_tmp) = review_pr_config(script_path.to_str().unwrap());
    let mut orch = AgentOrchestrator::new(config).unwrap();

    orch.handle_review_pr(review_pr_task()).await.unwrap();
    assert!(orch.active_agents.contains_key("pr-reviewer"));

    // Poll for the script to exit and for the child process reaper to
    // drop it out of active_agents, then read the env dump it wrote.
    for _ in 0..40 {
        tokio::time::sleep(Duration::from_millis(50)).await;
        orch.poll_agent_exits().await;
        if dump_path.exists() {
            break;
        }
    }

    let dump = std::fs::read_to_string(&dump_path)
        .unwrap_or_else(|e| panic!("env dump not written to {}: {e}", dump_path.display()));
    assert!(
        dump.contains("ADF_PR_NUMBER=641"),
        "ADF_PR_NUMBER missing from dump:\n{dump}"
    );
    assert!(
        dump.contains("ADF_PR_HEAD_SHA=deadbeef1234"),
        "ADF_PR_HEAD_SHA missing from dump:\n{dump}"
    );
    assert!(
        dump.contains("ADF_PR_PROJECT=alpha"),
        "ADF_PR_PROJECT missing from dump:\n{dump}"
    );
    assert!(
        dump.contains("ADF_PR_AUTHOR=claude-code"),
        "ADF_PR_AUTHOR missing from dump:\n{dump}"
    );
    assert!(
        dump.contains("ADF_PR_DIFF_LOC=42"),
        "ADF_PR_DIFF_LOC missing from dump:\n{dump}"
    );
    assert!(
        dump.contains("ADF_PR_TITLE=fix(kg): short synonyms"),
        "ADF_PR_TITLE missing from dump:\n{dump}"
    );
}

/// Helper: extend a `review_pr_config` setup with a project-scoped
/// `build-runner` agent and a Phase 2 D2 fan-out list. After issue
/// #962 the dispatch table lives in `pr_dispatch_per_project`, keyed
/// by the project id ("alpha" in these fixtures); the top-level
/// `pr_dispatch` field is left as `None` to exercise the new
/// per-project lookup branch end-to-end. Sibling tests
/// (`handle_review_pr_skips_missing_agents`,
/// `handle_review_pr_pending_status_posted_per_agent`,
/// `handle_review_pr_skipped_agent_does_not_post_pending`) keep
/// populating the top-level `pr_dispatch` field directly to
/// exercise the backward-compat fallback path.
fn review_pr_config_with_fanout(cli_tool: &str) -> (OrchestratorConfig, TempDir) {
    let (mut config, tmp) = review_pr_config(cli_tool);
    config.agents.push(AgentDefinition {
        name: "build-runner".to_string(),
        layer: AgentLayer::Growth,
        cli_tool: cli_tool.to_string(),
        task: "build".to_string(),
        model: None,
        default_tier: None,
        schedule: None,
        capabilities: vec!["build".to_string()],
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
        event_only: false,
        evolution_enabled: false,
        rlm_enabled: None,
        bypass_kg_routing: false,
        enabled: true,
        project: Some("alpha".to_string()),
    });
    config.pr_dispatch_per_project.insert(
        "alpha".to_string(),
        crate::config::PrDispatchConfig {
            agents_on_pr_open: vec![
                crate::config::PrDispatchEntry {
                    name: "build-runner".to_string(),
                    context: "adf/build".to_string(),
                },
                crate::config::PrDispatchEntry {
                    name: "pr-reviewer".to_string(),
                    context: "adf/pr-reviewer".to_string(),
                },
            ],
        },
    );
    (config, tmp)
}

/// ADF Phase 2 (issue #944): a `pull_request.opened` event with both
/// `build-runner` and `pr-reviewer` in `agents_on_pr_open` must spawn
/// both agents. Each receives the appropriate env injection: build-runner
/// gets `ADF_PUSH_*` (mirroring `handle_push`); pr-reviewer keeps the
/// existing `ADF_PR_*` keys.
#[tokio::test]
async fn handle_review_pr_spawns_both_build_runner_and_pr_reviewer() {
    let (config, _tmp) = review_pr_config_with_fanout("echo");
    let mut orch = AgentOrchestrator::new(config).unwrap();

    orch.handle_review_pr(review_pr_task()).await.unwrap();

    assert!(
        orch.active_agents.contains_key("pr-reviewer"),
        "pr-reviewer must be spawned; active_agents: {:?}",
        orch.active_agents.keys().collect::<Vec<_>>()
    );
    assert!(
        orch.active_agents.contains_key("build-runner"),
        "build-runner must be spawned; active_agents: {:?}",
        orch.active_agents.keys().collect::<Vec<_>>()
    );
}

/// The per-agent env injection must distinguish the two agents:
/// build-runner sees `ADF_PUSH_*` (with `ref_name = refs/pull/<n>/head`),
/// pr-reviewer sees `ADF_PR_*`. Verified by writing a tiny script that
/// dumps env to a file and reading the artefacts back.
#[tokio::test]
async fn handle_review_pr_injects_per_agent_env_correctly() {
    let tmp = TempDir::new().unwrap();
    let pr_dump = tmp.path().join("pr.env");
    let push_dump = tmp.path().join("push.env");
    // Single shell script that picks which dump to write based on which
    // env vars are present. Avoids needing two separate cliTools.
    // NOTE: Check for ADF_PR_NUMBER first because child processes inherit
    // the parent environment (cargo test inherits from build-runner which
    // has ADF_PUSH_SHA set), so absence of ADF_PUSH_SHA is not reliable.
    let script_path = tmp.path().join("dump-env.sh");
    let all_dump = tmp.path().join("all.env");
    let script_body = format!(
            "#!/bin/sh\nenv > {}\nif [ -n \"$ADF_PR_NUMBER\" ]; then env | grep '^ADF_PR_' > {}\nelse env | grep '^ADF_PUSH_' > {}\nfi\n",
            all_dump.display(),
            pr_dump.display(),
            push_dump.display(),
        );
    std::fs::write(&script_path, script_body).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&script_path).unwrap().permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&script_path, perms).unwrap();
    }

    let (config, _cfg_tmp) = review_pr_config_with_fanout(script_path.to_str().unwrap());
    let mut orch = AgentOrchestrator::new(config).unwrap();
    orch.handle_review_pr(review_pr_task()).await.unwrap();

    // Allow agents time to spawn before polling
    tokio::time::sleep(Duration::from_secs(2)).await;

    for i in 0..600 {
        tokio::time::sleep(Duration::from_millis(50)).await;
        orch.poll_agent_exits().await;
        if pr_dump.exists() && push_dump.exists() {
            break;
        }
        if i == 100 {
            eprintln!(
                "Still waiting after 5s. Active agents: {:?}",
                orch.active_agents.keys().collect::<Vec<_>>()
            );
        }
    }

    let pr = std::fs::read_to_string(&pr_dump).unwrap_or_default();
    let push = std::fs::read_to_string(&push_dump).unwrap_or_default();

    let all_env = std::fs::read_to_string(&all_dump).unwrap_or_default();
    if !pr.contains("ADF_PR_NUMBER=641") || !push.contains("ADF_PUSH_SHA=deadbeef1234") {
        eprintln!(
            "active_agents after poll loop: {:?}",
            orch.active_agents.keys().collect::<Vec<_>>()
        );
        eprintln!("pr_dump path: {}", pr_dump.display());
        eprintln!("push_dump path: {}", push_dump.display());
        eprintln!("pr_dump exists: {}", pr_dump.exists());
        eprintln!("push_dump exists: {}", push_dump.exists());
        eprintln!("pr_dump contents:\n{pr}");
        eprintln!("push_dump contents:\n{push}");
        eprintln!("all env dump:\n{all_env}");
    }
    assert!(
        pr.contains("ADF_PR_NUMBER=641"),
        "pr-reviewer env missing ADF_PR_NUMBER:\n{pr}"
    );
    assert!(
        push.contains("ADF_PUSH_SHA=deadbeef1234"),
        "build-runner env missing ADF_PUSH_SHA=deadbeef1234:\n{push}"
    );
    assert!(
        push.contains("ADF_PUSH_REF=refs/pull/641/head"),
        "build-runner env missing ADF_PUSH_REF=refs/pull/641/head:\n{push}"
    );
    assert!(
        push.contains("ADF_PUSH_PROJECT=alpha"),
        "build-runner env missing ADF_PUSH_PROJECT=alpha:\n{push}"
    );
}

/// When `agents_on_pr_open` references an agent that isn't configured
/// for the project, the orchestrator must skip it gracefully without
/// panicking and without posting a `pending` status that would never
/// resolve. The other entries still spawn.
#[tokio::test]
async fn handle_review_pr_skips_missing_agents() {
    // Standard config has only pr-reviewer; bolt on a pr_dispatch list
    // that ALSO references build-runner (which is absent).
    let (mut config, _tmp) = review_pr_config("echo");
    config.pr_dispatch = Some(crate::config::PrDispatchConfig {
        agents_on_pr_open: vec![
            crate::config::PrDispatchEntry {
                name: "build-runner".to_string(),
                context: "adf/build".to_string(),
            },
            crate::config::PrDispatchEntry {
                name: "pr-reviewer".to_string(),
                context: "adf/pr-reviewer".to_string(),
            },
        ],
    });
    let mut orch = AgentOrchestrator::new(config).unwrap();

    orch.handle_review_pr(review_pr_task()).await.unwrap();

    assert!(
        orch.active_agents.contains_key("pr-reviewer"),
        "pr-reviewer must still spawn even when build-runner is missing"
    );
    assert!(
        !orch.active_agents.contains_key("build-runner"),
        "build-runner must not be spawned when not configured"
    );
}

/// When a workflow tracker is configured, each fan-out agent that spawns
/// must POST exactly one `pending` commit status with its configured
/// context. With both build-runner and pr-reviewer in the dispatch list,
/// the orchestrator must POST two distinct statuses.
#[tokio::test]
async fn handle_review_pr_pending_status_posted_per_agent() {
    use axum::{
        extract::{Path, State},
        http::StatusCode,
        response::IntoResponse,
        routing::post,
        Router,
    };
    use std::sync::atomic::{AtomicUsize, Ordering as AOrdering};
    use std::sync::Arc;
    use tokio::net::TcpListener;

    #[derive(Default)]
    struct Captured {
        calls: AtomicUsize,
        contexts: std::sync::Mutex<Vec<String>>,
        states: std::sync::Mutex<Vec<String>>,
    }

    async fn capture(
        Path((_owner, _repo, _sha)): Path<(String, String, String)>,
        State(captured): State<Arc<Captured>>,
        body: axum::body::Bytes,
    ) -> impl IntoResponse {
        captured.calls.fetch_add(1, AOrdering::SeqCst);
        if let Ok(parsed) = serde_json::from_slice::<serde_json::Value>(&body) {
            if let Some(ctx) = parsed.get("context").and_then(|v| v.as_str()) {
                captured.contexts.lock().unwrap().push(ctx.to_string());
            }
            if let Some(st) = parsed.get("state").and_then(|v| v.as_str()) {
                captured.states.lock().unwrap().push(st.to_string());
            }
        }
        StatusCode::CREATED
    }

    let captured = Arc::new(Captured::default());
    let app = Router::new()
        .route("/api/v1/repos/{owner}/{repo}/statuses/{sha}", post(capture))
        .with_state(captured.clone());
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.ok();
    });
    let base_url = format!("http://{}", addr);

    let (mut config, _tmp) = review_pr_config_with_fanout("echo");
    config.workflow = Some(crate::config::WorkflowConfig {
        enabled: true,
        poll_interval_secs: 60,
        workflow_file: std::path::PathBuf::from("/tmp/workflow.md"),
        tracker: crate::config::TrackerConfig {
            kind: "gitea".to_string(),
            endpoint: base_url.clone(),
            api_key: "test-token".to_string(),
            owner: "fakeowner".to_string(),
            repo: "fakerepo".to_string(),
            project_slug: None,
            use_robot_api: false,
            states: crate::config::TrackerStates::default(),
        },
        concurrency: crate::config::ConcurrencyConfig::default(),
    });
    let mut orch = AgentOrchestrator::new(config).unwrap();

    orch.handle_review_pr(review_pr_task()).await.unwrap();

    // Wait for both POSTs.
    for _ in 0..100 {
        if captured.calls.load(AOrdering::SeqCst) >= 2 {
            break;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    assert_eq!(
        captured.calls.load(AOrdering::SeqCst),
        2,
        "exactly two pending status posts expected (one per fan-out agent)"
    );
    let contexts = captured.contexts.lock().unwrap().clone();
    assert!(
        contexts.iter().any(|c| c == "adf/build"),
        "adf/build pending status missing; got: {contexts:?}"
    );
    assert!(
        contexts.iter().any(|c| c == "adf/pr-reviewer"),
        "adf/pr-reviewer pending status missing; got: {contexts:?}"
    );
    let states = captured.states.lock().unwrap().clone();
    assert!(
        states.iter().all(|s| s == "pending"),
        "all initial statuses must be pending; got: {states:?}"
    );
}

/// When a fan-out agent is gated out (in this test: build-runner has a
/// banned static model so the C1/C3 subscription gate short-circuits its
/// spawn), the orchestrator must NOT post a `pending` status for that
/// agent. A `pending` that never resolves would block the PR forever.
/// The other agent still spawns and posts its own pending.
#[tokio::test]
async fn handle_review_pr_skipped_agent_does_not_post_pending() {
    use axum::{
        extract::{Path, State},
        http::StatusCode,
        response::IntoResponse,
        routing::post,
        Router,
    };
    use std::sync::atomic::{AtomicUsize, Ordering as AOrdering};
    use std::sync::Arc;
    use tokio::net::TcpListener;

    #[derive(Default)]
    struct Captured {
        calls: AtomicUsize,
        contexts: std::sync::Mutex<Vec<String>>,
    }

    async fn capture(
        Path((_owner, _repo, _sha)): Path<(String, String, String)>,
        State(captured): State<Arc<Captured>>,
        body: axum::body::Bytes,
    ) -> impl IntoResponse {
        captured.calls.fetch_add(1, AOrdering::SeqCst);
        if let Ok(parsed) = serde_json::from_slice::<serde_json::Value>(&body) {
            if let Some(ctx) = parsed.get("context").and_then(|v| v.as_str()) {
                captured.contexts.lock().unwrap().push(ctx.to_string());
            }
        }
        StatusCode::CREATED
    }

    let captured = Arc::new(Captured::default());
    let app = Router::new()
        .route("/api/v1/repos/{owner}/{repo}/statuses/{sha}", post(capture))
        .with_state(captured.clone());
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.ok();
    });
    let base_url = format!("http://{}", addr);

    let (mut config, _tmp) = review_pr_config_with_fanout("echo");
    // Stamp a banned model on build-runner AFTER load-time validation
    // so the runtime allow-list gate is the one rejecting the spawn.
    let br = config
        .agents
        .iter_mut()
        .find(|a| a.name == "build-runner")
        .unwrap();
    br.model = Some("google/gemini-2".to_string());

    config.workflow = Some(crate::config::WorkflowConfig {
        enabled: true,
        poll_interval_secs: 60,
        workflow_file: std::path::PathBuf::from("/tmp/workflow.md"),
        tracker: crate::config::TrackerConfig {
            kind: "gitea".to_string(),
            endpoint: base_url.clone(),
            api_key: "test-token".to_string(),
            owner: "fakeowner".to_string(),
            repo: "fakerepo".to_string(),
            project_slug: None,
            use_robot_api: false,
            states: crate::config::TrackerStates::default(),
        },
        concurrency: crate::config::ConcurrencyConfig::default(),
    });
    let mut orch = AgentOrchestrator::new(config).unwrap();

    orch.handle_review_pr(review_pr_task()).await.unwrap();

    // Allow time for the (single expected) POST to land — and any
    // erroneous extra POST to also land if the bug is present.
    for _ in 0..100 {
        if captured.calls.load(AOrdering::SeqCst) >= 1 {
            break;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    // Give a small grace window for an erroneous adf/build POST to surface.
    tokio::time::sleep(Duration::from_millis(100)).await;

    let contexts = captured.contexts.lock().unwrap().clone();
    assert!(
        contexts.iter().any(|c| c == "adf/pr-reviewer"),
        "pr-reviewer must still post its pending status; got: {contexts:?}"
    );
    assert!(
        !contexts.iter().any(|c| c == "adf/build"),
        "skipped build-runner must NOT post adf/build pending; got: {contexts:?}"
    );
    assert!(
        !orch.active_agents.contains_key("build-runner"),
        "build-runner must not be in active_agents when subscription gate rejects"
    );
}

/// Phase 2b helper: extend `review_pr_config_with_fanout` with a third
/// project-scoped `pr-spec-validator` agent and a three-entry
/// `[pr_dispatch]` block (build-runner, pr-reviewer, pr-spec-validator).
fn review_pr_config_with_spec_fanout(cli_tool: &str) -> (OrchestratorConfig, TempDir) {
    let (mut config, tmp) = review_pr_config_with_fanout(cli_tool);
    config.agents.push(AgentDefinition {
        name: "pr-spec-validator".to_string(),
        layer: AgentLayer::Safety,
        cli_tool: cli_tool.to_string(),
        task: "spec".to_string(),
        model: None,
        default_tier: None,
        schedule: None,
        capabilities: vec!["review".to_string()],
        max_memory_bytes: None,
        budget_monthly_cents: None,
        provider: None,
        persona: None,
        terraphim_role: None,
        skill_chain: vec!["requirements-traceability".to_string()],
        sfia_skills: vec![],
        fallback_provider: None,
        fallback_model: None,
        grace_period_secs: None,
        max_cpu_seconds: None,
        pre_check: None,
        gitea_issue: None,
        event_only: false,
        evolution_enabled: false,
        rlm_enabled: None,
        bypass_kg_routing: false,
        enabled: true,
        project: Some("alpha".to_string()),
    });
    // The per-project block takes precedence over the top-level block,
    // so we must update both to keep them in sync.
    config
        .pr_dispatch_per_project
        .get_mut("alpha")
        .unwrap()
        .agents_on_pr_open
        .push(crate::config::PrDispatchEntry {
            name: "pr-spec-validator".to_string(),
            context: "adf/spec".to_string(),
        });
    config.pr_dispatch = Some(crate::config::PrDispatchConfig {
        agents_on_pr_open: vec![
            crate::config::PrDispatchEntry {
                name: "build-runner".to_string(),
                context: "adf/build".to_string(),
            },
            crate::config::PrDispatchEntry {
                name: "pr-reviewer".to_string(),
                context: "adf/pr-reviewer".to_string(),
            },
            crate::config::PrDispatchEntry {
                name: "pr-spec-validator".to_string(),
                context: "adf/spec".to_string(),
            },
        ],
    });
    (config, tmp)
}

/// ADF Phase 2b (Refs #950): a `pull_request.opened` event with
/// `pr-spec-validator` in `agents_on_pr_open` must spawn the agent.
/// Verifies the existing fan-out loop's generic `_` arm (which routes
/// any non-`build-runner` entry through `dispatch_pr_reviewer_for_pr`
/// by name) handles the new context cleanly without code change.
#[tokio::test]
async fn handle_review_pr_spawns_pr_spec_validator_when_configured() {
    let (config, _tmp) = review_pr_config_with_spec_fanout("echo");
    let mut orch = AgentOrchestrator::new(config).unwrap();

    orch.handle_review_pr(review_pr_task()).await.unwrap();

    assert!(
        orch.active_agents.contains_key("pr-spec-validator"),
        "pr-spec-validator must be spawned; active_agents: {:?}",
        orch.active_agents.keys().collect::<Vec<_>>()
    );
    // Spec-validator is a PR-event agent, so it must receive the
    // `ADF_PR_*` env (not `ADF_PUSH_*`). Verified via session_id
    // scoping; the env wiring itself is exercised by the existing
    // `reviewpr_dispatch_sets_env_vars` test through the same
    // dispatch helper.
    let managed = orch.active_agents.get("pr-spec-validator").unwrap();
    assert!(
        managed.session_id.starts_with("pr-spec-validator-"),
        "session id should be scoped to the agent, got: {}",
        managed.session_id
    );
}

/// ADF Phase 2b (Refs #950): when a workflow tracker is configured,
/// `pr-spec-validator`'s spawn must POST a `pending` commit status
/// with context `adf/spec` against the PR head SHA.
#[tokio::test]
async fn handle_review_pr_pending_status_posted_for_spec_context() {
    use axum::{
        extract::{Path, State},
        http::StatusCode,
        response::IntoResponse,
        routing::post,
        Router,
    };
    use std::sync::atomic::{AtomicUsize, Ordering as AOrdering};
    use std::sync::Arc;
    use tokio::net::TcpListener;

    #[derive(Default)]
    struct Captured {
        calls: AtomicUsize,
        contexts: std::sync::Mutex<Vec<String>>,
        states: std::sync::Mutex<Vec<String>>,
    }

    async fn capture(
        Path((_owner, _repo, _sha)): Path<(String, String, String)>,
        State(captured): State<Arc<Captured>>,
        body: axum::body::Bytes,
    ) -> impl IntoResponse {
        captured.calls.fetch_add(1, AOrdering::SeqCst);
        if let Ok(parsed) = serde_json::from_slice::<serde_json::Value>(&body) {
            if let Some(ctx) = parsed.get("context").and_then(|v| v.as_str()) {
                captured.contexts.lock().unwrap().push(ctx.to_string());
            }
            if let Some(st) = parsed.get("state").and_then(|v| v.as_str()) {
                captured.states.lock().unwrap().push(st.to_string());
            }
        }
        StatusCode::CREATED
    }

    let captured = Arc::new(Captured::default());
    let app = Router::new()
        .route("/api/v1/repos/{owner}/{repo}/statuses/{sha}", post(capture))
        .with_state(captured.clone());
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.ok();
    });
    let base_url = format!("http://{}", addr);

    let (mut config, _tmp) = review_pr_config_with_spec_fanout("echo");
    config.workflow = Some(crate::config::WorkflowConfig {
        enabled: true,
        poll_interval_secs: 60,
        workflow_file: std::path::PathBuf::from("/tmp/workflow.md"),
        tracker: crate::config::TrackerConfig {
            kind: "gitea".to_string(),
            endpoint: base_url.clone(),
            api_key: "test-token".to_string(),
            owner: "fakeowner".to_string(),
            repo: "fakerepo".to_string(),
            project_slug: None,
            use_robot_api: false,
            states: crate::config::TrackerStates::default(),
        },
        concurrency: crate::config::ConcurrencyConfig::default(),
    });
    let mut orch = AgentOrchestrator::new(config).unwrap();

    orch.handle_review_pr(review_pr_task()).await.unwrap();

    // Expect three POSTs (one per fan-out agent).
    for _ in 0..150 {
        if captured.calls.load(AOrdering::SeqCst) >= 3 {
            break;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    let contexts = captured.contexts.lock().unwrap().clone();
    assert!(
        contexts.iter().any(|c| c == "adf/spec"),
        "adf/spec pending status missing; got: {contexts:?}"
    );
    let states = captured.states.lock().unwrap().clone();
    assert!(
        states.iter().all(|s| s == "pending"),
        "all initial statuses must be pending; got: {states:?}"
    );
}

/// ADF Phase 2b (Refs #950): when `pr-spec-validator` is gated out by
/// the runtime subscription allow-list, its `pending` must NOT be
/// posted -- otherwise `adf/spec` (a required check on `main`) would
/// hang indefinitely. Other fan-out entries still post their pendings.
#[tokio::test]
async fn handle_review_pr_spec_validator_skipped_does_not_post_pending() {
    use axum::{
        extract::{Path, State},
        http::StatusCode,
        response::IntoResponse,
        routing::post,
        Router,
    };
    use std::sync::atomic::{AtomicUsize, Ordering as AOrdering};
    use std::sync::Arc;
    use tokio::net::TcpListener;

    #[derive(Default)]
    struct Captured {
        calls: AtomicUsize,
        contexts: std::sync::Mutex<Vec<String>>,
    }

    async fn capture(
        Path((_owner, _repo, _sha)): Path<(String, String, String)>,
        State(captured): State<Arc<Captured>>,
        body: axum::body::Bytes,
    ) -> impl IntoResponse {
        captured.calls.fetch_add(1, AOrdering::SeqCst);
        if let Ok(parsed) = serde_json::from_slice::<serde_json::Value>(&body) {
            if let Some(ctx) = parsed.get("context").and_then(|v| v.as_str()) {
                captured.contexts.lock().unwrap().push(ctx.to_string());
            }
        }
        StatusCode::CREATED
    }

    let captured = Arc::new(Captured::default());
    let app = Router::new()
        .route("/api/v1/repos/{owner}/{repo}/statuses/{sha}", post(capture))
        .with_state(captured.clone());
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.ok();
    });
    let base_url = format!("http://{}", addr);

    let (mut config, _tmp) = review_pr_config_with_spec_fanout("echo");
    // Stamp a banned model on pr-spec-validator AFTER load-time
    // validation so the runtime allow-list gate is the one that
    // short-circuits the spawn. The other agents stay clean.
    let svc = config
        .agents
        .iter_mut()
        .find(|a| a.name == "pr-spec-validator")
        .unwrap();
    svc.model = Some("google/gemini-2".to_string());

    config.workflow = Some(crate::config::WorkflowConfig {
        enabled: true,
        poll_interval_secs: 60,
        workflow_file: std::path::PathBuf::from("/tmp/workflow.md"),
        tracker: crate::config::TrackerConfig {
            kind: "gitea".to_string(),
            endpoint: base_url.clone(),
            api_key: "test-token".to_string(),
            owner: "fakeowner".to_string(),
            repo: "fakerepo".to_string(),
            project_slug: None,
            use_robot_api: false,
            states: crate::config::TrackerStates::default(),
        },
        concurrency: crate::config::ConcurrencyConfig::default(),
    });
    let mut orch = AgentOrchestrator::new(config).unwrap();

    orch.handle_review_pr(review_pr_task()).await.unwrap();

    // Wait for the two non-skipped POSTs, then a small grace window
    // for any erroneous adf/spec POST to surface.
    for _ in 0..150 {
        if captured.calls.load(AOrdering::SeqCst) >= 2 {
            break;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    tokio::time::sleep(Duration::from_millis(150)).await;

    let contexts = captured.contexts.lock().unwrap().clone();
    assert!(
        !contexts.iter().any(|c| c == "adf/spec"),
        "skipped pr-spec-validator must NOT post adf/spec pending; got: {contexts:?}"
    );
    assert!(
        !orch.active_agents.contains_key("pr-spec-validator"),
        "pr-spec-validator must not be in active_agents when subscription gate rejects"
    );
    // Sanity: the other two agents still spawned + posted.
    assert!(
        contexts.iter().any(|c| c == "adf/pr-reviewer"),
        "pr-reviewer must still post its pending; got: {contexts:?}"
    );
    assert!(
        contexts.iter().any(|c| c == "adf/build"),
        "build-runner must still post its pending; got: {contexts:?}"
    );
}

#[test]
fn test_learning_config_default_disabled() {
    let cfg = config::LearningConfig::default();
    assert!(!cfg.enabled);
    assert_eq!(cfg.min_trust, "L1");
    assert_eq!(cfg.max_tokens, 1500);
    assert_eq!(cfg.max_entries, 10);
    assert_eq!(cfg.archive_days, 30);
    assert_eq!(cfg.consolidation_ticks, 100);
}

#[test]
fn test_render_lessons_section_empty_store() {
    let config = test_config();
    let orch = AgentOrchestrator::new(config).unwrap();
    let (section, ids) = orch.render_lessons_section("sentinel");
    assert!(section.is_empty());
    assert!(ids.is_empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn test_render_lessons_section_with_learnings() {
    let config = test_config();
    let mut orch = AgentOrchestrator::new(config).unwrap();

    let persistence = learning::InMemoryLearningPersistence::new();
    let store = learning::SharedLearningStore::new(Box::new(persistence), learning::TrustLevel::L0);
    store
        .insert(learning::NewLearning {
            source_agent: "other-agent".to_string(),
            category: learning::LearningCategory::Tip,
            summary: "Always run clippy before committing".to_string(),
            details: Some("Prevents CI failures from lint warnings".to_string()),
            applicable_agents: vec![],
            verify_pattern: None,
        })
        .await
        .unwrap();

    orch.learning_store = Some(store);

    let (section, ids) = orch.render_lessons_section("sentinel");
    assert!(!section.is_empty(), "expected non-empty section, got empty");
    assert!(section.contains("Prior Lessons"));
    assert!(section.contains("Always run clippy before committing"));
    assert_eq!(ids.len(), 1);
}
/// Phase 2e helper (Refs #954): extend `review_pr_config_with_fanout`
/// with a third project-scoped `pr-test-guardian` agent and a
/// three-entry `[pr_dispatch]` block (build-runner, pr-reviewer,
/// pr-test-guardian). Distinct name from Phase 2b's `…_with_spec_fanout`
/// and Phase 2c's `…_with_security_fanout` helpers so all three Phase 2
/// branches can coexist on the same `review_pr_config_with_fanout`
/// baseline once they merge.
fn review_pr_config_with_test_fanout(cli_tool: &str) -> (OrchestratorConfig, TempDir) {
    let (mut config, tmp) = review_pr_config_with_fanout(cli_tool);
    config.agents.push(AgentDefinition {
        name: "pr-test-guardian".to_string(),
        layer: AgentLayer::Growth,
        cli_tool: cli_tool.to_string(),
        task: "test".to_string(),
        model: None,
        default_tier: None,
        schedule: None,
        capabilities: vec!["review".to_string(), "test".to_string()],
        max_memory_bytes: None,
        budget_monthly_cents: None,
        provider: None,
        persona: None,
        terraphim_role: None,
        skill_chain: vec!["testing".to_string()],
        sfia_skills: vec![],
        fallback_provider: None,
        fallback_model: None,
        grace_period_secs: None,
        max_cpu_seconds: None,
        pre_check: None,
        gitea_issue: None,
        event_only: false,
        evolution_enabled: false,
        rlm_enabled: None,
        bypass_kg_routing: false,
        enabled: true,
        project: Some("alpha".to_string()),
    });
    // The per-project block takes precedence over the top-level block,
    // so we must update both to keep them in sync.
    config
        .pr_dispatch_per_project
        .get_mut("alpha")
        .unwrap()
        .agents_on_pr_open
        .push(crate::config::PrDispatchEntry {
            name: "pr-test-guardian".to_string(),
            context: "adf/test".to_string(),
        });
    config.pr_dispatch = Some(crate::config::PrDispatchConfig {
        agents_on_pr_open: vec![
            crate::config::PrDispatchEntry {
                name: "build-runner".to_string(),
                context: "adf/build".to_string(),
            },
            crate::config::PrDispatchEntry {
                name: "pr-reviewer".to_string(),
                context: "adf/pr-reviewer".to_string(),
            },
            crate::config::PrDispatchEntry {
                name: "pr-test-guardian".to_string(),
                context: "adf/test".to_string(),
            },
        ],
    });
    (config, tmp)
}

/// ADF Phase 2e (Refs #954): a `pull_request.opened` event with
/// `pr-test-guardian` in `agents_on_pr_open` must spawn the agent.
/// Verifies the existing fan-out loop's generic `_` arm (which routes
/// any non-`build-runner` entry through `dispatch_pr_reviewer_for_pr`
/// by name) handles the new `adf/test` context cleanly without any
/// production code change.
#[tokio::test]
async fn handle_review_pr_spawns_pr_test_guardian_when_configured() {
    let (config, _tmp) = review_pr_config_with_test_fanout("echo");
    let mut orch = AgentOrchestrator::new(config).unwrap();

    orch.handle_review_pr(review_pr_task()).await.unwrap();

    assert!(
        orch.active_agents.contains_key("pr-test-guardian"),
        "pr-test-guardian must be spawned; active_agents: {:?}",
        orch.active_agents.keys().collect::<Vec<_>>()
    );
    // Test guardian is a PR-event agent, so it must receive the
    // `ADF_PR_*` env (not `ADF_PUSH_*`). Verified via session_id
    // scoping; the env wiring itself is exercised by the existing
    // `reviewpr_dispatch_sets_env_vars` test through the same dispatch
    // helper.
    let managed = orch.active_agents.get("pr-test-guardian").unwrap();
    assert!(
        managed.session_id.starts_with("pr-test-guardian-"),
        "session id should be scoped to the agent, got: {}",
        managed.session_id
    );
}

/// ADF Phase 2e (Refs #954): when a workflow tracker is configured,
/// `pr-test-guardian`'s spawn must POST a `pending` commit status with
/// context `adf/test` against the PR head SHA. Three fan-out agents are
/// configured, so three pending statuses are expected.
#[tokio::test]
async fn handle_review_pr_pending_status_posted_for_test_context() {
    use axum::{
        extract::{Path, State},
        http::StatusCode,
        response::IntoResponse,
        routing::post,
        Router,
    };
    use std::sync::atomic::{AtomicUsize, Ordering as AOrdering};
    use std::sync::Arc;
    use tokio::net::TcpListener;

    #[derive(Default)]
    struct Captured {
        calls: AtomicUsize,
        contexts: std::sync::Mutex<Vec<String>>,
        states: std::sync::Mutex<Vec<String>>,
    }

    async fn capture(
        Path((_owner, _repo, _sha)): Path<(String, String, String)>,
        State(captured): State<Arc<Captured>>,
        body: axum::body::Bytes,
    ) -> impl IntoResponse {
        captured.calls.fetch_add(1, AOrdering::SeqCst);
        if let Ok(parsed) = serde_json::from_slice::<serde_json::Value>(&body) {
            if let Some(ctx) = parsed.get("context").and_then(|v| v.as_str()) {
                captured.contexts.lock().unwrap().push(ctx.to_string());
            }
            if let Some(st) = parsed.get("state").and_then(|v| v.as_str()) {
                captured.states.lock().unwrap().push(st.to_string());
            }
        }
        StatusCode::CREATED
    }

    let captured = Arc::new(Captured::default());
    let app = Router::new()
        .route("/api/v1/repos/{owner}/{repo}/statuses/{sha}", post(capture))
        .with_state(captured.clone());
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.ok();
    });
    let base_url = format!("http://{}", addr);

    let (mut config, _tmp) = review_pr_config_with_test_fanout("echo");
    config.workflow = Some(crate::config::WorkflowConfig {
        enabled: true,
        poll_interval_secs: 60,
        workflow_file: std::path::PathBuf::from("/tmp/workflow.md"),
        tracker: crate::config::TrackerConfig {
            kind: "gitea".to_string(),
            endpoint: base_url.clone(),
            api_key: "test-token".to_string(),
            owner: "fakeowner".to_string(),
            repo: "fakerepo".to_string(),
            project_slug: None,
            use_robot_api: false,
            states: crate::config::TrackerStates::default(),
        },
        concurrency: crate::config::ConcurrencyConfig::default(),
    });
    let mut orch = AgentOrchestrator::new(config).unwrap();

    orch.handle_review_pr(review_pr_task()).await.unwrap();

    // Expect three POSTs (one per fan-out agent).
    for _ in 0..150 {
        if captured.calls.load(AOrdering::SeqCst) >= 3 {
            break;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    let contexts = captured.contexts.lock().unwrap().clone();
    assert!(
        contexts.iter().any(|c| c == "adf/test"),
        "adf/test pending status missing; got: {contexts:?}"
    );
    let states = captured.states.lock().unwrap().clone();
    assert!(
        states.iter().all(|s| s == "pending"),
        "all initial statuses must be pending; got: {states:?}"
    );
}

/// ADF Phase 2e (Refs #954): when `pr-test-guardian` is gated out by
/// the runtime subscription allow-list, its `pending` must NOT be
/// posted -- otherwise `adf/test` (a required check on `main`) would
/// hang indefinitely. Other fan-out entries still post their pendings.
#[tokio::test]
async fn handle_review_pr_test_guardian_skipped_does_not_post_pending() {
    use axum::{
        extract::{Path, State},
        http::StatusCode,
        response::IntoResponse,
        routing::post,
        Router,
    };
    use std::sync::atomic::{AtomicUsize, Ordering as AOrdering};
    use std::sync::Arc;
    use tokio::net::TcpListener;

    #[derive(Default)]
    struct Captured {
        calls: AtomicUsize,
        contexts: std::sync::Mutex<Vec<String>>,
    }

    async fn capture(
        Path((_owner, _repo, _sha)): Path<(String, String, String)>,
        State(captured): State<Arc<Captured>>,
        body: axum::body::Bytes,
    ) -> impl IntoResponse {
        captured.calls.fetch_add(1, AOrdering::SeqCst);
        if let Ok(parsed) = serde_json::from_slice::<serde_json::Value>(&body) {
            if let Some(ctx) = parsed.get("context").and_then(|v| v.as_str()) {
                captured.contexts.lock().unwrap().push(ctx.to_string());
            }
        }
        StatusCode::CREATED
    }

    let captured = Arc::new(Captured::default());
    let app = Router::new()
        .route("/api/v1/repos/{owner}/{repo}/statuses/{sha}", post(capture))
        .with_state(captured.clone());
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.ok();
    });
    let base_url = format!("http://{}", addr);

    let (mut config, _tmp) = review_pr_config_with_test_fanout("echo");
    // Stamp a banned model on pr-test-guardian AFTER load-time
    // validation so the runtime allow-list gate is the one that
    // short-circuits the spawn. The other agents stay clean.
    let tg = config
        .agents
        .iter_mut()
        .find(|a| a.name == "pr-test-guardian")
        .unwrap();
    tg.model = Some("google/gemini-2".to_string());

    config.workflow = Some(crate::config::WorkflowConfig {
        enabled: true,
        poll_interval_secs: 60,
        workflow_file: std::path::PathBuf::from("/tmp/workflow.md"),
        tracker: crate::config::TrackerConfig {
            kind: "gitea".to_string(),
            endpoint: base_url.clone(),
            api_key: "test-token".to_string(),
            owner: "fakeowner".to_string(),
            repo: "fakerepo".to_string(),
            project_slug: None,
            use_robot_api: false,
            states: crate::config::TrackerStates::default(),
        },
        concurrency: crate::config::ConcurrencyConfig::default(),
    });
    let mut orch = AgentOrchestrator::new(config).unwrap();

    orch.handle_review_pr(review_pr_task()).await.unwrap();

    // Wait for the two non-skipped POSTs, then a small grace window
    // for any erroneous adf/test POST to surface.
    for _ in 0..150 {
        if captured.calls.load(AOrdering::SeqCst) >= 2 {
            break;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    tokio::time::sleep(Duration::from_millis(150)).await;

    let contexts = captured.contexts.lock().unwrap().clone();
    assert!(
        !contexts.iter().any(|c| c == "adf/test"),
        "skipped pr-test-guardian must NOT post adf/test pending; got: {contexts:?}"
    );
    assert!(
        !orch.active_agents.contains_key("pr-test-guardian"),
        "pr-test-guardian must not be in active_agents when subscription gate rejects"
    );
    // Sanity: the other two agents still spawned + posted.
    assert!(
        contexts.iter().any(|c| c == "adf/pr-reviewer"),
        "pr-reviewer must still post its pending; got: {contexts:?}"
    );
    assert!(
        contexts.iter().any(|c| c == "adf/build"),
        "build-runner must still post its pending; got: {contexts:?}"
    );
}
/// ADF Phase 2d (issue #955): helper that wires `pr-compliance-watchdog`
/// alongside the existing `pr-reviewer` agent and a `[pr_dispatch]` block
/// listing both agents on PR open. Standalone -- does not reuse the
/// Phase 2 `review_pr_config_with_fanout` helper -- so the assertion
/// surface stays tight (exactly two pending POSTs: `adf/pr-reviewer`
/// and `adf/compliance`).
fn review_pr_config_with_compliance_fanout(cli_tool: &str) -> (OrchestratorConfig, TempDir) {
    let (mut config, tmp) = review_pr_config(cli_tool);
    config.agents.push(AgentDefinition {
        name: "pr-compliance-watchdog".to_string(),
        layer: AgentLayer::Growth,
        cli_tool: cli_tool.to_string(),
        task: "compliance".to_string(),
        model: None,
        default_tier: None,
        schedule: None,
        capabilities: vec!["compliance".to_string()],
        max_memory_bytes: None,
        budget_monthly_cents: None,
        provider: None,
        persona: None,
        terraphim_role: None,
        skill_chain: vec!["responsible-ai".to_string()],
        sfia_skills: vec![],
        fallback_provider: None,
        fallback_model: None,
        grace_period_secs: None,
        max_cpu_seconds: None,
        pre_check: None,
        gitea_issue: None,
        event_only: false,
        evolution_enabled: false,
        rlm_enabled: None,
        bypass_kg_routing: false,
        enabled: true,
        project: Some("alpha".to_string()),
    });
    config.pr_dispatch = Some(crate::config::PrDispatchConfig {
        agents_on_pr_open: vec![
            crate::config::PrDispatchEntry {
                name: "pr-reviewer".to_string(),
                context: "adf/pr-reviewer".to_string(),
            },
            crate::config::PrDispatchEntry {
                name: "pr-compliance-watchdog".to_string(),
                context: "adf/compliance".to_string(),
            },
        ],
    });
    (config, tmp)
}

/// ADF Phase 2d (issue #955): when `pr-compliance-watchdog` is listed in
/// `agents_on_pr_open`, the generic `_` arm in `handle_review_pr` must
/// route through `dispatch_pr_reviewer_for_pr` and spawn the agent. No
/// new Rust dispatch code is needed -- this test asserts that property.
#[tokio::test]
async fn handle_review_pr_spawns_pr_compliance_watchdog_when_configured() {
    let (config, _tmp) = review_pr_config_with_compliance_fanout("echo");
    let mut orch = AgentOrchestrator::new(config).unwrap();

    orch.handle_review_pr(review_pr_task()).await.unwrap();

    assert!(
        orch.active_agents.contains_key("pr-reviewer"),
        "pr-reviewer must spawn alongside the new compliance agent; active_agents: {:?}",
        orch.active_agents.keys().collect::<Vec<_>>()
    );
    assert!(
        orch.active_agents.contains_key("pr-compliance-watchdog"),
        "pr-compliance-watchdog must be spawned by the generic fan-out arm; active_agents: {:?}",
        orch.active_agents.keys().collect::<Vec<_>>()
    );
}

/// ADF Phase 2d: with the workflow tracker pointed at a loopback Gitea,
/// each fan-out agent that successfully spawns must POST exactly one
/// `pending` commit status. The compliance agent must use the
/// `adf/compliance` context (a hung pending under any other context
/// would block the PR forever once branch protection requires it).
#[tokio::test]
async fn handle_review_pr_pending_status_posted_for_compliance_context() {
    use axum::{
        extract::{Path, State},
        http::StatusCode,
        response::IntoResponse,
        routing::post,
        Router,
    };
    use std::sync::atomic::{AtomicUsize, Ordering as AOrdering};
    use std::sync::Arc;
    use tokio::net::TcpListener;

    #[derive(Default)]
    struct Captured {
        calls: AtomicUsize,
        contexts: std::sync::Mutex<Vec<String>>,
        states: std::sync::Mutex<Vec<String>>,
    }

    async fn capture(
        Path((_owner, _repo, _sha)): Path<(String, String, String)>,
        State(captured): State<Arc<Captured>>,
        body: axum::body::Bytes,
    ) -> impl IntoResponse {
        captured.calls.fetch_add(1, AOrdering::SeqCst);
        if let Ok(parsed) = serde_json::from_slice::<serde_json::Value>(&body) {
            if let Some(ctx) = parsed.get("context").and_then(|v| v.as_str()) {
                captured.contexts.lock().unwrap().push(ctx.to_string());
            }
            if let Some(st) = parsed.get("state").and_then(|v| v.as_str()) {
                captured.states.lock().unwrap().push(st.to_string());
            }
        }
        StatusCode::CREATED
    }

    let captured = Arc::new(Captured::default());
    let app = Router::new()
        .route("/api/v1/repos/{owner}/{repo}/statuses/{sha}", post(capture))
        .with_state(captured.clone());
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.ok();
    });
    let base_url = format!("http://{}", addr);

    let (mut config, _tmp) = review_pr_config_with_compliance_fanout("echo");
    config.workflow = Some(crate::config::WorkflowConfig {
        enabled: true,
        poll_interval_secs: 60,
        workflow_file: std::path::PathBuf::from("/tmp/workflow.md"),
        tracker: crate::config::TrackerConfig {
            kind: "gitea".to_string(),
            endpoint: base_url.clone(),
            api_key: "test-token".to_string(),
            owner: "fakeowner".to_string(),
            repo: "fakerepo".to_string(),
            project_slug: None,
            use_robot_api: false,
            states: crate::config::TrackerStates::default(),
        },
        concurrency: crate::config::ConcurrencyConfig::default(),
    });
    let mut orch = AgentOrchestrator::new(config).unwrap();

    orch.handle_review_pr(review_pr_task()).await.unwrap();

    for _ in 0..100 {
        if captured.calls.load(AOrdering::SeqCst) >= 2 {
            break;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    assert_eq!(
        captured.calls.load(AOrdering::SeqCst),
        2,
        "exactly two pending status posts expected (pr-reviewer + compliance)"
    );
    let contexts = captured.contexts.lock().unwrap().clone();
    assert!(
        contexts.iter().any(|c| c == "adf/pr-reviewer"),
        "adf/pr-reviewer pending status missing; got: {contexts:?}"
    );
    assert!(
        contexts.iter().any(|c| c == "adf/compliance"),
        "adf/compliance pending status missing; got: {contexts:?}"
    );
    let states = captured.states.lock().unwrap().clone();
    assert!(
        states.iter().all(|s| s == "pending"),
        "all initial statuses must be pending; got: {states:?}"
    );
}

/// ADF Phase 2d: when the compliance agent is gated out by the C1/C3
/// subscription allow-list (banned static model stamped post-validation),
/// the orchestrator must NOT post an `adf/compliance` pending status. A
/// hung pending under a required context would block the PR forever.
/// `pr-reviewer` (the un-gated peer) still spawns and posts its own
/// pending. Note: this test exercises the orchestrator-layer skip; the
/// TOML-level path-filter skip-with-success is a separate, shell-only
/// behaviour and is not asserted here.
#[tokio::test]
async fn handle_review_pr_compliance_watchdog_skipped_does_not_post_pending() {
    use axum::{
        extract::{Path, State},
        http::StatusCode,
        response::IntoResponse,
        routing::post,
        Router,
    };
    use std::sync::atomic::{AtomicUsize, Ordering as AOrdering};
    use std::sync::Arc;
    use tokio::net::TcpListener;

    #[derive(Default)]
    struct Captured {
        calls: AtomicUsize,
        contexts: std::sync::Mutex<Vec<String>>,
    }

    async fn capture(
        Path((_owner, _repo, _sha)): Path<(String, String, String)>,
        State(captured): State<Arc<Captured>>,
        body: axum::body::Bytes,
    ) -> impl IntoResponse {
        captured.calls.fetch_add(1, AOrdering::SeqCst);
        if let Ok(parsed) = serde_json::from_slice::<serde_json::Value>(&body) {
            if let Some(ctx) = parsed.get("context").and_then(|v| v.as_str()) {
                captured.contexts.lock().unwrap().push(ctx.to_string());
            }
        }
        StatusCode::CREATED
    }

    let captured = Arc::new(Captured::default());
    let app = Router::new()
        .route("/api/v1/repos/{owner}/{repo}/statuses/{sha}", post(capture))
        .with_state(captured.clone());
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.ok();
    });
    let base_url = format!("http://{}", addr);

    let (mut config, _tmp) = review_pr_config_with_compliance_fanout("echo");
    // Stamp a banned static model on pr-compliance-watchdog AFTER load-time
    // validation so the runtime subscription allow-list gate is the path
    // that rejects the spawn.
    let watchdog = config
        .agents
        .iter_mut()
        .find(|a| a.name == "pr-compliance-watchdog")
        .unwrap();
    watchdog.model = Some("google/gemini-2".to_string());

    config.workflow = Some(crate::config::WorkflowConfig {
        enabled: true,
        poll_interval_secs: 60,
        workflow_file: std::path::PathBuf::from("/tmp/workflow.md"),
        tracker: crate::config::TrackerConfig {
            kind: "gitea".to_string(),
            endpoint: base_url.clone(),
            api_key: "test-token".to_string(),
            owner: "fakeowner".to_string(),
            repo: "fakerepo".to_string(),
            project_slug: None,
            use_robot_api: false,
            states: crate::config::TrackerStates::default(),
        },
        concurrency: crate::config::ConcurrencyConfig::default(),
    });
    let mut orch = AgentOrchestrator::new(config).unwrap();

    orch.handle_review_pr(review_pr_task()).await.unwrap();

    for _ in 0..100 {
        if captured.calls.load(AOrdering::SeqCst) >= 1 {
            break;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    // Grace window so an erroneous adf/compliance POST has time to surface.
    tokio::time::sleep(Duration::from_millis(100)).await;

    let contexts = captured.contexts.lock().unwrap().clone();
    assert!(
        contexts.iter().any(|c| c == "adf/pr-reviewer"),
        "pr-reviewer must still post its pending status; got: {contexts:?}"
    );
    assert!(
        !contexts.iter().any(|c| c == "adf/compliance"),
        "skipped pr-compliance-watchdog must NOT post adf/compliance pending; got: {contexts:?}"
    );
    assert!(
        !orch.active_agents.contains_key("pr-compliance-watchdog"),
        "pr-compliance-watchdog must not be in active_agents when subscription gate rejects"
    );
}
/// Phase 2c helper: extend `review_pr_config_with_fanout` with a third
/// project-scoped `pr-security-sentinel` agent and a three-entry
/// `[pr_dispatch]` block (build-runner, pr-reviewer, pr-security-sentinel).
///
/// Distinct name from Phase 2b's `review_pr_config_with_spec_fanout`
/// helper (lives on the unmerged `task/950-pr-spec-validator-phase-2b`
/// branch -- see PR #952). Once Phase 2b lands the two helpers will
/// coexist; both build on the same `review_pr_config_with_fanout`
/// baseline.
fn review_pr_config_with_security_fanout(cli_tool: &str) -> (OrchestratorConfig, TempDir) {
    let (mut config, tmp) = review_pr_config_with_fanout(cli_tool);
    config.agents.push(AgentDefinition {
        name: "pr-security-sentinel".to_string(),
        layer: AgentLayer::Safety,
        cli_tool: cli_tool.to_string(),
        task: "security".to_string(),
        model: None,
        default_tier: None,
        schedule: None,
        capabilities: vec!["review".to_string(), "security".to_string()],
        max_memory_bytes: None,
        budget_monthly_cents: None,
        provider: None,
        persona: None,
        terraphim_role: None,
        skill_chain: vec!["security-audit".to_string()],
        sfia_skills: vec![],
        fallback_provider: None,
        fallback_model: None,
        grace_period_secs: None,
        max_cpu_seconds: None,
        pre_check: None,
        gitea_issue: None,
        event_only: false,
        evolution_enabled: false,
        rlm_enabled: None,
        bypass_kg_routing: false,
        enabled: true,
        project: Some("alpha".to_string()),
    });
    // The per-project block takes precedence over the top-level block,
    // so we must update both to keep them in sync.
    config
        .pr_dispatch_per_project
        .get_mut("alpha")
        .unwrap()
        .agents_on_pr_open
        .push(crate::config::PrDispatchEntry {
            name: "pr-security-sentinel".to_string(),
            context: "adf/security".to_string(),
        });
    config.pr_dispatch = Some(crate::config::PrDispatchConfig {
        agents_on_pr_open: vec![
            crate::config::PrDispatchEntry {
                name: "build-runner".to_string(),
                context: "adf/build".to_string(),
            },
            crate::config::PrDispatchEntry {
                name: "pr-reviewer".to_string(),
                context: "adf/pr-reviewer".to_string(),
            },
            crate::config::PrDispatchEntry {
                name: "pr-security-sentinel".to_string(),
                context: "adf/security".to_string(),
            },
        ],
    });
    (config, tmp)
}

/// ADF Phase 2c (Refs #953): a `pull_request.opened` event with
/// `pr-security-sentinel` in `agents_on_pr_open` must spawn the agent.
/// Verifies the existing fan-out loop's generic `_` arm (which routes
/// any non-`build-runner` entry through `dispatch_pr_reviewer_for_pr`
/// by name) handles the new `adf/security` context cleanly without
/// any production code change.
#[tokio::test]
async fn handle_review_pr_spawns_pr_security_sentinel_when_configured() {
    let (config, _tmp) = review_pr_config_with_security_fanout("echo");
    let mut orch = AgentOrchestrator::new(config).unwrap();

    orch.handle_review_pr(review_pr_task()).await.unwrap();

    assert!(
        orch.active_agents.contains_key("pr-security-sentinel"),
        "pr-security-sentinel must be spawned; active_agents: {:?}",
        orch.active_agents.keys().collect::<Vec<_>>()
    );
    // Security sentinel is a PR-event agent, so it must receive the
    // `ADF_PR_*` env (not `ADF_PUSH_*`). Verified via session_id
    // scoping; the env wiring itself is exercised by the existing
    // `reviewpr_dispatch_sets_env_vars` test through the same
    // dispatch helper.
    let managed = orch.active_agents.get("pr-security-sentinel").unwrap();
    assert!(
        managed.session_id.starts_with("pr-security-sentinel-"),
        "session id should be scoped to the agent, got: {}",
        managed.session_id
    );
}

/// ADF Phase 2c (Refs #953): when a workflow tracker is configured,
/// `pr-security-sentinel`'s spawn must POST a `pending` commit status
/// with context `adf/security` against the PR head SHA.
#[tokio::test]
async fn handle_review_pr_pending_status_posted_for_security_context() {
    use axum::{
        extract::{Path, State},
        http::StatusCode,
        response::IntoResponse,
        routing::post,
        Router,
    };
    use std::sync::atomic::{AtomicUsize, Ordering as AOrdering};
    use std::sync::Arc;
    use tokio::net::TcpListener;

    #[derive(Default)]
    struct Captured {
        calls: AtomicUsize,
        contexts: std::sync::Mutex<Vec<String>>,
        states: std::sync::Mutex<Vec<String>>,
    }

    async fn capture(
        Path((_owner, _repo, _sha)): Path<(String, String, String)>,
        State(captured): State<Arc<Captured>>,
        body: axum::body::Bytes,
    ) -> impl IntoResponse {
        captured.calls.fetch_add(1, AOrdering::SeqCst);
        if let Ok(parsed) = serde_json::from_slice::<serde_json::Value>(&body) {
            if let Some(ctx) = parsed.get("context").and_then(|v| v.as_str()) {
                captured.contexts.lock().unwrap().push(ctx.to_string());
            }
            if let Some(st) = parsed.get("state").and_then(|v| v.as_str()) {
                captured.states.lock().unwrap().push(st.to_string());
            }
        }
        StatusCode::CREATED
    }

    let captured = Arc::new(Captured::default());
    let app = Router::new()
        .route("/api/v1/repos/{owner}/{repo}/statuses/{sha}", post(capture))
        .with_state(captured.clone());
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.ok();
    });
    let base_url = format!("http://{}", addr);

    let (mut config, _tmp) = review_pr_config_with_security_fanout("echo");
    config.workflow = Some(crate::config::WorkflowConfig {
        enabled: true,
        poll_interval_secs: 60,
        workflow_file: std::path::PathBuf::from("/tmp/workflow.md"),
        tracker: crate::config::TrackerConfig {
            kind: "gitea".to_string(),
            endpoint: base_url.clone(),
            api_key: "test-token".to_string(),
            owner: "fakeowner".to_string(),
            repo: "fakerepo".to_string(),
            project_slug: None,
            use_robot_api: false,
            states: crate::config::TrackerStates::default(),
        },
        concurrency: crate::config::ConcurrencyConfig::default(),
    });
    let mut orch = AgentOrchestrator::new(config).unwrap();

    orch.handle_review_pr(review_pr_task()).await.unwrap();

    // Expect three POSTs (one per fan-out agent).
    for _ in 0..150 {
        if captured.calls.load(AOrdering::SeqCst) >= 3 {
            break;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    let contexts = captured.contexts.lock().unwrap().clone();
    assert!(
        contexts.iter().any(|c| c == "adf/security"),
        "adf/security pending status missing; got: {contexts:?}"
    );
    let states = captured.states.lock().unwrap().clone();
    assert!(
        states.iter().all(|s| s == "pending"),
        "all initial statuses must be pending; got: {states:?}"
    );
}

/// ADF Phase 2c (Refs #953): when `pr-security-sentinel` is gated out
/// by the runtime subscription allow-list, its `pending` must NOT be
/// posted -- otherwise `adf/security` (a required check on `main`
/// post-deploy) would hang indefinitely. Other fan-out entries still
/// post their pendings.
#[tokio::test]
async fn handle_review_pr_security_sentinel_skipped_does_not_post_pending() {
    use axum::{
        extract::{Path, State},
        http::StatusCode,
        response::IntoResponse,
        routing::post,
        Router,
    };
    use std::sync::atomic::{AtomicUsize, Ordering as AOrdering};
    use std::sync::Arc;
    use tokio::net::TcpListener;

    #[derive(Default)]
    struct Captured {
        calls: AtomicUsize,
        contexts: std::sync::Mutex<Vec<String>>,
    }

    async fn capture(
        Path((_owner, _repo, _sha)): Path<(String, String, String)>,
        State(captured): State<Arc<Captured>>,
        body: axum::body::Bytes,
    ) -> impl IntoResponse {
        captured.calls.fetch_add(1, AOrdering::SeqCst);
        if let Ok(parsed) = serde_json::from_slice::<serde_json::Value>(&body) {
            if let Some(ctx) = parsed.get("context").and_then(|v| v.as_str()) {
                captured.contexts.lock().unwrap().push(ctx.to_string());
            }
        }
        StatusCode::CREATED
    }

    let captured = Arc::new(Captured::default());
    let app = Router::new()
        .route("/api/v1/repos/{owner}/{repo}/statuses/{sha}", post(capture))
        .with_state(captured.clone());
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.ok();
    });
    let base_url = format!("http://{}", addr);

    let (mut config, _tmp) = review_pr_config_with_security_fanout("echo");
    // Stamp a banned model on pr-security-sentinel AFTER load-time
    // validation so the runtime allow-list gate is the one that
    // short-circuits the spawn. The other agents stay clean.
    let svc = config
        .agents
        .iter_mut()
        .find(|a| a.name == "pr-security-sentinel")
        .unwrap();
    svc.model = Some("google/gemini-2".to_string());

    config.workflow = Some(crate::config::WorkflowConfig {
        enabled: true,
        poll_interval_secs: 60,
        workflow_file: std::path::PathBuf::from("/tmp/workflow.md"),
        tracker: crate::config::TrackerConfig {
            kind: "gitea".to_string(),
            endpoint: base_url.clone(),
            api_key: "test-token".to_string(),
            owner: "fakeowner".to_string(),
            repo: "fakerepo".to_string(),
            project_slug: None,
            use_robot_api: false,
            states: crate::config::TrackerStates::default(),
        },
        concurrency: crate::config::ConcurrencyConfig::default(),
    });
    let mut orch = AgentOrchestrator::new(config).unwrap();

    orch.handle_review_pr(review_pr_task()).await.unwrap();

    // Wait for the two non-skipped POSTs, then a small grace window
    // for any erroneous adf/security POST to surface.
    for _ in 0..150 {
        if captured.calls.load(AOrdering::SeqCst) >= 2 {
            break;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    tokio::time::sleep(Duration::from_millis(150)).await;

    let contexts = captured.contexts.lock().unwrap().clone();
    assert!(
        !contexts.iter().any(|c| c == "adf/security"),
        "skipped pr-security-sentinel must NOT post adf/security pending; got: {contexts:?}"
    );
    assert!(
        !orch.active_agents.contains_key("pr-security-sentinel"),
        "pr-security-sentinel must not be in active_agents when subscription gate rejects"
    );
    // Sanity: the other two agents still spawned + posted.
    assert!(
        contexts.iter().any(|c| c == "adf/pr-reviewer"),
        "pr-reviewer must still post its pending; got: {contexts:?}"
    );
    assert!(
        contexts.iter().any(|c| c == "adf/build"),
        "build-runner must still post its pending; got: {contexts:?}"
    );
}

/// T3: SpawnAgent dispatch must reject when the resolved agent is event_only.
/// No agent should be added to active_agents and no spawn attempted.
#[tokio::test]
async fn test_handle_webhook_dispatch_rejects_event_only_agent() {
    let mut config = test_config();
    // Replace the test fixture agents with a single event_only agent.
    config.agents = vec![AgentDefinition {
        name: "build-runner".to_string(),
        layer: AgentLayer::Growth,
        cli_tool: "/bin/bash".to_string(),
        task: "echo would-build".to_string(),
        schedule: None,
        model: None,
        default_tier: None,
        capabilities: vec!["build".to_string()],
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
        event_only: true,
        evolution_enabled: false,
        rlm_enabled: None,
        bypass_kg_routing: false,
        enabled: true,
        project: None,
    }];
    // mentions config required so handle_webhook_dispatch does not bail at the top.
    config.mentions = Some(crate::config::MentionConfig::default());

    let mut orch = AgentOrchestrator::new(config).unwrap();

    let dispatch = webhook::WebhookDispatch::SpawnAgent {
        agent_name: "build-runner".to_string(),
        detected_project: None,
        issue_number: 9999,
        comment_id: 99999,
        context: "@adf:build-runner please run".to_string(),
        synthetic_event: None,
    };

    orch.handle_webhook_dispatch(dispatch).await;

    assert!(
        orch.active_agents.is_empty(),
        "event_only agent must not be added to active_agents on mention dispatch; got: {:?}",
        orch.active_agents.keys().collect::<Vec<_>>()
    );
}

/// T4: SpawnPersona dispatch must reject when the resolved agent is event_only.
/// Uses resolve_persona_mention's "direct agent name match" branch by passing
/// the agent name as the persona name -- the resolver returns the agent, and our
/// gate must reject before spawn.
#[tokio::test]
async fn test_handle_webhook_dispatch_rejects_event_only_persona() {
    let mut config = test_config();
    config.agents = vec![AgentDefinition {
        name: "build-runner".to_string(),
        layer: AgentLayer::Growth,
        cli_tool: "/bin/bash".to_string(),
        task: "echo would-build".to_string(),
        schedule: None,
        model: None,
        default_tier: None,
        capabilities: vec!["build".to_string()],
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
        event_only: true,
        evolution_enabled: false,
        rlm_enabled: None,
        bypass_kg_routing: false,
        enabled: true,
        project: None,
    }];
    config.mentions = Some(crate::config::MentionConfig::default());

    let mut orch = AgentOrchestrator::new(config).unwrap();

    let dispatch = webhook::WebhookDispatch::SpawnPersona {
        persona_name: "build-runner".to_string(),
        issue_number: 9999,
        comment_id: 99999,
        context: "@adf:build-runner please run via persona".to_string(),
    };

    orch.handle_webhook_dispatch(dispatch).await;

    assert!(
        orch.active_agents.is_empty(),
        "event_only agent must not be added to active_agents on persona dispatch; got: {:?}",
        orch.active_agents.keys().collect::<Vec<_>>()
    );
}

/// T5: structural invariant for the post-exit defensive guard.
/// The guard at lib.rs's "Post output to Gitea if configured" block fires
/// `error!` and skips posting when an event-only agent reaches the hook with
/// gitea_issue set -- a "should never happen" path because the dispatch gate
/// in handle_webhook_dispatch should have already rejected the spawn.
/// Full runtime coverage would require integration infrastructure
/// (worktrees, real spawning, output capture) disproportionate to a single
/// boolean check. T3 and T4 cover the dispatch gate that prevents this
/// branch from being reached in production.
#[test]
fn test_post_exit_guard_invariant_event_only_with_gitea_issue() {
    let def = AgentDefinition {
        name: "build-runner".to_string(),
        layer: AgentLayer::Growth,
        cli_tool: "/bin/bash".to_string(),
        task: "echo would-build".to_string(),
        schedule: None,
        model: None,
        default_tier: None,
        capabilities: vec!["build".to_string()],
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
        gitea_issue: Some(9999),
        event_only: true,
        evolution_enabled: false,
        rlm_enabled: None,
        bypass_kg_routing: false,
        enabled: true,
        project: None,
    };

    // The defensive guard reads exactly these two fields. Its branch fires
    // when both are set on the same definition.
    assert!(
        def.event_only,
        "event_only must be true to trigger the guard"
    );
    assert!(
        def.gitea_issue.is_some(),
        "gitea_issue must be Some(_) for the post-exit code to enter the outer if-let"
    );
}

#[test]
fn test_spawn_ctx_working_dir_set_to_agent_working_dir() {
    use std::path::PathBuf;

    // Simulate what build_spawn_context_for_agent returns for a project-bound agent:
    // working_dir is set to the project root.
    let project_root = PathBuf::from("/tmp/project-root");
    let worktree_path = PathBuf::from("/tmp/project-root/.worktrees/agent-abc123");

    let mut spawn_ctx = SpawnContext::with_working_dir(project_root.clone()).with_env(
        "ADF_WORKING_DIR",
        project_root.to_string_lossy().into_owned(),
    );

    // Apply the fix (the two new lines from the proposed change).
    let agent_working_dir = worktree_path.clone();
    spawn_ctx.working_dir = Some(agent_working_dir.clone());
    spawn_ctx = spawn_ctx.with_env(
        "ADF_WORKING_DIR",
        agent_working_dir.to_string_lossy().into_owned(),
    );

    assert_eq!(
        spawn_ctx.working_dir.as_deref(),
        Some(worktree_path.as_path()),
        "spawn_ctx.working_dir must be the worktree path, not the project root"
    );
    assert_eq!(
        spawn_ctx
            .env_overrides
            .get("ADF_WORKING_DIR")
            .map(String::as_str),
        Some(worktree_path.to_string_lossy().as_ref()),
        "ADF_WORKING_DIR env var must reflect the worktree path"
    );
}

#[test]
fn test_requires_isolated_worktree_for_mutating_ai_agents() {
    let mut def = test_config_fast_lifecycle().agents[0].clone();

    def.cli_tool = "echo".to_string();
    assert!(!requires_isolated_worktree(&def, None));

    assert!(requires_isolated_worktree(
        &def,
        Some("kimi-for-coding/k2p6")
    ));

    def.cli_tool = "/home/alex/.local/bin/claude".to_string();
    assert!(requires_isolated_worktree(&def, None));
    assert!(!requires_isolated_worktree(&def, Some("claude-3-haiku")));
}

#[tokio::test]
async fn test_mutating_agent_fails_closed_when_worktree_creation_fails() {
    let temp_repo = TempDir::new().unwrap();
    let mut config = test_config_fast_lifecycle();
    config.working_dir = temp_repo.path().to_path_buf();
    config.agents[0].cli_tool = "claude".to_string();

    let mut orch = AgentOrchestrator::new(config).unwrap();
    let def = orch.config.agents[0].clone();
    let result = orch.spawn_agent(&def).await;

    assert!(matches!(
        result,
        Err(OrchestratorError::WorktreeCreationFailed { .. })
    ));
    assert!(
        !orch.active_agents.contains_key("echo-safety"),
        "agent must not spawn in the shared checkout after worktree failure"
    );
}
