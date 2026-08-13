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

pub mod auth;
pub mod cron;
pub mod dispatch;
pub mod health;
pub mod ingress;
pub mod sessions;
pub mod status;

use axum::Router;
use axum::routing::{get, post};
use std::net::SocketAddr;
use std::sync::Arc;
use terraphim_persistence::DeviceStorage;
use tokio::sync::Mutex;

use crate::bus::MessageBus;
use crate::config::ChannelsConfig;
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
    /// Bearer token required for `POST /api/cron/fire`. When `None`,
    /// the endpoint is unauthenticated (dev/test only — production must set
    /// this from `TINYCLAW_FIRE_TOKEN` env var).
    pub fire_token: Option<String>,
    /// Channel configs used by production webhook ingress routes.
    pub inbound_channels: ChannelsConfig,
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
            fire_token: None,
            inbound_channels: ChannelsConfig::default(),
        }
    }

    /// Attach configured channel ingress handlers to dashboard state.
    pub fn with_inbound_channels(mut self, inbound_channels: ChannelsConfig) -> Self {
        self.inbound_channels = inbound_channels;
        self
    }

    /// Attach the live gateway runtime so dashboard and webhook ingress
    /// dispatch into the same AgentLoop and expose the same sessions.
    pub fn with_runtime(
        mut self,
        bus: Arc<MessageBus>,
        sessions: Arc<Mutex<SessionManager>>,
    ) -> Self {
        self.bus = bus;
        self.sessions = sessions;
        self
    }

    /// Configure the bearer token protecting side-effecting dashboard routes.
    pub fn with_fire_token(mut self, fire_token: Option<String>) -> Self {
        self.fire_token = fire_token;
        self
    }
}

/// Build the axum Router with all dashboard routes.
pub fn router(state: DashboardState) -> Router {
    Router::new()
        .route("/api/health", get(health::get_health))
        .route("/api/status", get(status::get_status))
        .route("/api/agent/messages", post(dispatch::post_message))
        .route(
            "/webhooks/whatsapp",
            get(ingress::whatsapp_verify).post(ingress::whatsapp_webhook),
        )
        .route("/webhooks/teams", post(ingress::teams_webhook))
        .route("/api/cron/fire", post(cron::fire_webhook))
        .route(
            "/api/cron/jobs",
            get(cron::list_jobs).post(cron::create_job),
        )
        .route(
            "/api/cron/jobs/{id}",
            get(cron::get_job).delete(cron::delete_job),
        )
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
