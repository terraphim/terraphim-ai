use anyhow::{anyhow, Result};
use mcp_core::{
    resource::{Resource, ResourceContents},
    Content,
};
use terraphim_types::Document;
use url::Url;
use terraphim_config::{ConfigState, Config};
use terraphim_service::TerraphimService;
use std::sync::Arc;
use tokio::sync::Mutex;
use ahash::AHashMap;

/// URI scheme for Terraphim documents
const TERRAPHIM_URI_SCHEME: &str = "terraphim";

/// TerraphimResourceMapper handles mapping between Terraphim types and MCP resources
#[derive(Clone, Debug)]
pub struct TerraphimResourceMapper {
    config_state: Arc<ConfigState>,
}

impl TerraphimResourceMapper {
    /// Create a new resource mapper
    pub fn new() -> Self {
        // Create a config state with an empty config
        let config = Config::default();
        let config_state = ConfigState {
            config: Arc::new(Mutex::new(config)),
            roles: AHashMap::default(),
        };
        
        Self { 
            config_state: Arc::new(config_state),
        }
    }

    /// Set the config state
    pub fn with_config_state(mut self, config_state: Arc<ConfigState>) -> Self {
        self.config_state = config_state;
        self
    }

    /// Create a terraphim service instance
    fn terraphim_service(&self) -> TerraphimService {
        // Dereference the Arc to pass ConfigState instead of Arc<ConfigState>
        TerraphimService::new((*self.config_state).clone())
    }

    /// Convert a Terraphim document to an MCP resource
    pub async fn document_to_resource(&self, document: &Document, priority: f32) -> Result<Resource> {
        let uri = self.create_document_uri(&document.id);
        
        // Create a resource with the document title as the name
        let mut resource = Resource::with_uri(
            uri.clone(),
            document.title.clone(),
            priority,
            Some("text/markdown".to_string()),
        )?;
        
        // Add description if available
        if let Some(description) = &document.description {
            resource = resource.with_description(description.clone());
        }
        
        Ok(resource)
    }
    
    /// Convert a Terraphim document to MCP content
    pub async fn document_to_content(&self, document: &Document) -> Content {
        let uri = self.create_document_uri(&document.id);
        
        // Create resource contents
        let mut text = String::new();
        text.push_str(&format!("# {}\n\n", document.title));
        
        if let Some(description) = &document.description {
            text.push_str(&format!("{}\n\n", description));
        }
        
        text.push_str(&document.body);
        
        if let Some(tags) = &document.tags {
            text.push_str("\n\nTags: ");
            text.push_str(&tags.join(", "));
        }
        
        let contents = ResourceContents::TextResourceContents {
            uri,
            mime_type: Some("text/markdown".to_string()),
            text,
        };
        
        Content::resource(contents)
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
                "Invalid URI scheme '{}', expected '{}'",
                url.scheme(),
                TERRAPHIM_URI_SCHEME
            ));
        }
        
        Ok(url.host_str()
            .ok_or_else(|| anyhow!("Missing document ID in URI '{}'", uri))?
            .to_string())
    }
    
    /// Convert a list of Terraphim documents to MCP resources
    pub fn documents_to_resources(&self, documents: &[Document]) -> Result<Vec<Resource>> {
        let mut resources = Vec::with_capacity(documents.len());
        
        for (i, document) in documents.iter().enumerate() {
            let priority = 1.0 - (i as f32 * 0.02);
            // Using synchronous version for simplicity
            let uri = self.create_document_uri(&document.id);
            let mut resource = Resource::with_uri(
                uri.clone(),
                document.title.clone(),
                priority,
                Some("text/markdown".to_string()),
            )?;
            
            if let Some(description) = &document.description {
                resource = resource.with_description(description.clone());
            }
            
            resources.push(resource);
        }
        
        Ok(resources)
    }

    /// Get a document by its ID using TerraphimService
    pub async fn get_document(&self, document_id: &str) -> Result<Document, terraphim_service::ServiceError> {
        // Create a new service instance for each call
        let mut service = self.terraphim_service();
        
        // Search for the specific document
        let search_query = terraphim_types::SearchQuery {
            search_term: terraphim_types::NormalizedTermValue::new(document_id.to_string()),
            limit: Some(1),
            skip: None,
            role: None,
        };
        
        let documents = service.search(&search_query).await?;
        
        // Get the document that matches the ID
        documents.into_iter()
            .find(|doc| doc.id == document_id)
            .ok_or_else(|| terraphim_service::ServiceError::Config(format!("Document not found: {}", document_id)))
    }
} 