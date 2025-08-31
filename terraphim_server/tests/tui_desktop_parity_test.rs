use std::collections::HashSet;
use std::time::Duration;

use reqwest::Client;
use serde_json::Value;
use serial_test::serial;
use terraphim_tui::client::ApiClient;
use terraphim_types::{NormalizedTermValue, RoleName, SearchQuery};

const TEST_SERVER_URL: &str = "http://localhost:8000";

/// Test helper to check if server is running
async fn is_server_running() -> bool {
    Client::new()
        .get(format!("{}/health", TEST_SERVER_URL))
        .timeout(Duration::from_secs(5))
        .send()
        .await
        .map(|r| r.status().is_success())
        .unwrap_or(false)
}

/// Validate that TUI client and direct HTTP calls return identical search results
#[tokio::test]
#[serial]
async fn test_tui_vs_direct_api_search_parity() {
    if !is_server_running().await {
        println!("Server not running, skipping parity test");
        return;
    }

    let test_query = "test search";
    let test_role = "Default";
    let limit = 5;

    // TUI client search
    let tui_client = ApiClient::new(TEST_SERVER_URL);
    let tui_query = SearchQuery {
        search_term: NormalizedTermValue::from(test_query),
        search_terms: None,
        operator: None,
        skip: Some(0),
        limit: Some(limit),
        role: Some(RoleName::new(test_role)),
    };

    let tui_result = tui_client.search(&tui_query).await;
    assert!(tui_result.is_ok(), "TUI search should succeed");
    let tui_response = tui_result.unwrap();

    // Direct HTTP API search (simulating desktop)
    let http_client = Client::new();
    let http_result = http_client
        .post(format!("{}/documents/search", TEST_SERVER_URL))
        .json(&tui_query)
        .send()
        .await;

    assert!(http_result.is_ok(), "HTTP search should succeed");
    let http_response = http_result.unwrap();
    assert!(
        http_response.status().is_success(),
        "HTTP response should be successful"
    );

    let http_body: Value = http_response.json().await.unwrap();

    // Validate response structure matches
    assert_eq!(tui_response.status, http_body["status"].as_str().unwrap());
    assert_eq!(
        tui_response.total,
        http_body["total"].as_u64().unwrap() as usize
    );
    assert_eq!(
        tui_response.results.len(),
        http_body["results"].as_array().unwrap().len()
    );

    // Validate individual results match
    for (i, tui_doc) in tui_response.results.iter().enumerate() {
        let http_doc = &http_body["results"][i];
        assert_eq!(tui_doc.id, http_doc["id"].as_str().unwrap());
        assert_eq!(tui_doc.title, http_doc["title"].as_str().unwrap());
        assert_eq!(tui_doc.rank, http_doc["rank"].as_f64().map(|f| f as u64));
    }

    println!("✅ TUI and HTTP API search results are identical");
}

/// Validate that TUI and HTTP config responses are identical
#[tokio::test]
#[serial]
async fn test_tui_vs_direct_api_config_parity() {
    if !is_server_running().await {
        println!("Server not running, skipping config parity test");
        return;
    }

    // TUI client config
    let tui_client = ApiClient::new(TEST_SERVER_URL);
    let tui_result = tui_client.get_config().await;
    assert!(tui_result.is_ok(), "TUI config should succeed");
    let tui_response = tui_result.unwrap();

    // Direct HTTP API config
    let http_client = Client::new();
    let http_result = http_client
        .get(format!("{}/config", TEST_SERVER_URL))
        .send()
        .await;

    assert!(http_result.is_ok(), "HTTP config should succeed");
    let http_response = http_result.unwrap();
    assert!(
        http_response.status().is_success(),
        "HTTP response should be successful"
    );

    let http_body: Value = http_response.json().await.unwrap();

    // Validate basic response structure
    assert_eq!(tui_response.status, http_body["status"].as_str().unwrap());

    // Validate selected role matches
    assert_eq!(
        tui_response.config.selected_role.to_string(),
        http_body["config"]["selected_role"].as_str().unwrap()
    );

    // Validate roles count matches
    let tui_roles_count = tui_response.config.roles.len();
    let http_roles_count = http_body["config"]["roles"].as_object().unwrap().len();
    assert_eq!(tui_roles_count, http_roles_count);

    // Validate role names match
    let tui_role_names: HashSet<String> = tui_response
        .config
        .roles
        .keys()
        .map(|k| k.to_string())
        .collect();
    let http_role_names: HashSet<String> = http_body["config"]["roles"]
        .as_object()
        .unwrap()
        .keys()
        .cloned()
        .collect();
    assert_eq!(tui_role_names, http_role_names);

    println!("✅ TUI and HTTP API config responses are identical");
}

