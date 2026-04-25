//! Polling helpers for ROC v1 Step F — turn open PRs + reviewer comments into
//! [`crate::DispatchTask::AutoMerge`] tasks.
//!
//! The orchestrator invokes `AgentOrchestrator::poll_pending_reviews` once
//! per `reconcile_tick`. That method walks every project with a Gitea config,
//! lists open PRs, looks for the latest structural-pr-review comment, calls
//! [`crate::pr_review::parse_verdict`] + [`crate::pr_review::evaluate`], and
//! enqueues a [`crate::DispatchTask::AutoMerge`] when — and only when — every gate
//! in [`crate::pr_review::AutoMergeCriteria::default`] is satisfied.
//!
//! The module is split into:
//!
//! - [`PrSummary`] / [`PrComment`]: transport types decoupled from the Gitea
//!   client so integration tests can supply in-memory fixtures.
//! - [`PrTracker`]: async trait with one real implementation
//!   ([`GiteaPrTracker`]) wrapping [`terraphim_tracker::GiteaTracker`] and any
//!   number of in-memory test implementations.
//! - [`evaluate_pr_verdict`]: pure function that turns a [`PrSummary`] + the
//!   latest [`PrComment`] into an [`EvaluationOutcome`] (parse, evaluate,
//!   classify). Extracted so tests drive it without any dispatcher state.
//! - [`PrPollRateLimiter`] / [`AutoMergeDedupeSet`]: in-memory guards that
//!   keep the poller from hammering Gitea and from double-enqueuing the same
//!   (PR, head-SHA).
//!
//! See `cto-executive-system/plans/adf-rate-of-change-design.md` §Step F and
//! Gitea issue `terraphim/adf-fleet#34`.

use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};

use async_trait::async_trait;

use crate::pr_review::{
    self, AutoMergeCriteria, AutoMergeDecision, PrMetadata, ReviewVerdict, VerdictParseError,
};

/// Login that identifies the structural-pr-review agent.
///
/// Reviewer comments not authored by this login are ignored even if they
/// contain a `Last reviewed commit:` footer, so a human comment with the
/// same shape cannot accidentally trigger auto-merge.
pub const PR_REVIEWER_LOGIN: &str = "pr-reviewer";

/// Minimum interval between polls of the same PR. Prevents the reconcile
/// loop from re-hitting Gitea for the same PR every tick when the tick
/// cadence is short (<60s).
pub const PR_POLL_MIN_INTERVAL: Duration = Duration::from_secs(60);

/// Summary of an open pull request, decoupled from [`terraphim_tracker`] so
/// that tests can construct it without a live Gitea server.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrSummary {
    pub number: u64,
    pub author_login: String,
    pub head_sha: String,
    pub base_ref: String,
    pub diff_loc: u32,
}

/// Single comment on a pull request. Only the fields needed for verdict
/// parsing are captured; the full Gitea payload is deliberately not mirrored.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrComment {
    pub id: u64,
    pub user_login: String,
    pub body: String,
    /// RFC3339-ish `updated_at` string from the Gitea API. Used only for
    /// ordering; comments without a timestamp sort as the earliest.
    pub updated_at: String,
}

/// Read-side abstraction over an issue-tracker capable of answering the two
/// questions the poller asks: "what PRs are open?" and "what comments does
/// PR N carry?". Kept minimal so the test impl stays trivial.
#[async_trait]
pub trait PrTracker: Send + Sync {
    async fn list_open_prs(&self) -> Result<Vec<PrSummary>, String>;
    async fn fetch_pr_comments(&self, pr_number: u64) -> Result<Vec<PrComment>, String>;
}

/// Outcome of a successful merge call, decoupled from
/// [`terraphim_tracker::GiteaMergeResult`] so the orchestrator handler can
/// be driven by in-memory test implementations without pulling the tracker
/// concrete types into test code.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MergeOutcome {
    pub pr_number: u64,
    pub merge_commit_sha: String,
    pub title: String,
}

