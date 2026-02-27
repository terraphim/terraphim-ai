//! Session management tools for TinyClaw.
//!
//! These tools allow the agent to introspect and manage conversation sessions,
//! enabling cross-session orchestration and session analysis.

use crate::session::{MessageRole, SessionManager};
use crate::tools::{Tool, ToolError};
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Tool for listing all active sessions.
pub struct SessionListTool {
    sessions: Arc<Mutex<SessionManager>>,
}

impl SessionListTool {
    /// Create a new session list tool.
    pub fn new(sessions: Arc<Mutex<SessionManager>>) -> Self {
        Self { sessions }
    }

    /// List sessions with optional filtering.
    async fn list_sessions(
        &self,
        filter: Option<&str>,
        limit: usize,
    ) -> Result<Vec<SessionInfo>, ToolError> {
        let sessions = self.sessions.lock().await;
        let keys = sessions
            .list_sessions()
            .map_err(|e| ToolError::ExecutionFailed {
                tool: "sessions_list".to_string(),
                message: format!("Failed to list sessions: {}", e),
            })?;

        let mut infos = Vec::new();
        for key in keys {
            // Apply filter if specified
            if let Some(f) = filter {
                if !key.to_lowercase().contains(&f.to_lowercase()) {
                    continue;
                }
            }

            // Try to get session from cache
            if let Some(session) = sessions.get(&key) {
                infos.push(SessionInfo {
                    key: key.clone(),
                    message_count: session.message_count(),
                    created_at: session.created_at.to_rfc3339(),
                    updated_at: session.updated_at.to_rfc3339(),
                    has_summary: session.summary.is_some(),
                });
            }

            if infos.len() >= limit {
                break;
            }
        }

        Ok(infos)
    }
}

#[async_trait]
impl Tool for SessionListTool {
    fn name(&self) -> &str {
        "sessions_list"
    }

    fn description(&self) -> &str {
        "List all active sessions with metadata (message count, last activity, channel)"
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "filter": {
                    "type": "string",
                    "description": "Optional filter by channel or session key substring"
                },
                "limit": {
                    "type": "integer",
                    "description": "Maximum sessions to return",
                    "default": 50,
                    "minimum": 1,
                    "maximum": 100
                }
            }
        })
    }

    async fn execute(&self, args: serde_json::Value) -> Result<String, ToolError> {
        let filter = args["filter"].as_str();
        let limit = args["limit"].as_u64().unwrap_or(50) as usize;

        let sessions = self.list_sessions(filter, limit).await?;

        if sessions.is_empty() {
            return Ok("No sessions found.".to_string());
        }

        // Format as readable table
        let mut output = format!("Found {} session(s):\n\n", sessions.len());
        output.push_str("| Session Key | Messages | Created | Updated | Summary |\n");
        output.push_str("|-------------|----------|---------|---------|---------|\n");

        for info in sessions {
            let summary_indicator = if info.has_summary { "Yes" } else { "No" };
            let created = info
                .created_at
                .split('T')
                .next()
                .unwrap_or(&info.created_at);
            let updated = info
                .updated_at
                .split('T')
                .next()
                .unwrap_or(&info.updated_at);

            output.push_str(&format!(
                "| {} | {} | {} | {} | {} |\n",
                info.key, info.message_count, created, updated, summary_indicator
            ));
        }

        Ok(output)
    }
}

/// Tool for getting session message history.
pub struct SessionHistoryTool {
    sessions: Arc<Mutex<SessionManager>>,
}

impl SessionHistoryTool {
    /// Create a new session history tool.
    pub fn new(sessions: Arc<Mutex<SessionManager>>) -> Self {
        Self { sessions }
    }

    /// Get message history for a session.
    async fn get_history(&self, session_key: &str, limit: usize) -> Result<String, ToolError> {
        let sessions = self.sessions.lock().await;

        // First try to get from cache
        if let Some(session) = sessions.get(session_key) {
            return self.format_history(session, limit);
        }

        // If not in cache, try loading from disk by listing all sessions
        let keys = sessions
            .list_sessions()
            .map_err(|e| ToolError::ExecutionFailed {
                tool: "sessions_history".to_string(),
                message: format!("Failed to list sessions: {}", e),
            })?;

        if !keys.iter().any(|k| k == session_key) {
            return Err(ToolError::ExecutionFailed {
                tool: "sessions_history".to_string(),
                message: format!("Session '{}' not found", session_key),
            });
        }

        Err(ToolError::ExecutionFailed {
            tool: "sessions_history".to_string(),
            message: format!(
                "Session '{}' exists but is not currently loaded",
                session_key
            ),
        })
    }

