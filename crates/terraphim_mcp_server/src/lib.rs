use std::sync::Arc;

use anyhow::Result;
use rmcp::{
    model::{
        CallToolResult, Content, ListResourcesResult, ReadResourceRequestParam, ReadResourceResult,
        ServerInfo, ErrorData
    },
    service::RequestContext,
    tool, Error as McpError, RoleServer, ServerHandler,
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

#[tool(tool_box)]
impl McpService {
    /// Create a new service instance
    pub fn new(config_state: Arc<ConfigState>) -> Self {
        Self {
            config_state,
            resource_mapper: Arc::new(TerraphimResourceMapper::new()),
        }
    }

    /// Create a Terraphim service instance from the current configuration
    pub fn terraphim_service(&self) -> TerraphimService {
        TerraphimService::new((*self.config_state).clone())
    }

    /// Update the configuration
    pub async fn update_config(&self, new_config: Config) -> Result<()> {
        let config = self.config_state.config.clone();
        let mut current_config = config.lock().await;
        *current_config = new_config;
        Ok(())
    }

    #[tool(description = "Search for documents in the Terraphim knowledge graph")]
    pub async fn search(
        &self,
        #[tool(param)] query: String,
        #[tool(param)] role: Option<String>,
        #[tool(param)] limit: Option<i32>,
        #[tool(param)] skip: Option<i32>,
    ) -> Result<CallToolResult, McpError> {
        let mut service = self.terraphim_service();
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

    #[tool(description = "Update the Terraphim configuration")]
    pub async fn update_config_tool(
        &self,
        #[tool(param)] config_str: String,
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
    fn list_resources(
        &self,
        _request: Option<rmcp::model::PaginatedRequestParam>,
        _context: RequestContext<RoleServer>,
    ) -> impl std::future::Future<Output = Result<ListResourcesResult, ErrorData>> + Send + '_ {
        async move {
            let mut service = self.terraphim_service();
            let search_query = terraphim_types::SearchQuery {
                search_term: terraphim_types::NormalizedTermValue::new("".to_string()),
                limit: None,
                skip: None,
                role: None,
            };
            let documents = service
                .search(&search_query)
                .await
                .map_err(TerraphimMcpError::Service)?;
            let resources = self
                .resource_mapper
                .documents_to_resources(&documents)
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
            let mut service = self.terraphim_service();
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
                Err(McpError::resource_not_found(
                    format!("Document not found: {}", doc_id),
                    None,
                )
                .into())
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
