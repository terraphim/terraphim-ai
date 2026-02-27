//! Markdown skill parser with YAML frontmatter support.
//!
//! Parsing follows the same pattern used in Terraphim markdown loaders:
//! a leading `---` frontmatter block parsed by `serde_yaml`.

use super::types::{Skill, SkillInput, SkillStep};
use serde::Deserialize;
use serde_json::{Map, Value};
use std::path::Path;
use thiserror::Error;

/// Errors returned while parsing markdown-defined skills.
#[derive(Debug, Error)]
pub enum MarkdownSkillParseError {
    #[error("No YAML frontmatter found; markdown must start with `---`")]
    MissingFrontmatter,
    #[error("Unclosed YAML frontmatter; missing closing `---` delimiter")]
    UnclosedFrontmatter,
    #[error("Invalid frontmatter at line {line}: {message}")]
    InvalidFrontmatter { line: usize, message: String },
    #[error("Missing required field `{field}`")]
    MissingRequiredField { field: &'static str },
    #[error("Invalid field `{field}`: {message}")]
    InvalidField {
        field: &'static str,
        message: String,
    },
    #[error("Invalid step #{step}: {message}")]
    InvalidStep { step: usize, message: String },
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Debug, Deserialize)]
struct Frontmatter {
    name: Option<String>,
    version: Option<String>,
    description: Option<String>,
    #[serde(default)]
    author: Option<String>,
    steps: Option<Vec<FrontmatterStep>>,
    #[serde(default)]
    inputs: Option<Vec<FrontmatterInput>>,
}

#[derive(Debug, Deserialize)]
struct FrontmatterStep {
    #[serde(rename = "type")]
    step_type: String,
    #[serde(default)]
    tool: Option<String>,
    #[serde(default)]
    args: Option<serde_yaml::Value>,
    #[serde(default)]
    prompt: Option<String>,
    #[serde(default)]
    use_context: Option<bool>,
    #[serde(default)]
    command: Option<String>,
    #[serde(default)]
    working_dir: Option<String>,
}

#[derive(Debug, Deserialize)]
struct FrontmatterInput {
    name: Option<String>,
    description: Option<String>,
    #[serde(default)]
    required: Option<bool>,
    #[serde(default)]
    default: Option<serde_yaml::Value>,
}

/// Parse a markdown document containing YAML frontmatter into a [`Skill`].
pub fn parse_markdown_skill(content: &str) -> Result<Skill, MarkdownSkillParseError> {
    let frontmatter = extract_frontmatter(content)?;

    let parsed: Frontmatter = serde_yaml::from_str(&frontmatter).map_err(|error| {
        let line = error.location().map(|loc| loc.line()).unwrap_or(0);
        MarkdownSkillParseError::InvalidFrontmatter {
            line,
            message: error.to_string(),
        }
    })?;

    let name = required_field(parsed.name, "name")?;
    let version = required_field(parsed.version, "version")?;
    let description = required_field(parsed.description, "description")?;

    let raw_steps = parsed
        .steps
        .ok_or(MarkdownSkillParseError::MissingRequiredField { field: "steps" })?;

    let steps = raw_steps
        .into_iter()
        .enumerate()
        .map(|(idx, step)| convert_step(step, idx + 1))
        .collect::<Result<Vec<_>, _>>()?;

    let inputs = parsed
        .inputs
        .unwrap_or_default()
        .into_iter()
        .map(convert_input)
        .collect::<Result<Vec<_>, _>>()?;

    Ok(Skill {
        name,
        version,
        description,
        author: parsed.author.and_then(normalize_optional),
        steps,
        inputs,
    })
}

/// Read a markdown file and parse YAML frontmatter into a [`Skill`].
pub fn parse_markdown_skill_file(path: impl AsRef<Path>) -> Result<Skill, MarkdownSkillParseError> {
    let content = std::fs::read_to_string(path)?;
    parse_markdown_skill(&content)
}

fn extract_frontmatter(content: &str) -> Result<String, MarkdownSkillParseError> {
    let mut lines = content.lines();
    let Some(first_line) = lines.next() else {
        return Err(MarkdownSkillParseError::MissingFrontmatter);
    };

    if first_line.trim() != "---" {
        return Err(MarkdownSkillParseError::MissingFrontmatter);
    }

    let mut frontmatter_lines = Vec::new();
    for line in lines {
        if line.trim() == "---" {
            return Ok(frontmatter_lines.join("\n"));
        }
        frontmatter_lines.push(line);
    }

    Err(MarkdownSkillParseError::UnclosedFrontmatter)
}

