//! TinyClaw dashboard — axum-based HTTP server.
//!
//! Wave 5 (Phase C1) of the Hermes parity arc. Provides a subset of
//! Hermes' `hermes_cli/web_server.py` endpoints:
//!
//! - `GET  /api/health` — process liveness
//! - `GET  /api/status` — gateway/session summary
//! - `POST /api/cron/fire` — Chronos managed-cron fire webhook
//! - `POST /api/cron/jobs` — list cron jobs
//! - `GET  /api/sessions` — list active sessions
//! - `GET  /api/cron/jobs/{id}` — get a single cron job
//!
//! Run with `terraphim_tinyclaw serve-dashboard` or programmatically via
//! `dashboard::serve()`.

pub mod cron;
pub mod health;
pub mod sessions;
pub mod status;

use axum::Router;
use axum::routing::{get, post};
use std::net::SocketAddr;
use std::sync::Arc;
use terraphim_persistence::DeviceStorage;
use tokio::sync::Mutex;

use crate::bus::MessageBus;
use crate::cron::CronStore;
use crate::session::SessionManager;

/// Shared application state.
#[derive(Clone)]
pub struct DashboardState {
    pub sessions: Arc<Mutex<SessionManager>>,
    pub bus: Arc<MessageBus>,
    pub cron_store: CronStore,
    /// Whether the dashboard requires auth (cookie/JWT gate).
    pub auth_required: bool,
}

impl DashboardState {
    /// Construct state with an in-memory cron store (hermetic for tests).
    pub async fn new_in_memory(sessions_dir: std::path::PathBuf) -> Self {
        let _ = DeviceStorage::init_memory_only().await;
        let storage = DeviceStorage::arc_memory_only()
            .await
            .expect("arc memory-only DeviceStorage");
        let cron_store = CronStore::new(storage, "dashboard_cron_jobs");
        Self {
            sessions: Arc::new(Mutex::new(SessionManager::new(sessions_dir))),
            bus: Arc::new(MessageBus::new()),
            cron_store,
            auth_required: false,
        }
    }
}

/// Build the axum Router with all dashboard routes.
pub fn router(state: DashboardState) -> Router {
    Router::new()
        .route("/api/health", get(health::get_health))
        .route("/api/status", get(status::get_status))
        .route("/api/cron/fire", post(cron::fire_webhook))
        .route("/api/cron/jobs", get(cron::list_jobs).post(cron::create_job))
        .route("/api/cron/jobs/{id}", get(cron::get_job).delete(cron::delete_job))
        .route("/api/sessions", get(sessions::list_sessions))
        .with_state(state)
}

/// Start the dashboard server on the given address.
///
/// Returns the bound address (useful when port 0 is requested for tests).
pub async fn serve(state: DashboardState, addr: SocketAddr) -> Result<SocketAddr, std::io::Error> {
    let app = router(state);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    let bound = listener.local_addr()?;
    tokio::spawn(async move {
        if let Err(e) = axum::serve(listener, app).await {
            tracing::error!("dashboard server error: {e}");
        }
    });
    Ok(bound)
}
