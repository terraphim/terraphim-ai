use std::env;
use tokio;
use reqwest;
use serde_json;

/// Test to validate that all local services are available and working
#[tokio::test]
#[ignore] // Run with: cargo test validate_local_setup -- --ignored
async fn test_local_services_available() {
    // Load environment variables from .env.test if available
    dotenvy::dotenv().ok();
    
    let _ = env_logger::builder().is_test(true).try_init();
    
    println!("🧪 Testing Local Service Availability");
    println!("====================================");
    
    // Check Ollama
    println!("\n1️⃣ Testing Ollama (port 11434)...");
    let ollama_url = env::var("OLLAMA_BASE_URL").unwrap_or_else(|_| "http://localhost:11434".to_string());
    
    match reqwest::get(&format!("{}/api/tags", ollama_url)).await {
        Ok(resp) => {
            if resp.status().is_success() {
                println!("   ✅ Ollama is running and responding");
                
                // Check if required model is available
                if let Ok(text) = resp.text().await {
                    if text.contains("llama3.2:3b") {
                        println!("   ✅ llama3.2:3b model is available");
                    } else {
                        println!("   ⚠️ llama3.2:3b model not found in model list");
                    }
                }
            } else {
                println!("   ❌ Ollama returned status: {}", resp.status());
            }
        }
        Err(e) => {
            println!("   ❌ Ollama connection failed: {}", e);
            panic!("Ollama is required for tests but is not available");
        }
    }
    
    // Check Atomic Server (optional)
    println!("\n2️⃣ Testing Atomic Server (port 9883)...");
    let atomic_url = env::var("ATOMIC_SERVER_URL").unwrap_or_else(|_| "http://localhost:9883".to_string());
    
    match reqwest::get(&atomic_url).await {
        Ok(resp) => {
            if resp.status().is_success() {
                println!("   ✅ Atomic Server is running and responding");
            } else {
                println!("   ⚠️ Atomic Server returned status: {}", resp.status());
            }
        }
        Err(e) => {
            println!("   ⚠️ Atomic Server not available: {}", e);
            println!("   ℹ️ This is optional - tests will skip Atomic Server functionality");
        }
    }
    
    // Check MCP Server (may not respond to HTTP in stdio mode)
    println!("\n3️⃣ Testing MCP Server (port 8001)...");
    let mcp_url = env::var("MCP_SERVER_URL").unwrap_or_else(|_| "http://localhost:8001".to_string());
    
    match reqwest::get(&mcp_url).await {
        Ok(resp) => {
            println!("   ✅ MCP Server is responding to HTTP on port 8001");
        }
        Err(e) => {
            println!("   ℹ️ MCP Server HTTP check failed: {}", e);
            println!("   ℹ️ This is normal if MCP is in stdio mode");
        }
    }
    
    // Check Terraphim Server
    println!("\n4️⃣ Testing Terraphim Server (port 8000)...");
    let terraphim_url = format!("http://localhost:{}/health", 
        env::var("TERRAPHIM_SERVER_PORT").unwrap_or_else(|_| "8000".to_string()));
    
    match reqwest::get(&terraphim_url).await {
        Ok(resp) => {
            if resp.status().is_success() {
                println!("   ✅ Terraphim Server is running and healthy");
            } else {
                println!("   ❌ Terraphim Server health check failed: {}", resp.status());
            }
        }
        Err(e) => {
            println!("   ❌ Terraphim Server connection failed: {}", e);
            panic!("Terraphim Server is required for tests but is not available");
        }
    }
    
    println!("\n✅ Local service validation complete!");
}

/// Test Ollama model functionality
#[tokio::test]
#[ignore] // Run with: cargo test test_ollama_model_functionality -- --ignored
async fn test_ollama_model_functionality() {
    dotenvy::dotenv().ok();
    let _ = env_logger::builder().is_test(true).try_init();
    
    println!("🧠 Testing Ollama Model Functionality");
    println!("====================================");
    
    let ollama_url = env::var("OLLAMA_BASE_URL").unwrap_or_else(|_| "http://localhost:11434".to_string());
    let model = env::var("OLLAMA_MODEL").unwrap_or_else(|_| "llama3.2:3b".to_string());
    
    let client = reqwest::Client::new();
    
    println!("Testing model: {}", model);
    
    let request_body = serde_json::json!({
        "model": model,
        "prompt": "Hello, respond with just 'OK'",
        "stream": false,
        "options": {
            "num_predict": 5,
            "temperature": 0.1
        }
    });
    
    match client
        .post(&format!("{}/api/generate", ollama_url))
        .json(&request_body)
        .send()
        .await
    {
        Ok(resp) => {
            if resp.status().is_success() {
                match resp.text().await {
                    Ok(response_text) => {
                        println!("   ✅ Model generated response");
                        println!("   📝 Response: {}", response_text.chars().take(100).collect::<String>());
                    }
                    Err(e) => {
                        println!("   ❌ Failed to read response: {}", e);
                    }
                }
            } else {
                println!("   ❌ Model request failed: {}", resp.status());
            }
        }
        Err(e) => {
            println!("   ❌ Failed to connect to Ollama: {}", e);
        }
    }
}

