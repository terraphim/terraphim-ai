use std::time::Duration;

use serial_test::serial;
use terraphim_agent::client::ApiClient;
use terraphim_types::{Document, DocumentType, NormalizedTermValue, RoleName, SearchQuery};
use tokio::time::timeout;

const TEST_SERVER_URL: &str = "http://localhost:8000";
const INVALID_SERVER_URL: &str = "http://localhost:9999";
const TIMEOUT_SERVER_URL: &str = "http://192.0.2.1:8000"; // RFC5737 test address

/// Test helper to check if server is running
async fn is_server_running() -> bool {
    let client = ApiClient::new(TEST_SERVER_URL);
    timeout(Duration::from_secs(2), client.health())
        .await
        .map(|result| result.is_ok())
        .unwrap_or(false)
}

#[tokio::test]
#[serial]
async fn test_network_timeout_handling() {
    // Test with unreachable address that should timeout
    let client = ApiClient::new(TIMEOUT_SERVER_URL);

    let result = timeout(Duration::from_secs(5), client.health()).await;

    // Should either timeout or return an error
    match result {
        Ok(Ok(_)) => panic!("Should not succeed with unreachable server"),
        Ok(Err(e)) => {
            println!("✅ Network error handled correctly: {}", e);
            // Verify error message indicates network issue
            let error_str = e.to_string().to_lowercase();
            assert!(
                error_str.contains("timeout")
                    || error_str.contains("connect")
                    || error_str.contains("network")
                    || error_str.contains("unreachable"),
                "Error should indicate network issue: {}",
                e
            );
        }
        Err(_) => println!("✅ Request timed out as expected"),
    }
}

#[tokio::test]
#[serial]
async fn test_connection_refused_handling() {
    // Test with invalid port that should be refused
    let client = ApiClient::new(INVALID_SERVER_URL);

    let result = client.health().await;
    assert!(result.is_err(), "Should fail with connection refused");

    let error = result.unwrap_err();
    let error_str = error.to_string().to_lowercase();
    assert!(
        error_str.contains("connect")
            || error_str.contains("refused")
            || error_str.contains("network")
            || error_str.contains("connection")
            || error_str.contains("unreachable")
            || error_str.contains("sending request"),
        "Error should indicate connection issue: {}",
        error
    );

    println!("✅ Connection refused handled correctly: {}", error);
}

#[tokio::test]
#[serial]
async fn test_malformed_server_response() {
    if !is_server_running().await {
        println!("Server not running, skipping malformed response test");
        return;
    }

    // This test assumes the server might return malformed JSON in some edge cases
    // We'll test with extreme parameters that might cause issues

    let client = ApiClient::new(TEST_SERVER_URL);

    // Test with very large limit that might cause server issues
    let extreme_query = SearchQuery {
        search_term: NormalizedTermValue::from("test"),
        search_terms: None,
        operator: None,
        skip: Some(0),
        limit: Some(100000), // Extremely large limit
        role: Some(RoleName::new("Default")),
    };

    let result = client.search(&extreme_query).await;

    // Should either succeed with reasonable results or fail gracefully
    match result {
        Ok(response) => {
            println!("✅ Extreme parameters handled gracefully");
            // Response should be reasonable even with extreme params
            assert!(
                response.results.len() <= 10000,
                "Results should be capped to reasonable size"
            );
        }
        Err(e) => {
            println!("✅ Extreme parameters rejected appropriately: {}", e);
            // Error should not be a panic or internal server error
            let error_str = e.to_string().to_lowercase();
            assert!(
                !error_str.contains("panic") && !error_str.contains("internal server error"),
                "Should not be internal server error: {}",
                e
            );
        }
    }
}

#[tokio::test]
#[serial]
async fn test_invalid_role_handling() {
    if !is_server_running().await {
        println!("Server not running, skipping invalid role test");
        return;
    }

    let client = ApiClient::new(TEST_SERVER_URL);

    // Test with completely invalid role name
    let invalid_query = SearchQuery {
        search_term: NormalizedTermValue::from("test"),
        search_terms: None,
        operator: None,
        skip: Some(0),
        limit: Some(5),
        role: Some(RoleName::new("CompleteLyInvalidRoleName12345")),
    };

    let result = client.search(&invalid_query).await;

    // Should handle gracefully - either succeed with empty results or clear error
    match result {
        Ok(response) => {
            println!(
                "✅ Invalid role handled with response: status={}, results={}",
                response.status,
                response.results.len()
            );
            // Should not crash, status should indicate result
            assert!(!response.status.is_empty());
        }
        Err(e) => {
            println!("✅ Invalid role rejected appropriately: {}", e);
            let error_str = e.to_string().to_lowercase();
            // Should be a client error, not server crash
            assert!(
                error_str.contains("role")
                    || error_str.contains("not found")
                    || error_str.contains("400")
                    || error_str.contains("bad request"),
                "Should be role-related error: {}",
                e
            );
        }
    }
}

