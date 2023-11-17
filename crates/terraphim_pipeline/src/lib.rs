use ahash::AHashMap;
use itertools::Itertools;
use memoize::memoize;
use regex::Regex;
use std::collections::hash_map::Entry;
pub mod input;
use aho_corasick::{AhoCorasick, MatchKind};
use log::warn;
use serde::{Deserialize, Serialize};

use terraphim_automata::load_automata;
use terraphim_automata::matcher::Dictionary;
use thiserror::Error;
use unicode_segmentation::UnicodeSegmentation;

type Result<T> = std::result::Result<T, TerraphimPipelineError>;

#[derive(Error, Debug)]
pub enum TerraphimPipelineError {
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

// use tracing::{debug, error, info, span, warn, Level};

/// Document that can be indexed by the `RoleGraph`.
///
/// These are all articles and entities, which have fields that can be indexed.
#[derive(Debug, Clone)]
pub struct Document {
    /// Unique identifier of the document
    pub id: String,
    /// Title of the document
    pub title: String,
    /// Body of the document
    pub body: Option<String>,
    /// Description of the document
    pub description: Option<String>,
}

impl ToString for Document {
    fn to_string(&self) -> String {
        let mut text = String::new();
        text.push_str(&self.title);
        if let Some(body) = &self.body {
            text.push_str(body);
        }
        if let Some(description) = &self.description {
            text.push_str(description);
        }
        text
    }
}

/// Reference to external storage of documents, traditional indexes use
/// document, aka article or entity.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct IndexedDocument {
    /// UUID of the indexed document
    id: String,
    /// Matched to edges
    matched_to: Vec<Edge>,
    /// Graph rank (the sum of node rank, edge rank)
    rank: u64,
}

impl IndexedDocument {
    pub fn to_json_string(&self) -> Result<String> {
        Ok(serde_json::to_string(&self)?)
    }
}

//TODO: create top_k_nodes function where
// sort nodes by rank
// TODO create top_k_edges function where
//sort edges by rank
// TODO create top_k_documents function where
// sort document id by rank

#[derive(Debug, Clone)]
pub struct RoleGraph {
    // role filter
    role: String,
    nodes: AHashMap<u64, Node>,
    edges: AHashMap<u64, Edge>,
    documents: AHashMap<String, IndexedDocument>,
    automata_url: String,
    dict_hash: AHashMap<String, Dictionary>,
    //TODO: make it private once performance tests are fixed
    pub ac_values: Vec<u64>,
    pub ac: AhoCorasick,
}
impl RoleGraph {
    pub fn new(role: String, automata_url: &str) -> Result<Self> {
        let dict_hash = load_automata(automata_url)?;

        // We need to iterate over keys and values at the same time
        // because the order of entries is not guaranteed
        // when using `.keys()` and `.values()`.
        let (keys, values): (Vec<&str>, Vec<u64>) = dict_hash
            .iter()
            .map(|(key, value)| (key.as_str(), value.id))
            .unzip();

        let ac = AhoCorasick::builder()
            .match_kind(MatchKind::LeftmostLongest)
            .ascii_case_insensitive(true)
            .build(keys)?;

        Ok(Self {
            role,
            nodes: AHashMap::new(),
            edges: AHashMap::new(),
            documents: AHashMap::new(),
            automata_url: automata_url.to_string(),
            dict_hash,
            ac_values: values,
            ac,
        })
    }

    /// Find all matches int the rolegraph for the given text
    /// Returns a list of document ids
    fn find_matches_ids(&self, text: &str) -> Vec<u64> {
        let mut matches = Vec::new();
        for mat in self.ac.find_iter(text) {
            // println!("mat: {:?}", mat);
            let id = self.ac_values[mat.pattern()];
            matches.push(id);
        }
        matches
    }

    //  Query the graph using a query string, returns a list of document ids ranked and weighted by weighted mean average of node rank, edge rank and document rank

