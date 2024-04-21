use persistence::error;
use terraphim_config::{ConfigState, Role};
use terraphim_middleware::thesaurus::create_thesaurus_from_haystack;
use terraphim_types::{Document, IndexedDocument, SearchQuery};

mod score;

#[derive(thiserror::Error, Debug)]
pub enum ServiceError {
    #[error("An error occurred: {0}")]
    Middleware(#[from] terraphim_middleware::Error),

    #[error("OpenDal error: {0}")]
    OpenDal(#[from] opendal::Error),

    #[error("Persistence error: {0}")]
    Persistence(#[from] persistence::Error),

    #[error("Config error: {0}")]
    Config(String),
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

    /// Update a thesaurus from a haystack and update the knowledge graph automata URL
    async fn update_thesaurus(&self, search_query: &SearchQuery) -> Result<()> {
        Ok(create_thesaurus_from_haystack(self.config_state.clone(), search_query).await?)
    }

    /// Create document
    pub async fn create_document(&mut self, document: Document) -> Result<Document> {
        self.config_state.index_document(&document).await?;
        Ok(document)
    }

    /// Get the role for the given search query
    async fn get_search_role(&self, search_query: &SearchQuery) -> Result<Role> {
        let search_role = search_query.role.clone().unwrap_or_default();
        let Some(role) = self.config_state.get_role(&search_role).await else {
            return Err(ServiceError::Config(format!(
                "Role {} not found in config",
                search_role
            )));
        };
        Ok(role)
    }

    /// Search for documents in the haystacks
    pub async fn search_documents(&self, search_query: &SearchQuery) -> Result<Vec<Document>> {
        self.update_thesaurus(search_query).await?;

        let cached_documents =
            terraphim_middleware::search_haystacks(self.config_state.clone(), search_query.clone())
                .await?;
        let rolegraph_documents: Vec<IndexedDocument> =
            self.config_state.search_documents(search_query).await;

        let documents = terraphim_types::merge_and_serialize(cached_documents, rolegraph_documents);

        // Get the role from the config
        let role = self.get_search_role(search_query).await?;

        let relevance_function = role.relevance_function;
        // Use relevance function for ranking (scorer)

        // Sort the documents by relevance
        let documents = score::sort_documents(documents, relevance_function);

        Ok(documents)
    }

    /// Fetch the current config
    pub async fn fetch_config(&self) -> terraphim_config::Config {
        let current_config = self.config_state.config.lock().await;
        current_config.clone()
    }
}