#[tokio::test]
#[serial]
async fn test_empty_and_special_character_queries() {
    if !is_server_running().await {
        println!("Server not running, skipping special character test");
        return;
    }

    let client = ApiClient::new(TEST_SERVER_URL);

    let long_query = "a".repeat(10000);
    let special_queries = [
        "",                              // Empty query
        "   ",                           // Whitespace only
        "!@#$%^&*()",                    // Special characters
        "SELECT * FROM users",           // SQL injection attempt
        "<script>alert('xss')</script>", // XSS attempt
        "../../../../etc/passwd",        // Path traversal attempt
        "\0\n\r\t",                      // Control characters
        "🚀🔥💻",                        // Emojis
        "русский中文日本語",             // Unicode text
        &long_query,                     // Very long query
    ];

    for (i, query) in special_queries.iter().enumerate() {
        let search_query = SearchQuery {
            search_term: NormalizedTermValue::from(*query),
            search_terms: None,
            operator: None,
            skip: Some(0),
            limit: Some(5),
            role: Some(RoleName::new("Default")),
        };

        let result = client.search(&search_query).await;

        match result {
            Ok(response) => {
                println!(
                    "✅ Special query {} handled: status={}, results={}",
                    i,
                    response.status,
                    response.results.len()
                );
                // Should not crash and should have valid status
                assert!(!response.status.is_empty());
            }
            Err(e) => {
                println!("✅ Special query {} rejected appropriately: {}", i, e);
                let error_str = e.to_string().to_lowercase();
                // Should not be internal server error
                assert!(
                    !error_str.contains("internal server error") && !error_str.contains("500"),
                    "Should not be internal server error for query {}: {}",
                    i,
                    e
                );
            }
        }
    }
}

#[tokio::test]
#[serial]
async fn test_concurrent_request_handling() {
    if !is_server_running().await {
        println!("Server not running, skipping concurrent request test");
        return;
    }

    let client = ApiClient::new(TEST_SERVER_URL);

    // Create multiple concurrent requests
    let mut tasks = Vec::new();

    for i in 0..10 {
        let client_clone = client.clone();
        let task = tokio::spawn(async move {
            let query = SearchQuery {
                search_term: NormalizedTermValue::from(format!("concurrent test {}", i)),
                search_terms: None,
                operator: None,
                skip: Some(0),
                limit: Some(3),
                role: Some(RoleName::new("Default")),
            };
            client_clone.search(&query).await
        });
        tasks.push(task);
    }

    // Wait for all requests to complete
    let results = futures::future::join_all(tasks).await;

    let mut successes = 0;
    let mut errors = 0;

    for (i, result) in results.into_iter().enumerate() {
        match result {
            Ok(Ok(response)) => {
                successes += 1;
                println!(
                    "✅ Concurrent request {} succeeded: status={}",
                    i, response.status
                );
                assert!(!response.status.is_empty());
            }
            Ok(Err(e)) => {
                errors += 1;
                println!("⚠️ Concurrent request {} failed: {}", i, e);
                // Errors are acceptable under high concurrency, but should not be crashes
                let error_str = e.to_string().to_lowercase();
                assert!(
                    !error_str.contains("panic") && !error_str.contains("internal server error"),
                    "Should not be internal error: {}",
                    e
                );
            }
            Err(e) => {
                errors += 1;
                println!("⚠️ Concurrent request {} panicked: {}", i, e);
            }
        }
    }

    println!(
        "Concurrent requests: {} successes, {} errors",
        successes, errors
    );
    // At least some requests should succeed
    assert!(
        successes > 0,
        "At least some concurrent requests should succeed"
    );
}

#[tokio::test]
#[serial]
async fn test_config_error_scenarios() {
    if !is_server_running().await {
        println!("Server not running, skipping config error test");
        return;
    }

    let client = ApiClient::new(TEST_SERVER_URL);

    // Test invalid role update
    let result = client.update_selected_role("InvalidRoleName12345").await;

    match result {
        Ok(response) => {
            println!("✅ Invalid role update handled: status={}", response.status);
            // Should either reject or handle gracefully
            assert!(!response.status.is_empty());
        }
        Err(e) => {
            println!("✅ Invalid role update rejected: {}", e);
            let error_str = e.to_string().to_lowercase();
            assert!(
                error_str.contains("role")
                    || error_str.contains("not found")
                    || error_str.contains("400"),
                "Should be role-related error: {}",
                e
            );
        }
    }
}

