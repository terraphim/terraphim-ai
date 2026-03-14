//! Optional HTTP observability API for Symphony.
//!
//! Feature-gated behind `api`. Provides endpoints for inspecting
//! orchestrator state, per-issue details, and triggering refresh.

use crate::orchestrator::state::StateSnapshot;
use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    response::{Html, IntoResponse},
    routing::{get, post},
};
use serde::Serialize;
use std::sync::Arc;
use tokio::sync::{Mutex, Notify};

/// Shared state for the API server.
pub struct ApiState {
    /// Callback to get the current orchestrator snapshot.
    snapshot_fn: Box<dyn Fn() -> StateSnapshot + Send + Sync>,
    /// Signal to trigger an immediate poll refresh.
    refresh_notify: Arc<Notify>,
}

impl ApiState {
    /// Create a new API state.
    pub fn new(
        snapshot_fn: Box<dyn Fn() -> StateSnapshot + Send + Sync>,
        refresh_notify: Arc<Notify>,
    ) -> Self {
        Self {
            snapshot_fn,
            refresh_notify,
        }
    }
}

/// Build the axum Router for the Symphony API.
pub fn router(state: Arc<Mutex<ApiState>>) -> Router {
    Router::new()
        .route("/", get(dashboard))
        .route("/api/v1/state", get(get_state))
        .route("/api/v1/refresh", post(post_refresh))
        .route("/api/v1/{issue_identifier}", get(get_issue))
        .with_state(state)
}

/// Error response body.
#[derive(Debug, Serialize)]
struct ApiError {
    error: ApiErrorDetail,
}

#[derive(Debug, Serialize)]
struct ApiErrorDetail {
    code: String,
    message: String,
}

/// GET /api/v1/state - return orchestrator state snapshot.
async fn get_state(State(state): State<Arc<Mutex<ApiState>>>) -> impl IntoResponse {
    let locked = state.lock().await;
    let snapshot = (locked.snapshot_fn)();
    (StatusCode::OK, Json(snapshot))
}

/// POST /api/v1/refresh - trigger immediate poll + reconciliation.
async fn post_refresh(State(state): State<Arc<Mutex<ApiState>>>) -> impl IntoResponse {
    let locked = state.lock().await;
    locked.refresh_notify.notify_one();
    (
        StatusCode::ACCEPTED,
        Json(serde_json::json!({"status": "refresh_queued"})),
    )
}

/// GET /api/v1/:issue_identifier - issue-specific debug details.
async fn get_issue(
    State(state): State<Arc<Mutex<ApiState>>>,
    Path(identifier): Path<String>,
) -> impl IntoResponse {
    let locked = state.lock().await;
    let snapshot = (locked.snapshot_fn)();

    // Search running entries
    if let Some(running) = snapshot
        .running
        .iter()
        .find(|r| r.issue_identifier == identifier)
    {
        return (StatusCode::OK, Json(serde_json::to_value(running).unwrap())).into_response();
    }

    // Search retry queue
    if let Some(retrying) = snapshot
        .retrying
        .iter()
        .find(|r| r.issue_identifier == identifier)
    {
        return (
            StatusCode::OK,
            Json(serde_json::to_value(retrying).unwrap()),
        )
            .into_response();
    }

    let err = ApiError {
        error: ApiErrorDetail {
            code: "not_found".to_string(),
            message: format!("issue {identifier} not found in running or retry state"),
        },
    };
    (
        StatusCode::NOT_FOUND,
        Json(serde_json::to_value(err).unwrap()),
    )
        .into_response()
}

