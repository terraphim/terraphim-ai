use crate::types::{Defect, DefectType, GroundTruthManifest, PlanGroundTruth};
use anyhow::Result;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use uuid::Uuid;

/// Generates ground truth manifest from the plans directory
pub struct GroundTruthGenerator;

impl GroundTruthGenerator {
    /// Generate ground truth from existing plan files
    pub fn generate_from_plans(plans_dir: &Path) -> Result<GroundTruthManifest> {
        let mut plans = Vec::new();

        // Read plan files
        for entry in fs::read_dir(plans_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.extension().map(|e| e == "md").unwrap_or(false) {
                let plan_id = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("unknown")
                    .to_string();

                let content = fs::read_to_string(&path)?;
                let title = extract_title(&content);
                let difficulty = estimate_difficulty(&content);
                let domain = detect_domain(&content);

                // Parse seeded defects from the plan
                let defects = Self::parse_defects_from_plan(&plan_id, &content);

                plans.push(PlanGroundTruth {
                    plan_id,
                    title,
                    plan_path: path.to_string_lossy().to_string(),
                    defects,
                    difficulty,
                    domain,
                });
            }
        }

        // Sort plans by ID for consistency
        plans.sort_by(|a, b| a.plan_id.cmp(&b.plan_id));

        // Build defect summary
        let mut defect_summary: HashMap<String, usize> = HashMap::new();
        for plan in &plans {
            for defect in &plan.defects {
                *defect_summary
                    .entry(defect.defect_type.as_str().to_string())
                    .or_insert(0) += 1;
            }
        }

        Ok(GroundTruthManifest {
            experiment_id: format!("dumb-critic-{}", Uuid::new_v4().simple()),
            created_at: chrono::Utc::now().to_rfc3339(),
            plans,
            defect_summary,
        })
    }

    /// Parse defects marked in the plan content
    /// Defects are marked with HTML comments: <!-- DEFECT: type: missing_prerequisite -->
    fn parse_defects_from_plan(plan_id: &str, content: &str) -> Vec<Defect> {
        let mut defects = Vec::new();
        let lines: Vec<&str> = content.lines().collect();

        for (idx, line) in lines.iter().enumerate() {
            // Look for defect markers in HTML comments
            if line.contains("<!-- DEFECT:") {
                if let Some(defect) = Self::parse_defect_marker(plan_id, idx + 1, line, content) {
                    defects.push(defect);
                }
            }
        }

        defects
    }

    fn parse_defect_marker(
        plan_id: &str,
        line_num: usize,
        marker_line: &str,
        _content: &str,
    ) -> Option<Defect> {
        // Parse format: <!-- DEFECT: type=missing_prerequisite, description=..., expected=... -->
        let marker = marker_line.trim();

        // Extract between DEFECT: and --
        let start = marker.find("DEFECT:")? + 7;
        let end = marker.find("-->")?;
        let params_str = &marker[start..end];

        let mut defect_type = None;
        let mut description = None;
        let mut expected_fix = None;
        let mut is_seeded = true;

        // Parse key=value pairs
        for param in params_str.split(',') {
            let param = param.trim();
            if let Some(eq_pos) = param.find('=') {
                let key = param[..eq_pos].trim();
                let value = param[eq_pos + 1..].trim();

                match key {
                    "type" => defect_type = parse_defect_type(value),
                    "description" => description = Some(value.to_string()),
                    "expected" | "fix" | "expected_fix" => expected_fix = Some(value.to_string()),
                    "seeded" => is_seeded = value == "true" || value == "yes",
                    _ => {}
                }
            }
        }

        // Auto-detect defect type from description if not specified
        if defect_type.is_none() {
            defect_type = auto_detect_defect_type(description.as_deref().unwrap_or(""));
        }

        let defect_type = defect_type?;
        let description = description.unwrap_or_else(|| "Defect identified".to_string());
        let expected_fix = expected_fix.unwrap_or_else(|| "Review and fix".to_string());

        Some(Defect {
            id: format!("{}-defect-{}", plan_id, line_num),
            defect_type,
            location: Some(format!("line {}", line_num)),
            description,
            is_seeded,
            expected_fix,
        })
    }

    /// Save manifest to JSON file
    pub fn save_manifest(manifest: &GroundTruthManifest, path: &Path) -> Result<()> {
        let json = serde_json::to_string_pretty(manifest)?;
        fs::write(path, json)?;
        Ok(())
    }

    /// Load manifest from JSON file
    pub fn load_manifest(path: &Path) -> Result<GroundTruthManifest> {
        let json = fs::read_to_string(path)?;
        let manifest = serde_json::from_str(&json)?;
        Ok(manifest)
    }
}

