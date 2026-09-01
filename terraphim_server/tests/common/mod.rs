//! Shared integration-test helpers for `terraphim_server`.
//!
//! Every integration test that boots the live Axum server via
//! [`terraphim_server::axum_server`] must wait for readiness through
//! [`wait_for_server_ready`] **before** issuing any other request. This
//! replaces the previous mix of blind `sleep(3s)` startup waits (which raced
//! the router and caused flaky failures such as the port-8085
//! `test_default_role_ripgrep_integration`) and the four divergent private
//! poll loops that had silently drifted apart in budget and interval.
//!
//! Replaces the readiness logic requested by issue #2998.

use std::net::SocketAddr;
use std::time::Duration;

/// Interval between readiness probes.
///
/// 250ms gives sub-second happy-path startup detection while keeping the
/// request rate trivial on the router.
pub const READY_PROBE_INTERVAL: Duration = Duration::from_millis(250);

/// Maximum number of readiness probes before giving up.
///
/// 120 attempts at 250ms = a ≈30s wall-clock worst case. This is strictly
/// more generous than every prior private copy (5s / 5s / 5s / 30s) so it
/// cannot regress any test while tolerating contended CI runners.
pub const READY_PROBE_MAX_ATTEMPTS: u32 = 120;

/// Poll `GET /health` on `address` until it responds HTTP 200.
///
/// Panics with a descriptive message once the readiness budget
/// ([`READY_PROBE_MAX_ATTEMPTS`] × [`READY_PROBE_INTERVAL`]) is exhausted,
/// so a server that fails to boot surfaces as an immediate, attributed test
/// failure rather than a cascade of mysterious request errors.
///
/// Use this at the top of every integration test that starts the server,
/// instead of a fixed `sleep`, so startup timing can never race the
/// assertions. This is the contract requested by issue #2998.
pub async fn wait_for_server_ready(address: SocketAddr) {
    wait_for_server_ready_with_budget(address, READY_PROBE_MAX_ATTEMPTS, READY_PROBE_INTERVAL).await
}

/// Configurable variant for callers that need to bound the probe budget
/// (e.g. a fast-failing self-test of this helper).
pub async fn wait_for_server_ready_with_budget(
    address: SocketAddr,
    max_attempts: u32,
    interval: Duration,
) {
    let client = terraphim_service::http_client::create_default_client()
        .expect("Failed to create HTTP client");
    let health_url = format!("http://{address}/health");

    for attempt in 0..max_attempts {
        match client.get(&health_url).send().await {
            Ok(response) if response.status() == 200 => return,
            _ => {
                if attempt + 1 < max_attempts {
                    tokio::time::sleep(interval).await;
                }
            }
        }
    }

    let budget_secs = max_attempts as f64 * interval.as_secs_f64();
    panic!(
        "Server at {address} did not report ready via GET /health \
         after {max_attempts} attempts (≈{budget_secs:.1}s budget). \
         Ensure `axum_server` is spawned before waiting for readiness."
    );
}
