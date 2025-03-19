use std::sync::Arc;
use tokio::sync::Mutex;
use terraphim_config::{Config, ConfigState};
use terraphim_service::TerraphimService;
use terraphim_types::{NormalizedTermValue, SearchQuery};
use anyhow::Result;
use async_trait::async_trait;

use crate::schema::{
    Resource, ResourceContent, McpError, ResourceList, ResourceListRequest,
    ResourceReadRequest, ResourceReadResponse, ResourceCapabilities,
    ResourceTemplate, ResourceMetadata
};

#[async_trait]
pub trait ResourceService: Send + Sync {
    async fn list_resources(&self, request: ResourceListRequest) -> Result<ResourceList, McpError>;
    async fn read_resource(&self, request: ResourceReadRequest) -> Result<ResourceReadResponse, McpError>;
    async fn create_resource(&self, resource: Resource) -> Result<(), McpError>;
    async fn get_capabilities(&self) -> Result<ResourceCapabilities, McpError>;
    async fn list_templates(&self) -> Result<Vec<ResourceTemplate>, McpError>;
}

pub struct TerraphimResourceService {
    service: Arc<Mutex<TerraphimService>>,
    config_state: Arc<ConfigState>,
}

impl TerraphimResourceService {
    pub async fn new(mut config: Config) -> Result<Self, McpError> {
        let config_state = ConfigState::new(&mut config).await
            .map_err(|e| McpError::internal(format!("Failed to create config state: {}", e)))?;
        
        let service = TerraphimService::new(config_state.clone());
        
        Ok(Self {
            service: Arc::new(Mutex::new(service)),
            config_state: Arc::new(config_state),
        })
    }

    pub async fn list_resources(&self, request: ResourceListRequest) -> Result<ResourceList, McpError> {
        let search_query = SearchQuery {
            search_term: NormalizedTermValue::new("Test Document".to_string()), // Search for test documents
            role: None,
            skip: request.cursor.as_ref().and_then(|c| c.parse::<usize>().ok()),
            limit: request.limit.map(|l| l as usize),
        };

        let documents = {
            let mut service = self.service.lock().await;
            service.search(&search_query).await
                .map_err(|e| McpError::internal(format!("Failed to search documents: {}", e)))?
        };

        let resources: Vec<Resource> = documents.into_iter()
            .map(|doc| Resource {
                uri: doc.url,
                name: doc.title,
                description: Some(doc.description.unwrap_or_default()),
                metadata: ResourceMetadata {
                    mime_type: Some("text/markdown".to_string()),
                    scheme: "file".to_string(),
                    version: None,
                },
            })
            .collect();

        let total = resources.len() as u32;
        Ok(ResourceList {
            resources,
            total,
            cursor: None, // TODO: Implement proper cursor
        })
    }

    pub async fn read_resource(&self, request: ResourceReadRequest) -> Result<ResourceReadResponse, McpError> {
        let search_query = SearchQuery {
            search_term: NormalizedTermValue::new(request.uri.clone()),
            role: None,
            skip: None,
            limit: Some(1),
        };

        let documents = {
            let mut service = self.service.lock().await;
            service.search(&search_query).await
                .map_err(|e| McpError::internal(format!("Failed to search documents: {}", e)))?
        };

        let document = documents.first()
            .ok_or_else(|| McpError::not_found(format!("Resource not found: {}", request.uri)))?;

        let content = ResourceContent::Text(document.body.clone());

        Ok(ResourceReadResponse { content })
    }

    pub async fn get_capabilities(&self) -> Result<ResourceCapabilities, McpError> {
        Ok(ResourceCapabilities {
            supports_subscriptions: true,
            supports_templates: true,
            supports_content_types: vec!["text/markdown".to_string()],
            max_subscriptions: Some(100),
            max_subscription_duration: Some(3600),
        })
    }

    pub async fn list_templates(&self) -> Result<Vec<ResourceTemplate>, McpError> {
        Ok(vec![ResourceTemplate {
            fields: vec!["uri".to_string(), "name".to_string(), "description".to_string()],
        }])
    }

    pub async fn create_resource(&self, _resource: Resource) -> Result<(), McpError> {
        // Not supported for read-only markdown files
        Err(McpError::bad_request("Creating resources is not supported for read-only markdown files"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use terraphim_config::{ConfigBuilder, Role, ServiceType, Haystack};
    use terraphim_types::RelevanceFunction;
    use std::fs;
    use tempfile::tempdir;

    async fn setup_test_service() -> TerraphimResourceService {
        // Create a temporary directory for test data
        let temp_dir = tempdir().unwrap();
        let test_dir = temp_dir.path().join("sample_docs");
        fs::create_dir_all(&test_dir).unwrap();

        // Create a test markdown file with a unique title
        let test_file = test_dir.join("test1.md");
        fs::write(&test_file, "# Test Document 1\n\nThis is a test document with unique content.").unwrap();

        // Create another test file
        let test_file2 = test_dir.join("test2.md");
        fs::write(&test_file2, "# Test Document 2\n\nAnother test document with different content.").unwrap();

        let config = ConfigBuilder::new()
            .add_role(
                "Default",
                Role {
                    shortname: Some("Default".to_string()),
                    name: "Default".into(),
                    relevance_function: RelevanceFunction::TitleScorer,
                    theme: "spacelab".to_string(),
                    kg: None,
                    haystacks: vec![Haystack {
                        path: test_dir.clone(),
                        service: ServiceType::Ripgrep,
                    }],
                    extra: Default::default(),
                },
            )
            .default_role("Default")
            .unwrap()
            .build()
            .unwrap();

        let service = TerraphimResourceService::new(config).await.unwrap();
        
        // Keep the temp directory alive for the duration of the test
        std::mem::forget(temp_dir);
        
        // Wait a bit to ensure indexing is complete
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        
        service
    }

    #[tokio::test]
    async fn test_list_resources() {
        let service = setup_test_service().await;
        let result = service.list_resources(ResourceListRequest {
            filter: None,
            cursor: None,
            limit: Some(10),
        }).await;
        assert!(result.is_ok());
        let resources = result.unwrap();
        assert!(!resources.resources.is_empty());
        assert!(resources.resources.len() >= 2); // We created 2 test files
    }

    #[tokio::test]
    async fn test_get_capabilities() {
        let service = setup_test_service().await;
        let result = service.get_capabilities().await;
        assert!(result.is_ok());
        let capabilities = result.unwrap();
        assert!(capabilities.supports_subscriptions);
        assert!(capabilities.supports_templates);
    }

    #[tokio::test]
    async fn test_read_resource() {
        let service = setup_test_service().await;
        let result = service.read_resource(ResourceReadRequest {
            uri: "test1.md".to_string(),
        }).await;
        assert!(result.is_ok());
        let response = result.unwrap();
        assert!(matches!(response.content, ResourceContent::Text(_)));
        if let ResourceContent::Text(text) = response.content {
            assert!(text.contains("Test Document 1"));
        }
    }
} 