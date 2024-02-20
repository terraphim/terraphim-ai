use ahash::AHashMap;
use serde::{Deserialize, Serialize};

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

/// An article is a piece of content that can be indexed and searched.
///
/// It holds the title, body, description, tags, and rank.
/// The `id` is a unique identifier for the article.
#[derive(Deserialize, Serialize, Debug, Clone, Default)]
pub struct Article {
    /// Unique identifier for the article
    pub id: Option<String>,
    /// A short excerpt of the article
    pub stub: Option<String>,
    /// Title of the article
    pub title: String,
    /// URL of the article
    pub url: String,
    /// The article body
    pub body: String,
    /// A short description of the article
    pub description: Option<String>,
    /// Tags for the article
    pub tags: Option<Vec<String>>,
    /// Rank of the article in the search results
    pub rank: Option<u64>,
}

impl From<Article> for Document {
    fn from(val: Article) -> Self {
        // If the ID is not provided, generate a new one
        let id = match val.id {
            Some(id) => id,
            None => ulid::Ulid::new().to_string(),
        };

        Document {
            id,
            title: val.title,
            body: Some(val.body),
            description: val.description,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Edge {
    /// ID of the node
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

/// A `Node` represents single concept
///
/// Each node can have multiple edges to other nodes
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Node {
    /// Unique identifier of the node
    pub id: u64,
    /// Number of co-occurrences
    pub rank: u64,
    /// List of connected nodes
    pub connected_with: Vec<u64>,
}

impl Node {
    /// Create a new node with a given id and edge
    pub fn new(id: u64, edge: Edge) -> Self {
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

/// A thesaurus is a dictionary with synonyms which map to upper-level concepts.
///
/// It holds the normalized terms for a resource
/// where a resource can be as diverse as a Markdown file or a document in
/// Notion or AtomicServer
pub type Thesaurus = AHashMap<String, NormalizedTerm>;

/// An index is a hashmap of articles
///
/// It holds the articles that have been indexed
/// and can be searched through the `RoleGraph`.
pub type Index = AHashMap<String, Article>;

/// Reference to external storage of documents, traditional indexes use
/// document, aka article or entity.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct IndexedDocument {
    /// UUID of the indexed document, matching external storage id
    pub id: String,
    /// Matched to edges
    pub matched_to: Vec<Edge>,
    /// Graph rank (the sum of node rank, edge rank)
    pub rank: u64,
    /// tags, which are nodes turned into concepts for human readability
    pub tags: Vec<String>,
    /// list of node ids for validation of matching
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
    pub search_term: String,
    pub skip: Option<usize>,
    pub limit: Option<usize>,
    pub role: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum RelevanceFunction {
    #[serde(rename = "terraphim-graph")]
    TerraphimGraph,
    #[serde(rename = "redis-search")]
    RedisSearch,
}

/// Defines all supported inputs for the knowledge graph.
///
/// Every knowledge graph is built from a specific input, such as Markdown files
/// or JSON files.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum KnowledgeGraphInput {
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
pub fn merge_and_serialize(cached_articles: Index, docs: Vec<IndexedDocument>) -> Vec<Article> {
    let mut articles: Vec<Article> = Vec::new();
    for doc in docs {
        println!("doc: {:#?}", doc);
        if let Some(article) = cached_articles.get(&doc.id).cloned() {
            // Article found in cache
            let mut article = article;
            article.tags = Some(doc.tags.clone());
            article.rank = Some(doc.rank);
            articles.push(article.clone());
        } else {
            log::warn!("Article not found in cache");
        }
    }
    articles
}

/// A normalized term is a term that has been normalized to a concept, which is
/// a higher-level term.
///
/// It holds a unique identifier and the normalized value.
#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct NormalizedTerm {
    /// Unique identifier for the normalized term
    pub id: u64,
    /// The normalized value
    // This field is currently called `nterm` in the JSON
    #[serde(rename = "nterm")]
    pub value: String,
}
