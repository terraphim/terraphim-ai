//! Output capture with @mention detection and redaction
//!
//! Captures stdout/stderr from agent processes, detects @mentions, and
//! stores a bounded buffer of redacted events for timeout reporting.
//!
//! When constructed with an open [`Journal`] (terraphim-ai#3269), every
//! captured event is first redacted, then durably appended to the
//! per-run journal via a cloneable `DurableOutputWriter` that owns
//! the journal on a blocking thread. Only after the durable
//! acknowledgement does the event fan out to the in-memory buffer,
//! the mpsc channel and the broadcast subscribers. A journal failure
//! surfaces as [`OutputJournalError`] and suppresses all fanout.

use chrono::Utc;
use regex::Regex;
use std::collections::VecDeque;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{ChildStderr, ChildStdout};
use tokio::sync::{broadcast, mpsc, oneshot, Mutex as AsyncMutex};
use tokio::task::{JoinError, JoinHandle};
use uuid::Uuid;

use crate::journal::{Journal, JournalError, JournalRecord, OutputKind};
use crate::redaction;
use terraphim_types::capability::ProcessId;

/// Maximum number of captured events to retain per agent.
const MAX_CAPTURED_EVENTS: usize = 4096;

/// Capacity of the command channel feeding the blocking journal writer.
const WRITER_CHANNEL_CAPACITY: usize = 256;

/// Errors surfaced while durably persisting output events to a
/// [`Journal`].
#[derive(Debug, thiserror::Error)]
pub enum OutputJournalError {
    /// The journal rejected or failed a durable write.
    #[error("durable output journal error: {0}")]
    Journal(#[from] JournalError),

    /// The blocking journal writer task is gone, so the durable fate
    /// of the event is unknown.
    #[error("durable output writer is no longer running")]
    WriterClosed,

    /// The blocking journal writer panicked or was cancelled while it
    /// still owned the journal.
    #[error("durable output writer task failed: {0}")]
    WriterTask(JoinError),

    /// Crash-resume payload mismatch: a completion-tagged
    /// [`JournalRecord`] was recovered from the journal but its
    /// `kind` or `payload` did not match the terminal payload the
    /// caller asked the writer to seal. The on-disk journal is left
    /// untouched so the operator can reconcile the disagreement
    /// (typically by inspecting the recovered terminal record)
    /// before retrying `complete()`.
    #[error(
        "terminal payload conflict: caller asked for exit_code {expected:?}, \
         recovered kind {found_kind:?} payload {found}"
    )]
    TerminalPayloadConflict {
        /// Exit code passed to the `Complete` writer command.
        expected: Option<i32>,
        /// Kind carried by the recovered completion-tagged record.
        found_kind: OutputKind,
        /// Payload carried by the recovered completion-tagged record.
        found: serde_json::Value,
    },

    /// I/O error while reading captured output from the child pipe.
    ///
    /// Returned by the stdout/stderr capture tasks on EOF is **not**
    /// an error: that path produces `Ok(())` instead.
    #[error("captured {stream} I/O error: {source}")]
    CaptureIo {
        /// Which stream failed: `"stdout"` or `"stderr"`.
        stream: String,
        /// The underlying I/O error returned by `read_line`.
        #[source]
        source: std::io::Error,
    },

    /// A capture task panicked, was cancelled, or otherwise failed to
    /// produce a final result before [`OutputCapture::finish`] awaited
    /// it. Wraps the [`JoinError`] reported by `tokio::spawn`.
    #[error("output capture task failed: {0}")]
    CaptureTask(#[from] JoinError),
}

/// Events that can be captured from agent output
#[derive(Debug, Clone)]
pub enum OutputEvent {
    /// Standard output line
    Stdout { process_id: ProcessId, line: String },
    /// Standard error line
    Stderr { process_id: ProcessId, line: String },
    /// @mention detected in output
    Mention {
        process_id: ProcessId,
        target: String,
        message: String,
    },
    /// Process completed
    Completed {
        process_id: ProcessId,
        exit_code: Option<i32>,
    },
}

impl OutputEvent {
    /// Return a redacted copy of this event with secrets scrubbed.
    fn redacted(&self) -> Self {
        match self {
            Self::Stdout { process_id, line } => Self::Stdout {
                process_id: *process_id,
                line: redaction::redact(line),
            },
            Self::Stderr { process_id, line } => Self::Stderr {
                process_id: *process_id,
                line: redaction::redact(line),
            },
            Self::Mention {
                process_id,
                target,
                message,
            } => Self::Mention {
                process_id: *process_id,
                target: target.clone(),
                message: redaction::redact(message),
            },
            Self::Completed {
                process_id,
                exit_code,
            } => Self::Completed {
                process_id: *process_id,
                exit_code: *exit_code,
            },
        }
    }
}

/// Command sent to the blocking journal writer task.
#[derive(Debug)]
enum WriterCommand {
    /// Durably append one (already redacted) output event.
    Append(OutputEvent),
    /// Write the terminal `Completed` record and seal the journal.
    Complete { exit_code: Option<i32> },
    /// Close the journal owner without writing a terminal record.
    Close,
    #[cfg(test)]
    /// Test hook proving a blocking writer panic is surfaced via join.
    Panic,
}

/// Acknowledgement channel for a single writer command.
type WriterAck = oneshot::Sender<Result<(), OutputJournalError>>;

/// Cloneable handle to the blocking journal writer task.
///
/// The handle is a thin mpsc sender; the [`Journal`] itself is owned
/// exclusively by the writer task spawned on `spawn_blocking`, which
/// drains commands with `blocking_recv`. Every command round-trips a
/// oneshot acknowledgement so callers can await durable completion.
#[derive(Clone)]
struct DurableOutputWriter {
    lifecycle: Arc<AsyncMutex<WriterLifecycle>>,
    task: Arc<AsyncMutex<Option<JoinHandle<()>>>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WriterLifecycleStatus {
    Open,
    Completed(Option<i32>),
    Closed,
}

struct WriterLifecycle {
    tx: Option<mpsc::Sender<(WriterCommand, WriterAck)>>,
    status: WriterLifecycleStatus,
}

impl std::fmt::Debug for DurableOutputWriter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DurableOutputWriter")
            .finish_non_exhaustive()
    }
}

impl DurableOutputWriter {
    /// Spawn the blocking writer that owns `journal` for `process_id`.
    fn spawn(journal: Journal, process_id: ProcessId) -> Self {
        let (tx, rx) = mpsc::channel(WRITER_CHANNEL_CAPACITY);
        let task = tokio::task::spawn_blocking(move || run_writer(journal, process_id, rx));
        Self {
            lifecycle: Arc::new(AsyncMutex::new(WriterLifecycle {
                tx: Some(tx),
                status: WriterLifecycleStatus::Open,
            })),
            task: Arc::new(AsyncMutex::new(Some(task))),
        }
    }

    /// Send one command while the lifecycle lock serializes all callers.
    async fn send_open(&self, cmd: WriterCommand) -> Result<(), OutputJournalError> {
        let mut lifecycle = self.lifecycle.lock().await;
        if !matches!(lifecycle.status, WriterLifecycleStatus::Open) {
            return Err(OutputJournalError::WriterClosed);
        }
        let (ack_tx, ack_rx) = oneshot::channel();
        let send_result = lifecycle
            .tx
            .as_ref()
            .expect("open writer has sender")
            .send((cmd, ack_tx))
            .await;
        if send_result.is_err() {
            lifecycle.status = WriterLifecycleStatus::Closed;
            lifecycle.tx.take();
            drop(lifecycle);
            return self.join_after_channel_failure().await;
        }
        let ack = ack_rx.await;
        if let Ok(result) = ack {
            return result;
        }
        lifecycle.status = WriterLifecycleStatus::Closed;
        lifecycle.tx.take();
        drop(lifecycle);
        self.join_after_channel_failure().await
    }

    async fn join_after_channel_failure(&self) -> Result<(), OutputJournalError> {
        match self.join_task().await {
            Err(error) => Err(error),
            Ok(()) => Err(OutputJournalError::WriterClosed),
        }
    }

