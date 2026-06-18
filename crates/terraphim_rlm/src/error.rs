//! Error types for the RLM orchestration system.
//!
//! This module defines all error types that can occur during RLM operations,
//! including execution errors, budget violations, and validation failures.

use crate::types::SessionId;
use thiserror::Error;

/// The main error type for RLM operations.
#[derive(Error, Debug)]
pub enum RlmError {
    // Session errors
    /// Session not found.
    #[error("Session not found: {session_id}")]
    SessionNotFound {
        /// The session that was looked up but not found.
        session_id: SessionId,
    },

    /// Session has expired.
    #[error("Session expired: {session_id}")]
    SessionExpired {
        /// The session that expired.
        session_id: SessionId,
    },

    /// Session is in an invalid state for the requested operation.
    #[error("Session {session_id} is in invalid state {state} for operation {operation}")]
    InvalidSessionState {
        /// The session that is in an invalid state.
        session_id: SessionId,
        /// Current session state name.
        state: String,
        /// Requested operation that requires a different state.
        operation: String,
    },

    /// Maximum session extensions reached.
    #[error("Session {session_id} has reached maximum extensions ({max})")]
    MaxExtensionsReached {
        /// The session that cannot be extended further.
        session_id: SessionId,
        /// Maximum number of extensions allowed.
        max: u32,
    },

    // Budget errors
    /// Token budget exceeded.
    #[error("Token budget exceeded: used {used} of {budget} tokens")]
    TokenBudgetExceeded {
        /// Tokens consumed so far.
        used: u64,
        /// Configured token budget limit.
        budget: u64,
    },

    /// Time budget exceeded.
    #[error("Time budget exceeded: used {used_ms}ms of {budget_ms}ms")]
    TimeBudgetExceeded {
        /// Milliseconds elapsed.
        used_ms: u64,
        /// Configured time budget in milliseconds.
        budget_ms: u64,
    },

    /// Recursion depth limit exceeded.
    #[error("Recursion depth limit exceeded: {depth} >= {max_depth}")]
    RecursionDepthExceeded {
        /// Current recursion depth.
        depth: u32,
        /// Maximum allowed recursion depth.
        max_depth: u32,
    },

    // Execution errors
    /// Code execution failed.
    #[error("Code execution failed: {message}")]
    ExecutionFailed {
        /// Human-readable failure description.
        message: String,
        /// Process exit code, if available.
        exit_code: Option<i32>,
        /// Captured stdout, if available.
        stdout: Option<String>,
        /// Captured stderr, if available.
        stderr: Option<String>,
    },

    /// Command parsing failed.
    #[error("Failed to parse command from LLM output: {message}")]
    CommandParseFailed {
        /// Description of the parse failure.
        message: String,
    },

    /// Execution timed out.
    #[error("Execution timed out after {timeout_ms}ms")]
    ExecutionTimeout {
        /// Configured timeout in milliseconds.
        timeout_ms: u64,
    },

    /// VM crashed or became unresponsive.
    #[error("VM crashed: {message}")]
    VmCrashed {
        /// Description of the crash.
        message: String,
    },

    // Validation errors
    /// Knowledge graph validation failed.
    #[error("KG validation failed: unknown terms {unknown_terms:?}")]
    KgValidationFailed {
        /// Terms not found in the knowledge graph.
        unknown_terms: Vec<String>,
    },

    /// KG validation requires user escalation.
    #[error("KG validation requires user approval for terms: {unknown_terms:?}")]
    KgEscalationRequired {
        /// Terms requiring approval.
        unknown_terms: Vec<String>,
        /// Suggested remediation action.
        suggested_action: String,
        /// Context around the failing terms.
        context: String,
    },

    // Snapshot errors
    /// Snapshot not found.
    #[error("Snapshot not found: {snapshot_id}")]
    SnapshotNotFound {
        /// Snapshot identifier that was not found.
        snapshot_id: String,
    },

    /// Maximum snapshots per session reached.
    #[error("Maximum snapshots ({max}) reached for session")]
    MaxSnapshotsReached {
        /// Maximum allowed snapshots per session.
        max: u32,
    },

    /// Snapshot creation failed.
    #[error("Failed to create snapshot: {message}")]
    SnapshotCreationFailed {
        /// Description of the failure.
        message: String,
    },

    /// Snapshot restoration failed.
    #[error("Failed to restore snapshot: {message}")]
    SnapshotRestoreFailed {
        /// Description of the failure.
        message: String,
    },

