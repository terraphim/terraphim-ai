use serde_json::json;
use std::process::{Child, Command};
use std::time::Duration;

/// Debug test to understand why summarization isn't working
#[tokio::test]
async fn test_debug_summarization_flow() {
    if std::env::var("RUN_DEBUG_SUMMARIZATION_TEST")
        .map(|v| v != "1" && !v.eq_ignore_ascii_case("true"))
        .unwrap_or(true)
    {
        eprintln!("Skipping: set RUN_DEBUG_SUMMARIZATION_TEST=1 to run this debug test");
        return;
    }
    println!("🧪 Debug Summarization Flow Test");
    println!("=================================");

    // Start the server in the background
    let server_process = start_test_server().await;

    // Wait for server to start
    tokio::time::sleep(Duration::from_secs(5)).await;

    // Test a simple search
    println!("\n🔍 Testing search: 'tokio' with role: 'Rust Engineer'");

    match test_search_and_debug_summarization("tokio", "Rust Engineer").await {
        Ok(documents) => {
            println!("📊 Search returned {} documents", documents.len());

            for (i, doc) in documents.iter().enumerate().take(3) {
                println!("\n📄 Document {}: {}", i + 1, doc.id);
                println!("  Title: {}", doc.title);
                println!("  Body length: {} chars", doc.body.len());
                println!("  Has description: {}", doc.description.is_some());
                println!("  Has summarization: {}", doc.summarization.is_some());

                if let Some(desc) = &doc.description {
                    println!("  Description: {}...", &desc[..desc.len().min(100)]);
                }

                if let Some(summ) = &doc.summarization {
                    println!("  Summarization: {}...", &summ[..summ.len().min(100)]);
                }

                // Check if this document should get AI summarization
                let should_summarize = should_generate_ai_summary(doc);
                println!("  Should generate AI summary: {}", should_summarize);

                if !should_summarize {
                    if doc.body.trim().len() < 200 {
                        println!(
                            "    ❌ Body too short: {} chars < 200",
                            doc.body.trim().len()
                        );
                    }
                    if let Some(ref description) = doc.description {
                        if description.len() > 100 && !description.ends_with("...") {
                            println!(
                                "    ❌ Already has good description: {} chars",
                                description.len()
                            );
                        }
                    }
                    if doc.body.len() > 8000 {
                        println!("    ❌ Body too long: {} chars > 8000", doc.body.len());
                    }
                }
            }
        }
        Err(e) => {
            println!("❌ Search failed: {}", e);
        }
    }

    // Clean up - stop the server
    if let Some(mut process) = server_process {
        let _ = process.kill();
        let _ = process.wait();
        println!("🧹 Test server stopped");
    }
}

fn should_generate_ai_summary(document: &terraphim_types::Document) -> bool {
    // Don't enhance if the document body is too short to summarize meaningfully
    if document.body.trim().len() < 200 {
        return false;
    }

    // Don't enhance if we already have a high-quality description
    if let Some(ref description) = document.description {
        // If the description is substantial and doesn't look like a simple excerpt, keep it
        if description.len() > 100 && !description.ends_with("...") {
            return false;
        }
    }

    // Don't enhance very large documents (cost control)
    if document.body.len() > 8000 {
        return false;
    }

    // Good candidates for AI summarization
    true
}

async fn start_test_server() -> Option<Child> {
    println!("🚀 Starting test server...");

    // Start the server
    let server_result = Command::new("cargo")
        .args([
            "run",
            "--release",
            "-p",
            "terraphim_server",
            "--bin",
            "terraphim_server",
            "--",
            "--config",
            "terraphim_server/default/combined_roles_config.json",
        ])
        .current_dir(".")
        .spawn();

    match server_result {
        Ok(process) => {
            println!("✅ Test server started (PID: {})", process.id());
            Some(process)
        }
        Err(e) => {
            println!("❌ Failed to start server: {}", e);
            None
        }
    }
}

async fn test_search_and_debug_summarization(
    search_term: &str,
    role: &str,
) -> Result<Vec<terraphim_types::Document>, Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();
    let search_payload = json!({
        "search_term": search_term,
        "role": role
    });

    let response = client
        .post("http://localhost:8000/documents/search")
        .header("Content-Type", "application/json")
        .json(&search_payload)
        .send()
        .await?;

    if !response.status().is_success() {
        return Err(format!("Search request failed with status: {}", response.status()).into());
    }

    let response_json: serde_json::Value = response.json().await?;
    let empty_vec = vec![];
    let results = response_json
        .get("results")
        .unwrap()
        .as_array()
        .unwrap_or(&empty_vec);

    let mut documents = Vec::new();

    for result in results.iter() {
        if let Ok(doc) = serde_json::from_value::<terraphim_types::Document>(result.clone()) {
            documents.push(doc);
        }
    }

    Ok(documents)
}
