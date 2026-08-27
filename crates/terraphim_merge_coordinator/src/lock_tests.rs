//! RED tests for D5 (lock scoping) + D6 (fresh-state precondition).
//!
//! All numbered M1..M7 per the design's test plan; this file survives
//! the implementation as the regression suite.
//!
//! Compiled only under `#[cfg(test)]` — wired up from `lib.rs`.

use std::collections::HashMap;
use std::path::PathBuf;

use crate::evaluator::{evaluate_all, merge_and_close};
use crate::gitea::test_support::FakeGiteaClient;
use crate::gitea::{CommitCombinedStatus, PrSummary};
use crate::lock_path::{RunOutcome, exit_code_for_run_outcome, resolve_lock_path};

fn mergeable_pr(number: u64, body: &str, head_sha: Option<&str>) -> PrSummary {
    PrSummary {
        number,
        title: format!("PR {number}"),
        body: Some(body.into()),
        state: "open".into(),
        mergeable: Some(true),
        head_sha: head_sha.map(str::to_string),
    }
}

// ============================================================
//  D5 — lock scoping + no-steal + exit-code mapping (M5..M7)
// ============================================================

#[test]
fn resolve_lock_path_scopes_by_owner_repo() {
    // M5: (terraphim, terraphim-ai) != (other, repo); charset violations rejected.
    let a = resolve_lock_path(
        std::path::Path::new("/var/locks"),
        "terraphim",
        "terraphim-ai",
    )
    .expect("valid components");
    let b = resolve_lock_path(std::path::Path::new("/var/locks"), "other", "repo")
        .expect("valid components");
    assert_ne!(a, b, "per-repo keys must not collide");

    let key_a = a.file_name().expect("file name").to_str().unwrap();
    assert_eq!(
        key_a, "merge-coordinator-terraphim--terraphim-ai.lock",
        "exact filename per design §D5"
    );
}

#[test]
fn resolve_lock_path_rejects_charset_violations() {
    // M5: charset violations rejected.
    for bad_owner in ["../", "a/b", "café", ""] {
        let err = resolve_lock_path(std::path::Path::new("/var/locks"), bad_owner, "repo")
            .expect_err("invalid owner must be rejected");
        let _ = err;
    }
    for bad_repo in ["../", "a\\b", "terraphim/ai", ""] {
        let err = resolve_lock_path(std::path::Path::new("/var/locks"), "terraphim", bad_repo)
            .expect_err("invalid repo must be rejected");
        let _ = err;
    }
}

#[test]
fn resolve_lock_path_accepts_dotted_hyphen_underscore_names() {
    // Defensive: the cross-repo regex allows `[A-Za-z0-9._-]`; ensure that
    // dotted + hyphen + underscore names round-trip without aliasing.
    let p = resolve_lock_path(std::path::Path::new("/var/locks"), "my.org", "repo_name-v2")
        .expect("valid");
    let key = p.file_name().unwrap().to_str().unwrap();
    // The frozen §D5 separator is `--`; lock_path.rs asserts the exact
    // filename `merge-coordinator-my.org--repo_name-v2.lock` for the same
    // inputs, so match that contract here.
    assert!(key.contains("my.org--repo_name-v2"), "got {key:?}");
}

#[test]
fn exit_code_for_lock_held_maps_to_success() {
    // M7: LockHeld => ExitCode::Success (single-flight overlap is benign).
    use crate::types::ExitCode;
    let outcome: RunOutcome = terraphim_lockfile::LeaseError::LockHeld {
        holder_pid: Some(42),
    }
    .into();
    assert_eq!(exit_code_for_run_outcome(outcome), ExitCode::Success);
}

#[test]
fn exit_code_for_io_maps_to_critical() {
    // M7: Io => ExitCode::Critical (fail-closed).
    use crate::types::ExitCode;
    let outcome: RunOutcome = terraphim_lockfile::LeaseError::Io(std::io::Error::new(
        std::io::ErrorKind::PermissionDenied,
        "/var/locks not writable",
    ))
    .into();
    assert_eq!(exit_code_for_run_outcome(outcome), ExitCode::Critical);
}

