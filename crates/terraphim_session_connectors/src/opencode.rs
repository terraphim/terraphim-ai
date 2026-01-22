//! OpenCode Session Connector.
//!
//! Parses session logs from OpenCode AI coding assistant.
//!
//! OpenCode stores prompt history in `~/.local/state/opencode/prompt-history.jsonl`.

use crate::{
    ConnectorStatus, ImportOptions, NormalizedMessage, NormalizedSession, SessionConnector,
};
use anyhow::Result;
use serde::Deserialize;
use std::path::PathBuf;

/// OpenCode connector.
#[derive(Debug, Default)]
pub struct OpenCodeConnector;

/// OpenCode prompt history entry (JSONL format).
#[derive(Debug, Clone, Deserialize)]
struct OpenCodeEntry {
    /// User input prompt.
    #[serde(default)]
    input: Option<String>,
    /// Additional parts (usually empty).
    #[serde(default)]
    parts: Vec<serde_json::Value>,
    /// Mode (e.g., "normal").
    #[serde(default)]
    mode: Option<String>,
}

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
        home::home_dir().map(|h| h.join(".local").join("state").join("opencode"))
    }

    fn import(&self, options: &ImportOptions) -> Result<Vec<NormalizedSession>> {
        let path = options
            .path
            .clone()
            .or_else(|| self.default_path())
            .ok_or_else(|| anyhow::anyhow!("No path specified and default not found"))?;

        let history_file = path.join("prompt-history.jsonl");
        if !history_file.exists() {
            return Ok(Vec::new());
        }

        let content = std::fs::read_to_string(&history_file)?;
        let mut messages: Vec<NormalizedMessage> = Vec::new();

        for (idx, line) in content.lines().enumerate() {
            if line.trim().is_empty() {
                continue;
            }
            if let Ok(entry) = serde_json::from_str::<OpenCodeEntry>(line) {
                if let Some(input) = entry.input {
                    if !input.is_empty() {
                        messages.push(NormalizedMessage {
                            idx,
                            role: "user".to_string(),
                            author: None,
                            content: input,
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

        if let Some(limit) = options.limit {
            if limit > 0 {
                messages.truncate(limit);
            }
        }

        if messages.is_empty() {
            return Ok(Vec::new());
        }

        let session = NormalizedSession {
            source: "opencode".to_string(),
            external_id: "prompt-history".to_string(),
            title: Some("OpenCode Prompt History".to_string()),
            source_path: history_file,
            started_at: None,
            ended_at: None,
            messages,
            metadata: serde_json::json!({
                "type": "prompt-history",
            }),
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
        let json = r#"{"input":"hello","parts":[],"mode":"normal"}"#;
        let entry: OpenCodeEntry = serde_json::from_str(json).unwrap();
        assert_eq!(entry.input, Some("hello".to_string()));
        assert_eq!(entry.mode, Some("normal".to_string()));
        assert!(entry.parts.is_empty());
    }
}
