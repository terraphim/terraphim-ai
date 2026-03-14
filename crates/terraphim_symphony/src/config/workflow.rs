//! WORKFLOW.md parser.
//!
//! Parses a Markdown file with optional YAML front matter delimited by `---`.
//! Returns the parsed config map and the prompt template body.

use crate::error::{Result, SymphonyError};

/// Parsed workflow definition from `WORKFLOW.md`.
#[derive(Debug, Clone)]
pub struct WorkflowDefinition {
    /// YAML front matter as a mapping (empty if no front matter).
    pub config: serde_yaml::Mapping,
    /// Markdown body after front matter, trimmed.
    pub prompt_template: String,
}

impl WorkflowDefinition {
    /// Load and parse a workflow file from the given path.
    pub fn load(path: &std::path::Path) -> Result<Self> {
        let content = std::fs::read_to_string(path).map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                SymphonyError::MissingWorkflowFile {
                    path: path.display().to_string(),
                }
            } else {
                SymphonyError::Io(e)
            }
        })?;
        Self::parse(&content)
    }

    /// Parse workflow content from a string.
    pub fn parse(content: &str) -> Result<Self> {
        let (front_matter_str, body) = split_front_matter(content);

        let config = if let Some(yaml_str) = front_matter_str {
            let value: serde_yaml::Value =
                serde_yaml::from_str(yaml_str).map_err(|e| SymphonyError::WorkflowParseError {
                    reason: e.to_string(),
                })?;

            match value {
                serde_yaml::Value::Mapping(m) => m,
                serde_yaml::Value::Null => serde_yaml::Mapping::new(),
                _ => return Err(SymphonyError::WorkflowFrontMatterNotAMap),
            }
        } else {
            serde_yaml::Mapping::new()
        };

        Ok(WorkflowDefinition {
            config,
            prompt_template: body.trim().to_string(),
        })
    }
}

/// Split content into optional YAML front matter and body.
///
/// Front matter is delimited by `---` on its own line at the start of the file.
fn split_front_matter(content: &str) -> (Option<&str>, &str) {
    let trimmed = content.trim_start();
    if !trimmed.starts_with("---") {
        return (None, content);
    }

    // Find the opening delimiter line end
    let after_first = match trimmed.strip_prefix("---") {
        Some(rest) => rest.trim_start_matches([' ', '\t']),
        None => return (None, content),
    };

    // Must have a newline after the opening ---
    let after_first = match after_first.strip_prefix('\n') {
        Some(rest) => rest,
        None => match after_first.strip_prefix("\r\n") {
            Some(rest) => rest,
            None => {
                // --- with no newline means the whole thing is body
                if after_first.is_empty() {
                    return (Some(""), "");
                }
                return (None, content);
            }
        },
    };

    // Find the closing ---
    if let Some(pos) = find_closing_delimiter(after_first) {
        let yaml = &after_first[..pos];
        let rest = &after_first[pos..];
        // Skip the closing --- line
        let body = rest
            .strip_prefix("---")
            .unwrap_or(rest)
            .trim_start_matches([' ', '\t']);
        let body = body
            .strip_prefix('\n')
            .or_else(|| body.strip_prefix("\r\n"))
            .unwrap_or(body);
        (Some(yaml), body)
    } else {
        // No closing delimiter: treat everything after opening as YAML, no body
        (Some(after_first), "")
    }
}

/// Find the position of a closing `---` delimiter on its own line.
fn find_closing_delimiter(s: &str) -> Option<usize> {
    let mut pos = 0;
    for line in s.lines() {
        let trimmed = line.trim();
        if trimmed == "---" {
            return Some(pos);
        }
        pos += line.len() + 1; // +1 for the newline
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_with_front_matter_and_body() {
        let content = r#"---
tracker:
  kind: linear
  project_slug: my-project
agent:
  max_concurrent_agents: 5
---
You are working on issue {{ issue.identifier }}: {{ issue.title }}.
"#;
        let def = WorkflowDefinition::parse(content).unwrap();
        assert!(!def.config.is_empty());

        let tracker = def.config.get("tracker").unwrap().as_mapping().unwrap();
        let kind = tracker.get("kind").unwrap().as_str().unwrap();
        assert_eq!(kind, "linear");

        assert!(def.prompt_template.contains("{{ issue.identifier }}"));
    }

    #[test]
    fn parse_without_front_matter() {
        let content = "Just a prompt template with no YAML.";
        let def = WorkflowDefinition::parse(content).unwrap();
        assert!(def.config.is_empty());
        assert_eq!(def.prompt_template, "Just a prompt template with no YAML.");
    }

    #[test]
    fn parse_empty_front_matter() {
        let content = "---\n---\nSome prompt.";
        let def = WorkflowDefinition::parse(content).unwrap();
        assert!(def.config.is_empty());
        assert_eq!(def.prompt_template, "Some prompt.");
    }

    #[test]
    fn parse_front_matter_not_a_map() {
        let content = "---\n- list\n- item\n---\nBody.";
        let result = WorkflowDefinition::parse(content);
        assert!(matches!(
            result,
            Err(SymphonyError::WorkflowFrontMatterNotAMap)
        ));
    }

    #[test]
    fn parse_invalid_yaml() {
        let content = "---\n: invalid: yaml: [broken\n---\nBody.";
        let result = WorkflowDefinition::parse(content);
        assert!(matches!(
            result,
            Err(SymphonyError::WorkflowParseError { .. })
        ));
    }

    #[test]
    fn parse_body_is_trimmed() {
        let content = "---\nkey: value\n---\n\n  Some body  \n\n";
        let def = WorkflowDefinition::parse(content).unwrap();
        assert_eq!(def.prompt_template, "Some body");
    }

    #[test]
    fn parse_no_body_after_front_matter() {
        let content = "---\nkey: value\n---";
        let def = WorkflowDefinition::parse(content).unwrap();
        assert_eq!(def.prompt_template, "");
    }

    #[test]
    fn load_missing_file_returns_typed_error() {
        let result = WorkflowDefinition::load(std::path::Path::new("/nonexistent/WORKFLOW.md"));
        assert!(matches!(
            result,
            Err(SymphonyError::MissingWorkflowFile { .. })
        ));
    }
}
