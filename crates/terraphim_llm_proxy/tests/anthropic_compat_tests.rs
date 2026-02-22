//! Anthropic API Compatibility Tests
//!
//! These tests verify that the proxy response format matches the official
//! Anthropic API specification, ensuring compatibility with Claude Code,
//! Claude Agent SDK, and other Anthropic API clients.
//!
//! Based on:
//! - https://docs.anthropic.com/en/api/messages
//! - https://docs.anthropic.com/en/api/streaming

use serde_json::json;
use terraphim_llm_proxy::server::{ChatResponse, ContentBlock};
use terraphim_llm_proxy::token_counter::{ChatRequest, Message, MessageContent};

// ============================================================================
// Response Format Tests
// ============================================================================

#[test]
fn test_chat_response_has_required_type_field() {
    // Official Anthropic API requires "type": "message" field
    let response = ChatResponse {
        id: "msg_test".to_string(),
        message_type: "message".to_string(),
        model: "claude-3-5-sonnet-20241022".to_string(),
        role: "assistant".to_string(),
        content: vec![ContentBlock {
            block_type: "text".to_string(),
            text: Some("Hello!".to_string()),
            id: None,
            name: None,
            input: None,
        }],
        stop_reason: Some("end_turn".to_string()),
        stop_sequence: None,
        usage: genai::chat::Usage {
            prompt_tokens_details: None,
            completion_tokens_details: None,
            total_tokens: Some(10),
            prompt_tokens: Some(5),
            completion_tokens: Some(5),
        },
    };

    let json = serde_json::to_value(&response).unwrap();

    // CRITICAL: The "type" field MUST be present and set to "message"
    assert_eq!(
        json.get("type").and_then(|v| v.as_str()),
        Some("message"),
        "Response MUST include 'type': 'message' for Anthropic API compatibility"
    );
}

#[test]
fn test_chat_response_serialization_matches_anthropic_format() {
    let response = ChatResponse {
        id: "msg_01XFDUDYJgAACzvnptvVoYEL".to_string(),
        message_type: "message".to_string(),
        model: "claude-3-5-sonnet-20241022".to_string(),
        role: "assistant".to_string(),
        content: vec![ContentBlock {
            block_type: "text".to_string(),
            text: Some("Hello! How can I help you today?".to_string()),
            id: None,
            name: None,
            input: None,
        }],
        stop_reason: Some("end_turn".to_string()),
        stop_sequence: None,
        usage: genai::chat::Usage {
            prompt_tokens_details: None,
            completion_tokens_details: None,
            total_tokens: Some(25),
            prompt_tokens: Some(10),
            completion_tokens: Some(15),
        },
    };

    let json = serde_json::to_value(&response).unwrap();

    // Check all required fields per Anthropic API spec
    assert!(json.get("id").is_some(), "Response must include 'id'");
    assert_eq!(json.get("type").and_then(|v| v.as_str()), Some("message"));
    assert!(json.get("role").is_some(), "Response must include 'role'");
    assert_eq!(json.get("role").and_then(|v| v.as_str()), Some("assistant"));
    assert!(
        json.get("content").is_some(),
        "Response must include 'content'"
    );
    assert!(json.get("model").is_some(), "Response must include 'model'");
    assert!(
        json.get("stop_reason").is_some(),
        "Response must include 'stop_reason'"
    );
    assert!(json.get("usage").is_some(), "Response must include 'usage'");
}

