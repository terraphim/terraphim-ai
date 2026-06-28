//! Shared helpers for `terraphim_server` integration tests.
//!
//! Centralises the readiness-poll pattern so tests do not each reinvent a
//! `sleep` heuristic (the documented root cause of the
//! `test_default_role_ripgrep_integration` flake in #2998 / #2947).
//!
//! Usage:
//! ```no_run
//! # use std::net::SocketAddr;
//! # async fn spawn(_: SocketAddr) {}
//! use terraphim_server::axum_server;
//! mod common;
//! use common::wait_for_health;
//!
//! # #[tokio::test]
//! # async fn demo() {
//! let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
//! // NB: bind an ephemeral port first, then hand the resolved address in.
//! // see `bind_ephemeral` below.
//! let ready = wait_for_health(addr, 60).await;
//! # }
//! ```
use std::net::SocketAddr;
use std::time::Duration;

/// Default per-attempt sleep when polling `/health`.
const POLL_INTERVAL: Duration = Duration::from_millis(250);

/// Poll `GET /health` until it returns HTTP 200, panicking after `max_attempts`.
///
/// Deterministic replacement for `sleep(N seconds)` before issuing search
/// requests. Returns the address once the server is ready.
///
/// # Panics
/// Panics with a descriptive message if the server does not return 200 within
/// `max_attempts * 250ms`, surfacing the last transport error for diagnosis.
pub async fn wait_for_health(addr: SocketAddr, max_attempts: u32) -> SocketAddr {
    let client = terraphim_service::http_client::create_default_client()
        .expect("Failed to create HTTP client for /health polling");
    let health_url = format!("http://{addr}/health");

    let mut last_err = String::from("no attempt made");
    for attempt in 0..max_attempts {
        match client.get(&health_url).send().await {
            Ok(response) if response.status().is_success() => return addr,
            Ok(response) => {
                last_err = format!("HTTP {}", response.status());
            }
            Err(e) => {
                last_err = e.to_string();
            }
        }
        tokio::time::sleep(POLL_INTERVAL).await;
        if attempt > 0 && attempt % 8 == 0 {
            eprintln!(
                "wait_for_health: still not ready at {addr} after {attempt} attempts ({last_err})"
            );
        }
    }
    panic!(
        "Server at {addr} did not become ready within {} attempts ({})",
        max_attempts, last_err
    );
}
