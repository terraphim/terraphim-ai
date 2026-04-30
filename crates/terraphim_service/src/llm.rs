use std::sync::Arc;

use ahash::AHashMap;
use serde_json::Value;
use terraphim_types::LlmResult;

#[cfg(feature = "llm_router")]
pub use self::router_config::{MergedRouterConfig, RouterMode};
#[cfg(feature = "proxy")]
use crate::llm::proxy_client::ProxyLlmClient;
#[cfg(feature = "llm_router")]
mod bridge;
#[cfg(feature = "proxy")]
mod proxy_client;
#[cfg(feature = "llm_router")]
mod router_config;

use crate::Result as ServiceResult;

/// Options controlling the length of generated summaries.
#[derive(Clone, Debug)]
pub struct SummarizeOptions {
    /// Maximum character length of the generated summary.
    pub max_length: usize,
}

/// Options for chat-completion requests.
#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct ChatOptions {
    /// Maximum number of tokens in the model response.
    pub max_tokens: Option<u32>,
    /// Sampling temperature (0.0 = deterministic, 1.0 = creative).
    pub temperature: Option<f32>,
}

/// Trait implemented by all LLM provider backends (Ollama, OpenRouter, proxy).
///
/// Implementors must be `Send + Sync` so they can be held behind an `Arc`
/// and shared across async tasks.
#[async_trait::async_trait]
pub trait LlmClient: Send + Sync {
    /// Short identifier for this provider, e.g. `"ollama"` or `"openrouter"`.
    fn name(&self) -> &'static str;

    /// Summarise `content` to at most `opts.max_length` characters.
    async fn summarize(&self, content: &str, opts: SummarizeOptions) -> ServiceResult<String>;

    /// List available models for this provider (best-effort)
    async fn list_models(&self) -> ServiceResult<Vec<String>> {
        Err(crate::ServiceError::Config(
            "Listing models not supported by this provider".to_string(),
        ))
    }

    /// Perform a chat completion with messages
    async fn chat_completion(
        &self,
        _messages: Vec<serde_json::Value>,
        _opts: ChatOptions,
    ) -> ServiceResult<String> {
        Err(crate::ServiceError::Config(
            "Chat completion not supported by this provider".to_string(),
        ))
    }

    /// Perform a chat completion and return usage metadata when available.
    async fn chat_completion_with_usage(
        &self,
        messages: Vec<serde_json::Value>,
        opts: ChatOptions,
    ) -> ServiceResult<LlmResult> {
        let content = self.chat_completion(messages, opts).await?;
        Ok(LlmResult::new(content))
    }

    // Reserved for future: streaming chat
}

// ---------------- Provider selection ----------------

/// Determine if the role requests AI summarization via generic LLM config in `extra`.
pub fn role_wants_ai_summarize(role: &terraphim_config::Role) -> bool {
    // Look for `llm_auto_summarize: true` in role.extra
    get_bool_extra(&role.extra, "llm_auto_summarize").unwrap_or(false)
}

