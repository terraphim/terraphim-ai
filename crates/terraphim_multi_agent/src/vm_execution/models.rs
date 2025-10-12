use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Configuration for VM execution functionality
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VmExecutionConfig {
    /// Whether VM execution is enabled for this agent
    pub enabled: bool,
    /// Base URL for fcctl-web API
    pub api_base_url: String,
    /// Number of VMs to keep in pool
    pub vm_pool_size: u32,
    /// Default VM type to use
    pub default_vm_type: String,
    /// Execution timeout in milliseconds
    pub execution_timeout_ms: u64,
    /// Allowed programming languages
    pub allowed_languages: Vec<String>,
    /// Whether to auto-provision VMs when needed
    pub auto_provision: bool,
    /// Whether to validate code before execution
    pub code_validation: bool,
    /// Maximum code length in characters
    pub max_code_length: usize,
    /// History tracking configuration
    #[serde(default)]
    pub history: HistoryConfig,
}

/// Configuration for VM execution history tracking and rollback
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryConfig {
    /// Whether history tracking is enabled
    pub enabled: bool,
    /// Create snapshot before each command execution
    pub snapshot_on_execution: bool,
    /// Create snapshot only when command fails
    pub snapshot_on_failure: bool,
    /// Automatically rollback to last successful state on failure
    pub auto_rollback_on_failure: bool,
    /// Maximum number of history entries to keep per VM
    pub max_history_entries: usize,
    /// Persist history to database
    pub persist_history: bool,
    /// Integration mode: "http" for HTTP/WebSocket, "direct" for fcctl-repl Session
    pub integration_mode: String,
}

impl Default for HistoryConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            snapshot_on_execution: false,
            snapshot_on_failure: true,
            auto_rollback_on_failure: false,
            max_history_entries: 100,
            persist_history: true,
            integration_mode: "http".to_string(),
        }
    }
}

impl Default for VmExecutionConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            api_base_url: "http://localhost:8080".to_string(),
            vm_pool_size: 1,
            default_vm_type: "focal-optimized".to_string(),
            execution_timeout_ms: 30000,
            allowed_languages: vec![
                "python".to_string(),
                "javascript".to_string(),
                "bash".to_string(),
                "rust".to_string(),
            ],
            auto_provision: true,
            code_validation: true,
            max_code_length: 10000,
            history: HistoryConfig::default(),
        }
    }
}

/// A block of code extracted from LLM response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeBlock {
    /// Programming language
    pub language: String,
    /// The actual code content
    pub code: String,
    /// Confidence that this should be executed (0.0-1.0)
    pub execution_confidence: f64,
    /// Start position in original text
    pub start_pos: usize,
    /// End position in original text
    pub end_pos: usize,
    /// Additional metadata
    pub metadata: Option<serde_json::Value>,
}

/// Request to execute code in VM
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VmExecuteRequest {
    /// Agent ID making the request
    pub agent_id: String,
    /// Programming language
    pub language: String,
    /// Code to execute
    pub code: String,
    /// Optional VM ID (will auto-provision if None)
    pub vm_id: Option<String>,
    /// Required dependencies/packages
    pub requirements: Vec<String>,
    /// Execution timeout in seconds
    pub timeout_seconds: Option<u64>,
    /// Working directory for execution
    pub working_dir: Option<String>,
    /// Execution metadata
    pub metadata: Option<serde_json::Value>,
}

/// Response from VM code execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VmExecuteResponse {
    /// Unique execution ID
    pub execution_id: String,
    /// VM ID where code was executed
    pub vm_id: String,
    /// Exit code of the execution
    pub exit_code: i32,
    /// Standard output
    pub stdout: String,
    /// Standard error
    pub stderr: String,
    /// Execution duration in milliseconds
    pub duration_ms: u64,
    /// Timestamp when execution started
    pub started_at: DateTime<Utc>,
    /// Timestamp when execution completed
    pub completed_at: DateTime<Utc>,
    /// Any error that occurred
    pub error: Option<String>,
}

/// Request to parse LLM response and potentially execute code
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParseExecuteRequest {
    /// Agent ID making the request
    pub agent_id: String,
    /// LLM response text to parse
    pub llm_response: String,
    /// Whether to automatically execute detected code
    pub auto_execute: bool,
    /// Minimum confidence threshold for auto-execution
    pub auto_execute_threshold: f64,
    /// VM configuration override
    pub vm_config: Option<serde_json::Value>,
}

/// Response from parse-execute operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParseExecuteResponse {
    /// Extracted code blocks
    pub code_blocks: Vec<CodeBlock>,
    /// Execution results (if auto_execute was true)
    pub execution_results: Vec<VmExecuteResponse>,
    /// Any parsing or execution errors
    pub errors: Vec<String>,
}

/// VM instance information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VmInstance {
    /// VM ID
    pub id: String,
    /// VM name
    pub name: String,
    /// VM type
    pub vm_type: String,
    /// Current status
    pub status: String,
    /// IP address
    pub ip_address: Option<String>,
    /// Created timestamp
    pub created_at: DateTime<Utc>,
    /// Last activity timestamp
    pub last_activity: Option<DateTime<Utc>>,
}

/// Available VMs for an agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VmPoolResponse {
    /// Agent ID
    pub agent_id: String,
    /// Available VMs
    pub available_vms: Vec<VmInstance>,
    /// VMs currently in use
    pub in_use_vms: Vec<VmInstance>,
    /// Pool configuration
    pub pool_config: VmExecutionConfig,
}

