//! Shared learning store implementation
//!
//! Provides markdown-backed storage with BM25-based deduplication
//! and trust-gated promotion logic.

use std::collections::HashMap;

use chrono::Utc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use crate::shared_learning::markdown_store::{
    MarkdownLearningStore, MarkdownStoreConfig, MarkdownStoreError,
};
use crate::shared_learning::types::{SharedLearning, TrustLevel};
pub use terraphim_types::shared_learning::StoreError;
use terraphim_types::shared_learning::SuggestionStatus;

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
    #[cfg(feature = "shared-learning")]
    role_graph: Option<std::sync::RwLock<terraphim_rolegraph::RoleGraph>>,
}

impl SharedLearningStore {
    pub async fn open(config: StoreConfig) -> Result<Self, StoreError> {
        let backend = MarkdownLearningStore::with_config(config.markdown.clone());
        let store = Self {
            backend,
            index: RwLock::new(HashMap::new()),
            config,
            #[cfg(feature = "shared-learning")]
            role_graph: None,
        };
        store.load_all().await?;
        Ok(store)
    }

    async fn load_all(&self) -> Result<(), StoreError> {
        info!("Loading shared learnings from markdown backend");
        let all_learnings = self.backend.list_all_with_origin().await?;
        let discovered = all_learnings.len();

        let mut selected: HashMap<String, (bool, SharedLearning)> = HashMap::new();
        for (is_shared, learning) in all_learnings {
            match selected.get(&learning.id) {
                None => {
                    selected.insert(learning.id.clone(), (is_shared, learning));
                }
                Some((existing_is_shared, _)) => {
                    if *existing_is_shared && !is_shared {
                        selected.insert(learning.id.clone(), (is_shared, learning));
                    }
                }
            }
        }

        let mut index = self.index.write().await;
        for (_, learning) in selected.into_values() {
            index.insert(learning.id.clone(), learning);
        }
        let loaded = index.len();
        drop(index);

        info!(
            "Loaded {} shared learnings into index ({} discovered)",
            loaded, discovered
        );
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

    /// Record that a graph query touched this learning
    ///
    /// Increments the applied_count quality metric and persists the update.
    pub async fn record_graph_touch(&self, learning_id: &str) -> Result<(), StoreError> {
        let mut index = self.index.write().await;
        if let Some(learning) = index.get_mut(learning_id) {
            learning.quality.applied_count += 1;
            learning.updated_at = Utc::now();
            let updated = learning.clone();
            drop(index);
            self.persist(&updated).await?;
            Ok(())
        } else {
            Err(StoreError::NotFound(learning_id.to_string()))
        }
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

    pub async fn list_pending(&self) -> Result<Vec<SharedLearning>, StoreError> {
        self.list_by_status(SuggestionStatus::Pending).await
    }

    pub async fn list_by_status(
        &self,
        status: SuggestionStatus,
    ) -> Result<Vec<SharedLearning>, StoreError> {
        let index = self.index.read().await;
        Ok(index
            .values()
            .filter(|l| l.suggestion_status == status)
            .cloned()
            .collect())
    }

    pub async fn approve(&self, id: &str) -> Result<(), StoreError> {
        let mut index = self.index.write().await;
        let learning = index
            .get_mut(id)
            .ok_or_else(|| StoreError::NotFound(id.to_string()))?;
        learning.suggestion_status = SuggestionStatus::Approved;
        learning.promote_to_l3();
        let updated = learning.clone();
        drop(index);

        self.persist(&updated).await?;
        info!("Approved suggestion {}", id);
        Ok(())
    }

    pub async fn reject(&self, id: &str, reason: Option<&str>) -> Result<(), StoreError> {
        let mut index = self.index.write().await;
        let learning = index
            .get_mut(id)
            .ok_or_else(|| StoreError::NotFound(id.to_string()))?;
        learning.suggestion_status = SuggestionStatus::Rejected;
        learning.rejection_reason = reason.map(|r| r.to_string());
        learning.updated_at = Utc::now();
        let updated = learning.clone();
        drop(index);

        self.persist(&updated).await?;
        info!("Rejected suggestion {}", id);
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

    #[cfg(feature = "shared-learning")]
    pub fn set_role_graph(&mut self, graph: terraphim_rolegraph::RoleGraph) {
        self.role_graph = Some(std::sync::RwLock::new(graph));
    }

    #[cfg(feature = "shared-learning")]
    pub fn role_graph(&self) -> Option<&std::sync::RwLock<terraphim_rolegraph::RoleGraph>> {
        self.role_graph.as_ref()
    }
}

#[cfg(feature = "shared-learning")]
impl terraphim_middleware::feedback_loop::GraphTouchStore for SharedLearningStore {
    fn record_graph_touch<'a>(
        &'a self,
        learning_id: &'a str,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), StoreError>> + Send + 'a>>
    {
        Box::pin(async move {
            let mut index = self.index.write().await;
            if let Some(learning) = index.get_mut(learning_id) {
                learning.quality.applied_count += 1;
                learning.updated_at = chrono::Utc::now();
                let updated = learning.clone();
                drop(index);
                self.persist(&updated).await?;
                Ok(())
            } else {
                Err(StoreError::NotFound(learning_id.to_string()))
            }
        })
    }
}

#[cfg(feature = "shared-learning")]
fn block_on<F: std::future::Future>(fut: F) -> F::Output {
    tokio::task::block_in_place(|| {
        let rt = tokio::runtime::Handle::current();
        rt.block_on(fut)
    })
}

#[cfg(feature = "shared-learning")]
impl terraphim_types::shared_learning::LearningStore for SharedLearningStore {
    fn insert(
        &self,
        learning: terraphim_types::shared_learning::SharedLearning,
    ) -> Result<String, terraphim_types::shared_learning::StoreError> {
        let id = learning.id.clone();
        block_on(Self::insert(self, learning))?;
        Ok(id)
    }

    fn get(
        &self,
        id: &str,
    ) -> Result<
        terraphim_types::shared_learning::SharedLearning,
        terraphim_types::shared_learning::StoreError,
    > {
        block_on(Self::get(self, id))
    }

    fn query_relevant(
        &self,
        agent: &str,
        context: &str,
        min_trust: terraphim_types::shared_learning::TrustLevel,
        limit: usize,
    ) -> Result<
        Vec<terraphim_types::shared_learning::SharedLearning>,
        terraphim_types::shared_learning::StoreError,
    > {
        let index = block_on(self.index.read());
        let mut candidates: Vec<terraphim_types::shared_learning::SharedLearning> = index
            .values()
            .filter(|l| l.trust_level >= min_trust)
            .filter(|l| {
                l.applicable_agents.is_empty()
                    || l.applicable_agents
                        .iter()
                        .any(|a| a.eq_ignore_ascii_case(agent))
            })
            .cloned()
            .collect();
        drop(index);

        if candidates.is_empty() {
            return Ok(Vec::new());
        }

        if !context.is_empty() {
            let context_lower = context.to_lowercase();
            if let Some(ref graph_lock) = self.role_graph {
                if let Ok(graph) = graph_lock.read() {
                    if let Ok(graph_results) = graph.query_graph(context, None, None) {
                        if !graph_results.is_empty() {
                            let graph_id_rank: std::collections::HashMap<String, u64> =
                                graph_results
                                    .into_iter()
                                    .map(|(id, doc)| (id, doc.rank))
                                    .collect();
                            candidates.retain(|l| {
                                graph_id_rank.contains_key(&l.id)
                                    || l.extract_searchable_text().contains(&context_lower)
                            });
                            candidates.sort_by(|a, b| {
                                let a_rank = graph_id_rank.get(&a.id).copied().unwrap_or(0);
                                let b_rank = graph_id_rank.get(&b.id).copied().unwrap_or(0);
                                b_rank.cmp(&a_rank)
                            });
                            candidates.truncate(limit);
                            return Ok(candidates);
                        }
                    }
                }
            }

            candidates.retain(|l| l.extract_searchable_text().contains(&context_lower));
        }

        candidates.sort_by_key(|l| std::cmp::Reverse(l.trust_level.weight()));
        candidates.truncate(limit);
        Ok(candidates)
    }

    fn record_applied(&self, id: &str) -> Result<(), terraphim_types::shared_learning::StoreError> {
        block_on(self.record_application(id, "learning-store", false))
    }

    fn record_effective(
        &self,
        id: &str,
    ) -> Result<(), terraphim_types::shared_learning::StoreError> {
        block_on(self.record_application(id, "learning-store", true))
    }

    fn list_by_trust(
        &self,
        min_trust: terraphim_types::shared_learning::TrustLevel,
    ) -> Result<
        Vec<terraphim_types::shared_learning::SharedLearning>,
        terraphim_types::shared_learning::StoreError,
    > {
        let all = block_on(self.list_all())?;
        Ok(all
            .into_iter()
            .filter(|l| l.trust_level >= min_trust)
            .collect())
    }

    fn archive_stale(
        &self,
        max_age_days: u32,
    ) -> Result<usize, terraphim_types::shared_learning::StoreError> {
        let cutoff = chrono::Utc::now() - chrono::Duration::days(max_age_days as i64);
        let mut index = block_on(self.index.write());
        let before = index.len();
        let stale: Vec<(String, String)> = index
            .iter()
            .filter(|(_, l)| {
                l.trust_level <= terraphim_types::shared_learning::TrustLevel::L0
                    && l.updated_at <= cutoff
            })
            .map(|(id, l)| (id.clone(), l.source_agent.clone()))
            .collect();
        for (id, _) in &stale {
            index.remove(id.as_str());
        }
        drop(index);
        for (id, agent) in &stale {
            if let Err(e) = block_on(self.backend.delete(agent, id)) {
                warn!("Failed to delete markdown for stale learning {}: {e}", id);
            }
        }
        let removed = before - stale.len();
        Ok(removed)
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

        // Save a stale variant to shared directory with same ID.
        // Canonical should win after hydration.
        let mut stale_shared_copy = learning.clone();
        stale_shared_copy.title = "Stale Shared Copy".to_string();
        stale_shared_copy.trust_level = TrustLevel::L1;
        backend.save_to_shared(&stale_shared_copy).await.unwrap();

        // Now open the store - it should deduplicate
        let config = StoreConfig::default()
            .with_similarity_threshold(0.3)
            .with_markdown_config(markdown_config);
        let store = SharedLearningStore::open(config).await.unwrap();

        let all = store.list_all().await.unwrap();
        // Should only have 1 entry despite 2 files on disk
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].id, "dedup-test-id");
        assert_eq!(all[0].title, "Shared Dedup Test");
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

    #[tokio::test]
    async fn test_approve_promotes_to_l3() {
        let store = create_test_store().await;
        let learning = SharedLearning::new(
            "Approve Test".to_string(),
            "Content".to_string(),
            LearningSource::Manual,
            "agent".to_string(),
        );
        let id = learning.id.clone();
        store.insert(learning).await.unwrap();

        store.approve(&id).await.unwrap();

        let retrieved = store.get(&id).await.unwrap();
        assert_eq!(retrieved.trust_level, TrustLevel::L3);
        assert_eq!(retrieved.suggestion_status, SuggestionStatus::Approved);
    }

    #[tokio::test]
    async fn test_reject_sets_status() {
        let store = create_test_store().await;
        let learning = SharedLearning::new(
            "Reject Test".to_string(),
            "Content".to_string(),
            LearningSource::Manual,
            "agent".to_string(),
        );
        let id = learning.id.clone();
        store.insert(learning).await.unwrap();

        store.reject(&id, Some("not applicable")).await.unwrap();

        let retrieved = store.get(&id).await.unwrap();
        assert_eq!(retrieved.suggestion_status, SuggestionStatus::Rejected);
        assert_eq!(
            retrieved.rejection_reason.as_deref(),
            Some("not applicable")
        );
        assert_eq!(retrieved.trust_level, TrustLevel::L1);
    }

    #[tokio::test]
    async fn test_list_pending_filters() {
        let store = create_test_store().await;

        let pending = SharedLearning::new(
            "Pending".to_string(),
            "Content".to_string(),
            LearningSource::Manual,
            "agent".to_string(),
        );
        let pending_id = pending.id.clone();
        store.insert(pending).await.unwrap();

        let mut approved = SharedLearning::new(
            "Approved".to_string(),
            "Content".to_string(),
            LearningSource::Manual,
            "agent".to_string(),
        );
        approved.suggestion_status = SuggestionStatus::Approved;
        store.insert(approved).await.unwrap();

        let result = store.list_pending().await.unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].id, pending_id);
    }

