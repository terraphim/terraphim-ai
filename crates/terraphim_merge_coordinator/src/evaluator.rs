//! PR evaluation + merge-and-close orchestration (#1805).
//!
//! PRs are evaluated strictly sequentially within a run (no concurrency).
//! Partial failure is surfaced as CRITICAL: a merge that succeeds but whose
//! follow-up close call fails is not silently retried. Remediation
//! (comment + exit) is applied atomically per PR.

use tracing::{error, info, warn};

use crate::extract_fixes;
use crate::gitea::{GiteaOperations, PrSummary};
use crate::types::{BlockerKind, EvalVerdict, MergeCoordinatorError, MergeOutcome};

/// One evaluation of one open PR.
#[derive(Debug, Clone)]
pub struct PrEvaluation {
    /// Gitea PR index (number).
    pub pr_index: u64,
    /// Whether the PR is currently mergeable according to Gitea.
    pub mergeable: bool,
    /// Head commit SHA as seen during evaluation (#3295 §D6). The pre-merge
    /// refetch must observe the exact same non-empty SHA before any merge is
    /// attempted; a drift means the evaluated review no longer describes the
    /// code that would land.
    pub head_sha: Option<String>,
    /// Issue numbers referenced by `Fixes #N` in the PR body.
    pub fixes_issues: Vec<u64>,
    /// Verdict reached during evaluation.
    pub verdict: EvalVerdict,
    /// Classified blocker kind when verdict is Hold (None for Merge).
    pub blocker_kind: Option<BlockerKind>,
}

/// Evaluate all open PRs in `owner/repo`, sequentially. Each PR gets
/// a verdict; no merges are performed here.
///
/// `gitea` is generic over [`GiteaOperations`] so the business logic can be
/// exercised in tests with a concrete fake instead of a live server
/// (issue #2892). Behaviour is identical whether the caller passes a
/// production `GiteaClient` or a test fake.
pub async fn evaluate_all<T: GiteaOperations>(
    gitea: &T,
    owner: &str,
    repo: &str,
) -> Result<Vec<PrEvaluation>, MergeCoordinatorError> {
    let prs = gitea.list_open_prs(owner, repo).await?;
    let mut out = Vec::with_capacity(prs.len());
    for pr in prs {
        out.push(evaluate_one(Some(gitea), owner, repo, &pr).await);
    }
    info!(count = out.len(), owner, repo, "evaluated open PRs");
    Ok(out)
}

#[allow(clippy::collapsible_match)]
async fn evaluate_one<T: GiteaOperations>(
    gitea: Option<&T>,
    owner: &str,
    repo: &str,
    pr: &PrSummary,
) -> PrEvaluation {
    let mergeable = pr.mergeable.unwrap_or(false);
    let fixes_issues = extract_fixes(pr.body.as_deref().unwrap_or(""));

    // Check contamination before mergeability to prevent artefact PRs from merging.
    if let Some(c) = gitea
        && let Err(reason) = check_contamination(c, owner, repo, pr.number).await
    {
        return PrEvaluation {
            pr_index: pr.number,
            mergeable,
            head_sha: pr.head_sha.clone(),
            fixes_issues,
            verdict: EvalVerdict::Hold(reason),
            blocker_kind: None,
        };
    }

    let (verdict, blocker_kind) = if !mergeable {
        let kind = classify_blocker(gitea, owner, repo, pr).await;
        let reason = format!("not mergeable ({kind})");
        (EvalVerdict::Hold(reason), Some(kind))
    } else {
        (EvalVerdict::Merge, None)
    };
    PrEvaluation {
        pr_index: pr.number,
        mergeable,
        head_sha: pr.head_sha.clone(),
        fixes_issues,
        verdict,
        blocker_kind,
    }
}

