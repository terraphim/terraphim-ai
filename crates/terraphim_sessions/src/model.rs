//! Core data models for session management
//!
//! These models provide a unified representation of sessions and messages
//! from various AI coding assistants.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[cfg(feature = "enrichment")]
use crate::enrichment::SessionConcepts;

/// Unique identifier for a session
pub type SessionId = String;

/// Unique identifier for a message
pub type MessageId = String;

/// The role of a message participant
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    /// User/human message
    User,
    /// AI assistant message
    Assistant,
    /// System message
    System,
    /// Tool result message
    Tool,
    /// Unknown or other role
    #[serde(other)]
    Other,
}

impl From<&str> for MessageRole {
    fn from(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "user" | "human" => Self::User,
            "assistant" | "ai" | "bot" | "model" => Self::Assistant,
            "system" => Self::System,
            "tool" | "tool_result" => Self::Tool,
            _ => Self::Other,
        }
    }
}

impl std::fmt::Display for MessageRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::User => write!(f, "user"),
            Self::Assistant => write!(f, "assistant"),
            Self::System => write!(f, "system"),
            Self::Tool => write!(f, "tool"),
            Self::Other => write!(f, "other"),
        }
    }
}

/// Content block within a message
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentBlock {
    /// Plain text content
    Text { text: String },
    /// Tool use request
    ToolUse {
        id: String,
        name: String,
        input: serde_json::Value,
    },
    /// Tool result
    ToolResult {
        tool_use_id: String,
        content: String,
        is_error: bool,
    },
    /// Image content
    Image { source: String },
}

impl ContentBlock {
    /// Extract text content from block
    pub fn as_text(&self) -> Option<&str> {
        match self {
            Self::Text { text } => Some(text),
            _ => None,
        }
    }
}

/// A message within a session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    /// Message index within the session
    pub idx: usize,
    /// Message role
    pub role: MessageRole,
    /// Author identifier (model name, user, etc.)
    pub author: Option<String>,
    /// Message content (text representation)
    pub content: String,
    /// Structured content blocks (if available)
    #[serde(default)]
    pub blocks: Vec<ContentBlock>,
    /// Creation timestamp
    pub created_at: Option<jiff::Timestamp>,
    /// Additional metadata
    #[serde(default)]
    pub extra: serde_json::Value,
}

impl Message {
    /// Create a new text message
    pub fn text(idx: usize, role: MessageRole, content: impl Into<String>) -> Self {
        let content = content.into();
        Self {
            idx,
            role,
            author: None,
            content: content.clone(),
            blocks: vec![ContentBlock::Text { text: content }],
            created_at: None,
            extra: serde_json::Value::Null,
        }
    }

    /// Check if message contains tool usage
    pub fn has_tool_use(&self) -> bool {
        self.blocks
            .iter()
            .any(|b| matches!(b, ContentBlock::ToolUse { .. }))
    }

    /// Get tool names used in this message
    pub fn tool_names(&self) -> Vec<&str> {
        self.blocks
            .iter()
            .filter_map(|b| match b {
                ContentBlock::ToolUse { name, .. } => Some(name.as_str()),
                _ => None,
            })
            .collect()
    }
}

/// Metadata about a session
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SessionMetadata {
    /// Project path or working directory
    pub project_path: Option<String>,
    /// Model used in session
    pub model: Option<String>,
    /// Custom tags
    #[serde(default)]
    pub tags: Vec<String>,
    /// Additional fields
    #[serde(flatten)]
    pub extra: serde_json::Value,
    /// Knowledge graph enrichment results
    #[cfg(feature = "enrichment")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enrichment: Option<SessionConcepts>,
}

/// A coding assistant session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    /// Unique session identifier
    pub id: SessionId,
    /// Source connector ID (e.g., "claude-code", "cursor")
    pub source: String,
    /// External ID from the source system
    pub external_id: String,
    /// Session title or description
    pub title: Option<String>,
    /// Path to source file/database
    pub source_path: PathBuf,
    /// Session start time
    pub started_at: Option<jiff::Timestamp>,
    /// Session end time
    pub ended_at: Option<jiff::Timestamp>,
    /// Messages in the session
    pub messages: Vec<Message>,
    /// Session metadata
    pub metadata: SessionMetadata,
}

