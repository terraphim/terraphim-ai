//! Skill resolver for mapping skill chain names to actual skill file paths.
//!
//! This module provides functionality to resolve skill names from the terraphim-skills
//! and zestic-engineering-skills repositories to actual file paths and metadata.

use std::collections::HashMap;
use std::path::PathBuf;

/// Source of a skill - either from Terraphim or Zestic repositories.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum SkillSource {
    /// Skills from terraphim/terraphim-skills repository
    Terraphim,
    /// Skills from zestic-ai/6d-prompts repository (zestic-engineering-skills)
    Zestic,
}

impl SkillSource {
    /// Get the default base path for skills from this source.
    pub fn default_base_path(&self) -> PathBuf {
        match self {
            Self::Terraphim => PathBuf::from("~/.config/terraphim/skills"),
            Self::Zestic => PathBuf::from("~/.config/zestic/skills"),
        }
    }

    /// Get the source name as a string.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Terraphim => "terraphim",
            Self::Zestic => "zestic",
        }
    }
}

impl std::fmt::Display for SkillSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Metadata for a resolved skill.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ResolvedSkill {
    /// The skill name
    pub name: String,
    /// Human-readable description
    pub description: String,
    /// What this skill applies to (e.g., "code review", "security audit")
    pub applicable_to: Vec<String>,
    /// Path to the skill directory
    pub path: PathBuf,
    /// Path to the skill definition file (skill.toml or skill.md)
    pub definition_path: PathBuf,
    /// Source of the skill
    pub source: SkillSource,
}

/// Errors that can occur during skill resolution
#[derive(thiserror::Error, Debug)]
pub enum SkillResolutionError {
    #[error("Skill not found: {0}")]
    SkillNotFound(String),

    #[error("Invalid skill chain: {0}")]
    InvalidChain(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Skill definition error for '{skill}': {message}")]
    DefinitionError { skill: String, message: String },
}

/// Resolver for mapping skill names to actual skill file paths.
///
/// The resolver maintains a registry of known skills from both terraphim-skills
/// and zestic-engineering-skills repositories, mapping them to their file paths
/// and validating skill chain configurations.
#[derive(Debug, Clone)]
pub struct SkillResolver {
    /// Base path for terraphim skills
    terraphim_base_path: PathBuf,
    /// Base path for zestic skills
    zestic_base_path: PathBuf,
    /// Registry of terraphim skills (name -> metadata)
    terraphim_skills: HashMap<String, SkillMetadata>,
    /// Registry of zestic skills (name -> metadata)
    zestic_skills: HashMap<String, SkillMetadata>,
}

/// Internal metadata for a skill entry
#[derive(Debug, Clone)]
struct SkillMetadata {
    name: String,
    description: String,
    applicable_to: Vec<String>,
    source: SkillSource,
}

impl Default for SkillResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl SkillResolver {
    /// Create a new skill resolver with default skill registries.
    pub fn new() -> Self {
        let mut resolver = Self {
            terraphim_base_path: SkillSource::Terraphim.default_base_path(),
            zestic_base_path: SkillSource::Zestic.default_base_path(),
            terraphim_skills: HashMap::new(),
            zestic_skills: HashMap::new(),
        };

        resolver.initialize_terraphim_skills();
        resolver
    }

    /// Create a new skill resolver with custom base paths.
    pub fn with_paths(terraphim_path: impl Into<PathBuf>, zestic_path: impl Into<PathBuf>) -> Self {
        let mut resolver = Self {
            terraphim_base_path: terraphim_path.into(),
            zestic_base_path: zestic_path.into(),
            terraphim_skills: HashMap::new(),
            zestic_skills: HashMap::new(),
        };

        resolver.initialize_terraphim_skills();
        resolver
    }

