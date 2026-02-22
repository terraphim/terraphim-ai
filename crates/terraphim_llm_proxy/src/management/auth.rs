//! Management API authentication middleware.
//!
//! Provides authentication for management endpoints using a shared secret.
//! The secret can be provided via:
//! - `X-Management-Key` header
//! - `Authorization: Bearer <secret>` header
//!
//! # Security
//!
//! - Uses constant-time comparison to prevent timing attacks
//! - Secrets are stored as SHA256 hashes
//! - Failed attempts are logged for security monitoring

use axum::{
    body::Body,
    extract::State,
    http::{header::AUTHORIZATION, Request},
    middleware::Next,
    response::Response,
};
use sha2::{Digest, Sha256};
use tracing::{debug, warn};

use crate::management::error::ManagementError;

/// Header name for management API key
pub const MANAGEMENT_KEY_HEADER: &str = "X-Management-Key";

/// Shared state for management authentication.
#[derive(Clone)]
pub struct ManagementAuthState {
    /// SHA256 hash of the management secret
    secret_hash: [u8; 32],
    /// Whether authentication is enabled
    enabled: bool,
}

impl ManagementAuthState {
    /// Create a new auth state with the given secret.
    ///
    /// The secret is immediately hashed and the original is not stored.
    pub fn new(secret: &str) -> Self {
        Self {
            secret_hash: hash_secret(secret),
            enabled: true,
        }
    }

    /// Create a disabled auth state (for development/testing).
    ///
    /// When disabled, all requests are allowed through.
    pub fn disabled() -> Self {
        Self {
            secret_hash: [0u8; 32],
            enabled: false,
        }
    }

    /// Check if authentication is enabled.
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Verify a secret against the stored hash.
    pub fn verify(&self, secret: &str) -> bool {
        if !self.enabled {
            return true;
        }
        let provided_hash = hash_secret(secret);
        constant_time_compare(&provided_hash, &self.secret_hash)
    }
}

impl std::fmt::Debug for ManagementAuthState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ManagementAuthState")
            .field("enabled", &self.enabled)
            .field("secret_hash", &"[REDACTED]")
            .finish()
    }
}

/// Hash a secret using SHA256.
///
/// This provides a consistent way to store and compare secrets
/// without keeping the plaintext in memory longer than necessary.
pub fn hash_secret(secret: &str) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(secret.as_bytes());
    hasher.finalize().into()
}

/// Generate a hash string for storage (hex encoded).
pub fn hash_secret_hex(secret: &str) -> String {
    let hash = hash_secret(secret);
    hex::encode(&hash)
}

/// Verify a secret against a hex-encoded hash.
pub fn verify_secret_hex(secret: &str, hash_hex: &str) -> bool {
    if let Ok(stored_hash) = hex::decode(hash_hex) {
        if stored_hash.len() == 32 {
            let provided_hash = hash_secret(secret);
            let stored_array: [u8; 32] = stored_hash.try_into().unwrap();
            return constant_time_compare(&provided_hash, &stored_array);
        }
    }
    false
}

/// Constant-time comparison of two byte arrays.
///
/// This prevents timing attacks by ensuring the comparison takes
/// the same amount of time regardless of where the first difference occurs.
fn constant_time_compare(a: &[u8; 32], b: &[u8; 32]) -> bool {
    let mut result = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        result |= x ^ y;
    }
    result == 0
}

/// Extract the management secret from the request.
///
/// Checks in order:
/// 1. `X-Management-Key` header
/// 2. `Authorization: Bearer <secret>` header
fn extract_secret(request: &Request<Body>) -> Option<String> {
    // Try X-Management-Key header first
    if let Some(value) = request.headers().get(MANAGEMENT_KEY_HEADER) {
        if let Ok(secret) = value.to_str() {
            return Some(secret.to_string());
        }
    }

    // Try Authorization: Bearer header
    if let Some(auth) = request.headers().get(AUTHORIZATION) {
        if let Ok(auth_str) = auth.to_str() {
            if let Some(token) = auth_str.strip_prefix("Bearer ") {
                return Some(token.to_string());
            }
        }
    }

    None
}

