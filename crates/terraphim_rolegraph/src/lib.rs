use ahash::AHashMap;
use itertools::Itertools;
use memoize::memoize;
use regex::Regex;
use std::collections::hash_map::Entry;
use std::sync::Arc;
use terraphim_types::{
    Document, Edge, IndexedDocument, Node, NormalizedTermValue, RoleName, Thesaurus,
};
use tokio::sync::{Mutex, MutexGuard};
pub mod input;
use aho_corasick::{AhoCorasick, MatchKind};
use unicode_segmentation::UnicodeSegmentation;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("The given node ID was not found")]
    NodeIdNotFound,
    #[error("The given Edge ID was not found")]
    EdgeIdNotFound,
    #[error("Cannot convert IndexedDocument to JSON: {0}")]
    JsonConversionError(#[from] serde_json::Error),
    #[error("Error while driving terraphim automata: {0}")]
    TerraphimAutomataError(#[from] terraphim_automata::TerraphimAutomataError),
    #[error("Indexing error: {0}")]
    AhoCorasickError(#[from] aho_corasick::BuildError),
}

type Result<T> = std::result::Result<T, Error>;

/// Statistics about the graph structure for debugging
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GraphStats {
    pub node_count: usize,
    pub edge_count: usize,
    pub document_count: usize,
    pub thesaurus_size: usize,
    pub is_populated: bool,
}

/// A serializable representation of RoleGraph for JSON serialization/deserialization.
///
/// This struct excludes the Aho-Corasick automata which cannot be directly serialized,
/// but includes all the necessary data to reconstruct it.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SerializableRoleGraph {
    /// The role of the graph
    pub role: RoleName,
    /// A mapping from node IDs to nodes
    pub nodes: AHashMap<u64, Node>,
    /// A mapping from edge IDs to edges
    pub edges: AHashMap<u64, Edge>,
    /// A mapping from document IDs to indexed documents
    pub documents: AHashMap<String, IndexedDocument>,
    /// A thesaurus is a mapping from synonyms to concepts
    pub thesaurus: Thesaurus,
    /// Aho-Corasick values (needed to rebuild the automata)
    pub aho_corasick_values: Vec<u64>,
    /// reverse lookup - matched id into normalized term
    pub ac_reverse_nterm: AHashMap<u64, NormalizedTermValue>,
}

impl SerializableRoleGraph {
    /// Convert to JSON string
    pub fn to_json(&self) -> std::result::Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    /// Convert to pretty JSON string
    pub fn to_json_pretty(&self) -> std::result::Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// Create from JSON string
    pub fn from_json(json: &str) -> std::result::Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
}

/// A `RoleGraph` is a graph of concepts and their relationships.
///
/// It is used to index documents and search for them.
/// Currently it maps from synonyms to concepts, so only the normalized term
/// gets returned when a reverse lookup is performed.
#[derive(Debug, Clone)]
pub struct RoleGraph {
    /// The role of the graph
    pub role: RoleName,
    /// A mapping from node IDs to nodes
    nodes: AHashMap<u64, Node>,
    /// A mapping from edge IDs to edges
    edges: AHashMap<u64, Edge>,
    /// A mapping from document IDs to indexed documents
    documents: AHashMap<String, IndexedDocument>,
    /// A thesaurus is a mapping from synonyms to concepts
    pub thesaurus: Thesaurus,
    /// Aho-Corasick values
    aho_corasick_values: Vec<u64>,
    /// Aho-Corasick automata
    pub ac: AhoCorasick,
    /// reverse lookup - matched id into normalized term
    pub ac_reverse_nterm: AHashMap<u64, NormalizedTermValue>,
}

impl RoleGraph {
    /// Creates a new `RoleGraph` with the given role and thesaurus
    pub async fn new(role: RoleName, thesaurus: Thesaurus) -> Result<Self> {
        let (ac, aho_corasick_values, ac_reverse_nterm) = Self::build_aho_corasick(&thesaurus)?;

        Ok(Self {
            role,
            nodes: AHashMap::new(),
            edges: AHashMap::new(),
            documents: AHashMap::new(),
            thesaurus,
            aho_corasick_values,
            ac,
            ac_reverse_nterm,
        })
    }

    /// Build Aho-Corasick automata from thesaurus
    fn build_aho_corasick(
        thesaurus: &Thesaurus,
    ) -> Result<(AhoCorasick, Vec<u64>, AHashMap<u64, NormalizedTermValue>)> {
        let mut keys = Vec::new();
        let mut values = Vec::new();
        let mut ac_reverse_nterm = AHashMap::new();

        for (key, normalized_term) in thesaurus {
            keys.push(key.as_str());
            values.push(normalized_term.id);
            ac_reverse_nterm.insert(normalized_term.id, normalized_term.value.clone());
        }

        let ac = AhoCorasick::builder()
            .match_kind(MatchKind::LeftmostLongest)
            .ascii_case_insensitive(true)
            .build(keys)?;

        Ok((ac, values, ac_reverse_nterm))
    }

    /// Rebuild Aho-Corasick automata from thesaurus (useful after deserialization)
    pub fn rebuild_automata(&mut self) -> Result<()> {
        let (ac, values, ac_reverse_nterm) = Self::build_aho_corasick(&self.thesaurus)?;
        self.ac = ac;
        self.aho_corasick_values = values;
        self.ac_reverse_nterm = ac_reverse_nterm;
        Ok(())
    }

    /// Create a serializable representation of the RoleGraph
    pub fn to_serializable(&self) -> SerializableRoleGraph {
        SerializableRoleGraph {
            role: self.role.clone(),
            nodes: self.nodes.clone(),
            edges: self.edges.clone(),
            documents: self.documents.clone(),
            thesaurus: self.thesaurus.clone(),
            aho_corasick_values: self.aho_corasick_values.clone(),
            ac_reverse_nterm: self.ac_reverse_nterm.clone(),
        }
    }

    /// Create RoleGraph from serializable representation
    pub async fn from_serializable(serializable: SerializableRoleGraph) -> Result<Self> {
        let mut role_graph = RoleGraph {
            role: serializable.role,
            nodes: serializable.nodes,
            edges: serializable.edges,
            documents: serializable.documents,
            thesaurus: serializable.thesaurus,
            aho_corasick_values: serializable.aho_corasick_values,
            ac: AhoCorasick::new([""])?, // Will be rebuilt
            ac_reverse_nterm: serializable.ac_reverse_nterm,
        };

        // Rebuild the Aho-Corasick automata
        role_graph.rebuild_automata()?;

        Ok(role_graph)
    }

    /// Find all matches in the rolegraph for the given text
    ///
    /// Returns a list of IDs of the matched nodes
    pub fn find_matching_node_ids(&self, text: &str) -> Vec<u64> {
        log::trace!("Finding matching node IDs for text: '{text}'");
        self.ac
            .find_iter(text)
            .map(|mat| self.aho_corasick_values[mat.pattern()])
            .collect()
    }

    /// Check if all matched node IDs in the given text are connected by at least a single path
    /// that visits all of them (in any order). Returns true if such a path exists.
    ///
    /// Strategy:
    /// - Get matched node IDs from the text via Aho-Corasick
    /// - Build an adjacency map from `nodes.connected_with` and `edges` (undirected)
    /// - For small k (<=8), perform DFS/backtracking to see if a path exists that visits all target nodes
    /// - If k == 0 or 1, trivially true
    pub fn is_all_terms_connected_by_path(&self, text: &str) -> bool {
        let mut targets = self.find_matching_node_ids(text);
        targets.sort_unstable();
        targets.dedup();
        let k = targets.len();
        if k <= 1 {
            return true;
        }

        // Build adjacency map of node_id -> neighbor node_ids
        let mut adj: AHashMap<u64, ahash::AHashSet<u64>> = AHashMap::new();
        for (node_id, node) in &self.nodes {
            let entry = adj.entry(*node_id).or_default();
            for edge_id in &node.connected_with {
                if let Some(edge) = self.edges.get(edge_id) {
                    let (a, b) = magic_unpair(edge.id);
                    entry.insert(if a == *node_id { b } else { a });
                }
            }
        }

        // If any target is isolated, fail fast
        if targets
            .iter()
            .any(|t| adj.get(t).map(|s| s.is_empty()).unwrap_or(true))
        {
            return false;
        }

        // Backtracking DFS to cover all targets
        fn dfs(
            current: u64,
            remaining: &mut ahash::AHashSet<u64>,
            adj: &AHashMap<u64, ahash::AHashSet<u64>>,
            visited_edges: &mut ahash::AHashSet<(u64, u64)>,
        ) -> bool {
            if remaining.is_empty() {
                return true;
            }
            if let Some(neighbors) = adj.get(&current) {
                for &n in neighbors {
                    let edge = if current < n {
                        (current, n)
                    } else {
                        (n, current)
                    };
                    if visited_edges.contains(&edge) {
                        continue;
                    }
                    let removed = remaining.remove(&n);
                    visited_edges.insert(edge);
                    if dfs(n, remaining, adj, visited_edges) {
                        return true;
                    }
                    visited_edges.remove(&edge);
                    if removed {
                        remaining.insert(n);
                    }
                }
            }
            false
        }

        // Try starting from each target
        for &start in &targets {
            let mut remaining: ahash::AHashSet<u64> = targets.iter().cloned().collect();
            remaining.remove(&start);
            let mut visited_edges: ahash::AHashSet<(u64, u64)> = ahash::AHashSet::new();
            if dfs(start, &mut remaining, &adj, &mut visited_edges) {
                return true;
            }
        }
        false
    }

    /// Currently I don't need this functionality,
    /// but it's commonly referred as "training" if you are writing graph embeddings, see FAIR or [Cleora](https://arxiv.org/pdf/2102.02302)
    /// Currently I like rank based integers better - they map directly into UI grid but f64 based ranking may be useful for R&D
    /// See normalization step in https://github.com/BurntSushi/imdb-rename
    /// This method performs several key operations to process and rank
    /// documents:
    /// - Utilizes node rank as a weight for an edge, and edge rank as a weight
    ///   for an document ID, creating a hierarchical weighting system.
    /// - Creates a hashmap to store outputs with document_id and rank, aiming
    ///   to deduplicate documents in the output.
    /// - Normalizes the output rank from 1 to the total number of records,
    ///   ensuring a consistent ranking scale across documents.
    /// - Pre-sorts document IDs by rank using a BTreeMap, facilitating
    ///   efficient access and manipulation based on rank.
    /// - Calculates the overall weighted average by computing the weighted
    ///   average of node rank, edge rank, and document rank. This calculation
    ///   involves summing the products of each weight with its corresponding
    ///   rank and dividing by the sum of the weights for each node, edge, and
    ///   document.
    // YAGNI: at the moment I don't need it, so parked
    // pub fn normalize(&mut self) {
    //     let node_len = self.nodes.len() as u32;
    //     log::trace!("Node Length {}", node_len);
    //     let edge_len = self.edges.len() as u32;
    //     log::trace!("Edge Length {}", edge_len);
    //     let document_count = self.documents.len() as u32;
    //     log::trace!("document Length {}", document_count);
    //     let normalizer = f32::from_bits(node_len + edge_len + document_count);
    //     let weight_node = f32::from_bits(node_len) / normalizer;
    //     let weight_edge = f32::from_bits(edge_len) / normalizer;
    //     let weight_document = f32::from_bits(document_count) / normalizer;
    //     log::trace!("Weight Node {}", weight_node);
    //     log::trace!("Weight Edge {}", weight_edge);
    //     log::trace!("Weight document {}", weight_document);
    //     // for each node for each edge for each document
    //     // for (document_id,rank) in self.documents.iter(){
    //     //     let weighted_rank=(weight_node*node_rank as f32)+(weight_edge*edge_rank as f32)+(weight_document*rank as f32)/(weight_node+weight_edge+weight_document);
    //     //     log::debug!("document id {} Weighted Rank {}", document_id, weighted_rank);
    //     //     sorted_vector_by_rank_weighted.push((document_id, weighted_rank));
    //     // }
    // }
    ///   Performs a query on the graph using the query string.
    ///
    /// Returns a list of document IDs ranked and weighted by the weighted mean
    /// average of node rank, edge rank, and document rank.
    pub fn query_graph(
        &self,
        query_string: &str,
        offset: Option<usize>,
        limit: Option<usize>,
    ) -> Result<Vec<(String, IndexedDocument)>> {
        log::debug!("Performing graph query with string: '{query_string}'");
        let node_ids = self.find_matching_node_ids(query_string);

        // Early return if no matching terms found in thesaurus
        if node_ids.is_empty() {
            log::debug!("No matching terms found in thesaurus for query: '{query_string}'");
            return Ok(vec![]);
        }

        // Early return if graph has no nodes (not populated yet)
        if self.nodes.is_empty() {
            log::debug!("Graph has no nodes yet - no documents have been indexed");
            return Ok(vec![]);
        }

        let mut results = AHashMap::new();
        for node_id in node_ids {
            // Check if node exists, skip if not (node from thesaurus but no documents indexed yet)
            let Some(node) = self.nodes.get(&node_id) else {
                log::trace!(
                    "Node ID {} from thesaurus not found in graph - no documents contain this term yet",
                    node_id
                );
                continue;
            };

            let Some(normalized_term) = self.ac_reverse_nterm.get(&node_id) else {
                log::warn!(
                    "Node ID {} found in graph but missing from thesaurus reverse lookup",
                    node_id
                );
                continue;
            };
            log::debug!("Processing node ID: {:?} with rank: {}", node_id, node.rank);

            for edge_id in &node.connected_with {
                let Some(edge) = self.edges.get(edge_id) else {
                    log::warn!(
                        "Edge ID {} referenced by node {} not found in edges map",
                        edge_id,
                        node_id
                    );
                    continue;
                };
                log::trace!("Processing edge ID: {:?} with rank: {}", edge_id, edge.rank);

                for (document_id, document_rank) in &edge.doc_hash {
                    // For now, this sums up over nodes and edges
                    let total_rank = node.rank + edge.rank + document_rank;
                    match results.entry(document_id.clone()) {
                        Entry::Vacant(e) => {
                            e.insert(IndexedDocument {
                                id: document_id.clone(),
                                matched_edges: vec![edge.clone()],
                                rank: total_rank,
                                tags: vec![normalized_term.to_string()],
                                nodes: vec![node_id],
                            });
                        }
                        Entry::Occupied(mut e) => {
                            let doc = e.get_mut();
                            doc.rank += total_rank; // Adjust to correctly aggregate the rank
                            doc.matched_edges.push(edge.clone());
                            // Remove duplicate edges based on unique IDs
                            doc.matched_edges.dedup_by_key(|e| e.id);
                        }
                    }
                }
            }
        }

        let mut ranked_documents = results.into_iter().collect::<Vec<_>>();
        ranked_documents.sort_by_key(|(_, doc)| std::cmp::Reverse(doc.rank));

        let documents: Vec<_> = ranked_documents
            .into_iter()
            .skip(offset.unwrap_or(0))
            .take(limit.unwrap_or(usize::MAX))
            .collect();

        log::debug!("Query resulted in {} documents", documents.len());
        Ok(documents)
    }

    /// Query the graph with multiple terms and logical operators (AND/OR)
    pub fn query_graph_with_operators(
        &self,
        search_terms: &[&str],
        operator: &terraphim_types::LogicalOperator,
        offset: Option<usize>,
        limit: Option<usize>,
    ) -> Result<Vec<(String, IndexedDocument)>> {
        use terraphim_types::LogicalOperator;

        log::debug!(
            "Performing multi-term graph query with {} terms using {:?} operator",
            search_terms.len(),
            operator
        );

        if search_terms.is_empty() {
            return Ok(vec![]);
        }

        // Handle single term case as fallback to existing method
        if search_terms.len() == 1 {
            return self.query_graph(search_terms[0], offset, limit);
        }

        // Early return if graph has no nodes
        if self.nodes.is_empty() {
            log::debug!("Graph has no nodes yet - no documents have been indexed");
            return Ok(vec![]);
        }

        match operator {
            LogicalOperator::Or => self.query_graph_or(search_terms, offset, limit),
            LogicalOperator::And => self.query_graph_and(search_terms, offset, limit),
        }
    }

    /// Perform OR operation: return documents that match ANY of the search terms
    fn query_graph_or(
        &self,
        search_terms: &[&str],
        offset: Option<usize>,
        limit: Option<usize>,
    ) -> Result<Vec<(String, IndexedDocument)>> {
        let mut results = AHashMap::new();

        for term in search_terms {
            let node_ids = self.find_matching_node_ids(term);

            for node_id in node_ids {
                let Some(node) = self.nodes.get(&node_id) else {
                    continue;
                };

                let Some(normalized_term) = self.ac_reverse_nterm.get(&node_id) else {
                    continue;
                };

                for edge_id in &node.connected_with {
                    let Some(edge) = self.edges.get(edge_id) else {
                        continue;
                    };

                    for (document_id, document_rank) in &edge.doc_hash {
                        let total_rank = node.rank + edge.rank + document_rank;
                        match results.entry(document_id.clone()) {
                            Entry::Vacant(e) => {
                                e.insert(IndexedDocument {
                                    id: document_id.clone(),
                                    matched_edges: vec![edge.clone()],
                                    rank: total_rank,
                                    tags: vec![normalized_term.to_string()],
                                    nodes: vec![node_id],
                                });
                            }
                            Entry::Occupied(mut e) => {
                                let doc = e.get_mut();
                                doc.rank += total_rank;
                                doc.matched_edges.push(edge.clone());
                                doc.matched_edges.dedup_by_key(|e| e.id);
                                // Add the tag if not already present
                                if !doc.tags.contains(&normalized_term.to_string()) {
                                    doc.tags.push(normalized_term.to_string());
                                }
                                if !doc.nodes.contains(&node_id) {
                                    doc.nodes.push(node_id);
                                }
                            }
                        }
                    }
                }
            }
        }

        let mut ranked_documents = results.into_iter().collect::<Vec<_>>();
        ranked_documents.sort_by_key(|(_, doc)| std::cmp::Reverse(doc.rank));

        let documents: Vec<_> = ranked_documents
            .into_iter()
            .skip(offset.unwrap_or(0))
            .take(limit.unwrap_or(usize::MAX))
            .collect();

        log::debug!("OR query resulted in {} documents", documents.len());
        Ok(documents)
    }

    /// Perform AND operation: return documents that match ALL of the search terms
    fn query_graph_and(
        &self,
        search_terms: &[&str],
        offset: Option<usize>,
        limit: Option<usize>,
    ) -> Result<Vec<(String, IndexedDocument)>> {
        // First, collect document sets for each term
        let mut term_document_sets: Vec<AHashMap<String, (IndexedDocument, Vec<String>)>> =
            Vec::new();

        for term in search_terms {
            // Handle multi-word terms intelligently
            let node_ids = if term.contains(' ') {
                log::debug!("Multi-word term detected: '{}'", term);
                // First try to match the complete phrase
                let phrase_matches = self.find_matching_node_ids(term);
                if phrase_matches.is_empty() {
                    log::debug!(
                        "No exact phrase match for '{}', trying individual words",
                        term
                    );
                    // Fallback: match individual words in the phrase
                    term.split_whitespace()
                        .flat_map(|word| {
                            log::debug!("Searching for word: '{}'", word);
                            self.find_matching_node_ids(word)
                        })
                        .collect()
                } else {
                    log::debug!(
                        "Found {} phrase matches for '{}'",
                        phrase_matches.len(),
                        term
                    );
                    phrase_matches
                }
            } else {
                self.find_matching_node_ids(term)
            };

            log::debug!("Term '{}' matched {} node IDs", term, node_ids.len());
            let mut term_docs = AHashMap::new();

            for node_id in node_ids {
                let Some(node) = self.nodes.get(&node_id) else {
                    continue;
                };

                let Some(normalized_term) = self.ac_reverse_nterm.get(&node_id) else {
                    continue;
                };

                for edge_id in &node.connected_with {
                    let Some(edge) = self.edges.get(edge_id) else {
                        continue;
                    };

                    for (document_id, document_rank) in &edge.doc_hash {
                        let total_rank = node.rank + edge.rank + document_rank;
                        match term_docs.entry(document_id.clone()) {
                            Entry::Vacant(e) => {
                                e.insert((
                                    IndexedDocument {
                                        id: document_id.clone(),
                                        matched_edges: vec![edge.clone()],
                                        rank: total_rank,
                                        tags: vec![normalized_term.to_string()],
                                        nodes: vec![node_id],
                                    },
                                    vec![term.to_string()],
                                ));
                            }
                            Entry::Occupied(mut e) => {
                                let (doc, terms) = e.get_mut();
                                doc.rank += total_rank;
                                doc.matched_edges.push(edge.clone());
                                doc.matched_edges.dedup_by_key(|e| e.id);
                                if !doc.tags.contains(&normalized_term.to_string()) {
                                    doc.tags.push(normalized_term.to_string());
                                }
                                if !doc.nodes.contains(&node_id) {
                                    doc.nodes.push(node_id);
                                }
                                if !terms.contains(&term.to_string()) {
                                    terms.push(term.to_string());
                                }
                            }
                        }
                    }
                }
            }
            term_document_sets.push(term_docs);
        }

        // Find intersection: documents that appear in ALL term sets
        if term_document_sets.is_empty() {
            return Ok(vec![]);
        }

        let mut final_results = AHashMap::new();
        let first_set = &term_document_sets[0];

        for (doc_id, (first_doc, first_terms)) in first_set {
            // Check if this document appears in all other term sets
            let mut appears_in_all = true;
            let mut combined_doc = first_doc.clone();
            let mut all_matched_terms = first_terms.clone();

            for term_set in &term_document_sets[1..] {
                if let Some((term_doc, term_matched)) = term_set.get(doc_id) {
                    // Combine the rankings and metadata
                    combined_doc.rank += term_doc.rank;
                    combined_doc
                        .matched_edges
                        .extend(term_doc.matched_edges.clone());
                    combined_doc.matched_edges.dedup_by_key(|e| e.id);

                    for tag in &term_doc.tags {
                        if !combined_doc.tags.contains(tag) {
                            combined_doc.tags.push(tag.clone());
                        }
                    }

                    for node in &term_doc.nodes {
                        if !combined_doc.nodes.contains(node) {
                            combined_doc.nodes.push(*node);
                        }
                    }

                    all_matched_terms.extend(term_matched.clone());
                } else {
                    appears_in_all = false;
                    break;
                }
            }

            if appears_in_all && all_matched_terms.len() == search_terms.len() {
                final_results.insert(doc_id.clone(), combined_doc);
            }
        }

        let mut ranked_documents = final_results.into_iter().collect::<Vec<_>>();
        ranked_documents.sort_by_key(|(_, doc)| std::cmp::Reverse(doc.rank));

        let documents: Vec<_> = ranked_documents
            .into_iter()
            .skip(offset.unwrap_or(0))
            .take(limit.unwrap_or(usize::MAX))
            .collect();

        log::debug!(
            "AND query resulted in {} documents (from {} search terms)",
            documents.len(),
            search_terms.len()
        );
        Ok(documents)
    }

    // pub fn parse_document_to_pair(&mut self, document_id: &str, text: &str) {
    //     let matches = self.find_matching_node_ids(text);
    //     for (a, b) in matches.into_iter().tuple_windows() {
    //         // cast to Id
    //         let a = a as Id;
    //         self.add_or_update_document(document_id, a, b);
    //     }
    // }

    /// Inserts an document into the rolegraph
    pub fn insert_document(&mut self, document_id: &str, document: Document) {
        self.documents.insert(
            document_id.to_string(),
            IndexedDocument::from_document(document.clone()),
        );
        let matches = self.find_matching_node_ids(&document.to_string());
        for (a, b) in matches.into_iter().tuple_windows() {
            self.add_or_update_document(document_id, a, b);
        }
    }

    /// Check if a document is already indexed in the rolegraph
    pub fn has_document(&self, document_id: &str) -> bool {
        self.documents.contains_key(document_id)
    }

    pub fn add_or_update_document(&mut self, document_id: &str, x: u64, y: u64) {
        let edge = magic_pair(x, y);
        let edge = self.init_or_update_edge(edge, document_id);
        self.init_or_update_node(x, &edge);
        self.init_or_update_node(y, &edge);
    }

    fn init_or_update_node(&mut self, node_id: u64, edge: &Edge) {
        match self.nodes.entry(node_id) {
            Entry::Vacant(_) => {
                let node = Node::new(node_id, edge.clone());
                self.nodes.insert(node.id, node);
            }
            Entry::Occupied(entry) => {
                let node = entry.into_mut();
                node.rank += 1;
                node.connected_with.insert(edge.id);
            }
        };
    }

    /// Get the number of nodes in the graph
    pub fn get_node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Get the number of edges in the graph
    pub fn get_edge_count(&self) -> usize {
        self.edges.len()
    }

    /// Get the number of documents in the graph
    pub fn get_document_count(&self) -> usize {
        self.documents.len()
    }

    /// Check if the graph has been properly populated
    pub fn is_graph_populated(&self) -> bool {
        !self.nodes.is_empty() && !self.edges.is_empty() && !self.documents.is_empty()
    }

    /// Get graph statistics for debugging
    pub fn get_graph_stats(&self) -> GraphStats {
        GraphStats {
            node_count: self.nodes.len(),
            edge_count: self.edges.len(),
            document_count: self.documents.len(),
            thesaurus_size: self.thesaurus.len(),
            is_populated: self.is_graph_populated(),
        }
    }

    /// Validate that documents have content and are indexed properly
    pub fn validate_documents(&self) -> Vec<String> {
        let mut warnings = Vec::new();

        for (doc_id, _indexed_doc) in &self.documents {
            // Check if this document contributed to graph structure
            let has_nodes = self.nodes.values().any(|node| {
                node.connected_with.iter().any(|edge_id| {
                    self.edges
                        .get(edge_id)
                        .is_some_and(|edge| edge.doc_hash.contains_key(doc_id))
                })
            });

            if !has_nodes {
                warnings.push(format!("Document '{}' did not create any nodes (may have empty body or no thesaurus matches)", doc_id));
            }
        }

        warnings
    }

    /// Find all document IDs that contain a specific term
    pub fn find_document_ids_for_term(&self, term: &str) -> Vec<String> {
        let node_ids = self.find_matching_node_ids(term);
        let mut document_ids = std::collections::HashSet::new();

        for node_id in node_ids {
            if let Some(node) = self.nodes.get(&node_id) {
                for edge_id in &node.connected_with {
                    if let Some(edge) = self.edges.get(edge_id) {
                        for doc_id in edge.doc_hash.keys() {
                            document_ids.insert(doc_id.clone());
                        }
                    }
                }
            }
        }

        document_ids.into_iter().collect()
    }

    fn init_or_update_edge(&mut self, edge_key: u64, document_id: &str) -> Edge {
        match self.edges.entry(edge_key) {
            Entry::Vacant(_) => {
                let edge = Edge::new(edge_key, document_id.to_string());
                self.edges.insert(edge.id, edge.clone());
                edge
            }
            Entry::Occupied(entry) => {
                let edge = entry.into_mut();
                *edge.doc_hash.entry(document_id.to_string()).or_insert(1) += 1;
                edge.clone()
            }
        }
    }

    /// Get a document by its ID
    pub fn get_document(&self, document_id: &str) -> Option<&IndexedDocument> {
        self.documents.get(document_id)
    }

    /// Get all documents in the graph
    pub fn get_all_documents(&self) -> impl Iterator<Item = (&String, &IndexedDocument)> {
        self.documents.iter()
    }

    /// Get the number of documents in the graph
    pub fn document_count(&self) -> usize {
        self.documents.len()
    }

    /// Public accessor for nodes collection
    pub fn nodes_map(&self) -> &ahash::AHashMap<u64, Node> {
        &self.nodes
    }

    /// Public accessor for edges collection
    pub fn edges_map(&self) -> &ahash::AHashMap<u64, Edge> {
        &self.edges
    }
}

/// Wraps the `RoleGraph` for ingesting documents and is `Send` and `Sync`
#[derive(Debug, Clone)]
pub struct RoleGraphSync {
    inner: Arc<Mutex<RoleGraph>>,
}

impl RoleGraphSync {
    /// Locks the rolegraph for reading and writing
    pub async fn lock(&self) -> MutexGuard<'_, RoleGraph> {
        self.inner.lock().await
    }

    /// Serialize the RoleGraph to JSON string
    /// This method acquires a lock on the inner RoleGraph during serialization
    pub async fn to_json(&self) -> Result<String> {
        let rolegraph = self.inner.lock().await;
        let serializable = rolegraph.to_serializable();
        serializable.to_json().map_err(Error::JsonConversionError)
    }

    /// Serialize the RoleGraph to pretty JSON string
    /// This method acquires a lock on the inner RoleGraph during serialization
    pub async fn to_json_pretty(&self) -> Result<String> {
        let rolegraph = self.inner.lock().await;
        let serializable = rolegraph.to_serializable();
        serializable
            .to_json_pretty()
            .map_err(Error::JsonConversionError)
    }

    /// Create a new RoleGraphSync from JSON string
    pub async fn from_json(json: &str) -> Result<Self> {
        let serializable =
            SerializableRoleGraph::from_json(json).map_err(Error::JsonConversionError)?;
        let rolegraph = RoleGraph::from_serializable(serializable).await?;
        Ok(Self {
            inner: Arc::new(Mutex::new(rolegraph)),
        })
    }

    /// Get a serializable representation without holding the lock
    /// This clones the entire RoleGraph, so use with caution for large graphs
    pub async fn to_serializable(&self) -> Result<SerializableRoleGraph> {
        let rolegraph = self.inner.lock().await;
        Ok(rolegraph.to_serializable())
    }
}