/// Best-effort builder that inspects role settings and returns an LLM client if configured.
pub fn build_llm_from_role(role: &terraphim_config::Role) -> Option<Arc<dyn LlmClient>> {
    eprintln!("🔧 Building LLM client for role: {}", role.name);
    eprintln!(
        "🔧 Role extra keys: {:?}",
        role.extra.keys().collect::<Vec<_>>()
    );
    log::debug!("Building LLM client for role: {}", role.name);
    log::debug!(
        "Role extra keys: {:?}",
        role.extra.keys().collect::<Vec<_>>()
    );

    // If LLM is explicitly disabled, don't try to build a client
    if !role.llm_enabled {
        log::debug!(
            "LLM disabled for role '{}', skipping client build",
            role.name
        );
        return None;
    }

    // Check if there's a nested "extra" key (this handles a serialization issue)
    if let Some(nested_extra) = role.extra.get("extra") {
        log::debug!("Found nested extra field");
        // Try to extract from nested extra
        if let Some(nested_obj) = nested_extra.as_object() {
            let nested_map: AHashMap<String, Value> = nested_obj
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect();

            if let Some(provider) = get_string_extra(&nested_map, "llm_provider") {
                log::debug!("Found nested llm_provider: {}", provider);
                match provider.as_str() {
                    "ollama" => {
                        let client = build_ollama_from_nested_extra(&nested_map);
                        log::debug!(
                            "Built Ollama client from nested extra: {:?}",
                            client.is_some()
                        );
                        return client;
                    }
                    "openrouter" => {
                        return None;
                    }
                    #[cfg(feature = "genai")]
                    "genai" => {
                        let model = get_string_extra(&nested_map, "llm_model");
                        if let Some(m) = model {
                            let p = GenAiClient::provider_for_model(&m).to_string();
                            return Some(Arc::new(GenAiClient::new(p, m)) as Arc<dyn LlmClient>);
                        }
                        return None;
                    }
                    _ => {}
                }
            }
        }
    }

    // Prefer explicit `llm_provider` in `extra`
    if let Some(provider) = get_string_extra(&role.extra, "llm_provider") {
        log::debug!("Found llm_provider: {}", provider);
        match provider.as_str() {
            "ollama" => {
                let client =
                    build_ollama_from_role(role).or_else(|| build_openrouter_from_role(role));
                log::debug!(
                    "Built Ollama client from role extra: {:?}",
                    client.is_some()
                );
                return client;
            }
            "openrouter" => {
                return build_openrouter_from_role(role).or_else(|| build_ollama_from_role(role));
            }
            "genai" => {
                return build_genai_from_role(role);
            }
            _ => {}
        }
    }

    // Try genai client first when feature is enabled (provides usage tracking)
    #[cfg(feature = "genai")]
    {
        let model = get_string_extra(&role.extra, "llm_model").or_else(|| role.llm_model.clone());
        if model.is_some() {
            if let Some(client) = build_genai_from_role(role) {
                log::debug!("Built genai client as primary provider");
                return Some(client);
            }
        }
    }

    // Fallbacks: use OpenRouter if configured, else Ollama if extra hints exist
    if role_has_openrouter_config(role) {
        if let Some(client) = build_openrouter_from_role(role) {
            return Some(client);
        }
    }

    if has_ollama_hints(&role.extra) {
        log::debug!("Found Ollama hints in extra");
        if let Some(client) = build_ollama_from_role(role) {
            log::debug!("Built Ollama client from hints");
            // When routing is enabled, skip early return -- the bridge block below
            // handles capability-based routing with all available providers
            #[cfg(feature = "llm_router")]
            {
                if !role.llm_router_enabled {
                    return Some(client);
                }
            }
            #[cfg(not(feature = "llm_router"))]
            {
                return Some(client);
            }
        }
    }

    // Check if intelligent routing is enabled at the role level
    #[cfg(feature = "llm_router")]
    if role.llm_router_enabled {
        log::info!("Intelligent routing enabled for role: {}", role.name);
        let router_config = MergedRouterConfig::from_role_and_env(role.llm_router_config.as_ref());

        match router_config.mode {
            RouterMode::Library => {
                // Library mode: use RouterBridgeLlmClient with real capability-based routing
                let mut available_clients: Vec<bridge::LlmProviderDescriptor> = Vec::new();

                if let Some(ollama) = build_ollama_from_role(role) {
                    available_clients.push(bridge::LlmProviderDescriptor {
                        provider: bridge::provider_from_llm_client(ollama.as_ref(), role),
                        client: ollama,
                    });
                }
                if let Some(openrouter) = build_openrouter_from_role(role) {
                    available_clients.push(bridge::LlmProviderDescriptor {
                        provider: bridge::provider_from_llm_client(openrouter.as_ref(), role),
                        client: openrouter,
                    });
                }

                if !available_clients.is_empty() {
                    let fallback = available_clients[0].client.clone();
                    let mut bridge_client =
                        bridge::RouterBridgeLlmClient::new(fallback, router_config);
                    for descriptor in available_clients {
                        bridge_client.register_provider(descriptor);
                    }
                    return Some(Arc::new(bridge_client) as Arc<dyn LlmClient>);
                }
            }
            RouterMode::Service => {
                // Service mode: use external HTTP proxy client
                let proxy_url = router_config.get_proxy_url();
                log::info!("Service mode routing to: {}", proxy_url);
                let proxy_config = crate::llm::proxy_client::ProxyClientConfig {
                    base_url: proxy_url,
                    timeout_secs: 60,
                    log_requests: true,
                };
                return Some(Arc::new(ProxyLlmClient::new(proxy_config)) as Arc<dyn LlmClient>);
            }
        }
    }

    // Fallback: try terraphim-llm-proxy first (can route to multiple providers)
    #[cfg(feature = "proxy")]
    {
        log::info!(
            "Attempting terraphim-llm-proxy fallback for role '{}'",
            role.name
        );
        let proxy_config = crate::llm::proxy_client::ProxyClientConfig {
            base_url: "http://127.0.0.1:3456".to_string(),
            timeout_secs: 60,
            log_requests: true,
        };
        Some(Arc::new(ProxyLlmClient::new(proxy_config)) as Arc<dyn LlmClient>)
    }

    // Fallback: try Ollama with defaults (only when proxy feature disabled)
    #[cfg(not(feature = "proxy"))]
    {
        log::info!(
            "No proxy available for role '{}', attempting Ollama with defaults",
            role.name
        );
        if let Some(client) = build_ollama_from_role(role) {
            return Some(client);
        }

        log::debug!("No LLM client could be built for role: {}", role.name);
        None
    }
}

