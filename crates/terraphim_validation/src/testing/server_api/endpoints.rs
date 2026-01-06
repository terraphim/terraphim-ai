//! Basic API endpoint tests for terraphim server
//!
//! This module contains basic tests for core terraphim server API endpoints.
//!
//! Note: These tests require the `server-api-tests` feature to compile.

#![allow(unused_imports)]

#[cfg(feature = "server-api-tests")]
use crate::testing::server_api::{TestFixtures, TestServer};

/// Health check endpoint tests
#[cfg(feature = "server-api-tests")]
pub mod health_tests {
    use super::*;

    #[tokio::test]
    async fn test_health_check() {
        let server = TestServer::new()
            .await
            .expect("Failed to create test server");

        let response = server.get("/health").await.expect("Request failed");

        assert!(response.status().is_success());

        let body = response.text().await.expect("Failed to read response body");
        assert_eq!(body, "OK");
    }
}

/// Basic document management endpoint tests
#[cfg(feature = "server-api-tests")]
pub mod document_tests {
    use super::*;

    #[tokio::test]
    async fn test_create_document_success() {
        let server = TestServer::new()
            .await
            .expect("Failed to create test server");
        let document = TestFixtures::sample_document();

        let response = server
            .post("/documents", &document)
            .await
            .expect("Request failed");

        assert!(response.status().is_success());
    }

    #[tokio::test]
    async fn test_search_documents() {
        let server = TestServer::new()
            .await
            .expect("Failed to create test server");

        let response = server
            .get("/documents/search?query=test")
            .await
            .expect("Search request failed");

        assert!(response.status().is_success());
    }
}

/// Basic configuration endpoint tests
#[cfg(feature = "server-api-tests")]
pub mod config_tests {
    use super::*;

    #[tokio::test]
    async fn test_get_config() {
        let server = TestServer::new()
            .await
            .expect("Failed to create test server");

        let response = server.get("/config").await.expect("Config request failed");

        assert!(response.status().is_success());
    }
}
