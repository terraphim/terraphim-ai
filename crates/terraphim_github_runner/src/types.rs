//! Core types for the GitHub runner

use ahash::AHashMap;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Unique identifier for a runner instance
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RunnerId(pub String);

impl RunnerId {
    pub fn new() -> Self {
        Self(Uuid::new_v4().to_string())
    }
}

impl Default for RunnerId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for RunnerId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Runner state in its lifecycle
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RunnerState {
    /// Runner is starting up
    Initializing,
    /// Runner is idle, waiting for jobs
    Idle,
    /// Runner is executing a job
    Busy,
    /// Runner is shutting down
    Offline,
    /// Runner encountered an error
    Error,
}

/// Runner labels for job matching
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunnerLabels {
    /// Built-in labels (self-hosted, linux, x64, etc.)
    pub builtin: Vec<String>,
    /// Custom labels for specific capabilities
    pub custom: Vec<String>,
}

impl Default for RunnerLabels {
    fn default() -> Self {
        Self {
            builtin: vec![
                "self-hosted".to_string(),
                "linux".to_string(),
                "x64".to_string(),
            ],
            custom: vec![
                "terraphim".to_string(),
                "firecracker".to_string(),
                "knowledge-graph".to_string(),
            ],
        }
    }
}

/// Configuration for a runner instance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunnerConfig {
    /// Runner name (displayed in GitHub)
    pub name: String,
    /// Repository or organization to register with
    pub scope: RunnerScope,
    /// Registration token from GitHub
    pub registration_token: String,
    /// Runner labels
    pub labels: RunnerLabels,
    /// Working directory for job execution
    pub work_directory: String,
    /// Maximum concurrent jobs (usually 1)
    pub max_concurrent_jobs: usize,
    /// VM pool size for Firecracker
    pub vm_pool_size: usize,
    /// Enable LLM interpretation
    pub enable_llm: bool,
    /// LLM provider configuration
    pub llm_config: Option<LlmConfig>,
}

/// Scope for runner registration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RunnerScope {
    /// Repository-level runner
    Repository {
        owner: String,
        repo: String,
    },
    /// Organization-level runner
    Organization {
        org: String,
    },
    /// Enterprise-level runner
    Enterprise {
        enterprise: String,
    },
}

impl RunnerScope {
    /// Get the API URL for this scope
    pub fn api_url(&self, base_url: &str) -> String {
        match self {
            RunnerScope::Repository { owner, repo } => {
                format!("{}/repos/{}/{}/actions/runners", base_url, owner, repo)
            }
            RunnerScope::Organization { org } => {
                format!("{}/orgs/{}/actions/runners", base_url, org)
            }
            RunnerScope::Enterprise { enterprise } => {
                format!("{}/enterprises/{}/actions/runners", base_url, enterprise)
            }
        }
    }
}

/// LLM configuration for action interpretation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmConfig {
    /// Provider (ollama, openrouter, etc.)
    pub provider: String,
    /// Model name
    pub model: String,
    /// Base URL for the provider
    pub base_url: Option<String>,
    /// API key (for cloud providers)
    pub api_key: Option<String>,
}

/// A GitHub Actions workflow
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workflow {
    /// Workflow name
    pub name: String,
    /// Trigger events
    #[serde(rename = "on")]
    pub on_trigger: WorkflowTrigger,
    /// Environment variables
    #[serde(default)]
    pub env: AHashMap<String, String>,
    /// Jobs in the workflow
    pub jobs: AHashMap<String, Job>,
}

/// Workflow trigger configuration - can be simple string or complex object
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum WorkflowTrigger {
    /// Simple trigger (e.g., "push")
    Simple(String),
    /// Array of simple triggers (e.g., ["push", "pull_request"])
    Array(Vec<String>),
    /// Complex trigger configuration
    Complex(WorkflowTriggerConfig),
}

