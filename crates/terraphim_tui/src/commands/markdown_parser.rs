//! Markdown command definition parser with YAML frontmatter support
//!
//! This module parses markdown files containing command definitions with YAML frontmatter,
//! extracting both metadata and content for command registration.

use super::{CommandDefinition, CommandRegistryError, ParsedCommand};
use regex::Regex;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// Parser for markdown command definitions
#[derive(Debug)]
pub struct MarkdownCommandParser {
    /// Regex for extracting YAML frontmatter
    frontmatter_regex: Regex,
}

impl MarkdownCommandParser {
    /// Create a new markdown command parser
    pub fn new() -> Result<Self, CommandRegistryError> {
        let frontmatter_regex = Regex::new(r"^---\s*\n(.*?)\n---\s*\n(.*)$")
            .map_err(|e| CommandRegistryError::parse_error("regex", e.to_string()))?;

        Ok(Self { frontmatter_regex })
    }

    /// Parse a single markdown file containing a command definition
    pub async fn parse_file(
        &self,
        file_path: impl AsRef<Path>,
    ) -> Result<ParsedCommand, CommandRegistryError> {
        let path = file_path.as_ref();

        // Read the file content
        let content = tokio::fs::read_to_string(path)
            .await
            .map_err(|_e| CommandRegistryError::FileNotFound(path.to_string_lossy().to_string()))?;

        // Get file metadata
        let metadata = tokio::fs::metadata(path)
            .await
            .map_err(CommandRegistryError::IoError)?;
        let modified = metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH);

        // Parse the content
        self.parse_content(&content, path.to_path_buf(), modified)
    }

    /// Parse markdown content string
    pub fn parse_content(
        &self,
        content: &str,
        source_path: PathBuf,
        _modified: SystemTime,
    ) -> Result<ParsedCommand, CommandRegistryError> {
        // Extract frontmatter and content
        let captures = self.frontmatter_regex.captures(content).ok_or_else(|| {
            CommandRegistryError::invalid_frontmatter(
                &source_path,
                "No valid YAML frontmatter found. Expected format: ---\\nyaml\\n---\\ncontent",
            )
        })?;

        let frontmatter_yaml = captures.get(1).unwrap().as_str().trim();
        let _markdown_content = captures.get(2).unwrap().as_str().trim();

        // Parse YAML frontmatter
        let definition: CommandDefinition =
            serde_yaml::from_str(frontmatter_yaml).map_err(|e| {
                CommandRegistryError::invalid_frontmatter(
                    &source_path,
                    format!("YAML parsing error: {}", e),
                )
            })?;

        // Validate the command definition
        self.validate_definition(&definition, &source_path)?;

        Ok(ParsedCommand { definition })
    }

    /// Parse all command files in a directory recursively
    pub async fn parse_directory(
        &self,
        dir_path: impl AsRef<Path>,
    ) -> Result<Vec<ParsedCommand>, CommandRegistryError> {
        self.parse_directory_recursive(dir_path, 0).await
    }

    /// Internal recursive implementation with depth limiting
    async fn parse_directory_recursive(
        &self,
        dir_path: impl AsRef<Path>,
        depth: usize,
    ) -> Result<Vec<ParsedCommand>, CommandRegistryError> {
        // Prevent infinite recursion
        if depth > 10 {
            return Ok(Vec::new());
        }

        let mut commands = Vec::new();
        let mut entries = tokio::fs::read_dir(dir_path)
            .await
            .map_err(CommandRegistryError::IoError)?;

        while let Some(entry) = entries
            .next_entry()
            .await
            .map_err(CommandRegistryError::IoError)?
        {
            let path = entry.path();

            if path.is_dir() {
                // Recursively parse subdirectories - use Box::pin to avoid recursion issues
                match Box::pin(self.parse_directory_recursive(&path, depth + 1)).await {
                    Ok(mut sub_commands) => commands.append(&mut sub_commands),
                    Err(e) => {
                        eprintln!(
                            "Warning: Failed to parse directory {}: {}",
                            path.display(),
                            e
                        );
                        // Continue with other files
                    }
                }
            } else if path.extension().and_then(|s| s.to_str()) == Some("md") {
                // Parse markdown files
                match self.parse_file(&path).await {
                    Ok(command) => commands.push(command),
                    Err(e) => {
                        eprintln!(
                            "Warning: Failed to parse command file {}: {}",
                            path.display(),
                            e
                        );
                        // Continue with other files
                    }
                }
            }
        }

        Ok(commands)
    }

    /// Validate command definition
    fn validate_definition(
        &self,
        definition: &CommandDefinition,
        source_path: &Path,
    ) -> Result<(), CommandRegistryError> {
        // Validate command name
        if definition.name.is_empty() {
            return Err(CommandRegistryError::invalid_frontmatter(
                source_path,
                "Command name cannot be empty",
            ));
        }

        // Validate command name format (alphanumeric, hyphens, underscores)
        let name_regex = regex::Regex::new(r"^[a-zA-Z][a-zA-Z0-9_-]*$").unwrap();
        if !name_regex.is_match(&definition.name) {
            return Err(CommandRegistryError::invalid_frontmatter(
                source_path,
                format!("Invalid command name '{}'. Must start with letter and contain only alphanumeric characters, hyphens, and underscores", definition.name)
            ));
        }

        // Validate parameter names and types
        for param in &definition.parameters {
            if param.name.is_empty() {
                return Err(CommandRegistryError::invalid_frontmatter(
                    source_path,
                    "Parameter name cannot be empty",
                ));
            }

            // Validate parameter type
            match param.param_type.as_str() {
                "string" | "number" | "boolean" | "array" | "object" => {},
                _ => return Err(CommandRegistryError::invalid_frontmatter(
                    source_path,
                    format!("Invalid parameter type '{}' for parameter '{}'. Valid types: string, number, boolean, array, object", param.param_type, param.name)
                )),
            }

            // Validate parameter name format
            let param_name_regex = regex::Regex::new(r"^[a-zA-Z][a-zA-Z0-9_]*$").unwrap();
            if !param_name_regex.is_match(&param.name) {
                return Err(CommandRegistryError::invalid_frontmatter(
                    source_path,
                    format!("Invalid parameter name '{}'. Must start with letter and contain only alphanumeric characters and underscores", param.name)
                ));
            }
        }

        // Validate that required parameters don't have default values
        for param in &definition.parameters {
            if param.required && param.default_value.is_some() {
                return Err(CommandRegistryError::invalid_frontmatter(
                    source_path,
                    format!(
                        "Required parameter '{}' cannot have a default value",
                        param.name
                    ),
                ));
            }
        }

        // Validate timeout
        if let Some(timeout) = definition.timeout {
            if timeout == 0 {
                return Err(CommandRegistryError::invalid_frontmatter(
                    source_path,
                    "Timeout cannot be zero",
                ));
            }
        }

        // Validate resource limits
        if let Some(ref limits) = definition.resource_limits {
            if let Some(max_memory) = limits.max_memory_mb {
                if max_memory == 0 {
                    return Err(CommandRegistryError::invalid_frontmatter(
                        source_path,
                        "Max memory limit cannot be zero",
                    ));
                }
            }

            if let Some(max_cpu) = limits.max_cpu_time {
                if max_cpu == 0 {
                    return Err(CommandRegistryError::invalid_frontmatter(
                        source_path,
                        "Max CPU time cannot be zero",
                    ));
                }
            }

            if let Some(max_disk) = limits.max_disk_mb {
                if max_disk == 0 {
                    return Err(CommandRegistryError::invalid_frontmatter(
                        source_path,
                        "Max disk limit cannot be zero",
                    ));
                }
            }
        }

        Ok(())
    }
}

