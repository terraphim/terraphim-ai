//! PR evaluation + merge-and-close orchestration (#1805).
//!
//! PRs are evaluated strictly sequentially within a run (no concurrency).
//! Partial failure is surfaced as CRITICAL: a merge that succeeds but whose
//! follow-up close call fails is not silently retried. Remediation
//! (comment + exit) is applied atomically per PR.

use tracing::{error, info, warn};

use crate::extract_fixes;
use crate::gitea::{GiteaClient, PrSummary};
use crate::types::{BlockerKind, EvalVerdict, MergeCoordinatorError, MergeOutcome};

/// One evaluation of one open PR.
#[derive(Debug, Clone)]
pub struct PrEvaluation {
    /// Gitea PR index (number).
    pub pr_index: u64,
    /// Whether the PR is currently mergeable according to Gitea.
    pub mergeable: bool,
    /// Issue numbers referenced by `Fixes #N` in the PR body.
    pub fixes_issues: Vec<u64>,
    /// Verdict reached during evaluation.
    pub verdict: EvalVerdict,
    /// Classified blocker kind when verdict is Hold (None for Merge).
    pub blocker_kind: Option<BlockerKind>,
}

/// Evaluate all open PRs in `owner/repo`, sequentially. Each PR gets
/// a verdict; no merges are performed here.
pub async fn evaluate_all(
    gitea: &GiteaClient,
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
async fn evaluate_one(
    gitea: Option<&GiteaClient>,
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
        fixes_issues,
        verdict,
        blocker_kind,
    }
}

/// Check PR file list for contamination (artefacts, session dumps, etc.).
///
/// Returns `Ok(())` if clean, `Err(reason)` if contaminated.
async fn check_contamination(
    gitea: &GiteaClient,
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
            if file.starts_with(pattern) || file.contains(pattern) {
                return Err(format!("contaminated: {file} (pattern: {pattern})"));
            }
        }
    }

    Ok(())
}

/// Query CI status and classify why a PR is blocked.
async fn classify_blocker(
    gitea: Option<&GiteaClient>,
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

/// Merge a PR per its `PrEvaluation` and close any `Fixes #N` issues.
///
/// Failure-1: if merge succeeds but any close fails, returns
/// `PartialFailure` so the caller can emit CRITICAL + exit 2.
/// Failure-2: nothing is closed if the merge itself fails.
pub async fn merge_and_close(
    gitea: &GiteaClient,
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
        let e = evaluate_one(None, "o", "r", &p).await;
        assert!(matches!(e.verdict, EvalVerdict::Hold(_)));
        assert_eq!(e.fixes_issues, vec![2]);
        assert_eq!(e.blocker_kind, Some(BlockerKind::CiNoStatus));
    }

    #[tokio::test]
    async fn evaluate_one_merge_with_fixes() {
        // Both "Fixes #42" and "Closes #43" are now recognised closing keywords.
        let p = pr(7, "Fixes #42 Closes #43", true);
        let e = evaluate_one(None, "o", "r", &p).await;
        assert_eq!(e.verdict, EvalVerdict::Merge);
        assert_eq!(e.fixes_issues, vec![42, 43]);
        assert_eq!(e.blocker_kind, None);
    }

    #[tokio::test]
    async fn evaluate_one_merge_no_fixes_still_merges() {
        let p = pr(9, "feat: refactor", true);
        let e = evaluate_one(None, "o", "r", &p).await;
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
        let e = evaluate_one(None, "o", "r", &p).await;
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
            evaluations.push(evaluate_one(None, "o", "r", p).await);
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

        // Positive matches
        assert!(
            patterns
                .iter()
                .any(|p| ".sessions/session-123.md".contains(p))
        );
        assert!(
            patterns
                .iter()
                .any(|p| ".review_tmp/pr123/file.diff".contains(p))
        );
        assert!(
            patterns
                .iter()
                .any(|p| ".handoff/pr2664-review.md".contains(p))
        );
        assert!(patterns.iter().any(|p| ".beads/issues.jsonl".contains(p)));

        // Negative matches
        assert!(!patterns.iter().any(|p| "src/main.rs".contains(p)));
        assert!(
            !patterns
                .iter()
                .any(|p| "crates/terraphim_rlm/src/lib.rs".contains(p))
        );
        assert!(!patterns.iter().any(|p| "Cargo.toml".contains(p)));
        assert!(
            !patterns
                .iter()
                .any(|p| ".github/workflows/ci-pr.yml".contains(p))
        );
    }
}
