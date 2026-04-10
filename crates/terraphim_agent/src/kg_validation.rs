//! KG-based command validation for the PreToolUse hook pipeline.
//!
//! Uses Aho-Corasick matching against the knowledge graph thesaurus to detect
//! commands that have known alternatives (e.g., `npm install` -> `bun install`).
//! Returns validation results with matched patterns and suggested replacements.
//!
//! This module is designed to be fail-open: if the KG thesaurus cannot be loaded
//! or matching fails, an empty result is returned without blocking execution.

use serde::{Deserialize, Serialize};
use std::sync::OnceLock;
use terraphim_types::Thesaurus;

use crate::learnings::{build_kg_thesaurus_from_dir, find_kg_dir};

/// Global cache for the KG thesaurus used by command validation.
/// Built once from `docs/src/kg/*.md` files and reused across invocations.
static VALIDATION_KG_THESAURUS: OnceLock<Option<Thesaurus>> = OnceLock::new();

/// A single validation finding from KG pattern matching.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ValidationFinding {
    /// The term that was matched in the command (e.g., "npm install")
    pub matched_term: String,
    /// The suggested replacement (the normalized/canonical term, e.g., "bun install")
    pub suggested_replacement: String,
    /// Position in the input where the match was found (start, end)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub position: Option<(usize, usize)>,
}

/// Result of KG-based command validation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct KgValidationResult {
    /// Whether any KG patterns matched the command
    pub has_findings: bool,
    /// Individual findings with matched terms and suggestions
    pub findings: Vec<ValidationFinding>,
}

impl KgValidationResult {
    /// Create an empty result (no findings).
    pub fn empty() -> Self {
        Self {
            has_findings: false,
            findings: Vec::new(),
        }
    }
}

/// Validate a command against the KG thesaurus.
///
/// Loads the KG thesaurus from `docs/src/kg/*.md` files (cached after first call),
/// then uses Aho-Corasick matching to find terms in the command that have
/// known canonical replacements.
///
/// This function is fail-open: any errors during thesaurus loading or matching
/// result in an empty `KgValidationResult` rather than an error.
pub fn validate_command_against_kg(command: &str) -> KgValidationResult {
    let thesaurus_opt = VALIDATION_KG_THESAURUS.get_or_init(|| {
        let kg_dir = find_kg_dir()?;
        build_kg_thesaurus_from_dir(&kg_dir)
    });

    let thesaurus = match thesaurus_opt {
        Some(t) => t.clone(),
        None => return KgValidationResult::empty(),
    };

    validate_command_with_thesaurus(command, thesaurus)
}

