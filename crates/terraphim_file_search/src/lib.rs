//! File-system search with knowledge-graph scoring for Terraphim AI.
//!
//! Combines ripgrep-style recursive file search ([`watcher`]) with
//! knowledge-graph concept matching ([`kg_scorer`]) to produce ranked
//! [`terraphim_types::Document`] results from local haystacks.
//! Configuration is managed via [`config`].
pub mod config;
pub mod kg_scorer;
pub mod watcher;
