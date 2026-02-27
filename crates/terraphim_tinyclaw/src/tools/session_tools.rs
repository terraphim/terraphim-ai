use crate::bus::OutboundMessage;
use crate::session::{ChatMessage, MessageRole, SessionManager};
use crate::tools::{Tool, ToolError};
use async_trait::async_trait;
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::{Mutex, mpsc};

#[derive(Clone)]
struct RequesterContext {
    session_key: String,
    channel: String,
}

fn split_session_key(key: &str) -> Option<(&str, &str)> {
    key.split_once(':')
}

fn read_string_arg<'a>(args: &'a Value, primary: &str, secondary: &str) -> Option<&'a str> {
    args.get(primary)
        .and_then(Value::as_str)
        .or_else(|| args.get(secondary).and_then(Value::as_str))
}

fn requester_from_args(args: &Value) -> Result<RequesterContext, ToolError> {
    let session_key = read_string_arg(args, "requester_session_key", "requesterSessionKey")
        .ok_or_else(|| ToolError::InvalidArguments {
            tool: "sessions".to_string(),
            message: "Missing requester session context".to_string(),
        })?
        .trim()
        .to_string();

    let (channel, _) =
        split_session_key(&session_key).ok_or_else(|| ToolError::InvalidArguments {
            tool: "sessions".to_string(),
            message: format!("Invalid requester session key '{}'", session_key),
        })?;
    let channel = channel.to_string();

    Ok(RequesterContext {
        session_key,
        channel,
    })
}

fn ensure_same_channel(
    requester: &RequesterContext,
    target_key: &str,
    tool_name: &str,
) -> Result<(), ToolError> {
    let (target_channel, _) =
        split_session_key(target_key).ok_or_else(|| ToolError::InvalidArguments {
            tool: tool_name.to_string(),
            message: format!("Invalid target session key '{}'", target_key),
        })?;

    if target_channel != requester.channel {
        return Err(ToolError::Blocked {
            tool: tool_name.to_string(),
            reason: format!(
                "Cross-channel session access denied (requester channel: {}, target channel: {})",
                requester.channel, target_channel
            ),
        });
    }

    Ok(())
}

/// List sessions visible to the current requester channel.
pub struct SessionsListTool {
    sessions: Arc<Mutex<SessionManager>>,
}

impl SessionsListTool {
    pub fn new(sessions: Arc<Mutex<SessionManager>>) -> Self {
        Self { sessions }
    }
}

#[async_trait]
impl Tool for SessionsListTool {
    fn name(&self) -> &str {
        "sessions_list"
    }

    fn description(&self) -> &str {
        "List available sessions for the current requester channel"
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "limit": {
                    "type": "integer",
                    "minimum": 1,
                    "description": "Maximum number of sessions to return (default: 20)"
                },
                "requester_session_key": {
                    "type": "string",
                    "description": "Injected by runtime. Session key of the requester."
                }
            }
        })
    }

    async fn execute(&self, args: serde_json::Value) -> Result<String, ToolError> {
        let requester = requester_from_args(&args)?;
        let limit = args
            .get("limit")
            .and_then(Value::as_u64)
            .map(|value| value as usize)
            .unwrap_or(20)
            .clamp(1, 200);

        let mut sessions_guard = self.sessions.lock().await;
        let keys = sessions_guard
            .list_sessions()
            .map_err(|e| ToolError::ExecutionFailed {
                tool: self.name().to_string(),
                message: e.to_string(),
            })?;

        let mut rows = Vec::new();
        for key in keys {
            let Some((channel, _)) = split_session_key(&key) else {
                continue;
            };
            if channel != requester.channel {
                continue;
            }

            if let Some(snapshot) = sessions_guard.get_session_snapshot(&key) {
                rows.push(serde_json::json!({
                    "session_key": snapshot.key,
                    "message_count": snapshot.messages.len(),
                    "updated_at": snapshot.updated_at,
                    "has_summary": snapshot.summary.is_some()
                }));
            }
        }

        rows.sort_by(|a, b| {
            let a_updated = a
                .get("updated_at")
                .and_then(Value::as_str)
                .unwrap_or_default();
            let b_updated = b
                .get("updated_at")
                .and_then(Value::as_str)
                .unwrap_or_default();
            b_updated.cmp(a_updated)
        });
        rows.truncate(limit);

        Ok(serde_json::json!({
            "status": "ok",
            "count": rows.len(),
            "sessions": rows
        })
        .to_string())
    }
}