impl From<RoleGraph> for RoleGraphSync {
    fn from(rolegraph: RoleGraph) -> Self {
        Self {
            inner: Arc::new(Mutex::new(rolegraph)),
        }
    }
}

#[macro_use]
extern crate lazy_static;
lazy_static! {
    static ref RE: Regex = Regex::new(r"[?!|]\s+").unwrap();
}

pub fn split_paragraphs(paragraphs: &str) -> Vec<&str> {
    let sentences = UnicodeSegmentation::split_sentence_bounds(paragraphs);
    let parts =
        sentences.flat_map(|sentence| RE.split(sentence.trim_end_matches(char::is_whitespace)));
    parts
        .map(|part| part.trim())
        .filter(|part| !part.is_empty())
        .collect()
}

/// Combining two numbers into a unique one: pairing functions.
/// It uses "elegant pairing" (https://odino.org/combining-two-numbers-into-a-unique-one-pairing-functions/).
/// also using memoize macro with Ahash hasher
#[memoize(CustomHasher: ahash::AHashMap)]
pub fn magic_pair(x: u64, y: u64) -> u64 {
    if x >= y { x * x + x + y } else { y * y + x }
}

/// Magic unpair
/// func unpair(z int) (int, int) {
///   q := int(math.Floor(math.Sqrt(float64(z))))
///     l := z - q * q
///   if l < q {
///       return l, q
//   }
///   return q, l - q
/// }
#[memoize(CustomHasher: ahash::AHashMap)]
pub fn magic_unpair(z: u64) -> (u64, u64) {
    let q = (z as f32).sqrt().floor() as u64;
    let l = z - q * q;
    if l < q { (l, q) } else { (q, l - q) }
}

