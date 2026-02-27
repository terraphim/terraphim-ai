//! Markdown-defined slash command runtime.
//!
//! Parsing follows the same YAML frontmatter split pattern used in
//! `terraphim_agent` (`--- ... ---` + markdown body).

use anyhow::{Context, bail};
use regex::{Regex, RegexBuilder};
use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use std::path::Path;

/// Runtime registry for markdown-defined slash commands.
#[derive(Debug, Clone, Default)]
pub struct MarkdownCommandRuntime {
    commands: HashMap<String, MarkdownCommand>,
}

impl MarkdownCommandRuntime {
    /// Load markdown command definitions from a directory recursively.
    pub fn load_from_dir(dir: impl AsRef<Path>) -> anyhow::Result<Self> {
        let dir = dir.as_ref();

        if !dir.exists() {
            log::warn!(
                "Markdown commands directory does not exist: {}",
                dir.display()
            );
            return Ok(Self::default());
        }

        let parser = MarkdownCommandParser::new()?;
        let mut runtime = Self::default();
        let mut stack = vec![dir.to_path_buf()];

        while let Some(current_dir) = stack.pop() {
            for entry in std::fs::read_dir(&current_dir)
                .with_context(|| format!("Failed to read directory {}", current_dir.display()))?
            {
                let entry = entry?;
                let path = entry.path();

                if path.is_dir() {
                    stack.push(path);
                    continue;
                }

                if path.extension().and_then(|ext| ext.to_str()) != Some("md") {
                    continue;
                }

                match parser.parse_file(&path) {
                    Ok(command) => runtime.insert_command(command),
                    Err(error) => {
                        log::warn!(
                            "Skipping markdown command file {}: {}",
                            path.display(),
                            error
                        );
                    }
                }
            }
        }

        Ok(runtime)
    }

    /// Number of loaded command names/aliases.
    pub fn len(&self) -> usize {
        self.commands.len()
    }

    /// Whether there are no loaded command names/aliases.
    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }

    /// Dispatch a slash command line (e.g. `/deploy prod`) if it is known.
    pub fn dispatch_from_slash_message(&self, message: &str) -> Option<String> {
        let (name, args) = parse_slash_message(message)?;
        self.dispatch(&name, &args)
    }

    /// Dispatch by slash command name and argument list.
    pub fn dispatch(&self, name: &str, args: &[String]) -> Option<String> {
        let key = name.to_ascii_lowercase();
        self.commands.get(&key).map(|command| command.render(args))
    }

    /// Return help lines for unique custom commands.
    pub fn help_lines(&self) -> Vec<String> {
        let mut seen = HashSet::new();
        let mut lines = Vec::new();

        for command in self.commands.values() {
            if !seen.insert(command.name.clone()) {
                continue;
            }

            let usage = command.usage_line();
            if command.description.is_empty() {
                lines.push(usage);
            } else {
                lines.push(format!("{} - {}", usage, command.description));
            }
        }

        lines.sort();
        lines
    }

    fn insert_command(&mut self, command: MarkdownCommand) {
        let canonical_name = command.name.to_ascii_lowercase();
        self.commands.insert(canonical_name, command.clone());

        for alias in &command.aliases {
            let alias_key = alias.to_ascii_lowercase();
            self.commands.insert(alias_key, command.clone());
        }
    }
}

#[derive(Debug, Clone)]
struct MarkdownCommand {
    name: String,
    description: String,
    usage: Option<String>,
    aliases: Vec<String>,
    parameters: Vec<MarkdownCommandParameter>,
    content: String,
}

