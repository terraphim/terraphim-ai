//! RLM dispatch backend for isolated code execution.
//!
//! This module provides an alternative execution backend for agents that need
//! recursive code execution in isolated VMs via the RLM (Recursive Language Model)
//! system instead of spawning CLI subprocesses.

use std::collections::HashMap;
#[cfg(feature = "rlm")]
use std::sync::Arc;

#[cfg(feature = "rlm")]
use terraphim_rlm::{RlmConfig, SessionId, TerraphimRlm};
#[cfg(feature = "rlm")]
use tokio::sync::RwLock;

use crate::config::AgentDefinition;
use crate::error::OrchestratorError;

/// RLM session handle returned after dispatch.
///
/// Contains the session ID and agent name for later cleanup.
#[derive(Debug, Clone)]
pub struct RlmSession {
    /// The RLM session ID.
    pub session_id: String,
    /// The agent name that owns this session.
    pub agent_name: String,
}

/// RLM budget metrics for drift evaluation.
///
/// Tracks token and time consumption for agents using RLM backend.
#[derive(Debug, Clone, Default)]
pub struct RlmBudgetMetrics {
    /// Total tokens consumed.
    pub tokens_used: u64,
    /// Total time consumed in milliseconds.
    pub time_used_ms: u64,
    /// Number of queries executed.
    pub query_count: u64,
    /// Number of sessions created.
    pub session_count: u64,
}

/// Dispatcher for agents using the RLM execution backend.
///
/// Wraps `TerraphimRlm` and manages session lifecycle for agents
/// configured with `backend = "rlm"`.
#[cfg(feature = "rlm")]
pub struct RlmDispatcher {
    /// The underlying RLM instance.
    rlm: Arc<TerraphimRlm>,
    /// Active sessions: agent_name -> session_id.
    sessions: Arc<RwLock<HashMap<String, SessionId>>>,
    /// Budget metrics per agent.
    metrics: Arc<RwLock<HashMap<String, RlmBudgetMetrics>>>,
}

#[cfg(feature = "rlm")]
impl RlmDispatcher {
    /// Create a new RLM dispatcher with default configuration.
    ///
    /// # Errors
    ///
    /// Returns an error if the RLM backend cannot be initialized.
    pub async fn new() -> Result<Self, OrchestratorError> {
        let config = RlmConfig::default();
        Self::with_config(config).await
    }