    fn format_history(
        &self,
        session: &crate::session::Session,
        limit: usize,
    ) -> Result<String, ToolError> {
        let recent = session.get_recent_messages(limit);

        if recent.is_empty() {
            return Ok(format!("Session '{}' has no messages.", session.key));
        }

        let mut output = format!("Session: {}\n", session.key);
        output.push_str(&format!("Total messages: {}\n", session.message_count()));
        if let Some(summary) = &session.summary {
            output.push_str(&format!("Summary: {}\n", summary));
        }
        output.push_str("\n--- Recent Messages ---\n\n");

        for msg in recent {
            let role = match msg.role {
                MessageRole::User => "User",
                MessageRole::Assistant => "Assistant",
                MessageRole::System => "System",
                MessageRole::Tool => "Tool",
            };

            let time = msg.timestamp.format("%Y-%m-%d %H:%M:%S");
            let sender = msg.sender_id.as_deref().unwrap_or(role);

            output.push_str(&format!("[{}] {}:\n", time, sender));

            // Truncate long messages
            let content = if msg.content.len() > 500 {
                format!("{}... (truncated)", &msg.content[..500])
            } else {
                msg.content.clone()
            };

            output.push_str(&format!("{}\n\n", content));
        }

        Ok(output)
    }
}

#[async_trait]
impl Tool for SessionHistoryTool {
    fn name(&self) -> &str {
        "sessions_history"
    }

    fn description(&self) -> &str {
        "Get message history for a specific session by session key"
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "session_key": {
                    "type": "string",
                    "description": "Session key (format: channel:chat_id, e.g., 'cli:default' or 'telegram:123456')"
                },
                "limit": {
                    "type": "integer",
                    "description": "Number of recent messages to retrieve",
                    "default": 20,
                    "minimum": 1,
                    "maximum": 100
                }
            },
            "required": ["session_key"]
        })
    }

    async fn execute(&self, args: serde_json::Value) -> Result<String, ToolError> {
        let session_key =
            args["session_key"]
                .as_str()
                .ok_or_else(|| ToolError::InvalidArguments {
                    tool: "sessions_history".to_string(),
                    message: "Missing required parameter 'session_key'".to_string(),
                })?;

        let limit = args["limit"].as_u64().unwrap_or(20) as usize;

        self.get_history(session_key, limit).await
    }
}

/// Tool for sending a message to another session.
pub struct SessionSendTool {
    sessions: Arc<Mutex<SessionManager>>,
}

impl SessionSendTool {
    /// Create a new session send tool.
    pub fn new(sessions: Arc<Mutex<SessionManager>>) -> Self {
        Self { sessions }
    }
}

#[async_trait]
impl Tool for SessionSendTool {
    fn name(&self) -> &str {
        "sessions_send"
    }

    fn description(&self) -> &str {
        "Send a message to another session (cross-session communication)"
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "session_key": {
                    "type": "string",
                    "description": "Target session key (format: channel:chat_id)"
                },
                "message": {
                    "type": "string",
                    "description": "Message content to send"
                },
                "as_role": {
                    "type": "string",
                    "description": "Role to send as (user, assistant, system)",
                    "enum": ["user", "assistant", "system"],
                    "default": "assistant"
                }
            },
            "required": ["session_key", "message"]
        })
    }

    async fn execute(&self, args: serde_json::Value) -> Result<String, ToolError> {
        let session_key =
            args["session_key"]
                .as_str()
                .ok_or_else(|| ToolError::InvalidArguments {
                    tool: "sessions_send".to_string(),
                    message: "Missing required parameter 'session_key'".to_string(),
                })?;

        let message = args["message"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArguments {
                tool: "sessions_send".to_string(),
                message: "Missing required parameter 'message'".to_string(),
            })?;

        let role_str = args["as_role"].as_str().unwrap_or("assistant");
        let role = match role_str {
            "user" => MessageRole::User,
            "assistant" => MessageRole::Assistant,
            "system" => MessageRole::System,
            _ => MessageRole::Assistant,
        };

        let mut sessions = self.sessions.lock().await;

        // Get or create the target session
        let session = sessions.get_or_create(session_key);

        // Create the message
        let msg = crate::session::ChatMessage {
            role,
            content: message.to_string(),
            sender_id: Some("orchestrator".to_string()),
            timestamp: chrono::Utc::now(),
            metadata: std::collections::HashMap::new(),
        };

        session.add_message(msg);

        // Get message count before we lose the reference
        let message_count = session.message_count();

        // Clone the session for saving to avoid borrow issues
        let session_to_save = session.clone();

        // Drop the lock before saving
        drop(sessions);

        // Save the session - acquire a new lock
        let sessions = self.sessions.lock().await;
        sessions
            .save(&session_to_save)
            .map_err(|e| ToolError::ExecutionFailed {
                tool: "sessions_send".to_string(),
                message: format!("Failed to save session: {}", e),
            })?;
        drop(sessions);

        Ok(format!(
            "Message sent to session '{}'. Session now has {} messages.",
            session_key, message_count
        ))
    }
}

