use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
#[cfg(feature = "mcp-proxy")]
use terraphim_mcp_proxy::{ContentItem, McpProxy, Tool, ToolCallRequest};
use terraphim_persistence::mcp::McpPersistence;
use utoipa::ToSchema;

use crate::{AppState, Result};

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct McpTool {
    /// Tool name (prefixed with server name)
    pub name: String,
    /// Tool description
    pub description: String,
    /// Tool input JSON schema
    #[schema(value_type = Object)]
    pub input_schema: Option<serde_json::Value>,
}

impl From<Tool> for McpTool {
    fn from(tool: Tool) -> Self {
        Self {
            name: tool.name,
            description: tool.description,
            input_schema: tool.input_schema,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct ToolListResponse {
    /// List of available tools
    pub tools: Vec<McpTool>,
    /// Endpoint UUID
    pub endpoint_uuid: String,
    /// Namespace UUID
    pub namespace_uuid: String,
    /// Total number of tools
    pub count: usize,
}

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct ToolCallRequestPayload {
    /// Optional JSON arguments for the tool
    #[schema(value_type = Object)]
    pub arguments: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum McpContentItem {
    Text {
        text: String,
    },
    Image {
        data: String,
        mime_type: String,
    },
    Resource {
        uri: String,
        mime_type: Option<String>,
    },
}

impl From<ContentItem> for McpContentItem {
    fn from(item: ContentItem) -> Self {
        match item {
            ContentItem::Text { text } => McpContentItem::Text { text },
            ContentItem::Image { data, mime_type } => McpContentItem::Image { data, mime_type },
            ContentItem::Resource { uri, mime_type } => McpContentItem::Resource { uri, mime_type },
            ContentItem::Json { data } => McpContentItem::Text { 
                text: serde_json::to_string_pretty(&data).unwrap_or_else(|_| "{}".to_string())
            },
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct ToolCallResponsePayload {
    /// Response content items (text, image, resource)
    pub content: Vec<McpContentItem>,
    /// Whether the tool execution resulted in an error
    pub is_error: bool,
    /// Name of the executed tool
    pub tool_name: String,
    /// Endpoint UUID
    pub endpoint_uuid: String,
}

/// List all tools available for an endpoint
///
/// Returns all MCP tools registered under the namespace associated with this endpoint.
/// Tools are prefixed with their server name (ServerName__toolName) and filtered
/// based on namespace tool_overrides configuration.
#[utoipa::path(
    get,
    path = "/metamcp/endpoints/{endpoint_uuid}/tools",
    params(
        ("endpoint_uuid" = String, Path, description = "Endpoint UUID")
    ),
    responses(
        (status = 200, description = "List of tools retrieved successfully", body = ToolListResponse),
        (status = 404, description = "Endpoint or namespace not found"),
        (status = 500, description = "Internal server error")
    ),
    tag = "MCP Tools"
)]
pub async fn list_tools_for_endpoint(
    State(app_state): State<AppState>,
    Path(endpoint_uuid): Path<String>,
) -> Result<Json<ToolListResponse>> {
    let persistence = app_state.mcp_persistence.clone();

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
        tools: tools.into_iter().map(McpTool::from).collect(),
        endpoint_uuid: endpoint_uuid.clone(),
        namespace_uuid: endpoint.namespace_uuid.clone(),
    }))
}

/// Execute a specific MCP tool
///
/// Executes the specified tool with the provided arguments and returns the response.
/// The tool name should match one of the tools returned by the list endpoint.
#[utoipa::path(
    post,
    path = "/metamcp/endpoints/{endpoint_uuid}/tools/{tool_name}",
    params(
        ("endpoint_uuid" = String, Path, description = "Endpoint UUID"),
        ("tool_name" = String, Path, description = "Tool name (may be prefixed with ServerName__)")
    ),
    request_body = ToolCallRequestPayload,
    responses(
        (status = 200, description = "Tool executed successfully", body = ToolCallResponsePayload),
        (status = 404, description = "Endpoint, namespace, or tool not found"),
        (status = 500, description = "Tool execution failed or internal server error")
    ),
    tag = "MCP Tools"
)]
pub async fn execute_tool(
    State(app_state): State<AppState>,
    Path((endpoint_uuid, tool_name)): Path<(String, String)>,
    Json(payload): Json<ToolCallRequestPayload>,
) -> Result<Json<ToolCallResponsePayload>> {
    let persistence = app_state.mcp_persistence.clone();

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
        content: response
            .content
            .into_iter()
            .map(McpContentItem::from)
            .collect(),
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

    #[cfg(feature = "mcp-proxy")]
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use terraphim_persistence::mcp::{
        McpEndpointRecord, McpNamespaceRecord, McpPersistenceImpl, NamespaceVisibility,
    };

    #[tokio::test]
    async fn test_tool_list_response_serialization() {
        let response = ToolListResponse {
            tools: vec![McpTool {
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
            content: vec![McpContentItem::Text {
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
                McpContentItem::Text {
                    text: "First message".to_string(),
                },
                McpContentItem::Text {
                    text: "Second message".to_string(),
                },
                McpContentItem::Image {
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
