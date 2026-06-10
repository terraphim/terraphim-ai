//! Integration tests for ROC v1 Step F — gate polling + AutoMerge enqueue.
//!
//! These tests exercise [`AgentOrchestrator::poll_pending_reviews_for_project`]
//! against an in-memory [`PrTracker`] (no mocks, no network, no Gitea). They
//! assert that the orchestrator enqueues [`DispatchTask::AutoMerge`] when every
//! required commit status is green and every `adf:gate-result` block is fresh.

use std::collections::HashMap;
use std::path::PathBuf;

use async_trait::async_trait;
use terraphim_orchestrator::pr_gate::{CommitStatusState, CommitStatusSummary};
use terraphim_orchestrator::pr_poller::{
    PrComment, PrSummary, PrTracker, ADF_REVIEWER_CONTEXT, ADF_VALIDATION_CONTEXT,
    ADF_VERIFICATION_CONTEXT, NATIVE_CI_CONTEXT,
};
use terraphim_orchestrator::pr_review::AutoMergeCriteria;
use terraphim_orchestrator::{
    AgentOrchestrator, CompoundReviewConfig, DispatchTask, NightwatchConfig, OrchestratorConfig,
};

/// Project id used throughout these tests.
const PROJECT: &str = "terraphim-core";

/// Minimal [`OrchestratorConfig`] for the dispatcher-shaped tests below.
fn minimal_config(working_dir: PathBuf) -> OrchestratorConfig {
    let worktree_root = working_dir.join(".worktrees");
    OrchestratorConfig {
        working_dir,
        nightwatch: NightwatchConfig::default(),
        compound_review: CompoundReviewConfig {
            cli_tool: None,
            provider: None,
            model: None,
            schedule: "0 2 * * *".to_string(),
            max_duration_secs: 60,
            repo_path: PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../.."),
            create_prs: false,
            worktree_root,
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
        evolution: terraphim_orchestrator::EvolutionConfig::default(),
        pr_dispatch: None,
        pr_dispatch_per_project: Default::default(),
        gitea_skill_repo: None,
        direct_dispatch: None,
        gate_reconcile_interval_ticks: 20,
    }
}

#[derive(Default)]
struct InMemoryPrTracker {
    prs: Vec<PrSummary>,
    comments_by_pr: HashMap<u64, Vec<PrComment>>,
    statuses_by_sha: HashMap<String, Vec<CommitStatusSummary>>,
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

    fn with_statuses_for_head(
        mut self,
        head_sha: &str,
        statuses: Vec<CommitStatusSummary>,
    ) -> Self {
        self.statuses_by_sha.insert(head_sha.to_string(), statuses);
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

    async fn fetch_head_commit_statuses(
        &self,
        head_sha: &str,
    ) -> Result<Vec<CommitStatusSummary>, String> {
        Ok(self
            .statuses_by_sha
            .get(head_sha)
            .cloned()
            .unwrap_or_default())
    }
}

fn pr_summary(number: u64, author: &str, head: &str, diff_loc: u32) -> PrSummary {
    PrSummary {
        number,
        author_login: author.to_string(),
        head_sha: head.to_string(),
        base_ref: "main".to_string(),
        diff_loc,
    }
}

fn success_status(context: &str) -> CommitStatusSummary {
    CommitStatusSummary {
        context: context.to_string(),
        state: CommitStatusState::Success,
        created_at_unix: None,
    }
}

fn all_green_statuses() -> Vec<CommitStatusSummary> {
    vec![
        success_status(NATIVE_CI_CONTEXT),
        success_status(ADF_REVIEWER_CONTEXT),
        success_status(ADF_VALIDATION_CONTEXT),
        success_status(ADF_VERIFICATION_CONTEXT),
    ]
}

struct GateCommentSpec<'a> {
    id: u64,
    agent: &'a str,
    context: &'a str,
    pr_number: u64,
    head: &'a str,
    status: &'a str,
    blocking_findings: u32,
    updated_at: &'a str,
}

fn gate_comment(spec: GateCommentSpec<'_>) -> PrComment {
    let body = format!(
        "Gate report\n<!-- adf:gate-result\n{{\n  \"schema_version\": 1,\n  \"agent\": \"{}\",\n  \"context\": \"{}\",\n  \"pr_number\": {},\n  \"head_sha\": \"{}\",\n  \"status\": \"{}\",\n  \"confidence\": 4,\n  \"blocking_findings\": {},\n  \"summary\": \"ok\"\n}}\n-->",
        spec.agent,
        spec.context,
        spec.pr_number,
        spec.head,
        spec.status,
        spec.blocking_findings
    );
    PrComment {
        id: spec.id,
        user_login: spec.agent.to_string(),
        body,
        updated_at: spec.updated_at.to_string(),
    }
}

fn all_pass_gate_comments(pr_number: u64, head: &str) -> Vec<PrComment> {
    vec![
        gate_comment(GateCommentSpec {
            id: 1,
            agent: "pr-reviewer",
            context: ADF_REVIEWER_CONTEXT,
            pr_number,
            head,
            status: "pass",
            blocking_findings: 0,
            updated_at: "2026-01-02T00:00:00Z",
        }),
        gate_comment(GateCommentSpec {
            id: 2,
            agent: "pr-validator",
            context: ADF_VALIDATION_CONTEXT,
            pr_number,
            head,
            status: "pass",
            blocking_findings: 0,
            updated_at: "2026-01-02T00:00:01Z",
        }),
        gate_comment(GateCommentSpec {
            id: 3,
            agent: "pr-verifier",
            context: ADF_VERIFICATION_CONTEXT,
            pr_number,
            head,
            status: "pass",
            blocking_findings: 0,
            updated_at: "2026-01-02T00:00:02Z",
        }),
    ]
}

fn auto_merge_depth(orch: &AgentOrchestrator) -> u64 {
    orch.dispatcher()
        .stats()
        .by_source
        .get("auto_merge")
        .copied()
        .unwrap_or(0)
}

#[tokio::test]
async fn auto_merge_requires_all_gate_statuses_and_results() {
    let mut orch =
        AgentOrchestrator::new(minimal_config(tempfile::tempdir().unwrap().keep())).unwrap();

    let head = "2ef451d8";
    let tracker = InMemoryPrTracker::new()
        .with_pr(
            pr_summary(101, "claude-code", head, 42),
            all_pass_gate_comments(101, head),
        )
        .with_statuses_for_head(head, all_green_statuses());

    orch.poll_pending_reviews_for_project(PROJECT, &tracker, &AutoMergeCriteria::default())
        .await;

    assert_eq!(auto_merge_depth(&orch), 1);

    match orch.dispatcher().peek().cloned() {
        Some(DispatchTask::AutoMerge {
            pr_number,
            project,
            head_sha,
        }) => {
            assert_eq!(pr_number, 101);
            assert_eq!(project, PROJECT);
            assert_eq!(head_sha, head);
        }
        other => panic!("expected DispatchTask::AutoMerge, got {:?}", other),
    }
}

#[tokio::test]
async fn auto_merge_blocked_on_blocking_findings() {
    let mut orch =
        AgentOrchestrator::new(minimal_config(tempfile::tempdir().unwrap().keep())).unwrap();

    let head = "62672e38";
    let mut comments = all_pass_gate_comments(202, head);
    comments[0] = gate_comment(GateCommentSpec {
        id: 10,
        agent: "pr-reviewer",
        context: ADF_REVIEWER_CONTEXT,
        pr_number: 202,
        head,
        status: "pass",
        blocking_findings: 2,
        updated_at: "2026-01-02T00:00:00Z",
    });

    let tracker = InMemoryPrTracker::new()
        .with_pr(pr_summary(202, "claude-code", head, 42), comments)
        .with_statuses_for_head(head, all_green_statuses());

    orch.poll_pending_reviews_for_project(PROJECT, &tracker, &AutoMergeCriteria::default())
        .await;

    assert_eq!(auto_merge_depth(&orch), 0);
}

#[tokio::test]
async fn auto_merge_blocked_on_non_agent_author() {
    let mut orch =
        AgentOrchestrator::new(minimal_config(tempfile::tempdir().unwrap().keep())).unwrap();

    let head = "2ef451d8";
    let tracker = InMemoryPrTracker::new()
        .with_pr(
            pr_summary(303, "alice-human", head, 42),
            all_pass_gate_comments(303, head),
        )
        .with_statuses_for_head(head, all_green_statuses());

    orch.poll_pending_reviews_for_project(PROJECT, &tracker, &AutoMergeCriteria::default())
        .await;

    assert_eq!(auto_merge_depth(&orch), 0);
}

#[tokio::test]
async fn auto_merge_blocked_on_large_diff() {
    let mut orch =
        AgentOrchestrator::new(minimal_config(tempfile::tempdir().unwrap().keep())).unwrap();

    let head = "2ef451d8";
    let tracker = InMemoryPrTracker::new()
        .with_pr(
            pr_summary(404, "claude-code", head, 501),
            all_pass_gate_comments(404, head),
        )
        .with_statuses_for_head(head, all_green_statuses());

    orch.poll_pending_reviews_for_project(PROJECT, &tracker, &AutoMergeCriteria::default())
        .await;

    assert_eq!(auto_merge_depth(&orch), 0);
}

#[tokio::test]
async fn auto_merge_idempotent_across_ticks() {
    let mut orch =
        AgentOrchestrator::new(minimal_config(tempfile::tempdir().unwrap().keep())).unwrap();

    let head = "2ef451d8";
    let tracker = InMemoryPrTracker::new()
        .with_pr(
            pr_summary(505, "claude-code", head, 42),
            all_pass_gate_comments(505, head),
        )
        .with_statuses_for_head(head, all_green_statuses());

    orch.poll_pending_reviews_for_project(PROJECT, &tracker, &AutoMergeCriteria::default())
        .await;
    assert_eq!(auto_merge_depth(&orch), 1);

    for _ in 0..5 {
        orch.poll_pending_reviews_for_project(PROJECT, &tracker, &AutoMergeCriteria::default())
            .await;
    }
    assert_eq!(auto_merge_depth(&orch), 1);
}

#[tokio::test]
async fn poll_skips_prs_without_gate_results() {
    let mut orch =
        AgentOrchestrator::new(minimal_config(tempfile::tempdir().unwrap().keep())).unwrap();

    let head = "2ef451d8";
    let noise = PrComment {
        id: 9,
        user_login: "alice-human".to_string(),
        body: "looks good to me".to_string(),
        updated_at: "2026-01-02T00:00:00Z".to_string(),
    };
    let tracker = InMemoryPrTracker::new()
        .with_pr(pr_summary(606, "claude-code", head, 42), vec![noise])
        .with_statuses_for_head(head, all_green_statuses());

    orch.poll_pending_reviews_for_project(PROJECT, &tracker, &AutoMergeCriteria::default())
        .await;

    assert_eq!(auto_merge_depth(&orch), 0);
}

#[tokio::test]
async fn poll_waits_on_stale_gate_head_sha() {
    let mut orch =
        AgentOrchestrator::new(minimal_config(tempfile::tempdir().unwrap().keep())).unwrap();

    let head = "6becb2f7";
    let tracker = InMemoryPrTracker::new()
        .with_pr(
            pr_summary(707, "claude-code", head, 42),
            all_pass_gate_comments(707, "older-sha"),
        )
        .with_statuses_for_head(head, all_green_statuses());

    orch.poll_pending_reviews_for_project(PROJECT, &tracker, &AutoMergeCriteria::default())
        .await;

    assert_eq!(auto_merge_depth(&orch), 0);
}

#[tokio::test]
async fn terraphim_ai_holds_on_concerns() {
    let mut orch =
        AgentOrchestrator::new(minimal_config(tempfile::tempdir().unwrap().keep())).unwrap();

    let head = "abc12345";
    let mut comments = all_pass_gate_comments(808, head);
    comments[1] = gate_comment(GateCommentSpec {
        id: 21,
        agent: "pr-validator",
        context: ADF_VALIDATION_CONTEXT,
        pr_number: 808,
        head,
        status: "concerns",
        blocking_findings: 0,
        updated_at: "2026-01-02T00:00:01Z",
    });

    let tracker = InMemoryPrTracker::new()
        .with_pr(pr_summary(808, "claude-code", head, 42), comments)
        .with_statuses_for_head(head, all_green_statuses());

    orch.poll_pending_reviews_for_project("terraphim-ai", &tracker, &AutoMergeCriteria::default())
        .await;

    assert_eq!(auto_merge_depth(&orch), 0);
}

#[tokio::test]
async fn sub_repo_merges_on_concerns_without_blocking_findings() {
    let mut orch =
        AgentOrchestrator::new(minimal_config(tempfile::tempdir().unwrap().keep())).unwrap();

    let head = "abc12345";
    let mut comments = all_pass_gate_comments(909, head);
    comments[1] = gate_comment(GateCommentSpec {
        id: 31,
        agent: "pr-validator",
        context: ADF_VALIDATION_CONTEXT,
        pr_number: 909,
        head,
        status: "concerns",
        blocking_findings: 0,
        updated_at: "2026-01-02T00:00:01Z",
    });

    let tracker = InMemoryPrTracker::new()
        .with_pr(pr_summary(909, "claude-code", head, 42), comments)
        .with_statuses_for_head(head, all_green_statuses());

    orch.poll_pending_reviews_for_project(PROJECT, &tracker, &AutoMergeCriteria::default())
        .await;

    assert_eq!(auto_merge_depth(&orch), 1);
}
