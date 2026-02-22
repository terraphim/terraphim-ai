//! LLM-based workflow understanding and translation
//!
//! Converts GitHub Actions workflows into executable command sequences using LLM.

use crate::error::{GitHubRunnerError, Result};
use crate::models::{GitHubEvent, GitHubEventType};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use terraphim_service::llm::{ChatOptions, LlmClient};

/// System prompt for workflow understanding
const WORKFLOW_SYSTEM_PROMPT: &str = r#"You are an expert GitHub Actions workflow parser.
Your task is to analyze GitHub Actions workflows and translate them into executable shell commands.

When given a workflow YAML or event description:
1. Identify the trigger conditions (push, pull_request, workflow_dispatch)
2. Extract environment variables and secrets needed
3. List the steps in order as shell commands
4. Note any dependencies between steps
5. Identify caching opportunities

Output format (JSON):
{
    "name": "workflow name",
    "trigger": "push|pull_request|workflow_dispatch",
    "environment": {"VAR_NAME": "value or ${{ secrets.NAME }}"},
    "setup_commands": ["commands to run before main steps"],
    "steps": [
        {
            "name": "step name",
            "command": "shell command to execute",
            "working_dir": "/workspace or specific path",
            "continue_on_error": false,
            "timeout_seconds": 300
        }
    ],
    "cleanup_commands": ["commands to run after all steps"],
    "cache_paths": ["paths that should be cached between runs"]
}

Be precise and executable. Use standard shell commands. Do not include GitHub-specific actions syntax - translate them to shell equivalents."#;

/// A parsed workflow ready for execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedWorkflow {
    /// Workflow name
    pub name: String,
    /// Trigger type
    pub trigger: String,
    /// Environment variables to set
    pub environment: std::collections::HashMap<String, String>,
    /// Commands to run during setup
    pub setup_commands: Vec<String>,
    /// Main workflow steps
    pub steps: Vec<WorkflowStep>,
    /// Cleanup commands
    pub cleanup_commands: Vec<String>,
    /// Paths to cache
    pub cache_paths: Vec<String>,
}

impl Default for ParsedWorkflow {
    fn default() -> Self {
        Self {
            name: "default".to_string(),
            trigger: "push".to_string(),
            environment: std::collections::HashMap::new(),
            setup_commands: Vec::new(),
            steps: Vec::new(),
            cleanup_commands: Vec::new(),
            cache_paths: Vec::new(),
        }
    }
}

/// A single step in the workflow
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowStep {
    /// Step name/description
    pub name: String,
    /// Shell command to execute
    pub command: String,
    /// Working directory
    #[serde(default = "default_working_dir")]
    pub working_dir: String,
    /// Continue if this step fails
    #[serde(default)]
    pub continue_on_error: bool,
    /// Timeout in seconds
    #[serde(default = "default_timeout")]
    pub timeout_seconds: u64,
}

fn default_working_dir() -> String {
    "/workspace".to_string()
}

fn default_timeout() -> u64 {
    300
}

/// Workflow parser using LLM for understanding
pub struct WorkflowParser {
    llm_client: Arc<dyn LlmClient>,
}

impl WorkflowParser {
    /// Create a new workflow parser with the given LLM client
    pub fn new(llm_client: Arc<dyn LlmClient>) -> Self {
        Self { llm_client }
    }

    /// Parse a GitHub workflow YAML into executable commands
    pub async fn parse_workflow_yaml(&self, workflow_yaml: &str) -> Result<ParsedWorkflow> {
        let messages = vec![
            serde_json::json!({
                "role": "system",
                "content": WORKFLOW_SYSTEM_PROMPT
            }),
            serde_json::json!({
                "role": "user",
                "content": format!("Parse this GitHub Actions workflow into executable commands:\n\n```yaml\n{}\n```", workflow_yaml)
            }),
        ];

        let response = self
            .llm_client
            .chat_completion(
                messages,
                ChatOptions {
                    max_tokens: Some(2000),
                    temperature: Some(0.1), // Low temperature for precise output
                },
            )
            .await
            .map_err(|e| GitHubRunnerError::LlmUnderstanding(e.to_string()))?;

        self.parse_llm_response(&response)
    }

    /// Parse a GitHub event into a workflow context
    pub async fn parse_event(&self, event: &GitHubEvent) -> Result<ParsedWorkflow> {
        let event_description = self.describe_event(event);

        let messages = vec![
            serde_json::json!({
                "role": "system",
                "content": WORKFLOW_SYSTEM_PROMPT
            }),
            serde_json::json!({
                "role": "user",
                "content": format!("Generate a standard CI workflow for this GitHub event:\n\n{}", event_description)
            }),
        ];

        let response = self
            .llm_client
            .chat_completion(
                messages,
                ChatOptions {
                    max_tokens: Some(2000),
                    temperature: Some(0.1),
                },
            )
            .await
            .map_err(|e| GitHubRunnerError::LlmUnderstanding(e.to_string()))?;

        self.parse_llm_response(&response)
    }

