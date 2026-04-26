//! Domain model types for session log analysis: sessions, messages, agents, and tool invocations.

use indexmap::IndexMap;
use jiff::Timestamp;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt::{self, Display};
use std::str::FromStr;

/// Newtype wrappers for better type safety
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SessionId(String);

impl SessionId {
    /// Create a new `SessionId` from a string.
    #[must_use]
    #[allow(dead_code)]
    pub fn new(id: String) -> Self {
        Self(id)
    }

    /// Return the inner string slice.
    #[must_use]
    #[allow(dead_code)]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Display for SessionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<String> for SessionId {
    fn from(id: String) -> Self {
        Self(id)
    }
}

impl From<&str> for SessionId {
    fn from(id: &str) -> Self {
        Self(id.to_string())
    }
}

/// Newtype wrapper for agent type names (e.g. `"developer"`, `"rust-developer"`).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AgentType(String);

impl AgentType {
    /// Create a new `AgentType` from a string.
    #[must_use]
    #[allow(dead_code)]
    pub fn new(agent_type: String) -> Self {
        Self(agent_type)
    }

    /// Return the inner string slice.
    #[must_use]
    #[allow(dead_code)]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Display for AgentType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<String> for AgentType {
    fn from(agent_type: String) -> Self {
        Self(agent_type)
    }
}

impl From<&str> for AgentType {
    fn from(agent_type: &str) -> Self {
        Self(agent_type.to_string())
    }
}

/// Newtype wrapper for message UUIDs within a session.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MessageId(String);

impl MessageId {
    /// Create a new `MessageId` from a string.
    #[must_use]
    #[allow(dead_code)]
    pub fn new(id: String) -> Self {
        Self(id)
    }

    /// Return the inner string slice.
    #[must_use]
    #[allow(dead_code)]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Display for MessageId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<String> for MessageId {
    fn from(id: String) -> Self {
        Self(id)
    }
}

impl From<&str> for MessageId {
    fn from(id: &str) -> Self {
        Self(id.to_string())
    }
}

impl AsRef<str> for SessionId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl AsRef<str> for AgentType {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl AsRef<str> for MessageId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

/// A single JSONL entry from a Claude Code session log.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionEntry {
    /// Unique identifier for this entry.
    pub uuid: String,
    /// UUID of the parent entry, if any.
    pub parent_uuid: Option<String>,
    /// Identifier of the session this entry belongs to.
    pub session_id: String,
    /// ISO-8601 timestamp string.
    pub timestamp: String,
    /// Source of the entry: `"human"`, `"assistant"`, or `"tool"`.
    pub user_type: String,
    /// Parsed message payload.
    pub message: Message,
    #[serde(rename = "type")]
    /// Entry type discriminator (mirrors `user_type` for deserialization).
    pub entry_type: String,
    /// Working directory at the time the entry was recorded.
    pub cwd: Option<String>,
}

/// Message payload inside a [`SessionEntry`], tagged by role.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Message {
    /// A message authored by the human user.
    User {
        /// Always `"user"`.
        role: String,
        /// Plain-text content of the user turn.
        content: String,
    },
    /// A message authored by the assistant.
    Assistant {
        /// Always `"assistant"`.
        role: String,
        /// Structured content blocks (text and tool-use).
        content: Vec<ContentBlock>,
        #[serde(default)]
        /// Message ID assigned by the model API.
        id: Option<String>,
        #[serde(default)]
        /// Model identifier that generated this turn.
        model: Option<String>,
    },
    /// The result of a tool invocation returned to the assistant.
    ToolResult {
        /// Always `"tool"`.
        role: String,
        /// Tool result content blocks.
        content: Vec<ToolResultContent>,
    },
}

/// A structured block within an assistant message: plain text or a tool invocation.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentBlock {
    /// Plain text produced by the assistant.
    Text {
        /// The text content.
        text: String,
    },
    /// A request to invoke a tool.
    ToolUse {
        /// Tool call identifier.
        id: String,
        /// Name of the tool being invoked.
        name: String,
        /// JSON input arguments for the tool.
        input: serde_json::Value,
    },
}

/// Content returned by a tool invocation, linking back to the originating tool-use ID.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResultContent {
    /// Identifier of the tool-use request this result corresponds to.
    pub tool_use_id: String,
    #[serde(rename = "type")]
    /// MIME or semantic type of the result content.
    pub content_type: String,
    /// The raw result text returned by the tool.
    pub content: String,
}

