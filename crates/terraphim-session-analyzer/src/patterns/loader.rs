//! Pattern loader from TOML configuration
//!
//! This module handles loading tool patterns from TOML files.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Tool pattern configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolPattern {
    /// Unique name of the tool
    pub name: String,

    /// List of patterns to match (e.g., "npx wrangler", "bunx wrangler")
    pub patterns: Vec<String>,

    /// Metadata about the tool
    pub metadata: ToolMetadata,
}

/// Metadata associated with a tool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolMetadata {
    /// Category of the tool (e.g., "cloudflare", "package-manager")
    pub category: String,

    /// Human-readable description
    pub description: Option<String>,

    /// Confidence score (0.0 - 1.0) for pattern matches
    #[serde(default = "default_confidence")]
    pub confidence: f32,
}

fn default_confidence() -> f32 {
    0.9
}

/// Container for TOML file structure
#[derive(Debug, Deserialize)]
struct ToolPatternsConfig {
    tools: Vec<ToolPattern>,
}

/// Load patterns from built-in TOML configuration
///
/// # Errors
///
/// Returns an error if the built-in patterns cannot be parsed
pub fn load_patterns() -> Result<Vec<ToolPattern>> {
    let toml_content = include_str!("../patterns.toml");
    load_patterns_from_str(toml_content)
}

/// Load patterns from a custom TOML file
///
/// # Errors
///
/// Returns an error if the file cannot be read or parsed
pub fn load_patterns_from_file<P: AsRef<Path>>(path: P) -> Result<Vec<ToolPattern>> {
    let content = std::fs::read_to_string(path.as_ref())
        .with_context(|| format!("Failed to read patterns from {}", path.as_ref().display()))?;

    load_patterns_from_str(&content)
}

/// Load patterns from a TOML string
///
/// # Errors
///
/// Returns an error if the TOML cannot be parsed
pub fn load_patterns_from_str(toml_str: &str) -> Result<Vec<ToolPattern>> {
    let config: ToolPatternsConfig =
        toml::from_str(toml_str).context("Failed to parse tool patterns TOML")?;

    // Validate patterns
    for tool in &config.tools {
        if tool.patterns.is_empty() {
            anyhow::bail!("Tool '{}' has no patterns defined", tool.name);
        }

        if tool.metadata.confidence < 0.0 || tool.metadata.confidence > 1.0 {
            anyhow::bail!(
                "Tool '{}' has invalid confidence score: {}",
                tool.name,
                tool.metadata.confidence
            );
        }
    }

    Ok(config.tools)
}

/// Load user-defined patterns from config file
///
/// # Errors
///
/// Returns an error if the config file exists but cannot be read or parsed
pub fn load_user_patterns() -> Result<Vec<ToolPattern>> {
    let home = home::home_dir().context("No home directory")?;
    let config_path = home
        .join(".config")
        .join("claude-log-analyzer")
        .join("tools.toml");

    if !config_path.exists() {
        return Ok(Vec::new());
    }

    load_patterns_from_file(config_path)
}

/// Load and merge built-in + user patterns
///
/// User patterns with the same name as built-in patterns will override them.
///
/// # Errors
///
/// Returns an error if patterns cannot be loaded or merged
pub fn load_all_patterns() -> Result<Vec<ToolPattern>> {
    let builtin = load_patterns()?;
    let user = load_user_patterns()?;

    merge_patterns(builtin, user)
}

