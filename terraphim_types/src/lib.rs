use std::fmt::{self, Display, Formatter};
use std::ops::{Add, AddAssign};

use ahash::AHashMap;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::hash_map::Iter;
use std::iter::IntoIterator;
use ulid::Ulid;

/// A unique ID. The underlying type is an implementation detail and subject to change.
///
/// Currently, this is a wrapper around the `ulid` crate's `Ulid` type.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Id {
    /// A Ulid is a unique 128-bit lexicographically sortable identifier
    ulid: Ulid,
    // Bytes of the ULID (for use in the `AsRef<[u8]>` trait, which is used in
    // aho-corasick)
    bytes: Vec<u8>,
}

impl Id {
    pub fn new() -> Self {
        let ulid = Ulid::new();
        let bytes = ulid.to_string().into_bytes();
        Id { ulid, bytes }
    }

    pub fn as_u128(&self) -> u128 {
        self.ulid.into()
    }
}

impl Default for Id {
    fn default() -> Self {
        Self::new()
    }
}

// Custom Debug implementation to hide the `bytes` field
impl fmt::Debug for Id {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Convert the ulid to its u128 representation
        let ulid_as_u128: u128 = self.ulid.into();
        write!(f, "{}", ulid_as_u128)
    }
}

impl AsRef<[u8]> for Id {
    fn as_ref(&self) -> &[u8] {
        &self.bytes
    }
}

impl From<Ulid> for Id {
    fn from(ulid: Ulid) -> Self {
        Self {
            ulid,
            bytes: ulid.to_string().into_bytes(),
        }
    }
}

impl From<u128> for Id {
    fn from(id: u128) -> Self {
        let ulid = Ulid::from(id);
        Self {
            ulid,
            bytes: ulid.to_string().into_bytes(),
        }
    }
}

impl Display for Id {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.ulid)
    }
}

impl Serialize for Id {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // Serialize the ulid field as u128
        serializer.serialize_u128(self.ulid.0)
    }
}

impl<'de> Deserialize<'de> for Id {
    fn deserialize<D>(deserializer: D) -> Result<Id, D::Error>
    where
        D: Deserializer<'de>,
    {
        // We expect the ULID to be serialized as a u128
        let ulid = u128::deserialize(deserializer)?;
        Ok(Id::from(ulid))

        // // TODO
        // // let ulid = Ulid::deserialize(deserializer)?;
        // // Ok(Id::from(ulid))

        // Ok(Id::new())
    }
}

/// The value of a normalized term
///
/// This is a string that has been normalized to lowercase and trimmed.
#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NormalizedTermValue(String);

impl NormalizedTermValue {
    pub fn new(term: String) -> Self {
        let value = term.trim().to_lowercase();
        Self(value)
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

/// A normalized term is a higher-level term that has been normalized
///
/// It holds a unique identifier to an underlying and the normalized value.
#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct NormalizedTerm {
    /// Unique identifier for the normalized term
    pub id: Id,
    /// The normalized value
    // This field is currently called `nterm` in the JSON
    #[serde(rename = "nterm")]
    pub value: NormalizedTermValue,
}

impl NormalizedTerm {
    pub fn new(id: Id, value: NormalizedTermValue) -> Self {
        Self { id, value }
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
    pub id: Id,
    /// The normalized concept
    pub value: NormalizedTermValue,
}

impl Concept {
    pub fn new(value: NormalizedTermValue) -> Self {
        Self {
            id: Id::new(),
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

/// Rank of an article in the search results
#[derive(Debug, Copy, Clone, PartialOrd, Ord, Serialize, Deserialize, PartialEq, Eq)]
pub struct Rank(u64);

impl Rank {
    /// Create a new rank with a given value
    pub fn new(rank: u64) -> Self {
        Self(rank)
    }
}

impl Add for Rank {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self(self.0 + other.0)
    }
}

// Implement +=
impl Add for &mut Rank {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        self.0 += other.0;
        self
    }
}

impl AddAssign for Rank {
    fn add_assign(&mut self, other: Self) {
        self.0 += other.0;
    }
}

impl Display for Rank {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// An article is a piece of content that can be indexed and searched.
///
/// Article that can be indexed by the `RoleGraph`.
///
/// It holds the title, body, description, tags, and rank.
/// The `id` is a unique identifier for the article.
#[derive(Deserialize, Serialize, Debug, Clone, Default)]
pub struct Article {
    /// Unique identifier for the article
    pub id: String,
    /// Title of the article
    pub title: String,
    /// The article body
    pub body: String,
    /// A short description of the article
    pub description: Option<String>,

