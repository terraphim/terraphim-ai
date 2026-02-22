//! Kimi (Moonshot AI)-specific client for Anthropic-compatible endpoint handling.

use crate::{
    config::Provider,
    server::ChatResponse,
    token_counter::{ChatRequest, Tool},
    ProxyError, Result,
};
use futures::Stream;
use serde_json::Value;
use std::pin::Pin;

pub struct KimiClient {
    client: reqwest::Client,
}

impl Default for KimiClient {
    fn default() -> Self {
        Self::new()
    }
}

impl KimiClient {
    pub fn new() -> Self {
        let client = reqwest::Client::builder()
            .no_proxy()
            .http1_only()
            .timeout(std::time::Duration::from_secs(300))
            .build()
            .map_err(|e| ProxyError::ProviderError {
                provider: "kimi".to_string(),
                message: format!("Failed to create HTTP client: {}", e),
            })
            .unwrap();

        Self { client }
    }

    fn build_messages_endpoint(base: &str) -> String {
        let trimmed = base.trim_end_matches('/');
        if trimmed.ends_with("/messages") {
            trimmed.to_string()
        } else if trimmed.ends_with("/v1") {
            format!("{}/messages", trimmed)
        } else {
            format!("{}/v1/messages", trimmed)
        }
    }

    pub async fn send_request(
        &self,
        provider: &Provider,
        model: &str,
        request: &ChatRequest,
    ) -> Result<ChatResponse> {
        let endpoint = Self::build_messages_endpoint(&provider.api_base_url);

        let mut kimi_request = request.clone();
        kimi_request.model = model.to_string();
        kimi_request.stream = Some(false);

        // Convert OpenAI-format tools to Anthropic format for Kimi
        if let Some(ref tools) = kimi_request.tools {
            let converted: Vec<Tool> = tools
                .iter()
                .map(|t| Self::convert_tool_to_anthropic(t))
                .collect();
            kimi_request.tools = Some(converted);
        }

        // Convert "developer" role to "system" as Kimi doesn't support developer role
        for msg in &mut kimi_request.messages {
            if msg.role == "developer" {
                msg.role = "system".to_string();
            }
        }

        tracing::info!(
            provider = %provider.name,
            endpoint = %endpoint,
            model = %model,
            "Resolved service target (Kimi direct): adapter=Anthropic"
        );

        let response = self
            .client
            .post(&endpoint)
            .header("x-api-key", &provider.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("Content-Type", "application/json")
            .header("User-Agent", "Terraphim-LLM-Proxy/1.0")
            .json(&kimi_request)
            .send()
            .await
            .map_err(|e| ProxyError::ProviderError {
                provider: "kimi".to_string(),
                message: format!("HTTP request failed: {}", e),
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(ProxyError::ProviderError {
                provider: "kimi".to_string(),
                message: format!("HTTP {}: {}", status, error_text),
            });
        }

        let response_json: Value =
            response
                .json()
                .await
                .map_err(|e| ProxyError::ProviderError {
                    provider: "kimi".to_string(),
                    message: format!("Failed to parse response: {}", e),
                })?;

        Ok(Self::convert_anthropic_response(model, &response_json))
    }

    /// Send streaming request to Kimi using Anthropic-compatible API
    pub async fn send_streaming_request(
        &self,
        provider: &Provider,
        model: &str,
        request: &ChatRequest,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<String>> + Send>>> {
        let endpoint = Self::build_messages_endpoint(&provider.api_base_url);

        // Build the Anthropic-format request
        let mut anthropic_request = serde_json::json!({
            "model": model,
            "stream": true,
            "max_tokens": request.max_tokens.unwrap_or(4096),
        });

        // Add system message if present
        if let Some(system) = &request.system {
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
            anthropic_request["system"] = serde_json::json!(system_text);
        }

        // Convert messages to Anthropic format
        let mut messages: Vec<Value> = Vec::new();
        for msg in &request.messages {
            let mut msg_value = serde_json::to_value(msg)?;
            // Convert "developer" role to "system" as Kimi doesn't support developer role
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
        anthropic_request["messages"] = serde_json::json!(messages);

        // Convert tools to Anthropic format if present
        if let Some(ref tools) = request.tools {
            let converted: Vec<Tool> = tools
                .iter()
                .map(|t| Self::convert_tool_to_anthropic(t))
                .collect();
            anthropic_request["tools"] = serde_json::to_value(&converted)?;
        }

        // Add temperature if present
        if let Some(temperature) = request.temperature {
            anthropic_request["temperature"] = serde_json::json!(temperature);
        }

        tracing::info!(
            provider = %provider.name,
            endpoint = %endpoint,
            model = %model,
            "Resolved service target (Kimi streaming): adapter=Anthropic"
        );

        // Send request using raw HTTP streaming instead of EventSource
        // Kimi uses non-standard SSE format (no space after colon)
        let response = self
            .client
            .post(&endpoint)
            .header("x-api-key", &provider.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("Content-Type", "application/json")
            .header("Accept", "text/event-stream")
            .header("User-Agent", "Terraphim-LLM-Proxy/1.0")
            .json(&anthropic_request)
            .send()
            .await
            .map_err(|e| {
                tracing::error!(error = %e, "Failed to send streaming request to Kimi");
                ProxyError::ProviderError {
                    provider: "kimi".to_string(),
                    message: format!("HTTP request failed: {}", e),
                }
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            tracing::error!(status = %status, error = %error_text, "Kimi streaming request failed");
            return Err(ProxyError::ProviderError {
                provider: "kimi".to_string(),
                message: format!("HTTP error {}: {}", status, error_text),
            });
        }

        tracing::info!("Kimi streaming response received, starting SSE parsing");

        // Convert response body to stream of bytes
        let byte_stream = response.bytes_stream();

        // Create a stream that yields complete SSE events
        // client.rs expects full SSE format: "event: XXX\ndata: YYY\n\n"
        let stream = async_stream::try_stream! {
            let mut buffer = String::new();
            let mut current_event = String::new();

            for await chunk in byte_stream {
                let chunk = chunk.map_err(|e| ProxyError::ProviderError {
                    provider: "kimi".to_string(),
                    message: format!("Stream error: {}", e),
                })?;

                // Append chunk to buffer
                buffer.push_str(&String::from_utf8_lossy(&chunk));

                // Process complete lines
                while let Some(pos) = buffer.find('\n') {
                    let line = buffer[..pos].trim_end_matches('\r').to_string();
                    buffer = buffer[pos + 1..].to_string();

                    if line.is_empty() {
                        // Empty line means end of event - yield accumulated event
                        if !current_event.is_empty() {
                            tracing::info!(event = %current_event, "Yielding complete SSE event from Kimi");
                            yield current_event.clone();
                            current_event.clear();
                        }
                    } else {
                        // Accumulate event lines
                        if !current_event.is_empty() {
                            current_event.push('\n');
                        }
                        current_event.push_str(&line);
                    }
                }
            }

            // Process any remaining data
            if !buffer.is_empty() {
                if !current_event.is_empty() {
                    current_event.push('\n');
                }
                current_event.push_str(buffer.trim_end_matches('\r'));
            }
            if !current_event.is_empty() {
                yield current_event;
            }
        };

        Ok(Box::pin(stream))
    }

    /// Convert an OpenAI-format tool to Anthropic format for Kimi.
    /// OpenAI: { type: "function", function: { name, description, parameters } }
    /// Anthropic: { name, description, input_schema }
    fn convert_tool_to_anthropic(tool: &Tool) -> Tool {
        if let Some(ref func) = tool.function {
            Tool {
                tool_type: None,
                function: None,
                name: Some(func.name.clone()),
                description: func.description.clone(),
                input_schema: Some(func.parameters.clone()),
            }
        } else {
            // Already in Anthropic format or unknown - pass through
            tool.clone()
        }
    }

    fn convert_anthropic_response(model: &str, response_json: &Value) -> ChatResponse {
        let mut content_blocks = Vec::new();

        if let Some(content) = response_json.get("content").and_then(|v| v.as_array()) {
            for block in content {
                let block_type = block.get("type").and_then(|v| v.as_str()).unwrap_or("");
                match block_type {
                    "text" => {
                        let text = block
                            .get("text")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();
                        content_blocks.push(crate::server::ContentBlock {
                            block_type: "text".to_string(),
                            text: Some(text),
                            id: None,
                            name: None,
                            input: None,
                        });
                    }
                    "tool_use" => {
                        content_blocks.push(crate::server::ContentBlock {
                            block_type: "tool_use".to_string(),
                            text: None,
                            id: block
                                .get("id")
                                .and_then(|v| v.as_str())
                                .map(|s| s.to_string()),
                            name: block
                                .get("name")
                                .and_then(|v| v.as_str())
                                .map(|s| s.to_string()),
                            input: block.get("input").cloned(),
                        });
                    }
                    _ => {}
                }
            }
        }

        if content_blocks.is_empty() {
            content_blocks.push(crate::server::ContentBlock {
                block_type: "text".to_string(),
                text: Some(String::new()),
                id: None,
                name: None,
                input: None,
            });
        }

        let usage = response_json.get("usage").cloned().unwrap_or(Value::Null);
        let input_tokens = usage
            .get("input_tokens")
            .and_then(|v| v.as_u64())
            .map(|v| v as i32);
        let output_tokens = usage
            .get("output_tokens")
            .and_then(|v| v.as_u64())
            .map(|v| v as i32);
        let total_tokens = match (input_tokens, output_tokens) {
            (Some(i), Some(o)) => Some(i + o),
            _ => None,
        };

        ChatResponse {
            id: response_json
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or("msg_kimi")
                .to_string(),
            message_type: "message".to_string(),
            model: response_json
                .get("model")
                .and_then(|v| v.as_str())
                .unwrap_or(model)
                .to_string(),
            role: response_json
                .get("role")
                .and_then(|v| v.as_str())
                .unwrap_or("assistant")
                .to_string(),
            content: content_blocks,
            stop_reason: response_json
                .get("stop_reason")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            stop_sequence: None,
            usage: genai::chat::Usage {
                prompt_tokens_details: None,
                completion_tokens_details: None,
                prompt_tokens: input_tokens,
                completion_tokens: output_tokens,
                total_tokens,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::KimiClient;
    use crate::token_counter::{FunctionTool, Tool};

    #[test]
    fn test_build_messages_endpoint() {
        assert_eq!(
            KimiClient::build_messages_endpoint("https://api.kimi.com/coding"),
            "https://api.kimi.com/coding/v1/messages"
        );
        assert_eq!(
            KimiClient::build_messages_endpoint("https://api.kimi.com/coding/v1"),
            "https://api.kimi.com/coding/v1/messages"
        );
        assert_eq!(
            KimiClient::build_messages_endpoint("https://api.kimi.com/coding/v1/messages"),
            "https://api.kimi.com/coding/v1/messages"
        );
    }

    #[test]
    fn test_convert_anthropic_response_text_usage() {
        let response_json = serde_json::json!({
            "id": "msg_1",
            "model": "kimi-for-coding",
            "role": "assistant",
            "content": [
                { "type": "text", "text": "hello" }
            ],
            "stop_reason": "end_turn",
            "usage": {
                "input_tokens": 10,
                "output_tokens": 5
            }
        });

        let response = KimiClient::convert_anthropic_response("kimi-for-coding", &response_json);
        assert_eq!(response.id, "msg_1");
        assert_eq!(response.model, "kimi-for-coding");
        assert_eq!(response.content.len(), 1);
        assert_eq!(response.content[0].block_type, "text");
        assert_eq!(response.content[0].text.as_deref(), Some("hello"));
        assert_eq!(response.usage.prompt_tokens, Some(10));
        assert_eq!(response.usage.completion_tokens, Some(5));
        assert_eq!(response.usage.total_tokens, Some(15));
    }

    #[test]
    fn test_convert_tool_openai_to_anthropic() {
        let openai_tool = Tool {
            tool_type: Some("function".to_string()),
            function: Some(FunctionTool {
                name: "calculator".to_string(),
                description: Some("Evaluate a math expression".to_string()),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "expression": { "type": "string" }
                    },
                    "required": ["expression"]
                }),
            }),
            name: None,
            description: None,
            input_schema: None,
        };

        let converted = KimiClient::convert_tool_to_anthropic(&openai_tool);

        // Should be Anthropic format: no type/function, has name/description/input_schema
        assert!(converted.tool_type.is_none());
        assert!(converted.function.is_none());
        assert_eq!(converted.name.as_deref(), Some("calculator"));
        assert_eq!(
            converted.description.as_deref(),
            Some("Evaluate a math expression")
        );
        assert!(converted.input_schema.is_some());
        let schema = converted.input_schema.unwrap();
        assert_eq!(schema["type"], "object");
        assert!(schema["properties"]["expression"].is_object());
    }

    #[test]
    fn test_convert_tool_already_anthropic_passthrough() {
        let anthropic_tool = Tool {
            tool_type: None,
            function: None,
            name: Some("search".to_string()),
            description: Some("Search the web".to_string()),
            input_schema: Some(serde_json::json!({"type": "object"})),
        };

        let converted = KimiClient::convert_tool_to_anthropic(&anthropic_tool);
        assert_eq!(converted.name.as_deref(), Some("search"));
        assert_eq!(converted.description.as_deref(), Some("Search the web"));
        assert!(converted.input_schema.is_some());
    }
}
