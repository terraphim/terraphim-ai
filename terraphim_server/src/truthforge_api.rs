use ahash::AHashMap;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use uuid::Uuid;

use crate::{
    error::{Result, Status},
    workflows::WebSocketMessage,
    AppState,
};
use terraphim_multi_agent::GenAiLlmClient;
use terraphim_truthforge::{
    AudienceType, NarrativeContext, NarrativeInput, StakeType, TruthForgeAnalysisResult,
    TwoPassDebateWorkflow, UrgencyLevel,
};

fn emit_progress(
    broadcaster: &crate::workflows::WebSocketBroadcaster,
    session_id: Uuid,
    stage: &str,
    data: serde_json::Value,
) {
    let message = WebSocketMessage {
        message_type: "truthforge_progress".to_string(),
        workflow_id: None,
        session_id: Some(session_id.to_string()),
        data: serde_json::json!({
            "stage": stage,
            "details": data
        }),
        timestamp: chrono::Utc::now(),
    };

    let _ = broadcaster.send(message);
}

#[derive(Clone)]
pub struct SessionStore {
    sessions: Arc<RwLock<AHashMap<Uuid, TruthForgeAnalysisResult>>>,
}

impl SessionStore {
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(RwLock::new(AHashMap::new())),
        }
    }

    pub async fn store(&self, result: TruthForgeAnalysisResult) {
        let mut sessions = self.sessions.write().await;
        sessions.insert(result.session_id, result);
    }

    pub async fn get(&self, session_id: &Uuid) -> Option<TruthForgeAnalysisResult> {
        let sessions = self.sessions.read().await;
        sessions.get(session_id).cloned()
    }

    pub async fn list(&self) -> Vec<Uuid> {
        let sessions = self.sessions.read().await;
        sessions.keys().copied().collect()
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AnalyzeNarrativeRequest {
    pub text: String,
    pub urgency: Option<UrgencyLevel>,
    pub stakes: Option<Vec<StakeType>>,
    pub audience: Option<AudienceType>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AnalyzeNarrativeResponse {
    pub status: Status,
    pub session_id: Uuid,
    pub analysis_url: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GetAnalysisResponse {
    pub status: Status,
    pub result: Option<TruthForgeAnalysisResult>,
    pub error: Option<String>,
}

pub async fn analyze_narrative(
    State(app_state): State<AppState>,
    Json(request): Json<AnalyzeNarrativeRequest>,
) -> Result<Json<AnalyzeNarrativeResponse>> {
    log::info!(
        "TruthForge: Analyzing narrative ({} chars)",
        request.text.len()
    );

    let session_id = Uuid::new_v4();

    let narrative = NarrativeInput {
        session_id,
        text: request.text,
        context: NarrativeContext {
            urgency: request.urgency.unwrap_or(UrgencyLevel::Low),
            stakes: request
                .stakes
                .unwrap_or_else(|| vec![StakeType::Reputational]),
            audience: request.audience.unwrap_or(AudienceType::Internal),
        },
        submitted_at: chrono::Utc::now(),
    };

    log::info!("TruthForge: Checking for OPENROUTER_API_KEY environment variable...");
    let api_key_present = std::env::var("OPENROUTER_API_KEY").is_ok();
    log::info!(
        "TruthForge: OPENROUTER_API_KEY present: {}",
        api_key_present
    );

    let llm_client = if api_key_present {
        log::info!("TruthForge: Attempting to create OpenRouter client...");
        match GenAiLlmClient::new_openrouter(None) {
            Ok(client) => {
                log::info!("TruthForge: Successfully created OpenRouter LLM client");
                Some(Arc::new(client))
            }
            Err(e) => {
                log::error!("TruthForge: Failed to create OpenRouter client: {}", e);
                None
            }
        }
    } else {
        log::warn!("TruthForge: OPENROUTER_API_KEY not set, using mock implementation");
        None
    };

    let workflow = if let Some(client) = llm_client {
        TwoPassDebateWorkflow::new().with_llm_client(client)
    } else {
        TwoPassDebateWorkflow::new()
    };

    let session_store = app_state.truthforge_sessions.clone();
    let broadcaster = app_state.websocket_broadcaster.clone();

    tokio::spawn(async move {
        emit_progress(
            &broadcaster,
            session_id,
            "started",
            serde_json::json!({
                "message": "Analysis workflow initiated",
                "narrative_length": narrative.text.len()
            }),
        );

        match workflow.execute(&narrative).await {
            Ok(result) => {
                log::info!(
                    "TruthForge analysis complete for session {}: {} omissions, {} strategies",
                    session_id,
                    result.omission_catalog.omissions.len(),
                    result.response_strategies.len()
                );

                emit_progress(
                    &broadcaster,
                    session_id,
                    "completed",
                    serde_json::json!({
                        "omissions_count": result.omission_catalog.omissions.len(),
                        "strategies_count": result.response_strategies.len(),
                        "total_risk_score": result.omission_catalog.total_risk_score,
                        "processing_time_ms": result.processing_time_ms
                    }),
                );

                session_store.store(result).await;
            }
            Err(e) => {
                log::error!(
                    "TruthForge analysis failed for session {}: {}",
                    session_id,
                    e
                );

                emit_progress(
                    &broadcaster,
                    session_id,
                    "failed",
                    serde_json::json!({
                        "error": e.to_string()
                    }),
                );
            }
        }
    });

    Ok(Json(AnalyzeNarrativeResponse {
        status: Status::Success,
        session_id,
        analysis_url: format!("/api/v1/truthforge/{}", session_id),
    }))
}

pub async fn get_analysis(
    State(app_state): State<AppState>,
    Path(session_id): Path<Uuid>,
) -> Result<Json<GetAnalysisResponse>> {
    log::info!("TruthForge: Retrieving analysis for session {}", session_id);

    let result = app_state.truthforge_sessions.get(&session_id).await;

    Ok(Json(GetAnalysisResponse {
        status: Status::Success,
        result,
        error: None,
    }))
}

pub async fn list_analyses(State(app_state): State<AppState>) -> Result<Json<Vec<Uuid>>> {
    log::info!("TruthForge: Listing all analyses");
    let sessions = app_state.truthforge_sessions.list().await;
    Ok(Json(sessions))
}
