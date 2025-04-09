use std::future::Future;
use std::pin::Pin;
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use mcp_core::{
    handler::{PromptError, ResourceError, ToolError},
    prompt::Prompt,
    protocol::{ServerCapabilities, JsonRpcRequest, JsonRpcResponse, ErrorData},
    resource::Resource,
    tool::Tool,
    Content,
};
use mcp_server::router::{CapabilitiesBuilder, Router};
use serde_json::{Value, json};
use terraphim_config::{Config, ConfigState};
use terraphim_service::{ServiceError, TerraphimService};
use terraphim_types::Document;
use thiserror::Error;
use tracing::{debug, error, info, warn};

mod resource_mapper;
mod tool_handlers;

use crate::resource_mapper::TerraphimResourceMapper;
use crate::tool_handlers::{handle_search, handle_update_config, SearchToolParams, UpdateConfigToolParams};

/// Errors specific to the Terraphim MCP server integration
#[derive(Error, Debug)]
pub enum McpError {
    #[error("Service error: {0}")]
    Service(#[from] ServiceError),
    
    #[error("Resource error: {0}")]
    Resource(String),
    
    #[error("Tool execution error: {0}")]
    Tool(String),
    
    #[error("Invalid request: {0}")]
    InvalidRequest(String),
}

impl From<McpError> for ToolError {
    fn from(err: McpError) -> Self {
        match err {
            McpError::Service(e) => ToolError::ExecutionError(format!("Service error: {}", e)),
            McpError::Resource(e) => ToolError::ExecutionError(format!("Resource error: {}", e)),
            McpError::Tool(e) => ToolError::ExecutionError(e),
            McpError::InvalidRequest(e) => ToolError::InvalidParameters(e),
        }
    }
}

impl From<McpError> for ResourceError {
    fn from(err: McpError) -> Self {
        match err {
            McpError::Resource(e) => ResourceError::NotFound(e),
            _ => ResourceError::ExecutionError(format!("Resource error: {:?}", err)),
        }
    }
}

/// The main router type for the Terraphim MCP server
#[derive(Clone)]
pub struct TerraphimMcpRouter {
    config_state: Arc<ConfigState>,
    resource_mapper: Arc<TerraphimResourceMapper>,
}

impl TerraphimMcpRouter {
    /// Create a new router instance
    pub fn new(config_state: Arc<ConfigState>) -> Self {
        Self {
            config_state: config_state.clone(),
            resource_mapper: Arc::new(TerraphimResourceMapper::new().with_config_state(config_state)),
        }
    }
    
    /// Create a Terraphim service instance from the current configuration
    fn terraphim_service(&self) -> TerraphimService {
        TerraphimService::new((*self.config_state).clone())
    }
    
    /// Update the configuration
    pub async fn update_config(&self, new_config: Config) -> Result<(), McpError> {
        let config = self.config_state.config.clone();
        let mut current_config = config.lock().await;
        *current_config = new_config;
        Ok(())
    }
}

#[async_trait]
impl Router for TerraphimMcpRouter {
    fn name(&self) -> String {
        "terraphim-mcp".to_string()
    }

    fn instructions(&self) -> String {
        "This server provides Terraphim knowledge graph search capabilities through the Model Context Protocol. You can search for documents using the search tool and access resources that represent Terraphim documents.".to_string()
    }

    fn list_tools(&self) -> Vec<Tool> {
        vec![
            Tool::new(
                "search".to_string(),
                "Search for documents in the Terraphim knowledge graph".to_string(),
                serde_json::json!({
                    "type": "object",
                    "properties": {
                        "query": {
                            "type": "string",
                            "description": "The search query"
                        },
                        "role": {
                            "type": "string",
                            "description": "Optional role name to filter results"
                        },
                        "limit": {
                            "type": "integer",
                            "description": "Optional maximum number of results to return"
                        },
                        "skip": {
                            "type": "integer",
                            "description": "Optional number of results to skip"
                        }
                    },
                    "required": ["query"]
                }),
            ),
            Tool::new(
                "update_config".to_string(),
                "Update the Terraphim configuration".to_string(),
                serde_json::json!({
                    "type": "object",
                    "properties": {
                        "config_str": {
                            "type": "string",
                            "description": "The new configuration as a JSON string"
                        }
                    },
                    "required": ["config_str"]
                }),
            ),
        ]
    }

