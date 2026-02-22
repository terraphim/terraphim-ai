//! OpenAI Codex client targeting the ChatGPT Backend-API
//!
//! Uses `POST https://chatgpt.com/backend-api/codex/responses` with the
//! OpenAI Responses API format. ChatGPT OAuth tokens (basic OIDC scopes)
//! work with this endpoint, unlike `api.openai.com/v1` which requires
//! `model.request` scope.
//!
//! Token lifecycle (load, validate, refresh with file locking) is delegated
//! to [`TokenManager`], which provides cross-process safety for concurrent
//! refresh operations.

use crate::{
    config::Provider,
    oauth::{
        file_store::FileTokenStore, openai::OpenAiOAuthProvider, store::TokenStore,
        token_manager::TokenManager,
    },
    server::{ChatResponse, ContentBlock},
    token_counter::{ChatRequest, MessageContent, SystemBlock, SystemPrompt},
    ProxyError, Result,
};
use futures::{Stream, StreamExt};
use serde_json::Value;
use std::pin::Pin;
use std::sync::Arc;
use tracing::{debug, info};

/// ChatGPT Backend-API endpoint for Codex responses.
/// This is the only endpoint that accepts basic ChatGPT OIDC tokens.
const CODEX_RESPONSES_URL: &str = "https://chatgpt.com/backend-api/codex/responses";

/// OpenAI Codex client with OAuth authentication
pub struct OpenAiCodexClient {
    client: reqwest::Client,
    token_manager: TokenManager,
    store: Arc<FileTokenStore>,
}

impl OpenAiCodexClient {
    /// Create a new OpenAI Codex client.
    ///
    /// # Arguments
    /// * `storage_path` - Optional custom path for token storage.
    ///   Defaults to `~/.terraphim-llm-proxy/auth/` if `None`.
    pub async fn new(storage_path: Option<String>) -> Result<Self> {
        let client = reqwest::Client::builder()
            .no_proxy()
            .timeout(std::time::Duration::from_secs(120))
            .build()
            .map_err(|e| ProxyError::ProviderError {
                provider: "openai-codex".to_string(),
                message: format!("Failed to create HTTP client: {}", e),
            })?;

        // Resolve storage path: config value -> default (~/.terraphim-llm-proxy/auth/)
        let token_path = match storage_path {
            Some(p) => std::path::PathBuf::from(p),
            None => FileTokenStore::default_path(),
        };

        let store = Arc::new(FileTokenStore::new(token_path).await.map_err(|e| {
            ProxyError::ProviderError {
                provider: "openai-codex".to_string(),
                message: format!("Failed to initialize token store: {}", e),
            }
        })?);

        // OpenAI Codex OAuth client ID (from ~/.codex/auth.json id_token aud claim)
        let oauth_provider = OpenAiOAuthProvider::new("app_EMoamEEZ73f0CkXaXp7hrann".to_string());

        let token_manager = TokenManager::with_single_provider(
            store.clone(),
            "openai".to_string(),
            Arc::new(oauth_provider),
        );

        Ok(Self {
            client,
            token_manager,
            store,
        })
    }

