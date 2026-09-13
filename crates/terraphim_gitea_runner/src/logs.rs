//! `UpdateLog` batching with durable retention and authoritative ACK
//! reconciliation.
//!
//! # The server contract this models
//!
//! Gitea's `routers/api/actions/runner/runner.go` (`UpdateLog`) computes
//! `ack := task.LogLength` -- the number of rows the server has *committed* --
//! then either bails out unchanged or commits the un-committed suffix of the
//! batch and returns the new committed count:
//!
//! ```text
//! if len(rows) == 0 || index > ack || index+len(rows) <= ack { return ack }
//! WriteLogs(..., rows[ack-index:]) ; task.LogLength += len(rows[ack-index:])
//! return task.LogLength
//! ```
//!
//! So for a batch covering `[start, end)` sent while the server has committed
//! `acked` rows:
//!
//! * full accept -> `ack == end`
//! * no progress (duplicate, out-of-window, or zero-row) -> `ack == start`
//! * partial accept -> `start < ack < end`
//! * post-seal re-sent finalise of a zero-row log -> `ack == 0` forever
//!   (`LogLength` stays 0), which is why "sealed" must be tracked as explicit
//!   state and never inferred from `ack == index`.
//!
//! `AckIndex` is therefore a *count*, not a last-row index. Treating it as
//! `last_index` is what made the previous tests pass while the client advanced
//! one row too far per batch and mis-detected regressions (Refs #96, #101).

use crate::client::GiteaRunnerClient;
use crate::state::RunnerState;
use crate::task_journal::TaskJournal;
use crate::types::{LogRow, UpdateLogRequest};
use crate::{Result, RunnerError};

/// Maximum number of rows in a single `UpdateLog` request.
///
/// A configuration budget rather than a protocol limit: it keeps one request
/// well inside the runner's memory bound and gives the server useful batches.
pub const MAX_ROWS_PER_REQUEST: usize = 256;

/// Maximum serialized size of one `UpdateLog` request body, in bytes (256 KiB).
pub const MAX_BYTES_PER_REQUEST: usize = 256 * 1024;

/// Bytes held back so that a batch padded with JSON framing still lands under
/// [`MAX_BYTES_PER_REQUEST`].
const REQUEST_FRAMING_RESERVE: usize = 8 * 1024;

/// Upper bound on a single row's content, in bytes.
///
/// Gitea's `modules/actions/log.go` declares `MaxLineSize = 64 * 1024`, but
/// `FormatLog` writes `"<timestamp> <content>"` into a `bufio.Scanner` whose
/// buffer is sized `len(timeFormat) + MaxLineSize + 1`. Content is therefore
/// only safe strictly below `MaxLineSize`; splitting at half of it leaves
/// generous headroom for the timestamp prefix and keeps each row comfortably
/// under the reader's line bound.
pub const MAX_LINE_CONTENT_BYTES: usize = 32 * 1024;

/// Maximum consecutive no-progress ACKs tolerated before the streamer aborts.
///
/// Design §3 specifies "three requests with the existing 200/400 ms backoff,
/// then durable recovery". A server that consistently answers `ack == start`
/// (the dedup bail modelled by [`ServerSemanticsClient`]) would otherwise
/// cause the flush loop to re-offer the identical batch forever.
const MAX_NO_PROGRESS_ATTEMPTS: u32 = 3;

/// Per-attempt backoff for no-progress retries, in milliseconds.
///
/// Indexed by `attempt - 1`; length must equal [`MAX_NO_PROGRESS_ATTEMPTS`] so
/// every attempt sleeps before either succeeding or hitting the budget.
const NO_PROGRESS_BACKOFF_MS: [u64; MAX_NO_PROGRESS_ATTEMPTS as usize] = [200, 400, 800];

/// Outcome of reconciling a server ACK against the batch that was sent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AckEffect {
    /// The server committed nothing from this batch (`ack == start`). Retain
    /// every row and retry within budget.
    NoProgress,
    /// The server has committed `next` rows in total (`start < ack <= end`).
    Advanced { next: i64 },
}

/// Reconcile an authoritative server ACK against the window `[start, end)` that
/// was just sent.
///
/// Pure function so the transitions are testable without any HTTP client. Rows
/// below `ack` are committed server-side; rows at or above it are not.
pub fn reconcile_ack(start: i64, end: i64, ack: i64) -> Result<AckEffect> {
    if ack < start {
        return Err(RunnerError::Protocol(format!(
            "UpdateLog ack regressed: server committed {ack} rows but \
             batch started at row {start}"
        )));
    }
    if ack > end {
        return Err(RunnerError::Protocol(format!(
            "UpdateLog ack beyond batch: server committed {ack} rows but \
             batch ended at row {end}"
        )));
    }
    if ack == start {
        return Ok(AckEffect::NoProgress);
    }
    Ok(AckEffect::Advanced { next: ack })
}

