//! Core type definitions for the Terraphim AI system.
//!
//! This crate provides the fundamental data structures used throughout the Terraphim ecosystem:
//!
//! - **Knowledge Graph Types**: [`Concept`], [`Node`], [`Edge`], [`Thesaurus`]
//! - **Document Management**: [`Document`], [`Index`], [`IndexedDocument`]
//! - **Search Operations**: [`SearchQuery`], [`LogicalOperator`], [`RelevanceFunction`]
//! - **Conversation Context**: [`Conversation`], [`ChatMessage`], [`ContextItem`]
//! - **LLM Routing**: [`RoutingRule`], [`RoutingDecision`], [`Priority`]
//! - **Multi-Agent Coordination**: [`MultiAgentContext`], [`AgentInfo`]
//!
//! # Features
//!
//! - `typescript`: Enable TypeScript type generation via tsify for WASM compatibility
//!
//! # Examples
//!
//! ## Creating a Search Query
//!
//! ```
//! use terraphim_types::{SearchQuery, NormalizedTermValue, LogicalOperator, RoleName};
//!
//! // Simple single-term query
//! let query = SearchQuery {
//!     search_term: NormalizedTermValue::from("rust"),
//!     search_terms: None,
//!     operator: None,
//!     skip: None,
//!     limit: Some(10),
//!     role: Some(RoleName::new("engineer")),
//! };
//!
//! // Multi-term AND query
//! let multi_query = SearchQuery::with_terms_and_operator(
//!     NormalizedTermValue::from("async"),
//!     vec![NormalizedTermValue::from("programming")],
//!     LogicalOperator::And,
//!     Some(RoleName::new("engineer")),
//! );
//! ```
//!
//! ## Working with Documents
//!
//! ```
//! use terraphim_types::Document;
//!
//! let document = Document {
//!     id: "doc-1".to_string(),
//!     url: "https://example.com/article".to_string(),
//!     title: "Introduction to Rust".to_string(),
//!     body: "Rust is a systems programming language...".to_string(),
//!     description: Some("A guide to Rust".to_string()),
//!     summarization: None,
//!     stub: None,
//!     tags: Some(vec!["rust".to_string(), "programming".to_string()]),
//!     rank: None,
//!     source_haystack: None,
//! };
//! ```
//!
//! ## Building a Knowledge Graph
//!
//! ```
//! use terraphim_types::{Thesaurus, NormalizedTermValue, NormalizedTerm};
//!
//! let mut thesaurus = Thesaurus::new("programming".to_string());
//! thesaurus.insert(
//!     NormalizedTermValue::from("rust"),
//!     NormalizedTerm::new(1, NormalizedTermValue::from("rust programming language"))
//!         .with_url("https://rust-lang.org".to_string())
//! );
//! ```

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

/// A role name with case-insensitive lookup support.
///
/// Stores both the original casing and a lowercase version for efficient
/// case-insensitive operations. Roles represent different user profiles or
/// personas in the Terraphim system, each with specific knowledge domains
/// and search preferences.
///
/// Note: Equality is based on both fields, so two instances with different
/// original casing are not equal. Use `as_lowercase()` for case-insensitive comparisons.
///
/// # Examples
///
/// ```
/// use terraphim_types::RoleName;
///
/// let role = RoleName::new("DataScientist");
/// assert_eq!(role.as_str(), "DataScientist");
/// assert_eq!(role.as_lowercase(), "datascientist");
///
/// // Compare using lowercase for case-insensitive matching
/// let role2 = RoleName::new("datascientist");
/// assert_eq!(role.as_lowercase(), role2.as_lowercase());
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(Tsify))]
#[cfg_attr(feature = "typescript", tsify(into_wasm_abi, from_wasm_abi))]
pub struct RoleName {
    /// The original role name preserving the original casing
    pub original: String,
    /// Lowercase version for case-insensitive comparisons
    pub lowercase: String,
}

impl RoleName {
    /// Creates a new role name from a string.
    ///
    /// # Arguments
    ///
    /// * `name` - The role name with any casing
    ///
    /// # Examples
    ///
    /// ```
    /// use terraphim_types::RoleName;
    ///
    /// let role = RoleName::new("SoftwareEngineer");
    /// ```
    pub fn new(name: &str) -> Self {
        RoleName {
            original: name.to_string(),
            lowercase: name.to_lowercase(),
        }
    }

    /// Returns the lowercase version of the role name.
    ///
    /// Use this for case-insensitive comparisons.
    pub fn as_lowercase(&self) -> &str {
        &self.lowercase
    }

    /// Returns the original role name with preserved casing.
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
/// The `display_value` field stores the original case for output purposes.
#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct NormalizedTerm {
    /// Unique identifier for the normalized term
    pub id: u64,
    /// The normalized value (lowercase, used for case-insensitive matching)
    // This field is currently called `nterm` in the JSON
    #[serde(rename = "nterm")]
    pub value: NormalizedTermValue,
    /// The display value with original case preserved (used for replacement output)
    /// Falls back to `value` if None for backward compatibility
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_value: Option<String>,
    /// The URL of the normalized term
    pub url: Option<String>,
}

impl NormalizedTerm {
    /// Create a new normalized term with the given id and value.
    /// The display_value will be None (falls back to value for output).
    pub fn new(id: u64, value: NormalizedTermValue) -> Self {
        Self {
            id,
            value,
            display_value: None,
            url: None,
        }
    }

    /// Set the display value (original case for output).
    /// Use this to preserve the original case from markdown headings.
    pub fn with_display_value(mut self, display_value: String) -> Self {
        self.display_value = Some(display_value);
        self
    }

    /// Set the URL for this term.
    pub fn with_url(mut self, url: String) -> Self {
        self.url = Some(url);
        self
    }

