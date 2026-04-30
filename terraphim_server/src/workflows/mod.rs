//! Workflow execution patterns and session management for the Terraphim server.
//!
//! Provides prompt-chain, routing, parallel, orchestration, optimisation, and
//! VM-sandboxed execution workflows, together with a shared [`WorkflowSessions`]
//! registry and WebSocket broadcast infrastructure.

use axum::{
    Router,
    extract::{Path, State},
    http::StatusCode,
    response::Json,
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::sync::{RwLock, broadcast};
use uuid::Uuid;

use crate::AppState;

/// Multi-agent workflow handlers.
pub mod multi_agent_handlers;
/// Workflow optimisation patterns.
pub mod optimization;
/// Agent orchestration workflow.
pub mod orchestration;
/// Parallel workflow execution.
pub mod parallel;
/// Prompt-chain workflow execution.
pub mod prompt_chain;
/// Routing-based workflow execution.
pub mod routing;
/// VM-sandboxed workflow execution.
pub mod vm_execution;
/// WebSocket transport for workflow progress events.
pub mod websocket;

/// LLM provider configuration for a single workflow or step.
#[derive(Debug, Deserialize, Clone)]
pub struct LlmConfig {
    /// LLM provider identifier (e.g. `"ollama"`, `"openrouter"`).
    pub llm_provider: Option<String>,
    /// Model name understood by the provider.
    pub llm_model: Option<String>,
    /// Base URL for the provider API.
    pub llm_base_url: Option<String>,
    /// Sampling temperature (0.0 – 1.0).
    pub llm_temperature: Option<f64>,
}

impl Default for LlmConfig {
    fn default() -> Self {
        Self {
            llm_provider: Some("ollama".to_string()),
            llm_model: Some("llama3.2:3b".to_string()),
            llm_base_url: Some("http://127.0.0.1:11434".to_string()),
            llm_temperature: Some(0.3),
        }
    }
}

/// Per-step configuration overriding top-level workflow settings.
#[derive(Debug, Deserialize, Clone)]
pub struct StepConfig {
    /// Unique step identifier within the workflow.
    pub id: String,
    /// Human-readable step name.
    pub name: String,
    /// Prompt template for this step.
    pub prompt: String,
    /// Optional role override for this step.
    pub role: Option<String>,
    /// Optional system-prompt override for this step.
    pub system_prompt: Option<String>,
    /// Optional LLM configuration override for this step.
    pub llm_config: Option<LlmConfig>,
}

/// Request body for workflow execution endpoints.
#[derive(Debug, Deserialize)]
pub struct WorkflowRequest {
    /// User prompt or task description.
    pub prompt: String,
    /// Terraphim role used for knowledge-graph context.
    pub role: Option<String>,
    /// Overall orchestrator role when running multi-step workflows.
    pub overall_role: Option<String>,
    /// Arbitrary workflow-specific configuration.
    pub config: Option<serde_json::Value>,
    /// Default LLM configuration for the workflow.
    pub llm_config: Option<LlmConfig>,
    /// Per-step configuration overrides.
    pub steps: Option<Vec<StepConfig>>,
}

/// Response returned by workflow execution endpoints.
#[derive(Debug, Serialize)]
pub struct WorkflowResponse {
    /// Unique identifier assigned to this workflow run.
    pub workflow_id: String,
    /// Whether the workflow completed without error.
    pub success: bool,
    /// Workflow output payload on success.
    pub result: Option<serde_json::Value>,
    /// Error description on failure.
    pub error: Option<String>,
    /// Execution metrics and context.
    pub metadata: WorkflowMetadata,
}

/// Metrics and context attached to a [`WorkflowResponse`].
#[derive(Debug, Serialize)]
pub struct WorkflowMetadata {
    /// Wall-clock time from start to completion in milliseconds.
    pub execution_time_ms: u64,
    /// Workflow pattern name (e.g. `"prompt-chain"`, `"parallel"`).
    pub pattern: String,
    /// Number of steps executed.
    pub steps: usize,
    /// Terraphim role used.
    pub role: String,
    /// Overall orchestrator role used.
    pub overall_role: String,
}

/// Live status of a running or completed workflow.
#[derive(Debug, Clone, Serialize)]
pub struct WorkflowStatus {
    /// Workflow identifier.
    pub id: String,
    /// Current execution state.
    pub status: ExecutionStatus,
    /// Completion fraction in the range `[0.0, 100.0]`.
    pub progress: f64,
    /// Name of the step currently executing, if any.
    pub current_step: Option<String>,
    /// UTC timestamp when the workflow started.
    pub started_at: chrono::DateTime<chrono::Utc>,
    /// UTC timestamp when the workflow finished, if complete.
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
    /// Final result payload, populated on completion.
    pub result: Option<serde_json::Value>,
    /// Error description, populated on failure.
    pub error: Option<String>,
}

/// Lifecycle state of a workflow execution.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionStatus {
    /// Queued but not yet started.
    Pending,
    /// Currently executing.
    Running,
    /// Finished successfully.
    Completed,
    /// Finished with an error.
    Failed,
    /// Cancelled before completion.
    Cancelled,
}