#[tokio::test]
#[serial]
async fn test_summarization_error_handling() {
    if !is_server_running().await {
        println!("Server not running, skipping summarization error test");
        return;
    }

    let client = ApiClient::new(TEST_SERVER_URL);

    // Test summarization with invalid document
    let invalid_doc = Document {
        id: "invalid-doc".to_string(),
        title: "".to_string(), // Empty title
        body: "".to_string(),  // No body
        url: "invalid-url".to_string(),
        description: None,
        summarization: None,
        stub: None,
        tags: None,
        rank: None,
        source_haystack: None,
        doc_type: DocumentType::KgEntry,
        synonyms: None,
        route: None,
        priority: None,
    };

    let result = client
        .summarize_document(&invalid_doc, Some("Default"))
        .await;

    match result {
        Ok(response) => {
            println!(
                "✅ Invalid document summarization handled: status={}",
                response.status
            );
            // Should handle gracefully
            assert!(!response.status.is_empty());
            if response.status == "Error" {
                assert!(
                    response.error.is_some(),
                    "Error status should have error message"
                );
            }
        }
        Err(e) => {
            println!("✅ Invalid document summarization rejected: {}", e);
            // Should not be internal server error
            let error_str = e.to_string().to_lowercase();
            assert!(
                !error_str.contains("internal server error") && !error_str.contains("500"),
                "Should not be internal server error: {}",
                e
            );
        }
    }
}

#[tokio::test]
#[serial]
async fn test_autocomplete_error_handling() {
    if !is_server_running().await {
        println!("Server not running, skipping autocomplete error test");
        return;
    }

    let client = ApiClient::new(TEST_SERVER_URL);

    // Test autocomplete with invalid parameters
    let long_autocomplete_query = "a".repeat(1000);
    let test_cases = vec![
        ("InvalidRole", "test"),
        ("Default", ""),                       // Empty query
        ("Default", &long_autocomplete_query), // Very long query
    ];

    for (role, query) in test_cases {
        let result = client.get_autocomplete(role, query).await;

        match result {
            Ok(response) => {
                println!(
                    "✅ Autocomplete handled: role={}, query_len={}, status={}",
                    role,
                    query.len(),
                    response.status
                );
                assert!(!response.status.is_empty());
            }
            Err(e) => {
                println!(
                    "✅ Autocomplete error handled: role={}, query_len={}, error={}",
                    role,
                    query.len(),
                    e
                );
                let error_str = e.to_string().to_lowercase();
                assert!(
                    !error_str.contains("internal server error") && !error_str.contains("500"),
                    "Should not be internal server error: {}",
                    e
                );
            }
        }
    }
}

#[tokio::test]
#[serial]
async fn test_client_timeout_configuration() {
    // Test that the client respects timeout settings
    let client = ApiClient::new("http://httpbin.org/delay/15"); // 15 second delay

    let start = std::time::Instant::now();
    let result = client.health().await;
    let duration = start.elapsed();

    // Should timeout before 15 seconds (client has 10 second timeout)
    assert!(
        duration < Duration::from_secs(12),
        "Should timeout before 12 seconds"
    );
    assert!(result.is_err(), "Should fail due to timeout");

    let error = result.unwrap_err();
    let error_str = error.to_string().to_lowercase();
    assert!(
        error_str.contains("timeout")
            || error_str.contains("connect")
            || error_str.contains("timed out")
            || error_str.contains("deadline")
            || error_str.contains("elapsed")
            || error_str.contains("health check failed"),
        "Error should indicate timeout: {}",
        error
    );

    println!(
        "✅ Client timeout configuration working correctly: {:?}",
        duration
    );
}

#[tokio::test]
#[serial]
async fn test_graceful_degradation() {
    if !is_server_running().await {
        println!("Server not running, skipping graceful degradation test");
        return;
    }

    let client = ApiClient::new(TEST_SERVER_URL);

    // Test multiple operations in sequence to ensure no state corruption
    let operations = vec![
        ("config", "get_config"),
        ("search", "search with empty query"),
        ("rolegraph", "get_rolegraph"),
        ("config", "get_config again"),
    ];

    let mut all_succeeded = true;

    for (op_type, description) in operations {
        let result = match op_type {
            "config" => client
                .get_config()
                .await
                .map(|_| "Success".to_string())
                .map_err(|e| e.to_string()),
            "search" => {
                let query = SearchQuery {
                    search_term: NormalizedTermValue::from(""),
                    search_terms: None,
                    operator: None,
                    skip: Some(0),
                    limit: Some(1),
                    role: Some(RoleName::new("Default")),
                };
                client
                    .search(&query)
                    .await
                    .map(|r| r.status)
                    .map_err(|e| e.to_string())
            }
            "rolegraph" => client
                .get_rolegraph_edges(Some("Default"))
                .await
                .map(|r| r.status)
                .map_err(|e| e.to_string()),
            _ => Ok("Unknown".to_string()),
        };

        match result {
            Ok(status) => {
                println!("✅ Operation {} succeeded: {}", description, status);
            }
            Err(e) => {
                println!("⚠️ Operation {} failed: {}", description, e);
                all_succeeded = false;

                // Even if it fails, should not corrupt client state
                assert!(!e.contains("panic"), "Should not panic: {}", e);
            }
        }
    }

    // Client should remain functional even after errors
    let final_health = client.health().await;
    match final_health {
        Ok(_) => println!("✅ Client remains functional after error sequence"),
        Err(e) => println!("⚠️ Client health check failed: {}", e),
    }

    // Note: Some operations may fail due to server state, this is expected
    println!(
        "Operation sequence complete. All operations succeeded: {}",
        all_succeeded
    );
}
