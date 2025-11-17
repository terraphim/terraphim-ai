//! Error types for the GitHub runner

use thiserror::Error;

/// Result type alias for runner operations
pub type RunnerResult<T> = Result<T, RunnerError>;

/// Errors that can occur during runner operations
#[derive(Error, Debug)]
pub enum RunnerError {
    /// GitHub API communication errors
    #[error("GitHub API error: {0}")]
    GitHubApi(String),

    /// Runner registration failed
    #[error("Registration failed: {0}")]
    Registration(String),

    /// Authentication errors
    #[error("Authentication error: {0}")]
    Authentication(String),

    /// Workflow parsing errors
    #[error("Workflow parsing error: {0}")]
    WorkflowParsing(String),

    /// Earthfile parsing errors
    #[error("Earthfile parsing error: {0}")]
    EarthfileParsing(String),

    /// Dockerfile parsing errors
    #[error("Dockerfile parsing error: {0}")]
    DockerfileParsing(String),

    /// Knowledge graph construction errors
    #[error("Knowledge graph error: {0}")]
    KnowledgeGraph(String),

    /// LLM interpretation errors
    #[error("LLM interpretation error: {0}")]
    LlmInterpretation(String),

    /// VM execution errors
    #[error("VM execution error: {0}")]
    VmExecution(String),

    /// Action execution errors
    #[error("Action execution error: {0}")]
    ActionExecution(String),

    /// History storage errors
    #[error("History storage error: {0}")]
    HistoryStorage(String),

    /// Configuration errors
    #[error("Configuration error: {0}")]
    Configuration(String),

    /// Network errors
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),

    /// Serialization errors
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// YAML parsing errors
    #[error("YAML error: {0}")]
    Yaml(#[from] serde_yaml::Error),

    /// IO errors
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// JWT errors
    #[error("JWT error: {0}")]
    Jwt(#[from] jsonwebtoken::errors::Error),

    /// Invalid state
    #[error("Invalid state: {0}")]
    InvalidState(String),

    /// Timeout
    #[error("Operation timed out: {0}")]
    Timeout(String),
}
