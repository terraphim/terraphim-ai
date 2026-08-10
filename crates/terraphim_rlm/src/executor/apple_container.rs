//! Apple Container execution backend (macOS 26 / Apple silicon).
//!
//! This module provides the [`AppleContainerExecutor`], an [`ExecutionEnvironment`]
//! implementation driving Apple's `container` CLI. Apple's runtime starts **one
//! lightweight Linux VM per container** on top of `Virtualization.framework`, so
//! this backend gives macOS an isolation model closer to Firecracker than to
//! Docker Desktop's single shared Linux VM.
//!
//! ## Requirements
//!
//! ```bash
//! brew install container   # Apple silicon, macOS 26
//! container system start   # one-time, operator-owned
//! ```
//!
//! The backend never runs `container system start` itself: starting the service
//! is host administration (it may install a kernel or prompt the user), so
//! lifecycle ownership stays with the operator. Availability is established by
//! *positive* evidence only — see [`AppleContainerExecutor::probe`].
//!
//! ## Design notes
//!
//! - Every CLI call is an argument vector. No host shell is ever constructed,
//!   so guest code and commands cannot escape into host-side word splitting.
//! - One detached container per [`SessionId`], created exactly once under a
//!   per-session mutex (same shape as [`super::DockerExecutor`]).
//! - Timeouts fail **closed**: the CLI child is killed and reaped, the session
//!   container is force-removed, and the affinity mapping is cleared so the next
//!   call gets a fresh VM. A timeout must never leave a process running inside a
//!   container that a later call would reuse.
//! - Process execution goes through the [`ProcessRunner`] seam so the argv
//!   contract, concurrency, timeout and teardown behaviour can be pinned by
//!   portable tests on non-Apple hosts. A fake-runner pass is **not** evidence
//!   that the Apple runtime works; real macOS evidence is a separate gate.

use async_trait::async_trait;
use dashmap::DashMap;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use ulid::Ulid;

use super::{Capability, ExecutionContext, ExecutionResult, SnapshotId, ValidationResult};
use crate::config::{BackendType, RlmConfig};
use crate::error::{RlmError, RlmResult};
use crate::types::SessionId;

/// Canonical backend name used in errors and diagnostics.
pub const BACKEND_NAME: &str = "apple-container";

/// Default CLI binary name, resolved from `PATH` when not overridden.
const DEFAULT_BINARY: &str = "container";

/// Prefix for every container this backend creates. Used for operator recovery
/// (`container list --all` / `container delete <name>`).
const CONTAINER_NAME_PREFIX: &str = "terraphim-rlm-";

/// CPU allocation per session container.
const DEFAULT_CPUS: &str = "1";
/// Memory allocation per session container (mirrors the Docker profile).
const DEFAULT_MEMORY: &str = "512M";

/// Bound applied to lifecycle commands (`system version`, `run`, `stop`,
/// `delete`). Execution commands use `ExecutionContext::timeout_ms` instead.
const LIFECYCLE_TIMEOUT: Duration = Duration::from_secs(60);
/// Bound applied to the availability probe, which must stay cheap.
const PROBE_TIMEOUT: Duration = Duration::from_secs(10);
/// Grace period, in seconds, handed to `container stop` before deletion.
const STOP_GRACE_SECS: &str = "5";
/// How long to keep draining stdout/stderr after the CLI child has exited or
/// been killed.
///
/// The child's pipes can be inherited by a grandchild that outlives it (a killed
/// `container exec` does not necessarily take the guest process with it), so the
/// readers are not awaited unconditionally — otherwise a timed-out call would
/// block until the grandchild exits, which is exactly the hang the timeout
/// contract exists to prevent.
const DRAIN_GRACE: Duration = Duration::from_secs(2);

/// Captured result of one CLI invocation.
///
/// Crate-internal: this is a test seam, not part of the crate's public API.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CommandOutput {
    /// Process exit code, or `-1` when the process was killed/timed out.
    pub(crate) exit_code: i32,
    /// Captured stdout (lossy UTF-8).
    pub(crate) stdout: String,
    /// Captured stderr (lossy UTF-8).
    pub(crate) stderr: String,
    /// Whether the call hit its deadline and the child was killed.
    pub(crate) timed_out: bool,
}

impl CommandOutput {
    /// Whether the process exited successfully and within its deadline.
    pub(crate) fn is_success(&self) -> bool {
        self.exit_code == 0 && !self.timed_out
    }
}

/// Seam over host process execution.
///
/// Implemented by [`TokioProcessRunner`] in production and by fakes in tests, so
/// the argv contract and failure handling can be verified without Apple software.
///
/// Crate-internal: this is a test seam, not part of the crate's public API.
#[async_trait]
pub(crate) trait ProcessRunner: Send + Sync + std::fmt::Debug {
    /// Run `program` with `args` as an argument vector, bounded by `timeout`.
    ///
    /// On timeout the implementation MUST kill and reap the child and return
    /// whatever output was captured so far with `timed_out: true`.
    async fn run(
        &self,
        program: &Path,
        args: &[String],
        timeout: Duration,
    ) -> std::io::Result<CommandOutput>;
}

/// Production [`ProcessRunner`] backed by `tokio::process`.
#[derive(Debug, Default, Clone, Copy)]
pub(crate) struct TokioProcessRunner;

async fn drain<R>(mut reader: R, sink: Arc<std::sync::Mutex<Vec<u8>>>)
where
    R: tokio::io::AsyncRead + Unpin + Send + 'static,
{
    use tokio::io::AsyncReadExt;
    let mut buf = [0u8; 8192];
    loop {
        match reader.read(&mut buf).await {
            Ok(0) | Err(_) => break,
            Ok(n) => {
                if let Ok(mut guard) = sink.lock() {
                    guard.extend_from_slice(&buf[..n]);
                }
            }
        }
    }
}

fn take_string(sink: &Arc<std::sync::Mutex<Vec<u8>>>) -> String {
    sink.lock()
        .map(|g| String::from_utf8_lossy(&g).into_owned())
        .unwrap_or_default()
}

#[async_trait]
impl ProcessRunner for TokioProcessRunner {
    async fn run(
        &self,
        program: &Path,
        args: &[String],
        timeout: Duration,
    ) -> std::io::Result<CommandOutput> {
        let mut child = tokio::process::Command::new(program)
            .args(args)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            // Belt and braces: if this future is dropped (cancellation), the
            // child still dies rather than outliving the executor.
            .kill_on_drop(true)
            .spawn()?;

        let out_buf = Arc::new(std::sync::Mutex::new(Vec::new()));
        let err_buf = Arc::new(std::sync::Mutex::new(Vec::new()));
        let stdout_task = child
            .stdout
            .take()
            .map(|s| tokio::spawn(drain(s, out_buf.clone())));
        let stderr_task = child
            .stderr
            .take()
            .map(|s| tokio::spawn(drain(s, err_buf.clone())));

        let timed_out;
        let exit_code;
        match tokio::time::timeout(timeout, child.wait()).await {
            Ok(status) => {
                let status = status?;
                timed_out = false;
                exit_code = status.code().unwrap_or(-1);
            }
            Err(_) => {
                // Kill AND reap: `Child::kill` waits for the process so no
                // zombie is left behind.
                let _ = child.kill().await;
                timed_out = true;
                exit_code = -1;
            }
        }

        // Bounded drain: collect whatever is still buffered, then stop reading
        // rather than waiting on a pipe a surviving grandchild may hold open.
        for mut task in [stdout_task, stderr_task].into_iter().flatten() {
            if tokio::time::timeout(DRAIN_GRACE, &mut task).await.is_err() {
                task.abort();
            }
        }

        Ok(CommandOutput {
            exit_code,
            stdout: take_string(&out_buf),
            stderr: take_string(&err_buf),
            timed_out,
        })
    }
}

/// Returns true when this build targets a platform Apple's `container` supports.
///
/// Apple ships `container` for Apple silicon only, and supports macOS 26. This
/// check is deliberately compile-time: on any other platform the CLI is not
/// probed at all.
pub(crate) fn platform_supported() -> bool {
    cfg!(all(target_os = "macos", target_arch = "aarch64"))
}