impl MarkdownCommand {
    fn render(&self, args: &[String]) -> String {
        let mut rendered = self.content.clone();
        rendered = rendered.replace("{args}", &args.join(" "));

        for (idx, arg) in args.iter().enumerate() {
            rendered = rendered.replace(&format!("{{arg{}}}", idx), arg);
            rendered = rendered.replace(&format!("{{{}}}", idx + 1), arg);
        }

        let mut missing_required = Vec::new();
        for (idx, parameter) in self.parameters.iter().enumerate() {
            let value = args
                .get(idx)
                .cloned()
                .or_else(|| parameter.default_value.clone());

            match value {
                Some(value) => {
                    rendered = rendered.replace(&format!("{{{}}}", parameter.name), &value);
                }
                None if parameter.required => missing_required.push(parameter.name.clone()),
                None => {}
            }
        }

        if !missing_required.is_empty() {
            return format!(
                "Missing required argument(s): {}.\nUsage: {}",
                missing_required.join(", "),
                self.usage_line()
            );
        }

        if rendered.is_empty() {
            format!("Usage: {}", self.usage_line())
        } else {
            format!("{}\n\nUsage: {}", rendered, self.usage_line())
        }
    }

    fn usage_line(&self) -> String {
        self.usage
            .clone()
            .unwrap_or_else(|| format!("/{}", self.name))
    }
}

#[derive(Debug, Clone)]
struct MarkdownCommandParameter {
    name: String,
    required: bool,
    default_value: Option<String>,
}

#[derive(Debug)]
struct MarkdownCommandParser {
    frontmatter_regex: Regex,
    command_name_regex: Regex,
    parameter_name_regex: Regex,
}

impl MarkdownCommandParser {
    fn new() -> anyhow::Result<Self> {
        let frontmatter_regex = RegexBuilder::new(r"^---\s*\n(.*?)\n---\s*\n(.*)$")
            .dot_matches_new_line(true)
            .build()?;
        let command_name_regex = Regex::new(r"^[a-zA-Z][a-zA-Z0-9_-]*$")?;
        let parameter_name_regex = Regex::new(r"^[a-zA-Z][a-zA-Z0-9_]*$")?;

        Ok(Self {
            frontmatter_regex,
            command_name_regex,
            parameter_name_regex,
        })
    }

    fn parse_file(&self, path: &Path) -> anyhow::Result<MarkdownCommand> {
        let raw = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read {}", path.display()))?;
        self.parse_content(&raw, path)
    }

    fn parse_content(&self, raw: &str, path: &Path) -> anyhow::Result<MarkdownCommand> {
        let captures = self.frontmatter_regex.captures(raw).ok_or_else(|| {
            anyhow::anyhow!(
                "No valid YAML frontmatter found. Expected format: ---\\nyaml\\n---\\ncontent"
            )
        })?;

        let frontmatter_yaml = captures
            .get(1)
            .map(|value| value.as_str().trim())
            .unwrap_or_default();
        let content = captures
            .get(2)
            .map(|value| value.as_str().trim().to_string())
            .unwrap_or_default();

        let frontmatter: CommandFrontmatter = serde_yaml::from_str(frontmatter_yaml)
            .with_context(|| format!("Invalid YAML frontmatter in {}", path.display()))?;

        if !self.command_name_regex.is_match(&frontmatter.name) {
            bail!(
                "Invalid command name '{}' in {}",
                frontmatter.name,
                path.display()
            );
        }

        let mut parameters = Vec::with_capacity(frontmatter.parameters.len());
        for parameter in &frontmatter.parameters {
            if !self.parameter_name_regex.is_match(&parameter.name) {
                bail!(
                    "Invalid parameter name '{}' in {}",
                    parameter.name,
                    path.display()
                );
            }

            let default_value = parameter
                .default_value
                .as_ref()
                .and_then(yaml_value_to_string)
                .or_else(|| parameter.default.as_ref().and_then(yaml_value_to_string));

            parameters.push(MarkdownCommandParameter {
                name: parameter.name.clone(),
                required: parameter.required,
                default_value,
            });
        }

        let description = frontmatter
            .description
            .or_else(|| content.lines().next().map(str::to_string))
            .unwrap_or_default();

        Ok(MarkdownCommand {
            name: frontmatter.name.to_ascii_lowercase(),
            description,
            usage: frontmatter.usage,
            aliases: frontmatter
                .aliases
                .into_iter()
                .map(|alias| alias.to_ascii_lowercase())
                .collect(),
            parameters,
            content,
        })
    }
}

