//! Shared Learning Store for ADF agents.
//!
//! Provides a persistence-backed store for learnings extracted from agent runs.
//! Learnings are shared across agents to improve reliability and efficiency.
//!
//! Uses `terraphim_persistence` for storage, following the same pattern as
//! `metrics_persistence` — a trait with in-memory and device-storage implementations.
//!
//! # Architecture
//!
//! ```text
//! Flow State Parser (Python)
//!     ↓ JSONL learnings
//! SharedLearningStore
//!     ↓ LearningPersistence trait
//!     ├── InMemoryLearningPersistence (tests)
//!     └── DeviceStorageLearningPersistence (production, uses terraphim_persistence)
//!          ↓
//!     Orchestrator (pre-spawn context injection)
//!          ↓ /tmp/adf-context-{agent}.md
//!     Agent Task
//! ```
//!
//! # Trust Levels
//!
//! - **L0**: Unverified — just extracted, not yet shown to agents
//! - **L1**: Verified once — passed verify_pattern or one effective application
//! - **L2**: Verified multiple times — candidate for Gitea wiki sync
//! - **L3**: Golden — manually verified, always included in context

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use thiserror::Error;
use tracing::{debug, info, warn};
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Errors for the shared learning store.
#[derive(Debug, Error)]
pub enum LearningError {
    #[error("storage error: {0}")]
    Storage(String),

    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("invalid trust level: {0}")]
    InvalidTrustLevel(String),

    #[error("learning not found: {0}")]
    NotFound(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

// ---------------------------------------------------------------------------
// Domain types
// ---------------------------------------------------------------------------

/// Trust levels for learnings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum TrustLevel {
    /// Unverified, just extracted
    L0 = 0,
    /// Verified once
    L1 = 1,
    /// Verified multiple times, candidate for wiki sync
    L2 = 2,
    /// Golden, manually verified
    L3 = 3,
}

impl std::fmt::Display for TrustLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TrustLevel::L0 => write!(f, "L0"),
            TrustLevel::L1 => write!(f, "L1"),
            TrustLevel::L2 => write!(f, "L2"),
            TrustLevel::L3 => write!(f, "L3"),
        }
    }
}

impl std::str::FromStr for TrustLevel {
    type Err = LearningError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_uppercase().as_str() {
            "L0" => Ok(TrustLevel::L0),
            "L1" => Ok(TrustLevel::L1),
            "L2" => Ok(TrustLevel::L2),
            "L3" => Ok(TrustLevel::L3),
            _ => Err(LearningError::InvalidTrustLevel(s.to_string())),
        }
    }
}

/// Categories of learnings.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LearningCategory {
    ModelError,
    StepFailure,
    ToolHealth,
    Pattern,
    Tip,
    TimingAnomaly,
    RecurringPattern,
}

impl std::fmt::Display for LearningCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LearningCategory::ModelError => write!(f, "model_error"),
            LearningCategory::StepFailure => write!(f, "step_failure"),
            LearningCategory::ToolHealth => write!(f, "tool_health"),
            LearningCategory::Pattern => write!(f, "pattern"),
            LearningCategory::Tip => write!(f, "tip"),
            LearningCategory::TimingAnomaly => write!(f, "timing_anomaly"),
            LearningCategory::RecurringPattern => write!(f, "recurring_pattern"),
        }
    }
}

impl std::str::FromStr for LearningCategory {
    type Err = LearningError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "model_error" => Ok(LearningCategory::ModelError),
            "step_failure" => Ok(LearningCategory::StepFailure),
            "tool_health" => Ok(LearningCategory::ToolHealth),
            "pattern" => Ok(LearningCategory::Pattern),
            "tip" => Ok(LearningCategory::Tip),
            "timing_anomaly" => Ok(LearningCategory::TimingAnomaly),
            "recurring_pattern" => Ok(LearningCategory::RecurringPattern),
            other => Err(LearningError::InvalidTrustLevel(format!(
                "unknown category: {other}"
            ))),
        }
    }
}

