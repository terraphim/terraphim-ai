//! Cerebras-specific client for proper URL handling
//!
//! This module provides a direct HTTP client implementation for Cerebras
//! to work around the genai library URL construction issues.
//! Unlike Groq which uses /openai/v1/, Cerebras uses just /v1/

use crate::{
    config::Provider, server::ChatResponse, token_counter::ChatRequest, ProxyError, Result,
};
use futures::{Stream, StreamExt};
use reqwest_eventsource::{Event, EventSource};
use serde_json::Value;
use std::pin::Pin;

/// Cerebras streaming client that handles correct URL construction
pub struct CerebrasClient {
    client: reqwest::Client,
}

impl CerebrasClient {
    /// Create a new Cerebras client
    pub fn new() -> Self {
        let client = reqwest::Client::builder()
            .no_proxy()
            .timeout(std::time::Duration::from_secs(60))
            .build()
            .map_err(|e| ProxyError::ProviderError {
                provider: "cerebras".to_string(),
                message: format!("Failed to create HTTP client: {}", e),
            })
            .unwrap();

        Self { client }
    }

    /// Send streaming request to Cerebras using correct URL construction
    pub async fn send_streaming_request(
        &self,
        provider: &Provider,
        model: &str,
        request: &ChatRequest,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<String>> + Send>>> {
        tracing::info!(
            provider = %provider.name,
            endpoint = %provider.api_base_url,
            model = %model,
            "Resolved service target (Cerebras direct): adapter=OpenAI"
        );

        // Validate model against known Cerebras models (warning only, don't fail)
        if !crate::cerebras_models::is_valid_cerebras_model(model) {
            tracing::warn!(
                model = %model,
                "Unknown Cerebras model - request may fail. Check https://inference-docs.cerebras.ai/"
            );
        }

        // Convert our ChatRequest to Cerebras format, using resolved model from routing
        let cerebras_request = self.convert_to_cerebras_request(model, request)?;

        // Build the request with correct URL
        // Cerebras uses /v1/ instead of /openai/v1/
        let endpoint = if provider.api_base_url.ends_with("/v1") {
            format!("{}/chat/completions", provider.api_base_url)
        } else {
            format!("{}/v1/chat/completions", provider.api_base_url)
        };

        tracing::debug!(
            endpoint = %endpoint,
            request_body = %serde_json::to_string_pretty(&cerebras_request).unwrap_or_default(),
            "Sending Cerebras streaming request"
        );

        let req_builder = self
            .client
            .post(endpoint)
            .header("Authorization", format!("Bearer {}", provider.api_key))
            .header("Content-Type", "application/json")
            .header("User-Agent", "Terraphim-LLM-Proxy/1.0")
            .json(&cerebras_request);

        // Send request and create EventSource
        let event_source = EventSource::new(req_builder).map_err(|e| {
            tracing::error!(error = %e, "Failed to create EventSource for Cerebras");
            ProxyError::ProviderError {
                provider: "cerebras".to_string(),
                message: format!("Failed to create EventSource: {}", e),
            }
        })?;

        tracing::info!("Cerebras EventSource created successfully");

        // Convert SSE events to raw strings for the server to handle
        let stream = event_source.map(move |result| match result {
            Ok(Event::Message(message)) => {
                tracing::debug!(
                    event_type = %message.event,
                    data = %message.data,
                    "Received SSE message from Cerebras"
                );
                Ok(message.data)
            }
            Ok(Event::Open) => {
                tracing::debug!("SSE connection opened to Cerebras");
                Ok("event: connected\ndata: {}\n\n".to_string())
            }
            Err(e) => {
                tracing::error!(error = %e, "SSE error from Cerebras");
                Err(ProxyError::ProviderError {
                    provider: "cerebras".to_string(),
                    message: format!("SSE error: {}", e),
                })
            }
        });

        Ok(Box::pin(stream))
    }

    /// Convert our ChatRequest to Cerebras API format
    /// Uses the resolved model parameter instead of req.model to support pattern routing
    fn convert_to_cerebras_request(&self, model: &str, req: &ChatRequest) -> Result<Value> {
        // Build messages array, prepending system message if present
        // Cerebras uses OpenAI format where system is a message, not a top-level field
        let mut messages: Vec<Value> = Vec::new();

        if let Some(system) = &req.system {
            let system_text = match system {
                crate::token_counter::SystemPrompt::Text(text) => text.clone(),
                crate::token_counter::SystemPrompt::Array(blocks) => blocks
                    .iter()
                    .map(|block| match block {
                        crate::token_counter::SystemBlock::Text { text } => text.clone(),
                        crate::token_counter::SystemBlock::CacheControl { text, .. } => {
                            text.clone()
                        }
                    })
                    .collect::<Vec<_>>()
                    .join("\n\n"),
            };
            messages.push(serde_json::json!({
                "role": "system",
                "content": system_text
            }));
        }

        // Add user/assistant messages
        // Convert "developer" role to "system" as Cerebras doesn't support developer role
        for msg in &req.messages {
            let mut msg_value = serde_json::to_value(msg)?;
            if let Some(role) = msg_value.get("role").and_then(|r| r.as_str()) {
                if role == "developer" {
                    if let Some(obj) = msg_value.as_object_mut() {
                        obj.insert(
                            "role".to_string(),
                            serde_json::Value::String("system".to_string()),
                        );
                    }
                }
            }
            messages.push(msg_value);
        }

        let mut cerebras_req = serde_json::json!({
            "model": model,
            "messages": messages,
            "stream": true
        });

        // Add optional parameters
        if let Some(max_tokens) = req.max_tokens {
            cerebras_req["max_tokens"] = Value::Number(max_tokens.into());
        }
        if let Some(temperature) = req.temperature {
            cerebras_req["temperature"] = Value::Number(
                serde_json::Number::from_f64(temperature as f64)
                    .unwrap_or(serde_json::Number::from(0)),
            );
        }
        if let Some(tools) = &req.tools {
            cerebras_req["tools"] = serde_json::to_value(tools)?;
        }

        Ok(cerebras_req)
    }

    /// Send non-streaming request to Cerebras
    pub async fn send_request(
        &self,
        provider: &Provider,
        model: &str,
        request: &ChatRequest,
    ) -> Result<ChatResponse> {
        tracing::debug!("Using direct HTTP client for Cerebras non-streaming request");

        // Validate model against known Cerebras models (warning only, don't fail)
        if !crate::cerebras_models::is_valid_cerebras_model(model) {
            tracing::warn!(
                model = %model,
                "Unknown Cerebras model - request may fail. Check https://inference-docs.cerebras.ai/"
            );
        }

        // Convert request to Cerebras format, using resolved model from routing
        let cerebras_request = self.convert_to_cerebras_request(model, request)?;

        // Remove stream flag for non-streaming
        let mut cerebras_req = cerebras_request;
        cerebras_req["stream"] = Value::Bool(false);

        // Build correct URL - Cerebras uses /v1/ instead of /openai/v1/
        let endpoint = if provider.api_base_url.ends_with("/v1") {
            format!("{}/chat/completions", provider.api_base_url)
        } else {
            format!("{}/v1/chat/completions", provider.api_base_url)
        };

        let response = self
            .client
            .post(endpoint)
            .header("Authorization", format!("Bearer {}", provider.api_key))
            .header("Content-Type", "application/json")
            .header("User-Agent", "Terraphim-LLM-Proxy/1.0")
            .json(&cerebras_req)
            .send()
            .await
            .map_err(|e| ProxyError::ProviderError {
                provider: "cerebras".to_string(),
                message: format!("HTTP request failed: {}", e),
            })?;

        if response.status().is_success() {
            let response_json: Value =
                response
                    .json()
                    .await
                    .map_err(|e| ProxyError::ProviderError {
                        provider: "cerebras".to_string(),
                        message: format!("Failed to parse response: {}", e),
                    })?;

            // Extract content from response
            let content = response_json
                .get("choices")
                .and_then(|c| c.as_array())
                .and_then(|arr| arr.first())
                .and_then(|choice| choice.get("message"))
                .and_then(|msg| msg.get("content"))
                .and_then(|content| content.as_str())
                .unwrap_or("");

            // Build content blocks: text + tool_calls
            let mut content_blocks = Vec::new();
            if !content.is_empty() {
                content_blocks.push(crate::server::ContentBlock {
                    block_type: "text".to_string(),
                    text: Some(content.to_string()),
                    id: None,
                    name: None,
                    input: None,
                });
            }
            content_blocks.extend(crate::tool_call_utils::extract_tool_calls_from_response(
                &response_json,
            ));
            if content_blocks.is_empty() {
                content_blocks.push(crate::server::ContentBlock {
                    block_type: "text".to_string(),
                    text: Some(String::new()),
                    id: None,
                    name: None,
                    input: None,
                });
            }

            let finish_reason = response_json
                .get("choices")
                .and_then(|c| c.as_array())
                .and_then(|arr| arr.first())
                .and_then(|choice| choice.get("finish_reason"))
                .and_then(|r| r.as_str())
                .map(|s| s.to_string());

            let stop_reason =
                crate::tool_call_utils::resolve_stop_reason(&content_blocks, finish_reason);

            let default_usage = serde_json::json!({});
            let usage = response_json.get("usage").unwrap_or(&default_usage);

            Ok(ChatResponse {
                id: response_json
                    .get("id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("msg_cerebras")
                    .to_string(),
                message_type: "message".to_string(),
                model: model.to_string(),
                role: "assistant".to_string(),
                content: content_blocks,
                stop_reason,
                stop_sequence: None,
                usage: genai::chat::Usage {
                    prompt_tokens_details: None,
                    completion_tokens_details: None,
                    prompt_tokens: usage
                        .get("prompt_tokens")
                        .and_then(|v| v.as_u64())
                        .map(|v| v as i32),
                    completion_tokens: usage
                        .get("completion_tokens")
                        .and_then(|v| v.as_u64())
                        .map(|v| v as i32),
                    total_tokens: usage
                        .get("total_tokens")
                        .and_then(|v| v.as_u64())
                        .map(|v| v as i32),
                },
            })
        } else {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            Err(ProxyError::ProviderError {
                provider: "cerebras".to_string(),
                message: format!("HTTP {} - {}", status, error_text),
            })
        }
    }
}

impl Default for CerebrasClient {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::token_counter::{Message, MessageContent, SystemPrompt};

    fn create_test_request() -> ChatRequest {
        ChatRequest {
            model: "llama3.1-8b".to_string(),
            messages: vec![Message {
                role: "user".to_string(),
                content: MessageContent::Text("Hello, world!".to_string()),
                tool_calls: None,
                tool_call_id: None,
                name: None,
            }],
            system: Some(SystemPrompt::Text(
                "You are a helpful assistant.".to_string(),
            )),
            max_tokens: Some(100),
            temperature: Some(0.7),
            stream: Some(true),
            tools: None,
            thinking: None,
            ..Default::default()
        }
    }

    #[test]
    fn test_cerebras_client_creation() {
        let _client = CerebrasClient::new();
    }

    #[test]
    fn test_convert_to_cerebras_request_simple() {
        let client = CerebrasClient::new();
        let request = create_test_request();

        let result = client
            .convert_to_cerebras_request("llama3.1-8b", &request)
            .unwrap();
        assert_eq!(result["model"], "llama3.1-8b");
        assert_eq!(result["max_tokens"], 100);
        assert_eq!(result["stream"], true);
    }

    #[test]
    fn test_convert_to_cerebras_request_with_system() {
        let client = CerebrasClient::new();
        let request = create_test_request();

        let result = client
            .convert_to_cerebras_request("llama3.1-8b", &request)
            .unwrap();
        let messages = result["messages"].as_array().unwrap();
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0]["role"], "system");
        assert_eq!(messages[0]["content"], "You are a helpful assistant.");
    }

    #[test]
    fn test_endpoint_construction() {
        // Test that endpoint is correctly constructed for Cerebras
        // With /v1 suffix
        let url = "https://api.cerebras.ai/v1";
        let endpoint = if url.ends_with("/v1") {
            format!("{}/chat/completions", url)
        } else {
            format!("{}/v1/chat/completions", url)
        };
        assert_eq!(endpoint, "https://api.cerebras.ai/v1/chat/completions");

        // Without /v1 suffix
        let url = "https://api.cerebras.ai";
        let endpoint = if url.ends_with("/v1") {
            format!("{}/chat/completions", url)
        } else {
            format!("{}/v1/chat/completions", url)
        };
        assert_eq!(endpoint, "https://api.cerebras.ai/v1/chat/completions");
    }
}
