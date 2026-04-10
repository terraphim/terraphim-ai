//! Unified replacement service for hooks.

use serde::{Deserialize, Serialize};
use terraphim_automata::LinkType as AutomataLinkType;
use terraphim_types::Thesaurus;
use thiserror::Error;

/// Re-export LinkType for convenience.
pub use terraphim_automata::LinkType;

/// Errors that can occur during replacement.
#[derive(Error, Debug)]
pub enum ReplacementError {
    #[error("Automata error: {0}")]
    Automata(#[from] terraphim_automata::TerraphimAutomataError),
    #[error("UTF-8 conversion error: {0}")]
    Utf8(#[from] std::string::FromUtf8Error),
}

/// Result of a replacement operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookResult {
    /// The resulting text after replacement.
    pub result: String,
    /// The original input text.
    pub original: String,
    /// Number of replacements made.
    pub replacements: usize,
    /// Whether any changes were made.
    pub changed: bool,
    /// Error message if replacement failed (only set in fail-open mode).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl HookResult {
    /// Create a successful result.
    pub fn success(original: String, result: String) -> Self {
        let changed = original != result;
        let replacements = if changed { 1 } else { 0 };
        Self {
            result,
            original,
            replacements,
            changed,
            error: None,
        }
    }

    /// Create a pass-through result (no changes).
    pub fn pass_through(original: String) -> Self {
        Self {
            result: original.clone(),
            original,
            replacements: 0,
            changed: false,
            error: None,
        }
    }

    /// Create a fail-open result with error message.
    pub fn fail_open(original: String, error: String) -> Self {
        Self {
            result: original.clone(),
            original,
            replacements: 0,
            changed: false,
            error: Some(error),
        }
    }
}

/// Unified replacement service using Terraphim knowledge graphs.
#[derive(Debug, Clone)]
pub struct ReplacementService {
    thesaurus: Thesaurus,
    link_type: AutomataLinkType,
}

impl ReplacementService {
    /// Create a new replacement service with a thesaurus.
    pub fn new(thesaurus: Thesaurus) -> Self {
        Self {
            thesaurus,
            link_type: AutomataLinkType::PlainText,
        }
    }

    /// Set the link type for replacements.
    pub fn with_link_type(mut self, link_type: AutomataLinkType) -> Self {
        self.link_type = link_type;
        self
    }

    /// Perform replacement on text.
    pub fn replace(&self, text: &str) -> Result<HookResult, ReplacementError> {
        let result_bytes =
            terraphim_automata::replace_matches(text, self.thesaurus.clone(), self.link_type)?;
        let result = String::from_utf8(result_bytes)?;
        Ok(HookResult::success(text.to_string(), result))
    }

    /// Perform replacement with fail-open semantics.
    ///
    /// If replacement fails, returns the original text unchanged with error in result.
    pub fn replace_fail_open(&self, text: &str) -> HookResult {
        match self.replace(text) {
            Ok(result) => result,
            Err(e) => HookResult::fail_open(text.to_string(), e.to_string()),
        }
    }

    /// Find matches in text without replacing.
    pub fn find_matches(
        &self,
        text: &str,
    ) -> Result<Vec<terraphim_automata::Matched>, ReplacementError> {
        Ok(terraphim_automata::find_matches(
            text,
            self.thesaurus.clone(),
            true,
        )?)
    }

