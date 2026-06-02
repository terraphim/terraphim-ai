//! `UpdateLog` batching with monotonic ack tracking.

use crate::client::GiteaRunnerClient;
use crate::state::RunnerState;
use crate::types::{LogRow, UpdateLogRequest};
use crate::{Result, RunnerError};

/// Accumulates log rows for a task and streams them via `UpdateLog`, tracking a
/// monotonic row index. Append-only: the index never moves backwards.
pub struct LogStreamer {
    task_id: i64,
    /// Index of the next row to be sent.
    next_index: i64,
    buf: Vec<LogRow>,
}

impl LogStreamer {
    /// Create a streamer for `task_id`.
    pub fn new(task_id: i64) -> Self {
        Self {
            task_id,
            next_index: 0,
            buf: Vec::new(),
        }
    }

    /// Index of the next row to be sent (number of rows committed so far).
    pub fn next_index(&self) -> i64 {
        self.next_index
    }

    /// Buffer a log line (timestamp applied now).
    pub fn add_line(&mut self, content: impl Into<String>) {
        self.buf.push(LogRow {
            time: chrono::Utc::now().to_rfc3339(),
            content: content.into(),
        });
    }

    /// Flush buffered rows. With `no_more = true` the server finalises the log
    /// (sent even if the buffer is empty, to close the stream).
    pub async fn flush<C: GiteaRunnerClient + ?Sized>(
        &mut self,
        client: &C,
        state: &RunnerState,
        no_more: bool,
    ) -> Result<()> {
        if self.buf.is_empty() && !no_more {
            return Ok(());
        }
        let index = self.next_index;
        let rows = std::mem::take(&mut self.buf);
        let count = rows.len() as i64;
        let ack = client
            .update_log(
                state,
                UpdateLogRequest {
                    task_id: self.task_id,
                    index,
                    rows,
                    no_more,
                },
            )
            .await?;
        // Monotonic guard: the server's ack index must not regress below what we
        // have already committed.
        if ack.ack_index + 1 < index {
            return Err(RunnerError::Protocol(format!(
                "UpdateLog ack regressed: ack_index={} < committed index={}",
                ack.ack_index, index
            )));
        }
        self.next_index += count;
        Ok(())
    }
}