#[derive(Debug, Deserialize)]
struct CommandFrontmatter {
    name: String,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    usage: Option<String>,
    #[serde(default)]
    aliases: Vec<String>,
    #[serde(default)]
    parameters: Vec<CommandParameterFrontmatter>,
}

#[derive(Debug, Deserialize)]
struct CommandParameterFrontmatter {
    name: String,
    #[serde(default)]
    required: bool,
    #[serde(default)]
    default: Option<serde_yaml::Value>,
    #[serde(default)]
    default_value: Option<serde_yaml::Value>,
}

fn parse_slash_message(input: &str) -> Option<(String, Vec<String>)> {
    let trimmed = input.trim();
    if !trimmed.starts_with('/') {
        return None;
    }

    let mut parts = trimmed.split_whitespace();
    let raw_name = parts.next()?;
    let name = raw_name.trim_start_matches('/').trim();

    if name.is_empty() {
        return None;
    }

    let args = parts.map(str::to_string).collect::<Vec<_>>();
    Some((name.to_ascii_lowercase(), args))
}

fn yaml_value_to_string(value: &serde_yaml::Value) -> Option<String> {
    match value {
        serde_yaml::Value::Null => None,
        serde_yaml::Value::Bool(value) => Some(value.to_string()),
        serde_yaml::Value::Number(value) => Some(value.to_string()),
        serde_yaml::Value::String(value) => Some(value.clone()),
        _ => serde_yaml::to_string(value)
            .ok()
            .map(|serialized| serialized.trim().to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn write_command(dir: &Path, file_name: &str, contents: &str) {
        std::fs::write(dir.join(file_name), contents).unwrap();
    }

    #[test]
    fn test_load_and_dispatch_markdown_command_with_substitution() {
        let temp = TempDir::new().unwrap();
        write_command(
            temp.path(),
            "greet.md",
            r#"---
name: greet
description: Say hello
usage: /greet <name>
parameters:
  - name: name
    required: true
---
Hello {name}! Raw args: {args}
"#,
        );

        let runtime = MarkdownCommandRuntime::load_from_dir(temp.path()).unwrap();
        let response = runtime.dispatch_from_slash_message("/greet Alice").unwrap();

        assert!(response.contains("Hello Alice!"));
        assert!(response.contains("Usage: /greet <name>"));
    }

    #[test]
    fn test_dispatch_reports_missing_required_parameter_usage() {
        let temp = TempDir::new().unwrap();
        write_command(
            temp.path(),
            "deploy.md",
            r#"---
name: deploy
usage: /deploy <env>
parameters:
  - name: env
    required: true
---
Deploying to {env}
"#,
        );

        let runtime = MarkdownCommandRuntime::load_from_dir(temp.path()).unwrap();
        let response = runtime.dispatch_from_slash_message("/deploy").unwrap();

        assert!(response.contains("Missing required argument(s): env."));
        assert!(response.contains("Usage: /deploy <env>"));
    }

    #[test]
    fn test_dispatch_supports_alias_and_default_value() {
        let temp = TempDir::new().unwrap();
        write_command(
            temp.path(),
            "hello.md",
            r#"---
name: hello
aliases: [hi]
usage: /hello [name]
parameters:
  - name: name
    required: false
    default: friend
---
Hello {name}
"#,
        );

        let runtime = MarkdownCommandRuntime::load_from_dir(temp.path()).unwrap();
        let response = runtime.dispatch_from_slash_message("/hi").unwrap();

        assert!(response.contains("Hello friend"));
    }

    #[test]
    fn test_dispatch_uses_zero_based_arg_placeholders() {
        let temp = TempDir::new().unwrap();
        write_command(
            temp.path(),
            "echo.md",
            r#"---
name: echo
usage: /echo <first> <second>
---
first={arg0}, second={arg1}, positional={2}
"#,
        );

        let runtime = MarkdownCommandRuntime::load_from_dir(temp.path()).unwrap();
        let response = runtime
            .dispatch_from_slash_message("/echo alpha beta")
            .unwrap();

        assert!(response.contains("first=alpha"));
        assert!(response.contains("second=beta"));
        assert!(response.contains("positional=beta"));
    }
}