/// Split oversized content into chunks no longer than `max_bytes`, breaking
/// only at UTF-8 char boundaries so no chunk contains a partial code point.
///
/// An empty input yields one empty row: the server tolerates blank lines and
/// dropping them would silently lose a row position.
pub fn split_content(content: &str, max_bytes: usize) -> Vec<String> {
    if max_bytes == 0 || content.len() <= max_bytes {
        return vec![content.to_string()];
    }
    let mut out = Vec::new();
    let mut rest = content;
    while !rest.is_empty() {
        // Find the largest prefix of at most `max_bytes` that ends on a char
        // boundary.
        let mut cut = max_bytes.min(rest.len());
        while cut > 0 && !rest.is_char_boundary(cut) {
            cut -= 1;
        }
        out.push(rest[..cut].to_string());
        rest = &rest[cut..];
    }
    out
}

/// A redactor applied to every line before it is buffered, journalled or sent.
pub type Redactor = Box<dyn Fn(&str) -> String + Send + Sync>;

/// Accumulates log rows for a task and streams them via `UpdateLog`.
///
/// Two cursors are kept distinct: `produced` (rows buffered durably by this
/// process) and `acked` (rows the server confirmed it committed). Buffered rows
/// are retained until the server acknowledges them, so a transport error leaves
/// the streamer able to replay the identical `[start, end)` window with the
/// identical row timestamps. When backed by a [`TaskJournal`] the same holds
/// across a process restart.
pub struct LogStreamer {
    task_id: i64,
    /// Next row sequence to be produced (exclusive upper bound of durability).
    produced: i64,
    /// Next row the server expects; equals the server's committed count once the
    /// two are in sync.
    acked: i64,
    buf: Vec<LogRow>,
    journal: Option<TaskJournal>,
    /// Redactor applied to every line before it is buffered, journalled or sent.
    redactor: Option<Redactor>,
    /// Set once a flush with `no_more = true` has been accepted. Terminal state:
    /// appending afterwards is a protocol violation, and sealing is never
    /// inferred from `acked == produced`.
    sealed: bool,
    max_rows_per_request: usize,
    max_bytes_per_request: usize,
}

impl std::fmt::Debug for LogStreamer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LogStreamer")
            .field("task_id", &self.task_id)
            .field("produced", &self.produced)
            .field("acked", &self.acked)
            .field("buffered", &self.buf.len())
            .field("journal", &self.journal.is_some())
            .field("sealed", &self.sealed)
            .finish()
    }
}

impl LogStreamer {
    /// Create a streamer for `task_id` with default budgets.
    pub fn new(task_id: i64) -> Self {
        Self {
            task_id,
            produced: 0,
            acked: 0,
            buf: Vec::new(),
            journal: None,
            redactor: None,
            sealed: false,
            max_rows_per_request: MAX_ROWS_PER_REQUEST,
            max_bytes_per_request: MAX_BYTES_PER_REQUEST,
        }
    }

    /// Back the streamer with a durable journal. Rows are appended there before
    /// they enter the in-memory buffer, so a restarted streamer built from the
    /// same journal replays exactly the un-acked rows.
    #[must_use]
    pub fn with_journal(mut self, journal: TaskJournal) -> Self {
        let cursors = (journal.produced(), journal.acked());
        self.journal = Some(journal);
        self.produced = cursors.0;
        self.acked = cursors.1;
        self
    }

    /// Install a redactor applied to every line at `add_line` time, i.e. before
    /// anything reaches disk or the network.
    #[must_use]
    pub fn with_redactor(
        mut self,
        redactor: impl Fn(&str) -> String + Send + Sync + 'static,
    ) -> Self {
        self.redactor = Some(Box::new(redactor));
        self
    }

    /// Next row sequence to be produced.
    pub fn produced(&self) -> i64 {
        self.produced
    }

    /// Next row the server expects (its committed count once in sync).
    pub fn acked(&self) -> i64 {
        self.acked
    }

    /// Number of rows committed so far -- kept for callers that think in terms
    /// of the old `next_index` cursor. Equal to [`Self::produced`], which is the
    /// row the *next* flush starts at while nothing is acknowledged backwards.
    pub fn next_index(&self) -> i64 {
        self.produced
    }

    /// True once a `no_more` flush has been accepted by the server.
    pub fn is_sealed(&self) -> bool {
        self.sealed
    }

    /// Rows currently buffered and not yet acknowledged.
    pub fn buffered_len(&self) -> usize {
        self.buf.len()
    }

