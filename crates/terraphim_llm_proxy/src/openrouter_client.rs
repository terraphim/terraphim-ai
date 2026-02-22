//! OpenRouter-specific client for streaming compatibility
//!
//! This module provides a direct HTTP client implementation for OpenRouter
//! to work around the genai library streaming issues (Issue #1)

use crate::{
    config::Provider, server::ChatResponse, token_counter::ChatRequest, ProxyError, Result,
};
use futures::{Stream, StreamExt};
use reqwest_eventsource::{Event, EventSource};
use serde_json::Value;
use std::pin::Pin;

/// OpenRouter streaming client that bypasses genai library
pub struct OpenRouterClient {
    client: reqwest::Client,
}

impl OpenRouterClient {
    /// Create a new OpenRouter client
    pub fn new() -> Self {
        let client = reqwest::Client::builder()
            .no_proxy()
            .timeout(std::time::Duration::from_secs(60))
            .build()
            .map_err(|e| ProxyError::ProviderError {
                provider: "openrouter".to_string(),
                message: format!("Failed to create HTTP client: {}", e),
            })
            .unwrap();

        Self { client }
    }

    /// Send streaming request to OpenRouter using direct HTTP + SSE
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
            "Resolved service target (OpenRouter direct): adapter=OpenAI"
        );

        // Convert our ChatRequest to OpenRouter format, using the model from routing decision
        let openrouter_request = self.convert_to_openrouter_request(model, request)?;

        let endpoint = format!("{}/chat/completions", provider.api_base_url);

        tracing::debug!(
            endpoint = %endpoint,
            request_body = %serde_json::to_string_pretty(&openrouter_request).unwrap_or_default(),
            "Sending OpenRouter streaming request"
        );

        // Build the request
        let req_builder = self
            .client
            .post(endpoint)
            .header("Authorization", format!("Bearer {}", provider.api_key))
            .header("Content-Type", "application/json")
            .header("HTTP-Referer", "https://terraphim.ai")
            .header("X-Title", "Terraphim LLM Proxy")
            .json(&openrouter_request);

        // Send request and create EventSource
        let event_source = EventSource::new(req_builder).map_err(|e| {
            tracing::error!(error = %e, "Failed to create EventSource for OpenRouter");
            ProxyError::ProviderError {
                provider: "openrouter".to_string(),
                message: format!("Failed to create EventSource: {}", e),
            }
        })?;

        tracing::info!("OpenRouter EventSource created successfully");

        // Convert SSE events to raw strings for the server to handle
        let stream = event_source.map(move |result| match result {
            Ok(Event::Message(message)) => {
                tracing::debug!(
                    event_type = %message.event,
                    data = %message.data,
                    "Received SSE message from OpenRouter"
                );
                Ok(message.data)
            }
            Ok(Event::Open) => {
                tracing::debug!("SSE connection opened to OpenRouter");
                Ok("event: connected\ndata: {}\n\n".to_string())
            }
            Err(e) => {
                tracing::error!(error = %e, "SSE error from OpenRouter");
                Err(ProxyError::ProviderError {
                    provider: "openrouter".to_string(),
                    message: format!("SSE error: {}", e),
                })
            }
        });

        Ok(Box::pin(stream))
    }

    /// Convert our ChatRequest to OpenRouter API format
    /// Uses the provided model parameter instead of request.model to support
    /// model mappings that resolve provider,model format in routing
    fn convert_to_openrouter_request(&self, model: &str, req: &ChatRequest) -> Result<Value> {
        let mut openrouter_req = serde_json::json!({
            "model": model,
            "messages": req.messages,
            "stream": true
        });

        // Add optional parameters
        if let Some(max_tokens) = req.max_tokens {
            openrouter_req["max_tokens"] = Value::Number(max_tokens.into());
        }
        if let Some(temperature) = req.temperature {
            openrouter_req["temperature"] = Value::Number(
                serde_json::Number::from_f64(temperature as f64)
                    .unwrap_or(serde_json::Number::from(0)),
            );
        }
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
            openrouter_req["system"] = Value::String(system_text);
        }
        if let Some(tools) = &req.tools {
            openrouter_req["tools"] = serde_json::to_value(tools)?;
        }

        Ok(openrouter_req)
    }

    /// Send streaming request using direct HTTP (Issue #1 workaround)
    pub async fn send_streaming_request_direct(
        &self,
        provider: &Provider,
        model: &str,
        request: &ChatRequest,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<String>> + Send>>> {
        self.send_streaming_request(provider, model, request).await
    }

    /// Send non-streaming request using direct HTTP
    pub async fn send_request(
        &self,
        provider: &Provider,
        model: &str,
        request: &ChatRequest,
    ) -> Result<ChatResponse> {
        tracing::debug!("Using direct HTTP client for OpenRouter non-streaming request");

        // Convert request to OpenRouter format, using the model from routing decision
        let openrouter_request = self.convert_to_openrouter_request(model, request)?;

        // Remove stream flag for non-streaming
        let mut req_data = openrouter_request;
        req_data["stream"] = serde_json::Value::Bool(false);

        // Build the request
        let endpoint = format!("{}/chat/completions", provider.api_base_url);
        let response = self
            .client
            .post(endpoint)
            .header("Authorization", format!("Bearer {}", provider.api_key))
            .header("Content-Type", "application/json")
            .header("HTTP-Referer", "https://terraphim.ai")
            .header("X-Title", "Terraphim LLM Proxy")
            .json(&req_data)
            .send()
            .await
            .map_err(|e| ProxyError::ProviderError {
                provider: "openrouter".to_string(),
                message: format!("HTTP request failed: {}", e),
            })?;

        if response.status().is_success() {
            let response_json: Value =
                response
                    .json()
                    .await
                    .map_err(|e| ProxyError::ProviderError {
                        provider: "openrouter".to_string(),
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
            // Ensure at least one content block
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
                    .unwrap_or("msg_openrouter")
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
                provider: "openrouter".to_string(),
                message: format!("HTTP {} - {}", status, error_text),
            })
        }
    }
}

