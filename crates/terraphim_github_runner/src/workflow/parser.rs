//! LLM-based workflow understanding and translation
//!
//! Converts GitHub Actions workflows into executable command sequences using LLM.

use crate::error::{GitHubRunnerError, Result};
use serde::{Deserialize, Serialize};
#[cfg(feature = "github-runner")]
use terraphim_service::llm::{ChatOptions, LlmClient};

/// System prompt for workflow understanding
#[cfg(feature = "github-runner")]
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

/// Default per-step timeout (seconds) when a workflow does not specify one.
///
/// 30 minutes: Rust workspace `cargo build`/`test` steps on larger repos easily
/// exceed the old 5-minute default on a cold sccache. Four steps at this default
/// stay within the executor's 2-hour overall `max_execution_time` cap.
fn default_timeout() -> u64 {
    1800
}

fn indent_of(s: &str) -> usize {
    s.len() - s.trim_start().len()
}

/// Parse a Gitea `WorkflowPayload` into a [`ParsedWorkflow`].
///
/// The payload is the SingleWorkflow YAML for one job (nektos/act format),
/// already base64-decoded; it may be gzip-compressed. Used by the native Gitea
/// runner (#1910). Returns an error if no `run:` steps are found.
pub fn parse_workflow_payload(payload: &[u8]) -> Result<ParsedWorkflow> {
    let yaml = if payload.len() >= 2 && payload[0] == 0x1f && payload[1] == 0x8b {
        use std::io::Read;
        let mut decoder = flate2::read::GzDecoder::new(payload);
        let mut s = String::new();
        decoder.read_to_string(&mut s).map_err(|e| {
            GitHubRunnerError::WorkflowParsing(format!("failed to gunzip workflow payload: {e}"))
        })?;
        s
    } else {
        String::from_utf8(payload.to_vec()).map_err(|e| {
            GitHubRunnerError::WorkflowParsing(format!("workflow payload is not valid UTF-8: {e}"))
        })?
    };
    parse_single_workflow_yaml(&yaml)
}

/// Deterministic line-based parser for a single-job SingleWorkflow YAML.
///
/// Extracts each step's `run:` command (and optional `name:`) from
/// `jobs.<job>.steps[]`. Handles inline `run:` and block scalars (`run: |`,
/// `run: >`). `uses:` steps (marketplace actions) are skipped -- not supported
/// in M1. The top-level `name:` (column 0) becomes the workflow name.
pub fn parse_single_workflow_yaml(yaml: &str) -> Result<ParsedWorkflow> {
    let mut wf = ParsedWorkflow {
        trigger: "gitea".to_string(),
        ..ParsedWorkflow::default()
    };
    let mut steps: Vec<WorkflowStep> = Vec::new();
    let mut cur_name: Option<String> = None;
    let mut cur_run: Option<String> = None;
    let mut cur_uses = false;
    // When collecting a block scalar: indent of the `run:` key.
    let mut block_key_indent: Option<usize> = None;
    let mut block_lines: Vec<String> = Vec::new();

    fn push_step(
        steps: &mut Vec<WorkflowStep>,
        name: &mut Option<String>,
        run: &mut Option<String>,
        uses: &mut bool,
    ) {
        if !*uses && let Some(cmd) = run.take() {
            let cmd = cmd.trim_end().to_string();
            if !cmd.trim().is_empty() {
                let nm = name
                    .clone()
                    .unwrap_or_else(|| cmd.lines().next().unwrap_or("step").trim().to_string());
                steps.push(WorkflowStep {
                    name: nm,
                    command: cmd,
                    working_dir: default_working_dir(),
                    continue_on_error: false,
                    timeout_seconds: default_timeout(),
                });
            }
        }
        *name = None;
        *run = None;
        *uses = false;
    }

    for raw in yaml.lines() {
        // If collecting a block scalar, consume more-indented (or blank) lines.
        if let Some(key_indent) = block_key_indent {
            if raw.trim().is_empty() || indent_of(raw) > key_indent {
                block_lines.push(raw.trim_start().to_string());
                continue;
            }
            // Block ended: commit it, then fall through to process this line.
            cur_run = Some(block_lines.join("\n"));
            block_key_indent = None;
            block_lines.clear();
        }

        let ind = indent_of(raw);
        let trimmed = raw.trim_start();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        // A new list item under steps flushes the previous step.
        if trimmed.starts_with("- ") {
            push_step(&mut steps, &mut cur_name, &mut cur_run, &mut cur_uses);
        }
        let body = trimmed.strip_prefix("- ").unwrap_or(trimmed);

        if let Some(rest) = body.strip_prefix("run:") {
            let val = rest.trim();
            if matches!(val, "|" | ">" | "|-" | ">-" | "|+" | ">+") {
                block_key_indent = Some(ind);
                block_lines.clear();
            } else {
                cur_run = Some(val.to_string());
            }
        } else if body.strip_prefix("uses:").is_some() {
            cur_uses = true;
        } else if let Some(rest) = body.strip_prefix("name:") {
            if ind == 0 {
                wf.name = rest.trim().to_string();
            } else {
                cur_name = Some(rest.trim().to_string());
            }
        }
    }

    // Flush a trailing block scalar and the final step.
    if block_key_indent.is_some() {
        cur_run = Some(block_lines.join("\n"));
    }
    push_step(&mut steps, &mut cur_name, &mut cur_run, &mut cur_uses);

    if steps.is_empty() {
        return Err(GitHubRunnerError::WorkflowParsing(
            "no run: steps found in workflow payload".to_string(),
        ));
    }
    wf.steps = steps;
    Ok(wf)
}

