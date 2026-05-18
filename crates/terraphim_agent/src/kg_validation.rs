//! KG-based command validation for the PreToolUse hook pipeline.
//!
//! Uses Aho-Corasick matching against the knowledge graph thesaurus to detect
//! commands that have known alternatives (e.g., `npm install` -> `bun install`).
//! Returns validation results with matched patterns and suggested replacements.
//!
//! This module is designed to be fail-open: if the KG thesaurus cannot be loaded
//! or matching fails, an empty result is returned without blocking execution.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::RwLock;
use terraphim_automata::builder::compute_kg_source_hash;
use terraphim_types::Thesaurus;

use crate::learnings::{build_kg_thesaurus_with_hash, find_kg_dir};

/// Cached thesaurus with metadata for auto-invalidation.
struct CachedThesaurus {
    thesaurus: Thesaurus,
    source_hash: String,
    kg_path: PathBuf,
}

/// Global cache for the KG thesaurus used by command validation.
/// Built once from `docs/src/kg/*.md` files and reused across invocations.
/// Automatically rebuilds when source hash changes.
static KG_CACHE: RwLock<Option<CachedThesaurus>> = RwLock::new(None);

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
///
/// The cache is automatically invalidated when KG source files change.
pub fn validate_command_against_kg(command: &str) -> KgValidationResult {
    let thesaurus = match get_thesaurus_with_auto_rebuild() {
        Some(t) => t,
        None => return KgValidationResult::empty(),
    };

    validate_command_with_thesaurus(command, thesaurus)
}

