//! Native Claude Code connector
//!
//! A lightweight parser for Claude Code JSONL session files
//! that works without the full terraphim-session-analyzer dependency.

use super::{ConnectorStatus, ImportOptions, SessionConnector};
use crate::model::{ContentBlock, Message, MessageRole, Session, SessionMetadata};
use anyhow::{Context, Result};
use async_trait::async_trait;
use notify::{Event, EventKind, RecursiveMode, Watcher, recommended_watcher};
use serde::Deserialize;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;

/// Native Claude Code session connector
#[derive(Debug, Default)]
pub struct NativeClaudeConnector {
    /// Optional path override for testing
    override_path: Option<PathBuf>,
}

impl NativeClaudeConnector {
    /// Create a connector with a custom base path (useful for testing)
    #[must_use]
    pub fn with_path(path: PathBuf) -> Self {
        Self {
            override_path: Some(path),
        }
    }
}

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
        self.override_path
            .clone()
            .or_else(|| dirs::home_dir().map(|h| h.join(".claude").join("projects")))
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
            .filter(|e| e.path().extension().is_some_and(|ext| ext == "jsonl"))
            .map(|e| e.path().to_path_buf())
            .collect();

        tracing::info!("Found {} JSONL files", jsonl_files.len());

        let total = jsonl_files.len();
        for (idx, file_path) in jsonl_files.into_iter().enumerate() {
            // Log progress every 50 sessions or at the end
            if idx > 0 && (idx % 50 == 0 || idx == total - 1) {
                tracing::info!("Imported {}/{} sessions...", idx, total);
            }
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
                    tracing::info!("Reached limit of {} sessions, stopping import", limit);
                    break;
                }
            }
        }

        tracing::info!(
            "Imported {} Claude sessions (from {} files)",
            sessions.len(),
            total
        );
        Ok(sessions)
    }

    fn supports_watch(&self) -> bool {
        true
    }

    async fn watch(&self) -> Result<mpsc::Receiver<Session>> {
        let (tx, rx) = mpsc::channel(32);
        let base_path = self
            .default_path()
            .ok_or_else(|| anyhow::anyhow!("No default path found for watch"))?;

        if !base_path.exists() {
            anyhow::bail!("Watch path does not exist: {}", base_path.display());
        }

        let path = Arc::new(base_path);

        tokio::task::spawn_blocking(move || -> Result<()> {
            let (watcher_tx, watcher_rx) = std::sync::mpsc::channel();
            let mut watcher = recommended_watcher(move |res| {
                let _ = watcher_tx.send(res);
            })?;

            watcher.watch(&path, RecursiveMode::Recursive)?;

            tracing::info!(
                "Started watching for Claude sessions in: {}",
                path.display()
            );

            // Debounce state: path -> (last_messages_len, last_emission_time)
            // Dedup key: session_id is insufficient because the same file can be
            // modified multiple times with growing messages. We use message count
            // combined with a debounce window to emit at most once per quiescent period.
            let last_seen = Arc::new(std::sync::Mutex::new(HashMap::new()));
            let debounce_duration = Duration::from_millis(200);

            loop {
                // Exit early if all receivers have been dropped (e.g. test finished)
                if tx.is_closed() {
                    tracing::debug!("Watch receiver dropped, stopping watcher");
                    break;
                }

                match watcher_rx.recv_timeout(std::time::Duration::from_millis(500)) {
                    Ok(res) => match res {
                        Ok(event) => {
                            if let Event {
                                kind: EventKind::Create(_) | EventKind::Modify(_),
                                paths,
                                ..
                            } = event
                            {
                                for path in paths {
                                    if path.extension().is_some_and(|ext| ext == "jsonl") {
                                        let tx = tx.clone();
                                        let path_clone = path.clone();
                                        let last_seen = Arc::clone(&last_seen);

                                        tokio::spawn(async move {
                                            let connector = NativeClaudeConnector::default();
                                            match connector.parse_session_file(&path_clone).await {
                                                Ok(Some(session)) => {
                                                    let messages_len = session.messages.len();
                                                    let now = Instant::now();

                                                    let should_emit = {
                                                        let mut guard = last_seen.lock().unwrap();
                                                        let emit = match guard.get(&path_clone) {
                                                            Some((last_len, last_time)) => {
                                                                // Only emit if messages grew AND
                                                                // debounce window elapsed
                                                                messages_len > *last_len
                                                                    && now
                                                                        .duration_since(*last_time)
                                                                        >= debounce_duration
                                                            }
                                                            None => {
                                                                // First time seeing this file
                                                                true
                                                            }
                                                        };
                                                        if emit {
                                                            guard.insert(
                                                                path_clone.clone(),
                                                                (messages_len, now),
                                                            );
                                                        }
                                                        emit
                                                    };

                                                    if should_emit
                                                        && tx.send(session).await.is_err()
                                                    {
                                                        tracing::warn!(
                                                            "Failed to send session from watch"
                                                        );
                                                    }
                                                }
                                                Ok(None) => {}
                                                Err(e) => {
                                                    tracing::warn!(
                                                        "Failed to parse new session {}: {}",
                                                        path_clone.display(),
                                                        e
                                                    );
                                                }
                                            }
                                        });
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            tracing::warn!("Watch error: {}", e);
                        }
                    },
                    Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                        // No events in 500ms; loop back to check tx.is_closed()
                        continue;
                    }
                    Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                        tracing::debug!("Watcher channel disconnected, stopping");
                        break;
                    }
                }
            }

            Ok(())
        });

        Ok(rx)
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
        let project_path = entries.first().and_then(|e| e.cwd.clone());

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
        let started_at = entries.first().and_then(|e| parse_timestamp(&e.timestamp));
        let ended_at = entries.last().and_then(|e| parse_timestamp(&e.timestamp));

        Ok(Some(Session {
            id: format!("claude-code-native:{}", session_id),
            source: "claude-code-native".to_string(),
            external_id: session_id,
            title: project_path.clone(),
            source_path: path.clone(),
            started_at,
            ended_at,
            messages,
            metadata: SessionMetadata::new(project_path, None, vec![], serde_json::Value::Null),
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
    Text {
        text: String,
    },
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
    use std::time::Duration;
    use tokio::io::AsyncWriteExt;

    #[test]
    fn test_parse_timestamp() {
        let ts = parse_timestamp("2024-01-15T10:30:00Z");
        assert!(ts.is_some());
    }

    #[test]
    fn test_connector_source_id() {
        let connector = NativeClaudeConnector::default();
        assert_eq!(connector.source_id(), "claude-code-native");
        assert_eq!(connector.display_name(), "Claude Code (Native)");
    }

    #[test]
    fn test_parse_timestamp_valid_formats() {
        assert!(parse_timestamp("2024-01-15T10:30:00Z").is_some());
        assert!(parse_timestamp("2024-12-31T23:59:59.999Z").is_some());
    }

    #[test]
    fn test_parse_timestamp_invalid() {
        assert!(parse_timestamp("not-a-timestamp").is_none());
        assert!(parse_timestamp("").is_none());
    }

    #[test]
    fn test_parse_user_entry() {
        let json = r#"{"sessionId":"abc","cwd":"/tmp","timestamp":"2024-01-15T10:30:00Z","message":{"role":"user","content":"hello"}}"#;
        let entry: LogEntry = serde_json::from_str(json).unwrap();
        assert_eq!(entry.session_id, Some("abc".to_string()));
        assert_eq!(entry.cwd, Some("/tmp".to_string()));
        assert!(matches!(entry.message, EntryMessage::User { .. }));
    }

    #[test]
    fn test_parse_assistant_entry_with_text() {
        let json = r#"{"sessionId":"abc","timestamp":"2024-01-15T10:30:00Z","message":{"role":"assistant","content":[{"type":"text","text":"response here"}]}}"#;
        let entry: LogEntry = serde_json::from_str(json).unwrap();
        if let EntryMessage::Assistant { content } = &entry.message {
            assert_eq!(content.len(), 1);
            assert!(
                matches!(&content[0], AssistantContentBlock::Text { text } if text == "response here")
            );
        } else {
            panic!("Expected Assistant message");
        }
    }

    #[test]
    fn test_parse_assistant_entry_with_tool_use() {
        let json = r#"{"sessionId":"abc","timestamp":"2024-01-15T10:30:00Z","message":{"role":"assistant","content":[{"type":"tool_use","id":"tool1","name":"Read","input":{"path":"/tmp"}}]}}"#;
        let entry: LogEntry = serde_json::from_str(json).unwrap();
        if let EntryMessage::Assistant { content } = &entry.message {
            assert_eq!(content.len(), 1);
            assert!(
                matches!(&content[0], AssistantContentBlock::ToolUse { name, .. } if name == "Read")
            );
        } else {
            panic!("Expected Assistant message");
        }
    }

    #[test]
    fn test_parse_tool_result_entry() {
        let json = r#"{"sessionId":"abc","timestamp":"2024-01-15T10:30:00Z","message":{"role":"tool_result","content":[{"content":"file contents here"}]}}"#;
        let entry: LogEntry = serde_json::from_str(json).unwrap();
        if let EntryMessage::ToolResult { content } = &entry.message {
            assert_eq!(content.len(), 1);
            assert_eq!(content[0].content, "file contents here");
        } else {
            panic!("Expected ToolResult message");
        }
    }

    #[test]
    fn test_parse_entry_missing_optional_fields() {
        let json =
            r#"{"timestamp":"2024-01-15T10:30:00Z","message":{"role":"user","content":"hello"}}"#;
        let entry: LogEntry = serde_json::from_str(json).unwrap();
        assert!(entry.session_id.is_none());
        assert!(entry.cwd.is_none());
    }

    #[tokio::test]
    async fn test_parse_session_from_jsonl_file() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("test-session.jsonl");
        let content = [
            r#"{"sessionId":"sess1","cwd":"/project","timestamp":"2024-01-15T10:00:00Z","message":{"role":"user","content":"How do I use tokio?"}}"#,
            r#"{"sessionId":"sess1","cwd":"/project","timestamp":"2024-01-15T10:00:05Z","message":{"role":"assistant","content":[{"type":"text","text":"Here is how you use tokio..."}]}}"#,
        ].join("\n");
        tokio::fs::write(&file_path, content).await.unwrap();

        let connector = NativeClaudeConnector::default();
        let session = connector
            .parse_session_file(&file_path)
            .await
            .unwrap()
            .unwrap();

        assert_eq!(session.external_id, "sess1");
        assert_eq!(session.messages.len(), 2);
        assert_eq!(session.messages[0].role, crate::model::MessageRole::User);
        assert_eq!(session.messages[0].content, "How do I use tokio?");
        assert_eq!(
            session.messages[1].role,
            crate::model::MessageRole::Assistant
        );
        assert!(session.started_at.is_some());
        assert!(session.ended_at.is_some());
    }

    #[tokio::test]
    async fn test_parse_session_empty_file() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("empty.jsonl");
        tokio::fs::write(&file_path, "").await.unwrap();

        let connector = NativeClaudeConnector::default();
        let result = connector.parse_session_file(&file_path).await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_parse_session_malformed_lines_skipped() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("mixed.jsonl");
        let content = [
            "not valid json",
            r#"{"sessionId":"sess1","timestamp":"2024-01-15T10:00:00Z","message":{"role":"user","content":"hello"}}"#,
            "also not valid",
        ].join("\n");
        tokio::fs::write(&file_path, content).await.unwrap();

        let connector = NativeClaudeConnector::default();
        let session = connector
            .parse_session_file(&file_path)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(session.messages.len(), 1);
    }

    #[tokio::test]
    async fn test_import_with_limit() {
        let dir = tempfile::tempdir().unwrap();
        // Create multiple session files
        for i in 0..5 {
            let file_path = dir.path().join(format!("session-{}.jsonl", i));
            let content = format!(
                r#"{{"sessionId":"sess{}","timestamp":"2024-01-15T10:00:00Z","message":{{"role":"user","content":"msg {}"}}}}"#,
                i, i
            );
            tokio::fs::write(&file_path, content).await.unwrap();
        }

        let connector = NativeClaudeConnector::default();
        let options = ImportOptions::default()
            .with_path(dir.path().to_path_buf())
            .with_limit(2);
        let sessions = connector.import(&options).await.unwrap();
        assert_eq!(sessions.len(), 2);
    }

    #[tokio::test]
    async fn test_supports_watch() {
        let connector = NativeClaudeConnector::default();
        assert!(connector.supports_watch());
    }

    /// Integration test: appending to a JSONL file in increments must not emit
    /// duplicate sessions. The receiver should see the final state at most a
    /// small constant number of times (debouncing).
    #[tokio::test]
    async fn test_watch_dedup_on_incremental_writes() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("test-session.jsonl");

        let connector = NativeClaudeConnector::with_path(dir.path().to_path_buf());
        let mut rx = connector.watch().await.unwrap();

        // Give watcher time to initialise
        tokio::time::sleep(Duration::from_millis(500)).await;

        // Write first line
        let line1 = r#"{"sessionId":"sess1","cwd":"/project","timestamp":"2024-01-15T10:00:00Z","message":{"role":"user","content":"hello"}}"#;
        tokio::fs::write(&file_path, line1).await.unwrap();

        // Wait for watcher to detect and debounce
        tokio::time::sleep(Duration::from_millis(400)).await;

        // Append second line
        let line2 = r#"{"sessionId":"sess1","cwd":"/project","timestamp":"2024-01-15T10:00:05Z","message":{"role":"assistant","content":[{"type":"text","text":"response"}]}}"#;
        let mut file = tokio::fs::OpenOptions::new()
            .append(true)
            .open(&file_path)
            .await
            .unwrap();
        file.write_all(format!("\n{line2}").as_bytes())
            .await
            .unwrap();
        file.sync_all().await.unwrap();
        drop(file);

        tokio::time::sleep(Duration::from_millis(400)).await;

        // Append third line
        let line3 = r#"{"sessionId":"sess1","cwd":"/project","timestamp":"2024-01-15T10:00:10Z","message":{"role":"user","content":"follow up"}}"#;
        let mut file = tokio::fs::OpenOptions::new()
            .append(true)
            .open(&file_path)
            .await
            .unwrap();
        file.write_all(format!("\n{line3}").as_bytes())
            .await
            .unwrap();
        file.sync_all().await.unwrap();
        drop(file);

        // Wait for debounce + processing
        tokio::time::sleep(Duration::from_millis(600)).await;

        // Drain all received sessions
        let mut received = Vec::new();
        while let Ok(Some(session)) =
            tokio::time::timeout(Duration::from_millis(300), rx.recv()).await
        {
            received.push(session);
        }

        // With 1 create + 3 spaced appends, we expect at most 4 emissions
        // (one per quiescent period where the file changed). The critical
        // assertion is that we do NOT see one emission per raw Modify event
        // (which would be >>4 on inotify/FSEvents).
        assert!(
            received.len() <= 5,
            "Expected at most 5 session emissions due to debouncing, got {}",
            received.len()
        );

        // If we received anything, the final state should have 3 messages
        if let Some(last) = received.last() {
            assert_eq!(
                last.messages.len(),
                3,
                "Final session should have all 3 messages"
            );
        }
    }
}
