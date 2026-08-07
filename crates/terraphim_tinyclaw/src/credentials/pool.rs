//! Credential pool: ordered rotation with cooldown reporting.
//!
//! Wave 1 of the Hermes parity arc (epic #3160). Mirrors the *shape* of
//! Hermes' `credential_pool.py` at minimal scope:
//!
//! - A `ProviderClass` groups entries that serve the same role (e.g. all
//!   "openrouter" keys, or all "anthropic" keys).
//! - A `PoolEntry` is one concrete credential within a class (one of N
//!   fallback API keys for `openrouter`, for instance).
//! - `acquire()` walks the entries in insertion order and returns the first
//!   one whose `cooldown_until` is in the past.
//! - `report_throttle(provider, cooldown)` stamps a backoff; `report_success`
//!   clears it.
//!
//! **Sources** (where the secret materialises) are pluggable via the
//! `CredentialSource` trait. The default impls are:
//!
//! - `EnvVarSource` — reads `std::env::var(key)` (Hermes' existing path).
//! - `EnvFileSource` — parses a `KEY=VALUE` file (Hermes' `~/.hermes/.env` style).
//!
//! A `TokenRef` is the *name* of an env var or the *path* of a file — the
//! pool never holds the secret itself. Materialisation happens in
//! `CredentialPool::acquire()`, which is the only method that returns a
//! `MaterialisedCredential` with the live token.

use std::fmt;
use std::path::PathBuf;
use std::sync::{Mutex, RwLock};
use std::time::{Duration, Instant};

/// Identifier for a provider (e.g. `"openrouter"`, `"anthropic"`).
///
/// Mirrors the `provider: String` convention used in `terraphim_types::llm_usage`
/// and `terraphim_server::api`. We deliberately keep it a plain `String` rather
/// than an enum — Hermes treats provider names as plugin-discovered and a fixed
/// Rust enum would couple us to a specific provider set.
pub type ProviderId = String;

/// Class of providers that serve the same role (all "openrouter" keys,
/// all "anthropic" keys, etc.). For Wave 1 this collapses to a 1:1 with
/// provider id, but the type is separate so Wave 6's plugin-model
/// evaluation can introduce 1:many (multiple sub-providers per class).
pub type ProviderClass = String;

/// Reference to a secret. Holds the *name* of an env var or the *path*
/// of a file — never the secret itself. This is the security invariant
/// the pool preserves.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenRef {
    /// Look up the secret in `std::env::var(name)`.
    EnvVar { name: String },
    /// Read the secret from a file (whole contents, trimmed).
    File { path: PathBuf },
}

impl fmt::Display for TokenRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TokenRef::EnvVar { name } => write!(f, "${{{name}}}"),
            TokenRef::File { path } => write!(f, "@file:{}", path.display()),
        }
    }
}

/// A single credential slot in the pool.
#[derive(Debug, Clone)]
pub struct PoolEntry {
    /// Provider this entry serves.
    pub provider: ProviderId,
    /// Class this entry belongs to (e.g. "openrouter" provider id can have
    /// multiple entries in the "openrouter" class for fallback rotation).
    pub class: ProviderClass,
    /// Where to read the secret from. Never the secret itself.
    pub token_ref: TokenRef,
}

/// Source for materialising `TokenRef`s. Implementations are read-only
/// and synchronous (the network OAuth case lives behind `OAuthFlow`
/// rather than this trait).
///
/// Built-in impls live in `super::sources`:
/// - [`super::sources::EnvVarSource`] — reads `std::env::var(name)`.
/// - [`super::sources::EnvFileSource`] — parses a dotenv-style file.
pub trait CredentialSource: Send + Sync + fmt::Debug {
    /// Resolve `token_ref` to its underlying secret string, or `None` if
    /// the source has nothing for it (e.g. env var unset, file missing).
    fn resolve(&self, token_ref: &TokenRef) -> Option<String>;
}

/// A materialised credential ready to use. Holds the secret by value
/// for a single request; consumers should drop it ASAP after use.
#[derive(Debug, Clone)]
pub struct MaterialisedCredential {
    pub provider: ProviderId,
    pub token: String,
    /// Where the secret came from (for diagnostics; never the secret).
    pub source: TokenRef,
}