/// Test role switching parity between TUI and direct API
#[tokio::test]
#[serial]
async fn test_tui_vs_direct_api_role_switching_parity() {
    if !is_server_running().await {
        println!("Server not running, skipping role switching parity test");
        return;
    }

    let tui_client = ApiClient::new(TEST_SERVER_URL);

    // Get available roles
    let config_result = tui_client.get_config().await;
    assert!(config_result.is_ok());
    let config = config_result.unwrap();

    let roles: Vec<String> = config.config.roles.keys().map(|k| k.to_string()).collect();

    if roles.is_empty() {
        println!("No roles available, skipping role switching test");
        return;
    }

    let test_role = &roles[0];

    // TUI client role update
    let tui_result = tui_client.update_selected_role(test_role).await;
    assert!(tui_result.is_ok(), "TUI role update should succeed");
    let tui_response = tui_result.unwrap();

    // Direct HTTP API role update
    let http_client = Client::new();
    let payload = serde_json::json!({
        "selected_role": test_role
    });

    let http_result = http_client
        .post(format!("{}/config/selected_role", TEST_SERVER_URL))
        .json(&payload)
        .send()
        .await;

    assert!(http_result.is_ok(), "HTTP role update should succeed");
    let http_response = http_result.unwrap();
    assert!(
        http_response.status().is_success(),
        "HTTP response should be successful"
    );

    let http_body: Value = http_response.json().await.unwrap();

    // Validate responses match
    assert_eq!(tui_response.status, http_body["status"].as_str().unwrap());
    assert_eq!(
        tui_response.config.selected_role.to_string(),
        http_body["config"]["selected_role"].as_str().unwrap()
    );
    assert_eq!(tui_response.config.selected_role.to_string(), *test_role);

    println!("✅ TUI and HTTP API role switching results are identical");
}

/// Test rolegraph retrieval parity
#[tokio::test]
#[serial]
async fn test_tui_vs_direct_api_rolegraph_parity() {
    if !is_server_running().await {
        println!("Server not running, skipping rolegraph parity test");
        return;
    }

    let test_role = "Default";

    // TUI client rolegraph
    let tui_client = ApiClient::new(TEST_SERVER_URL);
    let tui_result = tui_client.get_rolegraph_edges(Some(test_role)).await;
    assert!(tui_result.is_ok(), "TUI rolegraph should succeed");
    let tui_response = tui_result.unwrap();

    // Direct HTTP API rolegraph
    let http_client = Client::new();
    let http_result = http_client
        .get(format!("{}/rolegraph?role={}", TEST_SERVER_URL, test_role))
        .send()
        .await;

    assert!(http_result.is_ok(), "HTTP rolegraph should succeed");
    let http_response = http_result.unwrap();
    assert!(
        http_response.status().is_success(),
        "HTTP response should be successful"
    );

    let http_body: Value = http_response.json().await.unwrap();

    // Validate basic response structure
    assert_eq!(tui_response.status, http_body["status"].as_str().unwrap());
    assert_eq!(
        tui_response.nodes.len(),
        http_body["nodes"].as_array().unwrap().len()
    );
    assert_eq!(
        tui_response.edges.len(),
        http_body["edges"].as_array().unwrap().len()
    );

    // Validate first few nodes match (if any exist)
    for (i, tui_node) in tui_response.nodes.iter().take(3).enumerate() {
        let http_node = &http_body["nodes"][i];
        assert_eq!(tui_node.id, http_node["id"].as_u64().unwrap());
        assert_eq!(tui_node.label, http_node["label"].as_str().unwrap());
        assert_eq!(tui_node.rank, http_node["rank"].as_u64().unwrap());
    }

    println!("✅ TUI and HTTP API rolegraph responses are identical");
}

