//! Integration tests for ROC v1 Step G — AutoMerge handler execution.
//!
//! Drives [`AgentOrchestrator::handle_auto_merge_for_project`] against an
//! in-memory [`AutoMergeExecutor`] that records every call. No mocks, no
//! network, no Gitea — the test impl is a plain struct backed by `Mutex`
//! so the handler can observe state mutations across async calls.
//!
//! Asserts four behaviours:
//!
//! 1. Merge success enqueues a [`DispatchTask::PostMergeTestGate`] carrying
//!    the merge commit SHA + PR title, and records the revision in the
//!    dedupe set so late polls never re-enqueue.
//! 2. A HEAD SHA mismatch between the dispatch payload and the live PR
//!    causes the handler to skip the merge entirely (stale decision).
//! 3. A merge failure (conflict, API error) opens an `[ADF]` tracking
//!    issue on the project repo and does **not** enqueue a post-merge gate.
//! 4. A successful merge leaves the dedupe set marked — documented via
//!    [`AgentOrchestrator::auto_merge_enqueued`].
//!
//! See `cto-executive-system/plans/adf-rate-of-change-design.md` §Step G
//! and Gitea issue `terraphim/adf-fleet#35`.

use std::path::PathBuf;
use std::sync::Mutex;

use async_trait::async_trait;
use terraphim_orchestrator::pr_poller::{
    AutoMergeExecutor, MergeOutcome, PrComment, PrSummary, PrTracker,
};
use terraphim_orchestrator::{
    AgentOrchestrator, CompoundReviewConfig, DispatchTask, NightwatchConfig, OrchestratorConfig,
};

const PROJECT: &str = "alpha";

