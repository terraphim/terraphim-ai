//! merge-coordinator binary entry (#1805).
//!
//! Cron-invoked. Reads env GITEA_URL + GITEA_TOKEN, evaluates open
//! PRs in OWNER/REPO (default terraphim/terraphim-ai), merges
//! mergeable ones, auto-closes Fixes #N. Exit codes per
//! Operational-1: 0 success, 1 evaluation failures, 2 critical.

use std::path::PathBuf;
use std::process;

use serde_json::json;

use terraphim_merge_coordinator::{
    evaluator::{evaluate_all, merge_and_close},
    gitea::GiteaClient,
    jsonlog::emit,
    pid_lock::acquire_pid_lock,
    types::{ExitCode, MergeCoordinatorError, MergeOutcome},
};

const DEFAULT_OWNER: &str = "terraphim";
const DEFAULT_REPO: &str = "terraphim-ai";
const LOCK_PATH: &str = "/tmp/merge-coordinator.lock";
const LOCK_STALE_SECS: u64 = 30;

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let exit = run().await;
    emit("run.complete", &[("exit_code", json!(exit as i32))]);
    process::exit(exit as i32);
}

async fn run() -> ExitCode {
    let owner = std::env::var("MERGE_COORDINATOR_OWNER").unwrap_or_else(|_| DEFAULT_OWNER.into());
    let repo = std::env::var("MERGE_COORDINATOR_REPO").unwrap_or_else(|_| DEFAULT_REPO.into());
    let base_url = match std::env::var("GITEA_URL") {
        Ok(v) => v,
        Err(_) => {
            emit("config.error", &[("missing", json!("GITEA_URL"))]);
            return ExitCode::Critical;
        }
    };
    let token = match std::env::var("GITEA_TOKEN") {
        Ok(v) => v,
        Err(_) => {
            emit("config.error", &[("missing", json!("GITEA_TOKEN"))]);
            return ExitCode::Critical;
        }
    };

    let _guard = match acquire_pid_lock(&PathBuf::from(LOCK_PATH), LOCK_STALE_SECS) {
        Ok(g) => g,
        Err(MergeCoordinatorError::LockHeld { pid, age_secs }) => {
            emit(
                "lock.held",
                &[("holder_pid", json!(pid)), ("age_secs", json!(age_secs))],
            );
            return ExitCode::Critical;
        }
        Err(e) => {
            emit("lock.error", &[("error", json!(e.to_string()))]);
            return ExitCode::Critical;
        }
    };
    emit(
        "run.start",
        &[("owner", json!(owner)), ("repo", json!(repo))],
    );

    let gitea = GiteaClient::new(base_url, token);

    let evaluations = match evaluate_all(&gitea, &owner, &repo).await {
        Ok(v) => v,
        Err(e) => {
            emit("evaluation.error", &[("error", json!(e.to_string()))]);
            return ExitCode::Critical;
        }
    };

    let mut had_eval_failures = false;
    let mut had_critical = false;

    for eval in &evaluations {
        emit(
            "pr.evaluated",
            &[
                ("pr_index", json!(eval.pr_index)),
                ("mergeable", json!(eval.mergeable)),
                ("fixes", json!(eval.fixes_issues)),
                ("verdict", json!(eval.verdict.to_string())),
            ],
        );
        match merge_and_close(&gitea, &owner, &repo, eval).await {
            Ok(MergeOutcome::Merged { closed_issues }) => emit(
                "pr.merged",
                &[
                    ("pr_index", json!(eval.pr_index)),
                    ("closed_issues", json!(closed_issues)),
                ],
            ),
            Ok(MergeOutcome::PartialFailure {
                merged,
                close_errors,
            }) => {
                emit(
                    "pr.partial_failure",
                    &[
                        ("pr_index", json!(eval.pr_index)),
                        ("merged", json!(merged)),
                        ("close_errors", json!(close_errors)),
                    ],
                );
                // Per Failure-1: partial failure -> CRITICAL.
                had_critical = true;
                // Stop further evaluation per spec.
                break;
            }
            Ok(MergeOutcome::Skipped(reason)) => emit(
                "pr.skipped",
                &[
                    ("pr_index", json!(eval.pr_index)),
                    ("reason", json!(reason)),
                ],
            ),
            Err(e) => {
                emit(
                    "pr.evaluation_error",
                    &[
                        ("pr_index", json!(eval.pr_index)),
                        ("error", json!(e.to_string())),
                    ],
                );
                had_eval_failures = true;
            }
        }
    }

    if had_critical {
        ExitCode::Critical
    } else if had_eval_failures {
        ExitCode::EvaluationFailures
    } else {
        ExitCode::Success
    }
}
