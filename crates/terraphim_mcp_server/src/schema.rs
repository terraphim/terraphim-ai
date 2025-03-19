use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;
use url::Url;
use base64;

#[derive(Debug, Error)]
pub enum McpError {
    #[error("Invalid URI: {0}")]
    InvalidUri(String),
    #[error("Resource not found: {0}")]
    NotFound(String),
    #[error("Internal error: {0}")]
    Internal(String),
    #[error("Access denied: {0}")]
    AccessDenied(String),
    #[error("Invalid MIME type: {0}")]
    InvalidMimeType(String),
    #[error("Serialization error: {0}")]
    SerializationError(String),
}

/// Represents a unique identifier for a resource
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceUri {
    pub scheme: String,
    pub path: String,
}

impl ResourceUri {
    pub fn new(scheme: &str, path: &str) -> Result<Self, McpError> {
        if !["https", "file", "git"].contains(&scheme) {
            return Err(McpError::InvalidUri(format!("Unsupported scheme: {}", scheme)));
        }

        Ok(Self {
            scheme: scheme.to_string(),
            path: path.to_string(),
        })
    }

    pub fn from_url(url: &Url) -> Result<Self, McpError> {
        Ok(Self {
            scheme: url.scheme().to_string(),
            path: url.path().to_string(),
        })
    }

    pub fn to_string(&self) -> String {
        format!("{}://{}", self.scheme, self.path)
    }
}

/// Represents a resource in the MCP system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Resource {
    pub uri: ResourceUri,
    pub content: String,
    pub metadata: ResourceMetadata,
    pub mime_type: Option<String>,
    pub name: String,
}

/// Represents the content of a resource
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ResourceContent {
    Text {
        uri: ResourceUri,
        mime_type: String,
        text: String,
    },
    Binary {
        uri: ResourceUri,
        mime_type: String,
        #[serde(serialize_with = "serialize_base64", deserialize_with = "deserialize_base64")]
        blob: Vec<u8>,
    },
}

fn serialize_base64<S>(bytes: &[u8], serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    let encoded = base64::encode(bytes);
    serializer.serialize_str(&encoded)
}

fn deserialize_base64<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let encoded = String::deserialize(deserializer)?;
    base64::decode(encoded).map_err(serde::de::Error::custom)
}

/// Represents server capabilities for resources
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceCapabilities {
    pub subscribe: bool,
    pub list_changed: bool,
}

/// Represents a resource template
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceTemplate {
    pub name: String,
    pub description: String,
    pub schema: String,
    pub uri_template: String,
    pub mime_type: Option<String>,
}

/// Represents a list of resources with pagination
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceList {
    pub resources: Vec<Resource>,
    pub next_cursor: Option<String>,
}

/// Represents a resource subscription
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceSubscription {
    pub uri: ResourceUri,
    pub subscriber_id: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Represents a resource change notification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceChangeNotification {
    pub uri: ResourceUri,
    pub change_type: ChangeType,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChangeType {
    Created,
    Updated,
    Deleted,
}

/// Represents a resource read request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceReadRequest {
    pub uri: ResourceUri,
    pub version: Option<String>,
}

/// Represents a resource read response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceReadResponse {
    pub resource: Resource,
    pub contents: Vec<ResourceContent>,
    pub version: Option<String>,
}

/// Represents a resource list request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceListRequest {
    pub filter: Option<ResourceFilter>,
    pub cursor: Option<String>,
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceFilter {
    pub scheme: Option<String>,
    pub tags: Option<Vec<String>>,
    pub mime_type: Option<String>,
    pub name_pattern: Option<String>,
}

/// Represents a resource subscription request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceSubscribeRequest {
    pub uri: ResourceUri,
    pub subscriber_id: String,
}

/// Represents a resource unsubscribe request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceUnsubscribeRequest {
    pub uri: ResourceUri,
    pub subscriber_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceMetadata {
    pub title: String,
    pub description: Option<String>,
    pub tags: Vec<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerCapabilities {
    pub supported_schemes: Vec<String>,
    pub max_request_size: usize,
    pub max_response_size: usize,
    pub max_subscriptions: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceListResponse {
    pub resources: Vec<Resource>,
    pub next_cursor: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resource_uri() {
        let uri = ResourceUri::new("file", "/path/to/resource").unwrap();
        assert_eq!(uri.scheme, "file");
        assert_eq!(uri.path, "/path/to/resource");
        assert_eq!(uri.to_string(), "file:///path/to/resource");
    }

    #[test]
    fn test_resource_uri_invalid_scheme() {
        let result = ResourceUri::new("invalid", "/path");
        assert!(matches!(result, Err(McpError::InvalidUri(_))));
    }
} 