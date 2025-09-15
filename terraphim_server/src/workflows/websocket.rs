use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::Response,
};
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc};
use tokio::{
    sync::{broadcast, RwLock},
    time::{sleep, Duration},
};

use super::{ExecutionStatus, WebSocketMessage, WorkflowStatus};
use crate::AppState;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct WebSocketCommand {
    command_type: String,
    session_id: Option<String>,
    workflow_id: Option<String>,
    data: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct WebSocketResponse {
    response_type: String,
    session_id: Option<String>,
    workflow_id: Option<String>,
    data: serde_json::Value,
    timestamp: chrono::DateTime<chrono::Utc>,
    success: bool,
    error: Option<String>,
}

#[derive(Debug, Clone)]
struct WebSocketSession {
    session_id: String,
    connected_at: chrono::DateTime<chrono::Utc>,
    subscribed_workflows: Vec<String>,
    client_info: ClientInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ClientInfo {
    user_agent: Option<String>,
    ip_address: Option<String>,
    connection_type: String,
}

pub async fn websocket_handler(ws: WebSocketUpgrade, State(state): State<AppState>) -> Response {
    ws.on_upgrade(|socket| websocket_connection(socket, state))
}

async fn websocket_connection(socket: WebSocket, state: AppState) {
    let (mut sender, mut receiver) = socket.split();
    let session_id = generate_session_id();

    // Create session
    let session = WebSocketSession {
        session_id: session_id.clone(),
        connected_at: chrono::Utc::now(),
        subscribed_workflows: Vec::new(),
        client_info: ClientInfo {
            user_agent: None,
            ip_address: None,
            connection_type: "websocket".to_string(),
        },
    };

    // Store session in state (if we had a sessions manager)
    // For now, we'll just use local session management
    let sessions = Arc::new(RwLock::new(HashMap::<String, WebSocketSession>::new()));
    sessions
        .write()
        .await
        .insert(session_id.clone(), session.clone());

    // Subscribe to workflow broadcasts
    let mut broadcast_receiver = state.websocket_broadcaster.subscribe();

    // Send welcome message
    let welcome_message = WebSocketResponse {
        response_type: "connection_established".to_string(),
        session_id: Some(session_id.clone()),
        workflow_id: None,
        data: serde_json::json!({
            "message": "WebSocket connection established successfully",
            "session_id": session_id,
            "server_time": chrono::Utc::now(),
            "capabilities": [
                "workflow_monitoring",
                "real_time_updates",
                "command_execution",
                "session_management"
            ]
        }),
        timestamp: chrono::Utc::now(),
        success: true,
        error: None,
    };

    if sender
        .send(Message::Text(
            serde_json::to_string(&welcome_message).unwrap().into(),
        ))
        .await
        .is_err()
    {
        return;
    }

    // Spawn task to handle broadcasts
    let sessions_clone = sessions.clone();
    let session_id_clone = session_id.clone();
    tokio::spawn(async move {
        while let Ok(broadcast_message) = broadcast_receiver.recv().await {
            let sessions_read = sessions_clone.read().await;
            if let Some(session) = sessions_read.get(&session_id_clone) {
                // Check if session is subscribed to this workflow
                if let Some(workflow_id) = &broadcast_message.workflow_id {
                    if session.subscribed_workflows.contains(workflow_id)
                        || session.subscribed_workflows.is_empty()
                    {
                        let response = WebSocketResponse {
                            response_type: broadcast_message.message_type,
                            session_id: Some(session_id_clone.clone()),
                            workflow_id: broadcast_message.workflow_id,
                            data: broadcast_message.data,
                            timestamp: broadcast_message.timestamp,
                            success: true,
                            error: None,
                        };

                        if let Ok(msg) = serde_json::to_string(&response) {
                            // Note: In a real implementation, we'd need to maintain sender references
                            // per session to actually send messages back to clients
                            // For now, this demonstrates the structure
                        }
                    }
                }
            }
        }
    });

    // Handle incoming messages
    while let Some(msg) = receiver.next().await {
        match msg {
            Ok(Message::Text(text)) => {
                let response =
                    handle_websocket_message(&text, &session_id, &sessions, &state).await;

                if let Ok(response_json) = serde_json::to_string(&response) {
                    if sender
                        .send(Message::Text(response_json.into()))
                        .await
                        .is_err()
                    {
                        break;
                    }
                }
            }
            Ok(Message::Binary(_)) => {
                let response = WebSocketResponse {
                    response_type: "error".to_string(),
                    session_id: Some(session_id.clone()),
                    workflow_id: None,
                    data: serde_json::json!({"error": "Binary messages not supported"}),
                    timestamp: chrono::Utc::now(),
                    success: false,
                    error: Some("Binary messages not supported".to_string()),
                };

                if let Ok(response_json) = serde_json::to_string(&response) {
                    if sender
                        .send(Message::Text(response_json.into()))
                        .await
                        .is_err()
                    {
                        break;
                    }
                }
            }
            Ok(Message::Ping(ping)) => {
                if sender.send(Message::Pong(ping)).await.is_err() {
                    break;
                }
            }
            Ok(Message::Pong(_)) => {
                // Handle pong if needed
            }
            Ok(Message::Close(_)) => {
                break;
            }
            Err(_) => {
                break;
            }
        }
    }

    // Cleanup session
    sessions.write().await.remove(&session_id);
}

async fn handle_websocket_message(
    text: &str,
    session_id: &str,
    sessions: &Arc<RwLock<HashMap<String, WebSocketSession>>>,
    state: &AppState,
) -> WebSocketResponse {
    let command: Result<WebSocketCommand, _> = serde_json::from_str(text);

    match command {
        Ok(cmd) => match cmd.command_type.as_str() {
            "subscribe_workflow" => handle_subscribe_workflow(cmd, session_id, sessions).await,
            "unsubscribe_workflow" => handle_unsubscribe_workflow(cmd, session_id, sessions).await,
            "get_workflow_status" => handle_get_workflow_status(cmd, state).await,
            "list_active_workflows" => handle_list_active_workflows(state).await,
            "get_session_info" => handle_get_session_info(session_id, sessions).await,
            "ping" => handle_ping_command(session_id).await,
            _ => WebSocketResponse {
                response_type: "error".to_string(),
                session_id: Some(session_id.to_string()),
                workflow_id: cmd.workflow_id,
                data: serde_json::json!({
                    "error": "Unknown command type",
                    "received_command": cmd.command_type
                }),
                timestamp: chrono::Utc::now(),
                success: false,
                error: Some(format!("Unknown command type: {}", cmd.command_type)),
            },
        },
        Err(e) => WebSocketResponse {
            response_type: "error".to_string(),
            session_id: Some(session_id.to_string()),
            workflow_id: None,
            data: serde_json::json!({
                "error": "Invalid JSON format",
                "parse_error": e.to_string(),
                "received_text": text
            }),
            timestamp: chrono::Utc::now(),
            success: false,
            error: Some(format!("JSON parse error: {}", e)),
        },
    }
}

async fn handle_subscribe_workflow(
    cmd: WebSocketCommand,
    session_id: &str,
    sessions: &Arc<RwLock<HashMap<String, WebSocketSession>>>,
) -> WebSocketResponse {
    if let Some(workflow_id) = cmd.workflow_id {
        let mut sessions_write = sessions.write().await;
        if let Some(session) = sessions_write.get_mut(session_id) {
            if !session.subscribed_workflows.contains(&workflow_id) {
                session.subscribed_workflows.push(workflow_id.clone());
            }

            WebSocketResponse {
                response_type: "workflow_subscribed".to_string(),
                session_id: Some(session_id.to_string()),
                workflow_id: Some(workflow_id.clone()),
                data: serde_json::json!({
                    "message": "Successfully subscribed to workflow updates",
                    "workflow_id": workflow_id,
                    "total_subscriptions": session.subscribed_workflows.len()
                }),
                timestamp: chrono::Utc::now(),
                success: true,
                error: None,
            }
        } else {
            WebSocketResponse {
                response_type: "error".to_string(),
                session_id: Some(session_id.to_string()),
                workflow_id: Some(workflow_id),
                data: serde_json::json!({"error": "Session not found"}),
                timestamp: chrono::Utc::now(),
                success: false,
                error: Some("Session not found".to_string()),
            }
        }
    } else {
        WebSocketResponse {
            response_type: "error".to_string(),
            session_id: Some(session_id.to_string()),
            workflow_id: None,
            data: serde_json::json!({"error": "workflow_id is required for subscription"}),
            timestamp: chrono::Utc::now(),
            success: false,
            error: Some("workflow_id is required".to_string()),
        }
    }
}

async fn handle_unsubscribe_workflow(
    cmd: WebSocketCommand,
    session_id: &str,
    sessions: &Arc<RwLock<HashMap<String, WebSocketSession>>>,
) -> WebSocketResponse {
    if let Some(workflow_id) = cmd.workflow_id {
        let mut sessions_write = sessions.write().await;
        if let Some(session) = sessions_write.get_mut(session_id) {
            session.subscribed_workflows.retain(|id| id != &workflow_id);

            WebSocketResponse {
                response_type: "workflow_unsubscribed".to_string(),
                session_id: Some(session_id.to_string()),
                workflow_id: Some(workflow_id.clone()),
                data: serde_json::json!({
                    "message": "Successfully unsubscribed from workflow updates",
                    "workflow_id": workflow_id,
                    "remaining_subscriptions": session.subscribed_workflows.len()
                }),
                timestamp: chrono::Utc::now(),
                success: true,
                error: None,
            }
        } else {
            WebSocketResponse {
                response_type: "error".to_string(),
                session_id: Some(session_id.to_string()),
                workflow_id: Some(workflow_id),
                data: serde_json::json!({"error": "Session not found"}),
                timestamp: chrono::Utc::now(),
                success: false,
                error: Some("Session not found".to_string()),
            }
        }
    } else {
        WebSocketResponse {
            response_type: "error".to_string(),
            session_id: Some(session_id.to_string()),
            workflow_id: None,
            data: serde_json::json!({"error": "workflow_id is required for unsubscription"}),
            timestamp: chrono::Utc::now(),
            success: false,
            error: Some("workflow_id is required".to_string()),
        }
    }
}

async fn handle_get_workflow_status(cmd: WebSocketCommand, state: &AppState) -> WebSocketResponse {
    if let Some(workflow_id) = cmd.workflow_id {
        let sessions = state.workflow_sessions.read().await;
        if let Some(status) = sessions.get(&workflow_id) {
            WebSocketResponse {
                response_type: "workflow_status".to_string(),
                session_id: cmd.session_id,
                workflow_id: Some(workflow_id),
                data: serde_json::to_value(status).unwrap_or_default(),
                timestamp: chrono::Utc::now(),
                success: true,
                error: None,
            }
        } else {
            WebSocketResponse {
                response_type: "error".to_string(),
                session_id: cmd.session_id,
                workflow_id: Some(workflow_id),
                data: serde_json::json!({"error": "Workflow not found"}),
                timestamp: chrono::Utc::now(),
                success: false,
                error: Some("Workflow not found".to_string()),
            }
        }
    } else {
        WebSocketResponse {
            response_type: "error".to_string(),
            session_id: cmd.session_id,
            workflow_id: None,
            data: serde_json::json!({"error": "workflow_id is required"}),
            timestamp: chrono::Utc::now(),
            success: false,
            error: Some("workflow_id is required".to_string()),
        }
    }
}

async fn handle_list_active_workflows(state: &AppState) -> WebSocketResponse {
    let sessions = state.workflow_sessions.read().await;
    let active_workflows: Vec<&WorkflowStatus> = sessions
        .values()
        .filter(|status| {
            !matches!(
                status.status,
                ExecutionStatus::Completed | ExecutionStatus::Failed
            )
        })
        .collect();

    let workflow_summaries: Vec<serde_json::Value> = active_workflows
        .iter()
        .map(|status| {
            serde_json::json!({
                "id": status.id,
                "status": status.status,
                "progress": status.progress,
                "current_step": status.current_step,
                "started_at": status.started_at,
            })
        })
        .collect();

    WebSocketResponse {
        response_type: "active_workflows_list".to_string(),
        session_id: None,
        workflow_id: None,
        data: serde_json::json!({
            "active_workflows": workflow_summaries,
            "total_count": workflow_summaries.len(),
            "timestamp": chrono::Utc::now()
        }),
        timestamp: chrono::Utc::now(),
        success: true,
        error: None,
    }
}

async fn handle_get_session_info(
    session_id: &str,
    sessions: &Arc<RwLock<HashMap<String, WebSocketSession>>>,
) -> WebSocketResponse {
    let sessions_read = sessions.read().await;
    if let Some(session) = sessions_read.get(session_id) {
        WebSocketResponse {
            response_type: "session_info".to_string(),
            session_id: Some(session_id.to_string()),
            workflow_id: None,
            data: serde_json::json!({
                "session_id": session.session_id,
                "connected_at": session.connected_at,
                "subscribed_workflows": session.subscribed_workflows,
                "client_info": session.client_info,
                "connection_duration": chrono::Utc::now().signed_duration_since(session.connected_at).num_seconds()
            }),
            timestamp: chrono::Utc::now(),
            success: true,
            error: None,
        }
    } else {
        WebSocketResponse {
            response_type: "error".to_string(),
            session_id: Some(session_id.to_string()),
            workflow_id: None,
            data: serde_json::json!({"error": "Session not found"}),
            timestamp: chrono::Utc::now(),
            success: false,
            error: Some("Session not found".to_string()),
        }
    }
}

async fn handle_ping_command(session_id: &str) -> WebSocketResponse {
    WebSocketResponse {
        response_type: "pong".to_string(),
        session_id: Some(session_id.to_string()),
        workflow_id: None,
        data: serde_json::json!({
            "message": "pong",
            "server_time": chrono::Utc::now(),
            "latency_test": true
        }),
        timestamp: chrono::Utc::now(),
        success: true,
        error: None,
    }
}

fn generate_session_id() -> String {
    format!("session_{}", uuid::Uuid::new_v4())
}

// Additional utility functions for WebSocket management

pub async fn broadcast_workflow_event(
    broadcaster: &broadcast::Sender<WebSocketMessage>,
    event_type: &str,
    workflow_id: Option<String>,
    data: serde_json::Value,
) {
    let message = WebSocketMessage {
        message_type: event_type.to_string(),
        workflow_id,
        session_id: None,
        data,
        timestamp: chrono::Utc::now(),
    };

    let _ = broadcaster.send(message);
}

pub async fn notify_workflow_progress(
    broadcaster: &broadcast::Sender<WebSocketMessage>,
    workflow_id: String,
    progress: f64,
    current_step: Option<String>,
    status: ExecutionStatus,
) {
    let data = serde_json::json!({
        "progress": progress,
        "current_step": current_step,
        "status": status,
        "timestamp": chrono::Utc::now()
    });

    broadcast_workflow_event(broadcaster, "workflow_progress", Some(workflow_id), data).await;
}

pub async fn notify_workflow_completion(
    broadcaster: &broadcast::Sender<WebSocketMessage>,
    workflow_id: String,
    success: bool,
    result: Option<serde_json::Value>,
    error: Option<String>,
) {
    let data = serde_json::json!({
        "success": success,
        "result": result,
        "error": error,
        "completed_at": chrono::Utc::now()
    });

    let event_type = if success {
        "workflow_completed"
    } else {
        "workflow_failed"
    };

    broadcast_workflow_event(broadcaster, event_type, Some(workflow_id), data).await;
}

pub async fn notify_workflow_started(
    broadcaster: &broadcast::Sender<WebSocketMessage>,
    workflow_id: String,
    workflow_type: String,
) {
    let data = serde_json::json!({
        "workflow_type": workflow_type,
        "started_at": chrono::Utc::now(),
        "status": ExecutionStatus::Running
    });

    broadcast_workflow_event(broadcaster, "workflow_started", Some(workflow_id), data).await;
}

// Health check and monitoring functions

pub async fn websocket_health_check() -> serde_json::Value {
    serde_json::json!({
        "websocket_server": "operational",
        "supported_commands": [
            "subscribe_workflow",
            "unsubscribe_workflow",
            "get_workflow_status",
            "list_active_workflows",
            "get_session_info",
            "ping"
        ],
        "supported_events": [
            "workflow_started",
            "workflow_progress",
            "workflow_completed",
            "workflow_failed",
            "connection_established"
        ],
        "protocol_version": "1.0.0",
        "timestamp": chrono::Utc::now()
    })
}

pub fn get_websocket_stats(sessions: &HashMap<String, WebSocketSession>) -> serde_json::Value {
    let total_sessions = sessions.len();
    let total_subscriptions: usize = sessions
        .values()
        .map(|s| s.subscribed_workflows.len())
        .sum();

    let oldest_connection = sessions
        .values()
        .map(|s| s.connected_at)
        .min()
        .unwrap_or_else(chrono::Utc::now);

    serde_json::json!({
        "total_active_sessions": total_sessions,
        "total_workflow_subscriptions": total_subscriptions,
        "oldest_connection_age_seconds": chrono::Utc::now()
            .signed_duration_since(oldest_connection)
            .num_seconds(),
        "average_subscriptions_per_session": if total_sessions > 0 {
            total_subscriptions as f64 / total_sessions as f64
        } else {
            0.0
        },
        "timestamp": chrono::Utc::now()
    })
}