/// Get the thesaurus with automatic rebuild on source change.
///
/// Returns a cloned copy of the cached thesaurus, rebuilding it if the
/// underlying KG files have changed since last build.
fn get_thesaurus_with_auto_rebuild() -> Option<Thesaurus> {
    // Fast path: check if cache is populated and still valid (read lock)
    {
        let guard = KG_CACHE.read().ok()?;
        if let Some(cached) = guard.as_ref() {
            if let Ok(Some(current_hash)) = compute_kg_source_hash(&cached.kg_path) {
                if current_hash == cached.source_hash {
                    return Some(cached.thesaurus.clone());
                }
            }
        }
    }

    // Slow path: need to rebuild (write lock)
    let mut guard = KG_CACHE.write().ok()?;

    // Re-check after acquiring write lock (another thread may have updated)
    if let Some(cached) = guard.as_ref() {
        if let Ok(Some(current_hash)) = compute_kg_source_hash(&cached.kg_path) {
            if current_hash == cached.source_hash {
                return Some(cached.thesaurus.clone());
            }
        }
    }

    // Build new cache
    let kg_dir = find_kg_dir()?;
    let (thesaurus, source_hash) = build_kg_thesaurus_with_hash(&kg_dir)?;
    let result = thesaurus.clone();
    let cached = CachedThesaurus {
        thesaurus,
        source_hash,
        kg_path: kg_dir,
    };
    *guard = Some(cached);
    Some(result)
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

    #[test]
    fn test_build_kg_thesaurus_with_hash_returns_thesaurus_and_hash() {
        use std::io::Write;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let kg_dir = temp_dir.path();

        let concept_file = kg_dir.join("test-concept.md");
        let mut file = std::fs::File::create(&concept_file).unwrap();
        writeln!(file, "synonyms:: test-alias").unwrap();

        let result = crate::learnings::build_kg_thesaurus_with_hash(kg_dir);
        assert!(result.is_some());

        let (_thesaurus, hash) = result.unwrap();
        assert!(!hash.is_empty());
        assert!(hash != "unknown");
    }

    #[test]
    fn test_build_kg_thesaurus_with_hash_empty_dir() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let kg_dir = temp_dir.path();

        // Empty directory returns None (build_kg_thesaurus_from_dir returns None for empty)
        let result = crate::learnings::build_kg_thesaurus_with_hash(kg_dir);
        assert!(result.is_none());
    }

    #[test]
    fn test_build_kg_thesaurus_with_hash_nonexistent_dir() {
        let result = crate::learnings::build_kg_thesaurus_with_hash(std::path::Path::new(
            "/nonexistent/path/xyz",
        ));
        assert!(result.is_none());
    }

    /// AC1: Adding a new .md file changes the source hash so a rebuild detects it.
    #[test]
    fn test_hash_changes_when_new_file_added() {
        use std::io::Write;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let kg_dir = temp_dir.path();

        // Initial state: one concept file
        let file1 = kg_dir.join("concept-alpha.md");
        let mut f = std::fs::File::create(&file1).unwrap();
        writeln!(f, "synonyms:: alpha-alias").unwrap();

        let (t1, hash1) = crate::learnings::build_kg_thesaurus_with_hash(kg_dir).unwrap();
        // "alpha-alias" should match
        assert!(
            t1.get(&terraphim_types::NormalizedTermValue::from("alpha-alias"))
                .is_some()
        );

        // Add a second concept file (AC1: new file)
        let file2 = kg_dir.join("concept-beta.md");
        let mut f2 = std::fs::File::create(&file2).unwrap();
        writeln!(f2, "synonyms:: beta-alias").unwrap();

        let (t2, hash2) = crate::learnings::build_kg_thesaurus_with_hash(kg_dir).unwrap();
        // Hash must change after adding a file
        assert_ne!(hash1, hash2, "Hash must change when a new file is added");
        // New concept must be present
        assert!(
            t2.get(&terraphim_types::NormalizedTermValue::from("beta-alias"))
                .is_some()
        );
    }

    /// AC2: Modifying a .md file changes the source hash so a rebuild detects it.
    #[test]
    fn test_hash_changes_when_file_modified() {
        use std::io::Write;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let kg_dir = temp_dir.path();

        let concept_file = kg_dir.join("concept-gamma.md");
        std::fs::write(&concept_file, "synonyms:: gamma-v1\n").unwrap();

        let (_, hash1) = crate::learnings::build_kg_thesaurus_with_hash(kg_dir).unwrap();

        // Modify the file (AC2: modify existing file)
        let mut f = std::fs::OpenOptions::new()
            .write(true)
            .truncate(true)
            .open(&concept_file)
            .unwrap();
        writeln!(f, "synonyms:: gamma-v1, gamma-v2").unwrap();

        let (t2, hash2) = crate::learnings::build_kg_thesaurus_with_hash(kg_dir).unwrap();
        assert_ne!(hash1, hash2, "Hash must change when a file is modified");
        assert!(
            t2.get(&terraphim_types::NormalizedTermValue::from("gamma-v2"))
                .is_some()
        );
    }

    /// AC3: Deleting a .md file changes the source hash so a rebuild detects it.
    #[test]
    fn test_hash_changes_when_file_deleted() {
        use std::io::Write;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let kg_dir = temp_dir.path();

        // Two concept files
        let file1 = kg_dir.join("concept-delta.md");
        let mut f1 = std::fs::File::create(&file1).unwrap();
        writeln!(f1, "synonyms:: delta-alias").unwrap();

        let file2 = kg_dir.join("concept-epsilon.md");
        let mut f2 = std::fs::File::create(&file2).unwrap();
        writeln!(f2, "synonyms:: epsilon-alias").unwrap();

        let (_, hash1) = crate::learnings::build_kg_thesaurus_with_hash(kg_dir).unwrap();

        // Delete one file (AC3: delete file)
        std::fs::remove_file(&file2).unwrap();

        let (t2, hash2) = crate::learnings::build_kg_thesaurus_with_hash(kg_dir).unwrap();
        assert_ne!(hash1, hash2, "Hash must change when a file is deleted");
        // Deleted concept must no longer appear
        assert!(
            t2.get(&terraphim_types::NormalizedTermValue::from("epsilon-alias"))
                .is_none(),
            "Deleted concept must not appear after rebuild"
        );
    }
}