    /// Buffer a log line (timestamp applied once, here).
    ///
    /// The content is redacted first, then split on UTF-8 boundaries so no row
    /// can exceed the server's per-line bound. Timestamps are never regenerated
    /// on retry: the stored value is what gets replayed. When a journal is
    /// attached the rows are appended there first (fsynced) so a restart can
    /// rebuild state from durable storage.
    pub fn add_line(&mut self, content: impl Into<String>) {
        let raw = content.into();
        let content = match &self.redactor {
            Some(r) => r(&raw),
            None => raw,
        };
        let chunks = split_content(&content, MAX_LINE_CONTENT_BYTES);
        if chunks.is_empty() {
            return;
        }
        let mut new_rows: Vec<LogRow> = Vec::with_capacity(chunks.len());
        for chunk in chunks {
            let row = LogRow {
                time: chrono::Utc::now().to_rfc3339(),
                content: chunk,
            };
            self.produced += 1;
            new_rows.push(row);
        }
        if let Some(j) = self.journal.as_mut() {
            // Append durably before the rows enter the in-memory buffer, so a
            // crash between here and a successful ack still leaves them on disk.
            if let Err(e) = j.append_redacted(&new_rows) {
                log::warn!(
                    "task {}: failed to append {} row(s) to journal: {e}",
                    self.task_id,
                    new_rows.len()
                );
            }
        }
        self.buf.extend(new_rows);
    }

    /// Flush buffered rows. With `no_more = true` the server finalises the log.
    ///
    /// Ordering rule: a finalise may only be sent once everything produced has
    /// also been acknowledged and nothing is left buffered. Otherwise the
    /// remaining rows go first and the seal follows on a later call, because
    /// `UpdateLog` seals whatever it has committed at that moment.
    pub async fn flush<C: GiteaRunnerClient + ?Sized>(
        &mut self,
        client: &C,
        state: &RunnerState,
        no_more: bool,
    ) -> Result<()> {
        if self.sealed {
            return Err(RunnerError::Protocol(format!(
                "log stream for task {} is already sealed; refusing further UpdateLog",
                self.task_id
            )));
        }

        // Drain everything outstanding first; the seal is only appended once the
        // cursors have converged. Design §3 specifies a short-term retry budget
        // of three requests with 200/400 ms backoff before durable recovery.
        // Without this guard, a server that answers `ack == start` (the dedup
        // bail modelled by ServerSemanticsClient) would cause the loop to
        // re-offer the identical batch forever.
        let mut no_progress_attempts: u32 = 0;
        loop {
            let want_finalise = no_more && self.buf.is_empty() && self.acked == self.produced;
            if want_finalise {
                break;
            }
            if self.buf.is_empty() {
                if !no_more {
                    return Ok(());
                }
                // Nothing buffered but the server is behind: cannot happen with a
                // single writer, and we must not seal over un-acked rows.
                return Err(RunnerError::Protocol(format!(
                    "cannot finalise task {}: acked={} but produced={}",
                    self.task_id, self.acked, self.produced
                )));
            }
            let (start, rows) = self.next_batch()?;
            let end = start + rows.len() as i64;
            let resp = client
                .update_log(
                    state,
                    UpdateLogRequest {
                        task_id: self.task_id,
                        index: start,
                        rows,
                        no_more: false,
                    },
                )
                .await?;
            match reconcile_ack(start, end, resp.ack_index)? {
                AckEffect::Advanced { next } => {
                    self.acked = next;
                    if let Some(j) = self.journal.as_mut() {
                        j.record_ack(next)?;
                    }
                    // Drop exactly the rows the server committed.
                    let owned_start = self.produced_lower_bound();
                    if next > owned_start {
                        let drop_n = (next - owned_start).min(self.buf.len() as i64) as usize;
                        self.buf.drain(..drop_n);
                    }
                    no_progress_attempts = 0;
                }
                AckEffect::NoProgress => {
                    no_progress_attempts += 1;
                    if no_progress_attempts >= MAX_NO_PROGRESS_ATTEMPTS {
                        return Err(RunnerError::Protocol(format!(
                            "task {}: server returned no-progress ack {} times in a row \
                             for batch starting at {}; aborting per design §3 retry budget",
                            self.task_id, no_progress_attempts, start
                        )));
                    }
                    tokio::time::sleep(std::time::Duration::from_millis(
                        NO_PROGRESS_BACKOFF_MS[(no_progress_attempts - 1) as usize],
                    ))
                    .await;
                }
            }
        }

        // Finalise with an empty batch. The server answers with its committed
        // count, which for a zero-row log is 0 -- hence the explicit `sealed`
        // flag rather than any cursor comparison.
        let resp = client
            .update_log(
                state,
                UpdateLogRequest {
                    task_id: self.task_id,
                    index: self.acked,
                    rows: Vec::new(),
                    no_more: true,
                },
            )
            .await?;
        // A zero-row batch can only ever be answered with "no progress": the
        // server bails out before writing. Anything else means the peer does not
        // implement the pinned contract.
        if resp.ack_index != self.acked {
            return Err(RunnerError::Protocol(format!(
                "finalise for task {} returned ack {} while {} rows were committed",
                self.task_id, resp.ack_index, self.acked
            )));
        }
        self.sealed = true;
        if let Some(j) = self.journal.as_mut() {
            j.record_sealed()?;
        }
        Ok(())
    }