    /// Generate a default workflow for common scenarios
    pub fn default_workflow_for_event(&self, event: &GitHubEvent) -> ParsedWorkflow {
        let repo_name = &event.repository.full_name;
        let is_rust = repo_name.contains("rust") || repo_name.contains("-rs");

        if is_rust {
            self.default_rust_workflow(event)
        } else {
            self.default_generic_workflow(event)
        }
    }

    /// Default Rust project workflow
    fn default_rust_workflow(&self, event: &GitHubEvent) -> ParsedWorkflow {
        let mut env = std::collections::HashMap::new();
        env.insert("RUST_BACKTRACE".to_string(), "1".to_string());
        env.insert("CARGO_TERM_COLOR".to_string(), "always".to_string());

        let checkout_cmd = if let Some(sha) = &event.sha {
            format!("git checkout {}", sha)
        } else {
            "git checkout HEAD".to_string()
        };

        ParsedWorkflow {
            name: format!("CI for {}", event.repository.full_name),
            trigger: match event.event_type {
                GitHubEventType::PullRequest => "pull_request".to_string(),
                GitHubEventType::Push => "push".to_string(),
                GitHubEventType::WorkflowDispatch => "workflow_dispatch".to_string(),
                GitHubEventType::Unknown(ref s) => s.clone(),
            },
            environment: env,
            setup_commands: vec![
                "rustup update stable".to_string(),
                "rustup component add clippy rustfmt".to_string(),
            ],
            steps: vec![
                WorkflowStep {
                    name: "Checkout".to_string(),
                    command: checkout_cmd,
                    working_dir: "/workspace".to_string(),
                    continue_on_error: false,
                    timeout_seconds: 60,
                },
                WorkflowStep {
                    name: "Format check".to_string(),
                    command: "cargo fmt --all -- --check".to_string(),
                    working_dir: "/workspace".to_string(),
                    continue_on_error: false,
                    timeout_seconds: 120,
                },
                WorkflowStep {
                    name: "Clippy".to_string(),
                    command: "cargo clippy --all-targets --all-features -- -D warnings".to_string(),
                    working_dir: "/workspace".to_string(),
                    continue_on_error: false,
                    timeout_seconds: 300,
                },
                WorkflowStep {
                    name: "Build".to_string(),
                    command: "cargo build --all-targets".to_string(),
                    working_dir: "/workspace".to_string(),
                    continue_on_error: false,
                    timeout_seconds: 600,
                },
                WorkflowStep {
                    name: "Test".to_string(),
                    command: "cargo test --all".to_string(),
                    working_dir: "/workspace".to_string(),
                    continue_on_error: false,
                    timeout_seconds: 600,
                },
            ],
            cleanup_commands: vec![
                "cargo clean -p $(cargo read-manifest | jq -r .name)".to_string()
            ],
            cache_paths: vec![
                "~/.cargo/registry".to_string(),
                "~/.cargo/git".to_string(),
                "target".to_string(),
            ],
        }
    }

    /// Default generic workflow
    fn default_generic_workflow(&self, event: &GitHubEvent) -> ParsedWorkflow {
        let checkout_cmd = if let Some(sha) = &event.sha {
            format!("git checkout {}", sha)
        } else {
            "git checkout HEAD".to_string()
        };

        ParsedWorkflow {
            name: format!("CI for {}", event.repository.full_name),
            trigger: match event.event_type {
                GitHubEventType::PullRequest => "pull_request".to_string(),
                GitHubEventType::Push => "push".to_string(),
                GitHubEventType::WorkflowDispatch => "workflow_dispatch".to_string(),
                GitHubEventType::Unknown(ref s) => s.clone(),
            },
            environment: std::collections::HashMap::new(),
            setup_commands: vec![],
            steps: vec![
                WorkflowStep {
                    name: "Checkout".to_string(),
                    command: checkout_cmd,
                    working_dir: "/workspace".to_string(),
                    continue_on_error: false,
                    timeout_seconds: 60,
                },
                WorkflowStep {
                    name: "Show info".to_string(),
                    command: "echo 'Repository cloned successfully' && ls -la".to_string(),
                    working_dir: "/workspace".to_string(),
                    continue_on_error: false,
                    timeout_seconds: 30,
                },
            ],
            cleanup_commands: vec![],
            cache_paths: vec![],
        }
    }

    /// Describe a GitHub event for LLM understanding
    fn describe_event(&self, event: &GitHubEvent) -> String {
        let mut description = format!(
            "Event type: {:?}\nRepository: {}\n",
            event.event_type, event.repository.full_name
        );

        if let Some(action) = &event.action {
            description.push_str(&format!("Action: {}\n", action));
        }

        if let Some(git_ref) = &event.git_ref {
            description.push_str(&format!("Ref: {}\n", git_ref));
        }

        if let Some(sha) = &event.sha {
            description.push_str(&format!("Commit SHA: {}\n", sha));
        }

        if let Some(pr) = &event.pull_request {
            description.push_str(&format!(
                "Pull Request: #{} - {}\nHead: {:?}\nBase: {:?}\n",
                pr.number, pr.title, pr.head_branch, pr.base_branch
            ));
        }

        description
    }

