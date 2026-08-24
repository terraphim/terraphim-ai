//! Per-run durable journal for agent output (terraphim-ai#3269).
//!
//! Each spawned agent run gets a single journal file under a known root
//! directory. The file records every output event the agent produced
//! (stdout, stderr, mentions, heartbeats) as length-delimited SHA-256
//! checksummed JSON frames.
//!
//! Design intent (slice 1 of the journal work):
//!
//! * **Durable** — every append is `flush()`'d; the terminal `Complete`
//!   frame is followed by `sync_data()`. A torn tail is recovered by
//!   truncating back to the last verified frame boundary, so a partial
//!   write never poisons the file.
//! * **Tamper-evident** — each frame carries a SHA-256 over its JSON
//!   payload. Interior corruption, accidental rewrites or swapped frames
//!   are detected at recovery and surface as
//!   [`JournalError::ChecksumMismatch`] or
//!   [`JournalError::SequenceMismatch`].
//! * **Per-run permissioned** — journal files are created with mode
//!   `0600`, never follow symlinks, and reject run IDs that try to escape
//!   the supplied root.
//! * **Retention-aware** — a journal becomes eligible for deletion only
//!   when both a `Complete` and an `Ack` frame are present, **and** the
//!   acknowledgement is older than a caller-supplied duration.
//!
//! This module is intentionally synchronous and self-contained — it does
//! **not** integrate with [`crate::OutputCapture`] yet. Slice 2 will
//! stream `OutputEvent`s into a `Journal`; today the only writer is the
//! explicit `Journal::append` API and tests.

use std::fs::File;
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Component, Path, PathBuf};

#[cfg(unix)]
use std::ffi::OsString;
#[cfg(unix)]
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd, RawFd};

#[cfg(unix)]
use nix::errno::Errno;
#[cfg(unix)]
use nix::fcntl::{flock, open, openat, FlockArg, OFlag};
#[cfg(unix)]
use nix::sys::stat::{fstat, mkdirat, Mode, SFlag};
#[cfg(unix)]
use nix::unistd::{fsync, unlinkat, UnlinkatFlags};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;
use uuid::Uuid;

/// File permission for every journal file. Owner read+write only.
#[cfg(unix)]
const JOURNAL_FILE_MODE: u32 = 0o600;

/// Magic prefix at the start of every frame header.
const FRAME_MAGIC: [u8; 4] = *b"TPJ2";

/// Wire version of the journal frame format. Bump if `JournalFrame`
/// changes shape in an incompatible way.
const FRAME_VERSION: u8 = 0x02;

/// Authenticated header metadata: magic(4) + version(1) + length(4 BE).
const HEADER_METADATA_LEN: usize = 4 + 1 + 4;

/// Header layout: metadata(9) + metadata SHA-256(32) + payload SHA-256(32).
const HEADER_LEN: usize = HEADER_METADATA_LEN + 32 + 32;

/// Maximum single-frame payload size. 16 MiB comfortably holds a
/// long stdout line or a structured mention blob while still bounding
/// memory if a corrupt length prefix lies about the real size.
const MAX_FRAME_PAYLOAD: usize = 16 * 1024 * 1024;

/// Categories of output recorded into a [`JournalRecord`].
///
/// The variant is part of the durable wire format; new variants must be
/// added at the end so older readers keep working.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OutputKind {
    /// Redacted stdout line.
    Stdout,
    /// Redacted stderr line.
    Stderr,
    /// Mention event already routed to a target.
    Mention,
    /// Heartbeat / health probe marker.
    Heartbeat,
    /// Terminal completion frame (signals end-of-output).
    ///
    /// A `Completed` `JournalRecord` is not itself a [`JournalFrame`]
    /// — terminal finalisation is carried by [`JournalFrame::Complete`].
    /// This variant exists so consumers can tag the last data record
    /// before the `Complete` frame is written.
    Completed,
}

/// One durable record appended to the journal.
///
/// `payload` is whatever redacted, serializable representation the
/// caller wants persisted (most often a UTF-8 string under
/// `serde_json::Value::String`). The journal treats it as opaque.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JournalRecord {
    /// Unique identifier of the agent run this record belongs to.
    pub run_id: Uuid,
    /// Monotonically increasing per-run sequence number. The writer
    /// enforces strict contiguity starting at zero.
    pub sequence: u64,
    /// OS process id of the spawned agent (see [`crate::ProcessId`]).
    pub process_id: u64,
    /// Output category the record describes.
    pub kind: OutputKind,
    /// Already-redacted payload. Opaque to the journal.
    pub payload: serde_json::Value,
    /// Completion id present on the final data record of a run. Set on
    /// the record immediately preceding [`JournalFrame::Complete`] so
    /// readers can correlate without scanning the full log.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub completion_id: Option<Uuid>,
    /// When the underlying event was observed (UTC).
    pub observed_at: DateTime<Utc>,
}

/// All possible frames in the journal wire format.
///
/// Frames are JSON-encoded and self-describing — the `tag` field tells
/// readers which struct to deserialise into without a separate index.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "tag", rename_all = "snake_case")]
pub enum JournalFrame {
    /// A regular output record.
    Data(JournalRecord),
    /// Terminal completion frame, written once and only once per run.
    /// Carries the run-scoped completion id so downstream consumers can
    /// dedupe across restarts.
    Complete {
        run_id: Uuid,
        completion_id: Uuid,
        observed_at: DateTime<Utc>,
    },
    /// Acknowledgement frame, written after consumers have durably
    /// ingested the journal. Combined with [`JournalFrame::Complete`]
    /// it gates retention-eligibility.
    Ack {
        run_id: Uuid,
        observed_at: DateTime<Utc>,
    },
}

/// Snapshot returned by [`Journal::recover_run`].
///
/// Always carries the canonical run id the recovery was performed
/// against, even when the on-disk file holds no records yet. Callers
/// must use [`Self::run_id`] to populate `JournalCheckpoint.run_id`
/// and to gate `records_after` — there is no `Uuid::nil` sentinel for
/// the "empty journal" case.
#[derive(Debug, Clone, PartialEq)]
pub struct RecoveredJournal {
    /// Run id the journal belongs to. Equal to the `expected_run_id`
    /// passed to `recover_full`, and authoritative for every
    /// `checkpoint` / `records_after` check on this snapshot.
    pub run_id: Uuid,
    /// All data records in order, with strictly contiguous `sequence`
    /// numbers starting at zero. Empty for a freshly-created journal
    /// that has not yet recorded any output.
    pub records: Vec<JournalRecord>,
    /// Presence of a terminal completion frame, with its id.
    pub completion: Option<CompleteMarker>,
    /// Presence of an acknowledgement frame, with its timestamp.
    pub ack: Option<AckMarker>,
}

impl RecoveredJournal {
    /// Snapshot of a consumer's progress through this journal.
    ///
    /// Returned by [`RecoveredJournal::checkpoint`] and intended for
    /// serialisation between processes: the caller persists the
    /// checkpoint, restarts the consumer, and feeds the checkpoint
    /// back to [`RecoveredJournal::records_after`] to resume.
    ///
    /// `last_sequence` is the sequence number of the last record the
    /// consumer has durably ingested. `None` means "consumed nothing
    /// yet".
    pub fn checkpoint(&self) -> JournalCheckpoint {
        JournalCheckpoint {
            run_id: self.run_id,
            last_sequence: self.records.last().map(|r| r.sequence),
            completion_id: self.completion.as_ref().map(|c| c.completion_id),
        }
    }

    /// Return records strictly after `checkpoint.last_sequence`, bounded
    /// by `max_records`, with no gaps.
    ///
    /// Validation:
    /// * `checkpoint.run_id` must match the run that produced these
    ///   records (no cross-run consumption).
    /// * `checkpoint.completion_id`, if `Some`, must match the
    ///   recovered journal's completion id (no stale checkpoints).
    /// * `checkpoint.last_sequence`, if `Some`, must name a record that
    ///   exists *and* whose `sequence` agrees with the cursor. The
    ///   next record is then the one at `last_sequence + 1`.
    /// * `max_records` must be greater than zero.
    ///
    /// On success, returns at most `max_records` records in order. If
    /// the cursor has already reached the tail, returns an empty
    /// slice.
    pub fn records_after(
        &self,
        checkpoint: &JournalCheckpoint,
        max_records: usize,
    ) -> Result<Vec<JournalRecord>, JournalError> {
        if max_records == 0 {
            return Err(JournalError::InvalidState(
                "records_after requires max_records > 0".to_string(),
            ));
        }
        // Run identity gate. Always enforced against the recovered
        // journal's run id — there is no nil-sentinel path for the
        // "empty records" case.
        if self.run_id != checkpoint.run_id {
            return Err(JournalError::RunIdMismatch {
                journal: self.run_id,
                frame: checkpoint.run_id,
            });
        }
        // Completion identity gate.
        match (
            checkpoint.completion_id,
            self.completion.as_ref().map(|c| c.completion_id),
        ) {
            (Some(want), Some(have)) if want != have => {
                return Err(JournalError::CompletionConflict {
                    expected: Some(have),
                    found: Some(want),
                });
            }
            (Some(want), None) => {
                return Err(JournalError::CompletionConflict {
                    expected: None,
                    found: Some(want),
                });
            }
            _ => {}
        }
        // Cursor gate.
        let start = match checkpoint.last_sequence {
            None => 0,
            Some(seq) => {
                let idx = seq as usize;
                if idx >= self.records.len() {
                    return Err(JournalError::SequenceMismatch {
                        expected: self.records.len().saturating_sub(1) as u64,
                        got: seq,
                    });
                }
                if self.records[idx].sequence != seq {
                    return Err(JournalError::SequenceMismatch {
                        expected: seq,
                        got: self.records[idx].sequence,
                    });
                }
                idx + 1
            }
        };
        let end = start.saturating_add(max_records).min(self.records.len());
        Ok(self.records[start..end].to_vec())
    }
}

/// Terminal completion marker recovered from the journal.
#[derive(Debug, Clone, PartialEq)]
pub struct CompleteMarker {
    /// Run-scoped completion id copied verbatim from the on-disk
    /// `Complete` frame. Pairs with `JournalRecord::completion_id` (when
    /// stamped on the final data record) and with
    /// `JournalCheckpoint::completion_id` (when a consumer has witnessed
    /// the completion).
    pub completion_id: Uuid,
    /// Wall-clock UTC timestamp recorded when the `Complete` frame was
    /// durably written. Used by retention policies that need to reason
    /// about completion age.
    pub observed_at: DateTime<Utc>,
}

/// Acknowledgement marker recovered from the journal.
#[derive(Debug, Clone, PartialEq)]
pub struct AckMarker {
    /// Wall-clock UTC timestamp recorded when the `Ack` frame was
    /// durably written. Retention policies gate eligibility on the age
    /// of this timestamp relative to a caller-supplied `now`.
    pub observed_at: DateTime<Utc>,
}

/// Serialisable snapshot of a consumer's progress through a single run.
///
/// Produced by [`RecoveredJournal::checkpoint`] and consumed by
/// [`RecoveredJournal::records_after`]. The fields are stable across
/// process restarts so a long-running consumer can persist a
/// checkpoint, crash, and resume from the exact byte boundary without
/// skipping records or risking replay of already-ingested data.
///
/// `last_sequence` is the `sequence` number of the most recent record
/// the consumer has durably ingested; `None` means "consumed nothing
/// yet". `completion_id`, when `Some`, lets a consumer detect stale
/// checkpoints that point at a different completion than the one on
/// disk.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JournalCheckpoint {
    /// Run this checkpoint belongs to. Used to reject cross-run
    /// consumption.
    pub run_id: Uuid,
    /// Sequence number of the last record already ingested.
    pub last_sequence: Option<u64>,
    /// Completion id the consumer has witnessed, if any.
    pub completion_id: Option<Uuid>,
}

/// Resource bounds applied while recovering a journal from disk.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RecoveryLimits {
    /// Maximum journal file size accepted for recovery.
    pub max_bytes: u64,
    /// Maximum number of data records materialized by recovery.
    pub max_records: usize,
}

impl Default for RecoveryLimits {
    fn default() -> Self {
        Self {
            max_bytes: 64 * 1024 * 1024,
            max_records: 100_000,
        }
    }
}

/// Errors the journal can surface.
#[derive(Debug, Error)]
pub enum JournalError {
    /// Underlying I/O failure.
    #[error("journal I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// A frame failed SHA-256 verification (interior corruption, partial
    /// overwrite, or wrong frame boundary).
    #[error("journal checksum mismatch at byte offset {offset}")]
    ChecksumMismatch { offset: u64 },

    /// A frame header was malformed (bad magic, unknown version,
    /// impossible length).
    #[error("malformed journal frame header: {0}")]
    MalformedHeader(String),

    /// Frame JSON failed to deserialise.
    #[error("journal frame JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// Sequence number was not strictly contiguous from the prior
    /// record.
    #[error("journal sequence mismatch: expected {expected}, got {got}")]
    SequenceMismatch { expected: u64, got: u64 },

    /// `run_id` on a frame did not match the journal's run id.
    #[error("journal run id mismatch: journal={journal}, frame={frame}")]
    RunIdMismatch { journal: Uuid, frame: Uuid },

    /// More than one terminal marker was written where only one is
    /// allowed (duplicate completion or duplicate acknowledgement).
    #[error("{0}")]
    DuplicateTerminal(&'static str),

    /// Completion identity disagreement: a `JournalRecord`'s
    /// `completion_id`, the on-disk `Complete` frame's id, or a
    /// checkpoint's `completion_id` did not match.
    #[error("completion id mismatch: expected {expected:?}, got {found:?}")]
    CompletionConflict {
        expected: Option<Uuid>,
        found: Option<Uuid>,
    },

    /// A terminal operation was attempted in the wrong journal state
    /// (for example, acknowledging a journal that has not been
    /// completed).
    #[error("invalid journal state: {0}")]
    InvalidState(String),

    /// Caller supplied a path that failed the safety check (symlink,
    /// `..` component, outside the configured root).
    #[error("unsafe journal path: {0}")]
    UnsafePath(String),

    /// Durable descriptor-relative journals require Unix filesystem APIs.
    #[error("durable journals are unsupported on this platform")]
    UnsupportedPlatform,

    /// Another process or object currently owns the journal writer lock.
    #[error("journal writer already active for run {run_id} at {path}")]
    WriterBusy { run_id: Uuid, path: PathBuf },

    /// Frame size exceeded `MAX_FRAME_PAYLOAD`.
    #[error("journal frame too large: {size} bytes")]
    FrameTooLarge { size: usize },

    /// Journal file size exceeded the configured recovery byte limit.
    #[error("journal recovery byte limit exceeded: {size} bytes (limit {max_bytes})")]
    RecoveryByteLimitExceeded { size: u64, max_bytes: u64 },

    /// Journal contained more data records than recovery permits.
    #[error("journal recovery record limit exceeded: {records} records (limit {max_records})")]
    RecoveryRecordLimitExceeded { records: usize, max_records: usize },

    /// A previous mutation failed at the IO boundary
    /// (`write_frame`, `flush`, or `sync_data`); the journal is in an
    /// indeterminate on-disk state and refuses every subsequent
    /// `append` / `complete` / `acknowledge` call. Callers must discard
    /// the [`Journal`] and recover from on-disk state rather than
    /// retrying, since the bytes that did reach the file are no longer
    /// guaranteed to align with what would have been written on a
    /// successful run.
    #[error("journal is poisoned: a prior mutation failed at the IO boundary")]
    Poisoned,
}

/// Test-only descriptor of an IO boundary at which the next matching
/// call must fail. Used by the `cfg(test)` poison-injection helpers to
/// drive the journal through every error path deterministically. Has
/// no runtime presence in production builds.
#[cfg(test)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TestFailurePoint {
    /// Fail the next call to the `write_frame` boundary.
    Write,
    /// Fail the next call to the `File::flush` boundary.
    Flush,
    /// Fail the next call to the `File::sync_data` boundary.
    Sync,
}

#[cfg(all(test, unix))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TestDurabilityFailurePoint {
    DirectoryEntry,
    LeafData,
    LeafParent,
    Truncate,
}

#[cfg(all(test, unix))]
type RootWalkHook = Option<(PathBuf, Box<dyn FnOnce()>)>;

#[cfg(all(test, unix))]
std::thread_local! {
    static DURABILITY_FAILURE: std::cell::Cell<Option<TestDurabilityFailurePoint>> = const { std::cell::Cell::new(None) };
    static ROOT_WALK_HOOK: std::cell::RefCell<RootWalkHook> = std::cell::RefCell::new(None);
}

#[cfg(all(test, unix))]
fn inject_durability_failure(point: TestDurabilityFailurePoint) {
    DURABILITY_FAILURE.with(|slot| slot.set(Some(point)));
}

#[cfg(all(test, unix))]
fn clear_durability_failure() {
    DURABILITY_FAILURE.with(|slot| slot.set(None));
}

#[cfg(all(test, unix))]
fn maybe_inject_durability_failure(point: TestDurabilityFailurePoint) -> Result<(), JournalError> {
    DURABILITY_FAILURE.with(|slot| {
        if slot.get() == Some(point) {
            slot.set(None);
            Err(JournalError::Io(std::io::Error::other(format!(
                "injected durability failure at {point:?}"
            ))))
        } else {
            Ok(())
        }
    })
}

