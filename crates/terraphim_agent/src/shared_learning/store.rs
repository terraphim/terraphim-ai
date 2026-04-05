//! Shared learning store implementation
//!
//! Provides SQLite-backed storage with BM25-based deduplication
//! and trust-gated promotion logic.

use std::collections::HashMap;
use std::path::PathBuf;

use chrono::Utc;
use sqlx::{sqlite::SqlitePoolOptions, Pool, Sqlite};
use thiserror::Error;
use tracing::{debug, info};

use crate::shared_learning::types::{
    LearningSource, QualityMetrics, SharedLearning, TrustLevel,
};

/// Errors that can occur in the shared learning store
#[derive(Error, Debug)]
pub enum StoreError {
    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),
    #[error("persistence error: {0}")]
    Persistence(String),
    #[error("learning not found: {0}")]
    NotFound(String),
    #[error("BM25 calculation error: {0}")]
    Bm25(String),
    #[error("invalid input: {0}")]
    InvalidInput(String),
    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

impl From<terraphim_persistence::Error> for StoreError {
    fn from(e: terraphim_persistence::Error) -> Self {
        StoreError::Persistence(e.to_string())
    }
}

/// Configuration for the shared learning store
#[derive(Debug, Clone)]
pub struct StoreConfig {
    /// Database file path (use ":memory:" for in-memory database)
    pub db_path: PathBuf,
    /// Whether to use in-memory database
    pub in_memory: bool,
    /// Similarity threshold for deduplication (0.0-1.0, default 0.8)
    pub similarity_threshold: f64,
    /// Auto-promote L1 to L2 when criteria met
    pub auto_promote_l2: bool,
}

impl Default for StoreConfig {
    fn default() -> Self {
        let db_dir = dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("~/.local/share"))
            .join("terraphim")
            .join("shared_learnings");

        Self {
            db_path: db_dir.join("store.db"),
            in_memory: false,
            similarity_threshold: 0.8,
            auto_promote_l2: true,
        }
    }
}

impl StoreConfig {
    /// Create config with custom database path
    pub fn with_db_path(db_path: PathBuf) -> Self {
        Self {
            db_path,
            in_memory: false,
            ..Default::default()
        }
    }

    /// Create config for in-memory database (useful for testing)
    pub fn in_memory() -> Self {
        Self {
            db_path: PathBuf::from(":memory:"),
            in_memory: true,
            similarity_threshold: 0.8,
            auto_promote_l2: true,
        }
    }

    /// Set similarity threshold
    pub fn with_similarity_threshold(mut self, threshold: f64) -> Self {
        self.similarity_threshold = threshold.clamp(0.0, 1.0);
        self
    }
}

/// BM25 scoring for text similarity
pub struct Bm25Scorer {
    /// Average document length (for BM25 k1 parameter)
    avg_doc_len: f64,
    /// Total number of documents
    total_docs: usize,
    /// Inverse document frequencies for terms
    idf_cache: HashMap<String, f64>,
}

impl Bm25Scorer {
    /// Create new BM25 scorer
    pub fn new(total_docs: usize, avg_doc_len: f64) -> Self {
        Self {
            avg_doc_len,
            total_docs,
            idf_cache: HashMap::new(),
        }
    }

    /// Calculate IDF for a term
    fn calculate_idf(&mut self, term: &str, doc_freq: usize) -> f64 {
        if let Some(&idf) = self.idf_cache.get(term) {
            return idf;
        }

        // IDF formula: log((N - n + 0.5) / (n + 0.5))
        let n = doc_freq as f64;
        let n_docs = self.total_docs as f64;
        
        // Handle edge case: when all documents contain the term (or only 1 doc),
        // use a small constant IDF to allow matching based on term frequency
        let idf = if n_docs <= 1.0 || n >= n_docs {
            0.5 // Small constant for single-doc or universal terms
        } else {
            ((n_docs - n + 0.5) / (n + 0.5)).ln().max(0.0)
        };

        self.idf_cache.insert(term.to_string(), idf);
        idf
    }