    // node rank is a weight for edge and edge rank is a weight for document_id
    // create hashmap of output with document_id, rank to dedupe documents in output
    // normalise output rank from 1 to number of records
    // pre-sort document_id by rank using BtreeMap
    //  overall weighted average is calculated a weighted average of node rank and edge rank and document rank
    //  weighted average  can be calculated: sum of (weight*rank)/sum of weights for each node, edge and document.
    //  rank is a number of co-occurences normalised over number of documents (entities), see cleora train function
    // YAGNI: at the moment I don't need it, so parked
    pub fn normalise(&mut self) {
        let node_len = self.nodes.len() as u32;
        warn!("Node Length {}", node_len);
        let edge_len = self.edges.len() as u32;
        warn!("Edge Length {}", edge_len);
        let document_count = self.documents.len() as u32;
        warn!("document Length {}", document_count);
        let normalizer = f32::from_bits(node_len + edge_len + document_count);
        let weight_node = f32::from_bits(node_len) / normalizer;
        let weight_edge = f32::from_bits(edge_len) / normalizer;
        let weight_document = f32::from_bits(document_count) / normalizer;
        warn!("Weight Node {}", weight_node);
        warn!("Weight Edge {}", weight_edge);
        warn!("Weight document {}", weight_document);
        // for each node for each edge for each document
        // for (document_id,rank) in self.documents.iter(){
        //     let weighted_rank=(weight_node*node_rank as f32)+(weight_edge*edge_rank as f32)+(weight_document*rank as f32)/(weight_node+weight_edge+weight_document);
        //     warn!("document id {} Weighted Rank {}", document_id, weighted_rank);
        //     sorted_vector_by_rank_weighted.push((document_id, weighted_rank));
        // }
    }

    /// Query rolegraph without sorting
    pub fn query_inner(&self, query_string: &str) -> Result<AHashMap<&String, IndexedDocument>> {
        warn!("performing query");
        let nodes = self.find_matches_ids(query_string);

        //  turn into hashset by implementing hash and eq traits

        let mut results_map = AHashMap::new();
        for node_id in nodes.iter() {
            // warn!("Matched node {:?}", node_id);
            let node = self
                .nodes
                .get(node_id)
                .ok_or(TerraphimPipelineError::NodeIdNotFound)?;

            let node_rank = node.rank;
            // warn!("Node Rank {}", node_rank);
            // warn!("Node connected to Edges {:?}", node.connected_with);
            for each_edge_key in node.connected_with.iter() {
                let each_edge = self
                    .edges
                    .get(each_edge_key)
                    .ok_or(TerraphimPipelineError::EdgeIdNotFound)?;
                warn!("Edge Details{:?}", each_edge);
                let edge_rank = each_edge.rank;
                for (document_id, rank) in each_edge.doc_hash.iter() {
                    let total_rank = node_rank + edge_rank + rank;
                    match results_map.entry(document_id) {
                        Entry::Vacant(_) => {
                            let document = IndexedDocument {
                                id: document_id.to_string(),
                                matched_to: vec![each_edge.clone()],
                                rank: total_rank,
                            };

                            results_map.insert(document_id, document);
                        }
                        Entry::Occupied(entry) => {
                            let document = entry.into_mut();
                            document.rank += 1;
                            document.matched_to.push(each_edge.clone());
                            document.matched_to.dedup_by_key(|k| k.id);
                        }
                    }
                }
            }
        }
        Ok(results_map)
    }

    /// Query rolegraph with sorting by rank (default)
    /// Note that there are other ways to sort the results as well.
    /// See other methods on `RoleGraph`
    pub fn query(&self, query_string: &str) -> Result<Vec<(&String, IndexedDocument)>> {
        let mut results_map = self.query_inner(query_string)?;
        // Convert into vector because the map does not have inherent order
        let mut hash_vec = results_map.into_iter().collect::<Vec<_>>();

        // sort by rank by default
        hash_vec.sort_by(|a, b| b.1.rank.cmp(&a.1.rank));

        Ok(hash_vec)
    }

