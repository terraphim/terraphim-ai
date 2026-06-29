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

/// Multi-agent workflow HTTP handlers.
pub mod multi_agent_handlers;
/// Optimisation workflow HTTP handlers.
pub mod optimization;
/// Orchestration workflow HTTP handlers.
pub mod orchestration;
/// Parallel workflow HTTP handlers.
pub mod parallel;
/// Prompt-chain workflow HTTP handlers.
pub mod prompt_chain;
/// Routing workflow HTTP handlers.
pub mod routing;
/// VM-execution workflow HTTP handlers.
pub mod vm_execution;
/// WebSocket workflow HTTP handlers and broadcaster.
pub mod websocket;

/// LLM provider configuration used when executing a workflow step.
#[derive(Debug, Deserialize, Clone)]
pub struct LlmConfig {
    /// Provider identifier, e.g. `"ollama"` or `"openrouter"`.
    pub llm_provider: Option<String>,
    /// Model identifier understood by the chosen provider.
    pub llm_model: Option<String>,
    /// Base URL for the provider's API endpoint.
    pub llm_base_url: Option<String>,
    /// Sampling temperature; lower values produce more deterministic output.
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

/// Per-step LLM and prompt overrides within a multi-step workflow.
#[derive(Debug, Deserialize, Clone)]
pub struct StepConfig {
    /// Unique identifier for this step within the workflow.
    pub id: String,
    /// Human-readable name used in status messages.
    pub name: String,
    /// Prompt template sent to the LLM for this step.
    pub prompt: String,
    /// Knowledge-graph role to apply for this step; overrides the request-level role.
    pub role: Option<String>,
    /// System prompt to prepend for this step.
    pub system_prompt: Option<String>,
    /// LLM configuration override for this step.
    pub llm_config: Option<LlmConfig>,
}

/// Request body for all workflow execution endpoints.
#[derive(Debug, Deserialize)]
pub struct WorkflowRequest {
    /// User-supplied prompt that drives the workflow.
    pub prompt: String,
    /// Knowledge-graph role for context retrieval.
    pub role: Option<String>,
    /// Overall coordinator role used in orchestration workflows.
    pub overall_role: Option<String>,
    /// Workflow-specific configuration blob (schema varies per workflow type).
    pub config: Option<serde_json::Value>,
    /// LLM configuration override applied to all steps unless overridden per-step.
    pub llm_config: Option<LlmConfig>,
    /// Per-step configuration overrides.
    pub steps: Option<Vec<StepConfig>>,
}

/// Response body returned by all workflow execution endpoints.
#[derive(Debug, Serialize)]
pub struct WorkflowResponse {
    /// Unique identifier assigned to this workflow run.
    pub workflow_id: String,
    /// `true` when the workflow completed without error.
    pub success: bool,
    /// Structured result produced by the workflow; absent on failure.
    pub result: Option<serde_json::Value>,
    /// Human-readable error description; absent on success.
    pub error: Option<String>,
    /// Execution metadata for observability.
    pub metadata: WorkflowMetadata,
}

/// Execution metadata attached to every [`WorkflowResponse`].
#[derive(Debug, Serialize)]
pub struct WorkflowMetadata {
    /// Wall-clock time from start to completion in milliseconds.
    pub execution_time_ms: u64,
    /// Workflow pattern name, e.g. `"prompt_chain"`.
    pub pattern: String,
    /// Number of steps executed.
    pub steps: usize,
    /// Knowledge-graph role used.
    pub role: String,
    /// Overall coordinator role used.
    pub overall_role: String,
}

/// Live state of a workflow run tracked in [`WorkflowSessions`].
#[derive(Debug, Clone, Serialize)]
pub struct WorkflowStatus {
    /// Workflow run identifier.
    pub id: String,
    /// Current lifecycle phase.
    pub status: ExecutionStatus,
    /// Completion percentage in the range `[0.0, 100.0]`.
    pub progress: f64,
    /// Name of the step currently executing; absent when not running.
    pub current_step: Option<String>,
    /// UTC timestamp when the workflow was created.
    pub started_at: chrono::DateTime<chrono::Utc>,
    /// UTC timestamp when the workflow reached a terminal state; absent while running.
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
    /// Final result; absent until the workflow completes successfully.
    pub result: Option<serde_json::Value>,
    /// Error description; absent unless the workflow failed.
    pub error: Option<String>,
    /// Ordered list of steps recorded during execution.
    pub steps: Vec<WorkflowStep>,
}

/// Lifecycle phases of a workflow run.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionStatus {
    /// Accepted but not yet started.
    Pending,
    /// Currently executing steps.
    Running,
    /// All steps finished successfully.
    Completed,
    /// Execution aborted due to an error.
    Failed,
    /// Execution aborted by an external request.
    Cancelled,
}

/// Terminal state of a single workflow step.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StepStatus {
    /// Step is currently executing.
    Running,
    /// Step finished without error.
    Completed,
    /// Step aborted due to an error.
    Failed,
}

