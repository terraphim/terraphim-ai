//! Core data models for session management
//!
//! These models provide a unified representation of sessions and messages
//! from various AI coding assistants.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

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
}
