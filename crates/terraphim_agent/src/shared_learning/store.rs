//! Shared learning store implementation
//!
//! Provides markdown-backed storage with BM25-based deduplication
//! and trust-gated promotion logic.

use std::collections::HashMap;

use chrono::Utc;
use thiserror::Error;
use tokio::sync::RwLock;
use tracing::{debug, info};

use crate::shared_learning::markdown_store::{
    MarkdownLearningStore, MarkdownStoreConfig, MarkdownStoreError,
};
use crate::shared_learning::types::{SharedLearning, TrustLevel};

#[derive(Error, Debug)]
pub enum StoreError {
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

impl From<MarkdownStoreError> for StoreError {
    fn from(e: MarkdownStoreError) -> Self {
        StoreError::Persistence(e.to_string())
    }
}

#[derive(Debug, Clone)]
pub struct StoreConfig {
    pub similarity_threshold: f64,
    pub auto_promote_l2: bool,
    pub markdown: MarkdownStoreConfig,
}

impl Default for StoreConfig {
    fn default() -> Self {
        Self {
            similarity_threshold: 0.8,
            auto_promote_l2: true,
            markdown: MarkdownStoreConfig::default(),
        }
    }
}

impl StoreConfig {
    pub fn with_similarity_threshold(mut self, threshold: f64) -> Self {
        self.similarity_threshold = threshold.clamp(0.0, 1.0);
        self
    }

    pub fn with_markdown_config(mut self, config: MarkdownStoreConfig) -> Self {
        self.markdown = config;
        self
    }
}

/// BM25 scoring for text similarity
pub struct Bm25Scorer {
    avg_doc_len: f64,
    total_docs: usize,
    idf_cache: HashMap<String, f64>,
}

impl Bm25Scorer {
    pub fn new(total_docs: usize, avg_doc_len: f64) -> Self {
        Self {
            avg_doc_len,
            total_docs,
            idf_cache: HashMap::new(),
        }
    }

    fn calculate_idf(&mut self, term: &str, doc_freq: usize) -> f64 {
        if let Some(&idf) = self.idf_cache.get(term) {
            return idf;
        }

        let n = doc_freq as f64;
        let n_docs = self.total_docs as f64;

        let idf = if n_docs <= 1.0 || n >= n_docs {
            0.5
        } else {
            ((n_docs - n + 0.5) / (n + 0.5)).ln().max(0.0)
        };

        self.idf_cache.insert(term.to_string(), idf);
        idf
    }

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

        let mut doc_tf: HashMap<String, usize> = HashMap::new();
        for term in &doc_terms {
            *doc_tf.entry(term.clone()).or_insert(0) += 1;
        }

        for term in &query_terms {
            let f = *doc_tf.get(term).unwrap_or(&0) as f64;
            let doc_freq = *doc_freqs.get(term).unwrap_or(&1);
            let idf = self.calculate_idf(term, doc_freq);

            let numerator = f * (K1 + 1.0);
            let denominator = f + K1 * (1.0 - B + B * doc_len / self.avg_doc_len);

            score += idf * numerator / denominator;
        }

        score
    }

    pub fn normalize_score(&self, score: f64, query_len: usize) -> f64 {
        if query_len == 0 {
            return 0.0;
        }
        let normalized = (score / query_len as f64).tanh();
        normalized.clamp(0.0, 1.0)
    }
}

pub struct SharedLearningStore {
    backend: MarkdownLearningStore,
    index: RwLock<HashMap<String, SharedLearning>>,
    config: StoreConfig,
}

impl SharedLearningStore {
    pub async fn open(config: StoreConfig) -> Result<Self, StoreError> {
        let backend = MarkdownLearningStore::with_config(config.markdown.clone());
        let store = Self {
            backend,
            index: RwLock::new(HashMap::new()),
            config,
        };
        store.load_all().await?;
        Ok(store)
    }

    async fn load_all(&self) -> Result<(), StoreError> {
        info!("Loading shared learnings from markdown backend");
        let all_learnings = self.backend.list_all().await?;
        let count = all_learnings.len();

        let mut index = self.index.write().await;
        for learning in all_learnings {
            // Simple last-write-wins deduplication. Since list_all() doesn't return
            // path metadata, we can't prefer canonical over shared copies.
            // Filesystem order typically lists agent directories before shared,
            // so canonical copies are usually first. This is a known limitation
            // that can be improved when list_all() returns path metadata.
            index.insert(learning.id.clone(), learning);
        }
        drop(index);

        info!("Loaded {} shared learnings into index", count);
        Ok(())
    }

    async fn persist(&self, learning: &SharedLearning) -> Result<(), StoreError> {
        self.backend.save(learning).await?;
        Ok(())
    }