/// Check PR file list for contamination (artefacts, session dumps, etc.).
///
/// Returns `Ok(())` if clean, `Err(reason)` if contaminated.
///
/// Patterns match as directory components: a file is contaminated if it
/// starts with a pattern (e.g. `.sessions/session.md`) or contains the
/// pattern preceded by `/` (e.g. `path/.sessions/session.md`).  Plain
/// substring matching is avoided to prevent false positives like
/// `src/sessions_parser.rs` matching `.sessions/`.
async fn check_contamination<T: GiteaOperations>(
    gitea: &T,
    owner: &str,
    repo: &str,
    pr_index: u64,
) -> Result<(), String> {
    const CONTAMINATED_PATTERNS: &[&str] = &[".sessions/", ".review_tmp/", ".handoff/", ".beads/"];

    let files = gitea
        .list_pr_files(owner, repo, pr_index)
        .await
        .map_err(|e| format!("contamination check failed: {e}"))?;

    for file in &files {
        for pattern in CONTAMINATED_PATTERNS {
            if file.starts_with(pattern) {
                return Err(format!("contaminated: {file} (pattern: {pattern})"));
            }
            // Check for directory-component match: "/.sessions/" within path
            let component = ["/", pattern].concat();
            if file.contains(&component) {
                return Err(format!("contaminated: {file} (pattern: {pattern})"));
            }
        }
    }

    Ok(())
}

/// Query CI status and classify why a PR is blocked.
async fn classify_blocker<T: GiteaOperations>(
    gitea: Option<&T>,
    owner: &str,
    repo: &str,
    pr: &PrSummary,
) -> BlockerKind {
    let gitea = match gitea {
        Some(c) => c,
        None => return BlockerKind::CiNoStatus,
    };
    let sha = match pr.head_sha.as_deref() {
        Some(s) if !s.is_empty() => s,
        _ => return BlockerKind::CiNoStatus,
    };

    match gitea.get_commit_status(owner, repo, sha).await {
        Ok(Some(combined)) => match combined.state.as_str() {
            "failure" | "error" => BlockerKind::CiFailed,
            "pending" => BlockerKind::CiPending,
            _ => BlockerKind::NotMergeable,
        },
        Ok(None) => BlockerKind::CiNoStatus,
        Err(e) => {
            warn!(pr = pr.number, error = %e, "failed to query CI status");
            BlockerKind::CiNoStatus
        }
    }
}

/// Compare the freshly-refetched PR against what was evaluated.
///
/// Returns `Some(reason)` when the merge must be suppressed, `None` when the
/// fresh state matches the evaluated state exactly. Pure function over the
/// two snapshots so every suppression branch is unit-testable in isolation.
fn stale_state_reason(fresh: PrSummary, eval: &PrEvaluation) -> Option<String> {
    if !fresh.state.eq_ignore_ascii_case("open") {
        // Gitea reports both closed and already-merged PRs as "closed".
        return Some(format!(
            "PR closed or merged before merge (state: {})",
            fresh.state
        ));
    }
    if fresh.mergeable != Some(true) {
        return Some(format!(
            "PR no longer mergeable (mergeable: {:?})",
            fresh.mergeable
        ));
    }
    match (nonempty_sha(&eval.head_sha), nonempty_sha(&fresh.head_sha)) {
        (Some(evaluated), Some(current)) if evaluated == current => None,
        (Some(evaluated), Some(current)) => Some(format!(
            "PR head moved between evaluation and merge (evaluated {evaluated}, now {current})"
        )),
        _ => Some("PR head SHA unavailable at evaluation or refetch".to_string()),
    }
}

/// Normalise a head SHA to `Some(trimmed_non_empty)` or `None`.
///
/// Gitea omits `head.sha` in rare payload shapes and the fake can carry an
/// empty override; both must read as "unavailable" so the pre-merge check
/// fails closed uniformly.
fn nonempty_sha(sha: &Option<String>) -> Option<&str> {
    sha.as_deref().map(str::trim).filter(|s| !s.is_empty())
}