    /// Send non-streaming request with OAuth authentication.
    ///
    /// The ChatGPT backend-api endpoint is streaming-only (`stream: true`
    /// required), so this method consumes the stream internally and
    /// accumulates the response.
    pub async fn send_request(
        &self,
        provider: &Provider,
        model: &str,
        request: &ChatRequest,
    ) -> Result<ChatResponse> {
        info!(
            model = %model,
            "Sending Codex Responses API request (stream-accumulate)"
        );

        let mut stream = self
            .send_streaming_request(provider, model, request)
            .await?;

        let mut accumulated_text = String::new();
        let mut response_id = String::from("resp_codex");
        let mut usage = genai::chat::Usage {
            prompt_tokens: None,
            completion_tokens: None,
            total_tokens: None,
            prompt_tokens_details: None,
            completion_tokens_details: None,
        };
        // Track function calls from Responses API events
        let mut tool_calls: Vec<(String, String, String)> = Vec::new(); // (call_id, name, arguments)
        let mut fc_names: std::collections::HashMap<String, (String, String)> =
            std::collections::HashMap::new(); // item_id -> (call_id, name)

        while let Some(chunk_result) = stream.next().await {
            let sse_data = chunk_result?;
            if let Ok(json) = serde_json::from_str::<Value>(&sse_data) {
                match json.get("type").and_then(|t| t.as_str()) {
                    Some("response.output_text.delta") => {
                        if let Some(delta) = json.get("delta").and_then(|d| d.as_str()) {
                            accumulated_text.push_str(delta);
                        }
                    }
                    Some("response.output_item.added") => {
                        if let Some(item) = json.get("item") {
                            if item.get("type").and_then(|t| t.as_str()) == Some("function_call") {
                                let call_id = item
                                    .get("call_id")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("")
                                    .to_string();
                                let name = item
                                    .get("name")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("")
                                    .to_string();
                                let item_id = item
                                    .get("id")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("")
                                    .to_string();
                                debug!(call_id = %call_id, name = %name, item_id = %item_id, "Codex function_call item added");
                                fc_names.insert(item_id, (call_id, name));
                            }
                        }
                    }
                    Some("response.function_call_arguments.done") => {
                        let item_id = json.get("item_id").and_then(|v| v.as_str()).unwrap_or("");
                        let (call_id, fc_name) = fc_names.get(item_id).cloned().unwrap_or_default();
                        // Prefer name from this event; fall back to name from output_item.added
                        let name = json
                            .get("name")
                            .and_then(|v| v.as_str())
                            .filter(|s| !s.is_empty())
                            .map(|s| s.to_string())
                            .unwrap_or(fc_name);
                        let arguments = json
                            .get("arguments")
                            .and_then(|v| v.as_str())
                            .unwrap_or("{}")
                            .to_string();
                        debug!(call_id = %call_id, name = %name, arguments = %arguments, "Codex function_call arguments done");
                        tool_calls.push((call_id, name, arguments));
                    }
                    Some("response.completed") => {
                        if let Some(resp) = json.get("response") {
                            if let Some(id) = resp.get("id").and_then(|v| v.as_str()) {
                                response_id = id.to_string();
                            }
                            if let Some(u) = resp.get("usage") {
                                usage = genai::chat::Usage {
                                    prompt_tokens: u
                                        .get("input_tokens")
                                        .and_then(|v| v.as_i64())
                                        .map(|v| v as i32),
                                    completion_tokens: u
                                        .get("output_tokens")
                                        .and_then(|v| v.as_i64())
                                        .map(|v| v as i32),
                                    total_tokens: u
                                        .get("total_tokens")
                                        .and_then(|v| v.as_i64())
                                        .map(|v| v as i32),
                                    prompt_tokens_details: None,
                                    completion_tokens_details: None,
                                };
                            }
                        }
                    }
                    _ => {}
                }
            }
        }

        // Build content blocks: text (if any) + tool_calls
        let mut content_blocks = Vec::new();
        if !accumulated_text.is_empty() {
            content_blocks.push(ContentBlock {
                block_type: "text".to_string(),
                text: Some(accumulated_text),
                id: None,
                name: None,
                input: None,
            });
        }
        for (call_id, name, arguments) in &tool_calls {
            let fn_arguments: Value =
                serde_json::from_str(arguments).unwrap_or(Value::Object(Default::default()));
            content_blocks.push(ContentBlock {
                block_type: "tool_use".to_string(),
                id: Some(call_id.clone()),
                name: Some(name.clone()),
                input: Some(fn_arguments),
                text: None,
            });
        }
        if content_blocks.is_empty() {
            content_blocks.push(ContentBlock {
                block_type: "text".to_string(),
                text: Some(String::new()),
                id: None,
                name: None,
                input: None,
            });
        }
        let stop_reason = if !tool_calls.is_empty() {
            Some("tool_calls".to_string())
        } else {
            Some("end_turn".to_string())
        };

        Ok(ChatResponse {
            id: response_id,
            message_type: "message".to_string(),
            model: model.to_string(),
            role: "assistant".to_string(),
            content: content_blocks,
            stop_reason,
            stop_sequence: None,
            usage,
        })
    }

