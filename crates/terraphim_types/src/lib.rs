use ahash::AHashMap;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::hash_map::Iter;
use std::collections::HashSet;
use std::fmt::{self, Display, Formatter};
use std::iter::IntoIterator;
use std::ops::{Deref, DerefMut};

use schemars::JsonSchema;
use std::str::FromStr;
#[cfg(feature = "typescript")]
use tsify::Tsify;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(Tsify))]
#[cfg_attr(feature = "typescript", tsify(into_wasm_abi, from_wasm_abi))]
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

    pub fn as_str(&self) -> &str {
        &self.original
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
#[cfg_attr(feature = "typescript", derive(Tsify))]
#[cfg_attr(feature = "typescript", tsify(into_wasm_abi, from_wasm_abi))]
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
    pub url: Option<String>,
}

impl NormalizedTerm {
    pub fn new(id: u64, value: NormalizedTermValue) -> Self {
        Self {
            id,
            value,
            url: None,
        }
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
#[cfg_attr(feature = "typescript", derive(Tsify))]
#[cfg_attr(feature = "typescript", tsify(into_wasm_abi, from_wasm_abi))]
pub struct Document {
    /// Unique identifier for the document
    pub id: String,
    /// URL to the document
    pub url: String,
    /// Title of the document
    pub title: String,
    /// The document body
    pub body: String,

    /// A short description of the document (extracted from content)
    pub description: Option<String>,
    /// AI-generated summarization of the document content
    pub summarization: Option<String>,
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

        // Append summarization if it exists and is different from description
        if let Some(ref summarization) = self.summarization {
            if Some(summarization) != self.description.as_ref() {
                write!(f, " {}", summarization)?;
            }
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

    // pub fn sort_edges_by_value(&self) {
    //     // let count_b: BTreeMap<&u64, &Edge> =
    //     // self.connected_with.iter().map(|(k, v)| (v, k)).collect();
    //     // for (k, v) in self.connected_with.iter().map(|(k, v)| (v.rank, k)) {
    //     // log::warn!("k {:?} v {:?}", k, v);
    //     // }
    //     log::warn!("Connected with {:?}", self.connected_with);
    // }
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
    pub fn from_document(document: Document) -> Self {
        IndexedDocument {
            id: document.id,
            matched_edges: Vec::new(),
            rank: 0,
            tags: document.tags.unwrap_or_default(),
            nodes: Vec::new(),
        }
    }
}

/// Logical operators for combining multiple search terms
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(Tsify))]
#[cfg_attr(feature = "typescript", tsify(into_wasm_abi, from_wasm_abi))]
pub enum LogicalOperator {
    /// AND operator - documents must contain all terms
    #[serde(rename = "and")]
    And,
    /// OR operator - documents may contain any of the terms
    #[serde(rename = "or")]
    Or,
}

/// Query type for searching documents in the `RoleGraph`.
/// It contains the search term(s), logical operators, skip and limit parameters.
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[cfg_attr(feature = "typescript", derive(Tsify))]
#[cfg_attr(feature = "typescript", tsify(into_wasm_abi, from_wasm_abi))]
pub struct SearchQuery {
    /// Primary search term for backward compatibility
    pub search_term: NormalizedTermValue,
    /// Multiple search terms for logical operations
    pub search_terms: Option<Vec<NormalizedTermValue>>,
    /// Logical operator for combining multiple terms (defaults to OR if not specified)
    pub operator: Option<LogicalOperator>,
    pub skip: Option<usize>,
    pub limit: Option<usize>,
    pub role: Option<RoleName>,
}

impl SearchQuery {
    /// Get all search terms (both single and multiple)
    pub fn get_all_terms(&self) -> Vec<&NormalizedTermValue> {
        if let Some(ref multiple_terms) = self.search_terms {
            // For multi-term queries, use search_terms (which should contain all terms)
            multiple_terms.iter().collect()
        } else {
            // For single-term queries, use search_term
            vec![&self.search_term]
        }
    }

    /// Check if this is a multi-term query with logical operators
    pub fn is_multi_term_query(&self) -> bool {
        self.search_terms.is_some() && !self.search_terms.as_ref().unwrap().is_empty()
    }

    /// Get the effective logical operator (defaults to Or for multi-term queries)
    pub fn get_operator(&self) -> LogicalOperator {
        self.operator
            .as_ref()
            .unwrap_or(&LogicalOperator::Or)
            .clone()
    }