    /// Get the display value, falling back to the normalized value if not set.
    /// This is the value that should be used for replacement output.
    pub fn display(&self) -> &str {
        self.display_value
            .as_deref()
            .unwrap_or_else(|| self.value.as_str())
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

/// The central document type representing indexed and searchable content.
///
/// Documents are the primary unit of content in Terraphim. They can come from
/// various sources (local files, web pages, API responses) and are indexed for
/// semantic search using knowledge graphs.
///
/// # Fields
///
/// * `id` - Unique identifier (typically a UUID or URL-based ID)
/// * `url` - Source URL or file path
/// * `title` - Document title (used for display and basic search)
/// * `body` - Full text content
/// * `description` - Optional short description (extracted or provided)
/// * `summarization` - Optional AI-generated summary
/// * `stub` - Optional brief excerpt
/// * `tags` - Optional categorization tags (often from knowledge graph)
/// * `rank` - Optional relevance score from search results
/// * `source_haystack` - Optional identifier of the data source that provided this document
///
/// # Examples
///
/// ```
/// use terraphim_types::Document;
///
/// let doc = Document {
///     id: "rust-book-ch1".to_string(),
///     url: "https://doc.rust-lang.org/book/ch01-00-getting-started.html".to_string(),
///     title: "Getting Started".to_string(),
///     body: "Let's start your Rust journey...".to_string(),
///     description: Some("Introduction to Rust programming".to_string()),
///     summarization: None,
///     stub: None,
///     tags: Some(vec!["rust".to_string(), "tutorial".to_string()]),
///     rank: Some(95),
///     source_haystack: Some("rust-docs".to_string()),
///};
/// ```
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
    /// Source haystack location that this document came from
    pub source_haystack: Option<String>,
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

impl Document {
    /// Set the source haystack for this document
    pub fn with_source_haystack(mut self, haystack_location: String) -> Self {
        self.source_haystack = Some(haystack_location);
        self
    }

    /// Get the source haystack location
    pub fn get_source_haystack(&self) -> Option<&String> {
        self.source_haystack.as_ref()
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

    pub fn keys(
        &self,
    ) -> std::collections::hash_map::Keys<'_, NormalizedTermValue, NormalizedTerm> {
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

/// A search query for finding documents in the knowledge graph.
///
/// Supports both single-term and multi-term queries with logical operators (AND/OR).
/// Results can be paginated using `skip` and `limit`, and scoped to specific roles.
///
/// # Examples
///
/// ## Single-term query
///
/// ```
/// use terraphim_types::{SearchQuery, NormalizedTermValue, RoleName};
///
/// let query = SearchQuery {
///     search_term: NormalizedTermValue::from("machine learning"),
///     search_terms: None,
///     operator: None,
///     skip: None,
///     limit: Some(10),
///     role: Some(RoleName::new("data_scientist")),
/// };
/// ```
///
/// ## Multi-term AND query
///
/// ```
/// use terraphim_types::{SearchQuery, NormalizedTermValue, LogicalOperator, RoleName};
///
/// let query = SearchQuery::with_terms_and_operator(
///     NormalizedTermValue::from("rust"),
///     vec![NormalizedTermValue::from("async"), NormalizedTermValue::from("tokio")],
///     LogicalOperator::And,
///     Some(RoleName::new("engineer")),
/// );
/// assert!(query.is_multi_term_query());
/// assert_eq!(query.get_all_terms().len(), 3);
/// ```
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[cfg_attr(feature = "typescript", derive(Tsify))]
#[cfg_attr(feature = "typescript", tsify(into_wasm_abi, from_wasm_abi))]
pub struct SearchQuery {
    /// Primary search term for backward compatibility
    #[serde(alias = "query")]
    pub search_term: NormalizedTermValue,
    /// Multiple search terms for logical operations
    pub search_terms: Option<Vec<NormalizedTermValue>>,
    /// Logical operator for combining multiple terms (defaults to OR if not specified)
    pub operator: Option<LogicalOperator>,
    /// Number of results to skip (for pagination)
    pub skip: Option<usize>,
    /// Maximum number of results to return
    pub limit: Option<usize>,
    /// Role context for this search
    pub role: Option<RoleName>,
}

impl SearchQuery {
    /// Get all search terms (both single and multiple)
    pub fn get_all_terms(&self) -> Vec<&NormalizedTermValue> {
        if let Some(ref multiple_terms) = self.search_terms {
            // For multi-term queries, include primary term + additional terms
            let mut all_terms = vec![&self.search_term];
            all_terms.extend(multiple_terms.iter());
            all_terms
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

// Context Management Types for LLM Conversations

/// Unique identifier for conversations
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(Tsify))]
#[cfg_attr(feature = "typescript", tsify(into_wasm_abi, from_wasm_abi))]
pub struct ConversationId(pub String);

impl ConversationId {
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }

    pub fn from_string(id: String) -> Self {
        Self(id)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for ConversationId {
    fn default() -> Self {
        Self::new()
    }
}

impl Display for ConversationId {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Types of context that can be added to conversations
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(Tsify))]
#[cfg_attr(feature = "typescript", tsify(into_wasm_abi, from_wasm_abi))]
pub enum ContextType {
    /// System-level context
    System,
    /// User-provided context
    UserInput,
    /// Document-based context
    Document,
    /// Search result context
    SearchResult,
    /// External data or API context
    External,
    /// Context from KG term definition with synonyms and metadata
    KGTermDefinition,
    /// Context from complete knowledge graph index
    KGIndex,
}

/// Unique identifier for messages within conversations
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(Tsify))]
#[cfg_attr(feature = "typescript", tsify(into_wasm_abi, from_wasm_abi))]
pub struct MessageId(pub String);

impl MessageId {
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }

    pub fn from_string(id: String) -> Self {
        Self(id)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for MessageId {
    fn default() -> Self {
        Self::new()
    }
}

impl Display for MessageId {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Context item that can be added to LLM conversations
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(Tsify))]
#[cfg_attr(feature = "typescript", tsify(into_wasm_abi, from_wasm_abi))]
pub struct ContextItem {
    /// Unique identifier for the context item
    pub id: String,
    /// Type of context (document, search_result, user_input, etc.)
    pub context_type: ContextType,
    /// Title or summary of the context item
    pub title: String,
    /// Brief summary of the content (separate from full content)
    pub summary: Option<String>,
    /// The actual content to be included in the LLM context
    pub content: String,
    /// Metadata about the context (source, relevance score, etc.)
    pub metadata: AHashMap<String, String>,
    /// Timestamp when this context was added
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// Relevance score for ordering context items
    pub relevance_score: Option<f64>,
}

impl ContextItem {
    /// Create a new context item from a document
    pub fn from_document(document: &Document) -> Self {
        let mut metadata = AHashMap::new();
        metadata.insert("source_type".to_string(), "document".to_string());
        metadata.insert("document_id".to_string(), document.id.clone());
        if !document.url.is_empty() {
            metadata.insert("url".to_string(), document.url.clone());
        }
        if let Some(ref tags) = &document.tags {
            metadata.insert("tags".to_string(), tags.join(", "));
        }
        if let Some(rank) = document.rank {
            metadata.insert("rank".to_string(), rank.to_string());
        }

        Self {
            id: uuid::Uuid::new_v4().to_string(),
            context_type: ContextType::Document,
            title: if document.title.is_empty() {
                document.id.clone()
            } else {
                document.title.clone()
            },
            summary: document.description.clone(),
            content: format!(
                "Title: {}\n\n{}\n\n{}",
                document.title,
                document.description.as_deref().unwrap_or(""),
                document.body
            ),
            metadata,
            created_at: chrono::Utc::now(),
            relevance_score: document.rank.map(|r| r as f64),
        }
    }

    /// Create a new context item from search results
    pub fn from_search_result(query: &str, documents: &[Document]) -> Self {
        let mut metadata = AHashMap::new();
        metadata.insert("source_type".to_string(), "search_result".to_string());
        metadata.insert("query".to_string(), query.to_string());
        metadata.insert("result_count".to_string(), documents.len().to_string());

        let content = if documents.is_empty() {
            format!("Search query: '{}'\nNo results found.", query)
        } else {
            let mut content = format!("Search query: '{}'\nResults:\n\n", query);
            for (i, doc) in documents.iter().take(5).enumerate() {
                content.push_str(&format!(
                    "{}. {}\n   {}\n   Rank: {}\n\n",
                    i + 1,
                    doc.title,
                    doc.description.as_deref().unwrap_or("No description"),
                    doc.rank.unwrap_or(0)
                ));
            }
            if documents.len() > 5 {
                content.push_str(&format!("... and {} more results\n", documents.len() - 5));
            }
            content
        };

        Self {
            id: uuid::Uuid::new_v4().to_string(),
            context_type: ContextType::Document, // Changed from SearchResult to Document
            title: format!("Search: {}", query),
            summary: Some(format!(
                "Search results for '{}' - {} documents found",
                query,
                documents.len()
            )),
            content,
            metadata,
            created_at: chrono::Utc::now(),
            relevance_score: documents.first().and_then(|d| d.rank.map(|r| r as f64)),
        }
    }

    /// Create a new context item from a KG term definition
    pub fn from_kg_term_definition(kg_term: &KGTermDefinition) -> Self {
        let mut metadata = AHashMap::new();
        metadata.insert("source_type".to_string(), "kg_term".to_string());
        metadata.insert("term_id".to_string(), kg_term.id.to_string());
        metadata.insert(
            "normalized_term".to_string(),
            kg_term.normalized_term.to_string(),
        );
        metadata.insert(
            "synonyms_count".to_string(),
            kg_term.synonyms.len().to_string(),
        );
        metadata.insert(
            "related_terms_count".to_string(),
            kg_term.related_terms.len().to_string(),
        );
        metadata.insert(
            "usage_examples_count".to_string(),
            kg_term.usage_examples.len().to_string(),
        );

        if let Some(ref url) = kg_term.url {
            metadata.insert("url".to_string(), url.clone());
        }

        // Add KG-specific metadata
        for (key, value) in &kg_term.metadata {
            metadata.insert(format!("kg_{}", key), value.clone());
        }

        let mut content = format!("**Term:** {}\n", kg_term.term);

        if let Some(ref definition) = kg_term.definition {
            content.push_str(&format!("**Definition:** {}\n", definition));
        }

        if !kg_term.synonyms.is_empty() {
            content.push_str(&format!("**Synonyms:** {}\n", kg_term.synonyms.join(", ")));
        }

        if !kg_term.related_terms.is_empty() {
            content.push_str(&format!(
                "**Related Terms:** {}\n",
                kg_term.related_terms.join(", ")
            ));
        }

        if !kg_term.usage_examples.is_empty() {
            content.push_str("**Usage Examples:**\n");
            for (i, example) in kg_term.usage_examples.iter().enumerate() {
                content.push_str(&format!("{}. {}\n", i + 1, example));
            }
        }

        Self {
            id: uuid::Uuid::new_v4().to_string(),
            context_type: ContextType::KGTermDefinition,
            title: format!("KG Term: {}", kg_term.term),
            summary: Some(format!(
                "Knowledge Graph term '{}' with {} synonyms and {} related terms",
                kg_term.term,
                kg_term.synonyms.len(),
                kg_term.related_terms.len()
            )),
            content,
            metadata,
            created_at: chrono::Utc::now(),
            relevance_score: kg_term.relevance_score,
        }
    }

    /// Create a new context item from a complete KG index
    pub fn from_kg_index(kg_index: &KGIndexInfo) -> Self {
        let mut metadata = AHashMap::new();
        metadata.insert("source_type".to_string(), "kg_index".to_string());
        metadata.insert("kg_name".to_string(), kg_index.name.clone());
        metadata.insert("total_terms".to_string(), kg_index.total_terms.to_string());
        metadata.insert("total_nodes".to_string(), kg_index.total_nodes.to_string());
        metadata.insert("total_edges".to_string(), kg_index.total_edges.to_string());
        metadata.insert("source".to_string(), kg_index.source.clone());
        metadata.insert(
            "last_updated".to_string(),
            kg_index.last_updated.to_rfc3339(),
        );

        if let Some(ref version) = kg_index.version {
            metadata.insert("version".to_string(), version.clone());
        }

        let content = format!(
            "**Knowledge Graph Index: {}**\n\n\
            **Statistics:**\n\
            - Total Terms: {}\n\
            - Total Nodes: {}\n\
            - Total Edges: {}\n\
            - Source: {}\n\
            - Last Updated: {}\n\
            - Version: {}\n\n\
            This context includes the complete knowledge graph index with all terms, \
            relationships, and metadata available for reference.",
            kg_index.name,
            kg_index.total_terms,
            kg_index.total_nodes,
            kg_index.total_edges,
            kg_index.source,
            kg_index.last_updated.format("%Y-%m-%d %H:%M:%S UTC"),
            kg_index.version.as_deref().unwrap_or("N/A")
        );

        Self {
            id: uuid::Uuid::new_v4().to_string(),
            context_type: ContextType::KGIndex,
            title: format!("KG Index: {}", kg_index.name),
            summary: Some(format!(
                "Complete knowledge graph index with {} terms, {} nodes, and {} edges",
                kg_index.total_terms, kg_index.total_nodes, kg_index.total_edges
            )),
            content,
            metadata,
            created_at: chrono::Utc::now(),
            relevance_score: Some(1.0), // High relevance for complete index
        }
    }
}

/// Knowledge Graph term definition with comprehensive metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(Tsify))]
#[cfg_attr(feature = "typescript", tsify(into_wasm_abi, from_wasm_abi))]
pub struct KGTermDefinition {
    /// The primary term
    pub term: String,
    /// Normalized term value
    pub normalized_term: NormalizedTermValue,
    /// Unique identifier for the term
    pub id: u64,
    /// Definition of the term
    pub definition: Option<String>,
    /// Synonyms for the term
    pub synonyms: Vec<String>,
    /// Related terms
    pub related_terms: Vec<String>,
    /// Usage examples
    pub usage_examples: Vec<String>,
    /// URL reference if available
    pub url: Option<String>,
    /// Additional metadata
    pub metadata: AHashMap<String, String>,
    /// Relevance score for ranking
    pub relevance_score: Option<f64>,
}

/// Knowledge Graph index information
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(Tsify))]
#[cfg_attr(feature = "typescript", tsify(into_wasm_abi, from_wasm_abi))]
pub struct KGIndexInfo {
    /// Name of the knowledge graph
    pub name: String,
    /// Total number of terms in the index
    pub total_terms: usize,
    /// Number of nodes in the graph
    pub total_nodes: usize,
    /// Number of edges in the graph
    pub total_edges: usize,
    /// Last updated timestamp
    pub last_updated: chrono::DateTime<chrono::Utc>,
    /// Source of the knowledge graph
    pub source: String,
    /// Version of the knowledge graph
    pub version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(Tsify))]