fn get_string_extra(extra: &AHashMap<String, Value>, key: &str) -> Option<String> {
    extra
        .get(key)
        .and_then(|v| v.as_str().map(|s| s.to_string()))
}

fn get_bool_extra(extra: &AHashMap<String, Value>, key: &str) -> Option<bool> {
    extra.get(key).and_then(|v| v.as_bool())
}

fn has_ollama_hints(extra: &AHashMap<String, Value>) -> bool {
    extra.contains_key("llm_provider")
        || extra.contains_key("ollama_model")
        || extra.contains_key("llm_model")
        || extra.contains_key("ollama_base_url")
        || extra.contains_key("llm_base_url")
}

#[cfg(feature = "openrouter")]
fn role_has_openrouter_config(role: &terraphim_config::Role) -> bool {
    role.has_llm_config()
}

#[cfg(not(feature = "openrouter"))]
fn role_has_openrouter_config(_role: &terraphim_config::Role) -> bool {
    false
}

// ---------------- OpenRouter Adapter ----------------

#[cfg(feature = "openrouter")]
struct OpenRouterClient {
    inner: crate::openrouter::OpenRouterService,
}

#[cfg(feature = "openrouter")]
#[async_trait::async_trait]
impl LlmClient for OpenRouterClient {
    fn name(&self) -> &'static str {
        "openrouter"
    }

    async fn summarize(&self, content: &str, opts: SummarizeOptions) -> ServiceResult<String> {
        let summary = self
            .inner
            .generate_summary(content, opts.max_length)
            .await?;
        Ok(summary)
    }

    async fn list_models(&self) -> ServiceResult<Vec<String>> {
        let models = self.inner.list_models().await?;
        Ok(models)
    }

    async fn chat_completion(
        &self,
        messages: Vec<serde_json::Value>,
        opts: ChatOptions,
    ) -> ServiceResult<String> {
        let response = self
            .inner
            .chat_completion(messages, opts.max_tokens, opts.temperature)
            .await?;
        Ok(response)
    }
}

#[cfg(feature = "openrouter")]
fn build_openrouter_from_role(role: &terraphim_config::Role) -> Option<Arc<dyn LlmClient>> {
    let api_key = role.llm_api_key.as_deref()?;
    let model = role.llm_model.as_deref().unwrap_or("openai/gpt-3.5-turbo");
    match crate::openrouter::OpenRouterService::new(api_key, model) {
        Ok(inner) => Some(Arc::new(OpenRouterClient { inner }) as Arc<dyn LlmClient>),
        Err(e) => {
            log::warn!("Failed to init OpenRouter client: {}", e);
            None
        }
    }
}

#[cfg(not(feature = "openrouter"))]
fn build_openrouter_from_role(_role: &terraphim_config::Role) -> Option<Arc<dyn LlmClient>> {
    None
}

// ---------------- Ollama Adapter ----------------