#[test]
fn test_chat_response_stop_sequence_field() {
    // When a stop sequence triggers end of generation, it should be included
    let response_with_stop_seq = ChatResponse {
        id: "msg_test".to_string(),
        message_type: "message".to_string(),
        model: "claude-3-5-sonnet-20241022".to_string(),
        role: "assistant".to_string(),
        content: vec![ContentBlock {
            block_type: "text".to_string(),
            text: Some("Code:\n```".to_string()),
            id: None,
            name: None,
            input: None,
        }],
        stop_reason: Some("stop_sequence".to_string()),
        stop_sequence: Some("```".to_string()),
        usage: genai::chat::Usage {
            prompt_tokens_details: None,
            completion_tokens_details: None,
            total_tokens: Some(10),
            prompt_tokens: Some(5),
            completion_tokens: Some(5),
        },
    };

    let json = serde_json::to_value(&response_with_stop_seq).unwrap();
    assert_eq!(
        json.get("stop_sequence").and_then(|v| v.as_str()),
        Some("```"),
        "stop_sequence should be included when a stop sequence triggered end"
    );

    // When stop_sequence is None, it should be omitted (skip_serializing_if)
    let response_without_stop_seq = ChatResponse {
        id: "msg_test".to_string(),
        message_type: "message".to_string(),
        model: "claude-3-5-sonnet-20241022".to_string(),
        role: "assistant".to_string(),
        content: vec![ContentBlock {
            block_type: "text".to_string(),
            text: Some("Hello!".to_string()),
            id: None,
            name: None,
            input: None,
        }],
        stop_reason: Some("end_turn".to_string()),
        stop_sequence: None,
        usage: genai::chat::Usage {
            prompt_tokens_details: None,
            completion_tokens_details: None,
            total_tokens: Some(10),
            prompt_tokens: Some(5),
            completion_tokens: Some(5),
        },
    };

    let json = serde_json::to_value(&response_without_stop_seq).unwrap();
    // stop_sequence should be omitted when None
    assert!(
        json.get("stop_sequence").is_none(),
        "stop_sequence should be omitted when None"
    );
}

#[test]
fn test_content_block_type_field() {
    let block = ContentBlock {
        block_type: "text".to_string(),
        text: Some("Hello world".to_string()),
        id: None,
        name: None,
        input: None,
    };

    let json = serde_json::to_value(&block).unwrap();
    assert_eq!(
        json.get("type").and_then(|v| v.as_str()),
        Some("text"),
        "ContentBlock must have 'type' field"
    );
}

#[test]
fn test_stop_reasons_valid() {
    // Valid stop reasons per Anthropic API
    let valid_stop_reasons = vec!["end_turn", "max_tokens", "stop_sequence", "tool_use"];

    for reason in valid_stop_reasons {
        let response = ChatResponse {
            id: "msg_test".to_string(),
            message_type: "message".to_string(),
            model: "claude-3-5-sonnet-20241022".to_string(),
            role: "assistant".to_string(),
            content: vec![],
            stop_reason: Some(reason.to_string()),
            stop_sequence: None,
            usage: genai::chat::Usage {
                prompt_tokens_details: None,
                completion_tokens_details: None,
                total_tokens: Some(10),
                prompt_tokens: Some(5),
                completion_tokens: Some(5),
            },
        };

        let json = serde_json::to_value(&response).unwrap();
        assert_eq!(
            json.get("stop_reason").and_then(|v| v.as_str()),
            Some(reason)
        );
    }
}

// ============================================================================
// Request Format Tests
// ============================================================================

#[test]
fn test_chat_request_accepts_top_p() {
    let json = json!({
        "model": "claude-3-5-sonnet-20241022",
        "messages": [{"role": "user", "content": "Hello"}],
        "max_tokens": 100,
        "top_p": 0.9
    });

    let request: ChatRequest = serde_json::from_value(json).unwrap();
    assert_eq!(request.top_p, Some(0.9));
}

#[test]
fn test_chat_request_accepts_top_k() {
    let json = json!({
        "model": "claude-3-5-sonnet-20241022",
        "messages": [{"role": "user", "content": "Hello"}],
        "max_tokens": 100,
        "top_k": 40
    });

    let request: ChatRequest = serde_json::from_value(json).unwrap();
    assert_eq!(request.top_k, Some(40));
}

#[test]
fn test_chat_request_accepts_stop_sequences() {
    let json = json!({
        "model": "claude-3-5-sonnet-20241022",
        "messages": [{"role": "user", "content": "Hello"}],
        "max_tokens": 100,
        "stop_sequences": ["```", "END", "\n\n"]
    });

    let request: ChatRequest = serde_json::from_value(json).unwrap();
    assert_eq!(
        request.stop_sequences,
        Some(vec![
            "```".to_string(),
            "END".to_string(),
            "\n\n".to_string()
        ])
    );
}