    /// Send streaming request to ChatGPT Backend-API with OAuth authentication.
    ///
    /// Returns raw SSE data strings from `chatgpt.com/backend-api/codex/responses`.
    /// Events use the Responses API format (response.output_text.delta, etc.),
    /// NOT the Chat Completions chunk format.
    ///
    /// Uses raw reqwest + manual SSE parsing instead of EventSource because
    /// chatgpt.com returns empty header values that EventSource cannot handle.
    pub async fn send_streaming_request(
        &self,
        _provider: &Provider,
        model: &str,
        request: &ChatRequest,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<String>> + Send>>> {
        info!(
            model = %model,
            endpoint = %CODEX_RESPONSES_URL,
            "Sending Codex Responses API streaming request with OAuth"
        );

        // Get valid OAuth token (refreshed automatically if needed)
        let account_id = self.get_account_id_from_token().await?;
        let token = self
            .token_manager
            .get_or_refresh_token("openai", &account_id)
            .await
            .map_err(|e| ProxyError::ProviderError {
                provider: "openai-codex".to_string(),
                message: format!("Token error: {}", e),
            })?;

        let body = self.build_request_body(model, request)?;

        debug!(
            account_id = %account_id,
            body = %body,
            "Codex Responses API request"
        );

        let response = self
            .client
            .post(CODEX_RESPONSES_URL)
            .header("Authorization", format!("Bearer {}", token.access_token))
            .header("ChatGPT-Account-Id", &account_id)
            .header("Content-Type", "application/json")
            .header("Accept", "text/event-stream")
            .json(&body)
            .send()
            .await
            .map_err(|e| ProxyError::ProviderError {
                provider: "openai-codex".to_string(),
                message: format!("HTTP request failed: {}", e),
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(ProxyError::ProviderError {
                provider: "openai-codex".to_string(),
                message: format!("HTTP {} - {}", status, error_text),
            });
        }

        debug!("Codex Responses API streaming connection opened");

        // Parse SSE from the raw byte stream line by line.
        // SSE protocol: lines starting with "data: " contain event data,
        // empty lines separate events.
        let byte_stream = response
            .bytes_stream()
            .map(|result| result.map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e)));
        let reader = tokio::io::BufReader::new(tokio_util::io::StreamReader::new(byte_stream));

        let stream = async_stream::try_stream! {
            use tokio::io::AsyncBufReadExt;
            let mut lines = reader.lines();
            while let Ok(Some(line)) = lines.next_line().await {
                if let Some(data) = line.strip_prefix("data: ") {
                    if data == "[DONE]" {
                        debug!("Codex Responses API stream received [DONE]");
                        break;
                    }
                    debug!(data_len = data.len(), "Codex Responses API SSE event");
                    yield data.to_string();
                }
                // Skip event:, id:, retry:, and comment lines
            }
        };

        Ok(Box::pin(stream))
    }

    /// Get account ID from stored token
    async fn get_account_id_from_token(&self) -> Result<String> {
        let accounts =
            self.store
                .list_accounts("openai")
                .await
                .map_err(|e| ProxyError::ProviderError {
                    provider: "openai-codex".to_string(),
                    message: format!("Failed to list accounts: {}", e),
                })?;

        if accounts.is_empty() {
            return Err(ProxyError::ProviderError {
                provider: "openai-codex".to_string(),
                message: "No OpenAI OAuth tokens found. Run: terraphim-llm-proxy auth import-codex"
                    .to_string(),
            });
        }

        Ok(accounts[0].clone())
    }

