//! Tool schemas and parameter types for the 9-tool MCP channel bridge.
//!
//! Matches Hermes' `mcp_serve.py` surface (pinned commit `846b14ab`).

use rmcp::model::{CallToolResult, Content, JsonObject, Tool};
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::sync::Arc;

/// A conversation summary returned by `conversations_list`.
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct ConversationSummary {
    /// Unique conversation identifier.
    pub id: String,
    /// Channel platform (e.g. "telegram", "discord", "cli").
    pub channel: String,
    /// Human-readable display name.
    pub display_name: Option<String>,
    /// ISO 8601 timestamp of the last message.
    pub last_message_at: Option<String>,
    /// Number of messages in the conversation.
    pub message_count: usize,
}

/// A single message in a conversation.
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct ConversationMessage {
    /// Message identifier.
    pub id: String,
    /// Role: "user", "assistant", or "system".
    pub role: String,
    /// Message content.
    pub content: String,
    /// ISO 8601 timestamp.
    pub timestamp: String,
}

/// Parameters for `conversation_get`.
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct ConversationGetParams {
    /// Conversation identifier.
    pub conversation_id: String,
}

/// Parameters for `messages_read`.
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct MessagesReadParams {
    /// Conversation identifier.
    pub conversation_id: String,
    /// Maximum number of messages to return.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<usize>,
    /// Return messages before this message ID.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub before: Option<String>,
}

/// Parameters for `messages_send`.
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct MessagesSendParams {
    /// Conversation identifier.
    pub conversation_id: String,
    /// Message content to send.
    pub content: String,
}

/// Parameters for `events_wait`.
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct EventsWaitParams {
    /// Maximum time to wait in milliseconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout_ms: Option<u64>,
}

/// An approval request from the agent loop.
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct ApprovalRequest {
    /// Request identifier.
    pub id: String,
    /// Name of the tool requesting approval.
    pub tool_name: String,
    /// Tool arguments.
    pub arguments: serde_json::Value,
    /// ISO 8601 timestamp when the request was created.
    pub requested_at: String,
}

/// Parameters for `permissions_respond`.
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct PermissionsRespondParams {
    /// Request identifier.
    pub request_id: String,
    /// Whether the request is approved.
    pub approved: bool,
}

fn empty_schema() -> Arc<JsonObject> {
    let mut map = JsonObject::new();
    map.insert("type".to_string(), "object".into());
    map.insert("properties".to_string(), serde_json::json!({}));
    map.insert("required".to_string(), serde_json::json!([]));
    Arc::new(map)
}

fn object_schema(properties: serde_json::Value, required: Vec<&str>) -> Arc<JsonObject> {
    let mut map = JsonObject::new();
    map.insert("type".to_string(), "object".into());
    map.insert("properties".to_string(), properties);
    map.insert(
        "required".to_string(),
        serde_json::json!(required.iter().map(|s| s.to_string()).collect::<Vec<_>>()),
    );
    Arc::new(map)
}

fn make_tool(name: &'static str, description: &'static str, input_schema: Arc<JsonObject>) -> Tool {
    Tool {
        name: Cow::Borrowed(name),
        description: Some(Cow::Borrowed(description)),
        input_schema,
        annotations: None,
        title: None,
        output_schema: None,
        icons: None,
        meta: None,
    }
}

/// Build the `conversations_list` tool definition.
pub fn conversations_list_tool() -> Tool {
    make_tool(
        "conversations_list",
        "List conversations across platforms",
        empty_schema(),
    )
}

/// Build the `conversation_get` tool definition.
pub fn conversation_get_tool() -> Tool {
    make_tool(
        "conversation_get",
        "Get a single conversation by ID",
        object_schema(
            serde_json::json!({
                "conversation_id": { "type": "string" }
            }),
            vec!["conversation_id"],
        ),
    )
}

/// Build the `messages_read` tool definition.
pub fn messages_read_tool() -> Tool {
    make_tool(
        "messages_read",
        "Read message history for a conversation",
        object_schema(
            serde_json::json!({
                "conversation_id": { "type": "string" },
                "limit": { "type": "integer" },
                "before": { "type": "string" }
            }),
            vec!["conversation_id"],
        ),
    )
}

