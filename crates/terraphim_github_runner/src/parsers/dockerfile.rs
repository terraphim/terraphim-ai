//! Parser for Dockerfiles
//!
//! Extracts instructions, stages, ENV variables, and commands from Dockerfiles.

use crate::{BuildTerm, RunnerError, RunnerResult, TermType};
use super::BuildFileParser;
use uuid::Uuid;

/// Parser for Dockerfiles
pub struct DockerfileParser {
    /// Base URL for generated term references
    base_url: String,
}

impl DockerfileParser {
    /// Create a new Dockerfile parser
    pub fn new(base_url: &str) -> Self {
        Self {
            base_url: base_url.to_string(),
        }
    }

    /// Extract instructions from Dockerfile content
    fn extract_instructions(&self, content: &str) -> Vec<BuildTerm> {
        let mut terms = Vec::new();
        let mut current_stage: Option<String> = None;
        let mut line_continuation = String::new();

        for line in content.lines() {
            let trimmed = line.trim();

            // Handle line continuation
            if trimmed.ends_with('\\') {
                line_continuation.push_str(trimmed.trim_end_matches('\\'));
                line_continuation.push(' ');
                continue;
            }

            let full_line = if !line_continuation.is_empty() {
                let result = format!("{}{}", line_continuation, trimmed);
                line_continuation.clear();
                result
            } else {
                trimmed.to_string()
            };

            let trimmed = full_line.trim();

            // Skip comments and empty lines
            if trimmed.starts_with('#') || trimmed.is_empty() {
                continue;
            }

            // FROM instruction (may start a new stage)
            if trimmed.to_uppercase().starts_with("FROM ") {
                let rest = &trimmed[5..].trim();
                let parts: Vec<&str> = rest.split_whitespace().collect();
                let image = parts.first().unwrap_or(&"");

                // Check for AS clause (named stage)
                let stage_name = if let Some(as_idx) = parts.iter().position(|&p| p.to_uppercase() == "AS") {
                    parts.get(as_idx + 1).map(|s| s.to_string())
                } else {
                    None
                };

                current_stage = stage_name.clone();

                terms.push(BuildTerm {
                    id: Uuid::new_v4().to_string(),
                    nterm: format!("FROM {}", image),
                    url: format!("{}#from-{}", self.base_url, image.replace([':', '/'], "-")),
                    term_type: TermType::DockerInstruction,
                    parent: stage_name.clone(),
                    related: Vec::new(),
                });

                if let Some(stage) = stage_name {
                    terms.push(BuildTerm {
                        id: Uuid::new_v4().to_string(),
                        nterm: format!("stage:{}", stage),
                        url: format!("{}#stage-{}", self.base_url, stage),
                        term_type: TermType::DockerInstruction,
                        parent: None,
                        related: Vec::new(),
                    });
                }
                continue;
            }

            // RUN instruction
            if trimmed.to_uppercase().starts_with("RUN ") {
                let cmd = &trimmed[4..].trim();
                // Extract first command
                let first_cmd = cmd.split("&&").next().unwrap_or(cmd).trim();
                let cmd_name = first_cmd.split_whitespace().next().unwrap_or(first_cmd);

                terms.push(BuildTerm {
                    id: Uuid::new_v4().to_string(),
                    nterm: format!("RUN {}", cmd_name),
                    url: format!("{}#run-{}", self.base_url, cmd_name),
                    term_type: TermType::Command,
                    parent: current_stage.clone(),
                    related: Vec::new(),
                });
                continue;
            }

            // ENV instruction
            if trimmed.to_uppercase().starts_with("ENV ") {
                let rest = &trimmed[4..].trim();
                // ENV can be KEY=VALUE or KEY VALUE
                let key = if rest.contains('=') {
                    rest.split('=').next().unwrap_or("").trim()
                } else {
                    rest.split_whitespace().next().unwrap_or("").trim()
                };

                if !key.is_empty() {
                    terms.push(BuildTerm {
                        id: Uuid::new_v4().to_string(),
                        nterm: format!("ENV {}", key),
                        url: format!("{}#env-{}", self.base_url, key),
                        term_type: TermType::EnvVar,
                        parent: current_stage.clone(),
                        related: Vec::new(),
                    });
                }
                continue;
            }

            // ARG instruction
            if trimmed.to_uppercase().starts_with("ARG ") {
                let rest = &trimmed[4..].trim();
                let key = rest.split('=').next().unwrap_or(rest).trim();

                if !key.is_empty() {
                    terms.push(BuildTerm {
                        id: Uuid::new_v4().to_string(),
                        nterm: format!("ARG {}", key),
                        url: format!("{}#arg-{}", self.base_url, key),
                        term_type: TermType::EnvVar,
                        parent: current_stage.clone(),
                        related: Vec::new(),
                    });
                }
                continue;
            }

            // COPY instruction
            if trimmed.to_uppercase().starts_with("COPY ") {
                let rest = &trimmed[5..].trim();

                // Check for --from flag (copy from another stage)
                if rest.starts_with("--from=") {
                    let from_stage = rest
                        .split_whitespace()
                        .next()
                        .unwrap_or("")
                        .strip_prefix("--from=")
                        .unwrap_or("");

                    terms.push(BuildTerm {
                        id: Uuid::new_v4().to_string(),
                        nterm: format!("COPY --from={}", from_stage),
                        url: format!("{}#copy-from-{}", self.base_url, from_stage),
                        term_type: TermType::DockerInstruction,
                        parent: current_stage.clone(),
                        related: vec![format!("stage:{}", from_stage)],
                    });
                } else {
                    terms.push(BuildTerm {
                        id: Uuid::new_v4().to_string(),
                        nterm: "COPY".to_string(),
                        url: format!("{}#copy", self.base_url),
                        term_type: TermType::DockerInstruction,
                        parent: current_stage.clone(),
                        related: Vec::new(),
                    });
                }
                continue;
            }

            // WORKDIR instruction
            if trimmed.to_uppercase().starts_with("WORKDIR ") {
                let dir = &trimmed[8..].trim();
                terms.push(BuildTerm {
                    id: Uuid::new_v4().to_string(),
                    nterm: format!("WORKDIR {}", dir),
                    url: format!("{}#workdir", self.base_url),
                    term_type: TermType::DockerInstruction,
                    parent: current_stage.clone(),
                    related: Vec::new(),
                });
                continue;
            }

            // EXPOSE instruction
            if trimmed.to_uppercase().starts_with("EXPOSE ") {
                let port = &trimmed[7..].trim();
                terms.push(BuildTerm {
                    id: Uuid::new_v4().to_string(),
                    nterm: format!("EXPOSE {}", port),
                    url: format!("{}#expose-{}", self.base_url, port),
                    term_type: TermType::Service,
                    parent: current_stage.clone(),
                    related: Vec::new(),
                });
                continue;
            }

            // ENTRYPOINT instruction
            if trimmed.to_uppercase().starts_with("ENTRYPOINT ") {
                let entry = &trimmed[11..].trim();
                terms.push(BuildTerm {
                    id: Uuid::new_v4().to_string(),
                    nterm: format!("ENTRYPOINT {}", entry),
                    url: format!("{}#entrypoint", self.base_url),
                    term_type: TermType::Command,
                    parent: current_stage.clone(),
                    related: Vec::new(),
                });
                continue;
            }

            // CMD instruction
            if trimmed.to_uppercase().starts_with("CMD ") {
                let cmd = &trimmed[4..].trim();
                terms.push(BuildTerm {
                    id: Uuid::new_v4().to_string(),
                    nterm: format!("CMD {}", cmd),
                    url: format!("{}#cmd", self.base_url),
                    term_type: TermType::Command,
                    parent: current_stage.clone(),
                    related: Vec::new(),
                });
            }
        }

        terms
    }
}

