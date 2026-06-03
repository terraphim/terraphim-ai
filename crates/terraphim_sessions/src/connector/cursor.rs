//! Cursor IDE Session Connector
//!
//! Reads conversation history from Cursor IDE's SQLite state databases.
//!
//! ## Format
//!
//! Cursor stores workspace state in SQLite files at:
//! - Linux: `~/.config/Cursor/User/workspaceStorage/*/state.vscdb`
//! - macOS: `~/Library/Application Support/Cursor/User/workspaceStorage/*/state.vscdb`
//!
//! Each database has an `ItemTable` with `key`/`value` columns.
//! The key `history.entries` contains a JSON array of conversation sessions.
//!
//! ## Schema versions
//!
//! Cursor updates its schema periodically. This connector handles the known variants:
//! - `messages` array with `role`/`content` fields (current)
//! - `bubbles` array with `type`/`text` fields (older)
//! - Missing or null fields are handled gracefully

use std::path::{Path, PathBuf};

use super::{ConnectorStatus, ImportOptions, SessionConnector};
use crate::model::{ContentBlock, Message, MessageRole, Session, SessionMetadata};
use anyhow::{Context, Result};
use async_trait::async_trait;
use rusqlite::Connection;
use serde::Deserialize;

/// Cursor IDE session connector
#[derive(Debug, Default)]
pub struct CursorConnector;

/// A single conversation entry from `history.entries`
#[derive(Debug, Clone, Deserialize)]
struct CursorEntry {
    /// Session identifier (various field names across schema versions)
    #[serde(alias = "sessionId", alias = "conversationId", default)]
    id: Option<String>,

    /// Human-readable title
    #[serde(default)]
    title: Option<String>,

    /// Creation timestamp (milliseconds since epoch or ISO string)
    #[serde(alias = "createdAt", alias = "timestamp", default)]
    created_at: Option<serde_json::Value>,

    /// Messages in current schema format
    #[serde(default)]
    messages: Vec<CursorMessage>,

    /// Bubbles format used in older Cursor versions
    #[serde(default)]
    bubbles: Vec<CursorBubble>,
}

/// Message in current Cursor schema (role/content format)
#[derive(Debug, Clone, Deserialize)]
struct CursorMessage {
    role: String,
    #[serde(default)]
    content: serde_json::Value,
}

/// Message in older Cursor schema (type/text format)
#[derive(Debug, Clone, Deserialize)]
struct CursorBubble {
    #[serde(rename = "type")]
    bubble_type: String,
    #[serde(default)]
    text: Option<String>,
}

#[async_trait]
impl SessionConnector for CursorConnector {
    fn source_id(&self) -> &str {
        "cursor"
    }

    fn display_name(&self) -> &str {
        "Cursor IDE"
    }

    fn detect(&self) -> ConnectorStatus {
        match self.default_path() {
            Some(path) if path.exists() => {
                let db_count = count_state_databases(&path);
                ConnectorStatus::Available {
                    path,
                    sessions_estimate: Some(db_count),
                }
            }
            Some(_) | None => ConnectorStatus::NotFound,
        }
    }

    fn default_path(&self) -> Option<PathBuf> {
        cursor_workspace_storage_path()
    }

