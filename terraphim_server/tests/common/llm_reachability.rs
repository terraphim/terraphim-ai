//! Shared LLM and service reachability helpers for integration tests.
//!
//! Use these helpers instead of `#[ignore]` for tests that need LLM services.
//! They perform a fast reachability check (2-second deadline) and return early
//! with a clear skip message when the service is unavailable.
//!
//! # Usage
//!
//! ```rust,ignore
//! #[tokio::test]
//! async fn test_with_llm_proxy() {
//!     if !common::llm_reachability::require_llm_proxy().await {
//!         return;
//!     }
//!     // test body
//! }
//!
//! #[tokio::test]
//! async fn test_with_ollama() {
//!     if !common::llm_reachability::require_ollama().await {
//!         return;
//!     }
//!     // test body
//! }
//! ```

use std::env;
use std::time::Duration;

/// Returns the LLM proxy base URL from `LLM_PROXY_URL` env var.
/// Falls back to the terraphim-llm-proxy default.
pub fn llm_proxy_url() -> String {
    env::var("LLM_PROXY_URL").unwrap_or_else(|_| "http://127.0.0.1:3456".to_string())
}

/// Returns the Ollama base URL from `OLLAMA_BASE_URL` env var.
pub fn ollama_url() -> String {
    env::var("OLLAMA_BASE_URL").unwrap_or_else(|_| "http://127.0.0.1:11434".to_string())
}

/// Check if the terraphim-llm-proxy is reachable within 2 seconds.
///
/// Returns `true` if reachable. Returns `false` and prints a skip message
/// if the proxy is unreachable or times out.
///
/// Reads `LLM_PROXY_URL` from the environment (default: `http://127.0.0.1:3456`).
pub async fn require_llm_proxy() -> bool {
    let url = llm_proxy_url();
    check_http_head(&url).await
}

/// Check if Ollama is reachable within 2 seconds.
///
/// Returns `true` if reachable. Returns `false` and prints a skip message
/// if Ollama is unreachable or times out.
///
/// Reads `OLLAMA_BASE_URL` from the environment (default: `http://127.0.0.1:11434`).
pub async fn require_ollama() -> bool {
    let base = ollama_url();
    let health_url = format!("{}/api/tags", base);
    check_http_get(&health_url).await
}

/// Check if Ollama is reachable AND the specified model is loaded.
///
/// Returns `true` if Ollama responds with HTTP 200 from `/api/tags` AND
/// the specified model appears in the models list.
/// Returns `false` with a skip message if Ollama is unreachable, times out,
/// or the model is not available.
///
/// # Arguments
/// * `model` - The exact model name to check (e.g., `"llama3.2:3b"`)
///
/// Reads `OLLAMA_BASE_URL` from the environment (default: `http://127.0.0.1:11434`).
pub async fn require_ollama_with_model(model: &str) -> bool {
    let base = ollama_url();
    let tags_url = format!("{}/api/tags", base);

    let client = match reqwest::Client::builder()
        .timeout(Duration::from_secs(2))
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            eprintln!("skip: failed to build HTTP client: {}", e);
            return false;
        }
    };

    let body: serde_json::Value = match client.get(&tags_url).send().await {
        Ok(resp) if resp.status().is_success() => match resp.json().await {
            Ok(v) => v,
            Err(e) => {
                eprintln!("skip: failed to parse Ollama /api/tags response: {}", e);
                return false;
            }
        },
        Ok(resp) => {
            eprintln!("skip: Ollama /api/tags returned HTTP {}", resp.status());
            return false;
        }
        Err(e) => {
            eprintln!("skip: Ollama unreachable at {}: {}", tags_url, e);
            return false;
        }
    };

    let has_model = body
        .get("models")
        .and_then(|m| m.as_array())
        .map(|models| {
            models.iter().any(|m| {
                m.get("name")
                    .and_then(|n| n.as_str())
                    .map(|n| n == model)
                    .unwrap_or(false)
            })
        })
        .unwrap_or(false);

    if has_model {
        true
    } else {
        eprintln!("skip: model '{}' not loaded in Ollama at {}", model, base);
        false
    }
}