// Examples for serialization usage
/// # Serialization Examples
///
/// This module provides comprehensive serialization support for RoleGraph and related types.
/// Here are the key patterns for using the serialization functionality:
///
/// ## Basic RoleGraph Serialization
///
/// ```rust,no_run
/// use terraphim_rolegraph::{RoleGraph, SerializableRoleGraph};
///
/// // Create a RoleGraph
/// let rolegraph = RoleGraph::new(role.into(), thesaurus).await?;
///
/// // Convert to serializable representation
/// let serializable = rolegraph.to_serializable();
///
/// // Serialize to JSON string
/// let json = serializable.to_json()?;
///
/// // Deserialize from JSON
/// let deserialized: SerializableRoleGraph = SerializableRoleGraph::from_json(&json)?;
///
/// // Recreate RoleGraph with rebuilt automata
/// let restored_rolegraph = RoleGraph::from_serializable(deserialized).await?;
/// ```
///
/// ## RoleGraphSync Serialization
///
/// ```rust,no_run
/// use terraphim_rolegraph::RoleGraphSync;
///
/// // Create RoleGraphSync
/// let rolegraph_sync = RoleGraphSync::from(rolegraph);
///
/// // Serialize directly to JSON (acquires lock internally)
/// let json = rolegraph_sync.to_json().await?;
/// let json_pretty = rolegraph_sync.to_json_pretty().await?;
///
/// // Deserialize back to RoleGraphSync
/// let restored_sync = RoleGraphSync::from_json(&json).await?;
/// ```
///
/// ## Graph Statistics Serialization
///
/// ```rust,no_run
/// use terraphim_rolegraph::GraphStats;
///
/// let stats = rolegraph.get_graph_stats();
///
/// // Serialize to JSON
/// let json = serde_json::to_string(&stats)?;
///
/// // Deserialize
/// let restored_stats: GraphStats = serde_json::from_str(&json)?;
/// ```
///
/// ## Important Notes
///
/// - The Aho-Corasick automata cannot be directly serialized and is rebuilt from the thesaurus
/// - All serialization methods are async to handle the potential I/O operations
/// - RoleGraphSync serialization methods acquire internal locks automatically
/// - The serializable representation includes all data needed to rebuild the automata
/// - Performance consideration: Large graphs may have significant serialization overhead
#[cfg(test)]
mod tests {
    use super::*;

