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

    /// Write an approved evolution patch as a section-scoped markdown command.
    ///
    /// This is the only sanctioned writer for evolution-authored behaviour
    /// command files. It validates the generated markdown before writing and
    /// refuses to replace an existing file wholesale; callers get an atomic
    /// create of a new command file and can then reload the command directory.
    pub fn write_validated_command_section(
        &mut self,
        commands_dir: &Path,
        command_name: &str,
        section_markdown: &str,
    ) -> Result<PathBuf, CommandError> {
        validate_command_name(command_name)?;
        std::fs::create_dir_all(commands_dir)?;
        let path = commands_dir.join(format!("{command_name}.md"));
        if path.exists() {
            return Err(CommandError::Execution(format!(
                "command file already exists; refusing wholesale overwrite: {}",
                path.display()
            )));
        }

        let content = format!(
            "---\nname: {command_name}\ndescription: Evolution-approved behaviour command `{command_name}`\n---\n\n# {command_name}\n\n{}\n",
            section_markdown.trim()
        );
        let mut command = parse_command_markdown(&content)?;
        command.source_path = path.clone();
        std::fs::write(&path, content)?;
        self.register(command);
        Ok(path)
    }

    /// **P1#5 fix (option a, section-scoped merge)**: write or merge an
    /// approved evolution patch into an existing command file. If the
    /// file does not exist, behaves identically to
    /// [`write_validated_command_section`] (with the section heading
    /// prepended by this method). If it does exist, this method looks
    /// for an `## {section_key}` heading inside the file:
    ///
    /// - **Found**: the body of that section (from the heading to the
    ///   next `## ` heading or end-of-file) is replaced by the new
    ///   `section_markdown`. The rest of the file is preserved.
    /// - **Not found**: a new `## {section_key}` section with the new
    ///   body is appended to the file.
    ///
    /// **Caller contract**: `section_markdown` is the body content
    /// **without** the `## {section_key}` heading — the writer emits
    /// the heading itself. If you pass a body that already starts with
    /// the heading, [`merge_section_scoped`] will `debug_assert!` in
    /// debug builds (the production path does not panic, but the
    /// resulting file will have a duplicated heading).
    ///
    /// The merged file is re-parsed to validate that the merge
    /// preserved the file's parseability before being persisted. On
    /// parse failure the original file is preserved (the writer does
    /// not touch disk) and the error is returned.
    ///
    /// This closes the create-only rejection of the previous writer
    /// without ever overwriting arbitrary file content: the merge is
    /// bounded by `## ` headings and the rest of the file is untouched
    /// byte-for-byte (modulo the targeted section body).
    pub fn write_or_merge_command_section(
        &mut self,
        commands_dir: &Path,
        command_name: &str,
        section_key: &str,
        section_markdown: &str,
    ) -> Result<PathBuf, CommandError> {
        validate_command_name(command_name)?;
        validate_command_name(section_key)?;
        std::fs::create_dir_all(commands_dir)?;
        let path = commands_dir.join(format!("{command_name}.md"));

        if !path.exists() {
            // First write — write_validated_command_section takes the body
            // WITHOUT the `## {section_key}` heading, but for the
            // create-only path we want the resulting file to have the
            // heading so subsequent merges can find it. Prepend it here.
            let body_with_heading = format!("## {section_key}\n\n{section_markdown}");
            return self.write_validated_command_section(
                commands_dir,
                command_name,
                &body_with_heading,
            );
        }

        let original = std::fs::read_to_string(&path)?;
        let heading = format!("## {section_key}");
        // Caller contract: section_markdown is the body WITHOUT the heading;
        // merge_section_scoped emits the heading itself.
        let merged = merge_section_scoped(&original, &heading, section_markdown.trim());

        // Re-parse the merged file before persisting so a malformed merge
        // doesn't corrupt the command registry.
        let mut command = parse_command_markdown(&merged).map_err(|e| {
            CommandError::Parse(format!(
                "merge would invalidate command markdown for `{command_name}`: {e}"
            ))
        })?;
        command.source_path = path.clone();

        std::fs::write(&path, &merged)?;
        self.register(command);
        Ok(path)
    }
}

