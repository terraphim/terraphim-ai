use mcp_core::{
    handler::ToolError,
    resource::{Resource, ResourceContents},
    Content,
};
use serde::{Deserialize, Serialize};
use terraphim_types::{Document, NormalizedTermValue, RoleName, SearchQuery};

use crate::{McpError, TerraphimMcpRouter};

/// Parameters for the search tool
#[derive(Debug, Serialize, Deserialize)]
pub struct SearchToolParams {
    /// The search query
    pub query: String,
    /// Optional role name to filter results
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    /// Optional maximum number of results to return
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<usize>,
    /// Optional number of results to skip
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skip: Option<usize>,
}

/// Format a list of documents as a markdown list
fn format_documents_as_markdown(documents: &[Document]) -> String {
    let mut result = format!("Found {} documents:\n\n", documents.len());
    
    for (i, doc) in documents.iter().enumerate() {
        result.push_str(&format!("{}. **{}**", i + 1, doc.title));
        
        if let Some(description) = &doc.description {
            result.push_str(&format!(" - {}", description));
        }
        
        result.push_str(&format!(" [terraphim://{}]\n", doc.id));
    }
    
    result
}

/// Create an MCP resource content from a Resource
fn to_resource_content(resource: Resource) -> Content {
    // Convert Resource to ResourceContents
    let uri = resource.uri.clone();
    let mime_type = Some(resource.mime_type.clone());
    let resource_contents = ResourceContents::TextResourceContents {
        uri,
        mime_type,
        text: format!("# {}\n\n{}", 
            resource.name,
            resource.description.unwrap_or_default()),
    };
    
    Content::resource(resource_contents)
}

/// Handle the search tool call
pub async fn handle_search(
    router: TerraphimMcpRouter,
    params: SearchToolParams,
) -> Result<Vec<Content>, ToolError> {
    // Create a Terraphim service instance
    let mut terraphim_service = router.terraphim_service();
    
    // Convert tool parameters to Terraphim SearchQuery
    let search_query = SearchQuery {
        search_term: NormalizedTermValue::from(params.query),
        limit: params.limit,
        skip: params.skip,
        role: params.role.map(RoleName::from),
    };
    
    // Execute the search
    let documents = terraphim_service.search(&search_query)
        .await
        .map_err(|e| McpError::Service(e))
        .map_err(|e| ToolError::ExecutionError(format!("Search failed: {}", e)))?;
    
    if documents.is_empty() {
        return Ok(vec![Content::text("No documents found matching the query.")]);
    }
    
    // Convert search results to resources
    let resources = router.resource_mapper.documents_to_resources(&documents)
        .map_err(|e| ToolError::ExecutionError(format!("Failed to convert documents to resources: {}", e)))?;
    
    // Create a markdown summary of the results
    let summary = format_documents_as_markdown(&documents);
    
    // Return both the summary and the resources
    let mut contents = vec![Content::text(summary)];
    
    // Convert resources to content
    for resource in resources {
        contents.push(to_resource_content(resource));
    }
    
    Ok(contents)
} 