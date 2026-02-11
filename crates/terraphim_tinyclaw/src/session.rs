use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

/// A single chat message in a session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: MessageRole,
    pub content: String,
    pub sender_id: Option<String>,
    pub timestamp: DateTime<Utc>,
    pub metadata: HashMap<String, String>,
}

impl ChatMessage {
    /// Create a new user message.
    pub fn user(content: impl Into<String>, sender_id: impl Into<String>) -> Self {
        Self {
            role: MessageRole::User,
            content: content.into(),
            sender_id: Some(sender_id.into()),
            timestamp: Utc::now(),
            metadata: HashMap::new(),
        }
    }

    /// Create a new assistant message.
    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::Assistant,
            content: content.into(),
            sender_id: None,
            timestamp: Utc::now(),
            metadata: HashMap::new(),
        }
    }

    /// Create a new tool result message.
    pub fn tool(content: impl Into<String>, tool_name: impl Into<String>) -> Self {
        Self {
            role: MessageRole::Tool,
            content: content.into(),
            sender_id: Some(tool_name.into()),
            timestamp: Utc::now(),
            metadata: HashMap::new(),
        }
    }
}

/// Role of a message sender.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum MessageRole {
    User,
    Assistant,
    System,
    Tool,
}

/// A conversation session with message history.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    /// Unique session key (format: "channel:chat_id").
    pub key: String,

    /// Message history.
    pub messages: Vec<ChatMessage>,

    /// Summary of older messages (set after compression).
    pub summary: Option<String>,

    /// Creation timestamp.
    pub created_at: DateTime<Utc>,

    /// Last update timestamp.
    pub updated_at: DateTime<Utc>,

    /// Custom metadata.
    pub metadata: HashMap<String, String>,
}

impl Session {
    /// Create a new session with the given key.
    pub fn new(key: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            key: key.into(),
            messages: Vec::new(),
            summary: None,
            created_at: now,
            updated_at: now,
            metadata: HashMap::new(),
        }
    }

    /// Add a message to the session.
    pub fn add_message(&mut self, msg: ChatMessage) {
        self.messages.push(msg);
        self.updated_at = Utc::now();
    }

    /// Get recent messages up to a maximum count.
    /// Returns the most recent messages first.
    pub fn get_recent_messages(&self, max_count: usize) -> Vec<&ChatMessage> {
        let start = self.messages.len().saturating_sub(max_count);
        self.messages[start..].iter().collect()
    }

    /// Get recent messages as a formatted string for LLM context.
    pub fn format_recent_messages(&self, max_count: usize) -> String {
        let recent = self.get_recent_messages(max_count);
        recent
            .iter()
            .map(|m| format!("{}: {}", m.role.as_str(), m.content))
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Set a summary of older messages.
    pub fn set_summary(&mut self, summary: String) {
        self.summary = Some(summary);
        self.updated_at = Utc::now();
    }

    /// Clear all messages but keep summary.
    pub fn clear_messages(&mut self) {
        self.messages.clear();
        self.updated_at = Utc::now();
    }

    /// Get the total message count.
    pub fn message_count(&self) -> usize {
        self.messages.len()
    }

    /// Check if the session is empty.
    pub fn is_empty(&self) -> bool {
        self.messages.is_empty()
    }
}

impl MessageRole {
    fn as_str(&self) -> &'static str {
        match self {
            Self::User => "user",
            Self::Assistant => "assistant",
            Self::System => "system",
            Self::Tool => "tool",
        }
    }
}

/// Manages sessions with JSONL file persistence.
pub struct SessionManager {
    /// Directory where session files are stored.
    sessions_dir: PathBuf,

    /// In-memory cache of active sessions.
    cache: HashMap<String, Session>,

    /// Maximum number of recent messages to load into memory.
    max_in_memory: usize,
}

impl SessionManager {
    /// Create a new session manager.
    pub fn new(sessions_dir: PathBuf) -> Self {
        // Ensure the sessions directory exists
        if !sessions_dir.exists() {
            std::fs::create_dir_all(&sessions_dir).ok();
        }

        Self {
            sessions_dir,
            cache: HashMap::new(),
            max_in_memory: 200,
        }
    }

    /// Get or create a session.
    pub fn get_or_create(&mut self, key: &str) -> &mut Session {
        if !self.cache.contains_key(key) {
            // Try to load from disk
            let session = self.load(key).unwrap_or_else(|| Session::new(key));
            self.cache.insert(key.to_string(), session);
        }

        self.cache.get_mut(key).unwrap()
    }

    /// Get a session if it exists in cache.
    pub fn get(&self, key: &str) -> Option<&Session> {
        self.cache.get(key)
    }

    /// Save a session to disk.
    pub fn save(&self, session: &Session) -> anyhow::Result<()> {
        let file_path = self.session_file_path(&session.key);
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&file_path)?;

        // Serialize and append as JSON line
        let line = serde_json::to_string(session)?;
        writeln!(file, "{}", line)?;