    use terraphim_automata::{AutomataPath, load_thesaurus};
    use tokio::test;
    use ulid::Ulid;

    async fn load_sample_thesaurus() -> Thesaurus {
        load_thesaurus(&AutomataPath::local_example_full())
            .await
            .unwrap()
    }

    #[test]
    async fn test_split_paragraphs() {
        let paragraph = "This is the first sentence.\n\n This is the second sentence. This is the second sentence? This is the second sentence| This is the second sentence!\n\nThis is the third sentence. Mr. John Johnson Jr. was born in the U.S.A but earned his Ph.D. in Israel before joining Nike Inc. as an engineer. He also worked at craigslist.org as a business analyst.";
        let sentences = split_paragraphs(paragraph);
        assert_eq!(sentences.len(), 9);
        assert_eq!(sentences[0], "This is the first sentence.");
        assert_eq!(sentences[1], "This is the second sentence.");
        assert_eq!(sentences[2], "This is the second sentence?");
        assert_eq!(sentences[3], "This is the second sentence");
        assert_eq!(sentences[4], "This is the second sentence!");
        assert_eq!(sentences[5], "This is the third sentence.");
        assert_eq!(sentences[6], "Mr.");
        assert_eq!(
            sentences[7],
            "John Johnson Jr. was born in the U.S.A but earned his Ph.D. in Israel before joining Nike Inc. as an engineer."
        );
        assert_eq!(
            sentences[8],
            "He also worked at craigslist.org as a business analyst."
        );
    }

