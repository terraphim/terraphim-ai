//! Z.ai (Zhipu AI) specific client for proper URL handling
//!
//! This module provides a direct HTTP client implementation for Z.ai
//! to work around the genai library URL construction issues.
//!
//! Z.ai provides an OpenAI-compatible API at:
//! - Standard: https://api.z.ai/api/paas/v4/chat/completions
//! - Coding Plan: https://api.z.ai/api/coding/paas/v4/chat/completions

use crate::{
    config::Provider, server::ChatResponse, token_counter::ChatRequest, ProxyError, Result,
};
use futures::Stream;
use serde_json::Value;
use std::pin::Pin;

/// Z.ai client that handles correct URL construction
pub struct ZaiClient {
    client: reqwest::Client,
}

impl Default for ZaiClient {
    fn default() -> Self {
        Self::new()
    }
}

impl ZaiClient {
    /// Create a new Z.ai client
    pub fn new() -> Self {
        let client = reqwest::Client::builder()
            .no_proxy()
            .timeout(std::time::Duration::from_secs(300)) // 5 min timeout for Z.ai
            .build()
            .map_err(|e| ProxyError::ProviderError {
                provider: "zai".to_string(),
                message: format!("Failed to create HTTP client: {}", e),
            })
            .unwrap();

        Self { client }
    }

    /// Build the correct endpoint URL for Z.ai
    fn build_endpoint(&self, provider: &Provider) -> String {
        let base = &provider.api_base_url;

        // If already ends with /chat/completions, use as-is
        if base.ends_with("/chat/completions") {
            base.clone()
        } else {
            // Append /chat/completions to any other base URL
            format!("{}/chat/completions", base.trim_end_matches('/'))
        }
    }

