use salvo::prelude::*;
use salvo::oapi::{self, endpoint};
use serde_json::json;
use std::sync::Arc;

use crate::schema::*;
use crate::service::ResourceService;
use crate::error::ServerError;

#[derive(Clone)]
pub struct ResourceHandlers {
    pub service: Arc<dyn ResourceService>,
}

impl ResourceHandlers {
    pub fn new(service: Arc<dyn ResourceService>) -> Self {
        Self { service }
    }
}

/// List available resources
/// 
/// Returns a paginated list of available resources
#[handler]
pub async fn list_resources(
    req: &mut Request,
    depot: &mut Depot,
    res: &mut Response,
) {
    let handlers = depot.obtain::<ResourceHandlers>().unwrap();
    match req.parse_json::<ResourceListRequest>().await {
        Ok(request) => {
            match handlers.service.list_resources(request).await {
                Ok(result) => {
                    res.render(Json(result));
                }
                Err(e) => {
                    res.status_code(e.status_code());
                    res.render(Json(json!({
                        "error": e.to_string()
                    })));
                }
            }
        }
        Err(e) => {
            res.status_code(StatusCode::BAD_REQUEST);
            res.render(Json(json!({
                "error": format!("Invalid request: {}", e)
            })));
        }
    }
}

/// Read resource contents
/// 
/// Returns the contents of a specific resource
#[handler]
pub async fn read_resource(
    req: &mut Request,
    depot: &mut Depot,
    res: &mut Response,
) {
    let handlers = depot.obtain::<ResourceHandlers>().unwrap();
    match req.parse_json::<ResourceReadRequest>().await {
        Ok(request) => {
            match handlers.service.read_resource(request).await {
                Ok(result) => {
                    res.render(Json(result));
                }
                Err(e) => {
                    res.status_code(e.status_code());
                    res.render(Json(json!({
                        "error": e.to_string()
                    })));
                }
            }
        }
        Err(e) => {
            res.status_code(StatusCode::BAD_REQUEST);
            res.render(Json(json!({
                "error": format!("Invalid request: {}", e)
            })));
        }
    }
}

/// Subscribe to resource changes
/// 
/// Subscribe to notifications for changes to a specific resource
#[handler]
pub async fn subscribe(
    req: &mut Request,
    depot: &mut Depot,
    res: &mut Response,
) {
    let handlers = depot.obtain::<ResourceHandlers>().unwrap();
    match req.parse_json::<ResourceSubscribeRequest>().await {
        Ok(request) => {
            match handlers.service.subscribe(request).await {
                Ok(_) => {
                    res.render(Json(json!({
                        "status": "success",
                        "message": "Successfully subscribed to resource"
                    })));
                }
                Err(e) => {
                    res.status_code(e.status_code());
                    res.render(Json(json!({
                        "error": e.to_string()
                    })));
                }
            }
        }
        Err(e) => {
            res.status_code(StatusCode::BAD_REQUEST);
            res.render(Json(json!({
                "error": format!("Invalid request: {}", e)
            })));
        }
    }
}

/// Unsubscribe from resource changes
/// 
/// Unsubscribe from notifications for a specific resource
#[handler]
pub async fn unsubscribe(
    req: &mut Request,
    depot: &mut Depot,
    res: &mut Response,
) {
    let handlers = depot.obtain::<ResourceHandlers>().unwrap();
    match req.parse_json::<ResourceUnsubscribeRequest>().await {
        Ok(request) => {
            match handlers.service.unsubscribe(request).await {
                Ok(_) => {
                    res.render(Json(json!({
                        "status": "success",
                        "message": "Successfully unsubscribed from resource"
                    })));
                }
                Err(e) => {
                    res.status_code(e.status_code());
                    res.render(Json(json!({
                        "error": e.to_string()
                    })));
                }
            }
        }
        Err(e) => {
            res.status_code(StatusCode::BAD_REQUEST);
            res.render(Json(json!({
                "error": format!("Invalid request: {}", e)
            })));
        }
    }
}

/// Get server capabilities
/// 
/// Returns the server's capabilities
#[handler]
pub async fn get_capabilities(
    _req: &mut Request,
    depot: &mut Depot,
    res: &mut Response,
) {
    let handlers = depot.obtain::<ResourceHandlers>().unwrap();
    match handlers.service.get_capabilities().await {
        Ok(capabilities) => {
            res.render(Json(json!({
                "capabilities": capabilities
            })));
        }
        Err(e) => {
            res.status_code(e.status_code());
            res.render(Json(json!({
                "error": e.to_string()
            })));
        }
    }
}

/// List available resource templates
/// 
/// Returns a list of available resource templates
#[handler]
pub async fn list_templates(
    _req: &mut Request,
    depot: &mut Depot,
    res: &mut Response,
) {
    let handlers = depot.obtain::<ResourceHandlers>().unwrap();
    match handlers.service.list_templates().await {
        Ok(templates) => {
            res.render(Json(json!({
                "templates": templates
            })));
        }
        Err(e) => {
            res.status_code(e.status_code());
            res.render(Json(json!({
                "error": e.to_string()
            })));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::service::InMemoryResourceService;
    use salvo::test::{TestClient};
    use salvo::prelude::*;

    #[tokio::test]
    async fn test_get_capabilities() {
        // Create the service implementation
        let service = Arc::new(InMemoryResourceService::new());
        let handlers = ResourceHandlers::new(service);
        
        // Create a router with the handler
        let router = Router::new()
            .hoop(HandlersMiddleware { handlers: handlers.clone() })
            .get(get_capabilities);
        
        // Create a test service with the router
        let test_service = Service::new(router);
        
        // Send the request through the test client
        let resp = TestClient::get("http://127.0.0.1/")
            .send(&test_service)
            .await;
            
        // Check that the status code is OK
        assert_eq!(resp.status_code.unwrap(), StatusCode::OK);
    }
    
    // Middleware for injecting handlers in tests
    struct HandlersMiddleware {
        handlers: ResourceHandlers,
    }
    
    #[async_trait]
    impl Handler for HandlersMiddleware {
        async fn handle(&self, req: &mut Request, depot: &mut Depot, res: &mut Response, ctrl: &mut FlowCtrl) {
            depot.inject(self.handlers.clone());
            ctrl.call_next(req, depot, res).await;
        }
    }
} 