    /// Create a new RLM dispatcher with custom configuration.
    ///
    /// # Arguments
    ///
    /// * `config` - RLM configuration
    ///
    /// # Errors
    ///
    /// Returns an error if the RLM backend cannot be initialized.
    pub async fn with_config(config: RlmConfig) -> Result<Self, OrchestratorError> {
        let rlm = TerraphimRlm::new(config)
            .await
            .map_err(|e| OrchestratorError::RlmError {
                message: format!("Failed to initialize RLM: {}", e),
            })?;

        Ok(Self {
            rlm: Arc::new(rlm),
            sessions: Arc::new(RwLock::new(HashMap::new())),
            metrics: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    /// Dispatch an agent to execute in an isolated RLM session.
    ///
    /// Creates a new session for the agent, tracks it internally,
    /// and returns the session handle.
    ///
    /// # Arguments
    ///
    /// * `agent_def` - The agent definition
    ///
    /// # Returns
    ///
    /// The RLM session handle on success.
    ///
    /// # Errors
    ///
    /// Returns an error if session creation fails.
    pub async fn dispatch(
        &self,
        agent_def: &AgentDefinition,
    ) -> Result<RlmSession, OrchestratorError> {
        let session = self
            .rlm
            .create_session()
            .await
            .map_err(|e| OrchestratorError::RlmError {
                message: format!("Failed to create RLM session: {}", e),
            })?;

        let session_id = session.id;
        let agent_name = agent_def.name.clone();

        // Track the session
        {
            let mut sessions = self.sessions.write().await;
            sessions.insert(agent_name.clone(), session_id);
        }

        // Initialize metrics
        {
            let mut metrics = self.metrics.write().await;
            metrics.entry(agent_name.clone()).or_default().session_count += 1;
        }

        tracing::info!(
            agent = %agent_name,
            session_id = %session_id,
            "created RLM session for agent"
        );

        Ok(RlmSession {
            session_id: session_id.to_string(),
            agent_name,
        })
    }

    /// Execute code in an agent's RLM session.
    ///
    /// # Arguments
    ///
    /// * `agent_name` - The agent name
    /// * `code` - Python code to execute
    ///
    /// # Returns
    ///
    /// The execution result (stdout, stderr, exit code).
    ///
    /// # Errors
    ///
    /// Returns an error if the agent has no session or execution fails.
    pub async fn execute_code(
        &self,
        agent_name: &str,
        code: &str,
    ) -> Result<terraphim_rlm::ExecutionResult, OrchestratorError> {
        let session_id = {
            let sessions = self.sessions.read().await;
            sessions
                .get(agent_name)
                .cloned()
                .ok_or_else(|| OrchestratorError::AgentNotFound(agent_name.to_string()))?
        };

        let result = self
            .rlm
            .execute_code(&session_id, code)
            .await
            .map_err(|e| OrchestratorError::RlmError {
                message: format!("Code execution failed: {}", e),
            })?;

        // Update metrics
        {
            let mut metrics = self.metrics.write().await;
            if let Some(m) = metrics.get_mut(agent_name) {
                m.query_count += 1;
            }
        }

        Ok(result)
    }

    /// Execute a query in an agent's RLM session.
    ///
    /// This runs the full RLM query loop with LLM integration.
    ///
    /// # Arguments
    ///
    /// * `agent_name` - The agent name
    /// * `prompt` - The prompt for the LLM
    ///
    /// # Returns
    ///
    /// The query loop result.
    ///
    /// # Errors
    ///
    /// Returns an error if the agent has no session or query fails.
    pub async fn query(
        &self,
        agent_name: &str,
        prompt: &str,
    ) -> Result<terraphim_rlm::QueryLoopResult, OrchestratorError> {
        let session_id = {
            let sessions = self.sessions.read().await;
            sessions
                .get(agent_name)
                .cloned()
                .ok_or_else(|| OrchestratorError::AgentNotFound(agent_name.to_string()))?
        };

        let result =
            self.rlm
                .query(&session_id, prompt)
                .await
                .map_err(|e| OrchestratorError::RlmError {
                    message: format!("Query failed: {}", e),
                })?;

        // Get session info to extract budget metrics
        let session_info = self.rlm.get_session(&session_id).ok();

        // Update metrics from session budget status
        {
            let mut metrics = self.metrics.write().await;
            if let Some(m) = metrics.get_mut(agent_name) {
                m.query_count += 1;
                if let Some(info) = session_info {
                    m.tokens_used += info.budget_status.tokens_used;
                    m.time_used_ms += info.budget_status.time_used_ms;
                }
            }
        }

        Ok(result)
    }

    /// Destroy an agent's RLM session and clean up resources.
    ///
    /// # Arguments
    ///
    /// * `agent_name` - The agent name
    ///
    /// # Errors
    ///
    /// Returns an error if session destruction fails.
    pub async fn destroy_session(&self, agent_name: &str) -> Result<(), OrchestratorError> {
        let session_id = {
            let mut sessions = self.sessions.write().await;
            sessions
                .remove(agent_name)
                .ok_or_else(|| OrchestratorError::AgentNotFound(agent_name.to_string()))?
        };

        self.rlm
            .destroy_session(&session_id)
            .await
            .map_err(|e| OrchestratorError::RlmError {
                message: format!("Failed to destroy RLM session: {}", e),
            })?;

        tracing::info!(
            agent = %agent_name,
            session_id = %session_id,
            "destroyed RLM session"
        );

        Ok(())
    }

    /// Get the budget metrics for a specific agent.
    ///
    /// # Arguments
    ///
    /// * `agent_name` - The agent name
    ///
    /// # Returns
    ///
    /// The budget metrics, or None if the agent has no metrics.
    pub async fn get_metrics(&self, agent_name: &str) -> Option<RlmBudgetMetrics> {
        let metrics = self.metrics.read().await;
        metrics.get(agent_name).cloned()
    }

    /// Get all budget metrics.
    ///
    /// # Returns
    ///
    /// A map of agent names to their budget metrics.
    pub async fn get_all_metrics(&self) -> HashMap<String, RlmBudgetMetrics> {
        let metrics = self.metrics.read().await;
        metrics.clone()
    }

    /// Check if the RLM backend is healthy.
    ///
    /// # Returns
    ///
    /// `true` if healthy, `false` otherwise.
    pub async fn health_check(&self) -> bool {
        self.rlm.health_check().await.unwrap_or(false)
    }

    /// Get a reference to the underlying RLM instance.
    ///
    /// This is useful for advanced operations like snapshot management.
    pub fn rlm(&self) -> &TerraphimRlm {
        &self.rlm
    }
}

/// Non-RLM feature stub for when RLM is not enabled.
///
/// Provides type definitions without the full RLM implementation.
#[cfg(not(feature = "rlm"))]
pub struct RlmDispatcher;

#[cfg(not(feature = "rlm"))]
impl RlmDispatcher {
    /// Returns an error indicating RLM is not enabled.
    ///
    /// # Errors
    ///
    /// Always returns `OrchestratorError::RlmNotEnabled`.
    pub async fn new() -> Result<Self, OrchestratorError> {
        Err(OrchestratorError::RlmNotEnabled)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rlm_session_debug() {
        let session = RlmSession {
            session_id: "test-session".to_string(),
            agent_name: "test-agent".to_string(),
        };
        assert!(format!("{:?}", session).contains("test-session"));
    }

    #[test]
    fn test_rlm_budget_metrics_default() {
        let metrics = RlmBudgetMetrics::default();
        assert_eq!(metrics.tokens_used, 0);
        assert_eq!(metrics.time_used_ms, 0);
        assert_eq!(metrics.query_count, 0);
        assert_eq!(metrics.session_count, 0);
    }

    #[cfg(feature = "rlm")]
    #[tokio::test]
    async fn test_rlm_dispatcher_stub() {
        // This test verifies the RLM dispatcher stub compiles
        // The actual RLM functionality is tested in terraphim_rlm crate
        let result = RlmDispatcher::new().await;
        // This will fail because RLM needs a backend, but it proves the module compiles
        assert!(result.is_ok() || matches!(result, Err(OrchestratorError::RlmError { .. })));
    }
}
