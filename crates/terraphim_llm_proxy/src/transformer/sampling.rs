//! Sampling Transformer
//!
//! Advanced sampling parameters for fine-tuned model output control.
//! Supports temperature, top_p, top_k, repetition_penalty, and other parameters.

use crate::{server::ChatResponse, token_counter::ChatRequest, Result};
use async_trait::async_trait;

use tracing::debug;

/// Sampling transformer for advanced parameter control
pub struct SamplingTransformer {
    /// Default temperature if not specified
    default_temperature: f32,
    /// Default top_p if not specified  
    default_top_p: f32,
    /// Default top_k if not specified
    default_top_k: Option<u32>,
    /// Default repetition penalty if not specified
    default_repetition_penalty: Option<f32>,
}

impl SamplingTransformer {
    pub fn new() -> Self {
        Self {
            default_temperature: 0.7,
            default_top_p: 1.0,
            default_top_k: None,
            default_repetition_penalty: None,
        }
    }

    pub fn with_temperature(mut self, temperature: f32) -> Self {
        self.default_temperature = temperature.clamp(0.0, 2.0);
        self
    }

    pub fn with_top_p(mut self, top_p: f32) -> Self {
        self.default_top_p = top_p.clamp(0.0, 1.0);
        self
    }

    pub fn with_top_k(mut self, top_k: u32) -> Self {
        self.default_top_k = Some(top_k);
        self
    }

    pub fn with_repetition_penalty(mut self, penalty: f32) -> Self {
        self.default_repetition_penalty = Some(penalty.clamp(0.0, 2.0));
        self
    }
}

impl Default for SamplingTransformer {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl crate::transformer::Transformer for SamplingTransformer {
    fn name(&self) -> &str {
        "sampling"
    }

    async fn transform_request(&self, mut req: ChatRequest) -> Result<ChatRequest> {
        debug!("Applying Sampling transformer - configuring advanced parameters");

        // Set temperature if not already set
        if req.temperature.is_none() {
            req.temperature = Some(self.default_temperature);
            debug!("Set default temperature: {}", self.default_temperature);
        } else {
            // Validate and clamp existing temperature
            if let Some(temp) = req.temperature {
                req.temperature = Some(temp.clamp(0.0, 2.0));
            }
        }

        // Note: top_p, top_k, and repetition_penalty are not currently supported
        // in the ChatRequest structure. These would need to be added as additional fields
        // or handled differently based on the target provider's requirements.

        Ok(req)
    }

    async fn transform_response(&self, resp: ChatResponse) -> Result<ChatResponse> {
        debug!("Sampling transformer - response pass-through");
        Ok(resp)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transformer::Transformer;

    #[tokio::test]
    async fn test_applies_default_sampling_parameters() {
        let transformer = SamplingTransformer::new();

        let req = ChatRequest {
            model: "test-model".to_string(),
            ..Default::default()
        };

        let transformed = transformer.transform_request(req).await.unwrap();

        assert_eq!(transformed.temperature.unwrap(), 0.7);
    }

    #[tokio::test]
    async fn test_preserves_existing_parameters() {
        let transformer = SamplingTransformer::new();

        let req = ChatRequest {
            model: "test-model".to_string(),
            temperature: Some(1.5),
            ..Default::default()
        };

        let transformed = transformer.transform_request(req).await.unwrap();

        assert_eq!(transformed.temperature.unwrap(), 1.5);
    }

    #[tokio::test]
    async fn test_clamps_values() {
        let transformer = SamplingTransformer::new();

        let req = ChatRequest {
            model: "test-model".to_string(),
            temperature: Some(5.0), // Too high
            ..Default::default()
        };

        let transformed = transformer.transform_request(req).await.unwrap();

        assert_eq!(transformed.temperature.unwrap(), 2.0); // Clamped
    }
}