#[test]
fn lock_contention_returns_lock_held_without_steal() {
    // M6: holder present, payload forged with a 100 s-old timestamp — acquire
    // still `LockHeld` (stealing removed; replaces the deleted steal test).
    use terraphim_lockfile::LeaseLock;
    let dir = tempfile::tempdir().unwrap();
    let path = dir
        .path()
        .join("merge-coordinator-terraphim--terraphim-ai.lock");

    // Simulate a foreign holder that wrote a stale payload.
    let stale = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
        .saturating_sub(100);
    std::fs::write(&path, format!("pid=999999 acquired={stale}")).unwrap();
    let foreign = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .truncate(false)
        .open(&path)
        .unwrap();
    fs4::fs_std::FileExt::try_lock_exclusive(&foreign)
        .expect("foreign try_lock_exclusive io")
        .then_some(())
        .expect("foreign lock must be acquired");
    let _foreign_keep_alive = foreign; // keep locked across the assertion

    // The acquire call must return LockHeld regardless of payload age
    // (the lease primitive carries no timestamp-steal logic — INV3).
    match LeaseLock::acquire(dir.path(), "merge-coordinator-terraphim--terraphim-ai") {
        Err(terraphim_lockfile::LeaseError::LockHeld { holder_pid }) => {
            assert_eq!(holder_pid, Some(999_999));
        }
        other => panic!("expected LockHeld, got {other:?}"),
    }
}

#[test]
fn lock_drop_releases_next_acquirer() {
    // Adjacent: RAII Drop releases the lease; no manual cleanup needed.
    use terraphim_lockfile::LeaseLock;
    let dir = tempfile::tempdir().unwrap();
    let _guard = LeaseLock::acquire(dir.path(), "merge-coordinator-terraphim--terraphim-ai")
        .expect("first acquire");
    drop(_guard);
    let _second = LeaseLock::acquire(dir.path(), "merge-coordinator-terraphim--terraphim-ai")
        .expect("drop must release");
}

#[test]
fn lock_path_helpers_paths_use_lock_dir() {
    // Defensive: ensure resolve_lock_path returns a path under the dir.
    let p = resolve_lock_path(&PathBuf::from("/opt/ai-dark-factory/data/locks"), "a", "b").unwrap();
    assert!(p.starts_with("/opt/ai-dark-factory/data/locks"));
}

// ============================================================
//  D5b — run-level wiring seam consumed by main.rs (#3295)
// ============================================================
//
// `acquire_run_lock` is the exact call `main` makes: project-scoped
// (owner,repo) lease from `resolve_lock_dir()`, held for the whole run,
// LockHeld => benign successful no-op, everything else fail-closed.

#[test]
fn acquire_run_lock_holds_project_scoped_lease_for_run_lifetime() {
    use crate::lock_path::acquire_run_lock;
    let dir = tempfile::tempdir().unwrap();

    let guard = acquire_run_lock(dir.path(), "terraphim", "terraphim-ai")
        .expect("uncontended project acquire must succeed");
    let expected = dir
        .path()
        .join("merge-coordinator-terraphim--terraphim-ai.lock");
    assert_eq!(guard.path(), expected.as_path(), "exact §D5 filename");
    let payload = std::fs::read_to_string(&expected).unwrap();
    assert!(
        payload.starts_with(&format!("pid={} acquired=", std::process::id())),
        "frozen payload contract; got {payload:?}"
    );

    // Held for the whole run: a second acquire of the SAME project contends.
    match acquire_run_lock(dir.path(), "terraphim", "terraphim-ai") {
        Err(RunOutcome::LockHeld { holder_pid }) => {
            assert_eq!(holder_pid, Some(std::process::id()));
        }
        other => panic!("expected LockHeld while the run guard is alive, got {other:?}"),
    }

    // Run over: guard drop releases; the next run may acquire.
    drop(guard);
    let _next = acquire_run_lock(dir.path(), "terraphim", "terraphim-ai")
        .expect("guard drop must release the lease");
}

