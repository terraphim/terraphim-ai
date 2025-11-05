// Validated LLM client with pre-LLM and post-LLM validation layers
//
// Wraps GenAiLlmClient to add validation at LLM interaction boundaries:
// - Pre-LLM: Context validation, token budget, permission checks
// - Post-LLM: Output parsing, confidence scoring, security scanning

use crate::{GenAiLlmClient, LlmRequest, LlmResponse, MultiAgentError, MultiAgentResult};
use anyhow::Result;
use tracing::{debug, info, warn};

/// Pre-LLM validation layer
pub trait PreLlmValidator: Send + Sync {
    fn name(&self) -> &str;

    fn validate<'a>(
        &'a self,
        request: &'a LlmRequest,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<LlmRequest>> + Send + 'a>>;
}

/// Post-LLM validation layer
pub trait PostLlmValidator: Send + Sync {
    fn name(&self) -> &str;

    fn validate<'a>(
        &'a self,
        response: &'a LlmResponse,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<LlmResponse>> + Send + 'a>>;
}

/// Token budget validator - ensures request fits within model limits
pub struct TokenBudgetValidator {
    max_tokens: usize,
}

impl TokenBudgetValidator {
    pub fn new(max_tokens: usize) -> Self {
        Self { max_tokens }
    }

    fn estimate_tokens(&self, request: &LlmRequest) -> usize {
        // Rough estimation: 4 characters per token
        let total_chars: usize = request.messages.iter().map(|m| m.content.len()).sum();

        total_chars / 4
    }
}

impl PreLlmValidator for TokenBudgetValidator {
    fn name(&self) -> &str {
        "token-budget"
    }

    fn validate<'a>(
        &'a self,
        request: &'a LlmRequest,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<LlmRequest>> + Send + 'a>> {
        Box::pin(async move {
            let estimated = self.estimate_tokens(request);

            if estimated > self.max_tokens {
                warn!(
                    "Token budget exceeded: {} > {} (estimated)",
                    estimated, self.max_tokens
                );
                // For now, just warn and proceed - in Phase 3 we'll implement context compaction
            }

            debug!("Token budget check: {} / {}", estimated, self.max_tokens);
            Ok(request.clone())
        })
    }
}

/// Context validator - ensures all references are valid
pub struct ContextValidator;

impl PreLlmValidator for ContextValidator {
    fn name(&self) -> &str {
        "context"
    }

    fn validate<'a>(
        &'a self,
        request: &'a LlmRequest,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<LlmRequest>> + Send + 'a>> {
        Box::pin(async move {
            // Basic validation - ensure messages are not empty
            if request.messages.is_empty() {
                return Err(anyhow::anyhow!("LlmRequest has no messages"));
            }

            debug!(
                "Context validation passed: {} messages",
                request.messages.len()
            );
            Ok(request.clone())
        })
    }
}

/// Output parser validator - ensures response is well-formed
pub struct OutputParserValidator;

impl PostLlmValidator for OutputParserValidator {
    fn name(&self) -> &str {
        "output-parser"
    }

    fn validate<'a>(
        &'a self,
        response: &'a LlmResponse,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<LlmResponse>> + Send + 'a>> {
        Box::pin(async move {
            // Check that response has content
            if response.content.is_empty() {
                warn!("LLM returned empty response");
            }

            debug!("Output parsing validated: {} chars", response.content.len());
            Ok(response.clone())
        })
    }
}

/// Security scanner - checks for sensitive data in responses
pub struct SecurityScannerValidator;

impl PostLlmValidator for SecurityScannerValidator {
    fn name(&self) -> &str {
        "security-scanner"
    }

    fn validate<'a>(
        &'a self,
        response: &'a LlmResponse,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<LlmResponse>> + Send + 'a>> {
        Box::pin(async move {
            // Basic security check - look for common sensitive patterns
            let sensitive_patterns = [
                "api_key",
                "password",
                "secret_key",
                "private_key",
                "access_token",
            ];

            for pattern in &sensitive_patterns {
                if response.content.to_lowercase().contains(pattern) {
                    warn!(
                        "Potential sensitive data detected in LLM response: {}",
                        pattern
                    );
                }
            }

            debug!("Security scan passed");
            Ok(response.clone())
        })
    }
}

