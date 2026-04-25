//! Codex CLI Session Connector
//!
//! Parses session logs from OpenAI Codex CLI (`~/.codex/`).
//!
//! ## Format
//!
//! Codex stores sessions as JSONL files in `~/.codex/sessions/YYYY/MM/DD/`.
//! Each file has the pattern `rollout-<timestamp>-<session_id>.jsonl`.
//!
//! Entry types:
//! - `session_meta`: Session metadata (id, cwd, git info)
//! - `response_item`: User/assistant messages

use std::path::PathBuf;

use super::{ConnectorStatus, ImportOptions, SessionConnector};
use crate::model::{Message, MessageRole, Session, SessionMetadata};
use anyhow::Result;
use async_trait::async_trait;
use serde::Deserialize;

/// Codex CLI connector
#[derive(Debug, Default)]
pub struct CodexConnector;

/// Session metadata entry
#[derive(Debug, Clone, Deserialize)]
struct SessionMeta {
    id: String,
    timestamp: String,
    cwd: Option<String>,
    #[serde(default)]
    git: Option<GitInfo>,
    #[serde(default)]
    cli_version: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
struct GitInfo {
    #[serde(default)]
    commit_hash: Option<String>,
    #[serde(default)]
    branch: Option<String>,
    #[serde(default)]
    repository_url: Option<String>,
}

/// Response item entry
#[derive(Debug, Clone, Deserialize)]
struct ResponseItem {
    #[serde(rename = "type")]
    #[allow(dead_code)] // Required for deserializing "type" field
    msg_type: String,
    role: String,
    #[serde(default)]
    content: Vec<ContentBlock>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ContentBlock {
    InputText {
        text: String,
    },
    Text {
        text: String,
    },
    #[serde(other)]
    Other,
}

/// JSONL entry wrapper
#[derive(Debug, Clone, Deserialize)]
struct CodexEntry {
    timestamp: String,
    #[serde(rename = "type")]
    entry_type: String,
    payload: serde_json::Value,
}

#[async_trait]
impl SessionConnector for CodexConnector {
    fn source_id(&self) -> &str {
        "codex"
    }

    fn display_name(&self) -> &str {
        "OpenAI Codex CLI"
    }