impl Session {
    /// Calculate session duration in milliseconds
    pub fn duration_ms(&self) -> Option<i64> {
        match (self.started_at, self.ended_at) {
            (Some(start), Some(end)) => {
                let span = end - start;
                span.total(jiff::Unit::Millisecond).ok().map(|ms| ms as i64)
            }
            _ => None,
        }
    }

    /// Get message count
    pub fn message_count(&self) -> usize {
        self.messages.len()
    }

    /// Get user message count
    pub fn user_message_count(&self) -> usize {
        self.messages
            .iter()
            .filter(|m| m.role == MessageRole::User)
            .count()
    }

    /// Get assistant message count
    pub fn assistant_message_count(&self) -> usize {
        self.messages
            .iter()
            .filter(|m| m.role == MessageRole::Assistant)
            .count()
    }

    /// Get all unique tool names used in session
    pub fn tools_used(&self) -> Vec<String> {
        let mut tools: std::collections::HashSet<String> = std::collections::HashSet::new();
        for msg in &self.messages {
            for name in msg.tool_names() {
                tools.insert(name.to_string());
            }
        }
        let mut sorted: Vec<String> = tools.into_iter().collect();
        sorted.sort();
        sorted
    }

    /// Get first user message as summary
    pub fn summary(&self) -> Option<String> {
        self.messages
            .iter()
            .find(|m| m.role == MessageRole::User)
            .map(|m| {
                if m.content.len() > 100 {
                    format!("{}...", &m.content[..100])
                } else {
                    m.content.clone()
                }
            })
    }

