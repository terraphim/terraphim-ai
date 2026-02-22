//! Fallback Groq model list for offline builds
//!
//! This file contains a static list of known Groq models used when the build script
//! cannot fetch the model list from the Groq API (e.g., offline builds, CI without API key).
//!
//! Update this file periodically from https://console.groq.com/docs/models
//! Last updated: 2026-02-01

/// Fallback model data: (id, context_window, max_completion_tokens, owned_by)
pub const FALLBACK_MODELS: &[(&str, u32, Option<u32>, &str)] = &[
    // Production Models - Meta Llama
    ("llama-3.3-70b-versatile", 131072, Some(32768), "Meta"),
    ("llama-3.1-8b-instant", 131072, Some(8192), "Meta"),
    ("llama-guard-4-12b", 131072, Some(8192), "Meta"),
    // Production Models - OpenAI
    ("gpt-oss-20b", 131072, Some(8192), "OpenAI"),
    ("whisper-large-v3", 0, None, "OpenAI"),
    ("whisper-large-v3-turbo", 0, None, "OpenAI"),
    // Production Systems - Groq
    ("compound", 131072, Some(8192), "Groq"),
    ("compound-mini", 131072, Some(8192), "Groq"),
    // Preview Models (may change)
    ("llama-3.1-70b-versatile", 131072, Some(32768), "Meta"),
    ("mixtral-8x7b-32768", 32768, Some(32768), "Mistral"),
    ("gemma2-9b-it", 8192, Some(8192), "Google"),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fallback_models_not_empty() {
        assert!(
            !FALLBACK_MODELS.is_empty(),
            "Fallback models should not be empty"
        );
    }

    #[test]
    fn test_fallback_contains_key_models() {
        let model_ids: Vec<&str> = FALLBACK_MODELS.iter().map(|(id, _, _, _)| *id).collect();

        // Verify key production models are present
        assert!(
            model_ids.contains(&"llama-3.3-70b-versatile"),
            "Should contain llama-3.3-70b-versatile"
        );
        assert!(
            model_ids.contains(&"llama-3.1-8b-instant"),
            "Should contain llama-3.1-8b-instant"
        );
    }

    #[test]
    fn test_fallback_model_format() {
        for (id, context_window, _max_tokens, owned_by) in FALLBACK_MODELS {
            assert!(!id.is_empty(), "Model ID should not be empty");
            assert!(!owned_by.is_empty(), "Owner should not be empty");
            // Context window of 0 is valid for audio models
            if !id.contains("whisper") {
                assert!(
                    *context_window > 0,
                    "Context window should be > 0 for non-audio models"
                );
            }
        }
    }
}
