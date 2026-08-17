//! `GET /v1/models` — list available models (OpenAI compatibility).
//!
//! With an upstream proxy configured the list is fetched from the
//! upstream; otherwise the local TinyClaw model list is returned.

use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde_json::json;

use super::ProxyState;

/// `GET /v1/models`
pub async fn list_models(State(state): State<ProxyState>) -> Response {
    if let Some(upstream) = &state.upstream {
        let url = format!("{}/v1/models", upstream.base_url);
        let mut req = state.client.get(&url);
        if let Some(key) = &upstream.api_key {
            req = req.bearer_auth(key);
        }
        match req.send().await {
            Ok(resp) => {
                let status = resp.status();
                let ct = resp.headers().get(reqwest::header::CONTENT_TYPE).cloned();
                let bytes = match resp.bytes().await {
                    Ok(b) => b.to_vec(),
                    Err(e) => {
                        return (
                            StatusCode::BAD_GATEWAY,
                            Json(json!({"error": format!("upstream read error: {e}")})),
                        )
                            .into_response();
                    }
                };
                let mut builder = axum::http::Response::builder().status(status);
                if let Some(ct) = ct {
                    builder = builder.header("content-type", ct);
                }
                builder
                    .body(axum::body::Body::from(bytes))
                    .unwrap_or_else(|_| StatusCode::BAD_GATEWAY.into_response())
            }
            Err(e) => (
                StatusCode::BAD_GATEWAY,
                Json(json!({"error": format!("upstream unreachable: {e}")})),
            )
                .into_response(),
        }
    } else {
        let models = state.models.lock().await;
        Json(json!({
            "object": "list",
            "data": models.iter().collect::<Vec<_>>(),
        }))
        .into_response()
    }
}