    // Backend errors
    /// No execution backend available.
    #[error("No execution backend available. Tried: {tried:?}")]
    NoBackendAvailable {
        /// Backend names that were attempted.
        tried: Vec<String>,
    },

    /// Backend initialization failed.
    #[error("Failed to initialize {backend} backend: {message}")]
    BackendInitFailed {
        /// Backend that failed to initialise.
        backend: String,
        /// Description of the failure.
        message: String,
    },

    /// VM pool exhausted (all VMs busy, no overflow capacity).
    #[error(
        "VM pool exhausted: all {pool_size} VMs busy, overflow at capacity ({overflow_count}/{max_overflow})"
    )]
    PoolExhausted {
        /// Number of VMs in the pool.
        pool_size: u32,
        /// Number of overflow VMs currently running.
        overflow_count: u32,
        /// Maximum overflow VMs allowed.
        max_overflow: u32,
    },

    /// VM allocation timed out.
    #[error("VM allocation timed out after {timeout_ms}ms")]
    VmAllocationTimeout {
        /// Configured allocation timeout in milliseconds.
        timeout_ms: u64,
    },

    // Network/DNS errors
    /// DNS query blocked by allowlist.
    #[error("DNS query blocked: {domain} not in allowlist")]
    DnsBlocked {
        /// The domain that was blocked.
        domain: String,
    },

    /// Network request blocked.
    #[error("Network request blocked: {url}")]
    NetworkBlocked {
        /// The URL that was blocked.
        url: String,
    },

    // LLM errors
    /// LLM call failed.
    #[error("LLM call failed: {message}")]
    LlmCallFailed {
        /// Description of the failure.
        message: String,
    },

    /// No LLM client configured. Enable the `llm` feature and set an API key
    /// or run a local Ollama instance.
    #[error(
        "No LLM client configured. Enable the `llm` feature (--features llm) and set OPENROUTER_API_KEY or run Ollama on localhost:11434."
    )]
    LlmNotConfigured,

    /// LLM bridge authentication failed.
    #[error("LLM bridge authentication failed: invalid session token")]
    LlmBridgeAuthFailed,

    /// Invalid session token format.
    #[error("Invalid session token: {token}")]
    InvalidSessionToken {
        /// The invalid token that was presented.
        token: String,
    },

    /// Batch query size too large.
    #[error("Batch size {size} exceeds maximum {max}")]
    BatchSizeTooLarge {
        /// Number of items in the batch.
        size: usize,
        /// Maximum allowed batch size.
        max: usize,
    },

    // Output errors
    /// Output too large for inline return.
    #[error("Output exceeds inline limit ({size} > {limit} bytes), streamed to {file_path}")]
    OutputTooLarge {
        /// Actual output size in bytes.
        size: u64,
        /// Inline size limit in bytes.
        limit: u64,
        /// Path where the output was written.
        file_path: String,
    },

    // Operations errors
    /// Auto-remediation failed.
    #[error("Auto-remediation failed after {attempts} attempts: {message}")]
    AutoRemediationFailed {
        /// Number of remediation attempts made.
        attempts: u32,
        /// Description of the final failure.
        message: String,
    },

    /// Alert webhook failed.
    #[error("Failed to send alert to webhook: {message}")]
    AlertWebhookFailed {
        /// Description of the failure.
        message: String,
    },

    // Generic errors
    /// Configuration error.
    #[error("Configuration error: {message}")]
    ConfigError {
        /// Description of the configuration problem.
        message: String,
    },

    /// Operation is not supported by the selected backend.
    #[error("Backend '{backend}' does not support operation '{op}'")]
    NotSupported {
        /// Backend that received the unsupported request.
        backend: String,
        /// Operation that is not supported.
        op: String,
    },

    /// Internal error.
    #[error("Internal error: {message}")]
    Internal {
        /// Description of the internal error.
        message: String,
    },

    /// Cancelled by user or parent.
    #[error("Operation cancelled")]
    Cancelled,

    /// IO error wrapper.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Serialization error wrapper.
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

