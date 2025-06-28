use std::sync::Arc;

use anyhow::Result;
use rmcp::{
    model::{
        CallToolResult, Content, ListResourcesResult, ReadResourceRequestParam, ReadResourceResult,
        ServerInfo, ErrorData, ListToolsResult, Tool, CallToolRequestParam
    },
    service::RequestContext,
    Error as McpError, RoleServer, ServerHandler,
};
use terraphim_config::{Config, ConfigState};
use terraphim_service::TerraphimService;
use terraphim_types::{NormalizedTermValue, RoleName, SearchQuery};
use tracing::error;
use thiserror::Error;

pub mod resource_mapper;

use crate::resource_mapper::TerraphimResourceMapper;

#[derive(Error, Debug)]
pub enum TerraphimMcpError {
    #[error("Service error: {0}")]
    Service(#[from] terraphim_service::ServiceError),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("MCP error: {0}")]
    Mcp(#[from] McpError),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Anyhow error: {0}")]
    Anyhow(#[from] anyhow::Error),
}

impl From<TerraphimMcpError> for ErrorData {
    fn from(err: TerraphimMcpError) -> Self {
        ErrorData::internal_error(err.to_string(), None)
    }
}

/// The main service type for the Terraphim MCP server
#[derive(Clone)]
pub struct McpService {
    config_state: Arc<ConfigState>,
    resource_mapper: Arc<TerraphimResourceMapper>,
}

impl McpService {
    /// Create a new service instance
    pub fn new(config_state: Arc<ConfigState>) -> Self {
        Self {
            config_state,
            resource_mapper: Arc::new(TerraphimResourceMapper::new()),
        }
    }

    /// Create a Terraphim service instance from the current configuration
    pub async fn terraphim_service(&self) -> Result<TerraphimService, anyhow::Error> {
        // Instead of cloning the old ConfigState (which has stale roles), 
        // create a fresh ConfigState from the current config to ensure roles are up-to-date
        let config = self.config_state.config.clone();
        let current_config = config.lock().await;
        let mut fresh_config = current_config.clone();
        drop(current_config);
        
        let fresh_config_state = terraphim_config::ConfigState::new(&mut fresh_config)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to create fresh ConfigState: {}", e))?;
            
        Ok(TerraphimService::new(fresh_config_state))
    }

    /// Update the configuration
    pub async fn update_config(&self, new_config: Config) -> Result<()> {
        let config = self.config_state.config.clone();
        let mut current_config = config.lock().await;
        *current_config = new_config;
        Ok(())
    }

    /// Search for documents in the Terraphim knowledge graph
    pub async fn search(
        &self,
        query: String,
        role: Option<String>,
        limit: Option<i32>,
        skip: Option<i32>,
    ) -> Result<CallToolResult, McpError> {
        let mut service = self.terraphim_service().await
            .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
        let search_query = SearchQuery {
            search_term: NormalizedTermValue::from(query),
            limit: limit.map(|l| l as usize),
            skip: skip.map(|s| s as usize),
            role: role.map(RoleName::from),
        };

        match service.search(&search_query).await {
            Ok(documents) => {
                let mut contents = Vec::new();
                let summary = format!("Found {} documents matching your query.", documents.len());
                contents.push(Content::text(summary));

                let limit = limit.unwrap_or(documents.len() as i32) as usize;
                for (idx, doc) in documents.iter().enumerate() {
                    if idx >= limit {
                        break;
                    }

                    let resource_contents = self
                        .resource_mapper
                        .document_to_resource_contents(doc)
                        .unwrap();
                    contents.push(Content::resource(resource_contents));
                }

                Ok(CallToolResult::success(contents))
            }
            Err(e) => {
                error!("Search failed: {}", e);
                let error_content = Content::text(format!("Search failed: {}", e));
                Ok(CallToolResult::error(vec![error_content]))
            }
        }
    }

    /// Update the Terraphim configuration
    pub async fn update_config_tool(
        &self,
        config_str: String,
    ) -> Result<CallToolResult, McpError> {
        match serde_json::from_str::<Config>(&config_str) {
            Ok(new_config) => match self.update_config(new_config).await {
                Ok(()) => {
                    let content = Content::text("Configuration updated successfully".to_string());
                    Ok(CallToolResult::success(vec![content]))
                }
                Err(e) => {
                    error!("Failed to update configuration: {}", e);
                    let error_content =
                        Content::text(format!("Failed to update configuration: {}", e));
                    Ok(CallToolResult::error(vec![error_content]))
                }
            },
            Err(e) => {
                error!("Failed to parse config: {}", e);
                let error_content = Content::text(format!("Invalid configuration JSON: {}", e));
                Ok(CallToolResult::error(vec![error_content]))
            }
        }
    }
}

impl ServerHandler for McpService {
    fn list_tools(
        &self,
        _request: Option<rmcp::model::PaginatedRequestParam>,
        _context: RequestContext<RoleServer>,
    ) -> impl std::future::Future<Output = Result<ListToolsResult, ErrorData>> + Send + '_ {
        async move {
            // Convert JSON values to Arc<Map<String, Value>> for input_schema
            let search_schema = serde_json::json!({
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "The search query"
                    },
                    "role": {
                        "type": "string",
                        "description": "Optional role to filter by"
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Maximum number of results to return"
                    },
                    "skip": {
                        "type": "integer",
                        "description": "Number of results to skip"
                    }
                },
                "required": ["query"]
            });
            let search_map = search_schema.as_object().unwrap().clone();
            
            let config_schema = serde_json::json!({
                "type": "object",
                "properties": {
                    "config_str": {
                        "type": "string",
                        "description": "JSON configuration string"
                    }
                },
                "required": ["config_str"]
            });
            let config_map = config_schema.as_object().unwrap().clone();

            let tools = vec![
                Tool {
                    name: "search".into(),
                    description: Some("Search for documents in the Terraphim knowledge graph".into()),
                    input_schema: Arc::new(search_map),
                    annotations: None,
                },
                Tool {
                    name: "update_config_tool".into(),
                    description: Some("Update the Terraphim configuration".into()),
                    input_schema: Arc::new(config_map),
                    annotations: None,
                }
            ];
            Ok(ListToolsResult { 
                tools,
                next_cursor: None,
            })
        }
    }

