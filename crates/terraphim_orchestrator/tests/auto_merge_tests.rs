//! Integration tests for ROC v1 Step F — verdict polling + AutoMerge enqueue.
//!
//! These tests exercise [`AgentOrchestrator::poll_pending_reviews_for_project`]
//! against an in-memory [`PrTracker`] (no mocks, no network, no Gitea). They
//! assert that the orchestrator enqueues [`DispatchTask::AutoMerge`] exactly
//! when every gate in [`AutoMergeCriteria::default`] is satisfied and never
//! more than once per `(project, pr_number, head_sha)` triple.
//!
//! See `cto-executive-system/plans/adf-rate-of-change-design.md` §Step F and
//! Gitea issue `terraphim/adf-fleet#34`.

use std::collections::HashMap;
use std::path::PathBuf;

use async_trait::async_trait;
use terraphim_orchestrator::pr_poller::{PrComment, PrSummary, PrTracker};
use terraphim_orchestrator::pr_review::AutoMergeCriteria;
use terraphim_orchestrator::{
    AgentOrchestrator, CompoundReviewConfig, DispatchTask, NightwatchConfig, OrchestratorConfig,
};

/// Test fixture bodies copied verbatim from the structural-pr-review skill.
const GO_5_5_CLEAN: &str = include_str!("fixtures/pr_review/go_5_5_clean.md");
const NOGO_2_5: &str = include_str!("fixtures/pr_review/nogo_2_5.md");
const MALFORMED_NO_CONFIDENCE: &str = include_str!("fixtures/pr_review/malformed_no_confidence.md");

/// Reviewer login emitted by the structural-pr-review skill.
const PR_REVIEWER: &str = "pr-reviewer";

/// Project id used throughout these tests. Any non-empty string works; we
/// pick a concrete value to make rate-limiter / dedupe scoping explicit.
const PROJECT: &str = "alpha";

/// Minimal [`OrchestratorConfig`] for the dispatcher-shaped tests below.
/// No agents, no trackers, no projects — the poller drives itself off the
/// in-memory `InMemoryPrTracker` passed directly into
/// `poll_pending_reviews_for_project`.
fn minimal_config() -> OrchestratorConfig {
    OrchestratorConfig {
        working_dir: PathBuf::from("/tmp/terraphim-auto-merge-tests"),
        nightwatch: NightwatchConfig::default(),
        compound_review: CompoundReviewConfig {
            cli_tool: None,
            provider: None,
            model: None,
            schedule: "0 2 * * *".to_string(),
            max_duration_secs: 60,
            repo_path: PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../.."),
            create_prs: false,
            worktree_root: PathBuf::from("/tmp/terraphim-auto-merge-tests/.worktrees"),
            base_branch: "main".to_string(),
            max_concurrent_agents: 1,
            ..Default::default()
        },
        workflow: None,
        agents: vec![],
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
        post_merge_gate: None,
        learning: terraphim_orchestrator::LearningConfig::default(),
        pr_dispatch: None,
        pr_dispatch_per_project: Default::default(),
    }
}

/// In-memory [`PrTracker`] used by every test in this file. It holds a fixed
/// list of open PRs and a map of PR number -> comments. No mocks: the impl
/// is a plain struct doing plain lookups.
#[derive(Default)]
struct InMemoryPrTracker {
    prs: Vec<PrSummary>,
    comments_by_pr: HashMap<u64, Vec<PrComment>>,
}

impl InMemoryPrTracker {
    fn new() -> Self {
        Self::default()
    }

    fn with_pr(mut self, pr: PrSummary, comments: Vec<PrComment>) -> Self {
        self.comments_by_pr.insert(pr.number, comments);
        self.prs.push(pr);
        self
    }
}

#[async_trait]
impl PrTracker for InMemoryPrTracker {
    async fn list_open_prs(&self) -> Result<Vec<PrSummary>, String> {
        Ok(self.prs.clone())
    }