#[cfg_attr(feature = "typescript", tsify(into_wasm_abi, from_wasm_abi))]
pub struct ChatMessage {
    /// Unique identifier for this message
    pub id: MessageId,
    /// Role of the message sender
    pub role: String, // "system" | "user" | "assistant"
    /// The message content
    pub content: String,
    /// Context items associated with this message
    pub context_items: Vec<ContextItem>,
    /// Timestamp when the message was created
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// Token count for this message (if available)
    pub token_count: Option<u32>,
    /// Model used to generate this message (for assistant messages)
    pub model: Option<String>,
}

impl ChatMessage {
    /// Create a new user message
    pub fn user(content: String) -> Self {
        Self {
            id: MessageId::new(),
            role: "user".to_string(),
            content,
            context_items: Vec::new(),
            created_at: chrono::Utc::now(),
            token_count: None,
            model: None,
        }
    }

    /// Create a new assistant message
    pub fn assistant(content: String, model: Option<String>) -> Self {
        Self {
            id: MessageId::new(),
            role: "assistant".to_string(),
            content,
            context_items: Vec::new(),
            created_at: chrono::Utc::now(),
            token_count: None,
            model,
        }
    }

    /// Create a new system message
    pub fn system(content: String) -> Self {
        Self {
            id: MessageId::new(),
            role: "system".to_string(),
            content,
            context_items: Vec::new(),
            created_at: chrono::Utc::now(),
            token_count: None,
            model: None,
        }
    }

