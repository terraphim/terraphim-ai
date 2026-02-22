//! OpenRouter transformer
//!
//! Handles OpenRouter-specific request transformations.
//! Model name mappings should be configured via `[[router.model_mappings]]` in TOML.
//!
//! This transformer only handles:
//! - Stripping provider prefixes (e.g., "openrouter:model" -> "model")
//! - Adding "anthropic/" prefix for Claude models not already formatted

use crate::{server::ChatResponse, token_counter::ChatRequest, transformer::Transformer, Result};
use async_trait::async_trait;
use tracing::debug;

/// OpenRouter transformer - handles OpenRouter-specific format conversion
///
/// Model name mappings are now configured via TOML `[[router.model_mappings]]`.
/// This transformer only handles provider-prefix stripping and anthropic/ prefix addition.
pub struct OpenRouterTransformer;

impl OpenRouterTransformer {
    /// Process model name for OpenRouter format
    ///
    /// This function:
    /// 1. Strips provider prefixes (e.g., "openrouter:model" -> "model")
    /// 2. Adds "anthropic/" prefix for Claude models not already formatted
    /// 3. Passes through other models unchanged
    ///
    /// For complex model name mappings, use `[[router.model_mappings]]` in config.
    fn process_model_name(model: &str) -> String {
        // Remove provider prefix if present (e.g., "openrouter:model" -> "model")
        let model = if let Some(colon_pos) = model.find(':') {
            &model[colon_pos + 1..]
        } else {
            model
        };

        // If already in OpenRouter format (anthropic/...), keep as is
        if model.starts_with("anthropic/") {
            return model.to_string();
        }

        // Add anthropic/ prefix for Claude models not already formatted
        if model.starts_with("claude") {
            return format!("anthropic/{}", model);
        }

        // Pass through other models unchanged
        model.to_string()
    }
}

#[async_trait]
impl Transformer for OpenRouterTransformer {
    fn name(&self) -> &str {
        "openrouter"
    }

    async fn transform_request(&self, mut req: ChatRequest) -> Result<ChatRequest> {
        let original_model = req.model.clone();
        req.model = Self::process_model_name(&req.model);

        if original_model != req.model {
            debug!(
                original = %original_model,
                processed = %req.model,
                "OpenRouter: processed model name"
            );
        }

        Ok(req)
    }

    async fn transform_response(&self, resp: ChatResponse) -> Result<ChatResponse> {
        // OpenRouter responses need minimal transformation
        Ok(resp)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_process_model_name_passthrough() {
        // Already in OpenRouter format - should pass through
        assert_eq!(
            OpenRouterTransformer::process_model_name("anthropic/claude-3.5-sonnet"),
            "anthropic/claude-3.5-sonnet"
        );

        // Non-Claude models pass through unchanged
        assert_eq!(
            OpenRouterTransformer::process_model_name("gpt-4o"),
            "gpt-4o"
        );
    }

    #[test]
    fn test_process_model_name_strips_provider_prefix() {
        // Strips provider prefix and adds anthropic/
        assert_eq!(
            OpenRouterTransformer::process_model_name("openrouter:claude-3.5-sonnet"),
            "anthropic/claude-3.5-sonnet"
        );

        // Strips provider prefix for non-Claude models
        assert_eq!(
            OpenRouterTransformer::process_model_name("openrouter:gpt-4o"),
            "gpt-4o"
        );
    }

    #[test]
    fn test_process_model_name_adds_anthropic_prefix() {
        // Claude models get anthropic/ prefix added
        assert_eq!(
            OpenRouterTransformer::process_model_name("claude-3.5-sonnet"),
            "anthropic/claude-3.5-sonnet"
        );

        assert_eq!(
            OpenRouterTransformer::process_model_name("claude-3-opus"),
            "anthropic/claude-3-opus"
        );

        // Works with any Claude model name pattern
        assert_eq!(
            OpenRouterTransformer::process_model_name("claude-any-model"),
            "anthropic/claude-any-model"
        );
    }

    #[test]
    fn test_process_model_name_already_formatted() {
        // Already formatted models pass through unchanged
        assert_eq!(
            OpenRouterTransformer::process_model_name("anthropic/claude-sonnet-4.5"),
            "anthropic/claude-sonnet-4.5"
        );

        assert_eq!(
            OpenRouterTransformer::process_model_name("openrouter:anthropic/claude-3.5-sonnet"),
            "anthropic/claude-3.5-sonnet"
        );
    }
}
