//! Native Claude Code connector
//!
//! A lightweight parser for Claude Code JSONL session files
//! that works without the full claude-log-analyzer dependency.

use super::{ConnectorStatus, ImportOptions, SessionConnector};
use crate::model::{ContentBlock, Message, MessageRole, Session, SessionMetadata};
use anyhow::{Context, Result};
use async_trait::async_trait;
use serde::Deserialize;
use std::path::PathBuf;

/// Native Claude Code session connector
#[derive(Debug, Default)]
pub struct NativeClaudeConnector;

#[async_trait]
impl SessionConnector for NativeClaudeConnector {
    fn source_id(&self) -> &str {
        "claude-code-native"
    }

    fn display_name(&self) -> &str {
        "Claude Code (Native)"
    }

    fn detect(&self) -> ConnectorStatus {
        if let Some(path) = self.default_path() {
            if path.exists() {
                let count = walkdir::WalkDir::new(&path)
                    .max_depth(3)
                    .into_iter()
                    .filter_map(|e| e.ok())
                    .filter(|e| {
                        e.path()
                            .extension()
                            .is_some_and(|ext| ext == "jsonl")
                    })
                    .count();
                ConnectorStatus::Available {
                    path,
                    sessions_estimate: Some(count),
                }
            } else {
                ConnectorStatus::NotFound
            }
        } else {
            ConnectorStatus::NotFound
        }
    }

    fn default_path(&self) -> Option<PathBuf> {
        dirs::home_dir().map(|h| h.join(".claude").join("projects"))
    }

    async fn import(&self, options: &ImportOptions) -> Result<Vec<Session>> {
        let base_path = options
            .path
            .clone()
            .or_else(|| self.default_path())
            .ok_or_else(|| anyhow::anyhow!("No path specified and default not found"))?;

        tracing::info!("Importing Claude sessions from: {}", base_path.display());

        let mut sessions = Vec::new();

        // Find all JSONL files
        let jsonl_files: Vec<PathBuf> = walkdir::WalkDir::new(&base_path)
            .max_depth(3)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.path()
                    .extension()
                    .is_some_and(|ext| ext == "jsonl")
            })
            .map(|e| e.path().to_path_buf())
            .collect();

        tracing::info!("Found {} JSONL files", jsonl_files.len());

        for file_path in jsonl_files {
            match self.parse_session_file(&file_path).await {
                Ok(session) => {
                    if let Some(session) = session {
                        // Apply time filters
                        if let Some(since) = options.since {
                            if session.started_at.map(|t| t < since).unwrap_or(false) {
                                continue;
                            }
                        }
                        if let Some(until) = options.until {
                            if session.started_at.map(|t| t > until).unwrap_or(false) {
                                continue;
                            }
                        }

                        sessions.push(session);
                    }
                }
                Err(e) => {
                    tracing::warn!("Failed to parse {}: {}", file_path.display(), e);
                }
            }

            // Apply limit
            if let Some(limit) = options.limit {
                if sessions.len() >= limit {
                    break;
                }
            }
        }

        tracing::info!("Imported {} Claude sessions", sessions.len());
        Ok(sessions)
    }
}

impl NativeClaudeConnector {
    /// Parse a single session file
    async fn parse_session_file(&self, path: &PathBuf) -> Result<Option<Session>> {
        let content = tokio::fs::read_to_string(path)
            .await
            .with_context(|| format!("Failed to read {}", path.display()))?;

        let mut entries: Vec<LogEntry> = Vec::new();

        for line in content.lines() {
            if line.trim().is_empty() {
                continue;
            }
            match serde_json::from_str::<LogEntry>(line) {
                Ok(entry) => entries.push(entry),
                Err(e) => {
                    tracing::trace!("Skipping malformed line: {}", e);
                }
            }
        }

        if entries.is_empty() {
            return Ok(None);
        }

        // Extract session ID from first entry
        let session_id = entries
            .first()
            .and_then(|e| e.session_id.clone())
            .unwrap_or_else(|| {
                path.file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("unknown")
                    .to_string()
            });

        // Extract project path from first entry's cwd
        let project_path = entries
            .first()
            .and_then(|e| e.cwd.clone());

        // Convert entries to messages
        let messages: Vec<Message> = entries
            .iter()
            .enumerate()
            .filter_map(|(idx, entry)| self.entry_to_message(idx, entry))
            .collect();

        if messages.is_empty() {
            return Ok(None);
        }

        // Parse timestamps
        let started_at = entries
            .first()
            .and_then(|e| parse_timestamp(&e.timestamp));
        let ended_at = entries
            .last()
            .and_then(|e| parse_timestamp(&e.timestamp));

        Ok(Some(Session {
            id: format!("claude-code-native:{}", session_id),
            source: "claude-code-native".to_string(),
            external_id: session_id,
            title: project_path.clone(),
            source_path: path.clone(),
            started_at,
            ended_at,
            messages,
            metadata: SessionMetadata {
                project_path,
                model: None,
                tags: vec![],
                extra: serde_json::Value::Null,
            },
        }))
    }

