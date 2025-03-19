use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use dashmap::DashMap;
use chrono::Utc;
use uuid::Uuid;

use crate::schema::*;
use crate::error::ServerError;

/// Service trait for resource operations
#[async_trait::async_trait]
pub trait ResourceService: Send + Sync {
    async fn list_resources(&self, request: ResourceListRequest) -> Result<ResourceList, ServerError>;
    async fn read_resource(&self, request: ResourceReadRequest) -> Result<ResourceReadResponse, ServerError>;
    async fn subscribe(&self, request: ResourceSubscribeRequest) -> Result<(), ServerError>;
    async fn unsubscribe(&self, request: ResourceUnsubscribeRequest) -> Result<(), ServerError>;
    async fn get_capabilities(&self) -> Result<ResourceCapabilities, ServerError>;
    async fn list_templates(&self) -> Result<Vec<ResourceTemplate>, ServerError>;
}

/// In-memory implementation of ResourceService
pub struct InMemoryResourceService {
    resources: Arc<DashMap<String, Resource>>,
    subscriptions: Arc<DashMap<String, Vec<ResourceSubscription>>>,
    capabilities: ResourceCapabilities,
    templates: Vec<ResourceTemplate>,
}

impl InMemoryResourceService {
    pub fn new() -> Self {
        Self {
            resources: Arc::new(DashMap::new()),
            subscriptions: Arc::new(DashMap::new()),
            capabilities: ResourceCapabilities {
                subscribe: true,
                list_changed: true,
            },
            templates: vec![
                ResourceTemplate {
                    uri_template: "file:///{path}".to_string(),
                    name: "Project Files".to_string(),
                    description: "Access files in the project directory".to_string(),
                    schema: "file".to_string(),
                    mime_type: Some("application/octet-stream".to_string()),
                },
            ],
        }
    }

    fn generate_cursor(&self, resources: &[Resource], limit: usize) -> Option<String> {
        if resources.len() > limit {
            Some(Uuid::new_v4().to_string())
        } else {
            None
        }
    }

    fn apply_filters(&self, resources: Vec<Resource>, filter: &Option<ResourceFilter>) -> Vec<Resource> {
        if let Some(filter) = filter {
            resources
                .into_iter()
                .filter(|resource| {
                    if let Some(mime_type) = &filter.mime_type {
                        if resource.mime_type.as_ref() != Some(mime_type) {
                            return false;
                        }
                    }
                    if let Some(scheme) = &filter.scheme {
                        if resource.uri.scheme != *scheme {
                            return false;
                        }
                    }
                    if let Some(pattern) = &filter.name_pattern {
                        if !resource.name.contains(pattern) {
                            return false;
                        }
                    }
                    true
                })
                .collect()
        } else {
            resources
        }
    }
}

#[async_trait::async_trait]
impl ResourceService for InMemoryResourceService {
    async fn list_resources(&self, request: ResourceListRequest) -> Result<ResourceList, ServerError> {
        let mut resources: Vec<Resource> = self.resources.iter().map(|r| r.clone()).collect();
        
        // Apply filters
        resources = self.apply_filters(resources, &request.filter);
        
        // Apply pagination
        let limit = request.limit.unwrap_or(50);
        let start_idx = if let Some(_cursor) = &request.cursor {
            // In a real implementation, you would decode the cursor and use it to find the starting index
            0
        } else {
            0
        };
        
        let end_idx = (start_idx + limit).min(resources.len());
        let paginated_resources = resources[start_idx..end_idx].to_vec();
        
        Ok(ResourceList {
            resources: paginated_resources.clone(),
            next_cursor: self.generate_cursor(&paginated_resources, limit),
        })
    }

    async fn read_resource(&self, request: ResourceReadRequest) -> Result<ResourceReadResponse, ServerError> {
        let resource = self
            .resources
            .get(&request.uri.to_string())
            .ok_or_else(|| ServerError::NotFound(request.uri.to_string()))?;

        let content = match resource.mime_type.as_deref() {
            Some(mime_type) if mime_type.starts_with("text/") => {
                ResourceContent::Text {
                    uri: resource.uri.clone(),
                    mime_type: mime_type.to_string(),
                    text: "Sample text content".to_string(), // In a real implementation, you would load the actual content
                }
            }
            _ => ResourceContent::Binary {
                uri: resource.uri.clone(),
                mime_type: resource.mime_type.clone().unwrap_or_else(|| "application/octet-stream".to_string()),
                blob: vec![], // In a real implementation, you would load the actual binary content
            },
        };

        Ok(ResourceReadResponse {
            resource: resource.clone(),
            contents: vec![content],
            version: request.version,
        })
    }

    async fn subscribe(&self, request: ResourceSubscribeRequest) -> Result<(), ServerError> {
        let subscription = ResourceSubscription {
            uri: request.uri,
            subscriber_id: request.subscriber_id,
            created_at: Utc::now(),
        };

        self.subscriptions
            .entry(subscription.uri.to_string())
            .or_insert_with(Vec::new)
            .push(subscription);

        Ok(())
    }

    async fn unsubscribe(&self, request: ResourceUnsubscribeRequest) -> Result<(), ServerError> {
        if let Some(mut subscriptions) = self.subscriptions.get_mut(&request.uri.to_string()) {
            subscriptions.retain(|s| s.subscriber_id != request.subscriber_id);
        }

        Ok(())
    }

    async fn get_capabilities(&self) -> Result<ResourceCapabilities, ServerError> {
        Ok(self.capabilities.clone())
    }

    async fn list_templates(&self) -> Result<Vec<ResourceTemplate>, ServerError> {
        Ok(self.templates.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_list_resources() {
        let service = InMemoryResourceService::new();
        let request = ResourceListRequest {
            cursor: None,
            limit: Some(10),
            filter: None,
        };

        let result = service.list_resources(request).await.unwrap();
        assert!(result.resources.is_empty());
        assert!(result.next_cursor.is_none());
    }

    #[tokio::test]
    async fn test_get_capabilities() {
        let service = InMemoryResourceService::new();
        let capabilities = service.get_capabilities().await.unwrap();
        assert!(capabilities.subscribe);
        assert!(capabilities.list_changed);
    }
} 