    pub async fn insert(&self, learning: SharedLearning) -> Result<(), StoreError> {
        let id = learning.id.clone();
        self.persist(&learning).await?;
        self.index.write().await.insert(id, learning);
        Ok(())
    }

    pub async fn store_with_dedup(
        &self,
        learning: SharedLearning,
    ) -> Result<StoreResult, StoreError> {
        let search_text = learning.extract_searchable_text();
        let query_lower = search_text.to_lowercase();

        let index = self.index.read().await;
        let all_learnings: Vec<SharedLearning> = index.values().cloned().collect();
        drop(index);

        if !all_learnings.is_empty() {
            let mut doc_freqs: HashMap<String, usize> = HashMap::new();
            let mut total_doc_len = 0;

            for doc in &all_learnings {
                let text = doc.extract_searchable_text();
                let terms: std::collections::HashSet<String> = text
                    .to_lowercase()
                    .split_whitespace()
                    .map(|s| s.to_string())
                    .collect();
                total_doc_len += terms.len();
                for term in &terms {
                    *doc_freqs.entry(term.clone()).or_insert(0) += 1;
                }
            }

            let avg_doc_len = total_doc_len as f64 / all_learnings.len() as f64;
            let mut scorer = Bm25Scorer::new(all_learnings.len(), avg_doc_len);
            let query_len = query_lower.split_whitespace().count();

            let best_match = all_learnings
                .iter()
                .map(|doc| {
                    let doc_text = doc.extract_searchable_text();
                    let raw_score = scorer.score(&query_lower, &doc_text, &doc_freqs);
                    let normalized = scorer.normalize_score(raw_score, query_len);
                    (doc.id.clone(), normalized)
                })
                .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap());

            if let Some((existing_id, score)) = best_match {
                if score >= self.config.similarity_threshold {
                    debug!(
                        "Merging with existing learning {} (score={:.3})",
                        existing_id, score
                    );
                    self.merge_learning(&existing_id, &learning).await?;
                    return Ok(StoreResult::Merged(existing_id));
                }
            }
        }

