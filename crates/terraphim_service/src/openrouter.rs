//! OpenRouter service for AI-powered article summarization
//!
//! This module provides integration with OpenRouter's API to generate
//! intelligent summaries of article content instead of basic text excerpts.

#[cfg(feature = "openrouter")]
use reqwest;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum OpenRouterError {
    #[error("Feature not enabled: {0}")]
    FeatureDisabled(String),

    #[cfg(feature = "openrouter")]
    #[error("API error: {0}")]
    ApiError(String),

    #[cfg(feature = "openrouter")]
    #[error("HTTP error: {0}")]
    HttpError(#[from] reqwest::Error),

    #[cfg(feature = "openrouter")]
    #[error("Invalid configuration: {0}")]
    ConfigError(String),

    #[cfg(feature = "openrouter")]
    #[error("Rate limit exceeded")]
    RateLimited,

    #[cfg(feature = "openrouter")]
    #[error("Content too long: {0} characters (max: {1})")]
    ContentTooLong(usize, usize),
}

pub type Result<T> = std::result::Result<T, OpenRouterError>;

/// OpenRouter service for generating AI summaries
///
/// This service connects to OpenRouter's API to generate intelligent
/// summaries of article content using various language models.
#[cfg(feature = "openrouter")]
#[derive(Debug)]
pub struct OpenRouterService {
    client: reqwest::Client,
    api_key: String,
    model: String,
    base_url: String,
}

#[cfg(feature = "openrouter")]
impl OpenRouterService {
    /// Create a new OpenRouter service instance
    ///
    /// # Arguments
    /// * `api_key` - OpenRouter API key (starts with "sk-or-v1-")
    /// * `model` - Model name (e.g., "openai/gpt-3.5-turbo", "anthropic/claude-3-sonnet")
    ///
    /// # Examples
    /// ```rust
    /// use terraphim_service::openrouter::OpenRouterService;
    ///
    /// let service = OpenRouterService::new(
    ///     "sk-or-v1-your-api-key",
    ///     "openai/gpt-3.5-turbo"
    /// )?;
    /// ```
    pub fn new(api_key: &str, model: &str) -> Result<Self> {
        if api_key.is_empty() {
            return Err(OpenRouterError::ConfigError(
                "API key cannot be empty".to_string(),
            ));
        }

        if model.is_empty() {
            return Err(OpenRouterError::ConfigError(
                "Model name cannot be empty".to_string(),
            ));
        }

        let client =
            crate::http_client::create_api_client().unwrap_or_else(|_| reqwest::Client::new());

        // Determine base URL based on model and environment configuration
        let base_url = Self::determine_base_url(model);

        Ok(Self {
            client,
            api_key: api_key.to_string(),
            model: model.to_string(),
            base_url,
        })
    }

    /// Determine the appropriate base URL based on model and environment configuration
    ///
    /// This method supports the z.ai proxy for Anthropic models when the
    /// ANTHROPIC_BASE_URL environment variable is configured.
    fn determine_base_url(model: &str) -> String {
        // Check if this is an Anthropic model and z.ai proxy is configured
        if model.starts_with("anthropic/") || model.contains("claude") {
            // Check for z.ai proxy configuration
            if let Ok(anthropic_base_url) = std::env::var("ANTHROPIC_BASE_URL") {
                log::info!(
                    "🔗 Using z.ai proxy for Anthropic model: {} -> {}",
                    model,
                    anthropic_base_url
                );
                return anthropic_base_url;
            }
        }

        // Default to OpenRouter base URL (with environment override support)
        std::env::var("OPENROUTER_BASE_URL")
            .unwrap_or_else(|_| "https://openrouter.ai/api/v1".to_string())
    }

    /// Get the appropriate API key based on the model and configuration
    ///
    /// For Anthropic models using z.ai proxy, prefer ANTHROPIC_AUTH_TOKEN
    fn get_api_key(&self, provided_key: &str) -> String {
        // Check if this is an Anthropic model using z.ai proxy
        if (self.model.starts_with("anthropic/") || self.model.contains("claude"))
            && self.base_url.contains("z.ai")
        {
            // Prefer the z.ai auth token if available
            if let Ok(anthropic_token) = std::env::var("ANTHROPIC_AUTH_TOKEN") {
                log::info!("🔑 Using ANTHROPIC_AUTH_TOKEN for z.ai proxy");
                return anthropic_token;
            }
        }

        // Fall back to the provided API key
        provided_key.to_string()
    }

    /// Generate a summary for the given article content
    ///
    /// # Arguments
    /// * `content` - The article content to summarize
    /// * `max_length` - Maximum length of the summary in characters
    ///
    /// # Returns
    /// A concise summary of the article content
    ///
    /// # Examples
    /// ```rust
    /// let summary = service.generate_summary(
    ///     "Long article content...",
    ///     200
    /// ).await?;
    /// ```
    pub async fn generate_summary(&self, content: &str, max_length: usize) -> Result<String> {
        // Content validation
        const MAX_CONTENT_LENGTH: usize = 4000; // Reasonable limit for API calls
        if content.len() > MAX_CONTENT_LENGTH {
            return Err(OpenRouterError::ContentTooLong(
                content.len(),
                MAX_CONTENT_LENGTH,
            ));
        }

        if content.trim().is_empty() {
            return Ok("No content available for summarization.".to_string());
        }

        // Create the prompt for summarization
        let prompt = format!(
            "Please provide a concise and informative summary of the following article content. The summary should be approximately {} characters long and capture the main ideas, key points, and essential information. Focus on being clear and helpful to someone browsing search results.\n\nArticle content:\n{}",
            max_length, content
        );

        // Prepare the API request
        let request_body = serde_json::json!({
            "model": self.model,
            "messages": [
                {
                    "role": "user",
                    "content": prompt
                }
            ],
            "max_tokens": self.calculate_max_tokens(max_length),
            "temperature": 0.3, // Lower temperature for more focused summaries
            "top_p": 0.9,
            "stream": false
        });

        log::debug!("Sending OpenRouter API request for model: {}", self.model);

        // Get the appropriate API key for this request
        let api_key = self.get_api_key(&self.api_key);

        // Make the API call
        let response = self
            .client
            .post(format!("{}/chat/completions", self.base_url))
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .header("HTTP-Referer", "https://terraphim.ai") // Required by OpenRouter
            .header("X-Title", "Terraphim AI") // Optional but recommended
            .json(&request_body)
            .send()
            .await?;

        // Handle rate limiting
        if response.status() == 429 {
            log::warn!("OpenRouter API rate limit exceeded");
            return Err(OpenRouterError::RateLimited);
        }

        // Check for success status
        if !response.status().is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            log::error!("OpenRouter API error: {}", error_text);
            return Err(OpenRouterError::ApiError(error_text));
        }

        // Parse the response
        let response_json: serde_json::Value = response.json().await?;

        // Extract the summary from the response
        let summary = response_json
            .get("choices")
            .and_then(|choices| choices.get(0))
            .and_then(|choice| choice.get("message"))
            .and_then(|message| message.get("content"))
            .and_then(|content| content.as_str())
            .unwrap_or("Summary generation failed")
            .trim()
            .to_string();

        log::info!(
            "Generated summary: {} characters for model: {}",
            summary.len(),
            self.model
        );

        // Ensure the summary isn't too long
        if summary.len() > max_length + 50 {
            // Small buffer for natural language
            Ok(format!("{}...", &summary[..max_length.saturating_sub(3)]))
        } else {
            Ok(summary)
        }
    }

    /// Calculate appropriate max_tokens based on desired character length
    ///
    /// Rule of thumb: ~4 characters per token for English text
    fn calculate_max_tokens(&self, max_chars: usize) -> u32 {
        let tokens = (max_chars / 3).clamp(50, 500); // Reasonable bounds
        tokens as u32
    }

    /// Check if the service is properly configured
    pub fn is_configured(&self) -> bool {
        !self.api_key.is_empty() && !self.model.is_empty()
    }

    /// Get the model name being used
    pub fn get_model(&self) -> &str {
        &self.model
    }

    /// Get supported model recommendations
    pub fn get_recommended_models() -> Vec<(&'static str, &'static str, &'static str)> {
        vec![
            (
                "openai/gpt-3.5-turbo",
                "Fast and affordable",
                "General purpose",
            ),
            ("openai/gpt-4", "High quality summaries", "Premium quality"),
            (
                "anthropic/claude-3-sonnet",
                "Balanced performance",
                "Good middle ground",
            ),
            (
                "anthropic/claude-3-haiku",
                "Fast processing",
                "High throughput",
            ),
            (
                "mistralai/mixtral-8x7b-instruct",
                "Open source option",
                "Cost effective",
            ),
        ]
    }

    /// Perform a multi-turn chat completion with an array of messages
    pub async fn chat_completion(
        &self,
        messages: Vec<serde_json::Value>,
        max_tokens: Option<u32>,
        temperature: Option<f32>,
    ) -> Result<String> {
        let request_body = serde_json::json!({
            "model": self.model,
            "messages": messages,
            "max_tokens": max_tokens.unwrap_or(512),
            "temperature": temperature.unwrap_or(0.2),
            "stream": false
        });

        // Get the appropriate API key for this request
        let api_key = self.get_api_key(&self.api_key);

        let response = self
            .client
            .post(format!("{}/chat/completions", self.base_url))
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .header("HTTP-Referer", "https://terraphim.ai")
            .header("X-Title", "Terraphim AI")
            .json(&request_body)
            .send()
            .await?;

        if response.status() == 429 {
            return Err(OpenRouterError::RateLimited);
        }
        if !response.status().is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(OpenRouterError::ApiError(error_text));
        }

        let response_json: serde_json::Value = response.json().await?;
        let content = response_json
            .get("choices")
            .and_then(|c| c.get(0))
            .and_then(|c| c.get("message"))
            .and_then(|m| m.get("content"))
            .and_then(|t| t.as_str())
            .unwrap_or("")
            .to_string();
        Ok(content)
    }

    /// Fetch available models from OpenRouter
    pub async fn list_models(&self) -> Result<Vec<String>> {
        // Get the appropriate API key for this request
        let api_key = self.get_api_key(&self.api_key);

        let response = self
            .client
            .get(format!("{}/models", self.base_url))
            .header("Authorization", format!("Bearer {}", api_key))
            .header("HTTP-Referer", "https://terraphim.ai")
            .header("X-Title", "Terraphim AI")
            .send()
            .await?;

        if response.status() == 429 {
            return Err(OpenRouterError::RateLimited);
        }
        if !response.status().is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(OpenRouterError::ApiError(error_text));
        }

        let json: serde_json::Value = response.json().await?;
        // Accept both { data: [ {id: ..}, ... ] } and { models: [...] }
        let models = if let Some(arr) = json.get("data").and_then(|v| v.as_array()) {
            arr.iter()
                .filter_map(|m| {
                    m.get("id")
                        .and_then(|id| id.as_str())
                        .map(|s| s.to_string())
                })
                .collect::<Vec<_>>()
        } else if let Some(arr) = json.get("models").and_then(|v| v.as_array()) {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        } else {
            Vec::new()
        };
        Ok(models)
    }
}

