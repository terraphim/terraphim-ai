//! Markdown-defined commands for TinyClaw.
//!
//! This module provides support for loading and executing commands defined in Markdown files,
//! using the terraphim-markdown-parser for frontmatter extraction.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use thiserror::Error;

/// Errors that can occur when working with markdown commands.
#[derive(Debug, Error)]
pub enum CommandError {
    #[error("Command not found: {0}")]
    NotFound(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("YAML error: {0}")]
    Yaml(#[from] serde_yaml::Error),
    #[error("TOML error: {0}")]
    Toml(#[from] toml::de::Error),
    #[error("Parse error: {0}")]
    Parse(String),
    #[error("Template error: {0}")]
    Template(String),
    #[error("Missing required argument: {0}")]
    MissingArgument(String),
    #[error("Execution error: {0}")]
    Execution(String),
}

/// A command defined in a Markdown file.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct MarkdownCommand {
    /// Command name (unique identifier)
    pub name: String,
    /// Human-readable description
    pub description: String,
    /// Arguments the command accepts
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub arguments: Vec<CommandArgument>,
    /// Sequential steps to execute
    pub steps: Vec<CommandStep>,
    /// Source file path (not from frontmatter, set during loading)
    #[serde(skip)]
    pub source_path: PathBuf,
}

/// An argument definition for a command.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct CommandArgument {
    /// Argument name
    pub name: String,
    /// Human-readable description
    pub description: String,
    /// Whether this argument is required
    #[serde(default = "default_true")]
    pub required: bool,
    /// Default value if not provided
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<String>,
}

/// An individual step in a command workflow.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(tag = "type")]
pub enum CommandStep {
    /// Execute a tool
    #[serde(rename = "tool")]
    Tool {
        /// Tool name
        tool: String,
        /// Arguments for the tool (templated)
        args: serde_json::Value,
    },
    /// Send a prompt to the LLM
    #[serde(rename = "llm")]
    Llm {
        /// Prompt text (templated)
        prompt: String,
        /// Whether to include conversation history
        #[serde(default = "default_true")]
        use_context: bool,
    },
    /// Execute a shell command
    #[serde(rename = "shell")]
    Shell {
        /// Command to execute (templated)
        command: String,
        /// Working directory for the command
        #[serde(skip_serializing_if = "Option::is_none")]
        working_dir: Option<String>,
    },
    /// Respond with a templated message
    #[serde(rename = "respond")]
    Respond {
        /// Response template (templated)
        template: String,
    },
}

/// Registry for markdown commands.
pub struct CommandRegistry {
    /// Loaded commands by name
    commands: HashMap<String, MarkdownCommand>,
    /// Directories to load commands from
    search_paths: Vec<PathBuf>,
}

impl CommandRegistry {
    /// Create a new empty command registry.
    pub fn new() -> Self {
        Self {
            commands: HashMap::new(),
            search_paths: Vec::new(),
        }
    }

    /// Create a new registry with default search paths.
    pub fn with_defaults() -> Self {
        let mut registry = Self::new();
        if let Some(config_dir) = dirs::config_dir() {
            registry.add_search_path(config_dir.join("terraphim").join("commands"));
        }
        registry.add_search_path(PathBuf::from("./commands"));
        registry
    }

    /// Add a directory to search for commands.
    pub fn add_search_path(&mut self, path: impl Into<PathBuf>) {
        self.search_paths.push(path.into());
    }

    /// Register a command.
    pub fn register(&mut self, command: MarkdownCommand) {
        self.commands.insert(command.name.clone(), command);
    }

    /// Get a command by name.
    pub fn get(&self, name: &str) -> Option<&MarkdownCommand> {
        self.commands.get(name)
    }

    /// Check if a command exists.
    pub fn contains(&self, name: &str) -> bool {
        self.commands.contains_key(name)
    }

    /// List all registered commands.
    pub fn list(&self) -> Vec<&MarkdownCommand> {
        self.commands.values().collect()
    }

    /// Get command names.
    pub fn names(&self) -> Vec<&String> {
        self.commands.keys().collect()
    }

    /// Load all commands from search paths.
    pub fn load_all(&mut self) -> Result<usize, CommandError> {
        let paths: Vec<PathBuf> = self.search_paths.clone();
        let mut loaded = 0;
        for path in paths {
            if path.exists() {
                loaded += self.load_from_dir(&path)?;
            }
        }
        Ok(loaded)
    }

    /// Load commands from a directory.
    pub fn load_from_dir(&mut self, dir: &Path) -> Result<usize, CommandError> {
        let mut count = 0;

        if !dir.exists() {
            return Ok(0);
        }

        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.extension().is_some_and(|ext| ext == "md") {
                match self.load_from_file(&path) {
                    Ok(command) => {
                        log::debug!("Loaded command '{}' from {}", command.name, path.display());
                        self.register(command);
                        count += 1;
                    }
                    Err(e) => {
                        log::warn!("Failed to load command from {}: {}", path.display(), e);
                    }
                }
            }
        }