fn convert_step(
    step: FrontmatterStep,
    step_number: usize,
) -> Result<SkillStep, MarkdownSkillParseError> {
    let step_type =
        normalize_optional(step.step_type).ok_or(MarkdownSkillParseError::InvalidStep {
            step: step_number,
            message: "missing required `type`".to_string(),
        })?;

    match step_type.as_str() {
        "tool" => {
            let tool = normalize_required(
                step.tool,
                MarkdownSkillParseError::InvalidStep {
                    step: step_number,
                    message: "tool step requires `tool`".to_string(),
                },
            )?;

            let args = match step.args {
                Some(yaml) => serde_json::to_value(yaml).map_err(|error| {
                    MarkdownSkillParseError::InvalidStep {
                        step: step_number,
                        message: format!("invalid `args`: {}", error),
                    }
                })?,
                None => Value::Object(Map::new()),
            };

            Ok(SkillStep::Tool { tool, args })
        }
        "llm" => {
            let prompt = normalize_required(
                step.prompt,
                MarkdownSkillParseError::InvalidStep {
                    step: step_number,
                    message: "llm step requires `prompt`".to_string(),
                },
            )?;

            Ok(SkillStep::Llm {
                prompt,
                use_context: step.use_context.unwrap_or(true),
            })
        }
        "shell" => {
            let command = normalize_required(
                step.command,
                MarkdownSkillParseError::InvalidStep {
                    step: step_number,
                    message: "shell step requires `command`".to_string(),
                },
            )?;

            Ok(SkillStep::Shell {
                command,
                working_dir: step.working_dir.and_then(normalize_optional),
            })
        }
        other => Err(MarkdownSkillParseError::InvalidStep {
            step: step_number,
            message: format!("unknown step type `{}`", other),
        }),
    }
}

fn convert_input(input: FrontmatterInput) -> Result<SkillInput, MarkdownSkillParseError> {
    let name = normalize_required(
        input.name,
        MarkdownSkillParseError::InvalidField {
            field: "inputs",
            message: "each input requires `name`".to_string(),
        },
    )?;

    let description = normalize_required(
        input.description,
        MarkdownSkillParseError::InvalidField {
            field: "inputs",
            message: "each input requires `description`".to_string(),
        },
    )?;

    Ok(SkillInput {
        name,
        description,
        required: input.required.unwrap_or(true),
        default: input.default.as_ref().and_then(yaml_value_to_string),
    })
}

fn required_field(
    value: Option<String>,
    field: &'static str,
) -> Result<String, MarkdownSkillParseError> {
    normalize_required(
        value,
        MarkdownSkillParseError::MissingRequiredField { field },
    )
}

fn normalize_required(
    value: Option<String>,
    error: MarkdownSkillParseError,
) -> Result<String, MarkdownSkillParseError> {
    value.and_then(normalize_optional).ok_or(error)
}

fn normalize_optional(value: String) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
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

    #[test]
    fn test_parse_markdown_skill_valid_frontmatter() {
        let content = r#"---
name: repo-audit
version: "1.0.0"
description: Analyze repository risk
author: TinyClaw
steps:
  - type: shell
    command: echo "scan"
  - type: llm
    prompt: Summarize findings
    use_context: false
  - type: tool
    tool: filesystem
    args:
      operation: read_file
      path: /tmp/README.md
inputs:
  - name: repo_path
    description: Repository path
    required: true
---
# Repo Audit
"#;

        let skill = parse_markdown_skill(content).expect("valid markdown skill should parse");

        assert_eq!(skill.name, "repo-audit");
        assert_eq!(skill.version, "1.0.0");
        assert_eq!(skill.description, "Analyze repository risk");
        assert_eq!(skill.author, Some("TinyClaw".to_string()));
        assert_eq!(skill.steps.len(), 3);
        assert_eq!(skill.inputs.len(), 1);

        match &skill.steps[0] {
            SkillStep::Shell {
                command,
                working_dir,
            } => {
                assert_eq!(command, "echo \"scan\"");
                assert_eq!(working_dir, &None);
            }
            other => panic!("expected shell step, got: {other:?}"),
        }

        match &skill.steps[1] {
            SkillStep::Llm {
                prompt,
                use_context,
            } => {
                assert_eq!(prompt, "Summarize findings");
                assert!(!use_context);
            }
            other => panic!("expected llm step, got: {other:?}"),
        }

        match &skill.steps[2] {
            SkillStep::Tool { tool, args } => {
                assert_eq!(tool, "filesystem");
                assert_eq!(args["operation"], "read_file");
                assert_eq!(args["path"], "/tmp/README.md");
            }
            other => panic!("expected tool step, got: {other:?}"),
        }

        assert_eq!(skill.inputs[0].name, "repo_path");
        assert_eq!(skill.inputs[0].description, "Repository path");
        assert!(skill.inputs[0].required);
    }

    #[test]
    fn test_parse_markdown_skill_invalid_frontmatter() {
        let content = r#"---
name repo-audit
version: 1.0.0
description: Missing colon on first line
steps:
  - type: shell
    command: echo ok
---
Body
"#;

        let result = parse_markdown_skill(content);
        assert!(result.is_err(), "expected invalid frontmatter error");
        match result.unwrap_err() {
            MarkdownSkillParseError::InvalidFrontmatter { .. } => {}
            other => panic!("expected InvalidFrontmatter, got: {other:?}"),
        }
    }

    #[test]
    fn test_parse_markdown_skill_missing_required_fields() {
        let content = r#"---
version: 1.0.0
description: Missing name
steps:
  - type: shell
    command: echo ok
---
Body
"#;

        let result = parse_markdown_skill(content);
        assert!(result.is_err(), "expected missing required field error");
        match result.unwrap_err() {
            MarkdownSkillParseError::MissingRequiredField { field } => assert_eq!(field, "name"),
            other => panic!("expected MissingRequiredField, got: {other:?}"),
        }
    }
}