/// Agent invocation tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentInvocation {
    /// Timestamp when the agent was invoked.
    pub timestamp: Timestamp,
    /// Type or name of the agent (e.g. `"developer"`, `"rust-developer"`).
    pub agent_type: String,
    /// Human-readable description of the task the agent performed.
    pub task_description: String,
    /// Full prompt sent to the agent.
    pub prompt: String,
    /// List of file paths modified during this invocation.
    pub files_modified: Vec<String>,
    /// Names of tools used during this invocation.
    pub tools_used: Vec<String>,
    /// Wall-clock duration of the invocation in milliseconds, if known.
    pub duration_ms: Option<u64>,
    /// Message ID of the parent turn that triggered this agent.
    pub parent_message_id: String,
    /// Session this invocation belongs to.
    pub session_id: String,
}

/// File operations extracted from tool uses
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileOperation {
    /// Timestamp when the file operation occurred.
    pub timestamp: Timestamp,
    /// Kind of file operation performed.
    pub operation: FileOpType,
    /// Absolute or relative path of the file that was operated on.
    pub file_path: String,
    /// Agent type that performed the operation, if determinable.
    pub agent_context: Option<String>,
    /// Session this operation belongs to.
    pub session_id: String,
    /// Message in which this operation was recorded.
    pub message_id: String,
}

/// Tool invocation extracted from Bash commands
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolInvocation {
    /// Timestamp when the tool was invoked.
    pub timestamp: Timestamp,
    /// Name of the tool or executable invoked.
    pub tool_name: String,
    /// High-level category the tool belongs to.
    pub tool_category: ToolCategory,
    /// Full command line string as seen in the session log.
    pub command_line: String,
    /// Positional arguments passed to the tool.
    pub arguments: Vec<String>,
    /// Named flags passed to the tool, keyed by flag name.
    pub flags: HashMap<String, String>,
    /// Exit code returned by the process, if captured.
    pub exit_code: Option<i32>,
    /// Agent type that issued the invocation, if determinable.
    pub agent_context: Option<String>,
    /// Session this invocation belongs to.
    pub session_id: String,
    /// Message in which this invocation was recorded.
    pub message_id: String,
}

/// Category of tool being used
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ToolCategory {
    /// Package manager tools such as cargo, npm, or bun.
    PackageManager,
    /// Build tools such as cargo build or make.
    BuildTool,
    /// Testing tools such as cargo test or jest.
    Testing,
    /// Linting and formatting tools such as clippy or eslint.
    Linting,
    /// Version-control tools such as git.
    Git,
    /// Cloud deployment tools such as kubectl or terraform.
    CloudDeploy,
    /// Database tools such as psql or sqlite3.
    Database,
    /// Any tool that does not fit the named categories.
    Other(String),
}

impl ToolCategory {
    /// Parse a string category into ToolCategory
    /// Used in parser for converting string categories
    #[must_use]
    #[allow(dead_code)]
    pub fn from_string(s: &str) -> Self {
        match s {
            "PackageManager" => ToolCategory::PackageManager,
            "BuildTool" => ToolCategory::BuildTool,
            "Testing" => ToolCategory::Testing,
            "Linting" => ToolCategory::Linting,
            "Git" => ToolCategory::Git,
            "CloudDeploy" => ToolCategory::CloudDeploy,
            "Database" => ToolCategory::Database,
            _ => ToolCategory::Other(s.to_string()),
        }
    }
}

/// Statistics for a specific tool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolStatistics {
    /// Name of the tool these statistics describe.
    pub tool_name: String,
    /// Category the tool belongs to.
    pub category: ToolCategory,
    /// Total number of times the tool was invoked across all sessions.
    pub total_invocations: u32,
    /// Agent types that used this tool.
    pub agents_using: Vec<String>,
    /// Number of invocations that exited with code 0.
    pub success_count: u32,
    /// Number of invocations that exited with a non-zero code.
    pub failure_count: u32,
    /// Timestamp of the earliest recorded invocation.
    pub first_seen: Timestamp,
    /// Timestamp of the most recent recorded invocation.
    pub last_seen: Timestamp,
    /// Representative command-line patterns observed for this tool.
    pub command_patterns: Vec<String>,
    /// Session IDs in which this tool was used.
    pub sessions: Vec<String>,
}

