//! Knowledge graph based command validation service.
//!
//! This module provides validation capabilities using Aho-Corasick automaton
//! matching against knowledge graph patterns.

use std::time::Instant;

use crate::replacement::ReplacementService;
use crate::validation_types::{ValidationError, ValidationResult};
use terraphim_types::Thesaurus;

/// Command validation service.
///
/// Validates commands and operations against knowledge graph patterns
/// using high performance Aho-Corasick automaton matching.
#[derive(Debug, Clone)]
pub struct ValidationService {
    replacement_service: ReplacementService,
}

impl ValidationService {
    /// Create a new validation service with the provided command pattern thesaurus.
    pub fn new(command_thesaurus: Thesaurus) -> Self {
        Self {
            replacement_service: ReplacementService::new(command_thesaurus),
        }
    }

    /// Validate a command string.
    ///
    /// Returns a validation result indicating if the command should be allowed,
    /// warned, blocked, or modified.
    pub fn validate(&self, command: &str) -> ValidationResult {
        let start = Instant::now();

        let matches = match self.replacement_service.find_matches(command) {
            Ok(matches) => matches,
            Err(e) => {
                let duration = start.elapsed().as_millis() as u64;
                return ValidationResult::fail_open(
                    command.to_string(),
                    ValidationError {
                        message: e.to_string(),
                        code: None,
                    },
                    duration,
                );
            }
        };

        let duration = start.elapsed().as_millis() as u64;

        if matches.is_empty() {
            return ValidationResult::allow(command.to_string(), duration);
        }

        // For v1 implementation, first match wins.
        // More complex rule evaluation will be added in future versions.
        let first_match = &matches[0];
        let normalized = first_match.normalized_term.value.as_str();

        match normalized {
            "allow" => ValidationResult::allow(command.to_string(), duration),
            "warn" => ValidationResult::warn(
                command.to_string(),
                format!("Matched warning pattern: {}", first_match.term),
                duration,
            ),
            "block" => ValidationResult::block(
                command.to_string(),
                format!("Blocked by pattern: {}", first_match.term),
                duration,
            ),
            _ => ValidationResult::modify(
                command.to_string(),
                command.replace(&first_match.term, normalized),
                duration,
            ),
        }
    }

    /// Validate a command with fail-open semantics.
    ///
    /// If validation fails internally, always returns Allow outcome
    /// with error information attached.
    pub fn validate_fail_open(&self, command: &str) -> ValidationResult {
        self.validate(command)
    }

    /// Check if a command contains any validation patterns.
    ///
    /// Returns true if any pattern matches, false otherwise.
    pub fn has_matches(&self, command: &str) -> bool {
        self.replacement_service.contains_matches(command)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use terraphim_types::{NormalizedTerm, NormalizedTermValue};

    fn create_test_thesaurus() -> Thesaurus {
        let mut thesaurus = Thesaurus::new("command_validation".to_string());

        // Allow patterns
        let allow_term = NormalizedTerm::new(1u64, NormalizedTermValue::from("allow"));
        thesaurus.insert(NormalizedTermValue::from("cargo build"), allow_term.clone());
        thesaurus.insert(NormalizedTermValue::from("cargo test"), allow_term.clone());

        // Warning patterns
        let warn_term = NormalizedTerm::new(2u64, NormalizedTermValue::from("warn"));
        thesaurus.insert(NormalizedTermValue::from("sudo"), warn_term.clone());
        thesaurus.insert(NormalizedTermValue::from("curl"), warn_term.clone());

        // Block patterns
        let block_term = NormalizedTerm::new(3u64, NormalizedTermValue::from("block"));
        thesaurus.insert(NormalizedTermValue::from("rm -rf /"), block_term.clone());
        thesaurus.insert(
            NormalizedTermValue::from("dd if=/dev/zero"),
            block_term.clone(),
        );

        // Modify patterns
        let bun_term = NormalizedTerm::new(4u64, NormalizedTermValue::from("bun"));
        thesaurus.insert(NormalizedTermValue::from("npm"), bun_term.clone());
        thesaurus.insert(NormalizedTermValue::from("yarn"), bun_term.clone());

        thesaurus
    }

    #[test]
    fn test_validate_allow() {
        let service = ValidationService::new(create_test_thesaurus());
        let result = service.validate("cargo build --release");

        assert!(result.is_allowed());
        assert!(!result.should_block());
        assert!(!result.is_modified());
        assert!(result.error.is_none());
    }

    #[test]
    fn test_validate_warn() {
        let service = ValidationService::new(create_test_thesaurus());
        let result = service.validate("sudo systemctl restart nginx");

        assert!(result.is_allowed());
        assert!(!result.should_block());
        assert!(!result.is_modified());

        if let crate::ValidationOutcome::Warn(msg) = result.outcome {
            assert!(msg.contains("sudo"));
        } else {
            panic!("Expected Warn outcome");
        }
    }

    #[test]
    fn test_validate_block() {
        let service = ValidationService::new(create_test_thesaurus());
        let result = service.validate("rm -rf /var/log");

        assert!(!result.is_allowed());
        assert!(result.should_block());
        assert!(!result.is_modified());

        if let crate::ValidationOutcome::Block(reason) = result.outcome {
            assert!(reason.contains("rm -rf /"));
        } else {
            panic!("Expected Block outcome");
        }
    }

    #[test]
    fn test_validate_modify() {
        let service = ValidationService::new(create_test_thesaurus());
        let result = service.validate("npm install express");

        assert!(result.is_allowed());
        assert!(!result.should_block());
        assert!(result.is_modified());
        assert_eq!(result.final_text(), "bun install express");

        if let crate::ValidationOutcome::Modify(modified) = result.outcome {
            assert_eq!(modified, "bun install express");
        } else {
            panic!("Expected Modify outcome");
        }
    }

    #[test]
    fn test_validate_no_matches() {
        let service = ValidationService::new(create_test_thesaurus());
        let result = service.validate("ls -la");

        assert!(result.is_allowed());
        assert!(!result.should_block());
        assert!(!result.is_modified());
        assert!(result.error.is_none());
    }

    #[test]
    fn test_has_matches() {
        let service = ValidationService::new(create_test_thesaurus());

        assert!(service.has_matches("cargo build"));
        assert!(service.has_matches("sudo"));
        assert!(service.has_matches("rm -rf /"));
        assert!(service.has_matches("npm install"));
        assert!(!service.has_matches("ls -la"));
    }

    #[test]
    fn test_validate_fail_open() {
        let service = ValidationService::new(create_test_thesaurus());
        let result = service.validate_fail_open("rm -rf /");

        // Should still block even with fail-open
        assert!(result.should_block());
    }

    #[test]
    fn test_validate_latency() {
        let service = ValidationService::new(create_test_thesaurus());

        // Run 1000 iterations to measure performance
        let start = std::time::Instant::now();
        for _ in 0..1000 {
            let _ = service.validate("cargo build --release --all-targets");
        }
        let duration = start.elapsed();

        // Average should be well under 1ms
        let avg_ns = duration.as_nanos() / 1000;
        assert!(
            avg_ns < 1000000,
            "Average validation time {}ns > 1ms",
            avg_ns
        );
    }
}
