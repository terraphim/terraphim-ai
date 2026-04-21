//! ROC v1 Step H — post-merge test gate with git revert on red.
//!
//! When a PR is auto-merged, the orchestrator enqueues a
//! [`crate::dispatcher::DispatchTask::PostMergeTestGate`]. The handler runs
//! `cargo test --workspace` at the merge commit; on red it reverts the
//! merge commit via `git revert --no-edit` and pushes the revert back to
//! `main` so CI is restored without a human in the loop. On green it logs
//! the verification and emits a placeholder Quickwit event (Step I will
//! replace the placeholder with a typed event enum).
//!
//! The module is split into:
//! - [`CommandRunner`] trait abstracting subprocess execution so tests can
//!   inject a fake runner without actually spawning `cargo test`.
//! - [`TokioCommandRunner`] the real tokio-based implementation.
//! - Pure functions `run_workspace_tests`, `classify_failure`, `revert_merge`.
//!
//! See `cto-executive-system/plans/adf-rate-of-change-design.md` §Step H and
//! Gitea issue `terraphim/adf-fleet#36`.

use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use async_trait::async_trait;
use tokio::io::{AsyncBufReadExt, AsyncRead, BufReader};

/// Default maximum wall time for the workspace test run (10 min).
pub const DEFAULT_MAX_TEST_DURATION_SECS: u64 = 600;

/// Maximum number of lines retained from stdout/stderr for reporting.
pub const TAIL_LINE_CAP: usize = 500;

/// Subprocess result — exit status plus captured output tails.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandOutput {
    /// Process exit code. `None` when the process was signalled.
    pub exit_code: Option<i32>,
    /// `true` when the child exited with status 0.
    pub success: bool,
    /// Tail of stdout (last [`TAIL_LINE_CAP`] lines).
    pub stdout_tail: String,
    /// Tail of stderr (last [`TAIL_LINE_CAP`] lines).
    pub stderr_tail: String,
}

/// Errors produced by a [`CommandRunner`].
#[derive(Debug, thiserror::Error)]
pub enum CommandError {
    /// The command exceeded its wall-time budget and was killed.
    #[error("command timed out after {0:?}")]
    Timeout(Duration),
    /// The command could not be spawned or the OS returned an error.
    #[error("command io error: {0}")]
    Io(String),
}

/// Abstracts subprocess execution so the post-merge gate can be driven by
/// an in-memory fake runner without spawning real processes.
#[async_trait]
pub trait CommandRunner: Send + Sync {
    /// Run `cmd args` in `cwd` with a wall-time cap of `timeout`.
    ///
    /// Implementations MUST kill the child process when the timeout
    /// elapses and return [`CommandError::Timeout`]. They MUST return
    /// [`CommandOutput`] even when the child exits with a non-zero
    /// status — that is how callers detect test failures.
    async fn run(
        &self,
        cmd: &str,
        args: &[&str],
        cwd: &Path,
        timeout: Duration,
    ) -> Result<CommandOutput, CommandError>;
}

/// Outcome of a workspace test run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestRunOutcome {
    /// `true` when `cargo test` exited with status 0.
    pub passed: bool,
    /// Raw exit code from the child, if any.
    pub exit_code: Option<i32>,
    /// Tail of stdout from the run.
    pub stdout_tail: String,
    /// Tail of stderr from the run.
    pub stderr_tail: String,
    /// Wall time consumed by the run.
    pub wall_time: Duration,
    /// `true` when the run was killed for exceeding its wall-time budget.
    pub timed_out: bool,
}

/// Configuration for a single gate run.
#[derive(Debug, Clone)]
pub struct GateConfig {
    /// Path to the git repository the tests should run against.
    pub repo_root: PathBuf,
    /// Merge commit SHA under test. Carried through for log attribution
    /// and passed to [`revert_merge`] on red.
    pub merge_sha: String,
    /// Maximum wall time for `cargo test --workspace`.
    pub max_test_duration: Duration,
    /// Git remote to push the revert to. `None` skips the push (used by
    /// tests where no remote is configured).
    pub revert_push_remote: Option<String>,
    /// Branch to push the revert to (default "main").
    pub revert_push_branch: String,
}

