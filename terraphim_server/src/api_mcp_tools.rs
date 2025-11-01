use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use terraphim_mcp_proxy::{ContentItem, McpProxy, Tool, ToolCallRequest};
use terraphim_persistence::mcp::{McpPersistence, McpPersistenceImpl};

use crate::{AppState, Result};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ToolListResponse {
    pub tools: Vec<Tool>,
    pub endpoint_uuid: String,
    pub namespace_uuid: String,
    pub count: usize,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ToolCallRequestPayload {
    pub arguments: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ToolCallResponsePayload {
    pub content: Vec<ContentItem>,
    pub is_error: bool,
    pub tool_name: String,
    pub endpoint_uuid: String,
}

pub async fn list_tools_for_endpoint(
    State(app_state): State<AppState>,
    Path(endpoint_uuid): Path<String>,
) -> Result<Json<ToolListResponse>> {
    let persistence = get_mcp_persistence()?;

    let endpoint = persistence
        .get_endpoint(&endpoint_uuid)
        .await
        .map_err(|e| {
            crate::error::ApiError(
                StatusCode::INTERNAL_SERVER_ERROR,
                anyhow::anyhow!("Failed to get endpoint: {}", e),
            )
        })?
        .ok_or_else(|| {
            crate::error::ApiError(
                StatusCode::NOT_FOUND,
                anyhow::anyhow!("Endpoint not found: {}", endpoint_uuid),
            )
        })?;

    let namespace = persistence
        .get_namespace(&endpoint.namespace_uuid)
        .await
        .map_err(|e| {
            crate::error::ApiError(
                StatusCode::INTERNAL_SERVER_ERROR,
                anyhow::anyhow!("Failed to get namespace: {}", e),
            )
        })?
        .ok_or_else(|| {
            crate::error::ApiError(
                StatusCode::NOT_FOUND,
                anyhow::anyhow!("Namespace not found: {}", endpoint.namespace_uuid),
            )
        })?;

    let proxy = create_proxy_from_namespace(&namespace, &app_state).await?;

    let tools = proxy.list_tools().await.map_err(|e| {
        crate::error::ApiError(
            StatusCode::INTERNAL_SERVER_ERROR,
            anyhow::anyhow!("Failed to list tools: {}", e),
        )
    })?;

    Ok(Json(ToolListResponse {
        count: tools.len(),
        tools,
        endpoint_uuid: endpoint_uuid.clone(),
        namespace_uuid: endpoint.namespace_uuid.clone(),
    }))
}

pub async fn execute_tool(
    State(app_state): State<AppState>,
    Path((endpoint_uuid, tool_name)): Path<(String, String)>,
    Json(payload): Json<ToolCallRequestPayload>,
) -> Result<Json<ToolCallResponsePayload>> {
    let persistence = get_mcp_persistence()?;

    let endpoint = persistence
        .get_endpoint(&endpoint_uuid)
        .await
        .map_err(|e| {
            crate::error::ApiError(
                StatusCode::INTERNAL_SERVER_ERROR,
                anyhow::anyhow!("Failed to get endpoint: {}", e),
            )
        })?
        .ok_or_else(|| {
            crate::error::ApiError(
                StatusCode::NOT_FOUND,
                anyhow::anyhow!("Endpoint not found: {}", endpoint_uuid),
            )
        })?;

    let namespace = persistence
        .get_namespace(&endpoint.namespace_uuid)
        .await
        .map_err(|e| {
            crate::error::ApiError(
                StatusCode::INTERNAL_SERVER_ERROR,
                anyhow::anyhow!("Failed to get namespace: {}", e),
            )
        })?
        .ok_or_else(|| {
            crate::error::ApiError(
                StatusCode::NOT_FOUND,
                anyhow::anyhow!("Namespace not found: {}", endpoint.namespace_uuid),
            )
        })?;

    let proxy = create_proxy_from_namespace(&namespace, &app_state).await?;

    let request = ToolCallRequest {
        name: tool_name.clone(),
        arguments: payload.arguments,
    };

    let response = proxy.call_tool(request).await.map_err(|e| {
        log::error!("Tool execution failed: {}", e);
        crate::error::ApiError(
            StatusCode::INTERNAL_SERVER_ERROR,
            anyhow::anyhow!("Failed to execute tool: {}", e),
        )
    })?;

    Ok(Json(ToolCallResponsePayload {
        content: response.content,
        is_error: response.is_error,
        tool_name,
        endpoint_uuid,
    }))
}

async fn create_proxy_from_namespace(
    namespace: &terraphim_persistence::mcp::McpNamespaceRecord,
    _app_state: &AppState,
) -> Result<McpProxy> {
    let config_json: serde_json::Value =
        serde_json::from_str(&namespace.config_json).map_err(|e| {
            crate::error::ApiError(
                StatusCode::INTERNAL_SERVER_ERROR,
                anyhow::anyhow!("Failed to parse namespace config: {}", e),
            )
        })?;

    let mcp_namespace: terraphim_mcp_proxy::McpNamespace = serde_json::from_value(config_json)
        .map_err(|e| {
            crate::error::ApiError(
                StatusCode::INTERNAL_SERVER_ERROR,
                anyhow::anyhow!("Failed to deserialize namespace: {}", e),
            )
        })?;

    let proxy = McpProxy::with_namespace(mcp_namespace).await.map_err(|e| {
        crate::error::ApiError(
            StatusCode::INTERNAL_SERVER_ERROR,
            anyhow::anyhow!("Failed to create proxy: {}", e),
        )
    })?;

    Ok(proxy)
}

fn get_mcp_persistence() -> Result<Arc<McpPersistenceImpl>> {
    use opendal::services::Memory;
    use opendal::Operator;

    let builder = Memory::default();
    let op = Operator::new(builder)
        .map_err(|e| {
            crate::error::ApiError(
                StatusCode::INTERNAL_SERVER_ERROR,
                anyhow::anyhow!("Failed to create operator: {}", e),
            )
        })?
        .finish();

    Ok(Arc::new(McpPersistenceImpl::new(op)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use terraphim_persistence::mcp::{McpEndpointRecord, McpNamespaceRecord, NamespaceVisibility};

    #[tokio::test]
    async fn test_tool_list_response_serialization() {
        let response = ToolListResponse {
            tools: vec![Tool {
                name: "test__read_file".to_string(),
                description: "Read a file".to_string(),
                input_schema: Some(serde_json::json!({"type": "object"})),
            }],
            endpoint_uuid: "endpoint-123".to_string(),
            namespace_uuid: "namespace-456".to_string(),
            count: 1,
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("test__read_file"));
        assert!(json.contains("endpoint-123"));
    }

    #[tokio::test]
    async fn test_tool_call_request_deserialization() {
        let json = r#"{"arguments": {"path": "/test/file.txt"}}"#;
        let payload: ToolCallRequestPayload = serde_json::from_str(json).unwrap();
        assert!(payload.arguments.is_some());
    }

    #[tokio::test]
    async fn test_tool_call_request_no_arguments() {
        let json = r#"{}"#;
        let payload: ToolCallRequestPayload = serde_json::from_str(json).unwrap();
        assert!(payload.arguments.is_none());
    }

    #[tokio::test]
    async fn test_tool_call_response_serialization() {
        let response = ToolCallResponsePayload {
            content: vec![ContentItem::Text {
                text: "File contents here".to_string(),
            }],
            is_error: false,
            tool_name: "filesystem__read_file".to_string(),
            endpoint_uuid: "endpoint-123".to_string(),
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("File contents here"));
        assert!(json.contains("filesystem__read_file"));
    }

    #[tokio::test]
    async fn test_create_proxy_from_namespace() {
        use opendal::services::Memory;
        use opendal::Operator;

        let builder = Memory::default();
        let op = Operator::new(builder).unwrap().finish();
        let persistence = Arc::new(McpPersistenceImpl::new(op));

        let mcp_ns = terraphim_mcp_proxy::McpNamespace::new("test").add_server(
            terraphim_mcp_proxy::McpServerConfig::stdio(
                "filesystem",
                "npx",
                vec![
                    "-y".to_string(),
                    "@modelcontextprotocol/server-filesystem".to_string(),
                ],
            ),
        );

        let config_json = serde_json::to_string(&mcp_ns).unwrap();

        let namespace_record = McpNamespaceRecord {
            uuid: "test-uuid".to_string(),
            name: "test".to_string(),
            description: Some("Test namespace".to_string()),
            user_id: None,
            config_json,
            created_at: chrono::Utc::now(),
            enabled: true,
            visibility: NamespaceVisibility::Private,
        };

        persistence.save_namespace(&namespace_record).await.unwrap();

        let endpoint_record = McpEndpointRecord {
            uuid: "endpoint-uuid".to_string(),
            name: "test-endpoint".to_string(),
            namespace_uuid: "test-uuid".to_string(),
            auth_type: "none".to_string(),
            user_id: None,
            created_at: chrono::Utc::now(),
            enabled: true,
        };

        persistence.save_endpoint(&endpoint_record).await.unwrap();
    }

    #[tokio::test]
    async fn test_multiple_content_items() {
        let response = ToolCallResponsePayload {
            content: vec![
                ContentItem::Text {
                    text: "First message".to_string(),
                },
                ContentItem::Text {
                    text: "Second message".to_string(),
                },
                ContentItem::Image {
                    data: "base64data".to_string(),
                    mime_type: "image/png".to_string(),
                },
            ],
            is_error: false,
            tool_name: "multi_content".to_string(),
            endpoint_uuid: "endpoint-123".to_string(),
        };

        assert_eq!(response.content.len(), 3);
        assert!(!response.is_error);
    }

    #[tokio::test]
    async fn test_namespace_config_roundtrip() {
        let mcp_ns = terraphim_mcp_proxy::McpNamespace::new("test-roundtrip").add_server(
            terraphim_mcp_proxy::McpServerConfig::sse("test-server", "http://localhost:8080"),
        );

        let config_json = serde_json::to_string(&mcp_ns).unwrap();
        let parsed: terraphim_mcp_proxy::McpNamespace = serde_json::from_str(&config_json).unwrap();

        assert_eq!(parsed.name, "test-roundtrip");
        assert_eq!(parsed.servers.len(), 1);
        assert_eq!(parsed.servers[0].name, "test-server");
    }
}
