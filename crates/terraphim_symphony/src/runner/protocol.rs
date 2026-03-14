//! Codex app-server JSON-RPC protocol types.
//!
//! Defines the message types for line-delimited JSON communication
//! with the coding-agent app-server over stdio.

use serde::{Deserialize, Serialize};

/// A JSON-RPC request (client -> server or server -> client).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    /// Request ID (integer or string).
    pub id: serde_json::Value,
    /// Method name.
    pub method: String,
    /// Parameters.
    #[serde(default)]
    pub params: serde_json::Value,
}

/// A JSON-RPC response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcResponse {
    /// Response ID matching the request.
    pub id: serde_json::Value,
    /// Result (present on success).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    /// Error (present on failure).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

/// A JSON-RPC notification (no `id` field).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcNotification {
    /// Method name.
    pub method: String,
    /// Parameters.
    #[serde(default)]
    pub params: serde_json::Value,
}

/// A JSON-RPC error payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcError {
    /// Error code.
    pub code: i64,
    /// Error message.
    pub message: String,
    /// Optional additional data.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

/// Parsed message from the app-server stdout.
#[derive(Debug, Clone)]
pub enum AppServerMessage {
    /// A response to a client request.
    Response(JsonRpcResponse),
    /// A server-initiated notification.
    Notification(JsonRpcNotification),
    /// A server-initiated request (e.g. approval).
    Request(JsonRpcRequest),
    /// A line that could not be parsed as JSON-RPC.
    Malformed(String),
}

impl AppServerMessage {
    /// Try to parse a JSON line into an app-server message.
    pub fn parse_line(line: &str) -> Self {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            return AppServerMessage::Malformed(String::new());
        }

        let value: serde_json::Value = match serde_json::from_str(trimmed) {
            Ok(v) => v,
            Err(_) => return AppServerMessage::Malformed(trimmed.to_string()),
        };

        let obj = match value.as_object() {
            Some(o) => o,
            None => return AppServerMessage::Malformed(trimmed.to_string()),
        };

        // Distinguish by presence of `id` and/or `method`
        let has_id = obj.contains_key("id");
        let has_method = obj.contains_key("method");
        let has_result = obj.contains_key("result");
        let has_error = obj.contains_key("error");

        if has_id && (has_result || has_error) && !has_method {
            // Response
            match serde_json::from_value(value) {
                Ok(resp) => AppServerMessage::Response(resp),
                Err(_) => AppServerMessage::Malformed(trimmed.to_string()),
            }
        } else if has_id && has_method {
            // Server-initiated request
            match serde_json::from_value(value) {
                Ok(req) => AppServerMessage::Request(req),
                Err(_) => AppServerMessage::Malformed(trimmed.to_string()),
            }
        } else if has_method && !has_id {
            // Notification
            match serde_json::from_value(value) {
                Ok(notif) => AppServerMessage::Notification(notif),
                Err(_) => AppServerMessage::Malformed(trimmed.to_string()),
            }
        } else {
            AppServerMessage::Malformed(trimmed.to_string())
        }
    }
}

/// Events emitted by the agent runner to the orchestrator.
#[derive(Debug, Clone)]
pub enum AgentEvent {
    /// Agent session has been established.
    SessionStarted {
        session_id: String,
        thread_id: String,
        turn_id: String,
        pid: Option<u32>,
    },
    /// A turn completed successfully.
    TurnCompleted { turn_id: String, turn_count: u32 },
    /// A turn failed.
    TurnFailed { turn_id: String, reason: String },
    /// An approval was auto-approved.
    ApprovalAutoApproved { approval_type: String },
    /// An unsupported tool call was rejected.
    UnsupportedToolCall { tool_name: String },
    /// A general notification from the agent.
    Notification { message: String },
    /// Token usage update.
    TokenUsage {
        input_tokens: u64,
        output_tokens: u64,
        total_tokens: u64,
    },
    /// Startup failed.
    StartupFailed { reason: String },
}

/// Token usage counters.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TokenCounts {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub total_tokens: u64,
}

/// Aggregate token totals for the service.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TokenTotals {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub total_tokens: u64,
    /// Aggregate runtime seconds across all completed sessions.
    pub seconds_running: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_response() {
        let line = r#"{"id":1,"result":{"thread":{"id":"t1"}}}"#;
        match AppServerMessage::parse_line(line) {
            AppServerMessage::Response(r) => {
                assert_eq!(r.id, serde_json::json!(1));
                assert!(r.result.is_some());
                assert!(r.error.is_none());
            }
            other => panic!("expected Response, got {other:?}"),
        }
    }

    #[test]
    fn parse_notification() {
        let line = r#"{"method":"turn/completed","params":{"turnId":"t1"}}"#;
        match AppServerMessage::parse_line(line) {
            AppServerMessage::Notification(n) => {
                assert_eq!(n.method, "turn/completed");
            }
            other => panic!("expected Notification, got {other:?}"),
        }
    }

    #[test]
    fn parse_server_request() {
        let line = r#"{"id":"req-1","method":"item/commandExecution/requestApproval","params":{"command":"ls"}}"#;
        match AppServerMessage::parse_line(line) {
            AppServerMessage::Request(r) => {
                assert_eq!(r.method, "item/commandExecution/requestApproval");
            }
            other => panic!("expected Request, got {other:?}"),
        }
    }

    #[test]
    fn parse_error_response() {
        let line = r#"{"id":2,"error":{"code":-1,"message":"failed"}}"#;
        match AppServerMessage::parse_line(line) {
            AppServerMessage::Response(r) => {
                assert!(r.error.is_some());
                assert_eq!(r.error.unwrap().message, "failed");
            }
            other => panic!("expected Response, got {other:?}"),
        }
    }

    #[test]
    fn parse_malformed() {
        assert!(matches!(
            AppServerMessage::parse_line("not json"),
            AppServerMessage::Malformed(_)
        ));
    }

    #[test]
    fn parse_empty_line() {
        assert!(matches!(
            AppServerMessage::parse_line(""),
            AppServerMessage::Malformed(_)
        ));
    }

    #[test]
    fn parse_non_object_json() {
        assert!(matches!(
            AppServerMessage::parse_line("[1,2,3]"),
            AppServerMessage::Malformed(_)
        ));
    }
}