impl GateConfig {
    /// Construct a new gate config with production defaults for push
    /// target (`origin main`) and the default test timeout.
    pub fn new(repo_root: PathBuf, merge_sha: String) -> Self {
        Self {
            repo_root,
            merge_sha,
            max_test_duration: Duration::from_secs(DEFAULT_MAX_TEST_DURATION_SECS),
            revert_push_remote: Some("origin".to_string()),
            revert_push_branch: "main".to_string(),
        }
    }
}

/// Outcome of a revert attempt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RevertOutcome {
    /// SHA of the newly created revert commit.
    pub revert_sha: String,
    /// `true` when the revert was successfully pushed to the configured
    /// remote/branch.
    pub pushed: bool,
}

/// High-level classification of why a workspace test run failed. Used to
/// decorate the `[ADF]` issue body and log lines without parsing the
/// output downstream again.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FailureKind {
    /// Tests ran and at least one reported FAILED.
    TestFailure,
    /// The harness itself did not start — e.g. `cargo test` exited
    /// non-zero without a failure count (build error, cargo not found).
    HarnessError,
    /// The run exceeded its wall-time budget and was killed.
    Timeout,
    /// Unclassified non-zero exit.
    Unknown,
}

/// Parsed failure summary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FailureClassification {
    /// Category of failure.
    pub kind: FailureKind,
    /// Names of the failing tests, if any were parsed from the output.
    pub failing_tests: Vec<String>,
}

