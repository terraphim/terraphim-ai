//! Model aliasing with glob pattern matching.
//!
//! Provides model name resolution using configurable mappings with glob patterns.
//! Supports case-insensitive matching and bidirectional remapping.
//!
//! # Example
//!
//! ```rust,ignore
//! use terraphim_llm_proxy::routing::aliasing::{ModelMapping, resolve_model};
//!
//! let mappings = vec![
//!     ModelMapping::new("claude-fast", "deepseek,deepseek-chat"),
//!     ModelMapping::with_bidirectional("claude-*", "openrouter,anthropic/claude-3-opus"),
//! ];
//!
//! let (resolved, mapping) = resolve_model("claude-fast", &mappings);
//! assert_eq!(resolved, "deepseek,deepseek-chat");
//! ```

use glob_match::glob_match;
use serde::{Deserialize, Serialize};
use tracing::debug;

/// Model mapping configuration for alias resolution.
///
/// Maps client-requested model names to actual provider/model targets.
/// Supports glob patterns for flexible matching.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ModelMapping {
    /// Pattern to match against client-requested model (glob pattern).
    /// Supports `*` for any sequence of characters.
    /// Matching is case-insensitive.
    pub from: String,

    /// Target "provider,model" to route to.
    pub to: String,

    /// Whether to map response model name back to the alias.
    /// When true, responses will show the aliased name instead of the actual model.
    #[serde(default)]
    pub bidirectional: bool,
}

impl ModelMapping {
    /// Create a new model mapping.
    ///
    /// # Arguments
    /// * `from` - Pattern to match (supports glob wildcards)
    /// * `to` - Target provider,model string
    pub fn new(from: impl Into<String>, to: impl Into<String>) -> Self {
        Self {
            from: from.into(),
            to: to.into(),
            bidirectional: false,
        }
    }

    /// Create a new bidirectional model mapping.
    ///
    /// Bidirectional mappings will remap the model name in responses
    /// back to the original alias.
    pub fn with_bidirectional(from: impl Into<String>, to: impl Into<String>) -> Self {
        Self {
            from: from.into(),
            to: to.into(),
            bidirectional: true,
        }
    }

    /// Check if this mapping matches the given model name.
    ///
    /// Matching is case-insensitive and supports glob patterns.
    pub fn matches(&self, model: &str) -> bool {
        // Convert both to lowercase for case-insensitive matching
        let pattern = self.from.to_lowercase();
        let model_lower = model.to_lowercase();

        glob_match(&pattern, &model_lower)
    }

    /// Extract the model part from the target (after the comma).
    ///
    /// Returns the full target if no comma is present.
    pub fn target_model(&self) -> &str {
        self.to
            .split_once(',')
            .map(|(_, model)| model)
            .unwrap_or(&self.to)
    }

    /// Extract the provider part from the target (before the comma).
    ///
    /// Returns None if no comma is present.
    pub fn target_provider(&self) -> Option<&str> {
        self.to.split_once(',').map(|(provider, _)| provider)
    }
}

/// Resolve a model alias to the actual provider/model target.
///
/// Iterates through mappings in order and returns the first match.
/// If no mapping matches, returns the original model unchanged.
///
/// # Arguments
/// * `requested` - The model name requested by the client
/// * `mappings` - List of model mappings to check
///
/// # Returns
/// A tuple of (resolved_model, optional_matched_mapping)
/// - resolved_model: The target model or original if no match
/// - matched_mapping: The mapping that matched, if any
pub fn resolve_model<'a>(
    requested: &str,
    mappings: &'a [ModelMapping],
) -> (String, Option<&'a ModelMapping>) {
    for mapping in mappings {
        if mapping.matches(requested) {
            debug!(
                "Model alias resolved: '{}' -> '{}' (pattern: '{}')",
                requested, mapping.to, mapping.from
            );
            return (mapping.to.clone(), Some(mapping));
        }
    }

    debug!("No alias found for model '{}', using as-is", requested);
    (requested.to_string(), None)
}