/// A shared learning extracted from agent runs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Learning {
    pub id: String,
    pub source_agent: String,
    pub category: LearningCategory,
    pub summary: String,
    pub details: Option<String>,
    /// Agents this learning applies to. Empty means all agents.
    pub applicable_agents: Vec<String>,
    pub trust_level: TrustLevel,
    /// Shell command that must exit 0 for this learning to remain valid.
    pub verify_pattern: Option<String>,
    pub applied_count: u32,
    pub effective_count: u32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub archived_at: Option<DateTime<Utc>>,
}

/// Input for creating a new learning (no id/timestamps).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewLearning {
    pub source_agent: String,
    pub category: LearningCategory,
    pub summary: String,
    #[serde(default)]
    pub details: Option<String>,
    #[serde(default)]
    pub applicable_agents: Vec<String>,
    #[serde(default)]
    pub verify_pattern: Option<String>,
}

// ---------------------------------------------------------------------------
// Persistence trait
// ---------------------------------------------------------------------------

/// Trait for learning persistence, mirroring `MetricsPersistence` pattern.
#[async_trait]
pub trait LearningPersistence: Send + Sync {
    /// Insert a new learning. Returns the generated UUID.
    async fn insert(&self, learning: NewLearning) -> Result<String, LearningError>;

    /// Get a learning by id.
    async fn get(&self, id: &str) -> Result<Option<Learning>, LearningError>;

    /// Query learnings relevant to an agent.
    ///
    /// Returns non-archived learnings with trust_level >= `min_trust`
    /// that either apply to all agents or specifically to `agent_name`.
    async fn query_relevant(
        &self,
        agent_name: &str,
        min_trust: TrustLevel,
    ) -> Result<Vec<Learning>, LearningError>;

    /// Increment applied_count.
    async fn record_applied(&self, id: &str) -> Result<(), LearningError>;

    /// Increment effective_count and auto-promote trust level.
    async fn record_effective(&self, id: &str) -> Result<(), LearningError>;

    /// Archive stale learnings older than `max_age_days` that are still L0.
    async fn archive_stale(&self, max_age_days: u32) -> Result<usize, LearningError>;

    /// List all non-archived learning ids.
    async fn list_ids(&self) -> Result<Vec<String>, LearningError>;

    /// Delete a learning.
    async fn delete(&self, id: &str) -> Result<(), LearningError>;
}

// ---------------------------------------------------------------------------
// In-memory implementation (for tests)
// ---------------------------------------------------------------------------

/// In-memory learning persistence for testing and development.
pub struct InMemoryLearningPersistence {
    data: std::sync::RwLock<HashMap<String, Learning>>,
}

impl InMemoryLearningPersistence {
    pub fn new() -> Self {
        Self {
            data: std::sync::RwLock::new(HashMap::new()),
        }
    }
}

impl Default for InMemoryLearningPersistence {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl LearningPersistence for InMemoryLearningPersistence {
    async fn insert(&self, input: NewLearning) -> Result<String, LearningError> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now();
        let learning = Learning {
            id: id.clone(),
            source_agent: input.source_agent,
            category: input.category,
            summary: input.summary,
            details: input.details,
            applicable_agents: input.applicable_agents,
            trust_level: TrustLevel::L0,
            verify_pattern: input.verify_pattern,
            applied_count: 0,
            effective_count: 0,
            created_at: now,
            updated_at: now,
            archived_at: None,
        };
        let mut data = self
            .data
            .write()
            .map_err(|e| LearningError::Storage(format!("lock poisoned: {e}")))?;
        data.insert(id.clone(), learning);
        Ok(id)
    }

    async fn get(&self, id: &str) -> Result<Option<Learning>, LearningError> {
        let data = self
            .data
            .read()
            .map_err(|e| LearningError::Storage(format!("lock poisoned: {e}")))?;
        Ok(data.get(id).cloned())
    }

    async fn query_relevant(
        &self,
        agent_name: &str,
        min_trust: TrustLevel,
    ) -> Result<Vec<Learning>, LearningError> {
        let data = self
            .data
            .read()
            .map_err(|e| LearningError::Storage(format!("lock poisoned: {e}")))?;

        let mut results: Vec<Learning> = data
            .values()
            .filter(|l| {
                l.archived_at.is_none()
                    && l.trust_level >= min_trust
                    && (l.applicable_agents.is_empty()
                        || l.applicable_agents.iter().any(|a| a == agent_name))
            })
            .cloned()
            .collect();

        // Sort: highest trust first, then by effective_count desc
        results.sort_by(|a, b| {
            b.trust_level
                .cmp(&a.trust_level)
                .then(b.effective_count.cmp(&a.effective_count))
        });

        results.truncate(20);
        Ok(results)
    }

