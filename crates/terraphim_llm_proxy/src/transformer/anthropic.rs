//! Anthropic transformer (pass-through, no transformation needed)

use crate::transformer::Transformer;
use async_trait::async_trait;

/// Anthropic transformer - pass through unchanged (Claude API format)
pub struct AnthropicTransformer;

#[async_trait]
impl Transformer for AnthropicTransformer {
    fn name(&self) -> &str {
        "anthropic"
    }

    // Use default implementations (pass-through)
}
