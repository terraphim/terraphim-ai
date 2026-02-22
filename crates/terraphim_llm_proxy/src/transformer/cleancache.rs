//! CleanCache Transformer
//!
//! Removes cache_control fields from requests for providers that don't support
//! Claude's prompt caching feature.

use crate::{server::ChatResponse, token_counter::ChatRequest, Result};
use async_trait::async_trait;
use tracing::debug;

/// CleanCache transformer that strips cache control fields
pub struct CleanCacheTransformer;

impl CleanCacheTransformer {
    pub fn new() -> Self {
        Self
    }
}

impl Default for CleanCacheTransformer {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl crate::transformer::Transformer for CleanCacheTransformer {
    fn name(&self) -> &str {
        "cleancache"
    }

    async fn transform_request(&self, mut req: ChatRequest) -> Result<ChatRequest> {
        debug!("Applying CleanCache transformer - removing cache_control fields");

        // Remove cache_control from system prompt
        if let Some(system) = req.system.as_mut() {
            match system {
                crate::token_counter::SystemPrompt::Text(_) => {
                    // Text system prompts don't have cache control
                }
                crate::token_counter::SystemPrompt::Array(blocks) => {
                    // Filter out cache control blocks
                    let filtered_blocks: Vec<_> = blocks
                        .iter()
                        .filter(|block| {
                            !matches!(
                                block,
                                crate::token_counter::SystemBlock::CacheControl { .. }
                            )
                        })
                        .cloned()
                        .collect();

                    if filtered_blocks.len() != blocks.len() {
                        req.system =
                            Some(crate::token_counter::SystemPrompt::Array(filtered_blocks));
                    }
                }
            }
        }

        // For messages, ContentBlock doesn't have cache control, so we pass through
        debug!("CleanCache transformer - messages pass-through (no cache control in ContentBlock)");

        Ok(req)
    }

    async fn transform_response(&self, resp: ChatResponse) -> Result<ChatResponse> {
        // Response typically doesn't need cache cleaning, but implement for completeness
        debug!("CleanCache transformer - response pass-through");
        Ok(resp)
    }
}