#[test]
fn acquire_run_lock_never_steals_a_live_foreign_holder() {
    // Planted negative: a LIVE kernel holder with a forged 100 s-old payload.
    // Legacy pid_lock stole this; the project lease must return LockHeld,
    // leave the payload untouched, and map to benign Success.
    use crate::lock_path::acquire_run_lock;
    use crate::types::ExitCode;
    let dir = tempfile::tempdir().unwrap();
    let path = dir
        .path()
        .join("merge-coordinator-terraphim--terraphim-ai.lock");
    let stale = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
        .saturating_sub(100);
    std::fs::write(&path, format!("pid=999999 acquired={stale}")).unwrap();
    let foreign = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .truncate(false)
        .open(&path)
        .unwrap();
    fs4::fs_std::FileExt::try_lock_exclusive(&foreign)
        .expect("foreign try_lock_exclusive io")
        .then_some(())
        .expect("foreign lock must be acquired");

    let outcome = match acquire_run_lock(dir.path(), "terraphim", "terraphim-ai") {
        Err(o) => o,
        Ok(g) => panic!("live foreign holder must never be stolen, got {g:?}"),
    };
    assert_eq!(
        exit_code_for_run_outcome(outcome.clone()),
        ExitCode::Success,
        "contention is a benign successful no-op"
    );
    match outcome {
        RunOutcome::LockHeld { holder_pid } => assert_eq!(holder_pid, Some(999_999)),
        other => panic!("expected LockHeld, got {other:?}"),
    }
    let on_disk = std::fs::read_to_string(&path).unwrap();
    assert!(
        on_disk.starts_with("pid=999999"),
        "payload must not be rewritten while the holder lives; got {on_disk:?}"
    );

    drop(foreign); // kernel releases on fd close
    let _g = acquire_run_lock(dir.path(), "terraphim", "terraphim-ai")
        .expect("lease must be acquirable after the holder fd closes");
}

#[test]
fn acquire_run_lock_allows_different_projects_to_run_concurrently() {
    use crate::lock_path::acquire_run_lock;
    let dir = tempfile::tempdir().unwrap();
    let _a =
        acquire_run_lock(dir.path(), "terraphim", "terraphim-ai").expect("project A must acquire");
    let _b = acquire_run_lock(dir.path(), "other", "repo")
        .expect("project B must acquire concurrently (no global lock)");
    assert!(
        dir.path()
            .join("merge-coordinator-terraphim--terraphim-ai.lock")
            .exists()
    );
    assert!(
        dir.path()
            .join("merge-coordinator-other--repo.lock")
            .exists()
    );
}

#[test]
fn acquire_run_lock_fails_closed_on_lock_dir_errors() {
    use crate::lock_path::acquire_run_lock;
    use crate::types::ExitCode;
    let dir = tempfile::tempdir().unwrap();
    let not_a_dir = dir.path().join("occupied");
    std::fs::write(&not_a_dir, b"x").unwrap();

    let outcome = match acquire_run_lock(&not_a_dir, "terraphim", "terraphim-ai") {
        Err(o) => o,
        Ok(g) => panic!("lock-dir I/O failure must fail closed, got {g:?}"),
    };
    assert_eq!(
        exit_code_for_run_outcome(outcome.clone()),
        ExitCode::Critical
    );
    assert!(matches!(outcome, RunOutcome::Io(_)), "got {outcome:?}");
}

#[test]
fn acquire_run_lock_rejects_invalid_project_identity_fail_closed() {
    use crate::lock_path::acquire_run_lock;
    use crate::types::ExitCode;
    let dir = tempfile::tempdir().unwrap();
    for bad_owner in ["../evil", "a/b", "", "café"] {
        let outcome = match acquire_run_lock(dir.path(), bad_owner, "terraphim-ai") {
            Err(o) => o,
            Ok(g) => panic!("invalid owner {bad_owner:?} must fail closed, got {g:?}"),
        };
        assert_eq!(
            exit_code_for_run_outcome(outcome.clone()),
            ExitCode::Critical
        );
        assert!(
            matches!(outcome, RunOutcome::Io(_)),
            "owner {bad_owner:?}: {outcome:?}"
        );
    }
    for bad_repo in ["../evil", "a\\b", "", "café"] {
        let outcome = match acquire_run_lock(dir.path(), "terraphim", bad_repo) {
            Err(o) => o,
            Ok(g) => panic!("invalid repo {bad_repo:?} must fail closed, got {g:?}"),
        };
        assert_eq!(
            exit_code_for_run_outcome(outcome.clone()),
            ExitCode::Critical
        );
        assert!(
            matches!(outcome, RunOutcome::Io(_)),
            "repo {bad_repo:?}: {outcome:?}"
        );
    }
}