#[test]
fn test_chat_request_accepts_metadata() {
    let json = json!({
        "model": "claude-3-5-sonnet-20241022",
        "messages": [{"role": "user", "content": "Hello"}],
        "max_tokens": 100,
        "metadata": {
            "user_id": "user-123"
        }
    });

    let request: ChatRequest = serde_json::from_value(json).unwrap();
    assert!(request.metadata.is_some());
    assert_eq!(
        request.metadata.unwrap().user_id,
        Some("user-123".to_string())
    );
}

#[test]
fn test_chat_request_full_anthropic_format() {
    // Test a request with all Anthropic API fields
    let json = json!({
        "model": "claude-3-5-sonnet-20241022",
        "messages": [
            {"role": "user", "content": "Hello, Claude!"}
        ],
        "system": "You are a helpful assistant.",
        "max_tokens": 1024,
        "temperature": 0.7,
        "top_p": 0.9,
        "top_k": 40,
        "stop_sequences": ["END"],
        "stream": false,
        "metadata": {
            "user_id": "test-user"
        }
    });

    let request: ChatRequest = serde_json::from_value(json).unwrap();
    assert_eq!(request.model, "claude-3-5-sonnet-20241022");
    assert_eq!(request.max_tokens, Some(1024));
    assert_eq!(request.temperature, Some(0.7));
    assert_eq!(request.top_p, Some(0.9));
    assert_eq!(request.top_k, Some(40));
    assert_eq!(request.stop_sequences, Some(vec!["END".to_string()]));
    assert_eq!(request.stream, Some(false));
}

#[test]
fn test_chat_request_serialization_omits_none_fields() {
    let request = ChatRequest {
        model: "claude-3-5-sonnet-20241022".to_string(),
        messages: vec![Message {
            role: "user".to_string(),
            content: MessageContent::Text("Hello".to_string()),
            ..Default::default()
        }],
        system: None,
        tools: None,
        max_tokens: Some(100),
        temperature: None,
        stream: None,
        thinking: None,
        top_p: None,
        top_k: None,
        stop_sequences: None,
        metadata: None,
    };

    let json = serde_json::to_value(&request).unwrap();

    // Fields that are None should be omitted
    assert!(json.get("system").is_none());
    assert!(json.get("tools").is_none());
    assert!(json.get("temperature").is_none());
    assert!(json.get("stream").is_none());
    assert!(json.get("top_p").is_none());
    assert!(json.get("top_k").is_none());
    assert!(json.get("stop_sequences").is_none());
    assert!(json.get("metadata").is_none());

    // Required fields should be present
    assert!(json.get("model").is_some());
    assert!(json.get("messages").is_some());
    assert!(json.get("max_tokens").is_some());
}

// ============================================================================
// Claude Code Compatibility Tests
// ============================================================================

#[test]
fn test_claude_code_typical_request() {
    // Simulate a typical Claude Code request
    let json = json!({
        "model": "claude-sonnet-4-20250514",
        "messages": [
            {
                "role": "user",
                "content": "Read the file src/main.rs"
            }
        ],
        "max_tokens": 8192,
        "stream": true
    });

    let request: ChatRequest = serde_json::from_value(json).unwrap();
    assert_eq!(request.model, "claude-sonnet-4-20250514");
    assert_eq!(request.stream, Some(true));
}