    /// Add context item to this message
    pub fn add_context(&mut self, context: ContextItem) {
        self.context_items.push(context);
    }

    /// Add multiple context items to this message
    pub fn add_contexts(&mut self, contexts: Vec<ContextItem>) {
        self.context_items.extend(contexts);
    }
}

/// A conversation containing multiple messages and context
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(Tsify))]
#[cfg_attr(feature = "typescript", tsify(into_wasm_abi, from_wasm_abi))]
pub struct Conversation {
    /// Unique identifier for this conversation
    pub id: ConversationId,
    /// Human-readable title for the conversation
    pub title: String,
    /// Messages in this conversation
    pub messages: Vec<ChatMessage>,
    /// Global context items for the entire conversation
    pub global_context: Vec<ContextItem>,
    /// Role used for this conversation
    pub role: RoleName,
    /// When this conversation was created
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// When this conversation was last updated
    pub updated_at: chrono::DateTime<chrono::Utc>,
    /// Metadata about the conversation
    pub metadata: AHashMap<String, String>,
}

impl Conversation {
    /// Create a new conversation
    pub fn new(title: String, role: RoleName) -> Self {
        let now = chrono::Utc::now();
        Self {
            id: ConversationId::new(),
            title,
            messages: Vec::new(),
            global_context: Vec::new(),
            role,
            created_at: now,
            updated_at: now,
            metadata: AHashMap::new(),
        }
    }

    /// Add a message to the conversation
    pub fn add_message(&mut self, message: ChatMessage) {
        self.messages.push(message);
        self.updated_at = chrono::Utc::now();
    }

