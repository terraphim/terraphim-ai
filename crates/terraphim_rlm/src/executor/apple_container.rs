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
//!   per-session mutex (same shape as [`super::DockerExecutor`]). Each session
//!   slot carries an explicit lifecycle state ([`SessionSlot`]): teardown marks
//!   the session `Closing` and **leaves that tombstone in the map**, so
//!   `end_session` is terminal for a session id and no concurrent or later
//!   creation can resurrect it — except once `cleanup` has raised `closing`,
//!   where teardown inserts nothing, because the map terminal cleanup drained
//!   must stay drained and creation is already refused.
//! - Lifecycle coordination is a single rule, not a protocol: **one owned
//!   lifecycle read permit is acquired before an operation can create, use or
//!   reclaim a container, and it is held — inside the owning task — until that
//!   operation and every recovery it triggers is completely finished**
//!   ([`LifecyclePermit`]). `cleanup` sets `closing` and then takes the
//!   **write** side; acquiring it is by itself the proof that every operation
//!   that started earlier has finished, because a `RwLock` write cannot be taken
//!   while any read permit is outstanding. There is no recovery registry, no
//!   join list and no bounded retry: the permit *is* the registration, and it
//!   exists from before the work starts rather than being installed after it.
//!   Lock order is always permit-then-slot.
//! - A name is tracked from before it can name anything. It is recorded as
//!   `pending` *before* `container run` is spawned and dropped only once the
//!   runtime confirms the container gone, so a failed or timed-out creation
//!   cannot leave a container this executor never heard of. While a name is
//!   unconfirmed the session creates **no replacement**, which is what keeps a
//!   later deletion failure from having to choose between orphaning a live
//!   replacement and losing the old container.
//! - Teardown failures propagate. A container whose deletion failed stays
//!   tracked (as `pending`) so `cleanup`/`Drop` retries it; a failure is never
//!   dropped from tracking or reduced to a log line.
//! - Cancellation fails **closed** ([`ExecCancelGuard`]). Dropping or aborting
//!   an execution future synchronously quarantines the session slot and raises
//!   a [`CancelSignal`], then hands the owned CLI task **and the execution's
//!   lifecycle permit** to a single recovery task. That recovery awaits the CLI
//!   task — the runner kills **and reaps** its child and stops its
//!   stdout/stderr readers before returning — and then force-deletes the
//!   container, all while still holding the permit the execution acquired
//!   before it started. The permit is moved, never released and re-taken, so
//!   there is no spawn/register window for `cleanup` to slip through. The
//!   recovery is spawned on the runtime handle the guard captured when it was
//!   armed, so this is independent of the thread the execution future is
//!   dropped on — including a plain `std::thread` with no current runtime.
//! - A command is never executed twice by this backend. If `container exec`
//!   reports the session container missing, the container is discarded and the
//!   failure is returned; whether repeating the command is safe is the caller's
//!   decision, not a guess made from guest-controlled stderr.
//! - Timeouts fail **closed**: the CLI child is killed and reaped, the session
//!   container is force-removed, and the affinity mapping is cleared so the next
//!   call gets a fresh VM. A timeout must never leave a process running inside a
//!   container that a later call would reuse. A `container exec` that fails with
//!   a host I/O error takes the same path: such an error can be raised after the
//!   child was spawned, so it is no proof that no guest process ran.
//! - Process execution goes through the [`ProcessRunner`] seam so the argv
//!   contract, concurrency, timeout and teardown behaviour can be pinned by
//!   portable tests on non-Apple hosts. A fake-runner pass is **not** evidence
//!   that the Apple runtime works; real macOS evidence is a separate gate.

use async_trait::async_trait;
use dashmap::DashMap;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};
use tokio::sync::{Mutex, RwLock};
use ulid::Ulid;

use super::{Capability, ExecutionContext, ExecutionResult, SnapshotId, ValidationResult};
use crate::config::{BackendType, KgStrictness, RlmConfig};
use crate::error::{RlmError, RlmResult};
use crate::types::SessionId;

/// Canonical backend name used in errors and diagnostics.
pub const BACKEND_NAME: &str = "apple-container";

/// Default CLI binary name, resolved from `PATH` when not overridden.
const DEFAULT_BINARY: &str = "container";

/// Prefix for every container this backend creates. Used for operator recovery
/// (`container list --all` / `container delete <name>`).
const CONTAINER_NAME_PREFIX: &str = "terraphim-rlm-";

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
/// How long a cancellation recovery waits for the cancelled CLI task to finish
/// on its own before aborting it and awaiting the abort.
///
/// Cancellation is cooperative first ([`CancelSignal`]) so the runner can kill
/// **and reap** its child and terminate its drain tasks on a known path; the
/// abort is only the backstop for a runner that ignores the signal.
const CANCEL_JOIN_GRACE: Duration = Duration::from_secs(5);

/// An owned share of the executor-wide lifecycle gate.
///
/// Owned (rather than borrowed) because the operations that must hold it —
/// notably a cancellation recovery spawned from `Drop` — outlive the stack frame
/// that acquired it. Holding one is the *only* thing that entitles a task to
/// create, use or reclaim a container; `cleanup` waits for every outstanding
/// permit by taking the write side.
type LifecyclePermit = tokio::sync::OwnedRwLockReadGuard<()>;

/// Cooperative cancellation signal handed to [`ProcessRunner::run`].
///
/// A dropped execution future cannot await anything, so cancellation is
/// *signalled* synchronously here and *carried out* by the runner (kill and reap
/// the child, terminate the drain tasks) on a task whose completion the
/// cancellation recovery observes. That is what makes "the child is gone"
/// something the backend can wait for rather than hope for.
#[derive(Debug, Default)]
pub(crate) struct CancelSignal {
    cancelled: AtomicBool,
    notify: tokio::sync::Notify,
}

impl CancelSignal {
    /// Request cancellation. Synchronous, idempotent, safe from `Drop`.
    pub(crate) fn cancel(&self) {
        self.cancelled.store(true, Ordering::Release);
        self.notify.notify_waiters();
    }

    pub(crate) fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Acquire)
    }

    /// Resolves once [`Self::cancel`] has been called (immediately if it already
    /// has). Registering the notification *before* re-reading the flag is what
    /// closes the lost-wakeup window.
    pub(crate) async fn cancelled(&self) {
        loop {
            let notified = self.notify.notified();
            if self.is_cancelled() {
                return;
            }
            notified.await;
            if self.is_cancelled() {
                return;
            }
        }
    }

    /// A signal that is never raised, for calls with no cancellation owner
    /// (probe and lifecycle CLI calls).
    pub(crate) fn never() -> &'static Self {
        static NEVER: std::sync::OnceLock<CancelSignal> = std::sync::OnceLock::new();
        NEVER.get_or_init(CancelSignal::default)
    }
}

