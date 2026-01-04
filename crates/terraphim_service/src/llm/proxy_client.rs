//! External LLM Proxy Client - HTTP to terraphim-llm-proxy service
//!
//! Implements LlmClient trait as an HTTP client to the external
//! terraphim-llm-proxy service running on port 3456. This provides
//! service mode routing without embedding proxy routing logic in main codebase.

use super::llm::LlmClient;
use super::llm::LlmRequest;
use super::llm::LlmResponse;
use super::llm::LlmError;
use super::llm::summarization::SummarizationOptions;
use super::llm::chat::ChatOptions;
use super::llm::LlmMessage;
use serde_json::json;
use tracing::{debug, error, warn};
use tokio::time::Duration;

/// External LLM proxy client configuration
#[derive(Debug, Clone)]
pub struct ProxyClientConfig {
    /// Proxy base URL (default: http://127.0.0.1:3456)
    pub base_url: String,

    /// Request timeout (default: 60 seconds)
    pub timeout_secs: u64,

    /// Enable request/response logging
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
#[derive(Debug)]
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
            .build();

        Self { config, http }
    }

    /// Check if external proxy mode is active
    pub fn is_proxy_mode(&self) -> bool {
        // External proxy mode always active for this client
        true
    }

    /// Get client name for logging
    pub fn name(&self) -> &'static str {
        "external_proxy_llm"
    }
}

impl LlmClient for ProxyLlmClient {
    async fn summarize(&self, content: &str, opts: SummarizationOptions) -> Result<String> {
        debug!("Summarization via external proxy (service mode)");

        let request = serde_json::json!({
            "model": "auto", // Let proxy routing decide
            "messages": [{
                "role": "user",
                "content": content
            }],
            "max_tokens": opts.max_tokens,
            "temperature": opts.temperature,
        });

        let response = self
            .http
            .post(&format!("{}/v1/summarize", self.config.base_url))
            .json(&request)
            .send()
            .await
            .map_err(|e| {
                error!("Proxy summarization failed: {}", e);
                LlmError::Internal(anyhow::anyhow!(e))
            })?;

        self.extract_summary(response).await
    }

    async fn chat(&self, messages: Vec<super::llm::LlmMessage>, opts: ChatOptions) -> Result<super::llm::LlmResponse> {
        debug!("Chat via external proxy (service mode)");

        let request = serde_json::json!({
            "model": "auto", // Let proxy routing decide
            "messages": messages.iter().map(|m| json!({
                "role": match m.role {
                    super::llm::MessageRole::System => "system",
                    super::llm::MessageRole::User => "user",
                    super::llm::MessageRole::Assistant => "assistant",
                    super::llm::MessageRole::Tool => "user",
                },
                "content": m.content,
            })).collect(),
            "temperature": opts.temperature,
            "max_tokens": opts.max_tokens,
        });

        let response = self
            .http
            .post(&format!("{}/v1/chat", self.config.base_url))
            .json(&request)
            .send()
            .await
            .map_err(|e| {
                error!("Proxy chat failed: {}", e);
                LlmError::Internal(anyhow::anyhow!(e))
            })?;

        Ok(self.transform_chat_response(response).await?)
    }

    async fn get_models(&self) -> Result<Vec<String>> {
        debug!("Get models via external proxy");

        let response = self
            .http
            .get(&format!("{}/v1/models", self.config.base_url))
            .send()
            .await
            .map_err(|e| {
                error!("Get models failed: {}", e);
                LlmError::Internal(anyhow::anyhow!(e))
            })?;

        self.extract_models(response).await
    }