        let id = learning.id.clone();
        self.insert(learning).await?;
        info!("Created new learning: {}", id);
        Ok(StoreResult::Created)
    }

    async fn merge_learning(
        &self,
        existing_id: &str,
        new_learning: &SharedLearning,
    ) -> Result<(), StoreError> {
        let mut index = self.index.write().await;
        let existing = index
            .get_mut(existing_id)
            .ok_or_else(|| StoreError::NotFound(existing_id.to_string()))?;

        existing.quality.applied_count += new_learning.quality.applied_count;
        existing.quality.effective_count += new_learning.quality.effective_count;

        for agent in &new_learning.quality.agent_names {
            if !existing.quality.agent_names.contains(agent) {
                existing.quality.agent_names.push(agent.clone());
            }
        }

        existing.updated_at = Utc::now();
        let merged = existing.clone();
        drop(index);

        self.persist(&merged).await?;
        Ok(())
    }

    pub async fn get(&self, id: &str) -> Result<SharedLearning, StoreError> {
        let index = self.index.read().await;
        index
            .get(id)
            .cloned()
            .ok_or_else(|| StoreError::NotFound(id.to_string()))
    }

    pub async fn list_all(&self) -> Result<Vec<SharedLearning>, StoreError> {
        let index = self.index.read().await;
        Ok(index.values().cloned().collect())
    }

    pub async fn list_by_trust_level(
        &self,
        level: TrustLevel,
    ) -> Result<Vec<SharedLearning>, StoreError> {
        let index = self.index.read().await;
        Ok(index
            .values()
            .filter(|l| l.trust_level == level)
            .cloned()
            .collect())
    }

    pub async fn promote_to_l2(&self, id: &str) -> Result<(), StoreError> {
        let mut index = self.index.write().await;
        let learning = index
            .get_mut(id)
            .ok_or_else(|| StoreError::NotFound(id.to_string()))?;
        learning.promote_to_l2();
        let updated = learning.clone();
        drop(index);

        self.persist(&updated).await?;
        info!("Promoted learning {} to L2", id);
        Ok(())
    }

    pub async fn promote_to_l3(&self, id: &str) -> Result<(), StoreError> {
        let mut index = self.index.write().await;
        let learning = index
            .get_mut(id)
            .ok_or_else(|| StoreError::NotFound(id.to_string()))?;
        learning.promote_to_l3();
        let updated = learning.clone();
        drop(index);

        self.persist(&updated).await?;
        info!("Promoted learning {} to L3", id);
        Ok(())
    }

    pub async fn record_application(
        &self,
        id: &str,
        agent_name: &str,
        effective: bool,
    ) -> Result<(), StoreError> {
        let mut index = self.index.write().await;
        let learning = index
            .get_mut(id)
            .ok_or_else(|| StoreError::NotFound(id.to_string()))?;

        learning.quality.record_application(agent_name, effective);
        learning.updated_at = Utc::now();

        let should_auto_promote = self.config.auto_promote_l2
            && learning.trust_level == TrustLevel::L1
            && learning.quality.meets_l2_criteria();

        if should_auto_promote {
            learning.promote_to_l2();
            info!("Auto-promoted learning {} to L2", id);
        }

        let updated = learning.clone();
        drop(index);

        self.persist(&updated).await?;
        Ok(())
    }

    pub async fn find_similar(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<(f64, SharedLearning)>, StoreError> {
        let index = self.index.read().await;
        let all_learnings: Vec<SharedLearning> = index.values().cloned().collect();
        drop(index);

        if all_learnings.is_empty() {
            return Ok(Vec::new());
        }

        let mut doc_freqs: HashMap<String, usize> = HashMap::new();
        let mut total_doc_len = 0;

        for doc in &all_learnings {
            let text = doc.extract_searchable_text();
            let terms: std::collections::HashSet<String> = text
                .to_lowercase()
                .split_whitespace()
                .map(|s| s.to_string())
                .collect();
            total_doc_len += terms.len();
            for term in &terms {
                *doc_freqs.entry(term.clone()).or_insert(0) += 1;
            }
        }

        let avg_doc_len = total_doc_len as f64 / all_learnings.len() as f64;
        let mut scorer = Bm25Scorer::new(all_learnings.len(), avg_doc_len);

        let query_lower = query.to_lowercase();
        let query_len = query_lower.split_whitespace().count();

        let mut scored: Vec<(f64, SharedLearning)> = all_learnings
            .into_iter()
            .map(|doc| {
                let doc_text = doc.extract_searchable_text();
                let raw_score = scorer.score(&query_lower, &doc_text, &doc_freqs);
                let normalized = scorer.normalize_score(raw_score, query_len);
                let weighted = normalized * doc.trust_level.weight() as f64;
                (weighted, doc)
            })
            .filter(|(score, _)| *score > 0.0)
            .collect();

        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());

        if scored.len() > limit {
            scored.truncate(limit);
        }

        Ok(scored)
    }

    pub async fn suggest(
        &self,
        context: &str,
        agent_name: &str,
        limit: usize,
    ) -> Result<Vec<SharedLearning>, StoreError> {
        let index = self.index.read().await;
        let applicable: Vec<SharedLearning> = index
            .values()
            .filter(|doc| {
                doc.applicable_agents.is_empty()
                    || doc.applicable_agents.contains(&agent_name.to_string())
            })
            .cloned()
            .collect();
        drop(index);

        if applicable.is_empty() {
            return Ok(Vec::new());
        }

        let mut doc_freqs: HashMap<String, usize> = HashMap::new();
        let mut total_doc_len = 0;

        for doc in &applicable {
            let text = doc.extract_searchable_text();
            let terms: std::collections::HashSet<String> = text
                .to_lowercase()
                .split_whitespace()
                .map(|s| s.to_string())
                .collect();
            total_doc_len += terms.len();
            for term in &terms {
                *doc_freqs.entry(term.clone()).or_insert(0) += 1;
            }
        }

        let avg_doc_len = total_doc_len as f64 / applicable.len() as f64;
        let mut scorer = Bm25Scorer::new(applicable.len(), avg_doc_len);

        let query_lower = context.to_lowercase();
        let query_len = query_lower.split_whitespace().count();

        let mut scored: Vec<SharedLearning> = applicable
            .into_iter()
            .map(|doc| {
                let doc_text = doc.extract_searchable_text();
                let raw_score = scorer.score(&query_lower, &doc_text, &doc_freqs);
                let normalized = scorer.normalize_score(raw_score, query_len);
                let weighted = normalized * doc.trust_level.weight() as f64;
                (doc, weighted)
            })
            .filter(|(_, score)| *score > 0.0)
            .map(|(doc, _)| doc)
            .collect();

        scored.truncate(limit);
        Ok(scored)
    }

    pub async fn close(&self) {
        info!("Shared learning store closed");
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StoreResult {
    Created,
    Merged(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared_learning::types::LearningSource;
    use tempfile::TempDir;

    async fn create_test_store() -> SharedLearningStore {
        let temp_dir = TempDir::new().unwrap();
        let markdown_config = MarkdownStoreConfig {
            learnings_dir: temp_dir.path().to_path_buf(),
            shared_dir_name: "shared".to_string(),
        };
        let config = StoreConfig::default()
            .with_similarity_threshold(0.3)
            .with_markdown_config(markdown_config);
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

        let suggestions = store
            .suggest("git push problems", "test-agent", 5)
            .await
            .unwrap();
        assert!(!suggestions.is_empty());
        assert_eq!(suggestions[0].title, "Git Push Error");
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

        store.record_application(&id, "agent1", true).await.unwrap();
        store.record_application(&id, "agent1", true).await.unwrap();
        store.record_application(&id, "agent2", true).await.unwrap();

        let retrieved = store.get(&id).await.unwrap();
        assert_eq!(retrieved.trust_level, TrustLevel::L2);
    }

    #[tokio::test]
    async fn test_open_loads_existing_markdown_learnings() {
        // Create a temp dir and directly save learnings via the markdown backend
        let temp_dir = TempDir::new().unwrap();
        let markdown_config = MarkdownStoreConfig {
            learnings_dir: temp_dir.path().to_path_buf(),
            shared_dir_name: "shared".to_string(),
        };
        let backend = MarkdownLearningStore::with_config(markdown_config.clone());

        let learning1 = SharedLearning::new(
            "Pre-existing Learning".to_string(),
            "This learning was saved before the store opened.".to_string(),
            LearningSource::AutoExtract,
            "test-agent".to_string(),
        );
        let id1 = learning1.id.clone();
        backend.save(&learning1).await.unwrap();

        let learning2 = SharedLearning::new(
            "Another Pre-existing".to_string(),
            "Also saved before open.".to_string(),
            LearningSource::Manual,
            "other-agent".to_string(),
        );
        let id2 = learning2.id.clone();
        backend.save(&learning2).await.unwrap();

        // Now open the store - it should load existing learnings
        let config = StoreConfig::default()
            .with_similarity_threshold(0.3)
            .with_markdown_config(markdown_config);
        let store = SharedLearningStore::open(config).await.unwrap();

        let all = store.list_all().await.unwrap();
        assert_eq!(all.len(), 2);

        let retrieved1 = store.get(&id1).await.unwrap();
        assert_eq!(retrieved1.title, "Pre-existing Learning");

        let retrieved2 = store.get(&id2).await.unwrap();
        assert_eq!(retrieved2.title, "Another Pre-existing");
    }

    #[tokio::test]
    async fn test_open_dedups_shared_and_canonical_copies() {
        // Create a temp dir and save the same learning to both agent dir and shared dir
        let temp_dir = TempDir::new().unwrap();
        let markdown_config = MarkdownStoreConfig {
            learnings_dir: temp_dir.path().to_path_buf(),
            shared_dir_name: "shared".to_string(),
        };
        let backend = MarkdownLearningStore::with_config(markdown_config.clone());

        let mut learning = SharedLearning::new(
            "Shared Dedup Test".to_string(),
            "Testing deduplication.".to_string(),
            LearningSource::AutoExtract,
            "agent-x".to_string(),
        );
        learning.id = "dedup-test-id".to_string();

        // Save to agent directory (canonical)
        backend.save(&learning).await.unwrap();
        // Save to shared directory
        backend.save_to_shared(&learning).await.unwrap();

        // Now open the store - it should deduplicate
        let config = StoreConfig::default()
            .with_similarity_threshold(0.3)
            .with_markdown_config(markdown_config);
        let store = SharedLearningStore::open(config).await.unwrap();

        let all = store.list_all().await.unwrap();
        // Should only have 1 entry despite 2 files on disk
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].id, "dedup-test-id");
    }

    #[tokio::test]
    async fn test_persist_after_promotion_and_application() {
        let temp_dir = TempDir::new().unwrap();
        let markdown_config = MarkdownStoreConfig {
            learnings_dir: temp_dir.path().to_path_buf(),
            shared_dir_name: "shared".to_string(),
        };
        let config = StoreConfig::default()
            .with_similarity_threshold(0.3)
            .with_markdown_config(markdown_config.clone());

        let store = SharedLearningStore::open(config).await.unwrap();

        let learning = SharedLearning::new(
            "Persist Test".to_string(),
            "Testing persistence.".to_string(),
            LearningSource::Manual,
            "test-agent".to_string(),
        );
        let id = learning.id.clone();
        store.insert(learning).await.unwrap();

        // Promote to L2
        store.promote_to_l2(&id).await.unwrap();

        // Record applications
        store.record_application(&id, "agent1", true).await.unwrap();
        store.record_application(&id, "agent2", true).await.unwrap();

        // Close and reopen the store
        store.close().await;

        let config2 = StoreConfig::default()
            .with_similarity_threshold(0.3)
            .with_markdown_config(markdown_config);
        let reopened = SharedLearningStore::open(config2).await.unwrap();

        let retrieved = reopened.get(&id).await.unwrap();
        assert_eq!(retrieved.trust_level, TrustLevel::L2);
        assert_eq!(retrieved.quality.applied_count, 2);
        assert_eq!(retrieved.quality.effective_count, 2);
        assert_eq!(retrieved.quality.agent_count, 2);
        assert!(retrieved.promoted_at.is_some());
    }
}
