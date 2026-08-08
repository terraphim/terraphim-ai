//! MCP server exposing TinyClaw's conversations, messages, and events as MCP tools.
//!
//! Wave 2 of the Hermes parity arc (epic #3160). Matches Hermes' `mcp_serve.py`
//! 9-tool bridge surface (pinned commit `846b14ab`).

use super::tools::*;
use crate::bus::{MessageBus, OutboundMessage};
use crate::session::{MessageRole, SessionManager};
use rmcp::handler::server::router::tool::ToolRouter;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{CallToolResult, ServerCapabilities, ServerInfo};
use rmcp::{ServerHandler, ServiceExt};
use std::sync::Arc;
use tokio::sync::Mutex;

/// MCP server for TinyClaw channel bridge.
#[derive(Clone)]
pub struct TinyClawMcpServer {
    sessions: Arc<Mutex<SessionManager>>,
    bus: Arc<MessageBus>,
    tool_router: ToolRouter<Self>,
}

impl TinyClawMcpServer {
    /// Create a new MCP server.
    pub fn new(sessions: Arc<Mutex<SessionManager>>, bus: Arc<MessageBus>) -> Self {
        Self {
            sessions,
            bus,
            tool_router: Self::tool_router(),
        }
    }

    /// Convert a session key to a conversation summary.
    fn session_to_summary(
        &self,
        key: &str,
        session: &crate::session::Session,
    ) -> ConversationSummary {
        let channel = key.split(':').next().unwrap_or("unknown").to_string();
        let display_name = session.metadata.get("display_name").cloned();
        let last_message_at = session.messages.last().map(|m| m.timestamp.to_rfc3339());
        ConversationSummary {
            id: key.to_string(),
            channel,
            display_name,
            last_message_at,
            message_count: session.messages.len(),
        }
    }

    /// Convert a ChatMessage to a ConversationMessage.
    fn chat_to_conversation(msg: &crate::session::ChatMessage) -> ConversationMessage {
        ConversationMessage {
            id: uuid::Uuid::new_v4().to_string(),
            role: match msg.role {
                MessageRole::User => "user".to_string(),
                MessageRole::Assistant => "assistant".to_string(),
                MessageRole::System => "system".to_string(),
                MessageRole::Tool => "tool".to_string(),
            },
            content: msg.content.clone(),
            timestamp: msg.timestamp.to_rfc3339(),
        }
    }
}

#[rmcp::tool_router(router = tool_router)]
impl TinyClawMcpServer {
    /// List conversations across platforms.
    #[rmcp::tool(description = "List conversations across platforms")]
    pub async fn conversations_list(&self) -> Result<CallToolResult, rmcp::ErrorData> {
        let sessions = self.sessions.lock().await;
        let keys = sessions
            .list_sessions()
            .map_err(|e| rmcp::ErrorData::internal_error(e.to_string(), None))?;

        let mut summaries = Vec::new();
        for key in keys {
            if let Some(session) = sessions.get(&key) {
                summaries.push(self.session_to_summary(&key, session));
            }
        }

        // Hermes contract: wrap in {"count": N, "conversations": [...]}
        let body = serde_json::json!({
            "count": summaries.len(),
            "conversations": summaries,
        });
        Ok(json_result(&body))
    }

    /// Get a single conversation by ID.
    #[rmcp::tool(description = "Get a single conversation by ID")]
    pub async fn conversation_get(
        &self,
        params: Parameters<ConversationGetParams>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        let sessions = self.sessions.lock().await;
        let session = match sessions.get(&params.0.conversation_id) {
            Some(s) => s,
            None => {
                // Hermes contract: missing session returns error JSON, not Err
                let body = serde_json::json!({
                    "error": format!("Conversation not found: {}", params.0.conversation_id),
                });
                return Ok(json_result(&body));
            }
        };

        let messages: Vec<ConversationMessage> = session
            .messages
            .iter()
            .map(Self::chat_to_conversation)
            .collect();

        let summary = self.session_to_summary(&params.0.conversation_id, session);
        let body = serde_json::json!({
            "session_key": params.0.conversation_id,
            "messages": messages,
            "summary": summary,
        });
        Ok(json_result(&body))
    }

    /// Read message history for a conversation.
    #[rmcp::tool(description = "Read message history for a conversation")]
    pub async fn messages_read(
        &self,
        params: Parameters<MessagesReadParams>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        let sessions = self.sessions.lock().await;
        let session = sessions.get(&params.0.conversation_id).ok_or_else(|| {
            rmcp::ErrorData::invalid_params(
                format!("conversation not found: {}", params.0.conversation_id),
                None,
            )
        })?;

        let limit = params.0.limit.unwrap_or(50);
        let start = session.messages.len().saturating_sub(limit);
        let messages: Vec<ConversationMessage> = session.messages[start..]
            .iter()
            .map(Self::chat_to_conversation)
            .collect();

        Ok(json_result(&messages))
    }

