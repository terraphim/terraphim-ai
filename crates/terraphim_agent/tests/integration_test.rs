use std::process::Command;
use std::time::Duration;

use anyhow::Result;
use serial_test::serial;
use terraphim_agent::client::{ApiClient, ChatResponse, ConfigResponse, SearchResponse};
use terraphim_types::{NormalizedTermValue, RoleName, SearchQuery};

const TEST_SERVER_URL: &str = "http://localhost:8000";
#[allow(dead_code)]
const TEST_TIMEOUT: Duration = Duration::from_secs(10);

/// Test helper to check if server is running
async fn is_server_running() -> bool {
    let client = ApiClient::new(TEST_SERVER_URL);
    client.health().await.is_ok()
}

/// Test helper to wait for server startup
#[allow(dead_code)]
async fn wait_for_server() -> Result<()> {
    let max_attempts = 30;
    for _ in 0..max_attempts {
        if is_server_running().await {
            return Ok(());
        }
        tokio::time::sleep(Duration::from_millis(1000)).await;
    }
    anyhow::bail!("Server did not start within timeout")
}

#[tokio::test]
#[serial]
async fn test_api_client_health_check() {
    if !is_server_running().await {
        println!("Server not running, skipping test");
        return;
    }

    let client = ApiClient::new(TEST_SERVER_URL);
    let result = client.health().await;
    assert!(result.is_ok(), "Health check should succeed");
}

#[tokio::test]
#[serial]
async fn test_api_client_search() {
    if !is_server_running().await {
        println!("Server not running, skipping test");
        return;
    }

    let client = ApiClient::new(TEST_SERVER_URL);
    let query = SearchQuery {
        search_term: NormalizedTermValue::from("test"),
        search_terms: None,
        operator: None,
        skip: Some(0),
        limit: Some(5),
        role: Some(RoleName::new("Default")),
    };

    let result = client.search(&query).await;
    assert!(result.is_ok(), "Search should succeed");

    let response: SearchResponse = result.unwrap();
    assert_eq!(response.status, "Success");
    assert!(response.results.len() <= 5);
}

#[tokio::test]
#[serial]
async fn test_api_client_get_config() {
    if !is_server_running().await {
        println!("Server not running, skipping test");
        return;
    }

    let client = ApiClient::new(TEST_SERVER_URL);
    let result = client.get_config().await;
    assert!(result.is_ok(), "Get config should succeed");

    let response: ConfigResponse = result.unwrap();
    assert_eq!(response.status, "Success");
    assert!(!response.config.roles.is_empty());
}

#[tokio::test]
#[serial]
async fn test_api_client_update_selected_role() {
    if !is_server_running().await {
        println!("Server not running, skipping test");
        return;
    }

    let client = ApiClient::new(TEST_SERVER_URL);

    // Get current config to find available roles
    let config_result = client.get_config().await;
    assert!(config_result.is_ok());
    let config = config_result.unwrap();

    // Get first available role
    let role_names: Vec<String> = config.config.roles.keys().map(|k| k.to_string()).collect();

    if role_names.is_empty() {
        println!("No roles available, skipping role update test");
        return;
    }

    let test_role = &role_names[0];
    let result = client.update_selected_role(test_role).await;
    assert!(result.is_ok(), "Update selected role should succeed");

    let response: ConfigResponse = result.unwrap();
    assert_eq!(response.status, "Success");
    assert_eq!(response.config.selected_role.to_string(), *test_role);
}

#[tokio::test]
#[serial]
async fn test_api_client_get_rolegraph() {
    if !is_server_running().await {
        println!("Server not running, skipping test");
        return;
    }

    let client = ApiClient::new(TEST_SERVER_URL);
    let result = client.get_rolegraph_edges(None).await;
    assert!(result.is_ok(), "Get rolegraph should succeed");

    let response = result.unwrap();
    assert_eq!(response.status, "Success");
    // Nodes and edges can be empty, that's valid
}

#[tokio::test]
#[serial]
async fn test_api_client_chat() {
    if !is_server_running().await {
        println!("Server not running, skipping test");
        return;
    }

    let client = ApiClient::new(TEST_SERVER_URL);
    let result = client.chat("Default", "Hello, this is a test", None).await;
    assert!(result.is_ok(), "Chat should succeed");

    let response: ChatResponse = result.unwrap();
    // Chat might succeed or fail depending on LLM availability
    // We just check that the response structure is correct
    assert!(!response.status.is_empty());
}

#[tokio::test]
#[serial]
async fn test_api_client_network_timeout() {
    // Test with invalid URL to ensure timeout behavior
    let client = ApiClient::new("http://invalid-server:9999");
    let result = client.health().await;
    assert!(result.is_err(), "Should fail with network error");
}

#[tokio::test]
#[serial]
async fn test_search_with_different_roles() {
    if !is_server_running().await {
        println!("Server not running, skipping test");
        return;
    }

    let client = ApiClient::new(TEST_SERVER_URL);

    // Get available roles
    let config_result = client.get_config().await;
    assert!(config_result.is_ok());
    let config = config_result.unwrap();

    let role_names: Vec<String> = config.config.roles.keys().map(|k| k.to_string()).collect();

    if role_names.is_empty() {
        println!("No roles available, skipping multi-role search test");
        return;
    }

    // Test search with each available role
    for role_name in role_names {
        let query = SearchQuery {
            search_term: NormalizedTermValue::from("test"),
            search_terms: None,
            operator: None,
            skip: Some(0),
            limit: Some(3),
            role: Some(RoleName::new(&role_name)),
        };

        let result = client.search(&query).await;
        assert!(
            result.is_ok(),
            "Search with role {} should succeed",
            role_name
        );

        let response: SearchResponse = result.unwrap();
        assert_eq!(response.status, "Success");
        assert!(response.results.len() <= 3);
    }
}

