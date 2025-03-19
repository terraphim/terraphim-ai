use salvo::prelude::*;
use std::sync::Arc;
use crate::service::TerraphimResourceService;
use crate::schema::{
    Resource, ResourceListRequest, ResourceReadRequest,
    McpError
};
use salvo::http::StatusCode;
use crate::docs::{get_api_docs, get_swagger_ui};

pub struct ResourceHandlers {
    service: Arc<TerraphimResourceService>,
}

impl ResourceHandlers {
    pub fn new(service: Arc<TerraphimResourceService>) -> Self {
        Self { service }
    }

    pub fn router(&self) -> Router {
        Router::new()
            .push(
                Router::with_path("v1")
                    .push(
                        Router::with_path("resources")
                            .get(list_resources)
                            .post(create_resource)
                            .push(
                                Router::with_path("<uri>")
                                    .get(read_resource)
                            )
                    )
                    .push(
                        Router::with_path("capabilities")
                            .get(get_capabilities)
                    )
                    .push(
                        Router::with_path("templates")
                            .get(list_templates)
                    )
            )
            .push(
                Router::with_path("openapi.json")
                    .get(get_openapi_docs)
            )
            .push(
                Router::with_path("docs")
                    .get(get_docs_ui)
            )
            .hoop(InjectHandlersMiddleware::new(self.service.clone()))
    }
}

struct InjectHandlersMiddleware {
    service: Arc<TerraphimResourceService>,
}

impl InjectHandlersMiddleware {
    fn new(service: Arc<TerraphimResourceService>) -> Self {
        Self { service }
    }
}

#[async_trait]
impl Handler for InjectHandlersMiddleware {
    async fn handle(&self, req: &mut Request, depot: &mut Depot, res: &mut Response, ctrl: &mut FlowCtrl) {
        depot.inject(self.service.clone());
        ctrl.call_next(req, depot, res).await;
    }
}

#[handler]
async fn list_resources(_req: &mut Request, depot: &mut Depot, res: &mut Response) -> Result<(), StatusError> {
    let service = depot.obtain::<Arc<TerraphimResourceService>>()
        .map_err(|_| StatusError::internal_server_error())?;

    let request = ResourceListRequest {
        filter: None,
        cursor: None,
        limit: Some(10),
    };

    let result = service.list_resources(request).await
        .map_err(|e| match e {
            McpError::Internal(_) => StatusError::internal_server_error(),
            McpError::NotFound(_) => StatusError::not_found(),
            McpError::BadRequest(_) => StatusError::bad_request(),
        })?;

    res.render(Json(result));
    Ok(())
}

#[handler]
async fn read_resource(req: &mut Request, depot: &mut Depot, res: &mut Response) -> Result<(), StatusError> {
    let service = depot.obtain::<Arc<TerraphimResourceService>>()
        .map_err(|_| StatusError::internal_server_error())?;

    let uri = req.param::<String>("uri")
        .ok_or_else(|| StatusError::bad_request())?;

    let request = ResourceReadRequest { uri };
    let result = service.read_resource(request).await
        .map_err(|e| match e {
            McpError::NotFound(_) => StatusError::not_found(),
            McpError::Internal(_) => StatusError::internal_server_error(),
            McpError::BadRequest(_) => StatusError::bad_request(),
        })?;

    res.render(Json(result));
    Ok(())
}

#[handler]
async fn create_resource(req: &mut Request, depot: &mut Depot, res: &mut Response) -> Result<(), StatusError> {
    let service = depot.obtain::<Arc<TerraphimResourceService>>()
        .map_err(|_| StatusError::internal_server_error())?;

    let resource: Resource = req.parse_json().await
        .map_err(|_| StatusError::bad_request())?;

    service.create_resource(resource).await
        .map_err(|e| match e {
            McpError::BadRequest(_) => StatusError::bad_request(),
            McpError::Internal(_) => StatusError::internal_server_error(),
            McpError::NotFound(_) => StatusError::not_found(),
        })?;

    res.status_code(StatusCode::CREATED);
    Ok(())
}

#[handler]
async fn get_capabilities(_req: &mut Request, depot: &mut Depot, res: &mut Response) -> Result<(), StatusError> {
    let service = depot.obtain::<Arc<TerraphimResourceService>>()
        .map_err(|_| StatusError::internal_server_error())?;

    let result = service.get_capabilities().await
        .map_err(|e| match e {
            McpError::Internal(_) => StatusError::internal_server_error(),
            McpError::NotFound(_) => StatusError::not_found(),
            McpError::BadRequest(_) => StatusError::bad_request(),
        })?;

    res.render(Json(result));
    Ok(())
}

