//! Cursor IDE session connector
//!
//! Parses Cursor's SQLite state.vscdb databases to extract chat sessions.
//! Based on CASS (Coding Agent Session Search) implementation.
//!
//! ## Storage Locations
//!
//! - macOS: `~/Library/Application Support/Cursor/User/`
//! - Linux: `~/.config/Cursor/User/`
//! - Windows: `%APPDATA%/Cursor/User/`
//!
//! ## Database Structure
//!
//! Cursor stores data in `state.vscdb` SQLite databases:
//! - `globalStorage/state.vscdb` - Global chat history
//! - `workspaceStorage/{id}/state.vscdb` - Workspace-specific chats

use super::{
    ConnectorStatus, ImportOptions, NormalizedMessage, NormalizedSession, SessionConnector,
};
use anyhow::{Context, Result};
use rusqlite::Connection;
use serde::Deserialize;
use std::collections::HashSet;
use std::path::PathBuf;
use tracing::{debug, info, warn};

/// Cursor IDE session connector
#[derive(Debug, Default)]
pub struct CursorConnector;

impl SessionConnector for CursorConnector {
    fn source_id(&self) -> &str {
        "cursor"
    }

    fn display_name(&self) -> &str {
        "Cursor IDE"
    }

    fn detect(&self) -> ConnectorStatus {
        if let Some(path) = self.default_path() {
            if path.exists() {
                // Count state.vscdb files
                let count = walkdir::WalkDir::new(&path)
                    .max_depth(4)
                    .into_iter()
                    .filter_map(|e| e.ok())
                    .filter(|e| {
                        e.path()
                            .file_name()
                            .is_some_and(|name| name == "state.vscdb")
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
        #[cfg(target_os = "macos")]
        {
            home::home_dir().map(|h| {
                h.join("Library")
                    .join("Application Support")
                    .join("Cursor")
                    .join("User")
            })
        }

        #[cfg(target_os = "linux")]
        {
            home::home_dir().map(|h| h.join(".config").join("Cursor").join("User"))
        }

        #[cfg(target_os = "windows")]
        {
            std::env::var("APPDATA")
                .ok()
                .map(|appdata| PathBuf::from(appdata).join("Cursor").join("User"))
        }

        #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
        {
            None
        }
    }

    fn import(&self, options: &ImportOptions) -> Result<Vec<NormalizedSession>> {
        let base_path = options
            .path
            .clone()
            .or_else(|| self.default_path())
            .ok_or_else(|| anyhow::anyhow!("No path specified and default not found"))?;

        info!("Importing Cursor sessions from: {}", base_path.display());

        let mut sessions = Vec::new();
        let mut seen_ids = HashSet::new();

        // Find all state.vscdb files
        let db_files: Vec<PathBuf> = walkdir::WalkDir::new(&base_path)
            .max_depth(4)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.path()
                    .file_name()
                    .is_some_and(|name| name == "state.vscdb")
            })
            .map(|e| e.path().to_path_buf())
            .collect();

        info!("Found {} Cursor databases", db_files.len());

        for db_path in db_files {
            match self.parse_database(&db_path, &mut seen_ids) {
                Ok(mut db_sessions) => {
                    sessions.append(&mut db_sessions);
                }
                Err(e) => {
                    warn!("Failed to parse {}: {}", db_path.display(), e);
                }
            }

            // Apply limit if specified
            if let Some(limit) = options.limit {
                if sessions.len() >= limit {
                    sessions.truncate(limit);
                    break;
                }
            }
        }

        info!("Imported {} Cursor sessions", sessions.len());
        Ok(sessions)
    }
}

impl CursorConnector {
    /// Parse a single state.vscdb database
    fn parse_database(
        &self,
        db_path: &PathBuf,
        seen_ids: &mut HashSet<String>,
    ) -> Result<Vec<NormalizedSession>> {
        debug!("Parsing database: {}", db_path.display());

        let conn = Connection::open(db_path)
            .with_context(|| format!("Failed to open database: {}", db_path.display()))?;

        let mut sessions = Vec::new();

        // Try ComposerData format (newer Cursor versions)
        sessions.extend(self.parse_composer_data(&conn, db_path, seen_ids)?);

        // Try legacy ItemTable format (older Cursor versions)
        sessions.extend(self.parse_legacy_format(&conn, db_path, seen_ids)?);

        Ok(sessions)
    }

    /// Parse ComposerData format (newer Cursor)
    fn parse_composer_data(
        &self,
        conn: &Connection,
        db_path: &PathBuf,
        seen_ids: &mut HashSet<String>,
    ) -> Result<Vec<NormalizedSession>> {
        let mut sessions = Vec::new();

        // Check if cursorDiskKV table exists
        let table_exists: bool = conn
            .prepare("SELECT name FROM sqlite_master WHERE type='table' AND name='cursorDiskKV'")?
            .exists([])?;

        if !table_exists {
            return Ok(sessions);
        }

        let mut stmt =
            conn.prepare("SELECT key, value FROM cursorDiskKV WHERE key LIKE 'composerData:%'")?;

        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;

        for row in rows {
            let (key, value) = row?;
            let composer_id = key.strip_prefix("composerData:").unwrap_or(&key);

            // Skip duplicates
            if !seen_ids.insert(composer_id.to_string()) {
                continue;
            }

            match serde_json::from_str::<ComposerData>(&value) {
                Ok(data) => {
                    if let Some(session) = self.composer_to_session(composer_id, data, db_path) {
                        sessions.push(session);
                    }
                }
                Err(e) => {
                    debug!("Failed to parse composer data {}: {}", composer_id, e);
                }
            }
        }

        Ok(sessions)
    }

    /// Parse legacy ItemTable format (older Cursor)
    fn parse_legacy_format(
        &self,
        conn: &Connection,
        db_path: &PathBuf,
        seen_ids: &mut HashSet<String>,
    ) -> Result<Vec<NormalizedSession>> {
        let mut sessions = Vec::new();

        // Check if ItemTable exists
        let table_exists: bool = conn
            .prepare("SELECT name FROM sqlite_master WHERE type='table' AND name='ItemTable'")?
            .exists([])?;

        if !table_exists {
            return Ok(sessions);
        }

        let mut stmt = conn.prepare(
            "SELECT key, value FROM ItemTable WHERE key LIKE '%aichat%chatdata%' OR key LIKE '%composer%'",
        )?;

        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, Vec<u8>>(1)?))
        })?;

        for row in rows {
            let (key, value) = row?;

            // Skip duplicates
            if !seen_ids.insert(key.clone()) {
                continue;
            }

            // Try to parse as UTF-8 JSON
            if let Ok(value_str) = String::from_utf8(value) {
                match serde_json::from_str::<LegacyChatData>(&value_str) {
                    Ok(data) => {
                        if let Some(session) = self.legacy_to_session(&key, data, db_path) {
                            sessions.push(session);
                        }
                    }
                    Err(e) => {
                        debug!("Failed to parse legacy chat data {}: {}", key, e);
                    }
                }
            }
        }

        Ok(sessions)
    }

    /// Convert ComposerData to NormalizedSession
    fn composer_to_session(
        &self,
        id: &str,
        data: ComposerData,
        db_path: &PathBuf,
    ) -> Option<NormalizedSession> {
        let tabs = data.tabs.unwrap_or_default();
        if tabs.is_empty() {
            return None;
        }

        let mut all_messages = Vec::new();

        for tab in &tabs {
            for (idx, bubble) in tab.bubbles.iter().enumerate() {
                let role = normalize_role(&bubble.role);
                let content = bubble
                    .text
                    .clone()
                    .or(bubble.content.clone())
                    .or(bubble.message.clone())
                    .unwrap_or_default();

                if content.is_empty() {
                    continue;
                }

                let created_at = bubble
                    .timestamp
                    .and_then(|ts| jiff::Timestamp::from_millisecond(ts as i64).ok());

                all_messages.push(NormalizedMessage {
                    idx,
                    role,
                    author: bubble.model.clone(),
                    content,
                    created_at,
                    extra: serde_json::Value::Null,
                });
            }
        }

        if all_messages.is_empty() {
            return None;
        }

        // Derive title from first message or model
        let title = all_messages
            .first()
            .map(|m| {
                if m.content.len() > 50 {
                    format!("{}...", &m.content[..50])
                } else {
                    m.content.clone()
                }
            })
            .or_else(|| tabs.first().and_then(|t| t.model.clone()));

        let started_at = all_messages.first().and_then(|m| m.created_at);
        let ended_at = all_messages.last().and_then(|m| m.created_at);

        Some(NormalizedSession {
            source: "cursor".to_string(),
            external_id: id.to_string(),
            title,
            source_path: db_path.clone(),
            started_at,
            ended_at,
            messages: all_messages,
            metadata: serde_json::json!({
                "unified_mode": data.unified_mode,
            }),
        })
    }

    /// Convert legacy chat data to NormalizedSession
    fn legacy_to_session(
        &self,
        key: &str,
        data: LegacyChatData,
        db_path: &PathBuf,
    ) -> Option<NormalizedSession> {
        let messages: Vec<NormalizedMessage> = data
            .messages
            .unwrap_or_default()
            .into_iter()
            .enumerate()
            .map(|(idx, msg)| NormalizedMessage {
                idx,
                role: normalize_role(&msg.role),
                author: msg.model,
                content: msg.content.unwrap_or_default(),
                created_at: msg
                    .timestamp
                    .and_then(|ts| jiff::Timestamp::from_millisecond(ts as i64).ok()),
                extra: serde_json::Value::Null,
            })
            .filter(|m| !m.content.is_empty())
            .collect();

        if messages.is_empty() {
            return None;
        }

        let title = messages.first().map(|m| {
            if m.content.len() > 50 {
                format!("{}...", &m.content[..50])
            } else {
                m.content.clone()
            }
        });

        Some(NormalizedSession {
            source: "cursor".to_string(),
            external_id: key.to_string(),
            title,
            source_path: db_path.clone(),
            started_at: messages.first().and_then(|m| m.created_at),
            ended_at: messages.last().and_then(|m| m.created_at),
            messages,
            metadata: serde_json::Value::Null,
        })
    }
}

