//! Provider-scoped model exclusion with wildcard patterns.
//!
//! Provides model filtering based on configurable exclusion patterns per provider.
//! Supports glob patterns for flexible matching.
//!
//! # Example
//!
//! ```rust,ignore
//! use terraphim_llm_proxy::routing::exclusion::{ModelExclusion, is_excluded};
//!
//! let exclusions = vec![
//!     ModelExclusion::new("openrouter", vec!["*-preview", "*-beta"]),
//!     ModelExclusion::new("deepseek", vec!["*-test"]),
//! ];
//!
//! // Model excluded for openrouter
//! assert!(is_excluded("claude-3-preview", &exclusions, "openrouter"));
//!
//! // Same model not excluded for deepseek
//! assert!(!is_excluded("claude-3-preview", &exclusions, "deepseek"));
//! ```

use glob_match::glob_match;
use serde::{Deserialize, Serialize};
use tracing::debug;

/// Model exclusion configuration for provider-scoped filtering.
///
/// Defines patterns that should be excluded from routing for a specific provider.
/// Multiple exclusions can be defined per provider.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ModelExclusion {
    /// Provider this exclusion applies to.
    pub provider: String,

    /// Glob patterns to exclude (e.g., "*-preview", "*-beta").
    /// Models matching any pattern will be filtered out.
    pub patterns: Vec<String>,
}

impl ModelExclusion {
    /// Create a new model exclusion.
    ///
    /// # Arguments
    /// * `provider` - Provider name this exclusion applies to
    /// * `patterns` - List of glob patterns to exclude
    pub fn new(provider: impl Into<String>, patterns: Vec<impl Into<String>>) -> Self {
        Self {
            provider: provider.into(),
            patterns: patterns.into_iter().map(Into::into).collect(),
        }
    }

    /// Check if this exclusion applies to the given provider.
    ///
    /// Matching is case-insensitive.
    pub fn applies_to(&self, provider: &str) -> bool {
        self.provider.eq_ignore_ascii_case(provider)
    }

    /// Check if a model matches any exclusion pattern.
    ///
    /// Matching is case-insensitive and supports glob patterns.
    pub fn matches(&self, model: &str) -> bool {
        let model_lower = model.to_lowercase();

        for pattern in &self.patterns {
            let pattern_lower = pattern.to_lowercase();
            if glob_match(&pattern_lower, &model_lower) {
                return true;
            }
        }

        false
    }
}

/// Check if a model should be excluded for a specific provider.
///
/// Iterates through all exclusions and checks if any pattern matches
/// for the given provider. Uses case-insensitive glob matching.
///
/// # Arguments
/// * `model` - The model name to check
/// * `exclusions` - List of model exclusions to check against
/// * `provider` - The provider to check exclusions for
///
/// # Returns
/// true if the model matches any exclusion pattern for this provider
pub fn is_excluded(model: &str, exclusions: &[ModelExclusion], provider: &str) -> bool {
    for exclusion in exclusions {
        if exclusion.applies_to(provider) && exclusion.matches(model) {
            debug!(
                "Model '{}' excluded for provider '{}' (matched exclusion)",
                model, provider
            );
            return true;
        }
    }

    false
}

