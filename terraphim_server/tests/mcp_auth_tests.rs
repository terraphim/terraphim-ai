use axum::http::StatusCode;
use serde_json::json;
use utoipa::OpenApi;

/// RED Phase: Test that unauthenticated requests are rejected
#[tokio::test]
async fn test_unauthenticated_request_returns_401() {
    let app = create_test_server_with_auth().await;
    let client = axum_test::TestServer::new(app).unwrap();

    // Try to create a namespace without authentication
    let response = client
        .post("/metamcp/namespaces")
        .json(&json!({
            "name": "test-namespace",
            "description": "Test",
            "user_id": "test-user",
            "config_json": r#"{"servers":[]}"#,
            "enabled": true,
            "visibility": "Private"
        }))
        .await;

    assert_eq!(
        response.status_code(),
        StatusCode::UNAUTHORIZED,
        "Request without Authorization header should return 401"
    );
}

/// RED Phase: Test that invalid API keys are rejected
#[tokio::test]
async fn test_invalid_api_key_returns_401() {
    let app = create_test_server_with_auth().await;
    let client = axum_test::TestServer::new(app).unwrap();

    // Try with invalid API key
    let response = client
        .post("/metamcp/namespaces")
        .add_header(
            axum::http::HeaderName::from_static("authorization"),
            axum::http::HeaderValue::from_static("Bearer invalid-key-123"),
        )
        .json(&json!({
            "name": "test-namespace",
            "description": "Test",
            "user_id": "test-user",
            "config_json": r#"{"servers":[]}"#,
            "enabled": true,
            "visibility": "Private"
        }))
        .await;

    assert_eq!(
        response.status_code(),
        StatusCode::UNAUTHORIZED,
        "Request with invalid API key should return 401"
    );
}

/// RED Phase: Test that valid API keys grant access
#[tokio::test]
async fn test_valid_api_key_grants_access() {
    let (app, persistence) = create_test_server_with_auth_and_persistence().await;
    let client = axum_test::TestServer::new(app).unwrap();

    // First, create a valid API key
    let api_key = "test-api-key-12345";
    create_api_key_for_test(&persistence, api_key).await;

    // Now use that API key to create a namespace
    let auth_value = format!("Bearer {}", api_key);
    let response = client
        .post("/metamcp/namespaces")
        .add_header(
            axum::http::HeaderName::from_static("authorization"),
            axum::http::HeaderValue::from_str(&auth_value).unwrap(),
        )
        .json(&json!({
            "name": "test-namespace",
            "description": "Test",
            "user_id": "test-user",
            "config_json": r#"{"servers":[]}"#,
            "enabled": true,
            "visibility": "Private"
        }))
        .await;

    assert_eq!(
        response.status_code(),
        StatusCode::OK,
        "Request with valid API key should succeed"
    );
}

/// RED Phase: Test that health endpoint is public (no auth required)
#[tokio::test]
async fn test_health_endpoint_is_public() {
    let app = create_test_server_with_auth().await;
    let client = axum_test::TestServer::new(app).unwrap();

    // Health endpoint should work without authentication
    let response = client.get("/health").await;

    assert_eq!(
        response.status_code(),
        StatusCode::OK,
        "Health endpoint should be public"
    );
}

/// RED Phase: Test that OpenAPI endpoint is public
#[tokio::test]
async fn test_openapi_endpoint_is_public() {
    let app = create_test_server_with_auth().await;
    let client = axum_test::TestServer::new(app).unwrap();

    // OpenAPI endpoint should work without authentication
    let response = client.get("/openapi.json").await;

    assert_eq!(
        response.status_code(),
        StatusCode::OK,
        "OpenAPI endpoint should be public"
    );
}

/// RED Phase: Test that all MCP endpoints require authentication
#[tokio::test]
async fn test_all_mcp_endpoints_require_auth() {
    let app = create_test_server_with_auth().await;
    let client = axum_test::TestServer::new(app).unwrap();

    // Test various MCP endpoints without auth
    let endpoints = vec![
        ("GET", "/metamcp/namespaces"),
        ("POST", "/metamcp/namespaces"),
        ("GET", "/metamcp/endpoints"),
        ("POST", "/metamcp/endpoints"),
        ("GET", "/metamcp/audits"),
        ("POST", "/metamcp/api_keys"),
    ];

    for (method, path) in endpoints {
        let response = match method {
            "GET" => client.get(path).await,
            "POST" => client.post(path).json(&json!({"dummy": "data"})).await,
            _ => panic!("Unsupported method"),
        };

        assert_eq!(
            response.status_code(),
            StatusCode::UNAUTHORIZED,
            "{} {} should require authentication",
            method,
            path
        );
    }
}

