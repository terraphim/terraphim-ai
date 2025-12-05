//! OpenCode Session Connector
//!
//! Parses session logs from OpenCode AI coding assistant (`~/.opencode/`).
//!
//! ## Format
//!
//! OpenCode stores sessions as JSONL files, similar to Claude Code.
//! Sessions are stored in `~/.opencode/sessions/` with JSONL format.

use anyhow::Result;
use serde::Deserialize;
use std::path::PathBuf;
use walkdir::WalkDir;

use super::{
    ConnectorStatus, ImportOptions, NormalizedMessage, NormalizedSession, SessionConnector,
};

/// OpenCode connector
#[derive(Debug, Default)]
pub struct OpenCodeConnector;

/// OpenCode session entry (JSONL format)
#[derive(Debug, Clone, Deserialize)]
struct OpenCodeEntry {
    #[serde(rename = "sessionId", default)]
    session_id: Option<String>,
    #[serde(default)]
    timestamp: Option<String>,
    #[serde(default)]
    cwd: Option<String>,
    #[serde(default)]
    message: Option<OpenCodeMessage>,
    #[serde(rename = "type", default)]
    entry_type: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct OpenCodeMessage {
    role: String,
    #[serde(default)]
    content: serde_json::Value,
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
        home::home_dir().map(|h| h.join(".opencode"))
    }

    fn import(&self, options: &ImportOptions) -> Result<Vec<NormalizedSession>> {
        let path = options
            .path
            .clone()
            .or_else(|| self.default_path())
            .ok_or_else(|| anyhow::anyhow!("No path specified and default not found"))?;

        let mut sessions = Vec::new();
        let mut session_map: std::collections::HashMap<String, Vec<(String, OpenCodeEntry)>> =
            std::collections::HashMap::new();

        // Find all JSONL files
        for entry in WalkDir::new(&path)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().is_some_and(|ext| ext == "jsonl"))
        {
            let file_path = entry.path();
            if let Ok(content) = std::fs::read_to_string(file_path) {
                for line in content.lines() {
                    if line.trim().is_empty() {
                        continue;
                    }
                    if let Ok(entry) = serde_json::from_str::<OpenCodeEntry>(line) {
                        if let Some(session_id) = &entry.session_id {
                            session_map
                                .entry(session_id.clone())
                                .or_default()
                                .push((file_path.to_string_lossy().to_string(), entry));
                        }
                    }
                }
            }
        }

        // Convert to normalized sessions
        for (session_id, entries) in session_map {
            if let Some(limit) = options.limit {
                if sessions.len() >= limit {
                    break;
                }
            }

            let source_path = entries
                .first()
                .map(|(p, _)| PathBuf::from(p))
                .unwrap_or_default();

            let mut messages: Vec<NormalizedMessage> = Vec::new();
            let mut started_at: Option<jiff::Timestamp> = None;
            let mut ended_at: Option<jiff::Timestamp> = None;
            let mut cwd: Option<String> = None;

            for (idx, (_path, entry)) in entries.iter().enumerate() {
                // Track timestamps
                if let Some(ts) = &entry.timestamp {
                    let parsed = parse_timestamp(ts);
                    if started_at.is_none() {
                        started_at = parsed;
                    }
                    ended_at = parsed;
                }

                // Track cwd
                if cwd.is_none() {
                    cwd = entry.cwd.clone();
                }

                // Convert message
                if let Some(msg) = &entry.message {
                    let content = extract_content(&msg.content);
                    if !content.is_empty() {
                        messages.push(NormalizedMessage {
                            idx,
                            role: msg.role.clone(),
                            author: None,
                            content,
                            created_at: entry.timestamp.as_ref().and_then(|t| parse_timestamp(t)),
                            extra: serde_json::Value::Null,
                        });
                    }
                }
            }

            if !messages.is_empty() {
                sessions.push(NormalizedSession {
                    source: "opencode".to_string(),
                    external_id: session_id,
                    title: cwd.clone(),
                    source_path,
                    started_at,
                    ended_at,
                    messages,
                    metadata: serde_json::json!({
                        "cwd": cwd,
                    }),
                });
            }
        }

        Ok(sessions)
    }
}

fn parse_timestamp(ts: &str) -> Option<jiff::Timestamp> {
    // Try ISO 8601 format
    jiff::Timestamp::strptime("%Y-%m-%dT%H:%M:%S%.fZ", ts)
        .ok()
        .or_else(|| jiff::Timestamp::strptime("%Y-%m-%dT%H:%M:%S", ts).ok())
}

fn extract_content(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Array(arr) => arr
            .iter()
            .filter_map(|item| {
                if let Some(text) = item.get("text").and_then(|t| t.as_str()) {
                    Some(text.to_string())
                } else if let Some(text) = item.get("content").and_then(|t| t.as_str()) {
                    Some(text.to_string())
                } else {
                    None
                }
            })
            .collect::<Vec<_>>()
            .join("\n"),
        _ => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_connector_source_id() {
        let connector = OpenCodeConnector::default();
        assert_eq!(connector.source_id(), "opencode");
        assert_eq!(connector.display_name(), "OpenCode");
    }

    #[test]
    fn test_parse_entry() {
        let json = r#"{"sessionId":"test-123","timestamp":"2025-01-15T10:30:00.000Z","cwd":"/home/user/project","message":{"role":"user","content":"Hello"}}"#;
        let entry: OpenCodeEntry = serde_json::from_str(json).unwrap();
        assert_eq!(entry.session_id, Some("test-123".to_string()));
        assert_eq!(entry.message.unwrap().role, "user");
    }
}