    /// Extract all file accesses from the session
    ///
    /// Parses tool invocations to identify files that were read or written.
    /// Returns a vector of FileAccess records with timestamps from the
    /// corresponding messages.
    pub fn extract_file_accesses(&self) -> Vec<FileAccess> {
        let mut accesses = Vec::new();

        for msg in &self.messages {
            for block in &msg.blocks {
                if let ContentBlock::ToolUse { name, input, .. } = block {
                    // Determine operation type based on tool name
                    let operation = match name.as_str() {
                        "Read" | "Glob" | "Grep" => FileOperation::Read,
                        "Edit" | "Write" | "MultiEdit" | "NotebookEdit" => FileOperation::Write,
                        _ => continue, // Skip tools that don't access files
                    };

                    // Extract file path based on tool type
                    let path = match name.as_str() {
                        "Read" | "Edit" | "Write" | "MultiEdit" => {
                            input.get("file_path").and_then(|v| v.as_str())
                        }
                        "NotebookEdit" => input.get("notebook_path").and_then(|v| v.as_str()),
                        "Glob" | "Grep" => input.get("path").and_then(|v| v.as_str()),
                        _ => None,
                    };

                    if let Some(path) = path {
                        accesses.push(FileAccess {
                            path: path.to_string(),
                            operation,
                            timestamp: msg.created_at,
                            tool_name: name.clone(),
                        });
                    }
                }
            }
        }

        accesses
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_role_from_str() {
        assert_eq!(MessageRole::from("user"), MessageRole::User);
        assert_eq!(MessageRole::from("User"), MessageRole::User);
        assert_eq!(MessageRole::from("human"), MessageRole::User);
        assert_eq!(MessageRole::from("assistant"), MessageRole::Assistant);
        assert_eq!(MessageRole::from("AI"), MessageRole::Assistant);
        assert_eq!(MessageRole::from("system"), MessageRole::System);
        assert_eq!(MessageRole::from("tool"), MessageRole::Tool);
        assert_eq!(MessageRole::from("unknown"), MessageRole::Other);
    }

    #[test]
    fn test_message_text() {
        let msg = Message::text(0, MessageRole::User, "Hello, world!");
        assert_eq!(msg.content, "Hello, world!");
        assert_eq!(msg.role, MessageRole::User);
        assert!(!msg.has_tool_use());
    }

    #[test]
    fn test_session_counts() {
        let session = Session {
            id: "test".to_string(),
            source: "test".to_string(),
            external_id: "test".to_string(),
            title: None,
            source_path: PathBuf::from("."),
            started_at: None,
            ended_at: None,
            messages: vec![
                Message::text(0, MessageRole::User, "Hello"),
                Message::text(1, MessageRole::Assistant, "Hi there"),
                Message::text(2, MessageRole::User, "How are you?"),
            ],
            metadata: SessionMetadata::default(),
        };

        assert_eq!(session.message_count(), 3);
        assert_eq!(session.user_message_count(), 2);
        assert_eq!(session.assistant_message_count(), 1);
    }

    #[test]
    fn test_message_role_display() {
        assert_eq!(MessageRole::User.to_string(), "user");
        assert_eq!(MessageRole::Assistant.to_string(), "assistant");
        assert_eq!(MessageRole::System.to_string(), "system");
        assert_eq!(MessageRole::Tool.to_string(), "tool");
        assert_eq!(MessageRole::Other.to_string(), "other");
    }

    #[test]
    fn test_message_role_from_aliases() {
        assert_eq!(MessageRole::from("bot"), MessageRole::Assistant);
        assert_eq!(MessageRole::from("model"), MessageRole::Assistant);
        assert_eq!(MessageRole::from("tool_result"), MessageRole::Tool);
    }

    #[test]
    fn test_content_block_as_text() {
        let text_block = ContentBlock::Text {
            text: "hello".to_string(),
        };
        assert_eq!(text_block.as_text(), Some("hello"));

        let tool_block = ContentBlock::ToolUse {
            id: "1".to_string(),
            name: "Read".to_string(),
            input: serde_json::Value::Null,
        };
        assert_eq!(tool_block.as_text(), None);
    }

    #[test]
    fn test_message_has_tool_use() {
        let mut msg = Message::text(0, MessageRole::Assistant, "text");
        assert!(!msg.has_tool_use());

        msg.blocks.push(ContentBlock::ToolUse {
            id: "1".to_string(),
            name: "Write".to_string(),
            input: serde_json::Value::Null,
        });
        assert!(msg.has_tool_use());
    }

    #[test]
    fn test_message_tool_names() {
        let mut msg = Message::text(0, MessageRole::Assistant, "text");
        msg.blocks.push(ContentBlock::ToolUse {
            id: "1".to_string(),
            name: "Read".to_string(),
            input: serde_json::Value::Null,
        });
        msg.blocks.push(ContentBlock::ToolUse {
            id: "2".to_string(),
            name: "Write".to_string(),
            input: serde_json::Value::Null,
        });

        let names = msg.tool_names();
        assert_eq!(names, vec!["Read", "Write"]);
    }

    #[test]
    fn test_session_tools_used() {
        let mut msg = Message::text(0, MessageRole::Assistant, "text");
        msg.blocks.push(ContentBlock::ToolUse {
            id: "1".to_string(),
            name: "Read".to_string(),
            input: serde_json::Value::Null,
        });
        let mut msg2 = Message::text(1, MessageRole::Assistant, "text2");
        msg2.blocks.push(ContentBlock::ToolUse {
            id: "2".to_string(),
            name: "Read".to_string(),
            input: serde_json::Value::Null,
        });
        msg2.blocks.push(ContentBlock::ToolUse {
            id: "3".to_string(),
            name: "Write".to_string(),
            input: serde_json::Value::Null,
        });

        let session = Session {
            id: "test".to_string(),
            source: "test".to_string(),
            external_id: "test".to_string(),
            title: None,
            source_path: PathBuf::from("."),
            started_at: None,
            ended_at: None,
            messages: vec![msg, msg2],
            metadata: SessionMetadata::default(),
        };

        let tools = session.tools_used();
        assert_eq!(tools, vec!["Read", "Write"]);
    }

    #[test]
    fn test_session_summary_short_message() {
        let session = Session {
            id: "test".to_string(),
            source: "test".to_string(),
            external_id: "test".to_string(),
            title: None,
            source_path: PathBuf::from("."),
            started_at: None,
            ended_at: None,
            messages: vec![Message::text(0, MessageRole::User, "Short question")],
            metadata: SessionMetadata::default(),
        };
        assert_eq!(session.summary(), Some("Short question".to_string()));
    }

    #[test]
    fn test_session_summary_long_message_truncated() {
        let long_text = "a".repeat(200);
        let session = Session {
            id: "test".to_string(),
            source: "test".to_string(),
            external_id: "test".to_string(),
            title: None,
            source_path: PathBuf::from("."),
            started_at: None,
            ended_at: None,
            messages: vec![Message::text(0, MessageRole::User, long_text)],
            metadata: SessionMetadata::default(),
        };
        let summary = session.summary().unwrap();
        assert!(summary.ends_with("..."));
        assert!(summary.len() <= 103); // 100 chars + "..."
    }

    #[test]
    fn test_session_summary_no_user_messages() {
        let session = Session {
            id: "test".to_string(),
            source: "test".to_string(),
            external_id: "test".to_string(),
            title: None,
            source_path: PathBuf::from("."),
            started_at: None,
            ended_at: None,
            messages: vec![Message::text(0, MessageRole::Assistant, "response")],
            metadata: SessionMetadata::default(),
        };
        assert!(session.summary().is_none());
    }

    #[test]
    fn test_session_duration_with_timestamps() {
        let start: jiff::Timestamp = "2024-01-15T10:00:00Z".parse().unwrap();
        let end: jiff::Timestamp = "2024-01-15T10:05:00Z".parse().unwrap();

        let session = Session {
            id: "test".to_string(),
            source: "test".to_string(),
            external_id: "test".to_string(),
            title: None,
            source_path: PathBuf::from("."),
            started_at: Some(start),
            ended_at: Some(end),
            messages: vec![],
            metadata: SessionMetadata::default(),
        };

        let duration = session.duration_ms().unwrap();
        assert_eq!(duration, 300_000); // 5 minutes in ms
    }

    #[test]
    fn test_session_duration_no_timestamps() {
        let session = Session {
            id: "test".to_string(),
            source: "test".to_string(),
            external_id: "test".to_string(),
            title: None,
            source_path: PathBuf::from("."),
            started_at: None,
            ended_at: None,
            messages: vec![],
            metadata: SessionMetadata::default(),
        };
        assert!(session.duration_ms().is_none());
    }

    #[test]
    fn test_session_extract_file_accesses_read_tools() {
        use serde_json::json;

        let mut msg = Message::text(0, MessageRole::Assistant, "reading files");
        msg.created_at = Some("2024-01-15T10:00:00Z".parse().unwrap());
        msg.blocks.push(ContentBlock::ToolUse {
            id: "1".to_string(),
            name: "Read".to_string(),
            input: json!({"file_path": "/path/to/file.rs"}),
        });
        msg.blocks.push(ContentBlock::ToolUse {
            id: "2".to_string(),
            name: "Glob".to_string(),
            input: json!({"path": "/src/**/*.rs"}),
        });

        let session = Session {
            id: "test".to_string(),
            source: "test".to_string(),
            external_id: "test".to_string(),
            title: None,
            source_path: PathBuf::from("."),
            started_at: None,
            ended_at: None,
            messages: vec![msg],
            metadata: SessionMetadata::default(),
        };

        let accesses = session.extract_file_accesses();
        assert_eq!(accesses.len(), 2);
        assert_eq!(accesses[0].path, "/path/to/file.rs");
        assert_eq!(accesses[0].operation, FileOperation::Read);
        assert_eq!(accesses[0].tool_name, "Read");
        assert_eq!(accesses[1].path, "/src/**/*.rs");
        assert_eq!(accesses[1].operation, FileOperation::Read);
        assert_eq!(accesses[1].tool_name, "Glob");
    }

    #[test]
    fn test_session_extract_file_accesses_write_tools() {
        use serde_json::json;

        let mut msg = Message::text(0, MessageRole::Assistant, "writing files");
        msg.blocks.push(ContentBlock::ToolUse {
            id: "1".to_string(),
            name: "Edit".to_string(),
            input: json!({"file_path": "/path/to/file.rs"}),
        });
        msg.blocks.push(ContentBlock::ToolUse {
            id: "2".to_string(),
            name: "Write".to_string(),
            input: json!({"file_path": "/path/to/output.txt"}),
        });
        msg.blocks.push(ContentBlock::ToolUse {
            id: "3".to_string(),
            name: "MultiEdit".to_string(),
            input: json!({"file_path": "/path/to/multi.rs"}),
        });
        msg.blocks.push(ContentBlock::ToolUse {
            id: "4".to_string(),
            name: "NotebookEdit".to_string(),
            input: json!({"notebook_path": "/path/to/notebook.ipynb"}),
        });

        let session = Session {
            id: "test".to_string(),
            source: "test".to_string(),
            external_id: "test".to_string(),
            title: None,
            source_path: PathBuf::from("."),
            started_at: None,
            ended_at: None,
            messages: vec![msg],
            metadata: SessionMetadata::default(),
        };

        let accesses = session.extract_file_accesses();
        assert_eq!(accesses.len(), 4);
        assert_eq!(accesses[0].operation, FileOperation::Write);
        assert_eq!(accesses[1].operation, FileOperation::Write);
        assert_eq!(accesses[2].operation, FileOperation::Write);
        assert_eq!(accesses[3].operation, FileOperation::Write);
        assert_eq!(accesses[3].path, "/path/to/notebook.ipynb");
    }

    #[test]
    fn test_session_extract_file_accesses_skips_unknown_tools() {
        use serde_json::json;

        let mut msg = Message::text(0, MessageRole::Assistant, "using tools");
        msg.blocks.push(ContentBlock::ToolUse {
            id: "1".to_string(),
            name: "Read".to_string(),
            input: json!({"file_path": "/path/to/file.rs"}),
        });
        msg.blocks.push(ContentBlock::ToolUse {
            id: "2".to_string(),
            name: "UnknownTool".to_string(),
            input: json!({"file_path": "/path/to/other.rs"}),
        });

        let session = Session {
            id: "test".to_string(),
            source: "test".to_string(),
            external_id: "test".to_string(),
            title: None,
            source_path: PathBuf::from("."),
            started_at: None,
            ended_at: None,
            messages: vec![msg],
            metadata: SessionMetadata::default(),
        };

        let accesses = session.extract_file_accesses();
        assert_eq!(accesses.len(), 1);
        assert_eq!(accesses[0].tool_name, "Read");
    }

    #[test]
    fn test_session_extract_file_accesses_empty_session() {
        let session = Session {
            id: "test".to_string(),
            source: "test".to_string(),
            external_id: "test".to_string(),
            title: None,
            source_path: PathBuf::from("."),
            started_at: None,
            ended_at: None,
            messages: vec![],
            metadata: SessionMetadata::default(),
        };

        let accesses = session.extract_file_accesses();
        assert!(accesses.is_empty());
    }
}

/// File access operation type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FileOperation {
    Read,
    Write,
}

