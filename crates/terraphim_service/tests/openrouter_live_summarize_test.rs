#![cfg(feature = "openrouter")]

use std::env;
use terraphim_service::openrouter::OpenRouterService;

// Run only when OPENROUTER_API_KEY is present in env.
#[tokio::test]
async fn live_generate_summary_smoke() {
    let api_key = match env::var("OPENROUTER_API_KEY") {
        Ok(v) if v.starts_with("sk-or-") => v,
        _ => {
            eprintln!("OPENROUTER_API_KEY not set; skipping live test");
            return;
        }
    };

    let model = env::var("OPENROUTER_MODEL").unwrap_or_else(|_| "openai/gpt-4-turbo".to_string());
    let client = OpenRouterService::new(&api_key, &model).expect("client init");

    let content = "Rust is a systems programming language that focuses on safety and performance. It achieves memory safety without garbage collection via ownership and borrowing.";
    let summary = client
        .generate_summary(content, 160)
        .await
        .expect("summary call should succeed");

    assert!(!summary.trim().is_empty());
}
