//! Basic integration test for server API testing framework

#[cfg(test)]
mod basic_tests {
    use terraphim_validation::testing::server_api::*;

    #[tokio::test]
    async fn test_server_creation() {
        // This test just validates that we can create a test server
        let server_result = TestServer::new().await;
        assert!(server_result.is_ok(), "Failed to create test server");
    }

    #[tokio::test]
    async fn test_health_endpoint() {
        let server = TestServer::new()
            .await
            .expect("Failed to create test server");

        let response = server.get("/health").await;

        assert!(
            response.status().is_success(),
            "Health check should succeed"
        );
    }

    #[tokio::test]
    async fn test_fixture_creation() {
        let document = TestFixtures::sample_document();
        assert_eq!(document.title, "Test Document");
        assert_eq!(document.id, "test-doc-1");
    }
}