    async fn record_applied(&self, id: &str) -> Result<(), LearningError> {
        let mut data = self
            .data
            .write()
            .map_err(|e| LearningError::Storage(format!("lock poisoned: {e}")))?;
        if let Some(l) = data.get_mut(id) {
            l.applied_count += 1;
            l.updated_at = Utc::now();
            Ok(())
        } else {
            Err(LearningError::NotFound(id.to_string()))
        }
    }

    async fn record_effective(&self, id: &str) -> Result<(), LearningError> {
        let mut data = self
            .data
            .write()
            .map_err(|e| LearningError::Storage(format!("lock poisoned: {e}")))?;
        if let Some(l) = data.get_mut(id) {
            l.effective_count += 1;
            l.updated_at = Utc::now();
            // Auto-promote
            l.trust_level = match (l.trust_level, l.effective_count) {
                (TrustLevel::L0, n) if n >= 1 => TrustLevel::L1,
                (TrustLevel::L1, n) if n >= 3 => TrustLevel::L2,
                (TrustLevel::L2, n) if n >= 5 => TrustLevel::L3,
                (current, _) => current,
            };
            Ok(())
        } else {
            Err(LearningError::NotFound(id.to_string()))
        }
    }

    async fn archive_stale(&self, max_age_days: u32) -> Result<usize, LearningError> {
        let cutoff = Utc::now() - chrono::Duration::days(max_age_days as i64);
        let now = Utc::now();
        let mut data = self
            .data
            .write()
            .map_err(|e| LearningError::Storage(format!("lock poisoned: {e}")))?;

        let mut count = 0;
        for l in data.values_mut() {
            if l.archived_at.is_none() && l.trust_level == TrustLevel::L0 && l.updated_at < cutoff {
                l.archived_at = Some(now);
                count += 1;
            }
        }
        Ok(count)
    }

    async fn list_ids(&self) -> Result<Vec<String>, LearningError> {
        let data = self
            .data
            .read()
            .map_err(|e| LearningError::Storage(format!("lock poisoned: {e}")))?;
        Ok(data
            .values()
            .filter(|l| l.archived_at.is_none())
            .map(|l| l.id.clone())
            .collect())
    }