#[cfg(feature = "ollama")]
struct OllamaClient {
    http: reqwest::Client,
    base_url: String,
    model: String,
}

#[cfg(feature = "ollama")]
#[async_trait::async_trait]
impl LlmClient for OllamaClient {
    fn name(&self) -> &'static str {
        "ollama"
    }

    async fn summarize(&self, content: &str, opts: SummarizeOptions) -> ServiceResult<String> {
        // Craft a compact summarization instruction with strict length constraints
        let prompt = format!(
            "Please provide a concise and informative summary in EXACTLY {} characters or less. Be brief and focused.\n\nContent:\n{}",
            opts.max_length, content
        );

        // Small retry loop with timeouts for resilience
        let max_attempts = 3;
        let url = format!("{}/api/chat", self.base_url.trim_end_matches('/'));
        let mut last_err: Option<crate::ServiceError> = None;
        for attempt in 1..=max_attempts {
            let body = serde_json::json!({
                "model": self.model,
                "messages": [
                    {"role": "user", "content": prompt}
                ],
                "stream": false
            });

            let req = self
                .http
                .post(url.clone())
                .timeout(std::time::Duration::from_secs(30))
                .json(&body);

            match req.send().await {
                Ok(resp) => {
                    if !resp.status().is_success() {
                        let status = resp.status();
                        let text = resp.text().await.unwrap_or_default();
                        last_err = Some(crate::ServiceError::Config(format!(
                            "Ollama error {}: {}",
                            status, text
                        )));
                        // Retry on 5xx; break on 4xx
                        if status.is_server_error() && attempt < max_attempts {
                            continue;
                        } else {
                            break;
                        }
                    }

                    match resp.json::<serde_json::Value>().await {
                        Ok(json) => {
                            let mut content = json
                                .get("message")
                                .and_then(|m| m.get("content"))
                                .and_then(|c| c.as_str())
                                .unwrap_or("")
                                .trim()
                                .to_string();

                            // Post-process to respect max_length constraint
                            if content.len() > opts.max_length {
                                // Try to truncate at a word boundary
                                let truncated = if let Some(last_space) =
                                    content[..opts.max_length].rfind(' ')
                                {
                                    if last_space > opts.max_length * 3 / 4 {
                                        // Only truncate if we can keep most of the content
                                        format!("{}...", &content[..last_space])
                                    } else {
                                        format!("{}...", &content[..opts.max_length])
                                    }
                                } else {
                                    format!("{}...", &content[..opts.max_length])
                                };
                                content = truncated;
                            }

                            return Ok(content);
                        }
                        Err(e) => {
                            last_err = Some(crate::ServiceError::Config(format!(
                                "Invalid Ollama response: {}",
                                e
                            )));
                            if attempt < max_attempts {
                                continue;
                            }
                        }
                    }
                }
                Err(e) => {
                    last_err = Some(crate::ServiceError::Config(format!(
                        "Ollama request failed: {}",
                        e
                    )));
                    if attempt < max_attempts {
                        continue;
                    }
                }
            }
        }
        Err(last_err.unwrap_or_else(|| {
            crate::ServiceError::Config("Ollama request failed (unknown)".to_string())
        }))
    }

    async fn list_models(&self) -> ServiceResult<Vec<String>> {
        let url = format!("{}/api/tags", self.base_url.trim_end_matches('/'));
        let resp = self
            .http
            .get(url)
            .timeout(std::time::Duration::from_secs(15))
            .send()
            .await
            .map_err(|e| crate::ServiceError::Config(format!("Ollama tags failed: {}", e)))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(crate::ServiceError::Config(format!(
                "Ollama tags error {}: {}",
                status, text
            )));
        }

        let json: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| crate::ServiceError::Config(format!("Invalid tags response: {}", e)))?;

        let mut models = Vec::new();
        if let Some(arr) = json.get("models").and_then(|v| v.as_array()) {
            for m in arr {
                if let Some(name) = m.get("name").and_then(|n| n.as_str()) {
                    models.push(name.to_string());
                }
            }
        }
        Ok(models)
    }

    async fn chat_completion(
        &self,
        messages: Vec<serde_json::Value>,
        opts: ChatOptions,
    ) -> ServiceResult<String> {
        let max_attempts = 3;
        let url = format!("{}/api/chat", self.base_url.trim_end_matches('/'));
        let mut last_err: Option<crate::ServiceError> = None;

        for attempt in 1..=max_attempts {
            let body = serde_json::json!({
                "model": self.model,
                "messages": messages,
                "stream": false,
                "options": {
                    "temperature": opts.temperature.unwrap_or(0.7),
                    "num_predict": opts.max_tokens.unwrap_or(1024)
                }
            });

            let req = self
                .http
                .post(url.clone())
                .timeout(std::time::Duration::from_secs(60))
                .json(&body);

            match req.send().await {
                Ok(resp) => {
                    if !resp.status().is_success() {
                        let status = resp.status();
                        let text = resp.text().await.unwrap_or_default();
                        last_err = Some(crate::ServiceError::Config(format!(
                            "Ollama chat error {}: {}",
                            status, text
                        )));
                        // Retry on 5xx; break on 4xx
                        if status.is_server_error() && attempt < max_attempts {
                            continue;
                        } else {
                            break;
                        }
                    }

                    match resp.json::<serde_json::Value>().await {
                        Ok(json) => {
                            let content = json
                                .get("message")
                                .and_then(|m| m.get("content"))
                                .and_then(|c| c.as_str())
                                .unwrap_or("")
                                .trim()
                                .to_string();

                            if content.is_empty() {
                                last_err = Some(crate::ServiceError::Config(
                                    "Ollama returned empty response".to_string(),
                                ));
                                if attempt < max_attempts {
                                    continue;
                                }
                            } else {
                                return Ok(content);
                            }
                        }
                        Err(e) => {
                            last_err = Some(crate::ServiceError::Config(format!(
                                "Invalid Ollama chat response: {}",
                                e
                            )));
                            if attempt < max_attempts {
                                continue;
                            }
                        }
                    }
                }
                Err(e) => {
                    last_err = Some(crate::ServiceError::Config(format!(
                        "Ollama chat request failed: {}",
                        e
                    )));
                    if attempt < max_attempts {
                        continue;
                    }
                }
            }
        }
        Err(last_err.unwrap_or_else(|| {
            crate::ServiceError::Config("Ollama chat request failed (unknown)".to_string())
        }))
    }
}

