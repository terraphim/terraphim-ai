use poem_openapi::types::ToJSON;
use poem_openapi::{Object, Tags};
use serde::{Deserialize, Serialize};
use terraphim_types::{Article, IndexedArticle};

#[derive(Debug, Object)]
pub struct SearchQuery {
    pub(crate) search_term: String,
    skip: usize,
    limit: usize,
    role: Option<String>,
}

/// Create article schema
#[derive(Deserialize, Serialize, Debug, Object)]
pub struct Article {
    pub id: Option<String>,
    pub stub: Option<String>,
    pub title: String,
    pub url: String,
    pub body: String,
    pub description: Option<String>,
    pub tags: Option<Vec<String>>,
}

impl Into<Article> for Article {
    fn into(self) -> Article {
        // If the ID is not provided, generate a new one
        let id = match self.id {
            Some(id) => id,
            None => ulid::Ulid::new().to_string(),
        };

        Article {
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

pub type Thesaurus = AHashMap<String, NormalizedTerm>;