/// Test that search results are consistent across multiple identical queries
#[tokio::test]
#[serial]
async fn test_search_consistency_across_interfaces() {
    if !is_server_running().await {
        println!("Server not running, skipping consistency test");
        return;
    }

    let test_queries = vec!["rust", "api", "config"];
    let test_role = "Default";

    for query in test_queries {
        // Run same search multiple times through TUI client
        let tui_client = ApiClient::new(TEST_SERVER_URL);
        let search_query = SearchQuery {
            search_term: NormalizedTermValue::from(query),
            search_terms: None,
            operator: None,
            skip: Some(0),
            limit: Some(3),
            role: Some(RoleName::new(test_role)),
        };

        let mut results = Vec::new();
        for _ in 0..3 {
            let result = tui_client.search(&search_query).await;
            if let Ok(response) = result {
                results.push(response);
            }
        }

        if !results.is_empty() {
            // Verify all results are identical
            for i in 1..results.len() {
                assert_eq!(results[0].status, results[i].status);
                assert_eq!(results[0].total, results[i].total);
                assert_eq!(results[0].results.len(), results[i].results.len());

                // Verify document IDs are identical in same order
                for (j, doc) in results[0].results.iter().enumerate() {
                    assert_eq!(doc.id, results[i].results[j].id);
                    assert_eq!(doc.title, results[i].results[j].title);
                }
            }
            println!("✅ Search consistency verified for query: {}", query);
        }
    }
}

/// Test pagination consistency between TUI and direct API
#[tokio::test]
#[serial]
async fn test_pagination_parity() {
    if !is_server_running().await {
        println!("Server not running, skipping pagination parity test");
        return;
    }

    let query = "test";
    let role = "Default";
    let limit = 2;

    // Test first page
    let tui_client = ApiClient::new(TEST_SERVER_URL);
    let page1_query = SearchQuery {
        search_term: NormalizedTermValue::from(query),
        search_terms: None,
        operator: None,
        skip: Some(0),
        limit: Some(limit),
        role: Some(RoleName::new(role)),
    };

    let tui_page1 = tui_client.search(&page1_query).await;
    assert!(tui_page1.is_ok());

    let http_client = Client::new();
    let http_page1 = http_client
        .post(format!("{}/documents/search", TEST_SERVER_URL))
        .json(&page1_query)
        .send()
        .await
        .unwrap()
        .json::<Value>()
        .await
        .unwrap();

    // Test second page
    let page2_query = SearchQuery {
        search_term: NormalizedTermValue::from(query),
        search_terms: None,
        operator: None,
        skip: Some(limit),
        limit: Some(limit),
        role: Some(RoleName::new(role)),
    };

    let tui_page2 = tui_client.search(&page2_query).await;
    assert!(tui_page2.is_ok());

    let http_page2 = http_client
        .post(format!("{}/documents/search", TEST_SERVER_URL))
        .json(&page2_query)
        .send()
        .await
        .unwrap()
        .json::<Value>()
        .await
        .unwrap();

    // Verify pagination results match between TUI and HTTP
    let tui_p1 = tui_page1.unwrap();
    let tui_p2 = tui_page2.unwrap();

    assert_eq!(
        tui_p1.results.len(),
        http_page1["results"].as_array().unwrap().len()
    );
    assert_eq!(
        tui_p2.results.len(),
        http_page2["results"].as_array().unwrap().len()
    );

    // Verify pages contain different results (if enough data exists)
    if tui_p1.results.len() == limit && !tui_p2.results.is_empty() {
        let p1_ids: HashSet<String> = tui_p1.results.iter().map(|d| d.id.clone()).collect();
        let p2_ids: HashSet<String> = tui_p2.results.iter().map(|d| d.id.clone()).collect();
        assert!(
            p1_ids.is_disjoint(&p2_ids),
            "Pages should contain different documents"
        );
    }

    println!("✅ Pagination parity verified between TUI and HTTP API");
}

/// Test error handling consistency
#[tokio::test]
#[serial]
async fn test_error_handling_parity() {
    if !is_server_running().await {
        println!("Server not running, skipping error handling parity test");
        return;
    }

    let tui_client = ApiClient::new(TEST_SERVER_URL);
    let http_client = Client::new();

    // Test invalid role
    let invalid_query = SearchQuery {
        search_term: NormalizedTermValue::from("test"),
        search_terms: None,
        operator: None,
        skip: Some(0),
        limit: Some(5),
        role: Some(RoleName::new("NonExistentRole")),
    };

    let tui_result = tui_client.search(&invalid_query).await;
    let http_result = http_client
        .post(format!("{}/documents/search", TEST_SERVER_URL))
        .json(&invalid_query)
        .send()
        .await;

    // Both should handle the invalid role gracefully
    // The exact behavior may vary (error vs empty results), but both should be consistent
    if let Ok(tui_resp) = tui_result {
        if let Ok(http_resp) = http_result {
            let http_body: Value = http_resp.json().await.unwrap();

            // If both succeed, they should have consistent status
            assert_eq!(tui_resp.status, http_body["status"].as_str().unwrap());
        }
    }

    println!("✅ Error handling consistency verified");
}