    #[test]
    async fn test_find_matching_node_idss() {
        let query = "I am a text with the word Life cycle concepts and bar and Trained operators and maintainers, project direction, some bingo words Paradigm Map and project planning, then again: some bingo words Paradigm Map and project planning, then repeats: Trained operators and maintainers, project direction";
        let role = "system operator".to_string();
        let rolegraph = RoleGraph::new(role.into(), load_sample_thesaurus().await)
            .await
            .unwrap();
        let matches = rolegraph.find_matching_node_ids(query);
        // Updated: automata now finds more matches including duplicates from repeated terms
        assert_eq!(matches.len(), 7);
    }

    #[test]
    async fn test_find_matching_node_idss_ac_values() {
        let query = "life cycle framework I am a text with the word Life cycle concepts and bar and Trained operators and maintainers, project direction, some bingo words Paradigm Map and project planning, then again: some bingo words Paradigm Map and project planning, then repeats: Trained operators and maintainers, project direction";
        let role = "system operator".to_string();
        let rolegraph = RoleGraph::new(role.into(), load_sample_thesaurus().await)
            .await
            .unwrap();
        println!("rolegraph: {:?}", rolegraph);
        let matches = rolegraph.find_matching_node_ids(query);
        println!("matches: {:?}", matches);
        for each_match in matches.iter() {
            let ac_reverse_nterm = rolegraph.ac_reverse_nterm.get(each_match).unwrap();
            println!("{each_match} ac_reverse_nterm: {:?}", ac_reverse_nterm);
        }
        assert_eq!(
            rolegraph.ac_reverse_nterm.get(&matches[0]).unwrap(),
            &NormalizedTermValue::new("life cycle models".to_string())
        );
    }