        Ok(count)
    }

    /// Load a command from a markdown file.
    fn load_from_file(&self, path: &Path) -> Result<MarkdownCommand, CommandError> {
        let content = std::fs::read_to_string(path)?;
        let mut command = parse_command_markdown(&content)?;
        command.source_path = path.to_path_buf();
        Ok(command)
    }
}

impl Default for CommandRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Parse a markdown command from content.
fn parse_command_markdown(content: &str) -> Result<MarkdownCommand, CommandError> {
    // Split frontmatter from body
    let (frontmatter, body) = split_frontmatter(content)?;

    // Parse frontmatter as YAML
    let metadata: CommandMetadata = serde_yaml::from_str(frontmatter)?;

    // Parse steps from markdown body
    let steps = parse_command_steps(body)?;

    Ok(MarkdownCommand {
        name: metadata.name,
        description: metadata.description,
        arguments: metadata.arguments,
        steps,
        source_path: PathBuf::new(),
    })
}

/// Metadata from frontmatter.
#[derive(Debug, Clone, Deserialize)]
struct CommandMetadata {
    name: String,
    description: String,
    #[serde(default)]
    arguments: Vec<CommandArgument>,
}

/// Split content into frontmatter and body.
fn split_frontmatter(content: &str) -> Result<(&str, &str), CommandError> {
    // Look for --- at the start
    if !content.starts_with("---") {
        return Err(CommandError::Parse(
            "Markdown command must start with frontmatter (---)".to_string(),
        ));
    }

    // Find the end of frontmatter (second ---)
    let after_first = &content[3..];
    if let Some(end_idx) = after_first.find("---") {
        let frontmatter = after_first[..end_idx].trim();
        let body = after_first[end_idx + 3..].trim();
        Ok((frontmatter, body))
    } else {
        Err(CommandError::Parse(
            "Frontmatter not properly closed (missing ---)".to_string(),
        ))
    }
}

/// Parse command steps from markdown body.
fn parse_command_steps(body: &str) -> Result<Vec<CommandStep>, CommandError> {
    let mut steps = Vec::new();

    // Parse code blocks with tool: prefix
    for line in body.lines() {
        let trimmed = line.trim();

        // Look for ```tool:<type> blocks
        if trimmed.starts_with("```tool:shell") {
            // Parse shell step
            if let Some(step) = parse_shell_step(body, line)? {
                steps.push(step);
            }
        } else if trimmed.starts_with("```tool:llm") {
            // Parse LLM step
            if let Some(step) = parse_llm_step(body, line)? {
                steps.push(step);
            }
        } else if trimmed.starts_with("```tool:") {
            // Parse generic tool step
            if let Some(step) = parse_tool_step(body, line)? {
                steps.push(step);
            }
        } else if trimmed.starts_with("```respond") {
            // Parse respond step
            if let Some(step) = parse_respond_step(body, line)? {
                steps.push(step);
            }
        }
    }

    // Simple parser: extract fenced code blocks with tool: prefix
    let mut in_code_block = false;
    let mut current_block_type: Option<&str> = None;
    let mut current_content = String::new();

    for line in body.lines() {
        let trimmed = line.trim();

        if trimmed.starts_with("```") && !in_code_block {
            // Start of code block
            in_code_block = true;
            current_block_type = Some(trimmed);
            current_content.clear();
        } else if trimmed == "```" && in_code_block {
            // End of code block
            in_code_block = false;

            if let Some(block_type) = current_block_type {
                if let Some(step) = parse_step_from_block(block_type, &current_content) {
                    steps.push(step);
                }
            }

            current_block_type = None;
            current_content.clear();
        } else if in_code_block {
            current_content.push_str(line);
            current_content.push('\n');
        }
    }

    Ok(steps)
}

fn parse_step_from_block(block_type: &str, content: &str) -> Option<CommandStep> {
    if block_type.starts_with("```tool:shell") {
        parse_shell_block(content)
    } else if block_type.starts_with("```tool:llm") {
        parse_llm_block(content)
    } else if block_type.starts_with("```tool:") {
        let tool_name = block_type.strip_prefix("```tool:")?;
        parse_generic_tool_block(tool_name, content)
    } else if block_type.starts_with("```respond") {
        parse_respond_block(content)
    } else {
        None
    }
}

fn parse_shell_block(content: &str) -> Option<CommandStep> {
    // Parse YAML content for shell step
    #[derive(Deserialize)]
    struct ShellConfig {
        command: String,
        #[serde(default)]
        working_dir: Option<String>,
    }

    let config: ShellConfig = serde_yaml::from_str(content).ok()?;

    Some(CommandStep::Shell {
        command: config.command,
        working_dir: config.working_dir,
    })
}

