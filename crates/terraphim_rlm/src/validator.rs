//! Knowledge Graph Validator for RLM command validation.
//!
//! This module provides validation of commands and text against a knowledge graph
//! using term matching and path connectivity analysis. It supports configurable
//! strictness levels and retry logic with LLM rephrasing.
//!
//! ## Architecture
//!
//! ```text
//! KnowledgeGraphValidator
//!     ├── TermMatcher (Aho-Corasick via terraphim_automata)
//!     ├── PathValidator (DFS connectivity via terraphim_rolegraph)
//!     └── RetryPolicy (LLM rephrasing on failure)
//! ```
//!
//! ## Strictness Levels
//!
//! - **Permissive**: Warns but allows execution (log only)
//! - **Normal**: Requires at least one known term (default)
//! - **Strict**: Requires all terms to be connected by a graph path

use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use terraphim_rolegraph::RoleGraph;
use terraphim_types::Thesaurus;

use crate::config::KgStrictness;
use crate::error::RlmError;
use crate::types::SessionId;

/// Result type for this module.
pub type RlmResult<T> = Result<T, RlmError>;

/// Configuration for the knowledge graph validator.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidatorConfig {
    /// Strictness level for validation.
    pub strictness: KgStrictness,
    /// Maximum retries before escalation.
    pub max_retries: u32,
    /// Minimum match ratio for normal mode (0.0 to 1.0).
    pub min_match_ratio: f32,
    /// Whether to require path connectivity.
    pub require_connectivity: bool,
}

impl Default for ValidatorConfig {
    fn default() -> Self {
        Self {
            strictness: KgStrictness::Normal,
            max_retries: 3,
            min_match_ratio: 0.1, // At least 10% of words should match
            require_connectivity: false,
        }
    }
}

impl ValidatorConfig {
    /// Create a permissive configuration (warn only).
    pub fn permissive() -> Self {
        Self {
            strictness: KgStrictness::Permissive,
            max_retries: 0,
            min_match_ratio: 0.0,
            require_connectivity: false,
        }
    }

    /// Create a strict configuration (require connectivity).
    pub fn strict() -> Self {
        Self {
            strictness: KgStrictness::Strict,
            max_retries: 3,
            min_match_ratio: 0.3,
            require_connectivity: true,
        }
    }
}

/// Result of command validation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    /// Whether the command passed validation.
    pub passed: bool,
    /// Matched terms from the knowledge graph.
    pub matched_terms: Vec<String>,
    /// Unmatched words from the command.
    pub unmatched_words: Vec<String>,
    /// Whether matched terms are connected by a graph path.
    pub terms_connected: bool,
    /// Match ratio (matched terms / total words).
    pub match_ratio: f32,
    /// Validation message explaining the result.
    pub message: String,
    /// Suggested rephrasings (if validation failed).
    pub suggestions: Vec<String>,
    /// Number of retries attempted.
    pub retry_count: u32,
    /// Whether escalation to user is required.
    pub escalation_required: bool,
}

impl ValidationResult {
    /// Create a passed validation result.
    pub fn passed(
        matched_terms: Vec<String>,
        unmatched_words: Vec<String>,
        terms_connected: bool,
        match_ratio: f32,
    ) -> Self {
        Self {
            passed: true,
            matched_terms,
            unmatched_words,
            terms_connected,
            match_ratio,
            message: "Validation passed".to_string(),
            suggestions: Vec::new(),
            retry_count: 0,
            escalation_required: false,
        }
    }

    /// Create a failed validation result.
    pub fn failed(
        matched_terms: Vec<String>,
        unmatched_words: Vec<String>,
        terms_connected: bool,
        match_ratio: f32,
        message: String,
    ) -> Self {
        Self {
            passed: false,
            matched_terms,
            unmatched_words,
            terms_connected,
            match_ratio,
            message,
            suggestions: Vec::new(),
            retry_count: 0,
            escalation_required: false,
        }
    }

    /// Mark as requiring escalation to user.
    pub fn with_escalation(mut self) -> Self {
        self.escalation_required = true;
        self
    }

    /// Add suggestions for rephrasing.
    pub fn with_suggestions(mut self, suggestions: Vec<String>) -> Self {
        self.suggestions = suggestions;
        self
    }

    /// Set retry count.
    pub fn with_retry_count(mut self, count: u32) -> Self {
        self.retry_count = count;
        self
    }
}

/// Validation context for tracking retry state.
#[derive(Debug, Clone)]
pub struct ValidationContext {
    /// Session ID for this validation.
    pub session_id: SessionId,
    /// Number of retries attempted.
    pub retry_count: u32,
    /// Previous validation results.
    pub history: Vec<ValidationResult>,
}