#[test]
fn resolve_lock_dir_default_matches_adf_contract() {
    // main.rs resolves the lock root exclusively via resolve_lock_dir();
    // pin the default so the /tmp global path can never silently return.
    assert_eq!(
        crate::lock_path::LOCK_DIR_DEFAULT,
        "/opt/ai-dark-factory/data/locks"
    );
}

// ============================================================
//  D6 — fresh-state precondition (M1..M4b)
// ============================================================

#[tokio::test]
async fn merge_and_close_skips_when_pr_closed_after_evaluation() {
    // M1 (FIRST RED for P2).
    let fake = FakeGiteaClient {
        open_prs: vec![mergeable_pr(7, "Fixes #70", Some("aaa111"))],
        ..FakeGiteaClient::new()
    };
    let evals = evaluate_all(&fake, "owner", "repo").await.unwrap();
    assert_eq!(evals.len(), 1);

    // After evaluation, Gitea's view of PR 7 changes: closed.
    fake.pr_states
        .lock()
        .unwrap()
        .insert(7, "closed".to_string());

    let outcome = merge_and_close(&fake, "owner", "repo", &evals[0])
        .await
        .unwrap();
    match outcome {
        crate::types::MergeOutcome::Skipped(s) => {
            assert!(s.contains("closed"), "skip reason: {s:?}");
        }
        other => panic!("expected Skipped, got {other:?}"),
    }
    assert_eq!(*fake.merge_calls.lock().unwrap(), 0);
    assert_eq!(
        *fake.close_calls.lock().unwrap(),
        0,
        "no issue close may happen when the merge is suppressed"
    );
}

#[tokio::test]
async fn merge_and_close_skips_when_pr_merged_after_evaluation() {
    // M1b: Gitea reports already-merged PRs as state "closed" — the same
    // fail-closed path must suppress every side effect. Another coordinator
    // (or a human) winning the race is the scenario this guards.
    let fake = FakeGiteaClient {
        open_prs: vec![mergeable_pr(14, "Fixes #140", Some("hhh888"))],
        ..FakeGiteaClient::new()
    };
    let evals = evaluate_all(&fake, "owner", "repo").await.unwrap();

    // Merged elsewhere between evaluation and the pre-merge refetch.
    fake.pr_states
        .lock()
        .unwrap()
        .insert(14, "closed".to_string());
    fake.pr_mergeables.lock().unwrap().insert(14, None);

    let outcome = merge_and_close(&fake, "owner", "repo", &evals[0])
        .await
        .unwrap();
    match outcome {
        crate::types::MergeOutcome::Skipped(s) => {
            assert!(s.contains("closed"), "skip reason: {s:?}");
        }
        other => panic!("expected Skipped, got {other:?}"),
    }
    assert_eq!(*fake.merge_calls.lock().unwrap(), 0);
    assert_eq!(
        *fake.close_calls.lock().unwrap(),
        0,
        "no issue close may happen when the merge is suppressed"
    );
}

#[tokio::test]
async fn merge_and_close_skips_on_head_sha_drift() {
    // M2.
    let fake = FakeGiteaClient {
        open_prs: vec![mergeable_pr(8, "Fixes #80", Some("aaa111"))],
        ..FakeGiteaClient::new()
    };
    let evals = evaluate_all(&fake, "owner", "repo").await.unwrap();

    fake.pr_head_shas
        .lock()
        .unwrap()
        .insert(8, "bbb222".to_string());

    let outcome = merge_and_close(&fake, "owner", "repo", &evals[0])
        .await
        .unwrap();
    match outcome {
        crate::types::MergeOutcome::Skipped(s) => {
            assert!(s.contains("head moved"), "skip reason: {s:?}");
        }
        other => panic!("expected Skipped, got {other:?}"),
    }
    assert_eq!(*fake.merge_calls.lock().unwrap(), 0);
    assert_eq!(
        *fake.close_calls.lock().unwrap(),
        0,
        "no issue close may happen when the merge is suppressed"
    );
}

#[tokio::test]
async fn merge_and_close_skips_when_head_sha_unavailable_evaluated_missing() {
    // M2b case A.
    let fake = FakeGiteaClient {
        open_prs: vec![mergeable_pr(9, "Fixes #90", None)],
        ..FakeGiteaClient::new()
    };
    let evals = evaluate_all(&fake, "owner", "repo").await.unwrap();

    let outcome = merge_and_close(&fake, "owner", "repo", &evals[0])
        .await
        .unwrap();
    match outcome {
        crate::types::MergeOutcome::Skipped(s) => {
            assert!(s.contains("head SHA unavailable"), "skip reason: {s:?}");
        }
        other => panic!("expected Skipped, got {other:?}"),
    }
    assert_eq!(*fake.merge_calls.lock().unwrap(), 0);
    assert_eq!(
        *fake.close_calls.lock().unwrap(),
        0,
        "no issue close may happen when the merge is suppressed"
    );
}

