use axum::{
    extract::{Request, State},
    http::{HeaderMap, StatusCode},
    middleware::Next,
    response::Response,
};
use sha2::{Digest, Sha256};

use crate::AppState;
use terraphim_persistence::mcp::McpPersistence;

pub async fn validate_api_key(
    State(_state): State<AppState>,
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

    let persistence = get_mcp_persistence().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    match McpPersistence::verify_api_key(&*persistence, &key_hash).await {
        Ok(Some(_record)) => Ok(next.run(request).await),
        Ok(None) => Err(StatusCode::UNAUTHORIZED),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

fn hash_api_key(key: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(key.as_bytes());
    format!("{:x}", hasher.finalize())
}

fn get_mcp_persistence() -> Result<
    std::sync::Arc<terraphim_persistence::mcp::McpPersistenceImpl>,
    Box<dyn std::error::Error>,
> {
    use opendal::services::Memory;
    use opendal::Operator;

    let builder = Memory::default();
    let op = Operator::new(builder)?.finish();
    Ok(std::sync::Arc::new(
        terraphim_persistence::mcp::McpPersistenceImpl::new(op),
    ))
}
