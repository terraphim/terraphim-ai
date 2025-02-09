use ahash::AHashMap;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::hash_map::Iter;
use std::collections::HashSet;
use std::fmt::{self, Display, Formatter};
use std::iter::IntoIterator;
use std::ops::{Deref, DerefMut};

use std::str::FromStr;

/// Combining two numbers into a unique one: pairing functions.
/// It uses "elegant pairing" (https://odino.org/combining-two-numbers-into-a-unique-one-pairing-functions/).
pub fn magic_pair(x: u64, y: u64) -> u64 {
    if x >= y {
        x * x + x + y
    } else {
        y * y + x
    }
}

/// Magic unpair function to recover the original pair of numbers
pub fn magic_unpair(z: u64) -> (u64, u64) {
    let q = (z as f32).sqrt().floor() as u64;
    let l = z - q * q;
    if l < q {
        (l, q)
    } else {
        (q, l - q)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct RoleName {
    pub original: String,
    pub lowercase: String,
}

impl RoleName {
    pub fn new(name: &str) -> Self {
        RoleName {
            original: name.to_string(),
            lowercase: name.to_lowercase(),
        }
    }

    pub fn as_lowercase(&self) -> &str {
        &self.lowercase
    }
}

impl fmt::Display for RoleName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.original)
    }
}

impl FromStr for RoleName {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(RoleName::new(s))
    }
}

impl From<&str> for RoleName {
    fn from(s: &str) -> Self {
        RoleName::new(s)
    }
}

impl From<String> for RoleName {
    fn from(s: String) -> Self {
        RoleName::new(&s)
    }
}

impl Serialize for RoleName {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.original)
    }
}

impl<'de> Deserialize<'de> for RoleName {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Ok(RoleName::new(&s))
    }
}
/// The value of a normalized term
///
/// This is a string that has been normalized to lowercase and trimmed.
#[derive(Default, Debug, Deserialize, Serialize, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NormalizedTermValue(String);

impl NormalizedTermValue {
    pub fn new(term: String) -> Self {
        let value = term.trim().to_lowercase();
        Self(value)
    }
    // convert to &str
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<String> for NormalizedTermValue {
    fn from(term: String) -> Self {
        Self::new(term)
    }
}

impl From<&str> for NormalizedTermValue {
    fn from(term: &str) -> Self {
        Self::new(term.to_string())
    }
}

impl Display for NormalizedTermValue {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl AsRef<[u8]> for NormalizedTermValue {
    fn as_ref(&self) -> &[u8] {
        self.0.as_bytes()
    }
}

// FIXME: this can be greatly improved, ID only shall be unique per KG
use std::sync::atomic::{AtomicU64, Ordering};
static INT_SEQ: AtomicU64 = AtomicU64::new(1);
fn get_int_id() -> u64 {
    INT_SEQ.fetch_add(1, Ordering::SeqCst)
}

/// A normalized term is a higher-level term that has been normalized
///
/// It holds a unique identifier to an underlying and the normalized value.
#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct NormalizedTerm {
    /// Unique identifier for the normalized term
    pub id: u64,
    /// The normalized value
    // This field is currently called `nterm` in the JSON
    #[serde(rename = "nterm")]
    pub value: NormalizedTermValue,
    /// The URL of the normalized term
    pub url: Option<String>
}

impl NormalizedTerm {
    pub fn new(id: u64, value: NormalizedTermValue) -> Self {
        Self { id, value, url: None }
    }
}

/// A concept is a higher-level, normalized term.
///
/// It describes a unique, abstract idea in a machine-readable format.
///
/// An example of a concept is "machine learning" which is normalized from
/// "Machine Learning"
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Concept {
    /// A unique identifier for the concept
    pub id: u64,
    /// The normalized concept
    pub value: NormalizedTermValue,
}

impl Concept {
    pub fn new(value: NormalizedTermValue) -> Self {
        Self {
            id: get_int_id(),
            value,
        }
    }
}

impl From<String> for Concept {
    fn from(concept: String) -> Self {
        let concept = NormalizedTermValue::new(concept);
        Self::new(concept)
    }
}

impl Display for Concept {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.value)
    }
}