/// Read history from an existing session.
pub struct SessionsHistoryTool {
    sessions: Arc<Mutex<SessionManager>>,
}

impl SessionsHistoryTool {
    pub fn new(sessions: Arc<Mutex<SessionManager>>) -> Self {
        Self { sessions }
    }
}

#[async_trait]
impl Tool for SessionsHistoryTool {
    fn name(&self) -> &str {
        "sessions_history"
    }

    fn description(&self) -> &str {
        "Get recent message history for a session in the current requester channel"
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "session_key": {
                    "type": "string",
                    "description": "Session key to inspect; defaults to requester session"
                },
                "limit": {
                    "type": "integer",
                    "minimum": 1,
                    "description": "Maximum messages to return (default: 50)"
                },
                "include_tool_messages": {
                    "type": "boolean",
                    "description": "Whether to include tool messages"
                },
                "requester_session_key": {
                    "type": "string",
                    "description": "Injected by runtime. Session key of the requester."
                }
            }
        })
    }

    async fn execute(&self, args: serde_json::Value) -> Result<String, ToolError> {
        let requester = requester_from_args(&args)?;
        let target_key = read_string_arg(&args, "session_key", "sessionKey")
            .unwrap_or(&requester.session_key)
            .trim()
            .to_string();
        let limit = args
            .get("limit")
            .and_then(Value::as_u64)
            .map(|value| value as usize)
            .unwrap_or(50)
            .clamp(1, 500);
        let include_tool_messages = args
            .get("include_tool_messages")
            .or_else(|| args.get("includeTools"))
            .and_then(Value::as_bool)
            .unwrap_or(false);

        ensure_same_channel(&requester, &target_key, self.name())?;

        let mut sessions_guard = self.sessions.lock().await;
        let snapshot = sessions_guard
            .get_session_snapshot(&target_key)
            .ok_or_else(|| ToolError::ExecutionFailed {
                tool: self.name().to_string(),
                message: format!("Session '{}' was not found", target_key),
            })?;

        let start = snapshot.messages.len().saturating_sub(limit);
        let mut messages = snapshot.messages[start..].to_vec();
        if !include_tool_messages {
            messages.retain(|message| message.role != MessageRole::Tool);
        }

        Ok(serde_json::json!({
            "status": "ok",
            "session_key": snapshot.key,
            "total_messages": snapshot.messages.len(),
            "returned_messages": messages.len(),
            "summary": snapshot.summary,
            "messages": messages
        })
        .to_string())
    }
}

/// Send a message to another session on the same channel.
pub struct SessionsSendTool {
    sessions: Arc<Mutex<SessionManager>>,
    outbound_tx: mpsc::Sender<OutboundMessage>,
}

impl SessionsSendTool {
    pub fn new(
        sessions: Arc<Mutex<SessionManager>>,
        outbound_tx: mpsc::Sender<OutboundMessage>,
    ) -> Self {
        Self {
            sessions,
            outbound_tx,
        }
    }
}

#[async_trait]
impl Tool for SessionsSendTool {
    fn name(&self) -> &str {
        "sessions_send"
    }