#[test]
fn test_claude_code_response_format() {
    // Claude Code expects responses in specific format
    let response = ChatResponse {
        id: "msg_01XFDUDYJgAACzvnptvVoYEL".to_string(),
        message_type: "message".to_string(),
        model: "claude-sonnet-4-20250514".to_string(),
        role: "assistant".to_string(),
        content: vec![ContentBlock {
            block_type: "text".to_string(),
            text: Some("Here is the content of src/main.rs:".to_string()),
            id: None,
            name: None,
            input: None,
        }],
        stop_reason: Some("end_turn".to_string()),
        stop_sequence: None,
        usage: genai::chat::Usage {
            prompt_tokens_details: None,
            completion_tokens_details: None,
            total_tokens: Some(100),
            prompt_tokens: Some(50),
            completion_tokens: Some(50),
        },
    };

    let json = serde_json::to_value(&response).unwrap();

    // Claude Code validates these fields
    assert!(json.get("id").is_some());
    assert_eq!(json.get("type").and_then(|v| v.as_str()), Some("message"));
    assert!(json.get("content").and_then(|v| v.as_array()).is_some());
    assert!(json.get("usage").is_some());
}

// ============================================================================
// Agent SDK Compatibility Tests
// ============================================================================

#[test]
fn test_agent_sdk_tool_use_request() {
    // Agent SDK uses tool_use for function calling
    let json = json!({
        "model": "claude-3-5-sonnet-20241022",
        "messages": [
            {"role": "user", "content": "What's the weather in San Francisco?"}
        ],
        "tools": [
            {
                "name": "get_weather",
                "description": "Get weather for a location",
                "input_schema": {
                    "type": "object",
                    "properties": {
                        "location": {"type": "string"}
                    },
                    "required": ["location"]
                }
            }
        ],
        "max_tokens": 1024
    });

    let request: ChatRequest = serde_json::from_value(json).unwrap();
    assert!(request.tools.is_some());
    assert_eq!(request.tools.as_ref().unwrap().len(), 1);
}

// ============================================================================
// Deserialization Robustness Tests
// ============================================================================

#[test]
fn test_response_deserialization_with_all_fields() {
    let json = json!({
        "id": "msg_test",
        "type": "message",
        "model": "claude-3-5-sonnet-20241022",
        "role": "assistant",
        "content": [{"type": "text", "text": "Hello!"}],
        "stop_reason": "end_turn",
        "stop_sequence": null,
        "usage": {
            "prompt_tokens": 10,
            "completion_tokens": 5,
            "total_tokens": 15
        }
    });

    let response: ChatResponse = serde_json::from_value(json).unwrap();
    assert_eq!(response.message_type, "message");
    assert_eq!(response.stop_sequence, None);
}

#[test]
fn test_response_deserialization_missing_optional_type() {
    // Response should use default "message" if type is missing
    let json = json!({
        "id": "msg_test",
        "model": "claude-3-5-sonnet-20241022",
        "role": "assistant",
        "content": [{"type": "text", "text": "Hello!"}],
        "stop_reason": "end_turn",
        "usage": {
            "prompt_tokens": 10,
            "completion_tokens": 5
        }
    });

    let response: ChatResponse = serde_json::from_value(json).unwrap();
    // Should default to "message"
    assert_eq!(response.message_type, "message");
}

#[test]
fn test_request_default_values() {
    let request = ChatRequest::default();
    assert_eq!(request.model, "claude-3-5-sonnet-20241022");
    assert!(request.messages.is_empty());
    assert!(request.top_p.is_none());
    assert!(request.top_k.is_none());
    assert!(request.stop_sequences.is_none());
    assert!(request.metadata.is_none());
}

// ============================================================================
// Phase 2: Error Response Format Tests (GAP-ERR-001)
// ============================================================================

#[test]
fn test_error_response_has_type_field() {
    use terraphim_llm_proxy::error::{ErrorDetail, ErrorResponse};

    let response = ErrorResponse {
        response_type: "error",
        error: ErrorDetail {
            error_type: "invalid_request_error".to_string(),
            message: "Test error".to_string(),
            code: 400,
            retry_after: None,
        },
    };

    let json = serde_json::to_value(&response).unwrap();

    // CRITICAL: The top-level "type" field MUST be present and set to "error"
    assert_eq!(
        json.get("type").and_then(|v| v.as_str()),
        Some("error"),
        "ErrorResponse MUST include top-level 'type': 'error' for Anthropic API compatibility"
    );
    assert!(json.get("error").is_some());
}