impl ValidationContext {
    /// Create a new validation context.
    pub fn new(session_id: SessionId) -> Self {
        Self {
            session_id,
            retry_count: 0,
            history: Vec::new(),
        }
    }

    /// Increment retry count and record result.
    pub fn record_attempt(&mut self, result: ValidationResult) {
        self.retry_count += 1;
        self.history.push(result);
    }

    /// Check if max retries exceeded.
    pub fn max_retries_exceeded(&self, max_retries: u32) -> bool {
        self.retry_count >= max_retries
    }
}

/// Knowledge graph validator for command validation.
pub struct KnowledgeGraphValidator {
    config: ValidatorConfig,
    thesaurus: Option<Thesaurus>,
    role_graph: Option<RoleGraph>,
}

impl KnowledgeGraphValidator {
    /// Create a new validator with the given configuration.
    pub fn new(config: ValidatorConfig) -> Self {
        Self {
            config,
            thesaurus: None,
            role_graph: None,
        }
    }

    /// Create a disabled validator that always passes.
    pub fn disabled() -> Self {
        Self {
            config: ValidatorConfig::permissive(),
            thesaurus: None,
            role_graph: None,
        }
    }

    /// Set the thesaurus for term matching.
    pub fn with_thesaurus(mut self, thesaurus: Thesaurus) -> Self {
        self.thesaurus = Some(thesaurus);
        self
    }

    /// Set the role graph for path connectivity.
    pub fn with_role_graph(mut self, role_graph: RoleGraph) -> Self {
        self.role_graph = Some(role_graph);
        self
    }

    /// Validate a command string.
    ///
    /// Returns a validation result indicating whether the command passes
    /// the configured validation rules.
    pub fn validate(&self, command: &str) -> RlmResult<ValidationResult> {
        // Skip validation in permissive mode with no thesaurus
        if self.config.strictness == KgStrictness::Permissive && self.thesaurus.is_none() {
            return Ok(ValidationResult::passed(vec![], vec![], true, 0.0));
        }

        // Extract words from command
        let words = extract_words(command);
        if words.is_empty() {
            return Ok(ValidationResult::passed(vec![], vec![], true, 0.0));
        }

        // Find matched terms
        let (matched_terms, unmatched_words) = self.find_matches(command, &words)?;

        // Calculate match ratio
        let match_ratio = if words.is_empty() {
            0.0
        } else {
            matched_terms.len() as f32 / words.len() as f32
        };

        // Check path connectivity
        let terms_connected = self.check_connectivity(command);

        // Apply validation rules based on strictness
        match self.config.strictness {
            KgStrictness::Permissive => {
                // Always pass, but log if there are issues
                if matched_terms.is_empty() {
                    log::warn!(
                        "KG validation (permissive): No matched terms in command: {}",
                        truncate_for_log(command)
                    );
                }
                Ok(ValidationResult::passed(
                    matched_terms,
                    unmatched_words,
                    terms_connected,
                    match_ratio,
                ))
            }
            KgStrictness::Normal => {
                // Require at least one matched term or minimum ratio
                if matched_terms.is_empty() && self.thesaurus.is_some() {
                    let msg = format!(
                        "No known terms found. Please use domain-specific terminology. Unrecognized: {:?}",
                        &unmatched_words[..unmatched_words.len().min(5)]
                    );
                    Ok(ValidationResult::failed(
                        matched_terms,
                        unmatched_words,
                        terms_connected,
                        match_ratio,
                        msg,
                    ))
                } else if match_ratio < self.config.min_match_ratio && self.thesaurus.is_some() {
                    let msg = format!(
                        "Match ratio {:.1}% below threshold {:.1}%. Consider using more specific terms.",
                        match_ratio * 100.0,
                        self.config.min_match_ratio * 100.0
                    );
                    Ok(ValidationResult::failed(
                        matched_terms,
                        unmatched_words,
                        terms_connected,
                        match_ratio,
                        msg,
                    ))
                } else {
                    Ok(ValidationResult::passed(
                        matched_terms,
                        unmatched_words,
                        terms_connected,
                        match_ratio,
                    ))
                }
            }
            KgStrictness::Strict => {
                // Require connectivity between all matched terms
                if matched_terms.is_empty() && self.thesaurus.is_some() {
                    let msg = "No known terms found. Strict mode requires domain terminology."
                        .to_string();
                    Ok(ValidationResult::failed(
                        matched_terms,
                        unmatched_words,
                        terms_connected,
                        match_ratio,
                        msg,
                    ))
                } else if self.config.require_connectivity
                    && !terms_connected
                    && matched_terms.len() > 1
                {
                    let msg = format!(
                        "Terms {:?} are not connected in the knowledge graph. Please rephrase for semantic coherence.",
                        &matched_terms[..matched_terms.len().min(5)]
                    );
                    Ok(ValidationResult::failed(
                        matched_terms,
                        unmatched_words,
                        terms_connected,
                        match_ratio,
                        msg,
                    ))
                } else {
                    Ok(ValidationResult::passed(
                        matched_terms,
                        unmatched_words,
                        terms_connected,
                        match_ratio,
                    ))
                }
            }
        }
    }