    async fn fetch_pr_comments(&self, pr_number: u64) -> Result<Vec<PrComment>, String> {
        Ok(self
            .comments_by_pr
            .get(&pr_number)
            .cloned()
            .unwrap_or_default())
    }
}

/// Build a [`PrSummary`] with sensible defaults. Tests override any field
/// they care about via struct update syntax.
fn pr_summary(number: u64, author: &str, head: &str, diff_loc: u32) -> PrSummary {
    PrSummary {
        number,
        author_login: author.to_string(),
        head_sha: head.to_string(),
        base_ref: "main".to_string(),
        diff_loc,
    }
}

fn reviewer_comment(id: u64, body: &str, updated_at: &str) -> PrComment {
    PrComment {
        id,
        user_login: PR_REVIEWER.to_string(),
        body: body.to_string(),
        updated_at: updated_at.to_string(),
    }
}

/// Helper: how many AutoMerge tasks are currently in the dispatcher queue?
fn auto_merge_depth(orch: &AgentOrchestrator) -> u64 {
    orch.dispatcher()
        .stats()
        .by_source
        .get("auto_merge")
        .copied()
        .unwrap_or(0)
}

// -------- Test 1: green review + all gates cleared -> enqueue AutoMerge --------

#[tokio::test]
async fn auto_merge_requires_all_gates() {
    let mut orch = AgentOrchestrator::new(minimal_config()).unwrap();

    let tracker = InMemoryPrTracker::new().with_pr(
        pr_summary(101, "claude-code", "2ef451d8", 42),
        vec![reviewer_comment(1, GO_5_5_CLEAN, "2026-01-02T00:00:00Z")],
    );

    orch.poll_pending_reviews_for_project(PROJECT, &tracker, &AutoMergeCriteria::default())
        .await;

    assert_eq!(
        auto_merge_depth(&orch),
        1,
        "a clean 5/5 review with 0 P0/P1 and all acceptance criteria checked must enqueue AutoMerge"
    );

    let next = orch.dispatcher().peek().cloned();
    match next {
        Some(DispatchTask::AutoMerge {
            pr_number,
            project,
            head_sha,
        }) => {
            assert_eq!(pr_number, 101);
            assert_eq!(project, PROJECT);
            assert_eq!(head_sha, "2ef451d8");
        }
        other => panic!("expected DispatchTask::AutoMerge, got {:?}", other),
    }
}

// -------- Test 2: P1 finding present -> blocked --------

#[tokio::test]
async fn auto_merge_blocked_on_p1_present() {
    let mut orch = AgentOrchestrator::new(minimal_config()).unwrap();

    let tracker = InMemoryPrTracker::new().with_pr(
        pr_summary(202, "claude-code", "62672e38", 42),
        vec![reviewer_comment(2, NOGO_2_5, "2026-01-02T00:00:00Z")],
    );

    orch.poll_pending_reviews_for_project(PROJECT, &tracker, &AutoMergeCriteria::default())
        .await;

    assert_eq!(
        auto_merge_depth(&orch),
        0,
        "a review with 2 P0 + 1 P1 findings must block AutoMerge even before other gates"
    );
}

// -------- Test 3: non-agent author -> blocked --------

#[tokio::test]
async fn auto_merge_blocked_on_non_agent_author() {
    let mut orch = AgentOrchestrator::new(minimal_config()).unwrap();

    // Reuse the clean fixture but swap the author login to a human.
    let tracker = InMemoryPrTracker::new().with_pr(
        pr_summary(303, "alice-human", "2ef451d8", 42),
        vec![reviewer_comment(3, GO_5_5_CLEAN, "2026-01-02T00:00:00Z")],
    );

    orch.poll_pending_reviews_for_project(PROJECT, &tracker, &AutoMergeCriteria::default())
        .await;

    assert_eq!(
        auto_merge_depth(&orch),
        0,
        "require_agent_author gate must block AutoMerge for human-authored PRs"
    );
}

// -------- Test 4: diff LoC over the cap -> blocked --------

