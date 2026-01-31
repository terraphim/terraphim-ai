//! Test server harness for API testing
//!
//! This module provides a test server that can be used to test terraphim server API endpoints
//! in isolation with mocked dependencies.

use terraphim_config::ConfigState;

// Import the axum-test TestServer and alias it to avoid conflicts
use axum_test::TestServer as AxumTestServer;

/// Test harness for running terraphim server in integration tests
pub struct ServerHarness {
    pub server: AxumTestServer,
    pub base_url: String,
}

impl ServerHarness {
    /// Start a terraphim server with config for testing
    pub async fn start_with_config(_config_state: ConfigState) -> Self {
        // Build router using the same function as tests
        let router = terraphim_server::build_router_for_tests().await;
        let server = AxumTestServer::new(router).unwrap();
        let base_url = "http://localhost:8080".to_string();

        Self { server, base_url }
    }

    /// Get the test server instance for making requests
    pub fn server(&self) -> &AxumTestServer {
        &self.server
    }
}

/// Test server for API endpoint validation (legacy compatibility)
pub struct TestServer {
    /// The axum-test server instance
    pub server: AxumTestServer,
    /// Base URL of the test server
    pub base_url: String,
}

impl TestServer {
    /// Create a new test server with default configuration
    pub async fn new() -> Result<Self, Box<dyn std::error::Error>> {
        // Build router with test configuration
        let router = terraphim_server::build_router_for_tests().await;
        let server = AxumTestServer::new(router)?;
        let base_url = "http://localhost:8080".to_string();

        Ok(Self { server, base_url })
    }

    /// Make a GET request to the test server
    pub async fn get(&self, path: &str) -> axum_test::TestResponse {
        self.server.get(path).await
    }

    /// Make a POST request to the test server with JSON body
    pub async fn post<T: serde::Serialize>(&self, path: &str, body: &T) -> axum_test::TestResponse {
        self.server.post(path).json(body).await
    }

    /// Make a PUT request to the test server with JSON body
    pub async fn put<T: serde::Serialize>(&self, path: &str, body: &T) -> axum_test::TestResponse {
        self.server.put(path).json(body).await
    }

    /// Make a DELETE request to the test server
    pub async fn delete(&self, path: &str) -> axum_test::TestResponse {
        self.server.delete(path).await
    }
}