    /// Validate with retry context.
    ///
    /// Tracks retry attempts and escalates to user after max retries.
    pub fn validate_with_context(
        &self,
        command: &str,
        context: &mut ValidationContext,
    ) -> RlmResult<ValidationResult> {
        let mut result = self.validate(command)?;

        if !result.passed {
            // Check if we need to escalate
            if context.max_retries_exceeded(self.config.max_retries) {
                result = result.with_escalation();
                result.message = format!(
                    "Validation failed after {} attempts. User intervention required: {}",
                    context.retry_count, result.message
                );
            } else {
                // Generate suggestions for rephrasing
                let suggestions = self.generate_suggestions(command, &result);
                result = result.with_suggestions(suggestions);
            }

            result = result.with_retry_count(context.retry_count);
            context.record_attempt(result.clone());
        }

        Ok(result)
    }

    /// Find matched and unmatched terms in the command.
    fn find_matches(
        &self,
        command: &str,
        words: &[String],
    ) -> RlmResult<(Vec<String>, Vec<String>)> {
        let Some(ref thesaurus) = self.thesaurus else {
            // No thesaurus, return empty matches
            return Ok((vec![], words.to_vec()));
        };

        // Use terraphim_automata for term matching
        let matches =
            terraphim_automata::find_matches(command, thesaurus.clone(), true).map_err(|e| {
                RlmError::ConfigError {
                    message: format!("Term matching failed: {}", e),
                }
            })?;

        let matched_terms: Vec<String> = matches.iter().map(|m| m.term.clone()).collect();
        let matched_set: HashSet<_> = matched_terms.iter().map(|s| s.to_lowercase()).collect();

        // Find unmatched words
        let unmatched_words: Vec<String> = words
            .iter()
            .filter(|w| !matched_set.contains(&w.to_lowercase()))
            .cloned()
            .collect();

        Ok((matched_terms, unmatched_words))
    }

    /// Check if all matched terms are connected by a graph path.
    fn check_connectivity(&self, text: &str) -> bool {
        if let Some(ref role_graph) = self.role_graph {
            role_graph.is_all_terms_connected_by_path(text)
        } else {
            // No role graph, assume connected
            true
        }
    }

    /// Generate suggestions for rephrasing a failed command.
    fn generate_suggestions(&self, command: &str, result: &ValidationResult) -> Vec<String> {
        let mut suggestions = Vec::new();

        // Suggest using matched terms more prominently
        if !result.matched_terms.is_empty() {
            suggestions.push(format!(
                "Try rephrasing using these known terms: {}",
                result.matched_terms.join(", ")
            ));
        }

        // Suggest being more specific
        if result.unmatched_words.len() > 3 {
            suggestions.push(
                "Consider using more domain-specific terminology instead of generic terms."
                    .to_string(),
            );
        }

        // Suggest breaking down the command
        if command.len() > 100 {
            suggestions
                .push("Consider breaking this into smaller, more focused commands.".to_string());
        }

        suggestions
    }

    /// Get the current configuration.
    pub fn config(&self) -> &ValidatorConfig {
        &self.config
    }

    /// Check if the validator has a thesaurus.
    pub fn has_thesaurus(&self) -> bool {
        self.thesaurus.is_some()
    }

    /// Check if the validator has a role graph.
    pub fn has_role_graph(&self) -> bool {
        self.role_graph.is_some()
    }
}

/// Extract words from a command string.
fn extract_words(text: &str) -> Vec<String> {
    // Split on any non-word character (not alphanumeric, underscore, or hyphen)
    text.split(|c: char| !c.is_alphanumeric() && c != '_' && c != '-')
        .map(|s| s.to_string())
        .filter(|s| !s.is_empty() && s.len() > 2) // Skip very short words
        .collect()
}

