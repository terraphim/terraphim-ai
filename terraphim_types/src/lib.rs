use opendal::Result as OpendalResult;
use serde::{Deserialize, Serialize};
use terraphim_config::TerraphimConfig;
use terraphim_pipeline::{Document, Error as TerraphimPipelineError};
use terraphim_pipeline::{IndexedDocument, RoleGraph};
use tokio::sync::{Mutex, MutexGuard};

use std::collections::HashMap;
use std::sync::Arc;

// terraphim error type based on thiserror
#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Error: {0}")]
    Article(String),

    #[error("Error: {0}")]
    Pipeline(#[from] TerraphimPipelineError),

    #[error("Persistence error: {0}")]
    Persistence(#[from] persistence::Error),
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
    pub tags: Option<Vec<String>>,
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

/// Merge articles from the cache and the output of query results
pub fn merge_and_serialize(
    cached_articles: HashMap<String, Article>,
    docs: Vec<IndexedDocument>,
) -> Result<Vec<Article>> {
    let mut articles: Vec<Article> = Vec::new();
    for doc in docs {
        println!("doc: {:#?}", doc);
        let mut article = match cached_articles.get(&doc.id) {
            Some(article) => article.clone(),
            None => {
                log::warn!("Article with id {} not found", doc.id);
                Article::default()
            }
        };
        article.tags = Some(doc.tags.clone());
        article.rank = Some(doc.rank);

        articles.push(article.clone());
    }
    Ok(articles)
}

/// ConfigState for the Terraphim (Actor)
/// Config state can be updated using the API or Atomic Server
///
/// Holds the Terraphim Config and the RoleGraphs
#[derive(Debug, Clone)]
pub struct ConfigState {
    /// Terraphim Config
    pub config: Arc<Mutex<TerraphimConfig>>,
    /// RoleGraphs
    pub roles: HashMap<String, RoleGraphSync>,
}

impl ConfigState {
    pub async fn new(config: &mut TerraphimConfig) -> Result<Self> {
        // Try to load the existing state from the config
        // TODO: Is this really needed? To be clarified
        // let config = config.load("configstate").await?;
        // println!("Config loaded");
        // println!("{:#?}", config.roles);

        // For each role in a config, initialize a rolegraph
        // and add it to the config state
        let mut roles = HashMap::new();
        for (name, role) in &config.roles {
            let automata_url = role.kg.automata_url.as_str();
            let role_name = name.to_lowercase();
            // FIXME: turn into log info
            println!("Loading Role {} - Url {}", role_name.clone(), automata_url);
            let rolegraph = RoleGraph::new(role_name.clone(), automata_url).await?;
            roles.insert(role_name, RoleGraphSync::from(rolegraph));
        }

        Ok(ConfigState {
            config: Arc::new(Mutex::new(config.clone())),
            roles,
        })
    }

    /// Index article into all rolegraphs
    pub async fn index_article(&mut self, mut article: Article) -> OpendalResult<()> {
        let id = article
            .id
            // lazily initialize `article.id` only if it's `None`.
            .get_or_insert_with(|| ulid::Ulid::new().to_string())
            .clone();

        for rolegraph_state in self.roles.values() {
            let mut rolegraph = rolegraph_state.lock().await;
            rolegraph.parse_document(&id, article.clone());
        }
        Ok(())
    }

    /// Search articles in rolegraph using the search query
    pub async fn search_articles(&self, search_query: SearchQuery) -> Vec<IndexedDocument> {
        println!("search_articles: {:#?}", search_query);
        let current_config_state = self.config.lock().await.clone();
        let default_role = current_config_state.default_role.clone();

        // if role is not provided, use the default role in the config
        let role = search_query.role.unwrap_or(default_role);

        let role = role.to_lowercase();
        let rolegraph = self.roles.get(&role).unwrap().lock().await;
        let documents: Vec<(&String, IndexedDocument)> = match rolegraph.query(
            &search_query.search_term,
            search_query.skip,
            search_query.limit,
        ) {
            Ok(docs) => docs,
            Err(e) => {
                log::error!("Error: {}", e);
                return Vec::default();
            }
        };

        documents.into_iter().map(|(_id, doc)| doc).collect()
    }
}

/// Wraps the `RoleGraph` for ingesting documents
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
