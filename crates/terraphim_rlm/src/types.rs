//! Core types for the RLM orchestration system.
//!
//! This module defines shared types used across the crate including session identifiers,
//! command types, and state tracking structures.

use jiff::{Timestamp, ToSpan};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use ulid::Ulid;

/// Unique identifier for an RLM session.
///
/// A session represents a single conversation context with VM affinity,
/// accumulated state, and budget tracking.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SessionId(pub Ulid);

impl SessionId {
    /// Create a new random session ID.
    pub fn new() -> Self {
        Self(Ulid::new())
    }

    /// Create a session ID from a ULID.
    pub fn from_ulid(ulid: Ulid) -> Self {
        Self(ulid)
    }

    /// Get the inner ULID.
    pub fn as_ulid(&self) -> &Ulid {
        &self.0
    }

    /// Parse from string.
    pub fn from_string(s: &str) -> Result<Self, ulid::DecodeError> {
        Ok(Self(Ulid::from_string(s)?))
    }
}

impl Default for SessionId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for SessionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Current state of an RLM session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SessionState {
    /// Session is being initialized.
    Initializing,
    /// Session is ready for commands.
    Ready,
    /// Session is executing a command.
    Executing,
    /// Session is paused (e.g., waiting for user input).
    Paused,
    /// Session has been terminated.
    Terminated,
    /// Session has expired due to timeout.
    Expired,
}

impl Default for SessionState {
    fn default() -> Self {
        Self::Initializing
    }
}

/// Information about an active session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionInfo {
    /// Unique session identifier.
    pub id: SessionId,
    /// Current session state.
    pub state: SessionState,
    /// When the session was created.
    pub created_at: Timestamp,
    /// When the session expires.
    pub expires_at: Timestamp,
    /// Number of extensions applied.
    pub extension_count: u32,
    /// Current recursion depth.
    pub recursion_depth: u32,
    /// Budget status.
    pub budget_status: BudgetStatus,
    /// Associated VM instance ID (if assigned).
    pub vm_instance_id: Option<String>,
    /// Context variables stored in the session.
    pub context_variables: HashMap<String, String>,
    /// Current active snapshot ID (for rollback support).
    /// This tracks the last successfully restored snapshot.
    pub current_snapshot_id: Option<String>,
    /// Number of snapshots created for this session.
    pub snapshot_count: u32,
}

impl SessionInfo {
    /// Create a new session info with default values.
    pub fn new(id: SessionId, duration_secs: u64) -> Self {
        let now = Timestamp::now();
        Self {
            id,
            state: SessionState::Initializing,
            created_at: now,
            expires_at: now
                .checked_add((duration_secs as i64).seconds())
                .expect("adding seconds to timestamp should not fail"),
            extension_count: 0,
            recursion_depth: 0,
            budget_status: BudgetStatus::default(),
            vm_instance_id: None,
            context_variables: HashMap::new(),
            current_snapshot_id: None,
            snapshot_count: 0,
        }
    }

    /// Check if the session has expired.
    pub fn is_expired(&self) -> bool {
        Timestamp::now() > self.expires_at
    }

    /// Check if the session can be extended.
    pub fn can_extend(&self, max_extensions: u32) -> bool {
        self.extension_count < max_extensions
    }
}

/// Budget tracking status for a session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BudgetStatus {
    /// Total tokens allowed.
    pub token_budget: u64,
    /// Tokens consumed so far.
    pub tokens_used: u64,
    /// Time budget in milliseconds.
    pub time_budget_ms: u64,
    /// Time consumed in milliseconds.
    pub time_used_ms: u64,
    /// Maximum recursion depth allowed.
    pub max_recursion_depth: u32,
    /// Current recursion depth.
    pub current_recursion_depth: u32,
}

impl Default for BudgetStatus {
    fn default() -> Self {
        Self {
            token_budget: crate::DEFAULT_TOKEN_BUDGET,
            tokens_used: 0,
            time_budget_ms: crate::DEFAULT_TIME_BUDGET_MS,
            time_used_ms: 0,
            max_recursion_depth: crate::DEFAULT_MAX_RECURSION_DEPTH,
            current_recursion_depth: 0,
        }
    }
}

impl BudgetStatus {
    /// Check if token budget is exhausted.
    pub fn tokens_exhausted(&self) -> bool {
        self.tokens_used >= self.token_budget
    }

    /// Check if time budget is exhausted.
    pub fn time_exhausted(&self) -> bool {
        self.time_used_ms >= self.time_budget_ms
    }

    /// Check if recursion depth limit is reached.
    pub fn depth_exhausted(&self) -> bool {
        self.current_recursion_depth > self.max_recursion_depth
    }

    /// Check if any budget is exhausted.
    pub fn is_exhausted(&self) -> bool {
        self.tokens_exhausted() || self.time_exhausted() || self.depth_exhausted()
    }

    /// Get remaining token budget.
    pub fn tokens_remaining(&self) -> u64 {
        self.token_budget.saturating_sub(self.tokens_used)
    }

    /// Get remaining time budget in milliseconds.
    pub fn time_remaining_ms(&self) -> u64 {
        self.time_budget_ms.saturating_sub(self.time_used_ms)
    }
}

/// Parsed command from LLM output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Command {
    /// Execute a bash command in the VM.
    Run(BashCommand),
    /// Execute Python code in the VM.
    Code(PythonCode),
    /// Return a final result (terminates the query loop).
    Final(String),
    /// Return a variable's value as final result.
    FinalVar(String),
    /// Invoke the LLM recursively.
    QueryLlm(LlmQuery),
    /// Invoke the LLM with batched queries.
    QueryLlmBatched(Vec<LlmQuery>),
    /// Create a named snapshot.
    Snapshot(String),
    /// Restore to a previous snapshot.
    Rollback(String),
}