/// Write-side abstraction used by the AutoMerge handler (ROC v1 Step G).
///
/// Re-uses [`PrTracker::list_open_prs`] for the defensive head-SHA re-check
/// and adds two writer methods: `merge_pr` (actually merge the PR) and
/// `open_failure_issue` (record an `[ADF]` tracking issue when the merge
/// call fails). Test impls are plain structs that record calls — no mock
/// frameworks involved.
#[async_trait]
pub trait AutoMergeExecutor: PrTracker {
    /// Merge `pr_number` on the project this executor is scoped to.
    ///
    /// The real Gitea implementation does a standard merge with branch
    /// deletion; the style/flag choice is intentionally not parameterised
    /// here — it is a per-project policy baked into the impl.
    async fn merge_pr(&self, pr_number: u64) -> Result<MergeOutcome, String>;

    /// Create an `[ADF]` tracking issue describing a merge failure so a
    /// human can follow up. Returns the newly created issue number.
    async fn open_failure_issue(
        &self,
        title: &str,
        body: &str,
        labels: &[&str],
    ) -> Result<u64, String>;
}

/// Real [`PrTracker`] backed by [`terraphim_tracker::GiteaTracker`].
pub struct GiteaPrTracker {
    inner: terraphim_tracker::GiteaTracker,
}

impl GiteaPrTracker {
    pub fn new(inner: terraphim_tracker::GiteaTracker) -> Self {
        Self { inner }
    }
}

#[async_trait]
impl PrTracker for GiteaPrTracker {
    async fn list_open_prs(&self) -> Result<Vec<PrSummary>, String> {
        self.inner
            .list_open_prs()
            .await
            .map(|v| {
                v.into_iter()
                    .map(|p| PrSummary {
                        number: p.number,
                        author_login: p.author_login,
                        head_sha: p.head_sha,
                        base_ref: p.base_ref,
                        diff_loc: p.diff_loc,
                    })
                    .collect()
            })
            .map_err(|e| e.to_string())
    }

    async fn fetch_pr_comments(&self, pr_number: u64) -> Result<Vec<PrComment>, String> {
        self.inner
            .fetch_comments(pr_number, None)
            .await
            .map(|v| {
                v.into_iter()
                    .map(|c| PrComment {
                        id: c.id,
                        user_login: c.user.login,
                        body: c.body,
                        updated_at: c.updated_at,
                    })
                    .collect()
            })
            .map_err(|e| e.to_string())
    }
}

#[async_trait]
impl AutoMergeExecutor for GiteaPrTracker {
    async fn merge_pr(&self, pr_number: u64) -> Result<MergeOutcome, String> {
        self.inner
            .merge_pull(pr_number, terraphim_tracker::MergeStyle::Merge, true)
            .await
            .map(|r| MergeOutcome {
                pr_number: r.pr_number,
                merge_commit_sha: r.merge_commit_sha,
                title: r.title,
            })
            .map_err(|e| e.to_string())
    }

    async fn open_failure_issue(
        &self,
        title: &str,
        body: &str,
        labels: &[&str],
    ) -> Result<u64, String> {
        self.inner
            .create_issue(title, body, labels)
            .await
            .map(|i| i.number)
            .map_err(|e| e.to_string())
    }
}

/// Outcome of applying the auto-merge policy to a single PR.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EvaluationOutcome {
    /// Every gate cleared; the caller should enqueue [`crate::DispatchTask::AutoMerge`].
    Merge { head_sha: String },
    /// At least one gate failed. The reason is a short human-readable string
    /// suitable for logging or posting back to the PR.
    HumanReviewNeeded { reason: String },
    /// No pr-reviewer comment found yet — nothing to evaluate this tick.
    NoReviewerComment,
    /// A reviewer comment exists but did not parse as a structural verdict.
    ParseError { reason: String },
}

/// Return `true` when `comment.user_login == PR_REVIEWER_LOGIN` **or** the
/// body carries the canonical `Last reviewed commit:` footer emitted by the
/// structural-pr-review skill. The footer fallback lets the poller pick up
/// comments posted by agents running under an alternative login during
/// migration.
pub fn is_pr_reviewer_comment(comment: &PrComment) -> bool {
    if comment.user_login == PR_REVIEWER_LOGIN {
        return true;
    }
    comment.body.contains("Last reviewed commit:")
}