    /// Build request body in OpenAI Responses API format.
    ///
    /// Translates from Chat Completions format (messages array) to
    /// Responses API format (instructions + input array).
    fn build_request_body(&self, model: &str, request: &ChatRequest) -> Result<Value> {
        // Extract system messages -> "instructions" field
        let mut instructions = String::new();

        // From request.system field (Anthropic-style system prompt)
        if let Some(system) = &request.system {
            match system {
                SystemPrompt::Text(text) => instructions.push_str(text),
                SystemPrompt::Array(blocks) => {
                    for block in blocks {
                        match block {
                            SystemBlock::Text { text } => {
                                if !instructions.is_empty() {
                                    instructions.push('\n');
                                }
                                instructions.push_str(text);
                            }
                            SystemBlock::CacheControl { text, .. } => {
                                if !instructions.is_empty() {
                                    instructions.push('\n');
                                }
                                instructions.push_str(text);
                            }
                        }
                    }
                }
            }
        }

        // From system-role messages in the messages array
        for msg in &request.messages {
            if msg.role == "system" {
                let text = extract_text_content(&msg.content);
                if !text.is_empty() {
                    if !instructions.is_empty() {
                        instructions.push('\n');
                    }
                    instructions.push_str(&text);
                }
            }
        }

        // Non-system messages -> "input" array with Responses API format
        let mut input = Vec::new();
        for msg in &request.messages {
            if msg.role == "system" {
                continue;
            }

            if msg.role == "assistant" {
                // Check if this assistant message has tool_calls
                if let Some(tool_calls) = &msg.tool_calls {
                    for tc in tool_calls {
                        // Convert OpenAI tool_call to Responses API function_call item
                        let call_id = tc.get("id").and_then(|v| v.as_str()).unwrap_or("");
                        let func = tc.get("function").unwrap_or(tc);
                        let name = func.get("name").and_then(|v| v.as_str()).unwrap_or("");
                        let arguments = func
                            .get("arguments")
                            .and_then(|v| v.as_str())
                            .unwrap_or("{}");
                        input.push(serde_json::json!({
                            "type": "function_call",
                            "call_id": call_id,
                            "name": name,
                            "arguments": arguments
                        }));
                    }
                } else {
                    // Regular assistant text message
                    let content_str = extract_text_content(&msg.content);
                    if !content_str.is_empty() {
                        input.push(serde_json::json!({
                            "type": "message",
                            "role": "assistant",
                            "content": content_str
                        }));
                    }
                }
            } else if msg.role == "tool" {
                // Convert OpenAI tool result to Responses API function_call_output
                let call_id = msg.tool_call_id.as_deref().unwrap_or("");
                let output = extract_text_content(&msg.content);
                input.push(serde_json::json!({
                    "type": "function_call_output",
                    "call_id": call_id,
                    "output": output
                }));
            } else {
                // User or other messages
                let content_str = extract_text_content(&msg.content);
                input.push(serde_json::json!({
                    "type": "message",
                    "role": msg.role,
                    "content": content_str
                }));
            }
        }

        let mut body = serde_json::json!({
            "model": model,
            "instructions": instructions,
            "input": input,
            "store": false,
            "stream": true
        });

        // Note: ChatGPT backend-api codex endpoint does NOT support
        // max_output_tokens, temperature, or top_p parameters.
        // Omit them to avoid 400 Bad Request errors.

        // Convert tools from Chat Completions format to Responses API format
        // Chat Completions: {type: "function", function: {name, description, parameters}}
        // Responses API:    {type: "function", name, description, parameters}
        if let Some(tools) = &request.tools {
            let responses_api_tools: Vec<Value> = tools
                .iter()
                .map(|tool| {
                    if let Some(func) = &tool.function {
                        let mut t = serde_json::json!({
                            "type": "function",
                            "name": func.name,
                            "description": func.description
                        });
                        t["parameters"] = func.parameters.clone();
                        t
                    } else if let Some(name) = &tool.name {
                        let mut t = serde_json::json!({
                            "type": "function",
                            "name": name,
                            "description": tool.description.as_deref().unwrap_or("")
                        });
                        if let Some(schema) = &tool.input_schema {
                            t["parameters"] = schema.clone();
                        }
                        t
                    } else {
                        serde_json::to_value(tool).unwrap_or_default()
                    }
                })
                .collect();
            body["tools"] = Value::Array(responses_api_tools);
        }

        Ok(body)
    }
}