/// Executes code in per-session Apple Container VMs via the `container` CLI.
pub struct AppleContainerExecutor {
    runner: Arc<dyn ProcessRunner>,
    binary: PathBuf,
    image: String,
    session_to_container: DashMap<SessionId, Arc<Mutex<Option<String>>>>,
    capabilities: Vec<Capability>,
    validator: Option<Arc<crate::validator::KnowledgeGraphValidator>>,
    platform_supported: bool,
}

fn unsupported(op: &'static str) -> RlmError {
    RlmError::NotSupported {
        backend: BACKEND_NAME.to_string(),
        op: op.to_string(),
    }
}

fn init_failed(message: impl Into<String>) -> RlmError {
    RlmError::BackendInitFailed {
        backend: BACKEND_NAME.to_string(),
        message: message.into(),
    }
}

/// Whether a failed `stop`/`delete` means the container is already gone.
///
/// Teardown is idempotent, so "no such container" is success; anything else is
/// surfaced to the caller with the backend and container name.
fn is_already_absent(output: &CommandOutput) -> bool {
    let text = format!("{} {}", output.stdout, output.stderr).to_lowercase();
    text.contains("not found")
        || text.contains("no such container")
        || text.contains("does not exist")
        || text.contains("notfound")
}

/// Whether a failed `container exec` means the *session container* is gone,
/// rather than the guest command having failed.
///
/// A bare substring match is unsafe here: guest failures routinely contain the
/// same words (`bash: frobnicate: command not found`, exit 127), and treating
/// those as a dead container would throw away a healthy session and re-run the
/// caller's command. So an absence phrase is only believed when the line also
/// looks like the CLI speaking about *our* container — an `Error:`-style prefix
/// or a mention of the generated container name.
fn exec_reports_container_missing(output: &CommandOutput, name: &str) -> bool {
    let name_lc = name.to_lowercase();
    output.stderr.lines().map(str::trim).any(|line| {
        let line_lc = line.to_lowercase();
        let absent = line_lc.contains("not found")
            || line_lc.contains("no such container")
            || line_lc.contains("does not exist");
        let from_cli = line_lc.starts_with("error:")
            || line_lc.starts_with("error ")
            || line_lc.contains(&name_lc);
        absent && from_cli
    })
}

/// Outcome of one `container exec` round trip.
enum ExecAttempt {
    /// A result to hand back to the caller (success, guest failure, or timeout).
    Done(ExecutionResult),
    /// The session's container no longer exists; affinity has been cleared.
    ContainerMissing(CommandOutput),
}

/// Human-readable note about `container system status --format json` output,
/// for logging only.
///
/// **Advisory, never authoritative.** The CLI's zero exit status is the only
/// availability signal this backend acts on. Apple documents that `--format
/// json` exists but not the payload it produces, so any schema asserted here
/// would be a guess. Vetoing on a guessed marker is the wrong direction on a
/// security-relevant path: a healthy service whose status document happens to
/// carry an unrelated sub-entry (a registry or DNS helper reported `inactive`)
/// would fail the probe, and `select_executor` would silently fall through —
/// on a Mac without Docker, all the way to `LocalExecutor`, which has no
/// isolation at all. Losing isolation is worse than using a service whose
/// status JSON we did not fully understand but which answered every command
/// successfully.
///
/// Returns `None` when there is nothing worth logging.
fn status_advisory(stdout: &str) -> Option<String> {
    let trimmed = stdout.trim();
    if trimmed.is_empty() {
        // Some CLI versions print nothing on success.
        return None;
    }
    let value: serde_json::Value = match serde_json::from_str(trimmed) {
        Ok(v) => v,
        Err(e) => {
            return Some(format!(
                "`system status --format json` returned non-JSON: {e}"
            ));
        }
    };

    let mut stopped_marker: Option<String> = None;
    let mut running_marker = false;
    walk_status(&value, &mut stopped_marker, &mut running_marker);

    match (stopped_marker, running_marker) {
        (Some(marker), _) => Some(format!(
            "`system status` exited zero but reported a stopped-looking marker \
             ({marker}); treating the exit status as authoritative"
        )),
        (None, false) => Some(
            "`system status` JSON had no recognised running marker; \
             relying on the CLI exit status"
                .to_string(),
        ),
        (None, true) => None,
    }
}

fn walk_status(value: &serde_json::Value, stopped: &mut Option<String>, running: &mut bool) {
    match value {
        serde_json::Value::Object(map) => {
            for (key, val) in map {
                let key_lc = key.to_lowercase();
                let is_state_key = key_lc.contains("status")
                    || key_lc.contains("state")
                    || key_lc.contains("running");
                match val {
                    serde_json::Value::Bool(b) if is_state_key => {
                        if *b {
                            *running = true;
                        } else {
                            *stopped = Some(format!("{key}=false"));
                        }
                    }
                    serde_json::Value::String(s) if is_state_key => {
                        let s_lc = s.to_lowercase();
                        if s_lc.contains("not running")
                            || s_lc.contains("stopped")
                            || s_lc.contains("inactive")
                        {
                            *stopped = Some(s.clone());
                        } else if s_lc.contains("running")
                            || s_lc.contains("ready")
                            || s_lc.contains("active")
                        {
                            *running = true;
                        }
                    }
                    other => walk_status(other, stopped, running),
                }
            }
        }
        serde_json::Value::Array(items) => {
            for item in items {
                walk_status(item, stopped, running);
            }
        }
        _ => {}
    }
}

impl AppleContainerExecutor {
    /// Build an executor from config using the real process runner.
    ///
    /// This does not touch the host: use [`Self::probe`] to establish
    /// availability before executing anything.
    pub fn new(
        config: RlmConfig,
        validator: Option<Arc<crate::validator::KnowledgeGraphValidator>>,
    ) -> RlmResult<Self> {
        Ok(Self::with_runner(
            config,
            validator,
            Arc::new(TokioProcessRunner),
        ))
    }

    /// Build an executor with an injected [`ProcessRunner`].
    ///
    /// Used by portable tests to pin the CLI argument contract without Apple
    /// software present.
    pub(crate) fn with_runner(
        config: RlmConfig,
        validator: Option<Arc<crate::validator::KnowledgeGraphValidator>>,
        runner: Arc<dyn ProcessRunner>,
    ) -> Self {
        let binary = config
            .apple_container_binary
            .clone()
            .unwrap_or_else(|| PathBuf::from(DEFAULT_BINARY));
        Self {
            runner,
            binary,
            image: config.apple_container_image.clone(),
            session_to_container: DashMap::new(),
            capabilities: vec![
                // One lightweight Linux VM per container is Apple's documented
                // model, so both isolation claims are truthful here.
                Capability::VmIsolation,
                Capability::ContainerIsolation,
                Capability::PythonExecution,
                Capability::BashExecution,
                Capability::FileOperations,
            ],
            validator,
            platform_supported: platform_supported(),
        }
    }

    /// Override the platform gate. Test-only seam: lets portable tests exercise
    /// the CLI contract on non-Apple hosts. Compiled out of release builds so
    /// no downstream caller can bypass the platform gate.
    #[cfg(test)]
    fn with_platform_supported(mut self, supported: bool) -> Self {
        self.platform_supported = supported;
        self
    }

    /// The CLI binary this executor invokes.
    pub fn binary(&self) -> &Path {
        &self.binary
    }