/// Truncate a string for logging (max 100 chars).
fn truncate_for_log(s: &str) -> String {
    if s.len() > 100 {
        format!("{}...", &s[..97])
    } else {
        s.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validator_config_default() {
        let config = ValidatorConfig::default();
        assert_eq!(config.strictness, KgStrictness::Normal);
        assert_eq!(config.max_retries, 3);
    }

    #[test]
    fn test_validator_config_permissive() {
        let config = ValidatorConfig::permissive();
        assert_eq!(config.strictness, KgStrictness::Permissive);
        assert_eq!(config.max_retries, 0);
    }

    #[test]
    fn test_validator_config_strict() {
        let config = ValidatorConfig::strict();
        assert_eq!(config.strictness, KgStrictness::Strict);
        assert!(config.require_connectivity);
    }

    #[test]
    fn test_validation_result_passed() {
        let result = ValidationResult::passed(
            vec!["term1".to_string()],
            vec!["unknown".to_string()],
            true,
            0.5,
        );
        assert!(result.passed);
        assert!(!result.escalation_required);
    }

    #[test]
    fn test_validation_result_failed() {
        let result = ValidationResult::failed(
            vec![],
            vec!["unknown".to_string()],
            false,
            0.0,
            "No matches".to_string(),
        );
        assert!(!result.passed);
    }

    #[test]
    fn test_validation_result_with_escalation() {
        let result = ValidationResult::failed(
            vec![],
            vec!["unknown".to_string()],
            false,
            0.0,
            "Failed".to_string(),
        )
        .with_escalation();
        assert!(result.escalation_required);
    }

    #[test]
    fn test_validation_context() {
        let session_id = SessionId::new();
        let mut context = ValidationContext::new(session_id);

        assert_eq!(context.retry_count, 0);
        assert!(!context.max_retries_exceeded(3));

        context.record_attempt(ValidationResult::failed(
            vec![],
            vec![],
            false,
            0.0,
            "Failed".to_string(),
        ));
        assert_eq!(context.retry_count, 1);
        assert_eq!(context.history.len(), 1);
    }

    #[test]
    fn test_disabled_validator() {
        let validator = KnowledgeGraphValidator::disabled();
        let result = validator.validate("any command here").unwrap();
        assert!(result.passed);
    }

    #[test]
    fn test_validator_empty_command() {
        let validator = KnowledgeGraphValidator::new(ValidatorConfig::default());
        let result = validator.validate("").unwrap();
        assert!(result.passed);
    }

    #[test]
    fn test_validator_no_thesaurus_permissive() {
        let validator = KnowledgeGraphValidator::new(ValidatorConfig::permissive());
        let result = validator.validate("print hello world").unwrap();
        assert!(result.passed);
    }

    #[test]
    fn test_validator_no_thesaurus_normal() {
        let validator = KnowledgeGraphValidator::new(ValidatorConfig::default());
        // Without a thesaurus, normal mode should pass (no thesaurus = no validation)
        let result = validator.validate("print hello world").unwrap();
        assert!(result.passed);
    }

    #[test]
    fn test_extract_words() {
        let words = extract_words("print('hello, world!')");
        assert!(words.contains(&"print".to_string()));
        assert!(words.contains(&"hello".to_string()));
        assert!(words.contains(&"world".to_string()));
    }

    #[test]
    fn test_extract_words_filters_short() {
        let words = extract_words("a b cd this_is_longer");
        // Should filter out "a", "b", "cd" (2 chars or less)
        assert!(!words.contains(&"a".to_string()));
        assert!(!words.contains(&"b".to_string()));
        assert!(!words.contains(&"cd".to_string()));
        assert!(words.contains(&"this_is_longer".to_string()));
    }

    #[test]
    fn test_truncate_for_log() {
        let short = "short string";
        assert_eq!(truncate_for_log(short), short);

        let long = "a".repeat(150);
        let truncated = truncate_for_log(&long);
        assert!(truncated.len() < 150);
        assert!(truncated.ends_with("..."));
    }

    #[test]
    fn test_validation_context_max_retries() {
        let session_id = SessionId::new();
        let mut context = ValidationContext::new(session_id);

        // Add 3 retry attempts
        for _ in 0..3 {
            context.record_attempt(ValidationResult::failed(
                vec![],
                vec![],
                false,
                0.0,
                "Failed".to_string(),
            ));
        }

        assert!(context.max_retries_exceeded(3));
        assert!(!context.max_retries_exceeded(4));
    }

    #[test]
    fn test_generate_suggestions() {
        let validator = KnowledgeGraphValidator::new(ValidatorConfig::default());
        let result = ValidationResult::failed(
            vec!["term1".to_string(), "term2".to_string()],
            vec![
                "unknown1".to_string(),
                "unknown2".to_string(),
                "unknown3".to_string(),
                "unknown4".to_string(),
            ],
            false,
            0.3,
            "Failed".to_string(),
        );

        let suggestions =
            validator.generate_suggestions("some long command that needs rephrasing", &result);
        assert!(!suggestions.is_empty());
        // Should suggest using known terms
        assert!(
            suggestions
                .iter()
                .any(|s| s.contains("known terms") || s.contains("term1"))
        );
    }
}
