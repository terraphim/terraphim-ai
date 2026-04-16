//! Compile captured corrections into a thesaurus for the replace command.
//!
//! Scans the learnings directory for `correction-*.md` files, parses them,
//! and generates thesaurus entries from `ToolPreference` corrections.
//! This closes the learning feedback loop: user says "use X instead of Y"
//! -> correction captured -> compiled into thesaurus -> next `replace` call
//! uses the new mapping.

use std::fs;
use std::path::Path;

use terraphim_types::{NormalizedTerm, NormalizedTermValue, Thesaurus};

use super::capture::{CorrectionEvent, CorrectionType};

/// Scan learnings directory for `correction-*.md` files, parse them,
/// and generate thesaurus entries from `ToolPreference` corrections.
///
/// Each `ToolPreference` correction maps:
/// - `original` -> the synonym/pattern to match (thesaurus key)
/// - `corrected` -> the normalized term (nterm value)
///
/// Non-ToolPreference corrections are silently skipped.
/// Returns an empty thesaurus if the directory is empty or has no
/// qualifying corrections.
pub fn compile_corrections_to_thesaurus(learnings_dir: &Path) -> Result<Thesaurus, std::io::Error> {
    let mut thesaurus = Thesaurus::new("compiled_corrections".to_string());

    if !learnings_dir.exists() || !learnings_dir.is_dir() {
        return Ok(thesaurus);
    }

    let entries: Vec<_> = fs::read_dir(learnings_dir)?.flatten().collect();

    let mut concept_id: u64 = 1;

    for entry in entries {
        let path = entry.path();

        // Only process correction-*.md files
        let filename = match path.file_name().and_then(|n| n.to_str()) {
            Some(name) if name.starts_with("correction-") && name.ends_with(".md") => name,
            _ => continue,
        };

        let content = match fs::read_to_string(&path) {
            Ok(c) => c,
            Err(e) => {
                log::warn!("Cannot read correction file {:?}: {}", filename, e);
                continue;
            }
        };

        let correction = match CorrectionEvent::from_markdown(&content) {
            Some(c) => c,
            None => {
                log::debug!("Could not parse correction from {:?}", filename);
                continue;
            }
        };

        // Only compile ToolPreference corrections into the thesaurus
        if correction.correction_type != CorrectionType::ToolPreference {
            continue;
        }

        if correction.original.is_empty() || correction.corrected.is_empty() {
            continue;
        }

        // The original text becomes the key (pattern to match).
        // The corrected text becomes the nterm (what to replace with).
        let corrected_value = NormalizedTermValue::from(correction.corrected.as_str());
        let nterm = NormalizedTerm::new(concept_id, corrected_value)
            .with_display_value(correction.corrected.clone());

        let key = NormalizedTermValue::from(correction.original.as_str());
        thesaurus.insert(key, nterm);

        concept_id += 1;
    }

    log::info!(
        "Compiled {} correction(s) from {:?}",
        thesaurus.len(),
        learnings_dir
    );

    Ok(thesaurus)
}

/// Merge compiled corrections with an existing curated thesaurus.
///
/// Compiled corrections override curated entries with the same key
/// (learned preferences win over curated defaults).
pub fn merge_thesauruses(curated: Thesaurus, compiled: Thesaurus) -> Thesaurus {
    let mut merged = Thesaurus::new(format!("merged_{}_{}", curated.name(), compiled.name()));

    // Insert all curated entries first
    for (key, value) in &curated {
        merged.insert(key.clone(), value.clone());
    }

    // Override with compiled entries (learned preferences win)
    for (key, value) in &compiled {
        merged.insert(key.clone(), value.clone());
    }

    merged
}