/// GET / - human-readable dashboard.
async fn dashboard(State(state): State<Arc<Mutex<ApiState>>>) -> impl IntoResponse {
    let locked = state.lock().await;
    let snapshot = (locked.snapshot_fn)();

    let running_rows: String = if snapshot.running.is_empty() {
        "<tr><td colspan=\"6\">No running sessions</td></tr>".to_string()
    } else {
        snapshot
            .running
            .iter()
            .map(|r| {
                format!(
                    "<tr><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td></tr>",
                    r.issue_identifier,
                    r.state,
                    r.session_id,
                    r.turn_count,
                    r.last_event.as_deref().unwrap_or("-"),
                    r.started_at.format("%H:%M:%S"),
                )
            })
            .collect()
    };

    let retry_rows: String = if snapshot.retrying.is_empty() {
        "<tr><td colspan=\"4\">No pending retries</td></tr>".to_string()
    } else {
        snapshot
            .retrying
            .iter()
            .map(|r| {
                format!(
                    "<tr><td>{}</td><td>{}</td><td>{}</td><td>{}</td></tr>",
                    r.issue_identifier,
                    r.attempt,
                    r.due_at,
                    r.error.as_deref().unwrap_or("-"),
                )
            })
            .collect()
    };

    let html = format!(
        r#"<!DOCTYPE html>
<html>
<head>
  <title>Symphony Dashboard</title>
  <meta charset="utf-8">
  <style>
    body {{ font-family: monospace; margin: 2em; background: #1a1a2e; color: #e0e0e0; }}
    h1 {{ color: #16213e; background: #0f3460; padding: 0.5em; border-radius: 4px; color: #e94560; }}
    h2 {{ color: #e94560; margin-top: 1.5em; }}
    table {{ border-collapse: collapse; width: 100%; margin-top: 0.5em; }}
    th, td {{ border: 1px solid #0f3460; padding: 0.4em 0.8em; text-align: left; }}
    th {{ background: #16213e; color: #e94560; }}
    tr:nth-child(even) {{ background: #16213e; }}
    .stats {{ display: flex; gap: 2em; margin: 1em 0; }}
    .stat {{ background: #16213e; padding: 1em; border-radius: 4px; min-width: 120px; }}
    .stat-value {{ font-size: 1.5em; font-weight: bold; color: #e94560; }}
    .stat-label {{ color: #888; font-size: 0.9em; }}
    .refresh {{ margin-top: 1em; }}
    .refresh button {{ background: #e94560; color: white; border: none; padding: 0.5em 1em;
      cursor: pointer; border-radius: 4px; font-family: monospace; }}
    .refresh button:hover {{ background: #c23152; }}
  </style>
</head>
<body>
  <h1>Symphony Orchestrator</h1>
  <p>Generated at: {generated_at}</p>

  <div class="stats">
    <div class="stat"><div class="stat-value">{running}</div><div class="stat-label">Running</div></div>
    <div class="stat"><div class="stat-value">{retrying}</div><div class="stat-label">Retrying</div></div>
    <div class="stat"><div class="stat-value">{total_tokens}</div><div class="stat-label">Total Tokens</div></div>
    <div class="stat"><div class="stat-value">{runtime:.1}s</div><div class="stat-label">Runtime</div></div>
  </div>

  <h2>Running Sessions</h2>
  <table>
    <tr><th>Issue</th><th>State</th><th>Session</th><th>Turns</th><th>Last Event</th><th>Started</th></tr>
    {running_rows}
  </table>

  <h2>Retry Queue</h2>
  <table>
    <tr><th>Issue</th><th>Attempt</th><th>Due At</th><th>Error</th></tr>
    {retry_rows}
  </table>

  <div class="refresh">
    <form method="POST" action="/api/v1/refresh">
      <button type="submit">Trigger Refresh</button>
    </form>
  </div>
</body>
</html>"#,
        generated_at = snapshot.generated_at.format("%Y-%m-%d %H:%M:%S UTC"),
        running = snapshot.counts.running,
        retrying = snapshot.counts.retrying,
        total_tokens = snapshot.codex_totals.total_tokens,
        runtime = snapshot.codex_totals.seconds_running,
        running_rows = running_rows,
        retry_rows = retry_rows,
    );

    Html(html)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::orchestrator::state::{SnapshotCounts, StateSnapshot};
    use crate::runner::TokenTotals;

    fn empty_snapshot() -> StateSnapshot {
        StateSnapshot {
            generated_at: chrono::Utc::now(),
            counts: SnapshotCounts {
                running: 0,
                retrying: 0,
            },
            running: vec![],
            retrying: vec![],
            codex_totals: TokenTotals::default(),
            rate_limits: None,
        }
    }

    #[tokio::test]
    async fn api_state_endpoint() {
        let api_state = Arc::new(Mutex::new(ApiState::new(
            Box::new(empty_snapshot),
            Arc::new(Notify::new()),
        )));

        let app = router(api_state);

        let response = axum::serve(
            tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap(),
            app,
        );

        // Just verify the router builds without panicking
        drop(response);
    }

    #[tokio::test]
    async fn api_router_builds() {
        let api_state = Arc::new(Mutex::new(ApiState::new(
            Box::new(empty_snapshot),
            Arc::new(Notify::new()),
        )));
        let _app = router(api_state);
    }
}