/// RED Phase: Test malformed Authorization header
#[tokio::test]
async fn test_malformed_auth_header_returns_401() {
    let app = create_test_server_with_auth().await;
    let client = axum_test::TestServer::new(app).unwrap();

    // Test without "Bearer " prefix
    let response = client
        .post("/metamcp/namespaces")
        .add_header(
            axum::http::HeaderName::from_static("authorization"),
            axum::http::HeaderValue::from_static("invalid-format-key"),
        )
        .json(&json!({
            "name": "test-namespace",
            "description": "Test",
            "user_id": "test-user",
            "config_json": r#"{"servers":[]}"#,
            "enabled": true,
            "visibility": "Private"
        }))
        .await;

    assert_eq!(
        response.status_code(),
        StatusCode::UNAUTHORIZED,
        "Malformed Authorization header should return 401"
    );
}

// Helper functions - GREEN phase implementation

use axum::{
    middleware,
    routing::{delete, get, post},
    Router,
};
use std::sync::Arc;
use terraphim_persistence::mcp::{McpApiKeyRecord, McpPersistence, McpPersistenceImpl};
use terraphim_server::{api_mcp, api_mcp_tools, AppState};

/// Creates a test server with authentication middleware enabled
async fn create_test_server_with_auth() -> axum::Router {
    let (router, _) = create_test_server_with_auth_and_persistence().await;
    router
}

/// Creates a test server with authentication middleware and returns persistence for test setup
async fn create_test_server_with_auth_and_persistence() -> (axum::Router, Arc<McpPersistenceImpl>) {
    use ahash::AHashMap;
    use opendal::services::Memory;
    use opendal::Operator;
    use std::collections::HashMap;
    use tokio::sync::{broadcast, Mutex, RwLock};

    // Create a minimal test config using ConfigBuilder
    let config = terraphim_config::ConfigBuilder::new()
        .global_shortcut("Ctrl+Space")
        .build()
        .expect("Failed to build test config");

    // Create minimal ConfigState for testing
    let config_state = terraphim_server::ConfigState {
        config: Arc::new(Mutex::new(config)),
        roles: AHashMap::new(),
    };

    let workflow_sessions = Arc::new(RwLock::new(HashMap::new()));
    let (websocket_broadcaster, _) = broadcast::channel(1000);

    // Create MCP persistence
    let builder = Memory::default();
    let op = Operator::new(builder).unwrap().finish();
    let mcp_persistence = Arc::new(McpPersistenceImpl::new(op));

    let app_state = AppState {
        config_state,
        workflow_sessions,
        websocket_broadcaster,
        mcp_persistence: mcp_persistence.clone(),
    };

    // Create protected MCP routes with auth middleware
    let protected_mcp_routes = Router::new()
        .route("/metamcp/namespaces", get(api_mcp::list_namespaces))
        .route("/metamcp/namespaces", post(api_mcp::create_namespace))
        .route("/metamcp/namespaces/{uuid}", get(api_mcp::get_namespace))
        .route(
            "/metamcp/namespaces/{uuid}",
            delete(api_mcp::delete_namespace),
        )
        .route("/metamcp/endpoints", get(api_mcp::list_endpoints))
        .route("/metamcp/endpoints", post(api_mcp::create_endpoint))
        .route("/metamcp/endpoints/{uuid}", get(api_mcp::get_endpoint))
        .route(
            "/metamcp/endpoints/{uuid}",
            delete(api_mcp::delete_endpoint),
        )
        .route("/metamcp/api_keys", post(api_mcp::create_api_key))
        .route("/metamcp/audits", get(api_mcp::list_audits))
        .route(
            "/metamcp/endpoints/{endpoint_uuid}/tools",
            get(api_mcp_tools::list_tools_for_endpoint),
        )
        .route(
            "/metamcp/endpoints/{endpoint_uuid}/tools/{tool_name}",
            post(api_mcp_tools::execute_tool),
        )
        .route_layer(middleware::from_fn_with_state(
            app_state.clone(),
            terraphim_server::mcp_auth::validate_api_key,
        ));

    // Public routes (no auth required)
    let router = Router::new()
        .route("/health", get(terraphim_server::health))
        .route(
            "/openapi.json",
            get(|| async { axum::Json(terraphim_server::api_mcp_openapi::McpApiDoc::openapi()) }),
        )
        .merge(protected_mcp_routes)
        .with_state(app_state);

    (router, mcp_persistence)
}

/// Helper to create an API key for testing
async fn create_api_key_for_test(persistence: &Arc<McpPersistenceImpl>, api_key: &str) {
    let key_hash = terraphim_server::mcp_auth::hash_api_key(api_key);

    let record = McpApiKeyRecord {
        uuid: uuid::Uuid::new_v4().to_string(),
        key_hash,
        endpoint_uuid: "test-endpoint".to_string(),
        user_id: Some("test-user".to_string()),
        created_at: chrono::Utc::now(),
        expires_at: None,
        enabled: true,
    };

    persistence
        .save_api_key(&record)
        .await
        .expect("Failed to save test API key");
}