    /// Query rolegraph with sorting by rank and return top n results
    pub fn query_top_n<T: Ord>(
        &self,
        query_string: &str,
        n: usize,
    ) -> Result<Vec<(&String, IndexedDocument)>> {
        let results_map = self.query_inner(query_string)?;
        let mut hash_vec = results_map.into_iter().collect::<Vec<_>>();
        hash_vec.sort_by(|a, b| b.1.rank.cmp(&a.1.rank));
        Ok(hash_vec.into_iter().take(n).collect::<Vec<_>>())
    }

    /// Query rolegraph with sorting by rank and return bottom (last) n results
    pub fn query_bottom_n<T: Ord>(
        &self,
        query_string: &str,
        n: usize,
    ) -> Result<Vec<(&String, IndexedDocument)>> {
        // let results_map = self.query_inner(query_string)?;
        // let mut hash_vec = results_map.into_iter().collect::<Vec<_>>();
        // // Sort by rank in reverse order
        // hash_vec.sort_by(|a, b| a.1.rank.cmp(&b.1.rank));
        // Ok(hash_vec.into_iter().take(n).collect::<Vec<_>>())

        let hash_map = self.query_inner(query_string)?;
        SortedDocuments::new(hash_map).top_n(n).into()

    }

    // pub fn least_n<T: Ord>(n: usize, mut from: impl Iterator<Item = T>) -> impl Iterator<Item = T> {
    //     let mut h = BinaryHeap::from_iter(from.by_ref().take(n));

    //     for it in from {
    //         // heap thinks the smallest is the greatest because of reverse order
    //         let mut greatest = h.peek_mut().unwrap();

    //         if it < *greatest {
    //             // heap rebalances after the smart pointer is dropped
    //             *greatest = it;
    //         }
    //     }
    //     h.into_iter()
    // }

