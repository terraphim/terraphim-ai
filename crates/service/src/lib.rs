use persistence::Persistable;
use terraphim_config::Config;
use terraphim_config::ConfigState;
use terraphim_middleware::thesaurus::create_thesaurus_from_haystack;
use terraphim_types::{Article, IndexedDocument, SearchQuery};

#[derive(thiserror::Error, Debug)]
pub enum ServiceError {
    #[error("An error occurred: {0}")]
    Middleware(#[from] terraphim_middleware::Error),

    #[error("OpenDal error: {0}")]
    OpenDal(#[from] opendal::Error),

    #[error("Persistence error: {0}")]
    Persistence(#[from] persistence::Error),
}

pub type Result<T> = std::result::Result<T, ServiceError>;

pub struct TerraphimService {
    config_state: ConfigState,
}

impl<'a> TerraphimService {
    /// Create a new TerraphimService
    pub fn new(config_state: ConfigState) -> Self {
        Self { config_state }
    }

    /// Create a thesaurus from a haystack
    pub async fn create_thesaurus(&self, search_query: SearchQuery) -> Result<()> {
        Ok(create_thesaurus_from_haystack(self.config_state.clone(), search_query.clone()).await?)
    }

    /// Create article
    pub async fn create_article(&mut self, article: Article) -> Result<Article> {
        self.config_state.index_article(&article).await?;
        Ok(article)
    }

    /// Search for articles in the haystacks
    pub async fn search_articles(&self, search_query: SearchQuery) -> Result<Vec<Article>> {
        let cached_articles =
            terraphim_middleware::search_haystacks(self.config_state.clone(), search_query.clone())
                .await?;
        let docs: Vec<IndexedDocument> = self.config_state.search_articles(search_query).await;
        let articles = terraphim_types::merge_and_serialize(cached_articles, docs);

        Ok(articles)
    }

    /// Fetch the current config
    pub async fn fetch_config(&self) -> terraphim_config::Config {
        let current_config = self.config_state.config.lock().await;
        current_config.clone()
    }

    /// Update the current config
    pub async fn update_config(&self, config_new: Config) -> Result<terraphim_config::Config> {
        let mut config_state_lock = self.config_state.config.lock().await;
        config_state_lock.update(config_new.clone());
        config_state_lock.save().await?;
        Ok(config_state_lock.clone())
    }
}