    /// Probe availability using positive evidence only.
    ///
    /// All of the following must hold:
    /// 1. the platform is macOS on aarch64;
    /// 2. the `container` binary resolves and `container system version
    ///    --format json` succeeds;
    /// 3. `container system status --format json` succeeds.
    ///
    /// The status *payload* is only ever logged (see [`status_advisory`]): its
    /// schema is undocumented, so it must not veto an otherwise-successful
    /// probe and downgrade the caller to an unisolated backend.
    ///
    /// Returns a precise reason string on failure so the selector can record it
    /// and continue to the next backend. Never starts the host service.
    pub async fn probe(&self) -> Result<(), String> {
        if !self.platform_supported {
            // Do not spawn the CLI at all off Apple silicon macOS.
            return Err(
                "unsupported platform (Apple container requires macOS on aarch64)".to_string(),
            );
        }

        let version = self
            .run_cli(&["system", "version", "--format", "json"], PROBE_TIMEOUT)
            .await
            .map_err(|e| {
                format!(
                    "`{}` not runnable: {e} (install with `brew install container`)",
                    self.binary.display()
                )
            })?;
        if !version.is_success() {
            return Err(format!(
                "`container system version` failed (exit {}): {}",
                version.exit_code,
                first_line(&version.stderr, &version.stdout)
            ));
        }

        let status = self
            .run_cli(&["system", "status", "--format", "json"], PROBE_TIMEOUT)
            .await
            .map_err(|e| format!("`container system status` not runnable: {e}"))?;
        if !status.is_success() {
            return Err(format!(
                "`container system status` failed (exit {}): {} (run `container system start`)",
                status.exit_code,
                first_line(&status.stderr, &status.stdout)
            ));
        }

        if let Some(note) = status_advisory(&status.stdout) {
            log::debug!("apple-container: {note}");
        }
        Ok(())
    }

    async fn run_cli(&self, args: &[&str], timeout: Duration) -> std::io::Result<CommandOutput> {
        let argv: Vec<String> = args.iter().map(|a| (*a).to_string()).collect();
        self.runner.run(&self.binary, &argv, timeout).await
    }

    async fn run_cli_owned(
        &self,
        args: Vec<String>,
        timeout: Duration,
    ) -> std::io::Result<CommandOutput> {
        self.runner.run(&self.binary, &args, timeout).await
    }

    /// Generate a CLI-safe, collision-free container name.
    ///
    /// Names are derived from a fresh ULID, never from user input, so no caller
    /// can influence the argv passed to `container`.
    fn generate_container_name() -> String {
        format!(
            "{}{}",
            CONTAINER_NAME_PREFIX,
            Ulid::new().to_string().to_lowercase()
        )
    }

    /// Argument vector used to create a session container.
    fn create_argv(&self, name: &str) -> Vec<String> {
        vec![
            "run".to_string(),
            "--detach".to_string(),
            "--name".to_string(),
            name.to_string(),
            "--cpus".to_string(),
            DEFAULT_CPUS.to_string(),
            "--memory".to_string(),
            DEFAULT_MEMORY.to_string(),
            "--cap-drop".to_string(),
            "ALL".to_string(),
            self.image.clone(),
            "sleep".to_string(),
            "infinity".to_string(),
        ]
    }

    /// Resolve (creating at most once) the container bound to `session_id`.
    async fn ensure_container(&self, session_id: &SessionId) -> RlmResult<String> {
        let entry = self
            .session_to_container
            .entry(*session_id)
            .or_insert_with(|| Arc::new(Mutex::new(None)))
            .clone();

        let mut guard = entry.lock().await;
        if let Some(name) = guard.as_ref() {
            return Ok(name.clone());
        }
        // A failed create leaves `guard` as None, so the map is not poisoned
        // and the next call retries with a fresh name.
        let name = self.create_container().await?;
        *guard = Some(name.clone());
        Ok(name)
    }

    async fn create_container(&self) -> RlmResult<String> {
        let name = Self::generate_container_name();
        let output = self
            .run_cli_owned(self.create_argv(&name), LIFECYCLE_TIMEOUT)
            .await
            .map_err(|e| init_failed(format!("failed to spawn `container run`: {e}")))?;

        if !output.is_success() {
            // Best-effort: a partially created container must not leak.
            let _ = self.force_delete(&name).await;
            return Err(init_failed(format!(
                "`container run` failed for {name} (exit {}{}): {}",
                output.exit_code,
                if output.timed_out { ", timed out" } else { "" },
                first_line(&output.stderr, &output.stdout)
            )));
        }
        // The name we generated is authoritative; stdout may carry an id, a
        // progress log, or nothing at all depending on CLI version.
        Ok(name)
    }

    /// Option flags placed between `exec` and the container name.
    ///
    /// Apple's `container exec` documents `--env <key=value>` and
    /// `--workdir/-w`, and both `LocalExecutor` and `SshExecutor` honour these
    /// context fields, so dropping them here would silently execute in the
    /// image's default directory with none of the caller's environment.
    /// Variables are emitted in sorted key order so the argv is deterministic.
    fn exec_option_argv(ctx: &ExecutionContext) -> Vec<String> {
        let mut argv = Vec::new();
        if let Some(dir) = &ctx.working_dir {
            argv.push("--workdir".to_string());
            argv.push(dir.clone());
        }
        let mut keys: Vec<&String> = ctx.env_vars.keys().collect();
        keys.sort();
        for key in keys {
            argv.push("--env".to_string());
            // One argv element: no host shell splits on the value.
            argv.push(format!("{key}={}", ctx.env_vars[key]));
        }
        argv
    }

    /// Execute an argv inside the session container, failing closed on timeout.
    ///
    /// If the session's container has vanished between calls (removed by hand,
    /// service restarted, host slept), the affinity mapping is dropped and the
    /// call is retried exactly once against a fresh container. Without that,
    /// the dead name would be reused forever and every subsequent call for the
    /// session would return the CLI's "container not found" as if it were a
    /// guest failure.
    async fn exec_in_container(
        &self,
        guest_argv: Vec<String>,
        ctx: &ExecutionContext,
    ) -> RlmResult<ExecutionResult> {
        match self.exec_attempt(&guest_argv, ctx).await? {
            ExecAttempt::Done(result) => Ok(result),
            ExecAttempt::ContainerMissing(first) => {
                log::warn!(
                    "apple-container: session container vanished ({}); retrying once with a fresh container",
                    first_line(&first.stderr, &first.stdout)
                );
                match self.exec_attempt(&guest_argv, ctx).await? {
                    ExecAttempt::Done(result) => Ok(result),
                    // Two consecutive vanishings mean the runtime, not the
                    // container, is broken: report it as a backend failure
                    // rather than a guest exit code the caller would retry.
                    ExecAttempt::ContainerMissing(second) => Err(RlmError::ExecutionFailed {
                        message: format!(
                            "{BACKEND_NAME}: session container disappeared twice in a row; \
                             the `container` service may not be running"
                        ),
                        exit_code: Some(second.exit_code),
                        stdout: Some(second.stdout),
                        stderr: Some(second.stderr),
                    }),
                }
            }
        }
    }

    /// One `container exec` round trip against the session's container.
    async fn exec_attempt(
        &self,
        guest_argv: &[String],
        ctx: &ExecutionContext,
    ) -> RlmResult<ExecAttempt> {
        let name = self.ensure_container(&ctx.session_id).await?;

        let options = Self::exec_option_argv(ctx);
        let mut argv = Vec::with_capacity(guest_argv.len() + options.len() + 2);
        argv.push("exec".to_string());
        argv.extend(options);
        argv.push(name.clone());
        argv.extend_from_slice(guest_argv);

        let start = Instant::now();
        let output = self
            .run_cli_owned(argv, Duration::from_millis(ctx.timeout_ms))
            .await
            .map_err(|e| RlmError::ExecutionFailed {
                message: format!("failed to spawn `container exec` for {name}: {e}"),
                exit_code: None,
                stdout: None,
                stderr: None,
            })?;
        let execution_time_ms = start.elapsed().as_millis() as u64;

        if output.timed_out {
            // The guest exec process cannot be proven dead, so destroy the
            // whole session container and clear affinity. The next call gets a
            // fresh VM rather than a container with a runaway process in it.
            self.abandon_session(&ctx.session_id, &name).await;
            return Ok(ExecAttempt::Done(
                ExecutionResult::timeout(output.stdout, output.stderr)
                    .with_execution_time(execution_time_ms),
            ));
        }

        if !output.is_success() && exec_reports_container_missing(&output, &name) {
            // Only the mapping is cleared: the container is already gone, so
            // there is nothing to delete.
            self.session_to_container.remove(&ctx.session_id);
            return Ok(ExecAttempt::ContainerMissing(output));
        }

        Ok(ExecAttempt::Done(ExecutionResult {
            stdout: output.stdout,
            stderr: output.stderr,
            exit_code: output.exit_code,
            execution_time_ms,
            output_truncated: false,
            output_file_path: None,
            timed_out: false,
            metadata: std::collections::HashMap::from([
                ("backend".to_string(), BACKEND_NAME.to_string()),
                ("container".to_string(), name),
            ]),
        }))
    }

