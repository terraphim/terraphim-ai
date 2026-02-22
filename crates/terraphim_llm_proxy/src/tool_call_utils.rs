//! Shared utility functions for extracting and converting tool_calls
//! across provider client implementations (non-streaming and streaming).

use crate::server::ContentBlock;
use genai::chat::{ChatStreamEvent, ToolCall, ToolChunk};
use serde_json::Value;
use std::collections::HashMap;

/// Extract tool_calls from an OpenAI-format response JSON into ContentBlock entries.
/// Parses `choices[0].message.tool_calls` array and creates `tool_use` blocks.
/// Returns empty Vec if no tool_calls are present.
pub fn extract_tool_calls_from_response(response_json: &Value) -> Vec<ContentBlock> {
    let tool_calls = response_json
        .get("choices")
        .and_then(|c| c.as_array())
        .and_then(|arr| arr.first())
        .and_then(|choice| choice.get("message"))
        .and_then(|msg| msg.get("tool_calls"));

    let Some(tools) = tool_calls else {
        return Vec::new();
    };
    let Some(tools_array) = tools.as_array() else {
        return Vec::new();
    };

    tools_array
        .iter()
        .filter_map(|tool| {
            let id = tool.get("id").and_then(|v| v.as_str())?;
            let func = tool.get("function")?;
            let name = func
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");
            let args = func
                .get("arguments")
                .and_then(|v| v.as_str())
                .unwrap_or("{}");
            let input: Value = serde_json::from_str(args).unwrap_or_else(|_| serde_json::json!({}));

            Some(ContentBlock {
                block_type: "tool_use".to_string(),
                text: None,
                id: Some(id.to_string()),
                name: Some(name.to_string()),
                input: Some(input),
            })
        })
        .collect()
}

/// Determine the correct stop_reason given content blocks.
/// If any `tool_use` block is present, returns "tool_calls".
/// Otherwise returns the original finish_reason from the provider response.
pub fn resolve_stop_reason(
    content_blocks: &[ContentBlock],
    provider_finish_reason: Option<String>,
) -> Option<String> {
    if content_blocks.iter().any(|b| b.block_type == "tool_use") {
        Some("tool_calls".to_string())
    } else {
        provider_finish_reason
    }
}

/// State for tracking streaming tool call deltas.
/// Maps OpenAI tool_call index -> (call_id, fn_name).
pub type StreamingToolCallState = HashMap<u64, (String, String)>;

