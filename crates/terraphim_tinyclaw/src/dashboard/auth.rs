//! Shared bearer-token authorization for dashboard endpoints.
//!
//! The dashboard exposes two endpoints that drive side-effecting
//! behaviour from the network: `POST /api/cron/fire` (Chronos managed
//! fire webhook) and `POST /api/agent/messages` (agent-loop injection).
//! Both must enforce the same `Authorization: Bearer <token>`
//! contract when `DashboardState::fire_token` is configured (from the
//! `TINYCLAW_FIRE_TOKEN` env var).
//!
//! Centralising the check here keeps the two endpoints from drifting
//! and keeps the failure mode uniform. When `fire_token` is `None`
//! (dev/test mode) the helper returns `None` and the endpoint stays
//! open; callers are responsible for network-level isolation.

use axum::Json;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use serde_json::json;

/// Check the dashboard's bearer-token authorization.
///
/// Returns:
/// - `None` — caller may proceed (no token configured, OR the request
///   carries a matching `Authorization: Bearer <token>` header).
/// - `Some(response)` — the caller must return this 401 response
///   verbatim. The body shape is `{"error": "invalid auth token"}` so
///   it stays aligned with the cron-fire contract documented in
///   Hermes' `web_server.py`.
pub fn require_fire_token(fire_token: &Option<String>, headers: &HeaderMap) -> Option<Response> {
    let expected = fire_token.as_deref()?;
    let provided = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "));
    if provided == Some(expected) {
        return None;
    }
    Some(unauthorized_response())
}

/// 401 response with the canonical error shape.
fn unauthorized_response() -> Response {
    (
        StatusCode::UNAUTHORIZED,
        Json(json!({ "error": "invalid auth token" })),
    )
        .into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderValue;

    fn headers_with_bearer(value: &str) -> HeaderMap {
        let mut h = HeaderMap::new();
        h.insert(
            axum::http::header::AUTHORIZATION,
            HeaderValue::from_str(value).unwrap(),
        );
        h
    }

    #[test]
    fn require_fire_token_passes_when_no_token_configured() {
        let headers = HeaderMap::new();
        assert!(require_fire_token(&None, &headers).is_none());
        assert!(require_fire_token(&None, &headers_with_bearer("Bearer x")).is_none());
    }

    #[test]
    fn require_fire_token_passes_with_correct_bearer() {
        let headers = headers_with_bearer("Bearer expected");
        assert!(require_fire_token(&Some("expected".into()), &headers).is_none());
    }

    #[test]
    fn require_fire_token_rejects_missing_header_when_configured() {
        let headers = HeaderMap::new();
        let resp = require_fire_token(&Some("expected".into()), &headers).expect("401 response");
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[test]
    fn require_fire_token_rejects_wrong_bearer() {
        let headers = headers_with_bearer("Bearer wrong");
        let resp = require_fire_token(&Some("expected".into()), &headers).expect("401 response");
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[test]
    fn require_fire_token_rejects_non_bearer_scheme() {
        let headers = headers_with_bearer("Basic expected");
        let resp = require_fire_token(&Some("expected".into()), &headers).expect("401 response");
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }
}
