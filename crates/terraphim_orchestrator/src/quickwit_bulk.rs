//! Quickwit ES-compatible bulk ingestion with retry logic.
//!
//! Replaces the native `/api/v1/{index}/ingest` endpoint with the
//! Elasticsearch `_bulk` API (`/api/v1/_elastic/{index}/_bulk`), which
//! supports `refresh=true` for immediate visibility.
//!
//! Uses `reqwest-retry` for automatic retries on transient errors
//! including 429 Too Many Requests.

use std::time::Duration;

use reqwest::header::{HeaderMap, HeaderValue, CONTENT_TYPE};
use reqwest::Body;
use reqwest_retry::policies::ExponentialBackoff;
use reqwest_retry::RetryTransientMiddleware;
use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

use crate::quickwit::{LogDocument, QuickwitError};

/// ES bulk action for indexing documents.
#[derive(Debug, Serialize)]
struct BulkAction {
    index: BulkIndex,
}

#[derive(Debug, Serialize)]
struct BulkIndex {
    #[serde(rename = "_index")]
    index: String,
}

/// Response item from ES bulk API.
#[derive(Debug, Deserialize)]
struct BulkResponseItem {
    index: Option<BulkItemResult>,
}

#[derive(Debug, Deserialize)]
struct BulkItemResult {
    #[serde(rename = "_index")]
    index: String,
    #[serde(rename = "_id")]
    id: String,
    status: u16,
    error: Option<BulkError>,
}

#[derive(Debug, Deserialize)]
struct BulkError {
    #[serde(rename = "type")]
    error_type: String,
    reason: String,
}

/// ES bulk response wrapper.
#[derive(Debug, Deserialize)]
struct BulkResponse {
    took: u64,
    errors: bool,
    items: Vec<BulkResponseItem>,
}

/// Async ES bulk log shipper for Quickwit with retry logic.
#[derive(Debug, Clone)]
pub struct QuickwitEsBulkSink {
    client: reqwest_middleware::ClientWithMiddleware,
    endpoint: String,
    index_id: String,
}

impl QuickwitEsBulkSink {
    /// Create a new ES bulk sink.
    ///
    /// # Arguments
    /// * `endpoint` - Quickwit API endpoint (e.g., "http://127.0.0.1:7280")
    /// * `index_id` - Index name (e.g., "adf-logs")
    pub fn new(endpoint: String, index_id: String) -> Self {
        let retry_policy = ExponentialBackoff::builder()
            .base(2)
            .retry_bounds(Duration::from_millis(100), Duration::from_secs(10))
            .build_with_max_retries(3);

        let client = reqwest_middleware::ClientBuilder::new(reqwest::Client::new())
            .with(RetryTransientMiddleware::new_with_policy(retry_policy))
            .build();

        Self {
            client,
            endpoint,
            index_id,
        }
    }

    /// Build the ES bulk URL with refresh parameter.
    fn bulk_url(&self) -> String {
        format!(
            "{}/api/v1/_elastic/{}/_bulk?refresh=true",
            self.endpoint, self.index_id
        )
    }

    /// Send a batch of documents using ES bulk API.
    ///
    /// Returns the number of successfully indexed documents.
    pub async fn send_batch(
        &self,
        documents: Vec<LogDocument>,
    ) -> Result<usize, QuickwitError> {
        if documents.is_empty() {
            return Ok(0);
        }

        let body = self.build_bulk_body(&documents).map_err(|e| {
            QuickwitError::SerializationError(format!("failed to build bulk body: {}", e))
        })?;

        let mut headers = HeaderMap::new();
        headers.insert(
            CONTENT_TYPE,
            HeaderValue::from_static("application/x-ndjson"),
        );

        let url = self.bulk_url();
        debug!(
            url = %url,
            doc_count = documents.len(),
            body_bytes = body.len(),
            "sending ES bulk request"
        );

        let response = self
            .client
            .post(&url)
            .headers(headers)
            .body(Body::from(body))
            .send()
            .await
            .map_err(|e| QuickwitError::HttpError(format!("request failed: {}", e)))?;

        let status = response.status();
        let response_body = response
            .text()
            .await
            .map_err(|e| QuickwitError::HttpError(format!("failed to read response: {}", e)))?;

        if !status.is_success() {
            warn!(
                status = %status,
                body = %response_body,
                "ES bulk request failed"
            );
            return Err(QuickwitError::HttpError(format!(
                "HTTP {}: {}",
                status, response_body
            )));
        }

        // Parse response to count successes
        let bulk_response: BulkResponse =
            serde_json::from_str(&response_body).map_err(|e| {
                QuickwitError::SerializationError(format!(
                    "failed to parse bulk response: {} (body: {})",
                    e, response_body
                ))
            })?;

        let success_count = bulk_response
            .items
            .iter()
            .filter(|item| {
                item.index
                    .as_ref()
                    .map(|idx| idx.status >= 200 && idx.status < 300)
                    .unwrap_or(false)
            })
            .count();

        let error_count = documents.len() - success_count;

        if bulk_response.errors {
            warn!(
                success = success_count,
                errors = error_count,
                took_ms = bulk_response.took,
                "ES bulk completed with errors"
            );
        } else {
            info!(
                success = success_count,
                took_ms = bulk_response.took,
                "ES bulk completed successfully"
            );
        }

        if error_count > 0 {
            // Log details of failed items
            for item in &bulk_response.items {
                if let Some(idx) = &item.index {
                    if idx.status >= 400 {
                        if let Some(error) = &idx.error {
                            warn!(
                                index = %idx.index,
                                id = %idx.id,
                                status = idx.status,
                                error_type = %error.error_type,
                                reason = %error.reason,
                                "bulk item failed"
                            );
                        }
                    }
                }
            }
        }

        Ok(success_count)
    }

    /// Build the NDJSON bulk body.
    fn build_bulk_body(&self, documents: &[LogDocument]
    ) -> Result<String, serde_json::Error> {
        let mut body = String::new();

        for doc in documents {
            // Action line
            let action = BulkAction {
                index: BulkIndex {
                    index: self.index_id.clone(),
                },
            };
            let action_json = serde_json::to_string(&action)?;
            body.push_str(&action_json);
            body.push('\n');

            // Document line
            let doc_json = serde_json::to_string(doc)?;
            body.push_str(&doc_json);
            body.push('\n');
        }

        Ok(body)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::quickwit::LogDocument;

    #[test]
    fn test_build_bulk_body() {
        let sink = QuickwitEsBulkSink::new(
            "http://localhost:7280".to_string(),
            "test-index".to_string(),
        );

        let docs = vec![
            LogDocument {
                timestamp: "2026-05-11T10:00:00Z".to_string(),
                project_id: "test".to_string(),
                level: "INFO".to_string(),
                agent_name: "test-agent".to_string(),
                layer: "Core".to_string(),
                source: "orchestrator".to_string(),
                message: "test message".to_string(),
                ..Default::default()
            },
        ];

        let body = sink.build_bulk_body(&docs).unwrap();
        assert!(body.contains("\"index\""));
        assert!(body.contains("\"test-index\""));
        assert!(body.contains("test-agent"));
    }

    #[test]
    fn test_bulk_url() {
        let sink = QuickwitEsBulkSink::new(
            "http://localhost:7280".to_string(),
            "adf-logs".to_string(),
        );
        assert_eq!(
            sink.bulk_url(),
            "http://localhost:7280/api/v1/_elastic/adf-logs/_bulk?refresh=true"
        );
    }
}