/// The kind of file system operation performed via a tool call.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FileOpType {
    /// File was read from disk.
    Read,
    /// File was written (created or overwritten).
    Write,
    /// File was partially edited in-place.
    Edit,
    /// Multiple edits were applied to a file in a single call.
    MultiEdit,
    /// File was deleted.
    Delete,
    /// A glob pattern was used to enumerate matching paths.
    Glob,
    /// File contents were searched with a pattern.
    Grep,
}

impl FromStr for FileOpType {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Read" => Ok(FileOpType::Read),
            "Write" => Ok(FileOpType::Write),
            "Edit" => Ok(FileOpType::Edit),
            "MultiEdit" => Ok(FileOpType::MultiEdit),
            "Delete" => Ok(FileOpType::Delete),
            "Glob" => Ok(FileOpType::Glob),
            "Grep" => Ok(FileOpType::Grep),
            _ => Err(anyhow::anyhow!("Unknown file operation type: {s}")),
        }
    }
}

/// Analysis results for a session
#[derive(Debug, Serialize, Deserialize)]
pub struct SessionAnalysis {
    /// Unique identifier of the analysed session.
    pub session_id: String,
    /// Filesystem path of the project directory for the session.
    pub project_path: String,
    /// Timestamp of the first event in the session.
    pub start_time: Timestamp,
    /// Timestamp of the last event in the session.
    pub end_time: Timestamp,
    /// Total session duration in milliseconds.
    pub duration_ms: u64,
    /// All agent invocations recorded during the session.
    pub agents: Vec<AgentInvocation>,
    /// All file operations recorded during the session.
    pub file_operations: Vec<FileOperation>,
    /// Mapping from file path to the agents attributed to that file.
    pub file_to_agents: IndexMap<String, Vec<AgentAttribution>>,
    /// Per-agent statistics keyed by agent type name.
    pub agent_stats: IndexMap<String, AgentStatistics>,
    /// Detected collaboration patterns between agents.
    pub collaboration_patterns: Vec<CollaborationPattern>,
}

/// Attribution of a file to an agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentAttribution {
    /// Type or name of the agent attributed to this file.
    pub agent_type: String,
    /// Percentage of file changes attributed to this agent (0.0–100.0).
    pub contribution_percent: f32,
    /// Confidence in the attribution estimate (0.0–1.0).
    pub confidence_score: f32,
    /// Names of file operations this agent performed on the file.
    pub operations: Vec<String>,
    /// Timestamp of the agent's earliest interaction with the file.
    pub first_interaction: Timestamp,
    /// Timestamp of the agent's most recent interaction with the file.
    pub last_interaction: Timestamp,
}

/// Statistics for an individual agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentStatistics {
    /// Type or name of the agent these statistics describe.
    pub agent_type: String,
    /// Total number of times this agent was invoked.
    pub total_invocations: u32,
    /// Cumulative wall-clock time spent in this agent across all invocations (ms).
    pub total_duration_ms: u64,
    /// Number of distinct files this agent interacted with.
    pub files_touched: u32,
    /// Names of tools this agent used.
    pub tools_used: Vec<String>,
    /// Timestamp of the agent's earliest recorded invocation.
    pub first_seen: Timestamp,
    /// Timestamp of the agent's most recent recorded invocation.
    pub last_seen: Timestamp,
}

/// Collaboration patterns between agents
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollaborationPattern {
    /// Short identifier for the type of collaboration pattern detected.
    pub pattern_type: String,
    /// Agent types involved in this collaboration pattern.
    pub agents: Vec<String>,
    /// Human-readable explanation of the pattern.
    pub description: String,
    /// Number of times this pattern was observed.
    pub frequency: u32,
    /// Confidence score for the pattern detection (0.0–1.0).
    pub confidence: f32,
}

/// Correlation between agents and tools
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentToolCorrelation {
    /// Agent type involved in the correlation.
    pub agent_type: String,
    /// Tool name involved in the correlation.
    pub tool_name: String,
    /// Number of times this agent used this tool.
    pub usage_count: u32,
    /// Fraction of invocations that succeeded (0.0–1.0).
    pub success_rate: f32,
    /// Mean number of invocations per session for this agent-tool pair.
    pub average_invocations_per_session: f32,
}