/// Merge built-in and user patterns
///
/// User patterns override built-in patterns with the same name.
/// All unique patterns are preserved.
///
/// # Errors
///
/// Returns an error if pattern validation fails
fn merge_patterns(builtin: Vec<ToolPattern>, user: Vec<ToolPattern>) -> Result<Vec<ToolPattern>> {
    use std::collections::HashMap;

    // Create a map of tool name -> pattern
    let mut pattern_map: HashMap<String, ToolPattern> = HashMap::new();

    // Add built-in patterns first
    for pattern in builtin {
        pattern_map.insert(pattern.name.clone(), pattern);
    }

    // User patterns override built-in with same name
    for pattern in user {
        pattern_map.insert(pattern.name.clone(), pattern);
    }

    // Convert back to vector and validate
    let mut merged: Vec<ToolPattern> = pattern_map.into_values().collect();

    // Sort by name for consistent ordering
    merged.sort_by(|a, b| a.name.cmp(&b.name));

    // Validate the merged patterns
    for tool in &merged {
        if tool.patterns.is_empty() {
            anyhow::bail!("Tool '{}' has no patterns defined", tool.name);
        }

        if tool.metadata.confidence < 0.0 || tool.metadata.confidence > 1.0 {
            anyhow::bail!(
                "Tool '{}' has invalid confidence score: {}",
                tool.name,
                tool.metadata.confidence
            );
        }
    }

    Ok(merged)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_patterns_from_str() {
        let toml = r#"
[[tools]]
name = "wrangler"
patterns = ["npx wrangler", "bunx wrangler"]

[tools.metadata]
category = "cloudflare"
description = "Cloudflare Workers CLI"
confidence = 0.95

[[tools]]
name = "npm"
patterns = ["npm "]

[tools.metadata]
category = "package-manager"
description = "Node package manager"
confidence = 0.9
"#;

        let patterns = load_patterns_from_str(toml).unwrap();
        assert_eq!(patterns.len(), 2);

        assert_eq!(patterns[0].name, "wrangler");
        assert_eq!(patterns[0].patterns.len(), 2);
        assert_eq!(patterns[0].metadata.category, "cloudflare");
        assert_eq!(patterns[0].metadata.confidence, 0.95);

        assert_eq!(patterns[1].name, "npm");
        assert_eq!(patterns[1].patterns.len(), 1);
        assert_eq!(patterns[1].metadata.category, "package-manager");
    }

    #[test]
    fn test_default_confidence() {
        let toml = r#"
[[tools]]
name = "test"
patterns = ["test"]

[tools.metadata]
category = "test"
"#;

        let patterns = load_patterns_from_str(toml).unwrap();
        assert_eq!(patterns[0].metadata.confidence, 0.9);
    }

    #[test]
    fn test_empty_patterns_validation() {
        let toml = r#"
[[tools]]
name = "empty"
patterns = []

[tools.metadata]
category = "test"
"#;

        let result = load_patterns_from_str(toml);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("no patterns"));
    }

    #[test]
    fn test_invalid_confidence_validation() {
        let toml = r#"
[[tools]]
name = "invalid"
patterns = ["test"]

[tools.metadata]
category = "test"
confidence = 1.5
"#;

        let result = load_patterns_from_str(toml);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("invalid confidence")
        );
    }

    #[test]
    fn test_load_built_in_patterns() {
        // This will test the actual patterns.toml file once created
        let result = load_patterns();
        assert!(
            result.is_ok(),
            "Failed to load built-in patterns: {:?}",
            result.err()
        );

        let patterns = result.unwrap();
        assert!(
            !patterns.is_empty(),
            "Built-in patterns should not be empty"
        );

        // Verify some expected tools exist
        let tool_names: Vec<&str> = patterns.iter().map(|p| p.name.as_str()).collect();
        assert!(
            tool_names.contains(&"wrangler"),
            "Expected wrangler pattern"
        );
        assert!(tool_names.contains(&"npm"), "Expected npm pattern");
    }

    #[test]
    fn test_merge_patterns_unique() {
        let builtin = vec![
            ToolPattern {
                name: "npm".to_string(),
                patterns: vec!["npm ".to_string()],
                metadata: ToolMetadata {
                    category: "package-manager".to_string(),
                    description: Some("Node package manager".to_string()),
                    confidence: 0.9,
                },
            },
            ToolPattern {
                name: "cargo".to_string(),
                patterns: vec!["cargo ".to_string()],
                metadata: ToolMetadata {
                    category: "rust-toolchain".to_string(),
                    description: Some("Rust package manager".to_string()),
                    confidence: 0.95,
                },
            },
        ];

        let user = vec![ToolPattern {
            name: "custom".to_string(),
            patterns: vec!["custom ".to_string()],
            metadata: ToolMetadata {
                category: "custom".to_string(),
                description: Some("Custom tool".to_string()),
                confidence: 0.8,
            },
        }];

        let merged = merge_patterns(builtin, user).unwrap();
        assert_eq!(merged.len(), 3);

        let tool_names: Vec<&str> = merged.iter().map(|p| p.name.as_str()).collect();
        assert!(tool_names.contains(&"npm"));
        assert!(tool_names.contains(&"cargo"));
        assert!(tool_names.contains(&"custom"));
    }

    #[test]
    fn test_merge_patterns_override() {
        let builtin = vec![ToolPattern {
            name: "npm".to_string(),
            patterns: vec!["npm ".to_string()],
            metadata: ToolMetadata {
                category: "package-manager".to_string(),
                description: Some("Node package manager".to_string()),
                confidence: 0.9,
            },
        }];

        let user = vec![ToolPattern {
            name: "npm".to_string(),
            patterns: vec!["npm install".to_string(), "npm run".to_string()],
            metadata: ToolMetadata {
                category: "package-manager".to_string(),
                description: Some("Custom npm config".to_string()),
                confidence: 0.95,
            },
        }];

        let merged = merge_patterns(builtin, user).unwrap();
        assert_eq!(merged.len(), 1);

        let npm = merged.iter().find(|p| p.name == "npm").unwrap();
        assert_eq!(npm.patterns.len(), 2);
        assert_eq!(
            npm.metadata.description.as_deref(),
            Some("Custom npm config")
        );
        assert_eq!(npm.metadata.confidence, 0.95);
    }

    #[test]
    fn test_merge_patterns_validation_fails() {
        let builtin = vec![];

        let user = vec![ToolPattern {
            name: "invalid".to_string(),
            patterns: vec![],
            metadata: ToolMetadata {
                category: "test".to_string(),
                description: None,
                confidence: 0.9,
            },
        }];

        let result = merge_patterns(builtin, user);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("no patterns"));
    }

    #[test]
    fn test_load_user_patterns_no_file() {
        // This should succeed and return empty vec when no user config exists
        let result = load_user_patterns();
        assert!(result.is_ok());
    }

    #[test]
    fn test_load_all_patterns() {
        // Should at minimum load built-in patterns even without user config
        let result = load_all_patterns();
        assert!(result.is_ok());

        let patterns = result.unwrap();
        assert!(
            !patterns.is_empty(),
            "Should have at least built-in patterns"
        );
    }
}
