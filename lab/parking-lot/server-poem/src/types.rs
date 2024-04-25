use poem_openapi::types::ToJSON;
use poem_openapi::{Object, Tags};
use serde::{Deserialize, Serialize};
use terraphim_types::{Document, IndexedDocument};

#[derive(Debug, Object)]
pub struct SearchQuery {
    pub(crate) search_term: String,
    skip: usize,
    limit: usize,
    role: Option<String>,
}

/// Create document schema
#[derive(Deserialize, Serialize, Debug, Object)]
pub struct Document {
    pub id: Option<String>,
    pub stub: Option<String>,
    pub title: String,
    pub url: String,
    pub body: String,
    pub description: Option<String>,
    pub tags: Option<Vec<String>>,
}

#[derive(Tags)]
pub(crate) enum ApiTags {
    /// Document operations
    Document,
    /// Config operations
    Config,
    /// Search operations
    Search,
    Save,
}

pub type Thesaurus = AHashMap<String, NormalizedTerm>;