/// Errors produced while running or reverting on the gate.
#[derive(Debug, thiserror::Error)]
pub enum GateError {
    #[error("command error: {0}")]
    Command(#[from] CommandError),
    #[error("revert failed: {0}")]
    Revert(String),
}

/// Run `cargo test --workspace --no-fail-fast` at the repo root.
///
/// Returns a [`TestRunOutcome`] describing exit code, captured tails, and
/// wall time. When the wall-time budget is exceeded the outcome is marked
/// `timed_out = true` and `passed = false` — callers treat that as a
/// red gate.
pub async fn run_workspace_tests<R: CommandRunner + ?Sized>(
    runner: &R,
    cfg: &GateConfig,
) -> Result<TestRunOutcome, GateError> {
    let started = std::time::Instant::now();
    let result = runner
        .run(
            "cargo",
            &["test", "--workspace", "--no-fail-fast"],
            &cfg.repo_root,
            cfg.max_test_duration,
        )
        .await;
    let wall_time = started.elapsed();

    match result {
        Ok(out) => Ok(TestRunOutcome {
            passed: out.success,
            exit_code: out.exit_code,
            stdout_tail: out.stdout_tail,
            stderr_tail: out.stderr_tail,
            wall_time,
            timed_out: false,
        }),
        Err(CommandError::Timeout(_)) => Ok(TestRunOutcome {
            passed: false,
            exit_code: None,
            stdout_tail: String::new(),
            stderr_tail: format!(
                "post_merge_gate: cargo test exceeded {:?}; child killed",
                cfg.max_test_duration
            ),
            wall_time,
            timed_out: true,
        }),
        Err(e) => Err(GateError::Command(e)),
    }
}

/// Classify a test run outcome into a [`FailureClassification`].
pub fn classify_failure(outcome: &TestRunOutcome) -> FailureClassification {
    if outcome.timed_out {
        return FailureClassification {
            kind: FailureKind::Timeout,
            failing_tests: Vec::new(),
        };
    }
    if outcome.passed {
        return FailureClassification {
            kind: FailureKind::Unknown,
            failing_tests: Vec::new(),
        };
    }

    let combined = format!("{}\n{}", outcome.stdout_tail, outcome.stderr_tail);
    let failing = parse_failing_tests(&combined);

    let looks_like_harness = combined.contains("error: could not compile")
        || combined.contains("error: no such subcommand")
        || combined.contains("error: failed to compile")
        || combined.contains("error: Command")
        || combined.contains("error[E");

    let kind = if !failing.is_empty() {
        FailureKind::TestFailure
    } else if looks_like_harness {
        FailureKind::HarnessError
    } else {
        FailureKind::Unknown
    };

    FailureClassification {
        kind,
        failing_tests: failing,
    }
}

/// Parse failing test names from cargo test output.
///
/// cargo test prints a `failures:` section listing each failing test path
/// one per line (e.g. `    post_merge_gate::tests::foo`). We grab those
/// lines between `failures:` and `test result:` and return them.
fn parse_failing_tests(output: &str) -> Vec<String> {
    let mut failing = Vec::new();
    let mut in_failures_block = false;
    for line in output.lines() {
        let trimmed = line.trim();
        if trimmed == "failures:" {
            in_failures_block = true;
            continue;
        }
        if in_failures_block {
            if trimmed.starts_with("test result:") {
                in_failures_block = false;
                continue;
            }
            if trimmed.is_empty() {
                // Blank lines inside the block are padding; keep reading.
                continue;
            }
            // cargo test indents each failing test name with 4 spaces.
            let name = trimmed.to_string();
            if !name.starts_with("----") && !failing.contains(&name) {
                failing.push(name);
            }
        }
    }
    failing
}

/// Execute `git revert --no-edit <merge_sha>` at `repo_root`, capture the
/// new HEAD SHA, and optionally push it back to the configured remote.
///
/// The revert is attempted exactly once — no retries. Callers escalate
/// via an `[ADF]` issue when the push fails or the revert cannot be
/// produced.
pub async fn revert_merge<R: CommandRunner + ?Sized>(
    runner: &R,
    cfg: &GateConfig,
) -> Result<RevertOutcome, GateError> {
    let short_timeout = Duration::from_secs(60);

    let revert_out = runner
        .run(
            "git",
            &["revert", "--no-edit", "-m", "1", cfg.merge_sha.as_str()],
            &cfg.repo_root,
            short_timeout,
        )
        .await?;
    if !revert_out.success {
        // Try plain revert (non-merge commit path) as a fallback.
        let plain = runner
            .run(
                "git",
                &["revert", "--no-edit", cfg.merge_sha.as_str()],
                &cfg.repo_root,
                short_timeout,
            )
            .await?;
        if !plain.success {
            return Err(GateError::Revert(format!(
                "git revert exited with code {:?}: {}",
                plain.exit_code.or(revert_out.exit_code),
                plain.stderr_tail
            )));
        }
    }

    let rev_parse = runner
        .run("git", &["rev-parse", "HEAD"], &cfg.repo_root, short_timeout)
        .await?;
    if !rev_parse.success {
        return Err(GateError::Revert(format!(
            "git rev-parse HEAD failed: {}",
            rev_parse.stderr_tail
        )));
    }
    let revert_sha = rev_parse.stdout_tail.trim().to_string();

    let pushed = match &cfg.revert_push_remote {
        Some(remote) => {
            let refspec = format!("HEAD:{}", cfg.revert_push_branch);
            let push_out = runner
                .run(
                    "git",
                    &["push", remote.as_str(), refspec.as_str()],
                    &cfg.repo_root,
                    short_timeout,
                )
                .await?;
            push_out.success
        }
        None => false,
    };

    Ok(RevertOutcome { revert_sha, pushed })
}

/// Real tokio-backed [`CommandRunner`]. Streams stdout/stderr into a
/// bounded ring buffer so very long test runs do not blow memory.
pub struct TokioCommandRunner;

#[async_trait]
impl CommandRunner for TokioCommandRunner {
    async fn run(
        &self,
        cmd: &str,
        args: &[&str],
        cwd: &Path,
        timeout: Duration,
    ) -> Result<CommandOutput, CommandError> {
        let mut child = tokio::process::Command::new(cmd)
            .args(args)
            .current_dir(cwd)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| CommandError::Io(format!("failed to spawn {cmd}: {e}")))?;

        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| CommandError::Io("child stdout pipe missing".to_string()))?;
        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| CommandError::Io("child stderr pipe missing".to_string()))?;

        let stdout_task = tokio::spawn(tail_stream(stdout, TAIL_LINE_CAP));
        let stderr_task = tokio::spawn(tail_stream(stderr, TAIL_LINE_CAP));

        let wait_fut = child.wait();
        let status_res = tokio::time::timeout(timeout, wait_fut).await;

        match status_res {
            Ok(Ok(status)) => {
                let stdout_tail = stdout_task.await.unwrap_or_default();
                let stderr_tail = stderr_task.await.unwrap_or_default();
                Ok(CommandOutput {
                    exit_code: status.code(),
                    success: status.success(),
                    stdout_tail,
                    stderr_tail,
                })
            }
            Ok(Err(e)) => Err(CommandError::Io(format!("child wait failed: {e}"))),
            Err(_) => {
                let _ = child.start_kill();
                // Best-effort cleanup: wait briefly so the reader tasks can drain.
                let _ = child.wait().await;
                let _ = stdout_task.await;
                let _ = stderr_task.await;
                Err(CommandError::Timeout(timeout))
            }
        }
    }
}

