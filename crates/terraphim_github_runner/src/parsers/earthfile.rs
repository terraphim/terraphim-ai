//! Parser for Earthfiles
//!
//! Extracts targets, ARGs, FROMs, and commands from Earthly build files.

use crate::{BuildTerm, RunnerError, RunnerResult, TermType};
use super::BuildFileParser;
use uuid::Uuid;

/// Parser for Earthly Earthfiles
pub struct EarthfileParser {
    /// Base URL for generated term references
    base_url: String,
}

impl EarthfileParser {
    /// Create a new Earthfile parser
    pub fn new(base_url: &str) -> Self {
        Self {
            base_url: base_url.to_string(),
        }
    }

    /// Extract targets from Earthfile content
    fn extract_targets(&self, content: &str) -> Vec<BuildTerm> {
        let mut terms = Vec::new();
        let mut current_target: Option<String> = None;

        for line in content.lines() {
            let trimmed = line.trim();

            // Target definition (starts with identifier followed by colon)
            if !trimmed.starts_with('#')
                && !trimmed.is_empty()
                && trimmed.ends_with(':')
                && !trimmed.contains(' ')
            {
                let target_name = trimmed.trim_end_matches(':');
                if !target_name.is_empty() && target_name.chars().next().map(|c| c.is_alphabetic() || c == '_').unwrap_or(false) {
                    current_target = Some(target_name.to_string());
                    terms.push(BuildTerm {
                        id: Uuid::new_v4().to_string(),
                        nterm: format!("+{}", target_name),
                        url: format!("{}#target-{}", self.base_url, target_name),
                        term_type: TermType::EarthlyTarget,
                        parent: None,
                        related: Vec::new(),
                    });
                }
            }

            // ARG definitions
            if trimmed.starts_with("ARG ") {
                let arg_part = trimmed.strip_prefix("ARG ").unwrap();
                let arg_name = arg_part.split('=').next().unwrap_or(arg_part).trim();
                if !arg_name.is_empty() {
                    terms.push(BuildTerm {
                        id: Uuid::new_v4().to_string(),
                        nterm: format!("ARG {}", arg_name),
                        url: format!("{}#arg-{}", self.base_url, arg_name),
                        term_type: TermType::EnvVar,
                        parent: current_target.clone(),
                        related: Vec::new(),
                    });
                }
            }

            // FROM instructions
            if trimmed.starts_with("FROM ") {
                let from_part = trimmed.strip_prefix("FROM ").unwrap().trim();
                // Handle FROM +target syntax
                if from_part.starts_with('+') {
                    let target_ref = from_part.split_whitespace().next().unwrap_or(from_part);
                    terms.push(BuildTerm {
                        id: Uuid::new_v4().to_string(),
                        nterm: format!("FROM {}", target_ref),
                        url: format!("{}#from-{}", self.base_url, target_ref.trim_start_matches('+')),
                        term_type: TermType::EarthlyTarget,
                        parent: current_target.clone(),
                        related: vec![target_ref.to_string()],
                    });
                } else {
                    // Regular Docker image
                    let image = from_part.split_whitespace().next().unwrap_or(from_part);
                    terms.push(BuildTerm {
                        id: Uuid::new_v4().to_string(),
                        nterm: format!("FROM {}", image),
                        url: format!("{}#from-{}", self.base_url, image.replace([':', '/'], "-")),
                        term_type: TermType::DockerInstruction,
                        parent: current_target.clone(),
                        related: Vec::new(),
                    });
                }
            }

            // RUN commands
            if trimmed.starts_with("RUN ") {
                let cmd = trimmed.strip_prefix("RUN ").unwrap().trim();
                // Extract the first command (before &&, ||, etc.)
                let first_cmd = cmd.split("&&").next().unwrap_or(cmd).trim();
                let cmd_name = first_cmd.split_whitespace().next().unwrap_or(first_cmd);

                terms.push(BuildTerm {
                    id: Uuid::new_v4().to_string(),
                    nterm: format!("RUN {}", cmd_name),
                    url: format!("{}#run-{}", self.base_url, cmd_name),
                    term_type: TermType::Command,
                    parent: current_target.clone(),
                    related: Vec::new(),
                });
            }

            // SAVE ARTIFACT
            if trimmed.starts_with("SAVE ARTIFACT ") {
                let artifact = trimmed.strip_prefix("SAVE ARTIFACT ").unwrap().trim();
                let artifact_path = artifact.split_whitespace().next().unwrap_or(artifact);

                terms.push(BuildTerm {
                    id: Uuid::new_v4().to_string(),
                    nterm: format!("SAVE ARTIFACT {}", artifact_path),
                    url: format!("{}#artifact-{}", self.base_url, artifact_path.replace('/', "-")),
                    term_type: TermType::Artifact,
                    parent: current_target.clone(),
                    related: Vec::new(),
                });
            }

            // SAVE IMAGE
            if trimmed.starts_with("SAVE IMAGE ") {
                let image = trimmed.strip_prefix("SAVE IMAGE ").unwrap().trim();
                let image_name = image.split_whitespace().next().unwrap_or(image);

                terms.push(BuildTerm {
                    id: Uuid::new_v4().to_string(),
                    nterm: format!("SAVE IMAGE {}", image_name),
                    url: format!("{}#image-{}", self.base_url, image_name.replace([':', '/'], "-")),
                    term_type: TermType::Artifact,
                    parent: current_target.clone(),
                    related: Vec::new(),
                });
            }
        }

        terms
    }
}

impl BuildFileParser for EarthfileParser {
    fn parse(&self, content: &str) -> RunnerResult<Vec<BuildTerm>> {
        if content.trim().is_empty() {
            return Err(RunnerError::EarthfileParsing("Empty Earthfile".to_string()));
        }
        Ok(self.extract_targets(content))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_earthfile() {
        let content = r#"
VERSION 0.8

build:
    FROM rust:1.75
    COPY src src
    RUN cargo build --release
    SAVE ARTIFACT target/release/myapp

test:
    FROM +build
    RUN cargo test
"#;

        let parser = EarthfileParser::new("https://example.com/Earthfile");
        let terms = parser.parse(content).unwrap();

        // Should find targets, FROM, RUN, and SAVE ARTIFACT
        assert!(!terms.is_empty());

        // Check for build target
        assert!(terms.iter().any(|t| t.nterm == "+build"));

        // Check for test target
        assert!(terms.iter().any(|t| t.nterm == "+test"));

        // Check for FROM commands
        assert!(terms.iter().any(|t| t.nterm.starts_with("FROM ")));

        // Check for RUN commands
        assert!(terms.iter().any(|t| t.nterm.starts_with("RUN ")));
    }
}