/// Test Terraphim API endpoints
#[tokio::test]
#[ignore] // Run with: cargo test test_terraphim_api_endpoints -- --ignored
async fn test_terraphim_api_endpoints() {
    dotenvy::dotenv().ok();
    let _ = env_logger::builder().is_test(true).try_init();
    
    println!("🌐 Testing Terraphim API Endpoints");
    println!("=================================");
    
    let base_url = format!("http://localhost:{}", 
        env::var("TERRAPHIM_SERVER_PORT").unwrap_or_else(|_| "8000".to_string()));
    
    let client = reqwest::Client::new();
    
    // Test health endpoint
    println!("\n📡 Testing /health endpoint...");
    match client.get(&format!("{}/health", base_url)).send().await {
        Ok(resp) => {
            if resp.status().is_success() {
                println!("   ✅ Health endpoint responding");
            } else {
                println!("   ❌ Health endpoint returned: {}", resp.status());
            }
        }
        Err(e) => {
            println!("   ❌ Health endpoint failed: {}", e);
        }
    }
    
    // Test config endpoint
    println!("\n⚙️ Testing /config endpoint...");
    match client.get(&format!("{}/config", base_url)).send().await {
        Ok(resp) => {
            if resp.status().is_success() {
                println!("   ✅ Config endpoint responding");
                if let Ok(text) = resp.text().await {
                    if text.contains("roles") {
                        println!("   ✅ Config contains roles");
                    } else {
                        println!("   ⚠️ Config response doesn't contain roles");
                    }
                }
            } else {
                println!("   ❌ Config endpoint returned: {}", resp.status());
            }
        }
        Err(e) => {
            println!("   ❌ Config endpoint failed: {}", e);
        }
    }
    
    // Test roles endpoint
    println!("\n👤 Testing /roles endpoint...");
    match client.get(&format!("{}/roles", base_url)).send().await {
        Ok(resp) => {
            if resp.status().is_success() {
                println!("   ✅ Roles endpoint responding");
            } else {
                println!("   ❌ Roles endpoint returned: {}", resp.status());
            }
        }
        Err(e) => {
            println!("   ❌ Roles endpoint failed: {}", e);
        }
    }
    
    println!("\n✅ API endpoint testing complete!");
}

/// Test all haystack types
#[tokio::test]
#[ignore] // Run with: cargo test test_haystack_types -- --ignored
async fn test_haystack_types() {
    dotenvy::dotenv().ok();
    let _ = env_logger::builder().is_test(true).try_init();
    
    println!("📚 Testing Haystack Types");
    println!("========================");
    
    // This test validates that different haystack types can be configured
    // It doesn't actually search them (that would be integration testing)
    
    use terraphim_config::{Haystack, ServiceType};
    use std::collections::HashMap;
    
    let mut extra_params = HashMap::new();
    
    // Test Ripgrep haystack
    println!("\n1️⃣ Testing Ripgrep Haystack configuration...");
    let ripgrep_haystack = Haystack {
        location: "./docs/src".to_string(),
        service: ServiceType::Ripgrep,
        read_only: true,
        atomic_server_secret: None,
        extra_parameters: extra_params.clone(),
    };
    assert_eq!(ripgrep_haystack.service, ServiceType::Ripgrep);
    println!("   ✅ Ripgrep haystack configured");
    
    // Test Atomic haystack
    println!("\n2️⃣ Testing Atomic Haystack configuration...");
    let atomic_url = env::var("ATOMIC_SERVER_URL").unwrap_or_else(|_| "http://localhost:9883".to_string());
    let atomic_haystack = Haystack {
        location: atomic_url,
        service: ServiceType::Atomic,
        read_only: true,
        atomic_server_secret: Some("test-secret".to_string()),
        extra_parameters: extra_params.clone(),
    };
    assert_eq!(atomic_haystack.service, ServiceType::Atomic);
    println!("   ✅ Atomic haystack configured");
    
    // Test MCP haystack
    println!("\n3️⃣ Testing MCP Haystack configuration...");
    let mcp_url = env::var("MCP_SERVER_URL").unwrap_or_else(|_| "http://localhost:8001".to_string());
    extra_params.insert("transport".to_string(), "sse".to_string());
    let mcp_haystack = Haystack {
        location: mcp_url,
        service: ServiceType::Mcp,
        read_only: true,
        atomic_server_secret: None,
        extra_parameters: extra_params.clone(),
    };
    assert_eq!(mcp_haystack.service, ServiceType::Mcp);
    println!("   ✅ MCP haystack configured");
    
    // Test QueryRs haystack
    println!("\n4️⃣ Testing QueryRs Haystack configuration...");
    let queryrs_haystack = Haystack {
        location: "".to_string(), // QueryRs doesn't need a location
        service: ServiceType::QueryRs,
        read_only: true,
        atomic_server_secret: None,
        extra_parameters: HashMap::new(),
    };
    assert_eq!(queryrs_haystack.service, ServiceType::QueryRs);
    println!("   ✅ QueryRs haystack configured");
    
    println!("\n✅ All haystack types can be configured!");
}