async fn tail_stream<R: AsyncRead + Unpin>(reader: R, max_lines: usize) -> String {
    let mut lines = BufReader::new(reader).lines();
    let mut ring: VecDeque<String> = VecDeque::with_capacity(max_lines + 1);
    while let Ok(Some(line)) = lines.next_line().await {
        if ring.len() >= max_lines {
            ring.pop_front();
        }
        ring.push_back(line);
    }
    ring.into_iter().collect::<Vec<_>>().join("\n")
}

/// Record of a single [`CommandRunner::run`] call, used by test impls to
/// assert the handler invoked the expected commands in the expected order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CallRecord {
    pub cmd: String,
    pub args: Vec<String>,
    pub cwd: PathBuf,
}

/// Shared programmable runner used by unit tests. Each queued response is
/// returned in FIFO order; calls beyond the queued responses return a
/// default successful [`CommandOutput`].
#[derive(Clone, Default)]
pub struct ScriptedRunner {
    responses: Arc<Mutex<VecDeque<Result<CommandOutput, CommandError>>>>,
    calls: Arc<Mutex<Vec<CallRecord>>>,
}

impl ScriptedRunner {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push_ok(&self, code: i32, stdout: &str, stderr: &str) {
        self.responses.lock().unwrap().push_back(Ok(CommandOutput {
            exit_code: Some(code),
            success: code == 0,
            stdout_tail: stdout.to_string(),
            stderr_tail: stderr.to_string(),
        }));
    }

    pub fn push_err(&self, err: CommandError) {
        self.responses.lock().unwrap().push_back(Err(err));
    }

    pub fn calls(&self) -> Vec<CallRecord> {
        self.calls.lock().unwrap().clone()
    }
}

