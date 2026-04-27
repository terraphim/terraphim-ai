//! Workflow execution endpoints and shared workflow state.
//!
//! This module exposes the request and response types used by the workflow
//! HTTP handlers together with the in-memory session store and router factory.

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

/// Multi-agent coordination handlers.
pub mod multi_agent_handlers;
/// Optimisation workflow pattern.
pub mod optimization;
/// Orchestration workflow pattern.
pub mod orchestration;
/// Parallel workflow pattern.
pub mod parallel;
/// Prompt-chain workflow pattern.
pub mod prompt_chain;
/// Routing workflow pattern.
pub mod routing;
/// VM execution workflow pattern.
pub mod vm_execution;
/// WebSocket upgrade and subscription handlers.
pub mod websocket;

/// LLM configuration used by workflow execution steps.
#[derive(Debug, Deserialize, Clone)]
pub struct LlmConfig {
    /// LLM provider identifier (e.g. `"ollama"`, `"openrouter"`).
    pub llm_provider: Option<String>,
    /// Model name to request from the provider.
    pub llm_model: Option<String>,
    /// Base URL for the provider's API endpoint.
    pub llm_base_url: Option<String>,
    /// Sampling temperature controlling output randomness.
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

/// Per-step workflow configuration.
#[derive(Debug, Deserialize, Clone)]
pub struct StepConfig {
    /// Unique identifier for this step.
    pub id: String,
    /// Human-readable step label.
    pub name: String,
    /// Prompt template to send to the LLM for this step.
    pub prompt: String,
    /// Optional role override for this step.
    pub role: Option<String>,
    /// Optional system prompt to prepend for this step.
    pub system_prompt: Option<String>,
    /// Optional LLM configuration override for this step.
    pub llm_config: Option<LlmConfig>,
}

/// Request payload for workflow execution.
#[derive(Debug, Deserialize)]
pub struct WorkflowRequest {
    /// User-facing prompt to drive the workflow.
    pub prompt: String,
    /// Role name to use for this execution.
    pub role: Option<String>,
    /// Overall role applied across the entire workflow.
    pub overall_role: Option<String>,
    /// Arbitrary extra configuration as JSON.
    pub config: Option<serde_json::Value>,
    /// Global LLM configuration overriding crate defaults.
    pub llm_config: Option<LlmConfig>,
    /// Per-step configuration list.
    pub steps: Option<Vec<StepConfig>>,
}

/// Response returned by workflow execution endpoints.
#[derive(Debug, Serialize)]
pub struct WorkflowResponse {
    /// Unique identifier for the executed workflow session.
    pub workflow_id: String,
    /// Whether the workflow completed without error.
    pub success: bool,
    /// Result payload produced by the workflow.
    pub result: Option<serde_json::Value>,
    /// Error message if the workflow failed.
    pub error: Option<String>,
    /// Execution metadata including timing and step counts.
    pub metadata: WorkflowMetadata,
}

/// Metadata returned alongside workflow execution results.
#[derive(Debug, Serialize)]
pub struct WorkflowMetadata {
    /// Wall-clock time the workflow took to complete, in milliseconds.
    pub execution_time_ms: u64,
    /// Workflow pattern identifier (e.g. `"prompt_chain"`, `"parallel"`).
    pub pattern: String,
    /// Number of steps executed.
    pub steps: usize,
    /// Role used for this execution.
    pub role: String,
    /// Overall role applied across the workflow.
    pub overall_role: String,
}

/// Current status for a workflow session.
#[derive(Debug, Clone, Serialize)]
pub struct WorkflowStatus {
    /// Workflow session identifier.
    pub id: String,
    /// Current lifecycle state of the workflow.
    pub status: ExecutionStatus,
    /// Completion percentage in the range `[0.0, 100.0]`.
    pub progress: f64,
    /// Name of the step currently executing, if any.
    pub current_step: Option<String>,
    /// UTC timestamp when the workflow started.
    pub started_at: chrono::DateTime<chrono::Utc>,
    /// UTC timestamp when the workflow finished, if complete.
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
    /// Result payload once the workflow has completed.
    pub result: Option<serde_json::Value>,
    /// Error message if the workflow failed.
    pub error: Option<String>,
}

/// Lifecycle state for a workflow session.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionStatus {
    /// Workflow has been created but not yet started.
    Pending,
    /// Workflow is actively executing.
    Running,
    /// Workflow finished successfully.
    Completed,
    /// Workflow finished with an error.
    Failed,
    /// Workflow was cancelled before completion.
    Cancelled,
}

/// Broadcast message sent to WebSocket subscribers.
#[derive(Debug, Clone, Serialize)]
pub struct WebSocketMessage {
    /// Event type string (e.g. `"workflow_started"`, `"step_completed"`).
    pub message_type: String,
    /// Identifier of the workflow that triggered this message, if applicable.
    pub workflow_id: Option<String>,
    /// Identifier of the WebSocket session this message targets, if applicable.
    pub session_id: Option<String>,
    /// Arbitrary event payload as JSON.
    pub data: serde_json::Value,
    /// UTC timestamp when this message was emitted.
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// In-memory workflow sessions keyed by workflow identifier.
pub type WorkflowSessions = RwLock<HashMap<String, WorkflowStatus>>;
/// Broadcast channel for workflow WebSocket messages.
pub type WebSocketBroadcaster = broadcast::Sender<WebSocketMessage>;

/// Build the workflow router and mount all workflow endpoints.
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

/// Generate a unique workflow identifier.
pub fn generate_workflow_id() -> String {
    format!("workflow_{}", Uuid::new_v4())
}

/// Update a workflow session and broadcast the new state.
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

/// Create a new workflow session entry and broadcast a `workflow_started` event.
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

/// Mark a workflow session as completed and broadcast the result.
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

/// Mark a workflow session as failed and broadcast the error.
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