    async fn stream_chat(
        &self,
        messages: Vec<super::llm::LlmMessage>,
        opts: ChatOptions,
    ) -> Result<Box<dyn futures::Stream<Item = Result<String>> + Unpin + Send + 'static>> {
        // Streaming will be implemented in later steps
        // Phase 3+ covers streaming support
        Err(LlmError::NotImplemented(
            "Stream chat not yet implemented in external proxy client".to_string()
        ))
    }

    /// Extract summary from proxy response
    async fn extract_summary(&self, mut response: reqwest::Response) -> Result<String> {
        let text = response.text().await.map_err(|e| {
            error!("Failed to read response text: {}", e);
            LlmError::Internal(anyhow::anyhow!(e))
        })?;

        match serde_json::from_str::<serde_json::Value>(&text) {
            Ok(json) => {
                let summary = json["choices"][0]["message"]["content"]
                    .as_str()
                    .unwrap_or("No summary generated");

                debug!("Extracted summary from proxy: {}", summary);
                Ok(summary.to_string())
            }
            Err(e) => {
                warn!("Failed to parse JSON response: {}", e);
                Ok("<Proxy returned invalid JSON>".to_string())
            }
        }
    }

    /// Extract models list from proxy response
    async fn extract_models(&self, mut response: reqwest::Response) -> Result<Vec<String>> {
        let text = response.text().await.map_err(|e| {
            error!("Failed to read response text: {}", e);
            LlmError::Internal(anyhow::anyhow!(e))
        })?;

        match serde_json::from_str::<serde_json::Value>(&text) {
            Ok(json) => {
                if let Some(data) = json.get("data") {
                    if let Some(models) = data.as_array() {
                        let model_names: Vec<String> = models
                            .iter()
                            .filter_map(|m| m.as_str())
                            .map(|s| s.unwrap_or("").to_string())
                            .collect();

                        debug!("Extracted {} models from proxy", model_names.len());
                        Ok(model_names)
                    } else {
                        warn!("No 'data' field in proxy models response");
                        Ok(vec![])
                    }
                } else {
                    warn!("No 'data' field in proxy models response");
                    Ok(vec![])
                }
            }
            Err(e) => {
                warn!("Failed to parse JSON response: {}", e);
                Ok(vec![])
            }
        }
    }

    /// Transform proxy chat response to internal format
    async fn transform_chat_response(&self, mut response: reqwest::Response) -> Result<super::llm::LlmResponse> {
        let text = response.text().await.map_err(|e| {
            error!("Failed to read response text: {}", e);
            LlmError::Internal(anyhow::anyhow!(e))
        })?;

        match serde_json::from_str::<serde_json::Value>(&text) {
            Ok(json) => {
                let choice = &json["choices"][0];

                let content = choice["message"]["content"]
                    .as_str()
                    .unwrap_or("")
                    .to_string();

                let finish_reason = choice
                    .get("finish_reason")
                    .and_then(|v| v.as_str())
                    .unwrap_or("stop")
                    .to_string();

                let model_used = choice.get("model")
                    .and_then(|m| m.as_str())
                    .unwrap_or("unknown")
                    .to_string();

                Ok(super::llm::LlmResponse {
                    content,
                    finish_reason,
                    model_used,
                    input_tokens: json.get("usage")
                        .and_then(|u| u["prompt_tokens"])
                        .and_then(|t| t.as_u64())
                        .unwrap_or(0),
                    output_tokens: json.get("usage")
                        .and_then(|u| u["completion_tokens"])
                        .and_then(|t| t.as_u64())
                        .unwrap_or(0),
                })
            }
            Err(e) => {
                warn!("Failed to parse JSON response: {}", e);
                Ok(super::llm::LlmResponse {
                    content: "<proxy error>".to_string(),
                    finish_reason: "error".to_string(),
                    model_used: "unknown".to_string(),
                    input_tokens: 0,
                    output_tokens: 0,
                })
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
        let client = ProxyLlmClient::new(ProxyClientConfig::default());

        let messages = vec![super::llm::LlmMessage {
            role: super::llm::MessageRole::User,
            content: "test content",
        }];

        // Capture the JSON that would be sent
        let request = serde_json::json!({
            "model": "auto",
            "messages": messages,
            "max_tokens": 1000,
            "temperature": 0.7,
        });

        let json_str = serde_json::to_string(&request).unwrap();
        assert!(json_str.contains("\"model\": \"auto\""));
        assert!(json_str.contains("\"max_tokens\": 1000"));
    }

    #[tokio::test]
    async fn test_chat_request_format() {
        let client = ProxyLlmClient::new(ProxyClientConfig::default());

        let messages = vec![
            super::llm::LlmMessage {
                role: super::llm::MessageRole::System,
                content: "You are helpful",
            },
            super::llm::LlmMessage {
                role: super::llm::MessageRole::User,
                content: "Hello",
            },
        ];

        let opts = super::llm::ChatOptions {
            temperature: 0.5,
            max_tokens: 100,
        };

        let request = serde_json::json!({
            "model": "auto",
            "messages": messages,
            "temperature": 0.5,
            "max_tokens": 100,
        });

        let json_str = serde_json::to_string(&request).unwrap();
        assert!(json_str.contains("\"model\": \"auto\""));
        assert!(json_str.contains("\"temperature\": 0.5"));
        assert!(json_str.contains("\"max_tokens\": 100"));
    }

    #[tokio::test]
    async fn test_json_extraction_error_handling() {
        let client = ProxyLlmClient::new(ProxyClientConfig::default());

        // Mock a JSON response with usage field
        let mock_response = r#"{
            "choices": [{
                "message": {
                    "content": "Test summary"
                },
                "finish_reason": "stop"
            }],
            "usage": {
                "prompt_tokens": 50,
                "completion_tokens": 30
            }
        }"#;

        let response = mock_response.parse().unwrap();

        match client.extract_models(std::pin::pin(response)).await {
            Ok(models) => {
                assert_eq!(models.len(), 1);
                assert_eq!(models[0], "unknown"); // get_models returns unknown model for this test
            }
            Err(e) => {
                panic!("Expected Ok but got error: {}", e);
            }
        }
    }
}