#[async_trait]
impl CommandRunner for ScriptedRunner {
    async fn run(
        &self,
        cmd: &str,
        args: &[&str],
        cwd: &Path,
        _timeout: Duration,
    ) -> Result<CommandOutput, CommandError> {
        self.calls.lock().unwrap().push(CallRecord {
            cmd: cmd.to_string(),
            args: args.iter().map(|s| s.to_string()).collect(),
            cwd: cwd.to_path_buf(),
        });
        let mut queue = self.responses.lock().unwrap();
        queue.pop_front().unwrap_or_else(|| {
            Ok(CommandOutput {
                exit_code: Some(0),
                success: true,
                stdout_tail: String::new(),
                stderr_tail: String::new(),
            })
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base_cfg() -> GateConfig {
        let mut cfg = GateConfig::new(PathBuf::from("/tmp/fake"), "deadbeef".to_string());
        cfg.revert_push_remote = None;
        cfg
    }

    #[tokio::test]
    async fn run_workspace_tests_reports_green_when_exit_zero() {
        let runner = ScriptedRunner::new();
        runner.push_ok(0, "", "");
        let cfg = base_cfg();
        let out = run_workspace_tests(&runner, &cfg).await.unwrap();
        assert!(out.passed);
        assert!(!out.timed_out);
        assert_eq!(out.exit_code, Some(0));

        let calls = runner.calls();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].cmd, "cargo");
        assert_eq!(calls[0].args, vec!["test", "--workspace", "--no-fail-fast"]);
    }

    #[tokio::test]
    async fn run_workspace_tests_marks_timeout() {
        let runner = ScriptedRunner::new();
        runner.push_err(CommandError::Timeout(Duration::from_secs(600)));
        let out = run_workspace_tests(&runner, &base_cfg()).await.unwrap();
        assert!(out.timed_out);
        assert!(!out.passed);
    }

    #[tokio::test]
    async fn run_workspace_tests_propagates_io_error() {
        let runner = ScriptedRunner::new();
        runner.push_err(CommandError::Io("no such file".to_string()));
        let res = run_workspace_tests(&runner, &base_cfg()).await;
        assert!(matches!(res, Err(GateError::Command(_))));
    }

    #[test]
    fn classify_failure_parses_test_failures() {
        let outcome = TestRunOutcome {
            passed: false,
            exit_code: Some(101),
            stdout_tail: "\
running 3 tests
test foo ... ok
test bar ... FAILED

failures:

    mod::bar
    mod::baz

test result: FAILED. 1 passed; 2 failed
"
            .to_string(),
            stderr_tail: String::new(),
            wall_time: Duration::from_secs(1),
            timed_out: false,
        };
        let c = classify_failure(&outcome);
        assert_eq!(c.kind, FailureKind::TestFailure);
        assert_eq!(c.failing_tests, vec!["mod::bar", "mod::baz"]);
    }

    #[test]
    fn classify_failure_detects_harness_error() {
        let outcome = TestRunOutcome {
            passed: false,
            exit_code: Some(101),
            stdout_tail: String::new(),
            stderr_tail: "error: could not compile `foo` due to previous error".to_string(),
            wall_time: Duration::from_secs(1),
            timed_out: false,
        };
        let c = classify_failure(&outcome);
        assert_eq!(c.kind, FailureKind::HarnessError);
        assert!(c.failing_tests.is_empty());
    }

    #[test]
    fn classify_failure_detects_timeout() {
        let outcome = TestRunOutcome {
            passed: false,
            exit_code: None,
            stdout_tail: String::new(),
            stderr_tail: String::new(),
            wall_time: Duration::from_secs(600),
            timed_out: true,
        };
        let c = classify_failure(&outcome);
        assert_eq!(c.kind, FailureKind::Timeout);
    }

    #[tokio::test]
    async fn revert_merge_captures_new_sha_no_push() {
        let runner = ScriptedRunner::new();
        // git revert -m 1
        runner.push_ok(0, "", "");
        // git rev-parse HEAD
        runner.push_ok(0, "abc123def456\n", "");
        let out = revert_merge(&runner, &base_cfg()).await.unwrap();
        assert_eq!(out.revert_sha, "abc123def456");
        assert!(!out.pushed);

        let calls = runner.calls();
        assert_eq!(calls.len(), 2);
        assert_eq!(calls[0].args[0], "revert");
        assert_eq!(calls[1].args[0], "rev-parse");
    }

    #[tokio::test]
    async fn revert_merge_pushes_when_remote_configured() {
        let runner = ScriptedRunner::new();
        runner.push_ok(0, "", ""); // revert
        runner.push_ok(0, "cafef00d\n", ""); // rev-parse
        runner.push_ok(0, "", ""); // push

        let mut cfg = base_cfg();
        cfg.revert_push_remote = Some("origin".to_string());
        cfg.revert_push_branch = "main".to_string();
        let out = revert_merge(&runner, &cfg).await.unwrap();
        assert_eq!(out.revert_sha, "cafef00d");
        assert!(out.pushed);

        let calls = runner.calls();
        assert_eq!(calls.len(), 3);
        assert_eq!(calls[2].cmd, "git");
        assert_eq!(calls[2].args, vec!["push", "origin", "HEAD:main"]);
    }

    #[tokio::test]
    async fn revert_merge_fails_when_all_revert_paths_fail() {
        let runner = ScriptedRunner::new();
        // first revert attempt (with -m 1) fails
        runner.push_ok(1, "", "fatal: not a merge");
        // fallback revert (without -m) also fails
        runner.push_ok(1, "", "fatal: commit not found");
        let res = revert_merge(&runner, &base_cfg()).await;
        assert!(matches!(res, Err(GateError::Revert(_))));
    }
}
