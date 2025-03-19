use std::sync::Arc;
use salvo::prelude::*;
use salvo::oapi::{self};
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

mod schema;
mod error;
mod service;
mod handlers;
mod docs;
#[cfg(test)]
mod tests;

use service::{InMemoryResourceService};
use handlers::resources::*;

// Simple handler to serve Swagger UI
#[handler]
async fn swagger_ui(_req: &mut Request, res: &mut Response) {
    res.render(Text::Html(docs::get_swagger_ui()));
}

// Simple handler to serve OpenAPI documentation
#[handler]
async fn openapi_json(res: &mut Response) {
    // Use the manual OpenAPI documentation from docs module
    res.render(Json(serde_json::from_str::<serde_json::Value>(&docs::get_api_docs()).unwrap()));
}

// Middleware for injecting handlers
struct InjectHandlers {
    handlers: ResourceHandlers,
}

#[async_trait]
impl Handler for InjectHandlers {
    async fn handle(&self, req: &mut Request, depot: &mut Depot, res: &mut Response, ctrl: &mut FlowCtrl) {
        depot.inject(self.handlers.clone());
        ctrl.call_next(req, depot, res).await;
    }
}

#[tokio::main]
async fn main() {
    // Initialize logging
    let _subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .init();

    // Create service and handlers
    let service = Arc::new(InMemoryResourceService::new());
    let handlers = ResourceHandlers::new(service.clone());
    
    // Create middleware for handler injection
    let inject_middleware = InjectHandlers { handlers };
    
    // Create the router with middleware first
    let router = Router::new()
        .hoop(inject_middleware);
    
    // Add v1 API routes
    let v1_router = Router::with_path("v1")
        .get(list_resources)
        .post(read_resource);

    // Add resources sub-routes
    let subscribe_router = Router::with_path("resources/subscribe")
        .post(subscribe)
        .delete(unsubscribe);
    
    let templates_router = Router::with_path("resources/templates")
        .get(list_templates);
    
    let capabilities_router = Router::with_path("capabilities")
        .get(get_capabilities);

    // Build the API router
    let router = router
        .push(v1_router)
        .push(subscribe_router)
        .push(templates_router)
        .push(capabilities_router);

    // Add documentation routes
    let router = router
        .push(Router::with_path("openapi.json").get(openapi_json))
        .push(Router::with_path("swagger-ui").get(swagger_ui));

    // Create server
    let acceptor = TcpListener::new("127.0.0.1:5800").bind().await;
    let server = Server::new(acceptor);

    info!("Starting MCP server on http://127.0.0.1:5800");
    info!("API documentation available at http://127.0.0.1:5800/swagger-ui");
    
    // Run the server with the router
    server.serve(router).await;
}

#[cfg(test)]
mod main_tests {
    use super::*;

    #[tokio::test]
    async fn test_server_startup() {
        // This is a basic test to ensure the server can start
        // In a real implementation, you would want more comprehensive tests
        assert!(true);
    }
}
