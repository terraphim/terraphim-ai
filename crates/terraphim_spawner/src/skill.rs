//! Skill resolution for agent prompts.
//!
//! Provides functionality to resolve skill chain names into formatted content
//! that can be injected into agent prompts. Skills are loaded from a directory
//! structure where each skill has its own subdirectory containing a SKILL.md file.

use std::path::{Path, PathBuf};
use tracing::{info, warn};

/// Resolver for skill chain content.
///
/// Loads skills from a configurable directory and formats them for injection
/// into agent prompts. Each skill is expected to be in a subdirectory with
/// a SKILL.md file.
#[derive(Debug, Clone)]
pub struct SkillResolver {
    /// Base directory containing skill subdirectories
    skills_dir: PathBuf,
}

impl SkillResolver {
    /// Create a new SkillResolver with the given skills directory.
    ///
    /// # Arguments
    /// * `skills_dir` - Path to the directory containing skill subdirectories
    pub fn new(skills_dir: impl Into<PathBuf>) -> Self {
        Self {
            skills_dir: skills_dir.into(),
        }
    }

    /// Create a SkillResolver with the default skills directory.
    ///
    /// The default location is `~/.claude/skills` (respecting the HOME env var).
    /// Returns `None` if HOME is not set.
    pub fn with_default_dir() -> Option<Self> {
        std::env::var("HOME")
            .ok()
            .map(|home| Self::new(PathBuf::from(home).join(".claude").join("skills")))
    }

    /// Resolve a chain of skill names into formatted content.
    ///
    /// For each skill name in the chain:
    /// 1. Loads the SKILL.md file from `{skills_dir}/{name}/SKILL.md`
    /// 2. Strips YAML frontmatter (content between `---` markers)
    /// 3. Formats the skill with a header
    ///
    /// Returns a formatted string containing all resolved skills, or an empty
    /// string if no skills could be resolved.
    ///
    /// # Arguments
    /// * `skill_chain` - Vector of skill names to resolve
    pub fn resolve_chain(&self, skill_chain: &[String]) -> String {
        if skill_chain.is_empty() {
            return String::new();
        }

        let mut sections = Vec::new();

        for skill_name in skill_chain {
            match self.load_skill(skill_name) {
                Ok(content) => {
                    sections.push(format!("### Skill: {}\n\n{}", skill_name, content.trim()));
                    info!(
                        skill = %skill_name,
                        bytes = content.len(),
                        "loaded skill content"
                    );
                }
                Err(e) => {
                    warn!(
                        skill = %skill_name,
                        error = %e,
                        "failed to load skill, skipping"
                    );
                }
            }
        }

        if sections.is_empty() {
            return String::new();
        }

        format!(
            "\n\n## Active Skills\n\nApply the following skill instructions to your work:\n\n{}\n",
            sections.join("\n\n---\n\n")
        )
    }

    /// Load a single skill's content.
    ///
    /// Reads the SKILL.md file and strips YAML frontmatter.
    ///
    /// # Arguments
    /// * `skill_name` - Name of the skill (subdirectory name)
    fn load_skill(&self, skill_name: &str) -> Result<String, SkillError> {
        let skill_path = self.skills_dir.join(skill_name).join("SKILL.md");

        let content = std::fs::read_to_string(&skill_path)
            .map_err(|e| SkillError::LoadFailed(skill_path.clone(), e.to_string()))?;

        // Strip YAML frontmatter (between --- markers)
        let body = if let Some(after_prefix) = content.strip_prefix("---") {
            if let Some(end) = after_prefix.find("---") {
                after_prefix[end + 3..].trim_start().to_string()
            } else {
                content
            }
        } else {
            content
        };

        Ok(body)
    }

    /// Get the skills directory path.
    pub fn skills_dir(&self) -> &Path {
        &self.skills_dir
    }
}

impl Default for SkillResolver {
    /// Creates a SkillResolver with the default directory.
    /// Panics if HOME environment variable is not set.
    fn default() -> Self {
        Self::with_default_dir()
            .expect("HOME environment variable must be set for default SkillResolver")
    }
}

/// Errors that can occur during skill resolution.
#[derive(Debug, thiserror::Error)]
pub enum SkillError {
    #[error("failed to load skill from {0}: {1}")]
    LoadFailed(PathBuf, String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    fn create_test_skill(dir: &Path, name: &str, content: &str) {
        let skill_dir = dir.join(name);
        std::fs::create_dir_all(&skill_dir).unwrap();
        let mut file = std::fs::File::create(skill_dir.join("SKILL.md")).unwrap();
        file.write_all(content.as_bytes()).unwrap();
    }

    #[test]
    fn test_resolve_chain_empty() {
        let temp_dir = TempDir::new().unwrap();
        let resolver = SkillResolver::new(temp_dir.path());
        let result = resolver.resolve_chain(&[]);
        assert!(result.is_empty());
    }

    #[test]
    fn test_resolve_chain_single_skill() {
        let temp_dir = TempDir::new().unwrap();
        create_test_skill(temp_dir.path(), "test-skill", "Test skill content");

        let resolver = SkillResolver::new(temp_dir.path());
        let result = resolver.resolve_chain(&["test-skill".to_string()]);

        assert!(result.contains("## Active Skills"));
        assert!(result.contains("### Skill: test-skill"));
        assert!(result.contains("Test skill content"));
    }

    #[test]
    fn test_resolve_chain_strips_frontmatter() {
        let temp_dir = TempDir::new().unwrap();
        create_test_skill(
            temp_dir.path(),
            "with-frontmatter",
            "---\nname: Test\nversion: 1.0\n---\n\nActual content here",
        );

        let resolver = SkillResolver::new(temp_dir.path());
        let result = resolver.resolve_chain(&["with-frontmatter".to_string()]);

        assert!(result.contains("Actual content here"));
        assert!(!result.contains("name: Test"));
        assert!(!result.contains("---"));
    }

    #[test]
    fn test_resolve_chain_missing_skill_skipped() {
        let temp_dir = TempDir::new().unwrap();
        create_test_skill(temp_dir.path(), "exists", "This skill exists");

        let resolver = SkillResolver::new(temp_dir.path());
        let result = resolver.resolve_chain(&["exists".to_string(), "missing".to_string()]);

        assert!(result.contains("### Skill: exists"));
        assert!(!result.contains("missing"));
    }

    #[test]
    fn test_resolve_chain_all_missing_returns_empty() {
        let temp_dir = TempDir::new().unwrap();
        let resolver = SkillResolver::new(temp_dir.path());
        let result = resolver.resolve_chain(&["missing1".to_string(), "missing2".to_string()]);
        assert!(result.is_empty());
    }
}
