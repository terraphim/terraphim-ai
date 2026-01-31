use anyhow::{Result, anyhow};
use rmcp::model::{Annotated, RawResource, Resource, ResourceContents};
use terraphim_types::Document;

/// URI scheme for Terraphim documents
const TERRAPHIM_URI_SCHEME: &str = "terraphim";

/// A helper struct to map between Terraphim documents and MCP resources
#[derive(Clone, Default)]
pub struct TerraphimResourceMapper;

impl TerraphimResourceMapper {
    /// Create a new resource mapper
    pub fn new() -> Self {
        Self
    }

    /// Convert a list of documents to a list of resources
    pub fn documents_to_resources(&self, documents: &[Document]) -> Result<Vec<Resource>> {
        documents
            .iter()
            .map(|doc| self.document_to_resource(doc))
            .collect()
    }

    /// Convert a single document to a resource
    pub fn document_to_resource(&self, document: &Document) -> Result<Resource> {
        let uri = self.create_document_uri(&document.id);
        let raw = RawResource::new(uri.clone(), document.title.clone());
        Ok(Annotated::new(raw, None))
    }

    /// Convert a URI to a resource ID
    pub fn uri_to_id(&self, uri: &str) -> Result<String> {
        uri.strip_prefix(&format!("{}://", TERRAPHIM_URI_SCHEME))
            .map(|s| s.to_string())
            .ok_or_else(|| anyhow!("Invalid URI: {}", uri))
    }

    /// Convert a Terraphim document to MCP content
    pub fn document_to_resource_contents(&self, document: &Document) -> Result<ResourceContents> {
        let mut text = format!("# {}\n\n", document.title);
        text.push_str(&document.body);

        if let Some(tags) = &document.tags {
            text.push_str("\n\n**Tags:**\n\n");
            for tag in tags {
                text.push_str(&format!("- {}\n", tag));
            }
        }
        let uri = self.create_document_uri(&document.id);
        Ok(ResourceContents::text(text, uri))
    }

    /// Create a URI for a document
    pub fn create_document_uri(&self, document_id: &str) -> String {
        format!("{}://{}", TERRAPHIM_URI_SCHEME, document_id)
    }
}
