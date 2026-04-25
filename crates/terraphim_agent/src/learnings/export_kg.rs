//! Export captured corrections as reviewable KG markdown artefacts.
//!
//! Reads `correction-*.md` files from the learnings directory, groups
//! compatible corrections by their canonical `corrected` value, and emits
//! one Logseq-style KG markdown file per unique corrected term.

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;

use super::capture::{CorrectionEvent, CorrectionType};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CorrectionTypeFilter {
    ToolPreference,
    All,
}

impl CorrectionTypeFilter {
    fn matches(&self, ct: &CorrectionType) -> bool {
        match self {
            CorrectionTypeFilter::All => true,
            CorrectionTypeFilter::ToolPreference => *ct == CorrectionType::ToolPreference,
        }
    }
}

#[derive(Debug)]
struct MergedCorrection {
    corrected: String,
    originals: Vec<String>,
    session_ids: Vec<String>,
    contexts: Vec<String>,
    captured_at: Option<chrono::DateTime<chrono::Utc>>,
}

impl MergedCorrection {
    fn to_kg_markdown(&self) -> String {
        let mut md = String::new();

        let synonyms = self.originals.join(", ");
        let captured = self
            .captured_at
            .map(|dt| dt.to_rfc3339())
            .unwrap_or_else(|| "unknown".to_string());

        md.push_str("---\n");
        md.push_str("type::: kg_entry\n");
        md.push_str("correction_type: tool_preference\n");
        md.push_str(&format!("captured_at: {}\n", captured));
        md.push_str("source: learned\n");
        md.push_str(&format!("synonyms:: {}\n", synonyms));
        md.push_str("priority:: 70\n");
        md.push_str("pinned:: false\n");
        md.push_str("---\n\n");

        md.push_str(&format!("# {}\n\n", self.corrected));

        md.push_str("## Original\n\n");
        md.push_str(&format!(
            "Multiple alternatives resolved to this correction:\n{}\n\n",
            self.originals
                .iter()
                .map(|o| format!("- `{}`", o))
                .collect::<Vec<_>>()
                .join("\n")
        ));

        md.push_str("## Corrected\n\n");
        md.push_str(&format!("`{}`\n\n", self.corrected));

        if !self.contexts.is_empty() && !self.contexts.iter().all(|c| c.is_empty()) {
            md.push_str("## Context\n\n");
            for ctx in &self.contexts {
                if !ctx.is_empty() {
                    md.push_str(&format!("- {}\n", ctx));
                }
            }
            md.push('\n');
        }

        if !self.session_ids.is_empty() {
            md.push_str("## Sessions\n\n");
            for sid in &self.session_ids {
                md.push_str(&format!("- {}\n", sid));
            }
            md.push('\n');
        }

        md
    }
}

fn slugify(s: &str) -> String {
    let mut result = String::new();
    let mut prev_was_sep = false;

    for c in s.chars() {
        if c.is_alphanumeric() || c == '-' || c == '_' {
            if c.is_uppercase() {
                for low in c.to_lowercase() {
                    result.push(low);
                }
            } else {
                result.push(c);
            }
            prev_was_sep = false;
        } else if !prev_was_sep {
            result.push('-');
            prev_was_sep = true;
        }
    }

    while result.ends_with('-') {
        result.pop();
    }

    result
}

fn deterministic_filename(corrected: &str, index: usize) -> String {
    let base = slugify(corrected);
    if index == 0 {
        format!("{}.md", base)
    } else {
        format!("{}-{}.md", base, index)
    }
}

fn read_corrections(learnings_dir: &Path) -> Result<Vec<CorrectionEvent>, std::io::Error> {
    let mut corrections = Vec::new();

    if !learnings_dir.exists() || !learnings_dir.is_dir() {
        return Ok(corrections);
    }

    for entry in fs::read_dir(learnings_dir)? {
        let entry = entry?;
        let path = entry.path();

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

        if let Some(correction) = CorrectionEvent::from_markdown(&content) {
            corrections.push(correction);
        }
    }

    Ok(corrections)
}

fn group_corrections(
    corrections: Vec<CorrectionEvent>,
    filter: CorrectionTypeFilter,
) -> HashMap<String, MergedCorrection> {
    let mut groups: HashMap<String, MergedCorrection> = HashMap::new();

    for correction in corrections {
        if !filter.matches(&correction.correction_type) {
            continue;
        }

        if correction.original.is_empty() || correction.corrected.is_empty() {
            continue;
        }

        let key = correction.corrected.clone();
        let entry = groups.entry(key).or_insert_with(|| MergedCorrection {
            corrected: correction.corrected.clone(),
            originals: Vec::new(),
            session_ids: Vec::new(),
            contexts: Vec::new(),
            captured_at: None,
        });

        if !entry.originals.contains(&correction.original) {
            entry.originals.push(correction.original.clone());
        }

        if let Some(ref sid) = correction.session_id {
            if !entry.session_ids.contains(sid) {
                entry.session_ids.push(sid.clone());
            }
        }

        if !correction.context_description.is_empty()
            && !entry.contexts.contains(&correction.context_description)
        {
            entry.contexts.push(correction.context_description.clone());
        }

        if entry.captured_at.is_none()
            || correction
                .context
                .captured_at
                .lt(&entry.captured_at.unwrap_or(correction.context.captured_at))
        {
            entry.captured_at = Some(correction.context.captured_at);
        }
    }

    groups
}

