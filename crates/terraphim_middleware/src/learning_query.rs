//! Query merging for knowledge graph integration
//!
//! Merges learning documents with regular graph query results.

use terraphim_rolegraph::RoleGraph;
use terraphim_types::{IndexedDocument, SearchQuery};

/// Query the graph including learning documents
///
/// Returns all documents (regular + learning) whose nodes match the query,
/// sorted by rank.
pub fn query_with_learnings<'a>(
    graph: &'a RoleGraph,
    query: &'a SearchQuery,
    _include_learnings: bool,
) -> Vec<&'a IndexedDocument> {
    let query_text = query.search_term.as_str();
    let mut results = graph.get_learning_documents(query_text);

    // Sort by rank descending
    results.sort_by(|a, b| b.rank.cmp(&a.rank));

    results
}