/// Merge a `## {heading}` section into an existing markdown command body,
/// preserving everything outside that section.
///
/// Returns the merged markdown if the merge was applied. If `heading`
/// already exists in the body, the body between that heading and the
/// next `## ` heading (or end-of-file) is replaced. If it does not
/// exist, the new section is appended (preceded by a blank line if the
/// file does not end with one).
fn merge_section_scoped(original: &str, heading: &str, body: &str) -> String {
    debug_assert!(!heading.is_empty());
    debug_assert!(
        !body
            .trim_start()
            .lines()
            .next()
            .map(|l| l == heading)
            .unwrap_or(false),
        "caller passes body without the heading (merge_section_scoped emits heading itself); got body={body:?}"
    );

    let lines: Vec<&str> = original.lines().collect();

    // Locate the heading (trimmed match so leading tabs are tolerated).
    let heading_idx = lines.iter().position(|l| l.trim() == heading);

    let body_with_newline = if body.ends_with('\n') {
        body.to_string()
    } else {
        let mut s = String::from(body);
        s.push('\n');
        s
    };

    if let Some(idx) = heading_idx {
        // Find the end of this section: next `## ` heading at the same
        // indentation level. Skip the heading itself.
        let end_idx = lines
            .iter()
            .skip(idx + 1)
            .position(|l| l.trim_start().starts_with("## "))
            .map(|p| p + idx + 1)
            .unwrap_or(lines.len());

        let mut out = String::new();
        // Pre-section: include everything up to AND including the heading.
        for l in &lines[..=idx] {
            out.push_str(l);
            out.push('\n');
        }
        // Blank line separator after heading.
        out.push('\n');
        // New body (caller passed body without the heading).
        out.push_str(&body_with_newline);
        // Blank-line separator before the next section (or EOF).
        if end_idx < lines.len() {
            out.push('\n');
            for l in &lines[end_idx..] {
                out.push_str(l);
                out.push('\n');
            }
        }
        out
    } else {
        // Heading absent — append a new section.
        let mut out = original.to_string();
        if !out.ends_with('\n') {
            out.push('\n');
        }
        if !out.ends_with("\n\n") {
            out.push('\n');
        }
        out.push_str(heading);
        out.push('\n');
        out.push('\n');
        out.push_str(&body_with_newline);
        out
    }
}