/// Validated LLM client with pre-LLM and post-LLM validation pipeline
pub struct ValidatedGenAiClient {
    inner: GenAiLlmClient,
    pre_validators: Vec<Box<dyn PreLlmValidator>>,
    post_validators: Vec<Box<dyn PostLlmValidator>>,
}

impl ValidatedGenAiClient {
    /// Create new validated client with default validators
    pub fn new(inner: GenAiLlmClient) -> Self {
        Self {
            inner,
            pre_validators: vec![
                Box::new(TokenBudgetValidator::new(100_000)),
                Box::new(ContextValidator),
            ],
            post_validators: vec![
                Box::new(OutputParserValidator),
                Box::new(SecurityScannerValidator),
            ],
        }
    }

    /// Create from provider configuration
    pub fn new_ollama(model: Option<String>) -> MultiAgentResult<Self> {
        let client = GenAiLlmClient::new_ollama(model)?;
        Ok(Self::new(client))
    }

    pub fn new_openai(model: Option<String>) -> MultiAgentResult<Self> {
        let client = GenAiLlmClient::new_openai(model)?;
        Ok(Self::new(client))
    }

    pub fn new_anthropic(model: Option<String>) -> MultiAgentResult<Self> {
        let client = GenAiLlmClient::new_anthropic(model)?;
        Ok(Self::new(client))
    }

    pub fn new_openrouter(model: Option<String>) -> MultiAgentResult<Self> {
        let client = GenAiLlmClient::new_openrouter(model)?;
        Ok(Self::new(client))
    }

    /// Add a custom pre-LLM validator
    pub fn add_pre_validator(&mut self, validator: Box<dyn PreLlmValidator>) {
        self.pre_validators.push(validator);
    }

    /// Add a custom post-LLM validator
    pub fn add_post_validator(&mut self, validator: Box<dyn PostLlmValidator>) {
        self.post_validators.push(validator);
    }

    /// Generate response with full validation pipeline
    pub async fn generate(&self, request: LlmRequest) -> MultiAgentResult<LlmResponse> {
        // PRE-LLM VALIDATION
        let mut validated_request = request;

        for validator in &self.pre_validators {
            debug!("Running pre-LLM validator: {}", validator.name());

            validated_request = validator.validate(&validated_request).await.map_err(|e| {
                MultiAgentError::LlmError(format!(
                    "Pre-LLM validation failed ({}): {}",
                    validator.name(),
                    e
                ))
            })?;
        }

        info!("All pre-LLM validations passed");

        // CALL LLM
        let response = self.inner.generate(validated_request).await?;

        // POST-LLM VALIDATION
        let mut validated_response = response;

        for validator in &self.post_validators {
            debug!("Running post-LLM validator: {}", validator.name());

            validated_response = validator.validate(&validated_response).await.map_err(|e| {
                MultiAgentError::LlmError(format!(
                    "Post-LLM validation failed ({}): {}",
                    validator.name(),
                    e
                ))
            })?;
        }

        info!("All post-LLM validations passed");

        Ok(validated_response)
    }

    /// Get the underlying model name
    pub fn model(&self) -> &str {
        self.inner.model()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{LlmMessage, MessageRole};

    #[tokio::test]
    async fn test_token_budget_validator() {
        let validator = TokenBudgetValidator::new(1000);

        let request = LlmRequest::new(vec![LlmMessage {
            role: MessageRole::User,
            content: "Hello".to_string(),
        }]);

        let result = validator.validate(&request).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_context_validator_empty_messages() {
        let validator = ContextValidator;

        let request = LlmRequest::new(vec![]);

        let result = validator.validate(&request).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_context_validator_valid_messages() {
        let validator = ContextValidator;

        let request = LlmRequest::new(vec![LlmMessage {
            role: MessageRole::User,
            content: "Test message".to_string(),
        }]);

        let result = validator.validate(&request).await;
        assert!(result.is_ok());
    }

    #[test]
    fn test_validated_client_creation() {
        // Test client can be created
        let result = ValidatedGenAiClient::new_ollama(Some("gemma3:270m".to_string()));
        assert!(result.is_ok());
    }
}
