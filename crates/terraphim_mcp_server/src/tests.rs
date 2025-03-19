use std::sync::Arc;
use salvo::prelude::*;
use salvo::test::TestClient;
use salvo::http::StatusCode;
use serde_json::json;
use futures;
use std::path::PathBuf;
use tokio::net::TcpListener;
use terraphim_config::{ConfigBuilder, Role, ServiceType, Haystack};
use terraphim_types::{RelevanceFunction, SearchQuery, NormalizedTermValue};
use crate::service::TerraphimResourceService;
use crate::handlers::resources::ResourceHandlers;
use crate::schema::{Resource, ResourceUri, ResourceList, ResourceCapabilities, ResourceMetadata, McpError};

use salvo::conn::TcpAcceptor;
use reqwest::Client;

pub struct TestServer {
    pub addr: String,
    pub service: Arc<TerraphimResourceService>,
}

impl TestServer {
    pub async fn new() -> Self {
        let mut config = ConfigBuilder::new()
            .add_role(
                "Default",
                Role {
                    shortname: Some("Default".to_string()),
                    name: "Default".into(),
                    relevance_function: RelevanceFunction::TitleScorer,
                    theme: "spacelab".to_string(),
                    kg: None,
                    haystacks: vec![Haystack {
                        path: PathBuf::from("tests/test_data/sample_docs"),
                        service: ServiceType::Ripgrep,
                    }],
                    extra: Default::default(),
                },
            )
            .default_role("Default")
            .unwrap()
            .build()
            .unwrap();

        let service = Arc::new(TerraphimResourceService::new(config).await.unwrap());
        
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap().to_string();
        
        let handlers = ResourceHandlers::new(service.clone());
        let router = handlers.router();
        
        let acceptor = TcpAcceptor::new(listener);
        let server = Server::new(acceptor);
        
        tokio::spawn(async move {
            server.serve(router).await;
        });
        
        Self { addr, service }
    }
}

#[tokio::test]
async fn test_list_resources() {
    let server = TestServer::new().await;
    let client = Client::new();
    
    let response = client
        .get(format!("http://{}/v1/resources", server.addr))
        .send()
        .await
        .unwrap();
    
    assert_eq!(response.status(), 200);
    let resources: ResourceList = response.json().await.unwrap();
    assert!(!resources.resources.is_empty());
    assert_eq!(resources.resources.len(), 2);
}

#[tokio::test]
async fn test_read_resource() {
    let server = TestServer::new().await;
    let client = Client::new();
    
    // First get the list of resources
    let response = client
        .get(format!("http://{}/v1/resources", server.addr))
        .send()
        .await
        .unwrap();
    
    let resources: ResourceList = response.json().await.unwrap();
    let first_uri = &resources.resources[0].uri;
    
    // Then read the specific resource
    let response = client
        .get(format!("http://{}/v1/resources/{}", server.addr, first_uri))
        .send()
        .await
        .unwrap();
    
    assert_eq!(response.status(), 200);
    let resource: Resource = response.json().await.unwrap();
    assert_eq!(resource.uri, *first_uri);
}

#[tokio::test]
async fn test_get_capabilities() {
    let server = TestServer::new().await;
    let client = Client::new();
    
    let response = client
        .get(format!("http://{}/v1/capabilities", server.addr))
        .send()
        .await
        .unwrap();
    
    assert_eq!(response.status(), 200);
    let capabilities: ResourceCapabilities = response.json().await.unwrap();
    assert!(capabilities.supports_subscriptions);
    assert!(capabilities.supports_templates);
    assert!(capabilities.supports_content_types.contains(&"text/markdown".to_string()));
}

#[tokio::test]
async fn test_list_templates() {
    let server = TestServer::new().await;
    let client = Client::new();
    
    let response = client
        .get(format!("http://{}/v1/templates", server.addr))
        .send()
        .await
        .unwrap();
    
    assert_eq!(response.status(), 200);
    let templates = response.json::<Vec<serde_json::Value>>().await.unwrap();
    assert!(!templates.is_empty());
    assert!(templates[0]["fields"].as_array().unwrap().contains(&json!("uri")));
}

