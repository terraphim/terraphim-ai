//! LLM Bridge for VM-to-host communication.
//!
//! This module provides an HTTP endpoint that allows code running inside VMs
//! to invoke the LLM recursively. The bridge:
//!
//! - Validates session tokens before processing requests
//! - Forwards queries to the LLM service
//! - Tracks budget consumption (tokens, time)
//! - Supports batched queries for concurrent execution
//!
//! ## Security
//!
//! Every request must include an `X-Session-Token` header containing a valid
//! session ID. The bridge validates this against the SessionManager before
//! processing any queries.
//!
//! ## API Endpoints
//!
//! - `POST /query` - Single LLM query
//! - `POST /query_batched` - Multiple concurrent queries
//! - `GET /health` - Bridge health check

use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::budget::BudgetTracker;
use crate::error::{RlmError, RlmResult};
use crate::session::SessionManager;
use crate::types::{LlmQuery, SessionId};

/// Request body for single LLM query.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryRequest {
    /// The prompt to send to the LLM.
    pub prompt: String,
    /// Optional model override.
    pub model: Option<String>,
    /// Optional temperature override.
    pub temperature: Option<f32>,
    /// Optional max tokens override.
    pub max_tokens: Option<u32>,
}

impl From<QueryRequest> for LlmQuery {
    fn from(req: QueryRequest) -> Self {
        LlmQuery {
            prompt: req.prompt,
            model: req.model,
            temperature: req.temperature,
            max_tokens: req.max_tokens,
        }
    }
}

/// Response body for LLM query.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryResponse {
    /// The LLM's response text.
    pub response: String,
    /// Tokens consumed by this query.
    pub tokens_used: u64,
    /// Time taken in milliseconds.
    pub time_ms: u64,
    /// Remaining token budget after this query.
    pub tokens_remaining: u64,
    /// Remaining time budget in milliseconds.
    pub time_remaining_ms: u64,
}

/// Request body for batched LLM queries.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchedQueryRequest {
    /// List of queries to execute concurrently.
    pub queries: Vec<QueryRequest>,
}

/// Response body for batched queries.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchedQueryResponse {
    /// List of responses in the same order as queries.
    pub responses: Vec<Result<QueryResponse, String>>,
    /// Total tokens consumed by all queries.
    pub total_tokens_used: u64,
    /// Total time taken in milliseconds.
    pub total_time_ms: u64,
}

/// LLM Bridge server configuration.
#[derive(Debug, Clone)]
pub struct LlmBridgeConfig {
    /// Port to listen on (for VM access).
    pub port: u16,
    /// Bind address (typically "0.0.0.0" for VM access).
    pub bind_addr: String,
    /// Maximum concurrent queries in a batch.
    pub max_batch_size: usize,
    /// Request timeout in milliseconds.
    pub request_timeout_ms: u64,
}

impl Default for LlmBridgeConfig {
    fn default() -> Self {
        Self {
            port: 8080,
            bind_addr: "0.0.0.0".to_string(),
            max_batch_size: 10,
            request_timeout_ms: 30_000,
        }
    }
}

/// LLM Bridge handles VM-to-host LLM queries.
///
/// The bridge runs as an HTTP server accessible from VMs and forwards
/// queries to the LLM service while enforcing session-based authentication
/// and budget limits.
pub struct LlmBridge {
    /// Configuration for the bridge.
    config: LlmBridgeConfig,
    /// Session manager for token validation.
    session_manager: Arc<SessionManager>,
    /// Budget trackers per session.
    budget_trackers: dashmap::DashMap<SessionId, Arc<BudgetTracker>>,
}

impl LlmBridge {
    /// Create a new LLM bridge.
    pub fn new(config: LlmBridgeConfig, session_manager: Arc<SessionManager>) -> Self {
        Self {
            config,
            session_manager,
            budget_trackers: dashmap::DashMap::new(),
        }
    }

