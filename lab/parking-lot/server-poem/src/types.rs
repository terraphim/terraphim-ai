use poem_openapi::types::ToJSON;
use poem_openapi::{Object, Tags};
use serde::{Deserialize, Serialize};
use terraphim_pipeline::{Document, IndexedDocument};

#[derive(Debug, Object)]
pub struct SearchQuery {
    pub(crate) search_term: String,
    skip: usize,
    limit: usize,
    role: Option<String>,
}

/// Create article schema
#[derive(Deserialize, Serialize, Debug, Object)]
pub(crate) struct Article {
    pub(crate) id: Option<String>,
    pub(crate) stub: Option<String>,
    pub(crate) title: String,
    pub(crate) url: String,
    pub(crate) body: String,
    pub(crate) description: Option<String>,
    pub(crate) tags: Option<Vec<String>>,
}

impl Into<Document> for Article {
    fn into(self) -> Document {
        // If the ID is not provided, generate a new one
        let id = match self.id {
            Some(id) => id,
            None => ulid::Ulid::new().to_string(),
        };

        Document {
            id,
            title: self.title,
            body: Some(self.body),
            description: self.description,
        }
    }
}

#[derive(Tags)]
pub(crate) enum ApiTags {
    /// Article operations
    Article,
    /// Config operations
    Config,
    /// Search operations
    Search,
    Save,
}
