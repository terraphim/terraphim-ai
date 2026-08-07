//! OAuth stub — Hermes parity seam.
//!
//! In Hermes, credential sources beyond env vars include OAuth managers (Google,
//! GitHub, etc.) with refresh-token flows. Wave 1 ships the *trait* only — no
//! concrete OAuth provider is implemented. A future wave (probably Wave 6 with
//! the plugin-model evaluation) will decide whether OAuth ships in Rust at all
//! or stays as a Python-bridged concern.

use std::fmt;

/// Trait for OAuth flows that can mint short-lived tokens from a long-lived
/// refresh token.
///
/// Implementations are async because real OAuth providers require HTTP I/O.
/// The stub here is `Sync` to keep the type simple — concrete providers may
/// need an async runtime and a tokio client.
pub trait OAuthFlow: Send + Sync + fmt::Debug {
    /// Provider identifier (e.g. `"google"`, `"github"`).
    fn provider_id(&self) -> &str;

    /// Mint a fresh access token from the stored refresh token.
    ///
    /// Returns the access token and its lifetime in seconds, or an
    /// `OAuthError` if the refresh failed (network error, refresh token
    /// revoked, etc.).
    fn refresh(&self) -> Result<RefreshedToken, OAuthError>;
}

/// Token returned by a successful `OAuthFlow::refresh`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RefreshedToken {
    /// The bearer token to use for outgoing API calls.
    pub access_token: String,
    /// Lifetime in seconds (provider response's `expires_in`).
    pub expires_in_secs: u64,
}

/// OAuth flow errors. Mirrors the failure modes a real refresh would encounter.
#[derive(Debug, thiserror::Error)]
pub enum OAuthError {
    /// Network or transport-level failure.
    #[error("network error during OAuth refresh: {0}")]
    Network(String),

    /// The refresh token is no longer valid (revoked, expired, or the user
    /// revoked the app). The caller must restart the OAuth dance.
    #[error("refresh token revoked or invalid")]
    Revoked,

    /// Provider returned an unexpected HTTP status (5xx, etc.).
    #[error("provider error (HTTP {status}): {message}")]
    ProviderError { status: u16, message: String },
}

/// Placeholder type used in tests and as the default. Does not perform any I/O.
#[derive(Debug)]
pub struct NoopOAuthFlow {
    provider: String,
}

impl NoopOAuthFlow {
    /// Create a stub OAuth flow that always returns `Revoked`. Useful for
    /// hermetic tests and as the default value when no real provider is wired.
    pub fn new(provider: impl Into<String>) -> Self {
        Self {
            provider: provider.into(),
        }
    }
}

impl OAuthFlow for NoopOAuthFlow {
    fn provider_id(&self) -> &str {
        &self.provider
    }

    fn refresh(&self) -> Result<RefreshedToken, OAuthError> {
        Err(OAuthError::Revoked)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn noop_flow_returns_revoked() {
        let flow = NoopOAuthFlow::new("noop");
        assert_eq!(flow.provider_id(), "noop");
        assert!(matches!(flow.refresh(), Err(OAuthError::Revoked)));
    }

    #[test]
    fn refreshed_token_equality() {
        let a = RefreshedToken {
            access_token: "abc".to_string(),
            expires_in_secs: 3600,
        };
        let b = RefreshedToken {
            access_token: "abc".to_string(),
            expires_in_secs: 3600,
        };
        assert_eq!(a, b);
    }
}