    /// Validate a session token.
    ///
    /// Returns the session ID if valid.
    pub fn validate_token(&self, token: &str) -> RlmResult<SessionId> {
        let session_id = SessionId::from_string(token).map_err(|_| RlmError::InvalidSessionToken {
            token: token.to_string(),
        })?;

        // Validate session exists and is not expired
        self.session_manager.validate_session(&session_id)?;

        Ok(session_id)
    }

    /// Get or create a budget tracker for a session.
    pub fn get_budget_tracker(&self, session_id: &SessionId) -> Arc<BudgetTracker> {
        self.budget_trackers
            .entry(*session_id)
            .or_insert_with(|| {
                let session = self
                    .session_manager
                    .get_session(session_id)
                    .expect("session should exist after validation");
                Arc::new(BudgetTracker::from_status(&session.budget_status))
            })
            .clone()
    }

    /// Process a single LLM query.
    ///
    /// # Arguments
    ///
    /// * `session_id` - The validated session ID
    /// * `request` - The query request
    ///
    /// # Returns
    ///
    /// The query response with budget information.
    pub async fn query(
        &self,
        session_id: &SessionId,
        request: QueryRequest,
    ) -> RlmResult<QueryResponse> {
        let budget = self.get_budget_tracker(session_id);

        // Check budget before processing
        budget.check_all()?;

        // Increment recursion depth
        budget.push_recursion()?;

        let start = std::time::Instant::now();

        // TODO: Actually call the LLM service
        // For now, return a stub response
        let response_text = format!(
            "[LLM Bridge stub] Query: {}...",
            if request.prompt.len() > 50 {
                &request.prompt[..50]
            } else {
                &request.prompt
            }
        );

        // Estimate tokens (1 token ~= 4 chars for English text)
        let estimated_tokens = (request.prompt.len() / 4 + response_text.len() / 4) as u64;

        // Record token usage
        budget.add_tokens(estimated_tokens)?;

        let time_ms = start.elapsed().as_millis() as u64;

        // Pop recursion depth
        budget.pop_recursion();

        Ok(QueryResponse {
            response: response_text,
            tokens_used: estimated_tokens,
            time_ms,
            tokens_remaining: budget.tokens_remaining(),
            time_remaining_ms: budget.time_remaining_ms(),
        })
    }

    /// Process batched LLM queries concurrently.
    ///
    /// # Arguments
    ///
    /// * `session_id` - The validated session ID
    /// * `request` - The batched query request
    ///
    /// # Returns
    ///
    /// The batched response with all query results.
    pub async fn query_batched(
        &self,
        session_id: &SessionId,
        request: BatchedQueryRequest,
    ) -> RlmResult<BatchedQueryResponse> {
        let budget = self.get_budget_tracker(session_id);

        // Check budget before processing
        budget.check_all()?;

        // Enforce batch size limit
        if request.queries.len() > self.config.max_batch_size {
            return Err(RlmError::BatchSizeTooLarge {
                size: request.queries.len(),
                max: self.config.max_batch_size,
            });
        }

        let start = std::time::Instant::now();

        // Execute queries concurrently using tokio::join_all
        let futures: Vec<_> = request
            .queries
            .into_iter()
            .map(|query| {
                let session_id = *session_id;
                let this = self;
                async move { this.query(&session_id, query).await }
            })
            .collect();

        let results = futures::future::join_all(futures).await;

        // Convert results to response format
        let mut total_tokens = 0u64;
        let responses: Vec<Result<QueryResponse, String>> = results
            .into_iter()
            .map(|r| match r {
                Ok(resp) => {
                    total_tokens += resp.tokens_used;
                    Ok(resp)
                }
                Err(e) => Err(e.to_string()),
            })
            .collect();

        let total_time_ms = start.elapsed().as_millis() as u64;

        Ok(BatchedQueryResponse {
            responses,
            total_tokens_used: total_tokens,
            total_time_ms,
        })
    }

    /// Get the bind address for the HTTP server.
    pub fn bind_addr(&self) -> String {
        format!("{}:{}", self.config.bind_addr, self.config.port)
    }

    /// Get the configuration.
    pub fn config(&self) -> &LlmBridgeConfig {
        &self.config
    }
}