/// Merge a PR per its `PrEvaluation` and close any `Fixes #N` issues.
///
/// Failure-1: if merge succeeds but any close fails, returns
/// `PartialFailure` so the caller can emit CRITICAL + exit 2.
/// Failure-2: nothing is closed if the merge itself fails.
pub async fn merge_and_close<T: GiteaOperations>(
    gitea: &T,
    owner: &str,
    repo: &str,
    eval: &PrEvaluation,
) -> Result<MergeOutcome, MergeCoordinatorError> {
    match &eval.verdict {
        EvalVerdict::Hold(reason) => {
            info!(pr = eval.pr_index, reason, "skipping PR (Hold)");
            return Ok(MergeOutcome::Skipped(reason.clone()));
        }
        EvalVerdict::Conflicting => {
            warn!(
                pr = eval.pr_index,
                "conflicting subagent verdicts; not merging"
            );
            return Ok(MergeOutcome::Skipped("conflicting verdicts".into()));
        }
        EvalVerdict::Merge => {}
    }

    // #3295 design §D6 — fresh-state precondition. Refetch the PR
    // immediately before merging and require exactly the state that was
    // evaluated: `state=open`, `mergeable=Some(true)`, and the same
    // non-empty head SHA on both sides. Anything else suppresses ALL side
    // effects (no merge, no `Fixes #N` issue close). A refetch error
    // propagates so the run fails closed instead of merging on stale data.
    if let Some(reason) = stale_state_reason(gitea.get_pr(owner, repo, eval.pr_index).await?, eval)
    {
        warn!(
            pr = eval.pr_index,
            reason, "suppressing merge (stale state)"
        );
        return Ok(MergeOutcome::Skipped(reason));
    }

    gitea.merge_pr(owner, repo, eval.pr_index).await?;
    info!(pr = eval.pr_index, "merged");

    let mut closed = Vec::new();
    let mut close_errors = Vec::new();
    for &issue in &eval.fixes_issues {
        match gitea.close_issue(owner, repo, issue).await {
            Ok(()) => {
                info!(pr = eval.pr_index, issue, "closed referenced issue");
                closed.push(issue);
            }
            Err(e) => {
                error!(pr = eval.pr_index, issue, error = %e, "close issue failed after merge");
                close_errors.push(issue);
            }
        }
    }

    if close_errors.is_empty() {
        Ok(MergeOutcome::Merged {
            closed_issues: closed,
        })
    } else {
        Ok(MergeOutcome::PartialFailure {
            merged: true,
            close_errors,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pr(number: u64, body: &str, mergeable: bool) -> PrSummary {
        PrSummary {
            number,
            title: format!("PR {number}"),
            body: Some(body.into()),
            state: "open".into(),
            mergeable: Some(mergeable),
            head_sha: None,
        }
    }

    #[tokio::test]
    async fn evaluate_one_holds_when_not_mergeable() {
        let p = pr(1, "Fixes #2", false);
        let e = evaluate_one::<crate::gitea::GiteaClient>(None, "o", "r", &p).await;
        assert!(matches!(e.verdict, EvalVerdict::Hold(_)));
        assert_eq!(e.fixes_issues, vec![2]);
        assert_eq!(e.blocker_kind, Some(BlockerKind::CiNoStatus));
    }

    #[tokio::test]
    async fn evaluate_one_merge_with_fixes() {
        // Both "Fixes #42" and "Closes #43" are now recognised closing keywords.
        let p = pr(7, "Fixes #42 Closes #43", true);
        let e = evaluate_one::<crate::gitea::GiteaClient>(None, "o", "r", &p).await;
        assert_eq!(e.verdict, EvalVerdict::Merge);
        assert_eq!(e.fixes_issues, vec![42, 43]);
        assert_eq!(e.blocker_kind, None);
    }

    #[tokio::test]
    async fn evaluate_one_merge_no_fixes_still_merges() {
        let p = pr(9, "feat: refactor", true);
        let e = evaluate_one::<crate::gitea::GiteaClient>(None, "o", "r", &p).await;
        assert_eq!(e.verdict, EvalVerdict::Merge);
        assert!(e.fixes_issues.is_empty());
        assert_eq!(e.blocker_kind, None);
    }

    #[tokio::test]
    async fn evaluate_one_handles_missing_body() {
        let p = PrSummary {
            number: 11,
            title: "x".into(),
            body: None,
            state: "open".into(),
            mergeable: Some(true),
            head_sha: None,
        };
        let e = evaluate_one::<crate::gitea::GiteaClient>(None, "o", "r", &p).await;
        assert_eq!(e.verdict, EvalVerdict::Merge);
        assert!(e.fixes_issues.is_empty());
        assert_eq!(e.blocker_kind, None);
    }

    #[tokio::test]
    async fn evaluate_one_processes_51_prs_without_truncation() {
        let prs: Vec<PrSummary> = (1u64..=51)
            .map(|n| pr(n, &format!("Fixes #{n}"), true))
            .collect();
        let mut evaluations = Vec::with_capacity(prs.len());
        for p in &prs {
            evaluations.push(evaluate_one::<crate::gitea::GiteaClient>(None, "o", "r", p).await);
        }
        assert_eq!(
            evaluations.len(),
            51,
            "all 51 PRs must receive an evaluation verdict"
        );
        let last = &evaluations[50];
        assert_eq!(last.pr_index, 51);
        assert_eq!(last.verdict, EvalVerdict::Merge);
        assert_eq!(last.blocker_kind, None);
    }

    #[test]
    fn contamination_patterns_match_artefact_files() {
        let patterns: &[&str] = &[".sessions/", ".review_tmp/", ".handoff/", ".beads/"];

        // Helper: matches as directory component (starts_with or contains "/.sessions/")
        let is_contaminated = |file: &str| -> bool {
            patterns
                .iter()
                .any(|p| file.starts_with(p) || file.contains(&["/", p].concat()))
        };

        // Positive matches — files inside contaminated directories
        assert!(is_contaminated(".sessions/session-123.md"));
        assert!(is_contaminated("subdir/.sessions/session-123.md"));
        assert!(is_contaminated(".review_tmp/pr123/file.diff"));
        assert!(is_contaminated(".handoff/pr2664-review.md"));
        assert!(is_contaminated(".beads/issues.jsonl"));
        assert!(is_contaminated("crates/foo/.beads/issues.jsonl"));

        // Negative matches — files NOT in contaminated directories
        assert!(!is_contaminated("src/main.rs"));
        assert!(!is_contaminated("crates/terraphim_rlm/src/lib.rs"));
        assert!(!is_contaminated("Cargo.toml"));
        assert!(!is_contaminated(".github/workflows/ci-pr.yml"));
        // False positive prevention: filename containing pattern string but not as directory
        assert!(!is_contaminated("src/sessions_parser.rs"));
        assert!(!is_contaminated("docs/review_tmp_guide.md"));
        assert!(!is_contaminated("tests/handoff_integration_test.rs"));
    }

    // ---- evaluate_all / merge_and_close via FakeGiteaClient (issue #2892) ----
    //
    // These tests are the regression guards the issue asks for: the whole
    // evaluation + merge path is now exercisable in-process without a live
    // Gitea server. They use the concrete `FakeGiteaClient` (not a mock).

    use crate::gitea::CommitCombinedStatus;
    use crate::gitea::test_support::FakeGiteaClient;

    fn mergeable_pr(number: u64, body: &str) -> PrSummary {
        PrSummary {
            number,
            title: format!("PR {number}"),
            body: Some(body.into()),
            state: "open".into(),
            mergeable: Some(true),
            // A concrete head SHA: the §D6 pre-merge refetch requires an
            // exact non-empty match, so merge-path tests must carry one.
            head_sha: Some(format!("deadbeef{number:04}")),
        }
    }

    #[tokio::test]
    async fn evaluate_all_persists_head_sha_in_evaluation() {
        // #3295 §D6: the evaluated head SHA must survive into the
        // evaluation so the pre-merge refetch can compare against it.
        let fake = FakeGiteaClient {
            open_prs: vec![mergeable_pr(77, "Fixes #770")],
            ..FakeGiteaClient::new()
        };
        let evals = evaluate_all(&fake, "owner", "repo").await.unwrap();
        assert_eq!(evals.len(), 1);
        assert_eq!(evals[0].head_sha.as_deref(), Some("deadbeef0077"));
    }

    #[test]
    fn stale_state_reason_covers_every_suppression_branch() {
        // Pure-function coverage of the §D6 decision table. Each row is a
        // distinct suppression (or the single pass-through) case.
        let eval = |sha: Option<&str>| PrEvaluation {
            pr_index: 1,
            mergeable: true,
            head_sha: sha.map(str::to_string),
            fixes_issues: vec![],
            verdict: EvalVerdict::Merge,
            blocker_kind: None,
        };
        let fresh = |state: &str, mergeable: Option<bool>, sha: Option<&str>| PrSummary {
            number: 1,
            title: "t".into(),
            body: None,
            state: state.into(),
            mergeable,
            head_sha: sha.map(str::to_string),
        };

        // Exact match on every axis -> proceed.
        assert_eq!(
            stale_state_reason(fresh("open", Some(true), Some("aaa")), &eval(Some("aaa"))),
            None
        );

        // State drift (closed covers "already merged" in Gitea's model).
        let r = stale_state_reason(fresh("closed", None, None), &eval(Some("aaa"))).unwrap();
        assert!(r.contains("closed"), "{r}");

        // Mergeability lost.
        let r = stale_state_reason(fresh("open", Some(false), Some("aaa")), &eval(Some("aaa")))
            .unwrap();
        assert!(r.contains("no longer mergeable"), "{r}");
        let r = stale_state_reason(fresh("open", None, Some("aaa")), &eval(Some("aaa"))).unwrap();
        assert!(r.contains("no longer mergeable"), "{r}");

        // Head drift.
        let r =
            stale_state_reason(fresh("open", Some(true), Some("bbb")), &eval(Some("aaa"))).unwrap();
        assert!(r.contains("head moved"), "{r}");

        // Missing SHA on either side.
        let r = stale_state_reason(fresh("open", Some(true), None), &eval(Some("aaa"))).unwrap();
        assert!(r.contains("head SHA unavailable"), "{r}");
        let r = stale_state_reason(fresh("open", Some(true), Some("aaa")), &eval(None)).unwrap();
        assert!(r.contains("head SHA unavailable"), "{r}");
        // Whitespace-only counts as unavailable.
        let r =
            stale_state_reason(fresh("open", Some(true), Some("   ")), &eval(Some("aaa"))).unwrap();
        assert!(r.contains("head SHA unavailable"), "{r}");
    }

    #[tokio::test]
    async fn evaluate_all_returns_empty_when_no_prs() {
        let fake = FakeGiteaClient::new();
        let evals = evaluate_all(&fake, "owner", "repo").await.unwrap();
        assert!(evals.is_empty(), "no open PRs -> no evaluations");
    }

    #[tokio::test]
    async fn evaluate_all_failopen_on_list_pr_files_error_still_verdicts_all_prs() {
        // AC: "evaluate_all with all PRs returning list_pr_files error → all
        // verdicts still computed (fail-open)". A list_pr_files error must
        // NOT abort the run; each PR still gets a verdict.
        let fake = FakeGiteaClient {
            open_prs: vec![
                mergeable_pr(10, "Fixes #100"),
                mergeable_pr(11, "Fixes #101"),
                mergeable_pr(12, "Fixes #102"),
            ],
            list_pr_files_error: Some(MergeCoordinatorError::api("simulated Gitea 500")),
            ..FakeGiteaClient::new()
        };
        let evals = evaluate_all(&fake, "owner", "repo").await.unwrap();
        assert_eq!(
            evals.len(),
            3,
            "all 3 PRs must receive a verdict despite list_pr_files errors"
        );
        // Every PR was returned, proving the run did not short-circuit.
        let indexes: Vec<u64> = evals.iter().map(|e| e.pr_index).collect();
        assert_eq!(indexes, vec![10, 11, 12]);
    }

    #[tokio::test]
    async fn evaluate_all_holds_contaminated_pr_and_merges_clean_prs() {
        // A contaminated PR is held; a clean, mergeable PR is merged. This
        // proves the contamination gate runs through the trait abstraction.
        let mut pr_files = std::collections::HashMap::new();
        pr_files.insert(20, vec!["src/lib.rs".to_string()]);
        pr_files.insert(21, vec![".sessions/session-1.md".to_string()]);
        let fake = FakeGiteaClient {
            open_prs: vec![
                mergeable_pr(20, "Fixes #200"),
                mergeable_pr(21, "Fixes #201"),
            ],
            pr_files,
            ..FakeGiteaClient::new()
        };
        let evals = evaluate_all(&fake, "owner", "repo").await.unwrap();
        assert_eq!(evals.len(), 2);
        assert_eq!(evals[0].pr_index, 20);
        assert_eq!(evals[0].verdict, EvalVerdict::Merge);
        assert_eq!(evals[1].pr_index, 21);
        assert!(
            matches!(evals[1].verdict, EvalVerdict::Hold(ref r) if r.contains("contaminated")),
            "contaminated PR must be held, got {:?}",
            evals[1].verdict
        );
    }

    #[tokio::test]
    async fn merge_and_close_merges_and_closes_referenced_issues() {
        // Full happy path through the generic API: merge + close the two
        // `Fixes #N` issues. Verifies merge_pr / close_issue call counts.
        let fake = FakeGiteaClient {
            open_prs: vec![mergeable_pr(30, "Fixes #300\nFixes #301")],
            pr_files: std::collections::HashMap::from([(30, vec!["src/a.rs".to_string()])]),
            ..FakeGiteaClient::new()
        };
        let evals = evaluate_all(&fake, "owner", "repo").await.unwrap();
        assert_eq!(evals[0].fixes_issues, vec![300, 301]);
        let outcome = merge_and_close(&fake, "owner", "repo", &evals[0])
            .await
            .unwrap();
        match outcome {
            MergeOutcome::Merged { closed_issues } => {
                assert_eq!(closed_issues, vec![300, 301]);
            }
            other => panic!("expected Merged, got {other:?}"),
        }
        assert_eq!(*fake.merge_calls.lock().unwrap(), 1, "exactly one merge");
        assert_eq!(
            *fake.close_calls.lock().unwrap(),
            2,
            "one close per referenced issue"
        );
        assert_eq!(
            *fake.merged_indexes.lock().unwrap(),
            vec![30],
            "merged the evaluated PR"
        );
        assert_eq!(
            *fake.closed_indexes.lock().unwrap(),
            vec![300, 301],
            "closed the referenced issues in order"
        );
    }

    #[tokio::test]
    async fn merge_and_close_reports_partial_failure_when_close_errors() {
        // Failure-1 from the merge_and_close contract: merge succeeds, one
        // close fails -> PartialFailure. Proves the error path is reachable
        // via the trait abstraction.
        let fake = FakeGiteaClient {
            open_prs: vec![mergeable_pr(40, "Fixes #400\nFixes #401")],
            pr_files: std::collections::HashMap::from([(40, vec!["src/b.rs".to_string()])]),
            close_issue_error_for: Some(401),
            ..FakeGiteaClient::new()
        };
        let evals = evaluate_all(&fake, "owner", "repo").await.unwrap();
        let outcome = merge_and_close(&fake, "owner", "repo", &evals[0])
            .await
            .unwrap();
        match outcome {
            MergeOutcome::PartialFailure {
                merged,
                close_errors,
            } => {
                assert!(merged, "merge did succeed");
                assert_eq!(close_errors, vec![401]);
            }
            other => panic!("expected PartialFailure, got {other:?}"),
        }
        assert_eq!(*fake.merge_calls.lock().unwrap(), 1);
        assert_eq!(*fake.close_calls.lock().unwrap(), 2);
    }

    #[tokio::test]
    async fn merge_and_close_skips_held_prs_without_merging() {
        // A held (contaminated) PR must be Skipped, and merge_pr must NOT be
        // called. Proves the Hold short-circuit survives the trait refactor.
        let mut pr_files = std::collections::HashMap::new();
        pr_files.insert(50, vec![".handoff/pr50.md".to_string()]);
        let fake = FakeGiteaClient {
            open_prs: vec![mergeable_pr(50, "Fixes #500")],
            pr_files,
            ..FakeGiteaClient::new()
        };
        let evals = evaluate_all(&fake, "owner", "repo").await.unwrap();
        let outcome = merge_and_close(&fake, "owner", "repo", &evals[0])
            .await
            .unwrap();
        assert!(
            matches!(outcome, MergeOutcome::Skipped(ref r) if r.contains("contaminated")),
            "expected Skipped(contaminated), got {outcome:?}"
        );
        assert_eq!(
            *fake.merge_calls.lock().unwrap(),
            0,
            "held PR must not be merged"
        );
        assert_eq!(
            *fake.close_calls.lock().unwrap(),
            0,
            "held PR must not close issues"
        );
    }

    #[tokio::test]
    async fn evaluate_all_classifies_unmergeable_via_commit_status() {
        // An unmergeable PR with a failing CI status is classified CiFailed.
        // Exercises classify_blocker -> get_commit_status through the trait.
        let unmergeable = PrSummary {
            number: 60,
            title: "x".into(),
            body: Some("Fixes #600".into()),
            state: "open".into(),
            mergeable: Some(false),
            head_sha: Some("deadbeef".into()),
        };
        let mut commit_status = std::collections::HashMap::new();
        commit_status.insert(
            "deadbeef".to_string(),
            CommitCombinedStatus {
                state: "failure".into(),
                statuses: Vec::new(),
            },
        );
        let fake = FakeGiteaClient {
            open_prs: vec![unmergeable],
            pr_files: std::collections::HashMap::from([(60, vec!["src/c.rs".to_string()])]),
            commit_status,
            ..FakeGiteaClient::new()
        };
        let evals = evaluate_all(&fake, "owner", "repo").await.unwrap();
        assert_eq!(evals[0].blocker_kind, Some(BlockerKind::CiFailed));
        assert!(matches!(evals[0].verdict, EvalVerdict::Hold(_)));
    }
}