#[cfg(all(test, unix))]
fn install_root_walk_hook(path: PathBuf, hook: impl FnOnce() + 'static) {
    ROOT_WALK_HOOK.with(|slot| *slot.borrow_mut() = Some((path, Box::new(hook))));
}

#[cfg(all(test, unix))]
fn clear_root_walk_hook() {
    ROOT_WALK_HOOK.with(|slot| *slot.borrow_mut() = None);
}

#[cfg(all(test, unix))]
fn run_root_walk_hook(path: &Path) {
    ROOT_WALK_HOOK.with(|slot| {
        let pending = slot.borrow_mut().take();
        match pending {
            Some((target, hook)) if target == path => hook(),
            other => *slot.borrow_mut() = other,
        }
    });
}

/// A single open, append-only journal file.
///
/// Construct via [`Journal::create`] or [`Journal::open`]; one instance
/// per agent run. The type owns an open [`File`] and exposes all
/// mutation methods through `&mut self`. If the journal must be
/// reachable from more than one thread, callers serialise access
/// themselves (e.g. a `Mutex<HashMap<Uuid, Journal>>`).
#[derive(Debug)]
pub struct Journal {
    run_id: Uuid,
    path: PathBuf,
    file: File,
    /// Next expected `sequence` value for an `append` call.
    next_sequence: u64,
    /// Run-scoped completion id, populated from the terminal `Complete`
    /// frame at [`Journal::open`] time, or set by [`Journal::complete`].
    /// `None` while the journal is still in-flight.
    completion_id: Option<Uuid>,
    /// First `Some(completion_id)` observed on a `JournalRecord`
    /// (either via [`Journal::open`] recovery or an in-flight
    /// [`Journal::append`]). Used to fail-fast on a record-vs-record
    /// disagreement before the mismatch is durably written.
    record_completion_id: Option<Uuid>,
    /// The unique completion-tagged [`JournalRecord`] observed on this
    /// journal — either recovered from disk by [`Journal::open`] from
    /// the unique completion-tagged record, or installed by a
    /// successful [`Journal::append`] whose `completion_id` is `Some`.
    ///
    /// Tagged records must carry `kind == OutputKind::Completed`;
    /// other kinds are rejected at `append` time. The journal holds at
    /// most one such record. Even after [`Self::completed`] flips to
    /// `true` the slot remains populated, but the
    /// [`Journal::pending_completion_record`] getter hides it because
    /// it is no longer "pending".
    completion_record: Option<JournalRecord>,
    /// Set once a terminal completion frame has been written.
    completed: bool,
    /// Set once an acknowledgement frame has been written.
    acknowledged: bool,
    /// Set to `true` when the previous mutation's IO boundary
    /// (`write_frame`, `flush`, or `sync_data`) returned an error to
    /// the OS — or, in tests, when a [`TestFailurePoint`] was set on
    /// that boundary. Once poisoned, every subsequent
    /// `append` / `complete` / `acknowledge` call returns
    /// [`JournalError::Poisoned`] without touching the file so callers
    /// cannot silently pile more writes on top of an indeterminate
    /// on-disk state.
    poisoned: bool,
    /// Whether this object currently owns the kernel exclusive lock.
    writer_lock_held: bool,
    /// Bounds used whenever this handle must refresh state after
    /// reacquiring ownership (for example before acknowledgement).
    recovery_limits: RecoveryLimits,
    /// Test-only: when `Some(boundary)`, the next call routed through
    /// the matching poison-guarded helper raises a synthetic I/O error
    /// and trips [`Self::poisoned`]. Cleared on first hit so the rest
    /// of the test can inspect the journal in its poisoned state.
    #[cfg(test)]
    fail_at: Option<TestFailurePoint>,
}

impl Journal {
    /// Create a new journal file for `run_id` under `root`.
    ///
    /// The journal leaf is created descriptor-relatively with
    /// `openat(O_CREAT | O_EXCL | O_NOFOLLOW)` and mode `0600`, so there
    /// is no permissive-mode or symlink-following window.
    pub fn create(root: &Path, run_id: Uuid) -> Result<Self, JournalError> {
        #[cfg(not(unix))]
        {
            let _ = (root, run_id);
            return Err(JournalError::UnsupportedPlatform);
        }
        #[cfg(unix)]
        {
            let opened_root = open_root_dir(root, true)?;
            let path = journal_path(&opened_root.path, run_id)?;
            let file = open_journal_leaf(&opened_root, run_id, true)?;
            acquire_writer_lock(&file, run_id, &path)?;
            if let Err(error) = prepare_journal_file(&file, true, &path)
                .and_then(|()| sync_new_leaf(&file))
                .and_then(|()| sync_leaf_parent(opened_root.fd.as_raw_fd()))
            {
                drop(file);
                let _ = unlinkat(
                    Some(opened_root.fd.as_raw_fd()),
                    journal_filename(run_id).as_str(),
                    UnlinkatFlags::NoRemoveDir,
                );
                let _ = fsync_fd(opened_root.fd.as_raw_fd());
                return Err(error);
            }

            Ok(Self {
                run_id,
                path,
                file,
                next_sequence: 0,
                completion_id: None,
                record_completion_id: None,
                completion_record: None,
                completed: false,
                acknowledged: false,
                poisoned: false,
                writer_lock_held: true,
                recovery_limits: RecoveryLimits::default(),
                #[cfg(test)]
                fail_at: None,
            })
        }
    }

    /// Open an existing journal for further appends.
    ///
    /// Recovery is performed immediately so `next_sequence`, `completed`,
    /// `completion_id` and `acknowledged` reflect the on-disk state.
    pub fn open(root: &Path, run_id: Uuid) -> Result<Self, JournalError> {
        Self::open_with_limits(root, run_id, RecoveryLimits::default())
    }

    /// Open an existing journal using explicit recovery resource bounds.
    pub fn open_with_limits(
        root: &Path,
        run_id: Uuid,
        limits: RecoveryLimits,
    ) -> Result<Self, JournalError> {
        #[cfg(not(unix))]
        {
            let _ = (root, run_id, limits);
            return Err(JournalError::UnsupportedPlatform);
        }
        #[cfg(unix)]
        {
            let opened_root = open_root_dir(root, false)?;
            let path = journal_path(&opened_root.path, run_id)?;
            let mut file = open_journal_leaf(&opened_root, run_id, false)?;
            acquire_writer_lock(&file, run_id, &path)?;
            prepare_journal_file(&file, false, &path)?;

            file.seek(SeekFrom::Start(0))?;
            let snapshot = recover_full(&mut file, run_id, limits)?;
            let next_sequence = snapshot.records.len() as u64;
            let completed = snapshot.completion.is_some();
            let completion_id = snapshot.completion.as_ref().map(|c| c.completion_id);
            let record_completion_id = snapshot.records.iter().find_map(|r| r.completion_id);
            let completion_record = snapshot
                .records
                .iter()
                .find(|r| r.completion_id.is_some())
                .cloned();
            let acknowledged = snapshot.ack.is_some();
            if completed {
                release_writer_lock_fd(&file)?;
            }

            Ok(Self {
                run_id,
                path,
                file,
                next_sequence,
                completion_id,
                record_completion_id,
                completion_record,
                completed,
                acknowledged,
                poisoned: false,
                writer_lock_held: !completed,
                recovery_limits: limits,
                #[cfg(test)]
                fail_at: None,
            })
        }
    }

    /// Run id the journal was created for.
    pub fn run_id(&self) -> Uuid {
        self.run_id
    }

    /// Absolute path of the journal file on disk.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Next expected `sequence` value for the next [`Journal::append`]
    /// call.
    ///
    /// Strictly monotonic. Equals the count of data records already
    /// durably written to this journal — zero on a freshly-created or
    /// freshly-recovered empty journal, `N` after `N` successful
    /// appends.
    pub fn next_sequence(&self) -> u64 {
        self.next_sequence
    }

    /// Whether the terminal `Complete` frame has been written for this
    /// journal.
    ///
    /// Once `true`, the journal refuses further [`Journal::append`]
    /// calls and its completion identity is sealed. The companion
    /// [`Journal::completion_identity`] getter then returns the id
    /// carried by the `Complete` frame rather than the id stamped on
    /// any pending completion-tagged data record.
    pub fn is_completed(&self) -> bool {
        self.completed
    }

    /// Run-scoped completion identity of this journal, if any.
    ///
    /// Resolution order:
    /// * If [`Self::is_completed`] is `true`, returns the id from the
    ///   terminal `Complete` frame — the canonical, sealed value.
    /// * Otherwise, if a completion-tagged data record is pending,
    ///   returns the id stamped on that record — the in-flight promise
    ///   of what the terminal marker will be.
    /// * Returns `None` when neither has been observed.
    pub fn completion_identity(&self) -> Option<Uuid> {
        if let Some(id) = self.completion_id {
            return Some(id);
        }
        self.completion_record
            .as_ref()
            .and_then(|r| r.completion_id)
    }

    /// The unique completion-tagged [`JournalRecord`] observed on this
    /// journal, if it is still "pending".
    ///
    /// Returns `Some` only *before* [`Journal::complete`] has been
    /// called for this journal: after the terminal marker is sealed
    /// the record is no longer "pending", so the getter returns `None`
    /// even though the in-memory slot still holds the record. Callers
    /// who need the final record after completion should consult the
    /// recovery snapshot via [`Journal::recover_run`].
    pub fn pending_completion_record(&self) -> Option<&JournalRecord> {
        if self.completed {
            return None;
        }
        self.completion_record.as_ref()
    }

    /// Canonical completion-tagged record, including after sealing.
    ///
    /// Crate-internal writer recovery uses this to verify that an
    /// idempotent completion retry carries the same terminal payload.
    pub(crate) fn completion_record(&self) -> Option<&JournalRecord> {
        self.completion_record.as_ref()
    }

    /// Append a data frame derived from `record`.
    ///
    /// `record.sequence` must match the journal's next sequence (the
    /// journal does not rewrite it). `record.run_id` must match the
    /// journal's run id — a mismatch is rejected with
    /// [`JournalError::RunIdMismatch`]. On success the in-memory
    /// counter advances and the frame is `flush()`'d to the OS (but
    /// not yet `sync_data`'d — see [`Journal::complete`]).
    pub fn append(&mut self, record: JournalRecord) -> Result<(), JournalError> {
        if self.poisoned {
            return Err(JournalError::Poisoned);
        }
        if self.completed {
            return Err(JournalError::DuplicateTerminal(
                "cannot append after journal completion",
            ));
        }
        if record.run_id != self.run_id {
            return Err(JournalError::RunIdMismatch {
                journal: self.run_id,
                frame: record.run_id,
            });
        }
        if record.sequence != self.next_sequence {
            return Err(JournalError::SequenceMismatch {
                expected: self.next_sequence,
                got: record.sequence,
            });
        }
        if self.completion_record.is_some() {
            if record.kind == OutputKind::Completed {
                return Err(JournalError::DuplicateTerminal(
                    "journal already contains a Completed record",
                ));
            }
            return Err(JournalError::InvalidState(
                "cannot append data after the Completed record".to_string(),
            ));
        }
        if record.kind == OutputKind::Completed && record.completion_id.is_none() {
            return Err(JournalError::InvalidState(
                "OutputKind::Completed requires a completion_id".to_string(),
            ));
        }
        // Tagged-record kind gate: a `completion_id` may only be
        // stamped on the terminal `Completed` data record. Tagging a
        // record of any other kind is a writer contract violation and
        // we refuse before durably writing the offending frame.
        if record.completion_id.is_some() && record.kind != OutputKind::Completed {
            return Err(JournalError::InvalidState(format!(
                "completion_id may only be set on OutputKind::Completed records, got {:?}",
                record.kind
            )));
        }
        // Capture the tagged record before it moves into the frame so
        // we can install it into the completion slot only after the
        // IO boundaries succeed.
        let pending_completion = if record.completion_id.is_some() {
            Some(record.clone())
        } else {
            None
        };
        // Completion-identity gate: any record that stamps a
        // `completion_id` must agree with what we've already seen on
        // records *and* with the journal's canonical completion id (set
        // by a prior `Complete` frame). A disagreement is a corruption
        // signal and we fail before durably writing the offending
        // frame.
        if let Some(new_id) = record.completion_id {
            if let Some(prior) = self.completion_id {
                if prior != new_id {
                    return Err(JournalError::CompletionConflict {
                        expected: Some(prior),
                        found: Some(new_id),
                    });
                }
            }
            if let Some(prior) = self.record_completion_id {
                if prior != new_id {
                    return Err(JournalError::CompletionConflict {
                        expected: Some(prior),
                        found: Some(new_id),
                    });
                }
            }
            self.record_completion_id = Some(new_id);
        }

        let frame = JournalFrame::Data(record);
        let payload = serde_json::to_vec(&frame)?;
        if payload.len() > MAX_FRAME_PAYLOAD {
            return Err(JournalError::FrameTooLarge {
                size: payload.len(),
            });
        }

        self.write_frame_guarded(&payload)?;
        self.flush_guarded()?;
        self.next_sequence += 1;
        if let Some(rec) = pending_completion {
            self.completion_record = Some(rec);
        }
        Ok(())
    }

    /// Write the terminal completion frame and `sync_data()` the file.
    ///
    /// Idempotent on the `completion_id`: calling `complete(id)` again with
    /// the *same* id after the journal has been completed returns `Ok(())`
    /// without writing another frame. A *different* id surfaces
    /// [`JournalError::CompletionConflict`].
    pub fn complete(&mut self, completion_id: Uuid) -> Result<(), JournalError> {
        if self.poisoned {
            return Err(JournalError::Poisoned);
        }
        if self.completed {
            // Idempotence: the same id was already sealed — don't
            // write a duplicate frame, just acknowledge the call.
            // A different id is a conflict.
            match self.completion_id {
                Some(existing) if existing == completion_id => return Ok(()),
                Some(existing) => {
                    return Err(JournalError::CompletionConflict {
                        expected: Some(existing),
                        found: Some(completion_id),
                    });
                }
                None => {
                    // Defensive: `completed == true` should imply
                    // `completion_id == Some(_)`. Treat as conflict.
                    return Err(JournalError::CompletionConflict {
                        expected: None,
                        found: Some(completion_id),
                    });
                }
            }
        }
        let Some(completion_record) = self.completion_record.as_ref() else {
            return Err(JournalError::InvalidState(
                "cannot complete journal without exactly one preceding Completed record"
                    .to_string(),
            ));
        };
        if completion_record.completion_id != Some(completion_id) {
            return Err(JournalError::CompletionConflict {
                expected: completion_record.completion_id,
                found: Some(completion_id),
            });
        }
        // If any data record already tagged a completion id, it must
        // match the value we're about to seal the file with.
        if let Some(prior) = self.record_completion_id {
            if prior != completion_id {
                return Err(JournalError::CompletionConflict {
                    expected: Some(prior),
                    found: Some(completion_id),
                });
            }
        }
        let frame = JournalFrame::Complete {
            run_id: self.run_id,
            completion_id,
            observed_at: Utc::now(),
        };
        let payload = serde_json::to_vec(&frame)?;
        self.write_frame_guarded(&payload)?;
        self.flush_guarded()?;
        self.sync_data_guarded()?;
        #[cfg(unix)]
        self.release_writer_lock()?;
        self.completed = true;
        self.completion_id = Some(completion_id);
        Ok(())
    }

    /// Write an acknowledgement frame and `sync_data()` the file.
    ///
    /// Returns [`JournalError::InvalidState`] if the journal has not
    /// yet been completed (ack must follow complete), or
    /// [`JournalError::DuplicateTerminal`] if it has already been
    /// acknowledged.
    pub fn acknowledge(&mut self) -> Result<(), JournalError> {
        if self.poisoned {
            return Err(JournalError::Poisoned);
        }
        if !self.completed {
            return Err(JournalError::InvalidState(
                "cannot acknowledge journal before complete".to_string(),
            ));
        }
        if self.acknowledged {
            return Err(JournalError::DuplicateTerminal(
                "journal already acknowledged",
            ));
        }
        #[cfg(unix)]
        if !self.writer_lock_held {
            acquire_writer_lock(&self.file, self.run_id, &self.path)?;
            self.writer_lock_held = true;

            // This completed handle may have been unlocked while another
            // handle acknowledged the run. Refresh under the exclusive lock
            // before mutating so stale objects cannot append a second Ack.
            if let Err(error) = self.file.seek(SeekFrom::Start(0)) {
                self.poisoned = true;
                return Err(JournalError::Io(error));
            }
            let snapshot = match recover_full(&mut self.file, self.run_id, self.recovery_limits) {
                Ok(snapshot) => snapshot,
                Err(error) => {
                    self.poisoned = true;
                    return Err(error);
                }
            };
            if snapshot.ack.is_some() {
                self.acknowledged = true;
                self.release_writer_lock()?;
                return Err(JournalError::DuplicateTerminal(
                    "journal already acknowledged",
                ));
            }
            if snapshot
                .completion
                .as_ref()
                .map(|marker| marker.completion_id)
                != self.completion_id
            {
                self.poisoned = true;
                return Err(JournalError::CompletionConflict {
                    expected: self.completion_id,
                    found: snapshot
                        .completion
                        .as_ref()
                        .map(|marker| marker.completion_id),
                });
            }
        }
        let frame = JournalFrame::Ack {
            run_id: self.run_id,
            observed_at: Utc::now(),
        };
        let payload = serde_json::to_vec(&frame)?;
        self.write_frame_guarded(&payload)?;
        self.flush_guarded()?;
        self.sync_data_guarded()?;
        #[cfg(unix)]
        self.release_writer_lock()?;
        self.acknowledged = true;
        Ok(())
    }

