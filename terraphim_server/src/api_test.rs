#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
        routing::post,
        Router,
    };
    use serde_json::json;
    use tower::ServiceExt;
    use terraphim_config::Config;
    use terraphim_types::RoleName;

    async fn setup_test_app() -> Router {
        let config = Config::default();
        let config_state = ConfigState::new(config);
        
        Router::new()
            .route("/nodes", post(list_ranked_nodes))
            .with_state(config_state)
    }

    #[tokio::test]
    async fn test_list_ranked_nodes() {
        let app = setup_test_app().await;

        // Create test request
        let request = Request::builder()
            .method("POST")
            .uri("/nodes")
            .header("Content-Type", "application/json")
            .body(Body::from(
                json!({
                    "role": "system operator"
                })
                .to_string(),
            ))
            .unwrap();

        // Send request
        let response = app.oneshot(request).await.unwrap();

        // Check status
        assert_eq!(response.status(), StatusCode::OK);

        // Get response body
        let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
        let response: NodesResponse = serde_json::from_slice(&body).unwrap();

        // Verify response structure
        assert_eq!(response.status, Status::Success);
        assert!(!response.nodes.is_empty(), "Response should contain nodes");

        // Verify nodes are properly sorted and deduplicated
        let nodes = response.nodes;
        for window in nodes.windows(2) {
            // Check sorting by total_documents
            assert!(
                window[0].total_documents >= window[1].total_documents,
                "Nodes should be sorted by total_documents in descending order"
            );

            // Check no duplicate normalized terms
            assert_ne!(
                window[0].normalized_term, window[1].normalized_term,
                "Nodes should not have duplicate normalized terms"
            );
        }
    }

    #[tokio::test]
    async fn test_list_ranked_nodes_invalid_role() {
        let app = setup_test_app().await;

        // Create test request with invalid role
        let request = Request::builder()
            .method("POST")
            .uri("/nodes")
            .header("Content-Type", "application/json")
            .body(Body::from(
                json!({
                    "role": "nonexistent_role"
                })
                .to_string(),
            ))
            .unwrap();

        // Send request
        let response = app.oneshot(request).await.unwrap();

        // Should return error for invalid role
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }
} 