/// Reverse resolve for bidirectional mappings.
///
/// Given an actual model name from a response, find if it should be
/// remapped back to an alias for the client.
///
/// Only considers mappings with `bidirectional: true`.
///
/// # Arguments
/// * `actual` - The actual model name from the provider response
/// * `mappings` - List of model mappings to check
///
/// # Returns
/// The aliased name if a bidirectional mapping matches, None otherwise
pub fn reverse_resolve<'a>(actual: &str, mappings: &'a [ModelMapping]) -> Option<&'a str> {
    for mapping in mappings {
        if mapping.bidirectional {
            // Check if the actual model matches the target
            let target_model = mapping.target_model();
            if actual.eq_ignore_ascii_case(target_model) {
                debug!(
                    "Reverse alias resolved: '{}' -> '{}' (bidirectional)",
                    actual, mapping.from
                );
                return Some(&mapping.from);
            }
        }
    }

    None
}

/// Check if a model name matches any exclusion pattern.
///
/// Used for model exclusion filtering.
///
/// # Arguments
/// * `model` - The model name to check
/// * `patterns` - List of glob patterns to match against
///
/// # Returns
/// true if the model matches any exclusion pattern
pub fn matches_exclusion(model: &str, patterns: &[String]) -> bool {
    let model_lower = model.to_lowercase();

    for pattern in patterns {
        let pattern_lower = pattern.to_lowercase();
        if glob_match(&pattern_lower, &model_lower) {
            debug!("Model '{}' matches exclusion pattern '{}'", model, pattern);
            return true;
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_model_mapping_exact_match() {
        let mapping = ModelMapping::new("claude-fast", "deepseek,deepseek-chat");

        assert!(mapping.matches("claude-fast"));
        assert!(!mapping.matches("claude-slow"));
        assert!(!mapping.matches("claude-fast-v2"));
    }

    #[test]
    fn test_model_mapping_glob_star() {
        let mapping = ModelMapping::new("claude-*", "openrouter,anthropic/claude-3-opus");

        assert!(mapping.matches("claude-fast"));
        assert!(mapping.matches("claude-3-opus"));
        assert!(mapping.matches("claude-anything"));
        assert!(!mapping.matches("gpt-4"));
        assert!(!mapping.matches("claudex")); // No hyphen
    }

    #[test]
    fn test_model_mapping_glob_suffix() {
        let mapping = ModelMapping::new("*-preview", "provider,model");

        assert!(mapping.matches("gpt-4-preview"));
        assert!(mapping.matches("claude-preview"));
        assert!(!mapping.matches("gpt-4"));
        assert!(!mapping.matches("preview"));
    }

    #[test]
    fn test_model_mapping_glob_middle() {
        let mapping = ModelMapping::new("claude-*-opus", "provider,model");

        assert!(mapping.matches("claude-3-opus"));
        assert!(mapping.matches("claude-v2-opus"));
        assert!(!mapping.matches("claude-opus"));
        assert!(!mapping.matches("claude-3-sonnet"));
    }

    #[test]
    fn test_model_mapping_case_insensitive() {
        let mapping = ModelMapping::new("CLAUDE-*", "provider,model");

        assert!(mapping.matches("claude-fast"));
        assert!(mapping.matches("CLAUDE-FAST"));
        assert!(mapping.matches("Claude-Fast"));
    }

    #[test]
    fn test_model_mapping_no_match() {
        let mappings = vec![
            ModelMapping::new("claude-*", "provider1,model1"),
            ModelMapping::new("gpt-*", "provider2,model2"),
        ];

        let (resolved, matched) = resolve_model("llama-2", &mappings);

        assert_eq!(resolved, "llama-2");
        assert!(matched.is_none());
    }

    #[test]
    fn test_resolve_first_match() {
        let mappings = vec![
            ModelMapping::new("claude-fast", "deepseek,deepseek-chat"),
            ModelMapping::new("claude-*", "openrouter,anthropic/claude-3-opus"),
        ];

        // First exact match should win
        let (resolved, matched) = resolve_model("claude-fast", &mappings);
        assert_eq!(resolved, "deepseek,deepseek-chat");
        assert!(matched.is_some());
        assert_eq!(matched.unwrap().from, "claude-fast");

        // Glob match for other claude models
        let (resolved, matched) = resolve_model("claude-opus", &mappings);
        assert_eq!(resolved, "openrouter,anthropic/claude-3-opus");
        assert!(matched.is_some());
        assert_eq!(matched.unwrap().from, "claude-*");
    }

    #[test]
    fn test_model_mapping_bidirectional() {
        let mapping = ModelMapping::with_bidirectional(
            "my-claude",
            "openrouter,anthropic/claude-3-opus-20240229",
        );

        assert!(mapping.bidirectional);
        assert!(mapping.matches("my-claude"));
    }

    #[test]
    fn test_reverse_resolve_bidirectional() {
        let mappings = vec![
            ModelMapping::new("claude-fast", "deepseek,deepseek-chat"),
            ModelMapping::with_bidirectional(
                "my-claude",
                "openrouter,anthropic/claude-3-opus-20240229",
            ),
        ];

        // Should find bidirectional mapping
        let result = reverse_resolve("anthropic/claude-3-opus-20240229", &mappings);
        assert_eq!(result, Some("my-claude"));

        // Should not find non-bidirectional mapping
        let result = reverse_resolve("deepseek-chat", &mappings);
        assert!(result.is_none());

        // Should not find unknown model
        let result = reverse_resolve("gpt-4", &mappings);
        assert!(result.is_none());
    }

    #[test]
    fn test_reverse_resolve_case_insensitive() {
        let mappings = vec![ModelMapping::with_bidirectional(
            "my-model",
            "provider,Target-Model",
        )];

        // Case insensitive matching for reverse resolve
        let result = reverse_resolve("target-model", &mappings);
        assert_eq!(result, Some("my-model"));

        let result = reverse_resolve("TARGET-MODEL", &mappings);
        assert_eq!(result, Some("my-model"));
    }

    #[test]
    fn test_target_model_extraction() {
        let mapping = ModelMapping::new("alias", "provider,model-name");

        assert_eq!(mapping.target_model(), "model-name");
        assert_eq!(mapping.target_provider(), Some("provider"));
    }

    #[test]
    fn test_target_model_no_comma() {
        let mapping = ModelMapping::new("alias", "just-model");

        assert_eq!(mapping.target_model(), "just-model");
        assert_eq!(mapping.target_provider(), None);
    }

    #[test]
    fn test_matches_exclusion() {
        let exclusions = vec![
            "gpt-4*".to_string(),
            "claude-*-preview".to_string(),
            "deprecated-model".to_string(),
        ];

        assert!(matches_exclusion("gpt-4", &exclusions));
        assert!(matches_exclusion("gpt-4-turbo", &exclusions));
        assert!(matches_exclusion("claude-3-preview", &exclusions));
        assert!(matches_exclusion("deprecated-model", &exclusions));

        assert!(!matches_exclusion("claude-3-opus", &exclusions));
        assert!(!matches_exclusion("llama-2", &exclusions));
    }

    #[test]
    fn test_matches_exclusion_case_insensitive() {
        let exclusions = vec!["GPT-*".to_string()];

        assert!(matches_exclusion("gpt-4", &exclusions));
        assert!(matches_exclusion("GPT-4", &exclusions));
        assert!(matches_exclusion("Gpt-4-Turbo", &exclusions));
    }

    #[test]
    fn test_matches_exclusion_empty() {
        let exclusions: Vec<String> = vec![];

        assert!(!matches_exclusion("any-model", &exclusions));
    }

    #[test]
    fn test_model_mapping_serialization() {
        let mapping = ModelMapping::with_bidirectional("claude-*", "provider,model");

        let json = serde_json::to_string(&mapping).unwrap();
        assert!(json.contains("\"from\":\"claude-*\""));
        assert!(json.contains("\"bidirectional\":true"));

        let deserialized: ModelMapping = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, mapping);
    }

    #[test]
    fn test_model_mapping_deserialization_default_bidirectional() {
        let json = r#"{"from": "alias", "to": "provider,model"}"#;

        let mapping: ModelMapping = serde_json::from_str(json).unwrap();
        assert!(!mapping.bidirectional);
    }
}