    /// Add global context to the conversation
    pub fn add_global_context(&mut self, context: ContextItem) {
        self.global_context.push(context);
        self.updated_at = chrono::Utc::now();
    }

    /// Get the total context length (approximation)
    pub fn estimated_context_length(&self) -> usize {
        let message_length: usize = self
            .messages
            .iter()
            .map(|m| {
                m.content.len()
                    + m.context_items
                        .iter()
                        .map(|c| c.content.len())
                        .sum::<usize>()
            })
            .sum();
        let global_context_length: usize =
            self.global_context.iter().map(|c| c.content.len()).sum();
        message_length + global_context_length
    }
}

/// Summary of a conversation for listing purposes
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(Tsify))]
#[cfg_attr(feature = "typescript", tsify(into_wasm_abi, from_wasm_abi))]
pub struct ConversationSummary {
    /// Unique identifier for this conversation
    pub id: ConversationId,
    /// Human-readable title for the conversation
    pub title: String,
    /// Role used for this conversation
    pub role: RoleName,
    /// Number of messages in the conversation
    pub message_count: usize,
    /// Number of context items in the conversation
    pub context_count: usize,
    /// When this conversation was created
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// When this conversation was last updated
    pub updated_at: chrono::DateTime<chrono::Utc>,
    /// Preview of the first user message (if any)
    pub preview: Option<String>,
}

// Note: Persistable implementation for Conversation will be added in the service layer
// to avoid circular dependencies

impl From<&Conversation> for ConversationSummary {
    fn from(conversation: &Conversation) -> Self {
        let context_count = conversation.global_context.len()
            + conversation
                .messages
                .iter()
                .map(|m| m.context_items.len())
                .sum::<usize>();

        let preview = conversation
            .messages
            .iter()
            .find(|m| m.role == "user")
            .map(|m| {
                if m.content.len() > 100 {
                    format!("{}...", &m.content[..100])
                } else {
                    m.content.clone()
                }
            });

        Self {
            id: conversation.id.clone(),
            title: conversation.title.clone(),
            role: conversation.role.clone(),
            message_count: conversation.messages.len(),
            context_count,
            created_at: conversation.created_at,
            updated_at: conversation.updated_at,
            preview,
        }
    }
}

/// Context history that tracks what context has been used across conversations
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(Tsify))]
#[cfg_attr(feature = "typescript", tsify(into_wasm_abi, from_wasm_abi))]
pub struct ContextHistory {
    /// Items that have been used in conversations
    pub used_contexts: Vec<ContextHistoryEntry>,
    /// Maximum number of history entries to keep
    pub max_entries: usize,
}

impl ContextHistory {
    pub fn new(max_entries: usize) -> Self {
        Self {
            used_contexts: Vec::new(),
            max_entries,
        }
    }

    /// Record that a context item was used
    pub fn record_usage(
        &mut self,
        context_id: &str,
        conversation_id: &ConversationId,
        usage_type: ContextUsageType,
    ) {
        let entry = ContextHistoryEntry {
            context_id: context_id.to_string(),
            conversation_id: conversation_id.clone(),
            usage_type,
            used_at: chrono::Utc::now(),
            usage_count: 1,
        };

        // Check if we already have this context for this conversation
        if let Some(existing) = self
            .used_contexts
            .iter_mut()
            .find(|e| e.context_id == context_id && e.conversation_id == *conversation_id)
        {
            existing.usage_count += 1;
            existing.used_at = chrono::Utc::now();
        } else {
            self.used_contexts.push(entry);
        }

        // Trim to max entries if needed
        if self.used_contexts.len() > self.max_entries {
            self.used_contexts.sort_by_key(|e| e.used_at);
            self.used_contexts
                .drain(0..self.used_contexts.len() - self.max_entries);
        }
    }

    /// Get frequently used contexts
    pub fn get_frequent_contexts(&self, limit: usize) -> Vec<&ContextHistoryEntry> {
        let mut entries = self.used_contexts.iter().collect::<Vec<_>>();
        entries.sort_by_key(|e| std::cmp::Reverse(e.usage_count));
        entries.into_iter().take(limit).collect()
    }
}

/// Entry in context usage history
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(Tsify))]
#[cfg_attr(feature = "typescript", tsify(into_wasm_abi, from_wasm_abi))]
pub struct ContextHistoryEntry {
    /// ID of the context item that was used
    pub context_id: String,
    /// Conversation where it was used
    pub conversation_id: ConversationId,
    /// How the context was used
    pub usage_type: ContextUsageType,
    /// When it was used
    pub used_at: chrono::DateTime<chrono::Utc>,
    /// How many times it's been used in this conversation
    pub usage_count: usize,
}

/// How a context item was used
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "typescript", derive(Tsify))]
#[cfg_attr(feature = "typescript", tsify(into_wasm_abi, from_wasm_abi))]
pub enum ContextUsageType {
    /// Added manually by user
    Manual,
    /// Added automatically by system
    Automatic,
    /// Added from search results
    SearchResult,
    /// Added from document reference
    DocumentReference,
}

// Routing and Priority Types

/// Priority level for routing rules and decisions
/// Higher numeric values indicate higher priority
#[derive(
    Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, JsonSchema, Default,
)]
#[cfg_attr(feature = "typescript", derive(Tsify))]
#[cfg_attr(feature = "typescript", tsify(into_wasm_abi, from_wasm_abi))]
pub struct Priority(pub u8);

impl Priority {
    /// Create a new priority with the given value
    pub fn new(value: u8) -> Self {
        Self(value.clamp(0, 100))
    }

    /// Get the priority value
    pub fn value(&self) -> u8 {
        self.0
    }

    /// Check if this is high priority (>= 80)
    pub fn is_high(&self) -> bool {
        self.0 >= 80
    }

    /// Check if this is medium priority (>= 40 && < 80)
    pub fn is_medium(&self) -> bool {
        self.0 >= 40 && self.0 < 80
    }

