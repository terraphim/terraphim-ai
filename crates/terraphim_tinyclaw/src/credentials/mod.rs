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

// The bin target doesn't reference every public item — the pool API is
// surface for the library + integration tests. Allow unused imports on the
// re-exports so the public API stays documented even when only a subset
// is wired in by the bin.
#[allow(unused_imports)]
pub use oauth::{OAuthError, OAuthFlow};
#[allow(unused_imports)]
pub use pool::{
    CredentialError, CredentialPool, PoolEntry, PoolStats, ProviderClass, ProviderId, TokenRef,
};
#[allow(unused_imports)]
pub use sources::{EnvFileSource, EnvVarSource};

// Re-export the trait so consumers can implement their own sources.
#[allow(unused_imports)]
pub use pool::CredentialSource;
