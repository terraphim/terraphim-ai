use std::sync::Arc;

use ahash::AHashMap;
use serde_json::Value;

use crate::Result as ServiceResult;

#[derive(Clone, Debug)]
pub struct SummarizeOptions {
    pub max_length: usize,
}

#[derive(Clone, Debug)]
pub struct ChatOptions {
    pub max_tokens: Option<u32>,
    pub temperature: Option<f32>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct ChatMessage {
    pub role: String, // "system", "user", "assistant"
    pub content: String,
}

#[async_trait::async_trait]
pub trait LlmClient: Send + Sync {
    fn name(&self) -> &'static str;

    async fn summarize(&self, content: &str, opts: SummarizeOptions) -> ServiceResult<String>;

    async fn chat_completion(
        &self,
        messages: Vec<ChatMessage>,
        opts: ChatOptions,
    ) -> ServiceResult<String>;

    /// List available models for this provider (best-effort)
    async fn list_models(&self) -> ServiceResult<Vec<String>> {
        Err(crate::ServiceError::Config(
            "Listing models not supported by this provider".to_string(),
        ))
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
    // Prefer explicit `llm_provider` in `extra`
    if let Some(provider) = get_string_extra(&role.extra, "llm_provider") {
        match provider.as_str() {
            "ollama" => {
                return build_ollama_from_role(role).or_else(|| build_openrouter_from_role(role))
            }
            "openrouter" => {
                return build_openrouter_from_role(role).or_else(|| build_ollama_from_role(role))
            }
            _ => {}
        }
    }

    // Fallbacks: use OpenRouter if configured, else Ollama if extra hints exist
    if role_has_openrouter_config(role) {
        if let Some(client) = build_openrouter_from_role(role) {
            return Some(client);
        }
    }

    if has_ollama_hints(&role.extra) {
        if let Some(client) = build_ollama_from_role(role) {
            return Some(client);
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

    async fn chat_completion(
        &self,
        messages: Vec<ChatMessage>,
        opts: ChatOptions,
    ) -> ServiceResult<String> {
        let messages_json: Vec<serde_json::Value> = messages
            .into_iter()
            .map(|msg| serde_json::json!({"role": msg.role, "content": msg.content}))
            .collect();

        let response = self
            .inner
            .chat_completion(messages_json, opts.max_tokens, opts.temperature)
            .await?;
        Ok(response)
    }

    async fn list_models(&self) -> ServiceResult<Vec<String>> {
        let models = self.inner.list_models().await?;
        Ok(models)
    }
}

#[cfg(feature = "openrouter")]
fn build_openrouter_from_role(role: &terraphim_config::Role) -> Option<Arc<dyn LlmClient>> {
    let api_key = role.openrouter_api_key.as_deref()?;
    let model = role
        .openrouter_model
        .as_deref()
        .unwrap_or("openai/gpt-3.5-turbo");
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

    async fn chat_completion(
        &self,
        messages: Vec<ChatMessage>,
        opts: ChatOptions,
    ) -> ServiceResult<String> {
        let max_attempts = 3;
        let url = format!("{}/api/chat", self.base_url.trim_end_matches('/'));
        let mut last_err: Option<crate::ServiceError> = None;

        // Convert ChatMessage to Ollama format
        let ollama_messages: Vec<serde_json::Value> = messages
            .into_iter()
            .map(|msg| {
                serde_json::json!({
                    "role": msg.role,
                    "content": msg.content
                })
            })
            .collect();

        for attempt in 1..=max_attempts {
            let body = serde_json::json!({
                "model": self.model,
                "messages": ollama_messages,
                "stream": false,
                "options": {
                    "num_predict": opts.max_tokens.unwrap_or(1024),
                    "temperature": opts.temperature.unwrap_or(0.7)
                }
            });

            let req = self
                .http
                .post(url.clone())
                .timeout(std::time::Duration::from_secs(60)) // Longer timeout for chat
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

                            return Ok(content);
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
}

#[cfg(feature = "ollama")]
fn build_ollama_from_role(role: &terraphim_config::Role) -> Option<Arc<dyn LlmClient>> {
    // Try llm_model or ollama_model, and base url
    let model = get_string_extra(&role.extra, "llm_model")
        .or_else(|| get_string_extra(&role.extra, "ollama_model"))
        .unwrap_or_else(|| "llama3.1".to_string());
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

#[cfg(not(feature = "ollama"))]
fn build_ollama_from_role(_role: &terraphim_config::Role) -> Option<Arc<dyn LlmClient>> {
    None
}
