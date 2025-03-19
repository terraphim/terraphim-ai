use serde::{Deserialize, Serialize};
use chrono::DateTime;
use thiserror::Error;
use url::Url;
use base64::Engine;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Resource {
    pub uri: String,
    pub name: String,
    pub description: Option<String>,
    pub metadata: ResourceMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceMetadata {
    pub mime_type: Option<String>,
    pub scheme: String,
    pub version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceCapabilities {
    pub supports_subscriptions: bool,
    pub supports_templates: bool,
    pub supports_content_types: Vec<String>,
    pub max_subscriptions: Option<u32>,
    pub max_subscription_duration: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceTemplate {
    pub fields: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceFilter {
    pub mime_type: Option<String>,
    pub scheme: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceList {
    pub resources: Vec<Resource>,
    pub total: u32,
    pub cursor: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceListRequest {
    pub filter: Option<ResourceFilter>,
    pub cursor: Option<String>,
    pub limit: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceReadRequest {
    pub uri: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceReadResponse {
    pub content: ResourceContent,
}

#[derive(Debug, Clone)]
pub enum ResourceContent {
    Text(String),
    Binary(Vec<u8>),
}

#[derive(Debug)]
pub enum McpError {
    Internal(String),
    NotFound(String),
    BadRequest(String),
}

impl McpError {
    pub fn internal(message: impl Into<String>) -> Self {
        McpError::Internal(message.into())
    }

    pub fn not_found(message: impl Into<String>) -> Self {
        McpError::NotFound(message.into())
    }

    pub fn bad_request(message: impl Into<String>) -> Self {
        McpError::BadRequest(message.into())
    }
}

impl std::fmt::Display for McpError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            McpError::Internal(msg) => write!(f, "Internal error: {}", msg),
            McpError::NotFound(msg) => write!(f, "Not found: {}", msg),
            McpError::BadRequest(msg) => write!(f, "Bad request: {}", msg),
        }
    }
}

impl std::error::Error for McpError {}

impl serde::Serialize for ResourceContent {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        match self {
            ResourceContent::Text(text) => {
                let mut state = serializer.serialize_struct("ResourceContent", 2)?;
                state.serialize_field("type", "text")?;
                state.serialize_field("text", text)?;
                state.end()
            }
            ResourceContent::Binary(data) => {
                let mut state = serializer.serialize_struct("ResourceContent", 2)?;
                state.serialize_field("type", "binary")?;
                state.serialize_field("data", &base64::engine::general_purpose::STANDARD.encode(data))?;
                state.end()
            }
        }
    }
}

impl<'de> serde::Deserialize<'de> for ResourceContent {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::{self, MapAccess, Visitor};
        use std::fmt;

        #[derive(serde::Deserialize)]
        #[serde(rename_all = "lowercase")]
        enum ContentType {
            Text,
            Binary,
        }

        struct ResourceContentVisitor;

        impl<'de> Visitor<'de> for ResourceContentVisitor {
            type Value = ResourceContent;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a ResourceContent object")
            }

            fn visit_map<V>(self, mut map: V) -> Result<ResourceContent, V::Error>
            where
                V: MapAccess<'de>,
            {
                let content_type: ContentType = map.next_value()?;
                match content_type {
                    ContentType::Text => {
                        let text: String = map.next_value()?;
                        Ok(ResourceContent::Text(text))
                    }
                    ContentType::Binary => {
                        let encoded: String = map.next_value()?;
                        let data = base64::engine::general_purpose::STANDARD
                            .decode(encoded)
                            .map_err(de::Error::custom)?;
                        Ok(ResourceContent::Binary(data))
                    }
                }
            }
        }

        const FIELDS: &[&str] = &["type", "text", "data"];
        deserializer.deserialize_struct("ResourceContent", FIELDS, ResourceContentVisitor)
    }
}

/// Represents a unique identifier for a resource
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceUri {
    pub scheme: String,
    pub path: String,
}

impl ResourceUri {
    pub fn new(scheme: &str, path: &str) -> Result<Self, McpError> {
        if !["https", "file", "git", "terraphim"].contains(&scheme) {
            return Err(McpError::internal(format!("Unsupported scheme: {}", scheme)));
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceError {
    pub code: u16,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubscriptionRequest {
    pub resource_uri: String,
    pub callback_url: String,
    pub duration: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubscriptionResponse {
    pub subscription_id: String,
    pub expiration: Option<String>,
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
    fn test_resource_uri_validation() {
        let result = ResourceUri::new("invalid", "path");
        assert!(matches!(result, Err(McpError::Internal(_))));
        
        let result = ResourceUri::new("https", "path");
        assert!(matches!(result, Ok(_)));
        
        let result = ResourceUri::new("file", "path");
        assert!(matches!(result, Ok(_)));
        
        let result = ResourceUri::new("git", "path");
        assert!(matches!(result, Ok(_)));
        
        let result = ResourceUri::new("terraphim", "path");
        assert!(matches!(result, Ok(_)));
    }

    #[test]
    fn test_resource_uri_from_url() {
        let url = Url::parse("https://example.com/path").unwrap();
        let uri = ResourceUri::from_url(&url).unwrap();
        assert_eq!(uri.scheme, "https");
        assert_eq!(uri.path, "/path");
    }

    #[test]
    fn test_resource_uri_to_string() {
        let uri = ResourceUri {
            scheme: "https".to_string(),
            path: "/path".to_string(),
        };
        assert_eq!(uri.to_string(), "https:///path");
    }
} 