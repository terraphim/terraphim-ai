//! OpenCode Session Connector
//!
//! Parses session logs from OpenCode AI coding assistant.
//!
//! ## Format
//!
//! OpenCode stores prompt history in `~/.local/state/opencode/prompt-history.jsonl`
//! with entries containing `input`, `parts`, and `mode` fields.

use std::path::PathBuf;

use super::{ConnectorStatus, ImportOptions, SessionConnector};
use crate::model::{Message, MessageRole, Session, SessionMetadata};
use anyhow::Result;
use async_trait::async_trait;
use serde::Deserialize;

/// OpenCode connector
#[derive(Debug, Default)]
pub struct OpenCodeConnector;

/// OpenCode prompt history entry (JSONL format)
#[derive(Debug, Clone, Deserialize)]
struct OpenCodeEntry {
    /// User input prompt
    #[serde(default)]
    input: Option<String>,
    /// Additional parts (usually empty)
    #[serde(default)]
    parts: Vec<serde_json::Value>,
    /// Mode (e.g., "normal")
    #[serde(default)]
    mode: Option<String>,
}

#[async_trait]
impl SessionConnector for OpenCodeConnector {
    fn source_id(&self) -> &str {
        "opencode"
    }

    fn display_name(&self) -> &str {
        "OpenCode"
    }

    fn detect(&self) -> ConnectorStatus {
        if let Some(path) = self.default_path() {
            let history_file = path.join("prompt-history.jsonl");
            if history_file.exists() {
                // Count lines in prompt history as session estimate
                let count = std::fs::read_to_string(&history_file)
                    .map(|c| c.lines().filter(|l| !l.trim().is_empty()).count())
                    .unwrap_or(0);
                ConnectorStatus::Available {
                    path,
                    sessions_estimate: Some(if count > 0 { 1 } else { 0 }),
                }
            } else {
                ConnectorStatus::NotFound
            }
        } else {
            ConnectorStatus::NotFound
        }
    }

    fn default_path(&self) -> Option<PathBuf> {
        dirs::home_dir().map(|h| h.join(".local").join("state").join("opencode"))
    }

    async fn import(&self, options: &ImportOptions) -> Result<Vec<Session>> {
        let path = options
            .path
            .clone()
            .or_else(|| self.default_path())
            .ok_or_else(|| anyhow::anyhow!("No path specified and default not found"))?;

        let history_file = path.join("prompt-history.jsonl");
        if !history_file.exists() {
            return Ok(Vec::new());
        }

        let content = tokio::fs::read_to_string(&history_file).await?;
        let mut messages: Vec<Message> = Vec::new();

        for (idx, line) in content.lines().enumerate() {
            if line.trim().is_empty() {
                continue;
            }
            if let Ok(entry) = serde_json::from_str::<OpenCodeEntry>(line) {
                if let Some(input) = entry.input {
                    if !input.is_empty() {
                        messages.push(Message {
                            idx,
                            role: MessageRole::User,
                            author: None,
                            content: input.clone(),
                            blocks: vec![crate::model::ContentBlock::Text { text: input }],
                            created_at: None,
                            extra: serde_json::json!({
                                "mode": entry.mode,
                                "parts": entry.parts,
                            }),
                        });
                    }
                }
            }
        }

        // Apply limit if specified
        if let Some(limit) = options.limit {
            if limit > 0 {
                messages.truncate(limit);
            }
        }

        if messages.is_empty() {
            return Ok(Vec::new());
        }

        // OpenCode stores all prompts in a single file, so we create one session
        let session = Session {
            id: "opencode:prompt-history".to_string(),
            source: "opencode".to_string(),
            external_id: "prompt-history".to_string(),
            title: Some("OpenCode Prompt History".to_string()),
            source_path: history_file.clone(),
            started_at: None,
            ended_at: None,
            messages,
            metadata: SessionMetadata::new(
                None,
                None,
                vec!["opencode".to_string()],
                serde_json::json!({
                    "type": "prompt-history",
                }),
            ),
        };

        Ok(vec![session])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_connector_source_id() {
        let connector = OpenCodeConnector;
        assert_eq!(connector.source_id(), "opencode");
        assert_eq!(connector.display_name(), "OpenCode");
    }

    #[test]
    fn test_parse_entry() {
        let json = r#"{"input":"use ask user tool to ask all questions one by one","parts":[],"mode":"normal"}"#;
        let entry: OpenCodeEntry = serde_json::from_str(json).unwrap();
        assert_eq!(
            entry.input,
            Some("use ask user tool to ask all questions one by one".to_string())
        );
        assert_eq!(entry.mode, Some("normal".to_string()));
        assert!(entry.parts.is_empty());
    }

    #[test]
    fn test_default_path() {
        let connector = OpenCodeConnector;
        let path = connector.default_path();
        assert!(path.is_some());
        let path = path.unwrap();
        assert!(path.ends_with(".local/state/opencode"));
    }

    #[tokio::test]
    async fn test_import_from_temp_file() {
        let dir = tempfile::tempdir().unwrap();
        let history_file = dir.path().join("prompt-history.jsonl");
        let content = [
            r#"{"input":"hello world","parts":[],"mode":"normal"}"#,
            r#"{"input":"second prompt","parts":[],"mode":"insert"}"#,
        ]
        .join("\n");
        tokio::fs::write(&history_file, content).await.unwrap();

        let connector = OpenCodeConnector;
        let options = ImportOptions::default().with_path(dir.path().to_path_buf());
        let sessions = connector.import(&options).await.unwrap();

        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].messages.len(), 2);
        assert_eq!(sessions[0].messages[0].role, MessageRole::User);
        assert_eq!(sessions[0].messages[0].content, "hello world");
        assert_eq!(sessions[0].messages[1].content, "second prompt");
    }

    #[tokio::test]
    async fn test_import_empty_file() {
        let dir = tempfile::tempdir().unwrap();
        let history_file = dir.path().join("prompt-history.jsonl");
        tokio::fs::write(&history_file, "").await.unwrap();

        let connector = OpenCodeConnector;
        let options = ImportOptions::default().with_path(dir.path().to_path_buf());
        let sessions = connector.import(&options).await.unwrap();

        assert!(sessions.is_empty());
    }

    #[tokio::test]
    async fn test_import_with_limit() {
        let dir = tempfile::tempdir().unwrap();
        let history_file = dir.path().join("prompt-history.jsonl");
        let content = [
            r#"{"input":"first","parts":[],"mode":"normal"}"#,
            r#"{"input":"second","parts":[],"mode":"normal"}"#,
            r#"{"input":"third","parts":[],"mode":"normal"}"#,
        ]
        .join("\n");
        tokio::fs::write(&history_file, content).await.unwrap();

        let connector = OpenCodeConnector;
        let options = ImportOptions::default()
            .with_path(dir.path().to_path_buf())
            .with_limit(2);
        let sessions = connector.import(&options).await.unwrap();

        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].messages.len(), 2);
    }
}