    /// Drop the session mapping and force-remove its container.
    async fn abandon_session(&self, session_id: &SessionId, name: &str) {
        self.session_to_container.remove(session_id);
        if let Err(e) = self.force_delete(name).await {
            log::warn!("apple-container: failed to destroy {name} after timeout: {e}");
        }
    }

    /// Stop (with grace) then delete a container. Already-absent is success.
    async fn stop_and_delete(&self, name: &str) -> RlmResult<()> {
        match self
            .run_cli_owned(
                vec![
                    "stop".to_string(),
                    "--time".to_string(),
                    STOP_GRACE_SECS.to_string(),
                    name.to_string(),
                ],
                LIFECYCLE_TIMEOUT,
            )
            .await
        {
            Ok(output) if output.is_success() || is_already_absent(&output) => {}
            Ok(output) => {
                // A stop failure is not fatal: `delete --force` is next and can
                // still reclaim the resource.
                log::debug!(
                    "apple-container: `container stop {name}` exit {}: {}",
                    output.exit_code,
                    first_line(&output.stderr, &output.stdout)
                );
            }
            Err(e) => log::debug!("apple-container: `container stop {name}` not runnable: {e}"),
        }
        self.force_delete(name).await
    }

    async fn force_delete(&self, name: &str) -> RlmResult<()> {
        let output = self
            .run_cli_owned(
                vec![
                    "delete".to_string(),
                    "--force".to_string(),
                    name.to_string(),
                ],
                LIFECYCLE_TIMEOUT,
            )
            .await
            .map_err(|e| RlmError::Internal {
                message: format!("{BACKEND_NAME}: failed to spawn `container delete {name}`: {e}"),
            })?;

        if output.is_success() || is_already_absent(&output) {
            return Ok(());
        }
        Err(RlmError::Internal {
            message: format!(
                "{BACKEND_NAME}: failed to remove container {name} (exit {}): {}",
                output.exit_code,
                first_line(&output.stderr, &output.stdout)
            ),
        })
    }

    /// Release the container bound to `session_id`, if any. Returns the removed
    /// container name.
    pub async fn release_session_container(&self, session_id: &SessionId) -> Option<String> {
        let removed = self.session_to_container.remove(session_id)?;
        let name = removed.1.lock().await.take()?;
        if let Err(e) = self.stop_and_delete(&name).await {
            log::warn!("apple-container: release_session_container({session_id}): {e}");
        }
        Some(name)
    }

    /// Drain every tracked session and return the container names.
    ///
    /// Note the ordering: the map is emptied *before* removal is attempted, so
    /// a container whose `delete` subsequently fails becomes untracked — `Drop`
    /// will not retry it and only the aggregate error and the warning log name
    /// it. That is deliberate (cleanup must not leave half-torn-down sessions
    /// reachable for reuse); recovery is the prefix sweep documented in
    /// `docs/apple-container-backend.md`.
    async fn drain_container_names(&self) -> Vec<String> {
        let entries: Vec<_> = self
            .session_to_container
            .iter()
            .map(|kv| kv.value().clone())
            .collect();
        self.session_to_container.clear();

        let mut names = Vec::with_capacity(entries.len());
        for entry in entries {
            if let Some(name) = entry.lock().await.take() {
                names.push(name);
            }
        }
        names
    }
}

/// First non-empty line of stderr, falling back to stdout, for diagnostics.
fn first_line(stderr: &str, stdout: &str) -> String {
    for text in [stderr, stdout] {
        if let Some(line) = text.lines().map(str::trim).find(|l| !l.is_empty()) {
            return line.to_string();
        }
    }
    "<no output>".to_string()
}

#[async_trait]
impl super::ExecutionEnvironment for AppleContainerExecutor {
    type Error = RlmError;

    async fn execute_code(
        &self,
        code: &str,
        ctx: &ExecutionContext,
    ) -> Result<ExecutionResult, Self::Error> {
        // `code` is a single argv element: no host shell sees it.
        self.exec_in_container(
            vec!["python3".to_string(), "-c".to_string(), code.to_string()],
            ctx,
        )
        .await
    }

    async fn execute_command(
        &self,
        cmd: &str,
        ctx: &ExecutionContext,
    ) -> Result<ExecutionResult, Self::Error> {
        // `cmd` is interpreted by bash *inside the guest*, never by a host shell.
        self.exec_in_container(
            vec!["bash".to_string(), "-lc".to_string(), cmd.to_string()],
            ctx,
        )
        .await
    }

    async fn validate(&self, input: &str) -> Result<ValidationResult, Self::Error> {
        match self.validator.as_ref() {
            Some(validator) if !input.trim().is_empty() => {
                let vr = validator.validate(input)?;
                Ok(ValidationResult::from_validator_result(
                    &vr,
                    crate::config::KgStrictness::Normal,
                ))
            }
            _ => Ok(ValidationResult::valid(Vec::new())),
        }
    }

    async fn create_snapshot(
        &self,
        _session_id: &SessionId,
        _name: &str,
    ) -> Result<SnapshotId, Self::Error> {
        Err(unsupported("create_snapshot"))
    }

    async fn restore_snapshot(&self, _id: &SnapshotId) -> Result<(), Self::Error> {
        Err(unsupported("restore_snapshot"))
    }

    async fn list_snapshots(
        &self,
        _session_id: &SessionId,
    ) -> Result<Vec<SnapshotId>, Self::Error> {
        Err(unsupported("list_snapshots"))
    }

    async fn delete_snapshot(&self, _id: &SnapshotId) -> Result<(), Self::Error> {
        Err(unsupported("delete_snapshot"))
    }

    async fn delete_session_snapshots(&self, _session_id: &SessionId) -> Result<(), Self::Error> {
        Err(unsupported("delete_session_snapshots"))
    }

    fn capabilities(&self) -> &[Capability] {
        &self.capabilities
    }

    fn backend_type(&self) -> BackendType {
        BackendType::AppleContainer
    }

    async fn health_check(&self) -> Result<bool, Self::Error> {
        Ok(self.probe().await.is_ok())
    }

    async fn cleanup(&self) -> Result<(), Self::Error> {
        let names = self.drain_container_names().await;
        let mut failures = Vec::new();
        // Every tracked resource is attempted even if an earlier one fails.
        for name in &names {
            if let Err(e) = self.stop_and_delete(name).await {
                log::warn!("apple-container: cleanup failed for {name}: {e}");
                failures.push(format!("{name}: {e}"));
            }
        }
        if failures.is_empty() {
            Ok(())
        } else {
            Err(RlmError::Internal {
                message: format!(
                    "{BACKEND_NAME}: cleanup failed for {}/{} containers: {}",
                    failures.len(),
                    names.len(),
                    failures.join("; ")
                ),
            })
        }
    }

    async fn end_session(&self, session_id: &SessionId) -> Result<(), Self::Error> {
        // Unknown sessions are a no-op, and teardown of an already-absent
        // container is success.
        let _ = self.release_session_container(session_id).await;
        Ok(())
    }
}

