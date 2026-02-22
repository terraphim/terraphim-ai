//! Fallback Kimi model definitions
//!
//! This module provides a static fallback list of Kimi models for when
//! the API is unreachable during build time.

/// Kimi model information
#[derive(Debug, Clone, Copy)]
pub struct KimiModel {
    pub id: &'static str,
    pub created: u64,
    pub display_name: &'static str,
    pub context_length: u64,
    pub supports_reasoning: bool,
}

/// Fallback Kimi models (used when API fetch fails)
pub const KIMI_MODELS: &[KimiModel] = &[KimiModel {
    id: "kimi-for-coding",
    created: 1761264000,
    display_name: "Kimi For Coding",
    context_length: 262144,
    supports_reasoning: true,
}];

/// Check if a model ID is a known Kimi model
pub fn is_valid_kimi_model(model_id: &str) -> bool {
    KIMI_MODELS.iter().any(|m| m.id == model_id)
}

/// Get model info by ID
pub fn get_kimi_model(model_id: &str) -> Option<&'static KimiModel> {
    KIMI_MODELS.iter().find(|m| m.id == model_id)
}
