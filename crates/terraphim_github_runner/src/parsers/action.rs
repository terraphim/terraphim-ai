//! Parser for GitHub Action metadata files (action.yml)
//!
//! Extracts action inputs, outputs, and execution configuration.

use crate::{BuildTerm, RunnerError, RunnerResult, TermType};
use super::BuildFileParser;
use ahash::AHashMap;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Parser for GitHub Action metadata
pub struct ActionParser {
    /// Base URL for generated term references
    base_url: String,
}

/// Action metadata structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionMetadata {
    /// Action name
    pub name: String,
    /// Action description
    pub description: String,
    /// Author
    pub author: Option<String>,
    /// Inputs
    pub inputs: Option<AHashMap<String, ActionInput>>,
    /// Outputs
    pub outputs: Option<AHashMap<String, ActionOutput>>,
    /// Execution configuration
    pub runs: ActionRuns,
    /// Branding
    pub branding: Option<ActionBranding>,
}

/// Action input definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionInput {
    /// Input description
    pub description: String,
    /// Whether input is required
    pub required: Option<bool>,
    /// Default value
    pub default: Option<String>,
    /// Deprecation message
    pub deprecation_message: Option<String>,
}

/// Action output definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionOutput {
    /// Output description
    pub description: String,
    /// Value expression
    pub value: Option<String>,
}

/// Action execution configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "using")]
pub enum ActionRuns {
    /// Composite action
    #[serde(rename = "composite")]
    Composite {
        steps: Vec<CompositeStep>,
    },
    /// Node.js action
    #[serde(rename = "node20")]
    Node20 {
        main: String,
        pre: Option<String>,
        post: Option<String>,
    },
    /// Node.js 16 action
    #[serde(rename = "node16")]
    Node16 {
        main: String,
        pre: Option<String>,
        post: Option<String>,
    },
    /// Docker action
    #[serde(rename = "docker")]
    Docker {
        image: String,
        entrypoint: Option<String>,
        args: Option<Vec<String>>,
        env: Option<AHashMap<String, String>>,
    },
}

/// Composite action step
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompositeStep {
    /// Step ID
    pub id: Option<String>,
    /// Step name
    pub name: Option<String>,
    /// Shell command
    pub run: Option<String>,
    /// Shell type
    pub shell: Option<String>,
    /// Action to use
    pub uses: Option<String>,
    /// Working directory
    pub working_directory: Option<String>,
    /// Action inputs
    pub with: Option<AHashMap<String, String>>,
    /// Environment variables
    pub env: Option<AHashMap<String, String>>,
}

/// Action branding
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionBranding {
    /// Icon name
    pub icon: Option<String>,
    /// Color
    pub color: Option<String>,
}

impl ActionParser {
    /// Create a new action parser
    pub fn new(base_url: &str) -> Self {
        Self {
            base_url: base_url.to_string(),
        }
    }

    /// Parse action metadata
    pub fn parse_action(&self, content: &str) -> RunnerResult<ActionMetadata> {
        serde_yaml::from_str(content).map_err(|e| {
            RunnerError::WorkflowParsing(format!("Failed to parse action.yml: {}", e))
        })
    }