fn parse_llm_block(content: &str) -> Option<CommandStep> {
    // Parse YAML content for LLM step
    #[derive(Deserialize)]
    struct LlmConfig {
        prompt: String,
        #[serde(default = "default_true")]
        use_context: bool,
    }

    let config: LlmConfig = serde_yaml::from_str(content).ok()?;

    Some(CommandStep::Llm {
        prompt: config.prompt,
        use_context: config.use_context,
    })
}

fn parse_generic_tool_block(tool_name: &str, content: &str) -> Option<CommandStep> {
    // Parse YAML content as tool arguments
    let args: serde_json::Value = serde_yaml::from_str(content).ok()?;

    Some(CommandStep::Tool {
        tool: tool_name.to_string(),
        args,
    })
}

fn parse_respond_block(content: &str) -> Option<CommandStep> {
    // Parse YAML content for respond step
    #[derive(Deserialize)]
    struct RespondConfig {
        template: String,
    }

    let config: RespondConfig = serde_yaml::from_str(content).ok()?;

    Some(CommandStep::Respond {
        template: config.template,
    })
}

// Stub functions for the old parser
fn parse_shell_step(_body: &str, _line: &str) -> Result<Option<CommandStep>, CommandError> {
    Ok(None)
}

fn parse_llm_step(_body: &str, _line: &str) -> Result<Option<CommandStep>, CommandError> {
    Ok(None)
}

fn parse_tool_step(_body: &str, _line: &str) -> Result<Option<CommandStep>, CommandError> {
    Ok(None)
}

fn parse_respond_step(_body: &str, _line: &str) -> Result<Option<CommandStep>, CommandError> {
    Ok(None)
}

fn default_true() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_split_frontmatter() {
        let content = "---\nname: test\ndescription: A test command\n---\n\n# Body\n";
        let (frontmatter, body) = split_frontmatter(content).unwrap();
        assert!(frontmatter.contains("name: test"));
        assert!(body.contains("# Body"));
    }

    #[test]
    fn test_parse_command_markdown() {
        let content = r#"---
name: hello-world
description: A simple hello command
arguments:
  - name: name
    description: Name to greet
    required: false
    default: World
---

# Hello World

Say hello to someone.
"#;

        let command = parse_command_markdown(content).unwrap();
        assert_eq!(command.name, "hello-world");
        assert_eq!(command.description, "A simple hello command");
        assert_eq!(command.arguments.len(), 1);
        assert_eq!(command.arguments[0].name, "name");
    }

    #[test]
    fn test_parse_shell_step() {
        let content = r#"command: echo "Hello, {name}!"
"#;

        let step = parse_shell_block(content).unwrap();
        match step {
            CommandStep::Shell {
                command,
                working_dir,
            } => {
                assert!(command.contains("echo"));
                assert!(command.contains("{name}"));
                assert_eq!(working_dir, None);
            }
            _ => panic!("Expected Shell step"),
        }
    }

    #[test]
    fn test_parse_llm_step() {
        let content = r#"prompt: |
  Analyze this code for issues:

  {code}
use_context: true
"#;

        let step = parse_llm_block(content).unwrap();
        match step {
            CommandStep::Llm {
                prompt,
                use_context,
            } => {
                assert!(prompt.contains("Analyze this code"));
                assert!(prompt.contains("{code}"));
                assert!(use_context);
            }
            _ => panic!("Expected Llm step"),
        }
    }

    #[test]
    fn test_parse_respond_step() {
        let content = r#"template: |
  ## Results

  {output}
"#;

        let step = parse_respond_block(content).unwrap();
        match step {
            CommandStep::Respond { template } => {
                assert!(template.contains("## Results"));
                assert!(template.contains("{output}"));
            }
            _ => panic!("Expected Respond step"),
        }
    }

    #[test]
    fn test_command_registry() {
        let mut registry = CommandRegistry::new();

        let command = MarkdownCommand {
            name: "test-cmd".to_string(),
            description: "Test command".to_string(),
            arguments: vec![],
            steps: vec![],
            source_path: PathBuf::new(),
        };

        registry.register(command);

        assert!(registry.contains("test-cmd"));
        assert!(!registry.contains("missing"));

        let retrieved = registry.get("test-cmd").unwrap();
        assert_eq!(retrieved.name, "test-cmd");
    }

    #[test]
    fn test_parse_step_from_block() {
        // Test shell block
        let _shell_block = "```tool:shell\ncommand: ls -la\n```";
        let step = parse_step_from_block("```tool:shell", "command: ls -la");
        assert!(step.is_some());
        assert!(matches!(step.unwrap(), CommandStep::Shell { .. }));

        // Test respond block
        let step = parse_step_from_block("```respond", "template: Done!");
        assert!(step.is_some());
        assert!(matches!(step.unwrap(), CommandStep::Respond { .. }));
    }
}
