//! Aider Session Connector
//!
//! Parses chat history from Aider (`.aider.chat.history.md`).
//!
//! ## Format
//!
//! Aider stores chat history as Markdown files in project directories.
//! Format:
//! ```markdown
//! # aider chat started at 2025-06-19 14:32:16
//!
//! > User message or command
//!
//! #### User request heading
//! User message content
//!
//! Assistant response content
//! ```

use anyhow::Result;
use std::path::PathBuf;
use walkdir::WalkDir;

use super::{
    ConnectorStatus, ImportOptions, NormalizedMessage, NormalizedSession, SessionConnector,
};

/// Aider connector
#[derive(Debug, Default)]
pub struct AiderConnector;

impl SessionConnector for AiderConnector {
    fn source_id(&self) -> &str {
        "aider"
    }

    fn display_name(&self) -> &str {
        "Aider"
    }

    fn detect(&self) -> ConnectorStatus {
        // Aider stores files in project directories, search common locations
        let search_paths = vec![
            home::home_dir().map(|h| h.join("projects")),
            home::home_dir(),
            std::env::current_dir().ok(),
        ];

        for base_path in search_paths.into_iter().flatten() {
            let count = WalkDir::new(&base_path)
                .max_depth(4)
                .into_iter()
                .filter_map(|e| e.ok())
                .filter(|e| {
                    e.path()
                        .file_name()
                        .is_some_and(|n| n == ".aider.chat.history.md")
                })
                .count();

            if count > 0 {
                return ConnectorStatus::Available {
                    path: base_path,
                    sessions_estimate: Some(count),
                };
            }
        }

        ConnectorStatus::NotFound
    }

    fn default_path(&self) -> Option<PathBuf> {
        home::home_dir().map(|h| h.join("projects"))
    }

    fn import(&self, options: &ImportOptions) -> Result<Vec<NormalizedSession>> {
        let path = options
            .path
            .clone()
            .or_else(|| self.default_path())
            .ok_or_else(|| anyhow::anyhow!("No path specified and default not found"))?;

        let mut sessions = Vec::new();

        // Find all .aider.chat.history.md files
        for entry in WalkDir::new(&path)
            .max_depth(6)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.path()
                    .file_name()
                    .is_some_and(|n| n == ".aider.chat.history.md")
            })
        {
            if let Some(limit) = options.limit {
                if sessions.len() >= limit {
                    break;
                }
            }

            match self.parse_history_file(entry.path()) {
                Ok(parsed_sessions) => sessions.extend(parsed_sessions),
                Err(e) => {
                    tracing::warn!("Failed to parse {:?}: {}", entry.path(), e);
                    continue;
                }
            }
        }

        Ok(sessions)
    }
}

impl AiderConnector {
    fn parse_history_file(&self, path: &std::path::Path) -> Result<Vec<NormalizedSession>> {
        let content = std::fs::read_to_string(path)?;
        let mut sessions = Vec::new();
        let mut current_session: Option<SessionBuilder> = None;

        for line in content.lines() {
            // New session starts with "# aider chat started at"
            if line.starts_with("# aider chat started at") {
                // Save previous session if exists
                if let Some(builder) = current_session.take() {
                    if let Some(session) = builder.build(path) {
                        sessions.push(session);
                    }
                }

                // Parse timestamp: "# aider chat started at 2025-06-19 14:32:16"
                let timestamp_str = line.trim_start_matches("# aider chat started at").trim();
                current_session = Some(SessionBuilder::new(timestamp_str, path));
            } else if let Some(ref mut builder) = current_session {
                builder.add_line(line);
            }
        }

        // Don't forget the last session
        if let Some(builder) = current_session {
            if let Some(session) = builder.build(path) {
                sessions.push(session);
            }
        }

        Ok(sessions)
    }
}

struct SessionBuilder {
    started_at: Option<jiff::Timestamp>,
    project_path: PathBuf,
    messages: Vec<NormalizedMessage>,
    current_role: Option<String>,
    current_content: Vec<String>,
    msg_idx: usize,
}

impl SessionBuilder {
    fn new(timestamp_str: &str, path: &std::path::Path) -> Self {
        let started_at = parse_aider_timestamp(timestamp_str);
        let project_path = path.parent().unwrap_or(path).to_path_buf();

        Self {
            started_at,
            project_path,
            messages: Vec::new(),
            current_role: None,
            current_content: Vec::new(),
            msg_idx: 0,
        }
    }

    fn add_line(&mut self, line: &str) {
        // User input lines start with ">"
        if line.starts_with("> ") {
            self.flush_message();
            self.current_role = Some("user".to_string());
            self.current_content
                .push(line.trim_start_matches("> ").to_string());
        }
        // User request headings start with "####"
        else if line.starts_with("#### ") {
            self.flush_message();
            self.current_role = Some("user".to_string());
            self.current_content
                .push(line.trim_start_matches("#### ").to_string());
        }
        // Everything else is assistant response (if we're in a session)
        else if !line.is_empty() && self.current_role.is_none() && !self.messages.is_empty() {
            // This is an assistant response
            self.current_role = Some("assistant".to_string());
            self.current_content.push(line.to_string());
        } else if self.current_role.is_some() {
            self.current_content.push(line.to_string());
        }
    }

    fn flush_message(&mut self) {
        if let Some(role) = self.current_role.take() {
            let content = self.current_content.join("\n").trim().to_string();
            if !content.is_empty() {
                self.messages.push(NormalizedMessage {
                    idx: self.msg_idx,
                    role,
                    author: None,
                    content,
                    created_at: None,
                    extra: serde_json::Value::Null,
                });
                self.msg_idx += 1;
            }
            self.current_content.clear();
        }
    }

    fn build(mut self, source_path: &std::path::Path) -> Option<NormalizedSession> {
        self.flush_message();

        if self.messages.is_empty() {
            return None;
        }

        let session_id = format!(
            "aider-{}-{}",
            self.project_path
                .file_name()
                .map(|n| n.to_string_lossy())
                .unwrap_or_default(),
            self.started_at
                .map(|t| t.strftime("%Y%m%d%H%M%S").to_string())
                .unwrap_or_else(|| "unknown".to_string())
        );

        Some(NormalizedSession {
            source: "aider".to_string(),
            external_id: session_id,
            title: Some(
                self.project_path
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| "Unknown".to_string()),
            ),
            source_path: source_path.to_path_buf(),
            started_at: self.started_at,
            ended_at: None,
            messages: self.messages,
            metadata: serde_json::json!({
                "project_path": self.project_path,
            }),
        })
    }
}

fn parse_aider_timestamp(ts: &str) -> Option<jiff::Timestamp> {
    // Format: "2025-06-19 14:32:16" - convert to ISO 8601 for jiff parsing
    let iso_ts = ts.replace(' ', "T");
    jiff::Timestamp::from_str(&format!("{}Z", iso_ts)).ok()
}

use std::str::FromStr;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_connector_source_id() {
        let connector = AiderConnector::default();
        assert_eq!(connector.source_id(), "aider");
        assert_eq!(connector.display_name(), "Aider");
    }

    #[test]
    fn test_parse_timestamp() {
        let ts = parse_aider_timestamp("2025-06-19 14:32:16");
        assert!(ts.is_some());
    }
}