    /// Check if this is low priority (< 40)
    pub fn is_low(&self) -> bool {
        self.0 < 40
    }

    /// Maximum priority value
    pub const MAX: Self = Self(100);

    /// High priority (default for fast/expensive rules)
    pub const HIGH: Self = Self(80);

    /// Medium priority (default for standard rules)
    pub const MEDIUM: Self = Self(50);

    /// Low priority (default for fallback rules)
    pub const LOW: Self = Self(20);

    /// Minimum priority value
    pub const MIN: Self = Self(0);
}

impl fmt::Display for Priority {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<u8> for Priority {
    fn from(value: u8) -> Self {
        Self::new(value)
    }
}

impl From<i32> for Priority {
    fn from(value: i32) -> Self {
        Self::new(value as u8)
    }
}

/// A routing rule with pattern matching and priority
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "typescript", derive(Tsify))]
#[cfg_attr(feature = "typescript", tsify(into_wasm_abi, from_wasm_abi))]
pub struct RoutingRule {
    /// Unique identifier for this rule
    pub id: String,

    /// Name of the rule (human-readable)
    pub name: String,

    /// Pattern to match (can be regex, exact string, or concept name)
    pub pattern: String,

    /// Priority of this rule (higher = more important)
    pub priority: Priority,

    /// Provider to route to when this rule matches
    pub provider: String,

    /// Model to use when this rule matches
    pub model: String,

    /// Optional description of when this rule applies
    pub description: Option<String>,

    /// Tags for categorizing rules
    pub tags: Vec<String>,

    /// Whether this rule is enabled
    pub enabled: bool,

    /// When this rule was created
    pub created_at: chrono::DateTime<chrono::Utc>,

    /// When this rule was last updated
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl RoutingRule {
    /// Create a new routing rule
    pub fn new(
        id: String,
        name: String,
        pattern: String,
        priority: Priority,
        provider: String,
        model: String,
    ) -> Self {
        let now = chrono::Utc::now();
        Self {
            id,
            name,
            pattern,
            priority,
            provider,
            model,
            description: None,
            tags: Vec::new(),
            enabled: true,
            created_at: now,
            updated_at: now,
        }
    }

    /// Create a rule with default medium priority
    pub fn with_defaults(
        id: String,
        name: String,
        pattern: String,
        provider: String,
        model: String,
    ) -> Self {
        Self::new(id, name, pattern, Priority::MEDIUM, provider, model)
    }

    /// Set the description
    pub fn with_description(mut self, description: String) -> Self {
        self.description = Some(description);
        self
    }

    /// Add a tag
    pub fn with_tag(mut self, tag: String) -> Self {
        self.tags.push(tag);
        self
    }

    /// Set enabled status
    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Update the rule's timestamp
    pub fn touch(&mut self) {
        self.updated_at = chrono::Utc::now();
    }
}

/// Result of pattern matching with priority scoring
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "typescript", derive(Tsify))]
#[cfg_attr(feature = "typescript", tsify(into_wasm_abi, from_wasm_abi))]
pub struct PatternMatch {
    /// The concept that was matched
    pub concept: String,

    /// Provider to route to
    pub provider: String,

    /// Model to use
    pub model: String,

    /// Match score (0.0 to 1.0)
    pub score: f64,

    /// Priority of the matched rule
    pub priority: Priority,

    /// Combined weighted score (score * priority_factor)
    pub weighted_score: f64,

    /// The rule that was matched
    pub rule_id: String,
}

impl PatternMatch {
    /// Create a new pattern match
    pub fn new(
        concept: String,
        provider: String,
        model: String,
        score: f64,
        priority: Priority,
        rule_id: String,
    ) -> Self {
        let priority_factor = priority.value() as f64 / 100.0;
        let weighted_score = score * priority_factor;

        Self {
            concept,
            provider,
            model,
            score,
            priority,
            weighted_score,
            rule_id,
        }
    }

    /// Create a simple pattern match with default priority
    pub fn simple(concept: String, provider: String, model: String, score: f64) -> Self {
        Self::new(
            concept,
            provider,
            model,
            score,
            Priority::MEDIUM,
            "default".to_string(),
        )
    }
}

/// Routing decision with priority information
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "typescript", derive(Tsify))]
#[cfg_attr(feature = "typescript", tsify(into_wasm_abi, from_wasm_abi))]
pub struct RoutingDecision {
    /// Provider to route to
    pub provider: String,

    /// Model to use
    pub model: String,

    /// The scenario that was matched
    pub scenario: RoutingScenario,

    /// Priority of this decision
    pub priority: Priority,

    /// Confidence score (0.0 to 1.0)
    pub confidence: f64,

    /// The rule that led to this decision (if any)
    pub rule_id: Option<String>,

    /// Reason for this decision
    pub reason: String,
}

impl RoutingDecision {
    /// Create a new routing decision
    pub fn new(
        provider: String,
        model: String,
        scenario: RoutingScenario,
        priority: Priority,
        confidence: f64,
        reason: String,
    ) -> Self {
        Self {
            provider,
            model,
            scenario,
            priority,
            confidence,
            rule_id: None,
            reason,
        }
    }

    /// Create a decision with a specific rule
    pub fn with_rule(
        provider: String,
        model: String,
        scenario: RoutingScenario,
        priority: Priority,
        confidence: f64,
        rule_id: String,
        reason: String,
    ) -> Self {
        Self {
            provider,
            model,
            scenario,
            priority,
            confidence,
            rule_id: Some(rule_id),
            reason,
        }
    }

    /// Create a simple default decision
    pub fn default(provider: String, model: String) -> Self {
        Self::new(
            provider,
            model,
            RoutingScenario::Default,
            Priority::LOW,
            0.5,
            "Default routing".to_string(),
        )
    }
}

