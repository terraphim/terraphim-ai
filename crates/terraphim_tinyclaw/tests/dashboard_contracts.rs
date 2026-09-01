//! Hermetic contract tests for the dashboard.
//!
//! Ports of Hermes' `hermes_cli/web_server.py` endpoints:
//! - `GET /api/health` (web_server.py:3064-3072)
//! - `GET /api/status` (web_server.py:3074-3457)
//! - `POST /api/cron/fire` (web_server.py:12673-12729)
//! - `GET/POST /api/cron/jobs` (cron/jobs.py CRUD)
//! - `GET/DELETE /api/cron/jobs/{id}`
//! - `GET /api/sessions`

mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use serde_json::{Value, json};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;
use terraphim_tinyclaw::bus::MessageBus;
use terraphim_tinyclaw::config::{ChannelsConfig, TeamsConfig, WhatsAppConfig};
use terraphim_tinyclaw::dashboard::{DashboardState, router};
use terraphim_tinyclaw::session::SessionManager;
use tokio::sync::Mutex;
use tower::ServiceExt; // for oneshot

async fn make_app() -> (DashboardState, axum::Router) {
    common::scrub_env();
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
        fire_token: None,
        inbound_channels: ChannelsConfig::default(),
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

#[tokio::test]
async fn contract_health_reports_auth_not_required_without_fire_token() {
    let (_state, app) = make_app().await;
    let (status, body) = send_json(app, "GET", "/api/health", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["auth_required"], false);
}

#[tokio::test]
async fn contract_health_reports_auth_required_with_fire_token() {
    let (_state, app) = make_app_with_fire_token("super-secret-token").await;
    let (status, body) = send_json(app, "GET", "/api/health", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["auth_required"], true);
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
async fn contract_cron_create_job_rejects_missing_auth_without_mutation_when_token_configured() {
    let (_state, app) = make_app_with_fire_token("super-secret-token").await;

    let (status, body) = send_with_auth(
        app.clone(),
        "POST",
        "/api/cron/jobs",
        Some(json!({
            "prompt": "must not persist",
            "schedule": "1h"
        })),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(body["error"], "invalid auth token");

    let (_status, list) = send_json(app, "GET", "/api/cron/jobs", None).await;
    assert_eq!(list["count"], 0);
}

#[tokio::test]
async fn contract_cron_create_job_rejects_wrong_bearer_without_mutation_when_token_configured() {
    let (_state, app) = make_app_with_fire_token("super-secret-token").await;

    let (status, body) = send_with_auth(
        app.clone(),
        "POST",
        "/api/cron/jobs",
        Some(json!({
            "prompt": "must not persist",
            "schedule": "1h"
        })),
        Some("wrong-token"),
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(body["error"], "invalid auth token");

    let (_status, list) = send_json(app, "GET", "/api/cron/jobs", None).await;
    assert_eq!(list["count"], 0);
}

#[tokio::test]
async fn contract_cron_create_job_accepts_correct_bearer_when_token_configured() {
    let (_state, app) = make_app_with_fire_token("super-secret-token").await;

    let (status, body) = send_with_auth(
        app.clone(),
        "POST",
        "/api/cron/jobs",
        Some(json!({
            "prompt": "persist me",
            "schedule": "1h"
        })),
        Some("super-secret-token"),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(body["status"], "created");

    let (_status, list) = send_json(app, "GET", "/api/cron/jobs", None).await;
    assert_eq!(list["count"], 1);
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

#[tokio::test]
async fn contract_cron_delete_job_rejects_missing_auth_without_mutation_when_token_configured() {
    let (_state, app) = make_app_with_fire_token("super-secret-token").await;
    let (_create_status, created) = send_with_auth(
        app.clone(),
        "POST",
        "/api/cron/jobs",
        Some(json!({
            "prompt": "keep me",
            "schedule": "1h"
        })),
        Some("super-secret-token"),
    )
    .await;
    let id = created["id"].as_str().unwrap().to_string();

    let (status, body) = send_with_auth(
        app.clone(),
        "DELETE",
        &format!("/api/cron/jobs/{id}"),
        None,
        None,
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(body["error"], "invalid auth token");

    let (status, body) = send_json(app, "GET", &format!("/api/cron/jobs/{id}"), None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["id"], id);
}

#[tokio::test]
async fn contract_cron_delete_job_rejects_wrong_bearer_without_mutation_when_token_configured() {
    let (_state, app) = make_app_with_fire_token("super-secret-token").await;
    let (_create_status, created) = send_with_auth(
        app.clone(),
        "POST",
        "/api/cron/jobs",
        Some(json!({
            "prompt": "keep me",
            "schedule": "1h"
        })),
        Some("super-secret-token"),
    )
    .await;
    let id = created["id"].as_str().unwrap().to_string();

    let (status, body) = send_with_auth(
        app.clone(),
        "DELETE",
        &format!("/api/cron/jobs/{id}"),
        None,
        Some("wrong-token"),
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(body["error"], "invalid auth token");

    let (status, body) = send_json(app, "GET", &format!("/api/cron/jobs/{id}"), None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["id"], id);
}

#[tokio::test]
async fn contract_cron_delete_job_accepts_correct_bearer_when_token_configured() {
    let (_state, app) = make_app_with_fire_token("super-secret-token").await;
    let (_create_status, created) = send_with_auth(
        app.clone(),
        "POST",
        "/api/cron/jobs",
        Some(json!({
            "prompt": "delete me",
            "schedule": "1h"
        })),
        Some("super-secret-token"),
    )
    .await;
    let id = created["id"].as_str().unwrap().to_string();

    let (status, body) = send_with_auth(
        app,
        "DELETE",
        &format!("/api/cron/jobs/{id}"),
        None,
        Some("super-secret-token"),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "deleted");
    assert_eq!(body["id"], id);
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

// --- /api/agent/messages auth (P1 from review #3221) ----------------------

/// Helper: build a dashboard state with the same auth token the
/// `/api/cron/fire` endpoint uses, so both endpoints share the contract.
async fn make_app_with_fire_token(token: &str) -> (DashboardState, axum::Router) {
    let (state, _app) = make_app().await;
    let state = DashboardState {
        fire_token: Some(token.to_string()),
        ..state
    };
    let app = router(state.clone());
    (state, app)
}

fn whatsapp_config() -> WhatsAppConfig {
    WhatsAppConfig {
        access_token: "test-access-token".into(),
        phone_number_id: "phone-1".into(),
        verify_token: "verify-me".into(),
        app_secret: "app-secret".into(),
        graph_base_url: "http://127.0.0.1".into(),
        api_version: "v20.0".into(),
        allow_from: vec!["15551234567".into()],
    }
}

fn teams_config() -> TeamsConfig {
    TeamsConfig {
        app_id: "app-123".into(),
        app_password: "secret-456".into(),
        token_url: "http://127.0.0.1/token".into(),
        scope: "https://api.botframework.com/.default".into(),
        openid_metadata_url: "https://login.botframework.com/v1/.well-known/openidconfiguration"
            .into(),
        openid_jwks_url: None,
        jwt_issuer: "https://api.botframework.com".into(),
        allow_from: vec!["29:user".into()],
    }
}

async fn make_app_with_inbound_channels(
    channels: ChannelsConfig,
) -> (DashboardState, axum::Router) {
    let (state, _app) = make_app().await;
    let state = DashboardState {
        inbound_channels: channels,
        ..state
    };
    let app = router(state.clone());
    (state, app)
}

fn teams_config_with_openid_metadata(openid_metadata_url: String) -> TeamsConfig {
    TeamsConfig {
        openid_metadata_url,
        ..teams_config()
    }
}

#[tokio::test]
async fn contract_runtime_builder_reuses_live_gateway_bus_and_sessions() {
    common::scrub_env();
    let (state, _app) = make_app().await;
    let live_bus = Arc::new(MessageBus::new());
    let live_sessions = Arc::new(Mutex::new(SessionManager::new(
        tempfile::tempdir().unwrap().path().join("live-sessions"),
    )));

    let wired = state
        .with_runtime(live_bus.clone(), live_sessions.clone())
        .with_fire_token(Some("gateway-token".into()));

    assert!(Arc::ptr_eq(&wired.bus, &live_bus));
    assert!(Arc::ptr_eq(&wired.sessions, &live_sessions));
    assert_eq!(wired.fire_token.as_deref(), Some("gateway-token"));
}

fn whatsapp_sig(secret: &str, body: &[u8]) -> String {
    use hmac::{Hmac, Mac};
    use sha2::Sha256;

    let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes()).unwrap();
    mac.update(body);
    let digest = mac.finalize().into_bytes();
    let encoded = digest
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    format!("sha256={encoded}")
}

async fn send_with_auth(
    app: axum::Router,
    method: &str,
    path: &str,
    body: Option<Value>,
    bearer: Option<&str>,
) -> (StatusCode, Value) {
    let mut builder = Request::builder().method(method).uri(path);
    if let Some(bearer) = bearer {
        builder = builder.header("authorization", format!("Bearer {bearer}"));
    }
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

#[tokio::test]
async fn contract_agent_messages_rejects_missing_auth_when_token_configured() {
    common::scrub_env();
    let (_state, app) = make_app_with_fire_token("super-secret-token").await;
    let (status, body) = send_with_auth(
        app,
        "POST",
        "/api/agent/messages",
        Some(json!({ "content": "hello" })),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(body["error"], "invalid auth token");
}

#[tokio::test]
async fn contract_agent_messages_rejects_wrong_bearer_when_token_configured() {
    common::scrub_env();
    let (_state, app) = make_app_with_fire_token("super-secret-token").await;
    let (status, body) = send_with_auth(
        app,
        "POST",
        "/api/agent/messages",
        Some(json!({ "content": "hello" })),
        Some("not-the-token"),
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(body["error"], "invalid auth token");
}

#[tokio::test]
async fn contract_agent_messages_dispatches_with_correct_bearer() {
    common::scrub_env();
    let (state, app) = make_app_with_fire_token("super-secret-token").await;

    let (status, _body) = send_with_auth(
        app,
        "POST",
        "/api/agent/messages",
        Some(json!({ "content": "hello", "sender_id": "alice", "chat_id": "c1" })),
        Some("super-secret-token"),
    )
    .await;
    assert_eq!(status, StatusCode::ACCEPTED);

    // Verify the message reached the bus.
    let received = {
        let bus = state.bus.clone();
        let timeout = tokio::time::Duration::from_secs(2);
        tokio::time::timeout(timeout, async move {
            let mut rx = bus.inbound_rx.lock().await;
            rx.recv().await
        })
        .await
        .expect("dispatch reaches bus within timeout")
        .expect("dispatch produced a message")
    };
    assert_eq!(received.content, "hello");
    assert_eq!(received.sender_id, "alice");
    assert_eq!(received.chat_id, "c1");
}

#[tokio::test]
async fn contract_agent_messages_open_when_no_token_configured() {
    // When the dashboard is configured without a fire_token, agent
    // dispatch must remain open (dev/test mode) just like the cron fire
    // endpoint. This preserves backward compatibility for hermetic tests.
    common::scrub_env();
    let (_state, app) = make_app().await;
    let (status, _body) = send_with_auth(
        app,
        "POST",
        "/api/agent/messages",
        Some(json!({ "content": "open-mode" })),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::ACCEPTED);
}

// --- production channel ingress -------------------------------------------

#[tokio::test]
async fn contract_whatsapp_get_verification_returns_challenge() {
    common::scrub_env();
    let (_state, app) = make_app_with_inbound_channels(ChannelsConfig {
        whatsapp: Some(whatsapp_config()),
        teams: None,
        ..ChannelsConfig::default()
    })
    .await;

    let (status, body) = send_raw(
        app,
        "GET",
        "/webhooks/whatsapp?hub.mode=subscribe&hub.verify_token=verify-me&hub.challenge=abc123",
        None,
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, b"abc123");
}

#[tokio::test]
async fn contract_whatsapp_post_rejects_bad_signature_before_dispatch() {
    common::scrub_env();
    let (state, app) = make_app_with_inbound_channels(ChannelsConfig {
        whatsapp: Some(whatsapp_config()),
        teams: None,
        ..ChannelsConfig::default()
    })
    .await;

    let (status, _body) = send_with_header(
        app,
        "POST",
        "/webhooks/whatsapp",
        br#"{"entry":"malformed-would-fail-if-parsed"}"#,
        Some(("x-hub-signature-256", "sha256=bad")),
    )
    .await;

    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_no_inbound(state.bus).await;
}

#[tokio::test]
async fn contract_whatsapp_valid_signature_dispatches_allowed_message() {
    common::scrub_env();
    let (state, app) = make_app_with_inbound_channels(ChannelsConfig {
        whatsapp: Some(whatsapp_config()),
        teams: None,
        ..ChannelsConfig::default()
    })
    .await;
    let body = br#"{
      "entry": [{
        "changes": [{
          "value": {
            "contacts": [{"wa_id": "15551234567", "profile": {"name": "Alice"}}],
            "messages": [{"from":"15551234567","id":"wamid.1","type":"text","text":{"body":"hello whatsapp"}}]
          }
        }]
      }]
    }"#;
    let sig = whatsapp_sig("app-secret", body);

    let (status, _body) = send_with_header(
        app,
        "POST",
        "/webhooks/whatsapp",
        body,
        Some(("x-hub-signature-256", &sig)),
    )
    .await;

    assert_eq!(status, StatusCode::ACCEPTED);
    let msg = recv_inbound(state.bus).await;
    assert_eq!(msg.channel, "whatsapp");
    assert_eq!(msg.sender_id, "15551234567");
    assert_eq!(msg.content, "hello whatsapp");
}

/// A Meta webhook batch carrying several messages. Shared by the
/// batch-dispatch success and dispatch-failure contracts below.
fn whatsapp_batch_body() -> &'static [u8] {
    br#"{
      "entry": [{
        "changes": [{
          "value": {
            "contacts": [{"wa_id": "15551234567", "profile": {"name": "Alice"}}],
            "messages": [
              {"from":"15551234567","id":"wamid.1","type":"text","text":{"body":"first"}},
              {"from":"15551234567","id":"wamid.2","type":"text","text":{"body":"second"}},
              {"from":"15551234567","id":"wamid.3","type":"text","text":{"body":"third"}}
            ]
          }
        }]
      }]
    }"#
}

#[tokio::test]
async fn contract_whatsapp_batch_dispatches_every_message_on_success() {
    common::scrub_env();
    let (state, app) = make_app_with_inbound_channels(ChannelsConfig {
        whatsapp: Some(whatsapp_config()),
        teams: None,
        ..ChannelsConfig::default()
    })
    .await;
    let body = whatsapp_batch_body();
    let sig = whatsapp_sig("app-secret", body);

    let (status, _body) = send_with_header(
        app,
        "POST",
        "/webhooks/whatsapp",
        body,
        Some(("x-hub-signature-256", &sig)),
    )
    .await;

    assert_eq!(status, StatusCode::ACCEPTED);
    let mut contents = Vec::new();
    for _ in 0..3 {
        contents.push(recv_inbound(state.bus.clone()).await.content);
    }
    assert_eq!(contents, vec!["first", "second", "third"]);
    assert_no_inbound(state.bus).await;
}

#[tokio::test]
async fn contract_whatsapp_batch_returns_503_when_dispatch_fails() {
    // The provider only redelivers on a non-2xx. When the inbound bus
    // is closed (agent loop gone), a 202 would silently lose the batch.
    common::scrub_env();
    let (state, app) = make_app_with_inbound_channels(ChannelsConfig {
        whatsapp: Some(whatsapp_config()),
        teams: None,
        ..ChannelsConfig::default()
    })
    .await;
    state.bus.inbound_rx.lock().await.close();
    let body = whatsapp_batch_body();
    let sig = whatsapp_sig("app-secret", body);

    let (status, resp) = send_with_header(
        app,
        "POST",
        "/webhooks/whatsapp",
        body,
        Some(("x-hub-signature-256", &sig)),
    )
    .await;

    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(resp["failed"], 3);
    assert_eq!(resp["total"], 3);
}

#[tokio::test]
async fn contract_whatsapp_bad_signature_still_rejected_when_dispatch_unavailable() {
    // Signature verification stays fail-closed and runs before any
    // dispatch attempt, regardless of bus state.
    common::scrub_env();
    let (state, app) = make_app_with_inbound_channels(ChannelsConfig {
        whatsapp: Some(whatsapp_config()),
        teams: None,
        ..ChannelsConfig::default()
    })
    .await;
    state.bus.inbound_rx.lock().await.close();

    let (status, _body) = send_with_header(
        app,
        "POST",
        "/webhooks/whatsapp",
        whatsapp_batch_body(),
        Some(("x-hub-signature-256", "sha256=bad")),
    )
    .await;

    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn contract_teams_rejects_missing_bearer_before_parse_dispatch() {
    common::scrub_env();
    let (metadata_url, _jwks_url) = spawn_teams_openid_server().await;
    let (state, app) = make_app_with_inbound_channels(ChannelsConfig {
        whatsapp: None,
        teams: Some(teams_config_with_openid_metadata(metadata_url)),
        ..ChannelsConfig::default()
    })
    .await;

    let (status, _body) = send_with_header(
        app,
        "POST",
        "/webhooks/teams",
        br#"{"type":"message"}"#,
        None,
    )
    .await;

    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_no_inbound(state.bus).await;
}

#[tokio::test]
async fn contract_teams_valid_signed_jwt_dispatches_allowed_message() {
    common::scrub_env();
    let (metadata_url, _jwks_url) = spawn_teams_openid_server().await;
    let (state, app) = make_app_with_inbound_channels(ChannelsConfig {
        whatsapp: None,
        teams: Some(teams_config_with_openid_metadata(metadata_url)),
        ..ChannelsConfig::default()
    })
    .await;
    let body = br#"{
      "type": "message",
      "id": "activity-1",
      "serviceUrl": "https://smba.trafficmanager.net/emea/",
      "from": {"id": "29:user"},
      "conversation": {"id": "conv-1"},
      "text": "hello teams"
    }"#;
    let token = teams_jwt("app-123", 3600);

    let (status, _body) = send_with_header(
        app,
        "POST",
        "/webhooks/teams",
        body,
        Some(("authorization", &format!("Bearer {token}"))),
    )
    .await;

    assert_eq!(status, StatusCode::ACCEPTED);
    let msg = recv_inbound(state.bus).await;
    assert_eq!(msg.channel, "teams");
    assert_eq!(msg.sender_id, "29:user");
    assert_eq!(msg.chat_id, "https://smba.trafficmanager.net/emea/|conv-1");
    assert_eq!(msg.content, "hello teams");
}

#[tokio::test]
async fn contract_teams_forged_signature_rejected_before_dispatch() {
    common::scrub_env();
    let (metadata_url, _jwks_url) = spawn_teams_openid_server().await;
    let (state, app) = make_app_with_inbound_channels(ChannelsConfig {
        whatsapp: None,
        teams: Some(teams_config_with_openid_metadata(metadata_url)),
        ..ChannelsConfig::default()
    })
    .await;
    let body = teams_activity_body();
    let mut token = teams_jwt("app-123", 3600);
    token.push('x');

    let (status, _body) = send_with_header(
        app,
        "POST",
        "/webhooks/teams",
        body.as_bytes(),
        Some(("authorization", &format!("Bearer {token}"))),
    )
    .await;

    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_no_inbound(state.bus).await;
}

#[tokio::test]
async fn contract_teams_wrong_audience_rejected_before_dispatch() {
    common::scrub_env();
    let (metadata_url, _jwks_url) = spawn_teams_openid_server().await;
    let (state, app) = make_app_with_inbound_channels(ChannelsConfig {
        whatsapp: None,
        teams: Some(teams_config_with_openid_metadata(metadata_url)),
        ..ChannelsConfig::default()
    })
    .await;
    let body = teams_activity_body();
    let token = teams_jwt("other-app", 3600);

    let (status, _body) = send_with_header(
        app,
        "POST",
        "/webhooks/teams",
        body.as_bytes(),
        Some(("authorization", &format!("Bearer {token}"))),
    )
    .await;

    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_no_inbound(state.bus).await;
}

#[tokio::test]
async fn contract_teams_expired_token_rejected_before_dispatch() {
    common::scrub_env();
    let (metadata_url, _jwks_url) = spawn_teams_openid_server().await;
    let (state, app) = make_app_with_inbound_channels(ChannelsConfig {
        whatsapp: None,
        teams: Some(teams_config_with_openid_metadata(metadata_url)),
        ..ChannelsConfig::default()
    })
    .await;
    let body = teams_activity_body();
    let token = teams_jwt("app-123", -3600);

    let (status, _body) = send_with_header(
        app,
        "POST",
        "/webhooks/teams",
        body.as_bytes(),
        Some(("authorization", &format!("Bearer {token}"))),
    )
    .await;

    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_no_inbound(state.bus).await;
}

#[tokio::test]
async fn contract_teams_rotated_jwks_kid_refreshes_once_then_dispatches() {
    common::scrub_env();
    let jwks_fetches = Arc::new(AtomicUsize::new(0));
    let (metadata_url, _jwks_url) = spawn_teams_openid_server_with_jwks_sequence(
        jwks_fetches.clone(),
        vec![
            vec![("old-key", TEAMS_TEST_JWK_N, TEAMS_TEST_JWK_E)],
            vec![("rotated-key", TEAMS_TEST_JWK_N, TEAMS_TEST_JWK_E)],
        ],
    )
    .await;
    let (state, app) = make_app_with_inbound_channels(ChannelsConfig {
        whatsapp: None,
        teams: Some(teams_config_with_openid_metadata(metadata_url)),
        ..ChannelsConfig::default()
    })
    .await;
    let body = teams_activity_body();
    let token = teams_jwt_with_options(
        "app-123",
        3600,
        "rotated-key",
        "https://api.botframework.com",
        "https://smba.trafficmanager.net/emea/",
    );

    let (status, _body) = send_with_header(
        app,
        "POST",
        "/webhooks/teams",
        body.as_bytes(),
        Some(("authorization", &format!("Bearer {token}"))),
    )
    .await;

    assert_eq!(status, StatusCode::ACCEPTED);
    let msg = recv_inbound(state.bus).await;
    assert_eq!(msg.content, "hello teams");
    assert_eq!(jwks_fetches.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn contract_teams_unknown_kid_refreshes_once_then_rejects_before_dispatch() {
    common::scrub_env();
    let jwks_fetches = Arc::new(AtomicUsize::new(0));
    let (metadata_url, _jwks_url) = spawn_teams_openid_server_with_jwks_sequence(
        jwks_fetches.clone(),
        vec![
            vec![("old-key", TEAMS_TEST_JWK_N, TEAMS_TEST_JWK_E)],
            vec![("rotated-key", TEAMS_TEST_JWK_N, TEAMS_TEST_JWK_E)],
        ],
    )
    .await;
    let (state, app) = make_app_with_inbound_channels(ChannelsConfig {
        whatsapp: None,
        teams: Some(teams_config_with_openid_metadata(metadata_url)),
        ..ChannelsConfig::default()
    })
    .await;
    let body = teams_activity_body();
    let token = teams_jwt_with_options(
        "app-123",
        3600,
        "unknown-key",
        "https://api.botframework.com",
        "https://smba.trafficmanager.net/emea/",
    );

    let (status, _body) = send_with_header(
        app,
        "POST",
        "/webhooks/teams",
        body.as_bytes(),
        Some(("authorization", &format!("Bearer {token}"))),
    )
    .await;

    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(jwks_fetches.load(Ordering::SeqCst), 2);
    assert_no_inbound(state.bus).await;
}

#[tokio::test]
async fn contract_teams_service_url_claim_body_mismatch_rejected_before_dispatch() {
    common::scrub_env();
    let (metadata_url, _jwks_url) = spawn_teams_openid_server().await;
    let (state, app) = make_app_with_inbound_channels(ChannelsConfig {
        whatsapp: None,
        teams: Some(teams_config_with_openid_metadata(metadata_url)),
        ..ChannelsConfig::default()
    })
    .await;
    let body = teams_activity_body();
    let token = teams_jwt_with_options(
        "app-123",
        3600,
        TEAMS_TEST_KID,
        "https://api.botframework.com",
        "https://smba.trafficmanager.net/amer/",
    );

    let (status, _body) = send_with_header(
        app,
        "POST",
        "/webhooks/teams",
        body.as_bytes(),
        Some(("authorization", &format!("Bearer {token}"))),
    )
    .await;

    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_no_inbound(state.bus).await;
}

#[tokio::test]
async fn contract_teams_openid_metadata_issuer_mismatch_rejected_before_dispatch() {
    common::scrub_env();
    let (metadata_url, _jwks_url) =
        spawn_teams_openid_server_with_issuer("https://issuer.invalid").await;
    let (state, app) = make_app_with_inbound_channels(ChannelsConfig {
        whatsapp: None,
        teams: Some(teams_config_with_openid_metadata(metadata_url)),
        ..ChannelsConfig::default()
    })
    .await;
    let body = teams_activity_body();
    let token = teams_jwt("app-123", 3600);

    let (status, _body) = send_with_header(
        app,
        "POST",
        "/webhooks/teams",
        body.as_bytes(),
        Some(("authorization", &format!("Bearer {token}"))),
    )
    .await;

    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_no_inbound(state.bus).await;
}

const TEAMS_TEST_KID: &str = "tinyclaw-test-key";
const TEAMS_TEST_JWK_N: &str = "tYxRuIobku6bWfbLBSklyusK7WAKGPb24YFUpJLnG_sss8QchHEFoIFhKzDAgxSRIAmkeysjznEZv58vL0fQGuAguuYxeNwH6byTVWyFD-4MkAO7dmp9-iIg77PNr83zxJu9aDoQTTnt86eAqKTMb_RMb4BYKodyLk-wOR_SLXLqrtotjeC8WwPzMXgoBSDWJ4EcbbYNnaEzz2vlUaJv73mk6PGybsZdJbMvpNpXsFafVMdUjsys3hJ5CL3LRzzQCKCBe_xjqogu2Qd9kFNNq_25RxXfeO5Tg_M8cHyySyGsDsHrJkAW0SoaA3_Bv-Y9JnbnKkujUC70PVDfSpCrFQ";
const TEAMS_TEST_JWK_E: &str = "AQAB";
const TEAMS_TEST_PRIVATE_KEY: &str = r#"-----BEGIN PRIVATE KEY-----
MIIEvgIBADANBgkqhkiG9w0BAQEFAASCBKgwggSkAgEAAoIBAQC1jFG4ihuS7ptZ
9ssFKSXK6wrtYAoY9vbhgVSkkucb+yyzxByEcQWggWErMMCDFJEgCaR7KyPOcRm/
ny8vR9Aa4CC65jF43AfpvJNVbIUP7gyQA7t2an36IiDvs82vzfPEm71oOhBNOe3z
p4CopMxv9ExvgFgqh3IuT7A5H9Itcuqu2i2N4LxbA/MxeCgFINYngRxttg2doTPP
a+VRom/veaTo8bJuxl0lsy+k2lewVp9Ux1SOzKzeEnkIvctHPNAIoIF7/GOqiC7Z
B32QU02r/blHFd947lOD8zxwfLJLIawOwesmQBbRKhoDf8G/5j0mducqS6NQLvQ9
UN9KkKsVAgMBAAECggEAB8MSE2PEPgn5lmUY1QWnWf+mnfOHOv4EHCwdZFPrkaSx
ipYN6hOy++PMNb6F3JTah5Yh9CoqA7+OLEUwDUNywFNo0tTYtKQjqnini/Vx0vaI
jve54hw3eqRO6DC10JhrnbJS7Mveuo6Jf+tfctZo7zhwtCu2mUrlNNmvXSHhKJQ4
oQ+HUcmFpn/2CtVDyMBtoOojk39qEFdPO6LcE9xoTTVHn1cvk71HcR/qnBVg0VBF
XerChWKhYhi0cspmktIL5WzZIWE6ErjmBLJ+4995q/djvYnk3GNBvjqG51KCiQBC
ZMki4K/R+usg/bH6PsQ6LvkUSl2hM9i9mgMwGMQNmQKBgQD1l9/laDMmD1KkAKJb
9jMAJI2SWabazrcdLxTDTP0HLmBuWIM5mYE49aUSIken2SCj5KUJ/YLehWZ/ucxU
Bu24T67El3nKrKMJYy1hU756qSfZsO2f4bpWaBBSYTrHdWG/pnK8m+OOu3FOeKyg
XEs1dBVa9zvRDZTr2roV5uU46wKBgQC9PbOIKfyTjYJ7VopGTQlkmSyBVzZXjwRw
n8ih3Gbm3uvSMMqqRHjtg/w7eisuaDJD9pvyeuP1BJwG4FDoP1azovL3WCS54yDU
jKVXCnsZ9cPPa/sUE3JJWlqt9NDSk7Z6G5to2K/6Ce0QLBV3M97q5ij4PdmXSYzt
RdQsP6ir/wKBgQC0zuhmY41sktFbrp/Zpmsryr1zpo9B/fgwAy59Dlwmgk06T+3k
ZRAO2EJ8FEK+3wq5vqaGsV485udsV2SH5EX1cKyRTZe7z9eEHMEXK2lLsueEzE5V
bAEchiamD7EPkWqyhx9nnjktJvCnPg6RTQGNy/XA783Y/e/KqFBhuYi+mwKBgQC7
/lyoixVYO78A6eKpngQTxLrarpnUd8YwX/s/GKW8+n2IrGHYrfb39SMQrvfQvUAa
FhWr1/s3P+IETrjxT2LBR8JYh17Mr17A6AukoUvTaTuhhiLCOeNSDYmBwG/mkUk4
ms5TZNfE7DFt3G1iosvzzLusiDzttupiF/mED6VvXwKBgFSKRofIyXHQc18DedPd
hk2ltMOg3XqpkyifgW42gTcQYZiH+3OGsw7Z6mpAxVWsnbKalwiIpfMdz77Bi26g
7UfGWW2nzox55bdkY99ZfW2lnbuJfKz3j4f5H4ifvrKFX6oHxP03JrbC4I6/G2+f
vE0p1iWQBxTvqhzg+H32Up1N
-----END PRIVATE KEY-----
"#;

async fn spawn_teams_openid_server() -> (String, String) {
    spawn_teams_openid_server_with_issuer("https://api.botframework.com").await
}

async fn spawn_teams_openid_server_with_issuer(issuer: &'static str) -> (String, String) {
    let jwks_fetches = Arc::new(AtomicUsize::new(0));
    spawn_teams_openid_server_with_issuer_and_jwks_sequence(
        issuer,
        jwks_fetches,
        vec![vec![(TEAMS_TEST_KID, TEAMS_TEST_JWK_N, TEAMS_TEST_JWK_E)]],
    )
    .await
}

async fn spawn_teams_openid_server_with_jwks_sequence(
    jwks_fetches: Arc<AtomicUsize>,
    jwks_sequence: Vec<Vec<(&'static str, &'static str, &'static str)>>,
) -> (String, String) {
    spawn_teams_openid_server_with_issuer_and_jwks_sequence(
        "https://api.botframework.com",
        jwks_fetches,
        jwks_sequence,
    )
    .await
}

async fn spawn_teams_openid_server_with_issuer_and_jwks_sequence(
    issuer: &'static str,
    jwks_fetches: Arc<AtomicUsize>,
    jwks_sequence: Vec<Vec<(&'static str, &'static str, &'static str)>>,
) -> (String, String) {
    use axum::routing::get;

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let base = format!("http://{addr}");
    let jwks_url = format!("{base}/keys");
    let metadata_jwks_url = jwks_url.clone();
    let sequence = Arc::new(jwks_sequence);
    let keys_sequence = sequence.clone();
    let app = axum::Router::new()
        .route(
            "/metadata",
            get(move || {
                let jwks_uri = metadata_jwks_url.clone();
                async move {
                    axum::Json(json!({
                        "issuer": issuer,
                        "jwks_uri": jwks_uri,
                        "id_token_signing_alg_values_supported": ["RS256"]
                    }))
                }
            }),
        )
        .route(
            "/keys",
            get(move || {
                let jwks_fetches = jwks_fetches.clone();
                let keys_sequence = keys_sequence.clone();
                async move {
                    let fetch_index = jwks_fetches.fetch_add(1, Ordering::SeqCst);
                    let keys = keys_sequence
                        .get(fetch_index)
                        .or_else(|| keys_sequence.last())
                        .expect("JWKS sequence has at least one response")
                        .iter()
                        .map(|(kid, n, e)| {
                            json!({
                                "kty": "RSA",
                                "kid": kid,
                                "use": "sig",
                                "alg": "RS256",
                                "n": n,
                                "e": e
                            })
                        })
                        .collect::<Vec<_>>();
                    axum::Json(json!({ "keys": keys }))
                }
            }),
        );
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    (format!("{base}/metadata"), jwks_url)
}

fn teams_activity_body() -> String {
    r#"{
      "type": "message",
      "id": "activity-1",
      "serviceUrl": "https://smba.trafficmanager.net/emea/",
      "from": {"id": "29:user"},
      "conversation": {"id": "conv-1"},
      "text": "hello teams"
    }"#
    .to_string()
}

fn teams_jwt(audience: &str, expires_in_secs: i64) -> String {
    teams_jwt_with_options(
        audience,
        expires_in_secs,
        TEAMS_TEST_KID,
        "https://api.botframework.com",
        "https://smba.trafficmanager.net/emea/",
    )
}

fn teams_jwt_with_options(
    audience: &str,
    expires_in_secs: i64,
    kid: &str,
    issuer: &str,
    service_url: &str,
) -> String {
    use jsonwebtoken::{Algorithm, EncodingKey, Header, encode};
    use serde::Serialize;

    #[derive(Serialize)]
    struct Claims<'a> {
        iss: &'a str,
        aud: &'a str,
        nbf: i64,
        exp: i64,
        #[serde(rename = "serviceUrl")]
        service_url: &'a str,
    }

    let now = chrono::Utc::now().timestamp();
    let mut header = Header::new(Algorithm::RS256);
    header.kid = Some(kid.to_string());
    encode(
        &header,
        &Claims {
            iss: issuer,
            aud: audience,
            nbf: now - 60,
            exp: now + expires_in_secs,
            service_url,
        },
        &EncodingKey::from_rsa_pem(TEAMS_TEST_PRIVATE_KEY.as_bytes()).unwrap(),
    )
    .unwrap()
}

async fn send_with_header(
    app: axum::Router,
    method: &str,
    path: &str,
    body: &[u8],
    header: Option<(&str, &str)>,
) -> (StatusCode, Value) {
    let mut builder = Request::builder()
        .method(method)
        .uri(path)
        .header("content-type", "application/json");
    if let Some((name, value)) = header {
        builder = builder.header(name, value);
    }
    let req = builder.body(Body::from(body.to_vec())).unwrap();
    let resp = app.oneshot(req).await.unwrap();
    let status = resp.status();
    let bytes = axum::body::to_bytes(resp.into_body(), 1024 * 1024)
        .await
        .unwrap();
    let parsed = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    (status, parsed)
}

async fn send_raw(
    app: axum::Router,
    method: &str,
    path: &str,
    body: Option<&[u8]>,
) -> (StatusCode, Vec<u8>) {
    let req = Request::builder()
        .method(method)
        .uri(path)
        .body(match body {
            Some(body) => Body::from(body.to_vec()),
            None => Body::empty(),
        })
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    let status = resp.status();
    let bytes = axum::body::to_bytes(resp.into_body(), 1024 * 1024)
        .await
        .unwrap();
    (status, bytes.to_vec())
}

async fn recv_inbound(bus: Arc<MessageBus>) -> terraphim_tinyclaw::bus::InboundMessage {
    tokio::time::timeout(Duration::from_secs(2), async move {
        let mut rx = bus.inbound_rx.lock().await;
        rx.recv().await
    })
    .await
    .expect("inbound dispatch timed out")
    .expect("message dispatched")
}

async fn assert_no_inbound(bus: Arc<MessageBus>) {
    let result = tokio::time::timeout(Duration::from_millis(100), async move {
        let mut rx = bus.inbound_rx.lock().await;
        rx.recv().await
    })
    .await;
    assert!(result.is_err(), "invalid ingress unexpectedly dispatched");
}
