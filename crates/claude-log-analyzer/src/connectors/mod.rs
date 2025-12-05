//! Session Connectors for multi-agent support
//!
//! This module provides connectors for various AI coding assistants:
//! - Claude Code (JSONL) - via existing parser
//! - Cursor (SQLite) - via cursor module
//! - Codex (JSONL) - OpenAI Codex CLI
//! - Aider (Markdown) - Aider chat history
//! - OpenCode (JSONL) - OpenCode AI assistant
//!
//! Enable with `--features connectors`

use anyhow::Result;
use std::path::PathBuf;

#[cfg(feature = "connectors")]
pub mod aider;
#[cfg(feature = "connectors")]
pub mod codex;
#[cfg(feature = "connectors")]
pub mod cursor;
#[cfg(feature = "connectors")]
pub mod opencode;

/// Status of a connector's detection
#[derive(Debug, Clone)]
pub enum ConnectorStatus {
    /// Connector found with estimated session count
    Available {
        path: PathBuf,
        sessions_estimate: Option<usize>,
    },
    /// Connector's data directory not found
    NotFound,
    /// Connector found but has errors
    Error(String),
}

/// Trait for session connectors
pub trait SessionConnector: Send + Sync {
    /// Unique identifier for this connector
    fn source_id(&self) -> &str;

    /// Human-readable name
    fn display_name(&self) -> &str;

    /// Check if this connector's data source is available
    fn detect(&self) -> ConnectorStatus;

    /// Get the default data path for this connector
    fn default_path(&self) -> Option<PathBuf>;

    /// Import sessions from this source
    fn import(&self, options: &ImportOptions) -> Result<Vec<NormalizedSession>>;
}

/// Options for importing sessions
#[derive(Debug, Clone, Default)]
pub struct ImportOptions {
    /// Override the default path
    pub path: Option<PathBuf>,
    /// Only import sessions after this timestamp
    pub since: Option<jiff::Timestamp>,
    /// Only import sessions before this timestamp
    pub until: Option<jiff::Timestamp>,
    /// Maximum sessions to import
    pub limit: Option<usize>,
    /// Skip sessions already imported (for incremental updates)
    pub incremental: bool,
}

/// Normalized session from any connector
#[derive(Debug, Clone)]
pub struct NormalizedSession {
    /// Connector source ID
    pub source: String,
    /// Original session ID from the source
    pub external_id: String,
    /// Session title or description
    pub title: Option<String>,
    /// Path to source file/database
    pub source_path: PathBuf,
    /// Session start time
    pub started_at: Option<jiff::Timestamp>,
    /// Session end time
    pub ended_at: Option<jiff::Timestamp>,
    /// Normalized messages
    pub messages: Vec<NormalizedMessage>,
    /// Additional metadata
    pub metadata: serde_json::Value,
}

/// Normalized message from any connector
#[derive(Debug, Clone)]
pub struct NormalizedMessage {
    /// Message index in session
    pub idx: usize,
    /// Role: user, assistant, or system
    pub role: String,
    /// Author identifier (model name, user, etc.)
    pub author: Option<String>,
    /// Message content
    pub content: String,
    /// Message timestamp
    pub created_at: Option<jiff::Timestamp>,
    /// Additional fields
    pub extra: serde_json::Value,
}

/// Registry of available connectors
pub struct ConnectorRegistry {
    connectors: Vec<Box<dyn SessionConnector>>,
}

impl ConnectorRegistry {
    /// Create a new registry with all available connectors
    #[must_use]
    pub fn new() -> Self {
        let mut connectors: Vec<Box<dyn SessionConnector>> = Vec::new();

        // Add Claude Code connector (always available via parser)
        connectors.push(Box::new(ClaudeCodeConnector::default()));

        // Add additional connectors if feature enabled
        #[cfg(feature = "connectors")]
        {
            connectors.push(Box::new(cursor::CursorConnector::default()));
            connectors.push(Box::new(codex::CodexConnector::default()));
            connectors.push(Box::new(aider::AiderConnector::default()));
            connectors.push(Box::new(opencode::OpenCodeConnector::default()));
        }

        Self { connectors }
    }

    /// Get all available connectors
    #[must_use]
    pub fn connectors(&self) -> &[Box<dyn SessionConnector>] {
        &self.connectors
    }

    /// Find a connector by source ID
    #[must_use]
    pub fn get(&self, source_id: &str) -> Option<&dyn SessionConnector> {
        self.connectors
            .iter()
            .find(|c| c.source_id() == source_id)
            .map(|c| c.as_ref())
    }

    /// Detect all available connectors
    pub fn detect_all(&self) -> Vec<(&str, ConnectorStatus)> {
        self.connectors
            .iter()
            .map(|c| (c.source_id(), c.detect()))
            .collect()
    }
}

impl Default for ConnectorRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Claude Code connector (wraps existing parser)
#[derive(Debug, Default)]
pub struct ClaudeCodeConnector;

impl SessionConnector for ClaudeCodeConnector {
    fn source_id(&self) -> &str {
        "claude-code"
    }

    fn display_name(&self) -> &str {
        "Claude Code"
    }

    fn detect(&self) -> ConnectorStatus {
        if let Some(path) = self.default_path() {
            if path.exists() {
                // Count JSONL files
                let count = walkdir::WalkDir::new(&path)
                    .max_depth(3)
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
        home::home_dir().map(|h| h.join(".claude").join("projects"))
    }

    fn import(&self, options: &ImportOptions) -> Result<Vec<NormalizedSession>> {
        use crate::parser::SessionParser;

        let path = options
            .path
            .clone()
            .or_else(|| self.default_path())
            .ok_or_else(|| anyhow::anyhow!("No path specified and default not found"))?;

        let parsers = SessionParser::from_directory(&path)?;
        let mut sessions = Vec::new();

        for parser in parsers {
            // Convert SessionParser to NormalizedSession
            let entries = parser.entries();
            if entries.is_empty() {
                continue;
            }

            let first = entries.first().unwrap();
            let last = entries.last().unwrap();

            let messages: Vec<NormalizedMessage> = entries
                .iter()
                .enumerate()
                .map(|(idx, entry)| {
                    let (role, content) = match &entry.message {
                        crate::models::Message::User { content, .. } => {
                            ("user".to_string(), content.clone())
                        }
                        crate::models::Message::Assistant { content, .. } => {
                            let text = content
                                .iter()
                                .filter_map(|block| match block {
                                    crate::models::ContentBlock::Text { text } => {
                                        Some(text.clone())
                                    }
                                    _ => None,
                                })
                                .collect::<Vec<_>>()
                                .join("\n");
                            ("assistant".to_string(), text)
                        }
                        crate::models::Message::ToolResult { content, .. } => {
                            let text = content
                                .iter()
                                .map(|c| c.content.clone())
                                .collect::<Vec<_>>()
                                .join("\n");
                            ("tool".to_string(), text)
                        }
                    };

                    NormalizedMessage {
                        idx,
                        role,
                        author: None,
                        content,
                        created_at: crate::models::parse_timestamp(&entry.timestamp).ok(),
                        extra: serde_json::Value::Null,
                    }
                })
                .collect();

            sessions.push(NormalizedSession {
                source: "claude-code".to_string(),
                external_id: first.session_id.clone(),
                title: first.cwd.clone(),
                source_path: path.clone(),
                started_at: crate::models::parse_timestamp(&first.timestamp).ok(),
                ended_at: crate::models::parse_timestamp(&last.timestamp).ok(),
                messages,
                metadata: serde_json::json!({
                    "project_path": first.cwd,
                }),
            });
        }

        Ok(sessions)
    }
}
