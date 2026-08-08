//! Hermetic contract tests for the dashboard.
//!
//! Ports of Hermes' `hermes_cli/web_server.py` endpoints:
//! - `GET /api/health` (web_server.py:3064-3072)
//! - `GET /api/status` (web_server.py:3074-3457)
//! - `POST /api/cron/fire` (web_server.py:12673-12729)
//! - `GET/POST /api/cron/jobs` (cron/jobs.py CRUD)
//! - `GET/DELETE /api/cron/jobs/{id}`
//! - `GET /api/sessions`

use axum::body::Body;
use axum::http::{Request, StatusCode};
use serde_json::{Value, json};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use terraphim_tinyclaw::bus::MessageBus;
use terraphim_tinyclaw::dashboard::{DashboardState, router};
use terraphim_tinyclaw::session::SessionManager;
use tokio::sync::Mutex;
use tower::ServiceExt; // for oneshot

async fn make_app() -> (DashboardState, axum::Router) {
    use terraphim_persistence::DeviceStorage;
    use terraphim_tinyclaw::cron::CronStore;
    use uuid::Uuid;

    let _ = DeviceStorage::init_memory_only().await;
    let storage = DeviceStorage::arc_memory_only().await.unwrap();
    // Unique key per test to avoid cross-test interference on the shared
    // in-memory DeviceStorage singleton.
    let key = format!("dashboard_cron_jobs_{}", Uuid::new_v4().simple());
    let cron_store = CronStore::new(storage, key);
    let state = DashboardState {
        sessions: Arc::new(Mutex::new(SessionManager::new(PathBuf::from("/tmp")))),
        bus: Arc::new(MessageBus::new()),
        cron_store,
        auth_required: false,
    };
    let app = router(state.clone());
    (state, app)
}

async fn send_json(
    app: axum::Router,
    method: &str,
    path: &str,
    body: Option<Value>,
) -> (StatusCode, Value) {
    let mut builder = Request::builder().method(method).uri(path);
    let body = match body {
        Some(v) => {
            builder = builder.header("content-type", "application/json");
            Body::from(serde_json::to_vec(&v).unwrap())
        }
        None => Body::empty(),
    };
    let req = builder.body(body).unwrap();
    let resp = app.oneshot(req).await.unwrap();
    let status = resp.status();
    let bytes = axum::body::to_bytes(resp.into_body(), 1024 * 1024)
        .await
        .unwrap();
    let parsed: Value = if bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&bytes).unwrap_or(Value::Null)
    };
    (status, parsed)
}

// --- /api/health -----------------------------------------------------------

#[tokio::test]
async fn contract_health_returns_ok_true() {
    let (_state, app) = make_app().await;
    let (status, body) = send_json(app, "GET", "/api/health", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["ok"], true);
}

#[tokio::test]
async fn contract_health_returns_version_field() {
    let (_state, app) = make_app().await;
    let (status, body) = send_json(app, "GET", "/api/health", None).await;
    assert_eq!(status, StatusCode::OK);
    assert!(body["version"].is_string());
    assert!(!body["version"].as_str().unwrap().is_empty());
}

#[tokio::test]
async fn contract_health_includes_auth_required_flag() {
    let (_state, app) = make_app().await;
    let (status, body) = send_json(app, "GET", "/api/health", None).await;
    assert_eq!(status, StatusCode::OK);
    assert!(body["auth_required"].is_boolean());
}

// --- /api/status -----------------------------------------------------------

#[tokio::test]
async fn contract_status_returns_components_dict() {
    // Hermes contract: returns counts/enums only, no secrets
    let (_state, app) = make_app().await;
    let (status, body) = send_json(app, "GET", "/api/status", None).await;
    assert_eq!(status, StatusCode::OK);
    assert!(body["components"].is_object(), "missing components dict");
    assert!(body["components"]["sessions"].is_object());
    assert!(body["components"]["cron"].is_object());
    assert!(body["components"]["channels"].is_object());
    assert!(body["components"]["mcp"].is_object());
}

#[tokio::test]
async fn contract_status_profiles_is_list() {
    let (_state, app) = make_app().await;
    let (_status, body) = send_json(app, "GET", "/api/status", None).await;
    assert!(body["profiles"].is_array());
    assert!(!body["profiles"].as_array().unwrap().is_empty());
}

// --- /api/cron/fire -------------------------------------------------------

#[tokio::test]
async fn contract_cron_fire_missing_job_id_returns_400() {
    // Hermes contract: missing job_id → 400 {"error": "missing job_id"}
    let (_state, app) = make_app().await;
    let (status, body) = send_json(app, "POST", "/api/cron/fire", Some(json!({}))).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(body["error"].as_str().unwrap().contains("missing job_id"));
}

#[tokio::test]
async fn contract_cron_fire_unknown_job_returns_200_gone() {
    // Hermes contract: job not found → 200 {"status": "gone", "job_id": "..."}
    let (_state, app) = make_app().await;
    let (status, body) = send_json(
        app,
        "POST",
        "/api/cron/fire",
        Some(json!({ "job_id": "ghost" })),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "gone");
    assert_eq!(body["job_id"], "ghost");
}