    /// Create a new SearchQuery with multiple terms and an operator
    pub fn with_terms_and_operator(
        primary_term: NormalizedTermValue,
        additional_terms: Vec<NormalizedTermValue>,
        operator: LogicalOperator,
        role: Option<RoleName>,
    ) -> Self {
        Self {
            search_term: primary_term,
            search_terms: Some(additional_terms),
            operator: Some(operator),
            skip: None,
            limit: None,
            role,
        }
    }
}

/// Defines the relevance function (scorer) to be used for ranking search
/// results for the `Role`.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Copy, JsonSchema, Default)]
#[cfg_attr(feature = "typescript", derive(Tsify))]
#[cfg_attr(feature = "typescript", tsify(into_wasm_abi, from_wasm_abi))]
pub enum RelevanceFunction {
    /// Scorer for ranking search results based on the Terraphim graph
    ///
    /// This is based on filtered result outputs according to the ranking of the
    /// knowledge graph. The node, which is most connected will produce the
    /// highest ranking
    #[serde(rename = "terraphim-graph")]
    TerraphimGraph,
    /// Scorer for ranking search results based on the title of a document
    #[default]
    #[serde(rename = "title-scorer")]
    TitleScorer,
    /// BM25 (Okapi BM25) relevance function for probabilistic ranking
    #[serde(rename = "bm25")]
    BM25,
    /// BM25F relevance function with field-specific weights (title, body, description, tags)
    #[serde(rename = "bm25f")]
    BM25F,
    /// BM25Plus relevance function with enhanced parameters for fine-tuning
    #[serde(rename = "bm25plus")]
    BM25Plus,
}

/// Defines all supported inputs for the knowledge graph.
///
/// Every knowledge graph is built from a specific input, such as Markdown files
/// or JSON files.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(Tsify))]
#[cfg_attr(feature = "typescript", tsify(into_wasm_abi, from_wasm_abi))]
pub enum KnowledgeGraphInputType {
    /// A set of Markdown files
    #[serde(rename = "markdown")]
    Markdown,
    /// A JSON files
    #[serde(rename = "json")]
    Json,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_search_query_logical_operators() {
        // Test single term query (backward compatibility)
        let single_query = SearchQuery {
            search_term: NormalizedTermValue::new("rust".to_string()),
            search_terms: None,
            operator: None,
            skip: None,
            limit: Some(10),
            role: Some(RoleName::new("test")),
        };

        assert!(!single_query.is_multi_term_query());
        assert_eq!(single_query.get_all_terms().len(), 1);
        assert_eq!(single_query.get_operator(), LogicalOperator::Or); // Default

        // Test multi-term query with AND operator
        let and_query = SearchQuery::with_terms_and_operator(
            NormalizedTermValue::new("machine".to_string()),
            vec![NormalizedTermValue::new("learning".to_string())],
            LogicalOperator::And,
            Some(RoleName::new("test")),
        );

        assert!(and_query.is_multi_term_query());
        assert_eq!(and_query.get_all_terms().len(), 2);
        assert_eq!(and_query.get_operator(), LogicalOperator::And);

        // Test multi-term query with OR operator
        let or_query = SearchQuery::with_terms_and_operator(
            NormalizedTermValue::new("neural".to_string()),
            vec![NormalizedTermValue::new("networks".to_string())],
            LogicalOperator::Or,
            Some(RoleName::new("test")),
        );

        assert!(or_query.is_multi_term_query());
        assert_eq!(or_query.get_all_terms().len(), 2);
        assert_eq!(or_query.get_operator(), LogicalOperator::Or);
    }

    #[test]
    fn test_logical_operator_serialization() {
        // Test LogicalOperator serialization
        let and_op = LogicalOperator::And;
        let or_op = LogicalOperator::Or;

        let and_json = serde_json::to_string(&and_op).unwrap();
        let or_json = serde_json::to_string(&or_op).unwrap();

        assert_eq!(and_json, "\"and\"");
        assert_eq!(or_json, "\"or\"");

        // Test deserialization
        let and_deser: LogicalOperator = serde_json::from_str("\"and\"").unwrap();
        let or_deser: LogicalOperator = serde_json::from_str("\"or\"").unwrap();

        assert_eq!(and_deser, LogicalOperator::And);
        assert_eq!(or_deser, LogicalOperator::Or);
    }

    #[test]
    fn test_search_query_serialization() {
        let query = SearchQuery {
            search_term: NormalizedTermValue::new("test".to_string()),
            search_terms: Some(vec![
                NormalizedTermValue::new("additional".to_string()),
                NormalizedTermValue::new("terms".to_string()),
            ]),
            operator: Some(LogicalOperator::And),
            skip: Some(0),
            limit: Some(10),
            role: Some(RoleName::new("test_role")),
        };

        let json = serde_json::to_string(&query).unwrap();
        let deserialized: SearchQuery = serde_json::from_str(&json).unwrap();

        assert_eq!(query.search_term, deserialized.search_term);
        assert_eq!(query.search_terms, deserialized.search_terms);
        assert_eq!(query.operator, deserialized.operator);
        assert_eq!(query.skip, deserialized.skip);
        assert_eq!(query.limit, deserialized.limit);
        assert_eq!(query.role, deserialized.role);
    }
}