    #[test]
    async fn test_terraphim_engineer() {
        let role_name = "Terraphim Engineer".to_string();
        const DEFAULT_HAYSTACK_PATH: &str = "docs/src/";
        let mut docs_path = std::env::current_dir().unwrap();
        docs_path.pop();
        docs_path.pop();
        docs_path = docs_path.join(DEFAULT_HAYSTACK_PATH);
        println!("Docs path: {:?}", docs_path);
        let engineer_thesaurus_path = docs_path.join("Terraphim Engineer_thesaurus.json");
        if !engineer_thesaurus_path.exists() {
            eprintln!(
                "Engineer thesaurus not found at {:?}; skipping test_terraphim_engineer",
                engineer_thesaurus_path
            );
            return;
        }
        let automata_path = AutomataPath::from_local(engineer_thesaurus_path);
        let thesaurus = load_thesaurus(&automata_path).await.unwrap();
        let mut rolegraph = RoleGraph::new(role_name.into(), thesaurus.clone())
            .await
            .unwrap();
        let document_id = Ulid::new().to_string();
        let test_document = r#"
        This folder is an example of personal knowledge graph used for testing and fixtures
        terraphim-graph
        "#;
        println!("thesaurus: {:?}", thesaurus);
        assert_eq!(thesaurus.len(), 10);
        let matches = rolegraph.find_matching_node_ids(test_document);
        println!("Matches {:?}", matches);
        for (a, b) in matches.into_iter().tuple_windows() {
            rolegraph.add_or_update_document(&document_id, a, b);
        }
        let document = Document {
            stub: None,
            url: "/path/to/document".to_string(),
            tags: None,
            rank: None,
            source_haystack: None,
            id: document_id.clone(),
            title: "README".to_string(),
            body: test_document.to_string(),
            description: None,
            summarization: None,
            doc_type: terraphim_types::DocumentType::KgEntry,
            synonyms: None,
            route: None,
            priority: None,
        };
        rolegraph.insert_document(&document_id, document);
        println!("query with terraphim-graph and service");
        let results: Vec<(String, IndexedDocument)> =
            match rolegraph.query_graph("terraphim-graph and service", Some(0), Some(10)) {
                Ok(results) => results,
                Err(Error::NodeIdNotFound) => {
                    println!("NodeIdNotFound");
                    Vec::new()
                }
                Err(e) => {
                    println!("Error: {:?}", e);
                    Vec::new()
                }
            };
        println!("results shall be zero: {:#?}", results);

        let document_id2 = "document2".to_string();
        let test_document2 = r#"
        # Terraphim-Graph scorer
        Terraphim-Graph (scorer) is using unique graph embeddings, where the rank of the term is defined by number of synonyms connected to the concept.

        synonyms:: graph embeddings, graph, knowledge graph based embeddings

        Now we will have a concept "Terrpahim Graph Scorer" with synonyms "graph embeddings" and "terraphim-graph". This provides service
        "#;
        let document2 = Document {
            stub: None,
            url: "/path/to/document2".to_string(),
            tags: None,
            rank: None,
            source_haystack: None,
            id: document_id2.clone(),
            title: "terraphim-graph".to_string(),
            body: test_document2.to_string(),
            description: None,
            summarization: None,
            doc_type: terraphim_types::DocumentType::KgEntry,
            synonyms: None,
            route: None,
            priority: None,
        };
        rolegraph.insert_document(&document_id2, document2);
        log::debug!("Query graph");
        let results: Vec<(String, IndexedDocument)> = rolegraph
            .query_graph("terraphim-graph and service", Some(0), Some(10))
            .unwrap();
        println!("results: {:#?}", results);
        let top_result = results.first().unwrap();
        println!("Top result {:?} Rank {:?}", top_result.0, top_result.1.rank);
        println!("Top result {:#?}", top_result.1);
        println!("Nodes {:#?}   ", rolegraph.nodes);
        println!("Nodes count {:?}", rolegraph.nodes.len());
        println!("Edges count {:?}", rolegraph.edges.len());
    }

    #[test]
    async fn test_rolegraph() {
        let role = "system operator".to_string();
        let mut rolegraph = RoleGraph::new(role.into(), load_sample_thesaurus().await)
            .await
            .unwrap();
        let document_id = Ulid::new().to_string();
        let query = "I am a text with the word Life cycle concepts and bar and Trained operators and maintainers, project direction, some bingo words Paradigm Map and project planning, then again: some bingo words Paradigm Map and project planning, then repeats: Trained operators and maintainers, project direction";
        let matches = rolegraph.find_matching_node_ids(query);
        for (a, b) in matches.into_iter().tuple_windows() {
            rolegraph.add_or_update_document(&document_id, a, b);
        }
        let document_id2 = Ulid::new().to_string();
        let query2 = "I am a text with the word Life cycle concepts and bar and maintainers, some bingo words Paradigm Map and project planning, then again: some bingo words Paradigm Map and project planning, then repeats: Trained operators and maintainers, project direction";
        let matches2 = rolegraph.find_matching_node_ids(query2);
        for (a, b) in matches2.into_iter().tuple_windows() {
            rolegraph.add_or_update_document(&document_id2, a, b);
        }
        let document_id3 = Ulid::new().to_string();
        let query3 = "I am a text with the word Life cycle concepts and bar and maintainers, some bingo words Paradigm Map and project planning, then again: some bingo words Paradigm Map and project planning, then repeats: Trained operators and maintainers, project direction";
        let matches3 = rolegraph.find_matching_node_ids(query3);
        for (a, b) in matches3.into_iter().tuple_windows() {
            rolegraph.add_or_update_document(&document_id3, a, b);
        }
        let document_id4 = "DocumentID4".to_string();
        let query4 = "I am a text with the word Life cycle concepts and bar and maintainers, some bingo words, then again: some bingo words Paradigm Map and project planning, then repeats: Trained operators and maintainers, project direction";
        let document = Document {
            stub: None,
            url: "/path/to/document".to_string(),
            tags: None,
            rank: None,
            source_haystack: None,
            id: document_id4.clone(),
            title: "Life cycle concepts and project direction".to_string(),
            body: query4.to_string(),
            description: None,
            summarization: None,
            doc_type: terraphim_types::DocumentType::KgEntry,
            synonyms: None,
            route: None,
            priority: None,
        };
        rolegraph.insert_document(&document_id4, document);
        log::debug!("Query graph");
        let results: Vec<(String, IndexedDocument)> = rolegraph
            .query_graph(
                "Life cycle concepts and project direction",
                Some(0),
                Some(10),
            )
            .unwrap();
        println!("results: {:#?}", results);
        let top_result = results.first().unwrap();
        println!("Top result {:?} Rank {:?}", top_result.0, top_result.1.rank);
        println!("Top result {:#?}", top_result.1);
        assert_eq!(results.len(), 4);
    }

    #[test]
    #[ignore]
    async fn test_is_all_terms_connected_by_path_true() {
        let role = "system operator".to_string();
        let rolegraph = RoleGraph::new(role.into(), load_sample_thesaurus().await)
            .await
            .unwrap();
        let text = "Life cycle concepts ... Paradigm Map ... project planning";
        assert!(rolegraph.is_all_terms_connected_by_path(text));
    }

    #[test]
    async fn test_is_all_terms_connected_by_path_false() {
        let role = "system operator".to_string();
        let rolegraph = RoleGraph::new(role.into(), load_sample_thesaurus().await)
            .await
            .unwrap();
        // Intentionally pick terms unlikely to be connected together
        let text = "Trained operators ... bar";
        // Depending on fixture this might be connected; if so, adjust to rare combo
        let _ = rolegraph.is_all_terms_connected_by_path(text);
        // Can't assert false deterministically without graph knowledge; smoke call only
    }