/// A recorded execution step within a workflow run.
#[derive(Debug, Clone, Serialize)]
pub struct WorkflowStep {
    /// Step identifier, unique within a workflow run.
    pub id: String,
    /// Human-readable step name.
    pub name: String,
    /// Terminal lifecycle state of this step.
    pub status: StepStatus,
    /// UTC timestamp when this step started.
    pub started_at: chrono::DateTime<chrono::Utc>,
    /// UTC timestamp when this step finished; absent while the step is running.
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
    /// Output produced by this step; absent when not available.
    pub output: Option<String>,
}

/// Event broadcast over WebSocket to connected clients.
#[derive(Debug, Clone, Serialize)]
pub struct WebSocketMessage {
    /// Event kind, e.g. `"workflow_progress"` or `"workflow_completed"`.
    pub message_type: String,
    /// Workflow run identifier when the event relates to a specific run.
    pub workflow_id: Option<String>,
    /// WebSocket session identifier when the event is session-scoped.
    pub session_id: Option<String>,
    /// Event payload; schema varies with `message_type`.
    pub data: serde_json::Value,
    /// UTC timestamp when the event was produced.
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Shared map of active workflow runs keyed by workflow ID.
pub type WorkflowSessions = RwLock<HashMap<String, WorkflowStatus>>;
/// Multi-sender channel for broadcasting [`WebSocketMessage`] events.
pub type WebSocketBroadcaster = broadcast::Sender<WebSocketMessage>;

/// Builds the Axum sub-router for all workflow and WebSocket endpoints.
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
        let trace = serde_json::json!({
            "workflow_id": id,
            "status": status.status,
            "steps": status.steps,
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

/// Appends a completed step record to an existing workflow run.
pub async fn record_workflow_step(
    sessions: &WorkflowSessions,
    workflow_id: &str,
    step: WorkflowStep,
) {
    let mut sessions = sessions.write().await;
    if let Some(workflow) = sessions.get_mut(workflow_id) {
        workflow.steps.push(step);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn trace_includes_recorded_steps() {
        let sessions = RwLock::new(HashMap::new());
        let (broadcaster, _rx) = broadcast::channel(16);
        let wf_id = "wf_test_123".to_string();

        create_workflow_session(
            &sessions,
            &broadcaster,
            wf_id.clone(),
            "test_pattern".to_string(),
        )
        .await;

        let step = WorkflowStep {
            id: "step_1".to_string(),
            name: "Parse Input".to_string(),
            status: StepStatus::Completed,
            started_at: chrono::Utc::now(),
            completed_at: Some(chrono::Utc::now()),
            output: Some("parsed successfully".to_string()),
        };
        record_workflow_step(&sessions, &wf_id, step).await;

        let guard = sessions.read().await;
        let wf = guard.get(&wf_id).expect("workflow should exist");
        assert_eq!(wf.steps.len(), 1);
        assert_eq!(wf.steps[0].name, "Parse Input");
        assert_eq!(wf.steps[0].id, "step_1");
        assert!(matches!(wf.steps[0].status, StepStatus::Completed));
    }

    #[tokio::test]
    async fn trace_starts_with_empty_steps() {
        let sessions = RwLock::new(HashMap::new());
        let (broadcaster, _rx) = broadcast::channel(16);
        let wf_id = "wf_empty_456".to_string();

        create_workflow_session(
            &sessions,
            &broadcaster,
            wf_id.clone(),
            "test_pattern".to_string(),
        )
        .await;

        let guard = sessions.read().await;
        let wf = guard.get(&wf_id).expect("workflow should exist");
        assert!(wf.steps.is_empty());
    }

    #[tokio::test]
    async fn record_multiple_steps_preserves_order() {
        let sessions = RwLock::new(HashMap::new());
        let (broadcaster, _rx) = broadcast::channel(16);
        let wf_id = "wf_multi_789".to_string();

        create_workflow_session(
            &sessions,
            &broadcaster,
            wf_id.clone(),
            "test_pattern".to_string(),
        )
        .await;

        for i in 1..=3u32 {
            let step = WorkflowStep {
                id: format!("step_{i}"),
                name: format!("Step {i}"),
                status: StepStatus::Completed,
                started_at: chrono::Utc::now(),
                completed_at: Some(chrono::Utc::now()),
                output: None,
            };
            record_workflow_step(&sessions, &wf_id, step).await;
        }

        let guard = sessions.read().await;
        let wf = guard.get(&wf_id).expect("workflow should exist");
        assert_eq!(wf.steps.len(), 3);
        assert_eq!(wf.steps[0].name, "Step 1");
        assert_eq!(wf.steps[2].name, "Step 3");
    }
}

async fn list_workflows(State(state): State<AppState>) -> Json<Vec<WorkflowStatus>> {
    let sessions = state.workflow_sessions.read().await;
    let workflows: Vec<WorkflowStatus> = sessions.values().cloned().collect();
    Json(workflows)
}

/// Generates a collision-resistant workflow run identifier.
pub fn generate_workflow_id() -> String {
    format!("workflow_{}", Uuid::new_v4())
}

/// Updates the status of an existing workflow run and broadcasts the change via WebSocket.
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

/// Registers a new workflow run as `Running` and broadcasts a `workflow_started` event.
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
        steps: vec![],
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

/// Marks a workflow run as `Completed`, stores the result, and broadcasts a `workflow_completed` event.
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

/// Marks a workflow run as `Failed`, stores the error, and broadcasts a `workflow_failed` event.
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
