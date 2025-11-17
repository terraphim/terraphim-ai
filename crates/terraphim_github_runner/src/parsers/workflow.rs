//! Parser for GitHub Actions workflow files
//!
//! Extracts jobs, steps, actions, and configuration from workflow YAML files.

use crate::{BuildTerm, RunnerError, RunnerResult, TermType, Workflow};
use super::BuildFileParser;
use uuid::Uuid;

/// Parser for GitHub Actions workflow files
pub struct WorkflowParser {
    /// Base URL for generated term references
    base_url: String,
}

impl WorkflowParser {
    /// Create a new workflow parser
    pub fn new(base_url: &str) -> Self {
        Self {
            base_url: base_url.to_string(),
        }
    }

    /// Parse workflow and extract structured data
    pub fn parse_workflow(&self, content: &str) -> RunnerResult<Workflow> {
        serde_yaml::from_str(content).map_err(|e| {
            RunnerError::WorkflowParsing(format!("Failed to parse workflow: {}", e))
        })
    }

    /// Extract terms from workflow content
    fn extract_terms(&self, content: &str) -> RunnerResult<Vec<BuildTerm>> {
        let workflow = self.parse_workflow(content)?;
        let mut terms = Vec::new();

        // Workflow name
        terms.push(BuildTerm {
            id: Uuid::new_v4().to_string(),
            nterm: format!("workflow:{}", workflow.name),
            url: format!("{}#workflow", self.base_url),
            term_type: TermType::Action,
            parent: None,
            related: Vec::new(),
        });

        // Process each job
        for (job_id, job) in &workflow.jobs {
            let job_term_id = Uuid::new_v4().to_string();

            // Job entry
            terms.push(BuildTerm {
                id: job_term_id.clone(),
                nterm: format!("job:{}", job_id),
                url: format!("{}#job-{}", self.base_url, job_id),
                term_type: TermType::Action,
                parent: Some(format!("workflow:{}", workflow.name)),
                related: job.needs.clone().unwrap_or_default(),
            });

            // Process each step
            for (idx, step) in job.steps.iter().enumerate() {
                let step_id = step.id.clone().unwrap_or_else(|| format!("step-{}", idx));

                // Action usage
                if let Some(uses) = &step.uses {
                    let (action_name, action_version) = parse_action_ref(uses);

                    terms.push(BuildTerm {
                        id: Uuid::new_v4().to_string(),
                        nterm: format!("uses:{}", action_name),
                        url: format!("https://github.com/{}", action_name),
                        term_type: TermType::Action,
                        parent: Some(format!("job:{}", job_id)),
                        related: vec![format!("version:{}", action_version)],
                    });

                    // Extract well-known action patterns
                    if let Some(with) = &step.with {
                        for (key, value) in with {
                            terms.push(BuildTerm {
                                id: Uuid::new_v4().to_string(),
                                nterm: format!("{}:{}", action_name, key),
                                url: format!("{}#step-{}-{}", self.base_url, step_id, key),
                                term_type: TermType::EnvVar,
                                parent: Some(format!("uses:{}", action_name)),
                                related: Vec::new(),
                            });

                            // Special handling for common action inputs
                            let value_str = match value {
                                serde_yaml::Value::String(s) => Some(s.clone()),
                                serde_yaml::Value::Number(n) => Some(n.to_string()),
                                serde_yaml::Value::Bool(b) => Some(b.to_string()),
                                _ => None,
                            };

                            if let Some(val) = value_str {
                                // Extract meaningful values
                                if key == "node-version" || key == "python-version" || key == "go-version" {
                                    terms.push(BuildTerm {
                                        id: Uuid::new_v4().to_string(),
                                        nterm: format!("runtime:{}={}", key.replace("-version", ""), val),
                                        url: format!("{}#runtime", self.base_url),
                                        term_type: TermType::EnvVar,
                                        parent: Some(format!("uses:{}", action_name)),
                                        related: Vec::new(),
                                    });
                                }
                            }
                        }
                    }
                }

                // Shell command
                if let Some(run) = &step.run {
                    // Extract first command from each line
                    for cmd_line in run.lines() {
                        let trimmed = cmd_line.trim();
                        if trimmed.is_empty() || trimmed.starts_with('#') {
                            continue;
                        }

                        let first_cmd = trimmed
                            .split("&&")
                            .next()
                            .unwrap_or(trimmed)
                            .trim()
                            .split_whitespace()
                            .next()
                            .unwrap_or(trimmed);

                        // Skip common shell constructs
                        if !["if", "then", "else", "fi", "for", "do", "done", "case", "esac"]
                            .contains(&first_cmd)
                        {
                            terms.push(BuildTerm {
                                id: Uuid::new_v4().to_string(),
                                nterm: format!("run:{}", first_cmd),
                                url: format!("{}#step-{}", self.base_url, step_id),
                                term_type: TermType::Command,
                                parent: Some(format!("job:{}", job_id)),
                                related: Vec::new(),
                            });
                        }
                    }
                }

                // Environment variables
                if let Some(env) = &step.env {
                    for key in env.keys() {
                        terms.push(BuildTerm {
                            id: Uuid::new_v4().to_string(),
                            nterm: format!("env:{}", key),
                            url: format!("{}#step-{}-env", self.base_url, step_id),
                            term_type: TermType::EnvVar,
                            parent: Some(format!("job:{}", job_id)),
                            related: Vec::new(),
                        });
                    }
                }
            }

            // Services
            if let Some(services) = &job.services {
                for (service_id, service) in services {
                    terms.push(BuildTerm {
                        id: Uuid::new_v4().to_string(),
                        nterm: format!("service:{}", service_id),
                        url: format!("{}#service-{}", self.base_url, service_id),
                        term_type: TermType::Service,
                        parent: Some(format!("job:{}", job_id)),
                        related: vec![service.image.clone()],
                    });
                }
            }
        }

        Ok(terms)
    }
}

