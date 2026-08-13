//! Production HTTP ingress for webhook-backed channels.

use axum::body::Bytes;
use axum::extract::{Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use serde::Deserialize;
use serde_json::json;

use super::DashboardState;
use crate::channels::teams::TeamsChannel;
use crate::channels::whatsapp::WhatsAppChannel;

#[derive(Debug, Deserialize)]
pub struct WhatsAppVerifyQuery {
    #[serde(rename = "hub.mode")]
    mode: String,
    #[serde(rename = "hub.verify_token")]
    verify_token: String,
    #[serde(rename = "hub.challenge")]
    challenge: String,
}

pub async fn whatsapp_verify(
    State(state): State<DashboardState>,
    Query(query): Query<WhatsAppVerifyQuery>,
) -> Response {
    let Some(config) = state.inbound_channels.whatsapp.clone() else {
        return StatusCode::NOT_FOUND.into_response();
    };
    let channel = WhatsAppChannel::new(config);
    if !channel.verify_subscription(&query.mode, &query.verify_token) {
        return StatusCode::FORBIDDEN.into_response();
    }
    (StatusCode::OK, query.challenge).into_response()
}

pub async fn whatsapp_webhook(
    State(state): State<DashboardState>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let Some(config) = state.inbound_channels.whatsapp.clone() else {
        return StatusCode::NOT_FOUND.into_response();
    };
    let channel = WhatsAppChannel::new(config);
    let signature = headers
        .get("x-hub-signature-256")
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default();
    if !channel.verify_webhook_signature(&body, signature) {
        return StatusCode::UNAUTHORIZED.into_response();
    }

    let messages = match channel.parse_webhook(&body) {
        Ok(messages) => messages,
        Err(err) => {
            return (
                StatusCode::BAD_REQUEST,
                axum::Json(json!({ "error": err.to_string() })),
            )
                .into_response();
        }
    };
    for message in messages {
        if let Err(err) = state.bus.inbound_sender().send(message).await {
            log::warn!("WhatsApp webhook dispatch failed: {err}");
            return StatusCode::ACCEPTED.into_response();
        }
    }
    StatusCode::ACCEPTED.into_response()
}

pub async fn teams_webhook(
    State(state): State<DashboardState>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let Some(config) = state.inbound_channels.teams.clone() else {
        return StatusCode::NOT_FOUND.into_response();
    };
    let channel = TeamsChannel::new(config);
    let authorization = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default();
    if !channel.has_bearer_authorization(authorization) {
        return StatusCode::UNAUTHORIZED.into_response();
    }
    // This route enforces Bot Framework bearer-header shape before
    // parse/dispatch. It does not perform cryptographic JWT validation.
    let Some(message) = (match channel.parse_activity(&body) {
        Ok(message) => message,
        Err(err) => {
            return (
                StatusCode::BAD_REQUEST,
                axum::Json(json!({ "error": err.to_string() })),
            )
                .into_response();
        }
    }) else {
        return StatusCode::ACCEPTED.into_response();
    };

    if let Err(err) = state.bus.inbound_sender().send(message).await {
        log::warn!("Teams webhook dispatch failed: {err}");
    }
    StatusCode::ACCEPTED.into_response()
}