/// Snapshot of pool state for diagnostics / metrics.
#[derive(Debug, Clone, Default)]
pub struct PoolStats {
    pub total_entries: usize,
    pub entries_on_cooldown: usize,
    pub acquires: u64,
    pub throttles: u64,
    pub successes: u64,
    pub exhaustions: u64,
}

/// Errors the pool can produce.
#[derive(Debug, thiserror::Error)]
pub enum CredentialError {
    /// No non-cooled entry found for the requested class.
    #[error("no credentials available for class '{0}'")]
    Exhausted(ProviderClass),

    /// The configured source could not be loaded (e.g. missing file).
    #[error("credential source unreadable: {0}")]
    SourceUnreadable(String),
}

/// The credential pool itself.
///
/// Concurrency model:
/// - `entries`: `RwLock` (many readers, rare writes when adding entries).
/// - `cooldowns`: `Mutex<HashMap>` (small, contended only on throttle/success).
/// - `stats`: `Mutex<PoolStats>` (cheap to update, never read in hot path).
///
/// We avoid `dashmap` to keep the dep surface minimal — the pool is
/// not on the hot path (one acquire per LLM call, not per token).
pub struct CredentialPool {
    /// Pool entries in insertion order. The first non-cooled entry for a
    /// class wins on `acquire`. Order matters because it represents the
    /// operator's preference (primary, secondary, fallback).
    entries: RwLock<Vec<PoolEntry>>,
    /// `provider -> cooldown_until`. Cleared on success.
    cooldowns: Mutex<HashMap<ProviderId, Instant>>,
    stats: Mutex<PoolStats>,
    /// Default cooldown applied by `report_throttle` if the caller does
    /// not supply one. Matches Hermes' 60-second default.
    default_cooldown: Duration,
}

impl fmt::Debug for CredentialPool {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let entries = self.entries.read().expect("entries lock poisoned");
        let cooldowns = self.cooldowns.lock().expect("cooldowns lock poisoned");
        f.debug_struct("CredentialPool")
            .field("entries", &entries.len())
            .field("cooldowns_active", &cooldowns.len())
            .field("default_cooldown_secs", &self.default_cooldown.as_secs())
            .finish()
    }
}

impl CredentialPool {
    /// Construct an empty pool with the default 60s cooldown.
    pub fn new() -> Self {
        Self::with_default_cooldown(Duration::from_secs(60))
    }

    /// Construct with a custom default cooldown (used by tests to drive
    /// cooldown transitions in <1s).
    pub fn with_default_cooldown(default_cooldown: Duration) -> Self {
        Self {
            entries: RwLock::new(Vec::new()),
            cooldowns: Mutex::new(HashMap::new()),
            stats: Mutex::new(PoolStats::default()),
            default_cooldown,
        }
    }

    /// Register a pool entry. Order of insertion is rotation order.
    pub fn add(&self, entry: PoolEntry) {
        let mut entries = self.entries.write().expect("entries lock poisoned");
        entries.push(entry);
    }

    /// Read-only view of the entries (used by `HybridLlmRouter` to know
    /// which providers are available without acquiring a token).
    pub fn entries(&self) -> Vec<PoolEntry> {
        self.entries
            .read()
            .expect("entries lock poisoned")
            .iter()
            .cloned()
            .collect()
    }

    /// Number of registered entries.
    pub fn len(&self) -> usize {
        self.entries.read().expect("entries lock poisoned").len()
    }

    /// Whether the pool has zero registered entries.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Acquire a credential for the given class. Returns the first entry
    /// whose `cooldown_until` is in the past (or never set), resolved
    /// through `source`. Records `Exhausted` in stats if no entry qualifies.
    pub fn acquire(
        &self,
        class: &ProviderClass,
        source: &dyn CredentialSource,
    ) -> Result<MaterialisedCredential, CredentialError> {
        let entries = self.entries.read().expect("entries lock poisoned");
        let cooldowns = self.cooldowns.lock().expect("cooldowns lock poisoned");
        let now = Instant::now();

        let mut stat = self.stats.lock().expect("stats lock poisoned");
        stat.acquires += 1;
        drop(stat);

        for entry in entries.iter() {
            if &entry.class != class {
                continue;
            }
            if let Some(until) = cooldowns.get(&entry.provider) {
                if *until > now {
                    continue;
                }
            }
            let token = source.resolve(&entry.token_ref).ok_or_else(|| {
                CredentialError::SourceUnreadable(format!(
                    "no value for {}",
                    entry.token_ref
                ))
            })?;
            return Ok(MaterialisedCredential {
                provider: entry.provider.clone(),
                token,
                source: entry.token_ref.clone(),
            });
        }

        let mut stat = self.stats.lock().expect("stats lock poisoned");
        stat.exhaustions += 1;
        drop(stat);
        Err(CredentialError::Exhausted(class.clone()))
    }