impl Drop for AppleContainerExecutor {
    fn drop(&mut self) {
        let entries: Vec<_> = self
            .session_to_container
            .iter()
            .map(|kv| kv.value().clone())
            .collect();
        self.session_to_container.clear();
        if entries.is_empty() {
            return;
        }

        // Names are only reachable behind async mutexes, so resolve them with
        // try_lock: Drop must not block and must not claim success it cannot
        // deliver. Correctness relies on explicit end_session()/cleanup().
        let names: Vec<String> = entries
            .iter()
            .filter_map(|e| e.try_lock().ok().and_then(|mut g| g.take()))
            .collect();

        if names.is_empty() {
            log::warn!(
                "AppleContainerExecutor dropped with {} session entries still locked; \
                 containers may be leaked. Recover with: container list --all | grep {}",
                entries.len(),
                CONTAINER_NAME_PREFIX
            );
            return;
        }

        let binary = self.binary.clone();
        let runner = self.runner.clone();
        match tokio::runtime::Handle::try_current() {
            Ok(_) => {
                // Spawned cleanup is best effort: this Drop does not and cannot
                // report whether it succeeded.
                tokio::spawn(async move {
                    for name in names {
                        let argv = vec!["delete".to_string(), "--force".to_string(), name.clone()];
                        match runner.run(&binary, &argv, LIFECYCLE_TIMEOUT).await {
                            Ok(out) if out.is_success() || is_already_absent(&out) => {}
                            Ok(out) => log::warn!(
                                "apple-container: Drop failed to remove {name} (exit {}): {}",
                                out.exit_code,
                                first_line(&out.stderr, &out.stdout)
                            ),
                            Err(e) => log::warn!(
                                "apple-container: Drop could not spawn delete for {name}: {e}"
                            ),
                        }
                    }
                });
            }
            Err(_) => {
                log::warn!(
                    "AppleContainerExecutor dropped outside a Tokio runtime; \
                     these containers were NOT cleaned up: {}. Recover with: {}",
                    names.join(", "),
                    names
                        .iter()
                        .map(|n| format!("container delete --force {n}"))
                        .collect::<Vec<_>>()
                        .join(" && ")
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::executor::ExecutionEnvironment;
    use std::sync::Mutex as StdMutex;

    /// Scripted response to one CLI invocation, keyed on the argv it receives.
    type Handler = Box<dyn Fn(&[String]) -> std::io::Result<CommandOutput> + Send + Sync>;

    /// Scripted [`ProcessRunner`] that records every argv **and deadline** it
    /// is handed. Recording the deadline is what lets the timeout invariant be
    /// pinned: without it, swapping `ctx.timeout_ms` for `LIFECYCLE_TIMEOUT` at
    /// the exec call site would leave the whole suite green.
    struct FakeRunner {
        calls: StdMutex<Vec<(Vec<String>, Duration)>>,
        handler: Handler,
        delay: Duration,
    }

    impl std::fmt::Debug for FakeRunner {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.debug_struct("FakeRunner")
                .field("calls", &self.calls)
                .field("delay", &self.delay)
                .finish_non_exhaustive()
        }
    }

    impl FakeRunner {
        fn new<F>(handler: F) -> Arc<Self>
        where
            F: Fn(&[String]) -> std::io::Result<CommandOutput> + Send + Sync + 'static,
        {
            Arc::new(Self {
                calls: StdMutex::new(Vec::new()),
                handler: Box::new(handler),
                delay: Duration::ZERO,
            })
        }

        fn with_delay<F>(delay: Duration, handler: F) -> Arc<Self>
        where
            F: Fn(&[String]) -> std::io::Result<CommandOutput> + Send + Sync + 'static,
        {
            Arc::new(Self {
                calls: StdMutex::new(Vec::new()),
                handler: Box::new(handler),
                delay,
            })
        }

        /// Always-succeeding runner with empty output.
        fn ok() -> Arc<Self> {
            Self::new(|_| Ok(ok_output("")))
        }

        /// Every recorded argv, without its deadline.
        fn calls(&self) -> Vec<Vec<String>> {
            self.calls_with_timeout()
                .into_iter()
                .map(|(argv, _)| argv)
                .collect()
        }

        /// Every recorded argv paired with the deadline it was run under.
        fn calls_with_timeout(&self) -> Vec<(Vec<String>, Duration)> {
            self.calls.lock().unwrap().clone()
        }

        fn calls_starting_with(&self, verb: &str) -> Vec<Vec<String>> {
            self.calls()
                .into_iter()
                .filter(|c| c.first().map(|v| v == verb).unwrap_or(false))
                .collect()
        }

        /// Deadlines recorded for every call whose first argument is `verb`.
        fn timeouts_for(&self, verb: &str) -> Vec<Duration> {
            self.calls_with_timeout()
                .into_iter()
                .filter(|(argv, _)| argv.first().map(|v| v == verb).unwrap_or(false))
                .map(|(_, timeout)| timeout)
                .collect()
        }
    }

    #[async_trait]
    impl ProcessRunner for FakeRunner {
        async fn run(
            &self,
            _program: &Path,
            args: &[String],
            timeout: Duration,
        ) -> std::io::Result<CommandOutput> {
            self.calls.lock().unwrap().push((args.to_vec(), timeout));
            if !self.delay.is_zero() {
                tokio::time::sleep(self.delay).await;
            }
            (self.handler)(args)
        }
    }

    fn ok_output(stdout: &str) -> CommandOutput {
        CommandOutput {
            exit_code: 0,
            stdout: stdout.to_string(),
            stderr: String::new(),
            timed_out: false,
        }
    }

    fn fail_output(exit_code: i32, stderr: &str) -> CommandOutput {
        CommandOutput {
            exit_code,
            stdout: String::new(),
            stderr: stderr.to_string(),
            timed_out: false,
        }
    }

    fn executor(runner: Arc<dyn ProcessRunner>) -> AppleContainerExecutor {
        AppleContainerExecutor::with_runner(RlmConfig::minimal(), None, runner)
            .with_platform_supported(true)
    }

    fn ctx() -> ExecutionContext {
        ExecutionContext {
            session_id: SessionId::new(),
            timeout_ms: 30_000,
            ..Default::default()
        }
    }

    // ---------------------------------------------------------------
    // Step 2: availability probe
    // ---------------------------------------------------------------

    #[tokio::test]
    async fn probe_on_unsupported_platform_never_spawns_cli() {
        let runner = FakeRunner::ok();
        let exec = AppleContainerExecutor::with_runner(RlmConfig::minimal(), None, runner.clone())
            .with_platform_supported(false);

        let err = exec.probe().await.unwrap_err();
        assert!(err.contains("unsupported platform"), "{err}");
        assert!(
            runner.calls().is_empty(),
            "probe must not spawn the CLI off Apple silicon macOS"
        );
    }

    #[tokio::test]
    async fn probe_reports_missing_binary_distinctly() {
        let runner = FakeRunner::new(|_| {
            Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "No such file or directory",
            ))
        });
        let err = executor(runner).probe().await.unwrap_err();
        assert!(err.contains("not runnable"), "{err}");
        assert!(err.contains("brew install container"), "{err}");
    }

    #[tokio::test]
    async fn probe_reports_failed_system_version_distinctly() {
        let runner = FakeRunner::new(|args| {
            if args[1] == "version" {
                Ok(fail_output(1, "unknown command"))
            } else {
                Ok(ok_output("{}"))
            }
        });
        let err = executor(runner).probe().await.unwrap_err();
        assert!(err.contains("system version"), "{err}");
        assert!(err.contains("unknown command"), "{err}");
    }

    #[tokio::test]
    async fn probe_reports_failed_system_status_distinctly() {
        let runner = FakeRunner::new(|args| {
            if args[1] == "status" {
                Ok(fail_output(1, "XPC connection error"))
            } else {
                Ok(ok_output("{\"version\":\"1.2.2\"}"))
            }
        });
        let err = executor(runner).probe().await.unwrap_err();
        assert!(err.contains("system status"), "{err}");
        assert!(err.contains("container system start"), "{err}");
    }

    #[tokio::test]
    async fn probe_does_not_veto_on_an_undocumented_status_payload() {
        // Apple documents `--format json` but not its schema, so a sub-entry
        // that merely *looks* stopped must not fail an otherwise-successful
        // probe: doing so would silently drop the caller to LocalExecutor,
        // which has no isolation at all.
        for payload in [
            "{\"apiServer\":{\"status\":\"not running\"}}",
            "{\"services\":[{\"name\":\"dns\",\"status\":\"inactive\"}]}",
            "{\"totallyUnexpected\":42}",
            "not json at all",
            "",
        ] {
            let runner = FakeRunner::new(move |args| {
                if args[1] == "status" {
                    Ok(ok_output(payload))
                } else {
                    Ok(ok_output("{\"version\":\"1.2.2\"}"))
                }
            });
            assert!(
                executor(runner).probe().await.is_ok(),
                "payload must not veto a zero-exit probe: {payload}"
            );
        }
    }

    #[test]
    fn status_advisory_notes_are_diagnostic_only() {
        // Advisory text still flags the odd cases, it just never blocks.
        assert!(status_advisory("").is_none());
        assert!(status_advisory("{\"apiServer\":{\"status\":\"running\"}}").is_none());
        assert!(
            status_advisory("{\"apiServer\":{\"status\":\"stopped\"}}")
                .unwrap()
                .contains("stopped-looking")
        );
        assert!(
            status_advisory("not json at all")
                .unwrap()
                .contains("non-JSON")
        );
        assert!(
            status_advisory("{\"totallyUnexpected\":42}")
                .unwrap()
                .contains("no recognised running marker")
        );
    }

    #[tokio::test]
    async fn probe_accepts_healthy_status_and_never_starts_the_service() {
        let runner = FakeRunner::new(|args| {
            if args[1] == "status" {
                Ok(ok_output("{\"apiServer\":{\"status\":\"running\"}}"))
            } else {
                Ok(ok_output("{\"version\":\"1.2.2\"}"))
            }
        });
        let exec = executor(runner.clone());
        assert!(exec.probe().await.is_ok());
        assert!(exec.health_check().await.unwrap());

        for call in runner.calls() {
            assert!(
                !call.contains(&"start".to_string()),
                "probe must never run `container system start`: {call:?}"
            );
        }
    }

    // ---------------------------------------------------------------
    // Step 3: exactly-once per-session creation
    // ---------------------------------------------------------------

    #[test]
    fn generated_names_are_prefixed_and_cli_safe() {
        for _ in 0..64 {
            let name = AppleContainerExecutor::generate_container_name();
            assert!(name.starts_with(CONTAINER_NAME_PREFIX), "{name}");
            let suffix = &name[CONTAINER_NAME_PREFIX.len()..];
            assert_eq!(suffix.len(), 26, "ULID suffix length: {name}");
            assert!(
                suffix
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit()),
                "unexpected characters in {name}"
            );
        }
    }

