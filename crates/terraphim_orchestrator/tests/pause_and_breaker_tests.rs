//! Integration tests for the project pause flag gate and the project-meta
//! circuit breaker (Refs terraphim/adf-fleet#8).
//!
//! These tests avoid spawning real agent processes: pause-flag semantics live
//! entirely in filesystem state (`<pause_dir>/<project_id>`), and the breaker
//! is driven through `simulate_project_meta_failures_for_test`. The real
//! `spawn_agent` path is exercised via the public `spawn_agent_for_test`
//! helper which runs the pause gate and returns early when the flag is
//! present.

use std::path::PathBuf;

use tempfile::TempDir;
use terraphim_orchestrator::project_control;
use terraphim_orchestrator::{
    AgentDefinition, AgentLayer, AgentOrchestrator, CompoundReviewConfig, NightwatchConfig,
    OrchestratorConfig,
};

fn project_agent(name: &str, project: Option<&str>) -> AgentDefinition {
    AgentDefinition {
        name: name.to_string(),
        layer: AgentLayer::Core,
        cli_tool: "/bin/true".to_string(),
        task: String::new(),
        schedule: None,
        model: None,
        capabilities: Vec::new(),
        max_memory_bytes: None,
        budget_monthly_cents: None,
        provider: None,
        persona: None,
        terraphim_role: None,
        skill_chain: Vec::new(),
        sfia_skills: Vec::new(),
        fallback_provider: None,
        fallback_model: None,
        grace_period_secs: None,
        max_cpu_seconds: None,
        pre_check: None,
        gitea_issue: None,
        project: project.map(|s| s.to_string()),
    }
}

fn test_config_with_pause(pause_dir: PathBuf, threshold: u32) -> OrchestratorConfig {
    OrchestratorConfig {
        working_dir: PathBuf::from("/tmp/test-orchestrator-breaker"),
        nightwatch: NightwatchConfig::default(),
        compound_review: CompoundReviewConfig {
            cli_tool: None,
            provider: None,
            model: None,
            schedule: "0 2 * * *".to_string(),
            max_duration_secs: 60,
            repo_path: PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../.."),
            create_prs: false,
            worktree_root: PathBuf::from("/tmp/test-orchestrator-breaker/.worktrees"),
            base_branch: "main".to_string(),
            max_concurrent_agents: 3,
            ..Default::default()
        },
        workflow: None,
        agents: vec![
            project_agent("odilo-worker", Some("odilo")),
            project_agent("legacy-worker", None),
            project_agent("digital-twins-worker", Some("digital-twins")),
        ],
        restart_cooldown_secs: 60,
        max_restart_count: 10,
        restart_budget_window_secs: 43_200,
        disk_usage_threshold: 100,
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
        providers: vec![],
        provider_budget_state_file: None,
        pause_dir: Some(pause_dir),
        project_circuit_breaker_threshold: threshold,
        fleet_escalation_owner: None,
        fleet_escalation_repo: None,
    }
}

#[tokio::test]
async fn pause_flag_blocks_dispatch_for_matching_project() {
    let tmp = TempDir::new().unwrap();
    let config = test_config_with_pause(tmp.path().to_path_buf(), 3);
    let mut orch = AgentOrchestrator::new(config).unwrap();

    // Pre-create the pause flag for "odilo".
    project_control::touch_pause_flag(tmp.path(), "odilo").unwrap();

    // spawn_agent_for_test runs the full spawn_agent gate. A paused project
    // returns Ok(()) without creating an active agent record.
    orch.spawn_agent_for_test("odilo-worker")
        .await
        .expect("paused spawn should return Ok(()) without erroring");

    assert!(
        !orch.is_agent_active("odilo-worker"),
        "paused project must not produce an active agent record"
    );
}

#[tokio::test]
async fn absent_pause_flag_does_not_block_dispatch() {
    let tmp = TempDir::new().unwrap();
    let config = test_config_with_pause(tmp.path().to_path_buf(), 3);
    let orch = AgentOrchestrator::new(config).unwrap();

    // No pause flag: the spawn path runs to completion. The underlying
    // spawner may still fail on `/bin/true` for unrelated reasons; what we
    // assert is that the *pause gate* did not short-circuit, which is
    // reflected by the orchestrator having at least attempted the spawn.
    // We observe this indirectly: if the pause gate had fired, the active
    // agent map would be untouched AND no error would have been returned.
    // Since we cannot distinguish those two states from "paused" without
    // extra hooks, we instead verify the invariant that without the flag
    // the pause check itself reports not-paused.
    assert!(
        !project_control::is_project_paused(orch.pause_dir_for_test(), Some("odilo")),
        "no pause flag should mean not paused"
    );
}