    /// Calculate BM25 score between query and document
    ///
    /// Uses standard BM25 formula:
    /// score = sum(idf * (f * (k1 + 1)) / (f + k1 * (1 - b + b * |D|/avgdl)))
    pub fn score(&mut self, query: &str, doc: &str, doc_freqs: &HashMap<String, usize>) -> f64 {
        const K1: f64 = 1.2;
        const B: f64 = 0.75;

        let query_terms: Vec<String> = query
            .to_lowercase()
            .split_whitespace()
            .map(|s| s.to_string())
            .collect();

        let doc_terms: Vec<String> = doc
            .to_lowercase()
            .split_whitespace()
            .map(|s| s.to_string())
            .collect();

        let doc_len = doc_terms.len() as f64;
        let mut score = 0.0;

        // Build term frequency map for document
        let mut doc_tf: HashMap<String, usize> = HashMap::new();
        for term in &doc_terms {
            *doc_tf.entry(term.clone()).or_insert(0) += 1;
        }

        for term in &query_terms {
            let f = *doc_tf.get(term).unwrap_or(&0) as f64;
            let doc_freq = *doc_freqs.get(term).unwrap_or(&1);
            let idf = self.calculate_idf(term, doc_freq);

            // BM25 formula
            let numerator = f * (K1 + 1.0);
            let denominator = f + K1 * (1.0 - B + B * doc_len / self.avg_doc_len);

            score += idf * numerator / denominator;
        }

        score
    }

    /// Normalize score to 0.0-1.0 range
    pub fn normalize_score(&self, score: f64, query_len: usize) -> f64 {
        if query_len == 0 {
            return 0.0;
        }
        // Normalize by query length to get per-term score
        let normalized = (score / query_len as f64).tanh();
        normalized.clamp(0.0, 1.0)
    }
}

/// Shared learning store with SQLite backend
pub struct SharedLearningStore {
    /// Database connection pool
    pool: Pool<Sqlite>,
    /// Store configuration
    config: StoreConfig,
}

/// Database row representation of a shared learning
#[derive(sqlx::FromRow)]
struct SharedLearningRow {
    id: String,
    title: String,
    content: String,
    trust_level: String,
    applied_count: i64,
    effective_count: i64,
    agent_count: i64,
    agent_names: String, // JSON array
    last_applied_at: Option<chrono::DateTime<Utc>>,
    success_rate: Option<f64>,
    source: String,
    source_agent: String,
    applicable_agents: String, // JSON array
    keywords: String,          // JSON array
    verify_pattern: Option<String>,
    created_at: chrono::DateTime<Utc>,
    updated_at: chrono::DateTime<Utc>,
    promoted_at: Option<chrono::DateTime<Utc>>,
    wiki_page_name: Option<String>,
    original_command: Option<String>,
    error_context: Option<String>,
    correction: Option<String>,
}

impl From<SharedLearningRow> for SharedLearning {
    fn from(row: SharedLearningRow) -> Self {
        let trust_level = row.trust_level.parse().unwrap_or(TrustLevel::L1);
        let source = match row.source.as_str() {
            "bash-hook" => LearningSource::BashHook,
            "auto-extract" => LearningSource::AutoExtract,
            "tool-health" => LearningSource::ToolHealth,
            "gitea-comment" => LearningSource::GiteaComment,
            "cje-verdict" => LearningSource::CjeVerdict,
            _ => LearningSource::Manual,
        };

        SharedLearning {
            id: row.id,
            title: row.title,
            content: row.content,
            trust_level,
            quality: QualityMetrics {
                applied_count: row.applied_count as u32,
                effective_count: row.effective_count as u32,
                agent_count: row.agent_count as u32,
                agent_names: serde_json::from_str(&row.agent_names).unwrap_or_default(),
                last_applied_at: row.last_applied_at,
                success_rate: row.success_rate,
            },
            source,
            source_agent: row.source_agent,
            applicable_agents: serde_json::from_str(&row.applicable_agents).unwrap_or_default(),
            keywords: serde_json::from_str(&row.keywords).unwrap_or_default(),
            verify_pattern: row.verify_pattern,
            created_at: row.created_at,
            updated_at: row.updated_at,
            promoted_at: row.promoted_at,
            wiki_page_name: row.wiki_page_name,
            original_command: row.original_command,
            error_context: row.error_context,
            correction: row.correction,
        }
    }
}