/// Information about a session for listing.
#[derive(Debug)]
struct SessionInfo {
    key: String,
    message_count: usize,
    created_at: String,
    updated_at: String,
    has_summary: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_session_manager() -> (SessionManager, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let sessions = SessionManager::new(temp_dir.path().to_path_buf());
        (sessions, temp_dir)
    }

    #[tokio::test]
    async fn test_session_list_tool() {
        let (sessions, _temp) = create_test_session_manager();
        let sessions = Arc::new(Mutex::new(sessions));

        let tool = SessionListTool::new(sessions.clone());

        // Initially no sessions
        let result = tool.execute(serde_json::json!({})).await.unwrap();
        assert!(result.contains("No sessions found"));

        // Create a session
        {
            let mut guard = sessions.lock().await;
            let session = guard.get_or_create("cli:test");
            session.add_message(crate::session::ChatMessage::user("Hello", "user1"));
            let session_to_save = session.clone();
            drop(guard);
            let guard = sessions.lock().await;
            guard.save(&session_to_save).unwrap();
        }

        // Now should list the session
        let result = tool.execute(serde_json::json!({})).await.unwrap();
        assert!(result.contains("cli:test"));
        assert!(result.contains("1")); // message count
    }

    #[tokio::test]
    async fn test_session_history_tool() {
        let (sessions, _temp) = create_test_session_manager();
        let sessions = Arc::new(Mutex::new(sessions));

        let tool = SessionHistoryTool::new(sessions.clone());

        // Create a session with messages
        {
            let mut guard = sessions.lock().await;
            let session = guard.get_or_create("cli:test");
            session.add_message(crate::session::ChatMessage::user("Hello", "user1"));
            session.add_message(crate::session::ChatMessage::assistant("Hi there"));
            let session_to_save = session.clone();
            drop(guard);
            let guard = sessions.lock().await;
            guard.save(&session_to_save).unwrap();
        }

        // Get history
        let result = tool
            .execute(serde_json::json!({
                "session_key": "cli:test",
                "limit": 10
            }))
            .await
            .unwrap();

        assert!(result.contains("cli:test"));
        assert!(result.contains("Hello"));
        assert!(result.contains("Hi there"));
    }

    #[tokio::test]
    async fn test_session_send_tool() {
        let (sessions, _temp) = create_test_session_manager();
        let sessions = Arc::new(Mutex::new(sessions));

        let tool = SessionSendTool::new(sessions.clone());

        // Send a message to a new session
        let result = tool
            .execute(serde_json::json!({
                "session_key": "cli:target",
                "message": "Hello from orchestrator",
                "as_role": "assistant"
            }))
            .await
            .unwrap();

        assert!(result.contains("Message sent"));
        assert!(result.contains("1 messages"));

        // Verify the message was added
        let history_tool = SessionHistoryTool::new(sessions.clone());
        let history = history_tool
            .execute(serde_json::json!({"session_key": "cli:target"}))
            .await
            .unwrap();

        assert!(history.contains("orchestrator"));
        assert!(history.contains("Hello from orchestrator"));
    }

    #[tokio::test]
    async fn test_session_list_with_filter() {
        let (sessions, _temp) = create_test_session_manager();
        let sessions = Arc::new(Mutex::new(sessions));

        // Create multiple sessions
        {
            let mut guard = sessions.lock().await;

            let session1 = guard.get_or_create("cli:session1");
            session1.add_message(crate::session::ChatMessage::user("Hello", "user1"));
            let session1_to_save = session1.clone();
            drop(guard);

            let guard = sessions.lock().await;
            guard.save(&session1_to_save).unwrap();
            drop(guard);

            let mut guard = sessions.lock().await;
            let session2 = guard.get_or_create("telegram:chat1");
            session2.add_message(crate::session::ChatMessage::user("Hi", "user2"));
            let session2_to_save = session2.clone();
            drop(guard);

            let guard = sessions.lock().await;
            guard.save(&session2_to_save).unwrap();
        }

        let tool = SessionListTool::new(sessions.clone());

        // Filter by cli
        let result = tool
            .execute(serde_json::json!({"filter": "cli"}))
            .await
            .unwrap();

        assert!(result.contains("cli:session1"));
        assert!(!result.contains("telegram"));
    }
}
