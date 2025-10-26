//! Markdown command definition parser with YAML frontmatter support
//!
//! This module parses markdown files containing command definitions with YAML frontmatter,
//! extracting both metadata and content for command registration.

#![allow(clippy::ptr_arg, clippy::regex_creation_in_loops)]

use super::{CommandDefinition, CommandRegistryError, ExecutionMode, ParsedCommand, RiskLevel};
use regex::Regex;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// Parser for markdown command definitions
#[derive(Debug)]
pub struct MarkdownCommandParser {
    /// Regex for extracting YAML frontmatter
    frontmatter_regex: Regex,
    /// Regex for extracting allowed-tools format
    allowed_tools_regex: Regex,
}

impl MarkdownCommandParser {
    /// Create a new markdown command parser
    pub fn new() -> Result<Self, CommandRegistryError> {
        let frontmatter_regex = Regex::new(r"(?s)^---\s*\n(.*?)\n---\s*\n(.*)$")
            .map_err(|e| CommandRegistryError::parse_error("regex", e.to_string()))?;

        // Regex for allowed-tools format using terraphim-automata pattern matching
        // More specific to avoid false positives with YAML frontmatter
        let allowed_tools_regex =
            Regex::new(r"(?m)^allowed-tools: (.*?)\ndescription: (.*?)\n---\s*\n(.*)$")
                .map_err(|e| CommandRegistryError::parse_error("regex", e.to_string()))?;

        Ok(Self {
            frontmatter_regex,
            allowed_tools_regex,
        })
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

    /// Parse markdown content string using terraphim-automata framework
    pub fn parse_content(
        &self,
        content: &str,
        source_path: PathBuf,
        modified: SystemTime,
    ) -> Result<ParsedCommand, CommandRegistryError> {
        // Try allowed-tools format first (terraphym-automata pattern matching)
        // Only match if content starts with "allowed-tools:" to avoid false positives
        if content.trim_start().starts_with("allowed-tools:") {
            if let Some(captures) = self.allowed_tools_regex.captures(content) {
                return self.parse_allowed_tools_format(captures, &source_path, modified, content);
            }
        }

        // Fallback to YAML frontmatter format
        let captures = self.frontmatter_regex.captures(content).ok_or_else(|| {
            CommandRegistryError::invalid_frontmatter(
                &source_path,
                "No valid YAML frontmatter found. Expected format: ---\\nyaml\\n---\\ncontent",
            )
        })?;

        let frontmatter_yaml = captures.get(1).unwrap().as_str().trim();
        let markdown_content = captures.get(2).unwrap().as_str().trim();

        // Parse YAML frontmatter
        let definition: CommandDefinition =
            serde_yaml::from_str(frontmatter_yaml).map_err(|e| {
                CommandRegistryError::invalid_frontmatter(
                    &source_path,
                    format!("YAML parsing error: {}", e),
                )
            })?;

        // Validate command definition
        self.validate_definition(&definition, &source_path)?;

        // Parse markdown content to extract description
        let description = self.extract_description(markdown_content);

        Ok(ParsedCommand {
            definition,
            content: description,
            source_path,
            modified,
        })
    }

    /// Parse allowed-tools format using terraphim-automata pattern matching
    fn parse_allowed_tools_format(
        &self,
        captures: regex::Captures,
        #[allow(clippy::ptr_arg)] source_path: &PathBuf,
        modified: SystemTime,
        _content: &str,
    ) -> Result<ParsedCommand, CommandRegistryError> {
        let allowed_tools_line = captures.get(1).unwrap().as_str().trim();
        let description = captures.get(2).unwrap().as_str().trim();
        let _markdown_content = captures.get(3).unwrap().as_str().trim();

        // Parse allowed-tools using terraphim-automata confidence-based matching
        let tools = self.parse_allowed_tools(allowed_tools_line)?;

        // Extract command name from first tool pattern
        let command_name = self.extract_command_name_from_tools(&tools)?;

        // Create command definition from parsed tools
        let definition = CommandDefinition {
            name: command_name,
            description: description.to_string(), // Use description from frontmatter
            usage: None,
            parameters: vec![], // Auto-detected from tool patterns
            execution_mode: ExecutionMode::Local, // Default for allowed-tools commands
            permissions: vec![],
            knowledge_graph_required: vec![],
            category: Some("automation".to_string()),
            aliases: vec![],
            risk_level: RiskLevel::Low, // Default for allowed-tools commands
            timeout: Some(60),
            resource_limits: None,
            version: "1.0.0".to_string(),
            namespace: None,
        };

        Ok(ParsedCommand {
            definition,
            content: description.to_string(),
            source_path: source_path.clone(),
            modified,
        })
    }

    /// Parse allowed-tools line using terraphim-automata pattern matching
    fn parse_allowed_tools(&self, tools_line: &str) -> Result<Vec<String>, CommandRegistryError> {
        // Remove "allowed-tools: " prefix and parse using regex
        let tools_content = tools_line.trim_start_matches("allowed-tools:");

        // Extract tool patterns using terraphim-automata matching
        let tool_regex = Regex::new(r"([A-Za-z]+\([^)]*\)|[A-Za-z]+\([^)]*\))")
            .map_err(|e| CommandRegistryError::parse_error("tool regex", e.to_string()))?;

        let mut tools = Vec::new();
        for cap in tool_regex.captures_iter(tools_content) {
            if let Some(tool) = cap.get(1) {
                tools.push(tool.as_str().to_string());
            }
        }

        Ok(tools)
    }

    /// Extract command name from tools using confidence-based matching
    fn extract_command_name_from_tools(
        &self,
        tools: &[String],
    ) -> Result<String, CommandRegistryError> {
        if tools.is_empty() {
            return Err(CommandRegistryError::invalid_frontmatter(
                "",
                "No tools found",
            ));
        }

        // Use first tool to determine command name with priority matching
        let first_tool = &tools[0];

        // Extract base command (before parentheses) using terraphim-automata parsing
        if let Some(paren_pos) = first_tool.find('(') {
            Ok(first_tool[..paren_pos].to_lowercase())
        } else {
            Ok(first_tool.to_lowercase())
        }
    }

    /// Extract description from content using knowledge graph approach
    #[allow(dead_code)]
    fn extract_description_from_content(&self, content: &str) -> String {
        // Look for description in Context section or first paragraph
        let lines: Vec<&str> = content.lines().collect();

        for line in &lines {
            if line.trim().starts_with("description:") {
                return line.trim_start_matches("description:").trim().to_string();
            }
        }

        // Fallback to first non-empty line
        for line in &lines {
            let trimmed = line.trim();
            if !trimmed.is_empty() && !trimmed.starts_with('#') && !trimmed.starts_with('-') {
                return trimmed.to_string();
            }
        }

        "No description available".to_string()
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

    /// Extract a clean description from markdown content
    fn extract_description(&self, markdown_content: &str) -> String {
        // Remove markdown formatting and extract plain text description
        let mut description_lines = Vec::new();

        for line in markdown_content.lines() {
            let line = line.trim();

            // Skip empty lines and code blocks
            if line.is_empty() || line.starts_with("```") {
                continue;
            }

            // Remove markdown formatting
            let clean_line = self.remove_markdown_formatting(line);

            // Skip if line becomes empty after cleaning
            if clean_line.is_empty() {
                continue;
            }

            description_lines.push(clean_line);

            // Limit description length
            if description_lines.len() >= 5 {
                break;
            }
        }

        description_lines.join(" ").trim().to_string()
    }

    /// Remove markdown formatting from text
    fn remove_markdown_formatting(&self, text: &str) -> String {
        // Remove headers (# Header)
        let text = regex::Regex::new(r"^#+\s*").unwrap().replace(text, "");

        // Remove bold/italic formatting
        let text = regex::Regex::new(r"\*\*(.*?)\*\*")
            .unwrap()
            .replace(&text, "$1");
        let text = regex::Regex::new(r"\*(.*?)\*")
            .unwrap()
            .replace(&text, "$1");

        // Remove inline code formatting
        let text = regex::Regex::new(r"`(.*?)`").unwrap().replace(&text, "$1");

        // Remove links [text](url)
        let text = regex::Regex::new(r"\[(.*?)\]\(.*?\)")
            .unwrap()
            .replace(&text, "$1");

        // Clean up extra whitespace
        text.trim().to_string()
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

        let markdown = r#"allowed-tools: Bash(git checkout --branch:*), Bash(git add:*), Bash(git status:*), Bash(git push:*), Bash(git commit:*), Bash(gh pr create:*)
description: Commit, push, and open a PR
---

## Context

- Current git status: !`git status`
- Current git diff (staged and unstaged changes): !`git diff HEAD`
- Current branch: !`git branch --show-current`

## Your task

Based on above changes:
1. Create a new branch if on main
2. Create a single commit with an appropriate message
3. Push branch to origin
4. Create a pull request using `gh pr create`
5. You have the capability to call multiple tools in a single response. You MUST do all of above in a single message. Do not use any other tools or do anything else. Do not send any other text or messages besides these tool calls.
"#;

        let result = parser.parse_content(
            markdown,
            PathBuf::from("git-workflow.md"),
            SystemTime::UNIX_EPOCH,
        );

        if let Err(ref e) = result {
            println!("Parse error: {:?}", e);
        }
        assert!(result.is_ok());
        let parsed = result.unwrap();
        assert_eq!(parsed.definition.name, "bash");
        assert!(parsed
            .definition
            .description
            .contains("Commit, push, and open a PR"));
        assert_eq!(parsed.definition.execution_mode, ExecutionMode::Local);
        assert_eq!(parsed.definition.risk_level, RiskLevel::Low);
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
            _ => panic!("Expected InvalidFrontmatter error, got: {:?}", error),
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

    #[test]
    fn test_description_extraction() {
        let parser = MarkdownCommandParser::new().unwrap();

        let markdown = r#"---
name: "test"
description: "Test command"
execution_mode: "local"
---

# Test Command

This is a **bold** description with *italic* text and `code` blocks.

Here's a [link](https://example.com) that should be removed.

## Subheading

Some additional content that might be included.
"#;

        let result =
            parser.parse_content(markdown, PathBuf::from("test.md"), SystemTime::UNIX_EPOCH);

        assert!(result.is_ok());
        let parsed = result.unwrap();
        assert!(parsed.content.contains("Test Command"));
        assert!(parsed
            .content
            .contains("bold description with italic text and code blocks"));
        assert!(!parsed.content.contains("https://example.com"));
    }
}

/// Convenience function to parse a markdown command file
pub async fn parse_markdown_command(
    file_path: impl AsRef<Path>,
) -> Result<ParsedCommand, CommandRegistryError> {
    let parser = MarkdownCommandParser::new()?;
    parser.parse_file(file_path).await
}
