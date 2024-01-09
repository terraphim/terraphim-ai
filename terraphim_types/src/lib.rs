use opendal::Result as OpendalResult;
use persistance::Persistable;
use serde::{Deserialize, Serialize};
use terraphim_config::TerraphimConfig;
use terraphim_pipeline::{Document, Error as TerraphimPipelineError};

// terraphim error type based on thiserror
#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Error: {0}")]
    Article(String),

    #[error("Error: {0}")]
    Pipeline(#[from] TerraphimPipelineError),

    #[error("Persistance error: {0}")]
    Persistance(#[from] persistance::Error),
}

type Result<T> = std::result::Result<T, Error>;

/// Query type for searching documents in the `RoleGraph`.
/// It contains the search term, skip and limit parameters.
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct SearchQuery {
    pub search_term: String,
    pub skip: Option<usize>,
    pub limit: Option<usize>,
    pub role: Option<String>,
}

/// Create article schema
#[derive(Deserialize, Serialize, Debug, Clone, Default)]
pub struct Article {
    pub id: Option<String>,
    pub stub: Option<String>,
    pub title: String,
    pub url: String,
    pub body: String,
    pub description: Option<String>,
    pub tags: Vec<String>,
    pub rank: u64,
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

// impl to_atomic for Article {
//     fn to_atomic(&self) -> anyhow::Result<atomic::Document> {
//         let mut doc = atomic::Document::new();
//         doc.insert("id", self.id.clone().unwrap());
//         doc.insert("title", self.title.clone());
//         doc.insert("body", self.body.clone().unwrap());
//         doc.insert("description", self.description.clone().unwrap());
//         Ok(doc)
//     }

// }

/// Merge articles from the cache and the output of query results
pub fn merge_and_serialize(
    articles_cached: HashMap<String, Article>,
    docs: Vec<IndexedDocument>,
) -> Result<Vec<Article>> {
    let mut articles: Vec<Article> = Vec::new();
    for each_doc in docs.iter() {
        println!("each_doc: {:#?}", each_doc);
        let mut article = match articles_cached.get(&each_doc.id) {
            Some(article) => article.clone(),
            None => {
                return Err(Error::Article(format!(
                    "Article with id {} not found",
                    each_doc.id
                )))
            }
        };

        article.tags = each_doc.tags.clone();
        article.rank = each_doc.rank.clone();
        articles.push(article.clone());
    }
    Ok(articles)
}

use terraphim_pipeline::{IndexedDocument, RoleGraph};
use tokio::sync::Mutex;

use std::collections::HashMap;
use std::sync::Arc;

/// ConfigState for the Terraphim (Actor)
/// Config state can be updated using the API or Atomic Server
#[derive(Default, Debug, Clone)]
pub struct ConfigState {
    /// Terraphim Config
    pub config: Arc<Mutex<TerraphimConfig>>,
    pub roles: HashMap<String, RoleGraphState>,
}

impl ConfigState {
    pub async fn new() -> Result<Self> {
        let mut config = TerraphimConfig::new();
        // Try to load the existing state
        let config = config.load("configstate").await?;
        println!("Config loaded");
        let mut config_state = ConfigState {
            config: Arc::new(Mutex::new(config.clone())),
            roles: HashMap::new(),
        };

        // for each role in a config initialize a rolegraph
        // and add it to the config state
        for (role_name, each_role) in config.roles {
            let automata_url = each_role.kg.automata_url.as_str();
            println!("{} - {}", role_name.clone(), automata_url);
            let rolegraph = RoleGraph::new(role_name.clone(), automata_url).await?;
            config_state.roles.insert(
                role_name,
                RoleGraphState {
                    rolegraph: Arc::new(Mutex::new(rolegraph)),
                },
            );
        }
        Ok(config_state)
    }
    /// Index article in all rolegraphs
    pub async fn index_article(&mut self, article: Article) -> OpendalResult<()> {
        let mut article = article.clone();
        let id = if article.id.is_none() {
            let id = ulid::Ulid::new().to_string();
            article.id = Some(id.clone());
            id
        } else {
            article.id.clone().unwrap()
        };
        for rolegraph_state in self.roles.values() {
            let mut rolegraph = rolegraph_state.rolegraph.lock().await;
            rolegraph.parse_document(id.clone(), article.clone());
        }
        Ok(())
    }

    /// Search articles in rolegraph using the search query
    pub async fn search_articles(
        &self,
        search_query: SearchQuery,
    ) -> OpendalResult<Vec<IndexedDocument>> {
        let current_config_state = self.config.lock().await.clone();
        let default_role = current_config_state.default_role.clone();
        // if role is not provided, use the default role in the config
        let role = if search_query.role.is_none() {
            default_role.as_str()
        } else {
            search_query.role.as_ref().unwrap()
        };

        let rolegraph = self.roles.get(role).unwrap().rolegraph.lock().await;
        let documents: Vec<(&String, IndexedDocument)> = match rolegraph.query(
            &search_query.search_term,
            search_query.skip,
            search_query.limit,
        ) {
            Ok(docs) => docs,
            Err(e) => {
                log::error!("Error: {}", e);
                return Ok(vec![]);
            }
        };

        let docs: Vec<IndexedDocument> = documents.into_iter().map(|(_id, doc)| doc).collect();
        Ok(docs)
    }
}

#[derive(Debug, Clone)]
pub struct RoleGraphState {
    /// RoleGraph for ingesting documents
    pub rolegraph: Arc<Mutex<RoleGraph>>,
}