    fn call_tool(
        &self,
        request: CallToolRequestParam,
        _context: RequestContext<RoleServer>,
    ) -> impl std::future::Future<Output = Result<CallToolResult, ErrorData>> + Send + '_ {
        async move {
            match request.name.as_ref() {
                "search" => {
                    let arguments = request.arguments.unwrap_or_default();
                    let query = arguments.get("query")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| ErrorData::invalid_params("Missing 'query' parameter".to_string(), None))?
                        .to_string();
                    
                    let role = arguments.get("role")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());
                    
                    let limit = arguments.get("limit")
                        .and_then(|v| v.as_i64())
                        .map(|i| i as i32);
                    
                    let skip = arguments.get("skip")
                        .and_then(|v| v.as_i64())
                        .map(|i| i as i32);
                    
                    self.search(query, role, limit, skip).await
                        .map_err(TerraphimMcpError::Mcp)
                        .map_err(ErrorData::from)
                }
                "update_config_tool" => {
                    let arguments = request.arguments.unwrap_or_default();
                    let config_str = arguments.get("config_str")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| ErrorData::invalid_params("Missing 'config_str' parameter".to_string(), None))?
                        .to_string();
                    
                    self.update_config_tool(config_str).await
                        .map_err(TerraphimMcpError::Mcp)
                        .map_err(ErrorData::from)
                }
                _ => Err(ErrorData::method_not_found::<rmcp::model::CallToolRequestMethod>())
            }
        }
    }

    fn list_resources(
        &self,
        _request: Option<rmcp::model::PaginatedRequestParam>,
        _context: RequestContext<RoleServer>,
    ) -> impl std::future::Future<Output = Result<ListResourcesResult, ErrorData>> + Send + '_ {
        async move {
            let mut service = self.terraphim_service().await
                .map_err(|e| TerraphimMcpError::Anyhow(e))?;
            
            // Use a broad search term to find documents instead of empty search
            // We'll try common terms that should match documents in our KG
            let search_terms = vec!["terraphim", "graph", "service", "haystack"];
            let mut all_documents = std::collections::HashSet::new();
            
            // Perform multiple searches to gather available documents
            for term in search_terms {
                let search_query = terraphim_types::SearchQuery {
                    search_term: terraphim_types::NormalizedTermValue::new(term.to_string()),
                    limit: Some(50), // Reasonable limit per search
                    skip: None,
                    role: None,
                };
                
                match service.search(&search_query).await {
                    Ok(documents) => {
                        for doc in documents {
                            all_documents.insert(doc.id.clone());
                        }
                    }
                    Err(_) => {
                        // Continue with other terms if one fails
                        continue;
                    }
                }
            }
            
            // If we still have no documents, try a final broad search
            if all_documents.is_empty() {
                let fallback_query = terraphim_types::SearchQuery {
                    search_term: terraphim_types::NormalizedTermValue::new("*".to_string()),
                    limit: Some(100),
                    skip: None,
                    role: None,
                };
                
                let documents = service
                    .search(&fallback_query)
                    .await
                    .map_err(TerraphimMcpError::Service)?;
                    
                let resources = self
                    .resource_mapper
                    .documents_to_resources(&documents)
                    .map_err(TerraphimMcpError::Anyhow)?;
                    
                return Ok(ListResourcesResult {
                    resources,
                    next_cursor: None,
                });
            }
            
            // Convert unique document IDs back to documents for resource mapping
            // For now, we'll do individual searches to get full document objects
            let mut final_documents = Vec::new();
            for doc_id in all_documents.iter().take(50) { // Limit to 50 resources
                if let Ok(Some(doc)) = service.get_document_by_id(doc_id).await {
                    final_documents.push(doc);
                }
            }
            
            let resources = self
                .resource_mapper
                .documents_to_resources(&final_documents)
                .map_err(TerraphimMcpError::Anyhow)?;
                
            Ok(ListResourcesResult {
                resources,
                next_cursor: None,
            })
        }
    }

    fn read_resource(
        &self,
        request: ReadResourceRequestParam,
        _context: RequestContext<RoleServer>,
    ) -> impl std::future::Future<Output = Result<ReadResourceResult, ErrorData>> + Send + '_ {
        async move {
            let doc_id = self
                .resource_mapper
                .uri_to_id(&request.uri)
                .map_err(TerraphimMcpError::Anyhow)?;
            let mut service = self.terraphim_service().await
                .map_err(|e| TerraphimMcpError::Anyhow(e))?;
            let document = service
                .get_document_by_id(&doc_id)
                .await
                .map_err(TerraphimMcpError::Service)?;
            if let Some(doc) = document {
                let contents = self
                    .resource_mapper
                    .document_to_resource_contents(&doc)
                    .map_err(TerraphimMcpError::Anyhow)?;
                Ok(ReadResourceResult {
                    contents: vec![contents],
                })
            } else {
                Err(ErrorData::resource_not_found(
                    format!("Document not found: {}", doc_id),
                    None,
                ))
            }
        }
    }

    fn get_info(&self) -> ServerInfo {
        let server_info = ServerInfo {
            server_info: rmcp::model::Implementation {
                name: "terraphim-mcp".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
            },
            instructions: Some("This server provides Terraphim knowledge graph search capabilities through the Model Context Protocol. You can search for documents using the search tool and access resources that represent Terraphim documents.".to_string()),
            ..Default::default()
        };
        server_info
    }
}