    fn detect(&self) -> ConnectorStatus {
        if let Some(path) = self.default_path() {
            if path.exists() {
                let count = walkdir::WalkDir::new(&path)
                    .max_depth(4)
                    .follow_links(false)
                    .into_iter()
                    .filter_map(|e| e.ok())
                    .filter(|e| e.path().extension().is_some_and(|ext| ext == "jsonl"))
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
        dirs::home_dir().map(|h| h.join(".codex").join("sessions"))
    }

    async fn import(&self, options: &ImportOptions) -> Result<Vec<Session>> {
        let path = options
            .path
            .clone()
            .or_else(|| self.default_path())
            .ok_or_else(|| anyhow::anyhow!("No path specified and default not found"))?;

        let mut sessions = Vec::new();

        // Find all JSONL session files
        for entry in walkdir::WalkDir::new(&path)
            .max_depth(4)
            .follow_links(false)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().is_some_and(|ext| ext == "jsonl"))
        {
            if let Some(limit) = options.limit {
                if sessions.len() >= limit {
                    break;
                }
            }

            match self.parse_session_file(entry.path()).await {
                Ok(Some(session)) => sessions.push(session),
                Ok(None) => continue,
                Err(e) => {
                    tracing::warn!("Failed to parse {:?}: {}", entry.path(), e);
                    continue;
                }
            }
        }

        Ok(sessions)
    }
}

impl CodexConnector {
    async fn parse_session_file(&self, path: &std::path::Path) -> Result<Option<Session>> {
        let content = tokio::fs::read_to_string(path).await?;
        let mut session_meta: Option<SessionMeta> = None;
        let mut messages: Vec<Message> = Vec::new();
        let mut idx = 0;

        for line in content.lines() {
            if line.trim().is_empty() {
                continue;
            }

            let entry: CodexEntry = match serde_json::from_str(line) {
                Ok(e) => e,
                Err(_) => continue,
            };

            match entry.entry_type.as_str() {
                "session_meta" => {
                    if let Ok(meta) = serde_json::from_value::<SessionMeta>(entry.payload) {
                        session_meta = Some(meta);
                    }
                }
                "response_item" => {
                    if let Ok(item) = serde_json::from_value::<ResponseItem>(entry.payload) {
                        // Extract text content
                        let content: String = item
                            .content
                            .iter()
                            .filter_map(|block| match block {
                                ContentBlock::InputText { text } => Some(text.clone()),
                                ContentBlock::Text { text } => Some(text.clone()),
                                ContentBlock::Other => None,
                            })
                            .collect::<Vec<_>>()
                            .join("\n");

                        if !content.is_empty() {
                            let role = match item.role.as_str() {
                                "user" => MessageRole::User,
                                "assistant" => MessageRole::Assistant,
                                _ => MessageRole::Other,
                            };
                            messages.push(Message {
                                idx,
                                role,
                                author: None,
                                content: content.clone(),
                                blocks: vec![crate::model::ContentBlock::Text { text: content }],
                                created_at: parse_timestamp(&entry.timestamp),
                                extra: serde_json::Value::Null,
                            });
                            idx += 1;
                        }
                    }
                }
                _ => {}
            }
        }

        // Need at least metadata and one message
        let meta = match session_meta {
            Some(m) => m,
            None => return Ok(None),
        };

        if messages.is_empty() {
            return Ok(None);
        }

        let started_at = parse_timestamp(&meta.timestamp);
        let ended_at = messages.last().and_then(|m| m.created_at);

        let mut extra = serde_json::json!({
            "cwd": meta.cwd,
            "cli_version": meta.cli_version,
        });
        if let Some(git) = meta.git {
            extra["git"] = serde_json::json!({
                "commit_hash": git.commit_hash,
                "branch": git.branch,
                "repository_url": git.repository_url,
            });
        }

        Ok(Some(Session {
            id: format!("codex:{}", meta.id),
            source: "codex".to_string(),
            external_id: meta.id,
            title: meta.cwd.clone(),
            source_path: path.to_path_buf(),
            started_at,
            ended_at,
            messages,
            metadata: SessionMetadata::new(meta.cwd, None, vec!["codex".to_string()], extra),
        }))
    }
}

fn parse_timestamp(ts: &str) -> Option<jiff::Timestamp> {
    ts.parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_connector_source_id() {
        let connector = CodexConnector;
        assert_eq!(connector.source_id(), "codex");
        assert_eq!(connector.display_name(), "OpenAI Codex CLI");
    }

    #[test]
    fn test_parse_session_entry() {
        let entry_json = r#"{"timestamp":"2025-11-13T16:26:27.512Z","type":"session_meta","payload":{"id":"test-id","timestamp":"2025-11-13T16:26:27.509Z","cwd":"/home/test","originator":"codex_cli_rs","cli_version":"0.46.0"}}"#;
        let entry: CodexEntry = serde_json::from_str(entry_json).unwrap();
        assert_eq!(entry.entry_type, "session_meta");
    }

    #[test]
    fn test_parse_response_item() {
        let item_json = r#"{"timestamp":"2025-11-13T16:28:21.445Z","type":"response_item","payload":{"type":"message","role":"user","content":[{"type":"input_text","text":"Hello world"}]}}"#;
        let entry: CodexEntry = serde_json::from_str(item_json).unwrap();
        assert_eq!(entry.entry_type, "response_item");

        let item: ResponseItem = serde_json::from_value(entry.payload).unwrap();
        assert_eq!(item.role, "user");
    }

    #[tokio::test]
    async fn test_parse_session_file() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("rollout-2025-11-13-test.jsonl");
        let content = [
            r#"{"timestamp":"2025-11-13T16:26:27.512Z","type":"session_meta","payload":{"id":"test-id","timestamp":"2025-11-13T16:26:27.509Z","cwd":"/home/test","cli_version":"0.46.0"}}"#,
            r#"{"timestamp":"2025-11-13T16:28:21.445Z","type":"response_item","payload":{"type":"message","role":"user","content":[{"type":"input_text","text":"Hello world"}]}}"#,
            r#"{"timestamp":"2025-11-13T16:28:25.123Z","type":"response_item","payload":{"type":"message","role":"assistant","content":[{"type":"text","text":"Hi there!"}]}}"#,
        ]
        .join("\n");
        tokio::fs::write(&file_path, content).await.unwrap();

        let connector = CodexConnector;
        let session = connector
            .parse_session_file(&file_path)
            .await
            .unwrap()
            .unwrap();

        assert_eq!(session.external_id, "test-id");
        assert_eq!(session.messages.len(), 2);
        assert_eq!(session.messages[0].role, MessageRole::User);
        assert_eq!(session.messages[0].content, "Hello world");
        assert_eq!(session.messages[1].role, MessageRole::Assistant);
        assert_eq!(session.messages[1].content, "Hi there!");
        assert!(session.started_at.is_some());
    }

    #[tokio::test]
    async fn test_parse_session_file_no_meta() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("rollout-test.jsonl");
        let content = r#"{"timestamp":"2025-11-13T16:28:21.445Z","type":"response_item","payload":{"type":"message","role":"user","content":[{"type":"input_text","text":"Hello"}]}}"#;
        tokio::fs::write(&file_path, content).await.unwrap();

        let connector = CodexConnector;
        let result = connector.parse_session_file(&file_path).await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_parse_session_file_empty_messages() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("rollout-test.jsonl");
        let content = r#"{"timestamp":"2025-11-13T16:26:27.512Z","type":"session_meta","payload":{"id":"test-id","timestamp":"2025-11-13T16:26:27.509Z","cwd":"/home/test"}}"#;
        tokio::fs::write(&file_path, content).await.unwrap();

        let connector = CodexConnector;
        let result = connector.parse_session_file(&file_path).await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_import_with_limit() {
        let dir = tempfile::tempdir().unwrap();
        // Create multiple session files
        for i in 0..3 {
            let file_path = dir.path().join(format!("rollout-2025-11-13-{}.jsonl", i));
            let content = format!(
                "{{\"timestamp\":\"2025-11-13T16:26:27.512Z\",\"type\":\"session_meta\",\"payload\":{{\"id\":\"sess{}\",\"timestamp\":\"2025-11-13T16:26:27.509Z\",\"cwd\":\"/home/test{}\"}}}}\n{{\"timestamp\":\"2025-11-13T16:28:21.445Z\",\"type\":\"response_item\",\"payload\":{{\"type\":\"message\",\"role\":\"user\",\"content\":[{{\"type\":\"input_text\",\"text\":\"Hello {}\"}}]}}}}",
                i, i, i
            );
            tokio::fs::write(&file_path, content).await.unwrap();
        }

        let connector = CodexConnector;
        let options = ImportOptions::default()
            .with_path(dir.path().to_path_buf())
            .with_limit(2);
        let sessions = connector.import(&options).await.unwrap();
        assert_eq!(sessions.len(), 2);
    }
}