    /// Await the retained blocking task without holding the lifecycle mutex.
    async fn join_task(&self) -> Result<(), OutputJournalError> {
        let mut task = self.task.lock().await;
        let Some(handle) = task.as_mut() else {
            return Ok(());
        };
        let result = handle.await;
        task.take();
        result.map_err(OutputJournalError::WriterTask)
    }

    /// Durably append one redacted output event.
    async fn append(&self, event: OutputEvent) -> Result<(), OutputJournalError> {
        self.send_open(WriterCommand::Append(event)).await
    }

    /// Write the terminal `Completed` record, then seal the journal.
    async fn complete(&self, exit_code: Option<i32>) -> Result<(), OutputJournalError> {
        let mut lifecycle = self.lifecycle.lock().await;
        match lifecycle.status {
            WriterLifecycleStatus::Completed(completed_exit_code)
                if completed_exit_code == exit_code =>
            {
                drop(lifecycle);
                return self.join_task().await;
            }
            WriterLifecycleStatus::Completed(completed_exit_code) => {
                return Err(OutputJournalError::TerminalPayloadConflict {
                    expected: exit_code,
                    found_kind: OutputKind::Completed,
                    found: serde_json::json!({ "exit_code": completed_exit_code }),
                });
            }
            WriterLifecycleStatus::Closed => return Err(OutputJournalError::WriterClosed),
            WriterLifecycleStatus::Open => {}
        }

        let (ack_tx, ack_rx) = oneshot::channel();
        let send_result = lifecycle
            .tx
            .as_ref()
            .expect("open writer has sender")
            .send((WriterCommand::Complete { exit_code }, ack_tx))
            .await;
        if send_result.is_err() {
            lifecycle.status = WriterLifecycleStatus::Closed;
            lifecycle.tx.take();
            drop(lifecycle);
            return self.join_after_channel_failure().await;
        }
        match ack_rx.await {
            Ok(Ok(())) => {
                lifecycle.status = WriterLifecycleStatus::Completed(exit_code);
                lifecycle.tx.take();
                drop(lifecycle);
                self.join_task().await
            }
            Ok(Err(error)) => Err(error),
            Err(_) => {
                lifecycle.status = WriterLifecycleStatus::Closed;
                lifecycle.tx.take();
                drop(lifecycle);
                self.join_after_channel_failure().await
            }
        }
    }

    /// Stop the writer without adding a terminal record.
    async fn close(&self) -> Result<(), OutputJournalError> {
        let mut lifecycle = self.lifecycle.lock().await;
        if !matches!(lifecycle.status, WriterLifecycleStatus::Open) {
            drop(lifecycle);
            return self.join_task().await;
        }

        let (ack_tx, ack_rx) = oneshot::channel();
        let send_result = lifecycle
            .tx
            .as_ref()
            .expect("open writer has sender")
            .send((WriterCommand::Close, ack_tx))
            .await;
        if send_result.is_err() {
            lifecycle.status = WriterLifecycleStatus::Closed;
            lifecycle.tx.take();
            drop(lifecycle);
            return self.join_after_channel_failure().await;
        }

        // Once Close is queued, no later command may be admitted. Record
        // that transition and drop the final sender before awaiting the ack,
        // so cancellation of this caller still lets a retry join the task.
        lifecycle.status = WriterLifecycleStatus::Closed;
        lifecycle.tx.take();
        drop(lifecycle);

        match ack_rx.await {
            Ok(Ok(())) => self.join_task().await,
            Ok(Err(error)) => Err(error),
            Err(_) => self.join_after_channel_failure().await,
        }
    }

    #[cfg(test)]
    async fn panic_for_test(&self) -> Result<(), OutputJournalError> {
        self.send_open(WriterCommand::Panic).await
    }
}

/// Reopen an existing run journal and seal it through the same durable
/// writer lifecycle used by live output capture.
pub(crate) async fn seal_reopened_journal(
    root: &std::path::Path,
    run_id: Uuid,
    process_id: ProcessId,
    exit_code: Option<i32>,
) -> Result<(), OutputJournalError> {
    let journal = Journal::open(root, run_id)?;
    DurableOutputWriter::spawn(journal, process_id)
        .complete(exit_code)
        .await
}

/// Mutable state of the blocking journal writer, owned by the writer
/// task only.
struct WriterState {
    journal: Journal,
    run_id: Uuid,
    process_id: ProcessId,
    /// Next contiguous sequence number to assign.
    next_sequence: u64,
    /// Completion id generated once per writer; stable across retries
    /// so repeated `Complete` commands seal the same id.
    completion_id: Uuid,
    /// Set once the `Completed` record + `Complete` frame are durable.
    completed: bool,
}

impl WriterState {
    /// Append `event` as the next `JournalRecord`.
    ///
    /// The event must already be redacted; the writer assigns the
    /// contiguous `sequence` and stamps `completion_id` when supplied.
    fn append_event(
        &mut self,
        event: &OutputEvent,
        completion_id: Option<Uuid>,
    ) -> Result<(), OutputJournalError> {
        let (kind, payload) = match event {
            OutputEvent::Stdout { line, .. } => {
                (OutputKind::Stdout, serde_json::Value::String(line.clone()))
            }
            OutputEvent::Stderr { line, .. } => {
                (OutputKind::Stderr, serde_json::Value::String(line.clone()))
            }
            OutputEvent::Mention {
                target, message, ..
            } => (
                OutputKind::Mention,
                serde_json::json!({ "target": target, "message": message }),
            ),
            OutputEvent::Completed { exit_code, .. } => (
                OutputKind::Completed,
                serde_json::json!({ "exit_code": exit_code }),
            ),
        };
        let record = JournalRecord {
            run_id: self.run_id,
            sequence: self.next_sequence,
            process_id: self.process_id.0,
            kind,
            payload,
            completion_id,
            observed_at: Utc::now(),
        };
        self.journal.append(record)?;
        self.next_sequence += 1;
        Ok(())
    }
}

/// Blocking writer loop: drain commands, write to the journal, ack.
///
/// Runs on a `spawn_blocking` thread and therefore uses
/// [`mpsc::Receiver::blocking_recv`]. The loop exits when the last
/// writer handle is dropped.
fn run_writer(
    journal: Journal,
    process_id: ProcessId,
    mut rx: mpsc::Receiver<(WriterCommand, WriterAck)>,
) {
    let run_id = journal.run_id();
    // Seed the writer from the recovered journal state so a writer
    // spawned against a journal that already holds records resumes
    // cleanly instead of double-writing. `completion_record` is
    // canonical both before and after sealing so
    // idempotent retries can validate the original terminal payload.
    let completion_record = journal.completion_record().cloned();
    let next_sequence = journal.next_sequence();
    let completion_id = journal.completion_identity().unwrap_or_else(Uuid::new_v4);
    let completed = journal.is_completed();

    let mut state = WriterState {
        journal,
        run_id,
        process_id,
        next_sequence,
        completion_id,
        completed,
    };

    while let Some((cmd, ack)) = rx.blocking_recv() {
        let (result, close) = match cmd {
            WriterCommand::Append(event) => (state.append_event(&event, None), false),
            WriterCommand::Complete { exit_code } => (
                handle_complete(&mut state, completion_record.as_ref(), exit_code),
                false,
            ),
            WriterCommand::Close => (Ok(()), true),
            #[cfg(test)]
            WriterCommand::Panic => panic!("deliberate durable writer panic"),
        };
        let _ = ack.send(result);
        if close {
            break;
        }
    }
}

