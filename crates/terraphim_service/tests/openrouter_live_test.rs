#![cfg(feature = "openrouter")]

use std::env;
use terraphim_service::openrouter::OpenRouterService;

// Run only when OPENROUTER_API_KEY is present in env.
#[tokio::test]
async fn live_chat_completion_smoke() {
    let api_key = match env::var("OPENROUTER_API_KEY") {
        Ok(v) if v.starts_with("sk-or-") => v,
        _ => {
            eprintln!("OPENROUTER_API_KEY not set; skipping live test");
            return;
        }
    };

    let client = OpenRouterService::new(&api_key, "openai/gpt-3.5-turbo")
        .expect("client init");

    let reply = client
        .chat_completion(vec![serde_json::json!({"role":"user","content":"Say 'pong'"})], Some(64), Some(0.2))
        .await
        .expect("live chat call should succeed");

    assert!(!reply.trim().is_empty());
}


