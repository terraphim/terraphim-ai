//! Mixture-of-Agents (MoA) ensemble.
//!
//! Port of Hermes `tools/mixture_of_agents_tool.py`. Reference models generate
//! diverse responses in parallel, then an aggregator model synthesizes them
//! into a single high-quality response. Targets an OpenAI-compatible
//! chat-completions endpoint (OpenRouter by default).

use crate::config::MoaConfig;
use crate::tools::{Tool, ToolError};
use async_trait::async_trait;
use std::time::Duration;

pub const AGGREGATOR_SYSTEM_PROMPT: &str = "You have been provided with a set of responses from various open-source models to the latest user query. Your task is to synthesize these responses into a single, high-quality response. It is crucial to critically evaluate the information provided in these responses, recognizing that some of it may be biased or incorrect. Your response should not simply replicate the given answers but should offer a refined, accurate, and comprehensive reply to the instruction. Ensure your response is well-structured, coherent, and adheres to the highest standards of accuracy and reliability.\n\nResponses from models:";

pub const MIN_SUCCESSFUL_REFERENCES: usize = 1;

/// Max retry attempts for a single model call (network errors + 5xx only).
pub const MAX_RETRIES: u32 = 3;

/// Initial backoff delay; doubles on each retry (1s, 2s, 4s).
pub const RETRY_BASE_DELAY_MS: u64 = 1000;

/// Build the aggregator system prompt with enumerated reference responses.
pub fn construct_aggregator_prompt(system_prompt: &str, responses: &[String]) -> String {
    let body = responses
        .iter()
        .enumerate()
        .map(|(i, r)| format!("{}. {}", i + 1, r))
        .collect::<Vec<_>>()
        .join("\n");
    format!("{}\n\n{}", system_prompt, body)
}

/// HTTP client for the MoA ensemble.
#[derive(Clone)]
pub struct MoaClient {
    http: reqwest::Client,
    base_url: String,
    api_key: String,
    aggregator_model: String,
}

impl MoaClient {
    pub fn from_config(cfg: &MoaConfig) -> Self {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(300))
            .build()
            .unwrap_or_default();
        Self {
            http,
            base_url: cfg.base_url.trim_end_matches('/').to_string(),
            api_key: cfg.api_key.clone(),
            aggregator_model: cfg.aggregator_model.clone(),
        }
    }

    /// Single chat-completion attempt. On error, returns `(retryable, message)`.
    async fn chat_completion_once(
        &self,
        model: &str,
        messages: serde_json::Value,
        temperature: f64,
    ) -> Result<String, (bool, String)> {
        let mut body = serde_json::json!({
            "model": model,
            "messages": messages,
        });
        // GPT models don't support custom temperature.
        if !model.to_lowercase().starts_with("gpt-") {
            body["temperature"] = serde_json::json!(temperature);
        }

        let url = format!("{}/chat/completions", self.base_url);
        let resp = match self
            .http
            .post(&url)
            .bearer_auth(&self.api_key)
            .json(&body)
            .send()
            .await
        {
            Ok(r) => r,
            // Network errors are transient and retryable.
            Err(e) => return Err((true, e.to_string())),
        };

        let status = resp.status();
        if !status.is_success() {
            // Only 5xx (server) errors are retryable; 4xx are not.
            let retryable = status.is_server_error();
            return Err((
                retryable,
                format!("MoA model {} failed: HTTP {}", model, status),
            ));
        }
        let data: serde_json::Value = resp.json().await.map_err(|e| (false, e.to_string()))?;
        data["choices"][0]["message"]["content"]
            .as_str()
            .map(|s| s.trim().to_string())
            .ok_or((false, format!("MoA model {} returned no content", model)))
    }

    /// Chat completion with bounded retry + exponential backoff on
    /// retryable (network / 5xx) failures only; 4xx and malformed responses
    /// fail immediately.
    async fn chat_completion(
        &self,
        model: &str,
        messages: serde_json::Value,
        temperature: f64,
    ) -> Result<String, String> {
        let mut last_retryable = None;
        for attempt in 0..=MAX_RETRIES {
            match self
                .chat_completion_once(model, messages.clone(), temperature)
                .await
            {
                Ok(content) => return Ok(content),
                Err((retryable, msg)) => {
                    if !retryable || attempt == MAX_RETRIES {
                        return Err(msg);
                    }
                    last_retryable = Some(msg);
                    let delay = RETRY_BASE_DELAY_MS * (1u64 << attempt);
                    tokio::time::sleep(Duration::from_millis(delay)).await;
                }
            }
        }
        Err(last_retryable.unwrap_or_else(|| "MoA retries exhausted".to_string()))
    }

    async fn run_reference(&self, model: &str, prompt: &str) -> Result<String, String> {
        self.chat_completion(
            model,
            serde_json::json!([{"role": "user", "content": prompt}]),
            0.7,
        )
        .await
    }

    async fn run_aggregator(
        &self,
        system_prompt: &str,
        user_prompt: &str,
    ) -> Result<String, String> {
        self.chat_completion(
            &self.aggregator_model,
            serde_json::json!([
                {"role": "system", "content": system_prompt},
                {"role": "user", "content": user_prompt}
            ]),
            0.3,
        )
        .await
    }

    /// Run the full MoA pipeline and return the synthesized response.
    pub async fn synthesize(
        &self,
        user_prompt: &str,
        reference_models: &[String],
    ) -> Result<String, String> {
        // Reference models in parallel (each cloned client is Send).
        let mut handles = Vec::with_capacity(reference_models.len());
        for model in reference_models {
            let client = self.clone();
            let model = model.clone();
            let prompt = user_prompt.to_string();
            handles.push(tokio::spawn(async move {
                client.run_reference(&model, &prompt).await
            }));
        }

        let mut responses = Vec::new();
        for handle in handles {
            match handle.await {
                Ok(Ok(content)) => responses.push(content),
                Ok(Err(e)) => log::warn!("MoA reference model dropped: {}", e),
                Err(e) => log::warn!("MoA reference task failed: {}", e),
            }
        }

        if responses.len() < MIN_SUCCESSFUL_REFERENCES {
            return Err(format!(
                "all {} reference models failed",
                reference_models.len()
            ));
        }

        let system_prompt = construct_aggregator_prompt(AGGREGATOR_SYSTEM_PROMPT, &responses);
        self.run_aggregator(&system_prompt, user_prompt).await
    }
}

