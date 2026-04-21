//! Query merging for knowledge graph integration
//!
//! Merges learning documents with regular graph query results.

use terraphim_rolegraph::RoleGraph;
use terraphim_types::{IndexedDocument, SearchQuery};

/// Query the graph including learning documents
///
/// Returns all documents (regular + learning) whose nodes match the query,
/// sorted by rank.
pub fn query_with_learnings(
    graph: &RoleGraph,
    query: &SearchQuery,
    include_learnings: bool,
) -> Vec<IndexedDocument> {
    let query_text = query.search_term.as_str();
    let mut merged = std::collections::HashMap::new();

    if let Ok(results) = graph.query_graph(query_text, query.skip, query.limit) {
        for (id, doc) in results {
            merged.insert(id, doc);
        }
    }

    if include_learnings {
        for doc in graph.get_learning_documents(query_text) {
            merged
                .entry(doc.id.clone())
                .and_modify(|existing: &mut IndexedDocument| {
                    if doc.rank > existing.rank {
                        *existing = doc.clone();
                    }
                })
                .or_insert_with(|| doc.clone());
        }
    }

    let mut results: Vec<_> = merged.into_values().collect();
    // Sort by rank descending
    results.sort_by(|a, b| b.rank.cmp(&a.rank));

    results
}
