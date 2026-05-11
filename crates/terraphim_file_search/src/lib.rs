//! Local filesystem search with knowledge-graph scoring.
//!
//! Provides ripgrep-backed file search (`config`), a KG-aware relevance
//! scorer (`kg_scorer`), and a filesystem watcher (`watcher`) that triggers
//! re-indexing when monitored directories change.
pub mod config;
pub mod kg_scorer;
pub mod watcher;