    /// Convert a log entry to a message
    fn entry_to_message(&self, idx: usize, entry: &LogEntry) -> Option<Message> {
        match &entry.message {
            EntryMessage::User { content, .. } => Some(Message {
                idx,
                role: MessageRole::User,
                author: None,
                content: content.clone(),
                blocks: vec![ContentBlock::Text {
                    text: content.clone(),
                }],
                created_at: parse_timestamp(&entry.timestamp),
                extra: serde_json::Value::Null,
            }),
            EntryMessage::Assistant { content, .. } => {
                let (text_content, blocks) = self.parse_assistant_content(content);
                Some(Message {
                    idx,
                    role: MessageRole::Assistant,
                    author: None,
                    content: text_content,
                    blocks,
                    created_at: parse_timestamp(&entry.timestamp),
                    extra: serde_json::Value::Null,
                })
            }
            EntryMessage::ToolResult { content, .. } => {
                let text = content
                    .iter()
                    .map(|c| c.content.clone())
                    .collect::<Vec<_>>()
                    .join("\n");
                Some(Message {
                    idx,
                    role: MessageRole::Tool,
                    author: None,
                    content: text.clone(),
                    blocks: vec![ContentBlock::Text { text }],
                    created_at: parse_timestamp(&entry.timestamp),
                    extra: serde_json::Value::Null,
                })
            }
        }
    }

    /// Parse assistant content blocks
    fn parse_assistant_content(
        &self,
        content: &[AssistantContentBlock],
    ) -> (String, Vec<ContentBlock>) {
        let mut text_parts = Vec::new();
        let mut blocks = Vec::new();

        for block in content {
            match block {
                AssistantContentBlock::Text { text } => {
                    text_parts.push(text.clone());
                    blocks.push(ContentBlock::Text { text: text.clone() });
                }
                AssistantContentBlock::ToolUse { id, name, input } => {
                    blocks.push(ContentBlock::ToolUse {
                        id: id.clone(),
                        name: name.clone(),
                        input: input.clone(),
                    });
                }
            }
        }

        (text_parts.join("\n"), blocks)
    }
}

/// Parse ISO 8601 timestamp
fn parse_timestamp(ts: &str) -> Option<jiff::Timestamp> {
    ts.parse().ok()
}

// Serde structures for JSONL parsing

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LogEntry {
    #[serde(default)]
    session_id: Option<String>,
    #[serde(default)]
    cwd: Option<String>,
    #[serde(default)]
    timestamp: String,
    message: EntryMessage,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "role", rename_all = "lowercase")]
enum EntryMessage {
    User {
        content: String,
    },
    Assistant {
        content: Vec<AssistantContentBlock>,
    },
    #[serde(rename = "tool_result")]
    ToolResult {
        content: Vec<ToolResultContent>,
    },
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum AssistantContentBlock {
    Text { text: String },
    ToolUse {
        id: String,
        name: String,
        input: serde_json::Value,
    },
}

#[derive(Debug, Deserialize)]
struct ToolResultContent {
    content: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_timestamp() {
        let ts = parse_timestamp("2024-01-15T10:30:00Z");
        assert!(ts.is_some());
    }

    #[test]
    fn test_connector_source_id() {
        let connector = NativeClaudeConnector;
        assert_eq!(connector.source_id(), "claude-code-native");
        assert_eq!(connector.display_name(), "Claude Code (Native)");
    }
}