    fn description(&self) -> &str {
        "Send a message to another session in the current requester channel"
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "session_key": {
                    "type": "string",
                    "description": "Target session key"
                },
                "message": {
                    "type": "string",
                    "description": "Message content to send"
                },
                "requester_session_key": {
                    "type": "string",
                    "description": "Injected by runtime. Session key of the requester."
                }
            },
            "required": ["session_key", "message"]
        })
    }

    async fn execute(&self, args: serde_json::Value) -> Result<String, ToolError> {
        let requester = requester_from_args(&args)?;
        let target_key = read_string_arg(&args, "session_key", "sessionKey")
            .ok_or_else(|| ToolError::InvalidArguments {
                tool: self.name().to_string(),
                message: "Missing 'session_key' parameter".to_string(),
            })?
            .trim()
            .to_string();
        let message = read_string_arg(&args, "message", "content")
            .ok_or_else(|| ToolError::InvalidArguments {
                tool: self.name().to_string(),
                message: "Missing 'message' parameter".to_string(),
            })?
            .to_string();

        ensure_same_channel(&requester, &target_key, self.name())?;

        let (_, chat_id) =
            split_session_key(&target_key).ok_or_else(|| ToolError::InvalidArguments {
                tool: self.name().to_string(),
                message: format!("Invalid target session key '{}'", target_key),
            })?;

        {
            let mut sessions_guard = self.sessions.lock().await;
            let session = sessions_guard.get_or_create(&target_key);
            session.add_message(ChatMessage::user(
                message.clone(),
                requester.session_key.clone(),
            ));
            let snapshot = session.clone();
            sessions_guard
                .save(&snapshot)
                .map_err(|e| ToolError::ExecutionFailed {
                    tool: self.name().to_string(),
                    message: e.to_string(),
                })?;
        }

        self.outbound_tx
            .send(OutboundMessage::new(
                &requester.channel,
                chat_id,
                message.clone(),
            ))
            .await
            .map_err(|e| ToolError::ExecutionFailed {
                tool: self.name().to_string(),
                message: format!("Failed to enqueue outbound message: {}", e),
            })?;

        Ok(serde_json::json!({
            "status": "queued",
            "session_key": target_key,
            "channel": requester.channel,
            "chat_id": chat_id,
            "message": message
        })
        .to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::Session;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_sessions_list_filters_by_requester_channel() {
        let temp_dir = TempDir::new().unwrap();
        let sessions = Arc::new(Mutex::new(SessionManager::new(
            temp_dir.path().to_path_buf(),
        )));

        {
            let guard = sessions.lock().await;
            guard.save(&Session::new("cli:one")).unwrap();
            guard.save(&Session::new("telegram:two")).unwrap();
        }

        let tool = SessionsListTool::new(sessions);
        let output = tool
            .execute(serde_json::json!({
                "requester_session_key": "cli:main",
                "limit": 10
            }))
            .await
            .unwrap();

        let payload: Value = serde_json::from_str(&output).unwrap();
        assert_eq!(payload["count"], 1);
        assert_eq!(payload["sessions"][0]["session_key"], "cli:one");
    }

    #[tokio::test]
    async fn test_sessions_history_blocks_cross_channel_access() {
        let temp_dir = TempDir::new().unwrap();
        let sessions = Arc::new(Mutex::new(SessionManager::new(
            temp_dir.path().to_path_buf(),
        )));

        {
            let mut guard = sessions.lock().await;
            let session = guard.get_or_create("telegram:chat");
            session.add_message(ChatMessage::user("hello", "user"));
            let snapshot = session.clone();
            guard.save(&snapshot).unwrap();
        }

        let tool = SessionsHistoryTool::new(sessions);
        let err = tool
            .execute(serde_json::json!({
                "requester_session_key": "cli:main",
                "session_key": "telegram:chat"
            }))
            .await
            .unwrap_err();

        assert!(matches!(err, ToolError::Blocked { .. }));
    }

    #[tokio::test]
    async fn test_sessions_send_persists_and_enqueues_outbound() {
        let temp_dir = TempDir::new().unwrap();
        let sessions = Arc::new(Mutex::new(SessionManager::new(
            temp_dir.path().to_path_buf(),
        )));
        let (tx, mut rx) = mpsc::channel(4);

        let tool = SessionsSendTool::new(sessions.clone(), tx);
        let output = tool
            .execute(serde_json::json!({
                "requester_session_key": "cli:source",
                "session_key": "cli:target",
                "message": "ping"
            }))
            .await
            .unwrap();

        let payload: Value = serde_json::from_str(&output).unwrap();
        assert_eq!(payload["status"], "queued");

        let outbound = rx.recv().await.unwrap();
        assert_eq!(outbound.channel, "cli");
        assert_eq!(outbound.chat_id, "target");
        assert_eq!(outbound.content, "ping");

        let mut guard = sessions.lock().await;
        let snapshot = guard.get_session_snapshot("cli:target").unwrap();
        assert_eq!(snapshot.messages.len(), 1);
        assert_eq!(snapshot.messages[0].content, "ping");
    }
}
