use std::sync::Arc;
use salvo::prelude::*;
use salvo::test::TestClient;
use salvo::http::StatusCode;
use serde_json::json;
use futures;

use crate::service::InMemoryResourceService;
use crate::handlers::resources::{
    ResourceHandlers,
    list_resources,
    read_resource,
    subscribe,
    unsubscribe,
    get_capabilities,
    list_templates
};

// Middleware for injecting handlers
struct InjectHandlersMiddleware {
    handlers: ResourceHandlers,
}

#[async_trait]
impl Handler for InjectHandlersMiddleware {
    async fn handle(&self, req: &mut Request, depot: &mut Depot, res: &mut Response, ctrl: &mut FlowCtrl) {
        depot.inject(self.handlers.clone());
        ctrl.call_next(req, depot, res).await;
    }
}

async fn setup_test_server() -> Service {
    // Create service and handlers
    let service = Arc::new(InMemoryResourceService::new());
    let handlers = ResourceHandlers::new(service);
    
    // Create middleware
    let inject_middleware = InjectHandlersMiddleware { handlers };
    
    // Create router with middleware
    let router = Router::new()
        .hoop(inject_middleware);
    
    // Add v1 API routes
    let v1_router = Router::with_path("v1")
        .push(
            Router::with_path("resources")
                .get(list_resources)
                .post(read_resource)
                .push(
                    Router::with_path("subscribe")
                        .post(subscribe)
                        .delete(unsubscribe),
                )
                .push(
                    Router::with_path("templates")
                        .get(list_templates),
                ),
        )
        .push(Router::with_path("capabilities").get(get_capabilities));

    // Build the router
    let router = router.push(v1_router);
    
    Service::new(router)
}

#[tokio::test]
async fn test_list_resources() {
    let service = setup_test_server().await;
    
    let resp = TestClient::get("http://127.0.0.1/v1/resources")
        .send(&service)
        .await;
    
    assert_eq!(resp.status_code, Some(StatusCode::OK));
}

#[tokio::test]
async fn test_read_resource() {
    let service = setup_test_server().await;
    
    let resp = TestClient::post("http://127.0.0.1/v1/resources")
        .json(&json!({
            "uri": {
                "scheme": "file",
                "path": "/test/resource.txt"
            },
            "version": null
        }))
        .send(&service)
        .await;
    
    assert_eq!(resp.status_code, Some(StatusCode::OK));
}

#[tokio::test]
async fn test_subscribe() {
    let service = setup_test_server().await;
    
    let resp = TestClient::post("http://127.0.0.1/v1/resources/subscribe")
        .json(&json!({
            "uri": {
                "scheme": "file",
                "path": "/test/resource.txt"
            },
            "subscriber_id": "test_subscriber"
        }))
        .send(&service)
        .await;
    
    assert_eq!(resp.status_code, Some(StatusCode::OK));
}

#[tokio::test]
async fn test_unsubscribe() {
    let service = setup_test_server().await;
    
    let resp = TestClient::delete("http://127.0.0.1/v1/resources/subscribe")
        .json(&json!({
            "uri": {
                "scheme": "file",
                "path": "/test/resource.txt"
            },
            "subscriber_id": "test_subscriber"
        }))
        .send(&service)
        .await;
    
    assert_eq!(resp.status_code, Some(StatusCode::OK));
}

#[tokio::test]
async fn test_list_templates() {
    let service = setup_test_server().await;
    
    let resp = TestClient::get("http://127.0.0.1/v1/resources/templates")
        .send(&service)
        .await;
    
    assert_eq!(resp.status_code, Some(StatusCode::OK));
}

#[tokio::test]
async fn test_get_capabilities() {
    let service = setup_test_server().await;
    
    let resp = TestClient::get("http://127.0.0.1/v1/capabilities")
        .send(&service)
        .await;
    
    assert_eq!(resp.status_code, Some(StatusCode::OK));
}

#[tokio::test]
async fn test_error_responses() {
    let service = setup_test_server().await;
    
    // Test invalid resource read
    let resp = TestClient::post("http://127.0.0.1/v1/resources")
        .json(&json!({
            "uri": {
                "scheme": "invalid",
                "path": "/nonexistent"
            },
            "version": null
        }))
        .send(&service)
        .await;
    
    assert_eq!(resp.status_code, Some(StatusCode::NOT_FOUND));
}

#[tokio::test]
async fn test_pagination() {
    let service = setup_test_server().await;
    
    let resp = TestClient::get("http://127.0.0.1/v1/resources?limit=10&offset=0")
        .send(&service)
        .await;
    
    assert_eq!(resp.status_code, Some(StatusCode::OK));
}

#[tokio::test]
async fn test_resource_filtering() {
    let service = setup_test_server().await;
    
    let resp = TestClient::get("http://127.0.0.1/v1/resources?scheme=file")
        .send(&service)
        .await;
    
    assert_eq!(resp.status_code, Some(StatusCode::OK));
}

#[tokio::test]
async fn test_concurrent_requests() {
    let service = setup_test_server().await;
    
    let future1 = async {
        TestClient::get("http://127.0.0.1/v1/resources")
            .send(&service)
            .await
    };
    
    let future2 = async {
        TestClient::get("http://127.0.0.1/v1/capabilities")
            .send(&service)
            .await
    };
    
    let (resp1, resp2) = futures::join!(future1, future2);
    
    assert_eq!(resp1.status_code, Some(StatusCode::OK));
    assert_eq!(resp2.status_code, Some(StatusCode::OK));
}