/// Normalize role strings to standard values
fn normalize_role(role: &str) -> String {
    match role.to_lowercase().as_str() {
        "user" | "human" => "user".to_string(),
        "assistant" | "ai" | "bot" | "model" => "assistant".to_string(),
        "system" => "system".to_string(),
        _ => role.to_lowercase(),
    }
}

// JSON structures for Cursor data formats

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ComposerData {
    tabs: Option<Vec<ComposerTab>>,
    unified_mode: Option<bool>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ComposerTab {
    bubbles: Vec<Bubble>,
    model: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Bubble {
    role: String,
    text: Option<String>,
    content: Option<String>,
    message: Option<String>,
    timestamp: Option<u64>,
    model: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LegacyChatData {
    messages: Option<Vec<LegacyMessage>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LegacyMessage {
    role: String,
    content: Option<String>,
    timestamp: Option<u64>,
    model: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_role() {
        assert_eq!(normalize_role("user"), "user");
        assert_eq!(normalize_role("User"), "user");
        assert_eq!(normalize_role("human"), "user");
        assert_eq!(normalize_role("assistant"), "assistant");
        assert_eq!(normalize_role("AI"), "assistant");
        assert_eq!(normalize_role("bot"), "assistant");
        assert_eq!(normalize_role("system"), "system");
        assert_eq!(normalize_role("other"), "other");
    }

    #[test]
    fn test_connector_source_id() {
        let connector = CursorConnector;
        assert_eq!(connector.source_id(), "cursor");
        assert_eq!(connector.display_name(), "Cursor IDE");
    }
}