    #[tokio::test]
    async fn create_argv_matches_the_pinned_cli_contract() {
        let runner = FakeRunner::ok();
        let exec = executor(runner.clone());
        exec.execute_command("echo hi", &ctx()).await.unwrap();

        let runs = runner.calls_starting_with("run");
        assert_eq!(runs.len(), 1);
        let argv = &runs[0];
        assert_eq!(argv[1], "--detach");
        assert_eq!(argv[2], "--name");
        assert!(argv[3].starts_with(CONTAINER_NAME_PREFIX));
        assert_eq!(
            argv[4..10].to_vec(),
            vec!["--cpus", "1", "--memory", "512M", "--cap-drop", "ALL"]
        );
        assert_eq!(argv[10], "python:3.11-slim");
        assert_eq!(argv[11], "sleep");
        assert_eq!(argv[12], "infinity");

        // No host mounts, SSH forwarding, or privilege escalation by default.
        for forbidden in [
            "--volume",
            "-v",
            "--mount",
            "--ssh",
            "--privileged",
            "--cap-add",
            "--env-file",
        ] {
            assert!(
                !argv.iter().any(|a| a == forbidden),
                "default create argv must not contain {forbidden}: {argv:?}"
            );
        }
    }

    #[tokio::test]
    async fn concurrent_first_commands_create_exactly_one_container() {
        // The delay widens the race window so the per-session mutex is really
        // exercised rather than accidentally serialised by fast returns.
        let runner = FakeRunner::with_delay(Duration::from_millis(20), |_| Ok(ok_output("")));
        let exec = Arc::new(executor(runner.clone()));
        let session = SessionId::new();

        let mut handles = Vec::new();
        for _ in 0..8 {
            let exec = exec.clone();
            handles.push(tokio::spawn(async move {
                exec.ensure_container(&session).await.unwrap()
            }));
        }
        let names: Vec<String> = futures::future::join_all(handles)
            .await
            .into_iter()
            .map(|r| r.unwrap())
            .collect();

        assert!(names.iter().all(|n| n == &names[0]));
        assert_eq!(runner.calls_starting_with("run").len(), 1);
    }

    #[tokio::test]
    async fn distinct_sessions_get_distinct_containers() {
        let runner = FakeRunner::ok();
        let exec = executor(runner.clone());
        let a = exec.ensure_container(&SessionId::new()).await.unwrap();
        let b = exec.ensure_container(&SessionId::new()).await.unwrap();
        assert_ne!(a, b);
        assert_eq!(runner.calls_starting_with("run").len(), 2);
    }

    #[tokio::test]
    async fn failed_create_does_not_poison_the_session_map() {
        let attempts = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let seen = attempts.clone();
        let runner = FakeRunner::new(move |args| {
            if args[0] != "run" {
                return Ok(ok_output(""));
            }
            let n = seen.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            if n == 0 {
                Ok(fail_output(125, "image not found"))
            } else {
                Ok(ok_output(""))
            }
        });
        let exec = executor(runner);
        let session = SessionId::new();

        let err = exec.ensure_container(&session).await.unwrap_err();
        assert!(matches!(err, RlmError::BackendInitFailed { .. }), "{err:?}");

        // Retry must succeed with a fresh container.
        let name = exec.ensure_container(&session).await.unwrap();
        assert!(name.starts_with(CONTAINER_NAME_PREFIX));
    }

    // ---------------------------------------------------------------
    // Step 4: execution and result mapping
    // ---------------------------------------------------------------

    #[tokio::test]
    async fn python_code_is_one_argv_value_after_python3_dash_c() {
        let runner = FakeRunner::ok();
        let exec = executor(runner.clone());
        let code = "print('a b');\nprint(\"$(whoami)\" + '`id`' + '; rm -rf /')";
        exec.execute_code(code, &ctx()).await.unwrap();

        let execs = runner.calls_starting_with("exec");
        assert_eq!(execs.len(), 1);
        let argv = &execs[0];
        assert!(argv[1].starts_with(CONTAINER_NAME_PREFIX));
        assert_eq!(argv[2], "python3");
        assert_eq!(argv[3], "-c");
        assert_eq!(argv[4], code);
        assert_eq!(argv.len(), 5, "code must remain exactly one argv element");
    }

    #[tokio::test]
    async fn shell_metacharacters_remain_one_guest_argv_value() {
        let runner = FakeRunner::ok();
        let exec = executor(runner.clone());
        let cmd = "echo 'a b' && echo $HOME; echo `id`\necho done # comment";
        exec.execute_command(cmd, &ctx()).await.unwrap();

        let argv = &runner.calls_starting_with("exec")[0];
        assert_eq!(argv[2], "bash");
        assert_eq!(argv[3], "-lc");
        assert_eq!(argv[4], cmd);
        assert_eq!(argv.len(), 5);
    }

    #[tokio::test]
    async fn result_maps_stdout_stderr_exit_code_and_metadata() {
        let runner = FakeRunner::new(|args| {
            if args[0] == "exec" {
                Ok(CommandOutput {
                    exit_code: 3,
                    stdout: "out".to_string(),
                    stderr: "err".to_string(),
                    timed_out: false,
                })
            } else {
                Ok(ok_output(""))
            }
        });
        let exec = executor(runner);
        let result = exec.execute_command("false", &ctx()).await.unwrap();

        assert_eq!(result.stdout, "out");
        assert_eq!(result.stderr, "err");
        assert_eq!(result.exit_code, 3);
        assert!(!result.is_success());
        assert!(!result.timed_out);
        assert_eq!(
            result.metadata.get("backend").map(String::as_str),
            Some(BACKEND_NAME)
        );
        assert!(
            result.metadata["container"].starts_with(CONTAINER_NAME_PREFIX),
            "{:?}",
            result.metadata
        );
    }

    #[tokio::test]
    async fn create_uses_generated_name_not_command_stdout() {
        // Some CLI versions print an id or progress text; the authoritative
        // handle is the --name we generated.
        let runner = FakeRunner::new(|_| Ok(ok_output("deadbeefcafe\nPulling image...\n")));
        let exec = executor(runner.clone());
        exec.execute_command("true", &ctx()).await.unwrap();

        let created = runner.calls_starting_with("run")[0][3].clone();
        let used = runner.calls_starting_with("exec")[0][1].clone();
        assert_eq!(created, used);
        assert!(used.starts_with(CONTAINER_NAME_PREFIX));
    }