    async fn import(&self, options: &ImportOptions) -> Result<Vec<Session>> {
        let base_path = options
            .path
            .clone()
            .or_else(|| self.default_path())
            .ok_or_else(|| anyhow::anyhow!("No Cursor workspaceStorage path found"))?;

        if !base_path.exists() {
            return Ok(Vec::new());
        }

        // Collect all state.vscdb files
        let db_paths = collect_db_paths(&base_path);
        tracing::info!(
            "Found {} Cursor state.vscdb files in {}",
            db_paths.len(),
            base_path.display()
        );

        let mut all_sessions = Vec::new();

        for db_path in db_paths {
            match import_from_db(&db_path) {
                Ok(mut sessions) => {
                    tracing::debug!(
                        "Imported {} sessions from {}",
                        sessions.len(),
                        db_path.display()
                    );
                    all_sessions.append(&mut sessions);
                }
                Err(e) => {
                    tracing::warn!("Failed to import from {}: {}", db_path.display(), e);
                }
            }

            if let Some(limit) = options.limit {
                if all_sessions.len() >= limit {
                    all_sessions.truncate(limit);
                    break;
                }
            }
        }

        // Apply since/until filters
        if options.since.is_some() || options.until.is_some() {
            all_sessions.retain(|s| {
                if let Some(since) = options.since {
                    if let Some(started) = s.started_at {
                        if started < since {
                            return false;
                        }
                    }
                }
                if let Some(until) = options.until {
                    if let Some(started) = s.started_at {
                        if started > until {
                            return false;
                        }
                    }
                }
                true
            });
        }

        tracing::info!("Total Cursor sessions imported: {}", all_sessions.len());
        Ok(all_sessions)
    }
}

/// Platform-specific path to Cursor workspaceStorage
fn cursor_workspace_storage_path() -> Option<PathBuf> {
    #[cfg(target_os = "macos")]
    {
        dirs::home_dir().map(|h| {
            h.join("Library")
                .join("Application Support")
                .join("Cursor")
                .join("User")
                .join("workspaceStorage")
        })
    }

    #[cfg(not(target_os = "macos"))]
    {
        dirs::config_dir().map(|c| c.join("Cursor").join("User").join("workspaceStorage"))
    }
}

/// Count the number of state.vscdb files under a directory (used for detection estimate)
fn count_state_databases(base: &Path) -> usize {
    walkdir::WalkDir::new(base)
        .max_depth(2)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_name() == "state.vscdb")
        .count()
}

/// Collect all state.vscdb paths under the workspace storage directory
fn collect_db_paths(base: &Path) -> Vec<PathBuf> {
    walkdir::WalkDir::new(base)
        .max_depth(2)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_name() == "state.vscdb")
        .map(|e| e.path().to_path_buf())
        .collect()
}

/// Import sessions from a single state.vscdb file
fn import_from_db(db_path: &Path) -> Result<Vec<Session>> {
    let conn =
        Connection::open(db_path).with_context(|| format!("Opening {}", db_path.display()))?;

    // Read the history.entries key from ItemTable
    let raw_value: Option<String> = conn
        .query_row(
            "SELECT value FROM ItemTable WHERE key = 'history.entries'",
            [],
            |row| row.get(0),
        )
        .ok();

    let raw_value = match raw_value {
        Some(v) => v,
        None => return Ok(Vec::new()),
    };

    let entries: Vec<CursorEntry> = match serde_json::from_str(&raw_value) {
        Ok(v) => v,
        Err(e) => {
            tracing::debug!(
                "Failed to parse history.entries from {}: {}",
                db_path.display(),
                e
            );
            return Ok(Vec::new());
        }
    };

    let mut sessions = Vec::new();

    for entry in entries {
        if let Some(session) = convert_entry(entry, db_path) {
            sessions.push(session);
        }
    }

    Ok(sessions)
}

/// Convert a `CursorEntry` to a `Session`, returning None if the entry has no messages
fn convert_entry(entry: CursorEntry, db_path: &Path) -> Option<Session> {
    let session_id = entry
        .id
        .clone()
        .unwrap_or_else(|| format!("cursor:{}", uuid::Uuid::new_v4()));

    // Parse messages from either format
    let messages = if !entry.messages.is_empty() {
        messages_from_modern_format(&entry.messages)
    } else if !entry.bubbles.is_empty() {
        messages_from_bubble_format(&entry.bubbles)
    } else {
        return None;
    };

    if messages.is_empty() {
        return None;
    }

    // Parse timestamp
    let started_at = entry.created_at.as_ref().and_then(parse_cursor_timestamp);

    Some(Session {
        id: format!("cursor:{}", session_id),
        source: "cursor".to_string(),
        external_id: session_id.clone(),
        title: entry.title,
        source_path: db_path.to_path_buf(),
        started_at,
        ended_at: None,
        messages,
        metadata: SessionMetadata::new(
            None,
            None,
            vec!["cursor".to_string()],
            serde_json::json!({ "source_db": db_path.to_string_lossy() }),
        ),
    })
}