/// Perform an HTTP HEAD request with a 2-second deadline.
/// Treats any HTTP response (even 4xx/5xx) as "reachable" — only connection
/// failures or timeouts count as "unreachable".
async fn check_http_head(url: &str) -> bool {
    let client = match reqwest::Client::builder()
        .timeout(Duration::from_secs(2))
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            eprintln!("skip: failed to build HTTP client: {}", e);
            return false;
        }
    };

    match client.head(url).send().await {
        Ok(_) => true,
        Err(e) => {
            eprintln!("skip: LLM proxy unreachable at {}: {}", url, e);
            false
        }
    }
}

/// Perform an HTTP GET request with a 2-second deadline.
/// Returns `true` if the server responds with a success status.
async fn check_http_get(url: &str) -> bool {
    let client = match reqwest::Client::builder()
        .timeout(Duration::from_secs(2))
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            eprintln!("skip: failed to build HTTP client: {}", e);
            return false;
        }
    };

    match client.get(url).send().await {
        Ok(resp) if resp.status().is_success() => true,
        Ok(resp) => {
            eprintln!("skip: service at {} returned HTTP {}", url, resp.status());
            false
        }
        Err(e) => {
            eprintln!("skip: LLM proxy unreachable at {}: {}", url, e);
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn llm_proxy_url_defaults_to_port_3456() {
        // Temporarily remove env var to test default
        let original = env::var("LLM_PROXY_URL").ok();
        unsafe {
            env::remove_var("LLM_PROXY_URL");
        }
        assert_eq!(llm_proxy_url(), "http://127.0.0.1:3456");
        // Restore
        if let Some(v) = original {
            unsafe {
                env::set_var("LLM_PROXY_URL", v);
            }
        }
    }

    #[test]
    fn llm_proxy_url_reads_env_var() {
        unsafe {
            env::set_var("LLM_PROXY_URL", "http://example.com:9999");
        }
        assert_eq!(llm_proxy_url(), "http://example.com:9999");
        unsafe {
            env::remove_var("LLM_PROXY_URL");
        }
    }

    #[test]
    fn ollama_url_defaults_to_port_11434() {
        let original = env::var("OLLAMA_BASE_URL").ok();
        unsafe {
            env::remove_var("OLLAMA_BASE_URL");
        }
        assert_eq!(ollama_url(), "http://127.0.0.1:11434");
        if let Some(v) = original {
            unsafe {
                env::set_var("OLLAMA_BASE_URL", v);
            }
        }
    }

    #[tokio::test]
    async fn require_llm_proxy_returns_false_for_closed_port() {
        unsafe {
            env::set_var("LLM_PROXY_URL", "http://127.0.0.1:59998");
        }
        // Nothing listens on 59998 — must return false within 3 seconds
        let result = require_llm_proxy().await;
        assert!(!result, "expected false for closed port");
        unsafe {
            env::remove_var("LLM_PROXY_URL");
        }
    }

    #[tokio::test]
    async fn require_ollama_returns_false_for_closed_port() {
        unsafe {
            env::set_var("OLLAMA_BASE_URL", "http://127.0.0.1:59997");
        }
        let result = require_ollama().await;
        assert!(!result, "expected false for closed port");
        unsafe {
            env::remove_var("OLLAMA_BASE_URL");
        }
    }

    #[tokio::test]
    async fn require_ollama_with_model_returns_false_for_closed_port() {
        unsafe {
            env::set_var("OLLAMA_BASE_URL", "http://127.0.0.1:59996");
        }
        let result = require_ollama_with_model("llama3.2:3b").await;
        assert!(!result, "expected false when Ollama is unreachable");
        unsafe {
            env::remove_var("OLLAMA_BASE_URL");
        }
    }
}