    /// Extract terms from action metadata
    fn extract_terms(&self, content: &str) -> RunnerResult<Vec<BuildTerm>> {
        let action = self.parse_action(content)?;
        let mut terms = Vec::new();

        // Action name
        let action_id = Uuid::new_v4().to_string();
        terms.push(BuildTerm {
            id: action_id.clone(),
            nterm: format!("action:{}", action.name),
            url: self.base_url.clone(),
            term_type: TermType::Action,
            parent: None,
            related: Vec::new(),
        });

        // Inputs
        if let Some(inputs) = &action.inputs {
            for (input_name, input) in inputs {
                let mut related = Vec::new();
                if input.required.unwrap_or(false) {
                    related.push("required".to_string());
                }
                if input.default.is_some() {
                    related.push("has_default".to_string());
                }

                terms.push(BuildTerm {
                    id: Uuid::new_v4().to_string(),
                    nterm: format!("input:{}", input_name),
                    url: format!("{}#input-{}", self.base_url, input_name),
                    term_type: TermType::EnvVar,
                    parent: Some(format!("action:{}", action.name)),
                    related,
                });
            }
        }

        // Outputs
        if let Some(outputs) = &action.outputs {
            for output_name in outputs.keys() {
                terms.push(BuildTerm {
                    id: Uuid::new_v4().to_string(),
                    nterm: format!("output:{}", output_name),
                    url: format!("{}#output-{}", self.base_url, output_name),
                    term_type: TermType::Artifact,
                    parent: Some(format!("action:{}", action.name)),
                    related: Vec::new(),
                });
            }
        }

        // Execution type
        match &action.runs {
            ActionRuns::Composite { steps } => {
                terms.push(BuildTerm {
                    id: Uuid::new_v4().to_string(),
                    nterm: "runs:composite".to_string(),
                    url: format!("{}#runs", self.base_url),
                    term_type: TermType::Action,
                    parent: Some(format!("action:{}", action.name)),
                    related: Vec::new(),
                });

                // Extract commands from composite steps
                for (idx, step) in steps.iter().enumerate() {
                    if let Some(run) = &step.run {
                        let first_cmd = run
                            .lines()
                            .find(|l| !l.trim().is_empty() && !l.trim().starts_with('#'))
                            .and_then(|l| l.trim().split_whitespace().next())
                            .unwrap_or("shell");

                        terms.push(BuildTerm {
                            id: Uuid::new_v4().to_string(),
                            nterm: format!("step:{}:{}", idx, first_cmd),
                            url: format!("{}#step-{}", self.base_url, idx),
                            term_type: TermType::Command,
                            parent: Some(format!("action:{}", action.name)),
                            related: Vec::new(),
                        });
                    }

                    if let Some(uses) = &step.uses {
                        terms.push(BuildTerm {
                            id: Uuid::new_v4().to_string(),
                            nterm: format!("step:{}:uses:{}", idx, uses),
                            url: format!("{}#step-{}", self.base_url, idx),
                            term_type: TermType::Action,
                            parent: Some(format!("action:{}", action.name)),
                            related: Vec::new(),
                        });
                    }
                }
            }
            ActionRuns::Node20 { main, .. } | ActionRuns::Node16 { main, .. } => {
                terms.push(BuildTerm {
                    id: Uuid::new_v4().to_string(),
                    nterm: "runs:node".to_string(),
                    url: format!("{}#runs", self.base_url),
                    term_type: TermType::Action,
                    parent: Some(format!("action:{}", action.name)),
                    related: vec![main.clone()],
                });
            }
            ActionRuns::Docker { image, .. } => {
                terms.push(BuildTerm {
                    id: Uuid::new_v4().to_string(),
                    nterm: format!("runs:docker:{}", image),
                    url: format!("{}#runs", self.base_url),
                    term_type: TermType::Action,
                    parent: Some(format!("action:{}", action.name)),
                    related: vec![image.clone()],
                });
            }
        }

        Ok(terms)
    }
}

impl BuildFileParser for ActionParser {
    fn parse(&self, content: &str) -> RunnerResult<Vec<BuildTerm>> {
        if content.trim().is_empty() {
            return Err(RunnerError::WorkflowParsing("Empty action.yml file".to_string()));
        }
        self.extract_terms(content)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_composite_action() {
        let content = r#"
name: 'My Composite Action'
description: 'A test composite action'
inputs:
  name:
    description: 'Name to greet'
    required: true
  greeting:
    description: 'Greeting to use'
    default: 'Hello'
outputs:
  message:
    description: 'The greeting message'
    value: ${{ steps.greet.outputs.message }}
runs:
  using: 'composite'
  steps:
    - id: greet
      run: echo "${{ inputs.greeting }}, ${{ inputs.name }}!"
      shell: bash
"#;

        let parser = ActionParser::new("https://github.com/owner/my-action");
        let terms = parser.parse(content).unwrap();

        assert!(!terms.is_empty());

        // Check for action
        assert!(terms.iter().any(|t| t.nterm == "action:My Composite Action"));

        // Check for inputs
        assert!(terms.iter().any(|t| t.nterm == "input:name"));
        assert!(terms.iter().any(|t| t.nterm == "input:greeting"));

        // Check for outputs
        assert!(terms.iter().any(|t| t.nterm == "output:message"));

        // Check for runs type
        assert!(terms.iter().any(|t| t.nterm == "runs:composite"));
    }

    #[test]
    fn test_parse_node_action() {
        let content = r#"
name: 'My Node Action'
description: 'A test node action'
inputs:
  token:
    description: 'GitHub token'
    required: true
runs:
  using: 'node20'
  main: 'dist/index.js'
  post: 'dist/cleanup.js'
"#;

        let parser = ActionParser::new("https://github.com/owner/my-action");
        let terms = parser.parse(content).unwrap();

        assert!(terms.iter().any(|t| t.nterm == "runs:node"));
    }
}