    #[tokio::test]
    async fn validate_without_validator_is_permissive_like_local_and_docker() {
        let exec = executor(FakeRunner::ok());
        let result = exec.validate("anything at all").await.unwrap();
        assert!(result.is_valid);
    }

    #[tokio::test]
    async fn backend_identity_and_capabilities_are_truthful() {
        let exec = executor(FakeRunner::ok());
        assert_eq!(exec.backend_type(), BackendType::AppleContainer);
        assert!(exec.has_capability(Capability::VmIsolation));
        assert!(exec.has_capability(Capability::ContainerIsolation));
        assert!(exec.has_capability(Capability::PythonExecution));
        assert!(exec.has_capability(Capability::BashExecution));
        assert!(exec.has_capability(Capability::FileOperations));
        // Not claimed: snapshots and DNS allowlist enforcement.
        assert!(!exec.has_capability(Capability::Snapshots));
        assert!(!exec.has_capability(Capability::DnsAllowlist));
    }

    // ---------------------------------------------------------------
    // Step 5: timeout fails closed
    // ---------------------------------------------------------------

    #[tokio::test]
    async fn timeout_destroys_container_clears_affinity_and_keeps_partial_output() {
        let runner = FakeRunner::new(|args| {
            if args[0] == "exec" {
                Ok(CommandOutput {
                    exit_code: -1,
                    stdout: "partial-out".to_string(),
                    stderr: "partial-err".to_string(),
                    timed_out: true,
                })
            } else {
                Ok(ok_output(""))
            }
        });
        let exec = executor(runner.clone());
        let ctx = ctx();

        let result = exec.execute_command("sleep 100", &ctx).await.unwrap();
        assert!(result.timed_out);
        assert!(!result.is_success());
        assert_eq!(result.stdout, "partial-out");
        assert_eq!(result.stderr, "partial-err");

        let first_container = runner.calls_starting_with("run")[0][3].clone();
        let deletes = runner.calls_starting_with("delete");
        assert_eq!(deletes.len(), 1, "timed-out container must be destroyed");
        assert_eq!(deletes[0], vec!["delete", "--force", &first_container]);

        // Affinity cleared: the next execution creates a fresh container.
        assert!(!exec.session_to_container.contains_key(&ctx.session_id));
        exec.execute_command("echo ok", &ctx).await.unwrap();
        let runs = runner.calls_starting_with("run");
        assert_eq!(runs.len(), 2);
        assert_ne!(runs[1][3], first_container);
    }

    #[tokio::test]
    async fn exec_uses_the_context_deadline_and_lifecycle_calls_use_their_own() {
        // Pins the timeout invariant itself, not just its consequences: if the
        // exec call site were changed to pass LIFECYCLE_TIMEOUT, this fails.
        let runner = FakeRunner::ok();
        let exec = executor(runner.clone());
        let mut ctx = ctx();
        ctx.timeout_ms = 1_234;

        exec.execute_command("true", &ctx).await.unwrap();
        exec.end_session(&ctx.session_id).await.unwrap();

        assert_eq!(
            runner.timeouts_for("exec"),
            vec![Duration::from_millis(1_234)],
            "exec must be bounded by ExecutionContext::timeout_ms"
        );
        for verb in ["run", "stop", "delete"] {
            let timeouts = runner.timeouts_for(verb);
            assert!(!timeouts.is_empty(), "no `{verb}` call recorded");
            assert!(
                timeouts.iter().all(|t| *t == LIFECYCLE_TIMEOUT),
                "`{verb}` must use LIFECYCLE_TIMEOUT, got {timeouts:?}"
            );
        }
    }

    #[tokio::test]
    async fn probe_calls_are_bounded_by_the_probe_timeout() {
        let runner = FakeRunner::new(|_| Ok(ok_output("{\"version\":\"1.2.2\"}")));
        let exec = executor(runner.clone());
        exec.probe().await.unwrap();

        let timeouts: Vec<Duration> = runner
            .calls_with_timeout()
            .into_iter()
            .map(|(_, t)| t)
            .collect();
        assert_eq!(timeouts, vec![PROBE_TIMEOUT, PROBE_TIMEOUT]);
    }

    #[tokio::test]
    async fn timed_out_exec_container_teardown_uses_the_lifecycle_timeout() {
        let runner = FakeRunner::new(|args| {
            if args[0] == "exec" {
                Ok(CommandOutput {
                    exit_code: -1,
                    stdout: String::new(),
                    stderr: String::new(),
                    timed_out: true,
                })
            } else {
                Ok(ok_output(""))
            }
        });
        let exec = executor(runner.clone());
        let mut ctx = ctx();
        ctx.timeout_ms = 50;

        exec.execute_command("sleep 100", &ctx).await.unwrap();

        assert_eq!(runner.timeouts_for("exec"), vec![Duration::from_millis(50)]);
        assert_eq!(runner.timeouts_for("delete"), vec![LIFECYCLE_TIMEOUT]);
    }

    // ---------------------------------------------------------------
    // Vanished session container: recover instead of wedging
    // ---------------------------------------------------------------

    #[tokio::test]
    async fn vanished_container_is_replaced_and_the_call_retried_once() {
        // The container is removed out from under us (service restart, host
        // sleep, manual delete). The first exec fails with the CLI's own
        // "not found"; the session must recover rather than fail forever.
        let first_name = Arc::new(StdMutex::new(String::new()));
        let dead = first_name.clone();
        let runner = FakeRunner::new(move |args| {
            if args[0] == "run" {
                return Ok(ok_output(""));
            }
            if args[0] == "exec" && args[1] == *dead.lock().unwrap() {
                return Ok(fail_output(1, "Error: container not found"));
            }
            Ok(ok_output("second-try"))
        });
        let exec = executor(runner.clone());
        let ctx = ctx();

        // Establish the session, then declare its container dead.
        let name = exec.ensure_container(&ctx.session_id).await.unwrap();
        *first_name.lock().unwrap() = name.clone();

        let result = exec.execute_command("echo hi", &ctx).await.unwrap();
        assert!(result.is_success(), "{result:?}");
        assert_eq!(result.stdout, "second-try");

        let runs = runner.calls_starting_with("run");
        assert_eq!(runs.len(), 2, "a fresh container must be created");
        assert_ne!(runs[1][3], name);
        assert_eq!(
            result.metadata["container"], runs[1][3],
            "result must name the container that actually ran it"
        );
        assert_eq!(runner.calls_starting_with("exec").len(), 2, "retried once");
    }

    #[tokio::test]
    async fn a_container_that_vanishes_twice_is_a_backend_error_not_a_guest_exit() {
        let runner = FakeRunner::new(|args| {
            if args[0] == "exec" {
                Ok(fail_output(1, "Error: container not found"))
            } else {
                Ok(ok_output(""))
            }
        });
        let exec = executor(runner.clone());

        let err = exec.execute_command("echo hi", &ctx()).await.unwrap_err();
        match err {
            RlmError::ExecutionFailed { ref message, .. } => {
                assert!(message.contains("disappeared twice"), "{message}")
            }
            other => panic!("expected ExecutionFailed, got {other:?}"),
        }
        // Bounded: exactly one retry, not an unbounded loop.
        assert_eq!(runner.calls_starting_with("exec").len(), 2);
    }

    #[tokio::test]
    async fn guest_command_not_found_is_a_guest_failure_not_a_dead_container() {
        // `bash: frobnicate: command not found` must NOT be mistaken for a
        // missing container: recycling the session would discard guest state
        // and re-run the caller's command.
        let runner = FakeRunner::new(|args| {
            if args[0] == "exec" {
                Ok(fail_output(127, "bash: frobnicate: command not found"))
            } else {
                Ok(ok_output(""))
            }
        });
        let exec = executor(runner.clone());
        let ctx = ctx();

        let result = exec.execute_command("frobnicate", &ctx).await.unwrap();
        assert_eq!(result.exit_code, 127);
        assert_eq!(runner.calls_starting_with("exec").len(), 1, "no retry");
        assert_eq!(runner.calls_starting_with("run").len(), 1, "session kept");
        assert!(exec.session_to_container.contains_key(&ctx.session_id));
    }