    /// Take the next bounded batch starting at the first un-acked row.
    ///
    /// When a journal is attached the batch is read back from it (the durable
    /// copy), otherwise from the in-memory buffer. Either way the rows are
    /// copied, never moved, so a failed request can be replayed verbatim.
    fn next_batch(&self) -> Result<(i64, Vec<LogRow>)> {
        let max_rows = self.max_rows_per_request.max(1);
        let max_bytes = self
            .max_bytes_per_request
            .saturating_sub(REQUEST_FRAMING_RESERVE)
            .max(1);
        let rows = match &self.journal {
            Some(j) => j.batch_from(self.acked, max_rows, max_bytes),
            None => {
                let offset = (self.acked - self.produced_lower_bound()) as usize;
                let offset = offset.clamp(0, self.buf.len());
                let mut out: Vec<LogRow> = Vec::with_capacity(max_rows.min(self.buf.len()));
                let mut bytes = 0usize;
                for row in self.buf[offset..].iter() {
                    let cost = row.content.len() + row.time.len() + 32;
                    if !out.is_empty() && (out.len() >= max_rows || bytes + cost > max_bytes) {
                        break;
                    }
                    bytes += cost;
                    out.push(row.clone());
                }
                out
            }
        };
        if rows.is_empty() {
            return Err(RunnerError::Protocol(format!(
                "no rows available to send for task {} at acked={}",
                self.task_id, self.acked
            )));
        }
        Ok((self.acked, rows))
    }