/// Management authentication middleware.
///
/// Validates the management secret before allowing access to management endpoints.
///
/// # Returns
/// - Continues to next handler if authentication succeeds
/// - Returns 401 Unauthorized if no credentials provided
/// - Returns 403 Forbidden if credentials are invalid
pub async fn management_auth_middleware(
    State(auth_state): State<ManagementAuthState>,
    request: Request<Body>,
    next: Next,
) -> Result<Response, ManagementError> {
    // Skip auth if disabled
    if !auth_state.is_enabled() {
        debug!("Management auth disabled, allowing request");
        return Ok(next.run(request).await);
    }

    // Extract secret from request
    let secret = match extract_secret(&request) {
        Some(s) => s,
        None => {
            debug!("No management secret provided in request");
            return Err(ManagementError::Unauthorized);
        }
    };

    // Verify the secret
    if !auth_state.verify(&secret) {
        warn!(
            "Invalid management secret attempt from {:?}",
            request
                .headers()
                .get("x-forwarded-for")
                .or_else(|| request.headers().get("x-real-ip"))
        );
        return Err(ManagementError::InvalidSecret);
    }

    debug!("Management authentication successful");
    Ok(next.run(request).await)
}

/// Layer for applying management authentication to routes.
///
/// # Example
///
/// ```rust,ignore
/// use axum::{Router, middleware};
/// use terraphim_llm_proxy::management::auth::{ManagementAuthState, management_auth_middleware};
///
/// let auth_state = ManagementAuthState::new("my-secret");
/// let app = Router::new()
///     .route("/management/config", get(get_config))
///     .layer(middleware::from_fn_with_state(auth_state, management_auth_middleware));
/// ```
pub fn management_auth_layer(
    secret: &str,
) -> impl Fn(ManagementAuthState) -> ManagementAuthState + Clone {
    let state = ManagementAuthState::new(secret);
    move |_| state.clone()
}

// Hex encoding/decoding utilities (minimal implementation to avoid new dependency)
mod hex {
    pub fn encode(bytes: &[u8]) -> String {
        bytes.iter().map(|b| format!("{:02x}", b)).collect()
    }