#[handler]
async fn list_templates(_req: &mut Request, depot: &mut Depot, res: &mut Response) -> Result<(), StatusError> {
    let service = depot.obtain::<Arc<TerraphimResourceService>>()
        .map_err(|_| StatusError::internal_server_error())?;

    let result = service.list_templates().await
        .map_err(|e| match e {
            McpError::Internal(_) => StatusError::internal_server_error(),
            McpError::NotFound(_) => StatusError::not_found(),
            McpError::BadRequest(_) => StatusError::bad_request(),
        })?;

    res.render(Json(result));
    Ok(())
}

#[handler]
async fn get_openapi_docs(_req: &mut Request, _depot: &mut Depot, res: &mut Response) -> Result<(), StatusError> {
    res.render(Text::Plain(get_api_docs()));
    Ok(())
}

#[handler]
async fn get_docs_ui(_req: &mut Request, _depot: &mut Depot, res: &mut Response) -> Result<(), StatusError> {
    res.render(Text::Html(get_swagger_ui()));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use salvo::conn::tcp::TcpAcceptor;
    use tokio::net::TcpListener;
    use reqwest::StatusCode as ReqwestStatusCode;
    use tempfile::tempdir;
    use std::fs;
    use terraphim_config::{ConfigBuilder, Role, ServiceType, Haystack};
    use terraphim_types::RelevanceFunction;
    use tokio::time::{sleep, Duration};

    async fn setup_test_server() -> String {
        // Create a temporary directory for test data
        let temp_dir = tempdir().unwrap();
        let test_dir = temp_dir.path().join("sample_docs");
        fs::create_dir_all(&test_dir).unwrap();

        // Create test markdown files with unique content
        let test_file1 = test_dir.join("test1.md");
        let test_file2 = test_dir.join("test2.md");
        fs::write(&test_file1, "# Test Document 1\n\nThis is a test document with unique content.").unwrap();
        fs::write(&test_file2, "# Test Document 2\n\nThis is another test document with different content.").unwrap();

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

        let service = Arc::new(TerraphimResourceService::new(config).await.unwrap());
        let handlers = ResourceHandlers::new(service);
        let router = handlers.router();
        
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let acceptor = TcpAcceptor::try_from(listener).unwrap();
        
        let server = Server::new(acceptor);
        tokio::spawn(async move {
            server.serve(router).await;
        });

        // Keep the temp directory alive for the duration of the test
        std::mem::forget(temp_dir);

        // Give the service more time to index the files
        sleep(Duration::from_secs(1)).await;

        format!("http://{}", addr)
    }

    #[tokio::test]
    async fn test_list_resources() {
        let addr = setup_test_server().await;
        let client = reqwest::Client::new();
        let resp = client.get(&format!("{}/v1/resources", addr)).send().await.unwrap();
        assert_eq!(resp.status(), ReqwestStatusCode::OK);
        let resources: serde_json::Value = resp.json().await.unwrap();
        let resources_array = resources["resources"].as_array().unwrap();
        assert!(!resources_array.is_empty(), "Resources array should not be empty");
        assert!(resources_array.len() >= 2, "Should have at least 2 resources");
    }

    #[tokio::test]
    async fn test_read_resource() {
        let addr = setup_test_server().await;
        let client = reqwest::Client::new();

        // First list resources to get the URIs
        let resp = client.get(&format!("{}/v1/resources", addr)).send().await.unwrap();
        assert_eq!(resp.status(), ReqwestStatusCode::OK);
        let resources: serde_json::Value = resp.json().await.unwrap();
        let resources_array = resources["resources"].as_array().unwrap();
        let first_resource = &resources_array[0];
        let uri = first_resource["uri"].as_str().unwrap();

        // Then read the specific resource
        let resp = client.get(&format!("{}/v1/resources/{}", addr, uri)).send().await.unwrap();
        assert_eq!(resp.status(), ReqwestStatusCode::OK);
        let response: serde_json::Value = resp.json().await.unwrap();
        assert!(response["content"]["text"].as_str().unwrap().contains("Test Document"));
    }

    #[tokio::test]
    async fn test_get_capabilities() {
        let addr = setup_test_server().await;
        let client = reqwest::Client::new();
        let resp = client.get(&format!("{}/v1/capabilities", addr)).send().await.unwrap();
        assert_eq!(resp.status(), ReqwestStatusCode::OK);
        let capabilities: serde_json::Value = resp.json().await.unwrap();
        assert!(capabilities["supports_subscriptions"].as_bool().unwrap());
        assert!(capabilities["supports_templates"].as_bool().unwrap());
    }

    #[tokio::test]
    async fn test_list_templates() {
        let addr = setup_test_server().await;
        let client = reqwest::Client::new();
        let resp = client.get(&format!("{}/v1/templates", addr)).send().await.unwrap();
        assert_eq!(resp.status(), ReqwestStatusCode::OK);
        let templates: Vec<serde_json::Value> = resp.json().await.unwrap();
        assert!(!templates.is_empty());
        assert!(templates[0]["fields"].as_array().unwrap().contains(&serde_json::Value::String("uri".to_string())));
    }
} 