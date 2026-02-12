//! HTTP client for terraphim-llm-proxy.

use crate::tools::ToolCall;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

/// Configuration for the proxy client.
#[derive(Debug, Clone)]
pub struct ProxyClientConfig {
    /// Base URL for the proxy (e.g., "http://127.0.0.1:3456")
    pub base_url: String,
    /// API key for authentication
    pub api_key: String,
    /// Request timeout in milliseconds
    pub timeout_ms: u64,
    /// Model override (optional)
    pub model: Option<String>,
    /// Retry backoff after failure in seconds
    pub retry_after_secs: u64,
}

impl Default for ProxyClientConfig {
    fn default() -> Self {
        Self {
            base_url: "http://127.0.0.1:3456".to_string(),
            api_key: String::new(),
            timeout_ms: 60_000,
            model: None,
            retry_after_secs: 60,
        }
    }
}

/// HTTP client for communicating with terraphim-llm-proxy.
pub struct ProxyClient {
    config: ProxyClientConfig,
    http: Client,
    healthy: AtomicBool,
    last_failure: Mutex<Option<Instant>>,
}

impl ProxyClient {
    /// Create a new proxy client.
    pub fn new(config: ProxyClientConfig) -> Self {
        let http = Client::builder()
            .timeout(Duration::from_millis(config.timeout_ms))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            config,
            http,
            healthy: AtomicBool::new(true),
            last_failure: Mutex::new(None),
        }
    }

    /// Check if the proxy is considered healthy.
    /// Returns false if there was a recent failure and backoff hasn't elapsed.
    pub fn is_available(&self) -> bool {
        if self.healthy.load(Ordering::SeqCst) {
            return true;
        }

        // Check if backoff has elapsed
        let last_failure = self.last_failure.lock().unwrap();
        if let Some(failure_time) = *last_failure {
            let elapsed = failure_time.elapsed();
            if elapsed >= Duration::from_secs(self.config.retry_after_secs) {
                // Backoff elapsed, mark as healthy again
                drop(last_failure);
                self.healthy.store(true, Ordering::SeqCst);
                return true;
            }
        }

        false
    }

    /// Mark the proxy as unhealthy.
    fn mark_unhealthy(&self) {
        self.healthy.store(false, Ordering::SeqCst);
        *self.last_failure.lock().unwrap() = Some(Instant::now());
        log::warn!(
            "Proxy marked as unhealthy, will retry after {}s",
            self.config.retry_after_secs
        );
    }

    /// Send a chat request with tools to the proxy.
    pub async fn chat_with_tools(
        &self,
        messages: Vec<Message>,
        system: Option<String>,
        tools: Vec<ToolDefinition>,
    ) -> anyhow::Result<ProxyResponse> {
        if !self.is_available() {
            anyhow::bail!("Proxy is currently unavailable");
        }

        let url = format!("{}/v1/messages", self.config.base_url);

        let request_body = AnthropicRequest {
            model: self.config.model.clone().unwrap_or_default(),
            messages,
            system,
            tools,
            max_tokens: 4096,
        };

        let response = self
            .http
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await;

        match response {
            Ok(resp) => {
                if resp.status().is_success() {
                    let body: AnthropicResponse = resp.json().await.map_err(|e| {
                        self.mark_unhealthy();
                        anyhow::anyhow!("Failed to parse proxy response: {}", e)
                    })?;

                    self.healthy.store(true, Ordering::SeqCst);
                    Ok(self.convert_response(body))
                } else {
                    let status = resp.status();
                    let text = resp.text().await.unwrap_or_default();
                    self.mark_unhealthy();
                    anyhow::bail!("Proxy returned error {}: {}", status, text)
                }
            }
            Err(e) => {
                self.mark_unhealthy();
                anyhow::bail!("Failed to connect to proxy: {}", e)
            }
        }
    }

    /// Send a simple chat request without tools.
    pub async fn chat(
        &self,
        messages: Vec<Message>,
        system: Option<String>,
    ) -> anyhow::Result<ProxyResponse> {
        self.chat_with_tools(messages, system, vec![]).await
    }

    /// Convert Anthropic response format to our ProxyResponse.
    fn convert_response(&self, response: AnthropicResponse) -> ProxyResponse {
        let mut content = None;
        let mut tool_calls = Vec::new();

        for block in response.content {
            match block {
                ContentBlock::Text { text } => {
                    content = Some(text);
                }
                ContentBlock::ToolUse { id, name, input } => {
                    tool_calls.push(ToolCall {
                        id,
                        name,
                        arguments: input,
                    });
                }
            }
        }

        ProxyResponse {
            content,
            tool_calls,
            model: response.model,
            stop_reason: response.stop_reason,
            usage: TokenUsage {
                input_tokens: response.usage.input_tokens,
                output_tokens: response.usage.output_tokens,
            },
        }
    }
}

