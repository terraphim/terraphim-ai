//! SQLite key derivation strategy for the memory backend.
//!
//! Keys are derived from the per-instance `key_prefix` (passed at
//! [`SqliteBackend::new`] construction time). Format:
//!
//! - **Session document**: `<key_prefix>_session:<session_key>`
//! - **Index document**:  `<key_prefix>_memory_index`
//!
//! ## Why per-prefix
//!
//! `terraphim_persistence::DeviceStorage` is a process-wide singleton
//! (initialized once via `init_memory_only()` or `init_disk()`). Without
//! a prefix, two `SqliteBackend` instances in the same process would
//! collide on the same keys.
//!
//! ## Why two documents (session + index)
//!
//! Sessions are stored as full JSON documents (so they're cheap to read
//! whole on `get_or_create`). The index is a separate `Vec<String>` of
//! session keys, used by `list()`. Same pattern as `cron::store`.
//!
//! ## Alternative considered: per-session in a single document
//!
//! We could store all sessions in one big JSON document with a session
//! map. Pros: atomic across all sessions. Cons: re-writes the whole file
//! on every persist, scales O(N) with total sessions. Rejected for
//! tinyclaw's expected scale (10s of sessions, not 1000s).

/// Derive the storage key for a session document.
pub fn derive_session_key(prefix: &str, session_key: &str) -> String {
    format!("{prefix}_session:{session_key}")
}

/// Derive the storage key for the index document.
pub fn derive_index_key(prefix: &str) -> String {
    format!("{prefix}_memory_index")
}
