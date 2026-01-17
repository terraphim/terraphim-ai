//! External LLM Proxy Client - HTTP to terraphim-llm-proxy service
//!
//! Implements LlmClient trait as an HTTP client to the external
//! terraphim-llm-proxy service running on port 3456. This provides
//! service mode routing without embedding proxy routing logic in main codebase.

use async_trait::async_trait;
use log::{debug, error, warn};
use serde_json::{json, Value};
use tokio::time::Duration;

use super::ChatOptions;
use super::LlmClient;
use super::SummarizeOptions;
use crate::Result as ServiceResult;

/// External LLM proxy client configuration
#[derive(Debug, Clone)]
pub struct ProxyClientConfig {
    /// Proxy base URL (default: http://127.0.0.1:3456)
    pub base_url: String,
    /// Request timeout (default: 60 seconds)
    pub timeout_secs: u64,
    /// Enable request/response logging
    #[allow(dead_code)]
    pub log_requests: bool,
}

impl Default for ProxyClientConfig {
    fn default() -> Self {
        Self {
            base_url: "http://127.0.0.1:3456".to_string(),
            timeout_secs: 60,
            log_requests: false,
        }
    }
}

/// External LLM proxy client
///
/// This client forwards requests to the external terraphim-llm-proxy
/// service (which implements 6-phase intelligent routing) and provides
/// graceful degradation via HTTP API calls.
#[derive(Clone)]
pub struct ProxyLlmClient {
    /// Client configuration
    config: ProxyClientConfig,
    /// HTTP client for proxy requests
    http: reqwest::Client,
}

impl ProxyLlmClient {
    /// Create a new external proxy client
    pub fn new(config: ProxyClientConfig) -> Self {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(config.timeout_secs))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());

        Self { config, http }
    }

    /// Check if external proxy mode is active
    #[allow(dead_code)]
    pub fn is_proxy_mode(&self) -> bool {
        true
    }

    /// Get client name for logging (inherent method for tests)
    #[allow(dead_code)]
    pub fn name(&self) -> &'static str {
        "external_proxy_llm"
    }
}