    /// Drive the [`write_frame`] boundary through the poison guard.
    ///
    /// Every byte that the journal emits passes through this helper:
    /// `write_frame` itself cannot decide whether an `Ok` was real, so
    /// the wrapper owns the only failure surface that matters — any I/O
    /// error (real or test-injected) flips [`Self::poisoned`] to `true`
    /// *before* the original error is returned, so the next mutation
    /// attempt refuses rather than continuing on an inconsistent
    /// file.
    fn write_frame_guarded(&mut self, payload: &[u8]) -> Result<(), JournalError> {
        #[cfg(test)]
        self.maybe_inject_failure(TestFailurePoint::Write)?;
        match write_frame(&mut self.file, payload) {
            Ok(()) => Ok(()),
            Err(e) => {
                self.poisoned = true;
                Err(e)
            }
        }
    }

    /// Poison-guarded wrapper around [`std::fs::File::flush`].
    ///
    /// Sets [`Self::poisoned`] before propagating any OS error (real
    /// or test-injected), so the journal cannot continue mutating
    /// after an unflushed buffer.
    fn flush_guarded(&mut self) -> Result<(), JournalError> {
        #[cfg(test)]
        self.maybe_inject_failure(TestFailurePoint::Flush)?;
        match self.file.flush() {
            Ok(()) => Ok(()),
            Err(e) => {
                self.poisoned = true;
                Err(JournalError::Io(e))
            }
        }
    }

    /// Poison-guarded wrapper around [`std::fs::File::sync_data`].
    ///
    /// Sets [`Self::poisoned`] before propagating any OS error (real
    /// or test-injected). A sync failure means the on-disk tail may
    /// differ from what the caller believes, so subsequent mutations
    /// must refuse until the journal is re-opened from disk.
    fn sync_data_guarded(&mut self) -> Result<(), JournalError> {
        #[cfg(test)]
        self.maybe_inject_failure(TestFailurePoint::Sync)?;
        match self.file.sync_data() {
            Ok(()) => Ok(()),
            Err(e) => {
                self.poisoned = true;
                Err(JournalError::Io(e))
            }
        }
    }

    #[cfg(unix)]
    fn release_writer_lock(&mut self) -> Result<(), JournalError> {
        if !self.writer_lock_held {
            return Ok(());
        }
        if let Err(error) = release_writer_lock_fd(&self.file) {
            self.poisoned = true;
            return Err(error);
        }
        self.writer_lock_held = false;
        Ok(())
    }

    /// Test-only hook: when [`Self::fail_at`] is `Some(boundary)`,
    /// clear the slot, trip [`Self::poisoned`], and surface a synthetic
    /// I/O error so the caller observes the same control flow it would
    /// on a real OS failure. No-op when `fail_at` is `None` or matches
    /// a different boundary.
    #[cfg(test)]
    fn maybe_inject_failure(&mut self, boundary: TestFailurePoint) -> Result<(), JournalError> {
        if self.fail_at == Some(boundary) {
            self.fail_at = None;
            self.poisoned = true;
            return Err(JournalError::Io(std::io::Error::other(format!(
                "injected test failure at {boundary:?} boundary"
            ))));
        }
        Ok(())
    }

    /// Convenience wrapper that opens a journal by `(root, run_id)` and
    /// recovers its full frame stream.
    ///
    /// The supplied `run_id` is treated as authoritative: every `Data`,
    /// `Complete` and `Ack` frame is checked against it and the
    /// recovery fails with [`JournalError::RunIdMismatch`] if a frame
    /// disagrees.
    pub fn recover_run(root: &Path, run_id: Uuid) -> Result<RecoveredJournal, JournalError> {
        Self::recover_run_with_limits(root, run_id, RecoveryLimits::default())
    }

    /// Recover a journal using explicit byte and record bounds.
    pub fn recover_run_with_limits(
        root: &Path,
        run_id: Uuid,
        limits: RecoveryLimits,
    ) -> Result<RecoveredJournal, JournalError> {
        #[cfg(not(unix))]
        {
            let _ = (root, run_id, limits);
            return Err(JournalError::UnsupportedPlatform);
        }
        #[cfg(unix)]
        {
            let opened_root = open_root_dir(root, false)?;
            let path = journal_path(&opened_root.path, run_id)?;
            let mut file = open_journal_leaf(&opened_root, run_id, false)?;
            acquire_writer_lock(&file, run_id, &path)?;
            prepare_journal_file(&file, false, &path)?;
            file.seek(SeekFrom::Start(0))?;
            let recovered = recover_full(&mut file, run_id, limits);
            match recovered {
                Ok(snapshot) => {
                    release_writer_lock_fd(&file)?;
                    Ok(snapshot)
                }
                Err(error) => {
                    let _ = release_writer_lock_fd(&file);
                    Err(error)
                }
            }
        }
    }
}

impl Drop for Journal {
    fn drop(&mut self) {
        #[cfg(unix)]
        if self.writer_lock_held && !self.poisoned {
            // Do not rely solely on `close(2)`: a concurrently forked
            // child can transiently inherit this CLOEXEC descriptor until
            // exec. Explicit unlock on the shared open-file description
            // makes normal writer shutdown deterministic. Poisoned writers
            // intentionally retain ownership until their descriptor closes.
            let _ = release_writer_lock_fd(&self.file);
            self.writer_lock_held = false;
        }
    }
}

/// Recover full [`RecoveredJournal`] payload, including marker metadata.
///
/// Operating on a `File` (rather than a generic `Read + Seek`) lets us
/// both scan the journal and truncate the on-disk file when the tail is
/// torn, so the file must be opened in read+write mode.
///
/// Every `Data`, `Complete` and `Ack` frame's `run_id` is checked
/// against `expected_run_id`; a mismatch surfaces as
/// [`JournalError::RunIdMismatch`]. The supplied id is also stamped on
/// the returned [`RecoveredJournal::run_id`] so empty journals — which
/// carry no per-record run id — still expose an authoritative run
/// identity for downstream `checkpoint` / `records_after` checks.
///
/// Torn-tail handling: only a *short* EOF (a partial header or a
/// truncated body) is treated as a recoverable torn tail and triggers
/// in-place truncation. A *full* header whose magic bytes are wrong is
/// interior corruption and surfaces as
/// [`JournalError::MalformedHeader`].
fn recover_full(
    file: &mut File,
    expected_run_id: Uuid,
    limits: RecoveryLimits,
) -> Result<RecoveredJournal, JournalError> {
    // Check the opened descriptor itself before allocating recovery
    // buffers. This closes the metadata/open race in the public wrappers.
    enforce_byte_limit(file.metadata()?.len(), limits)?;
    let mut next_sequence: Option<u64> = None;
    let mut completion: Option<CompleteMarker> = None;
    let mut ack: Option<AckMarker> = None;
    let mut records = Vec::new();
    let mut completion_record: Option<JournalRecord> = None;

    loop {
        let offset = file.stream_position()?;
        let mut header = [0u8; HEADER_LEN];
        let n = file.read(&mut header)?;
        if n == 0 {
            break;
        }
        if n < HEADER_LEN {
            // Partial header — torn tail. Truncate and stop.
            truncate_at(file, offset)?;
            break;
        }

        let mut header_checksum = [0u8; 32];
        header_checksum.copy_from_slice(&header[HEADER_METADATA_LEN..HEADER_METADATA_LEN + 32]);
        verify_checksum(offset, &header_checksum, &header[..HEADER_METADATA_LEN])?;

        if header[0..4] != FRAME_MAGIC {
            // Full header read, but the magic is wrong — interior
            // corruption, not a torn tail. Refuse to silently truncate
            // valid bytes the caller may still want.
            return Err(JournalError::MalformedHeader(format!(
                "bad frame magic at offset {offset}"
            )));
        }
        if header[4] != FRAME_VERSION {
            return Err(JournalError::MalformedHeader(format!(
                "unknown frame version {}",
                header[4]
            )));
        }
        let length = u32::from_be_bytes([header[5], header[6], header[7], header[8]]) as usize;
        if length > MAX_FRAME_PAYLOAD {
            return Err(JournalError::FrameTooLarge { size: length });
        }
        let mut checksum = [0u8; 32];
        checksum.copy_from_slice(&header[HEADER_METADATA_LEN + 32..HEADER_LEN]);
        let mut payload = vec![0u8; length];
        match file.read_exact(&mut payload) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                // Truncated body — torn tail. Drop the partial frame
                // and the bogus header.
                truncate_at(file, offset)?;
                break;
            }
            Err(e) => return Err(JournalError::Io(e)),
        }

        verify_checksum(offset, &checksum, &payload)?;

        let frame: JournalFrame = serde_json::from_slice(&payload)?;
        match frame {
            JournalFrame::Data(record) => {
                if records.len() >= limits.max_records {
                    return Err(JournalError::RecoveryRecordLimitExceeded {
                        records: records.len() + 1,
                        max_records: limits.max_records,
                    });
                }
                if record.run_id != expected_run_id {
                    return Err(JournalError::RunIdMismatch {
                        journal: expected_run_id,
                        frame: record.run_id,
                    });
                }
                let expected_seq = next_sequence.unwrap_or(0);
                if record.sequence != expected_seq {
                    return Err(JournalError::SequenceMismatch {
                        expected: expected_seq,
                        got: record.sequence,
                    });
                }
                if completion.is_some() || ack.is_some() {
                    return Err(JournalError::InvalidState(
                        "data frame encountered after a terminal marker".to_string(),
                    ));
                }
                if completion_record.is_some() {
                    if record.kind == OutputKind::Completed {
                        return Err(JournalError::DuplicateTerminal(
                            "duplicate Completed record",
                        ));
                    }
                    return Err(JournalError::InvalidState(
                        "data frame encountered after the Completed record".to_string(),
                    ));
                }
                match (record.kind, record.completion_id) {
                    (OutputKind::Completed, None) => {
                        return Err(JournalError::InvalidState(
                            "OutputKind::Completed requires a completion_id".to_string(),
                        ));
                    }
                    (OutputKind::Completed, Some(_)) => {
                        completion_record = Some(record.clone());
                    }
                    (_, Some(_)) => {
                        return Err(JournalError::InvalidState(
                            "completion_id may only be set on OutputKind::Completed records"
                                .to_string(),
                        ));
                    }
                    (_, None) => {}
                }
                records.push(record);
                next_sequence = Some(expected_seq + 1);
            }
            JournalFrame::Complete {
                run_id,
                completion_id,
                observed_at,
            } => {
                if run_id != expected_run_id {
                    return Err(JournalError::RunIdMismatch {
                        journal: expected_run_id,
                        frame: run_id,
                    });
                }
                if ack.is_some() {
                    return Err(JournalError::InvalidState(
                        "Complete frame encountered after Ack".to_string(),
                    ));
                }
                if completion.is_some() {
                    return Err(JournalError::DuplicateTerminal(
                        "duplicate journal complete frame",
                    ));
                }
                let Some(record) = completion_record.as_ref() else {
                    return Err(JournalError::InvalidState(
                        "Complete frame requires exactly one preceding Completed record"
                            .to_string(),
                    ));
                };
                if record.completion_id != Some(completion_id) {
                    return Err(JournalError::CompletionConflict {
                        expected: Some(completion_id),
                        found: record.completion_id,
                    });
                }
                completion = Some(CompleteMarker {
                    completion_id,
                    observed_at,
                });
            }
            JournalFrame::Ack {
                run_id,
                observed_at,
            } => {
                if run_id != expected_run_id {
                    return Err(JournalError::RunIdMismatch {
                        journal: expected_run_id,
                        frame: run_id,
                    });
                }
                if completion.is_none() {
                    return Err(JournalError::InvalidState(
                        "Ack frame encountered before Complete".to_string(),
                    ));
                }
                if ack.is_some() {
                    return Err(JournalError::DuplicateTerminal(
                        "duplicate journal ack frame",
                    ));
                }
                ack = Some(AckMarker { observed_at });
            }
        }
    }

    // Completion-identity gate: every record that carries a
    // `completion_id` must agree with every other such record, and all
    // of them must agree with the terminal `Complete` frame's id.
    // Without a `Complete` frame the records can carry an arbitrary id
    // in flight, but they must still be self-consistent.
    let mut record_completion_id: Option<Uuid> = None;
    for rec in &records {
        if let Some(rid) = rec.completion_id {
            match record_completion_id {
                Some(prev) if prev != rid => {
                    return Err(JournalError::CompletionConflict {
                        expected: Some(prev),
                        found: Some(rid),
                    });
                }
                None => record_completion_id = Some(rid),
                _ => {}
            }
        }
    }
    if let Some(c) = completion.as_ref() {
        if let Some(rid) = record_completion_id {
            if rid != c.completion_id {
                return Err(JournalError::CompletionConflict {
                    expected: Some(c.completion_id),
                    found: Some(rid),
                });
            }
        }
    }

    Ok(RecoveredJournal {
        run_id: expected_run_id,
        records,
        completion,
        ack,
    })
}

fn enforce_byte_limit(size: u64, limits: RecoveryLimits) -> Result<(), JournalError> {
    if size > limits.max_bytes {
        return Err(JournalError::RecoveryByteLimitExceeded {
            size,
            max_bytes: limits.max_bytes,
        });
    }
    Ok(())
}

/// Write one framed payload to `writer`, including the magic / version /
/// length / checksum header.
fn write_frame<W: Write>(writer: &mut W, payload: &[u8]) -> Result<(), JournalError> {
    let payload_checksum = Sha256::digest(payload);
    let length = payload.len() as u32;
    let mut header = [0u8; HEADER_LEN];
    header[0..4].copy_from_slice(&FRAME_MAGIC);
    header[4] = FRAME_VERSION;
    header[5..9].copy_from_slice(&length.to_be_bytes());
    let header_checksum = Sha256::digest(&header[..HEADER_METADATA_LEN]);
    header[HEADER_METADATA_LEN..HEADER_METADATA_LEN + 32].copy_from_slice(&header_checksum);
    header[HEADER_METADATA_LEN + 32..HEADER_LEN].copy_from_slice(&payload_checksum);
    writer.write_all(&header)?;
    writer.write_all(payload)?;
    Ok(())
}

/// Verify a frame's SHA-256 against its on-disk checksum.
///
/// On interior corruption this short-circuits to
/// [`JournalError::ChecksumMismatch`] reporting the byte offset of the
/// suspicious frame.
fn verify_checksum(offset: u64, expected: &[u8; 32], payload: &[u8]) -> Result<(), JournalError> {
    let actual = Sha256::digest(payload);
    if &actual[..] != expected {
        return Err(JournalError::ChecksumMismatch { offset });
    }
    Ok(())
}

/// Truncate `file` to `offset`, then rewind.
fn truncate_at(file: &mut File, offset: u64) -> Result<(), JournalError> {
    file.seek(SeekFrom::Start(offset))?;
    file.set_len(offset)?;
    sync_truncated_file(file)?;
    file.seek(SeekFrom::Start(offset))?;
    Ok(())
}

/// Build the on-disk path for `run_id` under `root`.
///
/// The filename is `journal-<uuid>.log` — the prefix lets operators
/// distinguish journals from other persisted state at a glance, and the
/// hyphenated run id keeps the path both shell-safe and unique across
/// all runs the spawner has ever persisted.
fn journal_path(root: &Path, run_id: Uuid) -> Result<PathBuf, JournalError> {
    let filename = format!("journal-{}.log", run_id.hyphenated());
    Ok(root.join(filename))
}

#[cfg(unix)]
#[derive(Debug)]
struct OpenedRoot {
    fd: OwnedFd,
    path: PathBuf,
}

#[cfg(unix)]
fn validate_absolute_root(root: &Path) -> Result<Vec<OsString>, JournalError> {
    if !root.is_absolute() {
        return Err(JournalError::UnsafePath(format!(
            "journal root must be absolute, got {}",
            root.display()
        )));
    }
    let mut components = Vec::new();
    for component in root.components() {
        match component {
            Component::RootDir => {}
            Component::Normal(name) => components.push(name.to_os_string()),
            _ => {
                return Err(JournalError::UnsafePath(format!(
                    "journal root must contain only normal components: {}",
                    root.display()
                )));
            }
        }
    }
    Ok(components)
}

#[cfg(unix)]
fn owned_fd(raw_fd: RawFd) -> OwnedFd {
    // SAFETY: `raw_fd` was just returned by a successful nix open call and
    // has not been wrapped or transferred elsewhere; this immediately gives
    // it one RAII owner so every subsequent error path closes it.
    unsafe { OwnedFd::from_raw_fd(raw_fd) }
}

#[cfg(unix)]
fn unsafe_traversal(error: Errno, path: &Path) -> JournalError {
    if matches!(error, Errno::ELOOP | Errno::ENOTDIR) {
        JournalError::UnsafePath(format!(
            "symlink or non-directory in journal root traversal: {}",
            path.display()
        ))
    } else {
        JournalError::Io(std::io::Error::from_raw_os_error(error as i32))
    }
}