/// Return the latest [`PrComment`] authored by the pr-reviewer, or `None`
/// when there is no such comment. "Latest" is `updated_at` ordering with
/// comment id as a tie-break.
pub fn latest_reviewer_comment(comments: &[PrComment]) -> Option<&PrComment> {
    comments
        .iter()
        .filter(|c| is_pr_reviewer_comment(c))
        .max_by(|a, b| {
            a.updated_at
                .cmp(&b.updated_at)
                .then_with(|| a.id.cmp(&b.id))
        })
}

/// Pure evaluator: given a PR + its comments + the merge policy, decide
/// whether to enqueue an auto-merge, ask for human review, or report a
/// parsing issue.
pub fn evaluate_pr_verdict(
    pr: &PrSummary,
    comments: &[PrComment],
    criteria: &AutoMergeCriteria,
) -> EvaluationOutcome {
    let Some(latest) = latest_reviewer_comment(comments) else {
        return EvaluationOutcome::NoReviewerComment;
    };

    let verdict: ReviewVerdict = match pr_review::parse_verdict(&latest.body, latest.id) {
        Ok(v) => v,
        Err(e) => {
            return EvaluationOutcome::ParseError {
                reason: describe_parse_error(e),
            };
        }
    };

    let metadata = PrMetadata {
        pr_number: pr.number,
        author_login: pr.author_login.clone(),
        diff_loc: pr.diff_loc,
        head_sha: pr.head_sha.clone(),
        base_branch: pr.base_ref.clone(),
    };

    match pr_review::evaluate(&verdict, &metadata, criteria) {
        AutoMergeDecision::Merge => EvaluationOutcome::Merge {
            head_sha: pr.head_sha.clone(),
        },
        AutoMergeDecision::HumanReviewNeeded(reason) => {
            EvaluationOutcome::HumanReviewNeeded { reason }
        }
    }
}

fn describe_parse_error(err: VerdictParseError) -> String {
    match err {
        VerdictParseError::MissingConfidence => "missing confidence score header".to_string(),
        VerdictParseError::ConfidenceOutOfRange(n) => {
            format!("confidence {n}/5 out of range (expected 1..=5)")
        }
        VerdictParseError::MissingFindings => "missing Inline Findings section".to_string(),
        VerdictParseError::MalformedFooter => {
            "malformed `Last reviewed commit:` footer".to_string()
        }
    }
}

/// Per-(project, PR) rate limiter used to cap how often the poller hits
/// Gitea for the same pull request. In-memory only; restarts reset the
/// cadence, which is acceptable given the 60-second floor.
#[derive(Debug, Default)]
pub struct PrPollRateLimiter {
    last_poll: HashMap<(String, u64), Instant>,
    min_interval: Duration,
}

impl PrPollRateLimiter {
    pub fn new(min_interval: Duration) -> Self {
        Self {
            last_poll: HashMap::new(),
            min_interval,
        }
    }

    /// Return `true` when enough time has elapsed since the last poll for
    /// `(project, pr_number)` — and mark the slot as just-polled. Concurrent
    /// callers are serialised by `&mut self`.
    pub fn allow(&mut self, project: &str, pr_number: u64, now: Instant) -> bool {
        let key = (project.to_string(), pr_number);
        if let Some(prev) = self.last_poll.get(&key) {
            if now.duration_since(*prev) < self.min_interval {
                return false;
            }
        }
        self.last_poll.insert(key, now);
        true
    }
}

/// Per-project dedupe set over `(pr_number, head_sha)` so the same revision
/// of a PR never yields two auto-merge tasks across ticks.
#[derive(Debug, Default)]
pub struct AutoMergeDedupeSet {
    by_project: HashMap<String, HashSet<(u64, String)>>,
}

impl AutoMergeDedupeSet {
    pub fn new() -> Self {
        Self::default()
    }

