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

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use walkdir::WalkDir;

use super::{
    ConnectorStatus, ImportOptions, NormalizedMessage, NormalizedSession, SessionConnector,
};

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

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
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
                let count = WalkDir::new(&path)
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
        home::home_dir().map(|h| h.join(".codex").join("sessions"))
    }

    fn import(&self, options: &ImportOptions) -> Result<Vec<NormalizedSession>> {
        let path = options
            .path
            .clone()
            .or_else(|| self.default_path())
            .ok_or_else(|| anyhow::anyhow!("No path specified and default not found"))?;

        let mut sessions = Vec::new();

        // Find all JSONL session files
        for entry in WalkDir::new(&path)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().is_some_and(|ext| ext == "jsonl"))
        {
            if let Some(limit) = options.limit {
                if sessions.len() >= limit {
                    break;
                }
            }

            match self.parse_session_file(entry.path()) {
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
    fn parse_session_file(&self, path: &std::path::Path) -> Result<Option<NormalizedSession>> {
        let content = std::fs::read_to_string(path)?;
        let mut session_meta: Option<SessionMeta> = None;
        let mut messages: Vec<NormalizedMessage> = Vec::new();
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
                            messages.push(NormalizedMessage {
                                idx,
                                role: item.role,
                                author: None,
                                content,
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

        Ok(Some(NormalizedSession {
            source: "codex".to_string(),
            external_id: meta.id,
            title: meta.cwd.clone(),
            source_path: path.to_path_buf(),
            started_at,
            ended_at,
            messages,
            metadata: serde_json::json!({
                "cwd": meta.cwd,
                "git": meta.git,
                "cli_version": meta.cli_version,
            }),
        }))
    }
}

fn parse_timestamp(ts: &str) -> Option<jiff::Timestamp> {
    jiff::Timestamp::strptime("%Y-%m-%dT%H:%M:%S%.fZ", ts).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_connector_source_id() {
        let connector = CodexConnector::default();
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
}