/// `io::Error` returned by a runner whose call was cancelled.
fn cancelled_io_error() -> std::io::Error {
    std::io::Error::new(
        std::io::ErrorKind::Interrupted,
        "cancelled: the child was killed and reaped and its readers were stopped",
    )
}

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
    ///
    /// When `cancel` is raised the implementation MUST stop promptly, kill
    /// **and reap** the child, terminate any reader tasks it spawned, and
    /// return [`std::io::ErrorKind::Interrupted`]. Returning is what makes
    /// termination observable: the cancellation recovery awaits this call's
    /// task before it force-deletes the container.
    async fn run(
        &self,
        program: &Path,
        args: &[String],
        timeout: Duration,
        cancel: &CancelSignal,
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
        cancel: &CancelSignal,
    ) -> std::io::Result<CommandOutput> {
        if cancel.is_cancelled() {
            // Nothing was spawned, so there is nothing to reap.
            return Err(cancelled_io_error());
        }
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
        let mut cancelled = false;
        tokio::select! {
            // Biased so a signal raised before the first poll wins
            // deterministically rather than depending on the child's timing.
            biased;
            _ = cancel.cancelled() => {
                // Kill AND reap on the cancellation path too, so returning from
                // this call really does mean the child is gone.
                let _ = child.kill().await;
                cancelled = true;
                timed_out = false;
                exit_code = -1;
            }
            waited = tokio::time::timeout(timeout, child.wait()) => match waited {
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
            },
        }

        // Bounded drain: collect whatever is still buffered, then stop reading
        // rather than waiting on a pipe a surviving grandchild may hold open.
        // Every reader is joined after being aborted, so this call never leaves
        // a detached task behind — on cancellation the readers are stopped
        // immediately, because the caller is no longer waiting for output.
        for mut task in [stdout_task, stderr_task].into_iter().flatten() {
            if cancelled || tokio::time::timeout(DRAIN_GRACE, &mut task).await.is_err() {
                task.abort();
                let _ = task.await;
            }
        }

        if cancelled {
            return Err(cancelled_io_error());
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

/// Lifecycle state of one session's container slot.
///
/// The state is what makes teardown safe. A bare `Option<String>` cell cannot
/// distinguish "no container yet, create one" from "this session is being torn
/// down", so a concurrent [`AppleContainerExecutor::ensure_container`] holding
/// an `Arc` to a slot that teardown has already detached from the map would
/// create a container and store it where neither `end_session`, `cleanup` nor
/// `Drop` could ever find it.
#[derive(Debug)]
enum SessionSlot {
    /// The session may create or reuse a container.
    ///
    /// `bound` is the container the session executes in; `None` means nothing
    /// is bound yet (fresh session, or the previous container was abandoned).
    /// `pending` holds every name whose deletion this executor has started but
    /// not confirmed — including a name generated for a `container run` that
    /// failed or timed out, which may have created a container anyway.
    ///
    /// While `pending` is non-empty the session **may not create a
    /// replacement**: one unconfirmed container per session is already one too
    /// many, and allowing a replacement is exactly what used to make the old
    /// name unreachable when its deletion later failed.
    Active {
        bound: Option<String>,
        pending: Vec<String>,
    },
    /// Teardown has begun for this session, and the slot never leaves this
    /// state: it is the tombstone that makes `end_session` terminal for a
    /// [`SessionId`]. It is kept **in the map** until executor cleanup, so a
    /// later `ensure_container` for the same session is refused rather than
    /// silently resurrecting it with a fresh container.
    ///
    /// `pending` holds the containers whose deletion has not been *confirmed*
    /// yet: a name is added before `stop`/`delete` is attempted and removed
    /// only once the runtime reports the container gone. A failed delete
    /// therefore stays tracked, so `cleanup`/`Drop` can retry it instead of
    /// losing a live VM.
    Closing { pending: Vec<String> },
}

impl SessionSlot {
    fn new_active() -> Self {
        Self::Active {
            bound: None,
            pending: Vec::new(),
        }
    }

    fn pending(&self) -> &[String] {
        match self {
            Self::Active { pending, .. } | Self::Closing { pending } => pending,
        }
    }

    fn pending_mut(&mut self) -> &mut Vec<String> {
        match self {
            Self::Active { pending, .. } | Self::Closing { pending } => pending,
        }
    }

    /// The container the session currently executes in, if any.
    fn bound(&self) -> Option<&str> {
        match self {
            Self::Active { bound, .. } => bound.as_deref(),
            Self::Closing { .. } => None,
        }
    }

    /// Every container this slot is still responsible for — bound *and*
    /// awaiting confirmed deletion.
    #[cfg(test)]
    fn names(&self) -> Vec<String> {
        let mut names: Vec<String> = self.bound().map(str::to_string).into_iter().collect();
        names.extend(self.pending().iter().cloned());
        names
    }

    /// Whether any deletion started from this slot is still unconfirmed.
    fn has_pending(&self) -> bool {
        !self.pending().is_empty()
    }

    /// Track `name` as possibly-existing before any command that could create
    /// it runs. Called with the slot mutex held, so a name can never exist
    /// without the slot that owns it knowing about it.
    fn track_pending(&mut self, name: &str) {
        let pending = self.pending_mut();
        if !pending.iter().any(|n| n == name) {
            pending.push(name.to_string());
        }
    }

    /// Promote a confirmed-created container to the session's binding.
    fn bind_created(&mut self, name: &str) {
        if let Self::Active { bound, pending } = self {
            pending.retain(|n| n != name);
            *bound = Some(name.to_string());
        }
    }

    /// Move `name` out of the binding (only if it *is* the binding) and into
    /// `pending`, immediately before its deletion is attempted.
    ///
    /// Leaving a replacement binding alone is deliberate: while one call was
    /// failing against a dead container another may already have bound a
    /// healthy one, and unbinding that would orphan a live container.
    fn begin_pending(&mut self, name: &str) {
        if let Self::Active { bound, .. } = self
            && bound.as_deref() == Some(name)
        {
            *bound = None;
        }
        self.track_pending(name);
    }

    /// Mark the slot terminally `Closing` and return every container it is
    /// still responsible for. Nothing is dropped from tracking: the names stay
    /// in `pending` until the runtime confirms each one gone.
    fn close_taking_names(&mut self) -> Vec<String> {
        let mut pending = std::mem::take(self.pending_mut());
        if let Self::Active { bound, .. } = self
            && let Some(name) = bound.take()
            && !pending.iter().any(|n| n == &name)
        {
            pending.push(name);
        }
        *self = Self::Closing {
            pending: pending.clone(),
        };
        pending
    }

    /// Drop `name` from tracking. Called only after the runtime confirmed the
    /// container gone.
    fn forget_confirmed(&mut self, name: &str) {
        if let Self::Active { bound, .. } = self
            && bound.as_deref() == Some(name)
        {
            *bound = None;
        }
        self.pending_mut().retain(|n| n != name);
    }
}

/// One session's slot: the async-guarded lifecycle state plus a flag that can
/// be raised **without awaiting**.
///
/// The flag exists because cancellation is synchronous. When an execution
/// future is dropped, the guard that fails it closed runs in `Drop` and cannot
/// await the state mutex, yet the session must be unusable *before* the drop
/// returns — otherwise the very next call could reuse a container that may
/// still be running the cancelled workload. Raising an atomic bool is the one
/// thing `Drop` can do synchronously, and `ensure_container` reads it under the
/// state mutex, so it can never observe a cleared flag together with a stale
/// binding.
#[derive(Debug)]
struct SessionCell {
    state: Mutex<SessionSlot>,
    /// Raised synchronously by [`ExecCancelGuard`] on cancellation; cleared
    /// only by the cleanup watcher, and only once the container it bound has
    /// been confirmed deleted.
    unusable: AtomicBool,
}

impl SessionCell {
    fn new_active() -> Arc<Self> {
        Arc::new(Self {
            state: Mutex::new(SessionSlot::new_active()),
            unusable: AtomicBool::new(false),
        })
    }

    fn is_unusable(&self) -> bool {
        self.unusable.load(Ordering::Acquire)
    }

    fn mark_unusable(&self) {
        self.unusable.store(true, Ordering::Release);
    }

    fn clear_unusable(&self) {
        self.unusable.store(false, Ordering::Release);
    }
}

/// Everything a recovery needs to reach without borrowing the executor.
///
/// Cancellation recovery runs on its own task, so it cannot hold a `&self`
/// borrow of the executor; it holds an `Arc` of this instead. Keeping the map,
/// the gate and the closing flag together is also what lets recovery use
/// *exactly* the same ordering rules as the executor's own paths:
/// permit-then-slot, retrack-on-failure, never touch a detached cell.
struct LifecycleState {
    session_to_container: DashMap<SessionId, Arc<SessionCell>>,
    /// Executor-wide lifecycle gate.
    ///
    /// Every operation that may create, use or reclaim a container acquires an
    /// owned **read** permit ([`LifecyclePermit`]) *before* it starts and holds
    /// it, in the task that owns the work, until that work and every recovery
    /// it triggers has finished. `cleanup` takes the **write** side, and taking
    /// it is the whole proof: a write cannot be acquired while a single read
    /// permit is outstanding, so by the time `cleanup` holds it, every
    /// execution, creation, `end_session` and recovery that started earlier has
    /// run to completion. Anything starting afterwards blocks on the gate and
    /// then observes [`Self::closing`].
    ///
    /// `Arc` because the permit is owned: a cancellation recovery spawned from
    /// `Drop` outlives the frame that acquired the permit.
    ///
    /// Lock order is always permit-then-slot; nothing ever takes the gate while
    /// holding a slot mutex, and no task that holds a permit ever acquires a
    /// second one, so the two cannot deadlock.
    lifecycle: Arc<RwLock<()>>,
    /// Set once by `cleanup` and never cleared: `cleanup` is terminal shutdown,
    /// so no new session container may be created after it starts.
    closing: AtomicBool,
}

impl LifecycleState {
    fn new() -> Arc<Self> {
        Arc::new(Self {
            session_to_container: DashMap::new(),
            lifecycle: Arc::new(RwLock::new(())),
            closing: AtomicBool::new(false),
        })
    }

    /// Acquire an owned lifecycle read permit.
    ///
    /// Must be called **before** the operation it covers begins — before
    /// `container run`, before `container exec` — never after the work has
    /// already happened, which is what used to let `cleanup` overtake a
    /// recovery that had not yet asked for the gate.
    async fn permit(&self) -> LifecyclePermit {
        self.lifecycle.clone().read_owned().await
    }

    /// Re-attach `cell` to the map if it is not there.
    ///
    /// Only ever called while holding a permit, so `cleanup` cannot be draining
    /// concurrently and no recovery can restore a name into a cell that
    /// terminal cleanup has already detached: cleanup's drain happens strictly
    /// after every permit-holding operation has finished.
    fn retrack(&self, session_id: SessionId, cell: &Arc<SessionCell>) {
        self.session_to_container
            .entry(session_id)
            .or_insert_with(|| cell.clone());
    }

    /// Unbind `name` from its session, force-delete it, and re-track it if the
    /// deletion failed.
    ///
    /// This is the single recovery path shared by the elapsed-time timeout, the
    /// vanished-container case, a runner I/O error, a panicked exec task and
    /// cancellation, so all five have identical tracking guarantees:
    ///
    /// - the name is in `pending` before the delete is attempted, so it is
    ///   tracked even if this task dies;
    /// - `ensure_container` refuses to create a replacement while it is
    ///   pending, so a failed deletion can never be displaced by a new
    ///   container;
    /// - the cell is (re-)inserted into the map, so nothing is restored into a
    ///   detached cell; and
    /// - `cleanup` cannot drain until this returns, because the caller holds
    ///   the lifecycle permit it acquired **before the execution started** for
    ///   the whole of this call.
    ///
    /// The permit is passed in rather than acquired here: acquiring it at this
    /// point would be too late — the execution it belongs to has already run,
    /// so `cleanup` could have drained and returned in between.
    ///
    /// `clear_unusable_on_success` is set by cancellation recovery only: the
    /// quarantine that the cancellation `Drop` raised is lifted exactly when
    /// the container it quarantined is confirmed gone.
    async fn recover_container(
        &self,
        cli: Cli<'_>,
        session_id: SessionId,
        cell: Arc<SessionCell>,
        name: &str,
        clear_unusable_on_success: bool,
        _permit: &LifecyclePermit,
    ) {
        cell.state.lock().await.begin_pending(name);
        self.retrack(session_id, &cell);

        let deleted = force_delete_with(cli.runner, cli.binary, name).await;
        let mut guard = cell.state.lock().await;
        match deleted {
            Ok(()) => {
                guard.forget_confirmed(name);
                if clear_unusable_on_success && !guard.has_pending() {
                    // Cleared under the slot mutex, which `ensure_container`
                    // also holds when it reads the flag, so no caller can see a
                    // cleared flag together with an unconfirmed container.
                    cell.clear_unusable();
                }
            }
            Err(e) => log::warn!(
                "apple-container: failed to destroy {name}: {e}; it stays tracked for \
                 cleanup to retry and this session cannot create a replacement until it is gone"
            ),
        }
    }
}

/// The process seam a recovery needs, borrowed from whoever owns it.
///
/// Borrowed rather than cloned because a recovery spawned from `Drop` already
/// owns its own `Arc`/`PathBuf` copies, and the executor's paths can lend theirs
/// directly.
#[derive(Clone, Copy)]
struct Cli<'a> {
    runner: &'a Arc<dyn ProcessRunner>,
    binary: &'a Path,
}

/// Executes code in per-session Apple Container VMs via the `container` CLI.
pub struct AppleContainerExecutor {
    runner: Arc<dyn ProcessRunner>,
    binary: PathBuf,
    image: String,
    cpus: String,
    memory: String,
    state: Arc<LifecycleState>,
    capabilities: Vec<Capability>,
    validator: Option<Arc<crate::validator::KnowledgeGraphValidator>>,
    kg_strictness: KgStrictness,
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

/// Refusal to create a session container because teardown has begun.
///
/// Reported as a backend init failure rather than a guest exit code: no guest
/// command ran, and the caller must not read it as "the command failed".
fn closing_error(reason: impl std::fmt::Display) -> RlmError {
    init_failed(format!(
        "refusing to create a session container: {reason}. \
         Teardown is in progress, so a container created now could not be \
         reclaimed by end_session/cleanup."
    ))
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

/// Whether a failed `container exec` *looks like* the session container is
/// gone, rather than the guest command having failed.
///
/// **This is a heuristic, not provenance.** Guest code owns stderr byte for
/// byte, and the container name is disclosed to the caller in every
/// `ExecutionResult`'s `container` metadata, so a caller can echo it back and
/// make this predicate fire at will. Nothing here is treated as proof.
///
/// What bounds the damage is the *response*, not the detection: a match only
/// discards the session's own container and returns
/// [`RlmError::ExecutionFailed`] to the caller. The command is never re-run, so
/// the worst a forged message can achieve is losing the forger's own session
/// state — an effect the guest could produce anyway by killing its own
/// processes. No non-idempotent action can be replayed by tripping it.
///
/// The name requirement is still worth keeping because it is what stops
/// ordinary guest failures (`bash: frobnicate: command not found`, exit 127)
/// from needlessly destroying a healthy session.
fn exec_reports_container_missing(output: &CommandOutput, name: &str) -> bool {
    let name_lc = name.to_lowercase();
    output.stderr.lines().map(str::trim).any(|line| {
        let line_lc = line.to_lowercase();
        line_lc.contains(&name_lc)
            && (line_lc.contains("not found")
                || line_lc.contains("no such container")
                || line_lc.contains("does not exist"))
    })
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
            cpus: config.apple_container_cpus.to_string(),
            memory: config.apple_container_memory.clone(),
            state: LifecycleState::new(),
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
            kg_strictness: config.kg_strictness,
            platform_supported: platform_supported(),
        }
    }

    /// [`Self::ensure_container`], dropping the cell. Test-only convenience:
    /// production callers need the cell to fail cancellation closed.
    #[cfg(test)]
    async fn ensure_container_name(&self, session_id: &SessionId) -> RlmResult<String> {
        let permit = self.state.permit().await;
        self.ensure_container(session_id, &permit)
            .await
            .map(|(name, _)| name)
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

    /// This executor's process seam, for the shared recovery path.
    fn cli(&self) -> Cli<'_> {
        Cli {
            runner: &self.runner,
            binary: &self.binary,
        }
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
        self.runner
            .run(&self.binary, &argv, timeout, CancelSignal::never())
            .await
    }

    async fn run_cli_owned(
        &self,
        args: Vec<String>,
        timeout: Duration,
    ) -> std::io::Result<CommandOutput> {
        self.runner
            .run(&self.binary, &args, timeout, CancelSignal::never())
            .await
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
            self.cpus.clone(),
            "--memory".to_string(),
            self.memory.clone(),
            "--cap-drop".to_string(),
            "ALL".to_string(),
            self.image.clone(),
            "sleep".to_string(),
            "infinity".to_string(),
        ]
    }

    /// Resolve (creating at most once) the container bound to `session_id`.
    ///
    /// Refuses to create in four cases, each of which would otherwise produce a
    /// container no teardown path could reach, or hand back a container whose
    /// state is unknown:
    ///
    /// 1. the executor is shutting down (`cleanup` has started);
    /// 2. this session's slot is [`SessionSlot::Closing`] — `end_session`
    ///    marked it under the same mutex and, crucially, left the tombstone in
    ///    the map, so neither a caller holding a stale `Arc` nor a fresh lookup
    ///    can resurrect the session;
    /// 3. an execution future bound to this session was cancelled and the
    ///    container it used has not yet been confirmed deleted
    ///    ([`SessionCell::unusable`]); or
    /// 4. the slot still has an unconfirmed deletion pending — a recovery whose
    ///    `delete` failed, or a `container run` that failed and may have left a
    ///    container behind. **No replacement is created while an older name is
    ///    unconfirmed**, so a later failure can never find the slot occupied
    ///    and be forced to choose between orphaning a live replacement and
    ///    losing the old container.
    ///
    /// Returns the container name **and** the cell it is bound to, so the
    /// caller can fail that exact session closed on cancellation without a
    /// second lookup (which could race a rebind).
    ///
    /// The lifecycle permit is supplied by the caller and covers the whole
    /// operation the container is created *for*, not just the creation: an
    /// execution acquires it before `container exec` is launched and keeps it
    /// through recovery, so `cleanup` cannot slip between the two.
    async fn ensure_container(
        &self,
        session_id: &SessionId,
        _permit: &LifecyclePermit,
    ) -> RlmResult<(String, Arc<SessionCell>)> {
        if self.state.closing.load(Ordering::Acquire) {
            return Err(closing_error(
                "executor is shutting down (cleanup has started)",
            ));
        }

        let cell = self
            .state
            .session_to_container
            .entry(*session_id)
            .or_insert_with(SessionCell::new_active)
            .clone();

        let mut guard = cell.state.lock().await;
        // Read under the state mutex: the cancellation recovery clears the flag
        // while holding it, so no caller can see "usable" together with a
        // binding to a container whose deletion is still outstanding.
        if cell.is_unusable() {
            return Err(closing_error(format!(
                "session {session_id} had an execution cancelled and its container \
                 is still being destroyed"
            )));
        }
        if matches!(&*guard, SessionSlot::Closing { .. }) {
            return Err(closing_error(format!(
                "session {session_id} has been ended (end_session is terminal for a session id)"
            )));
        }
        if let Some(name) = guard.bound() {
            let name = name.to_string();
            drop(guard);
            return Ok((name, cell));
        }
        if guard.has_pending() {
            return Err(closing_error(format!(
                "session {session_id} still has container(s) awaiting confirmed deletion ({}); \
                 no replacement is created until the runtime reports them gone, so nothing this \
                 backend created can become untracked. Retry after cleanup, or recover with \
                 `container delete --force <name>`",
                guard.pending().join(", ")
            )));
        }
        // Held across creation: the name is tracked as `pending` *before*
        // `container run` can create anything, so a failed or timed-out run
        // cannot leave a container this executor has never heard of.
        let name = self.create_container(&mut guard).await?;
        drop(guard);
        Ok((name, cell))
    }

    /// Create one container and bind it into `slot`.
    ///
    /// The generated name is recorded as `pending` before `container run` is
    /// spawned, so every name that could name a real container is tracked from
    /// the instant it could exist. On failure the container is force-deleted
    /// and the name is dropped from tracking **only** if the runtime confirms
    /// it gone; otherwise it stays pending, which both keeps it reclaimable by
    /// `cleanup` and blocks a replacement for this session.
    async fn create_container(&self, slot: &mut SessionSlot) -> RlmResult<String> {
        let name = Self::generate_container_name();
        slot.track_pending(&name);

        let spawned = self
            .run_cli_owned(self.create_argv(&name), LIFECYCLE_TIMEOUT)
            .await;
        let output = match spawned {
            Ok(output) => output,
            Err(e) => {
                // The CLI never started, so no container exists — but proving
                // that is the delete's job, not an assumption.
                self.settle_failed_create(slot, &name).await;
                return Err(init_failed(format!("failed to spawn `container run`: {e}")));
            }
        };

        if !output.is_success() {
            self.settle_failed_create(slot, &name).await;
            return Err(init_failed(format!(
                "`container run` failed for {name} (exit {}{}): {}",
                output.exit_code,
                if output.timed_out { ", timed out" } else { "" },
                first_line(&output.stderr, &output.stdout)
            )));
        }
        // The name we generated is authoritative; stdout may carry an id, a
        // progress log, or nothing at all depending on CLI version.
        slot.bind_created(&name);
        Ok(name)
    }

    /// Force-delete a container whose creation failed, dropping it from
    /// tracking only once the runtime confirms it gone.
    async fn settle_failed_create(&self, slot: &mut SessionSlot, name: &str) {
        match self.force_delete(name).await {
            Ok(()) => slot.forget_confirmed(name),
            Err(e) => log::warn!(
                "apple-container: `container run` for {name} failed and the container could not \
                 be removed either ({e}); it stays tracked for cleanup to retry and this session \
                 creates no replacement until it is confirmed gone"
            ),
        }
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
    /// Exactly **one** `container exec` is issued per call. If the CLI reports
    /// that the session's container no longer exists (removed by hand, service
    /// restarted, host slept), the dead container is discarded — unbound from
    /// the session and force-removed — and the failure is returned as
    /// [`RlmError::ExecutionFailed`].
    ///
    /// The command is deliberately **not** replayed in a fresh container. A
    /// replay would execute the caller's command twice, and the only available
    /// trigger is `container exec` stderr, which is mixed with guest-controlled
    /// output; any non-idempotent action taken before the "container missing"
    /// text appeared would then happen twice. The session is left clean and
    /// unwedged, so the caller can decide whether re-issuing this particular
    /// command is safe — that decision needs knowledge this backend does not
    /// have.
    async fn exec_in_container(
        &self,
        guest_argv: Vec<String>,
        ctx: &ExecutionContext,
    ) -> RlmResult<ExecutionResult> {
        self.exec_once(&guest_argv, ctx).await
    }

    /// One `container exec` round trip against the session's container.
    ///
    /// The round trip runs in an **owned task** guarded by [`ExecCancelGuard`],
    /// so cancelling this future (task abort, an outer timeout, caller drop,
    /// shutdown) fails closed exactly like the elapsed-time timeout does rather
    /// than silently leaving the session bound to a container that may still be
    /// running the cancelled workload.
    ///
    /// A lifecycle permit is acquired **before** the container is resolved and
    /// before `container exec` is launched, and it is not released until this
    /// execution — including any timeout / vanished-container / panic recovery,
    /// or the cancellation recovery the guard hands it to — is completely
    /// finished. That single ownership rule is what makes `cleanup` terminal:
    /// it cannot take the write side while this permit exists, so it can never
    /// overtake a recovery that has not started yet.
    async fn exec_once(
        &self,
        guest_argv: &[String],
        ctx: &ExecutionContext,
    ) -> RlmResult<ExecutionResult> {
        let permit = self.state.permit().await;
        let (name, cell) = self.ensure_container(&ctx.session_id, &permit).await?;

        let options = Self::exec_option_argv(ctx);
        let mut argv = Vec::with_capacity(guest_argv.len() + options.len() + 2);
        argv.push("exec".to_string());
        argv.extend(options);
        argv.push(name.clone());
        argv.extend_from_slice(guest_argv);

        let start = Instant::now();
        let (output, permit) = {
            let runner = self.runner.clone();
            let binary = self.binary.clone();
            let deadline = Duration::from_millis(ctx.timeout_ms);
            let signal = Arc::new(CancelSignal::default());
            let task_signal = signal.clone();
            // Owned task: dropping *this* future does not drop the runner
            // future. The guard signals it and keeps its `JoinHandle`, so the
            // CLI child is killed and reaped — and the runner's stream readers
            // terminated — on a path whose completion is *observed* rather than
            // detached.
            let task = tokio::spawn(async move {
                runner
                    .run(&binary, &argv, deadline, task_signal.as_ref())
                    .await
            });
            let mut guard = ExecCancelGuard {
                armed: true,
                // Captured here, where a runtime is guaranteed (the exec task
                // above was just spawned on it), so cancellation recovery is
                // owned by *this* runtime no matter which thread the execution
                // future is later dropped on.
                runtime: tokio::runtime::Handle::current(),
                state: self.state.clone(),
                session_id: ctx.session_id,
                cell,
                name: name.clone(),
                runner: self.runner.clone(),
                binary: self.binary.clone(),
                signal,
                exec: Some(task),
                // The guard owns the permit for as long as it is armed, and
                // hands it to the cancellation recovery it spawns. Nothing
                // between "this execution started" and "its recovery finished"
                // is ever without a permit.
                permit: Some(permit),
            };
            // Awaited *through* the guard, so a cancellation here drops the
            // guard with the task **and the permit** still owned by it, and
            // nothing is detached.
            let joined = guard
                .exec
                .as_mut()
                .expect("exec task is owned until the guard is dropped")
                .await;
            // Completion path: the guard gives the permit back to this frame,
            // which now owns the container's fate — including any recovery.
            let permit = guard.disarm();
            match joined {
                Ok(Ok(output)) => (output, permit),
                Ok(Err(e)) => {
                    // The runner reports an I/O error, and by this point the
                    // exec task has already been launched: the error may come
                    // from spawning the CLI, but it may equally come from
                    // waiting on a child that had already started. Nothing here
                    // proves no guest process ran, so this is the same
                    // unknown-guest-state condition as a timeout or a panicked
                    // exec task, and it gets the same recovery — under the
                    // permit this execution has held since before it began.
                    self.abandon_container(&ctx.session_id, &name, &permit)
                        .await;
                    return Err(RlmError::ExecutionFailed {
                        message: format!(
                            "{BACKEND_NAME}: `container exec` for {name} failed ({e}); the guest \
                             process cannot be proven not to have started, so the container has \
                             been discarded"
                        ),
                        exit_code: None,
                        stdout: None,
                        stderr: None,
                    });
                }
                Err(join_error) => {
                    // The exec task itself died (panic). The container's state
                    // is unknown, so treat it exactly like a timeout — under
                    // the permit this execution has held since before it began.
                    self.abandon_container(&ctx.session_id, &name, &permit)
                        .await;
                    return Err(RlmError::ExecutionFailed {
                        message: format!(
                            "{BACKEND_NAME}: `container exec` task for {name} failed \
                             ({join_error}); the container has been discarded"
                        ),
                        exit_code: None,
                        stdout: None,
                        stderr: None,
                    });
                }
            }
        };
        let execution_time_ms = start.elapsed().as_millis() as u64;

        if output.timed_out {
            // The guest exec process cannot be proven dead, so destroy the
            // whole session container and clear affinity. The next call gets a
            // fresh VM rather than a container with a runaway process in it.
            self.abandon_container(&ctx.session_id, &name, &permit)
                .await;
            return Ok(ExecutionResult::timeout(output.stdout, output.stderr)
                .with_execution_time(execution_time_ms)
                // A timed-out result still identifies where it ran: callers
                // and operators need the backend and the container name to
                // correlate the failure and to audit recovery.
                .with_metadata("backend", BACKEND_NAME)
                .with_metadata("container", name));
        }

        if !output.is_success() && exec_reports_container_missing(&output, &name) {
            // The container is reported gone, but "reported" is the operative
            // word, so removal is still attempted: `delete` on an absent
            // container is success, and if the report was wrong the container
            // is destroyed rather than left running and untracked.
            self.abandon_container(&ctx.session_id, &name, &permit)
                .await;
            log::warn!(
                "apple-container: session container {name} reported missing; \
                 discarded it without re-running the command"
            );
            return Err(RlmError::ExecutionFailed {
                message: format!(
                    "{BACKEND_NAME}: `container exec` reported that session container {name} no \
                     longer exists. The container has been discarded and the session unbound, so \
                     a later command starts in a fresh container. This command was NOT re-run: \
                     whether repeating it is safe is the caller's decision."
                ),
                exit_code: Some(output.exit_code),
                stdout: Some(output.stdout),
                stderr: Some(output.stderr),
            });
        }

        Ok(ExecutionResult {
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
        })
    }

    /// Unbind `name` from `session_id` **only if** it is still the container
    /// bound to that session, then force-remove `name` — under the caller's
    /// lifecycle permit, and keeping `name` tracked until the runtime confirms
    /// it gone.
    ///
    /// The compare-and-clear happens under the session's own mutex, which is
    /// what makes a stale failure safe: while one call was failing against a
    /// dead container, another may already have bound a healthy replacement,
    /// and unbinding that replacement would orphan a live container and hand
    /// the next call a third one.
    ///
    /// The permit the caller has held **since before `container exec` was
    /// launched** is what makes this coordinate with terminal cleanup:
    /// `cleanup` waits for it rather than draining the map midway through, so a
    /// deletion that fails is retried by `cleanup` instead of surviving only in
    /// a detached cell — and, crucially, `cleanup` cannot have already returned
    /// by the time this starts.
    ///
    /// The container itself is always removed. Names are ULIDs generated per
    /// creation and never reused, so this cannot reach the wrong container, and
    /// `delete --force` treats an already-absent container as success — so the
    /// call is a no-op when the container really did vanish, and closes a leak
    /// when it did not.
    async fn abandon_container(
        &self,
        session_id: &SessionId,
        name: &str,
        permit: &LifecyclePermit,
    ) {
        let cell = self
            .state
            .session_to_container
            .get(session_id)
            .map(|e| e.value().clone())
            .unwrap_or_else(SessionCell::new_active);
        self.state
            .recover_container(self.cli(), *session_id, cell, name, false, permit)
            .await;
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
        force_delete_with(&self.runner, &self.binary, name).await
    }

    /// Release every container this session is still responsible for — the one
    /// it is bound to *and* any whose earlier deletion was never confirmed.
    /// Returns the names confirmed removed, or an aggregate of the failures.
    ///
    /// **Terminal for that `SessionId`.** The slot is left in the map as a
    /// [`SessionSlot::Closing`] tombstone, so neither a caller holding a stale
    /// `Arc` nor one doing a fresh map lookup can bring the session back: both
    /// are refused by [`Self::ensure_container`]. An unknown session gets a
    /// tombstone too — otherwise an `ensure_container` that has resolved the
    /// gate but not yet inserted its entry could create a container *after*
    /// teardown reported the session gone.
    ///
    /// **Except once `cleanup` has raised `closing`**, where the tombstone is
    /// pointless and harmful: `ensure_container` is already refused by the
    /// closing check, and inserting here would repopulate the map terminal
    /// cleanup drained. A call that arrives then tears down whatever is still
    /// tracked (a survivor cleanup could not delete) and inserts nothing.
    ///
    /// Ordering is what makes this race-free:
    ///
    /// 1. an owned lifecycle **read** permit is acquired before anything else
    ///    and held for the whole call, so `cleanup` (which takes the write
    ///    side) waits for this deletion to finish instead of clearing the map
    ///    out from under it and reporting success while the delete is still
    ///    outstanding;
    /// 2. the slot is marked `Closing` **under its own mutex**, retaining every
    ///    taken name as `pending`;
    /// 3. each container is stopped and deleted; a name leaves `pending` only
    ///    once the runtime confirms it is gone. A failure keeps the name tracked
    ///    and is returned to the caller, so `cleanup` can retry and aggregate
    ///    it. Every name is attempted even if an earlier one fails.
    ///
    /// Lock order is permit-then-slot here as everywhere else.
    pub async fn release_session_container(
        &self,
        session_id: &SessionId,
    ) -> RlmResult<Vec<String>> {
        let _permit = self.state.permit().await;
        // Read *after* the permit, so this observes cleanup's flag with
        // cleanup's own ordering: a teardown that was queued behind cleanup's
        // write gate gets here only once cleanup has finished draining, and
        // must not put a cell back into the map cleanup drained. An already
        // tracked survivor — a container cleanup re-inserted because its
        // deletion failed — is still owned and torn down below; only the
        // *creation* of a new cell is refused.
        let cell = if self.state.closing.load(Ordering::Acquire) {
            match self
                .state
                .session_to_container
                .get(session_id)
                .map(|e| e.value().clone())
            {
                Some(cell) => cell,
                // Nothing tracked and nothing may be created: cleanup already
                // reclaimed everything this session owned, so teardown of an
                // absent container is success, as it is for any other
                // already-absent container.
                None => return Ok(Vec::new()),
            }
        } else {
            self.state
                .session_to_container
                .entry(*session_id)
                .or_insert_with(SessionCell::new_active)
                .clone()
        };

        // Every name the slot is responsible for, not just the bound one: a
        // recovery whose deletion failed left its container pending here, and
        // teardown owns it too.
        let names = cell.state.lock().await.close_taking_names();
        if names.is_empty() {
            return Ok(Vec::new());
        }

        let mut removed = Vec::new();
        let mut failures = Vec::new();
        for name in names {
            match self.stop_and_delete(&name).await {
                Ok(()) => {
                    cell.state.lock().await.forget_confirmed(&name);
                    removed.push(name);
                }
                Err(e) => {
                    // `pending` still holds `name`, so the container stays
                    // tracked for `cleanup`/`Drop` to retry.
                    log::warn!("apple-container: release_session_container({session_id}): {e}");
                    failures.push(format!("{name}: {e}"));
                }
            }
        }
        if failures.is_empty() {
            Ok(removed)
        } else {
            Err(RlmError::Internal {
                message: format!(
                    "{BACKEND_NAME}: end_session({session_id}) could not remove {}: {}",
                    failures.len(),
                    failures.join("; ")
                ),
            })
        }
    }

    /// Close every tracked session and return each container still awaiting
    /// confirmed deletion, with the cell it belongs to.
    ///
    /// Each slot is marked [`SessionSlot::Closing`] under its own mutex before
    /// the map is emptied, so a concurrent caller holding a slot `Arc` refuses
    /// to create rather than binding a container into a detached slot. The
    /// caller must re-insert any cell whose deletion fails: a container this
    /// backend created is never dropped from tracking while it may still exist.
    async fn drain_pending_containers(&self) -> Vec<(SessionId, Arc<SessionCell>, String)> {
        let entries: Vec<(SessionId, Arc<SessionCell>)> = self
            .state
            .session_to_container
            .iter()
            .map(|kv| (*kv.key(), kv.value().clone()))
            .collect();

        let mut pending = Vec::with_capacity(entries.len());
        for (session_id, cell) in entries {
            for name in cell.state.lock().await.close_taking_names() {
                pending.push((session_id, cell.clone(), name));
            }
        }
        self.state.session_to_container.clear();
        pending
    }
}

/// `container delete --force <name>`, treating an already-absent container as
/// success.
///
/// Free-standing rather than a method because the cancellation recovery runs on
/// its own task and cannot borrow the executor.
async fn force_delete_with(
    runner: &Arc<dyn ProcessRunner>,
    binary: &Path,
    name: &str,
) -> RlmResult<()> {
    let argv = vec![
        "delete".to_string(),
        "--force".to_string(),
        name.to_string(),
    ];
    let output = runner
        .run(binary, &argv, LIFECYCLE_TIMEOUT, CancelSignal::never())
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
        // `-c`, not `-lc`: a login shell sources the guest's profile scripts,
        // which would let image-provided rc files rewrite PATH and the
        // environment this backend passes in via `--env`, making the same
        // command behave differently per image.
        self.exec_in_container(
            vec!["bash".to_string(), "-c".to_string(), cmd.to_string()],
            ctx,
        )
        .await
    }

    async fn validate(&self, input: &str) -> Result<ValidationResult, Self::Error> {
        match self.validator.as_ref() {
            Some(validator) if !input.trim().is_empty() => {
                let vr = validator.validate(input)?;
                // The configured strictness, not a hardcoded one: the reported
                // level is what callers use to decide whether an invalid result
                // blocks execution or is merely advisory.
                Ok(ValidationResult::from_validator_result(
                    &vr,
                    self.kg_strictness,
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

    /// Terminal shutdown: tear down every tracked session container and refuse
    /// to create any more.
    ///
    /// The closing flag is raised *before* the write gate is taken, and the
    /// gate is held for the whole drain. Together that closes the insertion
    /// race: an `ensure_container` already past the flag check holds a permit,
    /// so the drain waits for it and sees the container it bound; one arriving
    /// later blocks on the gate and then observes the flag. There is no
    /// interleaving in which a created container ends up untracked.
    ///
    /// **Acquiring the write side is the entire wait.** Every execution,
    /// creation, `end_session` and recovery holds an owned read permit from
    /// before it starts until after it (and any recovery it triggers) has
    /// finished, so a write acquisition is a proof that none of them is
    /// outstanding — no registry to consult, no rounds to retry, nothing that
    /// can register itself after the check. Equally, once this returns no
    /// permit-holding operation from before it exists, so nothing can re-insert
    /// a cell into the drained map afterwards.
    ///
    /// Idempotent, and permanent — the executor is not reusable afterwards.
    /// A container whose deletion fails is **re-tracked**, so a repeated
    /// `cleanup()` (and `Drop`) retries it rather than losing a live VM.
    async fn cleanup(&self) -> Result<(), Self::Error> {
        self.state.closing.store(true, Ordering::Release);
        // Waits for every outstanding lifecycle permit — so cleanup cannot
        // return while a cancelled execution's child termination, an in-flight
        // exec, a creation or any recovery is still in flight. Recoveries never
        // take a *new* permit (they inherit their execution's), so this cannot
        // deadlock against one.
        let _gate = self.state.lifecycle.write().await;

        let pending = self.drain_pending_containers().await;
        let total = pending.len();
        let mut failures = Vec::new();
        // Every tracked resource is attempted even if an earlier one fails.
        for (session_id, cell, name) in pending {
            match self.stop_and_delete(&name).await {
                Ok(()) => cell.state.lock().await.forget_confirmed(&name),
                Err(e) => {
                    log::warn!("apple-container: cleanup failed for {name}: {e}");
                    failures.push(format!("{name}: {e}"));
                    // Still `Closing { pending: [name, ..] }`, so re-inserting
                    // cannot make the session usable again — it only keeps the
                    // survivor reachable for a retry.
                    self.state.session_to_container.insert(session_id, cell);
                }
            }
        }
        if failures.is_empty() {
            Ok(())
        } else {
            Err(RlmError::Internal {
                message: format!(
                    "{BACKEND_NAME}: cleanup failed for {}/{total} containers: {}",
                    failures.len(),
                    failures.join("; ")
                ),
            })
        }
    }

    /// Terminal teardown of one session: the container is stopped and deleted,
    /// a `Closing` tombstone is retained so the session id cannot be reused,
    /// and a deletion failure is returned to the caller instead of logged and
    /// swallowed.
    async fn end_session(&self, session_id: &SessionId) -> Result<(), Self::Error> {
        // Unknown sessions do no CLI work (but are still tombstoned), and
        // teardown of an already-absent container is success.
        self.release_session_container(session_id).await.map(|_| ())
    }
}

/// Fails one in-flight `container exec` **closed** if its future is cancelled.
///
/// `kill_on_drop(true)` on the CLI child is not enough: killing `container
/// exec` does not prove the guest process died, so a cancelled call would
/// otherwise leave the session bound to a container that may still be running
/// the abandoned workload, and the next call would happily reuse it. That is
/// the same hazard the elapsed-time timeout path handles with
/// [`AppleContainerExecutor::abandon_container`], and cancellation must handle
/// it identically.
///
/// `Drop` cannot await, so the guard splits the work by what each part needs.
/// Synchronously, before the drop returns:
///
/// 1. **Quarantine.** Raise [`SessionCell::unusable`], so every later
///    `ensure_container` — which reads the flag under the slot mutex — refuses.
///    The slot is unusable *before* any subsequent call can reach it, without
///    awaiting anything.
/// 2. **Signal cancellation.** Raise the [`CancelSignal`] the runner is
///    watching. A cooperative runner kills **and reaps** its child and
///    terminates its stdout/stderr readers, then returns.
/// 3. **Move the owned exec task *and the execution's lifecycle permit* into a
///    single recovery task.** The permit is moved into the spawned future as it
///    is constructed, so it is never released; there is no window between
///    "guard dropped" and "recovery running" in which `cleanup` could acquire
///    the write side. The task is spawned on the runtime handle captured when
///    the guard was armed, so this holds for a future dropped on a thread with
///    no current runtime exactly as it does for one dropped inside the runtime.
///
/// The recovery task is then the single owner of the whole operation:
///
/// 4. it awaits the exec task (aborting and awaiting it if the runner ignored
///    the signal past [`CANCEL_JOIN_GRACE`]), so host-child termination and
///    reader termination are *observed*, not assumed;
/// 5. it force-deletes the container, still holding the permit, with the name
///    tracked as `pending` first — so `cleanup` waits for it, a failed delete
///    stays reclaimable, and no replacement can be created meanwhile;
/// 6. only once the runtime confirms the container gone does it drop the name
///    from tracking and clear the quarantine flag, letting the session start a
///    *fresh* container.
///
/// Nothing in this path is detached and nothing needs registering: terminal
/// cleanup cannot take the write gate until this task drops the permit, which
/// happens only after step 6.
struct ExecCancelGuard {
    armed: bool,
    /// The runtime that owns the exec task, captured when the guard is armed.
    ///
    /// `Drop` may run on *any* thread — a `Send` execution future can be moved
    /// out of the runtime and dropped on a plain `std::thread`, where
    /// `Handle::try_current()` fails. Recovery must not depend on that: the
    /// handle is captured at arming time, when a runtime is guaranteed, and
    /// every spawn/join below goes through it. There is therefore no drop site
    /// on which the permit is released without an owner.
    runtime: tokio::runtime::Handle,
    state: Arc<LifecycleState>,
    session_id: SessionId,
    cell: Arc<SessionCell>,
    name: String,
    runner: Arc<dyn ProcessRunner>,
    binary: PathBuf,
    signal: Arc<CancelSignal>,
    exec: Option<tokio::task::JoinHandle<std::io::Result<CommandOutput>>>,
    /// The permit acquired before `container exec` was launched. Handed to the
    /// recovery task on cancellation, or back to the execution frame by
    /// [`Self::disarm`] on completion.
    permit: Option<LifecyclePermit>,
}

impl ExecCancelGuard {
    /// The execution completed (successfully or not) on a path that already
    /// owns the container's fate, so cancellation handling must not fire.
    ///
    /// Returns the lifecycle permit to the completing frame, which keeps it
    /// through timeout / vanished-container / panic recovery.
    fn disarm(&mut self) -> LifecyclePermit {
        self.armed = false;
        self.permit
            .take()
            .expect("the guard owns the permit until it is disarmed or dropped")
    }
}

impl Drop for ExecCancelGuard {
    fn drop(&mut self) {
        if !self.armed {
            return;
        }
        // (1) Synchronous: no later call may reuse this container.
        self.cell.mark_unusable();
        // (2) Synchronous: tell the runner to kill and reap its child and stop
        // its readers. Cooperative, so the runner can do it on a path whose
        // completion the recovery below can await.
        self.signal.cancel();

        let exec = self.exec.take();
        // Taken, not re-acquired: the permit this execution has held since
        // before `container exec` started is *moved* into the recovery future
        // below, so `cleanup` sees one continuously outstanding permit rather
        // than a gap.
        let permit = self
            .permit
            .take()
            .expect("the guard owns the permit until it is disarmed or dropped");
        let runner = self.runner.clone();
        let binary = self.binary.clone();
        let name = self.name.clone();
        let cell = self.cell.clone();
        let state = self.state.clone();
        let session_id = self.session_id;
        // (3) One owned recovery task for the whole operation, carrying the
        // permit so terminal cleanup necessarily waits for it.
        //
        // Spawned through the handle captured when the guard was armed, never
        // through `Handle::try_current()`: this `drop` can run on a thread with
        // no current runtime (a `Send` execution future moved out of the
        // runtime and dropped there), and on such a thread `try_current` would
        // leave nobody to join the exec task or delete the container. The
        // captured handle makes the owner the same either way.
        self.runtime.spawn(async move {
            log::warn!(
                "apple-container: execution against {name} was cancelled; terminating the CLI \
                 child and destroying the container, because the guest process cannot be proven \
                 dead"
            );
            // (4) Observe host-child and reader termination before touching the
            // container: the runner returns only after it has killed and reaped
            // its child and stopped its stdout/stderr readers.
            if let Some(mut exec) = exec {
                match tokio::time::timeout(CANCEL_JOIN_GRACE, &mut exec).await {
                    Ok(Ok(_)) => {}
                    Ok(Err(join_error)) => log::warn!(
                        "apple-container: cancelled `container exec` task for {name} ended \
                         abnormally: {join_error}"
                    ),
                    Err(_) => {
                        // The runner ignored the signal. Abort and *await* the
                        // abort, so the runner future is dropped (killing the
                        // child via kill_on_drop) before the container is
                        // deleted, and no task is left running unobserved.
                        log::warn!(
                            "apple-container: cancelled `container exec` task for {name} did not \
                             finish within {CANCEL_JOIN_GRACE:?}; aborting it"
                        );
                        exec.abort();
                        let _ = exec.await;
                    }
                }
            }
            // (5)/(6) Shared recovery: track-then-delete, retrack on failure,
            // and lift the quarantine only on confirmed deletion — all under
            // the inherited permit, which is released only when this future
            // ends.
            state
                .recover_container(
                    Cli {
                        runner: &runner,
                        binary: &binary,
                    },
                    session_id,
                    cell,
                    &name,
                    true,
                    &permit,
                )
                .await;
        });
    }
}

impl Drop for AppleContainerExecutor {
    fn drop(&mut self) {
        let entries: Vec<_> = self
            .state
            .session_to_container
            .iter()
            .map(|kv| kv.value().clone())
            .collect();
        self.state.session_to_container.clear();
        if entries.is_empty() {
            return;
        }

        // Names are only reachable behind async mutexes, so resolve them with
        // try_lock: Drop must not block and must not claim success it cannot
        // deliver. Correctness relies on explicit end_session()/cleanup().
        let names: Vec<String> = entries
            .iter()
            .filter_map(|e| Some(e.state.try_lock().ok()?.close_taking_names()))
            .flatten()
            .collect();

        // Warned unconditionally and *before* any cleanup is attempted: every
        // path through this Drop is best effort, so an operator must be told to
        // check for residue whether or not names were resolvable. Emitting it
        // only in the all-locked case made the far more common partial case —
        // some names spawned into a detached task whose outcome is never
        // observed — look clean in the log.
        log::warn!(
            "AppleContainerExecutor dropped with {} tracked session(s) ({} name(s) resolved, \
             {} still locked); cleanup from Drop is best effort and its outcome is not reported. \
             Prefer explicit cleanup()/end_session(). Verify with: \
             container list --all | grep {}",
            entries.len(),
            names.len(),
            entries.len() - names.len(),
            CONTAINER_NAME_PREFIX
        );

        if names.is_empty() {
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
                        if let Err(e) = force_delete_with(&runner, &binary, &name).await {
                            log::warn!("apple-container: Drop failed to remove {name}: {e}");
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
    ///
    /// It doubles as the **step controller** for the lifecycle-race tests. When
    /// a [`Steps`] controller is attached, every call is published on a channel
    /// before it runs and a call whose verb has a registered gate parks until
    /// the test releases it. That is what makes those tests deterministic: the
    /// schedule is forced by the test, not hoped for by sleeping.
    struct FakeRunner {
        calls: StdMutex<Vec<(Vec<String>, Duration)>>,
        handler: Handler,
        delay: Duration,
        /// Publishes each argv as the call starts, so a test can await the
        /// exact lifecycle point it wants to interleave at.
        events: Option<tokio::sync::mpsc::UnboundedSender<Vec<String>>>,
        /// verb -> one-shot gate. The first call with that verb parks on it;
        /// later calls with the same verb are unaffected.
        gates: StdMutex<std::collections::HashMap<String, Arc<Gate>>>,
        /// verb -> permit pool. **Every** call with that verb parks until the
        /// test hands out a permit, which is what makes a schedule with many
        /// simultaneous in-flight operations deterministic rather than a race
        /// against a one-shot gate.
        holds: StdMutex<std::collections::HashMap<String, Arc<tokio::sync::Semaphore>>>,
        /// How many calls observed the cancellation signal and returned.
        cancelled: std::sync::atomic::AtomicUsize,
    }

    /// A one-shot park point: the first call to claim it waits for `notify`.
    #[derive(Debug, Default)]
    struct Gate {
        notify: tokio::sync::Notify,
        claimed: AtomicBool,
    }

    /// Upper bound on how long a forced schedule may wait for the next CLI
    /// call. Generous, because it is a failure bound and never a timing
    /// assertion: the schedules themselves are deterministic.
    const STEP_WAIT_LIMIT: Duration = Duration::from_secs(30);

    /// Test-side handle to a [`FakeRunner`]'s step control.
    struct Steps {
        events: tokio::sync::mpsc::UnboundedReceiver<Vec<String>>,
        runner: Arc<FakeRunner>,
    }

    impl Steps {
        /// Park the next call whose verb is `verb` until [`Self::release`].
        /// Registered *before* the operation is started, so there is no window.
        fn gate(&self, verb: &str) {
            self.runner
                .gates
                .lock()
                .unwrap()
                .insert(verb.to_string(), Arc::new(Gate::default()));
        }

        /// Park **every** call whose verb is `verb` until [`Self::release_held`]
        /// hands out permits. Unlike [`Self::gate`] this is not one-shot, so a
        /// schedule with many simultaneous operations can be pinned exactly.
        fn hold(&self, verb: &str) {
            self.runner
                .holds
                .lock()
                .unwrap()
                .insert(verb.to_string(), Arc::new(tokio::sync::Semaphore::new(0)));
        }

        /// Let `count` held calls of `verb` proceed. Permits are stored, so this
        /// is safe to call before the calls actually park.
        fn release_held(&self, verb: &str, count: usize) {
            let held = self.runner.holds.lock().unwrap().get(verb).cloned();
            held.expect("no hold registered for this verb")
                .add_permits(count);
        }

        /// Let the parked call proceed. `Notify::notify_one` stores a permit, so
        /// this is safe to call before the call actually parks.
        fn release(&self, verb: &str) {
            let gate = self.runner.gates.lock().unwrap().get(verb).cloned();
            gate.expect("no gate registered for this verb")
                .notify
                .notify_one();
        }

        /// Await the start of the next call whose verb is `verb`, returning its
        /// argv. Deterministic: it consumes published events, it does not poll.
        ///
        /// Bounded so that a regression which makes the awaited call
        /// *unreachable* — a lifecycle operation blocked behind a gate it
        /// should never have needed — fails the test loudly instead of hanging
        /// the suite.
        async fn wait_for(&mut self, verb: &str) -> Vec<String> {
            let awaited = async {
                loop {
                    let argv = self
                        .events
                        .recv()
                        .await
                        .expect("runner dropped before the awaited call started");
                    if argv.first().map(|v| v == verb).unwrap_or(false) {
                        return argv;
                    }
                }
            };
            match tokio::time::timeout(STEP_WAIT_LIMIT, awaited).await {
                Ok(argv) => argv,
                Err(_) => panic!(
                    "no `container {verb}` call started within {STEP_WAIT_LIMIT:?}; the \
                     lifecycle operation that should have issued it never got to run"
                ),
            }
        }
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
                events: None,
                gates: StdMutex::new(std::collections::HashMap::new()),
                holds: StdMutex::new(std::collections::HashMap::new()),
                cancelled: std::sync::atomic::AtomicUsize::new(0),
            })
        }

        /// Runner plus a [`Steps`] controller: every call is published before it
        /// runs, and gated verbs park until the test releases them.
        fn stepped<F>(handler: F) -> (Arc<Self>, Steps)
        where
            F: Fn(&[String]) -> std::io::Result<CommandOutput> + Send + Sync + 'static,
        {
            let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
            let runner = Arc::new(Self {
                calls: StdMutex::new(Vec::new()),
                handler: Box::new(handler),
                delay: Duration::ZERO,
                events: Some(tx),
                gates: StdMutex::new(std::collections::HashMap::new()),
                holds: StdMutex::new(std::collections::HashMap::new()),
                cancelled: std::sync::atomic::AtomicUsize::new(0),
            });
            let steps = Steps {
                events: rx,
                runner: runner.clone(),
            };
            (runner, steps)
        }

        fn with_delay<F>(delay: Duration, handler: F) -> Arc<Self>
        where
            F: Fn(&[String]) -> std::io::Result<CommandOutput> + Send + Sync + 'static,
        {
            Arc::new(Self {
                calls: StdMutex::new(Vec::new()),
                handler: Box::new(handler),
                delay,
                events: None,
                gates: StdMutex::new(std::collections::HashMap::new()),
                holds: StdMutex::new(std::collections::HashMap::new()),
                cancelled: std::sync::atomic::AtomicUsize::new(0),
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

        /// How many calls returned because the cancellation signal was raised,
        /// rather than because they were aborted from outside.
        fn cancelled_calls(&self) -> usize {
            self.cancelled.load(Ordering::SeqCst)
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
            cancel: &CancelSignal,
        ) -> std::io::Result<CommandOutput> {
            self.calls.lock().unwrap().push((args.to_vec(), timeout));
            if let Some(events) = &self.events {
                let _ = events.send(args.to_vec());
            }
            // Park here if the test is holding every call with this verb.
            // Cooperative, like the gate below and like the real runner.
            let held = args
                .first()
                .and_then(|verb| self.holds.lock().unwrap().get(verb).cloned());
            if let Some(held) = held {
                tokio::select! {
                    permit = held.acquire() => {
                        if let Ok(permit) = permit {
                            permit.forget();
                        }
                    }
                    _ = cancel.cancelled() => {
                        self.cancelled.fetch_add(1, Ordering::SeqCst);
                        return Err(cancelled_io_error());
                    }
                }
            }
            // Park here if the test registered a gate for this verb and no
            // earlier call has claimed it.
            let gate = args
                .first()
                .and_then(|verb| self.gates.lock().unwrap().get(verb).cloned())
                .filter(|gate| {
                    gate.claimed
                        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
                        .is_ok()
                });
            if let Some(gate) = gate {
                // Cooperative like the real runner: a parked call must observe
                // cancellation and return, so the recovery that awaits this
                // task is not held up by the gate.
                tokio::select! {
                    _ = gate.notify.notified() => {}
                    _ = cancel.cancelled() => {
                        self.cancelled.fetch_add(1, Ordering::SeqCst);
                        return Err(cancelled_io_error());
                    }
                }
            }
            if !self.delay.is_zero() {
                tokio::time::sleep(self.delay).await;
            }
            if cancel.is_cancelled() {
                self.cancelled.fetch_add(1, Ordering::SeqCst);
                return Err(cancelled_io_error());
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

    /// The container currently bound to `session`, if any.
    ///
    /// Recovery empties the binding in place rather than removing the map
    /// entry, so `contains_key` is not the question tests should ask.
    async fn bound_container(exec: &AppleContainerExecutor, session: &SessionId) -> Option<String> {
        let cell = exec
            .state
            .session_to_container
            .get(session)
            .map(|e| e.value().clone())?;
        let guard = cell.state.lock().await;
        guard.bound().map(str::to_string)
    }

    /// Containers this session has started deleting but not confirmed gone.
    async fn pending_containers(exec: &AppleContainerExecutor, session: &SessionId) -> Vec<String> {
        let Some(cell) = exec
            .state
            .session_to_container
            .get(session)
            .map(|e| e.value().clone())
        else {
            return Vec::new();
        };
        let guard = cell.state.lock().await;
        guard.pending().to_vec()
    }

    /// Whether `session` carries a terminal teardown tombstone.
    async fn is_tombstoned(exec: &AppleContainerExecutor, session: &SessionId) -> bool {
        let Some(cell) = exec
            .state
            .session_to_container
            .get(session)
            .map(|e| e.value().clone())
        else {
            return false;
        };
        let closing = matches!(&*cell.state.lock().await, SessionSlot::Closing { .. });
        closing
    }

    /// Every container name the runner was asked to create.
    fn created_names(runner: &FakeRunner) -> Vec<String> {
        runner
            .calls_starting_with("run")
            .into_iter()
            .map(|c| c[3].clone())
            .collect()
    }

    /// Every container name the runner was asked to delete.
    fn deleted_names(runner: &FakeRunner) -> Vec<String> {
        runner
            .calls_starting_with("delete")
            .into_iter()
            .map(|c| c[2].clone())
            .collect()
    }

    /// Every container name currently reachable from the session map, in any
    /// lifecycle state — a `Closing` slot whose deletion failed still tracks its
    /// container, and that is exactly what makes a retry possible.
    async fn tracked_names(exec: &AppleContainerExecutor) -> Vec<String> {
        let entries: Vec<_> = exec
            .state
            .session_to_container
            .iter()
            .map(|kv| kv.value().clone())
            .collect();
        let mut names = Vec::new();
        for entry in entries {
            names.extend(entry.state.lock().await.names());
        }
        names
    }

    /// The teardown invariant every race test asserts: no container the backend
    /// created may end up neither reachable for cleanup nor already deleted.
    async fn assert_no_untracked_container(exec: &AppleContainerExecutor, runner: &FakeRunner) {
        let created = created_names(runner);
        let deleted = deleted_names(runner);
        let tracked = tracked_names(exec).await;
        for name in &created {
            assert!(
                tracked.contains(name) || deleted.contains(name),
                "container {name} is neither tracked nor deleted: \
                 created={created:?} deleted={deleted:?} tracked={tracked:?}"
            );
        }
    }

    /// The terminal-cleanup postcondition: once `cleanup` has returned, no
    /// recovery may re-insert a cell into the map it drained.
    ///
    /// Yielding repeatedly is what gives this teeth: any recovery task that had
    /// been left behind would be scheduled during these yields and would
    /// repopulate the map.
    async fn assert_map_stays_empty_after_cleanup(exec: &AppleContainerExecutor) {
        for _ in 0..256 {
            tokio::task::yield_now().await;
            assert!(
                exec.state.session_to_container.is_empty(),
                "a cell was re-inserted after cleanup returned: {:?}",
                exec.state
                    .session_to_container
                    .iter()
                    .map(|kv| *kv.key())
                    .collect::<Vec<_>>()
            );
        }
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
    async fn configured_cpus_and_memory_reach_the_create_argv() {
        let runner = FakeRunner::ok();
        let config = RlmConfig {
            apple_container_cpus: 4,
            apple_container_memory: "2G".to_string(),
            ..RlmConfig::minimal()
        };
        let exec = AppleContainerExecutor::with_runner(config, None, runner.clone())
            .with_platform_supported(true);
        exec.ensure_container_name(&SessionId::new()).await.unwrap();

        let argv = &runner.calls_starting_with("run")[0];
        assert_eq!(
            argv[4..8].to_vec(),
            vec!["--cpus", "4", "--memory", "2G"],
            "{argv:?}"
        );
    }

    #[tokio::test]
    async fn default_cpus_and_memory_come_from_config_defaults() {
        // Pins the argv against the documented defaults rather than against
        // constants private to this module.
        let runner = FakeRunner::ok();
        let exec = executor(runner.clone());
        exec.ensure_container_name(&SessionId::new()).await.unwrap();

        let argv = &runner.calls_starting_with("run")[0];
        assert_eq!(
            argv[4..8].to_vec(),
            vec![
                "--cpus",
                &crate::config::DEFAULT_APPLE_CONTAINER_CPUS.to_string(),
                "--memory",
                crate::config::DEFAULT_APPLE_CONTAINER_MEMORY,
            ],
            "{argv:?}"
        );
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
                exec.ensure_container_name(&session).await.unwrap()
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
        let a = exec.ensure_container_name(&SessionId::new()).await.unwrap();
        let b = exec.ensure_container_name(&SessionId::new()).await.unwrap();
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

        let err = exec.ensure_container_name(&session).await.unwrap_err();
        assert!(matches!(err, RlmError::BackendInitFailed { .. }), "{err:?}");

        // Retry must succeed with a fresh container.
        let name = exec.ensure_container_name(&session).await.unwrap();
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
        assert_eq!(argv[3], "-c");
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

        // A timeout is still an execution that happened somewhere: the result
        // must say which backend and which container, like every other result.
        assert_eq!(
            result.metadata.get("backend").map(String::as_str),
            Some(BACKEND_NAME)
        );
        assert_eq!(
            result.metadata.get("container").map(String::as_str),
            Some(first_container.as_str()),
            "{:?}",
            result.metadata
        );

        // Affinity cleared: the next execution creates a fresh container.
        assert_eq!(bound_container(&exec, &ctx.session_id).await, None);
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
    // Vanished session container: discard it, never replay the command
    // ---------------------------------------------------------------

    #[tokio::test]
    async fn vanished_container_is_discarded_and_reported_without_replaying_the_command() {
        // The container is removed out from under us (service restart, host
        // sleep, manual delete). The backend must clear the dead container and
        // report the failure — it must NOT re-run the caller's command, which
        // could repeat a non-idempotent side effect the command already had.
        let first_name = Arc::new(StdMutex::new(String::new()));
        let dead = first_name.clone();
        let runner = FakeRunner::new(move |args| {
            if args[0] == "exec" && args[1] == *dead.lock().unwrap() {
                return Ok(fail_output(
                    1,
                    &format!("Error: no such container: {}", args[1]),
                ));
            }
            Ok(ok_output(""))
        });
        let exec = executor(runner.clone());
        let ctx = ctx();

        // Establish the session, then declare its container dead.
        let name = exec.ensure_container_name(&ctx.session_id).await.unwrap();
        *first_name.lock().unwrap() = name.clone();

        let err = exec.execute_command("echo hi", &ctx).await.unwrap_err();
        match err {
            RlmError::ExecutionFailed { ref message, .. } => {
                assert!(message.contains(&name), "{message}");
                assert!(message.contains("NOT re-run"), "{message}");
            }
            other => panic!("expected ExecutionFailed, got {other:?}"),
        }

        // Exactly one exec, and no replacement container created for it.
        assert_eq!(
            runner.calls_starting_with("exec").len(),
            1,
            "the command must run at most once"
        );
        assert_eq!(created_names(&runner), vec![name.clone()]);
        // The dead container is unbound and force-removed.
        assert_eq!(bound_container(&exec, &ctx.session_id).await, None);
        assert_eq!(deleted_names(&runner), vec![name.clone()]);

        // The session is not wedged: a later, separately chosen command gets a
        // fresh container.
        let result = exec.execute_command("echo hi", &ctx).await.unwrap();
        assert!(result.is_success(), "{result:?}");
        let created = created_names(&runner);
        assert_eq!(created.len(), 2);
        assert_ne!(created[1], name);
        assert_eq!(result.metadata["container"], created[1]);
    }

    #[tokio::test]
    async fn a_persistently_missing_container_never_multiplies_executions() {
        // Every exec reports its own container missing. Each call must issue
        // exactly one exec and return an error: no retry loop, no doubling.
        let runner = FakeRunner::new(|args| {
            if args[0] == "exec" {
                Ok(fail_output(
                    1,
                    &format!("Error: no such container: {}", args[1]),
                ))
            } else {
                Ok(ok_output(""))
            }
        });
        let exec = executor(runner.clone());
        let ctx = ctx();

        for expected in 1..=3 {
            let err = exec.execute_command("echo hi", &ctx).await.unwrap_err();
            assert!(matches!(err, RlmError::ExecutionFailed { .. }), "{err:?}");
            assert_eq!(
                runner.calls_starting_with("exec").len(),
                expected,
                "one exec per caller-issued command, never more"
            );
        }
        assert_no_untracked_container(&exec, &runner).await;
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
        assert!(
            exec.state
                .session_to_container
                .contains_key(&ctx.session_id)
        );
    }

    #[test]
    fn container_missing_detection_requires_the_generated_container_name() {
        let name = AppleContainerExecutor::generate_container_name();
        let missing = |stderr: &str| exec_reports_container_missing(&fail_output(1, stderr), &name);
        assert!(missing(&format!("Error: no such container: {name}")));
        assert!(missing(&format!("{name} does not exist")));
        assert!(missing(&format!("Error: container {name} not found")));

        // An `Error:` prefix is guest-forgeable and is NOT provenance: stderr
        // belongs to the guest byte for byte.
        assert!(!missing("Error: container not found"));
        assert!(!missing("error no such container"));
        assert!(!missing("Error: does not exist"));

        // Guest output that merely contains the words must not match either.
        assert!(!missing("bash: frobnicate: command not found"));
        assert!(!missing(
            "ModuleNotFoundError: No module named 'x' (not found)"
        ));
        assert!(!missing(""));

        // Naming a *different* container is not evidence about this one.
        let other = AppleContainerExecutor::generate_container_name();
        assert!(!missing(&format!("Error: no such container: {other}")));
    }

    #[tokio::test]
    async fn forged_missing_message_without_the_name_is_a_plain_guest_failure() {
        // The whole attack: guest prints the CLI's own absence wording to get
        // its session recycled and the caller's command re-run. Without the
        // generated name it must be handed back untouched — no delete, no
        // re-execution, no unbinding.
        let runner = FakeRunner::new(|args| {
            if args[0] == "exec" {
                Ok(fail_output(
                    1,
                    "Error: no such container\nError: container not found",
                ))
            } else {
                Ok(ok_output(""))
            }
        });
        let exec = executor(runner.clone());
        let ctx = ctx();

        let result = exec.execute_command("forge", &ctx).await.unwrap();
        assert_eq!(result.exit_code, 1);
        assert_eq!(runner.calls_starting_with("exec").len(), 1, "no re-run");
        assert_eq!(runner.calls_starting_with("run").len(), 1, "no new VM");
        assert!(runner.calls_starting_with("delete").is_empty());
        assert!(bound_container(&exec, &ctx.session_id).await.is_some());
    }

    #[tokio::test]
    async fn a_caller_echoing_the_disclosed_container_name_cannot_get_a_command_replayed() {
        // The container name is not a secret: every ExecutionResult returns it
        // in `metadata["container"]`, so a caller can feed it back and make the
        // absence heuristic fire. That must cost it its own session and nothing
        // more — in particular the command must not run a second time.
        let target = Arc::new(StdMutex::new(String::new()));
        let forged = target.clone();
        let runner = FakeRunner::new(move |args| {
            if args[0] == "exec" && args[1] == *forged.lock().unwrap() {
                // Guest-authored stderr, quoting the name it learned from the
                // previous result's metadata.
                return Ok(fail_output(
                    1,
                    &format!("Error: no such container: {}", args[1]),
                ));
            }
            Ok(ok_output("ok"))
        });
        let exec = executor(runner.clone());
        let ctx = ctx();

        // First call: legitimate, and it discloses the container name.
        let first = exec.execute_command("side-effect", &ctx).await.unwrap();
        let disclosed = first.metadata["container"].clone();
        *target.lock().unwrap() = disclosed.clone();

        let err = exec.execute_command("side-effect", &ctx).await.unwrap_err();
        assert!(matches!(err, RlmError::ExecutionFailed { .. }), "{err:?}");

        // Two caller-issued commands, two execs: the forged message bought no
        // extra execution of anything.
        assert_eq!(runner.calls_starting_with("exec").len(), 2);
        assert_eq!(created_names(&runner), vec![disclosed.clone()]);
        assert_eq!(deleted_names(&runner), vec![disclosed]);
        assert_no_untracked_container(&exec, &runner).await;
    }

    #[tokio::test]
    async fn stale_missing_failure_does_not_unbind_a_replacement_container() {
        // A call fails against container A while another call has already
        // bound replacement B. Clearing "the session's container" blindly
        // would orphan B (still running, no longer tracked) and hand the next
        // call a third container.
        let runner = FakeRunner::ok();
        let exec = executor(runner.clone());
        let session = SessionId::new();

        let stale = exec.ensure_container_name(&session).await.unwrap();
        // Simulate the replacement another task would have installed.
        let replacement = AppleContainerExecutor::generate_container_name();
        {
            let entry = exec
                .state
                .session_to_container
                .get(&session)
                .map(|e| e.value().clone())
                .unwrap();
            entry.state.lock().await.bind_created(&replacement);
        }

        let permit = exec.state.permit().await;
        exec.abandon_container(&session, &stale, &permit).await;
        drop(permit);

        assert_eq!(
            bound_container(&exec, &session).await,
            Some(replacement.clone()),
            "a stale failure must not unbind the replacement"
        );
        let deleted: Vec<String> = runner
            .calls_starting_with("delete")
            .into_iter()
            .map(|c| c[2].clone())
            .collect();
        assert_eq!(
            deleted,
            vec![stale],
            "only the observed container is removed"
        );
        assert!(!deleted.contains(&replacement));
    }

    #[tokio::test]
    async fn concurrent_vanish_failures_leave_no_leaks() {
        // Both in-flight calls are told the same container is gone. Whatever
        // the interleaving, the invariant is: every container ever created is
        // either still tracked or has been deleted, and neither call is
        // silently re-run.
        let dead = Arc::new(StdMutex::new(String::new()));
        let target = dead.clone();
        let runner = FakeRunner::with_delay(Duration::from_millis(10), move |args| {
            if args[0] == "exec" && args[1] == *target.lock().unwrap() {
                return Ok(fail_output(
                    1,
                    &format!("Error: no such container: {}", args[1]),
                ));
            }
            Ok(ok_output("ok"))
        });
        let exec = Arc::new(executor(runner.clone()));
        let ctx = ctx();

        let first = exec.ensure_container_name(&ctx.session_id).await.unwrap();
        *dead.lock().unwrap() = first.clone();

        let handles: Vec<_> = (0..2)
            .map(|_| {
                let exec = exec.clone();
                let ctx = ctx.clone();
                tokio::spawn(async move { exec.execute_command("echo hi", &ctx).await })
            })
            .collect();
        for handle in futures::future::join_all(handles).await {
            // Whoever hit the dead container gets the backend error; a call
            // that happened to run in a replacement succeeds.
            let _ = handle.unwrap();
        }

        // Two callers, two execs at most: no call was replayed.
        assert!(
            runner.calls_starting_with("exec").len() <= 2,
            "no command may be executed twice: {:?}",
            runner.calls_starting_with("exec")
        );
        assert!(
            deleted_names(&runner).contains(&first),
            "the dead container must be removed"
        );
        assert_no_untracked_container(&exec, &runner).await;
        let bound = bound_container(&exec, &ctx.session_id).await;
        assert!(
            bound
                .as_ref()
                .is_none_or(|b| !deleted_names(&runner).contains(b)),
            "the bound container must not have been deleted: bound={bound:?}"
        );
    }

    // ---------------------------------------------------------------
    // Teardown races: no interleaving may produce an untracked container
    // ---------------------------------------------------------------

    #[tokio::test]
    async fn ensure_holding_a_stale_slot_refuses_to_create_after_end_session() {
        // The exact interleaving the lifecycle state exists for: a caller
        // resolves the session's slot Arc, teardown then closes and detaches
        // that slot, and only afterwards does the caller lock it. With a bare
        // Option cell it would see None, create a container, and store it where
        // nothing could find it.
        let runner = FakeRunner::ok();
        let exec = executor(runner.clone());
        let session = SessionId::new();

        let name = exec.ensure_container_name(&session).await.unwrap();
        let stale_slot = exec
            .state
            .session_to_container
            .get(&session)
            .map(|e| e.value().clone())
            .expect("session slot");

        exec.end_session(&session).await.unwrap();
        assert_eq!(deleted_names(&runner), vec![name.clone()]);

        // The tombstone is retained: both the stale Arc and a fresh lookup are
        // refused, so the session id cannot be resurrected.
        assert!(
            matches!(&*stale_slot.state.lock().await, SessionSlot::Closing { pending } if pending.is_empty())
        );
        assert!(is_tombstoned(&exec, &session).await);
        let err = exec.ensure_container_name(&session).await.unwrap_err();
        assert!(err.to_string().contains("has been ended"), "{err}");

        // And nothing was created behind teardown's back.
        assert_eq!(created_names(&runner), vec![name]);
        assert_no_untracked_container(&exec, &runner).await;
    }

    #[tokio::test]
    async fn end_session_in_flight_refuses_a_racing_ensure_and_stays_terminal() {
        // Forced schedule, not a hoped-for one: teardown is parked *inside* its
        // delete, which is exactly the window in which the old code let a
        // concurrent caller bind a replacement container to a destroyed
        // session. `ensure` is then driven to completion while teardown is
        // still in flight.
        let (runner, mut steps) = FakeRunner::stepped(|_| Ok(ok_output("")));
        let exec = Arc::new(executor(runner.clone()));
        let session = SessionId::new();
        let name = exec.ensure_container_name(&session).await.unwrap();

        steps.gate("delete");
        let ending = {
            let exec = exec.clone();
            tokio::spawn(async move { exec.end_session(&session).await })
        };
        // Teardown now owns the slot and is mid-deletion.
        let argv = steps.wait_for("delete").await;
        assert_eq!(argv[2], name);

        // A caller arriving in that exact window is refused, and no container
        // is created for it.
        let err = exec.ensure_container_name(&session).await.unwrap_err();
        assert!(
            matches!(err, RlmError::BackendInitFailed { .. }),
            "a refusal must be a backend error, not a guest failure: {err:?}"
        );
        assert_eq!(created_names(&runner), vec![name.clone()]);

        steps.release("delete");
        ending.await.unwrap().unwrap();

        // Terminal afterwards too: the tombstone survives teardown.
        let err = exec.ensure_container_name(&session).await.unwrap_err();
        assert!(err.to_string().contains("has been ended"), "{err}");
        assert!(is_tombstoned(&exec, &session).await);
        assert_eq!(created_names(&runner), vec![name.clone()]);
        assert_eq!(deleted_names(&runner), vec![name]);
        assert_no_untracked_container(&exec, &runner).await;
    }

    #[tokio::test]
    async fn cleanup_waits_for_an_in_flight_creation_and_then_deletes_it() {
        // The creation is parked inside `container run`, so the container it is
        // about to bind does not exist in the map yet. `cleanup` is then polled
        // and must be *pending* — polling proves it is blocked on the lifecycle
        // gate, where a sleep would only have hoped so.
        let (runner, mut steps) = FakeRunner::stepped(|_| Ok(ok_output("")));
        let exec = Arc::new(executor(runner.clone()));
        let session = SessionId::new();

        steps.gate("run");
        let ensuring = {
            let exec = exec.clone();
            tokio::spawn(async move { exec.ensure_container_name(&session).await })
        };
        let argv = steps.wait_for("run").await;
        let name = argv[3].clone();

        let mut cleaning = Box::pin(exec.cleanup());
        assert!(
            futures::poll!(&mut cleaning).is_pending(),
            "cleanup must not proceed while a creation holds the lifecycle gate"
        );
        assert!(
            deleted_names(&runner).is_empty(),
            "cleanup deleted something before the in-flight creation finished"
        );

        steps.release("run");
        let created = ensuring.await.unwrap().unwrap();
        assert_eq!(created, name);

        cleaning.await.unwrap();
        assert_eq!(
            deleted_names(&runner),
            vec![name],
            "cleanup must delete the container the racing creation bound"
        );
        assert!(exec.state.session_to_container.is_empty());
        assert_no_untracked_container(&exec, &runner).await;
    }

    #[tokio::test]
    async fn cleanup_waits_for_an_in_flight_end_session_deletion() {
        // `cleanup` used to be able to observe an already-`Closing` slot with no
        // bound name, clear the map and report terminal success while the
        // outstanding delete was still running.
        let (runner, mut steps) = FakeRunner::stepped(|_| Ok(ok_output("")));
        let exec = Arc::new(executor(runner.clone()));
        let session = SessionId::new();
        let name = exec.ensure_container_name(&session).await.unwrap();

        steps.gate("delete");
        let ending = {
            let exec = exec.clone();
            tokio::spawn(async move { exec.end_session(&session).await })
        };
        steps.wait_for("delete").await;

        let mut cleaning = Box::pin(exec.cleanup());
        assert!(
            futures::poll!(&mut cleaning).is_pending(),
            "cleanup must not report success while end_session is mid-deletion"
        );

        steps.release("delete");
        ending.await.unwrap().unwrap();
        cleaning.await.unwrap();

        // One delete: cleanup waited for teardown rather than racing it, and
        // teardown confirmed the container gone before cleanup drained.
        assert_eq!(deleted_names(&runner), vec![name]);
        assert!(exec.state.session_to_container.is_empty());
        assert_no_untracked_container(&exec, &runner).await;
    }

    #[tokio::test]
    async fn a_failed_end_session_delete_is_returned_and_retried_by_cleanup() {
        // First delete fails, later ones succeed.
        let first_delete = Arc::new(AtomicBool::new(false));
        let flag = first_delete.clone();
        let runner = FakeRunner::new(move |args| {
            if args[0] == "delete" && !flag.swap(true, Ordering::SeqCst) {
                Ok(fail_output(1, "resource busy"))
            } else {
                Ok(ok_output(""))
            }
        });
        let exec = executor(runner.clone());
        let session = SessionId::new();
        let name = exec.ensure_container_name(&session).await.unwrap();

        // The failure reaches the caller instead of being logged and swallowed.
        let err = exec.end_session(&session).await.unwrap_err();
        assert!(err.to_string().contains(&name), "{err}");

        // And the container it could not remove is still tracked, so cleanup
        // retries it — a failed container is never dropped from tracking.
        assert_eq!(tracked_names(&exec).await, vec![name.clone()]);
        assert!(is_tombstoned(&exec, &session).await);
        assert!(
            exec.ensure_container_name(&session).await.is_err(),
            "a failed teardown must not make the session usable again"
        );

        exec.cleanup().await.unwrap();
        assert_eq!(deleted_names(&runner), vec![name.clone(), name]);
        assert!(exec.state.session_to_container.is_empty());
    }

    #[tokio::test]
    async fn cleanup_retains_a_container_it_could_not_delete() {
        let runner = FakeRunner::new(|args| {
            if args[0] == "delete" {
                Ok(fail_output(1, "resource busy"))
            } else {
                Ok(ok_output(""))
            }
        });
        let exec = executor(runner.clone());
        let name = exec.ensure_container_name(&SessionId::new()).await.unwrap();

        let err = exec.cleanup().await.unwrap_err();
        assert!(err.to_string().contains(&name), "{err}");
        assert_eq!(
            tracked_names(&exec).await,
            vec![name],
            "a container cleanup could not remove must stay tracked for a retry"
        );
    }

    #[tokio::test]
    async fn aborting_an_execution_future_fails_the_session_closed() {
        // The cancellation contract: the moment the execution future is
        // dropped, the session's container must be unusable, the host CLI child
        // must be gone, and the container must be force-deleted.
        let (runner, mut steps) = FakeRunner::stepped(|_| Ok(ok_output("")));
        let exec = Arc::new(executor(runner.clone()));
        let ctx = ctx();

        // Park the exec so the abort lands while the command is in flight.
        steps.gate("exec");
        let running = {
            let exec = exec.clone();
            let ctx = ctx.clone();
            tokio::spawn(async move { exec.execute_command("sleep 100", &ctx).await })
        };
        steps.wait_for("exec").await;
        let name = created_names(&runner)[0].clone();

        running.abort();
        assert!(running.await.unwrap_err().is_cancelled());

        // The abandoned container is force-deleted on a bounded, owned path.
        let deleted = steps.wait_for("delete").await;
        assert_eq!(deleted, vec!["delete", "--force", &name]);

        // And the session never hands that container back: either the recovery
        // is still outstanding (refused) or it completed and the next call gets
        // a *fresh* container. Never the cancelled one.
        match exec.ensure_container_name(&ctx.session_id).await {
            Ok(fresh) => assert_ne!(fresh, name, "a cancelled container must never be reused"),
            Err(e) => assert!(
                matches!(e, RlmError::BackendInitFailed { .. }),
                "a refusal must be a backend error: {e:?}"
            ),
        }
        assert_eq!(
            runner.calls_starting_with("exec").len(),
            1,
            "the cancelled command must not be re-run"
        );

        // The CLI call ended because it observed the cancellation signal — the
        // path on which the runner kills and reaps its child — not because
        // something aborted it blind.
        assert_eq!(runner.cancelled_calls(), 1);

        // Terminal cleanup joins the recovery instead of racing it, so it can
        // only return once every container is deleted or still tracked.
        exec.cleanup().await.unwrap();
        assert!(exec.state.session_to_container.is_empty());
        assert!(deleted_names(&runner).contains(&name));
        assert_no_untracked_container(&exec, &runner).await;
    }

    #[tokio::test]
    async fn cleanup_stays_pending_until_a_cancellation_recovery_completes() {
        // Forced schedule: the cancellation recovery is parked *inside* its
        // force-delete, and `cleanup` is polled from there. Terminal cleanup
        // must be pending — the recovery still holds the execution's lifecycle
        // permit — rather than reporting success while a cancelled execution's
        // container is still being destroyed.
        let (runner, mut steps) = FakeRunner::stepped(|_| Ok(ok_output("")));
        let exec = Arc::new(executor(runner.clone()));
        let ctx = ctx();

        steps.gate("exec");
        let mut running = Box::pin(exec.execute_command("sleep 100", &ctx));
        assert!(futures::poll!(&mut running).is_pending());
        steps.wait_for("exec").await;
        let name = created_names(&runner)[0].clone();

        steps.gate("delete");
        drop(running);

        // The recovery has already observed the cancelled CLI task (the runner
        // returned on the signal) and is now parked in the force-delete.
        assert_eq!(
            steps.wait_for("delete").await,
            vec!["delete", "--force", &name]
        );
        assert_eq!(runner.cancelled_calls(), 1);

        let mut cleaning = Box::pin(exec.cleanup());
        assert!(
            futures::poll!(&mut cleaning).is_pending(),
            "cleanup must wait for an outstanding cancellation recovery's permit before draining"
        );
        assert_eq!(
            pending_containers(&exec, &ctx.session_id).await,
            vec![name.clone()],
            "the cancelled container stays tracked until the runtime confirms it gone"
        );

        steps.release("delete");
        cleaning.await.unwrap();

        // One delete: cleanup waited for the recovery rather than duplicating
        // it, and drained a map the recovery had already emptied of names.
        assert_eq!(deleted_names(&runner), vec![name]);
        assert!(exec.state.session_to_container.is_empty());
        assert_map_stays_empty_after_cleanup(&exec).await;
        assert_no_untracked_container(&exec, &runner).await;
    }

    #[tokio::test]
    async fn cleanup_started_in_the_same_tick_as_the_cancellation_drop_still_waits() {
        // The schedule that a spawn-then-register design cannot close: `cleanup`
        // begins in the *same tick* as `ExecCancelGuard::drop`, before the
        // recovery task has ever been polled — the exact point at which a
        // registry would still be empty. There is no registry now: the guard
        // moves the permit the execution has held since before `container exec`
        // into the recovery future as it constructs it, so the permit is never
        // released and `cleanup` must be pending here.
        let (runner, mut steps) = FakeRunner::stepped(|_| Ok(ok_output("")));
        let exec = executor(runner.clone());
        let ctx = ctx();

        steps.gate("exec");
        let mut running = Box::pin(exec.execute_command("sleep 100", &ctx));
        assert!(futures::poll!(&mut running).is_pending());
        steps.wait_for("exec").await;
        let name = created_names(&runner)[0].clone();

        // Park the recovery's deletion so it cannot complete on its own.
        steps.gate("delete");
        drop(running);
        // No await between the drop and the first poll of cleanup: the recovery
        // task has not run at all yet.
        let mut cleaning = Box::pin(exec.cleanup());
        assert!(
            futures::poll!(&mut cleaning).is_pending(),
            "cleanup must not proceed in the window between the cancellation drop and the \
             recovery task's first poll"
        );
        assert!(
            deleted_names(&runner).is_empty(),
            "nothing may have been deleted yet: {:?}",
            deleted_names(&runner)
        );

        // Now let the recovery run to completion.
        assert_eq!(
            steps.wait_for("delete").await,
            vec!["delete", "--force", &name]
        );
        assert!(
            futures::poll!(&mut cleaning).is_pending(),
            "cleanup must still wait while the recovery is mid-delete"
        );
        steps.release("delete");
        cleaning.await.unwrap();

        assert_eq!(runner.cancelled_calls(), 1);
        assert_eq!(deleted_names(&runner), vec![name]);
        assert!(exec.state.session_to_container.is_empty());
        assert_map_stays_empty_after_cleanup(&exec).await;
        assert_no_untracked_container(&exec, &runner).await;
    }

    /// Which completion path an execution takes into recovery.
    #[derive(Clone, Copy, Debug)]
    enum CompletionRecovery {
        /// `container exec` hit its deadline.
        Timeout,
        /// `container exec` reported the session container gone.
        Missing,
        /// The owned exec task panicked, so the container's state is unknown.
        Panic,
    }

    /// `cleanup` is queued for the write side **while `container exec` is still
    /// running**, then the exec completes into recovery.
    ///
    /// This is the schedule the old design lost: the exec held no lifecycle
    /// participation, so `cleanup` could take the write gate the instant the
    /// exec returned, drain, and return — after which the recovery would
    /// acquire the read gate and re-insert a cell into a map that had already
    /// been declared terminal. The permit is now acquired *before* the exec is
    /// launched and released only after recovery, so `cleanup` cannot overtake
    /// it, and it cannot deadlock either: recovery inherits that permit rather
    /// than asking for a new one.
    async fn cleanup_queued_during_exec_waits_for(recovery: CompletionRecovery) {
        let (runner, mut steps) = FakeRunner::stepped(move |args| {
            if args[0] != "exec" {
                return Ok(ok_output(""));
            }
            match recovery {
                CompletionRecovery::Timeout => Ok(CommandOutput {
                    exit_code: -1,
                    stdout: String::new(),
                    stderr: String::new(),
                    timed_out: true,
                }),
                CompletionRecovery::Missing => Ok(fail_output(
                    1,
                    &format!("Error: no such container: {}", args[1]),
                )),
                CompletionRecovery::Panic => panic!("deliberate exec-task panic: {recovery:?}"),
            }
        });
        let exec = Arc::new(executor(runner.clone()));
        let ctx = ctx();

        steps.gate("exec");
        let running = {
            let exec = exec.clone();
            let ctx = ctx.clone();
            tokio::spawn(async move { exec.execute_command("work", &ctx).await })
        };
        steps.wait_for("exec").await;
        let name = created_names(&runner)[0].clone();

        // Terminal cleanup queues for the write side while the exec is still in
        // flight — and before the completion path that will need recovery has
        // even been chosen.
        let mut cleaning = Box::pin(exec.cleanup());
        assert!(
            futures::poll!(&mut cleaning).is_pending(),
            "cleanup must wait for the permit an in-flight execution holds"
        );

        // Park the recovery inside its force-delete, then let the exec finish.
        steps.gate("delete");
        steps.release("exec");
        assert_eq!(
            steps.wait_for("delete").await,
            vec!["delete", "--force", &name],
            "{recovery:?} must recover the container"
        );

        // The assertion this whole test exists for: the exec has completed and
        // recovery has started, and cleanup is *still* blocked.
        assert!(
            futures::poll!(&mut cleaning).is_pending(),
            "cleanup must not drain or return between {recovery:?} completion and the end of \
             its recovery"
        );

        steps.release("delete");
        // Timeout returns Ok(timed_out), missing/panic return Err; all three
        // must have recovered the container.
        let _ = running
            .await
            .expect("the execution task must not be aborted");
        cleaning.await.unwrap();

        assert_eq!(deleted_names(&runner), vec![name], "{recovery:?}");
        assert!(exec.state.session_to_container.is_empty(), "{recovery:?}");
        assert_map_stays_empty_after_cleanup(&exec).await;
        assert_no_untracked_container(&exec, &runner).await;
    }

    #[tokio::test]
    async fn cleanup_queued_during_exec_waits_for_timeout_recovery() {
        cleanup_queued_during_exec_waits_for(CompletionRecovery::Timeout).await;
    }

    #[tokio::test]
    async fn cleanup_queued_during_exec_waits_for_vanished_container_recovery() {
        cleanup_queued_during_exec_waits_for(CompletionRecovery::Missing).await;
    }

    #[tokio::test]
    async fn cleanup_queued_during_exec_waits_for_panicked_exec_recovery() {
        cleanup_queued_during_exec_waits_for(CompletionRecovery::Panic).await;
    }

    #[tokio::test]
    async fn cleanup_waits_for_far_more_cancellations_than_the_old_bounded_rounds() {
        // The old design gave up joining after a fixed number of rounds and
        // proceeded anyway, so enough paced cancellations could outlast it.
        // There are no rounds now — cleanup takes the write side exactly once,
        // and every one of these recoveries holds a read permit — so a count
        // well past the old bound is simply not special.
        const CANCELLATIONS: usize = 12;

        let (runner, mut steps) = FakeRunner::stepped(|_| Ok(ok_output("")));
        let exec = executor(runner.clone());

        // Every exec and every delete parks, so all 12 cancellations are
        // genuinely simultaneous rather than serialised by a one-shot gate.
        steps.hold("exec");
        steps.hold("delete");

        // Declared before `running` so the borrowed contexts outlive the
        // futures that borrow them.
        let contexts: Vec<ExecutionContext> = (0..CANCELLATIONS).map(|_| ctx()).collect();
        let mut running = Vec::new();
        for ctx in &contexts {
            let mut future = Box::pin(exec.execute_command("sleep 100", ctx));
            assert!(futures::poll!(&mut future).is_pending());
            // Yields to the runtime, so the owned exec task actually starts and
            // publishes before the next session is set up.
            steps.wait_for("exec").await;
            running.push(future);
        }
        let names = created_names(&runner);
        assert_eq!(names.len(), CANCELLATIONS);

        // Cancel all of them, then start cleanup in the same tick.
        drop(running);
        let mut cleaning = Box::pin(exec.cleanup());
        assert!(
            futures::poll!(&mut cleaning).is_pending(),
            "cleanup must wait for every cancellation recovery, however many there are"
        );

        // Drive each recovery up to its parked force-delete.
        for _ in 0..CANCELLATIONS {
            steps.wait_for("delete").await;
        }
        assert!(
            futures::poll!(&mut cleaning).is_pending(),
            "cleanup must still be waiting with {CANCELLATIONS} recoveries mid-delete"
        );

        steps.release_held("delete", CANCELLATIONS);
        cleaning.await.unwrap();

        assert_eq!(runner.cancelled_calls(), CANCELLATIONS);
        let mut deleted = deleted_names(&runner);
        deleted.sort();
        let mut expected = names;
        expected.sort();
        assert_eq!(
            deleted, expected,
            "every cancelled container must be deleted"
        );
        assert!(exec.state.session_to_container.is_empty());
        assert_map_stays_empty_after_cleanup(&exec).await;
        assert_no_untracked_container(&exec, &runner).await;
    }

    #[tokio::test]
    async fn nothing_can_reinsert_a_cell_after_cleanup_returns() {
        // The postcondition the terminal contract reduces to: once `cleanup`
        // has returned, the map it drained stays drained. Nothing that held a
        // permit before cleanup can still exist (cleanup waited for all of
        // them), and nothing arriving afterwards gets past the closing check —
        // which is read *before* the map entry would be created.
        let (runner, mut steps) = FakeRunner::stepped(|_| Ok(ok_output("")));
        let exec = executor(runner.clone());
        let cancelled_ctx = ctx();

        // One cancellation recovery and one timed-out execution are both
        // outstanding when cleanup starts, so both would have to be waited for.
        steps.gate("exec");
        let mut running = Box::pin(exec.execute_command("sleep 100", &cancelled_ctx));
        assert!(futures::poll!(&mut running).is_pending());
        steps.wait_for("exec").await;
        let cancelled_name = created_names(&runner)[0].clone();
        drop(running);

        exec.cleanup().await.unwrap();

        assert!(deleted_names(&runner).contains(&cancelled_name));
        assert!(exec.state.session_to_container.is_empty());
        assert_map_stays_empty_after_cleanup(&exec).await;

        // A caller arriving after cleanup is refused *before* it can create a
        // map entry, so a refusal cannot repopulate the map either.
        let err = exec.execute_command("echo hi", &ctx()).await.unwrap_err();
        assert!(err.to_string().contains("shutting down"), "{err}");
        assert!(
            exec.state.session_to_container.is_empty(),
            "a refused caller must not insert a session cell"
        );
        assert_no_untracked_container(&exec, &runner).await;
    }

    #[tokio::test]
    async fn a_runner_io_error_after_exec_started_destroys_the_container() {
        // A `ProcessRunner` error is not proof that no guest process ran — it
        // can come from waiting on a child that already started — so it is the
        // same unknown-guest-state condition as a timeout, and the container
        // must be destroyed rather than left bound and reusable.
        let runner = FakeRunner::new(|args| {
            if args[0] == "exec" {
                Err(std::io::Error::other(
                    "exec pipe broke after the child started",
                ))
            } else {
                Ok(ok_output(""))
            }
        });
        let exec = executor(runner.clone());
        let ctx = ctx();

        let err = exec.execute_command("work", &ctx).await.unwrap_err();
        let name = created_names(&runner)[0].clone();
        assert!(matches!(err, RlmError::ExecutionFailed { .. }), "{err:?}");
        assert!(err.to_string().contains(&name), "{err}");

        // Recovered exactly like a timeout: unbound, force-deleted, untracked.
        assert_eq!(deleted_names(&runner), vec![name.clone()]);
        assert_eq!(bound_container(&exec, &ctx.session_id).await, None);
        assert!(tracked_names(&exec).await.is_empty());
        assert_no_untracked_container(&exec, &runner).await;

        // And the container is never handed back: the next command on this
        // session gets a fresh one.
        let second = exec.execute_command("work", &ctx).await.unwrap_err();
        assert!(
            matches!(second, RlmError::ExecutionFailed { .. }),
            "{second:?}"
        );
        let created = created_names(&runner);
        assert_eq!(created.len(), 2, "{created:?}");
        assert_ne!(
            created[1], name,
            "a container whose exec failed with an I/O error must never be reused"
        );
    }

    #[tokio::test]
    async fn a_failed_io_error_recovery_delete_blocks_reuse_and_stays_tracked() {
        // The other half: when the recovery's delete fails, the container stays
        // tracked (it may still be alive and running the guest command) and no
        // replacement may be created for that session.
        let runner = FakeRunner::new(|args| match args[0].as_str() {
            "exec" => Err(std::io::Error::other(
                "exec pipe broke after the child started",
            )),
            "delete" => Ok(fail_output(1, "resource busy")),
            _ => Ok(ok_output("")),
        });
        let exec = executor(runner.clone());
        let ctx = ctx();

        exec.execute_command("work", &ctx).await.unwrap_err();
        let name = created_names(&runner)[0].clone();
        assert_eq!(tracked_names(&exec).await, vec![name.clone()]);

        let err = exec.execute_command("work", &ctx).await.unwrap_err();
        assert!(
            err.to_string().contains("awaiting confirmed deletion"),
            "{err}"
        );
        assert_eq!(
            created_names(&runner),
            vec![name.clone()],
            "no replacement while the I/O-failed container is unconfirmed"
        );
        assert_no_untracked_container(&exec, &runner).await;
    }

    #[tokio::test]
    async fn cleanup_cannot_overtake_an_io_error_recovery() {
        // The permit rule holds on this path too: `cleanup` queues for the
        // write side while the exec is in flight, the exec then fails with an
        // I/O error, and cleanup must still be pending until the recovery it
        // triggers has finished deleting the container.
        let (runner, mut steps) = FakeRunner::stepped(|args| {
            if args[0] == "exec" {
                Err(std::io::Error::other(
                    "exec pipe broke after the child started",
                ))
            } else {
                Ok(ok_output(""))
            }
        });
        let exec = Arc::new(executor(runner.clone()));
        let ctx = ctx();

        steps.gate("exec");
        let running = {
            let exec = exec.clone();
            let ctx = ctx.clone();
            tokio::spawn(async move { exec.execute_command("work", &ctx).await })
        };
        steps.wait_for("exec").await;
        let name = created_names(&runner)[0].clone();

        let mut cleaning = Box::pin(exec.cleanup());
        assert!(
            futures::poll!(&mut cleaning).is_pending(),
            "cleanup must wait for the permit an in-flight execution holds"
        );

        // Park the recovery inside its force-delete, then let the exec fail.
        steps.gate("delete");
        steps.release("exec");
        assert_eq!(
            steps.wait_for("delete").await,
            vec!["delete", "--force", &name],
            "an I/O error must recover the container"
        );
        assert!(
            futures::poll!(&mut cleaning).is_pending(),
            "cleanup must not drain between the I/O failure and the end of its recovery"
        );

        steps.release("delete");
        running
            .await
            .expect("the execution task must not be aborted")
            .unwrap_err();
        cleaning.await.unwrap();

        assert_eq!(deleted_names(&runner), vec![name]);
        assert!(exec.state.session_to_container.is_empty());
        assert_map_stays_empty_after_cleanup(&exec).await;
        assert_no_untracked_container(&exec, &runner).await;
    }

    #[tokio::test]
    async fn a_future_dropped_off_the_runtime_still_blocks_cleanup_until_recovery_ends() {
        // Cancellation ownership must not depend on *where* the future is
        // dropped. The execution is polled inside the runtime, then moved to a
        // plain OS thread with no current Tokio context and dropped there. The
        // guard spawns through the runtime handle it captured when it was
        // armed, so the permit moves into the recovery task exactly as it does
        // for an in-runtime drop, and cleanup must wait for the CLI task to be
        // joined and the container deleted.
        let (runner, mut steps) = FakeRunner::stepped(|_| Ok(ok_output("")));
        let exec = executor(runner.clone());
        let ctx = ctx();

        steps.gate("exec");
        let mut running = Box::pin(exec.execute_command("sleep 100", &ctx));
        assert!(futures::poll!(&mut running).is_pending());
        steps.wait_for("exec").await;
        let name = created_names(&runner)[0].clone();

        // Park the recovery's deletion so it cannot complete on its own.
        steps.gate("delete");
        // Scoped so the future may borrow `exec`/`ctx`; joined before anything
        // else happens, so the drop is genuinely complete and genuinely
        // off-runtime.
        std::thread::scope(|scope| {
            scope
                .spawn(move || {
                    assert!(
                        tokio::runtime::Handle::try_current().is_err(),
                        "this thread must have no current runtime, or the test proves nothing"
                    );
                    drop(running);
                })
                .join()
                .expect("the dropping thread must not panic");
        });

        // The permit was carried out of that thread into the recovery task.
        let mut cleaning = Box::pin(exec.cleanup());
        assert!(
            futures::poll!(&mut cleaning).is_pending(),
            "cleanup must wait for a recovery whose future was dropped off the runtime"
        );
        assert!(
            deleted_names(&runner).is_empty(),
            "nothing may have been deleted yet: {:?}",
            deleted_names(&runner)
        );

        // The recovery does run — on the captured handle — and cleanup is still
        // blocked while it is mid-delete.
        assert_eq!(
            steps.wait_for("delete").await,
            vec!["delete", "--force", &name]
        );
        assert!(
            futures::poll!(&mut cleaning).is_pending(),
            "cleanup must still wait while the off-runtime drop's recovery is mid-delete"
        );

        steps.release("delete");
        cleaning.await.unwrap();

        // The CLI task ended because it observed the cancellation signal, which
        // is the path on which the runner kills and reaps its child: the join
        // happened, it was not merely requested.
        assert_eq!(runner.cancelled_calls(), 1);
        assert_eq!(deleted_names(&runner), vec![name]);
        assert!(exec.state.session_to_container.is_empty());
        assert_map_stays_empty_after_cleanup(&exec).await;
        assert_no_untracked_container(&exec, &runner).await;
    }

    #[tokio::test]
    async fn end_session_after_cleanup_does_not_repopulate_the_map() {
        // The postcondition the P2 finding was about: `end_session` arriving
        // after terminal cleanup returned must not insert a tombstone into the
        // map cleanup drained.
        let runner = FakeRunner::ok();
        let exec = executor(runner.clone());
        let ctx = ctx();
        exec.execute_command("true", &ctx).await.unwrap();
        let name = created_names(&runner)[0].clone();

        exec.cleanup().await.unwrap();
        assert!(deleted_names(&runner).contains(&name));
        assert!(exec.state.session_to_container.is_empty());

        // Both a session cleanup already reclaimed and one it never saw.
        exec.end_session(&ctx.session_id).await.unwrap();
        exec.end_session(&SessionId::new()).await.unwrap();

        assert!(
            exec.state.session_to_container.is_empty(),
            "end_session after cleanup must not re-insert a cell: {:?}",
            exec.state
                .session_to_container
                .iter()
                .map(|kv| *kv.key())
                .collect::<Vec<_>>()
        );
        assert_map_stays_empty_after_cleanup(&exec).await;
        assert_eq!(
            deleted_names(&runner),
            vec![name],
            "there is nothing left for a post-cleanup end_session to delete"
        );
    }

    #[tokio::test]
    async fn end_session_queued_behind_cleanup_leaves_the_map_empty() {
        // The harder schedule: `end_session` asks for its read permit while
        // cleanup already holds the write gate, so it resumes *after* cleanup
        // has drained and returned. It must still find nothing to insert.
        let (runner, mut steps) = FakeRunner::stepped(|_| Ok(ok_output("")));
        let exec = Arc::new(executor(runner.clone()));
        let ctx = ctx();
        exec.execute_command("true", &ctx).await.unwrap();
        let name = created_names(&runner)[0].clone();

        // Park cleanup inside its delete, so it holds the write gate.
        steps.gate("delete");
        let cleaning = {
            let exec = exec.clone();
            tokio::spawn(async move { exec.cleanup().await })
        };
        assert_eq!(
            steps.wait_for("delete").await,
            vec!["delete", "--force", &name]
        );

        // Queued behind that write gate.
        let mut ending = Box::pin(exec.end_session(&ctx.session_id));
        assert!(
            futures::poll!(&mut ending).is_pending(),
            "end_session must wait behind cleanup's write gate"
        );

        steps.release("delete");
        cleaning.await.unwrap().unwrap();
        ending.await.unwrap();

        assert!(
            exec.state.session_to_container.is_empty(),
            "an end_session queued behind cleanup must not repopulate the drained map: {:?}",
            exec.state
                .session_to_container
                .iter()
                .map(|kv| *kv.key())
                .collect::<Vec<_>>()
        );
        assert_map_stays_empty_after_cleanup(&exec).await;
        assert_eq!(deleted_names(&runner), vec![name]);
        assert_no_untracked_container(&exec, &runner).await;
    }

    #[tokio::test]
    async fn end_session_after_cleanup_still_retries_a_survivor_cleanup_could_not_delete() {
        // Refusing to *create* a cell after cleanup must not stop teardown of
        // one that is still tracked: cleanup re-inserts a container it could
        // not delete, and a later end_session still owns it.
        let fail_delete = Arc::new(AtomicBool::new(true));
        let flag = fail_delete.clone();
        let runner = FakeRunner::new(move |args| {
            if args[0] == "delete" && flag.load(Ordering::SeqCst) {
                Ok(fail_output(1, "resource busy"))
            } else {
                Ok(ok_output(""))
            }
        });
        let exec = executor(runner.clone());
        let ctx = ctx();
        exec.execute_command("true", &ctx).await.unwrap();
        let name = created_names(&runner)[0].clone();

        exec.cleanup().await.unwrap_err();
        assert_eq!(
            tracked_names(&exec).await,
            vec![name.clone()],
            "cleanup must keep a container it could not delete reachable"
        );

        // Now the delete succeeds, and end_session reclaims the survivor.
        fail_delete.store(false, Ordering::SeqCst);
        exec.end_session(&ctx.session_id).await.unwrap();
        assert!(
            tracked_names(&exec).await.is_empty(),
            "the survivor must be reclaimed and untracked"
        );
        assert_eq!(deleted_names(&runner), vec![name.clone(), name]);
        assert_no_untracked_container(&exec, &runner).await;
    }

    #[tokio::test]
    async fn cleanup_forced_between_abandonment_unbind_and_delete_reclaims_the_container() {
        // The schedule the old code lost a container on: `cleanup` acquires its
        // gate after abandonment cleared the binding but before the delete
        // finished, so it drained no name from that slot and returned while the
        // deletion was still outstanding. Abandonment now holds the read gate
        // for its whole unbind/delete/retrack, so cleanup must be *pending*
        // here.
        let (runner, mut steps) = FakeRunner::stepped(|args| {
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
        let exec = Arc::new(executor(runner.clone()));
        let ctx = ctx();

        steps.gate("delete");
        let running = {
            let exec = exec.clone();
            let ctx = ctx.clone();
            tokio::spawn(async move { exec.execute_command("sleep 100", &ctx).await })
        };
        let argv = steps.wait_for("delete").await;
        let name = argv[2].clone();

        // Mid-abandonment: unbound, but still tracked as unconfirmed.
        assert_eq!(bound_container(&exec, &ctx.session_id).await, None);
        assert_eq!(
            pending_containers(&exec, &ctx.session_id).await,
            vec![name.clone()]
        );

        let mut cleaning = Box::pin(exec.cleanup());
        assert!(
            futures::poll!(&mut cleaning).is_pending(),
            "cleanup must wait for an in-flight abandonment instead of draining past it"
        );

        steps.release("delete");
        assert!(running.await.unwrap().unwrap().timed_out);
        cleaning.await.unwrap();

        assert_eq!(deleted_names(&runner), vec![name]);
        assert!(exec.state.session_to_container.is_empty());
        assert_no_untracked_container(&exec, &runner).await;
    }

    #[tokio::test]
    async fn a_failed_abandonment_delete_blocks_a_replacement_and_stays_tracked() {
        // Every delete fails. The abandoned container must stay tracked, and —
        // the invariant the old code lacked — the session must NOT bind a
        // replacement while it is unconfirmed, because a slot that holds a
        // replacement is a slot that cannot hold the survivor.
        let runner = FakeRunner::new(|args| match args[0].as_str() {
            "delete" => Ok(fail_output(1, "resource busy")),
            "exec" => Ok(CommandOutput {
                exit_code: -1,
                stdout: String::new(),
                stderr: String::new(),
                timed_out: true,
            }),
            _ => Ok(ok_output("")),
        });
        let exec = executor(runner.clone());
        let ctx = ctx();

        assert!(
            exec.execute_command("sleep 100", &ctx)
                .await
                .unwrap()
                .timed_out
        );
        let name = created_names(&runner)[0].clone();
        assert_eq!(tracked_names(&exec).await, vec![name.clone()]);

        let err = exec.execute_command("echo hi", &ctx).await.unwrap_err();
        assert!(
            err.to_string().contains("awaiting confirmed deletion"),
            "{err}"
        );
        assert_eq!(
            created_names(&runner),
            vec![name.clone()],
            "no replacement may be created while an older container is unconfirmed"
        );

        // cleanup retries it, fails again, and still keeps it reachable.
        let err = exec.cleanup().await.unwrap_err();
        assert!(err.to_string().contains(&name), "{err}");
        assert_eq!(tracked_names(&exec).await, vec![name]);
        assert_no_untracked_container(&exec, &runner).await;
    }

    #[tokio::test]
    async fn a_partially_created_container_that_cannot_be_deleted_stays_tracked() {
        // `container run` fails, so a container may or may not exist under the
        // generated name — and the cleanup of that partial creation fails too.
        // The name must stay tracked (it is the only handle to a possibly-live
        // VM), block a replacement, and be retried by cleanup.
        let runner = FakeRunner::new(|args| match args[0].as_str() {
            "run" => Ok(fail_output(125, "image not found")),
            "delete" => Ok(fail_output(1, "resource busy")),
            _ => Ok(ok_output("")),
        });
        let exec = executor(runner.clone());
        let session = SessionId::new();

        let err = exec.ensure_container_name(&session).await.unwrap_err();
        assert!(matches!(err, RlmError::BackendInitFailed { .. }), "{err:?}");
        let name = created_names(&runner)[0].clone();
        assert_eq!(
            tracked_names(&exec).await,
            vec![name.clone()],
            "a name `container run` may have created must stay tracked when its delete fails"
        );

        let err = exec.ensure_container_name(&session).await.unwrap_err();
        assert!(
            err.to_string().contains("awaiting confirmed deletion"),
            "{err}"
        );
        assert_eq!(
            created_names(&runner),
            vec![name.clone()],
            "no second `container run` while the first name is unconfirmed"
        );

        let err = exec.cleanup().await.unwrap_err();
        assert!(err.to_string().contains(&name), "{err}");
        assert_eq!(tracked_names(&exec).await, vec![name]);
        assert_no_untracked_container(&exec, &runner).await;
    }

    #[tokio::test]
    async fn a_partially_created_container_is_untracked_once_its_delete_is_confirmed() {
        // The other half: when the cleanup of a failed creation *is* confirmed,
        // the name is dropped from tracking and the session can retry.
        let runner = FakeRunner::new(|args| {
            if args[0] == "run" {
                Ok(fail_output(125, "image not found"))
            } else {
                Ok(ok_output(""))
            }
        });
        let exec = executor(runner.clone());
        let session = SessionId::new();

        exec.ensure_container_name(&session).await.unwrap_err();
        let name = created_names(&runner)[0].clone();
        assert_eq!(deleted_names(&runner), vec![name]);
        assert!(tracked_names(&exec).await.is_empty());
        assert!(pending_containers(&exec, &session).await.is_empty());
    }

    #[tokio::test]
    async fn a_cancelled_execution_marks_its_slot_unusable_before_the_drop_returns() {
        // The synchronous half of the contract, asserted without awaiting
        // anything in between: dropping the future must have made the slot
        // unusable by the time the drop returns, because the drop cannot await
        // the slot mutex and a later caller must still be refused.
        let (runner, mut steps) = FakeRunner::stepped(|_| Ok(ok_output("")));
        let exec = executor(runner.clone());
        let ctx = ctx();

        steps.gate("exec");
        let mut running = Box::pin(exec.execute_command("sleep 100", &ctx));
        assert!(futures::poll!(&mut running).is_pending());
        steps.wait_for("exec").await;

        let cell = exec
            .state
            .session_to_container
            .get(&ctx.session_id)
            .map(|e| e.value().clone())
            .expect("session slot");
        assert!(!cell.is_unusable());

        // Park the recovery deletion so the watcher cannot clear the flag
        // before the refusal below is observed: the assertion is about the
        // synchronous half of the guard, not about winning a race with it.
        steps.gate("delete");
        drop(running);
        // No await between the drop and this check.
        assert!(
            cell.is_unusable(),
            "the slot must be unusable the instant the execution future is dropped"
        );
        let err = exec
            .ensure_container_name(&ctx.session_id)
            .await
            .unwrap_err();
        assert!(err.to_string().contains("cancelled"), "{err}");

        // The bounded force-deletion still happens, on its own owned task.
        let name = created_names(&runner)[0].clone();
        assert_eq!(
            steps.wait_for("delete").await,
            vec!["delete", "--force", &name]
        );
        steps.release("delete");
    }

    #[tokio::test]
    async fn cleanup_is_terminal_and_refuses_further_session_creation() {
        let runner = FakeRunner::ok();
        let exec = executor(runner.clone());
        exec.ensure_container_name(&SessionId::new()).await.unwrap();
        exec.cleanup().await.unwrap();

        let before = runner.calls_starting_with("run").len();
        let err = exec
            .ensure_container_name(&SessionId::new())
            .await
            .unwrap_err();
        match err {
            RlmError::BackendInitFailed { ref message, .. } => {
                assert!(message.contains("shutting down"), "{message}")
            }
            other => panic!("expected BackendInitFailed, got {other:?}"),
        }
        assert_eq!(
            runner.calls_starting_with("run").len(),
            before,
            "no container may be created after cleanup"
        );
        assert!(exec.state.session_to_container.is_empty());
    }

    #[tokio::test]
    async fn validate_reports_the_configured_kg_strictness() {
        for strictness in [
            KgStrictness::Permissive,
            KgStrictness::Normal,
            KgStrictness::Strict,
        ] {
            let config = RlmConfig {
                kg_strictness: strictness,
                ..RlmConfig::minimal()
            };
            let exec = AppleContainerExecutor::with_runner(
                config,
                Some(Arc::new(
                    crate::validator::KnowledgeGraphValidator::disabled(),
                )),
                FakeRunner::ok(),
            );
            let result = exec.validate("some command").await.unwrap();
            assert_eq!(
                result.strictness, strictness,
                "validate must report the configured strictness, not a hardcoded one"
            );
        }
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
        assert_eq!(argv[8..].to_vec(), vec!["bash", "-c", "pwd"]);
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
            .run(sh, &args, Duration::from_millis(400), CancelSignal::never())
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

    /// Whether `pid` still exists **as a process table entry**.
    ///
    /// `kill -0` succeeds for a zombie, so a false here is evidence of reaping,
    /// not merely of killing.
    #[cfg(unix)]
    fn pid_alive(pid: &str) -> bool {
        std::process::Command::new("kill")
            .args(["-0", pid])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }

    /// Wait (bounded) for `path` to contain a complete line, without sleeping a
    /// fixed amount: the test polls a real condition and gives up loudly.
    #[cfg(unix)]
    async fn read_line_when_written(path: &Path) -> String {
        for _ in 0..500 {
            if let Ok(text) = std::fs::read_to_string(path)
                && let Some(line) = text.lines().next()
                && !line.trim().is_empty()
            {
                return line.trim().to_string();
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        panic!("{} was never written", path.display());
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn cancelling_a_real_child_kills_and_reaps_it_and_stops_the_drains() {
        // The cancellation contract on the *real* process path, which a fake
        // runner cannot demonstrate: raising the signal must terminate and reap
        // the host child, stop the stdout/stderr readers even though a
        // surviving grandchild still holds the pipe open, and return promptly.
        let sh = Path::new("/bin/sh");
        if !sh.exists() {
            eprintln!("skipping: /bin/sh not available");
            return;
        }
        let dir = std::env::temp_dir();
        let child_pid_file = dir.join(format!("terraphim-rlm-test-{}.child", Ulid::new()));
        let grandchild_pid_file = dir.join(format!("terraphim-rlm-test-{}.gc", Ulid::new()));

        // The backgrounded `sleep` inherits the stdout pipe and outlives the
        // child: if the readers were awaited unconditionally, this call could
        // not return for 120s.
        let script = format!(
            "sleep 120 & echo $! > {gc}; echo $$ > {child}; sleep 120",
            gc = grandchild_pid_file.display(),
            child = child_pid_file.display(),
        );
        let args = vec!["-c".to_string(), script];

        let signal = Arc::new(CancelSignal::default());
        let run_signal = signal.clone();
        let sh_path = sh.to_path_buf();
        let call = tokio::spawn(async move {
            TokioProcessRunner
                .run(
                    &sh_path,
                    &args,
                    Duration::from_secs(120),
                    run_signal.as_ref(),
                )
                .await
        });

        let child_pid = read_line_when_written(&child_pid_file).await;
        let grandchild_pid = read_line_when_written(&grandchild_pid_file).await;
        assert!(pid_alive(&child_pid), "the child should be running");

        let start = Instant::now();
        signal.cancel();
        let err = call
            .await
            .expect("the runner task must finish, not be aborted")
            .expect_err("a cancelled run must report cancellation");
        let elapsed = start.elapsed();

        assert_eq!(err.kind(), std::io::ErrorKind::Interrupted, "{err}");
        // Returning at all is the drain-termination evidence: the grandchild
        // still holds the stdout pipe open for another two minutes.
        assert!(
            pid_alive(&grandchild_pid),
            "grandchild should still hold the pipe, otherwise this test proves nothing about the drains"
        );
        assert!(
            elapsed < Duration::from_secs(10),
            "cancellation must not wait on a pipe a surviving grandchild holds open, took {elapsed:?}"
        );
        // Killed *and reaped*: `kill -0` on a zombie would still succeed.
        assert!(
            !pid_alive(&child_pid),
            "the host child must be killed and reaped before the call returns"
        );

        let _ = std::process::Command::new("kill")
            .args(["-9", &grandchild_pid])
            .status();
        let _ = std::fs::remove_file(&child_pid_file);
        let _ = std::fs::remove_file(&grandchild_pid_file);
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
                CancelSignal::never(),
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
        // The slot stays as a terminal tombstone with nothing left to reclaim.
        assert!(is_tombstoned(&exec, &ctx.session_id).await);
        assert!(tracked_names(&exec).await.is_empty());
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
            names.push(exec.ensure_container_name(&SessionId::new()).await.unwrap());
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
        // Only the container cleanup could not remove stays tracked, so a
        // repeated cleanup retries exactly it.
        assert_eq!(tracked_names(&exec).await, vec![names[1].clone()]);
    }

    #[tokio::test]
    async fn cleanup_succeeds_and_empties_the_map() {
        let runner = FakeRunner::ok();
        let exec = executor(runner);
        exec.ensure_container_name(&SessionId::new()).await.unwrap();
        exec.ensure_container_name(&SessionId::new()).await.unwrap();
        exec.cleanup().await.unwrap();
        assert!(exec.state.session_to_container.is_empty());
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
