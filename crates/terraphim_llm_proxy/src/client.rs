//! LLM client module using rust-genai
//!
//! Provides unified interface for communicating with multiple LLM providers.

use crate::{
    config::Provider,
    server::{ChatResponse, ContentBlock},
    token_counter::ChatRequest,
    ProxyError, Result,
};
use futures::{Stream, StreamExt};
use genai::{
    adapter::AdapterKind,
    chat::{
        ChatOptions, ChatRequest as GenaiChatRequest, ChatResponse as GenaiChatResponse,
        ChatStreamEvent, StreamChunk, StreamEnd, ToolCall, ToolChunk, Usage as GenaiUsage,
    },
    resolver::{AuthData, Endpoint, ServiceTargetResolver},
    Client, ModelIden, ServiceTarget,
};

use std::pin::Pin;
use tracing::{debug, info, warn};

#[derive(Default)]
struct CodexStreamState {
    function_calls: std::collections::HashMap<String, (String, String)>,
    emitted_content_or_tool: bool,
}

fn extract_text_from_message_item(item: &serde_json::Value) -> Option<String> {
    let content_items = item.get("content")?.as_array()?;
    let text = content_items
        .iter()
        .filter_map(|content| {
            let content_type = content.get("type").and_then(|v| v.as_str()).unwrap_or("");
            if content_type == "output_text" || content_type == "text" {
                return content
                    .get("text")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
            }
            None
        })
        .collect::<Vec<_>>()
        .join("\n");

    if text.is_empty() {
        None
    } else {
        Some(text)
    }
}

fn extract_text_from_response_output(response: &serde_json::Value) -> Option<String> {
    let output = response.get("output")?.as_array()?;
    let text = output
        .iter()
        .filter_map(|item| {
            if item.get("type").and_then(|v| v.as_str()) == Some("message") {
                return extract_text_from_message_item(item);
            }
            None
        })
        .collect::<Vec<_>>()
        .join("\n");

    if text.is_empty() {
        None
    } else {
        Some(text)
    }
}

fn parse_codex_sse_event(sse_data: &str, state: &mut CodexStreamState) -> Option<ChatStreamEvent> {
    let json = serde_json::from_str::<serde_json::Value>(sse_data).ok()?;

    match json.get("type").and_then(|t| t.as_str()) {
        Some("response.output_text.delta") => {
            if let Some(delta) = json.get("delta").and_then(|d| d.as_str()) {
                if !delta.is_empty() {
                    state.emitted_content_or_tool = true;
                    return Some(ChatStreamEvent::Chunk(StreamChunk {
                        content: delta.to_string(),
                    }));
                }
            }
        }
        Some("response.output_text.done") => {
            // Only emit if no deltas were received (recovery path).
            // Normally output_text.delta events already streamed this text.
            if !state.emitted_content_or_tool {
                if let Some(text) = json.get("text").and_then(|t| t.as_str()) {
                    if !text.is_empty() {
                        state.emitted_content_or_tool = true;
                        return Some(ChatStreamEvent::Chunk(StreamChunk {
                            content: text.to_string(),
                        }));
                    }
                }
            }
        }
        Some("response.content_part.added") | Some("response.content_part.done") => {
            // Only emit if no deltas were received (recovery path).
            // Normally output_text.delta events already streamed this text.
            if !state.emitted_content_or_tool {
                if let Some(part) = json.get("part") {
                    let part_type = part.get("type").and_then(|v| v.as_str()).unwrap_or("");
                    if (part_type == "output_text" || part_type == "text")
                        && part.get("text").and_then(|v| v.as_str()).is_some()
                    {
                        let text = part
                            .get("text")
                            .and_then(|v| v.as_str())
                            .unwrap_or_default();
                        if !text.is_empty() {
                            state.emitted_content_or_tool = true;
                            return Some(ChatStreamEvent::Chunk(StreamChunk {
                                content: text.to_string(),
                            }));
                        }
                    }
                }
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
                    state.function_calls.insert(item_id, (call_id, name));
                } else if !state.emitted_content_or_tool
                    && item.get("type").and_then(|t| t.as_str()) == Some("message")
                {
                    if let Some(text) = extract_text_from_message_item(item) {
                        state.emitted_content_or_tool = true;
                        return Some(ChatStreamEvent::Chunk(StreamChunk { content: text }));
                    }
                }
            }
        }
        Some("response.output_item.done") => {
            // Only emit if no deltas were received (recovery path).
            if !state.emitted_content_or_tool {
                if let Some(item) = json.get("item") {
                    if item.get("type").and_then(|t| t.as_str()) == Some("message") {
                        if let Some(text) = extract_text_from_message_item(item) {
                            state.emitted_content_or_tool = true;
                            return Some(ChatStreamEvent::Chunk(StreamChunk { content: text }));
                        }
                    }
                }
            }
        }
        Some("response.function_call_arguments.done") => {
            let item_id = json.get("item_id").and_then(|v| v.as_str()).unwrap_or("");
            let (call_id, stored_name) = state
                .function_calls
                .get(item_id)
                .cloned()
                .unwrap_or_default();

            let name = json
                .get("name")
                .and_then(|v| v.as_str())
                .filter(|s| !s.is_empty())
                .map(|s| s.to_string())
                .unwrap_or(stored_name);

            let arguments = json
                .get("arguments")
                .and_then(|v| v.as_str())
                .unwrap_or("{}");
            let fn_arguments = serde_json::from_str(arguments)
                .unwrap_or(serde_json::Value::Object(Default::default()));

            state.emitted_content_or_tool = true;
            return Some(ChatStreamEvent::ToolCallChunk(ToolChunk {
                tool_call: ToolCall {
                    call_id,
                    fn_name: name,
                    fn_arguments,
                    thought_signatures: None,
                },
            }));
        }
        Some("response.completed") => {
            if !state.emitted_content_or_tool {
                if let Some(response) = json.get("response") {
                    if let Some(text) = extract_text_from_response_output(response) {
                        state.emitted_content_or_tool = true;
                        return Some(ChatStreamEvent::Chunk(StreamChunk { content: text }));
                    }
                }
            }

            return Some(ChatStreamEvent::End(StreamEnd {
                captured_usage: Some(GenaiUsage::default()),
                captured_content: None,
                captured_reasoning_content: None,
            }));
        }
        _ => {}
    }

    None
}

