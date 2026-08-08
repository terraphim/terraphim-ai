//! JSON-RPC stdio router for ACP.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::AcpState;
use super::handlers::{
    AcpError, CancelRequest, InitializeRequest, SendMessageRequest, handle_cancel,
    handle_initialize, handle_list_sessions, handle_load_session, handle_new_session,
    handle_send_message,
};
use super::protocol::InitializeResult;

/// JSON-RPC 2.0 request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub method: String,
    #[serde(default)]
    pub params: Value,
    #[serde(default)]
    pub id: Option<Value>,
}

/// JSON-RPC 2.0 response (success or error).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<AcpError>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<Value>,
}

/// Dispatch a JSON-RPC request and return a response.
pub async fn dispatch(state: &AcpState, req: JsonRpcRequest) -> JsonRpcResponse {
    let id = req.id.clone();
    let result: Result<Value, AcpError> = match req.method.as_str() {
        "initialize" => {
            let params = req.params;
            let parsed: InitializeRequest = match serde_json::from_value(params) {
                Ok(p) => p,
                Err(e) => return error_response(id, -32602, format!("invalid params: {e}")),
            };
            let res: InitializeResult = handle_initialize(state, parsed);
            match serde_json::to_value(res) {
                Ok(v) => Ok(v),
                Err(e) => Err(AcpError {
                    code: -32603,
                    message: format!("serialize result: {e}"),
                }),
            }
        }
        "new_session" => {
            let params = req.params;
            let session_id: String = match serde_json::from_value(params) {
                Ok(s) => s,
                Err(e) => return error_response(id, -32602, format!("invalid params: {e}")),
            };
            match handle_new_session(state, session_id).await {
                Ok(res) => match serde_json::to_value(res) {
                    Ok(v) => Ok(v),
                    Err(e) => Err(AcpError {
                        code: -32603,
                        message: format!("serialize result: {e}"),
                    }),
                },
                Err(e) => Err(e),
            }
        }
        "load_session" => {
            let params = req.params;
            let session_id: String = match serde_json::from_value(params) {
                Ok(s) => s,
                Err(e) => return error_response(id, -32602, format!("invalid params: {e}")),
            };
            match handle_load_session(state, session_id).await {
                Ok(res) => match serde_json::to_value(res) {
                    Ok(v) => Ok(v),
                    Err(e) => Err(AcpError {
                        code: -32603,
                        message: format!("serialize result: {e}"),
                    }),
                },
                Err(e) => Err(e),
            }
        }
        "list_sessions" => match handle_list_sessions(state).await {
            Ok(res) => match serde_json::to_value(res) {
                Ok(v) => Ok(v),
                Err(e) => Err(AcpError {
                    code: -32603,
                    message: format!("serialize result: {e}"),
                }),
            },
            Err(e) => Err(e),
        },
        "send_message" => {
            let params = req.params;
            let parsed: SendMessageRequest = match serde_json::from_value(params) {
                Ok(p) => p,
                Err(e) => return error_response(id, -32602, format!("invalid params: {e}")),
            };
            match handle_send_message(state, parsed).await {
                Ok(res) => match serde_json::to_value(res) {
                    Ok(v) => Ok(v),
                    Err(e) => Err(AcpError {
                        code: -32603,
                        message: format!("serialize result: {e}"),
                    }),
                },
                Err(e) => Err(e),
            }
        }
        "cancel" => {
            let params = req.params;
            let parsed: CancelRequest = match serde_json::from_value(params) {
                Ok(p) => p,
                Err(e) => return error_response(id, -32602, format!("invalid params: {e}")),
            };
            match handle_cancel(state, parsed).await {
                Ok(res) => match serde_json::to_value(res) {
                    Ok(v) => Ok(v),
                    Err(e) => Err(AcpError {
                        code: -32603,
                        message: format!("serialize result: {e}"),
                    }),
                },
                Err(e) => Err(e),
            }
        }
        other => Err(AcpError {
            code: -32601,
            message: format!("method not found: {other}"),
        }),
    };

    match result {
        Ok(value) => JsonRpcResponse {
            jsonrpc: "2.0".into(),
            result: Some(value),
            error: None,
            id,
        },
        Err(e) => JsonRpcResponse {
            jsonrpc: "2.0".into(),
            result: None,
            error: Some(e),
            id,
        },
    }
}

/// Build a JSON-RPC error response (for params parsing failures).
fn error_response(id: Option<Value>, code: i32, message: String) -> JsonRpcResponse {
    JsonRpcResponse {
        jsonrpc: "2.0".into(),
        result: None,
        error: Some(AcpError { code, message }),
        id,
    }
}