/// A document is the central a piece of content that gets indexed and searched.
///
/// It holds the title, body, description, tags, and rank.
/// The `id` is a unique identifier for the document.
#[derive(Deserialize, Serialize, Debug, Clone, Default)]
pub struct Document {
    /// Unique identifier for the document
    pub id: String,
    /// URL to the document
    pub url: String,
    /// Title of the document
    pub title: String,
    /// The document body
    pub body: String,

    /// A short description of the document
    pub description: Option<String>,
    /// A short excerpt of the document
    pub stub: Option<String>,
    /// Tags for the document
    pub tags: Option<Vec<String>>,
    /// Rank of the document in the search results
    pub rank: Option<u64>,
}

impl fmt::Display for Document {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Start with title and body
        write!(f, "{} {}", self.title, self.body)?;

        // Append description if it exists
        if let Some(ref description) = self.description {
            write!(f, " {}", description)?;
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Edge {
    /// ID of the edge
    pub id: u64,
    /// Rank of the edge
    pub rank: u64,
    /// A hashmap of `document_id` to `rank`
    pub doc_hash: AHashMap<String, u64>,
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

/// A `Node` represents single concept and its connections to other concepts.
///
/// Each node can have multiple edges to other nodes
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Node {
    /// Unique identifier of the node
    pub id: u64,
    /// Number of co-occurrences
    pub rank: u64,
    /// List of connected edges
    pub connected_with: HashSet<u64>,
}

impl Node {
    /// Create a new node with a given id and edge
    pub fn new(id: u64, edge: Edge) -> Self {
        let mut connected_with = HashSet::new();
        connected_with.insert(edge.id);
        Self {
            id,
            rank: 1,
            connected_with,
        }
    }
        /// Lists all nodes in the graph with their ranks and connections
        pub fn list_ranked_nodes(&self) -> Vec<Rank> {
            let mut ranks = Vec::with_capacity(self.connected_with.len());
            
            for edge_id in &self.connected_with {
                ranks.push(Rank {
                    node_id: *edge_id,
                    connection_count: self.connected_with.len() as u64,
                    edge_weight: self.rank,
                });
            }
            
            // Sort by rank in descending order (using existing Ord implementation)
            ranks.sort_unstable();
            ranks
        }
    /// Optimized query to find connections from a list of node IDs
    /// Returns ranked connections sorted by edge weight
    pub fn query_optimised(&self, node_ids: &[u64], limit: Option<usize>) -> Vec<Rank> {
        let target_nodes: HashSet<u64> = node_ids.iter().cloned().collect();
        
        // For each edge ID in our connected edges, we need to check if it was created
        // from a pair of nodes where at least one is in our target set
        let mut ranks: Vec<Rank> = self.connected_with
            .iter()
            .filter(|&edge_id| {
                // Unpack the edge ID back into its component node IDs
                let (x, y) = magic_unpair(*edge_id);
                // The edge is relevant if either node is in our target set
                target_nodes.contains(&x) || target_nodes.contains(&y)
            })
            .map(|edge_id| Rank {
                node_id: *edge_id,
                connection_count: self.connected_with.len() as u64,
                edge_weight: self.rank,
            })
            .collect();

        // Sort using existing Rank comparison (higher edge_weight first)
        ranks.sort_unstable();
        
        if let Some(limit) = limit {
            ranks.truncate(limit);
        }
        
        ranks
    }

    // pub fn sort_edges_by_value(&self) {
    //     let count_b: BTreeMap<&u64, &Edge> =
    //     self.connected_with.iter().map(|(k, v)| (v, k)).collect();
    //     for (k, v) in self.connected_with.iter().map(|(k, v)| (v.rank, k)) {
    //     log::warn!("k {:?} v {:?}", k, v);
    //     }
    //     log::warn!("Connected with {:?}", self.connected_with);
    // }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RankedNode {
    pub id: u64,
    pub normalized_term: NormalizedTermValue,
    pub ranks: Vec<Rank>,
    pub total_documents: usize,
}

/// A thesaurus is a dictionary with synonyms which map to upper-level concepts.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct Thesaurus {
    /// Name of the thesaurus
    name: String,
    /// The inner hashmap of normalized terms
    data: AHashMap<NormalizedTermValue, NormalizedTerm>,
}

impl Thesaurus {
    /// Create a new, empty thesaurus
    pub fn new(name: String) -> Self {
        Self {
            name,
            data: AHashMap::new(),
        }
    }

    /// Get the name of the thesaurus
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Inserts a key-value pair into the thesaurus.
    pub fn insert(&mut self, key: NormalizedTermValue, value: NormalizedTerm) {
        self.data.insert(key, value);
    }

    /// Get the length of the thesaurus
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// Check if the thesaurus is empty
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Custom `get` method for the thesaurus, which accepts a
    /// `NormalizedTermValue` and returns a reference to the
    /// `NormalizedTerm`.
    pub fn get(&self, key: &NormalizedTermValue) -> Option<&NormalizedTerm> {
        self.data.get(key)
    }

    pub fn keys(&self) -> std::collections::hash_map::Keys<NormalizedTermValue, NormalizedTerm> {
        self.data.keys()
    }
}

// Implement `IntoIterator` for a reference to `Thesaurus`
impl<'a> IntoIterator for &'a Thesaurus {
    type Item = (&'a NormalizedTermValue, &'a NormalizedTerm);
    type IntoIter = Iter<'a, NormalizedTermValue, NormalizedTerm>;

    fn into_iter(self) -> Self::IntoIter {
        self.data.iter()
    }
}

/// An index is a hashmap of documents
///
/// It holds the documents that have been indexed
/// and can be searched through using the `RoleGraph`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Index {
    inner: AHashMap<String, Document>,
}

impl Default for Index {
    fn default() -> Self {
        Self::new()
    }
}

impl Index {
    /// Create a new, empty index
    pub fn new() -> Self {
        Self {
            inner: AHashMap::new(),
        }
    }

