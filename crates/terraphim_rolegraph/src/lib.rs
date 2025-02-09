use ahash::AHashMap;
use itertools::Itertools;
use memoize::memoize;
use regex::Regex;
use std::{collections::hash_map::Entry, result};
use std::sync::Arc;
use terraphim_types::{
    RankedNode, Rank, Document, Edge, IndexedDocument, Node, NormalizedTermValue, RoleName, Thesaurus,
    magic_pair, magic_unpair, NormalizedTerm,
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
        // We need to iterate over keys and values at the same time
        // because the order of entries is not guaranteed
        // when using `.keys()` and `.values()`.
        // let (keys, values): (Vec<&str>, Vec<Id>) = thesaurus
        //     .iter()
        //     .map(|(key, value)| (key.as_str(), value.id))
        //     .unzip();
        let mut keys = Vec::new();
        let mut values = Vec::new();
        let mut ac_reverse_nterm = AHashMap::new();

        for (key, normalized_term) in &thesaurus {
            keys.push(key);
            values.push(normalized_term.id);
            ac_reverse_nterm.insert(normalized_term.id, normalized_term.value.clone());
        }

        let ac = AhoCorasick::builder()
            .match_kind(MatchKind::LeftmostLongest)
            .ascii_case_insensitive(true)
            .build(keys)?;

        Ok(Self {
            role,
            nodes: AHashMap::new(),
            edges: AHashMap::new(),
            documents: AHashMap::new(),
            thesaurus,
            aho_corasick_values: values,
            ac,
            ac_reverse_nterm,
        })
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

    /// Performs a query on the graph using the query string.    /// Lists all nodes in the graph with their ranks and term information
    pub fn list_ranked_nodes(&self) -> Result<Vec<RankedNode>> {
        let mut ranked_nodes = Vec::with_capacity(self.nodes.len());
        
        for (node_id, node) in &self.nodes {
            let normalized_term = self.ac_reverse_nterm
                .get(node_id)
                .ok_or(Error::NodeIdNotFound)?
                .clone();
            
            let mut total_docs = 0;
            let mut ranks = Vec::new();
            
            for edge_id in &node.connected_with {
                if let Some(edge) = self.edges.get(edge_id) {
                    total_docs += edge.doc_hash.len();
                    ranks.push(Rank {
                        node_id: *edge_id,
                        connection_count: node.connected_with.len() as u64,
                        edge_weight: edge.rank,
                    });
                }
            }
            
            // Sort ranks using existing Rank comparison
            ranks.sort_unstable();

            ranked_nodes.push(RankedNode {
                id: *node_id,
                normalized_term,
                ranks,
                total_documents: total_docs,
            });
        }

        // Sort nodes by their highest rank's edge_weight
        ranked_nodes.sort_by(|a, b| {
            let a_max = a.ranks.first().map(|r| r.edge_weight).unwrap_or(0);
            let b_max = b.ranks.first().map(|r| r.edge_weight).unwrap_or(0);
            b_max.cmp(&a_max) // Descending order
        });
        
        Ok(ranked_nodes)
    }

    /// Optimized query to find connections and documents from a list of node IDs
    pub fn query_graph_optimised(
        &self,
        node_ids: &[u64],
        offset: Option<usize>,
        limit: Option<usize>,
    ) -> Result<Vec<(String, IndexedDocument)>> {
        log::debug!("Performing optimized graph query for {} nodes", node_ids.len());
        
        let mut results = AHashMap::new();
        for &node_id in node_ids {
            let node = self.nodes.get(&node_id).ok_or(Error::NodeIdNotFound)?;
            let Some(normalized_term) = self.ac_reverse_nterm.get(&node_id) else {
                return Err(Error::NodeIdNotFound);
            };

            // Get ranked connections for this node
            let ranked_connections = node.query_optimised(node_ids, None);
            
            for rank in ranked_connections {
                if let Some(edge) = self.edges.get(&rank.node_id) {
                    for (document_id, document_rank) in &edge.doc_hash {
                        let total_rank = rank.edge_weight + document_rank;
                        
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
                                if !doc.nodes.contains(&node_id) {
                                    doc.nodes.push(node_id);
                                }
                                if !doc.tags.contains(&normalized_term.to_string()) {
                                    doc.tags.push(normalized_term.to_string());
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
            .take(limit.unwrap_or(std::usize::MAX))
            .collect();

        log::debug!("Query resulted in {} documents", documents.len());
        Ok(documents)
    }

    /// Performs a query on the graph using the query string.
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

        let mut results = AHashMap::new();
        for node_id in node_ids {
            let node = self.nodes.get(&node_id).ok_or(Error::NodeIdNotFound)?;
            let Some(normalized_term) = self.ac_reverse_nterm.get(&node_id) else {
                return Err(Error::NodeIdNotFound);
            };
            log::debug!("Processing node ID: {:?} with rank: {}", node_id, node.rank);

            for edge_id in &node.connected_with {
                let edge = self.edges.get(edge_id).ok_or(Error::EdgeIdNotFound)?;
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
            .take(limit.unwrap_or(std::usize::MAX))
            .collect();

        log::debug!("Query resulted in {} documents", documents.len());
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
        let matches = self.find_matching_node_ids(&document.to_string());
        for (a, b) in matches.into_iter().tuple_windows() {
            self.add_or_update_document(document_id, a, b);
        }
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

    fn init_or_update_edge(&mut self, edge_key: u64, document_id: &str) -> Edge {
        let edge = match self.edges.entry(edge_key) {
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
        };
        edge
    }

    pub fn get_ranked_documents(&self, ranked_nodes: &[RankedNode]) -> Result<Vec<(String, IndexedDocument, Rank)>> {
        let mut results = AHashMap::new();
        
        // Process only the provided ranked nodes
        for ranked_node in ranked_nodes {
            let normalized_term = self.ac_reverse_nterm
                .get(&ranked_node.id)
                .ok_or(Error::NodeIdNotFound)?;

            // Use the pre-calculated ranks from the RankedNode
            for rank in &ranked_node.ranks {
                if let Some(edge) = self.edges.get(&rank.node_id) {
                    for (document_id, document_rank) in &edge.doc_hash {
                        let total_rank = rank.edge_weight + document_rank;
                        
                        match results.entry(document_id.clone()) {
                            Entry::Vacant(e) => {
                                e.insert((
                                    IndexedDocument {
                                        id: document_id.clone(),
                                        matched_edges: vec![edge.clone()],
                                        rank: total_rank,
                                        tags: vec![normalized_term.to_string()],
                                        nodes: vec![ranked_node.id],
                                    },
                                    rank.clone(),
                                ));
                            }
                            Entry::Occupied(mut e) => {
                                let (doc, existing_rank) = e.get_mut();
                                doc.rank += total_rank;
                                doc.matched_edges.push(edge.clone());
                                doc.matched_edges.dedup_by_key(|e| e.id);
                                if !doc.nodes.contains(&ranked_node.id) {
                                    doc.nodes.push(ranked_node.id);
                                }
                                if !doc.tags.contains(&normalized_term.to_string()) {
                                    doc.tags.push(normalized_term.to_string());
                                }
                                // Update rank if the new one has higher edge_weight
                                if rank.edge_weight > existing_rank.edge_weight {
                                    *existing_rank = rank.clone();
                                }
                            }
                        }
                    }
                }
            }
        }

        let mut ranked_documents: Vec<_> = results
            .into_iter()
            .map(|(id, (doc, rank))| (id, doc, rank))
            .collect();
            
        // Sort by document rank (descending) and then by edge weight (descending)
        ranked_documents.sort_by(|(_, doc_a, rank_a), (_, doc_b, rank_b)| {
            doc_b.rank
                .cmp(&doc_a.rank)
                .then(rank_b.edge_weight.cmp(&rank_a.edge_weight))
        });
        
        Ok(ranked_documents)
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

#[cfg(test)]
mod tests {
    use super::*;

    use terraphim_automata::{load_thesaurus, AutomataPath};
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
        assert_eq!(sentences[7],"John Johnson Jr. was born in the U.S.A but earned his Ph.D. in Israel before joining Nike Inc. as an engineer.");
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
        assert_eq!(matches.len(), 4);
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
    #[ignore]
    async fn test_terraphim_engineer() {
        let role_name = "Terraphim Engineer".to_string();
        let automata_path = AutomataPath::local_example_full();
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
        // Don't assert the exact size since it may change
        assert!(thesaurus.len() > 0);
        let matches = rolegraph.find_matching_node_ids(&test_document);
        println!("Matches {:?}", matches);
        for (a, b) in matches.into_iter().tuple_windows() {
            rolegraph.add_or_update_document(&document_id, a, b);
        }
        let document = Document {
            stub: None,
            url: "/path/to/document".to_string(),
            tags: None,
            rank: None,
            id: document_id.clone(),
            title: "README".to_string(),
            body: test_document.to_string(),
            description: None,
        };
        rolegraph.insert_document(&document_id, document);
        println!("query with {}", "terraphim-graph and service".to_string());
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
            id: document_id2.clone(),
            title: "terraphim-graph".to_string(),
            body: test_document2.to_string(),
            description: None,
        };
        rolegraph.insert_document(&document_id2, document2);
        log::debug!("Query graph");
        let results: Vec<(String, IndexedDocument)> = rolegraph
            .query_graph("terraphim-graph and service", Some(0), Some(10))
            .unwrap();
        println!("results: {:#?}", results);
        assert!(!results.is_empty(), "Should find at least one result");
        let top_result = results.get(0).unwrap();
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
            id: document_id4.clone(),
            title: "Life cycle concepts and project direction".to_string(),
            body: query4.to_string(),
            description: None,
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
        let top_result = results.get(0).unwrap();
        println!("Top result {:?} Rank {:?}", top_result.0, top_result.1.rank);
        println!("Top result {:#?}", top_result.1);
        assert_eq!(results.len(), 4);
    }

    #[test]
    async fn test_list_ranked_nodes() {
        let role = "system operator".to_string();
        let mut rolegraph = RoleGraph::new(role.into(), load_sample_thesaurus().await)
            .await
            .unwrap();

        // Insert test documents with known connections
        let document_id = Ulid::new().to_string();
        let test_document = "Life cycle concepts and project direction with Trained operators and maintainers";
        let document = Document {
            stub: None,
            url: "/test/doc".to_string(),
            tags: None,
            rank: None,
            id: document_id.clone(),
            title: test_document.to_string(),
            body: test_document.to_string(),
            description: None,
        };
        rolegraph.insert_document(&document_id, document);

        // Get all ranked nodes
        let ranked_nodes = rolegraph.list_ranked_nodes().unwrap();
        
        // Verify we have nodes
        assert!(!ranked_nodes.is_empty());

        // Check first node has expected structure
        let first_node = &ranked_nodes[0];
        assert!(!first_node.ranks.is_empty());
        assert!(first_node.total_documents > 0);
        
        // Verify nodes are sorted by highest rank
        for window in ranked_nodes.windows(2) {
            let first_max = window[0].ranks.first().map(|r| r.edge_weight).unwrap_or(0);
            let second_max = window[1].ranks.first().map(|r| r.edge_weight).unwrap_or(0);
            assert!(first_max >= second_max, "Nodes should be sorted by highest rank");
        }

        // Verify each node has correct normalized term
        for node in &ranked_nodes {
            assert!(rolegraph.ac_reverse_nterm.contains_key(&node.id));
            assert_eq!(
                rolegraph.ac_reverse_nterm.get(&node.id).unwrap(),
                &node.normalized_term
            );
        }
    }

    #[test]
    async fn test_query_graph_optimised() {
        let role = "system operator".to_string();
        let mut rolegraph = RoleGraph::new(role.into(), load_sample_thesaurus().await)
            .await
            .unwrap();

        // Insert test document with known consecutive matches
        let document_id = Ulid::new().to_string();
        let test_document = "Life cycle models and project direction";  // These terms should be consecutive
        let document = Document {
            stub: None,
            url: "/test/doc".to_string(),
            tags: None,
            rank: None,
            id: document_id.clone(),
            title: test_document.to_string(),
            body: test_document.to_string(),
            description: None,
        };
        rolegraph.insert_document(&document_id, document);

        // Get node IDs for testing - these should match our known terms
        let node_ids = rolegraph.find_matching_node_ids(test_document);
        assert!(!node_ids.is_empty());
        println!("Found node_ids: {:?}", node_ids);

        // Verify we found our known terms
        for node_id in &node_ids {
            let term = rolegraph.ac_reverse_nterm.get(node_id).unwrap();
            println!("Found term: {:?}", term);
            assert!(
                term.as_str() == "life cycle models" ||
                term.as_str() == "project direction",
                "Unexpected term found: {}", term
            );
        }

        // Test optimized query
        let results = rolegraph.query_graph_optimised(&node_ids, Some(0), Some(10)).unwrap();
        assert!(!results.is_empty());

        // Get the first result
        let first_result = &results[0].1;
        
        // Verify document metadata
        assert_eq!(first_result.id, document_id);
        assert!(!first_result.matched_edges.is_empty());
        assert!(!first_result.nodes.is_empty());
        assert!(!first_result.tags.is_empty());

        // Print debug info
        println!("Document edges: {:?}", first_result.matched_edges);
        println!("Document nodes: {:?}", first_result.nodes);
        println!("Document tags: {:?}", first_result.tags);

        // Verify results are properly ranked
        for window in results.windows(2) {
            assert!(
                window[0].1.rank >= window[1].1.rank,
                "Documents should be sorted by rank in descending order"
            );
        }
    }
}