/// Validate a command against a provided thesaurus (useful for testing).
///
/// This function is the core matching logic, separated from the global cache
/// so it can be tested with custom thesauruses.
pub fn validate_command_with_thesaurus(command: &str, thesaurus: Thesaurus) -> KgValidationResult {
    let matches = match terraphim_automata::find_matches(command, thesaurus, false) {
        Ok(m) => m,
        Err(_) => return KgValidationResult::empty(),
    };

    if matches.is_empty() {
        return KgValidationResult::empty();
    }

    let mut findings = Vec::new();
    for m in &matches {
        let suggested = m.normalized_term.display().to_string();
        let matched = m.term.clone();

        // Only report findings where the matched term differs from the suggestion
        // (i.e., the term is a synonym, not the canonical term itself)
        if matched.to_lowercase() != suggested.to_lowercase() {
            findings.push(ValidationFinding {
                matched_term: matched,
                suggested_replacement: suggested,
                position: m.pos,
            });
        }
    }

    KgValidationResult {
        has_findings: !findings.is_empty(),
        findings,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use terraphim_types::{NormalizedTerm, NormalizedTermValue};

    /// Create a test thesaurus with npm/yarn/pnpm -> bun mappings
    fn create_test_thesaurus() -> Thesaurus {
        let mut thesaurus = Thesaurus::new("test_kg_validation".to_string());

        // "bun install" is the canonical term; "npm install", "yarn install" are synonyms
        let bun_install_term = NormalizedTerm::new(1u64, NormalizedTermValue::from("bun install"))
            .with_display_value("bun install".to_string());
        thesaurus.insert(
            NormalizedTermValue::from("npm install"),
            bun_install_term.clone(),
        );
        thesaurus.insert(
            NormalizedTermValue::from("yarn install"),
            bun_install_term.clone(),
        );
        thesaurus.insert(
            NormalizedTermValue::from("pnpm install"),
            bun_install_term.clone(),
        );
        // Also insert the canonical term itself
        thesaurus.insert(NormalizedTermValue::from("bun install"), bun_install_term);

        // "bun" is the canonical term; "npm", "yarn", "pnpm" are synonyms
        let bun_term = NormalizedTerm::new(2u64, NormalizedTermValue::from("bun"))
            .with_display_value("bun".to_string());
        thesaurus.insert(NormalizedTermValue::from("npm"), bun_term.clone());
        thesaurus.insert(NormalizedTermValue::from("yarn"), bun_term.clone());
        thesaurus.insert(NormalizedTermValue::from("pnpm"), bun_term.clone());
        thesaurus.insert(NormalizedTermValue::from("bun"), bun_term);

        thesaurus
    }

    #[test]
    fn test_npm_install_suggests_bun_install() {
        let thesaurus = create_test_thesaurus();
        let result = validate_command_with_thesaurus("npm install express", thesaurus);

        assert!(result.has_findings);
        assert!(!result.findings.is_empty());

        // Should suggest bun install as replacement for npm install
        let finding = result
            .findings
            .iter()
            .find(|f| f.matched_term == "npm install")
            .expect("Should find npm install match");
        assert_eq!(finding.suggested_replacement, "bun install");
    }

    #[test]
    fn test_cargo_build_no_findings() {
        let thesaurus = create_test_thesaurus();
        let result = validate_command_with_thesaurus("cargo build --release", thesaurus);

        assert!(!result.has_findings);
        assert!(result.findings.is_empty());
    }

    #[test]
    fn test_bun_install_no_findings_for_canonical() {
        let thesaurus = create_test_thesaurus();
        let result = validate_command_with_thesaurus("bun install express", thesaurus);

        // The canonical term "bun install" should not produce findings
        // since matched_term == suggested_replacement
        assert!(!result.has_findings);
    }

    #[test]
    fn test_yarn_install_suggests_bun_install() {
        let thesaurus = create_test_thesaurus();
        let result = validate_command_with_thesaurus("yarn install", thesaurus);

        assert!(result.has_findings);
        let finding = result
            .findings
            .iter()
            .find(|f| f.matched_term == "yarn install")
            .expect("Should find yarn install match");
        assert_eq!(finding.suggested_replacement, "bun install");
    }

    #[test]
    fn test_empty_result_serialization() {
        let result = KgValidationResult::empty();
        let json = serde_json::to_string(&result).unwrap();
        let parsed: KgValidationResult = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, result);
        assert!(!parsed.has_findings);
    }

    #[test]
    fn test_finding_serialization() {
        let thesaurus = create_test_thesaurus();
        let result = validate_command_with_thesaurus("npm install", thesaurus);

        let json = serde_json::to_string(&result).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed["has_findings"], true);
        assert!(parsed["findings"].is_array());
    }

    #[test]
    fn test_empty_command() {
        let thesaurus = create_test_thesaurus();
        let result = validate_command_with_thesaurus("", thesaurus);
        assert!(!result.has_findings);
    }

    #[test]
    fn test_pnpm_install_suggests_bun_install() {
        let thesaurus = create_test_thesaurus();
        let result = validate_command_with_thesaurus("pnpm install lodash", thesaurus);

        assert!(result.has_findings);
        let finding = result
            .findings
            .iter()
            .find(|f| f.matched_term == "pnpm install")
            .expect("Should find pnpm install match");
        assert_eq!(finding.suggested_replacement, "bun install");
    }
}