    /// Initialize the terraphim skills registry with known skills.
    fn initialize_terraphim_skills(&mut self) {
        let terraphim_skills = vec![
            (
                "security-audit",
                "Security auditing for Rust/WebAssembly applications",
                vec!["security", "audit"],
            ),
            (
                "code-review",
                "Thorough code review for Rust/WebAssembly projects",
                vec!["review", "quality"],
            ),
            (
                "session-search",
                "Search and analyze AI coding assistant session history",
                vec!["search", "sessions"],
            ),
            (
                "local-knowledge",
                "Leverage personal notes and documentation through Terraphim",
                vec!["knowledge", "documentation"],
            ),
            (
                "git-safety-guard",
                "Blocks destructive git and filesystem commands",
                vec!["git", "safety"],
            ),
            (
                "devops",
                "DevOps automation for Rust projects",
                vec!["devops", "ci/cd", "deployment"],
            ),
            (
                "disciplined-research",
                "Phase 1 of disciplined development - deep problem understanding",
                vec!["research", "discovery"],
            ),
            (
                "architecture",
                "System architecture design for Rust/WebAssembly projects",
                vec!["architecture", "design"],
            ),
            (
                "disciplined-design",
                "Phase 2 of disciplined development - implementation planning",
                vec!["design", "planning"],
            ),
            (
                "requirements-traceability",
                "Create or audit requirements traceability",
                vec!["requirements", "traceability"],
            ),
            (
                "testing",
                "Comprehensive test writing and execution",
                vec!["testing", "tests"],
            ),
            (
                "acceptance-testing",
                "Plan and implement user acceptance tests",
                vec!["acceptance", "uat"],
            ),
            (
                "documentation",
                "Technical documentation for Rust projects",
                vec!["docs", "documentation"],
            ),
            (
                "md-book",
                "MD-Book documentation generator",
                vec!["documentation", "mdbook"],
            ),
            (
                "implementation",
                "Production-ready code implementation",
                vec!["implementation", "coding"],
            ),
            (
                "rust-development",
                "Idiomatic Rust development",
                vec!["rust", "development"],
            ),
            (
                "visual-testing",
                "Design and implement visual regression testing",
                vec!["testing", "visual"],
            ),
            (
                "quality-gate",
                "Right-side-of-V verification/validation orchestration",
                vec!["quality", "gate"],
            ),
        ];

        for (name, description, applicable_to) in terraphim_skills {
            self.terraphim_skills.insert(
                name.to_string(),
                SkillMetadata {
                    name: name.to_string(),
                    description: description.to_string(),
                    applicable_to: applicable_to.iter().map(|s| s.to_string()).collect(),
                    source: SkillSource::Terraphim,
                },
            );
        }
    }

    /// Validate that a skill chain contains only valid skills.
    ///
    /// Returns Ok(()) if all skills are valid, or Err with a list of invalid skill names.
    pub fn validate_skill_chain(&self, chain: &[String]) -> Result<(), Vec<String>> {
        let invalid: Vec<String> = chain
            .iter()
            .filter(|skill| !self.is_valid_skill(skill))
            .cloned()
            .collect();

        if invalid.is_empty() {
            Ok(())
        } else {
            Err(invalid)
        }
    }

    /// Check if a skill name is valid (exists in either registry).
    fn is_valid_skill(&self, name: &str) -> bool {
        self.terraphim_skills.contains_key(name) || self.zestic_skills.contains_key(name)
    }

    /// Resolve a single skill by name.
    ///
    /// Returns the resolved skill metadata and paths, or an error if not found.
    pub fn resolve_skill(&self, name: &str) -> Result<ResolvedSkill, SkillResolutionError> {
        // Check terraphim skills first
        if let Some(metadata) = self.terraphim_skills.get(name) {
            return self.build_resolved_skill(metadata);
        }

        // Check zestic skills
        if let Some(metadata) = self.zestic_skills.get(name) {
            return self.build_resolved_skill(metadata);
        }

        Err(SkillResolutionError::SkillNotFound(name.to_string()))
    }