/// Execution intent detected in text
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionIntent {
    /// Confidence that user wants code executed (0.0-1.0)
    pub confidence: f64,
    /// Keywords that triggered detection
    pub trigger_keywords: Vec<String>,
    /// Context clues
    pub context_clues: Vec<String>,
    /// Suggested action
    pub suggested_action: String,
}

/// Language-specific execution settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LanguageConfig {
    /// Language name
    pub name: String,
    /// File extension
    pub extension: String,
    /// Command to execute files
    pub execute_command: String,
    /// Common packages/dependencies
    pub common_packages: Vec<String>,
    /// Security restrictions
    pub restrictions: Vec<String>,
    /// Timeout multiplier (relative to base timeout)
    pub timeout_multiplier: f64,
}

impl Default for LanguageConfig {
    fn default() -> Self {
        Self {
            name: "unknown".to_string(),
            extension: "txt".to_string(),
            execute_command: "cat".to_string(),
            common_packages: vec![],
            restrictions: vec![],
            timeout_multiplier: 1.0,
        }
    }
}

/// Command history entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandHistoryEntry {
    /// Unique entry ID
    pub id: String,
    /// VM ID
    pub vm_id: String,
    /// Agent ID that executed this command
    pub agent_id: String,
    /// Command that was executed
    pub command: String,
    /// Programming language
    pub language: String,
    /// Snapshot ID created before/after execution
    pub snapshot_id: Option<String>,
    /// Whether command succeeded
    pub success: bool,
    /// Exit code
    pub exit_code: i32,
    /// Standard output
    pub stdout: String,
    /// Standard error
    pub stderr: String,
    /// Execution timestamp
    pub executed_at: DateTime<Utc>,
    /// Execution duration in milliseconds
    pub duration_ms: u64,
}

/// Request to query command history
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryQueryRequest {
    /// VM ID to query history for
    pub vm_id: String,
    /// Optional agent ID filter
    pub agent_id: Option<String>,
    /// Maximum number of entries to return
    pub limit: Option<usize>,
    /// Only return failed commands
    pub failures_only: bool,
    /// Start date filter
    pub start_date: Option<DateTime<Utc>>,
    /// End date filter
    pub end_date: Option<DateTime<Utc>>,
}

/// Response containing command history
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryQueryResponse {
    /// VM ID
    pub vm_id: String,
    /// History entries
    pub entries: Vec<CommandHistoryEntry>,
    /// Total number of entries matching filter
    pub total: usize,
}

/// Request to rollback VM to a previous state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RollbackRequest {
    /// VM ID to rollback
    pub vm_id: String,
    /// Snapshot ID to rollback to
    pub snapshot_id: String,
    /// Whether to create a snapshot before rollback
    pub create_pre_rollback_snapshot: bool,
}

/// Response from rollback operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RollbackResponse {
    /// VM ID
    pub vm_id: String,
    /// Snapshot ID that was restored
    pub restored_snapshot_id: String,
    /// Snapshot ID created before rollback (if requested)
    pub pre_rollback_snapshot_id: Option<String>,
    /// Timestamp of rollback
    pub rolled_back_at: DateTime<Utc>,
    /// Success status
    pub success: bool,
    /// Error message if failed
    pub error: Option<String>,
}

/// Error types for VM execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum VmExecutionError {
    /// VM not available or not found
    VmNotAvailable(String),
    /// Code validation failed
    ValidationFailed(String),
    /// Execution timeout
    Timeout(u64),
    /// Language not supported
    UnsupportedLanguage(String),
    /// Network/API error
    ApiError(String),
    /// Internal error
    Internal(String),
    /// History operation failed
    HistoryError(String),
    /// Snapshot not found
    SnapshotNotFound(String),
    /// Rollback failed
    RollbackFailed(String),
    /// Session not found
    SessionNotFound(String),
    /// Connection error
    ConnectionError(String),
    /// Configuration error
    ConfigError(String),
    /// Execution failed
    ExecutionFailed(String),
    /// Snapshot creation failed
    SnapshotFailed(String),
}

impl std::fmt::Display for VmExecutionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VmExecutionError::VmNotAvailable(vm_id) => write!(f, "VM not available: {}", vm_id),
            VmExecutionError::ValidationFailed(reason) => {
                write!(f, "Code validation failed: {}", reason)
            }
            VmExecutionError::Timeout(duration) => {
                write!(f, "Execution timeout after {}ms", duration)
            }
            VmExecutionError::UnsupportedLanguage(lang) => {
                write!(f, "Language not supported: {}", lang)
            }
            VmExecutionError::ApiError(msg) => write!(f, "API error: {}", msg),
            VmExecutionError::Internal(msg) => write!(f, "Internal error: {}", msg),
            VmExecutionError::HistoryError(msg) => write!(f, "History error: {}", msg),
            VmExecutionError::SnapshotNotFound(id) => write!(f, "Snapshot not found: {}", id),
            VmExecutionError::RollbackFailed(msg) => write!(f, "Rollback failed: {}", msg),
            VmExecutionError::SessionNotFound(id) => write!(f, "Session not found: {}", id),
            VmExecutionError::ConnectionError(msg) => write!(f, "Connection error: {}", msg),
            VmExecutionError::ConfigError(msg) => write!(f, "Configuration error: {}", msg),
            VmExecutionError::ExecutionFailed(msg) => write!(f, "Execution failed: {}", msg),
            VmExecutionError::SnapshotFailed(msg) => write!(f, "Snapshot creation failed: {}", msg),
        }
    }
}

impl std::error::Error for VmExecutionError {}