pub fn export_corrections_as_kg(
    learnings_dir: &Path,
    output_dir: &Path,
    filter: CorrectionTypeFilter,
) -> Result<usize, std::io::Error> {
    let corrections = read_corrections(learnings_dir)?;
    let groups = group_corrections(corrections, filter);

    if groups.is_empty() {
        log::info!("No corrections to export");
        return Ok(0);
    }

    if !output_dir.exists() {
        fs::create_dir_all(output_dir)?;
    }

    let mut exported = 0usize;
    let mut used_names: HashSet<String> = HashSet::new();

    for (_, mut merged) in groups {
        merged.originals.sort();
        merged.session_ids.sort();
        merged.contexts.sort();
        merged.contexts.dedup();

        let mut idx = 0;
        let filename = loop {
            let name = deterministic_filename(&merged.corrected, idx);
            if !used_names.contains(&name) {
                break name;
            }
            idx += 1;
        };
        used_names.insert(filename.clone());
        let path = output_dir.join(&filename);

        let content = merged.to_kg_markdown();
        fs::write(&path, content)?;

        log::info!("Exported correction KG: {:?}", path);
        exported += 1;
    }

    Ok(exported)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::learnings::LearningSource;
    use tempfile::TempDir;

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

    fn write_correction(dir: &Path, name: &str, event: &CorrectionEvent) {
        let filename = format!("correction-{}.md", name);
        let path = dir.join(filename);
        fs::write(path, event.to_markdown()).expect("failed to write test correction");
    }

    #[test]
    fn test_export_empty_dir() {
        let tmp = TempDir::new().unwrap();
        let out = TempDir::new().unwrap();

        let count =
            export_corrections_as_kg(tmp.path(), out.path(), CorrectionTypeFilter::All).unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn test_export_single_tool_preference() {
        let tmp = TempDir::new().unwrap();
        let out = TempDir::new().unwrap();

        let event = make_correction(CorrectionType::ToolPreference, "npm install", "bun install");
        write_correction(tmp.path(), "001", &event);

        let count =
            export_corrections_as_kg(tmp.path(), out.path(), CorrectionTypeFilter::All).unwrap();
        assert_eq!(count, 1);

        let expected = out.path().join("bun-install.md");
        assert!(expected.exists());

        let content = fs::read_to_string(&expected).unwrap();
        assert!(content.contains("# bun install"));
        assert!(content.contains("synonyms:: npm install"));
        assert!(content.contains("type::: kg_entry"));
    }

    #[test]
    fn test_export_merges_multiple_for_same_corrected() {
        let tmp = TempDir::new().unwrap();
        let out = TempDir::new().unwrap();

        let event1 = make_correction(CorrectionType::ToolPreference, "npm install", "bun install");
        let event2 = make_correction(CorrectionType::ToolPreference, "npm i", "bun install");
        let event3 = make_correction(CorrectionType::ToolPreference, "yarn add", "bun install");

        write_correction(tmp.path(), "001", &event1);
        write_correction(tmp.path(), "002", &event2);
        write_correction(tmp.path(), "003", &event3);

        let count =
            export_corrections_as_kg(tmp.path(), out.path(), CorrectionTypeFilter::All).unwrap();
        assert_eq!(count, 1);

        let expected = out.path().join("bun-install.md");
        let content = fs::read_to_string(&expected).unwrap();
        assert!(content.contains("npm install"));
        assert!(content.contains("npm i"));
    }

    #[test]
    fn test_export_filters_non_tool_preference() {
        let tmp = TempDir::new().unwrap();
        let out = TempDir::new().unwrap();

        let tool = make_correction(CorrectionType::ToolPreference, "npm", "bun");
        let naming = make_correction(CorrectionType::Naming, "foo", "bar");

        write_correction(tmp.path(), "tool", &tool);
        write_correction(tmp.path(), "naming", &naming);

        let count =
            export_corrections_as_kg(tmp.path(), out.path(), CorrectionTypeFilter::ToolPreference)
                .unwrap();
        assert_eq!(count, 1);

        assert!(out.path().join("bun.md").exists());
        assert!(!out.path().join("bar.md").exists());
    }

    #[test]
    fn test_export_all_includes_named_corrections() {
        let tmp = TempDir::new().unwrap();
        let out = TempDir::new().unwrap();

        let tool = make_correction(CorrectionType::ToolPreference, "npm", "bun");
        let naming = make_correction(CorrectionType::Naming, "foo", "bar");

        write_correction(tmp.path(), "tool", &tool);
        write_correction(tmp.path(), "naming", &naming);

        let count =
            export_corrections_as_kg(tmp.path(), out.path(), CorrectionTypeFilter::All).unwrap();
        assert_eq!(count, 2);

        assert!(out.path().join("bun.md").exists());
        assert!(out.path().join("bar.md").exists());
    }

    #[test]
    fn test_export_deterministic_filenames() {
        let tmp = TempDir::new().unwrap();
        let out = TempDir::new().unwrap();

        let e1 = make_correction(CorrectionType::ToolPreference, "npm", "bun");
        let e2 = make_correction(CorrectionType::ToolPreference, "yarn", "bun");

        write_correction(tmp.path(), "001", &e1);
        write_correction(tmp.path(), "002", &e2);

        let count =
            export_corrections_as_kg(tmp.path(), out.path(), CorrectionTypeFilter::All).unwrap();
        assert_eq!(count, 1);

        assert!(out.path().join("bun.md").exists());
    }

    #[test]
    fn test_slugify() {
        assert_eq!(slugify("Hello World!"), "hello-world");
        assert_eq!(slugify("npm install"), "npm-install");
        assert_eq!(slugify("foo/bar"), "foo-bar");
    }
}