impl BuildFileParser for DockerfileParser {
    fn parse(&self, content: &str) -> RunnerResult<Vec<BuildTerm>> {
        if content.trim().is_empty() {
            return Err(RunnerError::DockerfileParsing("Empty Dockerfile".to_string()));
        }
        Ok(self.extract_instructions(content))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_dockerfile() {
        let content = r#"
FROM rust:1.75 AS builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
COPY --from=builder /app/target/release/myapp /usr/local/bin/
ENV RUST_LOG=info
EXPOSE 8080
ENTRYPOINT ["myapp"]
"#;

        let parser = DockerfileParser::new("https://example.com/Dockerfile");
        let terms = parser.parse(content).unwrap();

        assert!(!terms.is_empty());

        // Check for stages
        assert!(terms.iter().any(|t| t.nterm == "stage:builder"));

        // Check for FROM
        assert!(terms.iter().any(|t| t.nterm.starts_with("FROM rust")));

        // Check for COPY --from
        assert!(terms.iter().any(|t| t.nterm == "COPY --from=builder"));

        // Check for ENV
        assert!(terms.iter().any(|t| t.nterm == "ENV RUST_LOG"));

        // Check for EXPOSE
        assert!(terms.iter().any(|t| t.nterm == "EXPOSE 8080"));
    }

    #[test]
    fn test_parse_multi_line_run() {
        let content = r#"
FROM ubuntu:22.04
RUN apt-get update && \
    apt-get install -y build-essential && \
    rm -rf /var/lib/apt/lists/*
"#;

        let parser = DockerfileParser::new("https://example.com/Dockerfile");
        let terms = parser.parse(content).unwrap();

        // Should extract the first command (apt-get)
        assert!(terms.iter().any(|t| t.nterm == "RUN apt-get"));
    }
}