#[tokio::test]
async fn merge_and_close_skips_when_head_sha_unavailable_fresh_missing() {
    // M2b case B.
    let fake = FakeGiteaClient {
        open_prs: vec![mergeable_pr(10, "Fixes #100", Some("ddd444"))],
        ..FakeGiteaClient::new()
    };
    let evals = evaluate_all(&fake, "owner", "repo").await.unwrap();

    // Force the fresh SHA to be missing.
    fake.pr_head_shas.lock().unwrap().insert(10, String::new());

    let outcome = merge_and_close(&fake, "owner", "repo", &evals[0])
        .await
        .unwrap();
    match outcome {
        crate::types::MergeOutcome::Skipped(s) => {
            assert!(s.contains("head SHA unavailable"), "skip reason: {s:?}");
        }
        other => panic!("expected Skipped, got {other:?}"),
    }
    assert_eq!(*fake.merge_calls.lock().unwrap(), 0);
    assert_eq!(
        *fake.close_calls.lock().unwrap(),
        0,
        "no issue close may happen when the merge is suppressed"
    );
}

#[tokio::test]
async fn merge_and_close_merges_when_fresh_state_matches() {
    // M3 (control).
    let fake = FakeGiteaClient {
        open_prs: vec![mergeable_pr(11, "Fixes #110", Some("eee555"))],
        pr_files: HashMap::from([(11, vec!["src/lib.rs".to_string()])]),
        ..FakeGiteaClient::new()
    };
    let evals = evaluate_all(&fake, "owner", "repo").await.unwrap();

    let outcome = merge_and_close(&fake, "owner", "repo", &evals[0])
        .await
        .unwrap();
    match outcome {
        crate::types::MergeOutcome::Merged { closed_issues } => {
            assert_eq!(closed_issues, vec![110]);
        }
        other => panic!("expected Merged, got {other:?}"),
    }
    assert_eq!(*fake.merge_calls.lock().unwrap(), 1);
}

#[tokio::test]
async fn merge_and_close_skips_when_mergeability_lost() {
    // M4.
    let fake = FakeGiteaClient {
        open_prs: vec![mergeable_pr(12, "Fixes #120", Some("fff666"))],
        ..FakeGiteaClient::new()
    };
    let evals = evaluate_all(&fake, "owner", "repo").await.unwrap();

    fake.pr_mergeables.lock().unwrap().insert(12, Some(false));

    let outcome = merge_and_close(&fake, "owner", "repo", &evals[0])
        .await
        .unwrap();
    match outcome {
        crate::types::MergeOutcome::Skipped(s) => {
            assert!(s.contains("no longer mergeable"), "skip reason: {s:?}");
        }
        other => panic!("expected Skipped, got {other:?}"),
    }
    assert_eq!(*fake.merge_calls.lock().unwrap(), 0);
    assert_eq!(
        *fake.close_calls.lock().unwrap(),
        0,
        "no issue close may happen when the merge is suppressed"
    );
}

#[tokio::test]
async fn merge_and_close_fails_closed_on_get_pr_error() {
    // M4b.
    let fake = FakeGiteaClient {
        open_prs: vec![mergeable_pr(13, "Fixes #130", Some("ggg777"))],
        get_pr_error: Some(crate::types::MergeCoordinatorError::api(
            "simulated Gitea 500",
        )),
        ..FakeGiteaClient::new()
    };
    let evals = evaluate_all(&fake, "owner", "repo").await.unwrap();

    let err = merge_and_close(&fake, "owner", "repo", &evals[0])
        .await
        .expect_err("must propagate transport error");
    let _ = err;
    assert_eq!(*fake.merge_calls.lock().unwrap(), 0);
}

#[allow(dead_code)]
fn _lint_silencer() {
    let _ = CommitCombinedStatus {
        state: "success".into(),
        statuses: Vec::new(),
    };
    let _: PathBuf = PathBuf::from("/var/locks");
}