#[tokio::test]
async fn contract_cron_fire_known_job_returns_202_accepted() {
    // Hermes contract: valid → 202 {"status": "accepted", "job_id": "..."}
    let (_state, app) = make_app().await;

    // First create a job via the CRUD endpoint
    let (_create_status, created) = send_json(
        app.clone(),
        "POST",
        "/api/cron/jobs",
        Some(json!({
            "prompt": "test",
            "schedule": "every 5m"
        })),
    )
    .await;
    let job_id = created["id"].as_str().unwrap().to_string();

    // Then fire it
    let (status, body) = send_json(
        app,
        "POST",
        "/api/cron/fire",
        Some(json!({ "job_id": job_id })),
    )
    .await;
    assert_eq!(status, StatusCode::ACCEPTED);
    assert_eq!(body["status"], "accepted");
    assert!(body["job_id"].is_string());
}

// --- /api/cron/jobs CRUD ---------------------------------------------------

#[tokio::test]
async fn contract_cron_list_jobs_returns_array() {
    let (_state, app) = make_app().await;
    let (status, body) = send_json(app, "GET", "/api/cron/jobs", None).await;
    assert_eq!(status, StatusCode::OK);
    assert!(body["jobs"].is_array());
    assert_eq!(body["count"], body["jobs"].as_array().unwrap().len());
}

#[tokio::test]
async fn contract_cron_create_job_with_delay_schedule() {
    let (_state, app) = make_app().await;
    let (status, body) = send_json(
        app,
        "POST",
        "/api/cron/jobs",
        Some(json!({
            "prompt": "test prompt",
            "schedule": "30m"
        })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    assert!(body["id"].is_string());
    assert_eq!(body["status"], "created");
}

#[tokio::test]
async fn contract_cron_create_job_with_cron_schedule() {
    let (_state, app) = make_app().await;
    let (status, body) = send_json(
        app,
        "POST",
        "/api/cron/jobs",
        Some(json!({
            "prompt": "daily briefing",
            "schedule": "0 9 * * *"
        })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    assert!(body["id"].is_string());
}

#[tokio::test]
async fn contract_cron_create_job_rejects_invalid_schedule() {
    let (_state, app) = make_app().await;
    let (status, body) = send_json(
        app,
        "POST",
        "/api/cron/jobs",
        Some(json!({
            "prompt": "test",
            "schedule": "this is not a valid schedule"
        })),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(body["error"].as_str().unwrap().contains("invalid"));
}

#[tokio::test]
async fn contract_cron_create_job_requires_schedule() {
    let (_state, app) = make_app().await;
    let (status, body) = send_json(
        app,
        "POST",
        "/api/cron/jobs",
        Some(json!({
            "prompt": "test"
        })),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(body["error"].is_string());
}

#[tokio::test]
async fn contract_cron_get_job_404_when_missing() {
    let (_state, app) = make_app().await;
    let (status, body) = send_json(app, "GET", "/api/cron/jobs/ghost", None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert!(body["error"].as_str().unwrap().contains("not found"));
}

#[tokio::test]
async fn contract_cron_get_job_returns_full_record() {
    let (_state, app) = make_app().await;
    let (_status, created) = send_json(
        app.clone(),
        "POST",
        "/api/cron/jobs",
        Some(json!({
            "prompt": "test prompt",
            "schedule": "every 1h"
        })),
    )
    .await;
    let id = created["id"].as_str().unwrap().to_string();

    let (status, body) = send_json(app, "GET", &format!("/api/cron/jobs/{id}"), None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["id"], id);
    assert_eq!(body["prompt"], "test prompt");
}

#[tokio::test]
async fn contract_cron_delete_job_returns_deleted_status() {
    let (_state, app) = make_app().await;
    let (create_status, created) = send_json(
        app.clone(),
        "POST",
        "/api/cron/jobs",
        Some(json!({
            "prompt": "test",
            "schedule": "1h"
        })),
    )
    .await;
    assert_eq!(
        create_status,
        StatusCode::CREATED,
        "create failed: {created}"
    );
    let id = created["id"].as_str().unwrap().to_string();

    let (status, body) = send_json(app, "DELETE", &format!("/api/cron/jobs/{id}"), None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "deleted");
    assert_eq!(body["id"], id);
}

#[tokio::test]
async fn contract_cron_delete_job_404_when_missing() {
    let (_state, app) = make_app().await;
    let (status, body) = send_json(app, "DELETE", "/api/cron/jobs/ghost", None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert!(body["error"].as_str().unwrap().contains("not found"));
}

// --- /api/sessions ---------------------------------------------------------

#[tokio::test]
async fn contract_sessions_returns_array() {
    let (_state, app) = make_app().await;
    let (status, body) = send_json(app, "GET", "/api/sessions", None).await;
    assert_eq!(status, StatusCode::OK);
    assert!(body["sessions"].is_array());
    assert_eq!(body["count"], body["sessions"].as_array().unwrap().len());
}

// --- integration: end-to-end dashboard server ------------------------------

#[tokio::test]
async fn integration_dashboard_serves_on_real_port() {
    use tokio::time::timeout;

    let state = DashboardState::new_in_memory(PathBuf::from("/tmp")).await;
    let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
    let bound = terraphim_tinyclaw::dashboard::serve(state, addr)
        .await
        .expect("dashboard serve");

    // Give the server a moment to start accepting
    tokio::time::sleep(Duration::from_millis(100)).await;

    let url = format!("http://{}/api/health", bound);
    let result = timeout(
        Duration::from_secs(5),
        reqwest::Client::new().get(&url).send(),
    )
    .await;
    let result = result
        .expect("health request timed out")
        .expect("health request failed");
    assert!(
        result.status().is_success(),
        "health check failed: {}",
        result.status()
    );
}