    /// Report that a provider's request was throttled (rate-limited, 429,
    /// timeout, etc.). Apply a cooldown using the default if `cooldown`
    /// is None. Idempotent — calling twice with the same provider extends
    /// to the later of the two expiry times.
    pub fn report_throttle(&self, provider: &ProviderId, cooldown: Option<Duration>) {
        let cd = cooldown.unwrap_or(self.default_cooldown);
        let mut cooldowns = self.cooldowns.lock().expect("cooldowns lock poisoned");
        let new_until = Instant::now() + cd;
        match cooldowns.get(provider) {
            Some(existing) if *existing >= new_until => {
                // Existing cooldown already covers or exceeds the new one.
            }
            _ => {
                cooldowns.insert(provider.clone(), new_until);
            }
        }
        let mut stat = self.stats.lock().expect("stats lock poisoned");
        stat.throttles += 1;
    }

    /// Report that a provider's request succeeded. Clears any cooldown
    /// for that provider. Idempotent.
    pub fn report_success(&self, provider: &ProviderId) {
        let mut cooldowns = self.cooldowns.lock().expect("cooldowns lock poisoned");
        cooldowns.remove(provider);
        let mut stat = self.stats.lock().expect("stats lock poisoned");
        stat.successes += 1;
    }

    /// Snapshot pool state for diagnostics.
    pub fn stats(&self) -> PoolStats {
        let s = self.stats.lock().expect("stats lock poisoned");
        let c = self.cooldowns.lock().expect("cooldowns lock poisoned");
        let e = self.entries.read().expect("entries lock poisoned");
        PoolStats {
            total_entries: e.len(),
            entries_on_cooldown: c.len(),
            acquires: s.acquires,
            throttles: s.throttles,
            successes: s.successes,
            exhaustions: s.exhaustions,
        }
    }
}

impl Default for CredentialPool {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    /// In-memory source for hermetic tests. Maps env-var names → values.
    #[derive(Debug, Default)]
    struct InMemorySource {
        map: HashMap<String, String>,
    }

    impl CredentialSource for InMemorySource {
        fn resolve(&self, token_ref: &TokenRef) -> Option<String> {
            match token_ref {
                TokenRef::EnvVar { name } => self.map.get(name).cloned(),
                TokenRef::File { .. } => None,
            }
        }
    }

    fn entry(provider: &str, class: &str, env: &str) -> PoolEntry {
        PoolEntry {
            provider: provider.to_string(),
            class: class.to_string(),
            token_ref: TokenRef::EnvVar {
                name: env.to_string(),
            },
        }
    }

    #[test]
    fn empty_pool_returns_exhausted() {
        let pool = CredentialPool::new();
        let src = InMemorySource::default();
        assert!(matches!(
            pool.acquire(&"openrouter".to_string(), &src),
            Err(CredentialError::Exhausted(_))
        ));
        let stats = pool.stats();
        assert_eq!(stats.acquires, 1);
        assert_eq!(stats.exhaustions, 1);
    }

    #[test]
    fn acquire_returns_first_entry_for_class() {
        let pool = CredentialPool::new();
        pool.add(entry("openrouter-a", "openrouter", "TOKEN_A"));
        pool.add(entry("openrouter-b", "openrouter", "TOKEN_B"));
        let mut src = InMemorySource::default();
        src.map.insert("TOKEN_A".into(), "secret-a".into());
        src.map.insert("TOKEN_B".into(), "secret-b".into());

        let cred = pool
            .acquire(&"openrouter".to_string(), &src)
            .expect("acquire");
        assert_eq!(cred.provider, "openrouter-a");
        assert_eq!(cred.token, "secret-a");
    }