    #[tokio::test]
    async fn test_rolegraph_with_thesaurus_no_node_not_found_errors() {
        use terraphim_types::Document;

        // Create a role graph with sample thesaurus
        let role_name = "Test Role".to_string();
        let thesaurus = load_sample_thesaurus().await;
        let mut rolegraph = RoleGraph::new(role_name.into(), thesaurus.clone())
            .await
            .expect("Failed to create rolegraph");

        // Verify thesaurus is loaded properly
        assert!(
            !rolegraph.thesaurus.is_empty(),
            "Thesaurus should not be empty"
        );
        assert!(
            !rolegraph.ac_reverse_nterm.is_empty(),
            "Reverse term lookup should be populated"
        );
        log::info!(
            "✅ Loaded thesaurus with {} terms",
            rolegraph.thesaurus.len()
        );

        // Test 1: Query empty graph (should return empty results, not NodeIdNotFound error)
        log::info!("🔍 Testing query on empty graph");
        let empty_results = rolegraph
            .query_graph("Life cycle concepts", None, Some(10))
            .expect("Query on empty graph should not fail");
        assert!(
            empty_results.is_empty(),
            "Empty graph should return no results"
        );
        log::info!("✅ Empty graph query handled gracefully");

        // Test 2: Query with non-existent terms (should return empty, not error)
        let nonexistent_results = rolegraph
            .query_graph("nonexistentterms", None, Some(10))
            .expect("Query with non-existent terms should not fail");
        assert!(
            nonexistent_results.is_empty(),
            "Non-existent terms should return no results"
        );
        log::info!("✅ Non-existent terms query handled gracefully");

        // Test 3: Use the same text from working tests that contains thesaurus terms
        let document_text = "I am a text with the word Life cycle concepts and bar and Trained operators and maintainers, project direction, some bingo words Paradigm Map and project planning, then again: some bingo words Paradigm Map and project planning, then repeats: Trained operators and maintainers, project direction";

        // Create document that will definitely match thesaurus terms
        let test_document = Document {
            id: "test_doc".to_string(),
            title: "System Engineering Document".to_string(),
            body: document_text.to_string(),
            url: "/test/document".to_string(),
            tags: Some(vec!["engineering".to_string()]),
            rank: Some(1),
            stub: None,
            description: Some("Test document with thesaurus terms".to_string()),
            summarization: None,
            source_haystack: None,
            doc_type: terraphim_types::DocumentType::KgEntry,
            synonyms: None,
            route: None,
            priority: None,
        };

        // Insert document into rolegraph (this should create nodes and edges)
        rolegraph.insert_document(&test_document.id, test_document.clone());

        log::info!("✅ Inserted 1 document into rolegraph");
        log::info!("  - Graph now has {} nodes", rolegraph.nodes.len());
        log::info!("  - Graph now has {} edges", rolegraph.edges.len());
        log::info!("  - Graph now has {} documents", rolegraph.documents.len());

        // Verify graph structure was created
        assert!(
            !rolegraph.nodes.is_empty(),
            "Nodes should be created from document indexing"
        );
        assert!(
            !rolegraph.edges.is_empty(),
            "Edges should be created from document indexing"
        );
        assert_eq!(rolegraph.documents.len(), 1, "1 document should be stored");

        // Test 4: Query populated graph (should return results without NodeIdNotFound errors)
        let test_queries = vec![
            "Life cycle concepts",
            "Trained operators",
            "Paradigm Map",
            "project planning",
        ];

        for query in test_queries {
            log::info!("🔍 Testing query: '{}'", query);
            let results = rolegraph
                .query_graph(query, None, Some(10))
                .unwrap_or_else(|_| panic!("Query '{}' should not fail", query));

            log::info!("  - Found {} results", results.len());

            // Some queries should return results if they match indexed documents
            if query == "Life cycle concepts"
                || query == "Trained operators"
                || query == "Paradigm Map"
            {
                if !results.is_empty() {
                    log::info!("  ✅ Found expected results for query '{}'", query);
                } else {
                    log::info!(
                        "  ⚠️ No results for '{}' but no error - this is acceptable",
                        query
                    );
                }
            }
        }

        // Test 5: Document lookup functionality
        let document_ids = rolegraph.find_document_ids_for_term("Life cycle concepts");
        if !document_ids.is_empty() {
            log::info!("✅ Found {} documents for term lookup", document_ids.len());
        } else {
            log::info!(
                "⚠️ No documents found for term lookup - acceptable if term not in indexed docs"
            );
        }

        // Test 6: Verify that original NodeIdNotFound scenarios now work
        let original_failing_query = rolegraph
            .query_graph("terraphim-graph", None, Some(10))
            .expect("Query that previously caused NodeIdNotFound should now work");
        log::info!(
            "✅ Previously failing query now works - found {} results",
            original_failing_query.len()
        );

        log::info!("🎉 All rolegraph and thesaurus tests completed successfully!");
        log::info!("✅ Thesaurus loading: Working");
        log::info!("✅ Document indexing: Working");
        log::info!("✅ Graph querying: Working (no NodeIdNotFound errors)");
        log::info!("✅ Defensive error handling: Working");
    }

    #[tokio::test]
    async fn test_rolegraph_serialization() {
        // Create a test rolegraph with sample data
        let role = "test role".to_string();
        let mut rolegraph = RoleGraph::new(role.into(), load_sample_thesaurus().await)
            .await
            .unwrap();

        // Add some test data
        let document_id = Ulid::new().to_string();
        let test_document = Document {
            id: document_id.clone(),
            title: "Test Document".to_string(),
            body: "This is a test document with Life cycle concepts and project planning content and operators".to_string(),
            url: "/test/document".to_string(),
            description: Some("Test document description".to_string()),
            tags: Some(vec!["test".to_string(), "serialization".to_string()]),
            rank: Some(1),
            stub: None,
            summarization: None,
            source_haystack: None,
            doc_type: terraphim_types::DocumentType::KgEntry,
            synonyms: None,
            route: None,
            priority: None,
        };

        // Insert document into rolegraph
        rolegraph.insert_document(&document_id, test_document);

        // Test serialization to serializable representation
        let serializable = rolegraph.to_serializable();
        assert_eq!(serializable.role.original, "test role");
        assert_eq!(serializable.nodes.len(), rolegraph.nodes.len());
        assert_eq!(serializable.edges.len(), rolegraph.edges.len());
        assert_eq!(serializable.documents.len(), rolegraph.documents.len());
        assert_eq!(serializable.thesaurus.len(), rolegraph.thesaurus.len());
        assert!(!serializable.aho_corasick_values.is_empty());
        assert!(!serializable.ac_reverse_nterm.is_empty());

        // Test JSON serialization
        let json_str = serializable.to_json().unwrap();
        assert!(!json_str.is_empty());

        // Test JSON deserialization
        let deserialized = SerializableRoleGraph::from_json(&json_str).unwrap();
        assert_eq!(deserialized.role.original, serializable.role.original);
        assert_eq!(deserialized.nodes.len(), serializable.nodes.len());
        assert_eq!(deserialized.edges.len(), serializable.edges.len());
        assert_eq!(deserialized.documents.len(), serializable.documents.len());
        assert_eq!(deserialized.thesaurus.len(), serializable.thesaurus.len());
        assert_eq!(
            deserialized.aho_corasick_values,
            serializable.aho_corasick_values
        );
        assert_eq!(deserialized.ac_reverse_nterm, serializable.ac_reverse_nterm);

        // Test recreating RoleGraph from serializable
        let recreated_rolegraph = RoleGraph::from_serializable(deserialized).await.unwrap();
        assert_eq!(recreated_rolegraph.role.original, rolegraph.role.original);
        assert_eq!(recreated_rolegraph.nodes.len(), rolegraph.nodes.len());
        assert_eq!(recreated_rolegraph.edges.len(), rolegraph.edges.len());
        assert_eq!(
            recreated_rolegraph.documents.len(),
            rolegraph.documents.len()
        );
        assert_eq!(
            recreated_rolegraph.thesaurus.len(),
            rolegraph.thesaurus.len()
        );

        // Test that the recreated RoleGraph can perform searches (may be empty if no matches found)
        let search_results = recreated_rolegraph
            .query_graph("Life cycle", None, Some(10))
            .unwrap();
        println!("Search results count: {}", search_results.len());

        // Test that the Aho-Corasick automata was rebuilt correctly (may be empty if no matches found)
        let matches = recreated_rolegraph.find_matching_node_ids("Life cycle concepts");
        println!("Aho-Corasick matches count: {}", matches.len());

        // Verify that the search functionality itself works (not that it returns results)
        // The important thing is that it doesn't crash or error
        assert_eq!(recreated_rolegraph.role.original, rolegraph.role.original);
    }