/// Detailed workflow trigger configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowTriggerConfig {
    /// Push events
    pub push: Option<PushTrigger>,
    /// Pull request events
    pub pull_request: Option<PullRequestTrigger>,
    /// Workflow dispatch (manual)
    pub workflow_dispatch: Option<WorkflowDispatchTrigger>,
    /// Schedule (cron)
    pub schedule: Option<Vec<ScheduleTrigger>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PushTrigger {
    pub branches: Option<Vec<String>>,
    pub tags: Option<Vec<String>>,
    pub paths: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PullRequestTrigger {
    pub branches: Option<Vec<String>>,
    pub types: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowDispatchTrigger {
    pub inputs: Option<AHashMap<String, WorkflowInput>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowInput {
    pub description: Option<String>,
    pub required: Option<bool>,
    pub default: Option<String>,
    #[serde(rename = "type")]
    pub input_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduleTrigger {
    pub cron: String,
}

/// A job within a workflow
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Job {
    /// Job name
    pub name: Option<String>,
    /// Runner requirements
    #[serde(rename = "runs-on")]
    pub runs_on: RunsOn,
    /// Job dependencies
    pub needs: Option<Vec<String>>,
    /// Conditional execution
    #[serde(rename = "if")]
    pub condition: Option<String>,
    /// Environment variables
    pub env: Option<AHashMap<String, String>>,
    /// Job steps
    pub steps: Vec<Step>,
    /// Services (docker containers)
    pub services: Option<AHashMap<String, Service>>,
    /// Strategy (matrix, fail-fast)
    pub strategy: Option<Strategy>,
}

/// Runner requirements for a job
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum RunsOn {
    /// Single label
    Label(String),
    /// Multiple labels (all must match)
    Labels(Vec<String>),
}

/// A step within a job
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Step {
    /// Step ID for outputs
    pub id: Option<String>,
    /// Step name
    pub name: Option<String>,
    /// Action to use
    pub uses: Option<String>,
    /// Shell command to run
    pub run: Option<String>,
    /// Working directory
    #[serde(rename = "working-directory")]
    pub working_directory: Option<String>,
    /// Shell to use
    pub shell: Option<String>,
    /// Conditional execution
    #[serde(rename = "if")]
    pub condition: Option<String>,
    /// Environment variables
    pub env: Option<AHashMap<String, String>>,
    /// Action inputs
    pub with: Option<AHashMap<String, serde_yaml::Value>>,
    /// Continue on error
    #[serde(rename = "continue-on-error")]
    pub continue_on_error: Option<bool>,
    /// Timeout in minutes
    #[serde(rename = "timeout-minutes")]
    pub timeout_minutes: Option<u32>,
}

/// Service container definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Service {
    /// Docker image
    pub image: String,
    /// Port mappings
    pub ports: Option<Vec<String>>,
    /// Environment variables
    pub env: Option<AHashMap<String, String>>,
    /// Volume mounts
    pub volumes: Option<Vec<String>>,
    /// Health check options
    pub options: Option<String>,
}

/// Job strategy configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Strategy {
    /// Matrix configuration
    pub matrix: Option<AHashMap<String, serde_yaml::Value>>,
    /// Fail fast setting
    #[serde(rename = "fail-fast")]
    pub fail_fast: Option<bool>,
    /// Max parallel jobs
    #[serde(rename = "max-parallel")]
    pub max_parallel: Option<usize>,
}

/// A workflow job event from GitHub
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowJobEvent {
    /// Event action (queued, in_progress, completed)
    pub action: String,
    /// Repository information
    pub repository: Repository,
    /// Workflow job details
    pub workflow_job: WorkflowJob,
}

/// Repository information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Repository {
    pub id: u64,
    pub name: String,
    pub full_name: String,
    pub html_url: String,
    pub clone_url: String,
}

/// Workflow job from GitHub API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowJob {
    pub id: u64,
    pub run_id: u64,
    pub workflow_name: Option<String>,
    pub head_branch: Option<String>,
    pub head_sha: String,
    pub status: String,
    pub conclusion: Option<String>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub name: String,
    pub steps: Option<Vec<JobStep>>,
    pub labels: Vec<String>,
    pub runner_id: Option<u64>,
    pub runner_name: Option<String>,
}

/// Step within a workflow job
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobStep {
    pub name: String,
    pub status: String,
    pub conclusion: Option<String>,
    pub number: u32,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
}

/// Result of executing a step
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepResult {
    /// Step identifier
    pub step_id: String,
    /// Exit code
    pub exit_code: i32,
    /// Standard output (truncated)
    pub stdout: String,
    /// Standard error (truncated)
    pub stderr: String,
    /// Execution duration in milliseconds
    pub duration_ms: u64,
    /// Outputs from the step
    pub outputs: AHashMap<String, String>,
    /// Artifacts produced
    pub artifacts: Vec<ArtifactRef>,
    /// VM snapshot ID (if captured)
    pub vm_snapshot_id: Option<String>,
}

/// Reference to an artifact
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactRef {
    /// Artifact name
    pub name: String,
    /// Path within the workspace
    pub path: String,
    /// Size in bytes
    pub size: u64,
    /// Content hash
    pub hash: String,
}

/// Interpreted action from LLM
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterpretedAction {
    /// Original action reference
    pub original: String,
    /// Semantic purpose
    pub purpose: String,
    /// Prerequisites (other actions that must run first)
    pub prerequisites: Vec<String>,
    /// What this action produces
    pub produces: Vec<String>,
    /// Whether output is deterministic (cacheable)
    pub cacheable: bool,
    /// Translated shell commands
    pub commands: Vec<String>,
    /// Required environment variables
    pub required_env: Vec<String>,
    /// Knowledge graph term IDs
    pub kg_terms: Vec<String>,
    /// Confidence score (0.0 - 1.0)
    pub confidence: f64,
}

/// Execution record for history
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionRecord {
    /// Unique record ID
    pub id: String,
    /// Workflow run ID
    pub workflow_id: String,
    /// Job ID
    pub job_id: String,
    /// Step index
    pub step_index: usize,
    /// Action reference
    pub action: String,
    /// Interpreted commands
    pub interpreted_commands: Vec<String>,
    /// VM snapshot ID
    pub vm_snapshot_id: Option<String>,
    /// Execution duration
    pub duration_ms: u64,
    /// Exit code
    pub exit_code: i32,
    /// Hash of stdout
    pub stdout_hash: String,
    /// Artifacts produced
    pub artifacts_produced: Vec<ArtifactRef>,
    /// Knowledge graph context
    pub kg_context: Vec<String>,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
    /// Repository
    pub repository: String,
    /// Branch
    pub branch: String,
}

/// Term extracted from build files for knowledge graph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildTerm {
    /// Unique identifier
    pub id: String,
    /// Normalized term
    pub nterm: String,
    /// Source URL or reference
    pub url: String,
    /// Term type (action, command, target, etc.)
    pub term_type: TermType,
    /// Parent term (for hierarchy)
    pub parent: Option<String>,
    /// Related terms
    pub related: Vec<String>,
}

/// Type of term in the knowledge graph
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TermType {
    /// GitHub Action
    Action,
    /// Shell command
    Command,
    /// Earthly target
    EarthlyTarget,
    /// Docker instruction
    DockerInstruction,
    /// Environment variable
    EnvVar,
    /// Artifact
    Artifact,
    /// Service
    Service,
}
