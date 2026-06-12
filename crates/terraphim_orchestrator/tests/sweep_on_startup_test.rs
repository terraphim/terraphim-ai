//! Layer 2 startup sweep integration test (epic #1567, Gitea
//! issue #1570).
//!
//! Property under test:
//!
//! > Constructing `AgentOrchestrator` MUST reconcile any stale
//! > `review-*` directories left under the configured
//! > `worktree_root`, AND any direct child of any path listed in
//! > `extra_roots`, BEFORE returning to the caller. The sweep is
//! > synchronous; no tick has fired yet.
//!
//! Compiled only when the `test-helpers` feature is enabled (so the
//! `scope::test_support` shared git-repo fixture is visible) and on
//! Unix (the WorktreeManager fallback path expects POSIX file
//! semantics).

#![cfg(all(unix, feature = "test-helpers"))]

use std::path::PathBuf;

use tempfile::TempDir;
use uuid::Uuid;

use terraphim_orchestrator::scope::{test_support::setup_git_repo, WORKTREE_REVIEW_PREFIX};
use terraphim_orchestrator::{
    AgentDefinition, AgentLayer, AgentOrchestrator, CompoundReviewConfig, LearningConfig,
    NightwatchConfig, OrchestratorConfig,
};

/// Build the minimal `OrchestratorConfig` required for
/// `AgentOrchestrator::new` to succeed against the supplied fixture.
///
/// Borrows the layout from `tests/orchestrator_tests.rs::test_config`
/// but points `repo_path` and `worktree_root` into the per-test
/// `TempDir` so the startup sweep never touches the live repo.
fn isolated_config(
    working_dir: PathBuf,
    repo_path: PathBuf,
    worktree_root: PathBuf,
) -> OrchestratorConfig {
    OrchestratorConfig {
        working_dir,
        nightwatch: NightwatchConfig::default(),
        compound_review: CompoundReviewConfig {
            cli_tool: None,
            provider: None,
            model: None,
            schedule: "0 2 * * *".to_string(),
            max_duration_secs: 60,
            repo_path,
            create_prs: false,
            worktree_root,
            base_branch: "main".to_string(),
            max_concurrent_agents: 1,
            ..Default::default()
        },
        workflow: None,
        // A single trivial agent; the sweep runs unconditionally in
        // `new` regardless of agent set.
        agents: vec![AgentDefinition {
            name: "noop".to_string(),
            layer: AgentLayer::Core,
            cli_tool: "echo".to_string(),
            task: "noop".to_string(),
            model: None,
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
            project: None,
        }],
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
        pause_dir: None,
        project_circuit_breaker_threshold: 3,
        fleet_escalation_owner: None,
        fleet_escalation_repo: None,
        auto_merge: None,
        post_merge_gate: None,
        learning: LearningConfig::default(),
        pr_dispatch: None,
        pr_dispatch_per_project: Default::default(),
        gitea_skill_repo: None,
        gate_reconcile_interval_ticks: 20,
        evolution: Default::default(),
    }
}

/// Pre-seed three `review-<uuid>` directories under `worktree_root`,
/// then build the orchestrator and assert all three are gone.
///
/// The directories are plain `mkdir` entries (not registered
/// worktrees), which is the exact shape SIGKILL leaves behind: the
/// directory survives but `<repo>/.git/worktrees/<name>` may or may
/// not. The sweep's fallback path handles both.
#[test]
fn sweep_removes_review_prefixed_residue_on_startup() {
    let working_dir = TempDir::new().expect("working dir");
    let (_repo_tmp, repo_path) = setup_git_repo();
    let worktree_root = working_dir.path().join("worktrees");
    std::fs::create_dir_all(&worktree_root).unwrap();

    // Seed three review-prefixed dirs to look like residue.
    let mut review_dirs = Vec::new();
    for _ in 0..3 {
        let dir = worktree_root.join(format!("{}{}", WORKTREE_REVIEW_PREFIX, Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("residue.txt"), "left by SIGKILL").unwrap();
        review_dirs.push(dir);
    }

    // Sanity: all present before construction.
    for d in &review_dirs {
        assert!(d.exists(), "fixture seeding failed for {}", d.display());
    }

    let config = isolated_config(
        working_dir.path().to_path_buf(),
        repo_path,
        worktree_root.clone(),
    );
    let _orch = AgentOrchestrator::new(config).expect("AgentOrchestrator::new must succeed");

    // After new() returns, all review residue must be gone.
    for d in &review_dirs {
        assert!(
            !d.exists(),
            "Layer 2 sweep failed to remove {}",
            d.display()
        );
    }
}

/// Non-review-prefixed siblings under the same `worktree_root` must
/// be preserved by the startup sweep.
#[test]
fn sweep_preserves_non_review_siblings_on_startup() {
    let working_dir = TempDir::new().expect("working dir");
    let (_repo_tmp, repo_path) = setup_git_repo();
    let worktree_root = working_dir.path().join("worktrees");
    std::fs::create_dir_all(&worktree_root).unwrap();

    let review_dir = worktree_root.join(format!("{}victim", WORKTREE_REVIEW_PREFIX));
    let keep_dir = worktree_root.join("keep-me");
    std::fs::create_dir_all(&review_dir).unwrap();
    std::fs::create_dir_all(&keep_dir).unwrap();
    std::fs::write(keep_dir.join("important.txt"), "do not delete").unwrap();

    let config = isolated_config(
        working_dir.path().to_path_buf(),
        repo_path,
        worktree_root.clone(),
    );
    let _orch = AgentOrchestrator::new(config).expect("AgentOrchestrator::new must succeed");

    assert!(!review_dir.exists(), "review-victim should be swept");
    assert!(keep_dir.exists(), "keep-me must survive the sweep");
    assert!(
        keep_dir.join("important.txt").exists(),
        "keep-me contents must survive the sweep"
    );
}

/// Direct children of any extra root passed via the sweep are removed
/// regardless of name prefix.
///
/// The orchestrator currently hard-codes `/tmp/adf-worktrees` as the
/// only extra root, so we cannot inject one through the public API.
/// To exercise the contract without colliding with a real
/// `/tmp/adf-worktrees` on the host, we drive `sweep_stale` directly
/// against an isolated temp root and assert the same outcome the
/// startup wiring would produce.
#[test]
fn sweep_removes_extra_root_children_regardless_of_prefix() {
    use terraphim_orchestrator::scope::WorktreeManager;

    let (_repo_tmp, repo_path) = setup_git_repo();
    let worktree_root = _repo_tmp.path().join(".worktrees");
    std::fs::create_dir_all(&worktree_root).unwrap();
    let manager = WorktreeManager::with_base(&repo_path, &worktree_root);

    // Per-host-unique extra root; avoids racing the real
    // /tmp/adf-worktrees if another agent is running locally.
    let extra_root = std::env::temp_dir().join(format!("adf-worktrees-test-{}", Uuid::new_v4()));
    std::fs::create_dir_all(&extra_root).unwrap();

    let agent_child = extra_root.join("agent-task-7");
    std::fs::create_dir_all(&agent_child).unwrap();
    std::fs::write(agent_child.join("scratch.txt"), "residue").unwrap();

    let report = manager.sweep_stale(std::slice::from_ref(&extra_root));

    assert_eq!(
        report.swept_count, 1,
        "extra-root child should be swept regardless of prefix"
    );
    assert!(
        !agent_child.exists(),
        "{} should be removed",
        agent_child.display()
    );
    assert_eq!(report.failed_count, 0);

    // Tidy up the now-empty extra root.
    let _ = std::fs::remove_dir_all(&extra_root);
}
