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
    /// UUID of the indexed document, matching external storage id
    pub id: String,
    /// Matched to edges
    matched_to: Vec<Edge>,
    /// Graph rank (the sum of node rank, edge rank)
    pub rank: u64,
    /// tags, which are nodes turned into concepts for human readability
    pub tags: Vec<String>,
    /// list of node ids for validation of matching
    nodes: Vec<u64>,
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

/// RoleGraph is a graph of concepts and their relationships.
/// It is used to index documents and search for them.
/// Currently it maps from synonyms to concepts,
/// so only normalized term returned when reverse lookup is performed
#[derive(Debug, Clone)]
pub struct RoleGraph {
    // role filter
    pub role: String,
    nodes: AHashMap<u64, Node>,
    edges: AHashMap<u64, Edge>,
    documents: AHashMap<String, IndexedDocument>,
    // TODO: Do we want to keep `automata_url` and `dict_hash`?
    // They are currently unused.
    pub automata_url: String,
    pub dict_hash: AHashMap<String, Dictionary>,
    //TODO: make it private once performance tests are fixed
    pub ac_values: Vec<u64>,
    pub ac: AhoCorasick,
    // reverse lookup - matched id into normalized term
    pub ac_reverse_nterm: AHashMap<u64, String>,
}
impl RoleGraph {
    pub async fn new(role: String, automata_url: &str) -> Result<Self> {
        let dict_hash = load_automata(automata_url).await?;

        // We need to iterate over keys and values at the same time
        // because the order of entries is not guaranteed
        // when using `.keys()` and `.values()`.
        // let (keys, values): (Vec<&str>, Vec<u64>) = dict_hash
        //     .iter()
        //     .map(|(key, value)| (key.as_str(), value.id))
        //     .unzip();
        let mut keys = Vec::new();
        let mut values = Vec::new();
        let mut ac_reverse_nterm = AHashMap::new();
        for (key, value) in dict_hash.iter() {
            keys.push(key.as_str());
            values.push(value.id);
            ac_reverse_nterm.insert(value.id, value.nterm.clone());
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
            automata_url: automata_url.to_string(),
            dict_hash,
            ac_values: values,
            ac,
            ac_reverse_nterm,
        })
    }

    /// Find all matches int the rolegraph for the given text
    /// Returns a list of ids of the matched nodes
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

    pub fn query(
        &self,
        query_string: &str,
        offset: Option<usize>,
        limit: Option<usize>,
    ) -> Result<Vec<(&String, IndexedDocument)>> {
        warn!("performing query");
        let nodes = self.find_matches_ids(query_string);

        //  TODO: turn into BinaryHeap by implementing hash and eq traits

        let mut results_map = AHashMap::new();
        for node_id in nodes.iter() {
            // warn!("Matched node {:?}", node_id);
            let node = self.nodes.get(node_id).ok_or(Error::NodeIdNotFound)?;
            let nterm = self.ac_reverse_nterm.get(node_id).unwrap();
            println!("Normalized term {nterm}");
            let node_rank = node.rank;
            // warn!("Node Rank {}", node_rank);
            // warn!("Node connected to Edges {:?}", node.connected_with);
            for each_edge_key in node.connected_with.iter() {
                let each_edge = self.edges.get(each_edge_key).ok_or(Error::EdgeIdNotFound)?;
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
                                tags: vec![nterm.clone()],
                                nodes: vec![*node_id],
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
        // warn!("Results Map {:#?}", results_map);
        let mut hash_vec = results_map.into_iter().collect::<Vec<_>>();
        hash_vec.sort_by(|a, b| b.1.rank.cmp(&a.1.rank));
        hash_vec = hash_vec
            .into_iter()
            .skip(offset.unwrap_or(0))
            .take(limit.unwrap_or(std::usize::MAX))
            .collect();
        Ok(hash_vec)
    }
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
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
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

    use tokio::test;
    use ulid::Ulid;

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
    async fn test_find_matches_ids() {
        let query = "I am a text with the word Life cycle concepts and bar and Trained operators and maintainers, project direction, some bingo words Paradigm Map and project planning, then again: some bingo words Paradigm Map and project planning, then repeats: Trained operators and maintainers, project direction";
        let role = "system operator".to_string();
        let automata_url = "https://system-operator.s3.eu-west-2.amazonaws.com/term_to_id.json";
        let rolegraph = RoleGraph::new(role, automata_url).await.unwrap();
        let matches = rolegraph.find_matches_ids(query);
        assert_eq!(matches.len(), 7);
    }

    #[test]
    async fn test_find_matches_ids_ac_values() {
        let query = "life cycle framework I am a text with the word Life cycle concepts and bar and Trained operators and maintainers, project direction, some bingo words Paradigm Map and project planning, then again: some bingo words Paradigm Map and project planning, then repeats: Trained operators and maintainers, project direction";
        let role = "system operator".to_string();
        let automata_url = "https://system-operator.s3.eu-west-2.amazonaws.com/term_to_id.json";
        let rolegraph = RoleGraph::new(role, automata_url).await.unwrap();
        let matches = rolegraph.find_matches_ids(query);
        println!("matches: {:?}", matches);
        for each_match in matches.iter() {
            let ac_reverse_nterm = rolegraph.ac_reverse_nterm.get(each_match).unwrap();
            println!("{each_match} ac_reverse_nterm: {:?}", ac_reverse_nterm);
        }
        assert_eq!(
            rolegraph.ac_reverse_nterm.get(&matches[0]).unwrap(),
            "life cycle models"
        );
    }

    #[test]
    async fn test_rolegraph() {
        let role = "system operator".to_string();
        let automata_url = "https://system-operator.s3.eu-west-2.amazonaws.com/term_to_id.json";
        let mut rolegraph = RoleGraph::new(role, automata_url).await.unwrap();
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
        let results: Vec<(&String, IndexedDocument)> = rolegraph
            .query(
                "Life cycle concepts and project direction",
                Some(0),
                Some(10),
            )
            .unwrap();
        assert_eq!(results.len(), 4);
    }
}