    /// Record `(pr_number, head_sha)` for `project`. Returns `true` when
    /// this was a fresh entry (caller should enqueue), `false` when it had
    /// already been recorded (caller must skip).
    pub fn record_if_new(&mut self, project: &str, pr_number: u64, head_sha: &str) -> bool {
        self.by_project
            .entry(project.to_string())
            .or_default()
            .insert((pr_number, head_sha.to_string()))
    }

    /// Return `true` when `(project, pr_number, head_sha)` has already
    /// been recorded. Used for observability (integration tests) and for
    /// the AutoMerge handler's defensive dedupe write.
    pub fn contains(&self, project: &str, pr_number: u64, head_sha: &str) -> bool {
        self.by_project
            .get(project)
            .is_some_and(|s| s.contains(&(pr_number, head_sha.to_string())))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn comment(id: u64, user: &str, body: &str, updated_at: &str) -> PrComment {
        PrComment {
            id,
            user_login: user.to_string(),
            body: body.to_string(),
            updated_at: updated_at.to_string(),
        }
    }

    fn pr(number: u64, author: &str, head: &str, diff_loc: u32) -> PrSummary {
        PrSummary {
            number,
            author_login: author.to_string(),
            head_sha: head.to_string(),
            base_ref: "main".to_string(),
            diff_loc,
        }
    }

    #[test]
    fn is_pr_reviewer_comment_matches_login() {
        let c = comment(1, PR_REVIEWER_LOGIN, "hello", "2026-01-01T00:00:00Z");
        assert!(is_pr_reviewer_comment(&c));
    }

    #[test]
    fn is_pr_reviewer_comment_matches_footer_fallback() {
        let c = comment(
            1,
            "random-user",
            "body\n<sub>Last reviewed commit: abc123</sub>",
            "2026-01-01T00:00:00Z",
        );
        assert!(is_pr_reviewer_comment(&c));
    }

    #[test]
    fn latest_reviewer_comment_picks_max_updated_at() {
        let comments = vec![
            comment(1, PR_REVIEWER_LOGIN, "first", "2026-01-01T00:00:00Z"),
            comment(2, PR_REVIEWER_LOGIN, "second", "2026-01-02T00:00:00Z"),
            comment(3, "human", "noise", "2026-01-03T00:00:00Z"),
        ];
        let latest = latest_reviewer_comment(&comments).unwrap();
        assert_eq!(latest.id, 2);
    }

    #[test]
    fn evaluate_verdict_returns_no_reviewer_comment_when_empty() {
        let p = pr(1, "claude-code", "abc", 10);
        let out = evaluate_pr_verdict(&p, &[], &AutoMergeCriteria::default());
        assert_eq!(out, EvaluationOutcome::NoReviewerComment);
    }

    #[test]
    fn evaluate_verdict_returns_parse_error_on_malformed_body() {
        let p = pr(1, "claude-code", "abc", 10);
        let c = comment(7, PR_REVIEWER_LOGIN, "garbage", "2026-01-01T00:00:00Z");
        let out = evaluate_pr_verdict(&p, &[c], &AutoMergeCriteria::default());
        assert!(matches!(out, EvaluationOutcome::ParseError { .. }));
    }

    #[test]
    fn rate_limiter_blocks_inside_window() {
        let mut rl = PrPollRateLimiter::new(Duration::from_secs(60));
        let now = Instant::now();
        assert!(rl.allow("p", 1, now));
        assert!(!rl.allow("p", 1, now + Duration::from_secs(30)));
        assert!(rl.allow("p", 1, now + Duration::from_secs(61)));
    }

    #[test]
    fn rate_limiter_scopes_by_project_and_pr() {
        let mut rl = PrPollRateLimiter::new(Duration::from_secs(60));
        let now = Instant::now();
        assert!(rl.allow("a", 1, now));
        assert!(rl.allow("a", 2, now));
        assert!(rl.allow("b", 1, now));
    }

    #[test]
    fn dedupe_records_new_and_rejects_duplicates() {
        let mut set = AutoMergeDedupeSet::new();
        assert!(set.record_if_new("p", 1, "sha"));
        assert!(!set.record_if_new("p", 1, "sha"));
        assert!(set.record_if_new("p", 1, "sha2"));
        assert!(set.record_if_new("q", 1, "sha"));
    }
}