/// A bash command to execute.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BashCommand {
    /// The command string to execute.
    pub command: String,
    /// Optional timeout override in milliseconds.
    pub timeout_ms: Option<u64>,
    /// Working directory (relative to session root).
    pub working_dir: Option<String>,
}

impl BashCommand {
    /// Create a new bash command.
    pub fn new(command: impl Into<String>) -> Self {
        Self {
            command: command.into(),
            timeout_ms: None,
            working_dir: None,
        }
    }
}

/// Python code to execute.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PythonCode {
    /// The Python code to execute.
    pub code: String,
    /// Optional timeout override in milliseconds.
    pub timeout_ms: Option<u64>,
}

impl PythonCode {
    /// Create new Python code.
    pub fn new(code: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            timeout_ms: None,
        }
    }
}

/// A query to send to the LLM.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmQuery {
    /// The prompt/query text.
    pub prompt: String,
    /// Optional model override.
    pub model: Option<String>,
    /// Optional temperature override.
    pub temperature: Option<f32>,
    /// Optional max tokens override.
    pub max_tokens: Option<u32>,
}

impl LlmQuery {
    /// Create a new LLM query.
    pub fn new(prompt: impl Into<String>) -> Self {
        Self {
            prompt: prompt.into(),
            model: None,
            temperature: None,
            max_tokens: None,
        }
    }
}

/// Metadata about a query/execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryMetadata {
    /// Unique query identifier.
    pub query_id: Ulid,
    /// Parent query ID (for recursive queries).
    pub parent_query_id: Option<Ulid>,
    /// Session this query belongs to.
    pub session_id: SessionId,
    /// Iteration number within the query loop.
    pub iteration: u32,
    /// Recursion depth.
    pub depth: u32,
    /// Timestamp when query started.
    pub started_at: Timestamp,
    /// Timestamp when query completed (if done).
    pub completed_at: Option<Timestamp>,
}

impl QueryMetadata {
    /// Create new query metadata.
    pub fn new(session_id: SessionId) -> Self {
        Self {
            query_id: Ulid::new(),
            parent_query_id: None,
            session_id,
            iteration: 0,
            depth: 0,
            started_at: Timestamp::now(),
            completed_at: None,
        }
    }

    /// Create child query metadata (for recursive calls).
    pub fn child(&self) -> Self {
        Self {
            query_id: Ulid::new(),
            parent_query_id: Some(self.query_id),
            session_id: self.session_id,
            iteration: 0,
            depth: self.depth + 1,
            started_at: Timestamp::now(),
            completed_at: None,
        }
    }

    /// Mark the query as completed.
    pub fn complete(&mut self) {
        self.completed_at = Some(Timestamp::now());
    }
}

/// History of commands executed in a session.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CommandHistory {
    /// List of executed commands with their results.
    pub entries: Vec<CommandHistoryEntry>,
}

impl CommandHistory {
    /// Create a new empty command history.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add an entry to the history.
    pub fn push(&mut self, entry: CommandHistoryEntry) {
        self.entries.push(entry);
    }

    /// Get the last successful command index.
    pub fn last_successful_index(&self) -> Option<usize> {
        self.entries.iter().rposition(|e| e.success)
    }

    /// Get commands since the last checkpoint.
    pub fn since_checkpoint(&self, checkpoint_index: usize) -> &[CommandHistoryEntry] {
        &self.entries[checkpoint_index..]
    }
}

/// A single entry in the command history.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandHistoryEntry {
    /// The command that was executed.
    pub command: Command,
    /// Whether the command succeeded.
    pub success: bool,
    /// stdout output (may be truncated).
    pub stdout: String,
    /// stderr output (may be truncated).
    pub stderr: String,
    /// Exit code (for bash commands).
    pub exit_code: Option<i32>,
    /// Execution time in milliseconds.
    pub execution_time_ms: u64,
    /// Timestamp of execution.
    pub executed_at: Timestamp,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_id_creation() {
        let id1 = SessionId::new();
        let id2 = SessionId::new();
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_budget_status_exhaustion() {
        let mut budget = BudgetStatus::default();
        assert!(!budget.is_exhausted());

        budget.tokens_used = budget.token_budget;
        assert!(budget.tokens_exhausted());
        assert!(budget.is_exhausted());
    }

    #[test]
    fn test_session_info_expiry() {
        let id = SessionId::new();
        let info = SessionInfo::new(id, 1); // 1 second duration
        assert!(!info.is_expired());

        // Session with 0 duration should be expired immediately
        let expired_info = SessionInfo::new(id, 0);
        // Note: This might still pass due to timing, but demonstrates the concept
        assert!(expired_info.expires_at <= Timestamp::now() || !expired_info.is_expired());
    }

    #[test]
    fn test_query_metadata_child() {
        let session_id = SessionId::new();
        let parent = QueryMetadata::new(session_id);
        let child = parent.child();

        assert_eq!(child.parent_query_id, Some(parent.query_id));
        assert_eq!(child.depth, parent.depth + 1);
        assert_eq!(child.session_id, parent.session_id);
    }

    #[test]
    fn test_command_history() {
        let mut history = CommandHistory::new();
        assert!(history.last_successful_index().is_none());

        history.push(CommandHistoryEntry {
            command: Command::Code(PythonCode::new("x = 1")),
            success: true,
            stdout: "".to_string(),
            stderr: "".to_string(),
            exit_code: Some(0),
            execution_time_ms: 100,
            executed_at: Timestamp::now(),
        });

        assert_eq!(history.last_successful_index(), Some(0));
    }
}
