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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{
        DeclareRequest, DeclareResponse, FetchTaskResponse, RegisterRequest, RunnerInfo,
        UpdateTaskRequest, UpdateTaskResponse,
    };
    use async_trait::async_trait;
    use std::sync::Mutex;

    /// Fake protocol client (external boundary, not an internal mock) that records
    /// each `UpdateLog` batch's starting index and acks the last row.
    #[derive(Default)]
    struct RecordingClient {
        indices: Mutex<Vec<i64>>,
    }

    #[async_trait]
    impl GiteaRunnerClient for RecordingClient {
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
            req: crate::types::UpdateLogRequest,
        ) -> Result<crate::types::UpdateLogResponse> {
            self.indices.lock().unwrap().push(req.index);
            let ack = req.index + req.rows.len() as i64 - 1;
            Ok(crate::types::UpdateLogResponse { ack_index: ack })
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

    #[tokio::test]
    async fn multi_batch_index_is_monotonic_and_contiguous() {
        let client = RecordingClient::default();
        let st = dummy_state();
        let mut s = LogStreamer::new(7);
        // Batch 1: two rows -> index 0.
        s.add_line("a");
        s.add_line("b");
        s.flush(&client, &st, false).await.unwrap();
        // Batch 2: one row -> index 2.
        s.add_line("c");
        s.flush(&client, &st, false).await.unwrap();
        // Final close (empty) -> index 3.
        s.flush(&client, &st, true).await.unwrap();

        assert_eq!(s.next_index(), 3);
        let idx = client.indices.lock().unwrap().clone();
        assert_eq!(idx, vec![0, 2, 3], "batch start indices are contiguous");
        assert!(
            idx.windows(2).all(|w| w[0] <= w[1]),
            "indices never regress"
        );
    }

    #[tokio::test]
    async fn empty_flush_without_no_more_is_a_noop() {
        let client = RecordingClient::default();
        let st = dummy_state();
        let mut s = LogStreamer::new(1);
        s.flush(&client, &st, false).await.unwrap();
        assert!(
            client.indices.lock().unwrap().is_empty(),
            "no UpdateLog sent"
        );
        assert_eq!(s.next_index(), 0);
    }
}
