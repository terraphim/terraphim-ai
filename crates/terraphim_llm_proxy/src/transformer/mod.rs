//! Transformer module for provider-specific API adaptations
//!
//! Transformers convert between Claude's API format and other providers' formats.

use crate::{server::ChatResponse, token_counter::ChatRequest, Result};
use async_trait::async_trait;

pub mod anthropic;
pub mod cleancache;
pub mod deepseek;
pub mod gemini;
pub mod ollama;
pub mod openai;
pub mod openrouter;
// Advanced transformers - enabling one by one
pub mod enhancetool;
pub mod maxtoken;
pub mod reasoning;
pub mod sampling;
pub mod tooluse;

/// Transformer trait for request/response transformations
#[async_trait]
pub trait Transformer: Send + Sync {
    /// Get the transformer name
    fn name(&self) -> &str;

    /// Transform outgoing request to provider format
    async fn transform_request(&self, req: ChatRequest) -> Result<ChatRequest> {
        // Default: pass through unchanged
        Ok(req)
    }

    /// Transform incoming response from provider format
    async fn transform_response(&self, resp: ChatResponse) -> Result<ChatResponse> {
        // Default: pass through unchanged
        Ok(resp)
    }
}

/// Chain of transformers applied in sequence
pub struct TransformerChain {
    transformers: Vec<Box<dyn Transformer>>,
}

impl TransformerChain {
    /// Create a new transformer chain
    pub fn new() -> Self {
        Self {
            transformers: Vec::new(),
        }
    }

    /// Add a transformer to the chain
    pub fn append(mut self, transformer: Box<dyn Transformer>) -> Self {
        self.transformers.push(transformer);
        self
    }

    /// Create chain from transformer names
    pub fn from_names(names: &[String]) -> Self {
        let mut chain = Self::new();

        for name in names {
            let transformer: Box<dyn Transformer> = match name.as_str() {
                "anthropic" => Box::new(anthropic::AnthropicTransformer),
                "cleancache" => Box::new(cleancache::CleanCacheTransformer),
                "deepseek" => Box::new(deepseek::DeepSeekTransformer),
                "gemini" => Box::new(gemini::GeminiTransformer),
                "openai" => Box::new(openai::OpenAITransformer),
                "openrouter" => Box::new(openrouter::OpenRouterTransformer),
                "ollama" => Box::new(ollama::OllamaTransformer),
                "reasoning" => Box::new(reasoning::ReasoningTransformer),
                // "enhancetool" => Box::new(enhancetool::EnhanceToolTransformer::default()),
                // "maxtoken" => Box::new(maxtoken::MaxTokenTransformer::default()),
                // "sampling" => Box::new(sampling::SamplingTransformer::default()),
                // "tooluse" => Box::new(tooluse::ToolUseTransformer::default()),
                _ => {
                    tracing::warn!("Unknown transformer: {}, skipping", name);
                    continue;
                }
            };
            chain = chain.append(transformer);
        }

        chain
    }

    /// Apply all transformers to a request
    pub async fn transform_request(&self, mut req: ChatRequest) -> Result<ChatRequest> {
        for transformer in &self.transformers {
            tracing::debug!("Applying transformer: {}", transformer.name());
            req = transformer.transform_request(req).await?;
        }
        Ok(req)
    }

    /// Apply all transformers to a response (in reverse order)
    pub async fn transform_response(&self, mut resp: ChatResponse) -> Result<ChatResponse> {
        // Apply in reverse order
        for transformer in self.transformers.iter().rev() {
            tracing::debug!("Applying response transformer: {}", transformer.name());
            resp = transformer.transform_response(resp).await?;
        }
        Ok(resp)
    }

    /// Get the number of transformers in the chain
    pub fn len(&self) -> usize {
        self.transformers.len()
    }

    /// Check if the chain is empty
    pub fn is_empty(&self) -> bool {
        self.transformers.is_empty()
    }
}

impl Default for TransformerChain {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_empty_chain() {
        let chain = TransformerChain::new();
        assert_eq!(chain.len(), 0);
        assert!(chain.is_empty());
    }

    #[tokio::test]
    async fn test_chain_from_names() {
        let names = vec![
            "anthropic".to_string(),
            "deepseek".to_string(),
            "unknown".to_string(),
        ];
        let chain = TransformerChain::from_names(&names);

        // Should have 2 transformers (unknown is skipped)
        assert_eq!(chain.len(), 2);
    }
}
