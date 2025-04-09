use mcp_core::{
    handler::ToolError,
    resource::{Resource, ResourceContents},
    Content,
    protocol::{JsonRpcResponse, JsonRpcRequest, ErrorData},
};
use serde::{Deserialize, Serialize};
use terraphim_types::{Document, NormalizedTermValue, RoleName, SearchQuery, RelevanceFunction};
use terraphim_config::{Config, Haystack, ServiceType, Role};
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{debug, error, info, warn};
use std::path::{Path, PathBuf};
use anyhow::Result;

use crate::{McpError, TerraphimMcpRouter};
use ahash::AHashMap;
use serde_json::{self, Value, json};

/// Parameters for the search tool
#[derive(Debug, Deserialize)]
pub struct SearchToolParams {
    /// The search query
    pub query: String,
    /// Optional role name to filter results
    pub role: Option<String>,
    /// Optional maximum number of results to return
    pub limit: Option<i32>,
    /// Optional number of results to skip
    pub skip: Option<i32>,
}

/// Parameters for the update_config tool
#[derive(Debug, Deserialize)]
pub struct UpdateConfigToolParams {
    /// The new configuration as a string representation
    pub config_str: String,
}

/// Format a list of documents as a markdown list
fn format_documents_as_markdown(documents: &[Document]) -> String {
    let mut result = format!("Found {} documents:\n\n", documents.len());
    
    for (i, doc) in documents.iter().enumerate() {
        result.push_str(&format!("{}. **{}**", i + 1, doc.title));
        
        if let Some(description) = &doc.description {
            result.push_str(&format!(" - {}", description));
        }
        
        result.push('\n');
    }
    
    result
}

/// Handle the search tool call
pub async fn handle_search(
    router: Arc<TerraphimMcpRouter>,
    request: JsonRpcRequest,
    params: SearchToolParams,
) -> Result<JsonRpcResponse, ToolError> {
    info!("Handling search request: {}", params.query);
    
    let mut service = router.terraphim_service();
    
    // Convert tool parameters to Terraphim SearchQuery
    let search_query = SearchQuery {
        search_term: NormalizedTermValue::from(params.query),
        limit: params.limit.map(|l| l as usize),
        skip: params.skip.map(|s| s as usize),
        role: params.role.map(RoleName::from),
    };
    
    // Execute the search using terraphim_service
    match service.search(&search_query).await {
        Ok(documents) => {
            let mut contents = Vec::new();
            
            // Add a summary as the first item
            let summary = format!("Found {} documents matching your query.", documents.len());
            contents.push(serde_json::json!({
                "text": summary
            }));
            
            // Save the limit value to apply later
            let limit = params.limit.unwrap_or(documents.len() as i32) as usize;
            
            // Add each document as a resource (but respect the limit)
            for (idx, doc) in documents.iter().enumerate() {
                // Only add documents up to the limit
                if idx >= limit {
                    break;
                }
                
                // Format document as a resource
                let resource_uri = format!("terraphim://{}", doc.id);
                
                // Create content that includes a resource field
                contents.push(serde_json::json!({
                    "resource": {
                        "uri": resource_uri,
                        "title": doc.title,
                        "description": doc.description.clone().unwrap_or_default(),
                        "tags": doc.tags.clone().unwrap_or_default()
                    },
                    "text": doc.body
                }));
            }
            
            Ok(JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id: request.id,
                result: Some(json!({
                    "contents": contents,
                })),
                error: None,
            })
        },
        Err(e) => {
            error!("Search failed: {}", e);
            Ok(JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id: request.id,
                result: None,
                error: Some(ErrorData {
                    code: -32603,
                    message: format!("Search failed: {}", e),
                    data: None,
                }),
            })
        }
    }
}

/// Handle the update_config tool call
pub async fn handle_update_config(
    router: Arc<TerraphimMcpRouter>,
    request: JsonRpcRequest,
    params: UpdateConfigToolParams,
) -> Result<JsonRpcResponse, ToolError> {
    info!("Handling update_config request");
    
    // Parse the config string
    match serde_json::from_str::<Config>(&params.config_str) {
        Ok(new_config) => {
            // Update the configuration
            match router.update_config(new_config).await {
                Ok(()) => Ok(JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    id: request.id,
                    result: Some(json!({
                        "status": "success",
                        "message": "Configuration updated successfully"
                    })),
                    error: None,
                }),
                Err(e) => Ok(JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    id: request.id,
                    result: None,
                    error: Some(ErrorData {
                        code: -32603,
                        message: format!("Failed to update configuration: {}", e),
                        data: None,
                    }),
                }),
            }
        },
        Err(e) => {
            error!("Failed to parse config: {}", e);
            Ok(JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id: request.id,
                result: None,
                error: Some(ErrorData {
                    code: -32602, // Invalid params
                    message: format!("Invalid configuration JSON: {}", e),
                    data: None,
                }),
            })
        }
    }
} 