#[tokio::test]
async fn auto_merge_blocked_on_large_diff() {
    let mut orch = AgentOrchestrator::new(minimal_config()).unwrap();

    // 5/5 clean review, agent author, but diff_loc exceeds the 500 LoC cap.
    let tracker = InMemoryPrTracker::new().with_pr(
        pr_summary(404, "claude-code", "2ef451d8", 501),
        vec![reviewer_comment(4, GO_5_5_CLEAN, "2026-01-02T00:00:00Z")],
    );

    orch.poll_pending_reviews_for_project(PROJECT, &tracker, &AutoMergeCriteria::default())
        .await;

    assert_eq!(
        auto_merge_depth(&orch),
        0,
        "max_diff_loc=500 must block AutoMerge for a 501-LoC diff"
    );
}

// -------- Test 5: idempotency across ticks --------

#[tokio::test]
async fn auto_merge_idempotent_across_ticks() {
    let mut orch = AgentOrchestrator::new(minimal_config()).unwrap();

    let tracker = InMemoryPrTracker::new().with_pr(
        pr_summary(505, "claude-code", "2ef451d8", 42),
        vec![reviewer_comment(5, GO_5_5_CLEAN, "2026-01-02T00:00:00Z")],
    );

    // First tick enqueues exactly one AutoMerge task.
    orch.poll_pending_reviews_for_project(PROJECT, &tracker, &AutoMergeCriteria::default())
        .await;
    assert_eq!(auto_merge_depth(&orch), 1, "first tick must enqueue");

    // Repeated polls for the same (pr_number, head_sha) within the same
    // orchestrator instance must never enqueue a duplicate. The rate
    // limiter and the dedupe set each enforce this separately; both paths
    // hold the observable invariant that the queue depth stays at 1.
    for _ in 0..5 {
        orch.poll_pending_reviews_for_project(PROJECT, &tracker, &AutoMergeCriteria::default())
            .await;
    }
    assert_eq!(
        auto_merge_depth(&orch),
        1,
        "repeated polls over the same revision must never enqueue a duplicate AutoMerge"
    );
}

// -------- Test 6: PR with no reviewer comment -> skipped silently --------

#[tokio::test]
async fn poll_skips_prs_without_reviewer_comment() {
    let mut orch = AgentOrchestrator::new(minimal_config()).unwrap();

    // One PR with a non-reviewer comment (human noise) and no reviewer
    // verdict yet. The poller should treat this as "no verdict, skip".
    let noise = PrComment {
        id: 9,
        user_login: "alice-human".to_string(),
        body: "looks good to me".to_string(),
        updated_at: "2026-01-02T00:00:00Z".to_string(),
    };
    let tracker = InMemoryPrTracker::new()
        .with_pr(pr_summary(606, "claude-code", "2ef451d8", 42), vec![noise]);

    orch.poll_pending_reviews_for_project(PROJECT, &tracker, &AutoMergeCriteria::default())
        .await;

    assert_eq!(
        auto_merge_depth(&orch),
        0,
        "no reviewer comment means no verdict to act on; AutoMerge queue must be empty"
    );
}

// -------- Test 7: malformed verdict -> parse error, no enqueue, no panic --------

#[tokio::test]
async fn poll_handles_verdict_parse_error_gracefully() {
    let mut orch = AgentOrchestrator::new(minimal_config()).unwrap();

    let tracker = InMemoryPrTracker::new().with_pr(
        pr_summary(707, "claude-code", "6becb2f7", 42),
        vec![reviewer_comment(
            7,
            MALFORMED_NO_CONFIDENCE,
            "2026-01-02T00:00:00Z",
        )],
    );

    // This call must not panic and must not enqueue anything.
    orch.poll_pending_reviews_for_project(PROJECT, &tracker, &AutoMergeCriteria::default())
        .await;

    assert_eq!(
        auto_merge_depth(&orch),
        0,
        "malformed reviewer comment must be logged and skipped, never enqueued"
    );
}