/// Write thesaurus to JSON file in the format expected by `terraphim_automata`.
///
/// The output format is:
/// ```json
/// {
///   "name": "...",
///   "data": {
///     "pattern_to_match": {
///       "id": 1,
///       "nterm": "replacement_term",
///       "url": null
///     }
///   }
/// }
/// ```
pub fn write_thesaurus_json(
    thesaurus: &Thesaurus,
    output_path: &Path,
) -> Result<(), std::io::Error> {
    let json = serde_json::to_string_pretty(thesaurus).map_err(|e| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("Failed to serialize thesaurus: {}", e),
        )
    })?;

    // Create parent directories if needed
    if let Some(parent) = output_path.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent)?;
        }
    }

    fs::write(output_path, json)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::learnings::capture::{CorrectionEvent, CorrectionType, LearningSource};
    use tempfile::TempDir;

    /// Helper: write a CorrectionEvent to a correction-*.md file in the given dir.
    fn write_correction(dir: &Path, name: &str, event: &CorrectionEvent) {
        let filename = format!("correction-{}.md", name);
        let path = dir.join(filename);
        fs::write(path, event.to_markdown()).expect("failed to write test correction");
    }

    /// Helper: create a CorrectionEvent with the given type/original/corrected.
    fn make_correction(
        correction_type: CorrectionType,
        original: &str,
        corrected: &str,
    ) -> CorrectionEvent {
        CorrectionEvent::new(
            correction_type,
            original.to_string(),
            corrected.to_string(),
            String::new(),
            LearningSource::Project,
        )
    }

    #[test]
    fn test_compile_empty_dir() {
        let tmp = TempDir::new().unwrap();
        let result = compile_corrections_to_thesaurus(tmp.path()).unwrap();
        assert!(result.is_empty());
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_compile_nonexistent_dir() {
        let result =
            compile_corrections_to_thesaurus(Path::new("/tmp/nonexistent_learnings_dir_xyz"))
                .unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_compile_single_correction() {
        let tmp = TempDir::new().unwrap();

        let event = make_correction(CorrectionType::ToolPreference, "npm install", "bun install");
        write_correction(tmp.path(), "001", &event);

        let thesaurus = compile_corrections_to_thesaurus(tmp.path()).unwrap();
        assert_eq!(thesaurus.len(), 1);

        // The original "npm install" is the key, the corrected "bun install" is the nterm
        let key = NormalizedTermValue::from("npm install");
        let entry = thesaurus
            .get(&key)
            .expect("entry for 'npm install' not found");
        assert_eq!(entry.value.as_str(), "bun install");
    }

    #[test]
    fn test_compile_multiple_corrections() {
        let tmp = TempDir::new().unwrap();

        let event1 = make_correction(CorrectionType::ToolPreference, "npm install", "bun install");
        let event2 = make_correction(CorrectionType::ToolPreference, "yarn add", "bun add");
        let event3 = make_correction(CorrectionType::ToolPreference, "npx", "bunx");

        write_correction(tmp.path(), "001", &event1);
        write_correction(tmp.path(), "002", &event2);
        write_correction(tmp.path(), "003", &event3);

        let thesaurus = compile_corrections_to_thesaurus(tmp.path()).unwrap();
        assert_eq!(thesaurus.len(), 3);

        assert!(
            thesaurus
                .get(&NormalizedTermValue::from("npm install"))
                .is_some()
        );
        assert!(
            thesaurus
                .get(&NormalizedTermValue::from("yarn add"))
                .is_some()
        );
        assert!(thesaurus.get(&NormalizedTermValue::from("npx")).is_some());

        let npx_entry = thesaurus.get(&NormalizedTermValue::from("npx")).unwrap();
        assert_eq!(npx_entry.value.as_str(), "bunx");
    }

    #[test]
    fn test_compile_ignores_non_tool_preference() {
        let tmp = TempDir::new().unwrap();

        // Only ToolPreference should be compiled
        let tool = make_correction(CorrectionType::ToolPreference, "npm install", "bun install");
        let naming = make_correction(CorrectionType::Naming, "foo", "bar");
        let code = make_correction(CorrectionType::CodePattern, "unwrap()", "expect()");
        let workflow = make_correction(CorrectionType::WorkflowStep, "skip tests", "run tests");
        let fact = make_correction(CorrectionType::FactCorrection, "/api/v1", "/api/v2");

        write_correction(tmp.path(), "tool", &tool);
        write_correction(tmp.path(), "naming", &naming);
        write_correction(tmp.path(), "code", &code);
        write_correction(tmp.path(), "workflow", &workflow);
        write_correction(tmp.path(), "fact", &fact);

        let thesaurus = compile_corrections_to_thesaurus(tmp.path()).unwrap();
        assert_eq!(thesaurus.len(), 1);

        let entry = thesaurus
            .get(&NormalizedTermValue::from("npm install"))
            .expect("ToolPreference entry should be present");
        assert_eq!(entry.value.as_str(), "bun install");

        // Others should not be present
        assert!(thesaurus.get(&NormalizedTermValue::from("foo")).is_none());
        assert!(
            thesaurus
                .get(&NormalizedTermValue::from("unwrap()"))
                .is_none()
        );
    }

    #[test]
    fn test_compile_skips_non_correction_files() {
        let tmp = TempDir::new().unwrap();

        // A valid correction file
        let event = make_correction(CorrectionType::ToolPreference, "npm", "bun");
        write_correction(tmp.path(), "valid", &event);

        // A non-correction md file (wrong prefix)
        fs::write(
            tmp.path().join("learning-something.md"),
            "---\nid: test\ntype: learning\n---\nSome content",
        )
        .unwrap();

        // A non-md file
        fs::write(tmp.path().join("notes.txt"), "just some notes").unwrap();

        let thesaurus = compile_corrections_to_thesaurus(tmp.path()).unwrap();
        assert_eq!(thesaurus.len(), 1);
    }

    #[test]
    fn test_merge_thesauruses() {
        // Build a curated thesaurus with two entries
        let mut curated = Thesaurus::new("curated".to_string());
        curated.insert(
            NormalizedTermValue::from("npm"),
            NormalizedTerm::new(1, NormalizedTermValue::from("bun"))
                .with_display_value("bun".to_string()),
        );
        curated.insert(
            NormalizedTermValue::from("yarn"),
            NormalizedTerm::new(2, NormalizedTermValue::from("bun"))
                .with_display_value("bun".to_string()),
        );

        // Build a compiled thesaurus that overrides "npm" and adds "pnpm"
        let mut compiled = Thesaurus::new("compiled".to_string());
        compiled.insert(
            NormalizedTermValue::from("npm"),
            NormalizedTerm::new(10, NormalizedTermValue::from("deno"))
                .with_display_value("deno".to_string()),
        );
        compiled.insert(
            NormalizedTermValue::from("pnpm"),
            NormalizedTerm::new(11, NormalizedTermValue::from("bun"))
                .with_display_value("bun".to_string()),
        );

        let merged = merge_thesauruses(curated, compiled);

        // Should have 3 entries: npm (overridden), yarn (from curated), pnpm (from compiled)
        assert_eq!(merged.len(), 3);

        // "npm" should be overridden by compiled value
        let npm = merged.get(&NormalizedTermValue::from("npm")).unwrap();
        assert_eq!(npm.value.as_str(), "deno");
        assert_eq!(npm.id, 10);

        // "yarn" should remain from curated
        let yarn = merged.get(&NormalizedTermValue::from("yarn")).unwrap();
        assert_eq!(yarn.value.as_str(), "bun");

        // "pnpm" should come from compiled
        let pnpm = merged.get(&NormalizedTermValue::from("pnpm")).unwrap();
        assert_eq!(pnpm.value.as_str(), "bun");
    }

    #[test]
    fn test_write_thesaurus_json() {
        let tmp = TempDir::new().unwrap();
        let output = tmp.path().join("output.json");

        let mut thesaurus = Thesaurus::new("test".to_string());
        thesaurus.insert(
            NormalizedTermValue::from("npm install"),
            NormalizedTerm::new(1, NormalizedTermValue::from("bun install"))
                .with_display_value("bun install".to_string()),
        );

        write_thesaurus_json(&thesaurus, &output).unwrap();

        // Verify the file exists and is valid JSON
        let content = fs::read_to_string(&output).unwrap();
        let loaded: serde_json::Value = serde_json::from_str(&content).unwrap();

        assert_eq!(loaded["name"], "test");
        assert!(loaded["data"].is_object());

        // Verify round-trip: load back as Thesaurus
        let reloaded: Thesaurus = serde_json::from_str(&content).unwrap();
        assert_eq!(reloaded.len(), 1);
        let entry = reloaded
            .get(&NormalizedTermValue::from("npm install"))
            .unwrap();
        assert_eq!(entry.value.as_str(), "bun install");
    }

    #[test]
    fn test_write_thesaurus_json_creates_parent_dirs() {
        let tmp = TempDir::new().unwrap();
        let output = tmp.path().join("nested").join("deep").join("output.json");

        let thesaurus = Thesaurus::new("empty".to_string());
        write_thesaurus_json(&thesaurus, &output).unwrap();

        assert!(output.exists());
    }
}