/// Extract title from markdown content
fn extract_title(content: &str) -> String {
    for line in content.lines() {
        if let Some(stripped) = line.strip_prefix("# ") {
            return stripped.trim().to_string();
        }
    }
    "Untitled Plan".to_string()
}

/// Estimate difficulty based on content length and complexity
fn estimate_difficulty(content: &str) -> u8 {
    let lines = content.lines().count();
    let words = content.split_whitespace().count();

    // Simple heuristic based on size
    match (lines, words) {
        (l, _) if l < 20 => 1,
        (l, _) if l < 50 => 2,
        (l, w) if l < 100 || w < 500 => 3,
        (l, w) if l < 200 || w < 1000 => 4,
        _ => 5,
    }
}

/// Detect domain from content keywords
fn detect_domain(content: &str) -> String {
    let content_lower = content.to_ascii_lowercase();

    if content_lower.contains("api") || content_lower.contains("endpoint") {
        "api".to_string()
    } else if content_lower.contains("ui")
        || content_lower.contains("frontend")
        || content_lower.contains("component")
    {
        "frontend".to_string()
    } else if content_lower.contains("database")
        || content_lower.contains("db")
        || content_lower.contains("schema")
    {
        "database".to_string()
    } else if content_lower.contains("test") || content_lower.contains("spec") {
        "testing".to_string()
    } else if content_lower.contains("deploy")
        || content_lower.contains("infrastructure")
        || content_lower.contains("ci")
    {
        "devops".to_string()
    } else {
        "general".to_string()
    }
}

/// Parse defect type from string
fn parse_defect_type(s: &str) -> Option<DefectType> {
    match s.to_ascii_lowercase().as_str() {
        "missing_prerequisite" | "missing_prereq" | "prerequisite" => {
            Some(DefectType::MissingPrerequisite)
        }
        "ambiguous_acceptance_criteria" | "ambiguous_criteria" | "acceptance" => {
            Some(DefectType::AmbiguousAcceptanceCriteria)
        }
        "wrong_ordering" | "wrong_order" | "ordering" => Some(DefectType::WrongOrdering),
        "scope_creep" | "scope" => Some(DefectType::ScopeCreep),
        "missing_rollback" | "rollback" | "failure_path" => Some(DefectType::MissingRollback),
        "contradictory_statements" | "contradictory" | "contradiction" => {
            Some(DefectType::ContradictoryStatements)
        }
        "stale_reference" | "stale" | "outdated" => Some(DefectType::StaleReference),
        _ => None,
    }
}

/// Auto-detect defect type from description
fn auto_detect_defect_type(description: &str) -> Option<DefectType> {
    let desc_lower = description.to_ascii_lowercase();

    if desc_lower.contains("prerequisite") || desc_lower.contains("depends") {
        Some(DefectType::MissingPrerequisite)
    } else if desc_lower.contains("ambiguous")
        || desc_lower.contains("unclear")
        || desc_lower.contains("vague")
    {
        Some(DefectType::AmbiguousAcceptanceCriteria)
    } else if desc_lower.contains("order")
        || desc_lower.contains("sequence")
        || desc_lower.contains("before")
    {
        Some(DefectType::WrongOrdering)
    } else if desc_lower.contains("scope") || desc_lower.contains("beyond") {
        Some(DefectType::ScopeCreep)
    } else if desc_lower.contains("rollback")
        || desc_lower.contains("failure")
        || desc_lower.contains("error")
    {
        Some(DefectType::MissingRollback)
    } else if desc_lower.contains("contradict") || desc_lower.contains("conflict") {
        Some(DefectType::ContradictoryStatements)
    } else if desc_lower.contains("stale")
        || desc_lower.contains("outdated")
        || desc_lower.contains("deprecated")
    {
        Some(DefectType::StaleReference)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_title() {
        let content = "# My Great Plan\n\nThis is the content.";
        assert_eq!(extract_title(content), "My Great Plan");
    }

    #[test]
    fn test_detect_domain() {
        assert_eq!(detect_domain("Create REST API endpoints"), "api");
        assert_eq!(detect_domain("Build React components"), "frontend");
        assert_eq!(detect_domain("Setup database schema"), "database");
    }

    #[test]
    fn test_parse_defect_type() {
        assert!(matches!(
            parse_defect_type("missing_prerequisite"),
            Some(DefectType::MissingPrerequisite)
        ));
        assert!(matches!(
            parse_defect_type("scope_creep"),
            Some(DefectType::ScopeCreep)
        ));
        assert!(parse_defect_type("unknown").is_none());
    }
}