    async fn delete(&self, id: &str) -> Result<(), LearningError> {
        let mut data = self
            .data
            .write()
            .map_err(|e| LearningError::Storage(format!("lock poisoned: {e}")))?;
        data.remove(id);
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// DeviceStorage-backed implementation (production)
// ---------------------------------------------------------------------------

/// Production learning persistence using `terraphim_persistence::DeviceStorage`.
///
/// Each learning is stored as a JSON document keyed by `adf/learnings/{uuid}`.
/// An index document at `adf/learnings/_index` maps ids for listing/querying.
pub struct DeviceStorageLearningPersistence {
    key_prefix: String,
    /// In-memory cache loaded from persistence on startup.
    cache: tokio::sync::RwLock<HashMap<String, Learning>>,
}

impl DeviceStorageLearningPersistence {
    /// Create and initialise, loading existing learnings from storage.
    pub async fn new(key_prefix: impl Into<String>) -> Result<Self, LearningError> {
        let key_prefix = key_prefix.into();
        let store = Self {
            key_prefix,
            cache: tokio::sync::RwLock::new(HashMap::new()),
        };
        store.load_all_from_storage().await?;
        Ok(store)
    }

    fn learning_key(&self, id: &str) -> String {
        format!("{}/{}", self.key_prefix, id)
    }

    fn index_key(&self) -> String {
        format!("{}/_index", self.key_prefix)
    }

    /// Persist a single learning to the fastest operator.
    async fn persist_learning(&self, learning: &Learning) -> Result<(), LearningError> {
        let storage = terraphim_persistence::DeviceStorage::instance()
            .await
            .map_err(|e| LearningError::Storage(format!("DeviceStorage init: {e}")))?;

        let key = self.learning_key(&learning.id);
        let json = serde_json::to_string(learning)?;
        storage
            .fastest_op
            .write(&key, json)
            .await
            .map_err(|e| LearningError::Storage(format!("write {key}: {e}")))?;
        Ok(())
    }

    /// Persist the index (list of ids) so we can enumerate on restart.
    async fn persist_index(&self) -> Result<(), LearningError> {
        let cache = self.cache.read().await;
        let ids: Vec<&str> = cache.keys().map(String::as_str).collect();
        let json = serde_json::to_string(&ids)?;

        let storage = terraphim_persistence::DeviceStorage::instance()
            .await
            .map_err(|e| LearningError::Storage(format!("DeviceStorage init: {e}")))?;

        storage
            .fastest_op
            .write(&self.index_key(), json)
            .await
            .map_err(|e| LearningError::Storage(format!("write index: {e}")))?;
        Ok(())
    }

    /// Load all learnings from storage into the in-memory cache.
    async fn load_all_from_storage(&self) -> Result<(), LearningError> {
        let storage = terraphim_persistence::DeviceStorage::instance()
            .await
            .map_err(|e| LearningError::Storage(format!("DeviceStorage init: {e}")))?;

        // Read index
        let index_key = self.index_key();
        let ids: Vec<String> = match storage.fastest_op.read(&index_key).await {
            Ok(bs) => serde_json::from_slice(&bs.to_vec()).unwrap_or_default(),
            Err(_) => {
                debug!("No learning index found at {index_key}, starting fresh");
                Vec::new()
            }
        };

        let mut cache = self.cache.write().await;
        for id in &ids {
            let key = self.learning_key(id);
            match storage.fastest_op.read(&key).await {
                Ok(bs) => match serde_json::from_slice::<Learning>(&bs.to_vec()) {
                    Ok(learning) => {
                        cache.insert(id.clone(), learning);
                    }
                    Err(e) => warn!("Failed to deserialize learning {id}: {e}"),
                },
                Err(e) => warn!("Failed to read learning {id}: {e}"),
            }
        }

        info!(
            "Loaded {} learnings from persistence (prefix={})",
            cache.len(),
            self.key_prefix
        );
        Ok(())
    }
}

#[async_trait]
impl LearningPersistence for DeviceStorageLearningPersistence {
    async fn insert(&self, input: NewLearning) -> Result<String, LearningError> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now();
        let learning = Learning {
            id: id.clone(),
            source_agent: input.source_agent,
            category: input.category,
            summary: input.summary,
            details: input.details,
            applicable_agents: input.applicable_agents,
            trust_level: TrustLevel::L0,
            verify_pattern: input.verify_pattern,
            applied_count: 0,
            effective_count: 0,
            created_at: now,
            updated_at: now,
            archived_at: None,
        };

        self.persist_learning(&learning).await?;

        let mut cache = self.cache.write().await;
        cache.insert(id.clone(), learning);
        drop(cache);

        self.persist_index().await?;
        info!("Inserted learning {id}");
        Ok(id)
    }

    async fn get(&self, id: &str) -> Result<Option<Learning>, LearningError> {
        let cache = self.cache.read().await;
        Ok(cache.get(id).cloned())
    }

    async fn query_relevant(
        &self,
        agent_name: &str,
        min_trust: TrustLevel,
    ) -> Result<Vec<Learning>, LearningError> {
        let cache = self.cache.read().await;

        let mut results: Vec<Learning> = cache
            .values()
            .filter(|l| {
                l.archived_at.is_none()
                    && l.trust_level >= min_trust
                    && (l.applicable_agents.is_empty()
                        || l.applicable_agents.iter().any(|a| a == agent_name))
            })
            .cloned()
            .collect();

        results.sort_by(|a, b| {
            b.trust_level
                .cmp(&a.trust_level)
                .then(b.effective_count.cmp(&a.effective_count))
        });
        results.truncate(20);
        Ok(results)
    }

    async fn record_applied(&self, id: &str) -> Result<(), LearningError> {
        let mut cache = self.cache.write().await;
        let learning = cache
            .get_mut(id)
            .ok_or_else(|| LearningError::NotFound(id.to_string()))?;
        learning.applied_count += 1;
        learning.updated_at = Utc::now();
        let snapshot = learning.clone();
        drop(cache);

        self.persist_learning(&snapshot).await
    }

    async fn record_effective(&self, id: &str) -> Result<(), LearningError> {
        let mut cache = self.cache.write().await;
        let learning = cache
            .get_mut(id)
            .ok_or_else(|| LearningError::NotFound(id.to_string()))?;
        learning.effective_count += 1;
        learning.updated_at = Utc::now();
        // Auto-promote
        learning.trust_level = match (learning.trust_level, learning.effective_count) {
            (TrustLevel::L0, n) if n >= 1 => TrustLevel::L1,
            (TrustLevel::L1, n) if n >= 3 => TrustLevel::L2,
            (TrustLevel::L2, n) if n >= 5 => TrustLevel::L3,
            (current, _) => current,
        };
        let snapshot = learning.clone();
        drop(cache);

        self.persist_learning(&snapshot).await
    }

    async fn archive_stale(&self, max_age_days: u32) -> Result<usize, LearningError> {
        let cutoff = Utc::now() - chrono::Duration::days(max_age_days as i64);
        let now = Utc::now();
        let mut cache = self.cache.write().await;

        let mut to_persist = Vec::new();
        let mut count = 0;
        for l in cache.values_mut() {
            if l.archived_at.is_none() && l.trust_level == TrustLevel::L0 && l.updated_at < cutoff {
                l.archived_at = Some(now);
                to_persist.push(l.clone());
                count += 1;
            }
        }
        drop(cache);

        for l in &to_persist {
            self.persist_learning(l).await?;
        }
        if count > 0 {
            info!("Archived {count} stale L0 learnings");
        }
        Ok(count)
    }

    async fn list_ids(&self) -> Result<Vec<String>, LearningError> {
        let cache = self.cache.read().await;
        Ok(cache
            .values()
            .filter(|l| l.archived_at.is_none())
            .map(|l| l.id.clone())
            .collect())
    }

    async fn delete(&self, id: &str) -> Result<(), LearningError> {
        let mut cache = self.cache.write().await;
        cache.remove(id);
        drop(cache);

        // Delete from storage
        let storage = terraphim_persistence::DeviceStorage::instance()
            .await
            .map_err(|e| LearningError::Storage(format!("DeviceStorage init: {e}")))?;
        let key = self.learning_key(id);
        let _ = storage.fastest_op.delete(&key).await; // best-effort

        self.persist_index().await
    }
}

// ---------------------------------------------------------------------------
// SharedLearningStore — high-level API wrapping the trait
// ---------------------------------------------------------------------------

/// High-level API for the shared learning store.
///
/// Wraps a `LearningPersistence` impl and adds context-file generation
/// and JSONL import capabilities.
pub struct SharedLearningStore {
    persistence: Box<dyn LearningPersistence>,
    min_trust: TrustLevel,
}

impl SharedLearningStore {
    /// Create a store backed by the given persistence implementation.
    pub fn new(persistence: Box<dyn LearningPersistence>, min_trust: TrustLevel) -> Self {
        Self {
            persistence,
            min_trust,
        }
    }

