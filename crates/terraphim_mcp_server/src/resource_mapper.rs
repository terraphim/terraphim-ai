use anyhow::{anyhow, Result};
use mcp_core::resource::Resource;
use terraphim_types::Document;
use url::Url;

/// URI scheme for Terraphim documents
const TERRAPHIM_URI_SCHEME: &str = "terraphim";

/// TerraphimResourceMapper handles mapping between Terraphim types and MCP resources
#[derive(Clone, Debug)]
pub struct TerraphimResourceMapper {}

impl TerraphimResourceMapper {
    /// Create a new resource mapper
    pub fn new() -> Self {
        Self {}
    }

    /// Convert a Terraphim document to an MCP resource
    pub fn document_to_resource(&self, document: &Document, priority: f32) -> Result<Resource> {
        let uri = self.create_document_uri(&document.id);
        
        // Create a resource with the document title as the name
        let mut resource = Resource::with_uri(
            uri,
            document.title.clone(),
            priority,
            Some("text".to_string()),
        )?;
        
        // Add description if available
        if let Some(description) = &document.description {
            resource = resource.with_description(description.clone());
        }
        
        Ok(resource)
    }
    
    /// Create a URI for a Terraphim document
    pub fn create_document_uri(&self, document_id: &str) -> String {
        format!("{}://{}", TERRAPHIM_URI_SCHEME, document_id)
    }
    
    /// Extract the document ID from a Terraphim resource URI
    pub fn extract_document_id_from_uri(&self, uri: &str) -> Result<String> {
        let url = Url::parse(uri)
            .map_err(|e| anyhow!("Invalid URI '{}': {}", uri, e))?;
            
        if url.scheme() != TERRAPHIM_URI_SCHEME {
            return Err(anyhow!(
                "Invalid URI scheme: expected '{}', got '{}'",
                TERRAPHIM_URI_SCHEME,
                url.scheme()
            ));
        }
        
        // The host part contains the document ID
        Ok(url.host_str()
            .ok_or_else(|| anyhow!("Missing document ID in URI: {}", uri))?
            .to_string())
    }
    
    /// Create a batch of resources from Terraphim documents with decreasing priority
    pub fn documents_to_resources(&self, documents: &[Document]) -> Result<Vec<Resource>> {
        let mut resources = Vec::with_capacity(documents.len());
        
        // Set decreasing priority based on order
        for (i, document) in documents.iter().enumerate() {
            // Priority decreases with index, starting at 1.0
            let priority = 1.0 - (i as f32 * 0.01);
            let priority = if priority < 0.0 { 0.0 } else { priority };
            
            let resource = self.document_to_resource(document, priority)?;
            resources.push(resource);
        }
        
        Ok(resources)
    }
} 