/// Parse an action reference into name and version
fn parse_action_ref(uses: &str) -> (String, String) {
    // Handle different formats:
    // - owner/repo@version
    // - owner/repo/path@version
    // - docker://image:tag
    // - ./local/path

    if uses.starts_with("docker://") {
        let image = uses.strip_prefix("docker://").unwrap();
        let parts: Vec<&str> = image.split(':').collect();
        let name = parts.first().unwrap_or(&image);
        let version = parts.get(1).unwrap_or(&"latest");
        return (format!("docker/{}", name), version.to_string());
    }

    if uses.starts_with("./") || uses.starts_with("../") {
        return (uses.to_string(), "local".to_string());
    }

    // Standard action reference
    let parts: Vec<&str> = uses.split('@').collect();
    let name = parts.first().unwrap_or(&uses);
    let version = parts.get(1).unwrap_or(&"main");

    (name.to_string(), version.to_string())
}

impl BuildFileParser for WorkflowParser {
    fn parse(&self, content: &str) -> RunnerResult<Vec<BuildTerm>> {
        if content.trim().is_empty() {
            return Err(RunnerError::WorkflowParsing("Empty workflow file".to_string()));
        }
        self.extract_terms(content)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_workflow() {
        let content = r#"
name: CI

on:
  push:
    branches: [main]
  pull_request:

jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Setup Node.js
        uses: actions/setup-node@v4
        with:
          node-version: '20'
      - run: npm ci
      - run: npm test
"#;

        let parser = WorkflowParser::new("https://example.com/.github/workflows/ci.yml");
        let terms = parser.parse(content).unwrap();

        assert!(!terms.is_empty());

        // Check for workflow
        assert!(terms.iter().any(|t| t.nterm == "workflow:CI"));

        // Check for job
        assert!(terms.iter().any(|t| t.nterm == "job:build"));

        // Check for actions
        assert!(terms.iter().any(|t| t.nterm == "uses:actions/checkout"));
        assert!(terms.iter().any(|t| t.nterm == "uses:actions/setup-node"));

        // Check for commands
        assert!(terms.iter().any(|t| t.nterm == "run:npm"));

        // Check for runtime version
        assert!(terms.iter().any(|t| t.nterm == "runtime:node=20"));
    }

    #[test]
    fn test_parse_action_ref() {
        let (name, version) = parse_action_ref("actions/checkout@v4");
        assert_eq!(name, "actions/checkout");
        assert_eq!(version, "v4");

        let (name, version) = parse_action_ref("docker://postgres:14");
        assert_eq!(name, "docker/postgres");
        assert_eq!(version, "14");

        let (name, version) = parse_action_ref("./local/action");
        assert_eq!(name, "./local/action");
        assert_eq!(version, "local");
    }
}