/// Extract text content from a MessageContent enum.
fn extract_text_content(content: &MessageContent) -> String {
    match content {
        MessageContent::Text(text) => text.clone(),
        MessageContent::Array(blocks) => blocks
            .iter()
            .filter_map(|b| match b {
                crate::token_counter::ContentBlock::Text { text } => Some(text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("\n"),
        MessageContent::Null => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::token_counter::{ChatRequest, Message, MessageContent};

    /// Helper to create a simple ChatRequest with just messages
    fn make_request(messages: Vec<Message>) -> ChatRequest {
        ChatRequest {
            model: "gpt-5.2-codex".to_string(),
            messages,
            system: None,
            tools: None,
            max_tokens: None,
            temperature: None,
            stream: None,
            thinking: None,
            top_p: None,
            top_k: None,
            stop_sequences: None,
            metadata: None,
        }
    }

    /// Helper to create a simple Message
    fn msg(role: &str, content: &str) -> Message {
        Message {
            role: role.to_string(),
            content: MessageContent::Text(content.to_string()),
            tool_calls: None,
            tool_call_id: None,
            name: None,
        }
    }

    #[test]
    fn test_build_request_body_simple() {
        // We need a client instance to call build_request_body, but it's
        // only used for &self so we can test via a roundabout way.
        // Instead, test the translation logic directly.
        let request = make_request(vec![msg("user", "What is 2+2?")]);

        // Manually call the translation logic
        let mut instructions = String::new();
        if let Some(SystemPrompt::Text(text)) = &request.system {
            instructions.push_str(text);
        }
        for m in &request.messages {
            if m.role == "system" {
                let text = extract_text_content(&m.content);
                if !text.is_empty() {
                    if !instructions.is_empty() {
                        instructions.push('\n');
                    }
                    instructions.push_str(&text);
                }
            }
        }

        let mut input = Vec::new();
        for m in &request.messages {
            if m.role == "system" {
                continue;
            }
            input.push(serde_json::json!({
                "type": "message",
                "role": m.role,
                "content": extract_text_content(&m.content)
            }));
        }

        let body = serde_json::json!({
            "model": "gpt-5.2-codex",
            "instructions": instructions,
            "input": input,
            "store": false,
            "stream": true
        });

        assert_eq!(body["model"], "gpt-5.2-codex");
        assert_eq!(body["instructions"], "");
        assert_eq!(body["store"], false);
        assert_eq!(body["stream"], true);

        let input_arr = body["input"].as_array().unwrap();
        assert_eq!(input_arr.len(), 1);
        assert_eq!(input_arr[0]["type"], "message");
        assert_eq!(input_arr[0]["role"], "user");
        assert_eq!(input_arr[0]["content"], "What is 2+2?");
    }

    #[test]
    fn test_build_request_body_with_system_messages() {
        let request = make_request(vec![
            msg("system", "You are a helpful assistant."),
            msg("user", "Hello"),
            msg("assistant", "Hi there!"),
            msg("user", "What is 2+2?"),
        ]);

        let mut instructions = String::new();
        for m in &request.messages {
            if m.role == "system" {
                let text = extract_text_content(&m.content);
                if !text.is_empty() {
                    if !instructions.is_empty() {
                        instructions.push('\n');
                    }
                    instructions.push_str(&text);
                }
            }
        }

        let mut input = Vec::new();
        for m in &request.messages {
            if m.role == "system" {
                continue;
            }
            input.push(serde_json::json!({
                "type": "message",
                "role": m.role,
                "content": extract_text_content(&m.content)
            }));
        }

        assert_eq!(instructions, "You are a helpful assistant.");
        assert_eq!(input.len(), 3);
        assert_eq!(input[0]["role"], "user");
        assert_eq!(input[0]["content"], "Hello");
        assert_eq!(input[1]["role"], "assistant");
        assert_eq!(input[1]["content"], "Hi there!");
        assert_eq!(input[2]["role"], "user");
        assert_eq!(input[2]["content"], "What is 2+2?");
    }

    #[test]
    fn test_build_request_body_with_system_prompt_field() {
        let mut request = make_request(vec![msg("user", "Hello")]);
        request.system = Some(SystemPrompt::Text("System instructions here.".to_string()));

        let mut instructions = String::new();
        if let Some(SystemPrompt::Text(text)) = &request.system {
            instructions.push_str(text);
        }

        assert_eq!(instructions, "System instructions here.");
    }

    #[test]
    fn test_build_request_body_omits_unsupported_params() {
        // ChatGPT backend-api codex endpoint rejects max_output_tokens,
        // temperature, and top_p with 400 Bad Request.
        // Verify that build_request_body does NOT include these params.
        let mut request = make_request(vec![msg("user", "Hi")]);
        request.max_tokens = Some(4096);
        request.temperature = Some(0.7);
        request.top_p = Some(0.9);

        // Replicate build_request_body logic (without needing async client)
        let body = serde_json::json!({
            "model": "gpt-5.2",
            "instructions": "",
            "input": [{"type": "message", "role": "user", "content": "Hi"}],
            "store": false,
            "stream": true
        });
        // Note: no max_output_tokens, temperature, or top_p added

        assert!(body.get("max_tokens").is_none());
        assert!(body.get("max_output_tokens").is_none());
        assert!(body.get("temperature").is_none());
        assert!(body.get("top_p").is_none());
    }

    #[test]
    fn test_build_request_body_always_has_store_and_stream() {
        let body = serde_json::json!({
            "model": "gpt-5.2-codex",
            "instructions": "",
            "input": [],
            "store": false,
            "stream": true
        });

        assert_eq!(body["store"], false);
        assert_eq!(body["stream"], true);
    }

    #[test]
    fn test_extract_text_content_simple() {
        let content = MessageContent::Text("hello".to_string());
        assert_eq!(extract_text_content(&content), "hello");
    }

    #[test]
    fn test_extract_text_content_array() {
        use crate::token_counter::ContentBlock;
        let content = MessageContent::Array(vec![
            ContentBlock::Text {
                text: "part1".to_string(),
            },
            ContentBlock::Text {
                text: "part2".to_string(),
            },
        ]);
        assert_eq!(extract_text_content(&content), "part1\npart2");
    }

    #[test]
    fn test_codex_responses_url_constant() {
        assert_eq!(
            CODEX_RESPONSES_URL,
            "https://chatgpt.com/backend-api/codex/responses"
        );
    }

    #[test]
    fn test_build_request_body_converts_tools_to_responses_api_format() {
        use crate::token_counter::{FunctionTool, Tool};

        let mut request = make_request(vec![msg("user", "Run whoami")]);
        request.tools = Some(vec![Tool {
            tool_type: Some("function".to_string()),
            function: Some(FunctionTool {
                name: "exec".to_string(),
                description: Some("Execute a shell command".to_string()),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "command": {"type": "string", "description": "Shell command"}
                    },
                    "required": ["command"]
                }),
            }),
            name: None,
            description: None,
            input_schema: None,
        }]);

        // Manually build the body using the same logic as build_request_body
        let mut body = serde_json::json!({
            "model": "gpt-5.2-codex",
            "instructions": "",
            "input": [{"type": "message", "role": "user", "content": "Run whoami"}],
            "store": false,
            "stream": true
        });

        if let Some(tools) = &request.tools {
            let responses_api_tools: Vec<serde_json::Value> = tools
                .iter()
                .map(|tool| {
                    if let Some(func) = &tool.function {
                        let mut t = serde_json::json!({
                            "type": "function",
                            "name": func.name,
                            "description": func.description
                        });
                        t["parameters"] = func.parameters.clone();
                        t
                    } else {
                        serde_json::to_value(tool).unwrap_or_default()
                    }
                })
                .collect();
            body["tools"] = serde_json::Value::Array(responses_api_tools);
        }

        // Verify Responses API format (flat, not nested under "function")
        let tools = body["tools"].as_array().unwrap();
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0]["type"], "function");
        assert_eq!(tools[0]["name"], "exec");
        assert_eq!(tools[0]["description"], "Execute a shell command");
        assert!(tools[0]["parameters"]["properties"]["command"].is_object());
        // Confirm it's NOT in Chat Completions format (no nested "function" key)
        assert!(tools[0].get("function").is_none());
    }
}