#[cfg(feature = "ollama")]
fn build_ollama_from_role(role: &terraphim_config::Role) -> Option<Arc<dyn LlmClient>> {
    // Try llm_model or ollama_model, and base url
    let model = get_string_extra(&role.extra, "llm_model")
        .or_else(|| get_string_extra(&role.extra, "ollama_model"))
        .unwrap_or_else(|| "llama3.2:3b".to_string());
    let base_url = get_string_extra(&role.extra, "llm_base_url")
        .or_else(|| get_string_extra(&role.extra, "ollama_base_url"))
        .unwrap_or_else(|| "http://127.0.0.1:11434".to_string());

    log::debug!(
        "Building Ollama client: model={}, base_url={}",
        model,
        base_url
    );

    let http = crate::http_client::create_api_client().unwrap_or_else(|_| reqwest::Client::new());
    Some(Arc::new(OllamaClient {
        http,
        base_url,
        model,
    }) as Arc<dyn LlmClient>)
}

#[cfg(feature = "ollama")]
fn build_ollama_from_nested_extra(
    nested_extra: &AHashMap<String, Value>,
) -> Option<Arc<dyn LlmClient>> {
    let model = get_string_extra(nested_extra, "llm_model")
        .or_else(|| get_string_extra(nested_extra, "ollama_model"))
        .unwrap_or_else(|| "llama3.1".to_string());
    let base_url = get_string_extra(nested_extra, "llm_base_url")
        .or_else(|| get_string_extra(nested_extra, "ollama_base_url"))
        .unwrap_or_else(|| "http://127.0.0.1:11434".to_string());

    log::debug!(
        "Building Ollama client from nested extra: model={}, base_url={}",
        model,
        base_url
    );
    log::debug!(
        "Nested extra keys: {:?}",
        nested_extra.keys().collect::<Vec<_>>()
    );

    let http = crate::http_client::create_api_client().unwrap_or_else(|_| reqwest::Client::new());
    Some(Arc::new(OllamaClient {
        http,
        base_url,
        model,
    }) as Arc<dyn LlmClient>)
}