/// Process a `Complete { exit_code }` command.
///
/// Three branches:
///
/// * `state.completed` — the journal is already sealed. The recovered
///   terminal record must match the retry payload before this returns
///   `Ok(())` without touching the file.
/// * `pending` is `Some(record)` — crash-resume. A prior writer
///   appended the terminal `Completed` record but crashed before
///   sealing the journal. The recovered payload must match the
///   caller's `exit_code` exactly; on mismatch return
///   [`OutputJournalError::TerminalPayloadConflict`] without writing
///   anything, on match seal with the recovered completion id and
///   skip re-writing the `Completed` record.
/// * `pending` is `None` — fresh writer. Append the terminal
///   `Completed` record and seal the journal, mirroring the original
///   pre-resume behaviour.
fn handle_complete(
    state: &mut WriterState,
    completion_record: Option<&JournalRecord>,
    exit_code: Option<i32>,
) -> Result<(), OutputJournalError> {
    if state.completed {
        let Some(completed) = state.journal.completion_record().or(completion_record) else {
            return Err(JournalError::InvalidState(
                "completed journal is missing its Completed record".to_string(),
            )
            .into());
        };
        if completed.kind != OutputKind::Completed
            || completed.payload != serde_json::json!({ "exit_code": exit_code })
        {
            return Err(OutputJournalError::TerminalPayloadConflict {
                expected: exit_code,
                found_kind: completed.kind,
                found: completed.payload.clone(),
            });
        }
        return Ok(());
    }
    if let Some(pending) = completion_record {
        if pending.kind != OutputKind::Completed
            || pending.payload != serde_json::json!({ "exit_code": exit_code })
        {
            return Err(OutputJournalError::TerminalPayloadConflict {
                expected: exit_code,
                found_kind: pending.kind,
                found: pending.payload.clone(),
            });
        }
        // A tagged completion record is contractually paired with a
        // non-None `completion_id`; the journal's pending getter
        // filters for `completion_id.is_some()` so the unwrap below
        // cannot fire on a record produced by this crate's API.
        let pending_completion_id = pending
            .completion_id
            .expect("pending completion record carries a completion_id");
        state.journal.complete(pending_completion_id)?;
        state.completed = true;
        return Ok(());
    }
    let completion_id = state.completion_id;
    let completed_event = OutputEvent::Completed {
        process_id: state.process_id,
        exit_code,
    };
    state
        .append_event(&completed_event, Some(completion_id))
        .and_then(|()| {
            state.journal.complete(completion_id)?;
            Ok(())
        })
        .map(|()| {
            state.completed = true;
        })
}

/// Shared fanout state used by both capture tasks and the public API.
#[derive(Debug)]
struct OutputFanout {
    event_sender: mpsc::Sender<OutputEvent>,
    broadcast_sender: broadcast::Sender<OutputEvent>,
    /// Bounded buffer of redacted events for timeout reporting.
    captured_events: Arc<Mutex<VecDeque<OutputEvent>>>,
    /// Optional durable journal writer (terraphim-ai#3269).
    durable_writer: Option<DurableOutputWriter>,
    /// Ensures the terminal event is fanned out at most once, after the
    /// durable terminal acknowledgement.
    terminal_fanned_out: AtomicBool,
}

impl OutputFanout {
    /// Core publish path.
    ///
    /// Redacts first, then (when a journal is attached) awaits the
    /// durable acknowledgement before any in-memory fanout. A journal
    /// failure returns [`OutputJournalError`] and performs no fanout;
    /// mpsc/broadcast delivery remains best-effort.
    async fn publish(&self, event: OutputEvent) -> Result<(), OutputJournalError> {
        let event = event.redacted();

        if let Some(writer) = &self.durable_writer {
            writer.append(event.clone()).await?;
        }

        OutputCapture::record_event(&self.captured_events, &event);
        let _ = self.event_sender.send(event.clone()).await;
        let _ = self.broadcast_sender.send(event);
        Ok(())
    }

    /// Fan out an event whose durable write has already been acknowledged.
    async fn fanout_after_durable(&self, event: OutputEvent) {
        let event = event.redacted();
        OutputCapture::record_event(&self.captured_events, &event);
        let _ = self.event_sender.send(event.clone()).await;
        let _ = self.broadcast_sender.send(event);
    }
}

/// Captures output from agent processes with @mention detection
#[derive(Debug)]
pub struct OutputCapture {
    process_id: ProcessId,
    mention_regex: Regex,
    /// Shared fanout state, cloned into the capture tasks.
    fanout: Arc<OutputFanout>,
    /// Optional path to write redacted stderr lines for post-mortem
    /// debugging.
    stderr_log_path: Option<std::path::PathBuf>,
    /// Join handles for the stdout/stderr capture tasks, retained so
    /// [`OutputCapture::finish`] can drain them deterministically and
    /// surface capture-side failures through [`OutputJournalError`].
    capture_tasks: Vec<JoinHandle<Result<(), OutputJournalError>>>,
}

impl OutputCapture {
    /// Create a new output capture
    pub fn new(
        process_id: ProcessId,
        stdout: BufReader<ChildStdout>,
        stderr: BufReader<ChildStderr>,
    ) -> Self {
        Self::new_inner(process_id, stdout, stderr, None, None)
    }

    /// Create a new output capture with optional stderr log file.
    ///
    /// When `stderr_log_path` is Some, every redacted stderr line is
    /// appended to the file in addition to being broadcast. This
    /// provides a durable fallback when the broadcast channel lags or
    /// the drain task drops events.
    pub fn new_with_stderr_log(
        process_id: ProcessId,
        stdout: BufReader<ChildStdout>,
        stderr: BufReader<ChildStderr>,
        stderr_log_path: Option<std::path::PathBuf>,
    ) -> Self {
        Self::new_inner(process_id, stdout, stderr, stderr_log_path, None)
    }

    /// Create a new output capture backed by an open [`Journal`]
    /// (terraphim-ai#3269).
    ///
    /// Every stdout/stderr/mention event is redacted, durably appended
    /// to the journal, and only then fanned out. The journal must be
    /// empty and not yet completed — a pre-completed journal rejects
    /// every publish with [`OutputJournalError`].
    pub fn new_with_journal(
        process_id: ProcessId,
        stdout: BufReader<ChildStdout>,
        stderr: BufReader<ChildStderr>,
        journal: Journal,
    ) -> Self {
        Self::new_inner(process_id, stdout, stderr, None, Some(journal))
    }

    /// Create output capture with both the authoritative journal and the
    /// post-journal redacted stderr mirror enabled.
    pub fn new_with_journal_and_stderr_log(
        process_id: ProcessId,
        stdout: BufReader<ChildStdout>,
        stderr: BufReader<ChildStderr>,
        journal: Journal,
        stderr_log_path: Option<std::path::PathBuf>,
    ) -> Self {
        Self::new_inner(process_id, stdout, stderr, stderr_log_path, Some(journal))
    }

    /// Shared constructor.
    fn new_inner(
        process_id: ProcessId,
        stdout: BufReader<ChildStdout>,
        stderr: BufReader<ChildStderr>,
        stderr_log_path: Option<std::path::PathBuf>,
        journal: Option<Journal>,
    ) -> Self {
        let (event_sender, _event_receiver) = mpsc::channel::<OutputEvent>(100);
        let (broadcast_sender, _) = broadcast::channel(256);
        let durable_writer = journal.map(|journal| DurableOutputWriter::spawn(journal, process_id));

        let fanout = Arc::new(OutputFanout {
            event_sender,
            broadcast_sender,
            captured_events: Arc::new(Mutex::new(VecDeque::new())),
            durable_writer,
            terminal_fanned_out: AtomicBool::new(false),
        });

        let mut capture = Self {
            process_id,
            mention_regex: Regex::new(r"@(\w+)").unwrap(),
            fanout,
            stderr_log_path,
            capture_tasks: Vec::with_capacity(2),
        };

        // Start capturing stdout and stderr; retain the join handles so
        // `finish` can drain them deterministically.
        capture.capture_tasks.push(capture.capture_stdout(stdout));
        capture.capture_tasks.push(capture.capture_stderr(stderr));

        capture
    }

    /// Subscribe to live output events via broadcast channel.
    ///
    /// Returns a receiver that gets a clone of every output event.
    /// Suitable for streaming to WebSocket clients.
    pub fn subscribe(&self) -> broadcast::Receiver<OutputEvent> {
        self.fanout.broadcast_sender.subscribe()
    }

    /// Return a snapshot of captured redacted output events.
    pub fn captured_events(&self) -> Vec<OutputEvent> {
        self.fanout
            .captured_events
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .iter()
            .cloned()
            .collect()
    }

    /// Record a redacted event into the bounded buffer.
    fn record_event(captured_events: &Arc<Mutex<VecDeque<OutputEvent>>>, event: &OutputEvent) {
        let mut events = captured_events.lock().unwrap_or_else(|e| e.into_inner());
        if events.len() >= MAX_CAPTURED_EVENTS {
            events.pop_front();
        }
        events.push_back(event.redacted());
    }