fn minimal_config() -> OrchestratorConfig {
    OrchestratorConfig {
        working_dir: PathBuf::from("/tmp/terraphim-auto-merge-execution-tests"),
        nightwatch: NightwatchConfig::default(),
        compound_review: CompoundReviewConfig {
            cli_tool: None,
            provider: None,
            model: None,
            schedule: "0 2 * * *".to_string(),
            max_duration_secs: 60,
            repo_path: PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../.."),
            create_prs: false,
            worktree_root: PathBuf::from("/tmp/terraphim-auto-merge-execution-tests/.worktrees"),
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
    }
}

/// Recorded `merge_pr` call.
#[derive(Debug, Clone, PartialEq, Eq)]
struct MergeCall {
    pr_number: u64,
}

/// Recorded `open_failure_issue` call.
#[derive(Debug, Clone, PartialEq, Eq)]
struct FailureIssueCall {
    title: String,
    body: String,
    labels: Vec<String>,
}

/// Configurable `merge_pr` outcome: either succeed with a specific merge
/// commit sha/title, or fail with an error message.
#[derive(Debug, Clone)]
enum MergeReply {
    Ok {
        merge_commit_sha: String,
        title: String,
    },
    Err(String),
}

/// In-memory [`AutoMergeExecutor`] that records every call and replays
/// programmable outcomes. No mock framework involved — just a struct with
/// mutex-guarded state.
struct RecordingExecutor {
    open_prs: Vec<PrSummary>,
    merge_reply: MergeReply,
    merge_calls: Mutex<Vec<MergeCall>>,
    failure_issue_calls: Mutex<Vec<FailureIssueCall>>,
    next_issue_number: Mutex<u64>,
}

impl RecordingExecutor {
    fn new(open_prs: Vec<PrSummary>, merge_reply: MergeReply) -> Self {
        Self {
            open_prs,
            merge_reply,
            merge_calls: Mutex::new(Vec::new()),
            failure_issue_calls: Mutex::new(Vec::new()),
            next_issue_number: Mutex::new(1001),
        }
    }

    fn merge_calls(&self) -> Vec<MergeCall> {
        self.merge_calls.lock().unwrap().clone()
    }

    fn failure_issues(&self) -> Vec<FailureIssueCall> {
        self.failure_issue_calls.lock().unwrap().clone()
    }
}

#[async_trait]
impl PrTracker for RecordingExecutor {
    async fn list_open_prs(&self) -> Result<Vec<PrSummary>, String> {
        Ok(self.open_prs.clone())
    }

    async fn fetch_pr_comments(&self, _pr_number: u64) -> Result<Vec<PrComment>, String> {
        Ok(vec![])
    }
}

#[async_trait]
impl AutoMergeExecutor for RecordingExecutor {
    async fn merge_pr(&self, pr_number: u64) -> Result<MergeOutcome, String> {
        self.merge_calls
            .lock()
            .unwrap()
            .push(MergeCall { pr_number });
        match &self.merge_reply {
            MergeReply::Ok {
                merge_commit_sha,
                title,
            } => Ok(MergeOutcome {
                pr_number,
                merge_commit_sha: merge_commit_sha.clone(),
                title: title.clone(),
            }),
            MergeReply::Err(msg) => Err(msg.clone()),
        }
    }

    async fn open_failure_issue(
        &self,
        title: &str,
        body: &str,
        labels: &[&str],
    ) -> Result<u64, String> {
        self.failure_issue_calls
            .lock()
            .unwrap()
            .push(FailureIssueCall {
                title: title.to_string(),
                body: body.to_string(),
                labels: labels.iter().map(|l| l.to_string()).collect(),
            });
        let mut n = self.next_issue_number.lock().unwrap();
        let issued = *n;
        *n += 1;
        Ok(issued)
    }
}

fn pr(number: u64, head: &str, diff_loc: u32) -> PrSummary {
    PrSummary {
        number,
        author_login: "claude-code".to_string(),
        head_sha: head.to_string(),
        base_ref: "main".to_string(),
        diff_loc,
    }
}

fn auto_merge_task(pr_number: u64, project: &str, head: &str) -> DispatchTask {
    DispatchTask::AutoMerge {
        pr_number,
        project: project.to_string(),
        head_sha: head.to_string(),
    }
}

fn post_merge_depth(orch: &AgentOrchestrator) -> u64 {
    orch.dispatcher()
        .stats()
        .by_source
        .get("post_merge_gate")
        .copied()
        .unwrap_or(0)
}

fn auto_merge_depth(orch: &AgentOrchestrator) -> u64 {
    orch.dispatcher()
        .stats()
        .by_source
        .get("auto_merge")
        .copied()
        .unwrap_or(0)
}

// -------- Test 1: merge success enqueues PostMergeTestGate --------

#[tokio::test]
async fn auto_merge_success_enqueues_post_merge_gate() {
    let mut orch = AgentOrchestrator::new(minimal_config()).unwrap();

    let executor = RecordingExecutor::new(
        vec![pr(101, "2ef451d8", 42)],
        MergeReply::Ok {
            merge_commit_sha: "merge-sha-abc".to_string(),
            title: "feat(foo): green change".to_string(),
        },
    );

    orch.handle_auto_merge_for_project(auto_merge_task(101, PROJECT, "2ef451d8"), &executor)
        .await
        .expect("handler succeeded");

    // Merge actually happened.
    assert_eq!(
        executor.merge_calls(),
        vec![MergeCall { pr_number: 101 }],
        "handler must have invoked merge_pr exactly once on success"
    );

    // No [ADF] issues opened on success.
    assert!(
        executor.failure_issues().is_empty(),
        "no failure issue must be opened on merge success"
    );

    // Exactly one PostMergeTestGate enqueued, carrying the merge sha + title.
    assert_eq!(
        post_merge_depth(&orch),
        1,
        "successful merge must enqueue one PostMergeTestGate"
    );
    match orch.dispatcher().peek().cloned() {
        Some(DispatchTask::PostMergeTestGate {
            pr_number,
            project,
            merge_sha,
            title,
        }) => {
            assert_eq!(pr_number, 101);
            assert_eq!(project, PROJECT);
            assert_eq!(merge_sha, "merge-sha-abc");
            assert_eq!(title, "feat(foo): green change");
        }
        other => panic!("expected DispatchTask::PostMergeTestGate, got {:?}", other),
    }
}

// -------- Test 2: HEAD SHA changed -> skip merge (stale decision) --------

#[tokio::test]
async fn auto_merge_skipped_when_head_sha_changed() {
    let mut orch = AgentOrchestrator::new(minimal_config()).unwrap();

    // Live PR has a DIFFERENT head sha from the dispatch payload.
    let executor = RecordingExecutor::new(
        vec![pr(202, "live-sha-xyz", 42)],
        MergeReply::Ok {
            merge_commit_sha: "should-not-happen".to_string(),
            title: "n/a".to_string(),
        },
    );

    orch.handle_auto_merge_for_project(auto_merge_task(202, PROJECT, "stale-sha-abc"), &executor)
        .await
        .expect("handler returns Ok(()) on skip");

    // The merge API must NEVER be called when the head sha has moved.
    assert!(
        executor.merge_calls().is_empty(),
        "stale head sha must prevent the merge call from ever being issued"
    );

    // And nothing must be enqueued downstream.
    assert_eq!(
        post_merge_depth(&orch),
        0,
        "skipped stale merge must not enqueue PostMergeTestGate"
    );
    assert_eq!(
        auto_merge_depth(&orch),
        0,
        "skipped stale merge must not re-enqueue AutoMerge"
    );
    assert!(
        executor.failure_issues().is_empty(),
        "a stale decision is not a failure; no [ADF] issue must be opened"
    );
}

// -------- Test 3: merge failure opens [ADF] tracking issue --------

#[tokio::test]
async fn auto_merge_failure_opens_adf_issue() {
    let mut orch = AgentOrchestrator::new(minimal_config()).unwrap();

    let executor = RecordingExecutor::new(
        vec![pr(303, "2ef451d8", 42)],
        MergeReply::Err(
            "Gitea merge_pull error 409 on PR 303: Please resolve merge conflicts".to_string(),
        ),
    );

    orch.handle_auto_merge_for_project(auto_merge_task(303, PROJECT, "2ef451d8"), &executor)
        .await
        .expect("handler returns Ok(()) even on merge failure");

    // Merge was attempted exactly once.
    assert_eq!(
        executor.merge_calls(),
        vec![MergeCall { pr_number: 303 }],
        "handler must have invoked merge_pr once before giving up"
    );

    // A single [ADF] failure issue was opened with the failure reason
    // and the PR number in the title, and the adf + auto-merge-failed
    // labels were attached.
    let failures = executor.failure_issues();
    assert_eq!(
        failures.len(),
        1,
        "merge failure must open exactly one [ADF] tracking issue"
    );
    let issue = &failures[0];
    assert!(
        issue.title.starts_with("[ADF]") && issue.title.contains("303"),
        "failure issue title must start with [ADF] and reference PR 303; got `{}`",
        issue.title
    );
    assert!(
        issue.body.contains("409") && issue.body.contains("2ef451d8"),
        "failure issue body must record the error + head sha for forensics; got `{}`",
        issue.body
    );
    assert!(
        issue.labels.iter().any(|l| l == "adf"),
        "failure issue must carry the `adf` label"
    );
    assert!(
        issue.labels.iter().any(|l| l == "auto-merge-failed"),
        "failure issue must carry the `auto-merge-failed` label"
    );

    // No PostMergeTestGate must be enqueued when the merge failed.
    assert_eq!(
        post_merge_depth(&orch),
        0,
        "merge failure must NOT enqueue PostMergeTestGate"
    );
}

// -------- Test 4: success marks dedupe set so subsequent polls skip --------

#[tokio::test]
async fn auto_merge_marks_dedupe_set_on_success() {
    let mut orch = AgentOrchestrator::new(minimal_config()).unwrap();

    let executor = RecordingExecutor::new(
        vec![pr(404, "2ef451d8", 42)],
        MergeReply::Ok {
            merge_commit_sha: "merge-sha-dedupe".to_string(),
            title: "feat: dedupe test".to_string(),
        },
    );

    orch.handle_auto_merge_for_project(auto_merge_task(404, PROJECT, "2ef451d8"), &executor)
        .await
        .expect("handler succeeded");

    // Dedupe set must contain the (project, pr_number, head_sha) triple
    // so a subsequent poll for the same revision skips re-enqueuing.
    assert!(
        orch.auto_merge_enqueued()
            .contains(PROJECT, 404, "2ef451d8"),
        "successful AutoMerge must record (project, pr, head_sha) in the dedupe set"
    );

    // PostMergeTestGate was enqueued; no AutoMerge is left in the queue.
    assert_eq!(post_merge_depth(&orch), 1);
    assert_eq!(
        auto_merge_depth(&orch),
        0,
        "handler never re-enqueues AutoMerge for itself"
    );
}

// -------- Test 5: PR no longer open -> skip merge silently --------

#[tokio::test]
async fn auto_merge_skipped_when_pr_already_closed() {
    let mut orch = AgentOrchestrator::new(minimal_config()).unwrap();

    // The open-PR list does NOT contain PR 505 — someone already merged or
    // closed it between the verdict and this tick.
    let executor = RecordingExecutor::new(
        vec![pr(999, "other-sha", 10)],
        MergeReply::Ok {
            merge_commit_sha: "should-not-happen".to_string(),
            title: "n/a".to_string(),
        },
    );

    orch.handle_auto_merge_for_project(auto_merge_task(505, PROJECT, "any-sha"), &executor)
        .await
        .expect("handler returns Ok(()) on skip");

    assert!(
        executor.merge_calls().is_empty(),
        "handler must not merge a PR that is no longer open"
    );
    assert_eq!(
        post_merge_depth(&orch),
        0,
        "no PostMergeTestGate when PR was already closed"
    );
    assert!(
        executor.failure_issues().is_empty(),
        "an already-closed PR is not a failure; no [ADF] issue"
    );
}