    fn call_tool(
        &self,
        tool_name: &str,
        arguments: Value,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<Content>, ToolError>> + Send + 'static>> {
        let this = self.clone();
        let tool_name = tool_name.to_string();
        let _arguments_clone = arguments.clone();
        
        Box::pin(async move {
            // Check if the tool name is valid
            match tool_name.as_str() {
                "search" | "update_config" => {
                    // Tool exists, continue with execution
                },
                _ => {
                    // Tool doesn't exist, return an error
                    return Err(ToolError::NotFound(format!("Tool '{}' not found", tool_name)));
                }
            }
            
            match tool_name.as_str() {
                "search" => {
                    // Parse the JSON-RPC request first
                    let request = serde_json::from_value::<JsonRpcRequest>(arguments.clone())
                        .map_err(|e| ToolError::InvalidParameters(format!("Invalid request: {}", e)))?;
                    
                    // Extract the params from the request
                    let params_value = request.params.clone().ok_or_else(|| {
                        ToolError::InvalidParameters("Missing params field in JSON-RPC request".to_string())
                    })?;
                    
                    // Parse the params into SearchToolParams
                    let params: SearchToolParams = serde_json::from_value(params_value)
                        .map_err(|e| ToolError::InvalidParameters(format!("Invalid search parameters: {}", e)))?;
                    
                    let response = handle_search(Arc::new(this), request, params).await?;
                    Ok(vec![Content::Text(mcp_core::content::TextContent { 
                        text: serde_json::to_string(&response).unwrap_or_default(),
                        annotations: None,
                    })])
                },
                "update_config" => {
                    // Parse the JSON-RPC request first
                    let request = serde_json::from_value::<JsonRpcRequest>(arguments.clone())
                        .map_err(|e| ToolError::InvalidParameters(format!("Invalid request: {}", e)))?;
                    
                    // Extract the params from the request
                    let params_value = request.params.clone().ok_or_else(|| {
                        ToolError::InvalidParameters("Missing params field in JSON-RPC request".to_string())
                    })?;
                    
                    // Parse the params into UpdateConfigToolParams
                    let params: UpdateConfigToolParams = serde_json::from_value(params_value)
                        .map_err(|e| ToolError::InvalidParameters(format!("Invalid update_config parameters: {}", e)))?;
                    
                    let response = handle_update_config(Arc::new(this), request, params).await?;
                    Ok(vec![Content::Text(mcp_core::content::TextContent { 
                        text: serde_json::to_string(&response).unwrap_or_default(),
                        annotations: None,
                    })])
                },
                _ => {
                    // This should never be reached due to the early check, but just in case
                    Err(ToolError::NotFound(format!("Tool '{}' not found", tool_name)))
                }
            }
        })
    }

    fn list_resources(&self) -> Vec<Resource> {
        let mut service = self.terraphim_service();
        
        // Create a wildcard search query to get all documents
        let search_query = terraphim_types::SearchQuery {
            search_term: terraphim_types::NormalizedTermValue::new("".to_string()),
            limit: None,
            skip: None,
            role: None,
        };
        
        // Create a runtime to run the async search synchronously
        let runtime = tokio::runtime::Runtime::new()
            .unwrap_or_else(|e| {
                error!("Failed to create Tokio runtime: {}", e);
                std::process::exit(1);
            });
        
        // Try to search for all documents
        match runtime.block_on(service.search(&search_query)) {
            Ok(documents) => {
                self.resource_mapper.documents_to_resources(&documents)
                    .unwrap_or_default()
            }
            Err(e) => {
                error!("Failed to list resources: {:?}", e);
                Vec::new()
            }
        }
    }

    fn read_resource(
        &self,
        uri: &str,
    ) -> Pin<Box<dyn Future<Output = Result<String, ResourceError>> + Send + 'static>> {
        let uri = uri.to_string();
        let this = self.clone();
        
        Box::pin(async move {
            let document_id = this.resource_mapper.extract_document_id_from_uri(&uri)
                .map_err(|e| ResourceError::NotFound(e.to_string()))?;
                
            // Use the resource mapper to get the document
            match this.resource_mapper.get_document(&document_id).await {
                Ok(document) => {
                    let mut text = String::new();
                    text.push_str(&format!("Title: {}\n", document.title));
                    
                    if let Some(description) = &document.description {
                        text.push_str(&format!("Description: {}\n", description));
                    }
                    
                    text.push_str("\nContent:\n");
                    text.push_str(&document.body);
                    
                    if let Some(tags) = &document.tags {
                        text.push_str("\n\nTags: ");
                        text.push_str(&tags.join(", "));
                    }
                    
                    Ok(text)
                },
                Err(e) => Err(ResourceError::NotFound(format!("Document not found: {}", e))),
            }
        })
    }

    fn list_prompts(&self) -> Vec<Prompt> {
        Vec::new()
    }

    fn get_prompt(
        &self,
        prompt_name: &str,
    ) -> Pin<Box<dyn Future<Output = Result<String, PromptError>> + Send + 'static>> {
        let prompt_name = prompt_name.to_string();
        Box::pin(async move {
            Err(PromptError::NotFound(format!(
                "Prompt {} not found",
                prompt_name
            )))
        })
    }

    fn capabilities(&self) -> ServerCapabilities {
        ServerCapabilities {
            tools: Some(mcp_core::protocol::ToolsCapability {
                list_changed: None,
            }),
            prompts: Some(mcp_core::protocol::PromptsCapability {
                list_changed: None,
            }),
            resources: Some(mcp_core::protocol::ResourcesCapability {
                subscribe: None,
                list_changed: None,
            }),
        }
    }
}
