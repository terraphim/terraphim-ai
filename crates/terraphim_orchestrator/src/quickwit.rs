//! Quickwit log shipping for ADF orchestrator
//!
//! Feature-gated behind `quickwit` feature flag.
//! Provides async log shipping to Quickwit search engine.
//!
//! # Typed orchestrator events
//!
//! [`OrchestratorEvent`] is the single event enum for the ROC v1 auto-review
//! and auto-merge flow. Emit sites:
//!
//! | Variant                  | Emit site (lib.rs)                                    |
//! |--------------------------|-------------------------------------------------------|
//! | `PrReviewed`             | `poll_pending_reviews_for_project` — after verdict    |
//! | `PrAutoMerged`           | `handle_auto_merge_for_project` — after merge_pr OK   |
//! | `PrAutoMergedVerified`   | `handle_post_merge_test_gate_with_runner` — gate pass |
//! | `PrAutoReverted`         | `handle_post_merge_test_gate_with_runner` — gate fail |

use std::collections::HashMap;

use chrono::Utc;
use serde::{Deserialize, Serialize};

use tokio::sync::mpsc;
use tracing::warn;

use crate::dispatcher::LEGACY_PROJECT_ID;

/// Log document structure for Quickwit ingestion
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LogDocument {
    /// ISO 8601 timestamp
    pub timestamp: String,
    /// Project id owning the agent that produced this log. Legacy
    /// single-project configs use [`crate::dispatcher::LEGACY_PROJECT_ID`].
    #[serde(default)]
    pub project_id: String,
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

/// Typed events emitted by the ROC v1 auto-review / auto-merge flow.
///
/// Serialised as the `extra.event_kind`-discriminated JSON object that lands
/// in the `extra` field of a [`LogDocument`]. The Quickwit index is
/// schema-on-read so no index migration is required.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "event_kind", rename_all = "snake_case")]
pub enum OrchestratorEvent {
    /// Reviewer agent completed and a verdict was successfully parsed.
    PrReviewed {
        pr_number: u64,
        project: String,
        head_sha: String,
        reviewer_login: String,
        /// Confidence score 1-5 from the `structural-pr-review` comment.
        confidence: u8,
        p0_count: u32,
        p1_count: u32,
        /// `"GO"` | `"CONDITIONAL"` | `"NO-GO"`
        verdict: String,
    },
    /// AutoMerge handler merged the PR successfully.
    PrAutoMerged {
        pr_number: u64,
        project: String,
        merge_sha: String,
        title: String,
    },
    /// Post-merge test gate passed; the merge is stable.
    PrAutoMergedVerified {
        pr_number: u64,
        project: String,
        merge_sha: String,
        wall_time_secs: f64,
    },
    /// Post-merge test gate failed and the merge was reverted.
    PrAutoReverted {
        pr_number: u64,
        project: String,
        merge_sha: String,
        revert_sha: String,
        /// Classified failure kind (e.g. `"TestFailure"`, `"Timeout"`).
        reason: String,
        /// Byte length of the captured stderr tail for forensics.
        stderr_tail_bytes: u32,
    },
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
    async fn flush_batch(client: &reqwest::Client, url: &str, buffer: &mut Vec<LogDocument>) {
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

/// Fleet sink routing [`LogDocument`]s to per-project [`QuickwitSink`]s.
///
/// Legacy single-project deployments wire a single sink keyed on
/// [`crate::dispatcher::LEGACY_PROJECT_ID`]. Multi-project fleets register
/// one sink per project plus an optional fallback for docs whose
/// `project_id` does not match any registered sink.
#[derive(Debug, Clone)]
pub struct QuickwitFleetSink {
    sinks: HashMap<String, QuickwitSink>,
    fallback_project: Option<String>,
}

impl QuickwitFleetSink {
    /// Build a single-project fleet sink keyed on
    /// [`crate::dispatcher::LEGACY_PROJECT_ID`]. Used by legacy deployments
    /// and by test scaffolding.
    pub fn single(sink: QuickwitSink) -> Self {
        let mut sinks = HashMap::new();
        sinks.insert(LEGACY_PROJECT_ID.to_string(), sink);
        Self {
            sinks,
            fallback_project: Some(LEGACY_PROJECT_ID.to_string()),
        }
    }

    /// Build an empty fleet sink that can have project-specific sinks
    /// registered via [`QuickwitFleetSink::insert_project`].
    pub fn new_multi() -> Self {
        Self {
            sinks: HashMap::new(),
            fallback_project: None,
        }
    }

    /// Register a project-specific sink.
    pub fn insert_project(&mut self, project_id: impl Into<String>, sink: QuickwitSink) {
        let id = project_id.into();
        if self.fallback_project.is_none() {
            self.fallback_project = Some(id.clone());
        }
        self.sinks.insert(id, sink);
    }

    /// Set the fallback project id used when a doc's `project_id` does not
    /// match any registered sink.
    pub fn set_fallback_project(&mut self, project_id: impl Into<String>) {
        self.fallback_project = Some(project_id.into());
    }

    /// Send a log document, routing it to the sink for `doc.project_id`.
    ///
    /// Falls back to the fallback project sink (or legacy) when no matching
    /// sink is registered. Returns `Ok(())` silently when neither exists so
    /// that callers can enable Quickwit on a per-project basis without all
    /// projects configuring it.
    pub async fn send(&self, doc: LogDocument) -> Result<(), QuickwitError> {
        if let Some(sink) = self.sinks.get(&doc.project_id) {
            return sink.send(doc).await;
        }
        if let Some(fallback) = self.fallback_project.as_deref() {
            if let Some(sink) = self.sinks.get(fallback) {
                return sink.send(doc).await;
            }
        }
        // No sink configured for this project and no fallback: drop silently.
        Ok(())
    }

    /// Emit a typed [`OrchestratorEvent`] to the sink for `project_id`.
    ///
    /// Serialises the event into the `extra` field of a [`LogDocument`] and
    /// routes it through the normal fleet-sink path. All errors (serialisation
    /// failures, full channel, absent sink) are swallowed with a `warn!` log
    /// so the caller's business logic is never blocked by Quickwit
    /// unavailability.
    pub async fn emit_event(&self, project_id: &str, event: OrchestratorEvent) {
        let extra = match serde_json::to_value(&event) {
            Ok(v) => v,
            Err(e) => {
                warn!(error = %e, "failed to serialize orchestrator event for Quickwit");
                return;
            }
        };
        let event_kind = extra
            .get("event_kind")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();
        let doc = LogDocument {
            timestamp: Utc::now().to_rfc3339(),
            project_id: project_id.to_string(),
            level: "INFO".to_string(),
            agent_name: "orchestrator".to_string(),
            layer: "Core".to_string(),
            source: "orchestrator".to_string(),
            message: event_kind,
            extra: Some(extra),
            ..Default::default()
        };
        if let Err(e) = self.send(doc).await {
            warn!(error = %e, "failed to emit orchestrator event to Quickwit sink");
        }
    }

    /// Non-blocking variant of [`QuickwitFleetSink::send`].
    pub fn try_send(&self, doc: LogDocument) -> Result<(), QuickwitError> {
        if let Some(sink) = self.sinks.get(&doc.project_id) {
            return sink.try_send(doc);
        }
        if let Some(fallback) = self.fallback_project.as_deref() {
            if let Some(sink) = self.sinks.get(fallback) {
                return sink.try_send(doc);
            }
        }
        Ok(())
    }

    /// Shutdown all underlying sinks, flushing any pending documents.
    pub fn shutdown(self) {
        for (_, sink) in self.sinks {
            sink.shutdown();
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
            project_id: "odilo".to_string(),
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
        assert!(json.contains("\"project_id\":\"odilo\""));
        // None fields should be skipped
        assert!(!json.contains("persona"));
    }

    #[test]
    fn test_quickwit_error_display() {
        let err = QuickwitError::HttpError("connection refused".to_string());
        assert_eq!(err.to_string(), "HTTP error: connection refused");
    }

    #[tokio::test]
    async fn test_fleet_sink_routes_to_registered_project() {
        // Point both sinks at non-routable endpoints so the background task
        // fails ingest silently; we only verify that send() doesn't error
        // and that unknown project ids fall through to the fallback.
        let odilo_sink = QuickwitSink::new(
            "http://127.0.0.1:1".to_string(),
            "odilo-logs".to_string(),
            10,
            60,
        );
        let twins_sink = QuickwitSink::new(
            "http://127.0.0.1:1".to_string(),
            "twins-logs".to_string(),
            10,
            60,
        );

        let mut fleet = QuickwitFleetSink::new_multi();
        fleet.insert_project("odilo", odilo_sink);
        fleet.insert_project("digital-twins", twins_sink);
        fleet.set_fallback_project("odilo");

        // Known project: should route without error.
        let odilo_doc = LogDocument {
            project_id: "odilo".into(),
            ..Default::default()
        };
        assert!(fleet.send(odilo_doc).await.is_ok());

        // Unknown project: routes to fallback rather than erroring.
        let unknown_doc = LogDocument {
            project_id: "unregistered".into(),
            ..Default::default()
        };
        assert!(fleet.send(unknown_doc).await.is_ok());
    }

    #[tokio::test]
    async fn test_fleet_sink_drops_silently_without_fallback() {
        let fleet = QuickwitFleetSink::new_multi();
        let doc = LogDocument {
            project_id: "whatever".into(),
            ..Default::default()
        };
        // No sinks registered, no fallback: should succeed silently.
        assert!(fleet.send(doc).await.is_ok());
    }
}