impl std::fmt::Display for FileOperation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Read => write!(f, "read"),
            Self::Write => write!(f, "write"),
        }
    }
}

/// Record of a file access in a session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileAccess {
    /// File path (as recorded in tool input)
    pub path: String,
    /// Type of file operation
    pub operation: FileOperation,
    /// Timestamp of the access (from message)
    pub timestamp: Option<jiff::Timestamp>,
    /// Name of the tool that accessed the file
    pub tool_name: String,
}

#[cfg(test)]
mod file_access_tests {
    use super::*;

    #[test]
    fn test_file_operation_display() {
        assert_eq!(FileOperation::Read.to_string(), "read");
        assert_eq!(FileOperation::Write.to_string(), "write");
    }

    #[test]
    fn test_file_access_creation() {
        let access = FileAccess {
            path: "/path/to/file.rs".to_string(),
            operation: FileOperation::Read,
            timestamp: None,
            tool_name: "Read".to_string(),
        };
        assert_eq!(access.path, "/path/to/file.rs");
        assert_eq!(access.operation, FileOperation::Read);
        assert_eq!(access.tool_name, "Read");
    }

    #[test]
    fn test_file_access_serialization() {
        let access = FileAccess {
            path: "/path/to/file.rs".to_string(),
            operation: FileOperation::Write,
            timestamp: None,
            tool_name: "Edit".to_string(),
        };
        let json = serde_json::to_string(&access).unwrap();
        assert!(json.contains("/path/to/file.rs"));
        // Serde serializes enum as "Write" (variant name)
        assert!(
            json.contains("Write"),
            "JSON should contain 'Write', got: {}",
            json
        );
        assert!(json.contains("Edit"));

        let deserialized: FileAccess = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.path, access.path);
        assert_eq!(deserialized.operation, access.operation);
    }
}