    /// Create an in-memory store (for testing).
    pub fn in_memory() -> Self {
        Self {
            persistence: Box::new(InMemoryLearningPersistence::new()),
            min_trust: TrustLevel::L1,
        }
    }

    /// Delegate to the persistence layer.
    pub async fn insert(&self, learning: NewLearning) -> Result<String, LearningError> {
        self.persistence.insert(learning).await
    }

    pub async fn get(&self, id: &str) -> Result<Option<Learning>, LearningError> {
        self.persistence.get(id).await
    }

    pub async fn query_relevant(&self, agent_name: &str) -> Result<Vec<Learning>, LearningError> {
        self.persistence
            .query_relevant(agent_name, self.min_trust)
            .await
    }

    pub async fn record_applied(&self, id: &str) -> Result<(), LearningError> {
        self.persistence.record_applied(id).await
    }

    pub async fn record_effective(&self, id: &str) -> Result<(), LearningError> {
        self.persistence.record_effective(id).await
    }

    pub async fn archive_stale(&self, max_age_days: u32) -> Result<usize, LearningError> {
        self.persistence.archive_stale(max_age_days).await
    }

    /// Generate Markdown context for an agent to consume.
    pub async fn generate_context(&self, agent_name: &str) -> Result<String, LearningError> {
        let learnings = self.query_relevant(agent_name).await?;

        if learnings.is_empty() {
            return Ok("# Shared Learnings\n\nNo relevant learnings found.\n".to_string());
        }

        let mut md = String::new();
        md.push_str("# Shared Learnings (auto-generated, do not edit)\n\n");
        md.push_str(&format!("Generated for: {agent_name}\n\n"));

        // Partition by category type
        let mut errors = Vec::new();
        let mut tips = Vec::new();
        let mut patterns = Vec::new();

        for l in &learnings {
            match l.category {
                LearningCategory::ModelError
                | LearningCategory::StepFailure
                | LearningCategory::ToolHealth => errors.push(l),
                LearningCategory::Tip => tips.push(l),
                _ => patterns.push(l),
            }
        }

        if !errors.is_empty() {
            md.push_str("## ⚠️ Known Issues\n\n");
            for l in &errors {
                md.push_str(&format!(
                    "- [{}] {} (trust: {}, verified {}x)\n",
                    l.category, l.summary, l.trust_level, l.effective_count
                ));
                if let Some(ref details) = l.details {
                    if let Some(first_line) = details.lines().next() {
                        md.push_str(&format!("  > {first_line}\n"));
                    }
                }
            }
            md.push('\n');
        }

        if !tips.is_empty() {
            md.push_str("## 💡 Tips from Peer Agents\n\n");
            for l in &tips {
                md.push_str(&format!(
                    "- {} (from {}, trust: {})\n",
                    l.summary, l.source_agent, l.trust_level
                ));
            }
            md.push('\n');
        }

        if !patterns.is_empty() {
            md.push_str("## 🔄 Recurring Patterns\n\n");
            for l in &patterns {
                md.push_str(&format!("- {} (seen {}x)\n", l.summary, l.applied_count));
            }
            md.push('\n');
        }

        Ok(md)
    }

