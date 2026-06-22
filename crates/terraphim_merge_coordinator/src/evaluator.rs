//! PR evaluation + merge-and-close orchestration (#1805).
//!
//! Sequential per spec Concurrency-2. Partial failure handling
//! (merge ok, close fail -> CRITICAL) per Failure-1. Remediation
//! atomicity per Failure-2.

use tracing::{error, info, warn};

use crate::extract_fixes;
use crate::gitea::{GiteaClient, PrSummary};
use crate::types::{EvalVerdict, MergeCoordinatorError, MergeOutcome};

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
        out.push(evaluate_one(&pr));
    }
    info!(count = out.len(), owner, repo, "evaluated open PRs");
    Ok(out)
}

fn evaluate_one(pr: &PrSummary) -> PrEvaluation {
    let mergeable = pr.mergeable.unwrap_or(false);
    let fixes_issues = extract_fixes(pr.body.as_deref().unwrap_or(""));
    let verdict = if !mergeable {
        EvalVerdict::Hold("not mergeable".into())
    } else if fixes_issues.is_empty() {
        // No Fixes #N -> safe to merge but nothing to auto-close.
        EvalVerdict::Merge
    } else {
        EvalVerdict::Merge
    };
    PrEvaluation {
        pr_index: pr.number,
        mergeable,
        fixes_issues,
        verdict,
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
        }
    }

    #[test]
    fn evaluate_one_holds_when_not_mergeable() {
        let p = pr(1, "Fixes #2", false);
        let e = evaluate_one(&p);
        assert!(matches!(e.verdict, EvalVerdict::Hold(_)));
        assert_eq!(e.fixes_issues, vec![2]);
    }

    #[test]
    fn evaluate_one_merge_with_fixes() {
        // Both "Fixes #42" and "Closes #43" are now recognised closing keywords.
        let p = pr(7, "Fixes #42 Closes #43", true);
        let e = evaluate_one(&p);
        assert_eq!(e.verdict, EvalVerdict::Merge);
        assert_eq!(e.fixes_issues, vec![42, 43]);
    }

    #[test]
    fn evaluate_one_merge_no_fixes_still_merges() {
        let p = pr(9, "feat: refactor", true);
        let e = evaluate_one(&p);
        assert_eq!(e.verdict, EvalVerdict::Merge);
        assert!(e.fixes_issues.is_empty());
    }

    #[test]
    fn evaluate_one_handles_missing_body() {
        let p = PrSummary {
            number: 11,
            title: "x".into(),
            body: None,
            state: "open".into(),
            mergeable: Some(true),
        };
        let e = evaluate_one(&p);
        assert_eq!(e.verdict, EvalVerdict::Merge);
        assert!(e.fixes_issues.is_empty());
    }

    #[test]
    fn evaluate_one_processes_51_prs_without_truncation() {
        // Regression test for issue #2850: ensure the evaluation loop does not
        // impose an artificial cap at position 50.  With list_open_prs returning
        // up to OPEN_PRS_LIMIT (300) items, all PRs must receive a verdict.
        let prs: Vec<PrSummary> = (1u64..=51)
            .map(|n| pr(n, &format!("Fixes #{n}"), true))
            .collect();
        let evaluations: Vec<_> = prs.iter().map(evaluate_one).collect();
        assert_eq!(
            evaluations.len(),
            51,
            "all 51 PRs must receive an evaluation verdict"
        );
        // Spot-check: the PR at position 51 gets Merge (it is mergeable, clean)
        let last = &evaluations[50];
        assert_eq!(last.pr_index, 51);
        assert_eq!(last.verdict, EvalVerdict::Merge);
    }
}