    /// Fetch attachments for a conversation.
    #[rmcp::tool(description = "Fetch attachments for a conversation")]
    pub async fn attachments_fetch(
        &self,
        params: Parameters<ConversationGetParams>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        // TinyClaw stores media URLs in InboundMessage.media, not in the session.
        // For Wave 2, return an empty list — attachments are ephemeral in the bus.
        let _ = params;
        Ok(json_result(&Vec::<String>::new()))
    }

    /// Poll for live events.
    #[rmcp::tool(description = "Poll for live events")]
    pub async fn events_poll(&self) -> Result<CallToolResult, rmcp::ErrorData> {
        let mut rx = self.bus.inbound_rx.lock().await;
        let events: Vec<serde_json::Value> = match rx.try_recv() {
            Ok(msg) => {
                let event = serde_json::json!({
                    "type": "message",
                    "channel": msg.channel,
                    "chat_id": msg.chat_id,
                    "sender_id": msg.sender_id,
                    "content": msg.content,
                });
                vec![event]
            }
            Err(_) => Vec::new(),
        };
        let body = serde_json::json!({
            "count": events.len(),
            "events": events,
        });
        Ok(json_result(&body))
    }

    /// Wait for live events (long-poll).
    #[rmcp::tool(description = "Wait for live events (long-poll)")]
    pub async fn events_wait(
        &self,
        params: Parameters<EventsWaitParams>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        let timeout_ms = params.0.timeout_ms.unwrap_or(30_000);
        let timeout = std::time::Duration::from_millis(timeout_ms);

        let mut rx = self.bus.inbound_rx.lock().await;
        match tokio::time::timeout(timeout, rx.recv()).await {
            Ok(Some(msg)) => {
                let event = serde_json::json!({
                    "type": "message",
                    "channel": msg.channel,
                    "chat_id": msg.chat_id,
                    "sender_id": msg.sender_id,
                    "content": msg.content,
                });
                Ok(json_result(&vec![event]))
            }
            Ok(None) => Ok(json_result(&Vec::<serde_json::Value>::new())),
            Err(_) => Ok(json_result(&Vec::<serde_json::Value>::new())),
        }
    }

    /// Send a message to a conversation.
    #[rmcp::tool(description = "Send a message to a conversation")]
    pub async fn messages_send(
        &self,
        params: Parameters<MessagesSendParams>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        let conversation_id = &params.0.conversation_id;
        let parts: Vec<&str> = conversation_id.split(':').collect();
        if parts.len() < 2 {
            // Hermes contract: invalid format returns error JSON, not Err
            let body = serde_json::json!({
                "status": "error",
                "error": format!(
                    "invalid conversation_id format: expected 'channel:chat_id', got '{}'",
                    conversation_id
                ),
            });
            return Ok(json_result(&body));
        }

        let channel = parts[0].to_string();
        let chat_id = parts[1..].join(":");

        let msg = OutboundMessage::new(channel, chat_id, params.0.content.clone());
        match self.bus.outbound_sender().send(msg).await {
            Ok(()) => {
                let body = serde_json::json!({
                    "status": "sent",
                    "conversation_id": conversation_id,
                });
                Ok(json_result(&body))
            }
            Err(e) => {
                let body = serde_json::json!({
                    "status": "error",
                    "error": e.to_string(),
                });
                Ok(json_result(&body))
            }
        }
    }

    /// List open approval requests.
    #[rmcp::tool(description = "List open approval requests")]
    pub async fn permissions_list_open(&self) -> Result<CallToolResult, rmcp::ErrorData> {
        // TinyClaw's ExecutionGuard is a pre-execution block/warn system, not an
        // approval queue. Wave 2 returns an empty list; a real approval system
        // is a Wave 5+ concern.
        // Hermes contract: wrap in {"permissions": [...], "count": N}
        let body = serde_json::json!({
            "count": 0,
            "permissions": Vec::<serde_json::Value>::new(),
        });
        Ok(json_result(&body))
    }

    /// Respond to an approval request.
    #[rmcp::tool(description = "Respond to an approval request")]
    pub async fn permissions_respond(
        &self,
        params: Parameters<PermissionsRespondParams>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        // No approval system in Wave 2 — respond with error JSON, not Err
        // (Hermes contract: error cases return JSON, not exceptions)
        let body = serde_json::json!({
            "status": "error",
            "request_id": params.0.request_id,
            "error": format!("approval request not found: {}", params.0.request_id),
        });
        Ok(json_result(&body))
    }

