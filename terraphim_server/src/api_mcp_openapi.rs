use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    paths(
        crate::api_mcp_tools::list_tools_for_endpoint,
        crate::api_mcp_tools::execute_tool,
    ),
    components(
        schemas(
            crate::api_mcp_tools::McpTool,
            crate::api_mcp_tools::McpContentItem,
            crate::api_mcp_tools::ToolListResponse,
            crate::api_mcp_tools::ToolCallRequestPayload,
            crate::api_mcp_tools::ToolCallResponsePayload,
        )
    ),
    tags(
        (name = "MCP Tools", description = "Model Context Protocol tool execution endpoints")
    ),
    info(
        title = "Terraphim MCP API",
        version = env!("CARGO_PKG_VERSION"),
        description = "REST API for executing MCP (Model Context Protocol) tools",
        contact(
            name = "Terraphim Contributors",
            email = "team@terraphim.ai",
            url = "https://terraphim.ai"
        ),
        license(
            name = "Apache-2.0",
            url = "https://www.apache.org/licenses/LICENSE-2.0"
        )
    )
)]
pub struct McpApiDoc;