/// Message broadcast over WebSocket for workflow progress events.
#[derive(Debug, Clone, Serialize)]
pub struct WebSocketMessage {
    /// Event type (e.g. `"workflow_started"`, `"workflow_progress"`).
    pub message_type: String,
    /// Workflow this message relates to, if applicable.
    pub workflow_id: Option<String>,
    /// WebSocket session this message relates to, if applicable.
    pub session_id: Option<String>,
    /// Event-specific payload.
    pub data: serde_json::Value,
    /// UTC timestamp when the message was emitted.
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Shared map of active and completed workflow sessions.
pub type WorkflowSessions = RwLock<HashMap<String, WorkflowStatus>>;
/// Broadcast channel for WebSocket workflow events.
pub type WebSocketBroadcaster = broadcast::Sender<WebSocketMessage>;

/// Builds the Axum router that mounts all workflow and WebSocket endpoints.
pub fn create_router() -> Router<AppState> {
    Router::new()
        // Workflow execution endpoints
        .route(
            "/workflows/prompt-chain",
            post(prompt_chain::execute_prompt_chain),
        )
        .route("/workflows/route", post(routing::execute_routing))
        .route("/workflows/parallel", post(parallel::execute_parallel))
        .route(
            "/workflows/orchestrate",
            post(orchestration::execute_orchestration),
        )
        .route(
            "/workflows/optimize",
            post(optimization::execute_optimization),
        )
        .route(
            "/workflows/vm-execution-demo",
            post(vm_execution::execute_vm_execution_demo),
        )
        // Workflow monitoring endpoints
        .route("/workflows/{id}/status", get(get_workflow_status))
        .route("/workflows/{id}/trace", get(get_execution_trace))
        .route("/workflows", get(list_workflows))
        // WebSocket endpoint
        .route("/ws", get(websocket::websocket_handler))
}

// Workflow monitoring handlers
async fn get_workflow_status(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<WorkflowStatus>, StatusCode> {
    let sessions = state.workflow_sessions.read().await;

    if let Some(status) = sessions.get(&id) {
        Ok(Json(status.clone()))
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

async fn get_execution_trace(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let sessions = state.workflow_sessions.read().await;

    if let Some(status) = sessions.get(&id) {
        // Return detailed execution trace
        let trace = serde_json::json!({
            "workflow_id": id,
            "status": status.status,
            "steps": [], // TODO: Implement detailed step tracking
            "timeline": {
                "started_at": status.started_at,
                "completed_at": status.completed_at
            },
            "performance": {
                "execution_time_ms": status.completed_at
                    .map(|end| (end - status.started_at).num_milliseconds())
                    .unwrap_or(0),
                "progress": status.progress
            }
        });

        Ok(Json(trace))
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

async fn list_workflows(State(state): State<AppState>) -> Json<Vec<WorkflowStatus>> {
    let sessions = state.workflow_sessions.read().await;
    let workflows: Vec<WorkflowStatus> = sessions.values().cloned().collect();
    Json(workflows)
}

/// Generates a unique workflow identifier prefixed with `workflow_`.
pub fn generate_workflow_id() -> String {
    format!("workflow_{}", Uuid::new_v4())
}

/// Updates the status and progress of a workflow session and broadcasts the change over WebSocket.
pub async fn update_workflow_status(
    sessions: &WorkflowSessions,
    broadcaster: &WebSocketBroadcaster,
    workflow_id: &str,
    status: ExecutionStatus,
    progress: f64,
    current_step: Option<String>,
) {
    let mut sessions = sessions.write().await;

    if let Some(workflow) = sessions.get_mut(workflow_id) {
        workflow.status = status.clone();
        workflow.progress = progress;
        workflow.current_step = current_step.clone();

        if matches!(status, ExecutionStatus::Completed | ExecutionStatus::Failed) {
            workflow.completed_at = Some(chrono::Utc::now());
        }

        // Broadcast update via WebSocket
        let message = WebSocketMessage {
            message_type: "workflow_progress".to_string(),
            workflow_id: Some(workflow_id.to_string()),
            session_id: None,
            data: serde_json::json!({
                "status": status,
                "progress": progress,
                "current_step": current_step
            }),
            timestamp: chrono::Utc::now(),
        };

        let _ = broadcaster.send(message);
    }
}

/// Creates a new workflow session and broadcasts a `workflow_started` event.
pub async fn create_workflow_session(
    sessions: &WorkflowSessions,
    broadcaster: &WebSocketBroadcaster,
    workflow_id: String,
    pattern: String,
) {
    let status = WorkflowStatus {
        id: workflow_id.clone(),
        status: ExecutionStatus::Running,
        progress: 0.0,
        current_step: Some("Initializing".to_string()),
        started_at: chrono::Utc::now(),
        completed_at: None,
        result: None,
        error: None,
    };

    sessions.write().await.insert(workflow_id.clone(), status);

    // Broadcast workflow started
    let message = WebSocketMessage {
        message_type: "workflow_started".to_string(),
        workflow_id: Some(workflow_id),
        session_id: None,
        data: serde_json::json!({
            "pattern": pattern,
            "started_at": chrono::Utc::now()
        }),
        timestamp: chrono::Utc::now(),
    };

    let _ = broadcaster.send(message);
}

/// Marks a workflow session as completed and broadcasts the result over WebSocket.
pub async fn complete_workflow_session(
    sessions: &WorkflowSessions,
    broadcaster: &WebSocketBroadcaster,
    workflow_id: String,
    result: serde_json::Value,
) {
    let mut sessions = sessions.write().await;

    if let Some(workflow) = sessions.get_mut(&workflow_id) {
        workflow.status = ExecutionStatus::Completed;
        workflow.progress = 100.0;
        workflow.completed_at = Some(chrono::Utc::now());
        workflow.result = Some(result.clone());

        // Broadcast completion
        let message = WebSocketMessage {
            message_type: "workflow_completed".to_string(),
            workflow_id: Some(workflow_id),
            session_id: None,
            data: serde_json::json!({
                "result": result,
                "completed_at": chrono::Utc::now(),
                "execution_time": workflow.completed_at
                    .map(|end| (end - workflow.started_at).num_milliseconds())
                    .unwrap_or(0)
            }),
            timestamp: chrono::Utc::now(),
        };

        let _ = broadcaster.send(message);
    }
}

/// Marks a workflow session as failed and broadcasts the error over WebSocket.
pub async fn fail_workflow_session(
    sessions: &WorkflowSessions,
    broadcaster: &WebSocketBroadcaster,
    workflow_id: String,
    error: String,
) {
    let mut sessions = sessions.write().await;

    if let Some(workflow) = sessions.get_mut(&workflow_id) {
        workflow.status = ExecutionStatus::Failed;
        workflow.completed_at = Some(chrono::Utc::now());
        workflow.error = Some(error.clone());

        // Broadcast error
        let message = WebSocketMessage {
            message_type: "workflow_error".to_string(),
            workflow_id: Some(workflow_id),
            session_id: None,
            data: serde_json::json!({
                "error": error,
                "failed_at": chrono::Utc::now()
            }),
            timestamp: chrono::Utc::now(),
        };

        let _ = broadcaster.send(message);
    }
}