#[tokio::test]
async fn test_error_responses() {
    let server = TestServer::new().await;
    let client = Client::new();
    
    // Test invalid resource read
    let response = client
        .get(format!("http://{}/v1/resources/nonexistent.md", server.addr))
        .send()
        .await
        .unwrap();
    
    assert_eq!(response.status(), 404);
}

#[tokio::test]
async fn test_pagination() {
    let server = TestServer::new().await;
    let client = Client::new();
    
    let response = client
        .get(format!("http://{}/v1/resources?limit=1", server.addr))
        .send()
        .await
        .unwrap();
    
    assert_eq!(response.status(), 200);
    let resources: ResourceList = response.json().await.unwrap();
    assert_eq!(resources.resources.len(), 1);
}

#[tokio::test]
async fn test_resource_filtering() {
    let server = TestServer::new().await;
    let client = Client::new();
    
    let response = client
        .get(format!("http://{}/v1/resources?mime_type=text/markdown", server.addr))
        .send()
        .await
        .unwrap();
    
    assert_eq!(response.status(), 200);
    let resources: ResourceList = response.json().await.unwrap();
    assert!(resources.resources.iter().all(|r| r.metadata.mime_type.as_deref() == Some("text/markdown")));
}

#[tokio::test]
async fn test_concurrent_requests() {
    let server = TestServer::new().await;
    
    let future1 = async {
        reqwest::get(format!("http://{}/v1/resources", server.addr))
            .await
            .unwrap()
            .status()
    };
    
    let future2 = async {
        reqwest::get(format!("http://{}/v1/capabilities", server.addr))
            .await
            .unwrap()
            .status()
    };
    
    let (status1, status2) = futures::join!(future1, future2);
    
    assert_eq!(status1, 200);
    assert_eq!(status2, 200);
}

#[tokio::test]
async fn test_mcp_haystack_integration() {
    let server = TestServer::new().await;
    let client = Client::new();
    
    // Test 1: List resources
    let response = client
        .get(format!("http://{}/v1/resources", server.addr))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    let resources: ResourceList = response.json().await.unwrap();
    assert!(!resources.resources.is_empty());
    assert_eq!(resources.resources.len(), 2);

    // Test 2: Read specific resource
    let response = client
        .get(format!("http://{}/v1/resources/test1.md", server.addr))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    let resource: Resource = response.json().await.unwrap();
    assert_eq!(resource.uri, "test1.md");

    // Test 3: Filter resources by MIME type
    let response = client
        .get(format!("http://{}/v1/resources?mime_type=text/markdown", server.addr))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    let resources: ResourceList = response.json().await.unwrap();
    assert!(resources.resources.iter().all(|r| r.metadata.mime_type.as_deref() == Some("text/markdown")));

    // Test 4: Pagination
    let response = client
        .get(format!("http://{}/v1/resources?limit=1", server.addr))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    let resources: ResourceList = response.json().await.unwrap();
    assert_eq!(resources.resources.len(), 1);

    // Test 5: Get capabilities
    let response = client
        .get(format!("http://{}/v1/capabilities", server.addr))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    let capabilities: ResourceCapabilities = response.json().await.unwrap();
    assert!(capabilities.supports_subscriptions);
    assert!(capabilities.supports_templates);
    assert!(capabilities.supports_content_types.contains(&"text/markdown".to_string()));

    // Test 6: List templates
    let response = client
        .get(format!("http://{}/v1/templates", server.addr))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    let templates = response.json::<Vec<serde_json::Value>>().await.unwrap();
    assert!(!templates.is_empty());
    assert!(templates[0]["fields"].as_array().unwrap().contains(&json!("uri")));

    // Test 7: Create resource (should fail as it's read-only)
    let response = client
        .post(format!("http://{}/v1/resources", server.addr))
        .json(&json!({
            "uri": "new.md",
            "name": "New Resource",
            "description": "Test resource",
            "metadata": {
                "mime_type": "text/markdown",
                "scheme": "file"
            }
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 400);
}