        log::debug!("Saved session {} to {}", session.key, file_path.display());
        Ok(())
    }

    /// Load a session from disk.
    fn load(&self, key: &str) -> Option<Session> {
        let file_path = self.session_file_path(key);

        if !file_path.exists() {
            return None;
        }

        let file = File::open(&file_path).ok()?;
        let reader = BufReader::new(file);

        // Read the last line (most recent save)
        let last_line = reader.lines().filter_map(|l| l.ok()).last()?;

        serde_json::from_str(&last_line).ok()
    }

    /// Get the file path for a session.
    fn session_file_path(&self, key: &str) -> PathBuf {
        // Sanitize key for filesystem (replace colons with underscores)
        let sanitized = key.replace(':', "_");
        self.sessions_dir.join(format!("{}.jsonl", sanitized))
    }

    /// List all session keys.
    pub fn list_sessions(&self) -> anyhow::Result<Vec<String>> {
        let mut keys = Vec::new();

        if !self.sessions_dir.exists() {
            return Ok(keys);
        }

        for entry in std::fs::read_dir(&self.sessions_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) == Some("jsonl") {
                if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                    // Convert back from sanitized format
                    let key = stem.replace('_', ":");
                    keys.push(key);
                }
            }
        }

        Ok(keys)
    }

    /// Delete a session.
    pub fn delete(&mut self, key: &str) -> anyhow::Result<()> {
        // Remove from cache
        self.cache.remove(key);

        // Remove from disk
        let file_path = self.session_file_path(key);
        if file_path.exists() {
            std::fs::remove_file(&file_path)?;
        }

        Ok(())
    }

    /// Flush all cached sessions to disk.
    pub fn flush_all(&self) -> anyhow::Result<()> {
        for (key, session) in &self.cache {
            self.save(session)?;
            log::debug!("Flushed session {}", key);
        }
        Ok(())
    }

    /// Set the maximum number of messages to keep in memory.
    pub fn with_max_in_memory(mut self, max: usize) -> Self {
        self.max_in_memory = max;
        self
    }
}

/// Information about a stored session.
#[derive(Debug, Clone)]
pub struct SessionInfo {
    pub key: String,
    pub message_count: usize,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_session_add_get_history() {
        let mut session = Session::new("cli:123");

        session.add_message(ChatMessage::user("Hello", "user1"));
        session.add_message(ChatMessage::assistant("Hi there!"));
        session.add_message(ChatMessage::user("How are you?", "user1"));

        assert_eq!(session.message_count(), 3);

        let recent = session.get_recent_messages(2);
        assert_eq!(recent.len(), 2);
        assert_eq!(recent[0].content, "Hi there!");
        assert_eq!(recent[1].content, "How are you?");
    }

    #[test]
    fn test_session_jsonl_persistence() {
        let temp_dir = TempDir::new().unwrap();
        let manager = SessionManager::new(temp_dir.path().to_path_buf());

        // Create and save a session
        let mut session = Session::new("test:chat123");
        session.add_message(ChatMessage::user("Hello", "user1"));
        session.add_message(ChatMessage::assistant("World!"));

        manager.save(&session).unwrap();

        // Load it back
        let loaded = manager.load("test:chat123").unwrap();
        assert_eq!(loaded.key, "test:chat123");
        assert_eq!(loaded.message_count(), 2);
    }

    #[test]
    fn test_session_manager_get_or_create() {
        let temp_dir = TempDir::new().unwrap();
        let mut manager = SessionManager::new(temp_dir.path().to_path_buf());

        // Create a session and add a message
        let mut session = Session::new("new:session");
        session.add_message(ChatMessage::user("Test", "user"));
        manager.save(&session).unwrap();

        // Create new manager instance and load from disk
        let mut manager2 = SessionManager::new(temp_dir.path().to_path_buf());
        let session2 = manager2.get_or_create("new:session");
        assert_eq!(session2.message_count(), 1);
    }

    #[test]
    fn test_session_summary() {
        let mut session = Session::new("test:session");
        session.set_summary("Previous conversation about Rust.".to_string());

        assert_eq!(
            session.summary,
            Some("Previous conversation about Rust.".to_string())
        );
    }

    #[test]
    fn test_session_format_messages() {
        let mut session = Session::new("test:session");
        session.add_message(ChatMessage::user("Hello", "user1"));
        session.add_message(ChatMessage::assistant("Hi!"));

        let formatted = session.format_recent_messages(10);
        assert!(formatted.contains("user:"));
        assert!(formatted.contains("Hello"));
        assert!(formatted.contains("assistant:"));
        assert!(formatted.contains("Hi!"));
    }

    #[test]
    fn test_session_manager_list() {
        let temp_dir = TempDir::new().unwrap();
        let manager = SessionManager::new(temp_dir.path().to_path_buf());

        // Create some sessions
        let session1 = Session::new("cli:123");
        let session2 = Session::new("telegram:456");
        manager.save(&session1).unwrap();
        manager.save(&session2).unwrap();

        let sessions = manager.list_sessions().unwrap();
        assert_eq!(sessions.len(), 2);
        assert!(sessions.contains(&"cli:123".to_string()));
        assert!(sessions.contains(&"telegram:456".to_string()));
    }
}