#[cfg(test)]
mod payload_tests {
    use super::*;

    #[test]
    fn parses_inline_and_block_run_steps() {
        let yaml = "name: CI\njobs:\n  build:\n    runs-on: terraphim-native\n    steps:\n      - name: Fmt\n        run: cargo fmt --all -- --check\n      - name: Build and test\n        run: |\n          cargo build --workspace\n          cargo test --workspace\n      - uses: actions/checkout@v4\n";
        let wf = parse_single_workflow_yaml(yaml).unwrap();
        assert_eq!(wf.name, "CI");
        assert_eq!(wf.steps.len(), 2, "uses: step must be skipped");
        assert_eq!(wf.steps[0].command, "cargo fmt --all -- --check");
        assert_eq!(wf.steps[0].name, "Fmt");
        assert_eq!(
            wf.steps[1].command,
            "cargo build --workspace\ncargo test --workspace"
        );
    }

    #[test]
    fn plain_payload_bytes_parse() {
        let wf =
            parse_workflow_payload(b"jobs:\n  j:\n    steps:\n      - run: echo hi\n").unwrap();
        assert_eq!(wf.steps.len(), 1);
        assert_eq!(wf.steps[0].command, "echo hi");
    }

    #[test]
    fn empty_workflow_errors() {
        assert!(parse_single_workflow_yaml("name: empty\njobs: {}\n").is_err());
    }

    #[test]
    fn gzip_payload_is_detected_and_inflated() {
        use std::io::Write;
        let yaml = "jobs:\n  j:\n    steps:\n      - run: echo hi\n";
        let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
        encoder.write_all(yaml.as_bytes()).unwrap();
        let gz = encoder.finish().unwrap();
        // gzip magic header present so the inflate branch is taken.
        assert_eq!(&gz[0..2], &[0x1f, 0x8b]);

        let wf = parse_workflow_payload(&gz).unwrap();
        assert_eq!(wf.steps.len(), 1);
        assert_eq!(wf.steps[0].command, "echo hi");
    }
}

/// Workflow parser using LLM for understanding
#[cfg(feature = "github-runner")]
pub struct WorkflowParser {
    llm_client: Arc<dyn LlmClient>,
}

#[cfg(feature = "github-runner")]
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
                "cargo clean -p $(cargo read-manifest | jq -r .name)".to_string(),
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

#[cfg(feature = "github-runner")]
impl Clone for WorkflowParser {
    fn clone(&self) -> Self {
        Self {
            llm_client: Arc::clone(&self.llm_client),
        }
    }
}

#[cfg(all(test, feature = "github-runner"))]
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
        assert_eq!(step.timeout_seconds, 1800);
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