    /// Core publish path for output events.
    ///
    /// See `OutputFanout::publish` for the ordering guarantees
    /// (redact -> durable append -> buffer/mpsc/broadcast).
    pub async fn publish_event(&self, event: OutputEvent) -> Result<(), OutputJournalError> {
        self.fanout.publish(event).await
    }

    /// Seal the durable journal for this run (no-op without a journal).
    ///
    /// Writes the terminal `Completed` record carrying the writer's
    /// stable completion id, then calls [`Journal::complete`].
    /// Idempotent: repeated calls succeed without writing additional
    /// terminal frames.
    pub async fn complete(&self, exit_code: Option<i32>) -> Result<(), OutputJournalError> {
        if let Some(writer) = &self.fanout.durable_writer {
            writer.complete(exit_code).await?;
        }

        if self
            .fanout
            .terminal_fanned_out
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_ok()
        {
            self.fanout
                .fanout_after_durable(OutputEvent::Completed {
                    process_id: self.process_id,
                    exit_code,
                })
                .await;
        }
        Ok(())
    }

    /// Drain the capture tasks without sealing the journal.
    ///
    /// Awaits every stdout/stderr capture handle retained at
    /// construction, mapping any [`JoinError`] into
    /// [`OutputJournalError::CaptureTask`]. The first capture error
    /// observed (whether from a join error or a task that returned
    /// `Err`) is returned to the caller. Unlike
    /// [`OutputCapture::finish`], this method does NOT call
    /// [`OutputCapture::complete`]: no terminal record is written, the
    /// journal stays unsealed, and no terminal event is fanned out.
    /// Used by the shared-journal fallback path (terraphim-ai#3269),
    /// where a failed primary must be drained while its run's journal
    /// remains open for the fallback attempt.
    ///
    /// Cancellation-safe: a JoinHandle is never removed from
    /// `capture_tasks` before its awaited result has been observed. If
    /// a caller wraps `drain` (or [`OutputCapture::finish`]) in a
    /// timeout that cancels the future mid-await, the in-flight
    /// handle stays in the vec (awaiting `&mut JoinHandle` neither
    /// aborts nor detaches the task), so a retry resumes the drain
    /// instead of finding an empty task list and silently dropping
    /// late output or a capture failure.
    pub(crate) async fn drain(&mut self) -> Result<(), OutputJournalError> {
        let mut first_error: Option<OutputJournalError> = None;
        while !self.capture_tasks.is_empty() {
            // Await the oldest handle by mutable reference; remove it
            // from the vec only after its result is in hand.
            let result = match (&mut self.capture_tasks[0]).await {
                Ok(Ok(())) => None,
                Ok(Err(e)) => Some(e),
                Err(join_err) => Some(OutputJournalError::CaptureTask(join_err)),
            };
            self.capture_tasks.remove(0);
            if let Some(err) = result {
                if first_error.is_none() {
                    first_error = Some(err);
                }
            }
        }
        if let Some(err) = first_error {
            return Err(err);
        }
        Ok(())
    }

    /// Drain all child output, then release the journal for a fallback
    /// owner without writing or fanning out a terminal event.
    pub(crate) async fn prepare_fallback_handoff(&mut self) -> Result<(), OutputJournalError> {
        let drain_result = self.drain().await;
        let close_result = match &self.fanout.durable_writer {
            Some(writer) => writer.close().await,
            None => Ok(()),
        };
        close_result?;
        drain_result
    }

    /// Drain the capture tasks and seal the durable journal.
    ///
    /// Awaits every stdout/stderr capture handle via
    /// `OutputCapture::drain`, surfacing the first capture error to
    /// the caller. Once all tasks have settled,
    /// [`OutputCapture::complete`] is invoked with `exit_code` to seal
    /// the journal and fan out the terminal event.
    ///
    pub async fn finish(&mut self, exit_code: Option<i32>) -> Result<(), OutputJournalError> {
        self.drain().await?;
        self.complete(exit_code).await
    }

    /// Start capturing stdout.
    ///
    /// Returns a join handle whose result reports the capture outcome:
    /// `Ok(())` on clean EOF,
    /// [`OutputJournalError::CaptureIo`] on a read error, or the
    /// propagated journal error if the fanout refuses an event. A
    /// fanout error aborts the loop immediately — the task never
    /// continues reading the pipe after the publisher fails.
    fn capture_stdout(
        &self,
        mut stdout: BufReader<ChildStdout>,
    ) -> JoinHandle<Result<(), OutputJournalError>> {
        let process_id = self.process_id;
        let mention_regex = self.mention_regex.clone();
        let fanout = Arc::clone(&self.fanout);

        tokio::spawn(async move {
            let mut line = String::new();

            loop {
                line.clear();
                match stdout.read_line(&mut line).await {
                    Ok(0) => break, // EOF
                    Ok(_) => {
                        let line = line.trim().to_string();
                        if line.is_empty() {
                            continue;
                        }

                        // Check for @mentions
                        if let Some(captures) = mention_regex.captures(&line) {
                            if let Some(target) = captures.get(1) {
                                let target = target.as_str().to_string();
                                let message = line.clone();

                                let mention_event = OutputEvent::Mention {
                                    process_id,
                                    target,
                                    message,
                                };
                                fanout.publish(mention_event).await?;
                            }
                        }

                        // Send stdout event
                        let stdout_event = OutputEvent::Stdout {
                            process_id,
                            line: line.clone(),
                        };
                        fanout.publish(stdout_event).await?;
                    }
                    Err(source) => {
                        return Err(OutputJournalError::CaptureIo {
                            stream: "stdout".to_string(),
                            source,
                        });
                    }
                }
            }
            Ok(())
        })
    }

    /// Start capturing stderr.
    ///
    /// Returns a join handle whose result reports the capture outcome
    /// (see [`OutputCapture::capture_stdout`] for the contract). The
    /// optional durable stderr log is written **only after** the
    /// fanout publish succeeds, so a journal failure never leaves
    /// orphan lines on disk that are missing from the journal.
    fn capture_stderr(
        &self,
        mut stderr: BufReader<ChildStderr>,
    ) -> JoinHandle<Result<(), OutputJournalError>> {
        let process_id = self.process_id;
        let fanout = Arc::clone(&self.fanout);
        let stderr_log_path = self.stderr_log_path.clone();

        tokio::spawn(async move {
            let mut line = String::new();
            let mut log_file = stderr_log_path.and_then(|path| {
                std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(&path)
                    .ok()
            });

            loop {
                line.clear();
                match stderr.read_line(&mut line).await {
                    Ok(0) => break, // EOF
                    Ok(_) => {
                        let line = line.trim().to_string();
                        if line.is_empty() {
                            continue;
                        }

                        let stderr_event = OutputEvent::Stderr {
                            process_id,
                            line: line.clone(),
                        };
                        fanout.publish(stderr_event).await?;

                        // After the fanout has accepted the line, mirror
                        // the redacted text to the durable stderr log if
                        // configured. The log must never hold secrets even
                        // though the broadcast/buffer paths redact
                        // independently.
                        if let Some(ref mut file) = log_file {
                            use std::io::Write;
                            let _ = writeln!(file, "{}", redaction::redact(&line));
                        }
                    }
                    Err(source) => {
                        return Err(OutputJournalError::CaptureIo {
                            stream: "stderr".to_string(),
                            source,
                        });
                    }
                }
            }
            Ok(())
        })
    }

