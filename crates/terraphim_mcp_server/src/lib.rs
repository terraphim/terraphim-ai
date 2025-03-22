use std::future::Future;
use std::pin::Pin;

use anyhow::Result;
use mcp_core::{
    handler::{PromptError, ResourceError, ToolError},
    prompt::Prompt,
    protocol::ServerCapabilities,
    resource::Resource,
    tool::Tool,
    Content,
};
use mcp_server::router::CapabilitiesBuilder;
use serde_json::Value;
use terraphim_config::ConfigState;
use terraphim_service::{ServiceError, TerraphimService};
use terraphim_types::Document;
use thiserror::Error;

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

/// Terraphim Model Context Protocol Server Router implementation
#[derive(Clone)]
pub struct TerraphimMcpRouter {
    /// Terraphim configuration state
    config_state: ConfigState,
    /// Resource mapper for converting Terraphim types to MCP resources
    resource_mapper: TerraphimResourceMapper,
}

impl TerraphimMcpRouter {
    /// Create a new Terraphim MCP Router with the provided configuration
    pub fn new(config_state: ConfigState) -> Self {
        Self {
            config_state,
            resource_mapper: TerraphimResourceMapper::new(),
        }
    }

    /// Create a Terraphim service instance from the current configuration
    fn terraphim_service(&self) -> TerraphimService {
        TerraphimService::new(self.config_state.clone())
    }

    /// Convert a Terraphim Document to MCP TextContent
    fn document_to_text_content(document: &Document) -> Content {
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
        
        Content::text(text)
    }
}

/// Implement the MCP Router trait for TerraphimMcpRouter
impl mcp_server::Router for TerraphimMcpRouter {
    fn name(&self) -> String {
        "terraphim-mcp".to_string()
    }

    fn instructions(&self) -> String {
        "This server provides Terraphim knowledge graph search capabilities through the Model Context Protocol. You can search for documents using the search tool and access resources that represent Terraphim documents.".to_string()
    }

    fn capabilities(&self) -> ServerCapabilities {
        CapabilitiesBuilder::new()
            .with_tools(true)
            .with_resources(true, false)
            .with_prompts(false)
            .build()
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
        
        Box::pin(async move {
            match tool_name.as_str() {
                "search" => {
                    // Parse the search parameters
                    match serde_json::from_value::<SearchToolParams>(arguments) {
                        Ok(params) => {
                            handle_search(this, params).await
                        }
                        Err(err) => {
                            Err(ToolError::InvalidParameters(format!("Invalid search parameters: {}", err)))
                        }
                    }
                }
                "update_config" => {
                    // Parse the update_config parameters
                    match serde_json::from_value::<UpdateConfigToolParams>(arguments) {
                        Ok(params) => {
                            handle_update_config(this, params).await
                        }
                        Err(err) => {
                            Err(ToolError::InvalidParameters(format!("Invalid configuration parameters: {}", err)))
                        }
                    }
                }
                _ => Err(ToolError::NotFound(format!("Tool {} not found", tool_name))),
            }
        })
    }

    fn list_resources(&self) -> Vec<Resource> {
        // In test mode, create a few test resources to make it easier to test the API
        if cfg!(test) || std::env::var("TERRAPHIM_TEST_MODE").is_ok() {
            let mut resources = Vec::new();
            
            // Create a few test document resources
            let test_docs = vec![
                ("test-doc-1", "Test Document 1", "A test document for e2e testing"),
                ("test-doc-2", "Test Document 2", "Another test document for e2e testing"),
                ("system-doc", "System Document", "A document about systems for testing search"),
                ("terraphim-doc", "Terraphim Document", "A document about terraphim for testing search"),
            ];
            
            for (i, (id, title, description)) in test_docs.iter().enumerate() {
                let uri = format!("terraphim://{}", id);
                let priority = 1.0 - (i as f32 * 0.1);
                
                if let Ok(resource) = Resource::with_uri(
                    uri,
                    title.to_string(),
                    priority,
                    Some("text".to_string()),
                ) {
                    let resource_with_desc = resource.with_description(description.to_string());
                    resources.push(resource_with_desc);
                }
            }
            
            tracing::debug!("Returning {} test resources for e2e testing", resources.len());
            return resources;
        }
        
        // Return an empty list for normal operation
        // Resources will be created dynamically based on search results
        Vec::new()
    }