#[cfg(unix)]
fn open_root_dir(root: &Path, create_missing: bool) -> Result<OpenedRoot, JournalError> {
    let components = validate_absolute_root(root)?;
    let root_flags = OFlag::O_RDONLY | OFlag::O_DIRECTORY | OFlag::O_CLOEXEC;
    let mut current = owned_fd(
        open(Path::new("/"), root_flags, Mode::empty())
            .map_err(|error| JournalError::Io(std::io::Error::from_raw_os_error(error as i32)))?,
    );
    let child_flags = root_flags | OFlag::O_NOFOLLOW;
    let mut diagnostic = PathBuf::from("/");

    for component in components {
        diagnostic.push(&component);
        let (opened, created) = match openat(
            current.as_raw_fd(),
            Path::new(&component),
            child_flags,
            Mode::empty(),
        ) {
            Ok(raw_fd) => (owned_fd(raw_fd), false),
            Err(Errno::ENOENT) if create_missing => {
                let created = match mkdirat(
                    current.as_raw_fd(),
                    Path::new(&component),
                    Mode::from_bits_truncate(0o700),
                ) {
                    Ok(()) => true,
                    Err(Errno::EEXIST) => false,
                    Err(error) => return Err(unsafe_traversal(error, &diagnostic)),
                };
                let child = owned_fd(
                    openat(
                        current.as_raw_fd(),
                        Path::new(&component),
                        child_flags,
                        Mode::empty(),
                    )
                    .map_err(|error| unsafe_traversal(error, &diagnostic))?,
                );
                if created {
                    nix::sys::stat::fchmod(child.as_raw_fd(), Mode::from_bits_truncate(0o700))
                        .map_err(|error| {
                            JournalError::Io(std::io::Error::from_raw_os_error(error as i32))
                        })?;
                }
                (child, created)
            }
            Err(error) => return Err(unsafe_traversal(error, &diagnostic)),
        };

        #[cfg(test)]
        run_root_walk_hook(&diagnostic);

        if create_missing {
            if created {
                sync_created_directory(opened.as_raw_fd())?;
            } else {
                fsync_fd(opened.as_raw_fd())?;
            }
            fsync_fd(current.as_raw_fd())?;
        }
        current = opened;
    }

    Ok(OpenedRoot {
        fd: current,
        path: root.to_path_buf(),
    })
}

#[cfg(unix)]
fn journal_filename(run_id: Uuid) -> String {
    format!("journal-{}.log", run_id.hyphenated())
}

#[cfg(unix)]
fn open_journal_leaf(
    root: &OpenedRoot,
    run_id: Uuid,
    create_new: bool,
) -> Result<File, JournalError> {
    let filename = journal_filename(run_id);
    let mut flags = OFlag::O_RDWR | OFlag::O_APPEND | OFlag::O_NOFOLLOW | OFlag::O_CLOEXEC;
    if create_new {
        flags |= OFlag::O_CREAT | OFlag::O_EXCL;
    }
    let raw_fd = openat(
        root.fd.as_raw_fd(),
        filename.as_str(),
        flags,
        Mode::from_bits_truncate(JOURNAL_FILE_MODE),
    )
    .map_err(|error| {
        if matches!(error, Errno::ELOOP | Errno::ENOTDIR | Errno::EEXIST) {
            JournalError::UnsafePath(format!(
                "unsafe or existing journal leaf at {}",
                root.path.join(&filename).display()
            ))
        } else {
            JournalError::Io(std::io::Error::from_raw_os_error(error as i32))
        }
    })?;
    Ok(File::from(owned_fd(raw_fd)))
}

#[cfg(unix)]
fn prepare_journal_file(file: &File, create_new: bool, path: &Path) -> Result<(), JournalError> {
    if create_new {
        nix::sys::stat::fchmod(
            file.as_raw_fd(),
            Mode::from_bits_truncate(JOURNAL_FILE_MODE),
        )
        .map_err(|error| JournalError::Io(std::io::Error::from_raw_os_error(error as i32)))?;
    }
    let stat = fstat(file.as_raw_fd())
        .map_err(|error| JournalError::Io(std::io::Error::from_raw_os_error(error as i32)))?;
    let file_type = SFlag::from_bits_truncate(stat.st_mode);
    let mode = stat.st_mode & 0o777;
    if !file_type.contains(SFlag::S_IFREG) || mode != JOURNAL_FILE_MODE {
        return Err(JournalError::UnsafePath(format!(
            "journal leaf {} must be a regular file with mode 0600 (found {:o})",
            path.display(),
            mode
        )));
    }
    Ok(())
}

#[cfg(unix)]
fn acquire_writer_lock(file: &File, run_id: Uuid, path: &Path) -> Result<(), JournalError> {
    match flock(file.as_raw_fd(), FlockArg::LockExclusiveNonblock) {
        Ok(()) => Ok(()),
        Err(Errno::EAGAIN) => Err(JournalError::WriterBusy {
            run_id,
            path: path.to_path_buf(),
        }),
        Err(error) => Err(JournalError::Io(std::io::Error::from_raw_os_error(
            error as i32,
        ))),
    }
}

#[cfg(unix)]
fn release_writer_lock_fd(file: &File) -> Result<(), JournalError> {
    flock(file.as_raw_fd(), FlockArg::Unlock)
        .map_err(|error| JournalError::Io(std::io::Error::from_raw_os_error(error as i32)))
}

#[cfg(unix)]
fn fsync_fd(fd: RawFd) -> Result<(), JournalError> {
    fsync(fd).map_err(|error| JournalError::Io(std::io::Error::from_raw_os_error(error as i32)))
}

#[cfg(unix)]
fn sync_created_directory(fd: RawFd) -> Result<(), JournalError> {
    #[cfg(test)]
    maybe_inject_durability_failure(TestDurabilityFailurePoint::DirectoryEntry)?;
    fsync_fd(fd)
}

#[cfg(unix)]
fn sync_new_leaf(file: &File) -> Result<(), JournalError> {
    #[cfg(test)]
    maybe_inject_durability_failure(TestDurabilityFailurePoint::LeafData)?;
    file.sync_data().map_err(JournalError::Io)
}

#[cfg(unix)]
fn sync_leaf_parent(fd: RawFd) -> Result<(), JournalError> {
    #[cfg(test)]
    maybe_inject_durability_failure(TestDurabilityFailurePoint::LeafParent)?;
    fsync_fd(fd)
}

fn sync_truncated_file(file: &File) -> Result<(), JournalError> {
    #[cfg(all(test, unix))]
    maybe_inject_durability_failure(TestDurabilityFailurePoint::Truncate)?;
    file.sync_data().map_err(JournalError::Io)
}

/// RAII guard that swaps the process-wide umask on construction and
/// restores the prior value on `Drop` — including panic unwind, since
/// `Drop` runs during stack unwinding.
///
/// Used by the `cfg(unix)` permissive-umask tests so every exit path
/// (early `return`, panic from a failed assertion, or normal completion)
/// leaves the spawner test process with its previous umask intact. The
/// guard is private and `cfg(test)`-only; production builds never
/// observe it.
#[cfg(test)]
struct UmaskGuard {
    prev: nix::libc::mode_t,
}

#[cfg(test)]
impl UmaskGuard {
    /// Install `new` as the process umask and return a guard that
    /// restores the previous value on drop.
    ///
    /// `umask(2)` is async-signal-safe and the journal constructors it
    /// guards never spawn threads internally, so a plain FFI call is
    /// the correct primitive here.
    fn install(new: nix::libc::mode_t) -> Self {
        // SAFETY: `umask` is async-signal-safe and the value it returns
        // is the only authoritative source of the prior mask.
        let prev = unsafe { nix::libc::umask(new) };
        Self { prev }
    }
}

#[cfg(test)]
impl Drop for UmaskGuard {
    fn drop(&mut self) {
        // SAFETY: restoring the previous umask; safe on every path
        // including unwind.
        unsafe { nix::libc::umask(self.prev) };
    }
}