    #[tokio::test]
    async fn test_list_by_status() {
        let store = create_test_store().await;

        let mut rejected = SharedLearning::new(
            "Rejected".to_string(),
            "Content".to_string(),
            LearningSource::Manual,
            "agent".to_string(),
        );
        rejected.suggestion_status = SuggestionStatus::Rejected;
        let rejected_id = rejected.id.clone();
        store.insert(rejected).await.unwrap();

        let result = store
            .list_by_status(SuggestionStatus::Rejected)
            .await
            .unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].id, rejected_id);
    }

    #[cfg(feature = "shared-learning")]
    mod learning_store_trait_tests {
        use super::*;
        use terraphim_types::shared_learning::{LearningStore, TrustLevel as Tl};

        async fn create_trait_test_store() -> SharedLearningStore {
            create_test_store().await
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn test_trait_insert_and_get() {
            let store = create_trait_test_store().await;
            let dyn_store: &dyn LearningStore = &store;

            let learning = SharedLearning::new(
                "Trait Test".to_string(),
                "Testing trait insert and get".to_string(),
                LearningSource::Manual,
                "test-agent".to_string(),
            );
            let id = dyn_store.insert(learning).unwrap();
            assert!(!id.is_empty());

            let retrieved = dyn_store.get(&id).unwrap();
            assert_eq!(retrieved.title, "Trait Test");
            assert_eq!(retrieved.source_agent, "test-agent");
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn test_trait_get_not_found() {
            let store = create_trait_test_store().await;
            let dyn_store: &dyn LearningStore = &store;
            assert!(dyn_store.get("nonexistent-id").is_err());
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn test_trait_record_applied_and_effective() {
            let store = create_trait_test_store().await;
            let dyn_store: &dyn LearningStore = &store;

            let learning = SharedLearning::new(
                "App Test".to_string(),
                "Content".to_string(),
                LearningSource::Manual,
                "agent".to_string(),
            );
            let id = dyn_store.insert(learning).unwrap();

            dyn_store.record_applied(&id).unwrap();
            let l = dyn_store.get(&id).unwrap();
            assert_eq!(l.quality.applied_count, 1);

            dyn_store.record_effective(&id).unwrap();
            let l = dyn_store.get(&id).unwrap();
            assert_eq!(l.quality.applied_count, 2);
            assert_eq!(l.quality.effective_count, 1);
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn test_trait_auto_promote_on_effective() {
            let store = create_trait_test_store().await;
            let dyn_store: &dyn LearningStore = &store;

            let learning = SharedLearning::new(
                "Promote Test".to_string(),
                "Content".to_string(),
                LearningSource::Manual,
                "agent".to_string(),
            );
            let id = dyn_store.insert(learning).unwrap();

            assert_eq!(dyn_store.get(&id).unwrap().trust_level, Tl::L1);

            dyn_store.record_effective(&id).unwrap();
            dyn_store.record_effective(&id).unwrap();
            dyn_store.record_effective(&id).unwrap();
            dyn_store.record_effective(&id).unwrap();

            let l = dyn_store.get(&id).unwrap();
            assert_eq!(l.quality.effective_count, 4);
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn test_trait_list_by_trust() {
            let store = create_trait_test_store().await;
            let dyn_store: &dyn LearningStore = &store;

            let l1 = SharedLearning::new(
                "L1".to_string(),
                "c".to_string(),
                LearningSource::Manual,
                "a".to_string(),
            );
            let mut l2 = SharedLearning::new(
                "L2".to_string(),
                "c".to_string(),
                LearningSource::Manual,
                "a".to_string(),
            );
            l2.promote_to_l2();
            dyn_store.insert(l1).unwrap();
            dyn_store.insert(l2).unwrap();

            let l1_plus = dyn_store.list_by_trust(Tl::L1).unwrap();
            assert_eq!(l1_plus.len(), 2);

            let l2_only = dyn_store.list_by_trust(Tl::L2).unwrap();
            assert_eq!(l2_only.len(), 1);
            assert_eq!(l2_only[0].title, "L2");
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn test_trait_query_relevant_respects_trust() {
            let store = create_trait_test_store().await;
            let dyn_store: &dyn LearningStore = &store;

            let mut l0 = SharedLearning::new(
                "L0 Test".to_string(),
                "l0 content".to_string(),
                LearningSource::Manual,
                "agent".to_string(),
            );
            l0.trust_level = Tl::L0;
            dyn_store.insert(l0).unwrap();

            let results = dyn_store
                .query_relevant("agent", "l0 content", Tl::L1, 10)
                .unwrap();
            assert!(results.is_empty(), "L0 should be filtered out at L1 min");
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn test_trait_query_relevant_respects_agents() {
            let store = create_trait_test_store().await;
            let dyn_store: &dyn LearningStore = &store;

            let learning = SharedLearning::new(
                "Agent Specific".to_string(),
                "Only for security".to_string(),
                LearningSource::Manual,
                "sec".to_string(),
            )
            .with_applicable_agents(vec!["security-audit".to_string()]);
            dyn_store.insert(learning).unwrap();

            let for_sec = dyn_store
                .query_relevant("security-audit", "security", Tl::L1, 10)
                .unwrap();
            assert_eq!(for_sec.len(), 1);

            let for_other = dyn_store
                .query_relevant("other-agent", "security", Tl::L1, 10)
                .unwrap();
            assert!(for_other.is_empty());
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn test_trait_archive_stale() {
            let temp_dir = TempDir::new().unwrap();
            let markdown_config = MarkdownStoreConfig {
                learnings_dir: temp_dir.path().to_path_buf(),
                shared_dir_name: "shared".to_string(),
            };
            let config = StoreConfig::default().with_markdown_config(markdown_config);
            let store = SharedLearningStore::open(config).await.unwrap();

            let mut l0_stale = SharedLearning::new(
                "stale".to_string(),
                "c".to_string(),
                LearningSource::Manual,
                "a".to_string(),
            );
            l0_stale.trust_level = Tl::L0;
            l0_stale.updated_at = chrono::Utc::now() - chrono::Duration::days(60);
            let mut l1_old = SharedLearning::new(
                "old but L1".to_string(),
                "c".to_string(),
                LearningSource::Manual,
                "a".to_string(),
            );
            l1_old.trust_level = Tl::L1;
            l1_old.updated_at = chrono::Utc::now() - chrono::Duration::days(60);

            let dyn_store: &dyn LearningStore = &store;
            dyn_store.insert(l0_stale).unwrap();
            dyn_store.insert(l1_old).unwrap();

            let archived = dyn_store.archive_stale(30).unwrap();
            assert_eq!(archived, 1);

            let remaining = dyn_store.list_by_trust(Tl::L0).unwrap();
            assert_eq!(remaining.len(), 1);
            assert_eq!(remaining[0].trust_level, Tl::L1);
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn test_trait_query_relevant_with_role_graph() {
            use terraphim_rolegraph::RoleGraph;
            use terraphim_types::{Document, NormalizedTerm, NormalizedTermValue, Thesaurus};

            let mut store = create_test_store().await;

            let mut thesaurus = Thesaurus::new("test".to_string());
            thesaurus.insert(
                NormalizedTermValue::from("git"),
                NormalizedTerm::new(1, NormalizedTermValue::from("git")),
            );
            thesaurus.insert(
                NormalizedTermValue::from("push"),
                NormalizedTerm::new(2, NormalizedTermValue::from("push")),
            );

            let mut graph =
                RoleGraph::new_sync(terraphim_types::RoleName::new("test-role"), thesaurus)
                    .unwrap();

            let doc = Document {
                id: "doc-1".to_string(),
                url: String::new(),
                title: "Git Push".to_string(),
                body: "Git push force error fix".to_string(),
                description: None,
                summarization: None,
                stub: None,
                tags: None,
                rank: None,
                source_haystack: None,
                doc_type: terraphim_types::DocumentType::default(),
                synonyms: None,
                route: None,
                priority: None,
            };
            let learning_id = "learning-graph-test";
            graph.insert_document(learning_id, doc);

            store.set_role_graph(graph);

            let learning = SharedLearning::new(
                "Graph Test Learning".to_string(),
                "Git push force error fix".to_string(),
                LearningSource::Manual,
                "agent".to_string(),
            );
            let mut l = learning;
            l.id = learning_id.to_string();
            l.trust_level = Tl::L2;
            let dyn_store: &dyn LearningStore = &store;
            dyn_store.insert(l).unwrap();

            let results = dyn_store
                .query_relevant("agent", "git push", Tl::L1, 10)
                .unwrap();
            assert!(!results.is_empty());
            assert_eq!(results[0].id, learning_id);
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn test_trait_query_relevant_without_graph() {
            let store = create_trait_test_store().await;
            let dyn_store: &dyn LearningStore = &store;

            let learning = SharedLearning::new(
                "Rust Error".to_string(),
                "Use cargo clippy for rust errors".to_string(),
                LearningSource::Manual,
                "agent".to_string(),
            )
            .with_keywords(vec!["rust".to_string(), "clippy".to_string()]);
            dyn_store.insert(learning).unwrap();

            let results = dyn_store
                .query_relevant("agent", "rust clippy", Tl::L1, 10)
                .unwrap();
            assert!(!results.is_empty());
        }
    }
}