/// Complete tool usage analysis
#[derive(Debug, Serialize, Deserialize)]
pub struct ToolAnalysis {
    /// Identifier of the session this analysis covers.
    pub session_id: String,
    /// Total number of tool invocations across the session.
    pub total_tool_invocations: u32,
    /// Per-tool statistics keyed by tool name.
    pub tool_statistics: IndexMap<String, ToolStatistics>,
    /// Observed correlations between agent types and specific tools.
    pub agent_tool_correlations: Vec<AgentToolCorrelation>,
    /// Frequently observed sequences of tool calls.
    pub tool_chains: Vec<ToolChain>,
    /// Invocation counts aggregated by tool category.
    pub category_breakdown: IndexMap<ToolCategory, u32>,
}

/// Sequence of tools used together
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolChain {
    /// Ordered list of tool names forming this chain.
    pub tools: Vec<String>,
    /// Number of times this exact chain was observed.
    pub frequency: u32,
    /// Mean elapsed time between consecutive tool calls in the chain (ms).
    pub average_time_between_ms: u64,
    /// Agent type most commonly associated with this chain, if determinable.
    pub typical_agent: Option<String>,
    /// Fraction of chain executions that completed without error (0.0–1.0).
    pub success_rate: f32,
}

/// Configuration for the analyzer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyzerConfig {
    /// Filesystem directories to scan for session JSONL files.
    pub session_dirs: Vec<String>,
    /// Minimum confidence score required to attribute an action to an agent.
    pub agent_confidence_threshold: f32,
    /// Time window (ms) within which file operations are attributed to the preceding agent.
    pub file_attribution_window_ms: u64,
    /// Glob patterns for paths to exclude from analysis.
    pub exclude_patterns: Vec<String>,
}

impl Default for AnalyzerConfig {
    fn default() -> Self {
        Self {
            session_dirs: vec![],
            agent_confidence_threshold: 0.7,
            file_attribution_window_ms: 300_000, // 5 minutes
            exclude_patterns: vec![
                "node_modules/".to_string(),
                "target/".to_string(),
                ".git/".to_string(),
            ],
        }
    }
}

/// Parse an ISO 8601 timestamp string into a `jiff::Timestamp`
///
/// # Errors
///
/// Returns an error if the timestamp string is malformed or cannot be parsed
pub fn parse_timestamp(timestamp_str: &str) -> Result<Timestamp, anyhow::Error> {
    // Handle ISO 8601 timestamps from Claude session logs
    Timestamp::from_str(timestamp_str)
        .map_err(|e| anyhow::anyhow!("Failed to parse timestamp '{timestamp_str}': {e}"))
}

/// Helper to extract file path from various tool inputs
#[must_use]
pub fn extract_file_path(input: &serde_json::Value) -> Option<String> {
    // Try different field names that might contain file paths
    for field in &["file_path", "path", "pattern"] {
        if let Some(path) = input.get(field).and_then(|v| v.as_str()) {
            return Some(path.to_string());
        }
    }

    // For MultiEdit, check the edits array
    if let Some(edits) = input.get("edits").and_then(|v| v.as_array()) {
        if !edits.is_empty() {
            if let Some(file_path) = input.get("file_path").and_then(|v| v.as_str()) {
                return Some(file_path.to_string());
            }
        }
    }

    None
}

/// Agent type utilities
/// Used in integration tests and public API
#[allow(dead_code)]
#[must_use]
pub fn normalize_agent_name(agent_type: &str) -> String {
    agent_type.to_lowercase().replace(['-', ' '], "_")
}