    /// Write context file for an agent to `/tmp/adf-context-{agent}.md`.
    pub async fn write_context_file(&self, agent_name: &str) -> Result<PathBuf, LearningError> {
        let content = self.generate_context(agent_name).await?;
        let path = PathBuf::from(format!("/tmp/adf-context-{agent_name}.md"));
        tokio::fs::write(&path, content)
            .await
            .map_err(|e| LearningError::Io(e))?;
        info!("Wrote context file for {agent_name} at {path:?}");
        Ok(path)
    }

    /// Import learnings from JSONL (as produced by `flow-state-parser.py`).
    pub async fn import_jsonl(&self, jsonl: &str) -> Result<usize, LearningError> {
        let mut count = 0;
        for line in jsonl.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            match serde_json::from_str::<NewLearning>(line) {
                Ok(input) => match self.insert(input).await {
                    Ok(_) => count += 1,
                    Err(e) => warn!("Failed to insert learning: {e}"),
                },
                Err(e) => {
                    warn!("Failed to parse JSONL line: {e}");
                }
            }
        }
        info!("Imported {count} learnings from JSONL");
        Ok(count)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_insert_and_get() {
        let store = SharedLearningStore::in_memory();

        let id = store
            .insert(NewLearning {
                source_agent: "test-agent".into(),
                category: LearningCategory::ModelError,
                summary: "k2p5 model fails".into(),
                details: Some("Use kimi-for-coding/k2p5".into()),
                applicable_agents: vec![],
                verify_pattern: None,
            })
            .await
            .unwrap();

        let l = store.get(&id).await.unwrap().unwrap();
        assert_eq!(l.summary, "k2p5 model fails");
        assert_eq!(l.trust_level, TrustLevel::L0);
    }