/// Parse a streaming delta for tool_calls and return a ChatStreamEvent if applicable.
/// Updates state with call_id and fn_name from the first chunk of each index.
/// Returns None if no tool_calls are present in the delta.
pub fn parse_streaming_tool_call_delta(
    delta: &Value,
    state: &mut StreamingToolCallState,
) -> Option<ChatStreamEvent> {
    let tool_calls = delta.get("tool_calls")?.as_array()?;
    let tc = tool_calls.first()?;

    let index = tc.get("index").and_then(|i| i.as_u64()).unwrap_or(0);

    // Store id and name from first chunk (subsequent chunks may omit them)
    if let Some(id) = tc.get("id").and_then(|i| i.as_str()) {
        let name = tc
            .get("function")
            .and_then(|f| f.get("name"))
            .and_then(|n| n.as_str())
            .unwrap_or("")
            .to_string();
        state.insert(index, (id.to_string(), name));
    }

    let (call_id, fn_name) = state.get(&index)?;

    let arguments = tc
        .get("function")
        .and_then(|f| f.get("arguments"))
        .and_then(|a| a.as_str())
        .unwrap_or("");

    Some(ChatStreamEvent::ToolCallChunk(ToolChunk {
        tool_call: ToolCall {
            call_id: call_id.clone(),
            fn_name: fn_name.clone(),
            fn_arguments: serde_json::Value::String(arguments.to_string()),
            thought_signatures: None,
        },
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_tool_calls_with_single_tool() {
        let response = serde_json::json!({
            "choices": [{
                "message": {
                    "content": "",
                    "tool_calls": [{
                        "id": "call_123",
                        "type": "function",
                        "function": {
                            "name": "get_weather",
                            "arguments": "{\"city\":\"London\"}"
                        }
                    }]
                },
                "finish_reason": "tool_calls"
            }]
        });
        let blocks = extract_tool_calls_from_response(&response);
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].block_type, "tool_use");
        assert_eq!(blocks[0].id.as_deref(), Some("call_123"));
        assert_eq!(blocks[0].name.as_deref(), Some("get_weather"));
        assert_eq!(blocks[0].input, Some(serde_json::json!({"city": "London"})));
    }

    #[test]
    fn test_extract_tool_calls_without_tools() {
        let response = serde_json::json!({
            "choices": [{
                "message": {
                    "content": "Hello world"
                },
                "finish_reason": "stop"
            }]
        });
        let blocks = extract_tool_calls_from_response(&response);
        assert!(blocks.is_empty());
    }

    #[test]
    fn test_extract_multiple_tool_calls() {
        let response = serde_json::json!({
            "choices": [{
                "message": {
                    "content": null,
                    "tool_calls": [
                        {
                            "id": "call_1",
                            "type": "function",
                            "function": { "name": "fn_a", "arguments": "{}" }
                        },
                        {
                            "id": "call_2",
                            "type": "function",
                            "function": { "name": "fn_b", "arguments": "{\"x\":1}" }
                        }
                    ]
                },
                "finish_reason": "tool_calls"
            }]
        });
        let blocks = extract_tool_calls_from_response(&response);
        assert_eq!(blocks.len(), 2);
        assert_eq!(blocks[0].name.as_deref(), Some("fn_a"));
        assert_eq!(blocks[1].name.as_deref(), Some("fn_b"));
        assert_eq!(blocks[1].input, Some(serde_json::json!({"x": 1})));
    }

    #[test]
    fn test_extract_tool_calls_invalid_arguments_json() {
        let response = serde_json::json!({
            "choices": [{
                "message": {
                    "tool_calls": [{
                        "id": "call_bad",
                        "type": "function",
                        "function": {
                            "name": "broken",
                            "arguments": "not valid json{{"
                        }
                    }]
                }
            }]
        });
        let blocks = extract_tool_calls_from_response(&response);
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].input, Some(serde_json::json!({})));
    }

    #[test]
    fn test_resolve_stop_reason_with_tool_use() {
        let blocks = vec![ContentBlock {
            block_type: "tool_use".to_string(),
            text: None,
            id: Some("x".into()),
            name: Some("y".into()),
            input: Some(serde_json::json!({})),
        }];
        assert_eq!(
            resolve_stop_reason(&blocks, Some("stop".to_string())),
            Some("tool_calls".to_string())
        );
    }

    #[test]
    fn test_resolve_stop_reason_without_tool_use() {
        let blocks = vec![ContentBlock {
            block_type: "text".to_string(),
            text: Some("hello".into()),
            id: None,
            name: None,
            input: None,
        }];
        assert_eq!(
            resolve_stop_reason(&blocks, Some("stop".to_string())),
            Some("stop".to_string())
        );
    }

    #[test]
    fn test_parse_streaming_initial_delta() {
        let delta = serde_json::json!({
            "tool_calls": [{
                "index": 0,
                "id": "call_abc",
                "type": "function",
                "function": {
                    "name": "exec",
                    "arguments": ""
                }
            }]
        });
        let mut state = StreamingToolCallState::new();
        let event = parse_streaming_tool_call_delta(&delta, &mut state);
        assert!(event.is_some());
        assert!(state.contains_key(&0));
        assert_eq!(state[&0], ("call_abc".to_string(), "exec".to_string()));
    }

    #[test]
    fn test_parse_streaming_continuation_delta() {
        let mut state = StreamingToolCallState::new();
        state.insert(0, ("call_abc".to_string(), "exec".to_string()));

        let delta = serde_json::json!({
            "tool_calls": [{
                "index": 0,
                "function": {
                    "arguments": "{\"cmd\":"
                }
            }]
        });
        let event = parse_streaming_tool_call_delta(&delta, &mut state);
        assert!(event.is_some());
    }

    #[test]
    fn test_parse_streaming_no_tool_calls() {
        let delta = serde_json::json!({
            "content": "hello"
        });
        let mut state = StreamingToolCallState::new();
        let event = parse_streaming_tool_call_delta(&delta, &mut state);
        assert!(event.is_none());
    }

    #[test]
    fn test_parse_streaming_multiple_indices() {
        let mut state = StreamingToolCallState::new();

        // First tool call
        let delta1 = serde_json::json!({
            "tool_calls": [{
                "index": 0,
                "id": "call_1",
                "function": { "name": "fn_a", "arguments": "{" }
            }]
        });
        parse_streaming_tool_call_delta(&delta1, &mut state);

        // Second tool call
        let delta2 = serde_json::json!({
            "tool_calls": [{
                "index": 1,
                "id": "call_2",
                "function": { "name": "fn_b", "arguments": "" }
            }]
        });
        parse_streaming_tool_call_delta(&delta2, &mut state);

        assert_eq!(state.len(), 2);
        assert_eq!(state[&0].0, "call_1");
        assert_eq!(state[&1].0, "call_2");
    }
}
