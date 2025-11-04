use axum::http::StatusCode;
use serde_json::json;
use tempfile::TempDir;

/// Test that MCP data persists across server restarts when using SQLite
#[tokio::test]
async fn test_namespace_persists_across_restart() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("mcp.db");

    // Create first server instance
    let namespace_uuid = {
        let app = create_test_server_with_sqlite(&db_path).await;
        let client = axum_test::TestServer::new(app).unwrap();

        // Create a namespace
        let response = client
            .post("/metamcp/namespaces")
            .json(&json!({
                "name": "test-namespace",
                "description": "Test namespace for persistence",
                "user_id": "test-user",
                "config_json": r#"{"name":"test-namespace","servers":[],"tool_overrides":{},"enabled":true}"#,
                "enabled": true,
                "visibility": "Private"
            }))
            .await;

        assert_eq!(response.status_code(), StatusCode::OK);

        let body: serde_json::Value = response.json();
        let uuid = body["namespace"]["uuid"].as_str().unwrap().to_string();
        assert!(!uuid.is_empty());
        uuid
    };
    // First server instance is dropped here

    // Create second server instance with same database
    {
        let app = create_test_server_with_sqlite(&db_path).await;
        let client = axum_test::TestServer::new(app).unwrap();

        // Verify namespace still exists
        let response = client
            .get(&format!("/metamcp/namespaces/{}", namespace_uuid))
            .await;

        assert_eq!(response.status_code(), StatusCode::OK);

        let body: serde_json::Value = response.json();
        assert_eq!(body["status"], "success");
        assert_eq!(body["namespace"]["uuid"], namespace_uuid);
        assert_eq!(body["namespace"]["name"], "test-namespace");
    }
}

/// Test concurrent writes to SQLite don't cause locking issues
#[tokio::test]
async fn test_concurrent_namespace_creation() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("mcp.db");
    let app = create_test_server_with_sqlite(&db_path).await;

    // Create 10 namespaces concurrently using separate clients
    let mut handles = vec![];
    for i in 0..10 {
        let app_clone = app.clone();
        let handle = tokio::spawn(async move {
            let client = axum_test::TestServer::new(app_clone).unwrap();
            client
                .post("/metamcp/namespaces")
                .json(&json!({
                    "name": format!("namespace-{}", i),
                    "description": "Concurrent test",
                    "user_id": "test-user",
                    "config_json": r#"{"name":"test","servers":[],"tool_overrides":{},"enabled":true}"#,
                    "enabled": true,
                    "visibility": "Private"
                }))
                .await
        });
        handles.push(handle);
    }

    // Wait for all requests to complete
    for handle in handles {
        let response = handle.await.unwrap();
        assert_eq!(response.status_code(), StatusCode::OK);
    }

    // Verify all 10 were created
    let client = axum_test::TestServer::new(app).unwrap();
    let response = client.get("/metamcp/namespaces").await;
    let body: serde_json::Value = response.json();
    assert_eq!(body["namespaces"].as_array().unwrap().len(), 10);
}

/// Test that SQLite connection pool is properly managed
#[tokio::test]
async fn test_sqlite_connection_pooling() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("mcp.db");
    let app = create_test_server_with_sqlite(&db_path).await;

    // Make 50 rapid requests to test connection pooling
    let mut handles = vec![];
    for i in 0..50 {
        let app_clone = app.clone();
        let handle = tokio::spawn(async move {
            let client = axum_test::TestServer::new(app_clone).unwrap();
            if i % 2 == 0 {
                // Half reads
                client.get("/metamcp/namespaces").await
            } else {
                // Half writes
                client
                    .post("/metamcp/namespaces")
                    .json(&json!({
                        "name": format!("pool-test-{}", i),
                        "description": "Pool test",
                        "user_id": "test-user",
                        "config_json": r#"{"name":"test","servers":[],"tool_overrides":{},"enabled":true}"#,
                        "enabled": true,
                        "visibility": "Private"
                    }))
                    .await
            }
        });
        handles.push(handle);
    }

    // All should succeed without connection pool exhaustion
    for handle in handles {
        let response = handle.await.unwrap();
        assert!(response.status_code().is_success());
    }
}

// Helper function - will be implemented to make tests pass
async fn create_test_server_with_sqlite(_db_path: &std::path::Path) -> axum::Router {
    // This is the RED phase - function doesn't exist yet
    // We'll implement it in the GREEN phase
    panic!("Not implemented yet - this is the RED phase of TDD")
}