// Stub implementation when the feature is disabled
#[cfg(not(feature = "openrouter"))]
pub struct OpenRouterService;

#[cfg(not(feature = "openrouter"))]
impl OpenRouterService {
    /// Create a stub service that returns an error when feature is disabled
    pub fn new(_api_key: &str, _model: &str) -> Result<Self> {
        Err(OpenRouterError::FeatureDisabled("openrouter".to_string()))
    }

    /// Stub method that returns an error when feature is disabled
    pub async fn generate_summary(&self, _content: &str, _max_length: usize) -> Result<String> {
        Err(OpenRouterError::FeatureDisabled("openrouter".to_string()))
    }

    /// Stub method for configuration check
    pub fn is_configured(&self) -> bool {
        false
    }

    /// Stub method for model name
    pub fn get_model(&self) -> &str {
        ""
    }

    /// Return empty recommendations when feature is disabled
    pub fn get_recommended_models() -> Vec<(&'static str, &'static str, &'static str)> {
        vec![]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_service_creation() {
        #[cfg(feature = "openrouter")]
        {
            let service = OpenRouterService::new("sk-or-v1-test-key", "openai/gpt-3.5-turbo");
            assert!(service.is_ok());

            let service = service.unwrap();
            assert!(service.is_configured());
            assert_eq!(service.get_model(), "openai/gpt-3.5-turbo");
        }

        #[cfg(not(feature = "openrouter"))]
        {
            let result = OpenRouterService::new("test", "test");
            assert!(matches!(
                result.unwrap_err(),
                OpenRouterError::FeatureDisabled(_)
            ));
        }
    }

    #[tokio::test]
    async fn test_configuration_validation() {
        #[cfg(feature = "openrouter")]
        {
            // Empty API key should fail
            let result = OpenRouterService::new("", "openai/gpt-3.5-turbo");
            assert!(matches!(
                result.unwrap_err(),
                OpenRouterError::ConfigError(_)
            ));

            // Empty model should fail
            let result = OpenRouterService::new("sk-or-v1-test", "");
            assert!(matches!(
                result.unwrap_err(),
                OpenRouterError::ConfigError(_)
            ));
        }
    }

    #[tokio::test]
    async fn test_content_validation() {
        #[cfg(feature = "openrouter")]
        {
            let service =
                OpenRouterService::new("sk-or-v1-test-key", "openai/gpt-3.5-turbo").unwrap();

            // Empty content should return appropriate message
            let result = service.generate_summary("", 100).await;
            assert!(result.is_ok());
            assert_eq!(result.unwrap(), "No content available for summarization.");

            // Very long content should be rejected
            let long_content = "a".repeat(5000);
            let result = service.generate_summary(&long_content, 100).await;
            assert!(matches!(
                result.unwrap_err(),
                OpenRouterError::ContentTooLong(_, _)
            ));
        }
    }

    #[test]
    fn test_recommended_models() {
        let models = OpenRouterService::get_recommended_models();

        #[cfg(feature = "openrouter")]
        {
            assert!(!models.is_empty());
            assert!(models.len() >= 5); // Should have at least 5 recommended models

            // Check that we have expected models
            let model_names: Vec<&str> = models.iter().map(|(name, _, _)| *name).collect();
            assert!(model_names.contains(&"openai/gpt-3.5-turbo"));
            assert!(model_names.contains(&"anthropic/claude-3-sonnet"));
        }

        #[cfg(not(feature = "openrouter"))]
        assert!(models.is_empty());
    }

    #[test]
    fn test_max_tokens_calculation() {
        #[cfg(feature = "openrouter")]
        {
            let service = OpenRouterService::new("sk-or-v1-test", "test").unwrap();

            // Test reasonable bounds
            assert_eq!(service.calculate_max_tokens(150), 50); // Minimum
            assert_eq!(service.calculate_max_tokens(300), 100);
            assert_eq!(service.calculate_max_tokens(1500), 500); // Maximum
            assert_eq!(service.calculate_max_tokens(3000), 500); // Capped at maximum
        }
    }

    #[cfg(feature = "openrouter")]
    #[tokio::test]
    async fn test_error_handling() {
        let service = OpenRouterService::new("invalid-key", "openai/gpt-3.5-turbo").unwrap();

        // Test with actual content but invalid key - should fail with API error
        let result = service
            .generate_summary("This is test content for summarization.", 100)
            .await;
        assert!(result.is_err());

        // The error should be either API error or HTTP error
        match result.unwrap_err() {
            OpenRouterError::ApiError(_)
            | OpenRouterError::HttpError(_)
            | OpenRouterError::RateLimited => {
                // Expected error types
            }
            other => panic!("Unexpected error type: {:?}", other),
        }
    }

    #[cfg(feature = "openrouter")]
    #[test]
    fn test_prompt_generation() {
        let service = OpenRouterService::new("sk-or-v1-test", "openai/gpt-3.5-turbo").unwrap();

        // Test that the service handles different content lengths appropriately
        let _short_content = "Short content.";
        let _medium_content = "This is a medium-length piece of content that should be suitable for summarization. It contains multiple sentences and provides enough context for a meaningful summary to be generated.";
        let _long_content = "a".repeat(4000);

        // These are internal tests - we can't easily test the actual HTTP call without mocking
        // But we can test the token calculation and validation logic
        assert_eq!(service.calculate_max_tokens(200), 66);
        assert_eq!(service.calculate_max_tokens(500), 166);
    }
}