    #[tokio::test]
    async fn test_query_respects_trust_level() {
        let store = SharedLearningStore::in_memory();

        // Insert at L0 — should NOT appear with default min_trust=L1
        store
            .insert(NewLearning {
                source_agent: "a".into(),
                category: LearningCategory::Tip,
                summary: "invisible".into(),
                details: None,
                applicable_agents: vec![],
                verify_pattern: None,
            })
            .await
            .unwrap();

        let results = store.query_relevant("any").await.unwrap();
        assert!(results.is_empty(), "L0 learnings should be hidden");
    }

    #[tokio::test]
    async fn test_auto_promotion() {
        let store = SharedLearningStore::in_memory();

        let id = store
            .insert(NewLearning {
                source_agent: "a".into(),
                category: LearningCategory::Tip,
                summary: "helpful tip".into(),
                details: None,
                applicable_agents: vec![],
                verify_pattern: None,
            })
            .await
            .unwrap();

        // L0 → record_effective → L1
        store.record_effective(&id).await.unwrap();
        let l = store.get(&id).await.unwrap().unwrap();
        assert_eq!(l.trust_level, TrustLevel::L1);

        // Now visible in queries
        let results = store.query_relevant("any").await.unwrap();
        assert_eq!(results.len(), 1);
    }

    #[tokio::test]
    async fn test_applicable_agents_filtering() {
        let store = SharedLearningStore::in_memory();

        let id = store
            .insert(NewLearning {
                source_agent: "sec".into(),
                category: LearningCategory::StepFailure,
                summary: "specific to sec-sentinel".into(),
                details: None,
                applicable_agents: vec!["sec-sentinel".into()],
                verify_pattern: None,
            })
            .await
            .unwrap();

        // Promote to L1
        store.record_effective(&id).await.unwrap();

        let for_sentinel = store.query_relevant("sec-sentinel").await.unwrap();
        assert_eq!(for_sentinel.len(), 1);

        let for_other = store.query_relevant("other-agent").await.unwrap();
        assert!(for_other.is_empty());
    }

    #[tokio::test]
    async fn test_context_generation() {
        let store = SharedLearningStore::in_memory();

        let id = store
            .insert(NewLearning {
                source_agent: "security-sentinel".into(),
                category: LearningCategory::ModelError,
                summary: "k2p5/. model ID fails".into(),
                details: Some("Use kimi-for-coding/k2p5 instead".into()),
                applicable_agents: vec![],
                verify_pattern: None,
            })
            .await
            .unwrap();

        store.record_effective(&id).await.unwrap();

        let ctx = store.generate_context("test-agent").await.unwrap();
        assert!(ctx.contains("Known Issues"));
        assert!(ctx.contains("k2p5"));
    }

    #[tokio::test]
    async fn test_archive_stale() {
        let persistence = InMemoryLearningPersistence::new();

        // Insert an old learning manually
        {
            let mut data = persistence.data.write().unwrap();
            data.insert(
                "old-id".into(),
                Learning {
                    id: "old-id".into(),
                    source_agent: "a".into(),
                    category: LearningCategory::Tip,
                    summary: "old".into(),
                    details: None,
                    applicable_agents: vec![],
                    trust_level: TrustLevel::L0,
                    verify_pattern: None,
                    applied_count: 0,
                    effective_count: 0,
                    created_at: Utc::now() - chrono::Duration::days(60),
                    updated_at: Utc::now() - chrono::Duration::days(60),
                    archived_at: None,
                },
            );
        }

        let store = SharedLearningStore::new(Box::new(persistence), TrustLevel::L1);
        let archived = store.archive_stale(30).await.unwrap();
        assert_eq!(archived, 1);

        let l = store.get("old-id").await.unwrap().unwrap();
        assert!(l.archived_at.is_some());
    }

    #[tokio::test]
    async fn test_import_jsonl() {
        let store = SharedLearningStore::in_memory();

        let jsonl = r#"{"source_agent":"test","category":"tip","summary":"use cargo check first","applicable_agents":[]}
{"source_agent":"test","category":"model_error","summary":"k2p5 fails","applicable_agents":[]}
"#;
        let count = store.import_jsonl(jsonl).await.unwrap();
        assert_eq!(count, 2);
    }
}
