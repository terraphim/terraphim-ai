//! Execution context and result types.
//!
//! These types are shared across all execution backends.

use jiff::Timestamp;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use ulid::Ulid;

use crate::types::SessionId;

/// Unique identifier for a snapshot.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SnapshotId {
    /// Internal ULID.
    pub id: Ulid,
    /// User-provided name.
    pub name: String,
    /// Session this snapshot belongs to.
    pub session_id: SessionId,
    /// When the snapshot was created.
    pub created_at: Timestamp,
}

impl SnapshotId {
    /// Create a new snapshot ID.
    pub fn new(name: impl Into<String>, session_id: SessionId) -> Self {
        Self {
            id: Ulid::new(),
            name: name.into(),
            session_id,
            created_at: Timestamp::now(),
        }
    }
}

impl std::fmt::Display for SnapshotId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.name, self.id)
    }
}

/// Context passed to execution operations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionContext {
    /// Session this execution belongs to.
    pub session_id: SessionId,

    /// Execution timeout in milliseconds.
    pub timeout_ms: u64,

    /// Working directory (relative to session root).
    pub working_dir: Option<String>,

    /// Environment variables to set.
    pub env_vars: HashMap<String, String>,

    /// Whether to capture stdout.
    pub capture_stdout: bool,

    /// Whether to capture stderr.
    pub capture_stderr: bool,

    /// Maximum output size before streaming to file.
    pub max_output_bytes: u64,

    /// Cancellation token (passed as ULID for serialization).
    pub cancellation_token: Option<Ulid>,

    /// Session token for LLM bridge authentication.
    pub session_token: Option<String>,

    /// Current recursion depth (for recursive LLM calls).
    pub recursion_depth: u32,
}

impl Default for ExecutionContext {
    fn default() -> Self {
        Self {
            session_id: SessionId::new(),
            timeout_ms: 30_000, // 30 seconds
            working_dir: None,
            env_vars: HashMap::new(),
            capture_stdout: true,
            capture_stderr: true,
            max_output_bytes: crate::DEFAULT_MAX_INLINE_OUTPUT_BYTES,
            cancellation_token: None,
            session_token: None,
            recursion_depth: 0,
        }
    }
}

impl ExecutionContext {
    /// Create a new context for a session.
    pub fn for_session(session_id: SessionId) -> Self {
        Self {
            session_id,
            session_token: Some(session_id.to_string()),
            ..Default::default()
        }
    }

    /// Set the execution timeout.
    pub fn with_timeout(mut self, timeout_ms: u64) -> Self {
        self.timeout_ms = timeout_ms;
        self
    }

    /// Set the working directory.
    pub fn with_working_dir(mut self, dir: impl Into<String>) -> Self {
        self.working_dir = Some(dir.into());
        self
    }

    /// Add an environment variable.
    pub fn with_env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.env_vars.insert(key.into(), value.into());
        self
    }

    /// Set multiple environment variables.
    pub fn with_env_vars(mut self, vars: HashMap<String, String>) -> Self {
        self.env_vars.extend(vars);
        self
    }

    /// Set the recursion depth.
    pub fn with_recursion_depth(mut self, depth: u32) -> Self {
        self.recursion_depth = depth;
        self
    }
}

/// Result of an execution operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResult {
    /// Standard output.
    pub stdout: String,

    /// Standard error.
    pub stderr: String,

    /// Exit code (0 = success).
    pub exit_code: i32,

    /// Execution time in milliseconds.
    pub execution_time_ms: u64,

    /// Whether output was truncated (too large for inline return).
    pub output_truncated: bool,

    /// Path to output file if output was streamed to file.
    pub output_file_path: Option<String>,

    /// Whether execution was killed due to timeout.
    pub timed_out: bool,

    /// Additional metadata from the execution.
    pub metadata: HashMap<String, String>,
}

impl ExecutionResult {
    /// Create a successful result.
    pub fn success(stdout: impl Into<String>) -> Self {
        Self {
            stdout: stdout.into(),
            stderr: String::new(),
            exit_code: 0,
            execution_time_ms: 0,
            output_truncated: false,
            output_file_path: None,
            timed_out: false,
            metadata: HashMap::new(),
        }
    }

    /// Create a failed result.
    pub fn failure(stderr: impl Into<String>, exit_code: i32) -> Self {
        Self {
            stdout: String::new(),
            stderr: stderr.into(),
            exit_code,
            execution_time_ms: 0,
            output_truncated: false,
            output_file_path: None,
            timed_out: false,
            metadata: HashMap::new(),
        }
    }