impl Default for MarkdownCommandParser {
    fn default() -> Self {
        Self::new().expect("Failed to create MarkdownCommandParser")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::time::SystemTime;

    #[test]
    fn test_parse_simple_command() {
        let parser = MarkdownCommandParser::new().unwrap();

        let markdown = r#"---
name: "hello"
description: "Say hello to someone"
parameters:
  - name: "name"
    type: "string"
    required: true
    description: "Name of person to greet"
execution_mode: "local"
risk_level: "low"
---

# Hello Command

This command says hello to someone with a friendly message.

## Usage

Just provide a name and get a greeting!
"#;

        let result =
            parser.parse_content(markdown, PathBuf::from("hello.md"), SystemTime::UNIX_EPOCH);

        assert!(result.is_ok());
        let parsed = result.unwrap();
        assert_eq!(parsed.definition.name, "hello");
        assert_eq!(parsed.definition.description, "Say hello to someone");
        assert_eq!(parsed.definition.parameters.len(), 1);
        assert_eq!(parsed.definition.parameters[0].name, "name");
        assert!(parsed.definition.parameters[0].required);
    }

    #[test]
    fn test_invalid_command_name() {
        let parser = MarkdownCommandParser::new().unwrap();

        let markdown = r#"---
name: "123invalid"
description: "Invalid command name"
execution_mode: "local"
---

Content here
"#;

        let result = parser.parse_content(
            markdown,
            PathBuf::from("invalid.md"),
            SystemTime::UNIX_EPOCH,
        );

        assert!(result.is_err());
        let error = result.unwrap_err();
        match error {
            CommandRegistryError::InvalidFrontmatter(_, msg) => {
                assert!(msg.contains("Invalid command name"));
            }
            _ => panic!("Expected InvalidFrontmatter error"),
        }
    }

    #[test]
    fn test_missing_frontmatter() {
        let parser = MarkdownCommandParser::new().unwrap();

        let markdown = r#"This is just plain markdown
without any frontmatter.
"#;

        let result = parser.parse_content(
            markdown,
            PathBuf::from("no-frontmatter.md"),
            SystemTime::UNIX_EPOCH,
        );

        assert!(result.is_err());
        let error = result.unwrap_err();
        match error {
            CommandRegistryError::InvalidFrontmatter(_, msg) => {
                assert!(msg.contains("No valid YAML frontmatter"));
            }
            _ => panic!("Expected InvalidFrontmatter error"),
        }
    }

}