    /// Converts all given indexed documents to documents
    ///
    /// Returns the all converted documents
    pub fn get_documents(&self, docs: Vec<IndexedDocument>) -> Vec<Document> {
        let mut documents: Vec<Document> = Vec::new();
        for doc in docs {
            log::trace!("doc: {:#?}", doc);
            if let Some(document) = self.get_document(&doc) {
                // Document found in cache
                let mut document = document;
                document.tags = Some(doc.tags.clone());
                // rank only available for terraphim graph
                // use scorer to populate the rank for all cases
                document.rank = Some(doc.rank);
                documents.push(document.clone());
            } else {
                log::warn!("Document not found in cache. Cannot convert.");
            }
        }
        documents
    }
    /// Returns all documents from the index for scorer without graph embeddings
    pub fn get_all_documents(&self) -> Vec<Document> {
        let documents: Vec<Document> = self.values().cloned().collect::<Vec<Document>>();
        documents
    }

    /// Get a document from the index (if it exists in the index)
    pub fn get_document(&self, doc: &IndexedDocument) -> Option<Document> {
        if let Some(document) = self.inner.get(&doc.id).cloned() {
            // Document found in cache
            let mut document = document;
            document.tags = Some(doc.tags.clone());
            // Rank only available for terraphim graph
            // use scorer to populate the rank for all cases
            document.rank = Some(doc.rank);
            Some(document)
        } else {
            None
        }
    }
}

impl Deref for Index {
    type Target = AHashMap<String, Document>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for Index {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl IntoIterator for Index {
    type Item = (String, Document);
    type IntoIter = std::collections::hash_map::IntoIter<String, Document>;

    fn into_iter(self) -> Self::IntoIter {
        self.inner.into_iter()
    }
}

/// Reference to external storage of documents
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct IndexedDocument {
    /// UUID of the indexed document, matching external storage id
    pub id: String,
    /// Matched to edges
    pub matched_edges: Vec<Edge>,
    /// Graph rank (the sum of node rank, edge rank)
    /// Number of nodes and edges connected to the document
    pub rank: u64,
    /// Tags, which are nodes turned into concepts for human readability
    pub tags: Vec<String>,
    /// List of node IDs for validation of matching
    pub nodes: Vec<u64>,
}

impl IndexedDocument {
    pub fn to_json_string(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(&self)
    }
}

/// Query type for searching documents in the `RoleGraph`.
/// It contains the search term, skip and limit parameters.
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct SearchQuery {
    pub search_term: NormalizedTermValue,
    pub skip: Option<usize>,
    pub limit: Option<usize>,
    pub role: Option<RoleName>,
}

/// Defines the relevance function (scorer) to be used for ranking search
/// results for the `Role`.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Copy)]
pub enum RelevanceFunction {
    /// Scorer for ranking search results based on the Terraphim graph
    ///
    /// This is based on filtered result outputs according to the ranking of the
    /// knowledge graph. The node, which is most connected will produce the
    /// highest ranking
    #[serde(rename = "terraphim-graph")]
    TerraphimGraph,
    /// Scorer for ranking search results based on the title of a document
    #[serde(rename = "title-scorer")]
    TitleScorer,
}

/// Defines all supported inputs for the knowledge graph.
///
/// Every knowledge graph is built from a specific input, such as Markdown files
/// or JSON files.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub enum KnowledgeGraphInputType {
    /// A set of Markdown files
    #[serde(rename = "markdown")]
    Markdown,
    /// A JSON files
    #[serde(rename = "json")]
    Json,
}

// Represents the rank of a node in the graph with its connections
#[derive(Debug, Clone, Serialize, Deserialize, Eq)]
pub struct Rank {
    pub node_id: u64,
    pub connection_count: u64,
    pub edge_weight: u64,
}

impl PartialEq for Rank {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.edge_weight == other.edge_weight
    }
}

impl PartialOrd for Rank {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Rank {
    #[inline]
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.edge_weight.cmp(&other.edge_weight)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rank_ordering() {
        let rank1 = Rank {
            node_id: 1,
            connection_count: 5,
            edge_weight: 10,
        };
        let rank2 = Rank {
            node_id: 2,
            connection_count: 3,
            edge_weight: 20,
        };
        let rank3 = Rank {
            node_id: 3,
            connection_count: 7,
            edge_weight: 10,
        };

        // Test ordering (higher edge_weight should come first)
        assert!(rank2 > rank1);
        assert!(rank1 == rank3); // Same edge_weight
        
        // Test sorting
        let mut ranks = vec![rank1.clone(), rank2.clone(), rank3.clone()];
        ranks.sort_unstable();
        assert_eq!(ranks, vec![rank1, rank3, rank2]);
    }

    #[test]
    fn test_ranked_node_creation() {
        let normalized_term = NormalizedTermValue::new("test term".to_string());
        let ranks = vec![
            Rank {
                node_id: 1,
                connection_count: 5,
                edge_weight: 10,
            },
            Rank {
                node_id: 2,
                connection_count: 3,
                edge_weight: 20,
            },
        ];

        let ranked_node = RankedNode {
            id: 1,
            normalized_term: normalized_term.clone(),
            ranks: ranks.clone(),
            total_documents: 2,
        };

        assert_eq!(ranked_node.id, 1);
        assert_eq!(ranked_node.normalized_term, normalized_term);
        assert_eq!(ranked_node.ranks, ranks);
        assert_eq!(ranked_node.total_documents, 2);
    }
}