/// Routing scenario types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, JsonSchema, Default)]
#[cfg_attr(feature = "typescript", derive(Tsify))]
#[cfg_attr(feature = "typescript", tsify(into_wasm_abi, from_wasm_abi))]
pub enum RoutingScenario {
    /// Default routing scenario
    #[serde(rename = "default")]
    #[default]
    Default,

    /// Background processing (low priority, cost-optimized)
    #[serde(rename = "background")]
    Background,

    /// Thinking/reasoning tasks (high quality)
    #[serde(rename = "think")]
    Think,

    /// Long context tasks
    #[serde(rename = "long_context")]
    LongContext,

    /// Web search required
    #[serde(rename = "web_search")]
    WebSearch,

    /// Image processing required
    #[serde(rename = "image")]
    Image,

    /// Pattern-based routing with concept name
    #[serde(rename = "pattern")]
    Pattern(String),

    /// Priority-based routing
    #[serde(rename = "priority")]
    Priority,

    /// Custom scenario
    #[serde(rename = "custom")]
    Custom(String),
}

impl fmt::Display for RoutingScenario {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Default => write!(f, "default"),
            Self::Background => write!(f, "background"),
            Self::Think => write!(f, "think"),
            Self::LongContext => write!(f, "long_context"),
            Self::WebSearch => write!(f, "web_search"),
            Self::Image => write!(f, "image"),
            Self::Pattern(concept) => write!(f, "pattern:{}", concept),
            Self::Priority => write!(f, "priority"),
            Self::Custom(name) => write!(f, "custom:{}", name),
        }
    }
}

/// Multi-agent context for coordinating between different AI agents
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(Tsify))]
#[cfg_attr(feature = "typescript", tsify(into_wasm_abi, from_wasm_abi))]
pub struct MultiAgentContext {
    /// Unique identifier for the multi-agent session
    pub session_id: String,
    /// Agents participating in this context
    pub agents: Vec<AgentInfo>,
    /// Shared context items available to all agents
    pub shared_context: Vec<ContextItem>,
    /// Agent-specific context
    pub agent_contexts: AHashMap<String, Vec<ContextItem>>,
    /// Communication log between agents
    pub agent_communications: Vec<AgentCommunication>,
    /// When this session was created
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// When this session was last updated
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl MultiAgentContext {
    pub fn new() -> Self {
        let now = chrono::Utc::now();
        Self {
            session_id: uuid::Uuid::new_v4().to_string(),
            agents: Vec::new(),
            shared_context: Vec::new(),
            agent_contexts: AHashMap::new(),
            agent_communications: Vec::new(),
            created_at: now,
            updated_at: now,
        }
    }

    /// Add an agent to the session
    pub fn add_agent(&mut self, agent: AgentInfo) {
        self.agents.push(agent.clone());
        self.agent_contexts.insert(agent.id, Vec::new());
        self.updated_at = chrono::Utc::now();
    }

    /// Add context for a specific agent
    pub fn add_agent_context(&mut self, agent_id: &str, context: ContextItem) {
        if let Some(contexts) = self.agent_contexts.get_mut(agent_id) {
            contexts.push(context);
            self.updated_at = chrono::Utc::now();
        }
    }

    /// Record communication between agents
    pub fn record_communication(
        &mut self,
        from_agent: &str,
        to_agent: Option<&str>,
        message: String,
    ) {
        let communication = AgentCommunication {
            from_agent: from_agent.to_string(),
            to_agent: to_agent.map(|s| s.to_string()),
            message,
            timestamp: chrono::Utc::now(),
        };
        self.agent_communications.push(communication);
        self.updated_at = chrono::Utc::now();
    }
}

impl Default for MultiAgentContext {
    fn default() -> Self {
        Self::new()
    }
}

/// Information about an AI agent in a multi-agent context
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(Tsify))]
#[cfg_attr(feature = "typescript", tsify(into_wasm_abi, from_wasm_abi))]
pub struct AgentInfo {
    /// Unique identifier for the agent
    pub id: String,
    /// Human-readable name of the agent
    pub name: String,
    /// Role/specialty of the agent
    pub role: String,
    /// Capabilities or description of what this agent does
    pub capabilities: Vec<String>,
    /// Model or provider powering this agent
    pub model: Option<String>,
}

/// Communication between agents in a multi-agent context
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(Tsify))]
#[cfg_attr(feature = "typescript", tsify(into_wasm_abi, from_wasm_abi))]
pub struct AgentCommunication {
    /// ID of the agent sending the message
    pub from_agent: String,
    /// ID of the agent receiving the message (None for broadcast)
    pub to_agent: Option<String>,
    /// The communication message
    pub message: String,
    /// When this communication occurred
    pub timestamp: chrono::DateTime<chrono::Utc>,
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

    #[test]
    fn test_priority_creation_and_comparison() {
        let high = Priority::HIGH;
        let medium = Priority::MEDIUM;
        let low = Priority::LOW;
        let custom = Priority::new(75);

        assert_eq!(high.value(), 80);
        assert_eq!(medium.value(), 50);
        assert_eq!(low.value(), 20);
        assert_eq!(custom.value(), 75);

        assert!(high.is_high());
        assert!(!medium.is_high());
        assert!(medium.is_medium());
        assert!(low.is_low());

        // Test ordering
        assert!(high > medium);
        assert!(medium > low);
        assert!(custom > medium);
        assert!(custom < high);

        // Test bounds
        let max = Priority::new(150);
        assert_eq!(max.value(), 100);
        let min = Priority::new(0);
        assert_eq!(min.value(), 0);
    }

    #[test]
    fn test_routing_rule_creation() {
        let rule = RoutingRule::new(
            "test-rule".to_string(),
            "Test Rule".to_string(),
            "test.*pattern".to_string(),
            Priority::HIGH,
            "openai".to_string(),
            "gpt-4".to_string(),
        )
        .with_description("A test rule for unit testing".to_string())
        .with_tag("test".to_string())
        .with_tag("example".to_string());

        assert_eq!(rule.id, "test-rule");
        assert_eq!(rule.name, "Test Rule");
        assert_eq!(rule.pattern, "test.*pattern");
        assert_eq!(rule.priority, Priority::HIGH);
        assert_eq!(rule.provider, "openai");
        assert_eq!(rule.model, "gpt-4");
        assert_eq!(
            rule.description,
            Some("A test rule for unit testing".to_string())
        );
        assert_eq!(rule.tags, vec!["test", "example"]);
        assert!(rule.enabled);
    }