#[tokio::test]
#[serial]
async fn test_search_pagination() {
    if !is_server_running().await {
        println!("Server not running, skipping test");
        return;
    }

    let client = ApiClient::new(TEST_SERVER_URL);

    // Search first page
    let query1 = SearchQuery {
        search_term: NormalizedTermValue::from("test"),
        search_terms: None,
        operator: None,
        skip: Some(0),
        limit: Some(2),
        role: Some(RoleName::new("Default")),
    };

    let result1 = client.search(&query1).await;
    assert!(result1.is_ok());

    // Search second page
    let query2 = SearchQuery {
        search_term: NormalizedTermValue::from("test"),
        search_terms: None,
        operator: None,
        skip: Some(2),
        limit: Some(2),
        role: Some(RoleName::new("Default")),
    };

    let result2 = client.search(&query2).await;
    assert!(result2.is_ok());

    let response1: SearchResponse = result1.unwrap();
    let response2: SearchResponse = result2.unwrap();

    // If there are enough results, pages should be different
    if response1.results.len() == 2 && !response2.results.is_empty() {
        // Results should be different (assuming different documents)
        let ids1: Vec<String> = response1.results.iter().map(|d| d.id.clone()).collect();
        let ids2: Vec<String> = response2.results.iter().map(|d| d.id.clone()).collect();

        // Should have different document IDs (no overlap)
        for id1 in &ids1 {
            assert!(!ids2.contains(id1), "Pages should have different documents");
        }
    }
}

#[test]
#[serial]
fn test_tui_cli_search_command() {
    if !std::process::Command::new("cargo")
        .args(["build", "--bin", "terraphim-agent"])
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
    {
        println!("Could not build TUI binary, skipping CLI test");
        return;
    }

    let output = Command::new("cargo")
        .args([
            "run",
            "--bin",
            "terraphim-agent",
            "--",
            "search",
            "test",
            "--limit",
            "3",
        ])
        .env("TERRAPHIM_SERVER", TEST_SERVER_URL)
        .output();

    if let Ok(output) = output {
        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            println!("CLI search output: {}", stdout);
            // Should contain some search results or handle gracefully
            assert!(!stdout.contains("error"), "CLI should not show errors");
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            println!("CLI search failed: {}", stderr);
            // This might fail if server is not running, which is okay for testing
        }
    }
}

#[test]
#[serial]
fn test_tui_cli_roles_list_command() {
    if !std::process::Command::new("cargo")
        .args(["build", "--bin", "terraphim-agent"])
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
    {
        println!("Could not build TUI binary, skipping CLI test");
        return;
    }

    let output = Command::new("cargo")
        .args(["run", "--bin", "terraphim-agent", "--", "roles", "list"])
        .env("TERRAPHIM_SERVER", TEST_SERVER_URL)
        .output();

    if let Ok(output) = output {
        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            println!("CLI roles list output: {}", stdout);
            // Should contain role names or handle gracefully
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            println!("CLI roles list failed: {}", stderr);
        }
    }
}

#[test]
#[serial]
fn test_tui_cli_config_show_command() {
    if !std::process::Command::new("cargo")
        .args(["build", "--bin", "terraphim-agent"])
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
    {
        println!("Could not build TUI binary, skipping CLI test");
        return;
    }

    let output = Command::new("cargo")
        .args(["run", "--bin", "terraphim-agent", "--", "config", "show"])
        .env("TERRAPHIM_SERVER", TEST_SERVER_URL)
        .output();

    if let Ok(output) = output {
        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            println!("CLI config show output: {}", stdout);
            // Should contain JSON config or handle gracefully
            if !stdout.is_empty() {
                // Try to parse as JSON if we got content
                if stdout.starts_with('{') {
                    assert!(
                        serde_json::from_str::<serde_json::Value>(&stdout).is_ok(),
                        "Config output should be valid JSON"
                    );
                }
            }
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            println!("CLI config show failed: {}", stderr);
        }
    }
}

#[test]
#[serial]
fn test_tui_cli_graph_command() {
    if !std::process::Command::new("cargo")
        .args(["build", "--bin", "terraphim-agent"])
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
    {
        println!("Could not build TUI binary, skipping CLI test");
        return;
    }

    let output = Command::new("cargo")
        .args([
            "run",
            "--bin",
            "terraphim-agent",
            "--",
            "graph",
            "--top-k",
            "5",
        ])
        .env("TERRAPHIM_SERVER", TEST_SERVER_URL)
        .output();

    if let Ok(output) = output {
        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            println!("CLI graph output: {}", stdout);
            // Should show nodes/edges count and top-k nodes
            assert!(
                stdout.contains("Nodes:") && stdout.contains("Edges:"),
                "Graph output should show node and edge counts"
            );
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            println!("CLI graph failed: {}", stderr);
        }
    }
}

#[tokio::test]
#[serial]
async fn test_api_error_handling() {
    if !is_server_running().await {
        println!("Server not running, skipping test");
        return;
    }

    let client = ApiClient::new(TEST_SERVER_URL);

    // Test search with invalid parameters
    let query = SearchQuery {
        search_term: NormalizedTermValue::from(""), // Empty search
        search_terms: None,
        operator: None,
        skip: Some(0),
        limit: Some(0), // Invalid limit
        role: Some(RoleName::new("NonExistentRole")),
    };

    let result = client.search(&query).await;
    // This might succeed or fail depending on server implementation
    // We just ensure it doesn't panic
    match result {
        Ok(response) => {
            println!("Empty search response: {:?}", response);
        }
        Err(e) => {
            println!("Empty search error (expected): {:?}", e);
        }
    }
}