    /// Get the event sender (for external use)
    pub fn event_sender(&self) -> mpsc::Sender<OutputEvent> {
        self.fanout.event_sender.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::journal::{Journal, JournalError, JournalRecord, OutputKind};
    use tempfile::TempDir;

    #[test]
    fn test_mention_regex() {
        let regex = Regex::new(r"@(\w+)").unwrap();

        let text = "Hello @kimiko, can you help?";
        let captures = regex.captures(text).unwrap();
        assert_eq!(captures.get(1).unwrap().as_str(), "kimiko");

        let text = "No mentions here";
        assert!(regex.captures(text).is_none());
    }

    #[test]
    fn test_output_event_redacted_scrubs_secrets() {
        let event = OutputEvent::Stdout {
            process_id: ProcessId::new(),
            line: "api_key=secret123".to_string(),
        };
        let redacted = event.redacted();
        match redacted {
            OutputEvent::Stdout { line, .. } => {
                assert!(line.contains("***REDACTED***"));
                assert!(!line.contains("secret123"));
            }
            _ => panic!("Expected Stdout event"),
        }
    }

    #[test]
    fn test_output_event_redacted_preserves_structure() {
        let event = OutputEvent::Stderr {
            process_id: ProcessId::new(),
            line: "Error: timeout after 30s".to_string(),
        };
        let redacted = event.redacted();
        match redacted {
            OutputEvent::Stderr { line, .. } => {
                assert_eq!(line, "Error: timeout after 30s");
            }
            _ => panic!("Expected Stderr event"),
        }
    }

    #[test]
    fn test_captured_events_bounded() {
        let (_event_sender, _event_receiver) = mpsc::channel::<OutputEvent>(100);
        let (_broadcast_sender, _) = broadcast::channel::<OutputEvent>(256);
        let captured = Arc::new(Mutex::new(VecDeque::new()));

        // Simulate recording MAX_CAPTURED_EVENTS + 10 events
        for i in 0..MAX_CAPTURED_EVENTS + 10 {
            let event = OutputEvent::Stdout {
                process_id: ProcessId::new(),
                line: format!("line {}", i),
            };
            OutputCapture::record_event(&captured, &event);
        }

        let events = captured.lock().unwrap();
        assert_eq!(events.len(), MAX_CAPTURED_EVENTS);
        // The oldest events should have been evicted
        assert!(!events
            .iter()
            .any(|e| matches!(e, OutputEvent::Stdout { line, .. } if line == "line 0")));
        assert!(events
            .iter()
            .any(|e| matches!(e, OutputEvent::Stdout { line, .. } if line == "line 10")));
    }

    #[test]
    fn test_captured_events_redacts_before_storage() {
        let (_event_sender, _event_receiver) = mpsc::channel::<OutputEvent>(100);
        let (_broadcast_sender, _) = broadcast::channel::<OutputEvent>(256);
        let captured = Arc::new(Mutex::new(VecDeque::new()));

        let event = OutputEvent::Stdout {
            process_id: ProcessId::new(),
            line: "api_key=secret123".to_string(),
        };
        OutputCapture::record_event(&captured, &event);

        let events = captured.lock().unwrap();
        assert_eq!(events.len(), 1);
        match &events[0] {
            OutputEvent::Stdout { line, .. } => {
                assert!(line.contains("***REDACTED***"));
                assert!(!line.contains("secret123"));
            }
            _ => panic!("Expected Stdout event"),
        }
    }

    // =========================================================================
    // Durable journal integration tests (terraphim-ai#3269)
    // =========================================================================

    fn precomplete_journal(journal: &mut Journal, process_id: ProcessId) {
        let completion_id = Uuid::new_v4();
        journal
            .append(JournalRecord {
                run_id: journal.run_id(),
                sequence: journal.next_sequence(),
                process_id: process_id.0,
                kind: OutputKind::Completed,
                payload: serde_json::json!({ "exit_code": Option::<i32>::None }),
                completion_id: Some(completion_id),
                observed_at: Utc::now(),
            })
            .expect("append pre-completion record");
        journal
            .complete(completion_id)
            .expect("pre-complete journal");
    }

    /// Spawn a silent, long-running child and return its piped
    /// stdout/stderr readers. The capture tasks stay idle on the open
    /// pipes for the lifetime of the test. `kill_on_drop` guarantees
    /// the child is reaped even when an assertion fails.
    fn spawn_silent_child() -> (
        tokio::process::Child,
        BufReader<ChildStdout>,
        BufReader<ChildStderr>,
    ) {
        let mut child = tokio::process::Command::new("sleep")
            .arg("300")
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .expect("spawn sleep helper");
        let stdout = BufReader::new(child.stdout.take().expect("child stdout piped"));
        let stderr = BufReader::new(child.stderr.take().expect("child stderr piped"));
        (child, stdout, stderr)
    }

    #[tokio::test]
    async fn test_stalled_broadcast_yields_exact_journal_records() {
        let root = TempDir::new().expect("tempdir");
        let run_id = Uuid::new_v4();
        let journal = Journal::create(root.path(), run_id).expect("create journal");

        let (mut child, stdout, stderr) = spawn_silent_child();
        let capture = OutputCapture::new_with_journal(ProcessId::new(), stdout, stderr, journal);

        // Subscribe and never drain: the broadcast receiver is stalled
        // for the whole run and must not affect durable journalling.
        let _stalled = capture.subscribe();

        const TOTAL: usize = 1500;
        for i in 0..TOTAL {
            capture
                .publish_event(OutputEvent::Stdout {
                    process_id: capture.process_id,
                    line: format!("line-{i}"),
                })
                .await
                .expect("publish should be journalled despite stalled broadcast");
        }

        // An active authoritative writer excludes recovery. Close and
        // join it before inspecting the durable stream.
        capture
            .fanout
            .durable_writer
            .as_ref()
            .expect("durable writer")
            .close()
            .await
            .expect("close writer");
        let recovered = Journal::recover_run(root.path(), run_id).expect("recover journal");
        assert_eq!(recovered.records.len(), TOTAL, "exact record count");
        for (i, record) in recovered.records.iter().enumerate() {
            assert_eq!(record.sequence, i as u64, "contiguous sequence");
            assert_eq!(record.run_id, run_id);
            assert_eq!(record.kind, OutputKind::Stdout);
            assert_eq!(
                record.payload,
                serde_json::Value::String(format!("line-{i}"))
            );
            assert_eq!(record.completion_id, None);
        }
        assert!(recovered.completion.is_none(), "run not completed");

        child.kill().await.expect("kill sleep helper");
    }

    #[tokio::test]
    async fn test_journal_persists_redacted_output() {
        let root = TempDir::new().expect("tempdir");
        let run_id = Uuid::new_v4();
        let journal = Journal::create(root.path(), run_id).expect("create journal");

        let (mut child, stdout, stderr) = spawn_silent_child();
        let capture = OutputCapture::new_with_journal(ProcessId::new(), stdout, stderr, journal);

        capture
            .publish_event(OutputEvent::Stdout {
                process_id: capture.process_id,
                line: "api_key=secret123 deploying".to_string(),
            })
            .await
            .expect("publish secret-bearing stdout");
        capture
            .publish_event(OutputEvent::Mention {
                process_id: capture.process_id,
                target: "kimiko".to_string(),
                message: "token=abc987xyz for @kimiko".to_string(),
            })
            .await
            .expect("publish secret-bearing mention");

        capture
            .fanout
            .durable_writer
            .as_ref()
            .expect("durable writer")
            .close()
            .await
            .expect("close writer");
        let recovered = Journal::recover_run(root.path(), run_id).expect("recover journal");
        assert_eq!(recovered.records.len(), 2);

        let stdout_payload = recovered.records[0]
            .payload
            .as_str()
            .expect("stdout string");
        assert!(stdout_payload.contains("***REDACTED***"));
        assert!(!stdout_payload.contains("secret123"));

        let mention = &recovered.records[1];
        assert_eq!(mention.kind, OutputKind::Mention);
        assert!(!mention.payload.to_string().contains("abc987xyz"));
        assert!(mention.payload.to_string().contains("***REDACTED***"));
        assert!(
            mention.payload.to_string().contains("kimiko"),
            "non-secret mention target survives redaction"
        );

        child.kill().await.expect("kill sleep helper");
    }

    #[tokio::test]
    async fn test_pre_completed_journal_rejects_publish_without_fanout() {
        let root = TempDir::new().expect("tempdir");
        let run_id = Uuid::new_v4();
        let mut journal = Journal::create(root.path(), run_id).expect("create journal");
        let process_id = ProcessId::new();
        precomplete_journal(&mut journal, process_id);

        let (mut child, stdout, stderr) = spawn_silent_child();
        let capture = OutputCapture::new_with_journal(process_id, stdout, stderr, journal);

        let mut broadcast_rx = capture.subscribe();

        let err = capture
            .publish_event(OutputEvent::Stdout {
                process_id: capture.process_id,
                line: "should not fan out".to_string(),
            })
            .await
            .expect_err("append after completion must fail");
        assert!(matches!(
            err,
            OutputJournalError::Journal(JournalError::DuplicateTerminal(_))
        ));

        // No fanout: nothing reached the buffer or broadcast.
        assert!(capture.captured_events().is_empty());
        match broadcast_rx.try_recv() {
            Err(tokio::sync::broadcast::error::TryRecvError::Empty) => {}
            other => panic!("expected empty broadcast, got {:?}", other.map(|_| ())),
        }

        // The on-disk journal is untouched apart from its terminal record/frame.
        let recovered = Journal::recover_run(root.path(), run_id).expect("recover journal");
        assert_eq!(recovered.records.len(), 1);
        assert_eq!(recovered.records[0].kind, OutputKind::Completed);
        assert!(recovered.completion.is_some());

        child.kill().await.expect("kill sleep helper");
    }

    #[tokio::test]
    async fn test_durable_writer_close_is_idempotent_and_leaves_journal_incomplete() {
        let root = TempDir::new().expect("tempdir");
        let run_id = Uuid::new_v4();
        let journal = Journal::create(root.path(), run_id).expect("create journal");
        let writer = DurableOutputWriter::spawn(journal, ProcessId::new());

        writer.close().await.expect("first close");
        writer.close().await.expect("second close is idempotent");

        let recovered = Journal::recover_run(root.path(), run_id).expect("recover journal");
        assert!(recovered.records.is_empty());
        assert!(recovered.completion.is_none());
    }

    #[tokio::test]
    async fn test_completion_remains_idempotent_after_writer_task_is_joined() {
        let root = TempDir::new().expect("tempdir");
        let run_id = Uuid::new_v4();
        let journal = Journal::create(root.path(), run_id).expect("create journal");
        let writer = DurableOutputWriter::spawn(journal, ProcessId::new());

        writer.complete(Some(0)).await.expect("complete and join");
        assert!(writer.task.lock().await.is_none(), "writer task was joined");
        writer
            .complete(Some(0))
            .await
            .expect("repeat after join is idempotent");
        let error = writer
            .complete(Some(1))
            .await
            .expect_err("different payload after join must conflict");
        assert!(matches!(
            error,
            OutputJournalError::TerminalPayloadConflict {
                expected: Some(1),
                found_kind: OutputKind::Completed,
                ..
            }
        ));
    }

    #[tokio::test]
    async fn test_reopened_completed_writer_rejects_conflicting_terminal_payload() {
        let root = TempDir::new().expect("tempdir");
        let run_id = Uuid::new_v4();
        let process_id = ProcessId::new();
        let mut journal = Journal::create(root.path(), run_id).expect("create journal");
        precomplete_journal(&mut journal, process_id);
        let writer = DurableOutputWriter::spawn(journal, process_id);

        let error = writer
            .complete(Some(1))
            .await
            .expect_err("sealed payload conflict must fail");
        assert!(matches!(
            error,
            OutputJournalError::TerminalPayloadConflict { .. }
        ));
        writer.close().await.expect("close conflicting writer");
        let recovered = Journal::recover_run(root.path(), run_id).expect("recover journal");
        assert_eq!(
            recovered.records[0].payload,
            serde_json::json!({"exit_code": Option::<i32>::None})
        );
    }

    #[tokio::test]
    async fn test_complete_retry_after_ack_cancellation_is_idempotent() {
        let root = TempDir::new().expect("tempdir");
        let run_id = Uuid::new_v4();
        let process_id = ProcessId::new();
        let journal = Journal::create(root.path(), run_id).expect("create journal");
        let path = journal.path().to_path_buf();
        let writer = DurableOutputWriter::spawn(journal, process_id);

        // Model cancellation after the task has durably processed Complete
        // but before DurableOutputWriter::complete updates lifecycle.status.
        let sender = {
            let lifecycle = writer.lifecycle.lock().await;
            lifecycle.tx.clone().expect("open writer sender")
        };
        let (ack_tx, ack_rx) = oneshot::channel();
        sender
            .send((WriterCommand::Complete { exit_code: Some(0) }, ack_tx))
            .await
            .expect("send first complete");
        ack_rx
            .await
            .expect("writer task alive")
            .expect("first complete durable");
        drop(sender);

        let sealed_len = std::fs::metadata(&path).expect("sealed metadata").len();
        writer
            .complete(Some(0))
            .await
            .expect("retry reconciles journal-owned completion record");
        assert_eq!(
            std::fs::metadata(&path).expect("retry metadata").len(),
            sealed_len,
            "retry must not append a second terminal frame"
        );

        let recovered = Journal::recover_run(root.path(), run_id).expect("recover journal");
        assert_eq!(recovered.records.len(), 1);
        assert_eq!(recovered.records[0].kind, OutputKind::Completed);
        assert_eq!(
            recovered.records[0].payload,
            serde_json::json!({"exit_code": 0})
        );
        assert!(recovered.completion.is_some());
    }

    #[tokio::test]
    async fn test_panicking_writer_surfaces_typed_join_error() {
        let root = TempDir::new().expect("tempdir");
        let journal = Journal::create(root.path(), Uuid::new_v4()).expect("create journal");
        let writer = DurableOutputWriter::spawn(journal, ProcessId::new());

        let error = writer
            .panic_for_test()
            .await
            .expect_err("writer panic must fail the command");
        assert!(matches!(error, OutputJournalError::WriterTask(error) if error.is_panic()));
    }

    #[tokio::test]
    async fn test_prepare_fallback_handoff_preserves_late_output_without_completion() {
        let root = TempDir::new().expect("tempdir");
        let run_id = Uuid::new_v4();
        let journal = Journal::create(root.path(), run_id).expect("create journal");
        let mut child = tokio::process::Command::new("sh")
            .arg("-c")
            .arg("sleep 0.05; echo late-output")
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .expect("spawn helper");
        let stdout = BufReader::new(child.stdout.take().expect("stdout"));
        let stderr = BufReader::new(child.stderr.take().expect("stderr"));
        let mut capture =
            OutputCapture::new_with_journal(ProcessId::new(), stdout, stderr, journal);

        capture
            .prepare_fallback_handoff()
            .await
            .expect("drain and close writer");

        let recovered = Journal::recover_run(root.path(), run_id).expect("recover journal");
        assert!(recovered.records.iter().any(|record| {
            record.kind == OutputKind::Stdout
                && record.payload == serde_json::Value::String("late-output".to_string())
        }));
        assert!(!recovered
            .records
            .iter()
            .any(|record| record.kind == OutputKind::Completed));
        assert!(recovered.completion.is_none());
        child.wait().await.expect("wait helper");
    }

    #[tokio::test]
    async fn test_seal_reopened_journal_uses_durable_writer_completion() {
        let root = TempDir::new().expect("tempdir");
        let run_id = Uuid::new_v4();
        drop(Journal::create(root.path(), run_id).expect("create journal"));
        let process_id = ProcessId::new();

        seal_reopened_journal(root.path(), run_id, process_id, Some(7))
            .await
            .expect("seal reopened journal");

        let recovered = Journal::recover_run(root.path(), run_id).expect("recover journal");
        assert_eq!(recovered.records.len(), 1);
        assert_eq!(recovered.records[0].process_id, process_id.0);
        assert_eq!(recovered.records[0].kind, OutputKind::Completed);
        assert_eq!(
            recovered.records[0].payload,
            serde_json::json!({ "exit_code": 7 })
        );
        assert!(recovered.completion.is_some());
    }

    #[tokio::test]
    async fn test_completion_is_durable_and_idempotent() {
        let root = TempDir::new().expect("tempdir");
        let run_id = Uuid::new_v4();
        let journal = Journal::create(root.path(), run_id).expect("create journal");

        let (mut child, stdout, stderr) = spawn_silent_child();
        let capture = OutputCapture::new_with_journal(ProcessId::new(), stdout, stderr, journal);

        capture
            .publish_event(OutputEvent::Stdout {
                process_id: capture.process_id,
                line: "final line".to_string(),
            })
            .await
            .expect("publish before completion");

        let mut terminal_rx = capture.subscribe();

        // The first call seals the journal before publishing the terminal event.
        capture.complete(Some(0)).await.expect("first complete");
        let recovered = Journal::recover_run(root.path(), run_id).expect("recover journal");
        assert!(
            recovered.completion.is_some(),
            "terminal frame is durable before terminal fanout is observed"
        );
        assert!(matches!(
            terminal_rx.recv().await.expect("terminal broadcast"),
            OutputEvent::Completed {
                exit_code: Some(0),
                ..
            }
        ));

        // Repeats must neither write nor fan out another terminal event.
        capture
            .complete(Some(0))
            .await
            .expect("second complete is idempotent");
        capture
            .complete(Some(0))
            .await
            .expect("third complete is idempotent");
        assert!(matches!(
            terminal_rx.try_recv(),
            Err(tokio::sync::broadcast::error::TryRecvError::Empty)
        ));

        // Successful recovery itself proves a single Complete frame
        // (recovery rejects duplicates). Check the durable shape.
        assert_eq!(recovered.records.len(), 2, "stdout + Completed records");

        let stdout_record = &recovered.records[0];
        assert_eq!(stdout_record.kind, OutputKind::Stdout);
        assert_eq!(stdout_record.completion_id, None);

        let completed = &recovered.records[1];
        assert_eq!(completed.kind, OutputKind::Completed);
        assert_eq!(completed.sequence, 1);
        assert_eq!(
            completed.payload,
            serde_json::json!({ "exit_code": 0 }),
            "exit code persisted on Completed record"
        );
        let completion_id = completed
            .completion_id
            .expect("Completed record carries completion id");

        let marker = recovered
            .completion
            .as_ref()
            .expect("Complete frame present");
        assert_eq!(
            marker.completion_id, completion_id,
            "stable completion id shared by record and terminal frame"
        );

        // Appends after completion keep failing without fanout.
        assert!(capture
            .publish_event(OutputEvent::Stdout {
                process_id: capture.process_id,
                line: "post-completion".to_string(),
            })
            .await
            .is_err());

        child.kill().await.expect("kill sleep helper");
    }

    // =========================================================================
    // Crash-resume tests (terraphim-ai#3269)
    //
    // Construct crash state by writing a completion-tagged Completed
    // record directly via `Journal::append` and dropping the journal
    // without calling `journal.complete()`. Reopen the journal,
    // attach a writer through `OutputCapture::new_with_journal`, and
    // assert the recovery contract: same exit code produces exactly
    // one record + one terminal marker; a different exit code surfaces
    // `TerminalPayloadConflict` and leaves the journal untouched.
    // =========================================================================

    /// Crash-resume happy path: a writer rebuilt against a journal
    /// that already holds a completion-tagged `Completed` record seals
    /// the journal using the recovered completion id, without writing
    /// a second `Completed` record. The on-disk shape is exactly one
    /// data record (the pre-existing terminal record) and one
    /// `Complete` frame marker.
    #[tokio::test]
    async fn test_crash_resume_completion_with_matching_exit_code() {
        let root = TempDir::new().expect("tempdir");
        let run_id = Uuid::new_v4();
        let completion_id = Uuid::new_v4();

        // Build the crash state: write a single completion-tagged
        // Completed record and drop the journal without sealing it.
        // This simulates a writer that crashed after appending the
        // terminal data record but before writing the Complete frame.
        {
            let mut journal = Journal::create(root.path(), run_id).expect("create journal");
            journal
                .append(JournalRecord {
                    run_id,
                    sequence: 0,
                    process_id: 42,
                    kind: OutputKind::Completed,
                    payload: serde_json::json!({ "exit_code": 0 }),
                    completion_id: Some(completion_id),
                    observed_at: Utc::now(),
                })
                .expect("append completion record");
            // Drop without `journal.complete(...)` — simulates crash.
        }

        // Reopen and create the writer. `run_writer` seeds itself
        // from the recovered journal: `next_sequence = 1`,
        // `completion_id = Some(completion_id)`, `completed = false`,
        // and the pending completion record is cloned into the
        // command loop so `Complete` can reconcile against it.
        let journal = Journal::open(root.path(), run_id).expect("reopen journal");
        let (mut child, stdout, stderr) = spawn_silent_child();
        let capture = OutputCapture::new_with_journal(ProcessId::new(), stdout, stderr, journal);

        // Same exit code → seal with the recovered completion id and
        // skip the redundant terminal record append.
        capture
            .complete(Some(0))
            .await
            .expect("complete with matching exit must seal");

        // On-disk shape: one record (the pre-existing terminal
        // record) and one marker (Complete frame written with the
        // matching completion id). No second Completed record was
        // appended.
        let recovered = Journal::recover_run(root.path(), run_id).expect("recover journal");
        assert_eq!(
            recovered.records.len(),
            1,
            "no extra Completed record should be written on match"
        );
        assert_eq!(recovered.records[0].kind, OutputKind::Completed);
        assert_eq!(recovered.records[0].sequence, 0);
        assert_eq!(recovered.records[0].completion_id, Some(completion_id));
        assert_eq!(
            recovered.records[0].payload,
            serde_json::json!({ "exit_code": 0 })
        );
        let marker = recovered
            .completion
            .as_ref()
            .expect("completion marker present after match");
        assert_eq!(
            marker.completion_id, completion_id,
            "sealed marker carries the recovered completion id"
        );

        // Idempotence: a second complete is a no-op (still 1 record,
        // 1 marker). This is the same shape-preservation guarantee the
        // non-resume path already provides, asserted end-to-end here.
        capture
            .complete(Some(0))
            .await
            .expect("second complete is idempotent");
        let recovered =
            Journal::recover_run(root.path(), run_id).expect("recover after second complete");
        assert_eq!(recovered.records.len(), 1, "no extra record on repeat");

        child.kill().await.expect("kill sleep helper");
    }

    /// Crash-resume mismatch path: a different exit code surfaces
    /// [`OutputJournalError::TerminalPayloadConflict`] and leaves the
    /// journal untouched (no new record, no terminal marker). The
    /// caller is responsible for reconciling the disagreement before
    /// retrying `complete()`.
    #[tokio::test]
    async fn test_crash_resume_completion_with_mismatched_exit_code() {
        let root = TempDir::new().expect("tempdir");
        let run_id = Uuid::new_v4();
        let completion_id = Uuid::new_v4();

        {
            let mut journal = Journal::create(root.path(), run_id).expect("create journal");
            journal
                .append(JournalRecord {
                    run_id,
                    sequence: 0,
                    process_id: 42,
                    kind: OutputKind::Completed,
                    payload: serde_json::json!({ "exit_code": 0 }),
                    completion_id: Some(completion_id),
                    observed_at: Utc::now(),
                })
                .expect("append completion record");
        }

        let journal = Journal::open(root.path(), run_id).expect("reopen journal");
        let (mut child, stdout, stderr) = spawn_silent_child();
        let capture = OutputCapture::new_with_journal(ProcessId::new(), stdout, stderr, journal);

        // Different exit code → TerminalPayloadConflict. The writer
        // refuses without touching disk so the recovered record
        // remains the source of truth.
        let err = capture
            .complete(Some(1))
            .await
            .expect_err("mismatched exit must surface TerminalPayloadConflict");
        assert!(
            matches!(err, OutputJournalError::TerminalPayloadConflict { .. }),
            "expected TerminalPayloadConflict, got: {err:?}"
        );
        assert!(matches!(
            Journal::recover_run(root.path(), run_id),
            Err(JournalError::WriterBusy { .. })
        ));
        capture
            .fanout
            .durable_writer
            .as_ref()
            .expect("durable writer")
            .close()
            .await
            .expect("close mismatched writer");

        // After deterministic ownership release, on-disk state is
        // unchanged: one record (the pre-existing terminal record), no
        // Complete marker, and no new record.
        let recovered = Journal::recover_run(root.path(), run_id).expect("recover journal");
        assert_eq!(
            recovered.records.len(),
            1,
            "no new record should be written on mismatch"
        );
        assert_eq!(recovered.records[0].kind, OutputKind::Completed);
        assert_eq!(
            recovered.records[0].payload,
            serde_json::json!({ "exit_code": 0 })
        );
        assert!(
            recovered.completion.is_none(),
            "no Complete marker should be written on mismatch"
        );

        child.kill().await.expect("kill sleep helper");
    }

    // =========================================================================
    // Cancellation-safety regression tests (terraphim-ai#3269)
    //
    // `OutputCapture::drain` must never remove a JoinHandle from
    // `capture_tasks` before its awaited result has been observed. A
    // caller may wrap `drain`/`finish` in a timeout that cancels the
    // future mid-await; the retry must resume the retained capture
    // tasks instead of finding an empty vec and skipping the drain.
    // =========================================================================

    /// A cancelled `finish` must not lose late output or completion.
    ///
    /// The child sleeps ~200ms before writing `late-output` and closing
    /// its stdout pipe, so a 20ms `finish` under timeout is cancelled
    /// while the capture task is parked on `read_line`. The retry must
    /// resume the retained capture tasks: `late-output` is journalled
    /// and the journal is sealed exactly once.
    #[tokio::test]
    async fn test_finish_retry_after_cancellation_preserves_late_output_and_completion() {
        let root = TempDir::new().expect("tempdir");
        let run_id = Uuid::new_v4();
        let journal = Journal::create(root.path(), run_id).expect("create journal");

        // Child sleeps long enough that the first `finish` (bounded by
        // a 20ms timeout) is cancelled while the pipe is still open.
        let mut child = tokio::process::Command::new("sh")
            .arg("-c")
            .arg("sleep 0.2; echo late-output")
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .expect("spawn sh helper");
        let stdout = BufReader::new(child.stdout.take().expect("child stdout piped"));
        let stderr = BufReader::new(child.stderr.take().expect("child stderr piped"));

        let mut capture =
            OutputCapture::new_with_journal(ProcessId::new(), stdout, stderr, journal);

        // Cancel the first finish mid-drain. The capture task is still
        // blocked on the open pipe (child sleeping), so the timeout
        // must fire and drop the `finish` future.
        let cancelled = tokio::time::timeout(
            std::time::Duration::from_millis(20),
            capture.finish(Some(0)),
        )
        .await;
        assert!(
            cancelled.is_err(),
            "first finish must be cancelled by the short timeout while the pipe is open"
        );

        // Retry: the retained capture tasks must drain to EOF so the
        // late line is durably journalled before the seal.
        capture
            .finish(Some(0))
            .await
            .expect("retry finish must drain the retained capture tasks and seal");

        let recovered = Journal::recover_run(root.path(), run_id).expect("recover journal");
        assert!(
            recovered.records.iter().any(|record| {
                record.kind == OutputKind::Stdout
                    && record
                        .payload
                        .as_str()
                        .is_some_and(|s| s.contains("late-output"))
            }),
            "journal must contain the late-output line drained after the retry, got: {:?}",
            recovered.records
        );
        let completed_records: Vec<&JournalRecord> = recovered
            .records
            .iter()
            .filter(|record| record.kind == OutputKind::Completed)
            .collect();
        assert_eq!(
            completed_records.len(),
            1,
            "journal must be sealed exactly once, got: {:?}",
            completed_records
        );
        assert!(
            recovered.completion.is_some(),
            "journal completion marker must be durable after the retry"
        );

        child.wait().await.expect("wait sh helper");
    }

    /// A cancelled `finish` must not lose a capture failure either.
    ///
    /// The journal is pre-completed so the capture task's publish is
    /// rejected with `DuplicateTerminal`. The first `finish` is
    /// cancelled by a short timeout while the child still sleeps; the
    /// retry must surface the retained capture task's journal error
    /// instead of reporting success over an empty task list.
    #[tokio::test]
    async fn test_finish_retry_after_cancellation_surfaces_capture_failure() {
        let root = TempDir::new().expect("tempdir");
        let run_id = Uuid::new_v4();

        // Pre-complete the journal so every append is rejected.
        let mut journal = Journal::create(root.path(), run_id).expect("create journal");
        precomplete_journal(&mut journal, ProcessId::new());

        // Child sleeps past the 20ms timeout, then writes to stderr.
        let mut child = tokio::process::Command::new("sh")
            .arg("-c")
            .arg("sleep 0.2; echo boom >&2")
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .expect("spawn sh helper");
        let stdout = BufReader::new(child.stdout.take().expect("child stdout piped"));
        let stderr = BufReader::new(child.stderr.take().expect("child stderr piped"));

        let mut capture =
            OutputCapture::new_with_journal(ProcessId::new(), stdout, stderr, journal);

        // Cancel the first finish mid-drain while the pipe is open.
        let cancelled = tokio::time::timeout(
            std::time::Duration::from_millis(20),
            capture.finish(Some(0)),
        )
        .await;
        assert!(
            cancelled.is_err(),
            "first finish must be cancelled by the short timeout while the pipe is open"
        );

        // Retry: the retained stderr capture task must observe the
        // journal rejection and `finish` must propagate it — the
        // capture failure must not be silently dropped with a
        // cancelled drain.
        let err = capture
            .finish(Some(0))
            .await
            .expect_err("retry finish must surface the retained capture failure");
        assert!(
            matches!(
                err,
                OutputJournalError::Journal(JournalError::DuplicateTerminal(_))
            ),
            "expected Journal(DuplicateTerminal(_)), got: {err:?}"
        );

        child.wait().await.expect("wait sh helper");
    }

    /// End-to-end: a pre-completed journal refuses every publish, the
    /// stderr capture task surfaces [`OutputJournalError::Journal`]
    /// wrapping [`JournalError::DuplicateTerminal`], and `finish`
    /// propagates that error. No fanout happens: the in-memory buffer
    /// stays empty, no broadcast receiver observes an event, and the
    /// post-journal redacted stderr mirror file is never written.
    #[tokio::test]
    async fn test_finish_surfaces_journal_duplicate_terminal_when_capture_task_fails() {
        let root = TempDir::new().expect("tempdir");
        let run_id = Uuid::new_v4();
        let stderr_mirror = root.path().join("stderr-mirror.log");

        // Pre-complete the journal so every append is rejected.
        let mut journal = Journal::create(root.path(), run_id).expect("create journal");
        precomplete_journal(&mut journal, ProcessId::new());

        // Spawn `sh -c` that writes one secret-bearing stderr line and
        // exits with a non-zero status. `kill_on_drop` guarantees the
        // child is reaped even if an assertion below panics.
        let mut child = tokio::process::Command::new("sh")
            .arg("-c")
            .arg("echo 'api_key=secret123' >&2; exit 7")
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .expect("spawn sh helper");
        let stdout = BufReader::new(child.stdout.take().expect("child stdout piped"));
        let stderr = BufReader::new(child.stderr.take().expect("child stderr piped"));

        let mut capture = OutputCapture::new_with_journal_and_stderr_log(
            ProcessId::new(),
            stdout,
            stderr,
            journal,
            Some(stderr_mirror.clone()),
        );
        let mut broadcast_rx = capture.subscribe();

        // Wait for the child to exit so its stderr pipe is closed and
        // the capture task observes EOF after the publish failure.
        let status = child.wait().await.expect("wait sh helper");
        assert!(!status.success(), "test child exits with code 7");

        // finish must surface the journal rejection from the stderr
        // capture task. It returns before calling `complete` because
        // `first_error` is set on the first iteration of the drain.
        let err = capture
            .finish(status.code())
            .await
            .expect_err("finish must surface DuplicateTerminal from capture task");
        assert!(
            matches!(
                err,
                OutputJournalError::Journal(JournalError::DuplicateTerminal(_))
            ),
            "expected Journal(DuplicateTerminal(_)), got: {err:?}"
        );

        // No in-memory fanout: the journal rejection short-circuited
        // the publish path before `record_event`, `event_sender.send`,
        // and `broadcast_sender.send` could run.
        assert!(
            capture.captured_events().is_empty(),
            "captured_events must be empty when journal rejects publish"
        );
        match broadcast_rx.try_recv() {
            Err(tokio::sync::broadcast::error::TryRecvError::Empty) => {}
            other => panic!("expected empty broadcast, got {:?}", other.map(|_| ())),
        }

        // The post-journal redacted stderr mirror file must not hold
        // the secret — the capture task wrote the mirror line *after*
        // a successful publish, so the failed publish prevents any
        // mirror write from happening.
        let mirror_exists = stderr_mirror.exists();
        if mirror_exists {
            let contents = std::fs::read_to_string(&stderr_mirror).expect("read mirror");
            assert!(
                contents.is_empty(),
                "stderr mirror must be empty after journal rejection; got {contents:?}"
            );
            assert!(
                !contents.contains("secret123"),
                "stderr mirror must not contain raw secret"
            );
        }
    }
}