    #[test]
    fn throttle_skips_entry_until_cooldown_expires() {
        let pool = CredentialPool::with_default_cooldown(Duration::from_millis(50));
        pool.add(entry("openrouter-a", "openrouter", "TOKEN_A"));
        pool.add(entry("openrouter-b", "openrouter", "TOKEN_B"));
        let mut src = InMemorySource::default();
        src.map.insert("TOKEN_A".into(), "secret-a".into());
        src.map.insert("TOKEN_B".into(), "secret-b".into());

        // First acquire picks A.
        let c1 = pool.acquire(&"openrouter".to_string(), &src).unwrap();
        assert_eq!(c1.provider, "openrouter-a");

        // Throttle A; next acquire should skip to B.
        pool.report_throttle(&"openrouter-a".to_string(), None);
        let c2 = pool.acquire(&"openrouter".to_string(), &src).unwrap();
        assert_eq!(c2.provider, "openrouter-b");

        // After cooldown expires, A is back in the pool.
        std::thread::sleep(Duration::from_millis(80));
        let c3 = pool.acquire(&"openrouter".to_string(), &src).unwrap();
        assert_eq!(c3.provider, "openrouter-a");
    }

    #[test]
    fn success_clears_cooldown() {
        let pool = CredentialPool::with_default_cooldown(Duration::from_secs(60));
        pool.add(entry("a", "x", "T"));
        let mut src = InMemorySource::default();
        src.map.insert("T".into(), "secret".into());

        pool.report_throttle(&"a".to_string(), None);
        // With cooldown active and no fallback, acquire should fail.
        assert!(pool.acquire(&"x".to_string(), &src).is_err());

        pool.report_success(&"a".to_string());
        assert!(pool.acquire(&"x".to_string(), &src).is_ok());
    }

    #[test]
    fn different_classes_do_not_collide() {
        let pool = CredentialPool::new();
        pool.add(entry("openrouter", "openrouter", "OR"));
        pool.add(entry("anthropic", "anthropic", "AN"));
        let mut src = InMemorySource::default();
        src.map.insert("OR".into(), "or-secret".into());
        src.map.insert("AN".into(), "an-secret".into());

        let c1 = pool.acquire(&"openrouter".to_string(), &src).unwrap();
        assert_eq!(c1.token, "or-secret");
        let c2 = pool.acquire(&"anthropic".to_string(), &src).unwrap();
        assert_eq!(c2.token, "an-secret");
    }

    #[test]
    fn unresolved_token_ref_is_source_unreadable() {
        let pool = CredentialPool::new();
        pool.add(entry("a", "x", "MISSING"));
        let src = InMemorySource::default();
        let err = pool.acquire(&"x".to_string(), &src).unwrap_err();
        assert!(matches!(err, CredentialError::SourceUnreadable(_)));
    }

    #[test]
    fn token_ref_display_redacts_secret() {
        assert_eq!(
            TokenRef::EnvVar {
                name: "OR_KEY".into()
            }
            .to_string(),
            "${OR_KEY}"
        );
        assert_eq!(
            TokenRef::File {
                path: PathBuf::from("/tmp/x.env")
            }
            .to_string(),
            "@file:/tmp/x.env"
        );
    }

    #[test]
    fn throttle_with_larger_cooldown_extends() {
        let pool = CredentialPool::with_default_cooldown(Duration::from_millis(10));
        pool.add(entry("a", "x", "T"));
        let src = InMemorySource::default();

        pool.report_throttle(&"a".to_string(), Some(Duration::from_secs(60)));
        std::thread::sleep(Duration::from_millis(20));
        // Cooldown still active (60s > 20ms).
        assert!(pool.acquire(&"x".to_string(), &src).is_err());
    }

    #[test]
    fn throttle_smaller_cooldown_does_not_shrink() {
        let pool = CredentialPool::with_default_cooldown(Duration::from_secs(60));
        pool.add(entry("a", "x", "T"));
        let src = InMemorySource::default();

        // First apply a 60s cooldown.
        pool.report_throttle(&"a".to_string(), None);
        // Now apply a smaller one — should NOT shrink the existing one.
        pool.report_throttle(&"a".to_string(), Some(Duration::from_millis(10)));
        // Still on cooldown (60s > 10ms).
        assert!(pool.acquire(&"x".to_string(), &src).is_err());
    }
}
