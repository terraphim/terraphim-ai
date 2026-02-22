//! Gemini transformer (placeholder for future implementation)

use crate::transformer::Transformer;
use async_trait::async_trait;

/// Gemini transformer - adapts to Google's Gemini API format
pub struct GeminiTransformer;

#[async_trait]
impl Transformer for GeminiTransformer {
    fn name(&self) -> &str {
        "gemini"
    }

    // TODO: Implement Gemini-specific transformations
    // - Convert system prompt to first user message
    // - Handle content structure differences
    // Use default pass-through for now
}