/// Get all exclusion patterns for a specific provider.
///
/// Collects patterns from all exclusions that apply to the given provider.
///
/// # Arguments
/// * `exclusions` - List of model exclusions
/// * `provider` - The provider to get exclusions for
///
/// # Returns
/// Vector of pattern strings that apply to this provider
pub fn get_exclusions_for_provider<'a>(
    exclusions: &'a [ModelExclusion],
    provider: &str,
) -> Vec<&'a str> {
    exclusions
        .iter()
        .filter(|e| e.applies_to(provider))
        .flat_map(|e| e.patterns.iter().map(String::as_str))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exclusion_exact_pattern() {
        let exclusions = vec![ModelExclusion::new("openrouter", vec!["deprecated-model"])];

        assert!(is_excluded("deprecated-model", &exclusions, "openrouter"));
        assert!(!is_excluded("other-model", &exclusions, "openrouter"));
    }

    #[test]
    fn test_exclusion_wildcard_prefix() {
        let exclusions = vec![ModelExclusion::new(
            "openrouter",
            vec!["*-preview", "*-beta"],
        )];

        assert!(is_excluded("claude-3-preview", &exclusions, "openrouter"));
        assert!(is_excluded("gpt-4-beta", &exclusions, "openrouter"));
        assert!(!is_excluded("claude-3-opus", &exclusions, "openrouter"));
    }

    #[test]
    fn test_exclusion_wildcard_suffix() {
        let exclusions = vec![ModelExclusion::new("openrouter", vec!["claude-*"])];

        assert!(is_excluded("claude-3-opus", &exclusions, "openrouter"));
        assert!(is_excluded("claude-fast", &exclusions, "openrouter"));
        assert!(!is_excluded("gpt-4", &exclusions, "openrouter"));
    }

    #[test]
    fn test_exclusion_wildcard_middle() {
        let exclusions = vec![ModelExclusion::new("openrouter", vec!["claude-*-preview"])];

        assert!(is_excluded("claude-3-preview", &exclusions, "openrouter"));
        assert!(is_excluded("claude-v2-preview", &exclusions, "openrouter"));
        assert!(!is_excluded("claude-3-opus", &exclusions, "openrouter"));
    }

    #[test]
    fn test_exclusion_case_insensitive() {
        let exclusions = vec![ModelExclusion::new("OpenRouter", vec!["*-PREVIEW"])];

        // Pattern case insensitive
        assert!(is_excluded("claude-3-preview", &exclusions, "openrouter"));
        assert!(is_excluded("CLAUDE-3-PREVIEW", &exclusions, "openrouter"));

        // Provider case insensitive
        assert!(is_excluded("claude-3-preview", &exclusions, "OPENROUTER"));
        assert!(is_excluded("claude-3-preview", &exclusions, "OpenRouter"));
    }

    #[test]
    fn test_exclusion_provider_scoped() {
        let exclusions = vec![
            ModelExclusion::new("openrouter", vec!["*-preview"]),
            ModelExclusion::new("deepseek", vec!["*-test"]),
        ];

        // Preview excluded for openrouter only
        assert!(is_excluded("claude-3-preview", &exclusions, "openrouter"));
        assert!(!is_excluded("claude-3-preview", &exclusions, "deepseek"));

        // Test excluded for deepseek only
        assert!(is_excluded("model-test", &exclusions, "deepseek"));
        assert!(!is_excluded("model-test", &exclusions, "openrouter"));
    }

    #[test]
    fn test_exclusion_multiple_patterns() {
        let exclusions = vec![ModelExclusion::new(
            "openrouter",
            vec!["*-preview", "*-beta", "*-experimental", "deprecated-*"],
        )];

        // Any pattern match should exclude
        assert!(is_excluded("claude-3-preview", &exclusions, "openrouter"));
        assert!(is_excluded("gpt-4-beta", &exclusions, "openrouter"));
        assert!(is_excluded("test-experimental", &exclusions, "openrouter"));
        assert!(is_excluded("deprecated-model", &exclusions, "openrouter"));

        // No match should pass
        assert!(!is_excluded("claude-3-opus", &exclusions, "openrouter"));
    }

    #[test]
    fn test_exclusion_no_match() {
        let exclusions = vec![ModelExclusion::new("openrouter", vec!["*-preview"])];

        assert!(!is_excluded("claude-3-opus", &exclusions, "openrouter"));
        assert!(!is_excluded("gpt-4-turbo", &exclusions, "openrouter"));
        assert!(!is_excluded("llama-2-70b", &exclusions, "openrouter"));
    }

    #[test]
    fn test_exclusion_empty_exclusions() {
        let exclusions: Vec<ModelExclusion> = vec![];

        assert!(!is_excluded("any-model", &exclusions, "any-provider"));
    }

    #[test]
    fn test_exclusion_unknown_provider() {
        let exclusions = vec![ModelExclusion::new("openrouter", vec!["*-preview"])];

        // Unknown provider should not match any exclusions
        assert!(!is_excluded("claude-3-preview", &exclusions, "unknown"));
    }

    #[test]
    fn test_get_exclusions_for_provider() {
        let exclusions = vec![
            ModelExclusion::new("openrouter", vec!["*-preview", "*-beta"]),
            ModelExclusion::new("deepseek", vec!["*-test"]),
            ModelExclusion::new("openrouter", vec!["deprecated-*"]),
        ];

        let openrouter_patterns = get_exclusions_for_provider(&exclusions, "openrouter");
        assert_eq!(openrouter_patterns.len(), 3);
        assert!(openrouter_patterns.contains(&"*-preview"));
        assert!(openrouter_patterns.contains(&"*-beta"));
        assert!(openrouter_patterns.contains(&"deprecated-*"));

        let deepseek_patterns = get_exclusions_for_provider(&exclusions, "deepseek");
        assert_eq!(deepseek_patterns.len(), 1);
        assert!(deepseek_patterns.contains(&"*-test"));

        let unknown_patterns = get_exclusions_for_provider(&exclusions, "unknown");
        assert!(unknown_patterns.is_empty());
    }

    #[test]
    fn test_get_exclusions_for_provider_case_insensitive() {
        let exclusions = vec![ModelExclusion::new("OpenRouter", vec!["*-preview"])];

        let patterns = get_exclusions_for_provider(&exclusions, "openrouter");
        assert_eq!(patterns.len(), 1);

        let patterns = get_exclusions_for_provider(&exclusions, "OPENROUTER");
        assert_eq!(patterns.len(), 1);
    }

    #[test]
    fn test_model_exclusion_new() {
        let exclusion = ModelExclusion::new("provider", vec!["pattern1", "pattern2"]);

        assert_eq!(exclusion.provider, "provider");
        assert_eq!(exclusion.patterns, vec!["pattern1", "pattern2"]);
    }

    #[test]
    fn test_model_exclusion_serialization() {
        let exclusion = ModelExclusion::new("openrouter", vec!["*-preview", "*-beta"]);

        let json = serde_json::to_string(&exclusion).unwrap();
        assert!(json.contains("\"provider\":\"openrouter\""));
        assert!(json.contains("\"patterns\":[\"*-preview\",\"*-beta\"]"));

        let deserialized: ModelExclusion = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, exclusion);
    }

    #[test]
    fn test_model_exclusion_applies_to() {
        let exclusion = ModelExclusion::new("OpenRouter", vec!["*-preview"]);

        assert!(exclusion.applies_to("openrouter"));
        assert!(exclusion.applies_to("OPENROUTER"));
        assert!(exclusion.applies_to("OpenRouter"));
        assert!(!exclusion.applies_to("deepseek"));
    }

    #[test]
    fn test_model_exclusion_matches() {
        let exclusion = ModelExclusion::new("provider", vec!["*-preview", "test-*"]);

        assert!(exclusion.matches("claude-3-preview"));
        assert!(exclusion.matches("test-model"));
        assert!(!exclusion.matches("claude-3-opus"));
    }
}