    /// Parse LLM response into ParsedWorkflow
    fn parse_llm_response(&self, response: &str) -> Result<ParsedWorkflow> {
        // Try to extract JSON from the response
        let json_str = self.extract_json(response)?;

        serde_json::from_str(&json_str).map_err(|e| {
            GitHubRunnerError::WorkflowParsing(format!("Failed to parse LLM response: {}", e))
        })
    }

    /// Extract JSON from LLM response (handles markdown code blocks)
    fn extract_json(&self, response: &str) -> Result<String> {
        // Check for JSON in code blocks
        if let Some(start) = response.find("```json") {
            let content_start = start + 7;
            if let Some(end) = response[content_start..].find("```") {
                return Ok(response[content_start..content_start + end]
                    .trim()
                    .to_string());
            }
        }

        // Check for plain code blocks
        if let Some(start) = response.find("```") {
            let content_start = start + 3;
            // Skip any language identifier on the same line
            let actual_start = response[content_start..]
                .find('\n')
                .map(|n| content_start + n + 1)
                .unwrap_or(content_start);
            if let Some(end) = response[actual_start..].find("```") {
                return Ok(response[actual_start..actual_start + end]
                    .trim()
                    .to_string());
            }
        }

        // Try to find raw JSON (starts with {)
        if let Some(start) = response.find('{') {
            // Find matching closing brace
            let mut depth = 0;
            let mut end_pos = start;
            for (i, c) in response[start..].chars().enumerate() {
                match c {
                    '{' => depth += 1,
                    '}' => {
                        depth -= 1;
                        if depth == 0 {
                            end_pos = start + i + 1;
                            break;
                        }
                    }
                    _ => {}
                }
            }
            if end_pos > start {
                return Ok(response[start..end_pos].to_string());
            }
        }

        Err(GitHubRunnerError::WorkflowParsing(
            "Could not extract JSON from LLM response".to_string(),
        ))
    }
}

impl Clone for WorkflowParser {
    fn clone(&self) -> Self {
        Self {
            llm_client: Arc::clone(&self.llm_client),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_json_from_code_block() {
        let parser = WorkflowParser::new(Arc::new(MockLlmClient));

        let response = r#"Here's the parsed workflow:
```json
{"name": "test", "trigger": "push", "environment": {}, "setup_commands": [], "steps": [], "cleanup_commands": [], "cache_paths": []}
```"#;

        let result = parser.extract_json(response);
        assert!(result.is_ok());
        let json: ParsedWorkflow = serde_json::from_str(&result.unwrap()).unwrap();
        assert_eq!(json.name, "test");
    }

    #[test]
    fn test_extract_json_raw() {
        let parser = WorkflowParser::new(Arc::new(MockLlmClient));

        let response = r#"{"name": "test", "trigger": "push", "environment": {}, "setup_commands": [], "steps": [], "cleanup_commands": [], "cache_paths": []}"#;

        let result = parser.extract_json(response);
        assert!(result.is_ok());
    }

    #[test]
    fn test_default_rust_workflow() {
        let parser = WorkflowParser::new(Arc::new(MockLlmClient));

        let event = GitHubEvent {
            event_type: GitHubEventType::PullRequest,
            action: Some("opened".to_string()),
            repository: crate::models::RepositoryInfo {
                full_name: "terraphim/terraphim-ai".to_string(),
                clone_url: Some("https://github.com/terraphim/terraphim-ai.git".to_string()),
                default_branch: Some("main".to_string()),
            },
            pull_request: None,
            git_ref: Some("refs/heads/feature".to_string()),
            sha: Some("abc123".to_string()),
            extra: std::collections::HashMap::new(),
        };

        let workflow = parser.default_workflow_for_event(&event);
        assert!(!workflow.steps.is_empty());
        assert!(workflow.steps.iter().any(|s| s.name.contains("Checkout")));
    }

    #[test]
    fn test_workflow_step_defaults() {
        let step: WorkflowStep =
            serde_json::from_str(r#"{"name": "test", "command": "echo hi"}"#).unwrap();
        assert_eq!(step.working_dir, "/workspace");
        assert_eq!(step.timeout_seconds, 300);
        assert!(!step.continue_on_error);
    }

    // Mock LLM client for testing
    struct MockLlmClient;

    #[async_trait::async_trait]
    impl LlmClient for MockLlmClient {
        fn name(&self) -> &'static str {
            "mock"
        }

        async fn summarize(
            &self,
            _content: &str,
            _opts: terraphim_service::llm::SummarizeOptions,
        ) -> terraphim_service::Result<String> {
            Ok("summary".to_string())
        }

        async fn chat_completion(
            &self,
            _messages: Vec<serde_json::Value>,
            _opts: ChatOptions,
        ) -> terraphim_service::Result<String> {
            Ok(r#"{"name": "mock", "trigger": "push", "environment": {}, "setup_commands": [], "steps": [{"name": "test", "command": "echo hello"}], "cleanup_commands": [], "cache_paths": []}"#.to_string())
        }
    }
}