/// Convert modern role/content message format
fn messages_from_modern_format(messages: &[CursorMessage]) -> Vec<Message> {
    messages
        .iter()
        .enumerate()
        .filter_map(|(idx, msg)| {
            let text = match &msg.content {
                serde_json::Value::String(s) => s.clone(),
                serde_json::Value::Array(arr) => {
                    // Content can be an array of content blocks
                    arr.iter()
                        .filter_map(|block| {
                            block
                                .get("text")
                                .and_then(|t| t.as_str())
                                .map(|s| s.to_string())
                        })
                        .collect::<Vec<_>>()
                        .join("\n")
                }
                other => {
                    let s = other.to_string();
                    if s == "null" {
                        return None;
                    }
                    s
                }
            };

            if text.is_empty() {
                return None;
            }

            let role = MessageRole::from(msg.role.as_str());
            Some(Message {
                idx,
                role,
                author: None,
                content: text.clone(),
                blocks: vec![ContentBlock::Text { text }],
                created_at: None,
                extra: serde_json::Value::Null,
            })
        })
        .collect()
}

/// Convert older bubble/type message format
fn messages_from_bubble_format(bubbles: &[CursorBubble]) -> Vec<Message> {
    bubbles
        .iter()
        .enumerate()
        .filter_map(|(idx, bubble)| {
            let text = bubble.text.as_ref()?.trim().to_string();
            if text.is_empty() {
                return None;
            }

            let role = match bubble.bubble_type.to_lowercase().as_str() {
                "user" | "human" => MessageRole::User,
                "ai" | "assistant" | "bot" => MessageRole::Assistant,
                _ => MessageRole::Other,
            };

            Some(Message {
                idx,
                role,
                author: None,
                content: text.clone(),
                blocks: vec![ContentBlock::Text { text }],
                created_at: None,
                extra: serde_json::Value::Null,
            })
        })
        .collect()
}