impl SharedLearningStore {
    /// Create or open a shared learning store
    pub async fn open(config: StoreConfig) -> Result<Self, StoreError> {
        let pool = if config.in_memory {
            // For in-memory database, use shared cache so all connections share the same database
            // We use a unique name per store instance to avoid conflicts between tests
            let db_name = format!("mem_{}", uuid::Uuid::new_v4().simple());
            let connection_string = format!("file:{}?mode=memory&cache=shared", db_name);
            
            SqlitePoolOptions::new()
                .max_connections(1) // Use single connection for in-memory to ensure consistency
                .connect(&connection_string)
                .await?
        } else {
            // Ensure parent directory exists for file-based database
            if let Some(parent) = config.db_path.parent() {
                tokio::fs::create_dir_all(parent).await.map_err(|e| {
                    StoreError::InvalidInput(format!("Cannot create directory: {}", e))
                })?;
            }

            SqlitePoolOptions::new()
                .max_connections(5)
                .connect(&format!("sqlite:{}", config.db_path.display()))
                .await?
        };

        let store = Self { pool, config: config.clone() };
        
        // Initialize database schema
        store.init_schema().await?;

        info!("SharedLearningStore opened: {}", 
            if config.in_memory { ":memory:".to_string() } else { config.db_path.display().to_string() });
        Ok(store)
    }

    /// Initialize database schema
    async fn init_schema(&self) -> Result<(), StoreError> {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS shared_learnings (
                id TEXT PRIMARY KEY,
                title TEXT NOT NULL,
                content TEXT NOT NULL,
                trust_level TEXT NOT NULL DEFAULT 'L1',
                applied_count INTEGER NOT NULL DEFAULT 0,
                effective_count INTEGER NOT NULL DEFAULT 0,
                agent_count INTEGER NOT NULL DEFAULT 0,
                agent_names TEXT NOT NULL DEFAULT '[]',
                last_applied_at TIMESTAMP,
                success_rate REAL,
                source TEXT NOT NULL,
                source_agent TEXT NOT NULL,
                applicable_agents TEXT NOT NULL DEFAULT '[]',
                keywords TEXT NOT NULL DEFAULT '[]',
                verify_pattern TEXT,
                created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
                updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
                promoted_at TIMESTAMP,
                wiki_page_name TEXT,
                original_command TEXT,
                error_context TEXT,
                correction TEXT
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        // Create indexes for efficient querying
        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_trust_level ON shared_learnings(trust_level);
            CREATE INDEX IF NOT EXISTS idx_source_agent ON shared_learnings(source_agent);
            CREATE INDEX IF NOT EXISTS idx_updated_at ON shared_learnings(updated_at);
            "#,
        )
        .execute(&self.pool)
        .await?;

        debug!("SharedLearningStore schema initialized");
        Ok(())
    }

    /// Find similar learnings using BM25 scoring
    pub async fn find_similar(
        &self,
        learning: &SharedLearning,
        limit: usize,
    ) -> Result<Vec<(SharedLearning, f64)>, StoreError> {
        let searchable_text = learning.extract_searchable_text();
        let all_learnings = self.list_all().await?;

        if all_learnings.is_empty() {
            return Ok(Vec::new());
        }

        // Build document frequency map for IDF calculation
        let mut doc_freqs: HashMap<String, usize> = HashMap::new();
        let mut total_doc_len = 0;

        for doc in &all_learnings {
            let text = doc.extract_searchable_text();
            let terms: Vec<String> = text.split_whitespace().map(|s| s.to_string()).collect();
            total_doc_len += terms.len();

            let unique_terms: std::collections::HashSet<String> = terms.into_iter().collect();
            for term in unique_terms {
                *doc_freqs.entry(term).or_insert(0) += 1;
            }
        }

        let avg_doc_len = total_doc_len as f64 / all_learnings.len() as f64;
        let mut scorer = Bm25Scorer::new(all_learnings.len(), avg_doc_len);

        // Score each document
        let mut scored: Vec<(SharedLearning, f64)> = all_learnings
            .into_iter()
            .filter(|doc| doc.id != learning.id) // Exclude self
            .map(|doc| {
                let doc_text = doc.extract_searchable_text();
                let score = scorer.score(&searchable_text, &doc_text, &doc_freqs);
                let normalized = scorer.normalize_score(score, searchable_text.split_whitespace().count());
                (doc, normalized)
            })
            .filter(|(_, score)| *score >= self.config.similarity_threshold)
            .collect();

        // Sort by score descending
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

        // Limit results
        if scored.len() > limit {
            scored.truncate(limit);
        }

        Ok(scored)
    }

