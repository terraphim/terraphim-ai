use axum::{
    extract::{Path, State, WebSocketUpgrade},
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::sync::{broadcast, RwLock};
use uuid::Uuid;

use crate::AppState;

pub mod optimization;
pub mod orchestration;
pub mod parallel;
pub mod prompt_chain;
pub mod routing;
pub mod websocket;

// Workflow execution request/response types
#[derive(Debug, Deserialize)]
pub struct WorkflowRequest {
    pub prompt: String,
    pub role: Option<String>,
    pub overall_role: Option<String>,
    pub config: Option<serde_json::Value>,
}

#[derive(Debug, Serialize)]
pub struct WorkflowResponse {
    pub workflow_id: String,
    pub success: bool,
    pub result: Option<serde_json::Value>,
    pub error: Option<String>,
    pub metadata: WorkflowMetadata,
}

#[derive(Debug, Serialize)]
pub struct WorkflowMetadata {
    pub execution_time_ms: u64,
    pub pattern: String,
    pub steps: usize,
    pub role: String,
    pub overall_role: String,
}

// Workflow status types
#[derive(Debug, Clone, Serialize)]
pub struct WorkflowStatus {
    pub id: String,
    pub status: ExecutionStatus,
    pub progress: f64,
    pub current_step: Option<String>,
    pub started_at: chrono::DateTime<chrono::Utc>,
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
    pub result: Option<serde_json::Value>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
}

// WebSocket message types
#[derive(Debug, Clone, Serialize)]
pub struct WebSocketMessage {
    pub message_type: String,
    pub workflow_id: Option<String>,
    pub session_id: Option<String>,
    pub data: serde_json::Value,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

// Workflow session management
pub type WorkflowSessions = RwLock<HashMap<String, WorkflowStatus>>;
pub type WebSocketBroadcaster = broadcast::Sender<WebSocketMessage>;

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

// Utility functions
pub fn generate_workflow_id() -> String {
    format!("workflow_{}", Uuid::new_v4())
}

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