/// Build the `attachments_fetch` tool definition.
pub fn attachments_fetch_tool() -> Tool {
    make_tool(
        "attachments_fetch",
        "Fetch attachments for a conversation",
        object_schema(
            serde_json::json!({
                "conversation_id": { "type": "string" }
            }),
            vec!["conversation_id"],
        ),
    )
}

/// Build the `events_poll` tool definition.
pub fn events_poll_tool() -> Tool {
    make_tool("events_poll", "Poll for live events", empty_schema())
}

/// Build the `events_wait` tool definition.
pub fn events_wait_tool() -> Tool {
    make_tool(
        "events_wait",
        "Wait for live events (long-poll)",
        object_schema(
            serde_json::json!({
                "timeout_ms": { "type": "integer" }
            }),
            vec![],
        ),
    )
}

/// Build the `messages_send` tool definition.
pub fn messages_send_tool() -> Tool {
    make_tool(
        "messages_send",
        "Send a message to a conversation",
        object_schema(
            serde_json::json!({
                "conversation_id": { "type": "string" },
                "content": { "type": "string" }
            }),
            vec!["conversation_id", "content"],
        ),
    )
}

/// Build the `permissions_list_open` tool definition.
pub fn permissions_list_open_tool() -> Tool {
    make_tool(
        "permissions_list_open",
        "List open approval requests",
        empty_schema(),
    )
}

/// Build the `permissions_respond` tool definition.
pub fn permissions_respond_tool() -> Tool {
    make_tool(
        "permissions_respond",
        "Respond to an approval request",
        object_schema(
            serde_json::json!({
                "request_id": { "type": "string" },
                "approved": { "type": "boolean" }
            }),
            vec!["request_id", "approved"],
        ),
    )
}

/// Build the `channels_list` tool definition (Hermes-specific extra).
pub fn channels_list_tool() -> Tool {
    make_tool("channels_list", "List connected channels", empty_schema())
}

/// Return all 10 tool definitions (9 bridge + `channels_list`).
pub fn all_bridge_tools() -> Vec<Tool> {
    vec![
        conversations_list_tool(),
        conversation_get_tool(),
        messages_read_tool(),
        attachments_fetch_tool(),
        events_poll_tool(),
        events_wait_tool(),
        messages_send_tool(),
        permissions_list_open_tool(),
        permissions_respond_tool(),
        channels_list_tool(),
    ]
}

/// Helper to create a successful text result.
pub fn text_result(text: impl Into<String>) -> CallToolResult {
    CallToolResult::success(vec![Content::text(text.into())])
}

/// Helper to create a successful JSON result.
pub fn json_result<T: Serialize>(value: &T) -> CallToolResult {
    match serde_json::to_string_pretty(value) {
        Ok(json) => CallToolResult::success(vec![Content::text(json)]),
        Err(e) => CallToolResult::error(vec![Content::text(format!("serialization error: {}", e))]),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_schemas_match_hermes() {
        let tools = all_bridge_tools();
        let names: Vec<&str> = tools.iter().map(|t| t.name.as_ref()).collect();
        assert_eq!(
            names,
            vec![
                "conversations_list",
                "conversation_get",
                "messages_read",
                "attachments_fetch",
                "events_poll",
                "events_wait",
                "messages_send",
                "permissions_list_open",
                "permissions_respond",
                "channels_list",
            ]
        );
    }

    #[test]
    fn test_conversation_get_params_serialize() {
        let params = ConversationGetParams {
            conversation_id: "test-123".into(),
        };
        let json = serde_json::to_value(&params).unwrap();
        assert_eq!(json["conversation_id"], "test-123");
    }

    #[test]
    fn test_messages_send_params_serialize() {
        let params = MessagesSendParams {
            conversation_id: "test-123".into(),
            content: "hello".into(),
        };
        let json = serde_json::to_value(&params).unwrap();
        assert_eq!(json["conversation_id"], "test-123");
        assert_eq!(json["content"], "hello");
    }

    #[test]
    fn test_events_wait_params_optional_timeout() {
        let params = EventsWaitParams { timeout_ms: None };
        let json = serde_json::to_value(&params).unwrap();
        assert!(json.get("timeout_ms").is_none());
    }
}