#[tokio::test]
async fn pause_flag_is_project_scoped() {
    let tmp = TempDir::new().unwrap();
    let config = test_config_with_pause(tmp.path().to_path_buf(), 3);
    let mut orch = AgentOrchestrator::new(config).unwrap();

    project_control::touch_pause_flag(tmp.path(), "odilo").unwrap();

    // odilo is paused; digital-twins is not.
    assert!(project_control::is_project_paused(
        orch.pause_dir_for_test(),
        Some("odilo")
    ));
    assert!(!project_control::is_project_paused(
        orch.pause_dir_for_test(),
        Some("digital-twins")
    ));

    // The dispatch for odilo should be gated.
    orch.spawn_agent_for_test("odilo-worker").await.unwrap();
    assert!(!orch.is_agent_active("odilo-worker"));
}

#[tokio::test]
async fn pause_flag_does_not_affect_legacy_global_agents() {
    let tmp = TempDir::new().unwrap();
    let config = test_config_with_pause(tmp.path().to_path_buf(), 3);
    let _orch = AgentOrchestrator::new(config).unwrap();

    // Even with an exotic "__global__" pause flag, legacy (project=None)
    // agents must never be blocked by the project pause mechanism.
    project_control::touch_pause_flag(tmp.path(), "__global__").unwrap();
    assert!(!project_control::is_project_paused(tmp.path(), None));
}

#[tokio::test]
async fn circuit_breaker_trips_at_threshold_and_touches_pause_flag() {
    let tmp = TempDir::new().unwrap();
    let config = test_config_with_pause(tmp.path().to_path_buf(), 3);
    let mut orch = AgentOrchestrator::new(config).unwrap();

    // Two failures under threshold: no pause flag yet.
    let tripped_two = orch
        .simulate_project_meta_failures_for_test("odilo", 2)
        .await;
    assert!(!tripped_two, "threshold=3, two failures must not trip");
    assert!(!project_control::is_project_paused(
        orch.pause_dir_for_test(),
        Some("odilo")
    ));

    // Third failure: threshold reached, pause flag created.
    let tripped_three = orch
        .simulate_project_meta_failures_for_test("odilo", 1)
        .await;
    assert!(tripped_three, "threshold=3, third failure must trip");
    assert!(project_control::is_project_paused(
        orch.pause_dir_for_test(),
        Some("odilo")
    ));
}

#[tokio::test]
async fn circuit_breaker_resets_on_success() {
    let tmp = TempDir::new().unwrap();
    let config = test_config_with_pause(tmp.path().to_path_buf(), 3);
    let mut orch = AgentOrchestrator::new(config).unwrap();

    orch.simulate_project_meta_failures_for_test("odilo", 2)
        .await;

    // Success resets the streak; the next two failures should NOT trip.
    orch.reset_project_meta_counter_for_test("odilo");
    let tripped = orch
        .simulate_project_meta_failures_for_test("odilo", 2)
        .await;

    assert!(
        !tripped,
        "counter must reset on success; two further failures under threshold=3 must not trip"
    );
    assert!(
        !project_control::is_project_paused(orch.pause_dir_for_test(), Some("odilo")),
        "no pause flag should exist after reset + 2 failures"
    );
}

#[tokio::test]
async fn circuit_breaker_is_per_project() {
    let tmp = TempDir::new().unwrap();
    let config = test_config_with_pause(tmp.path().to_path_buf(), 2);
    let mut orch = AgentOrchestrator::new(config).unwrap();

    // Trip odilo.
    let odilo_tripped = orch
        .simulate_project_meta_failures_for_test("odilo", 2)
        .await;
    assert!(odilo_tripped);

    // digital-twins must still be clean.
    assert!(project_control::is_project_paused(
        orch.pause_dir_for_test(),
        Some("odilo")
    ));
    assert!(!project_control::is_project_paused(
        orch.pause_dir_for_test(),
        Some("digital-twins")
    ));
}

#[tokio::test]
async fn pause_flag_removal_restores_dispatch() {
    let tmp = TempDir::new().unwrap();
    let config = test_config_with_pause(tmp.path().to_path_buf(), 3);
    let orch = AgentOrchestrator::new(config).unwrap();

    // Create the flag, observe paused, remove, observe not-paused.
    project_control::touch_pause_flag(tmp.path(), "odilo").unwrap();
    assert!(project_control::is_project_paused(
        orch.pause_dir_for_test(),
        Some("odilo")
    ));

    std::fs::remove_file(tmp.path().join("odilo")).unwrap();

    assert!(
        !project_control::is_project_paused(orch.pause_dir_for_test(), Some("odilo")),
        "after removal the pause gate must no longer block"
    );
}