/// LLM client wrapper around rust-genai
pub struct LlmClient {
    /// OAuth storage path passed to OpenAiCodexClient when constructed on-demand
    oauth_storage_path: Option<String>,
    /// Claude OAuth auth mode: "bearer" or "api_key"
    claude_auth_mode: Option<String>,
    /// Anthropic beta header value for Bearer auth
    anthropic_beta: Option<String>,
}

impl LlmClient {
    /// Create a new LLM client
    pub fn new(oauth_storage_path: Option<String>) -> Result<Self> {
        Ok(Self {
            oauth_storage_path,
            claude_auth_mode: None,
            anthropic_beta: None,
        })
    }

    /// Set Claude OAuth auth mode and beta header from config.
    pub fn with_claude_oauth(
        mut self,
        auth_mode: Option<String>,
        anthropic_beta: Option<String>,
    ) -> Self {
        self.claude_auth_mode = auth_mode;
        self.anthropic_beta = anthropic_beta;
        self
    }

    /// Create genai client with custom resolver for provider.
    /// If `auth_override` is provided, it replaces the provider's api_key for auth.
    fn create_client_for_provider(
        &self,
        provider: &Provider,
        auth_override: Option<AuthData>,
    ) -> Result<Client> {
        debug!(
            provider = %provider.name,
            base_url = %provider.api_base_url,
            "Creating genai client with custom resolver"
        );

        // Create ServiceTargetResolver to configure custom endpoints
        let provider_clone = provider.clone();
        let auth_override_clone = auth_override;
        let target_resolver = ServiceTargetResolver::from_resolver_fn(
            move |service_target: ServiceTarget| -> std::result::Result<ServiceTarget, genai::resolver::Error> {
                let ServiceTarget { model, .. } = service_target;

                // Determine adapter kind
                let adapter = match provider_clone.name.as_str() {
                    "openrouter" | "deepseek" | "groq" => AdapterKind::OpenAI,
                    "anthropic" | "minimax" | "kimi" => AdapterKind::Anthropic,
                    "ollama" => AdapterKind::Ollama,
                    "gemini" => AdapterKind::Gemini,
                    _ => AdapterKind::OpenAI, // Default to OpenAI-compatible
                };

                // Set custom endpoint with provider-specific URL construction
                let base_url = match provider_clone.name.as_str() {
                    // For OpenAI-compatible providers, ensure proper base URL
                    "groq" => {
                        // Groq needs the base URL without path for genai to append correctly
                        if provider_clone.api_base_url.ends_with("/openai/v1") {
                            provider_clone.api_base_url.clone()
                        } else {
                            // Ensure we have the correct base URL for Groq
                            "https://api.groq.com".to_string()
                        }
                    }
                    "deepseek" => {
                        // DeepSeek API base URL
                        if provider_clone.api_base_url.contains("deepseek.com") {
                            provider_clone.api_base_url.clone()
                        } else {
                            "https://api.deepseek.com".to_string()
                        }
                    }
                    "cerebras" => {
                        // Cerebras uses OpenAI-compatible API at /v1/
                        // genai appends /chat/completions, so we need the base with /v1
                        if provider_clone.api_base_url.ends_with("/v1") {
                            provider_clone.api_base_url.clone()
                        } else {
                            "https://api.cerebras.ai/v1".to_string()
                        }
                    }
                    // For other providers, use the configured URL
                    _ => provider_clone.api_base_url.clone(),
                };

                // Create model identifier with adapter (clone model_name to avoid move)
                let model_name = model.model_name.clone();
                let model_iden = ModelIden::new(adapter, model_name.clone());

                debug!(
                    adapter = ?adapter,
                    original_endpoint = %provider_clone.api_base_url,
                    resolved_endpoint = %base_url,
                    model = %model_name,
                    "Resolved service target"
                );

                let endpoint = Endpoint::from_owned(base_url);

                // Use auth override if provided (e.g. OAuth Bearer token),
                // otherwise fall back to provider's api_key
                let auth = if let Some(ref override_auth) = auth_override_clone {
                    override_auth.clone()
                } else {
                    AuthData::from_single(&provider_clone.api_key)
                };

                Ok(ServiceTarget {
                    endpoint,
                    auth,
                    model: model_iden,
                })
            },
        );

        // Build client with custom resolver
        let client = Client::builder()
            .with_service_target_resolver(target_resolver)
            .build();

        Ok(client)
    }

