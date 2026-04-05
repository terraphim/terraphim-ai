//! Quickwit log shipping for ADF orchestrator
//!
//! Feature-gated behind `quickwit` feature flag.
//! Provides async log shipping to Quickwit search engine.

use serde::{Deserialize, Serialize};

use tokio::sync::mpsc;
use tracing::warn;

/// Log document structure for Quickwit ingestion
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LogDocument {
    /// ISO 8601 timestamp
    pub timestamp: String,
    /// Log level (INFO, WARN, ERROR)
    pub level: String,
    /// Name of the agent
    pub agent_name: String,
    /// Agent layer (Safety, Core, Growth)
    pub layer: String,
    /// Source of the log (orchestrator, stdout, stderr)
    pub source: String,
    /// Log message
    pub message: String,
    /// Model used (if applicable)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// Persona used (if applicable)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub persona: Option<String>,
    /// Exit code (if applicable)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<i32>,
    /// Trigger event (if applicable)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trigger: Option<String>,
    /// Wall time in seconds (if applicable)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wall_time_secs: Option<f64>,
    /// Flow name (if applicable)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flow_name: Option<String>,
    /// Extra fields (if applicable)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extra: Option<serde_json::Value>,
}

/// Error type for QuickwitSink operations
#[derive(Debug)]
pub enum QuickwitError {
    /// Failed to send document to the channel
    SendError(String),
    /// HTTP client error
    HttpError(String),
    /// Serialization error
    SerializationError(String),
}

impl std::fmt::Display for QuickwitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            QuickwitError::SendError(msg) => write!(f, "Send error: {}", msg),
            QuickwitError::HttpError(msg) => write!(f, "HTTP error: {}", msg),
            QuickwitError::SerializationError(msg) => write!(f, "Serialization error: {}", msg),
        }
    }
}

impl std::error::Error for QuickwitError {}

impl<T> From<mpsc::error::SendError<T>> for QuickwitError {
    fn from(_: mpsc::error::SendError<T>) -> Self {
        QuickwitError::SendError("Channel closed".to_string())
    }
}

/// Async log shipper for Quickwit
#[derive(Debug, Clone)]
pub struct QuickwitSink {
    tx: mpsc::Sender<LogDocument>,
}

impl QuickwitSink {
    /// Create a new QuickwitSink and spawn the background ingestion task
    ///
    /// # Arguments
    /// * `endpoint` - Quickwit API endpoint (e.g., "http://127.0.0.1:7280")
    /// * `index_id` - Index name (e.g., "adf-logs")
    /// * `batch_size` - Maximum documents per batch
    /// * `flush_interval_secs` - Seconds between forced flushes
    pub fn new(
        endpoint: String,
        index_id: String,
        batch_size: usize,
        flush_interval_secs: u64,
    ) -> Self {
        let (tx, mut rx) = mpsc::channel::<LogDocument>(4096);
        let client = reqwest::Client::new();
        let ingest_url = format!("{}/api/v1/{}/ingest", endpoint, index_id);

        // Spawn background task for batching and sending
        tokio::spawn(async move {
            let mut buffer: Vec<LogDocument> = Vec::with_capacity(batch_size);
            let mut last_flush = tokio::time::Instant::now();
            let flush_interval = tokio::time::Duration::from_secs(flush_interval_secs);

            loop {
                let timeout = tokio::time::sleep_until(last_flush + flush_interval);
                tokio::pin!(timeout);

                tokio::select! {
                    Some(doc) = rx.recv() => {
                        buffer.push(doc);
                        if buffer.len() >= batch_size {
                            Self::flush_batch(&client, &ingest_url, &mut buffer).await;
                            last_flush = tokio::time::Instant::now();
                        }
                    }
                    _ = &mut timeout => {
                        if !buffer.is_empty() {
                            Self::flush_batch(&client, &ingest_url, &mut buffer).await;
                            last_flush = tokio::time::Instant::now();
                        }
                    }
                    else => {
                        // Channel closed, flush remaining and exit
                        if !buffer.is_empty() {
                            Self::flush_batch(&client, &ingest_url, &mut buffer).await;
                        }
                        break;
                    }
                }
            }
        });

        Self { tx }
    }

    /// Send a log document to the sink
    ///
    /// Returns an error if the channel is closed
    pub async fn send(&self, doc: LogDocument) -> Result<(), QuickwitError> {
        self.tx.send(doc).await.map_err(|e| e.into())
    }

    /// Send a log document without awaiting (for sync contexts)
    pub fn try_send(&self, doc: LogDocument) -> Result<(), QuickwitError> {
        self.tx
            .try_send(doc)
            .map_err(|e| QuickwitError::SendError(e.to_string()))
    }

    /// Shutdown the sink, flushing any pending documents
    ///
    /// Note: This drops the sender, causing the background task to flush
    /// remaining documents and exit
    pub fn shutdown(self) {
        // Dropping the sender will cause the background task to exit
        // after flushing any remaining documents
        drop(self.tx);
    }

    /// Flush a batch of documents to Quickwit
    ///
    /// Fire-and-forget: errors are logged but not propagated
    async fn flush_batch(
        client: &reqwest::Client,
        url: &str,
        buffer: &mut Vec<LogDocument>,
    ) {
        if buffer.is_empty() {
            return;
        }

        // Build NDJSON payload
        let mut ndjson = String::new();
        for doc in buffer.drain(..) {
            match serde_json::to_string(&doc) {
                Ok(json) => {
                    ndjson.push_str(&json);
                    ndjson.push('\n');
                }
                Err(e) => {
                    warn!(error = %e, "failed to serialize log document");
                }
            }
        }

        // Send to Quickwit
        match client
            .post(url)
            .header("Content-Type", "application/x-ndjson")
            .body(ndjson)
            .send()
            .await
        {
            Ok(response) => {
                if !response.status().is_success() {
                    let status = response.status();
                    let body = response.text().await.unwrap_or_default();
                    warn!(status = %status, body = %body, "Quickwit ingest failed");
                }
            }
            Err(e) => {
                warn!(error = %e, "failed to send logs to Quickwit");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_log_document_serialization() {
        let doc = LogDocument {
            timestamp: "2024-01-01T00:00:00Z".to_string(),
            level: "INFO".to_string(),
            agent_name: "test-agent".to_string(),
            layer: "Safety".to_string(),
            source: "orchestrator".to_string(),
            message: "test message".to_string(),
            model: Some("gpt-4".to_string()),
            persona: None,
            exit_code: Some(0),
            trigger: None,
            wall_time_secs: Some(1.5),
            flow_name: None,
            extra: None,
        };

        let json = serde_json::to_string(&doc).unwrap();
        assert!(json.contains("test-agent"));
        assert!(json.contains("INFO"));
        assert!(json.contains("gpt-4"));
        // None fields should be skipped
        assert!(!json.contains("persona"));
    }

    #[test]
    fn test_quickwit_error_display() {
        let err = QuickwitError::HttpError("connection refused".to_string());
        assert_eq!(err.to_string(), "HTTP error: connection refused");
    }
}