/// Used in integration tests and public API
#[allow(dead_code)]
#[must_use]
pub fn get_agent_category(agent_type: &str) -> &'static str {
    match agent_type {
        "architect" | "backend-architect" | "frontend-developer" => "architecture",
        "developer" | "rapid-prototyper" => "development",
        "rust-performance-expert" | "rust-code-reviewer" => "rust-expert",
        "debugger" | "test-writer-fixer" => "testing",
        "technical-writer" => "documentation",
        "devops-automator" | "overseer" => "operations",
        "general-purpose" => "general",
        _ => "other",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_timestamp() {
        let timestamp_str = "2025-10-01T09:05:21.902Z";
        let result = parse_timestamp(timestamp_str);
        assert!(result.is_ok());
    }

    #[test]
    fn test_newtype_wrappers() {
        // Test SessionId
        let session_id = SessionId::new("test-session".to_string());
        assert_eq!(session_id.as_str(), "test-session");
        assert_eq!(session_id.to_string(), "test-session");
        assert_eq!(session_id.as_ref(), "test-session");

        let session_id_from_str: SessionId = "another-session".into();
        assert_eq!(session_id_from_str.as_str(), "another-session");

        // Test AgentType
        let agent_type = AgentType::new("architect".to_string());
        assert_eq!(agent_type.as_str(), "architect");
        assert_eq!(agent_type.to_string(), "architect");

        // Test MessageId
        let message_id = MessageId::new("msg-123".to_string());
        assert_eq!(message_id.as_str(), "msg-123");
        assert_eq!(message_id.to_string(), "msg-123");
    }

    #[test]
    fn test_extract_file_path() {
        let input = serde_json::json!({
            "file_path": "/path/to/file.rs",
            "description": "Edit file"
        });

        let path = extract_file_path(&input);
        assert_eq!(path, Some("/path/to/file.rs".to_string()));
    }

    #[test]
    fn test_normalize_agent_name() {
        assert_eq!(
            normalize_agent_name("rust-performance-expert"),
            "rust_performance_expert"
        );
        assert_eq!(
            normalize_agent_name("backend-architect"),
            "backend_architect"
        );
    }

    mod proptest_tests {
        use super::*;
        use proptest::prelude::*;

        proptest! {
            #[test]
            fn test_normalize_agent_name_properties(
                input in "[a-zA-Z0-9 -]{1,50}"
            ) {
                let result = normalize_agent_name(&input);

                // Property 1: Result should only contain lowercase letters, numbers, and underscores
                prop_assert!(result.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_'));

                // Property 2: Result should not be empty if input was not empty
                if !input.trim().is_empty() {
                    prop_assert!(!result.is_empty());
                }
            }

            #[test]
            fn test_parse_timestamp_properties(
                year in 2020u16..2030,
                month in 1u8..=12,
                day in 1u8..=28, // Safe range to avoid month-specific issues
                hour in 0u8..=23,
                minute in 0u8..=59,
                second in 0u8..=59,
                millis in 0u16..1000
            ) {
                let timestamp_str = format!(
                    "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}.{:03}Z",
                    year, month, day, hour, minute, second, millis
                );

                let result = parse_timestamp(&timestamp_str);

                // Property: Valid ISO 8601 timestamps should always parse successfully
                prop_assert!(result.is_ok(), "Failed to parse valid timestamp: {}", timestamp_str);

                if let Ok(parsed) = result {
                    // Property: Parsed timestamp should roundtrip correctly
                    let reformatted = parsed.to_string();
                    prop_assert!(reformatted.starts_with(&year.to_string()));
                }
            }

            #[test]
            fn test_extract_file_path_properties(
                file_path in r"[a-zA-Z0-9_./\-]{1,100}"
            ) {
                let input = serde_json::json!({
                    "file_path": file_path
                });

                let result = extract_file_path(&input);

                // Property: If file_path field exists, it should be extracted
                prop_assert_eq!(result, Some(file_path.clone()));

                // Test with different field names
                let input_path = serde_json::json!({
                    "path": file_path
                });
                let result_path = extract_file_path(&input_path);
                prop_assert_eq!(result_path, Some(file_path.clone()));
            }

            #[test]
            fn test_newtype_wrapper_roundtrip(
                session_id in "[a-zA-Z0-9-]{10,50}",
                agent_type in "[a-zA-Z0-9-_]{3,30}",
                message_id in "[a-zA-Z0-9-]{10,50}"
            ) {
                // Test SessionId roundtrip
                let session = SessionId::new(session_id.clone());
                prop_assert_eq!(session.as_str(), &session_id);
                prop_assert_eq!(session.to_string(), session_id);

                // Test AgentType roundtrip
                let agent = AgentType::new(agent_type.clone());
                prop_assert_eq!(agent.as_str(), &agent_type);
                prop_assert_eq!(agent.to_string(), agent_type);

                // Test MessageId roundtrip
                let message = MessageId::new(message_id.clone());
                prop_assert_eq!(message.as_str(), &message_id);
                prop_assert_eq!(message.to_string(), message_id);
            }
        }
    }
}