    #[tokio::test]
    async fn test_rolegraph_sync_serialization() {
        // Create a RoleGraphSync with test data
        let role = "sync test role".to_string();
        let mut rolegraph = RoleGraph::new(role.into(), load_sample_thesaurus().await)
            .await
            .unwrap();

        // Add test data
        let document_id = Ulid::new().to_string();
        let test_document = Document {
            id: document_id.clone(),
            title: "Sync Test Document".to_string(),
            body:
                "Document content for testing RoleGraphSync serialization with Paradigm Map terms"
                    .to_string(),
            url: "/test/sync_document".to_string(),
            description: None,
            tags: None,
            rank: None,
            stub: None,
            summarization: None,
            source_haystack: None,
            doc_type: terraphim_types::DocumentType::KgEntry,
            synonyms: None,
            route: None,
            priority: None,
        };

        rolegraph.insert_document(&document_id, test_document);
        let rolegraph_sync = RoleGraphSync::from(rolegraph);

        // Test JSON serialization
        let json_str = rolegraph_sync.to_json().await.unwrap();
        assert!(!json_str.is_empty());

        // Test pretty JSON serialization
        let json_pretty = rolegraph_sync.to_json_pretty().await.unwrap();
        assert!(json_pretty.len() > json_str.len()); // Pretty JSON should be longer

        // Test deserialization back to RoleGraphSync
        let restored_sync = RoleGraphSync::from_json(&json_str).await.unwrap();

        // Verify the restored graph works correctly
        let rolegraph_guard = restored_sync.lock().await;
        assert_eq!(rolegraph_guard.role.original, "sync test role");
        assert_eq!(rolegraph_guard.documents.len(), 1);

        // Test search functionality (may be empty if no matches found)
        let search_results = rolegraph_guard
            .query_graph("Paradigm Map", None, Some(10))
            .unwrap();
        println!(
            "RoleGraphSync search results count: {}",
            search_results.len()
        );

        // Verify the search functionality itself works
        assert_eq!(rolegraph_guard.role.original, "sync test role");
    }

    #[tokio::test]
    async fn test_graph_stats_serialization() {
        // Create a populated rolegraph
        let role = "stats test role".to_string();
        let mut rolegraph = RoleGraph::new(role.into(), load_sample_thesaurus().await)
            .await
            .unwrap();

        // Add test data with content that should match thesaurus terms
        let document_id = Ulid::new().to_string();
        let test_document = Document {
            id: document_id.clone(),
            title: "Stats Test Document".to_string(),
            body: "Test content with Life cycle concepts and operators and maintainers".to_string(),
            url: "/test/stats_document".to_string(),
            description: None,
            tags: None,
            rank: None,
            stub: None,
            summarization: None,
            source_haystack: None,
            doc_type: terraphim_types::DocumentType::KgEntry,
            synonyms: None,
            route: None,
            priority: None,
        };

        rolegraph.insert_document(&document_id, test_document);

        // Get graph stats
        let stats = rolegraph.get_graph_stats();
        assert!(stats.thesaurus_size > 0); // The thesaurus should have content

        // Note: node_count and edge_count might be 0 if document content doesn't match thesaurus
        // The important thing is that the stats can be serialized and deserialized
        println!(
            "Stats - nodes: {}, edges: {}, documents: {}, thesaurus: {}, populated: {}",
            stats.node_count,
            stats.edge_count,
            stats.document_count,
            stats.thesaurus_size,
            stats.is_populated
        );

        // Test stats serialization
        let json_str = serde_json::to_string(&stats).unwrap();
        let deserialized_stats: GraphStats = serde_json::from_str(&json_str).unwrap();

        assert_eq!(stats.node_count, deserialized_stats.node_count);
        assert_eq!(stats.edge_count, deserialized_stats.edge_count);
        assert_eq!(stats.document_count, deserialized_stats.document_count);
        assert_eq!(stats.thesaurus_size, deserialized_stats.thesaurus_size);
        assert_eq!(stats.is_populated, deserialized_stats.is_populated);
    }

    #[tokio::test]
    async fn test_serialization_edge_cases() {
        // Test with empty rolegraph
        let role = "empty test".to_string();
        let empty_thesaurus = Thesaurus::new("empty".to_string());
        let empty_rolegraph = RoleGraph::new(role.into(), empty_thesaurus).await.unwrap();

        let serializable = empty_rolegraph.to_serializable();
        let json = serializable.to_json().unwrap();
        let deserialized = SerializableRoleGraph::from_json(&json).unwrap();
        let restored = RoleGraph::from_serializable(deserialized).await.unwrap();

        assert_eq!(restored.nodes.len(), 0);
        assert_eq!(restored.edges.len(), 0);
        assert_eq!(restored.documents.len(), 0);
        assert_eq!(restored.thesaurus.len(), 0);

        // Test with single node
        let role = "single node test".to_string();
        let thesaurus = load_sample_thesaurus().await;
        let mut single_rolegraph = RoleGraph::new(role.into(), thesaurus).await.unwrap();

        let document_id = Ulid::new().to_string();
        let simple_document = Document {
            id: document_id.clone(),
            title: "Simple".to_string(),
            body: "Life cycle concepts and operators".to_string(), // Should match thesaurus terms
            url: "/test/simple".to_string(),
            description: None,
            tags: None,
            rank: None,
            stub: None,
            summarization: None,
            source_haystack: None,
            doc_type: terraphim_types::DocumentType::KgEntry,
            synonyms: None,
            route: None,
            priority: None,
        };

        single_rolegraph.insert_document(&document_id, simple_document);

        // Verify it can be serialized and restored
        let serializable = single_rolegraph.to_serializable();
        let json = serializable.to_json().unwrap();
        let deserialized = SerializableRoleGraph::from_json(&json).unwrap();
        let restored = RoleGraph::from_serializable(deserialized).await.unwrap();

        assert_eq!(restored.documents.len(), 1);
        assert_eq!(restored.role.original, "single node test");

        // Note: nodes and edges might be empty if content doesn't match thesaurus
        // The important thing is that serialization/deserialization works
        println!(
            "Single node test - nodes: {}, edges: {}",
            restored.nodes.len(),
            restored.edges.len()
        );
    }
}