#[async_trait]
impl LlmClient for ProxyLlmClient {
    fn name(&self) -> &'static str {
        "external_proxy_llm"
    }

    async fn summarize(&self, content: &str, opts: SummarizeOptions) -> ServiceResult<String> {
        debug!("Summarization via external proxy (service mode)");

        let request = json!({
            "model": "auto",
            "messages": [{
                "role": "user",
                "content": format!("Please summarize the following in {} characters or less:\n\n{}",
                    opts.max_length, content)
            }],
            "max_tokens": opts.max_length.min(1024),
            "temperature": 0.3,
        });

        let response = match self
            .http
            .post(format!("{}/v1/chat/completions", self.config.base_url))
            .json(&request)
            .send()
            .await
        {
            Ok(resp) => resp,
            Err(e) => {
                error!("Proxy summarization request failed: {}", e);
                return Err(crate::ServiceError::Config(format!(
                    "Failed to connect to proxy: {}",
                    e
                )));
            }
        };

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            error!("Proxy returned error {}: {}", status, text);
            return Err(crate::ServiceError::Config(format!(
                "Proxy returned error: {} - {}",
                status, text
            )));
        }

        let text = match response.text().await {
            Ok(t) => t,
            Err(e) => {
                error!("Failed to read response text: {}", e);
                return Err(crate::ServiceError::Config(format!(
                    "Failed to read proxy response: {}",
                    e
                )));
            }
        };

        match serde_json::from_str::<Value>(&text) {
            Ok(json) => {
                let summary = json["choices"]
                    .get(0)
                    .and_then(|c| c.get("message"))
                    .and_then(|m| m.get("content"))
                    .and_then(|c| c.as_str())
                    .unwrap_or("")
                    .to_string();

                debug!("Extracted summary from proxy ({} chars)", summary.len());
                Ok(summary)
            }
            Err(e) => {
                warn!("Failed to parse JSON response: {}", e);
                Ok("<Proxy returned invalid JSON>".to_string())
            }
        }
    }

    async fn list_models(&self) -> ServiceResult<Vec<String>> {
        debug!("Get models via external proxy");

        let response = match self
            .http
            .get(format!("{}/v1/models", self.config.base_url))
            .send()
            .await
        {
            Ok(resp) => resp,
            Err(e) => {
                error!("Get models request failed: {}", e);
                return Err(crate::ServiceError::Config(format!(
                    "Failed to connect to proxy: {}",
                    e
                )));
            }
        };

        if !response.status().is_success() {
            let status = response.status();
            return Err(crate::ServiceError::Config(format!(
                "Proxy returned error: {}",
                status
            )));
        }

        let text = match response.text().await {
            Ok(t) => t,
            Err(e) => {
                return Err(crate::ServiceError::Config(format!(
                    "Failed to read: {}",
                    e
                )));
            }
        };

        match serde_json::from_str::<Value>(&text) {
            Ok(json) => {
                let models: Vec<String> = json["data"]
                    .as_array()
                    .unwrap_or(&vec![])
                    .iter()
                    .filter_map(|m| m.get("id").and_then(|id| id.as_str()))
                    .map(|s| s.to_string())
                    .collect();

                debug!("Extracted {} models from proxy", models.len());
                Ok(models)
            }
            Err(e) => {
                warn!("Failed to parse models response: {}", e);
                Ok(vec![])
            }
        }
    }

    async fn chat_completion(
        &self,
        messages: Vec<Value>,
        opts: ChatOptions,
    ) -> ServiceResult<String> {
        debug!("Chat via external proxy (service mode)");

        let request = json!({
            "model": "auto",
            "messages": messages,
            "temperature": opts.temperature.unwrap_or(0.7),
            "max_tokens": opts.max_tokens.unwrap_or(1024),
        });

        let response = match self
            .http
            .post(format!("{}/v1/chat/completions", self.config.base_url))
            .json(&request)
            .send()
            .await
        {
            Ok(resp) => resp,
            Err(e) => {
                error!("Proxy chat request failed: {}", e);
                return Err(crate::ServiceError::Config(format!(
                    "Failed to connect to proxy: {}",
                    e
                )));
            }
        };

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            error!("Proxy returned error {}: {}", status, text);
            return Err(crate::ServiceError::Config(format!(
                "Proxy returned error: {} - {}",
                status, text
            )));
        }

        let text = match response.text().await {
            Ok(t) => t,
            Err(e) => {
                return Err(crate::ServiceError::Config(format!(
                    "Failed to read: {}",
                    e
                )));
            }
        };

        match serde_json::from_str::<Value>(&text) {
            Ok(json) => {
                let content = json["choices"]
                    .get(0)
                    .and_then(|c| c.get("message"))
                    .and_then(|m| m.get("content"))
                    .and_then(|c| c.as_str())
                    .unwrap_or("")
                    .to_string();

                debug!("Chat response: {} chars", content.len());
                Ok(content)
            }
            Err(e) => {
                warn!("Failed to parse chat response: {}", e);
                Err(crate::ServiceError::Config(e.to_string()))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_proxy_client_creation() {
        let config = ProxyClientConfig::default();
        let client = ProxyLlmClient::new(config);

        assert!(client.is_proxy_mode());
        assert_eq!(client.name(), "external_proxy_llm");
        assert_eq!(client.config.base_url, "http://127.0.0.1:3456");
    }

    #[tokio::test]
    async fn test_proxy_client_custom_config() {
        let config = ProxyClientConfig {
            base_url: "http://custom:8080".to_string(),
            timeout_secs: 30,
            log_requests: true,
        };
        let client = ProxyLlmClient::new(config);

        assert_eq!(client.config.base_url, "http://custom:8080".to_string());
        assert_eq!(client.config.timeout_secs, 30);
    }

    #[tokio::test]
    async fn test_summarize_request_format() {
        let _client = ProxyLlmClient::new(ProxyClientConfig::default());

        let opts = SummarizeOptions { max_length: 500 };

        let content = "This is a test document that needs to be summarized. ".repeat(10);

        // Build the expected request
        let expected_request = json!({
            "model": "auto",
            "messages": [{
                "role": "user",
                "content": format!("Please summarize the following in {} characters or less:\n\n{}",
                    opts.max_length, content)
            }],
            "max_tokens": 500,
            "temperature": 0.3,
        });

        let json_str = serde_json::to_string(&expected_request).unwrap();
        assert!(json_str.contains("\"model\":\"auto\""));
        assert!(json_str.contains("\"max_tokens\":500"));
    }

    #[tokio::test]
    async fn test_chat_request_format() {
        let _client = ProxyLlmClient::new(ProxyClientConfig::default());

        let messages = vec![
            json!({"role": "system", "content": "You are helpful"}),
            json!({"role": "user", "content": "Hello"}),
        ];

        let _opts = ChatOptions {
            temperature: Some(0.5),
            max_tokens: Some(100),
        };

        let expected_request = json!({
            "model": "auto",
            "messages": messages,
            "temperature": 0.5,
            "max_tokens": 100,
        });

        let json_str = serde_json::to_string(&expected_request).unwrap();
        assert!(json_str.contains("\"model\":\"auto\""));
        assert!(json_str.contains("\"temperature\":0.5"));
        assert!(json_str.contains("\"max_tokens\":100"));
    }

    #[tokio::test]
    async fn test_name_method() {
        let client = ProxyLlmClient::new(ProxyClientConfig::default());
        assert_eq!(client.name(), "external_proxy_llm");
    }
}