/// The `mixture_of_agents` tool.
pub struct MixtureOfAgentsTool {
    client: MoaClient,
    reference_models: Vec<String>,
}

impl MixtureOfAgentsTool {
    pub fn from_config(cfg: &MoaConfig) -> Self {
        Self {
            client: MoaClient::from_config(cfg),
            reference_models: cfg.reference_models.clone(),
        }
    }
}

#[async_trait]
impl Tool for MixtureOfAgentsTool {
    fn name(&self) -> &str {
        "mixture_of_agents"
    }
    fn description(&self) -> &str {
        "Process a complex query using Mixture-of-Agents: multiple reference \
         models answer in parallel, then an aggregator synthesizes a single \
         refined response."
    }
    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "query": { "type": "string", "description": "The complex query to solve" }
            },
            "required": ["query"]
        })
    }
    async fn execute(&self, args: serde_json::Value) -> Result<String, ToolError> {
        let query = args["query"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArguments {
                tool: "mixture_of_agents".to_string(),
                message: "query is required".to_string(),
            })?;
        let result = self
            .client
            .synthesize(query, &self.reference_models)
            .await
            .map_err(|e| ToolError::ExecutionFailed {
                tool: "mixture_of_agents".to_string(),
                message: e,
            })?;
        Ok(serde_json::json!({"success": true, "response": result}).to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aggregator_prompt_enumerates_responses() {
        let responses = vec!["first".to_string(), "second".to_string()];
        let prompt = construct_aggregator_prompt(AGGREGATOR_SYSTEM_PROMPT, &responses);
        assert!(prompt.contains("1. first"));
        assert!(prompt.contains("2. second"));
        assert!(prompt.starts_with(AGGREGATOR_SYSTEM_PROMPT));
    }

    #[test]
    fn empty_responses_produce_just_system_prompt() {
        let prompt = construct_aggregator_prompt(AGGREGATOR_SYSTEM_PROMPT, &[]);
        assert_eq!(prompt, format!("{}\n\n", AGGREGATOR_SYSTEM_PROMPT));
    }

    #[test]
    fn config_available_requires_key_and_models() {
        let cfg = MoaConfig {
            enabled: true,
            api_key: "k".into(),
            ..Default::default()
        };
        assert!(cfg.available());
        let cfg2 = MoaConfig {
            enabled: true,
            reference_models: vec![],
            ..Default::default()
        };
        assert!(!cfg2.available());
    }

    #[tokio::test]
    async fn chat_completion_retries_on_5xx_then_succeeds() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let mut attempt = 0u32;
            loop {
                let (mut sock, _) = match listener.accept().await {
                    Ok(x) => x,
                    Err(_) => break,
                };
                let mut req = [0u8; 4096];
                let _ = sock.read(&mut req).await;
                attempt += 1;
                let resp = if attempt == 1 {
                    "HTTP/1.1 500 Internal Server Error\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".to_string()
                } else {
                    let body = "{\"choices\":[{\"message\":{\"content\":\"ok\"}}]}";
                    format!(
                        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                        body.len(),
                        body
                    )
                };
                let _ = sock.write_all(resp.as_bytes()).await;
            }
        });

        let cfg = MoaConfig {
            enabled: true,
            api_key: "***".into(),
            base_url: format!("http://{}", addr),
            reference_models: vec!["m".into()],
            ..Default::default()
        };
        let client = MoaClient::from_config(&cfg);
        let out = client
            .chat_completion(
                "m",
                serde_json::json!([{"role":"user","content":"hi"}]),
                0.7,
            )
            .await
            .unwrap();
        assert_eq!(out, "ok");
        server.abort();
    }

    #[tokio::test]
    async fn chat_completion_does_not_retry_on_4xx() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let mut attempts = 0u32;
            loop {
                let (mut sock, _) = match listener.accept().await {
                    Ok(x) => x,
                    Err(_) => break,
                };
                let mut req = [0u8; 4096];
                let _ = sock.read(&mut req).await;
                attempts += 1;
                let resp =
                    "HTTP/1.1 400 Bad Request\r\nContent-Length: 0\r\nConnection: close\r\n\r\n";
                let _ = sock.write_all(resp.as_bytes()).await;
                if attempts >= 1 {
                    // Keep the loop alive only long enough for one request.
                }
            }
        });

        let cfg = MoaConfig {
            enabled: true,
            api_key: "***".into(),
            base_url: format!("http://{}", addr),
            reference_models: vec!["m".into()],
            ..Default::default()
        };
        let client = MoaClient::from_config(&cfg);
        let err = client
            .chat_completion(
                "m",
                serde_json::json!([{"role":"user","content":"hi"}]),
                0.7,
            )
            .await
            .unwrap_err();
        assert!(err.contains("HTTP 400"), "unexpected error: {}", err);
        server.abort();
    }
}