/// Request format for Anthropic API (used by proxy).
#[derive(Serialize)]
struct AnthropicRequest {
    model: String,
    messages: Vec<Message>,
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    tools: Vec<ToolDefinition>,
    max_tokens: u32,
}

/// Response format from Anthropic API.
#[derive(Deserialize)]
struct AnthropicResponse {
    model: String,
    content: Vec<ContentBlock>,
    stop_reason: String,
    usage: AnthropicUsage,
}

#[derive(Deserialize)]
#[serde(tag = "type")]
enum ContentBlock {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "tool_use")]
    ToolUse {
        id: String,
        name: String,
        input: serde_json::Value,
    },
}

#[derive(Deserialize)]
struct AnthropicUsage {
    input_tokens: u32,
    output_tokens: u32,
}

/// Message format for LLM conversations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: String,
    pub content: String,
}

impl Message {
    /// Create a user message.
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: "user".to_string(),
            content: content.into(),
        }
    }

    /// Create an assistant message.
    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: "assistant".to_string(),
            content: content.into(),
        }
    }

    /// Create a tool result message.
    pub fn tool(tool_use_id: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            role: "user".to_string(),
            content: format!("<tool_result>{}</tool_result>", content.into()),
        }
    }
}

/// Tool definition for LLM.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
}

/// Response from the proxy.
#[derive(Debug)]
pub struct ProxyResponse {
    pub content: Option<String>,
    pub tool_calls: Vec<ToolCall>,
    pub model: String,
    pub stop_reason: String,
    pub usage: TokenUsage,
}

/// Token usage information.
#[derive(Debug, Default)]
pub struct TokenUsage {
    pub input_tokens: u32,
    pub output_tokens: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_proxy_response_parse() {
        let json = serde_json::json!({
            "model": "claude-3-sonnet",
            "content": [
                {"type": "text", "text": "I'll help you with that."},
                {"type": "tool_use", "id": "tool_1", "name": "read_file", "input": {"path": "/test"}}
            ],
            "stop_reason": "tool_use",
            "usage": {"input_tokens": 100, "output_tokens": 50}
        });

        let response: AnthropicResponse = serde_json::from_value(json).unwrap();
        assert_eq!(response.model, "claude-3-sonnet");
        assert_eq!(response.stop_reason, "tool_use");
        assert_eq!(response.content.len(), 2);
    }

    #[test]
    fn test_proxy_response_parse_no_tools() {
        let json = serde_json::json!({
            "model": "claude-3-sonnet",
            "content": [
                {"type": "text", "text": "Hello!"}
            ],
            "stop_reason": "end_turn",
            "usage": {"input_tokens": 10, "output_tokens": 2}
        });

        let response: AnthropicResponse = serde_json::from_value(json).unwrap();
        assert_eq!(response.stop_reason, "end_turn");
        assert_eq!(response.content.len(), 1);
    }

    #[test]
    fn test_proxy_on_failure_marks_unhealthy() {
        let config = ProxyClientConfig::default();
        let client = ProxyClient::new(config);

        // Initially healthy
        assert!(client.is_available());

        // Mark unhealthy
        client.mark_unhealthy();
        assert!(!client.is_available());
    }

    #[test]
    fn test_message_creation() {
        let msg = Message::user("Hello");
        assert_eq!(msg.role, "user");
        assert_eq!(msg.content, "Hello");

        let msg = Message::assistant("Hi there");
        assert_eq!(msg.role, "assistant");
        assert_eq!(msg.content, "Hi there");
    }
}
