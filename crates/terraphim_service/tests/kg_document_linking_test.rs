/// Test that verifies KG search returns links to original documents that formed KG nodes
/// For example, queries for "graph embeddings", "graph", "knowledge graph based embeddings"
/// should return links to @docs/src/kg/terraphim-graph.md and other relevant KG source files
///
/// This test is designed to work with a running Terraphim server on localhost:8000
/// Run with: RUST_LOG=info cargo test test_kg_search_with_running_server -- --ignored --nocapture
#[tokio::test]
#[ignore]
async fn test_kg_search_with_running_server() {
    let client = reqwest::Client::new();
    let server_url = "http://127.0.0.1:8000";

    // Test queries that should match KG nodes and return links to source documents
    let test_queries = vec!["graph embeddings", "terraphim-graph", "knowledge graph"];

    for query in &test_queries {
        println!("🔍 Testing query: '{}'", query);

        let search_request = serde_json::json!({
            "search_term": query,
            "role": "Terraphim Engineer"
        });

        let response = client
            .post(format!("{}/documents/search", server_url))
            .header("Content-Type", "application/json")
            .json(&search_request)
            .send()
            .await
            .expect("Failed to send search request");

        assert!(
            response.status().is_success(),
            "Search request failed for query: '{}'",
            query
        );

        let search_response: serde_json::Value = response
            .json()
            .await
            .expect("Failed to parse search response");

        assert_eq!(
            search_response["status"], "success",
            "Search returned error status"
        );

        let results = search_response["results"]
            .as_array()
            .expect("Search results should be an array");

        assert!(
            !results.is_empty(),
            "No search results found for query: '{}'",
            query
        );

        println!("📋 Found {} search results for '{}'", results.len(), query);

        let mut found_kg_source_link = false;
        let mut found_kg_links_in_content = false;

        let kg_source_files = vec![
            "docs/src/kg/terraphim-graph.md",
            "docs/src/kg/knowledge-graph.md",
            "docs/src/kg/knowledge-graph-system.md",
        ];

        for (i, result) in results.iter().enumerate() {
            let title = result["title"].as_str().unwrap_or("unknown");
            let url = result["url"].as_str().unwrap_or("unknown");
            let body = result["body"].as_str().unwrap_or("");

            println!("📄 Result #{}: '{}' (URL: {})", i + 1, title, url);

            // Check if the document URL references one of the KG source files
            for kg_file in &kg_source_files {
                if url.contains(kg_file) {
                    found_kg_source_link = true;
                    println!("   ✅ Found link to KG source file: {}", kg_file);
                    break;
                }
            }

            // Check the document body for KG links in markdown format [term](kg:concept)
            if body.contains("](kg:") {
                found_kg_links_in_content = true;

                let kg_links: Vec<&str> = body
                    .split("[")
                    .filter_map(|s| s.find("](kg:").map(|closing| &s[..closing]))
                    .take(3) // Limit to first 3 for brevity
                    .collect();

                if !kg_links.is_empty() {
                    println!(
                        "   🔗 Document contains KG links: [{}](kg:...)",
                        kg_links.join("], [")
                    );
                }
            }
        }

        // Assert that we found at least one document that links back to KG source files
        assert!(
            found_kg_source_link,
            "Query '{}' should return documents that link back to KG source files like @docs/src/kg/terraphim-graph.md",
            query
        );

        // Also assert that we found KG links in the content (validates KG preprocessing works)
        assert!(
            found_kg_links_in_content,
            "Query '{}' should return documents with KG links in content like [term](kg:concept)",
            query
        );

        println!(
            "✅ Query '{}' successfully returned both KG source documents and KG-linked content\n",
            query
        );
    }

    println!("🎉 All KG document linking tests passed! KG search successfully returns:");
    println!("   1. Original KG source documents (e.g., @docs/src/kg/terraphim-graph.md)");
    println!("   2. Documents with KG links in content (e.g., [graph](kg:terraphim-graph))");
}
