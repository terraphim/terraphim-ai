use terraphim_config::{ConfigState, Role};
use terraphim_middleware::thesaurus::{self, build_thesaurus_from_haystack};
use terraphim_persistence::error;
use terraphim_automata::{load_thesaurus, AutomataPath};
use terraphim_persistence::Persistable;
use terraphim_rolegraph::{RoleGraph, RoleGraphSync};

use terraphim_types::{Document, Index, IndexedDocument, RelevanceFunction, RoleName, SearchQuery, Thesaurus};
mod score;

#[derive(thiserror::Error, Debug)]
pub enum ServiceError {
    #[error("An error occurred: {0}")]
    Middleware(#[from] terraphim_middleware::Error),

    #[error("OpenDal error: {0}")]
    OpenDal(#[from] opendal::Error),

    #[error("Persistence error: {0}")]
    Persistence(#[from] terraphim_persistence::Error),

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

    /// Build a thesaurus from the haystack and update the knowledge graph automata URL
    async fn build_thesaurus(&mut self, search_query: &SearchQuery) -> Result<()> {
        Ok(build_thesaurus_from_haystack(&mut self.config_state, search_query).await?)
    }
    /// load thesaurus from config object and if absent make sure it's loaded from automata_url
    pub async fn ensure_thesaurus_loaded(&mut self, role_name: &RoleName) -> Result<Thesaurus> {

        println!("Loading thesaurus for role: {}", role_name);
        println!("Role keys {:?}", self.config_state.roles.keys());
        let mut rolegraphs = self.config_state.roles.clone();
        if let Some(rolegraph_value) = rolegraphs.get(role_name) {
            let mut thesaurus = rolegraph_value.lock().await.thesaurus.clone();
            thesaurus=thesaurus.load().await.unwrap();
            println!("Thesaurus loaded: {:#?}", thesaurus);
            log::info!("Rolegraph loaded: for role name {:?}", role_name);
            return Ok(thesaurus)
        }else{
            let role = self.config_state.get_role(role_name).await.unwrap();
            if let Some(automata_path) = role.kg.unwrap().automata_path {
                let thesaurus = load_thesaurus(&automata_path).await.unwrap();
                let rolegraph = RoleGraph::new(role_name.clone(), thesaurus.clone()).await;
                match rolegraph {
                    Ok(rolegraph) => {
                        let rolegraph_value = RoleGraphSync::from(rolegraph);
                        rolegraphs.insert(role_name.clone(), rolegraph_value);
                    }
                    Err(e) => log::error!("Failed to update role and thesaurus: {:?}", e),
                }
                return Ok(thesaurus)
            } else {
                return Err(ServiceError::Config("Automata path not found".into()));
            }
        }
    }

    /// Create document
    pub async fn create_document(&mut self, document: Document) -> Result<Document> {
        self.config_state.add_to_roles(&document).await?;
        Ok(document)
    }

    /// Get the role for the given search query
    async fn get_search_role(&self, search_query: &SearchQuery) -> Result<Role> {
        let search_role = match &search_query.role {
            Some(role) => role.clone(),
            None => self.config_state.get_default_role().await,
        };

        log::debug!("Searching for role: {:?}", search_role);
        let Some(role) = self.config_state.get_role(&search_role).await else {
            return Err(ServiceError::Config(format!(
                "Role `{}` not found in config",
                search_role
            )));
        };
        Ok(role)
    }

    /// Search for documents in the haystacks
    pub async fn search(&mut self, search_query: &SearchQuery) -> Result<Vec<Document>> {
        // Get the role from the config
        log::debug!("Role for searching: {:?}", search_query.role);
        let role = self.get_search_role(search_query).await?;

        log::trace!("Building index for search query: {:?}", search_query);
        let index: Index =
            terraphim_middleware::search_haystacks(self.config_state.clone(), search_query.clone())
                .await?;

        match role.relevance_function {
            RelevanceFunction::TitleScorer => {
                log::debug!("Searching haystack with title scorer");

                let documents = index.get_all_documents();

                log::debug!("Sorting documents by relevance");
                // Sort the documents by relevance
                let documents = score::sort_documents(search_query, documents);
                let total_length = documents.len();
                let mut docs_ranked = Vec::new();
                for (idx, doc) in documents.iter().enumerate() {
                    let document: &mut terraphim_types::Document = &mut doc.clone();
                    let rank = (total_length - idx).try_into().unwrap();
                    document.rank = Some(rank);
                    docs_ranked.push(document.clone());
                }
                Ok(docs_ranked)
            }
            RelevanceFunction::TerraphimGraph => {
                self.build_thesaurus(search_query).await?;
                let thesaurus = self.ensure_thesaurus_loaded(&role.name).await?;
                let scored_index_docs: Vec<IndexedDocument> = self
                    .config_state
                    .search_indexed_documents(search_query, &role)
                    .await;

                // Apply to ripgrep vector of document output
                // I.e. use the ranking of thesaurus to rank the documents here
                log::debug!("Ranking documents with thesaurus");
                println!("Ranking documents with thesaurus");
                let documents = index.get_documents(scored_index_docs);

                Ok(documents)
            }
        }
    }

    /// Fetch the current config
    pub async fn fetch_config(&self) -> terraphim_config::Config {
        let current_config = self.config_state.config.lock().await;
        current_config.clone()
    }

    /// Update the config
    ///
    /// Overwrites the config in the config state and returns the updated
    /// config.
    pub async fn update_config(
        &self,
        config: terraphim_config::Config,
    ) -> Result<terraphim_config::Config> {
        let mut current_config = self.config_state.config.lock().await;
        *current_config = config.clone();
        Ok(config)
    }
}