    /// Store a new learning with BM25 deduplication
    ///
    /// If a similar learning exists (BM25 similarity > threshold),
    /// merges the new learning into the existing one instead of creating a duplicate.
    pub async fn store_with_dedup(
        &self,
        learning: SharedLearning,
    ) -> Result<StoreResult, StoreError> {
        // Check for similar learnings
        let similar = self.find_similar(&learning, 1).await?;

        if let Some((existing, similarity)) = similar.into_iter().next() {
            // Merge into existing learning
            info!(
                "Merging learning '{}' into existing '{}' (similarity: {:.2})",
                learning.id, existing.id, similarity
            );
            let existing_id = existing.id.clone();
            self.merge_learning(existing_id.clone(), learning).await?;
            return Ok(StoreResult::Merged(existing_id));
        }

        // No similar learning found, insert new
        self.insert(learning).await?;
        Ok(StoreResult::Created)
    }

    /// Insert a new learning into the store
    async fn insert(&self, learning: SharedLearning) -> Result<(), StoreError> {
        let agent_names = serde_json::to_string(&learning.quality.agent_names)?;
        let applicable_agents = serde_json::to_string(&learning.applicable_agents)?;
        let keywords = serde_json::to_string(&learning.keywords)?;

        sqlx::query(
            r#"
            INSERT INTO shared_learnings (
                id, title, content, trust_level, applied_count, effective_count,
                agent_count, agent_names, last_applied_at, success_rate,
                source, source_agent, applicable_agents, keywords, verify_pattern,
                created_at, updated_at, promoted_at, wiki_page_name,
                original_command, error_context, correction
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&learning.id)
        .bind(&learning.title)
        .bind(&learning.content)
        .bind(learning.trust_level.as_str())
        .bind(learning.quality.applied_count as i64)
        .bind(learning.quality.effective_count as i64)
        .bind(learning.quality.agent_count as i64)
        .bind(agent_names)
        .bind(learning.quality.last_applied_at)
        .bind(learning.quality.success_rate)
        .bind(learning.source.to_string())
        .bind(&learning.source_agent)
        .bind(applicable_agents)
        .bind(keywords)
        .bind(&learning.verify_pattern)
        .bind(learning.created_at)
        .bind(learning.updated_at)
        .bind(learning.promoted_at)
        .bind(&learning.wiki_page_name)
        .bind(&learning.original_command)
        .bind(&learning.error_context)
        .bind(&learning.correction)
        .execute(&self.pool)
        .await?;

        info!("Inserted learning: {}", learning.id);
        Ok(())
    }

    /// Merge a new learning into an existing one
    async fn merge_learning(&self, existing_id: String, new_learning: SharedLearning) -> Result<(), StoreError> {
        // Get existing learning
        let existing = self.get(&existing_id).await?;

        // Merge quality metrics
        let applied_count = existing.quality.applied_count + 1;
        let effective_count = existing.quality.effective_count + new_learning.quality.effective_count;
        
        // Merge agent names
        let mut agent_names = existing.quality.agent_names.clone();
        if !agent_names.contains(&new_learning.source_agent) {
            agent_names.push(new_learning.source_agent.clone());
        }

        // Calculate new success rate
        let success_rate = if applied_count > 0 {
            Some(effective_count as f64 / applied_count as f64)
        } else {
            None
        };

        // Update the existing learning
        sqlx::query(
            r#"
            UPDATE shared_learnings
            SET applied_count = ?,
                effective_count = ?,
                agent_count = ?,
                agent_names = ?,
                last_applied_at = ?,
                success_rate = ?,
                updated_at = ?
            WHERE id = ?
            "#,
        )
        .bind(applied_count as i64)
        .bind(effective_count as i64)
        .bind(agent_names.len() as i64)
        .bind(serde_json::to_string(&agent_names)?)
        .bind(Utc::now())
        .bind(success_rate)
        .bind(Utc::now())
        .bind(&existing_id)
        .execute(&self.pool)
        .await?;

        // Check for auto-promotion
        if self.config.auto_promote_l2 {
            let agent_count = agent_names.len() as u32;
            if applied_count >= 3 && agent_count >= 2 && existing.trust_level == TrustLevel::L1 {
                self.promote_to_l2(&existing_id).await?;
            }
        }

        Ok(())
    }

    /// Get a learning by ID
    pub async fn get(&self, id: &str) -> Result<SharedLearning, StoreError> {
        let row: Option<SharedLearningRow> = sqlx::query_as(
            r#"
            SELECT * FROM shared_learnings WHERE id = ?
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(row) => Ok(row.into()),
            None => Err(StoreError::NotFound(id.to_string())),
        }
    }

    /// List all learnings
    pub async fn list_all(&self) -> Result<Vec<SharedLearning>, StoreError> {
        let rows: Vec<SharedLearningRow> = sqlx::query_as(
            r#"
            SELECT * FROM shared_learnings ORDER BY updated_at DESC
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    /// List learnings by trust level
    pub async fn list_by_trust_level(
        &self,
        trust_level: TrustLevel,
    ) -> Result<Vec<SharedLearning>, StoreError> {
        let rows: Vec<SharedLearningRow> = sqlx::query_as(
            r#"
            SELECT * FROM shared_learnings WHERE trust_level = ? ORDER BY updated_at DESC
            "#,
        )
        .bind(trust_level.as_str())
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    /// Promote a learning to L2
    pub async fn promote_to_l2(&self, id: &str) -> Result<(), StoreError> {
        sqlx::query(
            r#"
            UPDATE shared_learnings
            SET trust_level = 'L2',
                promoted_at = ?,
                updated_at = ?
            WHERE id = ? AND trust_level = 'L1'
            "#,
        )
        .bind(Utc::now())
        .bind(Utc::now())
        .bind(id)
        .execute(&self.pool)
        .await?;

        info!("Promoted learning {} to L2", id);
        Ok(())
    }

    /// Promote a learning to L3
    pub async fn promote_to_l3(&self, id: &str) -> Result<(), StoreError> {
        sqlx::query(
            r#"
            UPDATE shared_learnings
            SET trust_level = 'L3',
                promoted_at = ?,
                updated_at = ?
            WHERE id = ?
            "#,
        )
        .bind(Utc::now())
        .bind(Utc::now())
        .bind(id)
        .execute(&self.pool)
        .await?;

        info!("Promoted learning {} to L3", id);
        Ok(())
    }

    /// Record an application of a learning
    pub async fn record_application(
        &self,
        id: &str,
        agent_name: &str,
        effective: bool,
    ) -> Result<(), StoreError> {
        let learning = self.get(id).await?;
        let mut quality = learning.quality.clone();
        quality.record_application(agent_name, effective);

        let agent_names = serde_json::to_string(&quality.agent_names)?;

        sqlx::query(
            r#"
            UPDATE shared_learnings
            SET applied_count = ?,
                effective_count = ?,
                agent_count = ?,
                agent_names = ?,
                last_applied_at = ?,
                success_rate = ?,
                updated_at = ?
            WHERE id = ?
            "#,
        )
        .bind(quality.applied_count as i64)
        .bind(quality.effective_count as i64)
        .bind(quality.agent_count as i64)
        .bind(agent_names)
        .bind(quality.last_applied_at)
        .bind(quality.success_rate)
        .bind(Utc::now())
        .bind(id)
        .execute(&self.pool)
        .await?;

        // Check for auto-promotion
        if self.config.auto_promote_l2
            && learning.trust_level == TrustLevel::L1
            && quality.meets_l2_criteria()
        {
            self.promote_to_l2(id).await?;
        }

        Ok(())
    }

    /// Update wiki page name for a learning
    pub async fn update_wiki_page_name(
        &self,
        id: &str,
        wiki_page_name: &str,
    ) -> Result<(), StoreError> {
        sqlx::query(
            r#"
            UPDATE shared_learnings
            SET wiki_page_name = ?,
                updated_at = ?
            WHERE id = ?
            "#,
        )
        .bind(wiki_page_name)
        .bind(Utc::now())
        .bind(id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Suggest learnings based on context
    ///
    /// Returns learnings ranked by BM25 relevance score
    pub async fn suggest(
        &self,
        context: &str,
        agent_name: &str,
        limit: usize,
    ) -> Result<Vec<(SharedLearning, f64)>, StoreError> {
        let all_learnings = self.list_all().await?;

        if all_learnings.is_empty() {
            return Ok(Vec::new());
        }

        // Build document frequency map
        let mut doc_freqs: HashMap<String, usize> = HashMap::new();
        let mut total_doc_len = 0;

        for doc in &all_learnings {
            let text = doc.extract_searchable_text();
            let terms: Vec<String> = text.split_whitespace().map(|s| s.to_string()).collect();
            total_doc_len += terms.len();

            let unique_terms: std::collections::HashSet<String> = terms.into_iter().collect();
            for term in unique_terms {
                *doc_freqs.entry(term).or_insert(0) += 1;
            }
        }

        let avg_doc_len = total_doc_len as f64 / all_learnings.len() as f64;
        let mut scorer = Bm25Scorer::new(all_learnings.len(), avg_doc_len);

        let query_lower = context.to_lowercase();
        let query_len = query_lower.split_whitespace().count();

        // Score and filter learnings
        let mut scored: Vec<(SharedLearning, f64)> = all_learnings
            .into_iter()
            .filter(|doc| {
                // Filter by applicable agents if specified
                doc.applicable_agents.is_empty()
                    || doc.applicable_agents.contains(&agent_name.to_string())
            })
            .map(|doc| {
                let doc_text = doc.extract_searchable_text();
                let score = scorer.score(&query_lower, &doc_text, &doc_freqs);
                let normalized = scorer.normalize_score(score, query_len);
                
                // Weight by trust level
                let weighted = normalized * doc.trust_level.weight() as f64;
                
                (doc, weighted)
            })
            .filter(|(_, score)| *score > 0.0)
            .collect();

        // Sort by weighted score descending
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

        // Limit results
        if scored.len() > limit {
            scored.truncate(limit);
        }

        Ok(scored)
    }

    /// Close the store connection
    pub async fn close(&self) {
        self.pool.close().await;
    }
}

/// Result of storing a learning
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StoreResult {
    /// New learning was created
    Created,
    /// Merged into existing learning
    Merged(String), // Contains the ID of the existing learning
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn create_test_store() -> SharedLearningStore {
        let config = StoreConfig::in_memory()
            .with_similarity_threshold(0.3);
        SharedLearningStore::open(config).await.unwrap()
    }

    #[tokio::test]
    async fn test_store_open() {
        let store = create_test_store().await;
        let learnings = store.list_all().await.unwrap();
        assert!(learnings.is_empty());
    }

    #[tokio::test]
    async fn test_insert_and_get() {
        let store = create_test_store().await;

        let learning = SharedLearning::new(
            "Test Learning".to_string(),
            "Test content".to_string(),
            LearningSource::Manual,
            "test-agent".to_string(),
        );

        let id = learning.id.clone();
        store.insert(learning).await.unwrap();

        let retrieved = store.get(&id).await.unwrap();
        assert_eq!(retrieved.id, id);
        assert_eq!(retrieved.title, "Test Learning");
        assert_eq!(retrieved.trust_level, TrustLevel::L1);
    }

    #[tokio::test]
    async fn test_list_by_trust_level() {
        let store = create_test_store().await;

        let mut learning = SharedLearning::new(
            "L2 Learning".to_string(),
            "Content".to_string(),
            LearningSource::Manual,
            "agent".to_string(),
        );
        learning.promote_to_l2();

        store.insert(learning).await.unwrap();

        let l2_learnings = store.list_by_trust_level(TrustLevel::L2).await.unwrap();
        assert_eq!(l2_learnings.len(), 1);

        let l1_learnings = store.list_by_trust_level(TrustLevel::L1).await.unwrap();
        assert!(l1_learnings.is_empty());
    }

    #[tokio::test]
    async fn test_record_application() {
        let store = create_test_store().await;

        let learning = SharedLearning::new(
            "Test".to_string(),
            "Content".to_string(),
            LearningSource::Manual,
            "agent1".to_string(),
        );
        let id = learning.id.clone();
        store.insert(learning).await.unwrap();

        // Record applications
        store.record_application(&id, "agent1", true).await.unwrap();
        store.record_application(&id, "agent2", true).await.unwrap();
        store.record_application(&id, "agent2", true).await.unwrap();

        let retrieved = store.get(&id).await.unwrap();
        assert_eq!(retrieved.quality.applied_count, 3);
        assert_eq!(retrieved.quality.effective_count, 3);
        assert_eq!(retrieved.quality.agent_count, 2);
    }

    #[tokio::test]
    async fn test_promote_to_l2() {
        let store = create_test_store().await;

        let learning = SharedLearning::new(
            "Test".to_string(),
            "Content".to_string(),
            LearningSource::Manual,
            "agent".to_string(),
        );
        let id = learning.id.clone();
        store.insert(learning).await.unwrap();

        store.promote_to_l2(&id).await.unwrap();

        let retrieved = store.get(&id).await.unwrap();
        assert_eq!(retrieved.trust_level, TrustLevel::L2);
        assert!(retrieved.promoted_at.is_some());
    }

    #[tokio::test]
    async fn test_suggest() {
        let store = create_test_store().await;

        let learning = SharedLearning::new(
            "Git Push Error".to_string(),
            "How to fix git push errors".to_string(),
            LearningSource::Manual,
            "agent".to_string(),
        )
        .with_keywords(vec!["git".to_string(), "push".to_string()]);

        store.insert(learning).await.unwrap();

        let suggestions = store.suggest("git push problems", "test-agent", 5).await.unwrap();
        assert!(!suggestions.is_empty());
        assert_eq!(suggestions[0].0.title, "Git Push Error");
    }

    #[tokio::test]
    async fn test_store_with_dedup() {
        let store = create_test_store().await;

        let learning1 = SharedLearning::new(
            "Git Push Error".to_string(),
            "How to fix git push errors".to_string(),
            LearningSource::Manual,
            "agent1".to_string(),
        );

        let result1 = store.store_with_dedup(learning1).await.unwrap();
        assert_eq!(result1, StoreResult::Created);

        // Similar learning should be merged
        let learning2 = SharedLearning::new(
            "Git Push Issues".to_string(),
            "How to fix git push errors and issues".to_string(),
            LearningSource::Manual,
            "agent2".to_string(),
        );

        let result2 = store.store_with_dedup(learning2).await.unwrap();
        assert!(matches!(result2, StoreResult::Merged(_)));
    }

    #[tokio::test]
    async fn test_auto_promotion() {
        let store = create_test_store().await;

        let learning = SharedLearning::new(
            "Test".to_string(),
            "Content".to_string(),
            LearningSource::Manual,
            "agent1".to_string(),
        );
        let id = learning.id.clone();
        store.insert(learning).await.unwrap();

        // Record 3+ applications across 2+ agents
        store.record_application(&id, "agent1", true).await.unwrap();
        store.record_application(&id, "agent1", true).await.unwrap();
        store.record_application(&id, "agent2", true).await.unwrap();

        let retrieved = store.get(&id).await.unwrap();
        assert_eq!(retrieved.trust_level, TrustLevel::L2);
    }
}
