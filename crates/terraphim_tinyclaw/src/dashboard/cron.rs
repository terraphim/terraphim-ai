//! Cron dashboard endpoints.
//!
//! - `POST /api/cron/fire` — Chronos managed-cron fire webhook
//! - `GET  /api/cron/jobs` — list jobs
//! - `POST /api/cron/jobs` — create a job
//! - `GET  /api/cron/jobs/{id}` — get a single job
//! - `DELETE /api/cron/jobs/{id}` — delete a job
//!
//! Hermes contracts ported from `web_server.py:12673-12729` (fire webhook)
//! and `cron/jobs.py` (CRUD).

use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use serde::Deserialize;
use serde_json::json;

use super::DashboardState;

/// Request body for `POST /api/cron/fire` (Hermes contract).
///
/// Use `serde_json::Value` so missing fields don't trigger axum's
/// auto-422 deserialization error. We validate manually.
#[derive(Debug, Deserialize)]
pub struct FireRequest {
    #[serde(default)]
    pub job_id: String,
}

/// `POST /api/cron/fire`
///
/// Hermes contract:
/// - Missing/invalid auth → 401 `{"error": "invalid fire token"}`
/// - Missing `job_id` → 400 `{"error": "missing job_id"}`
/// - Job not found → 200 `{"status": "gone", "job_id": "..."}`
/// - Valid → 202 `{"status": "accepted", "job_id": "..."}`
///
/// Auth: `Authorization: Bearer <FIRE_TOKEN>` header. The token is
/// supplied via `DashboardState::fire_token` (set from
/// `TINYCLAW_FIRE_TOKEN` env var at startup). When the state has no
/// token configured (dev/test), the endpoint is unauthenticated and
/// the caller is responsible for network-level isolation.
pub async fn fire_webhook(
    State(state): State<DashboardState>,
    headers: axum::http::HeaderMap,
    Json(body): Json<FireRequest>,
) -> impl IntoResponse {
    // Auth gate — per Hermes contract, refuse without a matching Bearer token.
    if let Some(expected) = state.fire_token.as_deref() {
        let provided = headers
            .get(axum::http::header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.strip_prefix("Bearer "));
        if provided != Some(expected) {
            return (
                StatusCode::UNAUTHORIZED,
                Json(json!({ "error": "invalid fire token" })),
            );
        }
    }

    let job_id = body.job_id;
    if job_id.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "missing job_id" })),
        );
    }

    // Look up the job across all cron stores (in our case, just one).
    match state.cron_store.get_job(&job_id).await {
        Ok(Some(_job)) => (
            StatusCode::ACCEPTED,
            Json(json!({ "status": "accepted", "job_id": job_id })),
        ),
        Ok(None) => (
            StatusCode::OK,
            Json(json!({ "status": "gone", "job_id": job_id })),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        ),
    }
}

/// `GET /api/cron/jobs`
pub async fn list_jobs(State(state): State<DashboardState>) -> impl IntoResponse {
    match state.cron_store.load_all().await {
        Ok(jobs) => Json(json!({ "count": jobs.len(), "jobs": jobs })).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

/// Request body for `POST /api/cron/jobs`.
#[derive(Debug, Deserialize)]
pub struct CreateJobRequest {
    #[serde(default)]
    pub prompt: String,
    #[serde(default)]
    pub schedule: Option<String>,
}

/// `POST /api/cron/jobs`
pub async fn create_job(
    State(state): State<DashboardState>,
    Json(body): Json<CreateJobRequest>,
) -> impl IntoResponse {
    use crate::cron::{CronJob, Schedule};

    let schedule = match body.schedule {
        Some(s) => match Schedule::parse(&s) {
            Ok(sched) => sched,
            Err(e) => {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(json!({ "error": format!("invalid schedule: {e}") })),
                )
                    .into_response();
            }
        },
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": "schedule is required" })),
            )
                .into_response();
        }
    };

    let job = CronJob::new(body.prompt, schedule);
    let job_id = job.id.clone();

    let mut jobs = state.cron_store.load_all().await.unwrap_or_default();
    jobs.push(job);

    match state.cron_store.save_all(&jobs).await {
        Ok(()) => (
            StatusCode::CREATED,
            Json(json!({ "id": job_id, "status": "created" })),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

/// `GET /api/cron/jobs/{id}`
pub async fn get_job(
    State(state): State<DashboardState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match state.cron_store.get_job(&id).await {
        Ok(Some(job)) => Json(json!(job)).into_response(),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": format!("job not found: {id}") })),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

/// `DELETE /api/cron/jobs/{id}`
pub async fn delete_job(
    State(state): State<DashboardState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match state.cron_store.remove_job(&id).await {
        Ok(true) => (
            StatusCode::OK,
            Json(json!({ "status": "deleted", "id": id })),
        )
            .into_response(),
        Ok(false) => (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": format!("job not found: {id}") })),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}