#[cfg(not(feature = "ollama"))]
fn build_ollama_from_nested_extra(
    _nested_extra: &AHashMap<String, Value>,
) -> Option<Arc<dyn LlmClient>> {
    None
}

#[cfg(not(feature = "ollama"))]
fn build_ollama_from_role(_role: &terraphim_config::Role) -> Option<Arc<dyn LlmClient>> {
    None
}

// ---------------- GenAI Client (fork with usage tracking) ----------------

#[cfg(feature = "genai")]
struct GenAiClient {
    client: genai::Client,
    model: String,
    provider: String,
}

#[cfg(feature = "genai")]
impl GenAiClient {
    fn new(provider: String, model: String) -> Self {
        let client = genai::Client::default();
        Self {
            client,
            model,
            provider,
        }
    }

    fn provider_for_model(model: &str) -> &'static str {
        if model.starts_with("ollama/") || model.contains("llama") || model.contains("gemma") {
            "ollama"
        } else if model.starts_with("anthropic/") || model.starts_with("claude") {
            "anthropic"
        } else if model.starts_with("openai/") || model.starts_with("gpt") {
            "openai"
        } else if model.starts_with("google/") || model.starts_with("gemini") {
            "google"
        } else {
            "openrouter"
        }
    }
}

#[cfg(feature = "genai")]
#[async_trait::async_trait]
impl LlmClient for GenAiClient {
    fn name(&self) -> &'static str {
        "genai"
    }

    async fn summarize(&self, content: &str, opts: SummarizeOptions) -> ServiceResult<String> {
        let messages = vec![genai::chat::ChatMessage::user(format!(
            "Summarize the following in {} characters or less:\n\n{}",
            opts.max_length, content
        ))];
        let chat_req = genai::chat::ChatRequest::new(messages);
        let mut chat_opts = genai::chat::ChatOptions::default();
        chat_opts = chat_opts.with_max_tokens(opts.max_length as u32);

        let chat_res = self
            .client
            .exec_chat(&self.model, chat_req, Some(&chat_opts))
            .await
            .map_err(|e| crate::ServiceError::Config(format!("genai error: {}", e)))?;

        let summary = chat_res
            .content
            .joined_texts()
            .or_else(|| chat_res.content.first_text().map(|s| s.to_string()))
            .unwrap_or_default();
        Ok(summary)
    }

    async fn chat_completion(
        &self,
        messages: Vec<serde_json::Value>,
        opts: ChatOptions,
    ) -> ServiceResult<String> {
        let result = self.chat_completion_with_usage(messages, opts).await?;
        Ok(result.content)
    }

    async fn chat_completion_with_usage(
        &self,
        messages: Vec<serde_json::Value>,
        opts: ChatOptions,
    ) -> ServiceResult<LlmResult> {
        let start = std::time::Instant::now();

        let chat_messages: Vec<genai::chat::ChatMessage> = messages
            .iter()
            .filter_map(|m| {
                let role = m.get("role")?.as_str()?;
                let content = m.get("content")?.as_str()?;
                match role {
                    "system" => Some(genai::chat::ChatMessage::system(content)),
                    "assistant" => Some(genai::chat::ChatMessage::assistant(content)),
                    _ => Some(genai::chat::ChatMessage::user(content)),
                }
            })
            .collect();

        let chat_req = genai::chat::ChatRequest::new(chat_messages);
        let mut chat_opts = genai::chat::ChatOptions::default();
        if let Some(max_tokens) = opts.max_tokens {
            chat_opts = chat_opts.with_max_tokens(max_tokens);
        }
        if let Some(temp) = opts.temperature {
            chat_opts = chat_opts.with_temperature(temp as f64);
        }

        let chat_res = self
            .client
            .exec_chat(&self.model, chat_req, Some(&chat_opts))
            .await
            .map_err(|e| crate::ServiceError::Config(format!("genai error: {}", e)))?;

        let content = chat_res
            .content
            .joined_texts()
            .or_else(|| chat_res.content.first_text().map(|s| s.to_string()))
            .unwrap_or_default();

        let latency_ms = start.elapsed().as_millis() as u64;

        let usage = terraphim_types::LlmUsage {
            input_tokens: chat_res.usage.prompt_tokens.unwrap_or(0) as u64,
            output_tokens: chat_res.usage.completion_tokens.unwrap_or(0) as u64,
            model: self.model.clone(),
            provider: self.provider.clone(),
            cost_usd: None,
            latency_ms,
        };

        Ok(LlmResult::new(content).with_usage(usage))
    }
}