    /// Check if text contains any terms from the thesaurus.
    pub fn contains_matches(&self, text: &str) -> bool {
        self.find_matches(text)
            .map(|matches| !matches.is_empty())
            .unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use terraphim_types::{NormalizedTerm, NormalizedTermValue};

    fn create_test_thesaurus() -> Thesaurus {
        let mut thesaurus = Thesaurus::new("test".to_string());

        // Add npm -> bun mapping
        let bun_term = NormalizedTerm::new(1u64, NormalizedTermValue::from("bun"));
        thesaurus.insert(NormalizedTermValue::from("npm"), bun_term.clone());
        thesaurus.insert(NormalizedTermValue::from("yarn"), bun_term.clone());
        thesaurus.insert(NormalizedTermValue::from("pnpm"), bun_term);

        thesaurus
    }

    #[test]
    fn test_replacement_service_basic() {
        let thesaurus = create_test_thesaurus();
        let service = ReplacementService::new(thesaurus);

        let result = service.replace("npm install").unwrap();
        assert!(result.changed);
        assert_eq!(result.result, "bun install");
    }

    #[test]
    fn test_replacement_service_no_match() {
        let thesaurus = create_test_thesaurus();
        let service = ReplacementService::new(thesaurus);

        let result = service.replace("cargo build").unwrap();
        assert!(!result.changed);
        assert_eq!(result.result, "cargo build");
    }

    #[test]
    fn test_hook_result_success() {
        let result = HookResult::success("npm".to_string(), "bun".to_string());
        assert!(result.changed);
        assert_eq!(result.replacements, 1);
        assert!(result.error.is_none());
    }

    #[test]
    fn test_hook_result_pass_through() {
        let result = HookResult::pass_through("unchanged".to_string());
        assert!(!result.changed);
        assert_eq!(result.replacements, 0);
        assert_eq!(result.result, result.original);
    }

    #[test]
    fn test_hook_result_fail_open() {
        let result = HookResult::fail_open("original".to_string(), "error msg".to_string());
        assert!(!result.changed);
        assert_eq!(result.result, "original");
        assert_eq!(result.error, Some("error msg".to_string()));
    }

    #[test]
    fn test_replacement_multiple_terms_in_one_text() {
        let thesaurus = create_test_thesaurus();
        let service = ReplacementService::new(thesaurus);

        // Both npm and yarn should be replaced by bun
        let result = service.replace("npm install && yarn add foo").unwrap();
        assert!(result.changed);
        assert_eq!(result.result, "bun install && bun add foo");
    }

    #[test]
    fn test_replacement_service_pnpm() {
        let thesaurus = create_test_thesaurus();
        let service = ReplacementService::new(thesaurus);

        let result = service.replace("pnpm install express").unwrap();
        assert!(result.changed);
        assert_eq!(result.result, "bun install express");
    }

    #[test]
    fn test_find_matches_returns_matched_terms() {
        let thesaurus = create_test_thesaurus();
        let service = ReplacementService::new(thesaurus);

        let matches = service.find_matches("npm install && yarn add").unwrap();
        assert!(!matches.is_empty());
        let match_terms: Vec<&str> = matches.iter().map(|m| m.term.as_str()).collect();
        assert!(match_terms.contains(&"npm"));
        assert!(match_terms.contains(&"yarn"));
    }

    #[test]
    fn test_contains_matches_true() {
        let thesaurus = create_test_thesaurus();
        let service = ReplacementService::new(thesaurus);
        assert!(service.contains_matches("npm install"));
    }

    #[test]
    fn test_contains_matches_false() {
        let thesaurus = create_test_thesaurus();
        let service = ReplacementService::new(thesaurus);
        assert!(!service.contains_matches("cargo build"));
    }

    #[test]
    fn test_replace_fail_open_on_valid_input() {
        let thesaurus = create_test_thesaurus();
        let service = ReplacementService::new(thesaurus);

        let result = service.replace_fail_open("npm install");
        assert!(result.changed);
        assert_eq!(result.result, "bun install");
        assert!(result.error.is_none());
    }

    #[test]
    fn test_hook_result_success_when_unchanged() {
        let result = HookResult::success("same".to_string(), "same".to_string());
        assert!(!result.changed);
        assert_eq!(result.replacements, 0);
    }

    #[test]
    fn test_hook_result_serde_round_trip() {
        let result = HookResult::success("npm".to_string(), "bun".to_string());
        let json = serde_json::to_string(&result).unwrap();
        let deserialized: HookResult = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.result, "bun");
        assert_eq!(deserialized.original, "npm");
        assert!(deserialized.changed);
    }

    #[test]
    fn test_hook_result_fail_open_serialization_skips_none_error() {
        let result = HookResult::pass_through("text".to_string());
        let json = serde_json::to_string(&result).unwrap();
        assert!(!json.contains("error"));
    }

    #[test]
    fn test_replacement_with_unicode_text() {
        let thesaurus = create_test_thesaurus();
        let service = ReplacementService::new(thesaurus);

        let result = service.replace("npm install -- emoji").unwrap();
        assert!(result.changed);
        assert!(result.result.starts_with("bun"));
    }

    #[test]
    fn test_replacement_preserves_surrounding_text() {
        let thesaurus = create_test_thesaurus();
        let service = ReplacementService::new(thesaurus);

        let result = service.replace("before npm after").unwrap();
        assert!(result.changed);
        assert_eq!(result.result, "before bun after");
    }

    #[test]
    fn test_with_link_type_builder_pattern() {
        let thesaurus = create_test_thesaurus();
        let service = ReplacementService::new(thesaurus).with_link_type(LinkType::PlainText);
        // Just verify it compiles and doesn't panic
        let result = service.replace("npm install").unwrap();
        assert!(result.changed);
    }
}