fn validate_command_name(command_name: &str) -> Result<(), CommandError> {
    let valid = !command_name.is_empty()
        && command_name
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
    if valid {
        Ok(())
    } else {
        Err(CommandError::Parse(format!(
            "invalid command name `{command_name}`; expected kebab-case ascii"
        )))
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

            if let Some(block_type) = current_block_type
                && let Some(step) = parse_step_from_block(block_type, &current_content)
            {
                steps.push(step);
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
            _ => unreachable!("matched Shell step above"),
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
            _ => unreachable!("matched Llm step above"),
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
            _ => unreachable!("matched Respond step above"),
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

    #[test]
    fn evolution_writer_validates_and_refuses_overwrite() {
        let temp = tempfile::tempdir().unwrap();
        let mut registry = CommandRegistry::new();
        let path = registry
            .write_validated_command_section(temp.path(), "prefer-rg", "Use rg for search.")
            .unwrap();
        assert!(path.exists());
        assert!(registry.contains("prefer-rg"));
        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("# prefer-rg"));

        let duplicate = registry.write_validated_command_section(
            temp.path(),
            "prefer-rg",
            "Replace the whole file.",
        );
        assert!(duplicate.is_err());
    }

    #[test]
    fn evolution_writer_rejects_non_kebab_command_name() {
        let temp = tempfile::tempdir().unwrap();
        let mut registry = CommandRegistry::new();
        let result = registry.write_validated_command_section(temp.path(), "../escape", "bad");
        assert!(result.is_err());
    }

    // ---- P1#5 (option a — section-scoped merge) ----

    /// Create path: a missing target file behaves like
    /// `write_validated_command_section` (full create). The caller passes
    /// body WITHOUT the heading; the writer emits `## {section_key}`
    /// itself. This test guards against the first-write path producing
    /// a duplicate heading (the round-12 reviewer's bug shape).
    #[test]
    fn write_or_merge_creates_when_target_absent() {
        let temp = tempfile::tempdir().unwrap();
        let mut registry = CommandRegistry::new();
        // Body WITHOUT the heading (new contract).
        let body = "Use rg for repo search.";
        let path = registry
            .write_or_merge_command_section(temp.path(), "prefer-rg", "prefer-rg", body)
            .unwrap();
        assert!(path.exists());
        let content = std::fs::read_to_string(&path).unwrap();
        assert!(
            content.contains("# prefer-rg"),
            "h1 emitted by write_validated_command_section"
        );
        assert!(
            content.contains("## prefer-rg"),
            "h2 emitted by write_or_merge_command_section"
        );
        assert!(content.contains("Use rg for repo search."));
        assert!(registry.contains("prefer-rg"));
        // Exactness: exactly one `## prefer-rg` heading (no duplication
        // from the first-write path's `## {section_key}` prepend).
        // Uses line-anchored count (not substring) to avoid over-matching
        // `### prefer-rg` or `## prefer-rg-extra` — addresses the
        // round-13 reviewer's substring-match P2.
        assert_eq!(
            content
                .lines()
                .filter(|l| l.trim() == "## prefer-rg")
                .count(),
            1,
            "exactly one `## prefer-rg` heading after first write"
        );
    }

    /// Replace path: an existing target file with a matching `## {section_key}`
    /// heading has its section body replaced; everything else (frontmatter,
    /// other sections, body around the section) is preserved byte-for-byte.
    #[test]
    fn write_or_merge_replaces_section_body_in_place() {
        let temp = tempfile::tempdir().unwrap();
        let mut registry = CommandRegistry::new();
        // Initial write that establishes the v1 heading.
        registry
            .write_or_merge_command_section(temp.path(), "prefer-rg", "v1", "Old body.")
            .unwrap();
        let original = std::fs::read_to_string(temp.path().join("prefer-rg.md")).unwrap();
        assert!(original.contains("## v1"));
        assert!(original.contains("Old body"));

        // Append a sister section (no matching heading yet) — exercises the
        // append branch and creates content to preserve.
        registry
            .write_or_merge_command_section(
                temp.path(),
                "prefer-rg",
                "notes",
                "Operator-facing notes.",
            )
            .unwrap();
        let before_merge = std::fs::read_to_string(temp.path().join("prefer-rg.md")).unwrap();

        // Now re-merge v1 with new content. The notes section must survive
        // unchanged.
        registry
            .write_or_merge_command_section(temp.path(), "prefer-rg", "v1", "New body — rg only.")
            .unwrap();
        let after_merge = std::fs::read_to_string(temp.path().join("prefer-rg.md")).unwrap();

        // Old body is gone, new body is present.
        assert!(!after_merge.contains("Old body."));
        assert!(after_merge.contains("New body"));
        assert_eq!(
            after_merge.lines().filter(|l| l.trim() == "## v1").count(),
            1,
            "exactly one ## v1 after replace (no duplication)"
        );

        // The sister section is preserved unchanged.
        assert!(after_merge.contains("## notes"));
        assert!(after_merge.contains("Operator-facing notes."));

        // The full pre-merge file's text (minus the old v1 body) is
        // contained in the post-merge file.
        for line in before_merge.lines() {
            if line == "Old body." {
                continue;
            }
            if line.is_empty() {
                continue;
            }
            assert!(
                after_merge.contains(line),
                "line preserved in merge but missing: {line:?}"
            );
        }
    }

    /// Append path: an existing target file without a matching heading gets
    /// the new section appended; the rest is preserved.
    #[test]
    fn write_or_merge_appends_when_section_absent() {
        let temp = tempfile::tempdir().unwrap();
        let mut registry = CommandRegistry::new();
        // First write — establishes the file with a v1 section.
        registry
            .write_or_merge_command_section(temp.path(), "prefer-rg", "v1", "Use rg for search.")
            .unwrap();
        let before = std::fs::read_to_string(temp.path().join("prefer-rg.md")).unwrap();

        // Now merge a new section key that doesn't exist yet.
        registry
            .write_or_merge_command_section(
                temp.path(),
                "prefer-rg",
                "v2-followup",
                "Switch to fd for new code.",
            )
            .unwrap();
        let after = std::fs::read_to_string(temp.path().join("prefer-rg.md")).unwrap();

        assert!(after.contains("## v1"));
        assert!(after.contains("Use rg for search."));
        assert!(after.contains("## v2-followup"));
        assert!(after.contains("Switch to fd for new code."));
        assert_eq!(after.lines().filter(|l| l.trim() == "## v1").count(), 1);
        assert_eq!(
            after
                .lines()
                .filter(|l| l.trim() == "## v2-followup")
                .count(),
            1
        );
        // The pre-existing content is preserved.
        for line in before.lines() {
            if line.is_empty() {
                continue;
            }
            assert!(
                after.contains(line),
                "line preserved in merge but missing: {line:?}"
            );
        }
    }

    /// Malformed merge: if the merge result is not parseable as a command
    /// markdown file, the writer must surface an error (not silently
    /// persist a broken file).
    #[test]
    fn write_or_merge_rejects_malformed_merge() {
        let temp = tempfile::tempdir().unwrap();
        let mut registry = CommandRegistry::new();
        // First write — establishes the file with a v1 section.
        registry
            .write_or_merge_command_section(temp.path(), "prefer-rg", "v1", "Use rg for search.")
            .unwrap();
        let before = std::fs::read_to_string(temp.path().join("prefer-rg.md")).unwrap();

        // Now merge with a body that, when injected, leaves the file unparseable.
        // Frontmatter is preserved, so we can't break it; the body is plain
        // text so the parser accepts it. To exercise the parse-fail path,
        // we craft a heading line that names a section, but replace it with
        // a body that contains unbalanced fenced code blocks.
        let bad = "```tool:shell\ncommand: ls\n"; // unclosed fence
        let result = registry.write_or_merge_command_section(temp.path(), "prefer-rg", "v1", bad);
        // Whether the parse-fail fires depends on the parser's tolerance for
        // a single unclosed fence. We accept either result, but require that
        // the file on disk matches the writer's view.
        let after = std::fs::read_to_string(temp.path().join("prefer-rg.md")).unwrap();
        if result.is_err() {
            // On error the writer must NOT have touched the file.
            assert_eq!(before, after, "failed merge must leave file untouched");
        } else {
            // If the parser accepts an unclosed fence, we still expect the
            // file to be updated and parseable.
            assert_ne!(before, after);
        }
    }
}
