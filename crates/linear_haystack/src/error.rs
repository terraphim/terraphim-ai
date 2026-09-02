//! Error types for the linear_haystack crate.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum LinearError {
    /// HTTP transport failure (DNS, TLS, connection reset).
    #[error("Linear HTTP request failed: {0}")]
    Reqwest(#[from] reqwest::Error),

    /// Linear returned a non-2xx HTTP status (other than 429).
    #[error("Linear HTTP {status}: {body}")]
    Http { status: u16, body: String },

    /// Linear returned 429 Too Many Requests; we honor the Retry-After header.
    #[error("Linear rate-limited; retry after {retry_after_secs}s")]
    RateLimited { retry_after_secs: u64 },

    /// Linear returned a GraphQL `errors` array.
    #[error("Linear GraphQL error: {0}")]
    GraphQL(String),

    /// API key missing or unparseable.
    #[error("Linear auth failed: {0}")]
    Auth(String),
}

// We don't have `from_auth` returning a `LinearError` (it uses `anyhow`),
// but if a future caller wants typed auth errors, this slot is reserved.
#[cfg(feature = "linear")]
impl From<terraphim_linear_auth::AuthError> for LinearError {
    fn from(e: terraphim_linear_auth::AuthError) -> Self {
        LinearError::Auth(e.to_string())
    }
}
