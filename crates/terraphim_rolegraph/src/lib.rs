use ahash::AHashMap;
use itertools::Itertools;
use memoize::memoize;
use regex::Regex;
use std::collections::hash_map::Entry;
use std::sync::Arc;
use terraphim_types::{Document, Edge, Id, IndexedDocument, Node, NormalizedTermValue, Thesaurus};
use tokio::sync::{Mutex, MutexGuard};
pub mod input;
use aho_corasick::{AhoCorasick, MatchKind};
use log::warn;
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
/// Currently it maps from synonyms to concepts,
/// so only normalized term returned when reverse lookup is performed
#[derive(Debug, Clone)]
pub struct RoleGraph {
    // role filter
    pub role: String,
    nodes: AHashMap<Id, Node>,
    // TODO: Should this be a ULID?
    edges: AHashMap<Id, Edge>,
    documents: AHashMap<String, IndexedDocument>,
    pub thesaurus: Thesaurus,
    //TODO: make it private once performance tests are fixed
    pub ac_values: Vec<Id>,
    pub ac: AhoCorasick,
    // reverse lookup - matched id into normalized term
    pub ac_reverse_nterm: AHashMap<Id, NormalizedTermValue>,
}

impl RoleGraph {
    /// Creates a new `RoleGraph` with the given role and thesaurus
    pub async fn new(role: String, thesaurus: Thesaurus) -> Result<Self> {
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
            values.push(normalized_term.id.clone());
            ac_reverse_nterm.insert(normalized_term.id.clone(), normalized_term.value.clone());
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
            ac_values: values,
            ac,
            ac_reverse_nterm,
        })
    }

    /// Find all matches in the rolegraph for the given text
    ///
    /// Returns a list of IDs of the matched nodes
    pub fn find_matches_ids(&self, text: &str) -> Vec<Id> {
        let mut matches = Vec::new();
        for mat in self.ac.find_iter(text) {
            let id = &self.ac_values[mat.pattern()];
            matches.push(id.clone());
        }
        matches
    }

    /// This method performs several key operations to process and rank
    /// documents:
    /// - Utilizes node rank as a weight for an edge, and edge rank as a weight
    ///   for a document ID, creating a hierarchical weighting system.
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
    pub fn normalize(&mut self) {
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

    //  Query the graph using a query string, returns a list of document ids
    //  ranked and weighted by weighted mean average of node rank, edge rank and
    //  document rank
    pub fn query(
        &self,
        query_string: &str,
        offset: Option<usize>,
        limit: Option<usize>,
    ) -> Result<Vec<(String, IndexedDocument)>> {
        warn!("performing query");
        let nodes = self.find_matches_ids(query_string);

        //  TODO: turn into BinaryHeap by implementing hash and eq traits

        let mut results_map: AHashMap<String, IndexedDocument> = AHashMap::new();
        for node_id in nodes {
            // warn!("Matched node {:?}", node_id);
            let node = self.nodes.get(&node_id).ok_or(Error::NodeIdNotFound)?;
            let nterm = self.ac_reverse_nterm.get(&node_id).unwrap();
            // println!("Normalized term {nterm}");
            let node_rank = node.rank;
            // warn!("Node Rank {}", node_rank);
            // warn!("Node connected to Edges {:?}", node.connected_with);
            for each_edge_key in &node.connected_with {
                let each_edge = self.edges.get(each_edge_key).ok_or(Error::EdgeIdNotFound)?;
                warn!("Edge Details{:?}", each_edge);
                let edge_rank = each_edge.rank;
                for (document_id, rank) in &each_edge.doc_hash {
                    let total_rank = node_rank + edge_rank + rank;
                    match results_map.entry(document_id.clone()) {
                        Entry::Vacant(_) => {
                            let document = IndexedDocument {
                                id: document_id.to_string(),
                                matched_to: vec![each_edge.clone()],
                                rank: total_rank,
                                tags: vec![nterm.to_string()],
                                nodes: vec![node_id.clone()],
                            };

                            results_map.insert(document_id.clone(), document);
                        }
                        Entry::Occupied(entry) => {
                            let document = entry.into_mut();
                            document.rank += 1;
                            document.matched_to.push(each_edge.clone());
                            document.matched_to.dedup_by_key(|k| k.id.clone());
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

    pub fn parse_document_to_pair(&mut self, document_id: &str, text: &str) {
        let matches = self.find_matches_ids(text);
        for (a, b) in matches.into_iter().tuple_windows() {
            // cast to Id
            let a = a as Id;
            self.add_or_update_document(document_id, a, b);
        }
    }

    pub fn parse_document<T: Into<Document>>(&mut self, document_id: &str, input: T) {
        let document: Document = input.into();
        let matches = self.find_matches_ids(&document.to_string());
        for (a, b) in matches.into_iter().tuple_windows() {
            self.add_or_update_document(document_id, a, b);
        }
    }

    pub fn add_or_update_document(&mut self, document_id: &str, x: Id, y: Id) {
        let edge = magic_pair(x.as_u128(), y.as_u128());
        let edge = self.init_or_update_edge(Id::from(edge), document_id);
        self.init_or_update_node(x, &edge);
        self.init_or_update_node(y, &edge);
    }

    fn init_or_update_node(&mut self, node_id: Id, edge: &Edge) {
        match self.nodes.entry(node_id.clone()) {
            Entry::Vacant(_) => {
                let node = Node::new(node_id.clone(), edge.clone());
                self.nodes.insert(node.id.clone(), node);
            }
            Entry::Occupied(entry) => {
                let node = entry.into_mut();
                node.rank += 1;
                node.connected_with.push(edge.id.clone());
            }
        };
    }

    fn init_or_update_edge(&mut self, edge_key: Id, document_id: &str) -> Edge {
        let edge = match self.edges.entry(edge_key.clone()) {
            Entry::Vacant(_) => {
                let edge = Edge::new(edge_key, document_id.to_string());
                self.edges.insert(edge.id.clone(), edge.clone());
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

/// Combining two numbers into a unique one: pairing functions.
/// It uses "elegant pairing" (https://odino.org/combining-two-numbers-into-a-unique-one-pairing-functions/).
/// also using memoize macro with Ahash hasher
#[memoize(CustomHasher: ahash::AHashMap)]
pub fn magic_pair(x: u128, y: u128) -> u128 {
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
pub fn magic_unpair(z: u128) -> (u128, u128) {
    let q = (z as f32).sqrt().floor() as u128;
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

    use terraphim_automata::load_thesaurus;
    use tokio::test;
    use ulid::Ulid;

    async fn load_sample_thesaurus() -> Thesaurus {
        let automata_url = "https://system-operator.s3.eu-west-2.amazonaws.com/term_to_id.json";
        let thesaurus = load_thesaurus(automata_url).await.unwrap();
        thesaurus
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
    async fn test_find_matches_ids() {
        let query = "I am a text with the word Life cycle concepts and bar and Trained operators and maintainers, project direction, some bingo words Paradigm Map and project planning, then again: some bingo words Paradigm Map and project planning, then repeats: Trained operators and maintainers, project direction";
        let role = "system operator".to_string();
        let rolegraph = RoleGraph::new(role, load_sample_thesaurus().await)
            .await
            .unwrap();
        let matches = rolegraph.find_matches_ids(query);
        assert_eq!(matches.len(), 7);
    }

    #[test]
    async fn test_find_matches_ids_ac_values() {
        let query = "life cycle framework I am a text with the word Life cycle concepts and bar and Trained operators and maintainers, project direction, some bingo words Paradigm Map and project planning, then again: some bingo words Paradigm Map and project planning, then repeats: Trained operators and maintainers, project direction";
        let role = "system operator".to_string();
        let rolegraph = RoleGraph::new(role, load_sample_thesaurus().await)
            .await
            .unwrap();
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
        let mut rolegraph = RoleGraph::new(role, load_sample_thesaurus().await)
            .await
            .unwrap();
        let article_id = Ulid::new().to_string();
        let query = "I am a text with the word Life cycle concepts and bar and Trained operators and maintainers, project direction, some bingo words Paradigm Map and project planning, then again: some bingo words Paradigm Map and project planning, then repeats: Trained operators and maintainers, project direction";
        let matches = rolegraph.find_matches_ids(query);
        for (a, b) in matches.into_iter().tuple_windows() {
            rolegraph.add_or_update_document(&article_id, a, b);
        }
        let article_id2 = Ulid::new().to_string();
        let query2 = "I am a text with the word Life cycle concepts and bar and maintainers, some bingo words Paradigm Map and project planning, then again: some bingo words Paradigm Map and project planning, then repeats: Trained operators and maintainers, project direction";
        let matches2 = rolegraph.find_matches_ids(query2);
        for (a, b) in matches2.into_iter().tuple_windows() {
            rolegraph.add_or_update_document(&article_id2, a, b);
        }
        let article_id3 = Ulid::new().to_string();
        let query3 = "I am a text with the word Life cycle concepts and bar and maintainers, some bingo words Paradigm Map and project planning, then again: some bingo words Paradigm Map and project planning, then repeats: Trained operators and maintainers, project direction";
        let matches3 = rolegraph.find_matches_ids(query3);
        for (a, b) in matches3.into_iter().tuple_windows() {
            rolegraph.add_or_update_document(&article_id3, a, b);
        }
        let article_id4 = "ArticleID4".to_string();
        let query4 = "I am a text with the word Life cycle concepts and bar and maintainers, some bingo words, then again: some bingo words Paradigm Map and project planning, then repeats: Trained operators and maintainers, project direction";
        rolegraph.parse_document_to_pair(&article_id4, query4);
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