    fn read_resource(
        &self,
        uri: &str,
    ) -> Pin<Box<dyn Future<Output = Result<String, ResourceError>> + Send + 'static>> {
        let uri = uri.to_string();
        let this = self.clone();
        
        Box::pin(async move {
            let document_id = match this.resource_mapper.extract_document_id_from_uri(&uri) {
                Ok(id) => id,
                Err(err) => return Err(ResourceError::NotFound(format!("Invalid URI: {}", err))),
            };
            
            // In test mode, check for test resources first
            if cfg!(test) || std::env::var("TERRAPHIM_TEST_MODE").is_ok() {
                let test_docs = vec![
                    ("test-doc-1", "Test Document 1", "This is a test document for e2e testing.\n\nIt contains multiple paragraphs and demonstrates the resource reading functionality."),
                    ("test-doc-2", "Test Document 2", "Another test document with different content.\n\nThis document can be used to verify the resource reading API."),
                    ("system-doc", "System Document", "# System Documentation\n\nThis document describes various system concepts and terminology.\n\n## Components\n\n- Hardware\n- Software\n- Infrastructure\n\n## Maintenance\n\nRegular system maintenance is essential for optimal performance."),
                    ("terraphim-doc", "Terraphim Document", "# Terraphim Knowledge Graph\n\nTerraphim is a knowledge graph system for organizing and accessing information.\n\n## Features\n\n- Fast search\n- Document indexing\n- Role-based access\n\n## Use Cases\n\nTerraphim can be used for various knowledge management scenarios.")
                ];
                
                // Check if the document_id matches any test document
                for (id, title, content) in test_docs {
                    if id == document_id {
                        let formatted = format!("# {}\n\n{}", title, content);
                        tracing::debug!("Found test document: {}", id);
                        return Ok(formatted);
                    }
                }
                
                tracing::debug!("No test document found with id: {}", document_id);
            }
            
            // If not in test mode or test document not found, proceed with normal search
            let mut service = this.terraphim_service();
            
            // Use search with document ID to find the document
            let search_term = terraphim_types::NormalizedTermValue::from(document_id.clone());
            let search_query = terraphim_types::SearchQuery {
                search_term,
                skip: None,
                limit: Some(1),
                role: None,
            };
            
            let documents = match service.search(&search_query).await {
                Ok(docs) => docs,
                Err(err) => {
                    tracing::error!("Error searching for document {}: {}", document_id, err);
                    return Err(ResourceError::NotFound(format!(
                        "Document with id {} not found: {}", document_id, err
                    )))
                },
            };
            
            let document = match documents.first() {
                Some(doc) if doc.id == document_id => doc.clone(),
                _ => {
                    tracing::warn!("Document with id {} not found in search results", document_id);
                    return Err(ResourceError::NotFound(format!("Document with id {} not found", document_id)));
                }
            };
            
            // Format the document as text
            let mut content = String::new();
            content.push_str(&format!("# {}\n\n", document.title));
            
            if let Some(description) = &document.description {
                content.push_str(&format!("{}\n\n", description));
            }
            
            content.push_str(&document.body);
            
            if let Some(tags) = &document.tags {
                content.push_str("\n\n## Tags\n\n");
                content.push_str(&tags.join(", "));
            }
            
            Ok(content)
        })
    }

    fn list_prompts(&self) -> Vec<Prompt> {
        // No prompts are supported for now
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
}