impl RlmError {
    /// Check if this error is retryable.
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            RlmError::ExecutionTimeout { .. }
                | RlmError::VmAllocationTimeout { .. }
                | RlmError::LlmCallFailed { .. }
                | RlmError::AlertWebhookFailed { .. }
        )
    }

    /// Check if this error indicates a budget exhaustion.
    pub fn is_budget_exhausted(&self) -> bool {
        matches!(
            self,
            RlmError::TokenBudgetExceeded { .. }
                | RlmError::TimeBudgetExceeded { .. }
                | RlmError::RecursionDepthExceeded { .. }
        )
    }

    /// Check if this error requires user intervention.
    pub fn requires_user_action(&self) -> bool {
        matches!(
            self,
            RlmError::KgEscalationRequired { .. }
                | RlmError::MaxExtensionsReached { .. }
                | RlmError::NoBackendAvailable { .. }
        )
    }

    /// Convert to MCP error format.
    pub fn to_mcp_error(&self) -> McpError {
        McpError {
            code: self.mcp_error_code(),
            message: self.to_string(),
            data: self.mcp_error_data(),
        }
    }

    fn mcp_error_code(&self) -> i32 {
        match self {
            // Standard JSON-RPC error codes
            RlmError::CommandParseFailed { .. } => -32700, // Parse error
            RlmError::ConfigError { .. } => -32602,        // Invalid params

            // Custom error codes (starting from -32000)
            RlmError::SessionNotFound { .. } => -32001,
            RlmError::SessionExpired { .. } => -32002,
            RlmError::TokenBudgetExceeded { .. } => -32010,
            RlmError::TimeBudgetExceeded { .. } => -32011,
            RlmError::RecursionDepthExceeded { .. } => -32012,
            RlmError::ExecutionFailed { .. } => -32020,
            RlmError::ExecutionTimeout { .. } => -32021,
            RlmError::VmCrashed { .. } => -32022,
            RlmError::KgValidationFailed { .. } => -32030,
            RlmError::KgEscalationRequired { .. } => -32031,
            RlmError::SnapshotNotFound { .. } => -32040,
            RlmError::NoBackendAvailable { .. } => -32050,
            RlmError::DnsBlocked { .. } => -32060,
            RlmError::NotSupported { .. } => -32070,
            RlmError::Cancelled => -32099,
            _ => -32000, // Generic server error
        }
    }

    fn mcp_error_data(&self) -> Option<serde_json::Value> {
        match self {
            RlmError::KgEscalationRequired {
                unknown_terms,
                suggested_action,
                context,
            } => Some(serde_json::json!({
                "unknown_terms": unknown_terms,
                "suggested_action": suggested_action,
                "context": context,
            })),
            RlmError::ExecutionFailed {
                exit_code,
                stdout,
                stderr,
                ..
            } => Some(serde_json::json!({
                "exit_code": exit_code,
                "stdout": stdout,
                "stderr": stderr,
            })),
            RlmError::OutputTooLarge {
                size,
                limit,
                file_path,
            } => Some(serde_json::json!({
                "size": size,
                "limit": limit,
                "file_path": file_path,
            })),
            _ => None,
        }
    }
}

/// MCP-formatted error for protocol responses.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct McpError {
    /// Error code (JSON-RPC style).
    pub code: i32,
    /// Human-readable error message.
    pub message: String,
    /// Optional additional data.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

/// Result type alias for RLM operations.
pub type RlmResult<T> = Result<T, RlmError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_retryable() {
        let retryable = RlmError::ExecutionTimeout { timeout_ms: 1000 };
        assert!(retryable.is_retryable());

        let not_retryable = RlmError::Cancelled;
        assert!(!not_retryable.is_retryable());
    }

    #[test]
    fn test_error_budget_exhausted() {
        let budget = RlmError::TokenBudgetExceeded {
            used: 100,
            budget: 50,
        };
        assert!(budget.is_budget_exhausted());

        let not_budget = RlmError::Cancelled;
        assert!(!not_budget.is_budget_exhausted());
    }

    #[test]
    fn test_not_supported_not_retryable_and_has_code() {
        let err = RlmError::NotSupported {
            backend: "local".to_string(),
            op: "create_snapshot".to_string(),
        };
        assert!(!err.is_retryable());
        assert!(!err.is_budget_exhausted());
        assert_eq!(err.to_mcp_error().code, -32070);
        let display = err.to_string();
        assert!(display.contains("local"));
        assert!(display.contains("create_snapshot"));
    }

    #[test]
    fn test_mcp_error_conversion() {
        let error = RlmError::KgEscalationRequired {
            unknown_terms: vec!["foo".to_string(), "bar".to_string()],
            suggested_action: "approve".to_string(),
            context: "testing".to_string(),
        };

        let mcp = error.to_mcp_error();
        assert_eq!(mcp.code, -32031);
        assert!(mcp.data.is_some());
    }
}