    /// Create a timeout result.
    pub fn timeout(partial_stdout: String, partial_stderr: String) -> Self {
        Self {
            stdout: partial_stdout,
            stderr: partial_stderr,
            exit_code: -1,
            execution_time_ms: 0,
            output_truncated: false,
            output_file_path: None,
            timed_out: true,
            metadata: HashMap::new(),
        }
    }

    /// Check if the execution succeeded.
    pub fn is_success(&self) -> bool {
        self.exit_code == 0 && !self.timed_out
    }

    /// Set execution time.
    pub fn with_execution_time(mut self, time_ms: u64) -> Self {
        self.execution_time_ms = time_ms;
        self
    }

    /// Add metadata.
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }
}

/// Result of knowledge graph validation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    /// Whether the input is valid (all terms known).
    pub is_valid: bool,

    /// Terms that matched in the knowledge graph.
    pub matched_terms: Vec<String>,

    /// Terms that were not found in the knowledge graph.
    pub unknown_terms: Vec<String>,

    /// Suggested alternatives for unknown terms (if any).
    pub suggestions: HashMap<String, Vec<String>>,

    /// Validation strictness level used.
    pub strictness: crate::config::KgStrictness,
}

impl ValidationResult {
    /// Create a valid result.
    pub fn valid(matched_terms: Vec<String>) -> Self {
        Self {
            is_valid: true,
            matched_terms,
            unknown_terms: Vec::new(),
            suggestions: HashMap::new(),
            strictness: crate::config::KgStrictness::Normal,
        }
    }

    /// Create an invalid result.
    pub fn invalid(matched_terms: Vec<String>, unknown_terms: Vec<String>) -> Self {
        Self {
            is_valid: false,
            matched_terms,
            unknown_terms,
            suggestions: HashMap::new(),
            strictness: crate::config::KgStrictness::Normal,
        }
    }

    /// Add suggestions for unknown terms.
    pub fn with_suggestions(mut self, suggestions: HashMap<String, Vec<String>>) -> Self {
        self.suggestions = suggestions;
        self
    }

    /// Set the strictness level.
    pub fn with_strictness(mut self, strictness: crate::config::KgStrictness) -> Self {
        self.strictness = strictness;
        self
    }
}

/// Capabilities that an execution backend may support.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Capability {
    /// Full VM isolation (Firecracker).
    VmIsolation,
    /// Container isolation (Docker).
    ContainerIsolation,
    /// Create/restore snapshots.
    Snapshots,
    /// Network audit logging.
    NetworkAudit,
    /// OverlayFS for session packages.
    OverlayFs,
    /// Recursive LLM calls via bridge.
    LlmBridge,
    /// DNS allowlist enforcement.
    DnsAllowlist,
    /// Configurable resource limits.
    ResourceLimits,
    /// Python execution.
    PythonExecution,
    /// Bash execution.
    BashExecution,
    /// File operations.
    FileOperations,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_snapshot_id_creation() {
        let session_id = SessionId::new();
        let snapshot = SnapshotId::new("test-snapshot", session_id);
        assert_eq!(snapshot.name, "test-snapshot");
        assert_eq!(snapshot.session_id, session_id);
    }

    #[test]
    fn test_execution_context_builder() {
        let session_id = SessionId::new();
        let ctx = ExecutionContext::for_session(session_id)
            .with_timeout(60_000)
            .with_working_dir("/home/user")
            .with_env("FOO", "bar");

        assert_eq!(ctx.timeout_ms, 60_000);
        assert_eq!(ctx.working_dir, Some("/home/user".to_string()));
        assert_eq!(ctx.env_vars.get("FOO"), Some(&"bar".to_string()));
    }

    #[test]
    fn test_execution_result_success() {
        let result = ExecutionResult::success("hello world");
        assert!(result.is_success());
        assert_eq!(result.stdout, "hello world");
        assert_eq!(result.exit_code, 0);
    }

    #[test]
    fn test_execution_result_failure() {
        let result = ExecutionResult::failure("error message", 1);
        assert!(!result.is_success());
        assert_eq!(result.stderr, "error message");
        assert_eq!(result.exit_code, 1);
    }

    #[test]
    fn test_validation_result() {
        let result = ValidationResult::valid(vec!["python".to_string(), "pip".to_string()]);
        assert!(result.is_valid);
        assert_eq!(result.matched_terms.len(), 2);

        let invalid = ValidationResult::invalid(
            vec!["python".to_string()],
            vec!["foobar".to_string()],
        );
        assert!(!invalid.is_valid);
        assert_eq!(invalid.unknown_terms.len(), 1);
    }
}