/// Run-supplied retention policy: a journal is eligible for deletion
/// when both completion and acknowledgement frames are present, the
/// acknowledgement is at least `older_than` older than `now`, and the
/// acknowledgement timestamp is also at least one second in the past
/// (the "zero-age tolerance" prevents a freshly-acknowledged journal
/// from becoming immediately eligible just because the caller passed a
/// zero duration).
pub fn is_retention_eligible(
    journal: &RecoveredJournal,
    now: DateTime<Utc>,
    older_than: chrono::Duration,
) -> bool {
    let Some(completion) = &journal.completion else {
        return false;
    };
    let Some(ack) = &journal.ack else {
        return false;
    };
    let completion_id_on_record = journal.records.iter().rev().find_map(|r| r.completion_id);
    if let Some(on_record) = completion_id_on_record {
        if on_record != completion.completion_id {
            return false;
        }
    }
    let elapsed = now - ack.observed_at;
    elapsed >= older_than && elapsed >= chrono::Duration::seconds(1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::OpenOptions;
    use std::os::unix::fs::PermissionsExt;
    use tempfile::TempDir;

    fn temp_root() -> TempDir {
        // Force /tmp or equivalent; Cargo runs tests with cwd inside the
        // workspace, but TempDir is independent of cwd.
        tempfile::Builder::new()
            .prefix("terraphim-journal-test-")
            .tempdir()
            .expect("tempdir")
    }

    fn new_record(run_id: Uuid, sequence: u64, process_id: u64) -> JournalRecord {
        JournalRecord {
            run_id,
            sequence,
            process_id,
            kind: OutputKind::Stdout,
            payload: serde_json::Value::String(format!("hello #{sequence}")),
            completion_id: None,
            observed_at: Utc::now(),
        }
    }

    #[test]
    fn roundtrip_serializes_records() {
        let dir = temp_root();
        let run_id = Uuid::new_v4();
        let mut journal = Journal::create(dir.path(), run_id).expect("create");

        for seq in 0..5u64 {
            journal
                .append(new_record(run_id, seq, 4242))
                .expect("append data");
        }
        let completion_id = Uuid::new_v4();
        // Tag the last record with the completion id so readers can
        // correlate.
        journal
            .append(JournalRecord {
                run_id,
                sequence: 5,
                process_id: 4242,
                kind: OutputKind::Completed,
                payload: serde_json::json!({"exit_code": 0}),
                completion_id: Some(completion_id),
                observed_at: Utc::now(),
            })
            .expect("append completion record");
        journal.complete(completion_id).expect("complete journal");
        journal.acknowledge().expect("acknowledge journal");

        let recovered = Journal::recover_run(dir.path(), run_id).expect("recover run");
        assert_eq!(recovered.records.len(), 6);
        assert_eq!(
            recovered.completion.as_ref().map(|m| m.completion_id),
            Some(completion_id)
        );
        assert!(recovered.ack.is_some());

        for (idx, rec) in recovered.records.iter().enumerate() {
            assert_eq!(rec.sequence, idx as u64);
            assert_eq!(rec.run_id, run_id);
            assert_eq!(rec.process_id, 4242);
        }
    }

    #[test]
    fn roundtrip_over_thousand_contiguous_records() {
        let dir = temp_root();
        let run_id = Uuid::new_v4();
        let mut journal = Journal::create(dir.path(), run_id).expect("create");

        const TOTAL: u64 = 1500;
        let baseline_kind = [
            OutputKind::Stdout,
            OutputKind::Stderr,
            OutputKind::Mention,
            OutputKind::Heartbeat,
        ];
        for seq in 0..TOTAL {
            journal
                .append(JournalRecord {
                    run_id,
                    sequence: seq,
                    process_id: 99,
                    kind: baseline_kind[(seq as usize) % baseline_kind.len()],
                    payload: serde_json::Value::String(format!("line {seq}")),
                    completion_id: None,
                    observed_at: Utc::now(),
                })
                .expect("append under load");
        }
        drop(journal);

        let recovered = Journal::recover_run(dir.path(), run_id).expect("recover run");
        assert_eq!(recovered.records.len() as u64, TOTAL);
        // Contiguity invariant: sequence == position.
        for (idx, rec) in recovered.records.iter().enumerate() {
            assert_eq!(rec.sequence, idx as u64);
        }
    }

    #[test]
    fn torn_tail_is_truncated_to_last_good_frame() {
        let dir = temp_root();
        let run_id = Uuid::new_v4();
        let mut journal = Journal::create(dir.path(), run_id).expect("create");
        for seq in 0..6u64 {
            journal.append(new_record(run_id, seq, 7)).expect("append");
        }

        let path = journal.path().to_path_buf();
        let len = std::fs::metadata(&path).expect("metadata").len();
        assert!(
            len > 64,
            "journal should hold multiple frames; got {len} bytes"
        );

        // Truncate well inside the last frame to simulate a partial write
        // followed by process death.
        let truncate_to = len - 23;
        let f = std::fs::OpenOptions::new()
            .write(true)
            .open(&path)
            .expect("open for trunc");
        f.set_len(truncate_to).expect("set_len");
        drop(f);
        drop(journal);

        let recovered = Journal::recover_run(dir.path(), run_id).expect("recover");
        // Last three frames partially overlap the corrupted region; we
        // must see strictly contiguous frames ending before the torn
        // bytes.
        assert!(!recovered.records.is_empty());
        assert!(
            recovered.records.last().unwrap().sequence < 5,
            "torn tail should drop the trailing partial frame; got last seq {}",
            recovered.records.last().unwrap().sequence
        );

        // The file should now end on a frame boundary.
        let len_after = std::fs::metadata(&path).expect("metadata after").len();
        assert!(
            len_after < len,
            "recover should have shortened the file (was {len}, now {len_after})"
        );
        assert!(
            len_after > 0,
            "file should still hold the recovered records"
        );
    }

    #[test]
    fn partial_final_header_is_truncated_to_last_good_frame() {
        let dir = temp_root();
        let run_id = Uuid::new_v4();
        let mut journal = Journal::create(dir.path(), run_id).expect("create");
        journal.append(new_record(run_id, 0, 7)).expect("append");
        let path = journal.path().to_path_buf();
        let good_len = std::fs::metadata(&path).expect("metadata").len();
        drop(journal);

        let payload = serde_json::to_vec(&JournalFrame::Data(new_record(run_id, 1, 7)))
            .expect("serialize frame");
        let mut encoded = Vec::new();
        write_frame(&mut encoded, &payload).expect("encode frame");
        assert!(encoded.len() > 7);
        let mut file = OpenOptions::new()
            .append(true)
            .open(&path)
            .expect("open for partial header");
        file.write_all(&encoded[..7]).expect("write partial header");
        file.flush().expect("flush partial header");
        drop(file);

        let recovered = Journal::recover_run(dir.path(), run_id).expect("recover torn header");
        assert_eq!(recovered.records.len(), 1);
        assert_eq!(
            std::fs::metadata(&path).expect("metadata after").len(),
            good_len,
            "partial final header must be truncated"
        );
    }

    #[test]
    fn interior_corruption_is_rejected() {
        let dir = temp_root();
        let run_id = Uuid::new_v4();
        let mut journal = Journal::create(dir.path(), run_id).expect("create");
        for seq in 0..4u64 {
            journal.append(new_record(run_id, seq, 11)).expect("append");
        }
        let path = journal.path().to_path_buf();
        drop(journal);

        // Flip a single bit somewhere in the second frame's payload.
        let mut bytes = std::fs::read(&path).expect("read");
        // Skip the first header (HEADER_LEN bytes) and then flip a byte
        // well inside frame #2's payload area.
        let target = HEADER_LEN + 4;
        bytes[target] ^= 0xFF;
        std::fs::write(&path, &bytes).expect("write corrupted journal");

        let err = Journal::recover_run(dir.path(), run_id)
            .expect_err("recover must reject interior corruption");
        assert!(
            matches!(err, JournalError::ChecksumMismatch { .. }),
            "expected ChecksumMismatch, got {err:?}"
        );
    }

    #[test]
    fn interior_length_corruption_is_rejected_without_truncation() {
        let dir = temp_root();
        let run_id = Uuid::new_v4();
        let mut journal = Journal::create(dir.path(), run_id).expect("create");
        for seq in 0..3u64 {
            journal.append(new_record(run_id, seq, 11)).expect("append");
        }
        let path = journal.path().to_path_buf();
        drop(journal);

        let mut bytes = std::fs::read(&path).expect("read");
        let frame1_len = u32::from_be_bytes([bytes[5], bytes[6], bytes[7], bytes[8]]) as usize;
        let frame2_header = HEADER_LEN + frame1_len;
        bytes[frame2_header + 5..frame2_header + 9].copy_from_slice(&1_000_000u32.to_be_bytes());
        let original_len = bytes.len() as u64;
        std::fs::write(&path, &bytes).expect("write corrupted journal");

        let err = Journal::recover_run(dir.path(), run_id)
            .expect_err("authenticated frame length must reject interior corruption");
        assert!(
            matches!(err, JournalError::ChecksumMismatch { .. }),
            "expected ChecksumMismatch, got {err:?}"
        );
        assert_eq!(
            std::fs::metadata(&path).expect("metadata").len(),
            original_len,
            "interior length corruption must not truncate the journal"
        );
    }

    #[test]
    fn interior_magic_corruption_is_rejected_by_header_checksum() {
        let dir = temp_root();
        let run_id = Uuid::new_v4();
        let mut journal = Journal::create(dir.path(), run_id).expect("create");
        for seq in 0..4u64 {
            journal.append(new_record(run_id, seq, 11)).expect("append");
        }
        let path = journal.path().to_path_buf();
        drop(journal);

        // Overwrite the magic on frame #2 with garbage. Because the
        // full header is still readable, this is interior corruption,
        // not a torn tail — recovery must surface ChecksumMismatch
        // without truncating valid frames before it.
        let mut bytes = std::fs::read(&path).expect("read");
        let frame1_len = u32::from_be_bytes([bytes[5], bytes[6], bytes[7], bytes[8]]) as usize;
        let frame2_magic = HEADER_LEN + frame1_len;
        bytes[frame2_magic] = b'X';
        bytes[frame2_magic + 1] = b'X';
        bytes[frame2_magic + 2] = b'X';
        bytes[frame2_magic + 3] = b'X';
        let original_len = bytes.len() as u64;
        std::fs::write(&path, &bytes).expect("write corrupted journal");

        let err = Journal::recover_run(dir.path(), run_id)
            .expect_err("recover must reject interior magic corruption");
        assert!(
            matches!(err, JournalError::ChecksumMismatch { .. }),
            "expected ChecksumMismatch, got {err:?}"
        );

        // The file must not be silently truncated by recovery — frame
        // #1's bytes are valid and should remain on disk.
        let len_after = std::fs::metadata(&path).expect("metadata").len();
        assert_eq!(
            len_after, original_len,
            "interior magic corruption must not truncate the file"
        );
    }

    #[test]
    fn recovery_byte_limit_rejects_oversized_file_and_accepts_exact_bound() {
        let dir = temp_root();
        let run_id = Uuid::new_v4();
        let mut journal = Journal::create(dir.path(), run_id).expect("create");
        journal.append(new_record(run_id, 0, 11)).expect("append");
        let path = journal.path().to_path_buf();
        drop(journal);
        let file_len = std::fs::metadata(&path).expect("metadata").len();

        let exact = RecoveryLimits {
            max_bytes: file_len,
            max_records: 1,
        };
        let recovered = Journal::recover_run_with_limits(dir.path(), run_id, exact)
            .expect("exact byte and record bounds must succeed");
        assert_eq!(recovered.records.len(), 1);

        let too_small = RecoveryLimits {
            max_bytes: file_len - 1,
            max_records: 1,
        };
        let err = Journal::recover_run_with_limits(dir.path(), run_id, too_small)
            .expect_err("oversized journal must be rejected before recovery");
        assert!(matches!(
            err,
            JournalError::RecoveryByteLimitExceeded { size, max_bytes }
                if size == file_len && max_bytes == file_len - 1
        ));
    }

    #[test]
    fn recovery_record_limit_rejects_excess_and_accepts_exact_bound() {
        let dir = temp_root();
        let run_id = Uuid::new_v4();
        let mut journal = Journal::create(dir.path(), run_id).expect("create");
        journal.append(new_record(run_id, 0, 11)).expect("append");
        journal.append(new_record(run_id, 1, 11)).expect("append");
        let file_len = std::fs::metadata(journal.path()).expect("metadata").len();
        drop(journal);

        let exact = RecoveryLimits {
            max_bytes: file_len,
            max_records: 2,
        };
        let reopened = Journal::open_with_limits(dir.path(), run_id, exact)
            .expect("open must accept the exact record bound");
        assert_eq!(reopened.next_sequence(), 2);
        drop(reopened);

        let too_few = RecoveryLimits {
            max_bytes: file_len,
            max_records: 1,
        };
        let err = Journal::recover_run_with_limits(dir.path(), run_id, too_few)
            .expect_err("recovery must reject a record beyond the bound");
        assert!(matches!(
            err,
            JournalError::RecoveryRecordLimitExceeded {
                records: 2,
                max_records: 1
            }
        ));
    }

    #[test]
    fn append_with_mismatched_run_id_is_rejected() {
        let dir = temp_root();
        let journal_run_id = Uuid::new_v4();
        let other_run_id = Uuid::new_v4();
        let mut journal = Journal::create(dir.path(), journal_run_id).expect("create");

        // A record stamped with a *different* run id must be rejected
        // outright, not silently rewritten onto the journal's run id.
        let bad = JournalRecord {
            run_id: other_run_id,
            sequence: 0,
            process_id: 1,
            kind: OutputKind::Stdout,
            payload: serde_json::Value::String("oops".into()),
            completion_id: None,
            observed_at: Utc::now(),
        };
        let err = journal
            .append(bad)
            .expect_err("must reject mismatched run_id");
        assert!(
            matches!(err, JournalError::RunIdMismatch { .. }),
            "expected RunIdMismatch, got {err:?}"
        );
    }

    #[test]
    fn data_frame_with_swapped_run_id_is_rejected() {
        let dir = temp_root();
        let real_run_id = Uuid::new_v4();
        let other_run_id = Uuid::new_v4();
        let mut journal = Journal::create(dir.path(), real_run_id).expect("create");
        journal
            .append(new_record(real_run_id, 0, 1))
            .expect("append");
        let path = journal.path().to_path_buf();
        drop(journal);

        // Forge a second frame whose Data record carries the *other*
        // run id, simulating a frame swapped in from a different
        // journal.
        let bogus = JournalFrame::Data(new_record(other_run_id, 1, 1));
        let payload = serde_json::to_vec(&bogus).expect("encode bogus");
        append_raw_frame(&path, &payload);

        let err = Journal::recover_run(dir.path(), real_run_id)
            .expect_err("recover must reject swapped run_id on data frame");
        assert!(
            matches!(err, JournalError::RunIdMismatch { .. }),
            "expected RunIdMismatch, got {err:?}"
        );
    }

    #[test]
    fn complete_frame_with_mismatched_run_id_is_rejected() {
        let dir = temp_root();
        let real_run_id = Uuid::new_v4();
        let other_run_id = Uuid::new_v4();
        let mut journal = Journal::create(dir.path(), real_run_id).expect("create");
        journal
            .append(new_record(real_run_id, 0, 1))
            .expect("append");
        let path = journal.path().to_path_buf();
        drop(journal);

        let bogus = JournalFrame::Complete {
            run_id: other_run_id,
            completion_id: Uuid::new_v4(),
            observed_at: Utc::now(),
        };
        let payload = serde_json::to_vec(&bogus).expect("encode bogus complete");
        append_raw_frame(&path, &payload);

        let err = Journal::recover_run(dir.path(), real_run_id)
            .expect_err("recover must reject mismatched complete run_id");
        assert!(
            matches!(err, JournalError::RunIdMismatch { .. }),
            "expected RunIdMismatch, got {err:?}"
        );
    }

    #[test]
    fn ack_frame_with_mismatched_run_id_is_rejected() {
        let dir = temp_root();
        let real_run_id = Uuid::new_v4();
        let other_run_id = Uuid::new_v4();
        let mut journal = Journal::create(dir.path(), real_run_id).expect("create");
        journal
            .append(new_record(real_run_id, 0, 1))
            .expect("append");
        let completion_id = Uuid::new_v4();
        append_completed(&mut journal, completion_id);
        journal.complete(completion_id).expect("complete");
        let path = journal.path().to_path_buf();
        drop(journal);

        let bogus = JournalFrame::Ack {
            run_id: other_run_id,
            observed_at: Utc::now(),
        };
        let payload = serde_json::to_vec(&bogus).expect("encode bogus ack");
        append_raw_frame(&path, &payload);

        let err = Journal::recover_run(dir.path(), real_run_id)
            .expect_err("recover must reject mismatched ack run_id");
        assert!(
            matches!(err, JournalError::RunIdMismatch { .. }),
            "expected RunIdMismatch, got {err:?}"
        );
    }

    #[test]
    fn recover_run_for_swapped_journal_is_rejected() {
        // Build a journal for `real_run_id`, then ask `recover_run` to
        // load it as if it belonged to `other_run_id`. The file lives
        // at real_run_id's path, so we have to manually swap it onto
        // other_run_id's expected path before calling recover_run.
        let dir = temp_root();
        let real_run_id = Uuid::new_v4();
        let other_run_id = Uuid::new_v4();
        let mut journal = Journal::create(dir.path(), real_run_id).expect("create");
        journal
            .append(new_record(real_run_id, 0, 1))
            .expect("append");
        let completion_id = Uuid::new_v4();
        append_completed(&mut journal, completion_id);
        journal.complete(completion_id).expect("complete");
        journal.acknowledge().expect("ack");
        let real_path = journal.path().to_path_buf();
        drop(journal);

        // Move the journal file onto the swapped run id's expected
        // path so recover_run(other) actually opens it.
        let swapped_path = dir
            .path()
            .join(format!("journal-{}.log", other_run_id.hyphenated()));
        std::fs::rename(&real_path, &swapped_path).expect("rename journal");

        let err = Journal::recover_run(dir.path(), other_run_id)
            .expect_err("recover_run must reject swapped journal");
        assert!(
            matches!(err, JournalError::RunIdMismatch { .. }),
            "expected RunIdMismatch, got {err:?}"
        );
    }

    #[test]
    fn acknowledge_before_complete_is_rejected() {
        let dir = temp_root();
        let run_id = Uuid::new_v4();
        let mut journal = Journal::create(dir.path(), run_id).expect("create");
        journal.append(new_record(run_id, 0, 1)).expect("append");

        let err = journal
            .acknowledge()
            .expect_err("ack before complete must fail");
        assert!(
            matches!(err, JournalError::InvalidState(_)),
            "expected InvalidState, got {err:?}"
        );
    }

    /// Append a single on-disk frame (header + payload) to `path` using
    /// the public wire format. Used by tests that forge frames for
    /// corruption scenarios.
    fn append_raw_frame(path: &Path, payload: &[u8]) {
        use std::io::Write;
        let mut f = OpenOptions::new().append(true).open(path).expect("open");
        write_frame(&mut f, payload).expect("write forged frame");
        f.flush().expect("flush");
    }

    #[cfg(unix)]
    #[test]
    fn concurrent_incomplete_writer_is_rejected_nonblocking() {
        let dir = temp_root();
        let run_id = Uuid::new_v4();
        let journal = Journal::create(dir.path(), run_id).expect("create first writer");

        let err = Journal::open(dir.path(), run_id).expect_err("second writer must be busy");
        assert!(matches!(
            err,
            JournalError::WriterBusy { run_id: busy_id, ref path }
                if busy_id == run_id && path == journal.path()
        ));
    }

    #[cfg(unix)]
    #[test]
    fn drop_of_incomplete_writer_releases_lock() {
        let dir = temp_root();
        let run_id = Uuid::new_v4();
        let mut journal = Journal::create(dir.path(), run_id).expect("create writer");
        journal.append(new_record(run_id, 0, 1)).expect("append");
        drop(journal);

        let reopened = Journal::open(dir.path(), run_id).expect("drop releases lock");
        assert_eq!(reopened.next_sequence(), 1);
    }

    #[cfg(unix)]
    #[test]
    fn complete_releases_lock_before_journal_drop() {
        let dir = temp_root();
        let run_id = Uuid::new_v4();
        let mut journal = Journal::create(dir.path(), run_id).expect("create writer");
        let completion_id = Uuid::new_v4();
        append_completed(&mut journal, completion_id);
        journal.complete(completion_id).expect("complete");

        let recovered = Journal::recover_run(dir.path(), run_id)
            .expect("completed writer must release ownership");
        assert_eq!(
            recovered.completion.map(|marker| marker.completion_id),
            Some(completion_id)
        );
        drop(journal);
    }

    #[cfg(unix)]
    #[test]
    fn recover_run_refuses_active_incomplete_writer_without_truncating() {
        let dir = temp_root();
        let run_id = Uuid::new_v4();
        let journal = Journal::create(dir.path(), run_id).expect("create writer");
        let path = journal.path().to_path_buf();
        let mut torn = OpenOptions::new()
            .append(true)
            .open(&path)
            .expect("open torn tail");
        torn.write_all(b"TP").expect("write torn tail");
        torn.flush().expect("flush torn tail");
        drop(torn);
        let before = std::fs::metadata(&path).expect("metadata").len();

        let err = Journal::recover_run(dir.path(), run_id).expect_err("active writer must be busy");
        assert!(
            matches!(err, JournalError::WriterBusy { run_id: busy_id, .. } if busy_id == run_id)
        );
        assert_eq!(
            std::fs::metadata(path).expect("metadata after").len(),
            before
        );
    }

    #[cfg(unix)]
    #[test]
    fn directory_creation_sync_failure_is_reported_and_retry_succeeds() {
        let dir = temp_root();
        let root = dir.path().join("one").join("two");
        let run_id = Uuid::new_v4();
        inject_durability_failure(TestDurabilityFailurePoint::DirectoryEntry);

        let err = Journal::create(&root, run_id).expect_err("directory sync failure must surface");
        assert!(matches!(err, JournalError::Io(_)));
        clear_durability_failure();
        let journal = Journal::create(&root, run_id).expect("retry repairs durability");
        assert_eq!(journal.run_id(), run_id);
    }

    #[cfg(unix)]
    #[test]
    fn journal_leaf_creation_syncs_file_and_containing_directory() {
        for failure in [
            TestDurabilityFailurePoint::LeafData,
            TestDurabilityFailurePoint::LeafParent,
        ] {
            let dir = temp_root();
            let run_id = Uuid::new_v4();
            inject_durability_failure(failure);
            let err = Journal::create(dir.path(), run_id).expect_err("creation sync must surface");
            assert!(matches!(err, JournalError::Io(_)));
            clear_durability_failure();
            Journal::create(dir.path(), run_id).expect("failed create can be retried");
        }
    }

    #[cfg(unix)]
    #[test]
    fn torn_tail_truncation_sync_failure_is_reported_before_recovery_returns() {
        let dir = temp_root();
        let run_id = Uuid::new_v4();
        let mut journal = Journal::create(dir.path(), run_id).expect("create");
        journal.append(new_record(run_id, 0, 7)).expect("append");
        let path = journal.path().to_path_buf();
        drop(journal);
        let mut file = OpenOptions::new()
            .append(true)
            .open(&path)
            .expect("open tail");
        file.write_all(b"TP").expect("write torn tail");
        file.flush().expect("flush tail");
        drop(file);

        inject_durability_failure(TestDurabilityFailurePoint::Truncate);
        let err = Journal::recover_run(dir.path(), run_id).expect_err("truncate sync must surface");
        assert!(matches!(err, JournalError::Io(_)));
        clear_durability_failure();
        let recovered = Journal::recover_run(dir.path(), run_id).expect("retry recovery");
        assert_eq!(recovered.records.len(), 1);
    }

    #[cfg(unix)]
    #[test]
    fn ancestor_swap_cannot_redirect_journal_creation() {
        use std::os::unix::fs::symlink;

        let dir = temp_root();
        let anchor = dir.path().join("anchor");
        let original_root = anchor.join("journals");
        let moved = dir.path().join("moved-anchor");
        let target = dir.path().join("attacker");
        std::fs::create_dir(&anchor).expect("anchor");
        std::fs::create_dir(&original_root).expect("journal root");
        std::fs::create_dir(&target).expect("attacker target");
        let anchor_for_hook = anchor.clone();
        let moved_for_hook = moved.clone();
        let target_for_hook = target.clone();
        install_root_walk_hook(anchor.clone(), move || {
            std::fs::rename(&anchor_for_hook, &moved_for_hook).expect("rename opened ancestor");
            symlink(&target_for_hook, &anchor_for_hook).expect("replace pathname with symlink");
        });

        let run_id = Uuid::new_v4();
        let result = Journal::create(&original_root, run_id);
        clear_root_walk_hook();
        if result.is_ok() {
            assert!(moved
                .join("journals")
                .join(journal_filename(run_id))
                .exists());
        }
        assert!(
            !target
                .join("journals")
                .join(journal_filename(run_id))
                .exists(),
            "descriptor-relative traversal must never create in symlink target"
        );
    }

    #[test]
    fn journal_file_is_created_with_mode_0600() {
        let dir = temp_root();
        let run_id = Uuid::new_v4();
        let journal = Journal::create(dir.path(), run_id).expect("create");

        let meta = std::fs::metadata(journal.path()).expect("metadata");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = meta.permissions().mode() & 0o777;
            assert_eq!(mode, JOURNAL_FILE_MODE, "expected 0600, got {mode:o}");
        }
        #[cfg(not(unix))]
        let _ = meta;
    }

    #[test]
    fn same_id_complete_is_idempotent_before_reopen() {
        let dir = temp_root();
        let run_id = Uuid::new_v4();
        let mut journal = Journal::create(dir.path(), run_id).expect("create");
        journal.append(new_record(run_id, 0, 3)).expect("append");

        let first = Uuid::new_v4();
        append_completed(&mut journal, first);
        journal.complete(first).expect("first complete");

        // Re-issuing the same completion id must be a no-op (no
        // duplicate frame written, no error surfaced). The byte-level
        // length must therefore be unchanged after the second call.
        let len_before = std::fs::metadata(journal.path()).unwrap().len();
        journal
            .complete(first)
            .expect("same-id complete is idempotent");
        let len_after = std::fs::metadata(journal.path()).unwrap().len();
        assert_eq!(
            len_before, len_after,
            "idempotent complete must not write another frame"
        );
    }

    #[test]
    fn different_id_complete_returns_completion_conflict() {
        let dir = temp_root();
        let run_id = Uuid::new_v4();
        let mut journal = Journal::create(dir.path(), run_id).expect("create");
        journal.append(new_record(run_id, 0, 1)).expect("append");

        let first = Uuid::new_v4();
        append_completed(&mut journal, first);
        journal.complete(first).expect("complete first time");
        let err = journal
            .complete(Uuid::new_v4())
            .expect_err("different-id complete must fail");
        assert!(
            matches!(err, JournalError::CompletionConflict { .. }),
            "expected CompletionConflict, got {err:?}"
        );
    }

    #[test]
    fn ack_and_retention_eligibility() {
        let dir = temp_root();
        let run_id = Uuid::new_v4();
        let mut journal = Journal::create(dir.path(), run_id).expect("create");
        journal.append(new_record(run_id, 0, 1)).expect("append");

        // An incomplete writer owns the journal, so recovery cannot race it.
        assert!(matches!(
            Journal::recover_run(dir.path(), run_id),
            Err(JournalError::WriterBusy { .. })
        ));

        let completion_id = Uuid::new_v4();
        append_completed(&mut journal, completion_id);
        journal.complete(completion_id).expect("complete");
        let snapshot = Journal::recover_run(dir.path(), run_id).expect("recover");
        assert!(!is_retention_eligible(
            &snapshot,
            Utc::now(),
            chrono::Duration::seconds(0)
        ));

        journal.acknowledge().expect("ack");
        let snapshot = Journal::recover_run(dir.path(), run_id).expect("recover");

        // Fresh ack — not eligible yet (zero-age tolerance).
        assert!(!is_retention_eligible(
            &snapshot,
            Utc::now(),
            chrono::Duration::seconds(0)
        ));

        // One second in the future — eligible.
        let future = Utc::now() + chrono::Duration::seconds(1);
        assert!(is_retention_eligible(
            &snapshot,
            future,
            chrono::Duration::seconds(0)
        ));

        // Negative age tolerance — never eligible by clock alone.
        assert!(!is_retention_eligible(
            &snapshot,
            Utc::now(),
            chrono::Duration::seconds(-1)
        ));
    }

    #[test]
    fn append_after_complete_is_rejected() {
        let dir = temp_root();
        let run_id = Uuid::new_v4();
        let mut journal = Journal::create(dir.path(), run_id).expect("create");
        journal.append(new_record(run_id, 0, 1)).expect("append");
        let completion_id = Uuid::new_v4();
        append_completed(&mut journal, completion_id);
        journal.complete(completion_id).expect("complete");

        let err = journal
            .append(new_record(run_id, 1, 1))
            .expect_err("append after complete must fail");
        assert!(matches!(err, JournalError::DuplicateTerminal(_)));
    }

    #[test]
    fn sequence_mismatch_detected() {
        let dir = temp_root();
        let run_id = Uuid::new_v4();
        let mut journal = Journal::create(dir.path(), run_id).expect("create");
        // Skip sequence 0; journal expects 0 next.
        let bad = JournalRecord {
            run_id,
            sequence: 1,
            process_id: 1,
            kind: OutputKind::Stdout,
            payload: serde_json::Value::String("oops".into()),
            completion_id: None,
            observed_at: Utc::now(),
        };
        let err = journal.append(bad).expect_err("should reject");
        assert!(matches!(err, JournalError::SequenceMismatch { .. }));
    }

    #[test]
    fn unsafe_path_parent_dir_rejected() {
        let dir = temp_root();
        let bad = dir.path().join("..").join("escaped");
        let err = Journal::create(&bad, Uuid::new_v4()).expect_err("must reject ..");
        assert!(matches!(err, JournalError::UnsafePath(_)));
    }

    #[test]
    fn unsafe_root_symlink_rejected() {
        let dir = temp_root();
        let target = dir.path().join("target");
        std::fs::create_dir_all(&target).expect("mkdir");
        let link_path = dir.path().join("link");
        std::os::unix::fs::symlink(&target, &link_path).expect("symlink");

        let err =
            Journal::create(&link_path, Uuid::new_v4()).expect_err("must reject symlinked root");
        assert!(matches!(err, JournalError::UnsafePath(_)));
    }

    #[test]
    fn unsafe_journal_file_symlink_rejected() {
        let dir = temp_root();
        let run_id = Uuid::new_v4();
        // Create a journal then replace it with a symlink to a
        // different path. Open must refuse.
        let mut journal = Journal::create(dir.path(), run_id).expect("create");
        journal.append(new_record(run_id, 0, 1)).expect("append");
        let target_path = journal.path().to_path_buf();
        drop(journal);

        // Move the real file aside and replace it with a symlink.
        let stash = dir.path().join("stash.log");
        std::fs::rename(&target_path, &stash).expect("rename");
        std::os::unix::fs::symlink(&stash, &target_path).expect("symlink");

        let err = Journal::recover_run(dir.path(), run_id)
            .expect_err("must reject symlinked journal file");
        assert!(matches!(err, JournalError::UnsafePath(_)));
    }

    // -- Unix path hardening (terraphim-ai#3269) ---------------------
    //
    // Each test below pins a single property of the new O_NOFOLLOW /
    // ancestor-symlink rejection path. They only run on Unix because
    // the helpers are gated on `cfg(unix)`.

    /// A symlink anywhere in the absolute root path is rejected by
    /// [`reject_ancestor_symlinks`]. The leaf of the path is itself
    /// the symlink — `validate_root` would not detect this because
    /// the final component does not yet exist on disk.
    #[cfg(unix)]
    #[test]
    fn ancestor_symlink_rejected_by_create() {
        let dir = temp_root();
        let target = dir.path().join("target");
        std::fs::create_dir_all(&target).expect("mkdir target");

        // Build a path whose leaf is an *existing* symlink. We pick
        // `<dir>/sneaky -> <dir>/target` and feed `create` the path
        // `<dir>/sneaky/inner`. The leaf `inner` does not exist, so
        // `validate_root` accepts the path; the helper must reject
        // the `<dir>/sneaky` symlink in the chain.
        let sneaky = dir.path().join("sneaky");
        std::os::unix::fs::symlink(&target, &sneaky).expect("symlink");

        let root_with_ancestor_symlink = sneaky.join("inner");
        let err = Journal::create(&root_with_ancestor_symlink, Uuid::new_v4())
            .expect_err("must reject ancestor symlink at create time");
        assert!(
            matches!(err, JournalError::UnsafePath(_)),
            "expected UnsafePath, got {err:?}"
        );

        // `inner` must not have been created.
        assert!(!root_with_ancestor_symlink.exists());
    }

    /// Same as above, but reached via [`Journal::recover_run`].
    #[cfg(unix)]
    #[test]
    fn ancestor_symlink_rejected_by_recover_run() {
        let dir = temp_root();
        let target = dir.path().join("target");
        std::fs::create_dir_all(&target).expect("mkdir target");

        let sneaky = dir.path().join("sneaky");
        std::os::unix::fs::symlink(&target, &sneaky).expect("symlink");

        let root_with_ancestor_symlink = sneaky.join("inner");
        let err = Journal::recover_run(&root_with_ancestor_symlink, Uuid::new_v4())
            .expect_err("must reject ancestor symlink at recover_run time");
        assert!(
            matches!(err, JournalError::UnsafePath(_)),
            "expected UnsafePath, got {err:?}"
        );
    }

    /// If the operator pre-created the journal root, [`Journal::create`]
    /// must not chmod it. This is the contract that lets operators
    /// drop a stricter ACL or sticky bit on the directory without
    /// the spawner silently loosening or tightening it.
    #[cfg(unix)]
    #[test]
    fn preexisting_root_mode_is_preserved() {
        use std::os::unix::fs::PermissionsExt;
        let dir = temp_root();
        let root = dir.path().join("preexisting");
        std::fs::create_dir_all(&root).expect("mkdir preexisting");
        // Operator chose 0750.
        std::fs::set_permissions(&root, std::fs::Permissions::from_mode(0o750))
            .expect("chmod 0750");

        let run_id = Uuid::new_v4();
        let journal = Journal::create(&root, run_id).expect("create");
        drop(journal);

        let mode = std::fs::metadata(&root)
            .expect("metadata")
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(
            mode, 0o750,
            "operator-chosen root mode must be preserved; got {mode:o}"
        );
    }

    /// When `Journal::create` had to create the root itself, it
    /// tightens the directory to 0700. This complements the
    /// preservation contract above.
    #[cfg(unix)]
    #[test]
    fn freshly_created_root_is_chmodded_to_0700() {
        use std::os::unix::fs::PermissionsExt;
        let dir = temp_root();
        let root = dir.path().join("fresh");
        assert!(!root.exists(), "test precondition: root must not exist");

        let journal = Journal::create(&root, Uuid::new_v4()).expect("create");
        drop(journal);

        let mode = std::fs::metadata(&root)
            .expect("metadata")
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(
            mode, 0o700,
            "freshly created root must be 0700; got {mode:o}"
        );
    }

    /// The leaf journal file is opened with `O_NOFOLLOW`, so the
    /// kernel itself rejects it even if the metadata pre-check is
    /// bypassed. We exercise the rejection through `Journal::open`,
    /// which today runs both the metadata check *and* the open with
    /// the flag — the test asserts the combined behaviour is
    /// refusal.
    #[cfg(unix)]
    #[test]
    fn leaf_symlink_open_is_rejected() {
        let dir = temp_root();
        let run_id = Uuid::new_v4();
        let mut journal = Journal::create(dir.path(), run_id).expect("create");
        journal.append(new_record(run_id, 0, 1)).expect("append");
        let journal_path = journal.path().to_path_buf();
        drop(journal);

        // Move the real file aside and replace it with a symlink to
        // an unrelated target so a successful `open` would resolve to
        // a different inode.
        let stash = dir.path().join("stash-open.log");
        std::fs::rename(&journal_path, &stash).expect("rename aside");
        std::os::unix::fs::symlink(&stash, &journal_path).expect("symlink");

        let err = Journal::open(dir.path(), run_id)
            .expect_err("must refuse to open symlinked journal file");
        assert!(
            matches!(err, JournalError::UnsafePath(_)),
            "expected UnsafePath, got {err:?}"
        );
    }

    /// Same scenario, but driven through [`Journal::recover_run`].
    #[cfg(unix)]
    #[test]
    fn leaf_symlink_recover_run_is_rejected() {
        let dir = temp_root();
        let run_id = Uuid::new_v4();
        let mut journal = Journal::create(dir.path(), run_id).expect("create");
        journal.append(new_record(run_id, 0, 1)).expect("append");
        let journal_path = journal.path().to_path_buf();
        drop(journal);

        let stash = dir.path().join("stash-recover.log");
        std::fs::rename(&journal_path, &stash).expect("rename aside");
        std::os::unix::fs::symlink(&stash, &journal_path).expect("symlink");

        let err = Journal::recover_run(dir.path(), run_id)
            .expect_err("must refuse to recover symlinked journal file");
        assert!(
            matches!(err, JournalError::UnsafePath(_)),
            "expected UnsafePath, got {err:?}"
        );
    }

    /// Under a permissive umask (0), the freshly created journal
    /// file must still end up at mode `0600`. The atomic
    /// `OpenOptionsExt::mode` argument is what makes this work
    /// without a follow-up chmod.
    ///
    /// This test mutates the process-wide umask; it is gated on
    /// `cfg(unix)` and restores the previous umask on every exit
    /// path. Other tests running in parallel may observe a
    /// different umask briefly, but they all create files with
    /// explicit modes or recover with `0600` so the temporary
    /// permissiveness does not affect their assertions.
    #[cfg(unix)]
    #[test]
    fn created_file_is_0600_under_permissive_umask() {
        use std::os::unix::fs::PermissionsExt;
        let dir = temp_root();
        let run_id = Uuid::new_v4();

        // Permissive mask so a non-atomic mode would leave the file
        // world-writable. The guard restores the previous umask on
        // every exit path, including failed assertions.
        let _umask_guard = UmaskGuard::install(0);
        let journal = Journal::create(dir.path(), run_id).expect("create under permissive umask");

        let mode = std::fs::metadata(journal.path())
            .expect("metadata")
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(
            mode, JOURNAL_FILE_MODE,
            "expected 0600 under permissive umask; got {mode:o}"
        );
    }

    #[test]
    fn unsafe_relative_root_rejected() {
        let err = Journal::create(Path::new("relative/path"), Uuid::new_v4())
            .expect_err("must reject relative root");
        assert!(matches!(err, JournalError::UnsafePath(_)));
    }

    #[cfg(unix)]
    #[test]
    fn opening_existing_journal_with_wrong_mode_is_rejected() {
        let dir = temp_root();
        let run_id = Uuid::new_v4();
        let journal = Journal::create(dir.path(), run_id).expect("create");
        let path = journal.path().to_path_buf();
        drop(journal);

        // Widen permissions to 0644.
        let mut perms = std::fs::metadata(&path).unwrap().permissions();
        perms.set_mode(0o644);
        std::fs::set_permissions(&path, perms).expect("chmod");

        let err = Journal::open(dir.path(), run_id).expect_err("must reject widened mode");
        assert!(matches!(err, JournalError::UnsafePath(_)));
    }

    #[test]
    fn stale_completed_handle_cannot_append_duplicate_ack() {
        let dir = temp_root();
        let run_id = Uuid::new_v4();
        let completion_id = Uuid::new_v4();
        let mut first = Journal::create(dir.path(), run_id).expect("create");
        first
            .append(JournalRecord {
                run_id,
                sequence: 0,
                process_id: 7,
                kind: OutputKind::Completed,
                payload: serde_json::json!({"exit_code": 0}),
                completion_id: Some(completion_id),
                observed_at: Utc::now(),
            })
            .expect("append completed");
        first.complete(completion_id).expect("complete");

        let mut stale = Journal::open(dir.path(), run_id).expect("open stale completed handle");
        first.acknowledge().expect("first ack");
        assert!(matches!(
            stale.acknowledge(),
            Err(JournalError::DuplicateTerminal(_))
        ));
        let recovered = Journal::recover_run(dir.path(), run_id).expect("recover valid journal");
        assert!(recovered.ack.is_some());
    }

    #[test]
    fn duplicate_ack_rejected() {
        let dir = temp_root();
        let run_id = Uuid::new_v4();
        let mut journal = Journal::create(dir.path(), run_id).expect("create");
        journal.append(new_record(run_id, 0, 1)).expect("append");
        let completion_id = Uuid::new_v4();
        append_completed(&mut journal, completion_id);
        journal.complete(completion_id).expect("complete");
        journal.acknowledge().expect("first ack");

        let err = journal.acknowledge().expect_err("second ack must fail");
        assert!(matches!(err, JournalError::DuplicateTerminal(_)));
    }

    // -- completion identity / cursor / checkpoint tests (slice 2) --

    /// After `open()`, re-issuing `complete(same_id)` must remain a
    /// no-op: the on-disk frame is unchanged and the call returns
    /// `Ok(())`.
    #[test]
    fn same_id_complete_is_idempotent_after_reopen() {
        let dir = temp_root();
        let run_id = Uuid::new_v4();
        let completion_id = Uuid::new_v4();
        {
            let mut journal = Journal::create(dir.path(), run_id).expect("create");
            journal.append(new_record(run_id, 0, 1)).expect("append");
            append_completed(&mut journal, completion_id);
            journal.complete(completion_id).expect("complete");
        }
        let mut journal = Journal::open(dir.path(), run_id).expect("reopen");
        let len_before = std::fs::metadata(journal.path()).unwrap().len();
        journal
            .complete(completion_id)
            .expect("same-id complete after reopen is idempotent");
        let len_after = std::fs::metadata(journal.path()).unwrap().len();
        assert_eq!(
            len_before, len_after,
            "idempotent complete after reopen must not write another frame"
        );
    }

    /// Appending a record whose `completion_id` differs from a record
    /// already on disk must fail before any new frame is written.
    #[test]
    fn append_with_disagreeing_record_completion_id_is_rejected() {
        let dir = temp_root();
        let run_id = Uuid::new_v4();
        let mut journal = Journal::create(dir.path(), run_id).expect("create");
        journal
            .append(JournalRecord {
                run_id,
                sequence: 0,
                process_id: 1,
                kind: OutputKind::Completed,
                payload: serde_json::json!({"exit_code": 0}),
                completion_id: Some(Uuid::new_v4()),
                observed_at: Utc::now(),
            })
            .expect("first tagged record");
        let err = journal
            .append(JournalRecord {
                run_id,
                sequence: 1,
                process_id: 1,
                kind: OutputKind::Completed,
                payload: serde_json::json!({"exit_code": 0}),
                completion_id: Some(Uuid::new_v4()),
                observed_at: Utc::now(),
            })
            .expect_err("disagreeing record completion_id must fail");
        assert!(
            matches!(err, JournalError::DuplicateTerminal(_)),
            "expected DuplicateTerminal, got {err:?}"
        );
        // Offending frame must not be durably written.
        let len = std::fs::metadata(journal.path()).unwrap().len();
        assert!(len > 0, "first frame must remain on disk");
    }

    /// Calling `complete(id)` after a record stamped a *different*
    /// completion id must fail with `CompletionConflict`.
    #[test]
    fn complete_with_disagreeing_record_completion_id_is_rejected() {
        let dir = temp_root();
        let run_id = Uuid::new_v4();
        let mut journal = Journal::create(dir.path(), run_id).expect("create");
        journal
            .append(JournalRecord {
                run_id,
                sequence: 0,
                process_id: 1,
                kind: OutputKind::Completed,
                payload: serde_json::json!({"exit_code": 0}),
                completion_id: Some(Uuid::new_v4()),
                observed_at: Utc::now(),
            })
            .expect("tagged record");
        let err = journal
            .complete(Uuid::new_v4())
            .expect_err("complete must reject disagreeing record id");
        assert!(
            matches!(err, JournalError::CompletionConflict { .. }),
            "expected CompletionConflict, got {err:?}"
        );
    }

    /// On-disk mismatch: a `JournalRecord.completion_id` that disagrees
    /// with the `Complete` frame's id must be detected by recovery.
    #[test]
    fn recovery_rejects_record_vs_complete_completion_mismatch() {
        let dir = temp_root();
        let run_id = Uuid::new_v4();
        let mut journal = Journal::create(dir.path(), run_id).expect("create");
        journal
            .append(JournalRecord {
                run_id,
                sequence: 0,
                process_id: 1,
                kind: OutputKind::Completed,
                payload: serde_json::json!({"exit_code": 0}),
                completion_id: Some(Uuid::new_v4()),
                observed_at: Utc::now(),
            })
            .expect("record stamped with id");
        // Complete with a *different* id: the journal's append-time
        // validator allows this because no prior record completion_id
        // is known — but recovery must catch the disagreement.
        let record_id = journal
            .path()
            .to_path_buf()
            .canonicalize()
            .unwrap_or_else(|_| journal.path().to_path_buf());
        drop(journal);

        // Forge a Complete frame with a mismatched id by appending a
        // raw frame to disk.
        let forged = JournalFrame::Complete {
            run_id,
            completion_id: Uuid::new_v4(),
            observed_at: Utc::now(),
        };
        let payload = serde_json::to_vec(&forged).expect("encode forged complete");
        append_raw_frame(&record_id, &payload);

        let err = Journal::recover_run(dir.path(), run_id)
            .expect_err("recovery must reject record-vs-complete mismatch");
        assert!(
            matches!(err, JournalError::CompletionConflict { .. }),
            "expected CompletionConflict, got {err:?}"
        );
    }

    /// Two records stamped with different ids — with no Complete frame —
    /// must also be rejected at recovery time.
    #[test]
    fn recovery_rejects_record_vs_record_completion_mismatch() {
        let dir = temp_root();
        let run_id = Uuid::new_v4();
        let mut journal = Journal::create(dir.path(), run_id).expect("create");
        journal
            .append(JournalRecord {
                run_id,
                sequence: 0,
                process_id: 1,
                kind: OutputKind::Completed,
                payload: serde_json::json!({"exit_code": 0}),
                completion_id: Some(Uuid::new_v4()),
                observed_at: Utc::now(),
            })
            .expect("first stamped record");
        journal
            .append(JournalRecord {
                run_id,
                sequence: 1,
                process_id: 1,
                kind: OutputKind::Completed,
                payload: serde_json::json!({"exit_code": 0}),
                completion_id: Some(Uuid::new_v4()),
                observed_at: Utc::now(),
            })
            .expect_err("append-time check would reject; if it accepts, recovery still must");
        // Replay the journal on disk to make sure recovery catches the
        // disagreement if the append-time check ever weakens.
        let recovered = Journal::recover_run(dir.path(), run_id);
        // Either recovery returns CompletionConflict (good — record-vs-record
        // mismatch detected) or, if append-time allowed it, the recovery
        // path *must* catch it. The lenient path here is OK only when
        // append-time rejected the second frame and there is no
        // disagreement on disk.
        if let Ok(recovered) = recovered {
            // Append-time must have rejected the second record, so on
            // disk only the first (tagged) record exists.
            assert_eq!(recovered.records.len(), 1);
            assert_eq!(
                recovered.records[0].sequence, 0,
                "second tagged record must be rejected at append time"
            );
        }
    }

    /// Round-trip a checkpoint via JSON: serialize it, deserialize it,
    /// and resume consuming from the recovered journal.
    #[test]
    fn checkpoint_roundtrips_through_serde_and_resumes_consumption() {
        let dir = temp_root();
        let run_id = Uuid::new_v4();
        let completion_id = Uuid::new_v4();
        let mut journal = Journal::create(dir.path(), run_id).expect("create");
        for seq in 0..5u64 {
            journal.append(new_record(run_id, seq, 7)).expect("append");
        }
        append_completed(&mut journal, completion_id);
        journal.complete(completion_id).expect("complete");

        let recovered = Journal::recover_run(dir.path(), run_id).expect("recover");
        let cp = recovered.checkpoint();
        assert_eq!(cp.run_id, run_id);
        assert_eq!(cp.last_sequence, Some(5));
        assert_eq!(cp.completion_id, Some(completion_id));

        // Serialise and replay.
        let bytes = serde_json::to_vec(&cp).expect("serialize checkpoint");
        let restored: JournalCheckpoint = serde_json::from_slice(&bytes).expect("deserialize");

        // Resume from the checkpoint: with max_records large, we get
        // an empty slice (we've already consumed the tail).
        let tail = recovered
            .records_after(&restored, 100)
            .expect("resume after checkpoint");
        assert!(
            tail.is_empty(),
            "checkpoint at the tail must yield no further records"
        );

        // Resume from `Some(2)` with `max_records = 2`: we get seq 3, 4.
        let cp_mid = JournalCheckpoint {
            run_id,
            last_sequence: Some(2),
            completion_id: None,
        };
        let slice = recovered
            .records_after(&cp_mid, 2)
            .expect("records after mid-checkpoint");
        assert_eq!(slice.len(), 2);
        assert_eq!(slice[0].sequence, 3);
        assert_eq!(slice[1].sequence, 4);

        // Resume from `None`: we get records from the very start.
        let cp_begin = JournalCheckpoint {
            run_id,
            last_sequence: None,
            completion_id: None,
        };
        let slice = recovered
            .records_after(&cp_begin, 3)
            .expect("records from the start");
        assert_eq!(slice.len(), 3);
        assert_eq!(slice[0].sequence, 0);
        assert_eq!(slice[1].sequence, 1);
        assert_eq!(slice[2].sequence, 2);
    }

    /// `records_after` must reject a checkpoint whose `run_id` doesn't
    /// match the journal's run.
    #[test]
    fn records_after_rejects_wrong_run_id() {
        let dir = temp_root();
        let run_id = Uuid::new_v4();
        let mut journal = Journal::create(dir.path(), run_id).expect("create");
        journal.append(new_record(run_id, 0, 1)).expect("append");
        drop(journal);

        let recovered = Journal::recover_run(dir.path(), run_id).expect("recover");
        let wrong = JournalCheckpoint {
            run_id: Uuid::new_v4(),
            last_sequence: None,
            completion_id: None,
        };
        let err = recovered
            .records_after(&wrong, 10)
            .expect_err("must reject wrong run id");
        assert!(matches!(err, JournalError::RunIdMismatch { .. }));
    }

    /// `records_after` must reject a checkpoint whose cursor points
    /// beyond the journal's record tail.
    #[test]
    fn records_after_rejects_out_of_bounds_cursor() {
        let dir = temp_root();
        let run_id = Uuid::new_v4();
        let mut journal = Journal::create(dir.path(), run_id).expect("create");
        for seq in 0..3u64 {
            journal.append(new_record(run_id, seq, 1)).expect("append");
        }
        drop(journal);

        let recovered = Journal::recover_run(dir.path(), run_id).expect("recover");
        let bad = JournalCheckpoint {
            run_id,
            last_sequence: Some(99),
            completion_id: None,
        };
        let err = recovered
            .records_after(&bad, 10)
            .expect_err("must reject out-of-bounds cursor");
        assert!(matches!(err, JournalError::SequenceMismatch { .. }));
    }

    /// `records_after` must reject a checkpoint whose `completion_id`
    /// disagrees with the journal's completion frame.
    #[test]
    fn records_after_rejects_stale_completion_id() {
        let dir = temp_root();
        let run_id = Uuid::new_v4();
        let completion_id = Uuid::new_v4();
        let mut journal = Journal::create(dir.path(), run_id).expect("create");
        journal.append(new_record(run_id, 0, 1)).expect("append");
        append_completed(&mut journal, completion_id);
        journal.complete(completion_id).expect("complete");
        drop(journal);

        let recovered = Journal::recover_run(dir.path(), run_id).expect("recover");
        let stale = JournalCheckpoint {
            run_id,
            last_sequence: Some(1),
            completion_id: Some(Uuid::new_v4()),
        };
        let err = recovered
            .records_after(&stale, 10)
            .expect_err("must reject stale completion_id");
        assert!(
            matches!(err, JournalError::CompletionConflict { .. }),
            "expected CompletionConflict, got {err:?}"
        );
    }

    /// `records_after` must reject `max_records == 0`.
    #[test]
    fn records_after_rejects_zero_max_records() {
        let dir = temp_root();
        let run_id = Uuid::new_v4();
        let mut journal = Journal::create(dir.path(), run_id).expect("create");
        journal.append(new_record(run_id, 0, 1)).expect("append");
        drop(journal);

        let recovered = Journal::recover_run(dir.path(), run_id).expect("recover");
        let cp = JournalCheckpoint {
            run_id,
            last_sequence: None,
            completion_id: None,
        };
        let err = recovered
            .records_after(&cp, 0)
            .expect_err("must reject max_records = 0");
        assert!(matches!(err, JournalError::InvalidState(_)));
    }

    // -- empty-journal checkpoint / run-id gate (slice 2 follow-up) --
    //
    // Once `RecoveredJournal` carries `run_id` directly, recovery of an
    // empty file must still surface the run id, and the checkpoint
    // round-trip plus `records_after` path must work without records
    // and without the old `Uuid::nil` sentinel. The wrong-run-id case
    // proves the gate survives the empty path.

    /// A freshly-created journal holds no frames yet. Recovery must
    /// surface `RecoveredJournal::run_id` (so callers can build a
    /// checkpoint), and a checkpoint serialised to JSON must round-trip
    /// back into a value that resumes consumption with an empty
    /// record slice.
    #[test]
    fn empty_incomplete_checkpoint_roundtrips_through_serde_and_resumes_consumption() {
        let dir = temp_root();
        let run_id = Uuid::new_v4();
        // Create a journal and never append or complete — the file is
        // empty of frames.
        let journal = Journal::create(dir.path(), run_id).expect("create");
        drop(journal);

        let recovered = Journal::recover_run(dir.path(), run_id).expect("recover empty");
        // Authoritative run id must be present even with no records.
        assert_eq!(recovered.run_id, run_id);
        assert!(recovered.records.is_empty(), "no records appended yet");
        assert!(recovered.completion.is_none());
        assert!(recovered.ack.is_none());

        let cp = recovered.checkpoint();
        assert_eq!(cp.run_id, run_id, "checkpoint run_id must match");
        assert_eq!(cp.last_sequence, None);
        assert_eq!(cp.completion_id, None);

        // Serialise the checkpoint to JSON, deserialise it, and feed
        // the restored value back into `records_after` — the empty
        // path must yield an empty slice without errors.
        let bytes = serde_json::to_vec(&cp).expect("serialize checkpoint");
        let restored: JournalCheckpoint = serde_json::from_slice(&bytes).expect("deserialize");
        let tail = recovered
            .records_after(&restored, 16)
            .expect("resume after empty checkpoint");
        assert!(
            tail.is_empty(),
            "empty journal must yield no records; got {} records",
            tail.len()
        );
    }

    /// The writer must refuse to create a Complete frame without a
    /// preceding completion-tagged `Completed` record.
    #[test]
    fn complete_without_completed_record_is_rejected_before_write() {
        let dir = temp_root();
        let run_id = Uuid::new_v4();
        let completion_id = Uuid::new_v4();
        let mut journal = Journal::create(dir.path(), run_id).expect("create");
        let err = journal
            .complete(completion_id)
            .expect_err("complete-only journal must be rejected before write");
        assert!(matches!(err, JournalError::InvalidState(_)), "{err:?}");
        drop(journal);
        let recovered = Journal::recover_run(dir.path(), run_id).expect("recover empty journal");
        assert!(recovered.records.is_empty());
        assert!(recovered.completion.is_none());
    }

    /// A wrong `run_id` on a checkpoint fed to an empty journal must be
    /// rejected — the run-id gate cannot rely on a `Uuid::nil`
    /// sentinel or on "first record's run_id", because empty journals
    /// expose neither.
    #[test]
    fn empty_journal_rejects_wrong_run_id_in_records_after() {
        let dir = temp_root();
        let run_id = Uuid::new_v4();
        let journal = Journal::create(dir.path(), run_id).expect("create");
        drop(journal);

        let recovered = Journal::recover_run(dir.path(), run_id).expect("recover empty");
        let wrong = JournalCheckpoint {
            run_id: Uuid::new_v4(),
            last_sequence: None,
            completion_id: None,
        };
        let err = recovered
            .records_after(&wrong, 16)
            .expect_err("wrong run_id must be rejected on empty journal");
        match err {
            JournalError::RunIdMismatch { journal, frame } => {
                assert_eq!(journal, run_id);
                assert_eq!(frame, wrong.run_id);
            }
            other => panic!("expected RunIdMismatch, got {other:?}"),
        }
    }

    // -- write-poisoning tests (slice 3) --
    //
    // Each test below injects a failure at exactly one of the three
    // IO boundaries (`write_frame`, `flush`, `sync_data`) and asserts
    // that:
    //   * the call that crossed the boundary surfaces the *original*
    //     I/O error before any state mutation completes, and
    //   * every subsequent `append` / `complete` / `acknowledge` call
    //     refuses with [`JournalError::Poisoned`] without touching the
    //     file again.
    //
    // The injection helpers and `fail_at` slot are `cfg(test)` only,
    // so production builds exercise the same code paths with no
    // behavioural change.

    /// A failure at the `write_frame` boundary poisons the journal: the
    /// first `append` returns the original I/O error, every later
    /// mutation returns [`JournalError::Poisoned`].
    #[test]
    fn write_boundary_failure_poisons_journal() {
        let dir = temp_root();
        let run_id = Uuid::new_v4();
        let mut journal = Journal::create(dir.path(), run_id).expect("create");

        // Arm the next write to fail.
        journal.fail_at = Some(TestFailurePoint::Write);

        // First append: crosses the Write boundary and gets the
        // injected I/O error. State must not advance.
        let err = journal
            .append(new_record(run_id, 0, 1))
            .expect_err("write boundary must fail");
        assert!(
            matches!(err, JournalError::Io(_)),
            "expected Io, got {err:?}"
        );

        // Subsequent mutations refuse without touching the file.
        let err = journal
            .append(new_record(run_id, 0, 1))
            .expect_err("append after poison must fail");
        assert!(
            matches!(err, JournalError::Poisoned),
            "expected Poisoned, got {err:?}"
        );

        let err = journal
            .complete(Uuid::new_v4())
            .expect_err("complete after poison must fail");
        assert!(
            matches!(err, JournalError::Poisoned),
            "expected Poisoned, got {err:?}"
        );

        let err = journal
            .acknowledge()
            .expect_err("ack after poison must fail");
        assert!(
            matches!(err, JournalError::Poisoned),
            "expected Poisoned, got {err:?}"
        );
    }

    /// A failure at the `flush` boundary poisons the journal: the
    /// first `append` writes the frame but fails to flush, then every
    /// later mutation returns [`JournalError::Poisoned`].
    #[test]
    fn flush_boundary_failure_poisons_journal() {
        let dir = temp_root();
        let run_id = Uuid::new_v4();
        let mut journal = Journal::create(dir.path(), run_id).expect("create");

        // Arm the next flush to fail. The Write boundary runs first
        // and succeeds; flush is the second boundary in `append`.
        journal.fail_at = Some(TestFailurePoint::Flush);

        let err = journal
            .append(new_record(run_id, 0, 1))
            .expect_err("flush boundary must fail");
        assert!(
            matches!(err, JournalError::Io(_)),
            "expected Io, got {err:?}"
        );

        // State did not advance — same sequence still expected.
        let err = journal
            .append(new_record(run_id, 0, 1))
            .expect_err("append after poison must fail");
        assert!(
            matches!(err, JournalError::Poisoned),
            "expected Poisoned, got {err:?}"
        );

        let err = journal
            .complete(Uuid::new_v4())
            .expect_err("complete after poison must fail");
        assert!(
            matches!(err, JournalError::Poisoned),
            "expected Poisoned, got {err:?}"
        );

        let err = journal
            .acknowledge()
            .expect_err("ack after poison must fail");
        assert!(
            matches!(err, JournalError::Poisoned),
            "expected Poisoned, got {err:?}"
        );
    }

    /// A failure at the `sync_data` boundary poisons the journal: a
    /// successful `append` and the start of `complete` run, then
    /// `sync_data` fails and every later mutation returns
    /// [`JournalError::Poisoned`].
    #[test]
    fn sync_boundary_failure_poisons_journal() {
        let dir = temp_root();
        let run_id = Uuid::new_v4();
        let mut journal = Journal::create(dir.path(), run_id).expect("create");

        // Successfully append one record so we have a journal that can
        // reach the `sync_data` boundary inside `complete`.
        journal.append(new_record(run_id, 0, 1)).expect("append");
        let completion_id = Uuid::new_v4();
        append_completed(&mut journal, completion_id);

        // Arm the next sync to fail. Write and flush in `complete` run
        // first; only the third boundary in `complete` trips.
        journal.fail_at = Some(TestFailurePoint::Sync);

        let err = journal
            .complete(completion_id)
            .expect_err("sync boundary must fail");
        assert!(
            matches!(err, JournalError::Io(_)),
            "expected Io, got {err:?}"
        );

        // Subsequent mutations refuse.
        let err = journal
            .append(new_record(run_id, 1, 1))
            .expect_err("append after poison must fail");
        assert!(
            matches!(err, JournalError::Poisoned),
            "expected Poisoned, got {err:?}"
        );

        let err = journal
            .complete(Uuid::new_v4())
            .expect_err("complete after poison must fail");
        assert!(
            matches!(err, JournalError::Poisoned),
            "expected Poisoned, got {err:?}"
        );

        let err = journal
            .acknowledge()
            .expect_err("ack after poison must fail");
        assert!(
            matches!(err, JournalError::Poisoned),
            "expected Poisoned, got {err:?}"
        );

        let err = Journal::recover_run(dir.path(), run_id)
            .expect_err("failed terminal sync must retain ownership");
        assert!(matches!(err, JournalError::WriterBusy { .. }));
        drop(journal);
        Journal::recover_run(dir.path(), run_id).expect("drop releases poisoned ownership");
    }

    // -- completion_record + getter tests (slice 3) --

    /// Stamping a `completion_id` on a record whose kind is not
    /// `OutputKind::Completed` is a writer contract violation. The
    /// journal must reject the record before any bytes reach the file.
    #[test]
    fn append_with_completion_id_on_non_completed_kind_is_rejected() {
        let dir = temp_root();
        let run_id = Uuid::new_v4();
        let mut journal = Journal::create(dir.path(), run_id).expect("create");

        // Each non-Completed variant must be refused outright.
        for kind in [
            OutputKind::Stdout,
            OutputKind::Stderr,
            OutputKind::Mention,
            OutputKind::Heartbeat,
        ] {
            let err = journal
                .append(JournalRecord {
                    run_id,
                    sequence: 0,
                    process_id: 1,
                    kind,
                    payload: serde_json::Value::String("oops".into()),
                    completion_id: Some(Uuid::new_v4()),
                    observed_at: Utc::now(),
                })
                .expect_err("tagged non-Completed record must be rejected");
            assert!(
                matches!(err, JournalError::InvalidState(_)),
                "expected InvalidState for kind {kind:?}, got {err:?}"
            );
        }

        // No bytes must have reached the file: a fresh journal has
        // length zero.
        let len = std::fs::metadata(journal.path()).unwrap().len();
        assert_eq!(len, 0, "rejected frames must not be written to disk");

        // And the journal's in-memory getters must remain at their
        // initial state — no completion identity, no pending record.
        assert_eq!(journal.next_sequence(), 0);
        assert!(!journal.is_completed());
        assert_eq!(journal.completion_identity(), None);
        assert!(journal.pending_completion_record().is_none());
    }

    /// Positive counterpart: a `Completed` record stamped with a
    /// `completion_id` is accepted and installs the tagged record in
    /// the journal's completion slot.
    #[test]
    fn append_with_completion_id_on_completed_kind_is_accepted() {
        let dir = temp_root();
        let run_id = Uuid::new_v4();
        let completion_id = Uuid::new_v4();
        let mut journal = Journal::create(dir.path(), run_id).expect("create");

        journal.append(new_record(run_id, 0, 7)).expect("append");
        journal
            .append(JournalRecord {
                run_id,
                sequence: 1,
                process_id: 7,
                kind: OutputKind::Completed,
                payload: serde_json::json!({"exit_code": 0}),
                completion_id: Some(completion_id),
                observed_at: Utc::now(),
            })
            .expect("tagged Completed record must be accepted");

        assert_eq!(journal.next_sequence(), 2);
        assert!(!journal.is_completed());
        assert_eq!(journal.completion_identity(), Some(completion_id));
        let pending = journal
            .pending_completion_record()
            .expect("pending record must be visible before complete");
        assert_eq!(pending.completion_id, Some(completion_id));
        assert_eq!(pending.kind, OutputKind::Completed);
        assert_eq!(pending.sequence, 1);
    }

    /// End-to-end getter lifecycle: create → append → tagged append →
    /// complete → reopen, exercising every getter at every phase.
    #[test]
    fn getters_reflect_state_through_create_complete_and_reopen() {
        let dir = temp_root();
        let run_id = Uuid::new_v4();
        let completion_id = Uuid::new_v4();

        let mut journal = Journal::create(dir.path(), run_id).expect("create");

        // Phase 1: fresh create — every getter is at its initial value.
        assert_eq!(journal.next_sequence(), 0);
        assert!(!journal.is_completed());
        assert_eq!(journal.completion_identity(), None);
        assert!(journal.pending_completion_record().is_none());

        // Phase 2: append one untagged record, then the tagged
        // Completed record. The pending slot must surface it;
        // completion_identity must return the pending id (no terminal
        // marker yet).
        journal.append(new_record(run_id, 0, 1)).expect("append");
        assert_eq!(journal.next_sequence(), 1);
        assert!(!journal.is_completed());
        assert_eq!(journal.completion_identity(), None);
        assert!(journal.pending_completion_record().is_none());

        journal
            .append(JournalRecord {
                run_id,
                sequence: 1,
                process_id: 1,
                kind: OutputKind::Completed,
                payload: serde_json::json!({"exit_code": 0}),
                completion_id: Some(completion_id),
                observed_at: Utc::now(),
            })
            .expect("append tagged Completed");
        assert_eq!(journal.next_sequence(), 2);
        assert!(!journal.is_completed());
        assert_eq!(journal.completion_identity(), Some(completion_id));
        let pending = journal
            .pending_completion_record()
            .expect("pending record after tagged append");
        assert_eq!(pending.completion_id, Some(completion_id));
        assert_eq!(pending.sequence, 1);

        // Phase 3: complete with the matching id. is_completed flips;
        // completion_identity still returns the same id (terminal
        // marker is now the canonical source); pending_completion_record
        // returns None because the record is no longer "pending".
        journal.complete(completion_id).expect("complete");
        assert_eq!(journal.next_sequence(), 2);
        assert!(journal.is_completed());
        assert_eq!(journal.completion_identity(), Some(completion_id));
        assert!(
            journal.pending_completion_record().is_none(),
            "pending_completion_record must hide the sealed record"
        );
        drop(journal);

        // Phase 4: reopen after completion. Getters must reflect the
        // recovered on-disk state — completed, terminal marker id,
        // and no pending record (the tagged record survives in the
        // in-memory slot but is hidden by is_completed).
        let journal = Journal::open(dir.path(), run_id).expect("reopen after complete");
        assert_eq!(journal.next_sequence(), 2);
        assert!(journal.is_completed());
        assert_eq!(journal.completion_identity(), Some(completion_id));
        assert!(journal.pending_completion_record().is_none());
    }

    /// Reopen *before* completion must still surface the tagged record
    /// through the pending getter — the record is on disk and is the
    /// journal's only completion signal until `complete()` runs.
    #[test]
    fn pending_completion_record_survives_reopen_before_complete() {
        let dir = temp_root();
        let run_id = Uuid::new_v4();
        let pending_id = Uuid::new_v4();

        {
            let mut journal = Journal::create(dir.path(), run_id).expect("create");
            journal.append(new_record(run_id, 0, 1)).expect("append");
            journal
                .append(JournalRecord {
                    run_id,
                    sequence: 1,
                    process_id: 1,
                    kind: OutputKind::Completed,
                    payload: serde_json::json!({"exit_code": 0}),
                    completion_id: Some(pending_id),
                    observed_at: Utc::now(),
                })
                .expect("append tagged Completed");
            drop(journal);
        }

        let journal = Journal::open(dir.path(), run_id).expect("reopen before complete");
        assert_eq!(journal.next_sequence(), 2);
        assert!(!journal.is_completed());
        assert_eq!(journal.completion_identity(), Some(pending_id));
        let pending = journal
            .pending_completion_record()
            .expect("pending record must survive reopen");
        assert_eq!(pending.completion_id, Some(pending_id));
        assert_eq!(pending.kind, OutputKind::Completed);
        assert_eq!(pending.sequence, 1);
    }

    fn completed_record(
        run_id: Uuid,
        sequence: u64,
        completion_id: Option<Uuid>,
        payload: serde_json::Value,
    ) -> JournalRecord {
        JournalRecord {
            run_id,
            sequence,
            process_id: 1,
            kind: OutputKind::Completed,
            payload,
            completion_id,
            observed_at: Utc::now(),
        }
    }

    fn forge_frames(root: &Path, run_id: Uuid, frames: Vec<JournalFrame>) {
        let journal = Journal::create(root, run_id).expect("create forged journal");
        let path = journal.path().to_path_buf();
        drop(journal);
        for frame in frames {
            let payload = serde_json::to_vec(&frame).expect("encode forged frame");
            append_raw_frame(&path, &payload);
        }
    }

    fn append_completed(journal: &mut Journal, completion_id: Uuid) {
        journal
            .append(completed_record(
                journal.run_id(),
                journal.next_sequence(),
                Some(completion_id),
                serde_json::json!({"exit_code": 0}),
            ))
            .expect("append Completed record");
    }

    #[test]
    fn recovery_rejects_data_after_complete() {
        let dir = temp_root();
        let run_id = Uuid::new_v4();
        let completion_id = Uuid::new_v4();
        forge_frames(
            dir.path(),
            run_id,
            vec![
                JournalFrame::Data(completed_record(
                    run_id,
                    0,
                    Some(completion_id),
                    serde_json::json!({"exit_code": 0}),
                )),
                JournalFrame::Complete {
                    run_id,
                    completion_id,
                    observed_at: Utc::now(),
                },
                JournalFrame::Data(new_record(run_id, 1, 1)),
            ],
        );

        let err = Journal::recover_run(dir.path(), run_id)
            .expect_err("data after Complete must be rejected");
        assert!(matches!(err, JournalError::InvalidState(_)));
    }

    #[test]
    fn recovery_rejects_data_after_ack() {
        let dir = temp_root();
        let run_id = Uuid::new_v4();
        let completion_id = Uuid::new_v4();
        forge_frames(
            dir.path(),
            run_id,
            vec![
                JournalFrame::Data(completed_record(
                    run_id,
                    0,
                    Some(completion_id),
                    serde_json::json!({"exit_code": 0}),
                )),
                JournalFrame::Complete {
                    run_id,
                    completion_id,
                    observed_at: Utc::now(),
                },
                JournalFrame::Ack {
                    run_id,
                    observed_at: Utc::now(),
                },
                JournalFrame::Data(new_record(run_id, 1, 1)),
            ],
        );

        let err =
            Journal::recover_run(dir.path(), run_id).expect_err("data after Ack must be rejected");
        assert!(matches!(err, JournalError::InvalidState(_)));
    }

    #[test]
    fn recovery_rejects_ack_before_complete() {
        let dir = temp_root();
        let run_id = Uuid::new_v4();
        forge_frames(
            dir.path(),
            run_id,
            vec![JournalFrame::Ack {
                run_id,
                observed_at: Utc::now(),
            }],
        );

        let err = Journal::recover_run(dir.path(), run_id)
            .expect_err("Ack before Complete must be rejected");
        assert!(matches!(err, JournalError::InvalidState(_)));
    }

    #[test]
    fn recovery_rejects_complete_after_ack() {
        let dir = temp_root();
        let run_id = Uuid::new_v4();
        let completion_id = Uuid::new_v4();
        forge_frames(
            dir.path(),
            run_id,
            vec![
                JournalFrame::Data(completed_record(
                    run_id,
                    0,
                    Some(completion_id),
                    serde_json::json!({"exit_code": 0}),
                )),
                JournalFrame::Complete {
                    run_id,
                    completion_id,
                    observed_at: Utc::now(),
                },
                JournalFrame::Ack {
                    run_id,
                    observed_at: Utc::now(),
                },
                JournalFrame::Complete {
                    run_id,
                    completion_id,
                    observed_at: Utc::now(),
                },
            ],
        );

        let err = Journal::recover_run(dir.path(), run_id)
            .expect_err("Complete after Ack must be rejected");
        assert!(matches!(err, JournalError::InvalidState(_)));
    }

    #[test]
    fn recovery_rejects_completion_id_on_non_completed_record() {
        let dir = temp_root();
        let run_id = Uuid::new_v4();
        let mut record = new_record(run_id, 0, 1);
        record.completion_id = Some(Uuid::new_v4());
        forge_frames(dir.path(), run_id, vec![JournalFrame::Data(record)]);

        let err = Journal::recover_run(dir.path(), run_id)
            .expect_err("completion_id on non-Completed record must be rejected");
        assert!(matches!(err, JournalError::InvalidState(_)));
    }

    #[test]
    fn recovery_rejects_completed_without_completion_id() {
        let dir = temp_root();
        let run_id = Uuid::new_v4();
        forge_frames(
            dir.path(),
            run_id,
            vec![JournalFrame::Data(completed_record(
                run_id,
                0,
                None,
                serde_json::json!({"exit_code": 0}),
            ))],
        );

        let err = Journal::recover_run(dir.path(), run_id)
            .expect_err("Completed without completion_id must be rejected");
        assert!(matches!(err, JournalError::InvalidState(_)));
    }

    #[test]
    fn recovery_rejects_complete_without_completed_record() {
        let dir = temp_root();
        let run_id = Uuid::new_v4();
        let completion_id = Uuid::new_v4();
        forge_frames(
            dir.path(),
            run_id,
            vec![JournalFrame::Complete {
                run_id,
                completion_id,
                observed_at: Utc::now(),
            }],
        );

        let err = Journal::recover_run(dir.path(), run_id)
            .expect_err("Complete without a preceding Completed record must be rejected");
        assert!(matches!(err, JournalError::InvalidState(_)));
    }

    #[test]
    fn recovery_rejects_multiple_completed_records_with_same_id_and_payload() {
        let dir = temp_root();
        let run_id = Uuid::new_v4();
        let completion_id = Uuid::new_v4();
        let payload = serde_json::json!({"exit_code": 0});
        forge_frames(
            dir.path(),
            run_id,
            vec![
                JournalFrame::Data(completed_record(
                    run_id,
                    0,
                    Some(completion_id),
                    payload.clone(),
                )),
                JournalFrame::Data(completed_record(run_id, 1, Some(completion_id), payload)),
                JournalFrame::Complete {
                    run_id,
                    completion_id,
                    observed_at: Utc::now(),
                },
            ],
        );

        let err = Journal::recover_run(dir.path(), run_id)
            .expect_err("multiple Completed records must be rejected");
        assert!(matches!(err, JournalError::DuplicateTerminal(_)));
    }

    #[test]
    fn recovery_rejects_same_id_completed_records_with_different_payloads() {
        let dir = temp_root();
        let run_id = Uuid::new_v4();
        let completion_id = Uuid::new_v4();
        forge_frames(
            dir.path(),
            run_id,
            vec![
                JournalFrame::Data(completed_record(
                    run_id,
                    0,
                    Some(completion_id),
                    serde_json::json!({"exit_code": 0}),
                )),
                JournalFrame::Data(completed_record(
                    run_id,
                    1,
                    Some(completion_id),
                    serde_json::json!({"exit_code": 1}),
                )),
            ],
        );

        let err = Journal::recover_run(dir.path(), run_id)
            .expect_err("same-id terminal payload conflict must be rejected");
        assert!(matches!(err, JournalError::DuplicateTerminal(_)));
    }

    #[test]
    fn append_rejects_second_completed_record() {
        let dir = temp_root();
        let run_id = Uuid::new_v4();
        let completion_id = Uuid::new_v4();
        let mut journal = Journal::create(dir.path(), run_id).expect("create");
        journal
            .append(completed_record(
                run_id,
                0,
                Some(completion_id),
                serde_json::json!({"exit_code": 0}),
            ))
            .expect("first Completed record");

        let err = journal
            .append(completed_record(
                run_id,
                1,
                Some(completion_id),
                serde_json::json!({"exit_code": 1}),
            ))
            .expect_err("second Completed record must be rejected");
        assert!(matches!(err, JournalError::DuplicateTerminal(_)));
    }

    #[test]
    fn records_after_with_usize_max_returns_all_remaining_records() {
        let run_id = Uuid::new_v4();
        let recovered = RecoveredJournal {
            run_id,
            records: (0..3).map(|seq| new_record(run_id, seq, 1)).collect(),
            completion: None,
            ack: None,
        };
        let checkpoint = JournalCheckpoint {
            run_id,
            last_sequence: Some(0),
            completion_id: None,
        };

        let records = recovered
            .records_after(&checkpoint, usize::MAX)
            .expect("usize::MAX must not overflow");
        assert_eq!(records.len(), 2);
        assert_eq!(records[0].sequence, 1);
        assert_eq!(records[1].sequence, 2);
    }
}
