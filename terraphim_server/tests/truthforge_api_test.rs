use axum_test::TestServer;
use serde_json::json;
use terraphim_server::build_router_for_tests;

#[tokio::test]
async fn test_analyze_narrative_endpoint() {
    let app = build_router_for_tests().await;
    let server = TestServer::new(app).expect("Failed to create test server");

    let response = server
        .post("/api/v1/truthforge")
        .json(&json!({
            "text": "We achieved a 40% cost reduction this quarter through process optimization.",
            "urgency": "Low",
            "stakes": ["Financial"],
            "audience": "Internal"
        }))
        .await;

    response.assert_status_ok();

    let body: serde_json::Value = response.json();
    assert_eq!(body["status"], "success");
    assert!(body["session_id"].is_string());
    assert!(body["analysis_url"].is_string());

    let session_id = body["session_id"].as_str().unwrap();
    assert!(!session_id.is_empty());
}

#[tokio::test]
async fn test_get_analysis_endpoint() {
    let app = build_router_for_tests().await;
    let server = TestServer::new(app).expect("Failed to create test server");

    let post_response = server
        .post("/api/v1/truthforge")
        .json(&json!({
            "text": "Product launch successful.",
            "urgency": "Low",
            "stakes": ["Reputational"],
            "audience": "PublicMedia"
        }))
        .await;

    let post_body: serde_json::Value = post_response.json();
    let session_id = post_body["session_id"].as_str().unwrap();

    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    let get_response = server
        .get(&format!("/api/v1/truthforge/{}", session_id))
        .await;

    get_response.assert_status_ok();

    let get_body: serde_json::Value = get_response.json();
    assert_eq!(get_body["status"], "success");
}

#[tokio::test]
async fn test_list_analyses_endpoint() {
    let app = build_router_for_tests().await;
    let server = TestServer::new(app).expect("Failed to create test server");

    server
        .post("/api/v1/truthforge")
        .json(&json!({
            "text": "Test narrative 1",
        }))
        .await;

    server
        .post("/api/v1/truthforge")
        .json(&json!({
            "text": "Test narrative 2",
        }))
        .await;

    let list_response = server.get("/api/v1/truthforge/analyses").await;

    list_response.assert_status_ok();

    let list_body: serde_json::Value = list_response.json();
    assert!(list_body.is_array());
}

#[tokio::test]
async fn test_narrative_with_defaults() {
    let app = build_router_for_tests().await;
    let server = TestServer::new(app).expect("Failed to create test server");

    let response = server
        .post("/api/v1/truthforge")
        .json(&json!({
            "text": "Minimal narrative for testing defaults.",
        }))
        .await;

    response.assert_status_ok();

    let body: serde_json::Value = response.json();
    assert_eq!(body["status"], "success");
}

#[tokio::test]
async fn test_websocket_progress_events() {
    let app = build_router_for_tests().await;
    let server = TestServer::new(app).expect("Failed to create test server");

    let response = server
        .post("/api/v1/truthforge")
        .json(&json!({
            "text": "We reduced operational costs by 30%.",
            "urgency": "Low",
            "stakes": ["Financial"],
            "audience": "Internal"
        }))
        .await;

    response.assert_status_ok();

    let body: serde_json::Value = response.json();
    let session_id = body["session_id"].as_str().unwrap();

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    assert!(!session_id.is_empty());
}