    #[test]
    fn container_missing_detection_requires_cli_provenance() {
        let name = AppleContainerExecutor::generate_container_name();
        let missing = |stderr: &str| exec_reports_container_missing(&fail_output(1, stderr), &name);
        assert!(missing("Error: container not found"));
        assert!(missing(&format!("error: no such container: {name}")));
        assert!(missing(&format!("{name} does not exist")));
        // Guest output that merely contains the words must not match.
        assert!(!missing("bash: frobnicate: command not found"));
        assert!(!missing(
            "ModuleNotFoundError: No module named 'x' (not found)"
        ));
        assert!(!missing(""));
    }

    // ---------------------------------------------------------------
    // Execution context: env vars and working directory
    // ---------------------------------------------------------------

    #[tokio::test]
    async fn working_dir_and_env_vars_are_passed_as_exec_flags() {
        let runner = FakeRunner::ok();
        let exec = executor(runner.clone());
        let ctx = ExecutionContext {
            session_id: SessionId::new(),
            timeout_ms: 30_000,
            ..Default::default()
        }
        .with_working_dir("/work")
        .with_env("B_VAR", "two")
        .with_env("A_VAR", "one; rm -rf /");

        exec.execute_command("pwd", &ctx).await.unwrap();

        let argv = &runner.calls_starting_with("exec")[0];
        // Flags precede the container name, which precedes the guest argv.
        assert_eq!(
            argv[..7].to_vec(),
            vec![
                "exec",
                "--workdir",
                "/work",
                "--env",
                "A_VAR=one; rm -rf /", // still exactly one argv element
                "--env",
                "B_VAR=two",
            ],
            "{argv:?}"
        );
        assert!(argv[7].starts_with(CONTAINER_NAME_PREFIX));
        assert_eq!(argv[8..].to_vec(), vec!["bash", "-lc", "pwd"]);
    }

    #[tokio::test]
    async fn default_context_adds_no_exec_flags() {
        let runner = FakeRunner::ok();
        let exec = executor(runner.clone());
        exec.execute_command("true", &ctx()).await.unwrap();

        let argv = &runner.calls_starting_with("exec")[0];
        assert!(argv[1].starts_with(CONTAINER_NAME_PREFIX), "{argv:?}");
    }

    #[tokio::test]
    async fn tokio_runner_timeout_kills_and_reaps_child_preserving_partial_output() {
        // Exercises the real process path: a long-lived child that has already
        // written to stdout must be killed, reaped, and its output preserved.
        let sh = Path::new("/bin/sh");
        if !sh.exists() {
            eprintln!("skipping: /bin/sh not available");
            return;
        }
        let runner = TokioProcessRunner;
        let args = vec!["-c".to_string(), "echo partial; sleep 120".to_string()];
        let start = Instant::now();
        let out = runner
            .run(sh, &args, Duration::from_millis(400))
            .await
            .unwrap();

        assert!(out.timed_out);
        assert_eq!(out.exit_code, -1);
        assert!(out.stdout.contains("partial"), "stdout={:?}", out.stdout);
        // Must not wait on the surviving `sleep 120` grandchild that inherited
        // the stdout pipe: deadline + bounded drain, not the child's lifetime.
        assert!(
            start.elapsed() < Duration::from_secs(10),
            "runner must return promptly after killing the child, took {:?}",
            start.elapsed()
        );
    }

    #[tokio::test]
    async fn tokio_runner_captures_exit_code_and_streams() {
        let sh = Path::new("/bin/sh");
        if !sh.exists() {
            eprintln!("skipping: /bin/sh not available");
            return;
        }
        let out = TokioProcessRunner
            .run(
                sh,
                &["-c".to_string(), "echo o; echo e >&2; exit 7".to_string()],
                Duration::from_secs(10),
            )
            .await
            .unwrap();
        assert_eq!(out.exit_code, 7);
        assert!(!out.timed_out);
        assert_eq!(out.stdout.trim(), "o");
        assert_eq!(out.stderr.trim(), "e");
    }

    // ---------------------------------------------------------------
    // Step 6: teardown and cleanup
    // ---------------------------------------------------------------

    #[tokio::test]
    async fn end_session_for_unknown_session_is_a_no_op() {
        let runner = FakeRunner::ok();
        let exec = executor(runner.clone());
        exec.end_session(&SessionId::new()).await.unwrap();
        assert!(runner.calls().is_empty());
    }

    #[tokio::test]
    async fn end_session_stops_then_deletes_in_order() {
        let runner = FakeRunner::ok();
        let exec = executor(runner.clone());
        let ctx = ctx();
        exec.execute_command("true", &ctx).await.unwrap();
        let name = runner.calls_starting_with("run")[0][3].clone();

        exec.end_session(&ctx.session_id).await.unwrap();

        let lifecycle: Vec<String> = runner
            .calls()
            .iter()
            .filter(|c| c[0] == "stop" || c[0] == "delete")
            .map(|c| c[0].clone())
            .collect();
        assert_eq!(lifecycle, vec!["stop", "delete"]);
        assert_eq!(
            runner.calls_starting_with("stop")[0],
            vec!["stop", "--time", STOP_GRACE_SECS, &name]
        );
        assert_eq!(
            runner.calls_starting_with("delete")[0],
            vec!["delete", "--force", &name]
        );
        assert!(exec.session_to_container.is_empty());
    }

    #[tokio::test]
    async fn already_absent_container_is_teardown_success() {
        let runner = FakeRunner::new(|_| Ok(fail_output(1, "Error: container not found")));
        let exec = executor(runner);
        let name = AppleContainerExecutor::generate_container_name();
        assert!(exec.stop_and_delete(&name).await.is_ok());
    }

    #[tokio::test]
    async fn cleanup_attempts_every_session_even_when_one_fails() {
        let failing = Arc::new(StdMutex::new(String::new()));
        let target = failing.clone();
        let runner = FakeRunner::new(move |args| {
            if args[0] == "delete" && args[2] == *target.lock().unwrap() {
                Ok(fail_output(1, "resource busy"))
            } else {
                Ok(ok_output(""))
            }
        });
        let exec = executor(runner.clone());

        let mut names = Vec::new();
        for _ in 0..3 {
            names.push(exec.ensure_container(&SessionId::new()).await.unwrap());
        }
        *failing.lock().unwrap() = names[1].clone();

        let err = exec.cleanup().await.unwrap_err();
        assert!(err.to_string().contains(&names[1]), "{err}");

        let deleted: Vec<String> = runner
            .calls_starting_with("delete")
            .into_iter()
            .map(|c| c[2].clone())
            .collect();
        for name in &names {
            assert!(deleted.contains(name), "cleanup skipped {name}");
        }
        assert!(exec.session_to_container.is_empty());
    }

    #[tokio::test]
    async fn cleanup_succeeds_and_empties_the_map() {
        let runner = FakeRunner::ok();
        let exec = executor(runner);
        exec.ensure_container(&SessionId::new()).await.unwrap();
        exec.ensure_container(&SessionId::new()).await.unwrap();
        exec.cleanup().await.unwrap();
        assert!(exec.session_to_container.is_empty());
        // Idempotent: a second cleanup has nothing to do and still succeeds.
        exec.cleanup().await.unwrap();
    }

    #[tokio::test]
    async fn snapshot_operations_return_not_supported_naming_apple_container() {
        let exec = executor(FakeRunner::ok());
        let session = SessionId::new();
        let snapshot = SnapshotId::new("s", session);

        for err in [
            exec.create_snapshot(&session, "s").await.err().unwrap(),
            exec.restore_snapshot(&snapshot).await.err().unwrap(),
            exec.list_snapshots(&session).await.err().unwrap(),
            exec.delete_snapshot(&snapshot).await.err().unwrap(),
            exec.delete_session_snapshots(&session).await.err().unwrap(),
        ] {
            match err {
                RlmError::NotSupported { backend, .. } => assert_eq!(backend, BACKEND_NAME),
                other => panic!("expected NotSupported, got {other:?}"),
            }
        }
    }
}
