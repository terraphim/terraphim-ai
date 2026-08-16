//! Memory backend abstraction for TinyClaw session storage.
//!
//! Wave 6 of the Hermes parity arc. Defines the [`MemoryBackend`] trait
//! and re-exports the two implementations:
//!
//! - [`JsonlBackend`]: per-session JSON-line files (default; reuses
//!   existing `SessionManager`).
//! - [`SqliteBackend`]: keyed JSON via `terraphim_persistence::DeviceStorage`
//!   (sqlite backend, opendal-mediated).
//!
//! The agent loop (`agent::agent_loop::ToolCallingLoop`) persists all
//! session state through this trait (#3227, T4) and writes compression
//! summaries back to the agent-memory bridge with a
//! `session-compression:<session_key>` provenance tag.
//!
//! ## Trait shape
//!
//! The trait is intentionally minimal — three methods that cover all of
//! TinyClaw's session storage needs. Adding more methods should require
//! an ADR documenting why.
//!
//! ## Backward compat
//!
//! The legacy `SessionManager` (in `src/session.rs`) is preserved. The
//! `JsonlBackend` impl uses `SessionManager` internally for jsonl
//! persistence, so the existing filesystem format is unchanged.
//!
//! ## Object safety
//!
//! The trait is `async_trait`-based, which makes it object-safe (usable as
//! `Arc<dyn MemoryBackend>`). The `Send + Sync` super-trait bound is
//! required for `Arc<dyn>` sharing across Tokio worker threads.

pub mod jsonl;
pub mod sqlite;
pub mod sqlite_key;

use async_trait::async_trait;

use crate::session::Session;

/// Memory storage errors.
#[derive(Debug, thiserror::Error)]
pub enum MemoryError {
    /// I/O error (file not found, permission denied, etc.)
    #[error("memory io error: {0}")]
    Io(#[from] std::io::Error),

    /// Serialization/deserialization error
    #[error("memory serde error: {0}")]
    Serde(#[from] serde_json::Error),

    /// Generic storage backend error
    #[error("memory storage error: {0}")]
    Storage(String),
}

impl From<anyhow::Error> for MemoryError {
    fn from(e: anyhow::Error) -> Self {
        MemoryError::Storage(e.to_string())
    }
}

/// Memory backend trait.
///
/// All TinyClaw session storage goes through this trait. Two impls are
/// provided in this module:
///
/// - [`JsonlBackend`]: per-session JSON-line file in a sessions directory.
///   Default for desktop / single-machine use.
/// - [`SqliteBackend`]: keyed JSON via `terraphim_persistence::DeviceStorage`
///   (sqlite backend, opendal-mediated). Better for multi-device or
///   networked deployments.
///
/// ## Thread safety
///
/// All methods take `&self` (not `&mut self`). Implementations must be
/// safe to share across threads via `Arc<dyn MemoryBackend + Send + Sync>`.
#[async_trait]
pub trait MemoryBackend: Send + Sync {
    /// Get an existing session by ID, or create a new empty one if missing.
    ///
    /// The returned `Session` is owned by the caller. To persist it, call
    /// [`persist`] afterward.
    ///
    /// [`persist`]: MemoryBackend::persist
    async fn get_or_create(&self, session_id: &str) -> Session;

    /// Persist a session to storage.
    ///
    /// Idempotent: persisting the same `Session` twice produces identical
    /// storage state. Implementations should use atomic-write or
    /// upsert semantics to avoid partial-write corruption on crash.
    async fn persist(&self, session: &Session) -> Result<(), MemoryError>;

    /// List all session IDs known to this backend.
    ///
    /// Order is implementation-defined. Returns an empty `Vec` if no
    /// sessions exist.
    async fn list(&self) -> Result<Vec<String>, MemoryError>;
}

/// Convenience type alias for the standard backend sharing pattern.
pub type SharedBackend = std::sync::Arc<dyn MemoryBackend>;