    #[test]
    fn test_routing_rule_defaults() {
        let rule = RoutingRule::with_defaults(
            "default-rule".to_string(),
            "Default Rule".to_string(),
            "default".to_string(),
            "anthropic".to_string(),
            "claude-3-sonnet".to_string(),
        );

        assert_eq!(rule.priority, Priority::MEDIUM);
        assert!(rule.enabled);
        assert!(rule.tags.is_empty());
        assert!(rule.description.is_none());
    }

    #[test]
    fn test_pattern_match() {
        let pattern_match = PatternMatch::new(
            "machine-learning".to_string(),
            "openai".to_string(),
            "gpt-4".to_string(),
            0.95,
            Priority::HIGH,
            "ml-rule".to_string(),
        );

        assert_eq!(pattern_match.concept, "machine-learning");
        assert_eq!(pattern_match.provider, "openai");
        assert_eq!(pattern_match.model, "gpt-4");
        assert_eq!(pattern_match.score, 0.95);
        assert_eq!(pattern_match.priority, Priority::HIGH);
        assert_eq!(pattern_match.rule_id, "ml-rule");

        // Weighted score should be score * priority_factor
        assert_eq!(pattern_match.weighted_score, 0.95 * 0.8);
    }

    #[test]
    fn test_pattern_match_simple() {
        let simple = PatternMatch::simple(
            "test".to_string(),
            "anthropic".to_string(),
            "claude-3-haiku".to_string(),
            0.8,
        );

        assert_eq!(simple.priority, Priority::MEDIUM);
        assert_eq!(simple.rule_id, "default");
        assert_eq!(simple.weighted_score, 0.8 * 0.5);
    }

    #[test]
    fn test_routing_decision() {
        let decision = RoutingDecision::new(
            "openai".to_string(),
            "gpt-4".to_string(),
            RoutingScenario::Think,
            Priority::HIGH,
            0.9,
            "High priority thinking task".to_string(),
        );

        assert_eq!(decision.provider, "openai");
        assert_eq!(decision.model, "gpt-4");
        assert_eq!(decision.scenario, RoutingScenario::Think);
        assert_eq!(decision.priority, Priority::HIGH);
        assert_eq!(decision.confidence, 0.9);
        assert_eq!(decision.reason, "High priority thinking task");
        assert!(decision.rule_id.is_none());
    }

    #[test]
    fn test_routing_decision_with_rule() {
        let decision = RoutingDecision::with_rule(
            "anthropic".to_string(),
            "claude-3-sonnet".to_string(),
            RoutingScenario::Pattern("web-search".to_string()),
            Priority::MEDIUM,
            0.85,
            "web-rule".to_string(),
            "Web search pattern matched".to_string(),
        );

        assert_eq!(decision.rule_id, Some("web-rule".to_string()));
        assert_eq!(
            decision.scenario,
            RoutingScenario::Pattern("web-search".to_string())
        );
    }

    #[test]
    fn test_routing_decision_default() {
        let default = RoutingDecision::default("openai".to_string(), "gpt-3.5-turbo".to_string());

        assert_eq!(default.provider, "openai");
        assert_eq!(default.model, "gpt-3.5-turbo");
        assert_eq!(default.scenario, RoutingScenario::Default);
        assert_eq!(default.priority, Priority::LOW);
        assert_eq!(default.confidence, 0.5);
        assert_eq!(default.reason, "Default routing");
    }

    #[test]
    fn test_routing_scenario_serialization() {
        let scenarios = vec![
            RoutingScenario::Default,
            RoutingScenario::Background,
            RoutingScenario::Think,
            RoutingScenario::LongContext,
            RoutingScenario::WebSearch,
            RoutingScenario::Image,
            RoutingScenario::Pattern("test".to_string()),
            RoutingScenario::Priority,
            RoutingScenario::Custom("special".to_string()),
        ];

        for scenario in scenarios {
            let json = serde_json::to_string(&scenario).unwrap();
            let deserialized: RoutingScenario = serde_json::from_str(&json).unwrap();
            assert_eq!(scenario, deserialized);
        }
    }

    #[test]
    fn test_routing_scenario_display() {
        assert_eq!(format!("{}", RoutingScenario::Default), "default");
        assert_eq!(format!("{}", RoutingScenario::Think), "think");
        assert_eq!(
            format!("{}", RoutingScenario::Pattern("ml".to_string())),
            "pattern:ml"
        );
        assert_eq!(
            format!("{}", RoutingScenario::Custom("test".to_string())),
            "custom:test"
        );
    }

    #[test]
    fn test_priority_serialization() {
        let priority = Priority::new(75);
        let json = serde_json::to_string(&priority).unwrap();
        let deserialized: Priority = serde_json::from_str(&json).unwrap();
        assert_eq!(priority, deserialized);
        assert_eq!(deserialized.value(), 75);
    }

    #[test]
    fn test_routing_rule_serialization() {
        let rule = RoutingRule::new(
            "serialize-test".to_string(),
            "Serialize Test".to_string(),
            "test-pattern".to_string(),
            Priority::MEDIUM,
            "provider".to_string(),
            "model".to_string(),
        );

        let json = serde_json::to_string(&rule).unwrap();
        let deserialized: RoutingRule = serde_json::from_str(&json).unwrap();
        assert_eq!(rule.id, deserialized.id);
        assert_eq!(rule.name, deserialized.name);
        assert_eq!(rule.priority, deserialized.priority);
        assert_eq!(rule.provider, deserialized.provider);
        assert_eq!(rule.model, deserialized.model);
    }
}
