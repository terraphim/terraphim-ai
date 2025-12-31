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
    #[must_use]
    #[allow(dead_code)]
    pub fn new(id: String) -> Self {
        Self(id)
    }

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

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AgentType(String);

impl AgentType {
    #[must_use]
    #[allow(dead_code)]
    pub fn new(agent_type: String) -> Self {
        Self(agent_type)
    }

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

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MessageId(String);

impl MessageId {
    #[must_use]
    #[allow(dead_code)]
    pub fn new(id: String) -> Self {
        Self(id)
    }

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

/// Parse JSONL session entries from Claude Code
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionEntry {
    pub uuid: String,
    pub parent_uuid: Option<String>,
    pub session_id: String,
    pub timestamp: String,
    pub user_type: String,
    pub message: Message,
    #[serde(rename = "type")]
    pub entry_type: String,
    pub cwd: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Message {
    User {
        role: String,
        content: String,
    },
    Assistant {
        role: String,
        content: Vec<ContentBlock>,
        #[serde(default)]
        id: Option<String>,
        #[serde(default)]
        model: Option<String>,
    },
    ToolResult {
        role: String,
        content: Vec<ToolResultContent>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentBlock {
    Text {
        text: String,
    },
    ToolUse {
        id: String,
        name: String,
        input: serde_json::Value,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResultContent {
    pub tool_use_id: String,
    #[serde(rename = "type")]
    pub content_type: String,
    pub content: String,
}

/// Agent invocation tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentInvocation {
    pub timestamp: Timestamp,
    pub agent_type: String,
    pub task_description: String,
    pub prompt: String,
    pub files_modified: Vec<String>,
    pub tools_used: Vec<String>,
    pub duration_ms: Option<u64>,
    pub parent_message_id: String,
    pub session_id: String,
}

/// File operations extracted from tool uses
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileOperation {
    pub timestamp: Timestamp,
    pub operation: FileOpType,
    pub file_path: String,
    pub agent_context: Option<String>,
    pub session_id: String,
    pub message_id: String,
}

/// Tool invocation extracted from Bash commands
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolInvocation {
    pub timestamp: Timestamp,
    pub tool_name: String,
    pub tool_category: ToolCategory,
    pub command_line: String,
    pub arguments: Vec<String>,
    pub flags: HashMap<String, String>,
    pub exit_code: Option<i32>,
    pub agent_context: Option<String>,
    pub session_id: String,
    pub message_id: String,
}

/// Category of tool being used
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ToolCategory {
    PackageManager,
    BuildTool,
    Testing,
    Linting,
    Git,
    CloudDeploy,
    Database,
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
    pub tool_name: String,
    pub category: ToolCategory,
    pub total_invocations: u32,
    pub agents_using: Vec<String>,
    pub success_count: u32,
    pub failure_count: u32,
    pub first_seen: Timestamp,
    pub last_seen: Timestamp,
    pub command_patterns: Vec<String>,
    pub sessions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FileOpType {
    Read,
    Write,
    Edit,
    MultiEdit,
    Delete,
    Glob,
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
    pub session_id: String,
    pub project_path: String,
    pub start_time: Timestamp,
    pub end_time: Timestamp,
    pub duration_ms: u64,
    pub agents: Vec<AgentInvocation>,
    pub file_operations: Vec<FileOperation>,
    pub file_to_agents: IndexMap<String, Vec<AgentAttribution>>,
    pub agent_stats: IndexMap<String, AgentStatistics>,
    pub collaboration_patterns: Vec<CollaborationPattern>,
}

/// Attribution of a file to an agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentAttribution {
    pub agent_type: String,
    pub contribution_percent: f32,
    pub confidence_score: f32,
    pub operations: Vec<String>,
    pub first_interaction: Timestamp,
    pub last_interaction: Timestamp,
}

/// Statistics for an individual agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentStatistics {
    pub agent_type: String,
    pub total_invocations: u32,
    pub total_duration_ms: u64,
    pub files_touched: u32,
    pub tools_used: Vec<String>,
    pub first_seen: Timestamp,
    pub last_seen: Timestamp,
}

/// Collaboration patterns between agents
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollaborationPattern {
    pub pattern_type: String,
    pub agents: Vec<String>,
    pub description: String,
    pub frequency: u32,
    pub confidence: f32,
}

/// Correlation between agents and tools
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentToolCorrelation {
    pub agent_type: String,
    pub tool_name: String,
    pub usage_count: u32,
    pub success_rate: f32,
    pub average_invocations_per_session: f32,
}

/// Complete tool usage analysis
#[derive(Debug, Serialize, Deserialize)]
pub struct ToolAnalysis {
    pub session_id: String,
    pub total_tool_invocations: u32,
    pub tool_statistics: IndexMap<String, ToolStatistics>,
    pub agent_tool_correlations: Vec<AgentToolCorrelation>,
    pub tool_chains: Vec<ToolChain>,
    pub category_breakdown: IndexMap<ToolCategory, u32>,
}

/// Sequence of tools used together
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolChain {
    pub tools: Vec<String>,
    pub frequency: u32,
    pub average_time_between_ms: u64,
    pub typical_agent: Option<String>,
    pub success_rate: f32,
}

/// Configuration for the analyzer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyzerConfig {
    pub session_dirs: Vec<String>,
    pub agent_confidence_threshold: f32,
    pub file_attribution_window_ms: u64,
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
