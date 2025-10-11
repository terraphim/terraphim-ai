use axum::{extract::State, http::StatusCode, response::Json};
// use serde_json::json; // Unused import

use super::{
    complete_workflow_session, create_workflow_session, fail_workflow_session,
    generate_workflow_id, multi_agent_handlers::MultiAgentWorkflowExecutor,
    WorkflowMetadata, WorkflowRequest, WorkflowResponse,
};
use crate::AppState;

pub async fn execute_vm_execution_demo(
    State(state): State<AppState>,
    Json(request): Json<WorkflowRequest>,
) -> Result<Json<WorkflowResponse>, StatusCode> {
    let start_time = std::time::Instant::now();
    let workflow_id = generate_workflow_id();
    let role = request
        .role
        .unwrap_or_else(|| "DevelopmentAgent".to_string());
    let overall_role = request
        .overall_role
        .unwrap_or_else(|| "DevelopmentAgent".to_string());

    create_workflow_session(
        &state.workflow_sessions,
        &state.websocket_broadcaster,
        workflow_id.clone(),
        "vm_execution".to_string(),
    )
    .await;

    let result = match MultiAgentWorkflowExecutor::new_with_config(state.config_state.clone()).await
    {
        Ok(executor) => executor
            .execute_vm_execution_demo(
                &workflow_id,
                &request.prompt,
                &role,
                &overall_role,
                &state.workflow_sessions,
                &state.websocket_broadcaster,
                request.config.clone(),
            )
            .await
            .map_err(|e| e.to_string()),
        Err(e) => {
            log::error!("Failed to create multi-agent executor: {:?}", e);
            Err(format!("Failed to initialize multi-agent system: {}", e))
        }
    };

    let execution_time = start_time.elapsed().as_millis() as u64;

    match result {
        Ok(exec_result) => {
            complete_workflow_session(
                &state.workflow_sessions,
                &state.websocket_broadcaster,
                workflow_id.clone(),
                exec_result.clone(),
            )
            .await;

            Ok(Json(WorkflowResponse {
                workflow_id,
                success: true,
                result: Some(exec_result.clone()),
                error: None,
                metadata: WorkflowMetadata {
                    execution_time_ms: execution_time,
                    pattern: "vm_execution".to_string(),
                    steps: exec_result["execution_summary"]["code_blocks_executed"]
                        .as_u64()
                        .unwrap_or(1) as usize,
                    role,
                    overall_role,
                },
            }))
        }
        Err(error) => {
            fail_workflow_session(
                &state.workflow_sessions,
                &state.websocket_broadcaster,
                workflow_id.clone(),
                error.clone(),
            )
            .await;

            Ok(Json(WorkflowResponse {
                workflow_id,
                success: false,
                result: None,
                error: Some(error),
                metadata: WorkflowMetadata {
                    execution_time_ms: execution_time,
                    pattern: "vm_execution".to_string(),
                    steps: 0,
                    role,
                    overall_role,
                },
            }))
        }
    }
}