    /// List connected channels.
    #[rmcp::tool(description = "List connected channels")]
    pub async fn channels_list(&self) -> Result<CallToolResult, rmcp::ErrorData> {
        // TinyClaw channels are configured at startup; we can't enumerate them
        // from the bus alone. Return the channels we know about from config.
        // For Wave 2, return a static list based on feature flags.
        let channels = vec!["cli"];
        let body = serde_json::json!({
            "count": channels.len(),
            "channels": channels,
        });
        Ok(json_result(&body))
    }
}

#[rmcp::tool_handler(router = self.tool_router)]
impl ServerHandler for TinyClawMcpServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some("TinyClaw MCP channel bridge".into()),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            ..Default::default()
        }
    }
}

/// Start the MCP server on stdio.
pub async fn serve_mcp_stdio(
    sessions: Arc<Mutex<SessionManager>>,
    bus: Arc<MessageBus>,
) -> Result<(), super::McpError> {
    use rmcp::transport::io::stdio;

    let server = TinyClawMcpServer::new(sessions, bus);
    let service = server
        .serve(stdio())
        .await
        .map_err(|e| super::McpError::Server(e.to_string()))?;

    service
        .waiting()
        .await
        .map_err(|e| super::McpError::Server(e.to_string()))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::SessionManager;
    use tempfile::TempDir;

    fn make_server() -> (TinyClawMcpServer, TempDir) {
        let dir = TempDir::new().unwrap();
        let sessions = Arc::new(Mutex::new(SessionManager::new(dir.path().to_path_buf())));
        let bus = Arc::new(MessageBus::new());
        (TinyClawMcpServer::new(sessions, bus), dir)
    }

    #[tokio::test]
    async fn test_conversations_list_empty() {
        // Hermes contract: conversations_list returns {"count": 0, "conversations": []}
        let (server, _dir) = make_server();
        let result = server.conversations_list().await.unwrap();
        let text = result.content[0].as_text().unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&text.text).unwrap();
        assert_eq!(parsed["count"], 0);
        assert!(parsed["conversations"].is_array());
    }

    #[tokio::test]
    async fn test_conversation_get_not_found() {
        // Hermes contract: missing session returns error JSON, NOT Err
        let (server, _dir) = make_server();
        let params = Parameters(ConversationGetParams {
            conversation_id: "nonexistent".into(),
        });
        let result = server.conversation_get(params).await.unwrap();
        let text = result.content[0].as_text().unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&text.text).unwrap();
        assert!(parsed.get("error").is_some());
    }

    #[tokio::test]
    async fn test_messages_send_invalid_format() {
        // Hermes contract: invalid format returns error JSON, NOT Err
        let (server, _dir) = make_server();
        let params = Parameters(MessagesSendParams {
            conversation_id: "no-colon".into(),
            content: "hello".into(),
        });
        let result = server.messages_send(params).await.unwrap();
        let text = result.content[0].as_text().unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&text.text).unwrap();
        assert_eq!(parsed["status"], "error");
        assert!(parsed["error"].as_str().unwrap().contains("invalid"));
    }

    #[tokio::test]
    async fn test_permissions_list_open_empty() {
        // Hermes contract: permissions_list_open returns {"count": 0, "permissions": []}
        let (server, _dir) = make_server();
        let result = server.permissions_list_open().await.unwrap();
        let text = result.content[0].as_text().unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&text.text).unwrap();
        assert_eq!(parsed["count"], 0);
        assert!(parsed["permissions"].is_array());
    }

    #[tokio::test]
    async fn test_permissions_respond_not_found() {
        // Hermes contract: unknown request returns error JSON, NOT Err
        let (server, _dir) = make_server();
        let params = Parameters(PermissionsRespondParams {
            request_id: "req-123".into(),
            approved: true,
        });
        let result = server.permissions_respond(params).await.unwrap();
        let text = result.content[0].as_text().unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&text.text).unwrap();
        assert_eq!(parsed["status"], "error");
        assert_eq!(parsed["request_id"], "req-123");
    }

    #[tokio::test]
    async fn test_channels_list() {
        // Hermes contract: channels_list returns {"count": N, "channels": [...]}
        let (server, _dir) = make_server();
        let result = server.channels_list().await.unwrap();
        let text = result.content[0].as_text().unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&text.text).unwrap();
        assert!(parsed["channels"].is_array());
        assert!(
            parsed["channels"]
                .as_array()
                .unwrap()
                .contains(&serde_json::json!("cli"))
        );
    }
}