    /// Try to get Claude OAuth auth data for Anthropic requests.
    /// Returns `Some(AuthData)` if OAuth tokens are available, `None` otherwise.
    async fn try_get_claude_auth(&self) -> Option<AuthData> {
        use crate::oauth::{FileTokenStore, TokenStore};
        use std::path::PathBuf;
        use std::sync::Arc;

        let storage_path = self.oauth_storage_path.as_ref()?;

        // Only proceed if claude auth_mode is configured
        let auth_mode = self.claude_auth_mode.as_deref()?;

        let store = match FileTokenStore::new(PathBuf::from(storage_path)).await {
            Ok(store) => Arc::new(store),
            Err(e) => {
                warn!("Failed to create token store for Claude OAuth: {}", e);
                return None;
            }
        };

        // Find the first Claude account
        let accounts = match store.list_accounts("claude").await {
            Ok(accounts) => accounts,
            Err(e) => {
                debug!("No Claude OAuth accounts found: {}", e);
                return None;
            }
        };

        let account_id = accounts.first()?;

        let bundle = match store.load("claude", account_id).await {
            Ok(Some(bundle)) => bundle,
            Ok(None) => {
                debug!("No token bundle for Claude account {}", account_id);
                return None;
            }
            Err(e) => {
                warn!("Failed to load Claude token: {}", e);
                return None;
            }
        };

        match auth_mode {
            "bearer" => {
                // Use token manager for refresh, then return BearerToken
                // Create an empty providers map -- refresh will fail if no provider
                // is registered, but the token may still be valid
                let providers =
                    Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new()));
                let token_manager = crate::oauth::TokenManager::new(store, providers);
                match token_manager
                    .get_or_refresh_token("claude", account_id)
                    .await
                {
                    Ok(refreshed) => {
                        info!(
                            account = %account_id,
                            "Using Claude OAuth Bearer token for Anthropic API"
                        );
                        Some(AuthData::BearerToken(refreshed.access_token))
                    }
                    Err(e) => {
                        // Token might still be valid even if refresh fails
                        warn!("Token refresh failed, trying stored token: {}", e);
                        Some(AuthData::BearerToken(bundle.access_token))
                    }
                }
            }
            _ => {
                // Check for stored API key in metadata (api_key mode)
                if let Some(api_key_value) = bundle.metadata.get("api_key") {
                    if let Some(api_key) = api_key_value.as_str() {
                        info!(
                            account = %account_id,
                            "Using Claude OAuth-created API key for Anthropic API"
                        );
                        return Some(AuthData::from_single(api_key));
                    }
                }
                debug!("No API key in Claude token metadata, falling back to config");
                None
            }
        }
    }

    /// Send a non-streaming chat request
    pub async fn send_request(
        &self,
        provider: &Provider,
        model: &str,
        request: &ChatRequest,
    ) -> Result<ChatResponse> {
        debug!(
            provider = %provider.name,
            model = %model,
            api_base = %provider.api_base_url,
            "Sending non-streaming request to provider"
        );

        // Use custom clients for providers with specific requirements
        match provider.name.as_str() {
            "openrouter" => {
                debug!("Using OpenRouterClient for OpenRouter non-streaming request");
                let openrouter_client = crate::openrouter_client::OpenRouterClient::new();
                return openrouter_client
                    .send_request(provider, model, request)
                    .await;
            }
            "groq" => {
                debug!("Using GroqClient for Groq non-streaming request");
                let groq_client = crate::groq_client::GroqClient::new();
                return groq_client.send_request(provider, model, request).await;
            }
            "cerebras" => {
                debug!("Using CerebrasClient for Cerebras non-streaming request");
                let cerebras_client = crate::cerebras_client::CerebrasClient::new();
                return cerebras_client.send_request(provider, model, request).await;
            }
            "zai" => {
                debug!("Using ZaiClient for Z.ai non-streaming request");
                let zai_client = crate::zai_client::ZaiClient::new();
                return zai_client.send_request(provider, model, request).await;
            }
            "minimax" => {
                debug!("Using MiniMaxClient for MiniMax non-streaming request");
                let minimax_client = crate::minimax_client::MiniMaxClient::new();
                return minimax_client.send_request(provider, model, request).await;
            }
            "kimi" => {
                debug!("Using KimiClient for Kimi non-streaming request");
                let kimi_client = crate::kimi_client::KimiClient::new();
                return kimi_client.send_request(provider, model, request).await;
            }
            "openai-codex" => {
                debug!("Using OpenAiCodexClient for OpenAI Codex non-streaming request with OAuth");
                let openai_codex_client = crate::openai_codex_client::OpenAiCodexClient::new(
                    self.oauth_storage_path.clone(),
                )
                .await?;
                return openai_codex_client
                    .send_request(provider, model, request)
                    .await;
            }
            _ => {
                // Continue with genai client for other providers
            }
        }

        // Convert our ChatRequest to genai::ChatRequest
        let genai_request = self.convert_to_genai_request(request)?;

        // Configure options
        let mut options = ChatOptions::default();
        if let Some(max_tokens) = request.max_tokens {
            options = options.with_max_tokens(max_tokens as u32);
        }
        if let Some(temperature) = request.temperature {
            options = options.with_temperature(temperature as f64);
        }

        // Add provider-specific headers
        use std::collections::HashMap;
        let mut headers = HashMap::new();

        match provider.name.as_str() {
            "openrouter" => {
                headers.insert(
                    "HTTP-Referer".to_string(),
                    "https://terraphim.ai".to_string(),
                );
                headers.insert("X-Title".to_string(), "Terraphim LLM Proxy".to_string());
                debug!("Added OpenRouter required headers (HTTP-Referer, X-Title)");
            }
            "groq" => {
                headers.insert(
                    "User-Agent".to_string(),
                    "Terraphim-LLM-Proxy/1.0".to_string(),
                );
                debug!("Added Groq User-Agent header");
            }
            "deepseek" => {
                headers.insert(
                    "User-Agent".to_string(),
                    "Terraphim-LLM-Proxy/1.0".to_string(),
                );
                debug!("Added DeepSeek User-Agent header");
            }
            _ => {
                // Default headers for other providers
                headers.insert(
                    "User-Agent".to_string(),
                    "Terraphim-LLM-Proxy/1.0".to_string(),
                );
            }
        }

        if !headers.is_empty() {
            options = options.with_extra_headers(headers);
            debug!("Added provider-specific headers for {}", provider.name);
        }

        // For Anthropic, try OAuth auth before falling back to config api_key
        let auth_override = if provider.name == "anthropic" {
            self.try_get_claude_auth().await
        } else {
            None
        };

        // Create client with custom resolver for this provider
        let client = self.create_client_for_provider(provider, auth_override)?;

        // Send request using genai's exec_chat
        let response = client
            .exec_chat(model, genai_request, Some(&options))
            .await
            .map_err(|e| ProxyError::ProviderError {
                provider: provider.name.clone(),
                message: e.to_string(),
            })?;

        // Convert response back to our format
        let chat_response = self.convert_from_genai_response(model, &response)?;

        info!(
            provider = %provider.name,
            model = %model,
            prompt_tokens = chat_response.usage.prompt_tokens,
            completion_tokens = chat_response.usage.completion_tokens,
            "Request completed successfully"
        );

        Ok(chat_response)
    }

    /// Send a streaming chat request
    pub async fn send_streaming_request(
        &self,
        provider: &Provider,
        model: &str,
        request: &ChatRequest,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<ChatStreamEvent>> + Send>>> {
        debug!(
            provider = %provider.name,
            model = %model,
            api_base = %provider.api_base_url,
            "Sending streaming request to provider"
        );

        // Debug: Check if this is OpenRouter
        if provider.name == "openrouter" {
            debug!("Detected OpenRouter provider, will use OpenRouterClient");
        }

        // Use custom clients for providers with specific requirements
        match provider.name.as_str() {
            "openrouter" => {
                debug!("Using OpenRouterClient for OpenRouter streaming request");
                let openrouter_client = crate::openrouter_client::OpenRouterClient::new();
                let raw_stream = openrouter_client
                    .send_streaming_request(provider, model, request)
                    .await?;

                // Convert raw SSE strings to ChatStreamEvent
                let mut tool_call_state = crate::tool_call_utils::StreamingToolCallState::new();
                let stream = raw_stream.map(move |result| match result {
                    Ok(sse_data) => {
                        debug!(sse_data = %sse_data, "Processing SSE data from OpenRouterClient");

                        // Handle [DONE] message
                        if sse_data.trim() == "[DONE]" {
                            debug!("Received [DONE], ending stream");
                            return Ok(ChatStreamEvent::End(StreamEnd {
                                captured_usage: Some(GenaiUsage::default()),
                                captured_content: None,
                                captured_reasoning_content: None,
                            }));
                        }

                        // Parse JSON chunk directly (OpenRouterClient returns raw JSON)
                        if let Ok(chunk) = serde_json::from_str::<serde_json::Value>(&sse_data) {
                            debug!(json = %chunk, "Parsed JSON chunk");
                            if let Some(choices) = chunk.get("choices").and_then(|c| c.as_array()) {
                                if let Some(choice) = choices.first() {
                                    if let Some(delta) = choice.get("delta") {
                                        if let Some(content) =
                                            delta.get("content").and_then(|c| c.as_str())
                                        {
                                            debug!(content = %content, "Extracted content from delta");
                                            return Ok(ChatStreamEvent::Chunk(StreamChunk {
                                                content: content.to_string(),
                                            }));
                                        }
                                        // Check for tool_calls in delta
                                        if let Some(event) =
                                            crate::tool_call_utils::parse_streaming_tool_call_delta(
                                                delta,
                                                &mut tool_call_state,
                                            )
                                        {
                                            return Ok(event);
                                        }
                                    }
                                }
                            }
                        } else {
                            debug!(sse_data = %sse_data, "Failed to parse JSON, skipping");
                        }

                        // If no valid content found, return empty chunk
                        Ok(ChatStreamEvent::Chunk(StreamChunk {
                            content: String::new(),
                        }))
                    }
                    Err(e) => {
                        debug!(error = %e, "SSE stream error");
                        Err(e)
                    }
                });

                return Ok(Box::pin(stream));
            }
            "groq" => {
                debug!("Using GroqClient for Groq streaming request");
                let groq_client = crate::groq_client::GroqClient::new();
                let raw_stream = groq_client
                    .send_streaming_request(provider, model, request)
                    .await?;

                // Convert raw SSE strings to ChatStreamEvent
                let mut tool_call_state = crate::tool_call_utils::StreamingToolCallState::new();
                let stream = raw_stream.map(move |result| match result {
                    Ok(sse_data) => {
                        debug!(sse_data = %sse_data, "Processing SSE data from GroqClient");

                        // Handle [DONE] message
                        if sse_data.trim() == "[DONE]" {
                            debug!("Received [DONE], ending stream");
                            return Ok(ChatStreamEvent::End(StreamEnd {
                                captured_usage: Some(GenaiUsage::default()),
                                captured_content: None,
                                captured_reasoning_content: None,
                            }));
                        }

                        // Parse JSON chunk directly (GroqClient returns raw JSON)
                        if let Ok(chunk) = serde_json::from_str::<serde_json::Value>(&sse_data) {
                            debug!(json = %chunk, "Parsed JSON chunk from Groq");
                            if let Some(choices) = chunk.get("choices").and_then(|c| c.as_array()) {
                                if let Some(choice) = choices.first() {
                                    if let Some(delta) = choice.get("delta") {
                                        if let Some(content) =
                                            delta.get("content").and_then(|c| c.as_str())
                                        {
                                            debug!(content = %content, "Extracted content from delta");
                                            return Ok(ChatStreamEvent::Chunk(StreamChunk {
                                                content: content.to_string(),
                                            }));
                                        }
                                        if let Some(event) =
                                            crate::tool_call_utils::parse_streaming_tool_call_delta(
                                                delta,
                                                &mut tool_call_state,
                                            )
                                        {
                                            return Ok(event);
                                        }
                                    }
                                }
                            }
                        } else {
                            debug!(sse_data = %sse_data, "Failed to parse JSON, skipping");
                        }

                        // If no valid content found, return empty chunk
                        Ok(ChatStreamEvent::Chunk(StreamChunk {
                            content: String::new(),
                        }))
                    }
                    Err(e) => {
                        debug!(error = %e, "SSE stream error from Groq");
                        Err(e)
                    }
                });

                return Ok(Box::pin(stream));
            }
            "cerebras" => {
                debug!("Using CerebrasClient for Cerebras streaming request");
                let cerebras_client = crate::cerebras_client::CerebrasClient::new();
                let raw_stream = cerebras_client
                    .send_streaming_request(provider, model, request)
                    .await?;

                // Convert raw SSE strings to ChatStreamEvent
                let mut tool_call_state = crate::tool_call_utils::StreamingToolCallState::new();
                let stream = raw_stream.map(move |result| match result {
                    Ok(sse_data) => {
                        debug!(sse_data = %sse_data, "Processing SSE data from CerebrasClient");

                        // Handle [DONE] message
                        if sse_data.trim() == "[DONE]" {
                            debug!("Received [DONE], ending stream");
                            return Ok(ChatStreamEvent::End(StreamEnd {
                                captured_usage: Some(GenaiUsage::default()),
                                captured_content: None,
                                captured_reasoning_content: None,
                            }));
                        }

                        // Parse JSON chunk directly (CerebrasClient returns raw JSON)
                        if let Ok(chunk) = serde_json::from_str::<serde_json::Value>(&sse_data) {
                            debug!(json = %chunk, "Parsed JSON chunk from Cerebras");
                            if let Some(choices) = chunk.get("choices").and_then(|c| c.as_array()) {
                                if let Some(choice) = choices.first() {
                                    if let Some(delta) = choice.get("delta") {
                                        if let Some(content) =
                                            delta.get("content").and_then(|c| c.as_str())
                                        {
                                            debug!(content = %content, "Extracted content from delta");
                                            return Ok(ChatStreamEvent::Chunk(StreamChunk {
                                                content: content.to_string(),
                                            }));
                                        }
                                        if let Some(event) =
                                            crate::tool_call_utils::parse_streaming_tool_call_delta(
                                                delta,
                                                &mut tool_call_state,
                                            )
                                        {
                                            return Ok(event);
                                        }
                                    }
                                }
                            }
                        } else {
                            debug!(sse_data = %sse_data, "Failed to parse JSON, skipping");
                        }

                        // If no valid content found, return empty chunk
                        Ok(ChatStreamEvent::Chunk(StreamChunk {
                            content: String::new(),
                        }))
                    }
                    Err(e) => {
                        debug!(error = %e, "SSE stream error from Cerebras");
                        Err(e)
                    }
                });

                return Ok(Box::pin(stream));
            }
            "kimi" => {
                debug!("Using KimiClient for Kimi streaming request");
                let kimi_client = crate::kimi_client::KimiClient::new();
                let raw_stream = kimi_client
                    .send_streaming_request(provider, model, request)
                    .await?;

                // Convert raw SSE strings to ChatStreamEvent
                // Kimi uses Anthropic-compatible streaming format
                let mut current_text = String::new();
                let stream = raw_stream.map(move |result| match result {
                    Ok(sse_data) => {
                        debug!(sse_data = %sse_data, "Processing SSE data from KimiClient");

                        // Handle [DONE] message
                        if sse_data.trim() == "[DONE]" {
                            debug!("Received [DONE], ending stream");
                            return Ok(ChatStreamEvent::End(StreamEnd {
                                captured_usage: Some(GenaiUsage::default()),
                                captured_content: None,
                                captured_reasoning_content: None,
                            }));
                        }

                        // Parse Anthropic-format SSE event
                        // Anthropic events look like: event: content_block_delta\ndata: {"type":"content_block_delta",...}
                        let mut event_type = "";
                        let mut data = "";

                        for line in sse_data.lines() {
                            // Handle both formats: "event: message" and "event:message"
                            if let Some(stripped) = line.strip_prefix("event: ") {
                                event_type = stripped;
                            } else if let Some(stripped) = line.strip_prefix("event:") {
                                event_type = stripped;
                            } else if let Some(stripped) = line.strip_prefix("data: ") {
                                data = stripped;
                            } else if let Some(stripped) = line.strip_prefix("data:") {
                                data = stripped;
                            }
                        }

                        if !data.is_empty() {
                            if let Ok(chunk) = serde_json::from_str::<serde_json::Value>(data) {
                                debug!(event_type = %event_type, json = %chunk, "Parsed Anthropic chunk from Kimi");

                                match event_type {
                                    "content_block_delta" => {
                                        if let Some(delta) = chunk.get("delta") {
                                            if let Some(text) = delta.get("text").and_then(|t| t.as_str()) {
                                                current_text.push_str(text);
                                                return Ok(ChatStreamEvent::Chunk(StreamChunk {
                                                    content: text.to_string(),
                                                }));
                                            }
                                        }
                                    }
                                    "message_stop" => {
                                        return Ok(ChatStreamEvent::End(StreamEnd {
                                            captured_usage: Some(GenaiUsage::default()),
                                            captured_content: None,
                                            captured_reasoning_content: None,
                                        }));
                                    }
                                    _ => {
                                        // Other event types: message_start, content_block_start, etc.
                                        // We don't emit content for these
                                    }
                                }
                            }
                        }

                        // If no valid content found, return empty chunk
                        Ok(ChatStreamEvent::Chunk(StreamChunk {
                            content: String::new(),
                        }))
                    }
                    Err(e) => {
                        debug!(error = %e, "SSE stream error from Kimi");
                        Err(e)
                    }
                });

                return Ok(Box::pin(stream));
            }
            "zai" => {
                debug!("Using ZaiClient for Z.ai streaming request");
                let zai_client = crate::zai_client::ZaiClient::new();
                let raw_stream = zai_client
                    .send_streaming_request(provider, model, request)
                    .await?;

                // Convert raw SSE strings to ChatStreamEvent
                let mut tool_call_state = crate::tool_call_utils::StreamingToolCallState::new();
                let stream = raw_stream.map(move |result| match result {
                    Ok(sse_data) => {
                        debug!(sse_data = %sse_data, "Processing SSE data from ZaiClient");

                        // Handle [DONE] message
                        if sse_data.trim() == "[DONE]" {
                            debug!("Received [DONE], ending stream");
                            return Ok(ChatStreamEvent::End(StreamEnd {
                                captured_usage: Some(GenaiUsage::default()),
                                captured_content: None,
                                captured_reasoning_content: None,
                            }));
                        }

                        // Parse JSON chunk directly (ZaiClient returns raw JSON)
                        if let Ok(chunk) = serde_json::from_str::<serde_json::Value>(&sse_data) {
                            debug!(json = %chunk, "Parsed JSON chunk from Z.ai");
                            if let Some(choices) = chunk.get("choices").and_then(|c| c.as_array()) {
                                if let Some(choice) = choices.first() {
                                    if let Some(delta) = choice.get("delta") {
                                        // GLM-4.7 may use "reasoning_content" instead of "content"
                                        let content = delta
                                            .get("content")
                                            .and_then(|c| c.as_str())
                                            .or_else(|| {
                                                delta
                                                    .get("reasoning_content")
                                                    .and_then(|c| c.as_str())
                                            });
                                        if let Some(text) = content {
                                            debug!(content = %text, "Extracted content from delta");
                                            return Ok(ChatStreamEvent::Chunk(StreamChunk {
                                                content: text.to_string(),
                                            }));
                                        }
                                        if let Some(event) =
                                            crate::tool_call_utils::parse_streaming_tool_call_delta(
                                                delta,
                                                &mut tool_call_state,
                                            )
                                        {
                                            return Ok(event);
                                        }
                                    }
                                }
                            }
                        } else {
                            debug!(sse_data = %sse_data, "Failed to parse JSON, skipping");
                        }

                        // If no valid content found, return empty chunk
                        Ok(ChatStreamEvent::Chunk(StreamChunk {
                            content: String::new(),
                        }))
                    }
                    Err(e) => {
                        debug!(error = %e, "SSE stream error from Z.ai");
                        Err(e)
                    }
                });

                return Ok(Box::pin(stream));
            }
            "openai-codex" => {
                debug!("Using OpenAiCodexClient for Codex Responses API streaming request");
                let openai_codex_client = crate::openai_codex_client::OpenAiCodexClient::new(
                    self.oauth_storage_path.clone(),
                )
                .await?;
                let raw_stream = openai_codex_client
                    .send_streaming_request(provider, model, request)
                    .await?;

                // Convert Responses API SSE events to ChatStreamEvent
                let mut codex_state = CodexStreamState::default();
                let stream = raw_stream.map(move |result| match result {
                    Ok(sse_data) => {
                        debug!(sse_data = %sse_data, "Processing SSE data from Codex Responses API");

                        if let Some(event) = parse_codex_sse_event(&sse_data, &mut codex_state) {
                            return Ok(event);
                        }

                        // No content to emit for this event
                        Ok(ChatStreamEvent::Chunk(StreamChunk {
                            content: String::new(),
                        }))
                    }
                    Err(e) => {
                        debug!(error = %e, "SSE stream error from Codex");
                        Err(e)
                    }
                });

                return Ok(Box::pin(stream));
            }
            _ => {
                // Continue with genai client for other providers
            }
        }

        // Use genai client for other providers
        // Convert to genai request
        let genai_request = self.convert_to_genai_request(request)?;

        // Configure options
        let mut options = ChatOptions::default();
        if let Some(max_tokens) = request.max_tokens {
            options = options.with_max_tokens(max_tokens as u32);
        }
        if let Some(temperature) = request.temperature {
            options = options.with_temperature(temperature as f64);
        }

        // Add provider-specific headers
        use std::collections::HashMap;
        let mut headers = HashMap::new();

        match provider.name.as_str() {
            "openrouter" => {
                headers.insert(
                    "HTTP-Referer".to_string(),
                    "https://terraphim.ai".to_string(),
                );
                headers.insert("X-Title".to_string(), "Terraphim LLM Proxy".to_string());
                debug!("Added OpenRouter required headers (HTTP-Referer, X-Title)");
            }
            "groq" => {
                headers.insert(
                    "User-Agent".to_string(),
                    "Terraphim-LLM-Proxy/1.0".to_string(),
                );
                debug!("Added Groq User-Agent header");
            }
            "deepseek" => {
                headers.insert(
                    "User-Agent".to_string(),
                    "Terraphim-LLM-Proxy/1.0".to_string(),
                );
                debug!("Added DeepSeek User-Agent header");
            }
            _ => {
                // Default headers for other providers
                headers.insert(
                    "User-Agent".to_string(),
                    "Terraphim-LLM-Proxy/1.0".to_string(),
                );
            }
        }

        if !headers.is_empty() {
            options = options.with_extra_headers(headers);
            debug!("Added provider-specific headers for {}", provider.name);
        }

        // For Anthropic, try OAuth auth before falling back to config api_key
        let auth_override = if provider.name == "anthropic" {
            self.try_get_claude_auth().await
        } else {
            None
        };

        // Create client with custom resolver for this provider
        let client = self.create_client_for_provider(provider, auth_override)?;

        // Send streaming request
        let stream_response = client
            .exec_chat_stream(model, genai_request, Some(&options))
            .await
            .map_err(|e| ProxyError::ProviderError {
                provider: provider.name.clone(),
                message: e.to_string(),
            })?;

        // Map stream to our Result type
        let provider_name = provider.name.clone();
        let mapped_stream = stream_response.stream.map(move |result| {
            result.map_err(|e| ProxyError::ProviderError {
                provider: provider_name.clone(),
                message: e.to_string(),
            })
        });

        Ok(Box::pin(mapped_stream))
    }

    /// Convert our ChatRequest to genai::ChatRequest
    fn convert_to_genai_request(&self, req: &ChatRequest) -> Result<GenaiChatRequest> {
        use genai::chat::ChatMessage;

        // Convert messages
        let mut messages = Vec::new();

        // Add system prompt as first message if present
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

            messages.push(ChatMessage::system(system_text));
        }

        // Add regular messages
        for msg in &req.messages {
            let content = match &msg.content {
                crate::token_counter::MessageContent::Text(text) => text.clone(),
                crate::token_counter::MessageContent::Array(blocks) => {
                    // Flatten content blocks to text
                    blocks
                        .iter()
                        .filter_map(|block| match block {
                            crate::token_counter::ContentBlock::Text { text } => Some(text.clone()),
                            crate::token_counter::ContentBlock::ToolResult { content, .. } => {
                                Some(content.clone())
                            }
                            _ => None,
                        })
                        .collect::<Vec<_>>()
                        .join("\n\n")
                }
                crate::token_counter::MessageContent::Null => String::new(),
            };

            let message = match msg.role.as_str() {
                "user" => ChatMessage::user(content),
                "assistant" => ChatMessage::assistant(content),
                "system" => ChatMessage::system(content),
                _ => ChatMessage::user(content), // Default to user
            };

            messages.push(message);
        }

        // Create the genai chat request
        let genai_request = GenaiChatRequest::new(messages);

        Ok(genai_request)
    }

    /// Convert genai::ChatResponse to our ChatResponse
    fn convert_from_genai_response(
        &self,
        model: &str,
        response: &GenaiChatResponse,
    ) -> Result<ChatResponse> {
        let mut content_blocks = Vec::new();

        // Extract text content
        if let Some(text) = response.first_text() {
            if !text.is_empty() {
                content_blocks.push(ContentBlock {
                    block_type: "text".to_string(),
                    text: Some(text.to_string()),
                    id: None,
                    name: None,
                    input: None,
                });
            }
        }

        // Extract tool calls from genai response
        let tool_calls = response.tool_calls();
        for tc in &tool_calls {
            content_blocks.push(ContentBlock {
                block_type: "tool_use".to_string(),
                text: None,
                id: Some(tc.call_id.clone()),
                name: Some(tc.fn_name.clone()),
                input: Some(tc.fn_arguments.clone()),
            });
        }

        // Determine stop reason
        let stop_reason = if !tool_calls.is_empty() {
            Some("tool_calls".to_string())
        } else {
            Some("end_turn".to_string())
        };

        // Ensure at least one content block
        if content_blocks.is_empty() {
            content_blocks.push(ContentBlock {
                block_type: "text".to_string(),
                text: Some(String::new()),
                id: None,
                name: None,
                input: None,
            });
        }

        let chat_response = ChatResponse {
            id: "msg_genai".to_string(), // genai doesn't provide IDs
            message_type: "message".to_string(),
            model: model.to_string(),
            role: "assistant".to_string(),
            content: content_blocks,
            stop_reason,
            stop_sequence: None,
            usage: genai::chat::Usage {
                prompt_tokens_details: None,
                completion_tokens_details: None,
                prompt_tokens: Some(response.usage.prompt_tokens.unwrap_or(0)),
                completion_tokens: Some(response.usage.completion_tokens.unwrap_or(0)),
                total_tokens: Some(response.usage.total_tokens.unwrap_or(0)),
            },
        };

        Ok(chat_response)
    }
}

