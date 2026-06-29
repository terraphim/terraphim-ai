//! Cancellation property test for the compound review swarm
//! (epic #1567, Layer 1, Gitea issue #1569).
//!
//! Property under test:
//!
//! > Cancelling `CompoundReviewWorkflow::run` at an arbitrary `.await`
//! > point (here via `JoinHandle::abort`) must leave **zero** worktree
//! > directories on disk and **zero** agent subprocesses alive within
//! > a bounded time (2 s).
//!
//! This is the acceptance criterion that the bigbox storm violated.
//! The test uses real git, real subprocesses, and a real worktree --
//! no mocks. The agent subprocess is `/bin/sleep` so it does not
//! self-exit during the test window.
//!
//! Compiled only when the `test-helpers` feature is enabled (so the
//! `scope::test_support` shared fixture is visible) and on Unix
//! (the test uses `/bin/sleep` and `/proc/<pid>` for PID liveness).

#![cfg(all(unix, feature = "test-helpers"))]

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use tempfile::TempDir;
use tokio::time::sleep;

use terraphim_orchestrator::scope::{test_support::setup_git_repo, WORKTREE_REVIEW_PREFIX};
use terraphim_orchestrator::{CompoundReviewWorkflow, ReviewGroupDef, SwarmConfig};
use terraphim_types::FindingCategory;

/// Static prompt content for the test group. The struct requires a
/// `&'static str`, and we do not exercise prompt parsing here.
const TEST_PROMPT: &str = "ignored-prompt";

/// Write a tiny shell wrapper that ignores all args and execs
/// `/bin/sleep 999`, returning the absolute path. We need this
/// because the `run_single_agent` dispatcher appends `prompt` and
/// each changed-file path as positional args; `/bin/sleep` itself
/// rejects non-numeric args.
fn write_sleep_wrapper(dir: &Path) -> PathBuf {
    let script = dir.join("fake-agent.sh");
    std::fs::write(
        &script,
        "#!/bin/sh\n# Ignore all args; sleep long enough for the test.\nexec /bin/sleep 999\n",
    )
    .expect("write wrapper script");
    use std::os::unix::fs::PermissionsExt;
    let mut perms = std::fs::metadata(&script).unwrap().permissions();
    perms.set_mode(0o755);
    std::fs::set_permissions(&script, perms).expect("chmod wrapper");
    script
}

/// Build a SwarmConfig with a single long-sleep review group.
fn make_swarm_config(repo_path: PathBuf, worktree_root: PathBuf, cli_tool: &Path) -> SwarmConfig {
    let group = ReviewGroupDef {
        agent_name: "sleep-agent".to_string(),
        category: FindingCategory::Quality,
        llm_tier: "Quick".to_string(),
        cli_tool: cli_tool.to_string_lossy().into_owned(),
        model: None,
        prompt_template: "test".to_string(),
        prompt_content: TEST_PROMPT,
        visual_only: false,
        persona: None,
    };

    SwarmConfig {
        groups: vec![group],
        // Set the per-agent timeout very high so the subprocess does
        // not exit on its own during the test window. Cancellation
        // is exercised by aborting the outer JoinHandle below.
        timeout: Duration::from_secs(600),
        worktree_root,
        repo_path,
        base_branch: "HEAD".to_string(),
        max_concurrent_agents: 1,
        create_prs: false,
    }
}

/// Poll a closure until it returns true or the deadline elapses.
async fn poll_until<F>(deadline: Duration, mut check: F) -> bool
where
    F: FnMut() -> bool,
{
    let start = Instant::now();
    while start.elapsed() < deadline {
        if check() {
            return true;
        }
        sleep(Duration::from_millis(50)).await;
    }
    check()
}

/// Collect PIDs whose `/proc/<pid>/cwd` symlink resolves into the
/// given worktree subtree. Called BEFORE abort so the cwd is still
/// the live path (not `<path> (deleted)` after teardown).
fn collect_pids_with_cwd_under(prefix: &Path) -> Vec<u32> {
    let mut pids = Vec::new();
    let proc = match std::fs::read_dir("/proc") {
        Ok(p) => p,
        Err(_) => return pids,
    };
    for entry in proc.flatten() {
        let name = entry.file_name();
        let name_str = match name.to_str() {
            Some(s) => s,
            None => continue,
        };
        let pid: u32 = match name_str.parse() {
            Ok(n) => n,
            Err(_) => continue,
        };
        let cwd_link = entry.path().join("cwd");
        if let Ok(target) = std::fs::read_link(&cwd_link) {
            if target.starts_with(prefix) {
                pids.push(pid);
            }
        }
    }
    pids
}

