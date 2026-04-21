//! Feedback loop for knowledge graph integration
//!
//! Propagates quality metrics between learnings and graph rankings.

use std::future::Future;
use std::pin::Pin;

use terraphim_rolegraph::RoleGraph;
use terraphim_types::shared_learning::StoreError;

/// Trait for stores that can record graph touches on learnings
///
/// This trait is implemented by `SharedLearningStore` in `terraphim_agent`.
/// It is defined here to avoid a circular dependency between
/// `terraphim_middleware` and `terraphim_agent`.
pub trait GraphTouchStore: Send + Sync {
    /// Record that a graph query touched a learning
    fn record_graph_touch<'a>(
        &'a self,
        learning_id: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<(), StoreError>> + Send + 'a>>;
}

/// Configuration for feedback loop
#[derive(Debug, Clone)]
pub struct FeedbackConfig {
    /// How much to boost document rank per successful application
    pub rank_boost_per_success: u64,
    /// How much to penalise rank per failed application
    pub rank_penalty_per_failure: u64,
    /// Whether to update learning metrics on graph query
    pub update_on_query: bool,
}

impl Default for FeedbackConfig {
    fn default() -> Self {
        Self {
            rank_boost_per_success: 10,
            rank_penalty_per_failure: 5,
            update_on_query: true,
        }
    }
}

/// Record that a learning was applied and update graph ranks
///
/// Increments learning quality metrics and boosts/penalises linked document ranks.
pub async fn record_learning_application(
    graph: &mut RoleGraph,
    _store: &dyn GraphTouchStore,
    learning_id: &str,
    effective: bool,
    config: &FeedbackConfig,
) -> Result<(), FeedbackError> {
    // Update learning quality via store
    // Note: We assume the store's record_application is called externally
    // Here we just update the graph rank

    // Find the document in the graph
    let adjustment = if effective {
        config.rank_boost_per_success
    } else {
        config.rank_penalty_per_failure
    };

    update_document_rank(graph, learning_id, adjustment)?;

    Ok(())
}

/// Update the rank of a document in the graph
fn update_document_rank(
    _graph: &mut RoleGraph,
    document_id: &str,
    adjustment: u64,
) -> Result<(), FeedbackError> {
    // Access the documents field directly
    // Note: RoleGraph.documents is private, so we need to use public methods
    // Since index_learning_document just inserts, we can't easily update
    // For now, this is a no-op that would need RoleGraph to expose document mutation

    // TODO: Requires RoleGraph to expose document mutation API
    // This is a placeholder that logs the intent
    log::info!(
        "Would adjust rank of document {} by {}",
        document_id,
        adjustment
    );

    Ok(())
}

/// Record that a graph query touched nodes linked to learnings
///
/// Increments applied_count for linked learnings.
pub async fn record_graph_query(
    graph: &RoleGraph,
    store: &dyn GraphTouchStore,
    query: &str,
    config: &FeedbackConfig,
) -> Result<(), FeedbackError> {
    if !config.update_on_query {
        return Ok(());
    }

    let learning_docs = graph.get_learning_documents(query);

    for doc in learning_docs {
        // The document ID is the learning ID
        store
            .record_graph_touch(&doc.id)
            .await
            .map_err(FeedbackError::Store)?;
    }

    Ok(())
}

/// Error type for feedback loop
#[derive(Debug, thiserror::Error)]
pub enum FeedbackError {
    #[error("graph error: {0}")]
    Graph(#[from] terraphim_rolegraph::Error),

    #[error("store error: {0}")]
    Store(#[from] StoreError),
}