impl BudgetTracker {
    /// Create a budget tracker from a budget status.
    pub fn from_status(status: &crate::types::BudgetStatus) -> Self {
        use crate::config::RlmConfig;

        let config = RlmConfig {
            token_budget: status.token_budget,
            time_budget_ms: status.time_budget_ms,
            max_recursion_depth: status.max_recursion_depth,
            ..Default::default()
        };

        let tracker = Self::new(&config);

        // Restore current state
        if status.tokens_used > 0 {
            tracker.add_tokens(status.tokens_used).ok();
        }

        for _ in 0..status.current_recursion_depth {
            tracker.push_recursion().ok();
        }

        tracker
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::RlmConfig;

    fn create_test_bridge() -> (LlmBridge, SessionId) {
        let config = RlmConfig::default();
        let session_manager = Arc::new(SessionManager::new(config));
        let session = session_manager.create_session().unwrap();

        let bridge = LlmBridge::new(LlmBridgeConfig::default(), session_manager);

        (bridge, session.id)
    }

    #[test]
    fn test_token_validation() {
        let (bridge, session_id) = create_test_bridge();

        // Valid token should succeed
        let result = bridge.validate_token(&session_id.to_string());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), session_id);

        // Invalid token should fail
        let result = bridge.validate_token("invalid-token");
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_single_query() {
        let (bridge, session_id) = create_test_bridge();

        let request = QueryRequest {
            prompt: "Hello, world!".to_string(),
            model: None,
            temperature: None,
            max_tokens: None,
        };

        let result = bridge.query(&session_id, request).await;
        assert!(result.is_ok());

        let response = result.unwrap();
        assert!(!response.response.is_empty());
        assert!(response.tokens_used > 0);
    }

    #[tokio::test]
    async fn test_batched_query() {
        let (bridge, session_id) = create_test_bridge();

        let request = BatchedQueryRequest {
            queries: vec![
                QueryRequest {
                    prompt: "Query 1".to_string(),
                    model: None,
                    temperature: None,
                    max_tokens: None,
                },
                QueryRequest {
                    prompt: "Query 2".to_string(),
                    model: None,
                    temperature: None,
                    max_tokens: None,
                },
            ],
        };

        let result = bridge.query_batched(&session_id, request).await;
        assert!(result.is_ok());

        let response = result.unwrap();
        assert_eq!(response.responses.len(), 2);
        assert!(response.total_tokens_used > 0);
    }

    #[tokio::test]
    async fn test_batch_size_limit() {
        let config = LlmBridgeConfig {
            max_batch_size: 2,
            ..Default::default()
        };
        let rlm_config = RlmConfig::default();
        let session_manager = Arc::new(SessionManager::new(rlm_config));
        let session = session_manager.create_session().unwrap();

        let bridge = LlmBridge::new(config, session_manager);

        let request = BatchedQueryRequest {
            queries: vec![
                QueryRequest {
                    prompt: "Query 1".to_string(),
                    model: None,
                    temperature: None,
                    max_tokens: None,
                },
                QueryRequest {
                    prompt: "Query 2".to_string(),
                    model: None,
                    temperature: None,
                    max_tokens: None,
                },
                QueryRequest {
                    prompt: "Query 3".to_string(),
                    model: None,
                    temperature: None,
                    max_tokens: None,
                },
            ],
        };

        let result = bridge.query_batched(&session.id, request).await;
        assert!(matches!(result, Err(RlmError::BatchSizeTooLarge { .. })));
    }

    #[test]
    fn test_budget_tracker_from_status() {
        let status = crate::types::BudgetStatus {
            token_budget: 1000,
            tokens_used: 100,
            time_budget_ms: 60_000,
            time_used_ms: 0,
            max_recursion_depth: 5,
            current_recursion_depth: 2,
        };

        let tracker = BudgetTracker::from_status(&status);
        assert_eq!(tracker.tokens_used(), 100);
        assert_eq!(tracker.current_depth(), 2);
    }
}
