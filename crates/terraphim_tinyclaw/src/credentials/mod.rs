//! Credential pool for tinyclaw LLM providers.
//!
//! Wave 1 of the Hermes parity arc (epic #3160).
//!
//! Mirrors the *shape* of Hermes' `credential_pool.py` (2,806 LOC) at minimal
//! scope: an ordered list of `PoolEntry` records per `ProviderClass`, rotation
//! with cooldown reporting. Hermes' full feature set (OAuth managers, refresh
//! tokens, rate-limit backoff) is out of scope here; we ship the architectural
//! seam so Wave 2+ can plug richer sources in without touching `HybridLlmRouter`.
//!
//! **Default behaviour: disabled.** The pool is only consulted when
//! `Config.credentials.enabled = true`. Otherwise the existing env-var path
//! remains unchanged (rollback = config flag, no code revert).
//!
//! **Security invariant**: a `TokenRef` holds the *name* of an env var or the
//! *path* of a file — never the secret itself. The secret is materialised only
//! at the point of use (e.g. when constructing an HTTP `Authorization` header).

mod oauth;
mod pool;
mod sources;

pub use oauth::{OAuthError, OAuthFlow};
pub use pool::{
    CredentialError, CredentialPool, PoolEntry, PoolStats, ProviderClass, ProviderId, TokenRef,
};
pub use sources::{EnvFileSource, EnvVarSource};

// Re-export the trait so consumers can implement their own sources.
pub use pool::CredentialSource;
