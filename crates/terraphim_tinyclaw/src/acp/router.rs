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
    let params = req.params;
    let result = match req.method.as_str() {
        "initialize" => dispatch_initialize(state, params),
        "new_session" => dispatch_new_session(state, params).await,
        "load_session" => dispatch_load_session(state, params).await,
        "list_sessions" => dispatch_list_sessions(state).await,
        "send_message" => dispatch_send_message(state, params).await,
        "cancel" => dispatch_cancel(state, params).await,
        other => Err(AcpError {
            code: -32601,
            message: format!("method not found: {other}"),
        }),
    };
    build_response(id, result)
}

/// Build a JSON-RPC response from a result.
fn build_response(id: Option<Value>, result: Result<Value, AcpError>) -> JsonRpcResponse {
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

/// Serialize a result type into a JSON-RPC result value, mapping
/// serialization failures to a JSON-RPC internal error.
fn serialize_result<T: Serialize>(res: T) -> Result<Value, AcpError> {
    serde_json::to_value(res).map_err(|e| AcpError {
        code: -32603,
        message: format!("serialize result: {e}"),
    })
}

/// Parse typed params from JSON value, mapping deserialization failures
/// to a JSON-RPC invalid-params error.
fn parse_params<T: serde::de::DeserializeOwned>(params: Value) -> Result<T, AcpError> {
    serde_json::from_value(params).map_err(|e| AcpError {
        code: -32602,
        message: format!("invalid params: {e}"),
    })
}

fn dispatch_initialize(state: &AcpState, params: Value) -> Result<Value, AcpError> {
    let parsed: InitializeRequest = parse_params(params)?;
    let res: InitializeResult = handle_initialize(state, parsed);
    serialize_result(res)
}

async fn dispatch_new_session(state: &AcpState, params: Value) -> Result<Value, AcpError> {
    let session_id: String = parse_params(params)?;
    let res = handle_new_session(state, session_id).await?;
    serialize_result(res)
}

async fn dispatch_load_session(state: &AcpState, params: Value) -> Result<Value, AcpError> {
    let session_id: String = parse_params(params)?;
    let res = handle_load_session(state, session_id).await?;
    serialize_result(res)
}

async fn dispatch_list_sessions(state: &AcpState) -> Result<Value, AcpError> {
    let res = handle_list_sessions(state).await?;
    serialize_result(res)
}

async fn dispatch_send_message(state: &AcpState, params: Value) -> Result<Value, AcpError> {
    let parsed: SendMessageRequest = parse_params(params)?;
    let res = handle_send_message(state, parsed).await?;
    serialize_result(res)
}

async fn dispatch_cancel(state: &AcpState, params: Value) -> Result<Value, AcpError> {
    let parsed: CancelRequest = parse_params(params)?;
    let res = handle_cancel(state, parsed).await?;
    serialize_result(res)
}