    pub fn decode(s: &str) -> Result<Vec<u8>, ()> {
        if s.len() % 2 != 0 {
            return Err(());
        }

        (0..s.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&s[i..i + 2], 16).map_err(|_| ()))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
        routing::get,
        Router,
    };
    use tower::ServiceExt;

    #[test]
    fn test_hash_secret() {
        let hash1 = hash_secret("test-secret");
        let hash2 = hash_secret("test-secret");
        let hash3 = hash_secret("different-secret");

        // Same secret produces same hash
        assert_eq!(hash1, hash2);

        // Different secrets produce different hashes
        assert_ne!(hash1, hash3);

        // Hash is 32 bytes (SHA256)
        assert_eq!(hash1.len(), 32);
    }

    #[test]
    fn test_hash_secret_hex() {
        let hex = hash_secret_hex("test-secret");

        // Hex string should be 64 characters (32 bytes * 2)
        assert_eq!(hex.len(), 64);

        // Should be valid hex
        assert!(hex.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn test_verify_secret_hex() {
        let secret = "my-management-secret";
        let hash = hash_secret_hex(secret);

        assert!(verify_secret_hex(secret, &hash));
        assert!(!verify_secret_hex("wrong-secret", &hash));
        assert!(!verify_secret_hex(secret, "invalid-hash"));
        assert!(!verify_secret_hex(secret, "abc")); // Too short
    }

    #[test]
    fn test_constant_time_compare() {
        let a = [1u8; 32];
        let b = [1u8; 32];
        let c = [2u8; 32];

        assert!(constant_time_compare(&a, &b));
        assert!(!constant_time_compare(&a, &c));
    }

    #[test]
    fn test_management_auth_state_new() {
        let state = ManagementAuthState::new("secret");

        assert!(state.is_enabled());
        assert!(state.verify("secret"));
        assert!(!state.verify("wrong"));
    }

    #[test]
    fn test_management_auth_state_disabled() {
        let state = ManagementAuthState::disabled();

        assert!(!state.is_enabled());
        // When disabled, any secret should pass
        assert!(state.verify("anything"));
        assert!(state.verify(""));
    }

    #[test]
    fn test_management_auth_state_debug() {
        let state = ManagementAuthState::new("my-super-secret-key");
        let debug = format!("{:?}", state);

        // Secret hash should be redacted in debug output
        assert!(debug.contains("REDACTED"));
        // The actual secret value should not appear
        assert!(!debug.contains("my-super-secret-key"));
    }

    #[test]
    fn test_extract_secret_from_header() {
        let request = Request::builder()
            .header(MANAGEMENT_KEY_HEADER, "my-secret")
            .body(Body::empty())
            .unwrap();

        let secret = extract_secret(&request);
        assert_eq!(secret, Some("my-secret".to_string()));
    }

    #[test]
    fn test_extract_secret_from_bearer() {
        let request = Request::builder()
            .header(AUTHORIZATION, "Bearer my-bearer-token")
            .body(Body::empty())
            .unwrap();

        let secret = extract_secret(&request);
        assert_eq!(secret, Some("my-bearer-token".to_string()));
    }

    #[test]
    fn test_extract_secret_prefers_header() {
        let request = Request::builder()
            .header(MANAGEMENT_KEY_HEADER, "header-secret")
            .header(AUTHORIZATION, "Bearer bearer-secret")
            .body(Body::empty())
            .unwrap();

        let secret = extract_secret(&request);
        // X-Management-Key takes precedence
        assert_eq!(secret, Some("header-secret".to_string()));
    }

    #[test]
    fn test_extract_secret_none() {
        let request = Request::builder().body(Body::empty()).unwrap();

        let secret = extract_secret(&request);
        assert!(secret.is_none());
    }

    #[test]
    fn test_extract_secret_invalid_bearer() {
        let request = Request::builder()
            .header(AUTHORIZATION, "Basic dXNlcjpwYXNz") // Basic auth, not Bearer
            .body(Body::empty())
            .unwrap();

        let secret = extract_secret(&request);
        assert!(secret.is_none());
    }

    #[tokio::test]
    async fn test_middleware_allows_valid_secret() {
        let auth_state = ManagementAuthState::new("valid-secret");

        async fn handler() -> &'static str {
            "success"
        }

        let app =
            Router::new()
                .route("/test", get(handler))
                .layer(axum::middleware::from_fn_with_state(
                    auth_state,
                    management_auth_middleware,
                ));

        let request = Request::builder()
            .uri("/test")
            .header(MANAGEMENT_KEY_HEADER, "valid-secret")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_middleware_rejects_invalid_secret() {
        let auth_state = ManagementAuthState::new("valid-secret");

        async fn handler() -> &'static str {
            "success"
        }

        let app =
            Router::new()
                .route("/test", get(handler))
                .layer(axum::middleware::from_fn_with_state(
                    auth_state,
                    management_auth_middleware,
                ));

        let request = Request::builder()
            .uri("/test")
            .header(MANAGEMENT_KEY_HEADER, "wrong-secret")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn test_middleware_rejects_missing_secret() {
        let auth_state = ManagementAuthState::new("valid-secret");

        async fn handler() -> &'static str {
            "success"
        }

        let app =
            Router::new()
                .route("/test", get(handler))
                .layer(axum::middleware::from_fn_with_state(
                    auth_state,
                    management_auth_middleware,
                ));

        let request = Request::builder().uri("/test").body(Body::empty()).unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_middleware_allows_when_disabled() {
        let auth_state = ManagementAuthState::disabled();

        async fn handler() -> &'static str {
            "success"
        }

        let app =
            Router::new()
                .route("/test", get(handler))
                .layer(axum::middleware::from_fn_with_state(
                    auth_state,
                    management_auth_middleware,
                ));

        // No auth header at all
        let request = Request::builder().uri("/test").body(Body::empty()).unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_middleware_accepts_bearer_token() {
        let auth_state = ManagementAuthState::new("bearer-secret");

        async fn handler() -> &'static str {
            "success"
        }

        let app =
            Router::new()
                .route("/test", get(handler))
                .layer(axum::middleware::from_fn_with_state(
                    auth_state,
                    management_auth_middleware,
                ));

        let request = Request::builder()
            .uri("/test")
            .header(AUTHORIZATION, "Bearer bearer-secret")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[test]
    fn test_hex_encode_decode() {
        let original = [0xde, 0xad, 0xbe, 0xef];
        let encoded = hex::encode(&original);
        assert_eq!(encoded, "deadbeef");

        let decoded = hex::decode(&encoded).unwrap();
        assert_eq!(decoded, original);
    }

    #[test]
    fn test_hex_decode_invalid() {
        assert!(hex::decode("xyz").is_err()); // Invalid chars
        assert!(hex::decode("abc").is_err()); // Odd length
    }
}
