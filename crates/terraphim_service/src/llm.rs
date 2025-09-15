use std::sync::Arc;

use ahash::AHashMap;
use serde_json::Value;

use crate::Result as ServiceResult;

#[derive(Clone, Debug)]
pub struct SummarizeOptions {
    pub max_length: usize,
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct ChatOptions {
    pub max_tokens: Option<u32>,
    pub temperature: Option<f32>,
}

#[async_trait::async_trait]
pub trait LlmClient: Send + Sync {
    fn name(&self) -> &'static str;

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

    // Reserved for future: streaming chat
}

// ---------------- Provider selection ----------------

/// Determine if the role requests AI summarization via generic LLM config in `extra`.
pub fn role_wants_ai_summarize(role: &terraphim_config::Role) -> bool {
    let result = get_bool_extra(&role.extra, "llm_auto_summarize").unwrap_or(false);

    log::debug!(
        "role_wants_ai_summarize for role '{}': result={}, extra keys: {:?}",
        role.name,
        result,
        role.extra.keys().collect::<Vec<_>>()
    );

    if let Some(value) = role.extra.get("llm_auto_summarize") {
        log::debug!("llm_auto_summarize raw value: {:?}", value);
    }

    result
}

/// Best-effort builder that inspects role settings and returns an LLM client if configured.
pub fn build_llm_from_role(role: &terraphim_config::Role) -> Option<Arc<dyn LlmClient>> {
    build_llm_from_role_with_config(role, None)
}

/// Build an LLM client for chat purposes with config-level fallbacks
pub fn build_llm_for_chat(
    role: &terraphim_config::Role,
    config: Option<&terraphim_config::Config>,
) -> Option<Arc<dyn LlmClient>> {
    build_llm_from_role_with_config_and_model(
        role,
        config,
        config.and_then(|c| c.default_chat_model.as_deref()),
    )
}

/// Build an LLM client for summarization purposes with config-level fallbacks
pub fn build_llm_for_summarization(
    role: &terraphim_config::Role,
    config: Option<&terraphim_config::Config>,
) -> Option<Arc<dyn LlmClient>> {
    let preferred_model = config.and_then(|c| c.default_summarization_model.as_deref());
    log::debug!(
        "[DEBUGGER:llm:{}] build_llm_for_summarization for role '{}', preferred_model={:?}, config_provider={:?}",
        line!(),
        role.name,
        preferred_model,
        config.and_then(|c| c.default_model_provider.as_ref())
    );
    let result = build_llm_from_role_with_config_and_model(role, config, preferred_model);
    log::debug!(
        "[DEBUGGER:llm:{}] build_llm_for_summarization result: {}",
        line!(),
        if result.is_some() {
            "SUCCESS"
        } else {
            "FAILED"
        }
    );
    result
}

/// Internal helper that builds LLM client with config fallbacks
fn build_llm_from_role_with_config(
    role: &terraphim_config::Role,
    config: Option<&terraphim_config::Config>,
) -> Option<Arc<dyn LlmClient>> {
    build_llm_from_role_with_config_and_model(role, config, None)
}

/// Internal helper that builds LLM client with config fallbacks and specific model preference
fn build_llm_from_role_with_config_and_model(
    role: &terraphim_config::Role,
    config: Option<&terraphim_config::Config>,
    preferred_model: Option<&str>,
) -> Option<Arc<dyn LlmClient>> {
    log::debug!(
        "[DEBUGGER:llm:{}] build_llm_from_role_with_config_and_model for role '{}', preferred_model={:?}",
        line!(),
        role.name,
        preferred_model
    );
    log::debug!("[DEBUGGER:llm:{}] role.extra={:?}", line!(), role.extra);

    // Determine the provider from role config or config fallbacks
    let provider = get_string_extra(&role.extra, "llm_provider")
        .or_else(|| config.and_then(|c| c.default_model_provider.clone()));
    log::debug!(
        "[DEBUGGER:llm:{}] determined provider={:?}",
        line!(),
        provider
    );

    // Check if there's a nested "extra" key (this handles a serialization issue)
    if let Some(nested_extra) = role.extra.get("extra") {
        // Try to extract from nested extra
        if let Some(nested_obj) = nested_extra.as_object() {
            let nested_map: AHashMap<String, Value> = nested_obj
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect();

            if let Some(provider) = get_string_extra(&nested_map, "llm_provider") {
                match provider.as_str() {
                    "ollama" => {
                        return build_ollama_from_nested_extra_with_model(
                            &nested_map,
                            preferred_model,
                        );
                    }
                    "openrouter" => {
                        // Would implement similar nested extraction for OpenRouter
                        return build_openrouter_from_role_with_model(role, preferred_model);
                    }
                    _ => {}
                }
            }
        }
    }

    // Try role-specific provider or config fallback
    if let Some(provider) = provider {
        match provider.as_str() {
            "ollama" => {
                return build_ollama_from_role_with_model(role, preferred_model, config)
                    .or_else(|| build_openrouter_from_role_with_model(role, preferred_model))
            }
            "openrouter" => {
                return build_openrouter_from_role_with_model(role, preferred_model)
                    .or_else(|| build_ollama_from_role_with_model(role, preferred_model, config))
            }
            _ => {}
        }
    }

    // Fallbacks: use OpenRouter if configured, else Ollama if extra hints exist, else try config defaults
    if role_has_openrouter_config(role) {
        if let Some(client) = build_openrouter_from_role_with_model(role, preferred_model) {
            return Some(client);
        }
    }

    if has_ollama_hints(&role.extra) {
        if let Some(client) = build_ollama_from_role_with_model(role, preferred_model, config) {
            return Some(client);
        }
    }

    // Final fallback: try to build with config defaults if no role hints
    if let Some(config) = config {
        if let Some(default_provider) = &config.default_model_provider {
            match default_provider.as_str() {
                "ollama" => {
                    return build_ollama_from_config_defaults(config, preferred_model);
                }
                "openrouter" => {
                    // Would need OpenRouter defaults in config
                    return None;
                }
                _ => {}
            }
        }
    }

    None
}

fn get_string_extra(extra: &AHashMap<String, Value>, key: &str) -> Option<String> {
    extra
        .get(key)
        .and_then(|v| v.as_str().map(|s| s.to_string()))
}

fn get_bool_extra(extra: &AHashMap<String, Value>, key: &str) -> Option<bool> {
    extra.get(key).and_then(|v| {
        // Try direct boolean first
        if let Some(b) = v.as_bool() {
            Some(b)
        }
        // Try string parsing for "true"/"false"
        else if let Some(s) = v.as_str() {
            match s.to_lowercase().as_str() {
                "true" | "1" | "yes" | "on" => Some(true),
                "false" | "0" | "no" | "off" => Some(false),
                _ => None,
            }
        }
        // Try number parsing (non-zero = true)
        else {
            v.as_f64().map(|n| n != 0.0)
        }
    })
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
    role.has_openrouter_config()
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
fn build_ollama_from_role_with_model(
    role: &terraphim_config::Role,
    preferred_model: Option<&str>,
    config: Option<&terraphim_config::Config>,
) -> Option<Arc<dyn LlmClient>> {
    // Try preferred model, then role models, then config defaults, then hardcoded default
    let model = preferred_model
        .map(|s| s.to_string())
        .or_else(|| get_string_extra(&role.extra, "llm_model"))
        .or_else(|| get_string_extra(&role.extra, "ollama_model"))
        .or_else(|| config.and_then(|c| c.default_chat_model.clone()))
        .unwrap_or_else(|| "gemma2:2b".to_string());

    let base_url = get_string_extra(&role.extra, "llm_base_url")
        .or_else(|| get_string_extra(&role.extra, "ollama_base_url"))
        .unwrap_or_else(|| "http://127.0.0.1:11434".to_string());

    let http = crate::http_client::create_api_client().unwrap_or_else(|_| reqwest::Client::new());
    Some(Arc::new(OllamaClient {
        http,
        base_url,
        model,
    }) as Arc<dyn LlmClient>)
}

#[cfg(feature = "ollama")]
fn build_ollama_from_nested_extra_with_model(
    nested_extra: &AHashMap<String, Value>,
    preferred_model: Option<&str>,
) -> Option<Arc<dyn LlmClient>> {
    let model = preferred_model
        .map(|s| s.to_string())
        .or_else(|| get_string_extra(nested_extra, "llm_model"))
        .or_else(|| get_string_extra(nested_extra, "ollama_model"))
        .unwrap_or_else(|| "gemma2:2b".to_string());
    let base_url = get_string_extra(nested_extra, "llm_base_url")
        .or_else(|| get_string_extra(nested_extra, "ollama_base_url"))
        .unwrap_or_else(|| "http://127.0.0.1:11434".to_string());

    let http = crate::http_client::create_api_client().unwrap_or_else(|_| reqwest::Client::new());
    Some(Arc::new(OllamaClient {
        http,
        base_url,
        model,
    }) as Arc<dyn LlmClient>)
}

#[cfg(feature = "ollama")]
fn build_ollama_from_config_defaults(
    config: &terraphim_config::Config,
    preferred_model: Option<&str>,
) -> Option<Arc<dyn LlmClient>> {
    let model = preferred_model
        .map(|s| s.to_string())
        .or_else(|| config.default_chat_model.clone())
        .unwrap_or_else(|| "gemma2:2b".to_string());
    let base_url = "http://127.0.0.1:11434".to_string();

    let http = crate::http_client::create_api_client().unwrap_or_else(|_| reqwest::Client::new());
    Some(Arc::new(OllamaClient {
        http,
        base_url,
        model,
    }) as Arc<dyn LlmClient>)
}

#[cfg(not(feature = "ollama"))]
fn build_ollama_from_role_with_model(
    _role: &terraphim_config::Role,
    _preferred_model: Option<&str>,
    _config: Option<&terraphim_config::Config>,
) -> Option<Arc<dyn LlmClient>> {
    None
}

#[cfg(not(feature = "ollama"))]
fn build_ollama_from_nested_extra_with_model(
    _nested_extra: &AHashMap<String, Value>,
    _preferred_model: Option<&str>,
) -> Option<Arc<dyn LlmClient>> {
    None
}

#[cfg(not(feature = "ollama"))]
fn build_ollama_from_config_defaults(
    _config: &terraphim_config::Config,
    _preferred_model: Option<&str>,
) -> Option<Arc<dyn LlmClient>> {
    None
}

// ---------------- OpenRouter Builders ----------------

#[cfg(feature = "openrouter")]
fn build_openrouter_from_role_with_model(
    role: &terraphim_config::Role,
    preferred_model: Option<&str>,
) -> Option<Arc<dyn LlmClient>> {
    // Get API key from role config or environment
    let api_key = get_string_extra(&role.extra, "openrouter_api_key")
        .or_else(|| std::env::var("OPENROUTER_API_KEY").ok())?;

    // Get model from preferred_model, then role config, then default
    let model = preferred_model
        .map(|s| s.to_string())
        .or_else(|| get_string_extra(&role.extra, "openrouter_model"))
        .or_else(|| get_string_extra(&role.extra, "llm_model"))
        .unwrap_or_else(|| "openai/gpt-3.5-turbo".to_string());

    // Create OpenRouter service
    match crate::openrouter::OpenRouterService::new(&api_key, &model) {
        Ok(service) => Some(Arc::new(OpenRouterClient { inner: service }) as Arc<dyn LlmClient>),
        Err(e) => {
            log::warn!("Failed to create OpenRouter client: {}", e);
            None
        }
    }
}

#[cfg(not(feature = "openrouter"))]
fn build_openrouter_from_role_with_model(
    _role: &terraphim_config::Role,
    _preferred_model: Option<&str>,
) -> Option<Arc<dyn LlmClient>> {
    None
}
