//! Shared Learning System for Terraphim AI
//!
//! This module provides a shared learning store that aggregates learnings
//! across agents, deduplicates using BM25, and promotes learnings based on
//! trust levels (L1/L2/L3) with Gitea wiki synchronization.
//!
//! # Architecture
//!
//! ```text
//! Learning Capture → BM25 Deduplication → Trust Promotion → Gitea Wiki Sync
//!        ↓                    ↓                  ↓                  ↓
//!  Multiple Sources    Merge Duplicates   L1→L2→L3          Wiki Pages
//! ```
//!
//! # Trust Levels
//!
//! - **L1 (Unverified)**: Auto-captured from various sources, advisory only
//! - **L2 (Peer-Validated)**: Applied 3+ times across 2+ agents with positive outcome
//! - **L3 (Human-Approved)**: CTO review via `/evolve` or Gitea issue approval

mod markdown_store;
mod store;
mod types;
mod wiki_sync;

pub use markdown_store::{MarkdownLearningStore, MarkdownStoreConfig, MarkdownStoreError};
pub use store::{SharedLearningStore, StoreConfig};
pub use types::{LearningSource as SharedLearningSource, SharedLearning, TrustLevel};
pub use wiki_sync::{GiteaWikiClient, WikiSyncError};