impl Default for LlmClient {
    fn default() -> Self {
        Self::new(None).expect("Failed to create LLM client")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::token_counter::{Message, MessageContent, SystemPrompt};

    #[allow(dead_code)]
    fn create_test_provider() -> Provider {
        Provider {
            name: "test".to_string(),
            api_base_url: "http://localhost:8000".to_string(),
            api_key: "test_key".to_string(),
            models: vec!["test-model".to_string()],
            transformers: vec![],
        }
    }

    #[test]
    fn test_convert_simple_request() {
        let client = LlmClient::new(None).unwrap();

        let request = ChatRequest {
            model: "test".to_string(),
            messages: vec![Message {
                role: "user".to_string(),
                content: MessageContent::Text("Hello!".to_string()),
                tool_calls: None,
                tool_call_id: None,
                name: None,
            }],
            system: None,
            tools: None,
            max_tokens: None,
            temperature: None,
            stream: None,
            thinking: None,
            ..Default::default()
        };

        let genai_request = client.convert_to_genai_request(&request).unwrap();
        assert!(!genai_request.messages.is_empty());
    }

    #[test]
    fn test_convert_request_with_system() {
        let client = LlmClient::new(None).unwrap();

        let request = ChatRequest {
            model: "test".to_string(),
            messages: vec![Message {
                role: "user".to_string(),
                content: MessageContent::Text("Hello!".to_string()),
                tool_calls: None,
                tool_call_id: None,
                name: None,
            }],
            system: Some(SystemPrompt::Text("You are helpful".to_string())),
            tools: None,
            max_tokens: None,
            temperature: None,
            stream: None,
            thinking: None,
            ..Default::default()
        };

        let genai_request = client.convert_to_genai_request(&request).unwrap();
        // Should have 2 messages: system + user
        assert_eq!(genai_request.messages.len(), 2);
    }

    #[test]
    fn test_parse_codex_output_item_done_message_text_recovery() {
        // When no deltas were received, output_item.done should emit text (recovery)
        let mut state = CodexStreamState::default();
        let sse = serde_json::json!({
            "type": "response.output_item.done",
            "item": {
                "type": "message",
                "content": [
                    { "type": "output_text", "text": "Hello from done" }
                ]
            }
        })
        .to_string();

        let event = parse_codex_sse_event(&sse, &mut state);
        match event {
            Some(ChatStreamEvent::Chunk(chunk)) => {
                assert_eq!(chunk.content, "Hello from done");
            }
            _ => panic!("Expected chunk event in recovery path"),
        }
    }

    #[test]
    fn test_parse_codex_output_item_done_skipped_after_deltas() {
        // When deltas were already received, output_item.done should NOT re-emit
        let state = &mut CodexStreamState {
            emitted_content_or_tool: true, // deltas already streamed
            ..Default::default()
        };

        let sse = serde_json::json!({
            "type": "response.output_item.done",
            "item": {
                "type": "message",
                "content": [
                    { "type": "output_text", "text": "Hello from done" }
                ]
            }
        })
        .to_string();

        let event = parse_codex_sse_event(&sse, state);
        assert!(
            event.is_none(),
            "Summary event should be skipped after deltas"
        );
    }

    #[test]
    fn test_parse_codex_summary_events_skipped_after_deltas() {
        let mut state = CodexStreamState::default();

        // First, simulate receiving a delta (sets emitted_content_or_tool = true)
        let delta = serde_json::json!({
            "type": "response.output_text.delta",
            "delta": "Hello"
        })
        .to_string();
        let event = parse_codex_sse_event(&delta, &mut state);
        assert!(event.is_some());
        assert!(state.emitted_content_or_tool);

        // output_text.done should be skipped
        let done = serde_json::json!({
            "type": "response.output_text.done",
            "text": "Hello world"
        })
        .to_string();
        assert!(parse_codex_sse_event(&done, &mut state).is_none());

        // content_part.done should be skipped
        let part = serde_json::json!({
            "type": "response.content_part.done",
            "part": { "type": "output_text", "text": "Hello world" }
        })
        .to_string();
        assert!(parse_codex_sse_event(&part, &mut state).is_none());
    }

    #[test]
    fn test_parse_codex_response_completed_recovery_path() {
        let mut state = CodexStreamState::default();
        let sse = serde_json::json!({
            "type": "response.completed",
            "response": {
                "output": [
                    {
                        "type": "message",
                        "content": [
                            { "type": "output_text", "text": "Recovered text" }
                        ]
                    }
                ]
            }
        })
        .to_string();

        let event = parse_codex_sse_event(&sse, &mut state);
        match event {
            Some(ChatStreamEvent::Chunk(chunk)) => {
                assert_eq!(chunk.content, "Recovered text");
            }
            _ => panic!("Expected recovery chunk event"),
        }
    }

    #[test]
    fn test_parse_codex_function_call_arguments_done_uses_cached_metadata() {
        let mut state = CodexStreamState::default();
        let added = serde_json::json!({
            "type": "response.output_item.added",
            "item": {
                "type": "function_call",
                "id": "item_1",
                "call_id": "call_123",
                "name": "lookup"
            }
        })
        .to_string();
        let _ = parse_codex_sse_event(&added, &mut state);

        let args_done = serde_json::json!({
            "type": "response.function_call_arguments.done",
            "item_id": "item_1",
            "arguments": "{\"q\":\"rust\"}"
        })
        .to_string();

        let event = parse_codex_sse_event(&args_done, &mut state);
        match event {
            Some(ChatStreamEvent::ToolCallChunk(tool_chunk)) => {
                assert_eq!(tool_chunk.tool_call.call_id, "call_123");
                assert_eq!(tool_chunk.tool_call.fn_name, "lookup");
                assert_eq!(tool_chunk.tool_call.fn_arguments["q"], "rust");
            }
            _ => panic!("Expected tool call chunk event"),
        }
    }

    // Note: get_adapter_for_provider test removed - now handled by ServiceTargetResolver
}