    /// Resolve a skill chain to a list of resolved skills.
    ///
    /// Takes a vector of skill names and returns resolved metadata for each.
    /// If any skill is not found, returns an error listing the missing skills.
    pub fn resolve_skill_chain(
        &self,
        chain: Vec<String>,
    ) -> Result<Vec<ResolvedSkill>, SkillResolutionError> {
        // Validate first
        if let Err(invalid) = self.validate_skill_chain(&chain) {
            return Err(SkillResolutionError::InvalidChain(format!(
                "Unknown skills: {}",
                invalid.join(", ")
            )));
        }

        // Resolve each skill
        chain
            .into_iter()
            .map(|name| self.resolve_skill(&name))
            .collect()
    }

    /// Build a ResolvedSkill from metadata.
    fn build_resolved_skill(
        &self,
        metadata: &SkillMetadata,
    ) -> Result<ResolvedSkill, SkillResolutionError> {
        let base_path = match metadata.source {
            SkillSource::Terraphim => &self.terraphim_base_path,
            SkillSource::Zestic => &self.zestic_base_path,
        };

        let skill_path = base_path.join(format!("skills/{}", metadata.name));

        // Check for skill.toml first, then skill.md
        let definition_path = if skill_path.join("skill.toml").exists() {
            skill_path.join("skill.toml")
        } else {
            skill_path.join("skill.md")
        };

        Ok(ResolvedSkill {
            name: metadata.name.clone(),
            description: metadata.description.clone(),
            applicable_to: metadata.applicable_to.clone(),
            path: skill_path,
            definition_path,
            source: metadata.source,
        })
    }

    /// Get all available terraphim skill names.
    pub fn terraphim_skill_names(&self) -> Vec<String> {
        self.terraphim_skills.keys().cloned().collect()
    }

    /// Get all available zestic skill names.
    pub fn zestic_skill_names(&self) -> Vec<String> {
        self.zestic_skills.keys().cloned().collect()
    }

    /// Get all available skill names from both sources.
    pub fn all_skill_names(&self) -> Vec<String> {
        let mut names = self.terraphim_skill_names();
        names.extend(self.zestic_skill_names());
        names
    }

    /// Set the base path for terraphim skills (useful for testing).
    pub fn set_terraphim_base_path(&mut self, path: impl Into<PathBuf>) {
        self.terraphim_base_path = path.into();
    }