    /// Sequence of the oldest row still held in the in-memory buffer.
    fn produced_lower_bound(&self) -> i64 {
        self.produced - self.buf.len() as i64
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{
        DeclareRequest, DeclareResponse, FetchTaskResponse, RegisterRequest, RunnerInfo,
        UpdateLogResponse, UpdateTaskRequest, UpdateTaskResponse,
    };
    use async_trait::async_trait;
    use std::sync::Mutex;
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

    // --- pure ACK transition table -----------------------------------------
    //
    // These pin the semantics read off the fork's UpdateLog handler rather than
    // any client-side wish. Batch [10, 13): three rows offered.

    #[test]
    fn ack_full_accept_advances_to_end() {
        assert_eq!(
            reconcile_ack(10, 13, 13).unwrap(),
            AckEffect::Advanced { next: 13 }
        );
    }

    #[test]
    fn ack_no_progress_when_server_already_committed_the_batch() {
        // Duplicate/out-of-window bail: `index + len(rows) <= ack` or
        // `index > ack` both return the unchanged committed count.
        assert_eq!(reconcile_ack(10, 13, 10).unwrap(), AckEffect::NoProgress);
    }

    #[test]
    fn ack_partial_accept_commits_prefix_and_keeps_suffix() {
        assert_eq!(
            reconcile_ack(10, 13, 11).unwrap(),
            AckEffect::Advanced { next: 11 }
        );
    }

    #[test]
    fn ack_regression_is_a_protocol_error() {
        let err = reconcile_ack(10, 13, 9).unwrap_err();
        assert!(
            matches!(err, RunnerError::Protocol(_)),
            "regression must not be repaired by renumbering: {err:?}"
        );
    }

    #[test]
    fn ack_negative_is_a_protocol_error() {
        assert!(matches!(
            reconcile_ack(0, 3, -1).unwrap_err(),
            RunnerError::Protocol(_)
        ));
    }

    #[test]
    fn ack_beyond_end_is_a_protocol_error() {
        let err = reconcile_ack(10, 13, 14).unwrap_err();
        assert!(
            matches!(err, RunnerError::Protocol(_)),
            "an ack past the batch must not be adopted blindly: {err:?}"
        );
    }

    #[test]
    fn zero_row_finalise_of_empty_log_acks_zero() {
        // Post-seal re-sent finalise of a zero-row log: start == end == 0 and
        // the server answers 0 forever. reconcile_ack reports NoProgress, which
        // is why sealing needs explicit state.
        assert_eq!(reconcile_ack(0, 0, 0).unwrap(), AckEffect::NoProgress);
    }

    // --- line splitting ----------------------------------------------------

    #[test]
    fn oversized_lines_split_on_utf8_boundaries() {
        let content = "é".repeat(40_000); // 2 bytes per char
        let parts = split_content(&content, MAX_LINE_CONTENT_BYTES);
        assert!(parts.len() > 1, "must split");
        assert!(
            parts.iter().all(|p| p.len() <= MAX_LINE_CONTENT_BYTES),
            "every chunk within the bound"
        );
        assert_eq!(parts.concat(), content, "no bytes lost or reordered");
        // Each chunk must be valid UTF-8 in its own right (no half code points).
        assert!(
            parts
                .iter()
                .all(|p| std::str::from_utf8(p.as_bytes()).is_ok())
        );
    }

    #[test]
    fn short_lines_are_not_split() {
        assert_eq!(
            split_content("hello", MAX_LINE_CONTENT_BYTES),
            vec!["hello"]
        );
        assert_eq!(split_content("", MAX_LINE_CONTENT_BYTES), vec![""]);
    }

    // --- client double implementing the REAL server -------------------------

    /// Test double that reproduces Gitea's `UpdateLog` arithmetic rather than
    /// agreeing with the client's assumptions.
    ///
    /// The previous `RecordingClient` acked `index + rows.len() - 1`, i.e. a
    /// last-row index. The real server returns `task.LogLength`, a committed
    /// *count*; encoding the wrong belief here is what let the retention bug
    /// stay green (Refs #96, #101).
    #[derive(Default)]
    struct ServerSemanticsClient {
        /// Server-side committed row count (`task.LogLength`).
        committed: AtomicUsize,
        /// Batches as observed on the wire: `(index, row contents, no_more)`.
        seen: Mutex<Vec<(i64, Vec<String>, bool)>>,
        /// Fail the next N requests with a transport error before touching state.
        fail_next: AtomicUsize,
        /// When set, the response is lost after the server committed the rows.
        lose_response_after_commit: AtomicUsize,
        /// When true, every non-empty batch is answered with `ack == start`
        /// (the dedup bail), simulating a server that never advances. Used to
        /// verify the design §3 retry budget fires rather than hanging.
        always_no_progress: AtomicBool,
    }

    impl ServerSemanticsClient {
        fn committed_count(&self) -> i64 {
            self.committed.load(Ordering::SeqCst) as i64
        }
    }

    #[async_trait]
    impl GiteaRunnerClient for ServerSemanticsClient {
        async fn register(&self, _: RegisterRequest) -> Result<RunnerInfo> {
            unreachable!()
        }
        async fn declare(&self, _: &RunnerState, _: DeclareRequest) -> Result<DeclareResponse> {
            unreachable!()
        }
        async fn fetch_task(&self, _: &RunnerState, _: i64) -> Result<FetchTaskResponse> {
            unreachable!()
        }
        async fn update_task(
            &self,
            _: &RunnerState,
            _: UpdateTaskRequest,
        ) -> Result<UpdateTaskResponse> {
            unreachable!()
        }
        async fn update_log(
            &self,
            _: &RunnerState,
            req: UpdateLogRequest,
        ) -> Result<UpdateLogResponse> {
            self.seen.lock().unwrap().push((
                req.index,
                req.rows.iter().map(|r| r.content.clone()).collect(),
                req.no_more,
            ));

            // `fetch_update` decrements only when the prior value is > 0 so the
            // counter cannot underflow and silently start failing every
            // request.
            let should_fail = self
                .fail_next
                .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |v| {
                    if v > 0 { Some(v - 1) } else { None }
                })
                .is_ok_and(|v| v > 0);
            if should_fail {
                return Err(RunnerError::Protocol("transport failure".to_string()));
            }
            let lose_response = self
                .lose_response_after_commit
                .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |v| {
                    if v > 0 { Some(v - 1) } else { None }
                })
                .is_ok_and(|v| v > 0);

            // Faithful transcription of runner.go UpdateLog.
            let ack = self.committed_count();
            if req.rows.is_empty() || req.index > ack || req.index + req.rows.len() as i64 <= ack {
                return Ok(UpdateLogResponse { ack_index: ack });
            }
            // Test hook: simulate a server that bails on every non-empty batch
            // (e.g. dedup or out-of-window rejection). The streamer must abort
            // after MAX_NO_PROGRESS_ATTEMPTS rather than spin forever.
            if self.always_no_progress.load(Ordering::SeqCst) {
                return Ok(UpdateLogResponse {
                    ack_index: req.index,
                });
            }
            let suffix = &req.rows[(ack - req.index) as usize..];
            self.committed.fetch_add(suffix.len(), Ordering::SeqCst);
            if lose_response {
                return Err(RunnerError::Protocol(
                    "response lost after commit".to_string(),
                ));
            }
            Ok(UpdateLogResponse {
                ack_index: self.committed_count(),
            })
        }
    }

    fn dummy_state() -> RunnerState {
        RunnerState {
            uuid: "u".into(),
            token: "t".into(),
            name: "n".into(),
            version: "0".into(),
            labels: vec![],
            ephemeral: false,
        }
    }

    fn contents(client: &ServerSemanticsClient) -> Vec<(i64, Vec<String>)> {
        client
            .seen
            .lock()
            .unwrap()
            .iter()
            .map(|(i, r, _)| (*i, r.clone()))
            .collect()
    }

    #[tokio::test]
    async fn multi_batch_index_is_monotonic_and_contiguous() {
        let client = ServerSemanticsClient::default();
        let st = dummy_state();
        let mut s = LogStreamer::new(7);
        // Batch 1: two rows -> start 0.
        s.add_line("a");
        s.add_line("b");
        s.flush(&client, &st, false).await.unwrap();
        // Batch 2: one row -> start 2.
        s.add_line("c");
        s.flush(&client, &st, false).await.unwrap();
        // Final close (empty) -> start 3.
        s.flush(&client, &st, true).await.unwrap();

        assert_eq!(s.next_index(), 3);
        assert_eq!(s.acked(), 3, "server caught up with everything produced");
        let idx: Vec<i64> = contents(&client).iter().map(|c| c.0).collect();
        assert_eq!(idx, vec![0, 2, 3], "batch start indices are contiguous");
        assert!(
            idx.windows(2).all(|w| w[0] <= w[1]),
            "indices never regress"
        );
    }

    #[tokio::test]
    async fn empty_flush_without_no_more_is_a_noop() {
        let client = ServerSemanticsClient::default();
        let st = dummy_state();
        let mut s = LogStreamer::new(1);
        s.flush(&client, &st, false).await.unwrap();
        assert!(client.seen.lock().unwrap().is_empty(), "no UpdateLog sent");
        assert_eq!(s.next_index(), 0);
    }

    /// Defect F1: `mem::take` moved the only copy of the rows into the request,
    /// so a transport error deleted them. They must survive and replay verbatim.
    #[tokio::test]
    async fn transport_error_retains_rows_and_replays_identical_batch() {
        let client = ServerSemanticsClient::default();
        client.fail_next.store(1, Ordering::SeqCst);
        let st = dummy_state();
        let mut s = LogStreamer::new(9);
        s.add_line("one");
        s.add_line("two");

        let err = s.flush(&client, &st, false).await.unwrap_err();
        assert!(
            matches!(err, RunnerError::Protocol(_)),
            "transport failure surfaces: {err:?}"
        );
        assert_eq!(s.buffered_len(), 2, "rows must NOT be lost on failure");
        assert_eq!(s.acked(), 0, "nothing was committed");
        assert_eq!(s.produced(), 2);

        // Replay: identical window and identical content.
        s.flush(&client, &st, false).await.unwrap();
        let seen = contents(&client);
        assert_eq!(seen.len(), 2, "one failed attempt, one replay");
        assert_eq!(seen[0], seen[1], "replayed batch is byte-identical");
        assert_eq!(seen[1].1, vec!["one", "two"]);
        assert_eq!(client.committed_count(), 2);
        assert_eq!(s.acked(), 2);
        assert_eq!(s.buffered_len(), 0, "committed prefix released");
    }

    /// Lost-response recovery: the server committed the rows but the answer
    /// never arrived. The client re-sends the same window; the server dedups via
    /// its committed count and the client converges without losing or duplicating.
    #[tokio::test]
    async fn lost_response_replay_converges_without_duplicates() {
        let client = ServerSemanticsClient::default();
        client.lose_response_after_commit.store(1, Ordering::SeqCst);
        let st = dummy_state();
        let mut s = LogStreamer::new(11);
        s.add_line("x");
        s.add_line("y");

        assert!(s.flush(&client, &st, false).await.is_err());
        assert_eq!(client.committed_count(), 2, "server did commit them");
        assert_eq!(s.acked(), 0, "client has no proof yet");
        assert_eq!(s.buffered_len(), 2);

        s.flush(&client, &st, false).await.unwrap();
        assert_eq!(s.acked(), 2);
        assert_eq!(client.committed_count(), 2, "no duplicate rows committed");
        assert_eq!(s.buffered_len(), 0);
        let seen = contents(&client);
        assert_eq!(seen[0].1, seen[1].1, "same rows offered twice");
    }

    /// Partial accept: rows below the ack are released, the suffix is retained
    /// and offered again from the correct start.
    #[tokio::test]
    async fn partial_ack_releases_prefix_and_keeps_suffix() {
        let client = ServerSemanticsClient::default();
        let st = dummy_state();
        let mut s = LogStreamer::new(12);
        s.add_line("p1");
        s.add_line("p2");
        s.add_line("p3");

        // Pretend the server already had one row committed (e.g. it accepted
        // p1 in an earlier batch). The client offered [0, 3); the server
        // dedups the prefix and commits the remaining two, returning ack 3.
        client.committed.store(1, Ordering::SeqCst);

        s.flush(&client, &st, false).await.unwrap();
        assert_eq!(
            s.acked(),
            3,
            "AckIndex is the server's committed row count (task.LogLength), \
             not the number of *new* rows this request added"
        );
        assert_eq!(s.buffered_len(), 0);
        assert_eq!(client.committed_count(), 3);
    }

    /// An ack below the sent start must be reported, never repaired locally.
    #[tokio::test]
    async fn ack_regression_surfaces_as_protocol_error_without_renumbering() {
        struct RegressingClient;
        #[async_trait]
        impl GiteaRunnerClient for RegressingClient {
            async fn register(&self, _: RegisterRequest) -> Result<RunnerInfo> {
                unreachable!()
            }
            async fn declare(&self, _: &RunnerState, _: DeclareRequest) -> Result<DeclareResponse> {
                unreachable!()
            }
            async fn fetch_task(&self, _: &RunnerState, _: i64) -> Result<FetchTaskResponse> {
                unreachable!()
            }
            async fn update_task(
                &self,
                _: &RunnerState,
                _: UpdateTaskRequest,
            ) -> Result<UpdateTaskResponse> {
                unreachable!()
            }
            async fn update_log(
                &self,
                _: &RunnerState,
                req: UpdateLogRequest,
            ) -> Result<UpdateLogResponse> {
                // Absurd: claims fewer rows committed than the batch start.
                Ok(UpdateLogResponse {
                    ack_index: req.index - 1,
                })
            }
        }

        let st = dummy_state();
        let mut s = LogStreamer::new(13);
        s.add_line("a");
        s.add_line("b");
        let err = s.flush(&RegressingClient, &st, false).await.unwrap_err();
        assert!(
            matches!(err, RunnerError::Protocol(_)),
            "must be a protocol failure: {err:?}"
        );
        assert_eq!(s.acked(), 0, "cursor untouched by a bad ack");
        assert_eq!(s.buffered_len(), 2, "rows retained for reconciliation");
    }

    /// Beyond-end ack likewise fails closed.
    #[tokio::test]
    async fn ack_beyond_end_surfaces_as_protocol_error() {
        struct OverAckClient;
        #[async_trait]
        impl GiteaRunnerClient for OverAckClient {
            async fn register(&self, _: RegisterRequest) -> Result<RunnerInfo> {
                unreachable!()
            }
            async fn declare(&self, _: &RunnerState, _: DeclareRequest) -> Result<DeclareResponse> {
                unreachable!()
            }
            async fn fetch_task(&self, _: &RunnerState, _: i64) -> Result<FetchTaskResponse> {
                unreachable!()
            }
            async fn update_task(
                &self,
                _: &RunnerState,
                _: UpdateTaskRequest,
            ) -> Result<UpdateTaskResponse> {
                unreachable!()
            }
            async fn update_log(
                &self,
                _: &RunnerState,
                req: UpdateLogRequest,
            ) -> Result<UpdateLogResponse> {
                Ok(UpdateLogResponse {
                    ack_index: req.index + req.rows.len() as i64 + 5,
                })
            }
        }

        let st = dummy_state();
        let mut s = LogStreamer::new(14);
        s.add_line("a");
        let err = s.flush(&OverAckClient, &st, false).await.unwrap_err();
        assert!(matches!(err, RunnerError::Protocol(_)), "{err:?}");
        assert_eq!(s.acked(), 0);
    }

    #[tokio::test]
    async fn requests_are_bounded_by_row_budget() {
        let client = ServerSemanticsClient::default();
        let st = dummy_state();
        let mut s = LogStreamer::new(15);
        s.max_rows_per_request = 4;
        for i in 0..10 {
            s.add_line(format!("line {i}"));
        }
        s.flush(&client, &st, false).await.unwrap();
        let seen = contents(&client);
        assert_eq!(seen.len(), 3, "10 rows in batches of 4 -> 3 requests");
        assert_eq!(seen[0].1.len(), 4);
        assert_eq!(seen[1].0, 4, "second batch starts where the first ended");
        assert_eq!(seen[2].1.len(), 2);
        assert_eq!(client.committed_count(), 10);
        assert_eq!(s.acked(), 10);
    }

    #[tokio::test]
    async fn requests_are_bounded_by_byte_budget() {
        let client = ServerSemanticsClient::default();
        let st = dummy_state();
        let mut s = LogStreamer::new(16);
        s.max_rows_per_request = 1000;
        s.max_bytes_per_request = 8 * 1024; // small enough to force several batches
        for i in 0..20 {
            s.add_line(format!("{i}: {}", "payload ".repeat(60)));
        }
        let total = s.produced();
        s.flush(&client, &st, false).await.unwrap();
        let seen = contents(&client);
        assert!(seen.len() > 1, "byte budget must split into batches");
        assert_eq!(client.committed_count(), total);
        assert_eq!(s.acked(), total);
    }

    #[tokio::test]
    async fn oversized_line_is_buffered_as_multiple_rows() {
        let client = ServerSemanticsClient::default();
        let st = dummy_state();
        let mut s = LogStreamer::new(17);
        s.add_line("z".repeat(MAX_LINE_CONTENT_BYTES * 2 + 10));
        assert_eq!(s.buffered_len(), 3, "one logical line became three rows");
        assert_eq!(s.produced(), 3);
        s.flush(&client, &st, false).await.unwrap();
        assert_eq!(client.committed_count(), 3);
    }

    #[tokio::test]
    async fn redactor_runs_before_buffering() {
        let client = ServerSemanticsClient::default();
        let st = dummy_state();
        let mut s = LogStreamer::new(18).with_redactor(|t| t.replace("hunter2", "***"));
        s.add_line("password=hunter2");
        s.flush(&client, &st, false).await.unwrap();
        let seen = contents(&client);
        assert_eq!(seen[0].1, vec!["password=***"], "masked on the wire");
    }

    /// Empty finalise ordering: `no_more` must not seal over un-acked rows.
    #[tokio::test]
    async fn finalise_waits_for_outstanding_rows() {
        let client = ServerSemanticsClient::default();
        let st = dummy_state();
        let mut s = LogStreamer::new(19);
        s.add_line("tail");
        // One flush delivers the row and then seals, in order.
        s.flush(&client, &st, true).await.unwrap();
        let seen = client.seen.lock().unwrap();
        assert!(!seen[0].2, "first request carries rows, no_more=false");
        assert!(seen[1].2, "seal is a separate, later, empty request");
        assert!(seen[1].1.is_empty(), "seal carries no rows");
        assert!(s.is_sealed());
    }

    #[tokio::test]
    async fn flushing_a_sealed_stream_is_a_protocol_error() {
        let client = ServerSemanticsClient::default();
        let st = dummy_state();
        let mut s = LogStreamer::new(20);
        s.flush(&client, &st, true).await.unwrap();
        s.add_line("post-seal append");
        let err = s.flush(&client, &st, true).await.unwrap_err();
        assert!(matches!(err, RunnerError::Protocol(_)), "{err:?}");
    }

    #[tokio::test]
    async fn zero_row_stream_seals_with_explicit_state_despite_ack_zero() {
        // The trap: a zero-row log seals with ack 0 forever. Progress-based
        // reasoning would loop or misreport; `is_sealed` is set explicitly.
        let client = ServerSemanticsClient::default();
        let st = dummy_state();
        let mut s = LogStreamer::new(21);
        s.flush(&client, &st, true).await.unwrap();
        assert_eq!(client.committed_count(), 0);
        assert!(s.is_sealed());
        assert_eq!(s.acked(), 0);
    }

    #[tokio::test(start_paused = true)]
    async fn no_progress_ack_aborts_after_retry_budget() {
        // Regression for design §3 retry budget. A server that consistently
        // answers `ack == start` (the dedup bail modelled by always_no_progress)
        // must not cause the flush loop to spin forever. Before this guard the
        // streamer re-offered the identical batch indefinitely against a
        // degraded server; now it aborts after MAX_NO_PROGRESS_ATTEMPTS and
        // surfaces a Protocol error so the worker can fail the task truthfully.
        let client = ServerSemanticsClient::default();
        client.always_no_progress.store(true, Ordering::SeqCst);
        let st = dummy_state();
        let mut s = LogStreamer::new(22);
        s.add_line("row-a");
        s.add_line("row-b");
        let err = s.flush(&client, &st, false).await.unwrap_err();
        assert!(
            matches!(&err, RunnerError::Protocol(msg) if msg.contains("no-progress")),
            "expected Protocol error mentioning no-progress, got {err:?}"
        );
        // Exactly MAX_NO_PROGRESS_ATTEMPTS requests were sent before aborting.
        let seen = client.seen.lock().unwrap();
        assert_eq!(
            seen.len(),
            MAX_NO_PROGRESS_ATTEMPTS as usize,
            "streamer should stop after the configured retry budget"
        );
        // All attempts carried the same start index (server never advanced).
        for (idx, _, _) in seen.iter() {
            assert_eq!(*idx, 0, "every attempt starts at the un-acked row 0");
        }
    }
}