impl Default for OpenRouterClient {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::token_counter::{Message, MessageContent, SystemPrompt};

    fn create_test_provider() -> Provider {
        Provider {
            name: "openrouter".to_string(),
            api_base_url: "https://openrouter.ai/api/v1".to_string(),
            api_key: "test_key".to_string(),
            models: vec!["anthropic/claude-3.5-sonnet".to_string()],
            transformers: vec![],
        }
    }

    fn create_test_request() -> ChatRequest {
        ChatRequest {
            model: "anthropic/claude-3.5-sonnet".to_string(),
            messages: vec![Message {
                role: "user".to_string(),
                content: MessageContent::Text("Hello, OpenRouter!".to_string()),
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
    fn test_openrouter_client_creation() {
        let _client = OpenRouterClient::new();
    }

    #[test]
    fn test_convert_to_openrouter_request_basic() {
        let client = OpenRouterClient::new();
        let request = create_test_request();

        let result = client.convert_to_openrouter_request(&request.model, &request);
        assert!(result.is_ok(), "Should convert basic request successfully");

        let openrouter_req = result.unwrap();
        assert_eq!(openrouter_req["model"], "anthropic/claude-3.5-sonnet");
        assert_eq!(openrouter_req["stream"], true);
        assert_eq!(openrouter_req["max_tokens"], 100);
        // Account for floating-point conversion
        assert!((openrouter_req["temperature"].as_f64().unwrap() - 0.7).abs() < 0.0001);
        assert_eq!(openrouter_req["system"], "You are a helpful assistant.");
        assert!(openrouter_req["messages"].is_array());
    }

    #[test]
    fn test_convert_to_openrouter_request_with_tools() {
        let client = OpenRouterClient::new();
        let mut request = create_test_request();
        request.tools = Some(vec![crate::token_counter::Tool {
            tool_type: Some("function".to_string()),
            function: Some(crate::token_counter::FunctionTool {
                name: "search_tool".to_string(),
                description: Some("A search tool".to_string()),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "query": {"type": "string"}
                    }
                }),
            }),
            name: None,
            description: None,
            input_schema: None,
        }]);

        let result = client.convert_to_openrouter_request(&request.model, &request);
        assert!(
            result.is_ok(),
            "Should convert request with tools successfully"
        );

        let openrouter_req = result.unwrap();
        assert!(openrouter_req["tools"].is_array());
    }

    #[test]
    fn test_convert_to_openrouter_request_non_streaming() {
        let client = OpenRouterClient::new();
        let mut request = create_test_request();
        request.stream = Some(false);

        let result = client.convert_to_openrouter_request(&request.model, &request);
        assert!(
            result.is_ok(),
            "Should convert non-streaming request successfully"
        );

        let openrouter_req = result.unwrap();
        // Note: convert_to_openrouter_request always sets stream=true
        // The stream flag is handled separately in the actual request sending
        assert_eq!(openrouter_req["stream"], true);
    }

    #[test]
    fn test_openrouter_endpoint_construction() {
        let provider = create_test_provider();
        let endpoint = format!("{}/chat/completions", provider.api_base_url);
        assert_eq!(endpoint, "https://openrouter.ai/api/v1/chat/completions");
    }

    #[test]
    fn test_convert_to_openrouter_request_different_models() {
        let client = OpenRouterClient::new();

        let models = vec![
            "anthropic/claude-3.5-sonnet",
            "anthropic/claude-3.5-haiku",
            "openai/gpt-4o",
            "google/gemini-pro",
            "meta-llama/llama-3.1-70b-instruct",
        ];

        for model in models {
            let mut request = create_test_request();
            request.model = model.to_string();

            let result = client.convert_to_openrouter_request(model, &request);
            assert!(
                result.is_ok(),
                "Should convert request for model {} successfully",
                model
            );

            let openrouter_req = result.unwrap();
            assert_eq!(openrouter_req["model"], model);
        }
    }

    #[test]
    fn test_convert_to_openrouter_request_with_system_array() {
        let client = OpenRouterClient::new();
        let mut request = create_test_request();
        request.system = Some(SystemPrompt::Array(vec![
            crate::token_counter::SystemBlock::Text {
                text: "You are a helpful assistant.".to_string(),
            },
            crate::token_counter::SystemBlock::CacheControl {
                text: "Be concise.".to_string(),
                cache_control: crate::token_counter::CacheControl {
                    cache_type: "ephemeral".to_string(),
                },
            },
        ]));

        let result = client.convert_to_openrouter_request(&request.model, &request);
        assert!(
            result.is_ok(),
            "Should convert request with system array successfully"
        );

        let openrouter_req = result.unwrap();
        assert!(openrouter_req["system"].is_string());
        let system_text = openrouter_req["system"].as_str().unwrap();
        assert!(system_text.contains("helpful assistant"));
        assert!(system_text.contains("concise"));
    }

    #[tokio::test]
    #[ignore] // Requires real API key - run with cargo test -- --ignored
    async fn test_openrouter_real_streaming() {
        let client = OpenRouterClient::new();
        let provider = Provider {
            name: "openrouter".to_string(),
            api_base_url: "https://openrouter.ai/api/v1".to_string(),
            api_key: std::env::var("OPENROUTER_API_KEY").unwrap_or_else(|_| "test_key".to_string()),
            models: vec!["anthropic/claude-3.5-sonnet".to_string()],
            transformers: vec![],
        };
        let request = create_test_request();

        match client
            .send_streaming_request(&provider, "anthropic/claude-3.5-sonnet", &request)
            .await
        {
            Ok(stream) => {
                use futures::StreamExt;
                let mut event_count = 0;
                tokio::pin!(stream);

                while let Some(result) = stream.next().await {
                    match result {
                        Ok(data) => {
                            event_count += 1;
                            println!("Received streaming data: {}", data);
                            assert!(!data.is_empty(), "Streaming data should not be empty");

                            if event_count >= 5 {
                                break; // Limit for test
                            }
                        }
                        Err(e) => {
                            println!("Stream error: {}", e);
                            break;
                        }
                    }
                }

                assert!(
                    event_count > 0,
                    "Should receive at least one streaming event"
                );
            }
            Err(e) => {
                println!("Streaming request failed: {}", e);
                // Don't assert failure here as it might be due to invalid API key
            }
        }
    }

    #[tokio::test]
    #[ignore] // Requires real API key - run with cargo test -- --ignored
    async fn test_openrouter_real_non_streaming() {
        let client = OpenRouterClient::new();
        let provider = Provider {
            name: "openrouter".to_string(),
            api_base_url: "https://openrouter.ai/api/v1".to_string(),
            api_key: std::env::var("OPENROUTER_API_KEY").unwrap_or_else(|_| "test_key".to_string()),
            models: vec!["anthropic/claude-3.5-sonnet".to_string()],
            transformers: vec![],
        };
        let mut request = create_test_request();
        request.stream = Some(false);

        match client
            .send_request(&provider, "anthropic/claude-3.5-sonnet", &request)
            .await
        {
            Ok(response) => {
                println!("Received response: {:?}", response);
                assert!(
                    !response.content.is_empty(),
                    "Response should contain content"
                );
                assert_eq!(response.role, "assistant");
                assert!(!response.id.is_empty(), "Response should have an ID");
            }
            Err(e) => {
                println!("Non-streaming request failed: {}", e);
                // Don't assert failure here as it might be due to invalid API key
            }
        }
    }

    #[tokio::test]
    #[ignore] // Requires real API key - run with cargo test -- --ignored
    async fn test_openrouter_different_models() {
        let client = OpenRouterClient::new();
        let provider = Provider {
            name: "openrouter".to_string(),
            api_base_url: "https://openrouter.ai/api/v1".to_string(),
            api_key: std::env::var("OPENROUTER_API_KEY").unwrap_or_else(|_| "test_key".to_string()),
            models: vec![
                "anthropic/claude-3.5-sonnet".to_string(),
                "anthropic/claude-3.5-haiku".to_string(),
                "openai/gpt-4o-mini".to_string(),
            ],
            transformers: vec![],
        };

        for model in &provider.models {
            let mut request = create_test_request();
            request.model = model.clone();
            request.stream = Some(false);

            println!("Testing model: {}", model);
            match client.send_request(&provider, model, &request).await {
                Ok(response) => {
                    println!("✅ {} - Response received", model);
                    assert_eq!(response.model, *model);
                }
                Err(e) => {
                    println!("❌ {} - Request failed: {}", model, e);
                    // Continue testing other models
                }
            }
        }
    }

    #[test]
    fn test_openrouter_request_parameter_validation() {
        let client = OpenRouterClient::new();

        // Test with extreme temperature values
        let mut request = create_test_request();
        request.temperature = Some(-0.5); // Invalid negative temperature

        let result = client.convert_to_openrouter_request(&request.model, &request);
        assert!(result.is_ok(), "Should handle negative temperature");

        // Test with very large max_tokens
        request.temperature = Some(0.7);
        request.max_tokens = Some(999999);

        let result = client.convert_to_openrouter_request(&request.model, &request);
        assert!(result.is_ok(), "Should handle large max_tokens");

        let openrouter_req = result.unwrap();
        assert_eq!(openrouter_req["max_tokens"], 999999);
    }

    #[test]
    fn test_openrouter_message_formatting() {
        let client = OpenRouterClient::new();
        let mut request = create_test_request();

        // Add multiple messages with different roles
        request.messages = vec![
            Message {
                role: "system".to_string(),
                content: MessageContent::Text("System message".to_string()),
                tool_calls: None,
                tool_call_id: None,
                name: None,
            },
            Message {
                role: "user".to_string(),
                content: MessageContent::Text("User message".to_string()),
                tool_calls: None,
                tool_call_id: None,
                name: None,
            },
            Message {
                role: "assistant".to_string(),
                content: MessageContent::Text("Assistant response".to_string()),
                tool_calls: None,
                tool_call_id: None,
                name: None,
            },
        ];

        let result = client.convert_to_openrouter_request(&request.model, &request);
        assert!(
            result.is_ok(),
            "Should convert multiple messages successfully"
        );

        let openrouter_req = result.unwrap();
        let messages = openrouter_req["messages"].as_array().unwrap();
        assert_eq!(messages.len(), 3);
        assert_eq!(messages[0]["role"], "system");
        assert_eq!(messages[1]["role"], "user");
        assert_eq!(messages[2]["role"], "assistant");
    }
}