#[cfg(feature = "genai")]
fn build_genai_from_role(role: &terraphim_config::Role) -> Option<Arc<dyn LlmClient>> {
    let model = get_string_extra(&role.extra, "llm_model")
        .or_else(|| role.llm_model.clone())
        .unwrap_or_else(|| "ollama/gemma3:270m".to_string());

    let provider = get_string_extra(&role.extra, "llm_provider")
        .unwrap_or_else(|| GenAiClient::provider_for_model(&model).to_string());

    log::debug!(
        "Building genai client: model={}, provider={}",
        model,
        provider
    );

    Some(Arc::new(GenAiClient::new(provider, model)) as Arc<dyn LlmClient>)
}

#[cfg(not(feature = "genai"))]
fn build_genai_from_role(_role: &terraphim_config::Role) -> Option<Arc<dyn LlmClient>> {
    None
}

#[cfg(test)]
mod llm_router_tests {
    use super::*;
    use ahash::AHashMap;
    use terraphim_config::Role;

    fn create_test_role() -> Role {
        Role {
            name: "test-role".into(),
            shortname: None,
            relevance_function: terraphim_types::RelevanceFunction::TitleScorer,
            terraphim_it: false,
            theme: "default".to_string(),
            kg: None,
            haystacks: vec![],
            llm_enabled: false,
            llm_api_key: None,
            llm_model: None,
            llm_auto_summarize: false,
            llm_chat_enabled: false,
            llm_chat_system_prompt: None,
            llm_chat_model: None,
            llm_context_window: None,
            extra: AHashMap::new(),
            llm_router_enabled: false,
            llm_router_config: None,
        }
    }

    #[tokio::test]
    #[cfg(feature = "llm_router")]
    async fn test_routing_disabled_returns_static_client() {
        let mut role = create_test_role();
        role.llm_enabled = true;
        role.extra
            .insert("llm_model".to_string(), serde_json::json!("llama3.1"));
        let client = build_llm_from_role(&role);
        assert!(client.is_some());
        #[cfg(feature = "genai")]
        assert_eq!(client.unwrap().name(), "genai");
        #[cfg(not(feature = "genai"))]
        assert_eq!(client.unwrap().name(), "ollama");
    }

    #[tokio::test]
    #[cfg(feature = "llm_router")]
    async fn test_routing_enabled_returns_routed_client() {
        let mut role = create_test_role();
        role.llm_enabled = true;
        role.llm_router_enabled = true;
        role.extra
            .insert("llm_model".to_string(), serde_json::json!("llama3.1"));

        let client = build_llm_from_role(&role);
        assert!(client.is_some());
        #[cfg(feature = "genai")]
        assert_eq!(client.unwrap().name(), "genai");
        #[cfg(not(feature = "genai"))]
        assert_eq!(client.unwrap().name(), "routed_llm");
    }

    #[tokio::test]
    #[cfg(feature = "genai")]
    async fn test_genai_explicit_provider() {
        let mut role = create_test_role();
        role.llm_enabled = true;
        role.extra
            .insert("llm_provider".to_string(), serde_json::json!("genai"));
        role.extra
            .insert("llm_model".to_string(), serde_json::json!("openai/gpt-4o"));
        let client = build_llm_from_role(&role);
        assert!(client.is_some());
        assert_eq!(client.unwrap().name(), "genai");
    }

    #[tokio::test]
    #[cfg(not(feature = "llm_router"))]
    async fn test_without_llm_router_feature() {
        let mut role = create_test_role();
        role.llm_enabled = true;
        role.extra
            .insert("llm_model".to_string(), serde_json::json!("llama3.1"));
        let client = build_llm_from_role(&role);
        // Without feature, should still build static client if configured
        assert!(client.is_some());
    }
}
