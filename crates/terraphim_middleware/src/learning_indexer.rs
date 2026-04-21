//! Learning indexer for knowledge graph integration
//!
//! Converts SharedLearnings to IndexedDocuments and indexes them in the RoleGraph.

use std::collections::HashSet;

use terraphim_rolegraph::RoleGraph;
use terraphim_types::shared_learning::{SharedLearning, TrustLevel};
use terraphim_types::{Document, DocumentType, IndexedDocument};

/// Configuration for learning indexing
#[derive(Debug, Clone)]
pub struct LearningIndexerConfig {
    /// Minimum trust level to index
    pub min_trust_level: TrustLevel,
    /// Boost factor for success_rate when calculating document rank
    pub rank_boost_factor: f64,
    /// Whether to include learning content in the document body
    pub include_content: bool,
}

impl Default for LearningIndexerConfig {
    fn default() -> Self {
        Self {
            min_trust_level: TrustLevel::L2,
            rank_boost_factor: 100.0,
            include_content: true,
        }
    }
}

/// Convert a SharedLearning to an IndexedDocument
pub fn learning_to_indexed_document(
    learning: &SharedLearning,
    config: &LearningIndexerConfig,
) -> IndexedDocument {
    let base_rank = 1u64;
    let quality_boost =
        (learning.quality.success_rate.unwrap_or(0.5) * config.rank_boost_factor) as u64;
    let agent_boost = learning.quality.agent_count as u64 * 10;
    let rank = base_rank + quality_boost + agent_boost;

    let doc = Document {
        id: learning.id.clone(),
        url: String::new(),
        title: learning.title.clone(),
        body: if config.include_content {
            learning.content.clone()
        } else {
            String::new()
        },
        description: None,
        summarization: None,
        stub: None,
        doc_type: DocumentType::Document,
        rank: Some(rank),
        tags: Some(learning.keywords.clone()),
        source_haystack: None,
        synonyms: None,
        route: None,
        priority: None,
    };

    IndexedDocument {
        id: doc.id.clone(),
        matched_edges: Vec::new(),
        rank,
        tags: doc.tags.clone().unwrap_or_default(),
        nodes: Vec::new(), // Will be populated by resolve_keywords
        quality_score: None,
    }
}

/// Index a learning in the RoleGraph
///
/// Resolves learning keywords to node IDs via thesaurus lookup and indexes the document.
pub fn index_learning(
    graph: &mut RoleGraph,
    learning: &SharedLearning,
    config: &LearningIndexerConfig,
) -> Result<IndexedDocument, LearningIndexError> {
    if learning.trust_level < config.min_trust_level {
        return Err(LearningIndexError::TrustLevelTooLow {
            got: learning.trust_level,
            need: config.min_trust_level,
        });
    }

    let mut indexed = learning_to_indexed_document(learning, config);

    // Resolve keywords to node IDs via Aho-Corasick
    for keyword in &learning.keywords {
        let node_ids = graph.find_matching_node_ids(keyword);
        indexed.nodes.extend(node_ids);
    }

    // Deduplicate node IDs
    let unique_nodes: HashSet<_> = indexed.nodes.iter().cloned().collect();
    indexed.nodes = unique_nodes.into_iter().collect();

    graph
        .index_learning_document(indexed.clone())
        .map_err(LearningIndexError::Graph)?;

    Ok(indexed)
}

/// Batch index multiple learnings
pub fn index_learnings(
    graph: &mut RoleGraph,
    learnings: &[SharedLearning],
    config: &LearningIndexerConfig,
) -> Vec<Result<IndexedDocument, LearningIndexError>> {
    learnings
        .iter()
        .map(|learning| index_learning(graph, learning, config))
        .collect()
}

/// Error type for learning indexing
#[derive(Debug, thiserror::Error)]
pub enum LearningIndexError {
    #[error("learning trust level {got:?} below minimum {need:?}")]
    TrustLevelTooLow { got: TrustLevel, need: TrustLevel },

    #[error("graph error: {0}")]
    Graph(terraphim_rolegraph::Error),
}