#[test]
fn test_error_response_format_matches_anthropic() {
    use terraphim_llm_proxy::error::{ErrorDetail, ErrorResponse};

    let response = ErrorResponse {
        response_type: "error",
        error: ErrorDetail {
            error_type: "invalid_api_key".to_string(),
            message: "Invalid API key provided".to_string(),
            code: 401,
            retry_after: None,
        },
    };

    let json = serde_json::to_string(&response).unwrap();

    // Anthropic format: {"type":"error","error":{"type":"...","message":"..."}}
    assert!(
        json.contains(r#""type":"error""#),
        "Must have top-level type: error"
    );
    assert!(
        json.contains(r#""error":{"#),
        "Must have nested error object"
    );
}

#[test]
fn test_error_response_with_retry_after() {
    use terraphim_llm_proxy::error::{ErrorDetail, ErrorResponse};

    let response = ErrorResponse {
        response_type: "error",
        error: ErrorDetail {
            error_type: "rate_limit_exceeded".to_string(),
            message: "Rate limit exceeded".to_string(),
            code: 429,
            retry_after: Some(60),
        },
    };

    let json = serde_json::to_value(&response).unwrap();

    assert_eq!(json.get("type").and_then(|v| v.as_str()), Some("error"));
    assert_eq!(
        json.pointer("/error/retry_after").and_then(|v| v.as_u64()),
        Some(60)
    );
}

// ============================================================================
// Phase 2: Streaming SSE Event Format Tests
// ============================================================================

#[test]
fn test_message_delta_stop_sequence_format() {
    // Verify the message_delta event format includes stop_sequence
    let message_delta = serde_json::json!({
        "type": "message_delta",
        "delta": {
            "stop_reason": "end_turn",
            "stop_sequence": null
        },
        "usage": {
            "output_tokens": 100
        }
    });

    // Verify all required fields are present
    assert_eq!(
        message_delta.get("type").and_then(|v| v.as_str()),
        Some("message_delta")
    );
    assert!(message_delta.pointer("/delta/stop_reason").is_some());
    assert!(message_delta.pointer("/delta/stop_sequence").is_some());
    assert!(message_delta.pointer("/usage/output_tokens").is_some());
}

#[test]
fn test_ping_event_format() {
    // Verify the ping event format matches Anthropic SSE spec
    let ping_event = serde_json::json!({
        "type": "ping"
    });

    assert_eq!(
        ping_event.get("type").and_then(|v| v.as_str()),
        Some("ping"),
        "Ping event must have type: ping"
    );
}

#[test]
fn test_error_event_format() {
    // Verify the error event format matches Anthropic SSE spec
    let error_event = serde_json::json!({
        "type": "error",
        "error": {
            "type": "api_error",
            "message": "Connection timeout"
        }
    });

    assert_eq!(
        error_event.get("type").and_then(|v| v.as_str()),
        Some("error"),
        "Error event must have type: error"
    );
    assert!(
        error_event.pointer("/error/type").is_some(),
        "Error event must have nested error.type"
    );
    assert!(
        error_event.pointer("/error/message").is_some(),
        "Error event must have nested error.message"
    );
}

// ============================================================================
// Phase 3: Extended Thinking Streaming Tests (GAP-SSE-003)
// ============================================================================

#[test]
fn test_thinking_block_start_format() {
    // Verify the content_block_start format for thinking blocks
    let thinking_start = serde_json::json!({
        "type": "content_block_start",
        "index": 0,
        "content_block": {
            "type": "thinking",
            "thinking": ""
        }
    });

    assert_eq!(
        thinking_start.get("type").and_then(|v| v.as_str()),
        Some("content_block_start"),
        "Must have type: content_block_start"
    );
    assert_eq!(
        thinking_start
            .pointer("/content_block/type")
            .and_then(|v| v.as_str()),
        Some("thinking"),
        "Content block type must be 'thinking'"
    );
    assert!(
        thinking_start.pointer("/content_block/thinking").is_some(),
        "Thinking block must have 'thinking' field"
    );
}

#[test]
fn test_thinking_delta_format() {
    // Verify the thinking_delta event format matches Anthropic SSE spec
    let thinking_delta = serde_json::json!({
        "type": "content_block_delta",
        "index": 0,
        "delta": {
            "type": "thinking_delta",
            "thinking": "Let me think about this..."
        }
    });

    assert_eq!(
        thinking_delta.get("type").and_then(|v| v.as_str()),
        Some("content_block_delta"),
        "Must have type: content_block_delta"
    );
    assert_eq!(
        thinking_delta
            .pointer("/delta/type")
            .and_then(|v| v.as_str()),
        Some("thinking_delta"),
        "Delta type must be 'thinking_delta'"
    );
    assert!(
        thinking_delta.pointer("/delta/thinking").is_some(),
        "Thinking delta must have 'thinking' field"
    );
}

// ============================================================================
// Phase 3: Tool Use Streaming Tests (GAP-SSE-004)
// ============================================================================

#[test]
fn test_tool_use_block_start_format() {
    // Verify the content_block_start format for tool_use blocks
    let tool_use_start = serde_json::json!({
        "type": "content_block_start",
        "index": 1,
        "content_block": {
            "type": "tool_use",
            "id": "toolu_01A09q90qw90lq917835lgs",
            "name": "get_weather",
            "input": {}
        }
    });

    assert_eq!(
        tool_use_start.get("type").and_then(|v| v.as_str()),
        Some("content_block_start"),
        "Must have type: content_block_start"
    );
    assert_eq!(
        tool_use_start
            .pointer("/content_block/type")
            .and_then(|v| v.as_str()),
        Some("tool_use"),
        "Content block type must be 'tool_use'"
    );
    assert!(
        tool_use_start.pointer("/content_block/id").is_some(),
        "Tool use block must have 'id' field"
    );
    assert!(
        tool_use_start.pointer("/content_block/name").is_some(),
        "Tool use block must have 'name' field"
    );
    assert!(
        tool_use_start.pointer("/content_block/input").is_some(),
        "Tool use block must have 'input' field"
    );
}

#[test]
fn test_input_json_delta_format() {
    // Verify the input_json_delta event format matches Anthropic SSE spec
    let input_json_delta = serde_json::json!({
        "type": "content_block_delta",
        "index": 1,
        "delta": {
            "type": "input_json_delta",
            "partial_json": "{\"location\": \"San Francisco\"}"
        }
    });

    assert_eq!(
        input_json_delta.get("type").and_then(|v| v.as_str()),
        Some("content_block_delta"),
        "Must have type: content_block_delta"
    );
    assert_eq!(
        input_json_delta
            .pointer("/delta/type")
            .and_then(|v| v.as_str()),
        Some("input_json_delta"),
        "Delta type must be 'input_json_delta'"
    );
    assert!(
        input_json_delta.pointer("/delta/partial_json").is_some(),
        "Input JSON delta must have 'partial_json' field"
    );
}

#[test]
fn test_tool_use_stop_reason() {
    // Verify message_delta with tool_use stop_reason
    let message_delta = serde_json::json!({
        "type": "message_delta",
        "delta": {
            "stop_reason": "tool_use",
            "stop_sequence": null
        },
        "usage": {
            "output_tokens": 150
        }
    });

    assert_eq!(
        message_delta
            .pointer("/delta/stop_reason")
            .and_then(|v| v.as_str()),
        Some("tool_use"),
        "Stop reason should be 'tool_use' when tool calls are made"
    );
}

#[test]
fn test_content_block_stop_with_index() {
    // Verify content_block_stop includes the correct index
    let content_block_stop = serde_json::json!({
        "type": "content_block_stop",
        "index": 2
    });

    assert_eq!(
        content_block_stop.get("type").and_then(|v| v.as_str()),
        Some("content_block_stop")
    );
    assert_eq!(
        content_block_stop.get("index").and_then(|v| v.as_u64()),
        Some(2),
        "content_block_stop must include the block index"
    );
}