    /// Send streaming request to Z.ai using correct URL construction
    pub async fn send_streaming_request(
        &self,
        provider: &Provider,
        model: &str,
        request: &ChatRequest,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<String>> + Send>>> {
        let endpoint = self.build_endpoint(provider);

        tracing::info!(
            provider = %provider.name,
            endpoint = %endpoint,
            model = %model,
            "Resolved service target (Z.ai direct): adapter=OpenAI"
        );

        // Convert our ChatRequest to Z.ai/OpenAI format
        let zai_request = self.convert_to_openai_request(model, request, true)?;

        tracing::debug!(
            endpoint = %endpoint,
            request_body = %serde_json::to_string_pretty(&zai_request).unwrap_or_default(),
            "Sending Z.ai streaming request"
        );

        // Send request using raw HTTP streaming
        // Z.ai returns SSE with only "data:" lines (no "event:" prefix)
        let response = self
            .client
            .post(&endpoint)
            .header("Authorization", format!("Bearer {}", provider.api_key))
            .header("Content-Type", "application/json")
            .header("Accept", "text/event-stream")
            .header("User-Agent", "Terraphim-LLM-Proxy/1.0")
            .json(&zai_request)
            .send()
            .await
            .map_err(|e| {
                tracing::error!(error = %e, "Failed to send streaming request to Z.ai");
                ProxyError::ProviderError {
                    provider: "zai".to_string(),
                    message: format!("HTTP request failed: {}", e),
                }
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            tracing::error!(status = %status, error = %error_text, "Z.ai streaming request failed");
            return Err(ProxyError::ProviderError {
                provider: "zai".to_string(),
                message: format!("HTTP error {}: {}", status, error_text),
            });
        }

        tracing::info!("Z.ai streaming response received, starting SSE parsing");

        // Convert response body to stream of bytes
        let byte_stream = response.bytes_stream();

        // Create a stream that yields JSON data from SSE lines
        // Z.ai format: data: {...}\n\n (no event: prefix)
        // client.rs expects raw JSON, so we parse SSE and yield just the data
        let stream = async_stream::try_stream! {
            let mut buffer = String::new();
            let mut current_data: Option<String> = None;

            for await chunk in byte_stream {
                let chunk = chunk.map_err(|e| ProxyError::ProviderError {
                    provider: "zai".to_string(),
                    message: format!("Stream error: {}", e),
                })?;

                // Append chunk to buffer
                buffer.push_str(&String::from_utf8_lossy(&chunk));

                // Process complete lines
                while let Some(pos) = buffer.find('\n') {
                    let line = buffer[..pos].trim_end_matches('\r').to_string();
                    buffer = buffer[pos + 1..].to_string();

                    if line.is_empty() {
                        // Empty line means end of event - yield accumulated data
                        if let Some(data) = current_data.take() {
                            if data == "[DONE]" {
                                tracing::debug!("Received [DONE] from Z.ai");
                                yield "[DONE]".to_string();
                            } else {
                                tracing::debug!(data = %data, "Yielding JSON data from Z.ai");
                                yield data;
                            }
                        }
                    } else if let Some(data) = line.strip_prefix("data:") {
                        // Store data line (will yield at end of event)
                        current_data = Some(data.trim_start().to_string());
                    }
                }
            }

            // Process any remaining data in buffer
            if !buffer.is_empty() {
                let line = buffer.trim_end_matches('\r');
                if let Some(data) = line.strip_prefix("data:") {
                    let data = data.trim_start();
                    if data == "[DONE]" {
                        yield "[DONE]".to_string();
                    } else {
                        yield data.to_string();
                    }
                } else if let Some(ref current) = current_data {
                    yield current.clone();
                }
            }
        };

        Ok(Box::pin(stream))
    }

    /// Send non-streaming request to Z.ai
    pub async fn send_request(
        &self,
        provider: &Provider,
        model: &str,
        request: &ChatRequest,
    ) -> Result<ChatResponse> {
        let endpoint = self.build_endpoint(provider);

        tracing::debug!(
            endpoint = %endpoint,
            model = %model,
            "Using direct HTTP client for Z.ai non-streaming request"
        );

        // Convert request to OpenAI format
        let zai_request = self.convert_to_openai_request(model, request, false)?;

        // Log request details at error level to ensure visibility
        let request_json = serde_json::to_string(&zai_request).unwrap_or_default();
        tracing::error!(
            endpoint = %endpoint,
            has_tools = zai_request.get("tools").is_some(),
            request_size = request_json.len(),
            request_preview = &request_json[..request_json.len().min(500)],
            "Sending Z.ai non-streaming request - ERROR LEVEL FOR DEBUG"
        );

        let response = self
            .client
            .post(&endpoint)
            .header("Authorization", format!("Bearer {}", provider.api_key))
            .header("Content-Type", "application/json")
            .header("User-Agent", "Terraphim-LLM-Proxy/1.0")
            .json(&zai_request)
            .send()
            .await
            .map_err(|e| ProxyError::ProviderError {
                provider: "zai".to_string(),
                message: format!("HTTP request failed: {}", e),
            })?;

        if response.status().is_success() {
            let response_json: Value =
                response
                    .json()
                    .await
                    .map_err(|e| ProxyError::ProviderError {
                        provider: "zai".to_string(),
                        message: format!("Failed to parse response: {}", e),
                    })?;

            // Extract content from OpenAI-format response
            // Z.ai GLM-4.7 may use "reasoning_content" instead of "content" for thinking models
            let message = response_json
                .get("choices")
                .and_then(|c| c.as_array())
                .and_then(|arr| arr.first())
                .and_then(|choice| choice.get("message"));

            let content = message
                .and_then(|msg| msg.get("content"))
                .and_then(|c| c.as_str())
                .filter(|s| !s.is_empty())
                .or_else(|| {
                    // Fall back to reasoning_content for Z.ai thinking models
                    message
                        .and_then(|msg| msg.get("reasoning_content"))
                        .and_then(|c| c.as_str())
                })
                .unwrap_or("");

            let default_usage = serde_json::json!({});
            let usage = response_json.get("usage").unwrap_or(&default_usage);

            Ok(ChatResponse {
                id: response_json
                    .get("id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("msg_zai")
                    .to_string(),
                message_type: "message".to_string(),
                model: model.to_string(),
                role: "assistant".to_string(),
                content: vec![crate::server::ContentBlock {
                    block_type: "text".to_string(),
                    text: Some(content.to_string()),
                    id: None,
                    name: None,
                    input: None,
                }],
                stop_reason: response_json
                    .get("choices")
                    .and_then(|c| c.as_array())
                    .and_then(|arr| arr.first())
                    .and_then(|choice| choice.get("finish_reason"))
                    .and_then(|r| r.as_str())
                    .map(|s| s.to_string()),
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
                provider: "zai".to_string(),
                message: format!("HTTP {} - {}", status, error_text),
            })
        }
    }

    /// Convert our ChatRequest to OpenAI API format
    fn convert_to_openai_request(
        &self,
        model: &str,
        req: &ChatRequest,
        stream: bool,
    ) -> Result<Value> {
        // Build messages array, prepending system message if present
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
        // Convert "developer" role to "system" as Z.AI doesn't support developer role
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

        let mut openai_req = serde_json::json!({
            "model": model,
            "messages": messages,
            "stream": stream
        });

        // Don't add stream_options - it might cause 400 errors with ZAI
        // if stream {
        //     openai_req["stream_options"] = serde_json::json!({
        //         "include_usage": true
        //     });
        // }

        // Add optional parameters
        if let Some(max_tokens) = req.max_tokens {
            openai_req["max_tokens"] = Value::Number(max_tokens.into());
        }
        if let Some(temperature) = req.temperature {
            openai_req["temperature"] = Value::Number(
                serde_json::Number::from_f64(temperature as f64)
                    .unwrap_or(serde_json::Number::from(0)),
            );
        }
        // Pass tools in OpenAI format (ZAI uses OpenAI-compatible endpoint)
        if let Some(tools) = &req.tools {
            if !tools.is_empty() {
                openai_req["tools"] = serde_json::to_value(tools)?;
            }
        }

        Ok(openai_req)
    }
}