    /// A short excerpt of the article
    pub stub: Option<String>,
    /// URL of the article
    pub url: String,
    /// Tags for the article
    pub tags: Option<Vec<String>>,
    /// Rank of the article in the search results
    pub rank: Option<Rank>,
}

impl ToString for Article {
    fn to_string(&self) -> String {
        let mut text = String::new();
        text.push_str(&self.title);
        text.push_str(&self.body);
        if let Some(description) = &self.description {
            text.push_str(description);
        }
        text
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Edge {
    /// ID of the edge
    pub id: Id,
    /// Rank of the edge
    pub rank: u64,
    /// A hashmap of `article_id` to `rank`
    pub doc_hash: AHashMap<String, u64>,
}

impl Edge {
    pub fn new(id: Id, article_id: String) -> Self {
        let mut doc_hash = AHashMap::new();
        doc_hash.insert(article_id, 1);
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
    pub id: Id,
    /// Number of co-occurrences
    pub rank: u64,
    /// List of connected nodes
    pub connected_with: Vec<Id>,
}

impl Node {
    /// Create a new node with a given id and edge
    pub fn new(id: Id, edge: Edge) -> Self {
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
    //     // log::warn!("k {:?} v {:?}", k, v);
    //     // }
    //     log::warn!("Connected with {:?}", self.connected_with);
    // }
}

/// A thesaurus is a dictionary with synonyms which map to upper-level concepts.
///
/// It holds the normalized terms for a resource
/// where a resource can be as diverse as a Markdown file or an article in
/// Notion or AtomicServer
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct Thesaurus {
    /// Name of the thesaurus
    pub name: String,
    /// The inner hashmap of normalized terms
    inner: AHashMap<NormalizedTermValue, NormalizedTerm>,
}

impl Thesaurus {
    /// Create a new, empty thesaurus
    pub fn new(name: String) -> Self {
        Self {
            name,
            inner: AHashMap::new(),
        }
    }

    /// Inserts a key-value pair into the thesaurus.
    pub fn insert(&mut self, key: NormalizedTermValue, value: NormalizedTerm) {
        self.inner.insert(key, value);
    }

    /// Get the length of the thesaurus
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// Check if the thesaurus is empty
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Custom `get` method for the thesaurus, which accepts a
    /// `NormalizedTermValue` and returns a reference to the
    /// `NormalizedTerm`.
    pub fn get(&self, key: &NormalizedTermValue) -> Option<&NormalizedTerm> {
        self.inner.get(key)
    }

    pub fn keys(&self) -> std::collections::hash_map::Keys<NormalizedTermValue, NormalizedTerm> {
        self.inner.keys()
    }
}

// Implement `IntoIterator` for a reference to `Thesaurus`
impl<'a> IntoIterator for &'a Thesaurus {
    type Item = (&'a NormalizedTermValue, &'a NormalizedTerm);
    type IntoIter = Iter<'a, NormalizedTermValue, NormalizedTerm>;

    fn into_iter(self) -> Self::IntoIter {
        self.inner.iter()
    }
}

/// An index is a hashmap of articles
///
/// It holds the articles that have been indexed
/// and can be searched through using the `RoleGraph`.
pub type Index = AHashMap<String, Article>;

/// Reference to external storage of articles, traditional indexes use
/// article, aka document or entity.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct IndexedArticle {
    /// UUID of the indexed article, matching external storage id
    pub id: String,
    /// Matched to edges
    pub matched_edges: Vec<Edge>,
    /// Graph rank (the sum of node rank, edge rank)
    /// Number of nodes
    pub rank: Rank,
    /// tags, which are nodes turned into concepts for human readability
    pub tags: Vec<String>,
    /// list of node ids for validation of matching
    pub nodes: Vec<Id>,
}

impl IndexedArticle {
    pub fn to_json_string(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(&self)
    }
}

/// Query type for searching articles in the `RoleGraph`.
/// It contains the search term, skip and limit parameters.
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct SearchQuery {
    pub search_term: String,
    pub skip: Option<usize>,
    pub limit: Option<usize>,
    pub role: Option<String>,
}

/// Defines the relevance function (scorer) to be used for ranking search
/// results for the `Role`.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum RelevanceFunction {
    #[serde(rename = "terraphim-graph")]
    TerraphimGraph,
    #[serde(rename = "redis-search")]
    RedisSearch,
    /// Use the title scorer for relevance
    #[serde(rename = "title-scorer")]
    TitleScorer,
}

/// Defines all supported inputs for the knowledge graph.
///
/// Every knowledge graph is built from a specific input, such as Markdown files
/// or JSON files.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum KnowledgeGraphInputType {
    /// A set of Markdown files
    #[serde(rename = "markdown")]
    Markdown,
    /// A set of JSON files
    #[serde(rename = "json")]
    Json,
}

/// Merge articles from the cache and the output of query results
///
/// Returns the merged articles
pub fn merge_and_serialize(cached_articles: Index, docs: Vec<IndexedArticle>) -> Vec<Article> {
    // TODO: use relevance function for ranking (scorer)
    let mut articles: Vec<Article> = Vec::new();
    for doc in docs {
        log::trace!("doc: {:#?}", doc);
        if let Some(article) = cached_articles.get(&doc.id).cloned() {
            // Article found in cache
            let mut article = article;
            article.tags = Some(doc.tags.clone());
            // rank only available for terraphim graph
            // use scorer to populate the rank for all cases
            article.rank = Some(doc.rank);
            articles.push(article.clone());
        } else {
            log::warn!("Article not found in cache");
        }
    }
    articles
}