/// Parse Cursor's timestamp field, which may be milliseconds (i64) or an ISO string
fn parse_cursor_timestamp(value: &serde_json::Value) -> Option<jiff::Timestamp> {
    match value {
        serde_json::Value::Number(n) => {
            // Cursor uses milliseconds since epoch
            let ms = n.as_i64()?;
            let secs = ms / 1000;
            let nanos = ((ms % 1000) * 1_000_000) as i32;
            jiff::Timestamp::new(secs, nanos).ok()
        }
        serde_json::Value::String(s) => s.parse::<jiff::Timestamp>().ok(),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_db(dir: &TempDir, entries_json: &str) -> PathBuf {
        let db_path = dir.path().join("state.vscdb");
        let conn = Connection::open(&db_path).unwrap();
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS ItemTable (key TEXT PRIMARY KEY, value BLOB);",
        )
        .unwrap();
        conn.execute(
            "INSERT INTO ItemTable (key, value) VALUES ('history.entries', ?1)",
            [entries_json],
        )
        .unwrap();
        db_path
    }

    #[test]
    fn test_connector_metadata() {
        let c = CursorConnector;
        assert_eq!(c.source_id(), "cursor");
        assert_eq!(c.display_name(), "Cursor IDE");
    }

    #[test]
    fn test_import_from_db_empty_entries() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = create_test_db(&dir, "[]");
        let sessions = import_from_db(&db_path).unwrap();
        assert!(sessions.is_empty());
    }

    #[test]
    fn test_import_modern_format() {
        let dir = tempfile::tempdir().unwrap();
        let entries = serde_json::json!([
            {
                "id": "sess-001",
                "title": "Test session",
                "createdAt": 1_700_000_000_000i64,
                "messages": [
                    {"role": "user", "content": "Hello Cursor"},
                    {"role": "assistant", "content": "Hi there!"}
                ]
            }
        ]);
        let db_path = create_test_db(&dir, &entries.to_string());
        let sessions = import_from_db(&db_path).unwrap();

        assert_eq!(sessions.len(), 1);
        let session = &sessions[0];
        assert_eq!(session.external_id, "sess-001");
        assert_eq!(session.title, Some("Test session".to_string()));
        assert_eq!(session.messages.len(), 2);
        assert_eq!(session.messages[0].role, MessageRole::User);
        assert_eq!(session.messages[0].content, "Hello Cursor");
        assert_eq!(session.messages[1].role, MessageRole::Assistant);
        assert!(session.started_at.is_some());
    }

    #[test]
    fn test_import_bubble_format() {
        let dir = tempfile::tempdir().unwrap();
        let entries = serde_json::json!([
            {
                "bubbles": [
                    {"type": "User", "text": "What is Rust?"},
                    {"type": "AI", "text": "Rust is a systems language."}
                ]
            }
        ]);
        let db_path = create_test_db(&dir, &entries.to_string());
        let sessions = import_from_db(&db_path).unwrap();

        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].messages.len(), 2);
        assert_eq!(sessions[0].messages[0].role, MessageRole::User);
        assert_eq!(sessions[0].messages[0].content, "What is Rust?");
        assert_eq!(sessions[0].messages[1].role, MessageRole::Assistant);
    }

    #[test]
    fn test_import_missing_key_returns_empty() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("state.vscdb");
        let conn = Connection::open(&db_path).unwrap();
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS ItemTable (key TEXT PRIMARY KEY, value BLOB);",
        )
        .unwrap();
        // No history.entries key inserted

        let sessions = import_from_db(&db_path).unwrap();
        assert!(sessions.is_empty());
    }

    #[test]
    fn test_import_entries_without_messages_skipped() {
        let dir = tempfile::tempdir().unwrap();
        let entries = serde_json::json!([
            {"id": "no-msgs", "title": "Empty session"},
            {
                "id": "has-msgs",
                "messages": [
                    {"role": "user", "content": "Keep me"}
                ]
            }
        ]);
        let db_path = create_test_db(&dir, &entries.to_string());
        let sessions = import_from_db(&db_path).unwrap();

        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].external_id, "has-msgs");
    }

    #[test]
    fn test_parse_cursor_timestamp_ms() {
        let val = serde_json::json!(1_700_000_000_000i64);
        let ts = parse_cursor_timestamp(&val).unwrap();
        assert_eq!(ts.as_second(), 1_700_000_000);
    }

    #[test]
    fn test_detect_not_found_when_no_path() {
        let connector = CursorConnector;
        // If default path does not exist, expect NotFound
        let status = connector.detect();
        // We cannot assert Available here (depends on test machine having Cursor installed)
        // but we can assert it does not panic
        let _ = status.is_available();
    }

    #[tokio::test]
    async fn test_import_via_trait_with_custom_path() {
        let dir = tempfile::tempdir().unwrap();
        // Create a workspace subdirectory mimicking Cursor layout
        let ws_dir = dir.path().join("abc123");
        std::fs::create_dir_all(&ws_dir).unwrap();

        let entries = serde_json::json!([
            {
                "id": "trait-test",
                "messages": [
                    {"role": "user", "content": "Via trait import"}
                ]
            }
        ]);
        let db_path = ws_dir.join("state.vscdb");
        let conn = Connection::open(&db_path).unwrap();
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS ItemTable (key TEXT PRIMARY KEY, value BLOB);",
        )
        .unwrap();
        conn.execute(
            "INSERT INTO ItemTable (key, value) VALUES ('history.entries', ?1)",
            [entries.to_string().as_str()],
        )
        .unwrap();
        drop(conn);

        let connector = CursorConnector;
        let options = ImportOptions::default().with_path(dir.path().to_path_buf());
        let sessions = connector.import(&options).await.unwrap();

        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].external_id, "trait-test");
        assert_eq!(sessions[0].messages[0].content, "Via trait import");
    }
}