    pub fn parse_document_to_pair(&mut self, document_id: String, text: &str) {
        let matches = self.find_matches_ids(text);
        for (a, b) in matches.into_iter().tuple_windows() {
            self.add_or_update_document(document_id.clone(), a, b);
        }
    }
    pub fn parse_document<T: Into<Document>>(&mut self, document_id: String, input: T) {
        let document: Document = input.into();
        let matches = self.find_matches_ids(&document.to_string());
        for (a, b) in matches.into_iter().tuple_windows() {
            self.add_or_update_document(document_id.clone(), a, b);
        }
    }
    pub fn add_or_update_document(&mut self, document_id: String, x: u64, y: u64) {
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
                node.connected_with.push(edge.id);
            }
        };
    }
    fn init_or_update_edge(&mut self, edge_key: u64, document_id: String) -> Edge {
        let edge = match self.edges.entry(edge_key) {
            Entry::Vacant(_) => {
                let edge = Edge::new(edge_key, document_id);
                self.edges.insert(edge.id, edge.clone());
                edge
            }
            Entry::Occupied(entry) => {
                let edge = entry.into_mut();
                *edge.doc_hash.entry(document_id).or_insert(1) += 1;

                edge.clone()
            }
        };
        edge
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Edge {
    // id of the node
    id: u64,
    rank: u64,
    // hashmap document_id, rank
    doc_hash: AHashMap<String, u64>,
}
impl Edge {
    pub fn new(id: u64, document_id: String) -> Self {
        let mut doc_hash = AHashMap::new();
        doc_hash.insert(document_id, 1);
        Self {
            id,
            rank: 1,
            doc_hash,
        }
    }
}
// Node represent single concept
#[derive(Debug, Clone)]
pub struct Node {
    id: u64,
    // number of co-occureneces
    rank: u64,
    connected_with: Vec<u64>,
}
impl Node {
    fn new(id: u64, edge: Edge) -> Self {
        Self {
            id,
            rank: 1,
            connected_with: vec![edge.id],
        }
    }
    // pub fn sort_edges_by_value(&self) {
    //     // let count_b: BTreeMap<&u64, &Edge> =
    //     // self.connected_with.iter().map(|(k, v)| (v, k)).collect();
    //     // for (k, v) in self.connected_with.iter().map(|(k, v)| (v.rank, k)) {
    //     // warn!("k {:?} v {:?}", k, v);
    //     // }
    //     warn!("Connected with {:?}", self.connected_with);
    // }
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
    if x >= y {
        x * x + x + y
    } else {
        y * y + x
    }
}

// Magic unpair
// func unpair(z int) (int, int) {
//   q := int(math.Floor(math.Sqrt(float64(z))))
//     l := z - q * q

//   if l < q {
//       return l, q
//   }

//   return q, l - q
// }
#[memoize(CustomHasher: ahash::AHashMap)]
pub fn magic_unpair(z: u64) -> (u64, u64) {
    let q = (z as f32).sqrt().floor() as u64;
    let l = z - q * q;
    if l < q {
        (l, q)
    } else {
        (q, l - q)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use ulid::Ulid;

    #[test]
    fn test_split_paragraphs() {
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
    fn test_find_matches_ids() {
        let query = "I am a text with the word Life cycle concepts and bar and Trained operators and maintainers, project direction, some bingo words Paradigm Map and project planning, then again: some bingo words Paradigm Map and project planning, then repeats: Trained operators and maintainers, project direction";
        let role = "system operator".to_string();
        let automata_url = "https://system-operator.s3.eu-west-2.amazonaws.com/term_to_id.json";
        let rolegraph = RoleGraph::new(role, automata_url).unwrap();
        let matches = rolegraph.find_matches_ids(query);
        assert_eq!(matches.len(), 7);
    }

    #[test]
    fn test_rolegraph() {
        let role = "system operator".to_string();
        let automata_url = "https://system-operator.s3.eu-west-2.amazonaws.com/term_to_id.json";
        let mut rolegraph = RoleGraph::new(role, automata_url).unwrap();
        let article_id = Ulid::new().to_string();
        let query = "I am a text with the word Life cycle concepts and bar and Trained operators and maintainers, project direction, some bingo words Paradigm Map and project planning, then again: some bingo words Paradigm Map and project planning, then repeats: Trained operators and maintainers, project direction";
        let matches = rolegraph.find_matches_ids(query);
        for (a, b) in matches.into_iter().tuple_windows() {
            rolegraph.add_or_update_document(article_id.clone(), a, b);
        }
        let article_id2 = Ulid::new().to_string();
        let query2 = "I am a text with the word Life cycle concepts and bar and maintainers, some bingo words Paradigm Map and project planning, then again: some bingo words Paradigm Map and project planning, then repeats: Trained operators and maintainers, project direction";
        let matches2 = rolegraph.find_matches_ids(query2);
        for (a, b) in matches2.into_iter().tuple_windows() {
            rolegraph.add_or_update_document(article_id2.clone(), a, b);
        }
        let article_id3 = Ulid::new().to_string();
        let query3 = "I am a text with the word Life cycle concepts and bar and maintainers, some bingo words Paradigm Map and project planning, then again: some bingo words Paradigm Map and project planning, then repeats: Trained operators and maintainers, project direction";
        let matches3 = rolegraph.find_matches_ids(query3);
        for (a, b) in matches3.into_iter().tuple_windows() {
            rolegraph.add_or_update_document(article_id3.clone(), a, b);
        }
        let article_id4 = "ArticleID4".to_string();
        let query4 = "I am a text with the word Life cycle concepts and bar and maintainers, some bingo words, then again: some bingo words Paradigm Map and project planning, then repeats: Trained operators and maintainers, project direction";
        rolegraph.parse_document_to_pair(article_id4, query4);
        warn!("Query graph");
        let results_map = rolegraph
            .query("Life cycle concepts and project direction")
            .unwrap();
        assert_eq!(results_map.len(), 4);
    }
}