/// Return true if `/proc/<pid>` no longer exists (process gone).
fn pid_is_gone(pid: u32) -> bool {
    !PathBuf::from(format!("/proc/{}", pid)).exists()
}

/// Return the path of the first `review-*` directory under `base`,
/// or None if none exists.
fn first_review_dir(base: &Path) -> Option<PathBuf> {
    let entries = std::fs::read_dir(base).ok()?;
    for entry in entries.flatten() {
        let name = entry.file_name();
        if let Some(s) = name.to_str() {
            if s.starts_with(WORKTREE_REVIEW_PREFIX) {
                return Some(entry.path());
            }
        }
    }
    None
}

/// Return true if any `review-*` directory exists under `base`.
fn any_review_dir_exists(base: &Path) -> bool {
    let Ok(entries) = std::fs::read_dir(base) else {
        return false;
    };
    for entry in entries.flatten() {
        let name = entry.file_name();
        if let Some(s) = name.to_str() {
            if s.starts_with(WORKTREE_REVIEW_PREFIX) && entry.path().is_dir() {
                return true;
            }
        }
    }
    false
}

/// Return true if any `.git/worktrees/review-*` admin entry exists.
fn any_review_admin_entry(repo: &Path) -> bool {
    let admin_root = repo.join(".git").join("worktrees");
    let Ok(entries) = std::fs::read_dir(&admin_root) else {
        return false;
    };
    for entry in entries.flatten() {
        let name = entry.file_name();
        if let Some(s) = name.to_str() {
            if s.starts_with(WORKTREE_REVIEW_PREFIX) {
                return true;
            }
        }
    }
    false
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_cancellation_leaves_no_worktree() {
    // 1. Real git repo with a single commit.
    let (_repo_tempdir, repo_path) = setup_git_repo();

    // 2. Worktree base under a separate tempdir so we can scan it.
    let wt_tempdir = TempDir::new().expect("worktree tempdir");
    let worktree_root = wt_tempdir.path().to_path_buf();
    let script_tempdir = TempDir::new().expect("script tempdir");
    let cli_tool = write_sleep_wrapper(script_tempdir.path());

    let swarm = make_swarm_config(repo_path.clone(), worktree_root.clone(), &cli_tool);
    let workflow = CompoundReviewWorkflow::new(swarm);

    // 3. Spawn `workflow.run("HEAD", "HEAD")` so changed_files is
    //    empty (no diff against itself). The fake cli_tool wrapper
    //    ignores any args and execs `/bin/sleep 999`, so the
    //    subprocess is guaranteed to be live when we abort.
    let handle = tokio::spawn(async move { workflow.run("HEAD", "HEAD").await });

    // 4. Wait up to 5 s for the worktree to be created.
    let appeared = poll_until(Duration::from_secs(5), || {
        first_review_dir(&worktree_root).is_some()
    })
    .await;
    assert!(
        appeared,
        "worktree under {} never appeared within 5 s",
        worktree_root.display()
    );

    let review_dir = first_review_dir(&worktree_root).unwrap();
    println!("worktree created at {}", review_dir.display());

    // 5. Poll until at least one subprocess has its `cwd` inside the
    //    worktree. This validates the test's premise: the fake-agent
    //    wrapper actually got far enough to spawn `/bin/sleep`. We
    //    snapshot PIDs BEFORE abort so the readlink resolves the
    //    live path (after teardown, /proc/<pid>/cwd renders as
    //    "<path> (deleted)" which would defeat path matching).
    let mut subprocess_pids: Vec<u32> = Vec::new();
    let subprocess_live = poll_until(Duration::from_secs(5), || {
        subprocess_pids = collect_pids_with_cwd_under(&review_dir);
        !subprocess_pids.is_empty()
    })
    .await;
    assert!(
        subprocess_live,
        "no agent subprocess ever spawned with cwd under {}",
        review_dir.display()
    );
    println!("captured agent PIDs before abort: {:?}", subprocess_pids);

    // 6. Abort the outer task. Locals in `run` drop in reverse
    //    declaration order: tasks (JoinSet) drops first -> agent
    //    tasks aborted -> Child kill-on-drop fires -> subprocesses
    //    die. THEN the guard drops -> `git worktree remove --force`.
    handle.abort();
    // Await so the runtime processes the abort. Result is ignored;
    // it may be Cancelled or Err.
    let _ = handle.await;

    // 7. Within 2 s assert every captured PID is gone. This is the
    //    discriminating "no zombie agents" check: if `kill_on_drop`
    //    were missing, the JoinSet abort would drop the Child handle
    //    without signalling the OS process, and the sleep would
    //    still be live here.
    let pids_for_assert = subprocess_pids.clone();
    let no_zombie = poll_until(Duration::from_secs(2), || {
        pids_for_assert.iter().all(|p| pid_is_gone(*p))
    })
    .await;
    let still_alive: Vec<u32> = pids_for_assert
        .iter()
        .copied()
        .filter(|p| !pid_is_gone(*p))
        .collect();
    assert!(
        no_zombie,
        "agent subprocess(es) survived cancellation: {:?}",
        still_alive
    );

    // 8. Within 2 s assert no `review-*` dir remains under the base.
    let dir_gone = poll_until(Duration::from_secs(2), || {
        !any_review_dir_exists(&worktree_root)
    })
    .await;
    assert!(
        dir_gone,
        "worktree directory under {} survived cancellation: dir_exists={}, list={:?}",
        worktree_root.display(),
        any_review_dir_exists(&worktree_root),
        std::fs::read_dir(&worktree_root)
            .map(|d| d
                .flatten()
                .map(|e| e.file_name().to_string_lossy().to_string())
                .collect::<Vec<_>>())
            .unwrap_or_default()
    );

    // 9. Assert no `.git/worktrees/review-*` admin entry remains.
    let admin_gone = poll_until(Duration::from_secs(2), || {
        !any_review_admin_entry(&repo_path)
    })
    .await;
    assert!(
        admin_gone,
        "git admin entry under {}/.git/worktrees survived",
        repo_path.display()
    );
}

/// Storm-property variant: the Layer 0 cursor bug allowed
/// `check_cron_schedules` to fire the compound review repeatedly when
/// the reconcile tick cancellation kept the cursor unadvanced. The
/// property under Layer 1 is: even if the schedule re-fires before
/// the previous run finishes, every cancelled run leaves zero
/// worktrees on disk. We simulate the storm by spawning two
/// `run("HEAD", "HEAD")` calls back-to-back and aborting both.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_storm_cancellation_leaves_no_worktree() {
    let (_repo_tempdir, repo_path) = setup_git_repo();
    let wt_tempdir = TempDir::new().expect("worktree tempdir");
    let worktree_root = wt_tempdir.path().to_path_buf();
    let script_tempdir = TempDir::new().expect("script tempdir");
    let cli_tool = write_sleep_wrapper(script_tempdir.path());

    let swarm_a = make_swarm_config(repo_path.clone(), worktree_root.clone(), &cli_tool);
    let workflow_a = CompoundReviewWorkflow::new(swarm_a);
    let handle_a = tokio::spawn(async move { workflow_a.run("HEAD", "HEAD").await });

    // Wait for the first worktree dir, then trigger a second run
    // before the first finishes.
    let first_appeared = poll_until(Duration::from_secs(5), || {
        first_review_dir(&worktree_root).is_some()
    })
    .await;
    assert!(first_appeared, "first worktree did not appear");

    let swarm_b = make_swarm_config(repo_path.clone(), worktree_root.clone(), &cli_tool);
    let workflow_b = CompoundReviewWorkflow::new(swarm_b);
    let handle_b = tokio::spawn(async move { workflow_b.run("HEAD", "HEAD").await });

    // Brief yield so the second run advances past `create_worktree`
    // (which collides on the same name only if the UUIDs happened
    // to match; in practice they do not, so this is a second
    // distinct worktree).
    sleep(Duration::from_millis(200)).await;

    handle_a.abort();
    handle_b.abort();
    let _ = handle_a.await;
    let _ = handle_b.await;

    // Within 5 s assert no `review-*` dir remains. Storm-shaped
    // cancellation must drain to zero.
    let dir_gone = poll_until(Duration::from_secs(5), || {
        !any_review_dir_exists(&worktree_root)
    })
    .await;
    assert!(
        dir_gone,
        "storm-cancelled runs left worktree(s) on disk under {}: {:?}",
        worktree_root.display(),
        std::fs::read_dir(&worktree_root)
            .map(|d| d
                .flatten()
                .map(|e| e.file_name().to_string_lossy().to_string())
                .collect::<Vec<_>>())
            .unwrap_or_default()
    );

    let admin_gone = poll_until(Duration::from_secs(5), || {
        !any_review_admin_entry(&repo_path)
    })
    .await;
    assert!(admin_gone, "git admin entries survived storm cancellation");
}
