use axum::{
    extract::{Request, State},
    http::{HeaderMap, StatusCode},
    middleware::Next,
    response::Response,
};
use chrono::Utc;
use sha2::{Digest, Sha256};

use crate::AppState;
use terraphim_persistence::mcp::McpPersistence;

pub async fn validate_api_key(
    State(state): State<AppState>,
    headers: HeaderMap,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let auth_header = headers
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .ok_or(StatusCode::UNAUTHORIZED)?;

    if !auth_header.starts_with("Bearer ") {
        return Err(StatusCode::UNAUTHORIZED);
    }

    let api_key = &auth_header[7..];
    let key_hash = hash_api_key(api_key);

    // Use the shared persistence from AppState
    match McpPersistence::verify_api_key(&*state.mcp_persistence, &key_hash).await {
        Ok(Some(record)) => {
            // Check if the API key is enabled
            if !record.enabled {
                log::warn!("Attempt to use disabled API key: {}", &key_hash[..8]);
                return Err(StatusCode::UNAUTHORIZED);
            }

            // Check if the API key has expired
            if let Some(expires_at) = record.expires_at {
                if expires_at < Utc::now() {
                    log::warn!(
                        "Attempt to use expired API key: {} (expired at: {})",
                        &key_hash[..8],
                        expires_at
                    );
                    return Err(StatusCode::UNAUTHORIZED);
                }
            }

            Ok(next.run(request).await)
        }
        Ok(None) => {
            log::warn!("Invalid API key attempt: {}", &key_hash[..8]);
            Err(StatusCode::UNAUTHORIZED)
        }
        Err(e) => {
            log::error!("Database error during API key validation: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

pub fn hash_api_key(key: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(key.as_bytes());
    format!("{:x}", hasher.finalize())
}