    /// Set the base path for zestic skills (useful for testing).
    pub fn set_zestic_base_path(&mut self, path: impl Into<PathBuf>) {
        self.zestic_base_path = path.into();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_skill_source_default_paths() {
        assert_eq!(
            SkillSource::Terraphim.default_base_path(),
            PathBuf::from("~/.config/terraphim/skills")
        );
        assert_eq!(
            SkillSource::Zestic.default_base_path(),
            PathBuf::from("~/.config/zestic/skills")
        );
    }

    #[test]
    fn test_skill_source_as_str() {
        assert_eq!(SkillSource::Terraphim.as_str(), "terraphim");
        assert_eq!(SkillSource::Zestic.as_str(), "zestic");
    }

    #[test]
    fn test_skill_source_display() {
        assert_eq!(format!("{}", SkillSource::Terraphim), "terraphim");
        assert_eq!(format!("{}", SkillSource::Zestic), "zestic");
    }

    #[test]
    fn test_resolver_has_terraphim_skills() {
        let resolver = SkillResolver::new();

        // Check that expected terraphim skills are present
        assert!(resolver.terraphim_skills.contains_key("security-audit"));
        assert!(resolver.terraphim_skills.contains_key("code-review"));
        assert!(resolver.terraphim_skills.contains_key("rust-development"));
        assert!(resolver.terraphim_skills.contains_key("quality-gate"));
    }

    #[test]
    fn test_resolve_valid_skill() {
        let resolver = SkillResolver::new();

        let skill = resolver.resolve_skill("security-audit").unwrap();
        assert_eq!(skill.name, "security-audit");
        assert!(!skill.description.is_empty());
        assert_eq!(skill.source, SkillSource::Terraphim);
        assert!(skill.path.to_string_lossy().contains("security-audit"));
    }

    #[test]
    fn test_resolve_missing_skill() {
        let resolver = SkillResolver::new();

        let result = resolver.resolve_skill("nonexistent-skill");
        assert!(result.is_err());
        match result {
            Err(SkillResolutionError::SkillNotFound(name)) => {
                assert_eq!(name, "nonexistent-skill");
            }
            _ => panic!("Expected SkillNotFound error"),
        }
    }

    #[test]
    fn test_resolve_skill_chain_valid() {
        let resolver = SkillResolver::new();

        let chain = vec!["security-audit".to_string(), "code-review".to_string()];

        let resolved = resolver.resolve_skill_chain(chain).unwrap();
        assert_eq!(resolved.len(), 2);
        assert_eq!(resolved[0].name, "security-audit");
        assert_eq!(resolved[1].name, "code-review");
    }

    #[test]
    fn test_resolve_skill_chain_empty() {
        let resolver = SkillResolver::new();

        let chain: Vec<String> = vec![];
        let resolved = resolver.resolve_skill_chain(chain).unwrap();
        assert!(resolved.is_empty());
    }

    #[test]
    fn test_resolve_skill_chain_missing_skill() {
        let resolver = SkillResolver::new();

        let chain = vec![
            "security-audit".to_string(),
            "nonexistent-skill".to_string(),
        ];

        let result = resolver.resolve_skill_chain(chain);
        assert!(result.is_err());
        match result {
            Err(SkillResolutionError::InvalidChain(msg)) => {
                assert!(msg.contains("nonexistent-skill"));
            }
            _ => panic!("Expected InvalidChain error"),
        }
    }

    #[test]
    fn test_validate_skill_chain_valid() {
        let resolver = SkillResolver::new();

        let chain = vec![
            "security-audit".to_string(),
            "code-review".to_string(),
            "rust-development".to_string(),
        ];

        assert!(resolver.validate_skill_chain(&chain).is_ok());
    }

    #[test]
    fn test_validate_skill_chain_invalid() {
        let resolver = SkillResolver::new();

        let chain = vec!["security-audit".to_string(), "unknown-skill".to_string()];

        let result = resolver.validate_skill_chain(&chain);
        assert!(result.is_err());
        let invalid = result.unwrap_err();
        assert_eq!(invalid, vec!["unknown-skill"]);
    }

    #[test]
    fn test_validate_skill_chain_empty() {
        let resolver = SkillResolver::new();

        let chain: Vec<String> = vec![];
        assert!(resolver.validate_skill_chain(&chain).is_ok());
    }

    #[test]
    fn test_skill_names_collection() {
        let resolver = SkillResolver::new();

        let terraphim_names = resolver.terraphim_skill_names();
        assert!(terraphim_names.contains(&"security-audit".to_string()));
        assert!(terraphim_names.contains(&"code-review".to_string()));

        let all_names = resolver.all_skill_names();
        assert!(all_names.contains(&"security-audit".to_string()));
    }

    #[test]
    fn test_resolved_skill_structure() {
        let resolver = SkillResolver::new();

        let skill = resolver.resolve_skill("session-search").unwrap();

        assert_eq!(skill.name, "session-search");
        assert!(!skill.description.is_empty());
        assert!(!skill.applicable_to.is_empty());
        assert_eq!(skill.source, SkillSource::Terraphim);
        assert!(skill.path.to_string_lossy().contains("session-search"));
        assert!(skill.definition_path.to_string_lossy().contains("skill"));
    }

    #[test]
    fn test_resolver_with_custom_paths() {
        let resolver = SkillResolver::with_paths("/custom/terraphim", "/custom/zestic");

        let skill = resolver.resolve_skill("security-audit").unwrap();
        assert!(skill.path.to_string_lossy().contains("/custom/terraphim"));
    }
}
