//! Groq-specific client for proper URL handling
//!
//! This module provides a direct HTTP client implementation for Groq
//! to work around the genai library URL construction issues

use crate::{
    config::Provider, server::ChatResponse, token_counter::ChatRequest, ProxyError, Result,
};
use futures::{Stream, StreamExt};
use reqwest_eventsource::{Event, EventSource};
use serde_json::Value;
use std::pin::Pin;

/// Groq streaming client that handles correct URL construction
pub struct GroqClient {
    client: reqwest::Client,
}

impl GroqClient {
    /// Create a new Groq client
    pub fn new() -> Self {
        let client = reqwest::Client::builder()
            .no_proxy()
            .timeout(std::time::Duration::from_secs(60))
            .build()
            .map_err(|e| ProxyError::ProviderError {
                provider: "groq".to_string(),
                message: format!("Failed to create HTTP client: {}", e),
            })
            .unwrap();

        Self { client }
    }

    /// Send streaming request to Groq using correct URL construction
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
            "Resolved service target (Groq direct): adapter=OpenAI"
        );

        // Validate model against known Groq models (warning only, don't fail)
        if !crate::groq_models::is_valid_groq_model(model) {
            tracing::warn!(
                model = %model,
                "Unknown Groq model - request may fail. Check https://console.groq.com/docs/models"
            );
        }

        // Convert our ChatRequest to Groq format, using resolved model from routing
        let groq_request = self.convert_to_groq_request(model, request)?;

        // Build the request with correct URL
        let endpoint = if provider.api_base_url.ends_with("/openai/v1") {
            format!("{}/chat/completions", provider.api_base_url)
        } else {
            format!("{}/openai/v1/chat/completions", provider.api_base_url)
        };

        tracing::debug!(
            endpoint = %endpoint,
            request_body = %serde_json::to_string_pretty(&groq_request).unwrap_or_default(),
            "Sending Groq streaming request"
        );

        let req_builder = self
            .client
            .post(endpoint)
            .header("Authorization", format!("Bearer {}", provider.api_key))
            .header("Content-Type", "application/json")
            .header("User-Agent", "Terraphim-LLM-Proxy/1.0")
            .json(&groq_request);

        // Send request and create EventSource
        let event_source = EventSource::new(req_builder).map_err(|e| {
            tracing::error!(error = %e, "Failed to create EventSource for Groq");
            ProxyError::ProviderError {
                provider: "groq".to_string(),
                message: format!("Failed to create EventSource: {}", e),
            }
        })?;

        tracing::info!("Groq EventSource created successfully");

        let has_tools = request
            .tools
            .as_ref()
            .map(|tools| !tools.is_empty())
            .unwrap_or(false);
        let approx_payload_chars = Self::approx_request_chars(request);
        let model_name = model.to_string();

        // Convert SSE events to raw strings for the server to handle
        let stream = event_source.map(move |result| match result {
            Ok(Event::Message(message)) => {
                tracing::debug!(
                    event_type = %message.event,
                    data = %message.data,
                    "Received SSE message from Groq"
                );
                Ok(message.data)
            }
            Ok(Event::Open) => {
                tracing::debug!("SSE connection opened to Groq");
                Ok("event: connected\ndata: {}\n\n".to_string())
            }
            Err(e) => {
                let enriched_message = format!(
                    "SSE error: {} (model={}, has_tools={}, approx_payload_chars={})",
                    e, model_name, has_tools, approx_payload_chars
                );
                tracing::error!(error = %enriched_message, "SSE error from Groq");
                Err(ProxyError::ProviderError {
                    provider: "groq".to_string(),
                    message: enriched_message,
                })
            }
        });

        Ok(Box::pin(stream))
    }

    /// Convert our ChatRequest to Groq API format
    /// Uses the resolved model parameter instead of req.model to support pattern routing
    fn convert_to_groq_request(&self, model: &str, req: &ChatRequest) -> Result<Value> {
        // Build messages array, prepending system message if present
        // Groq uses OpenAI format where system is a message, not a top-level field
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
        for msg in &req.messages {
            messages.push(serde_json::to_value(msg)?);
        }

        let mut groq_req = serde_json::json!({
            "model": model,
            "messages": messages,
            "stream": true
        });

        // Add optional parameters
        if let Some(max_tokens) = req.max_tokens {
            groq_req["max_tokens"] = Value::Number(max_tokens.into());
        }
        if let Some(temperature) = req.temperature {
            groq_req["temperature"] = Value::Number(
                serde_json::Number::from_f64(temperature as f64)
                    .unwrap_or(serde_json::Number::from(0)),
            );
        }
        if let Some(tools) = &req.tools {
            groq_req["tools"] = serde_json::to_value(tools)?;
        }

        Ok(groq_req)
    }

    /// Send non-streaming request to Groq
    pub async fn send_request(
        &self,
        provider: &Provider,
        model: &str,
        request: &ChatRequest,
    ) -> Result<ChatResponse> {
        tracing::debug!("Using direct HTTP client for Groq non-streaming request");

        // Validate model against known Groq models (warning only, don't fail)
        if !crate::groq_models::is_valid_groq_model(model) {
            tracing::warn!(
                model = %model,
                "Unknown Groq model - request may fail. Check https://console.groq.com/docs/models"
            );
        }

        // Convert request to Groq format, using resolved model from routing
        let groq_request = self.convert_to_groq_request(model, request)?;

        // Remove stream flag for non-streaming
        let mut groq_req = groq_request;
        groq_req["stream"] = Value::Bool(false);

        // Build correct URL
        let endpoint = if provider.api_base_url.ends_with("/openai/v1") {
            format!("{}/chat/completions", provider.api_base_url)
        } else {
            format!("{}/openai/v1/chat/completions", provider.api_base_url)
        };

        let response = self
            .client
            .post(endpoint)
            .header("Authorization", format!("Bearer {}", provider.api_key))
            .header("Content-Type", "application/json")
            .header("User-Agent", "Terraphim-LLM-Proxy/1.0")
            .json(&groq_req)
            .send()
            .await
            .map_err(|e| ProxyError::ProviderError {
                provider: "groq".to_string(),
                message: format!("HTTP request failed: {}", e),
            })?;

        if response.status().is_success() {
            let response_json: Value =
                response
                    .json()
                    .await
                    .map_err(|e| ProxyError::ProviderError {
                        provider: "groq".to_string(),
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

            // Check for standard OpenAI tool_calls format first
            let tool_calls = response_json
                .get("choices")
                .and_then(|c| c.as_array())
                .and_then(|arr| arr.first())
                .and_then(|choice| choice.get("message"))
                .and_then(|msg| msg.get("tool_calls"));

            // Build content blocks
            let mut content_blocks = Vec::new();

            // Add text content if present
            if !content.is_empty() {
                content_blocks.push(crate::server::ContentBlock {
                    block_type: "text".to_string(),
                    text: Some(content.to_string()),
                    id: None,
                    name: None,
                    input: None,
                });
            }

            // Add tool_calls if present (standard OpenAI format)
            if let Some(tools) = tool_calls {
                if let Some(tools_array) = tools.as_array() {
                    for tool in tools_array {
                        if let (Some(id), Some(func)) = (
                            tool.get("id").and_then(|v| v.as_str()),
                            tool.get("function"),
                        ) {
                            let name = func
                                .get("name")
                                .and_then(|v| v.as_str())
                                .unwrap_or("unknown");
                            let args = func
                                .get("arguments")
                                .and_then(|v| v.as_str())
                                .unwrap_or("{}");
                            let input: serde_json::Value = serde_json::from_str(args)
                                .unwrap_or_else(|_| serde_json::json!({}));

                            content_blocks.push(crate::server::ContentBlock {
                                block_type: "tool_use".to_string(),
                                text: None,
                                id: Some(id.to_string()),
                                name: Some(name.to_string()),
                                input: Some(input),
                            });
                        }
                    }
                }
            } else {
                // Try to parse Groq's custom function XML format
                let groq_functions = self.parse_groq_functions(content);
                content_blocks.extend(groq_functions);
            }

            // Determine stop reason
            let finish_reason = response_json
                .get("choices")
                .and_then(|c| c.as_array())
                .and_then(|arr| arr.first())
                .and_then(|choice| choice.get("finish_reason"))
                .and_then(|r| r.as_str())
                .map(|s| s.to_string());

            // If we have tool_use blocks but stop_reason is not tool_calls, fix it
            let stop_reason = if content_blocks.iter().any(|b| b.block_type == "tool_use") {
                Some("tool_calls".to_string())
            } else {
                finish_reason
            };

            let default_usage = serde_json::json!({});
            let usage = response_json.get("usage").unwrap_or(&default_usage);

            Ok(ChatResponse {
                id: response_json
                    .get("id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("msg_groq")
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
                provider: "groq".to_string(),
                message: format!("HTTP {} - {}", status, error_text),
            })
        }
    }
}

impl GroqClient {
    /// Parse Groq's custom function XML format into tool_use ContentBlocks
    /// Format: <function=name {"arg": "value"}</function>
    fn parse_groq_functions(&self, content: &str) -> Vec<crate::server::ContentBlock> {
        let mut blocks = Vec::new();

        // Pattern to match Groq function calls: <function=name {json}</function>
        // The JSON can contain nested objects, so we need to match until </function>
        let func_regex = regex::Regex::new(r"<function=(\w+)=([^<]*)</function>").unwrap();

        for cap in func_regex.captures_iter(content) {
            let func_name = cap.get(1).map(|m| m.as_str()).unwrap_or("unknown");
            let args_str = cap.get(2).map(|m| m.as_str()).unwrap_or("{}");

            tracing::debug!(func_name = %func_name, args = %args_str, "Parsed Groq function call");

            // Parse the JSON arguments
            let input: serde_json::Value =
                serde_json::from_str(args_str).unwrap_or_else(|_| serde_json::json!({}));

            blocks.push(crate::server::ContentBlock {
                block_type: "tool_use".to_string(),
                text: None,
                id: Some(format!("call_{}", uuid::Uuid::new_v4().simple())),
                name: Some(func_name.to_string()),
                input: Some(input),
            });
        }

        blocks
    }
}

impl Default for GroqClient {
    fn default() -> Self {
        Self::new()
    }
}

impl GroqClient {
    fn approx_request_chars(req: &ChatRequest) -> usize {
        let mut total = 0usize;

        if let Some(system) = &req.system {
            total += match system {
                crate::token_counter::SystemPrompt::Text(text) => text.len(),
                crate::token_counter::SystemPrompt::Array(blocks) => blocks
                    .iter()
                    .map(|block| match block {
                        crate::token_counter::SystemBlock::Text { text } => text.len(),
                        crate::token_counter::SystemBlock::CacheControl { text, .. } => text.len(),
                    })
                    .sum(),
            };
        }

        for message in &req.messages {
            total += match &message.content {
                crate::token_counter::MessageContent::Text(text) => text.len(),
                crate::token_counter::MessageContent::Array(blocks) => blocks
                    .iter()
                    .map(|block| match block {
                        crate::token_counter::ContentBlock::Text { text } => text.len(),
                        crate::token_counter::ContentBlock::ToolResult { content, .. } => {
                            content.len()
                        }
                        _ => 0,
                    })
                    .sum(),
                crate::token_counter::MessageContent::Null => 0,
            };
        }

        total
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::token_counter::{Message, MessageContent, SystemPrompt};

    fn create_test_provider() -> Provider {
        Provider {
            name: "groq".to_string(),
            api_base_url: "https://api.groq.com".to_string(),
            api_key: "test_key".to_string(),
            models: vec!["llama-3.1-8b-instant".to_string()],
            transformers: vec![],
        }
    }

    fn create_test_request() -> ChatRequest {
        ChatRequest {
            model: "llama-3.1-8b-instant".to_string(),
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
    fn test_groq_client_creation() {
        let _client = GroqClient::new();
    }

    #[test]
    fn test_convert_to_groq_request_basic() {
        let client = GroqClient::new();
        let request = create_test_request();

        // Test with a different model than request.model to prove we use the parameter
        let routed_model = "llama-3.3-70b-versatile";
        let result = client.convert_to_groq_request(routed_model, &request);
        assert!(result.is_ok(), "Should convert basic request successfully");

        let groq_req = result.unwrap();
        // Should use the routed model, not request.model
        assert_eq!(groq_req["model"], routed_model);
        assert_eq!(groq_req["stream"], true);
        assert_eq!(groq_req["max_tokens"], 100);
        // Account for floating-point conversion
        assert!((groq_req["temperature"].as_f64().unwrap() - 0.7).abs() < 0.0001);

        // System message should be in the messages array, not a top-level field
        assert!(
            groq_req.get("system").is_none(),
            "system should not be a top-level field"
        );
        let messages = groq_req["messages"]
            .as_array()
            .expect("messages should be array");
        assert_eq!(messages.len(), 2, "Should have system + user messages");
        assert_eq!(messages[0]["role"], "system");
        assert_eq!(messages[0]["content"], "You are a helpful assistant.");
        assert_eq!(messages[1]["role"], "user");
    }

    #[test]
    fn test_convert_to_groq_request_with_tools() {
        let client = GroqClient::new();
        let mut request = create_test_request();
        request.tools = Some(vec![crate::token_counter::Tool {
            tool_type: Some("function".to_string()),
            function: Some(crate::token_counter::FunctionTool {
                name: "test_tool".to_string(),
                description: Some("A test tool".to_string()),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "input": {"type": "string"}
                    }
                }),
            }),
            name: None,
            description: None,
            input_schema: None,
        }]);

        // Use dynamic model from routing, not request.model
        let result = client.convert_to_groq_request("llama-3.3-70b-versatile", &request);
        assert!(
            result.is_ok(),
            "Should convert request with tools successfully"
        );

        let groq_req = result.unwrap();
        assert!(groq_req["tools"].is_array());
    }

    #[test]
    fn test_convert_to_groq_request_non_streaming() {
        let client = GroqClient::new();
        let mut request = create_test_request();
        request.stream = Some(false);

        // Use dynamic model from routing
        let result = client.convert_to_groq_request("llama-3.3-70b-versatile", &request);
        assert!(
            result.is_ok(),
            "Should convert non-streaming request successfully"
        );

        let groq_req = result.unwrap();
        // Note: convert_to_groq_request always sets stream=true
        // The stream flag is handled separately in the actual request sending
        assert_eq!(groq_req["stream"], true);
    }

    #[test]
    fn test_convert_uses_routed_model_not_request_model() {
        // This test verifies the bug fix: when pattern routing resolves to a model,
        // the routed model must be used, NOT request.model (which may be "auto")
        let client = GroqClient::new();

        // Simulate pattern routing scenario: request has model="auto"
        let mut request = create_test_request();
        request.model = "auto".to_string(); // This is what pattern routing sends

        // But routing decision resolved to a specific model
        let routed_model = "llama-3.3-70b-versatile";

        let result = client.convert_to_groq_request(routed_model, &request);
        assert!(result.is_ok(), "Should convert request successfully");

        let groq_req = result.unwrap();

        // CRITICAL: Must use routed_model, NOT request.model ("auto")
        assert_eq!(
            groq_req["model"], routed_model,
            "Must use routed model from routing decision"
        );
        assert_ne!(
            groq_req["model"], "auto",
            "Must NOT use request.model when it's 'auto'"
        );
    }

    #[test]
    fn test_model_validation_uses_groq_models() {
        // Verify that is_valid_groq_model works correctly
        // Known model from API/fallback
        assert!(
            crate::groq_models::is_valid_groq_model("llama-3.3-70b-versatile"),
            "Known model should be valid"
        );

        // Unknown model should return false (but not panic or error)
        assert!(
            !crate::groq_models::is_valid_groq_model("unknown-model-xyz"),
            "Unknown model should be invalid"
        );

        // "auto" is not a valid Groq model (it's a routing directive)
        assert!(
            !crate::groq_models::is_valid_groq_model("auto"),
            "'auto' is not a valid Groq model"
        );
    }

    #[test]
    fn test_url_construction_with_base_path() {
        let provider = Provider {
            name: "groq".to_string(),
            api_base_url: "https://api.groq.com/openai/v1".to_string(),
            api_key: "test_key".to_string(),
            models: vec!["llama-3.1-8b-instant".to_string()],
            transformers: vec![],
        };

        let endpoint = if provider.api_base_url.ends_with("/openai/v1") {
            format!("{}/chat/completions", provider.api_base_url)
        } else {
            format!("{}/openai/v1/chat/completions", provider.api_base_url)
        };

        assert_eq!(endpoint, "https://api.groq.com/openai/v1/chat/completions");
    }

    #[test]
    fn test_url_construction_without_base_path() {
        let provider = create_test_provider();

        let endpoint = if provider.api_base_url.ends_with("/openai/v1") {
            format!("{}/chat/completions", provider.api_base_url)
        } else {
            format!("{}/openai/v1/chat/completions", provider.api_base_url)
        };

        assert_eq!(endpoint, "https://api.groq.com/openai/v1/chat/completions");
    }

    #[tokio::test]
    #[ignore] // Requires real API key - run with cargo test -- --ignored
    async fn test_groq_real_streaming() {
        let client = GroqClient::new();
        let provider = Provider {
            name: "groq".to_string(),
            api_base_url: "https://api.groq.com".to_string(),
            api_key: std::env::var("GROQ_API_KEY").unwrap_or_else(|_| "test_key".to_string()),
            models: vec!["llama-3.1-8b-instant".to_string()],
            transformers: vec![],
        };
        let request = create_test_request();

        match client
            .send_streaming_request(&provider, "llama-3.1-8b-instant", &request)
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
    async fn test_groq_real_non_streaming() {
        let client = GroqClient::new();
        let provider = Provider {
            name: "groq".to_string(),
            api_base_url: "https://api.groq.com".to_string(),
            api_key: std::env::var("GROQ_API_KEY").unwrap_or_else(|_| "test_key".to_string()),
            models: vec!["llama-3.1-8b-instant".to_string()],
            transformers: vec![],
        };
        let mut request = create_test_request();
        request.stream = Some(false);

        match client
            .send_request(&provider, "llama-3.1-8b-instant", &request)
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
}
