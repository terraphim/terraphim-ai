//! Fallback Cerebras models for offline builds
//!
//! This module provides a static list of known Cerebras models used when
//! the Cerebras API is unreachable during build (offline builds, CI without API key).

/// Fallback model list when Cerebras API is unreachable
/// Format: (id, created_timestamp, owned_by)
pub const FALLBACK_MODELS: &[(&str, u64, &str)] = &[
    ("llama3.1-8b", 1721692800, "Meta"),
    ("llama3.1-70b", 1721692800, "Meta"),
    ("llama-3.3-70b", 1733443200, "Meta"),
    ("qwen-3-32b", 1733443200, "Alibaba"),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fallback_models_not_empty() {
        assert!(
            !FALLBACK_MODELS.is_empty(),
            "FALLBACK_MODELS should not be empty"
        );
    }

    #[test]
    fn test_fallback_contains_key_models() {
        let model_ids: Vec<&str> = FALLBACK_MODELS.iter().map(|(id, _, _)| *id).collect();

        assert!(
            model_ids.contains(&"llama3.1-8b"),
            "Should contain llama3.1-8b"
        );
        assert!(
            model_ids.contains(&"llama3.1-70b"),
            "Should contain llama3.1-70b"
        );
    }

    #[test]
    fn test_fallback_model_format() {
        for (id, created, owned_by) in FALLBACK_MODELS {
            assert!(!id.is_empty(), "Model ID should not be empty");
            assert!(*created > 0, "Created timestamp should be positive");
            assert!(!owned_by.is_empty(), "Owned by should not be empty");
        }
    }
}
