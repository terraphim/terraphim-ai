use ahash::AHashMap;
use terraphim_automata::{load_thesaurus, AutomataPath};
use terraphim_config::{ConfigState, Role};
use terraphim_middleware::thesaurus::{self, build_thesaurus_from_haystack};
use terraphim_persistence::error;
use terraphim_persistence::Persistable;
use terraphim_rolegraph::{RoleGraph, RoleGraphSync};
use terraphim_types::{
    Document, Index, IndexedDocument, RelevanceFunction, RoleName, SearchQuery, Thesaurus,
};
mod score;

#[derive(thiserror::Error, Debug)]
pub enum ServiceError {
    #[error("An error occurred: {0}")]
    Middleware(#[from] terraphim_middleware::Error),

    #[error("OpenDal error: {0}")]
    OpenDal(#[from] opendal::Error),

    #[error("Persistence error: {0}")]
    Persistence(#[from] terraphim_persistence::Error),

    #[error("RoleGraph error: {0}")]
    RoleGraph(#[from] terraphim_rolegraph::Error),

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
        async fn load_thesaurus_from_automata_path(
            config_state: &ConfigState,
            role_name: &RoleName,
            rolegraphs: &mut AHashMap<RoleName, RoleGraphSync>,
        ) -> Result<Thesaurus> {
            log::debug!("Attempting to load thesaurus from automata path for role: {}", role_name);
            let role = config_state.get_role(role_name).await
                .ok_or_else(|| ServiceError::Config(format!("Role {} not found", role_name)))?;
            
            let kg = role.kg
                .ok_or_else(|| ServiceError::Config(format!("No knowledge graph configuration found for role {}", role_name)))?;
                
            let automata_path = kg.automata_path
                .ok_or_else(|| ServiceError::Config(format!("No automata path found for role {}", role_name)))?;
                
            log::debug!("Found automata path: {:?}", automata_path);
            
            let thesaurus = load_thesaurus(&automata_path).await
                .map_err(|e| ServiceError::Config(format!("Failed to load thesaurus for role {}: {}", role_name, e)))?;
                
            log::debug!("Loaded thesaurus with {} entries", thesaurus.len());
            
            let rolegraph = RoleGraph::new(role_name.clone(), thesaurus.clone()).await
                .map_err(|e| ServiceError::Config(format!("Failed to create rolegraph for role {}: {}", role_name, e)))?;
                
            log::debug!("Created new rolegraph for {}", role_name);
            let rolegraph_value = RoleGraphSync::from(rolegraph);
            rolegraphs.insert(role_name.clone(), rolegraph_value);
            log::debug!("Inserted rolegraph into rolegraphs map");
            
            Ok(thesaurus)
        }
        log::debug!("Loading thesaurus for role: {}", role_name);
        log::debug!("Available role keys: {:?}", self.config_state.roles.keys());
        let mut rolegraphs = self.config_state.roles.clone();
        if let Some(rolegraph_value) = rolegraphs.get(role_name) {
            log::debug!("Found existing rolegraph for {}", role_name);
            let mut thesaurus_result = rolegraph_value.lock().await.thesaurus.clone().load().await;
            match thesaurus_result {
                Ok(thesaurus) => {
                    log::debug!("Successfully loaded thesaurus with {} entries", thesaurus.len());
                    log::info!("Rolegraph loaded for role name {:?}", role_name);
                    Ok(thesaurus)
                }
                Err(e) => {
                    log::error!("Failed to load thesaurus: {:?}", e);
                    log::debug!("Falling back to loading from automata path");
                    load_thesaurus_from_automata_path(
                        &self.config_state,
                        role_name,
                        &mut rolegraphs,
                    )
                    .await
                }
            }
        } else {
            log::debug!("No existing rolegraph found for {}, loading from automata path", role_name);
            load_thesaurus_from_automata_path(
                &self.config_state,
                role_name,
                &mut rolegraphs,
            )
            .await
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

    /// Get the rolegraph for the default role
    pub async fn get_rolegraph(&mut self) -> Result<RoleGraph> {
        let selected_role = self.config_state.get_selected_role().await;
        
        log::debug!("Selected role: {:?}", selected_role);
        match self.config_state.roles.get(&selected_role) {
            Some(rolegraph) => {
                let rolegraph = rolegraph.lock().await;
                Ok(rolegraph.clone())
            }
            None => Err(ServiceError::Config("Selected role not found".into()))
        }
    }

    /// Get the rolegraph for a specific role
    pub async fn get_rolegraph_by_role(&mut self, role: RoleName) -> Result<RoleGraph> {
        log::debug!("Getting rolegraph for role: {:?}", role);
        
        // First check if we have the role in our config
        if self.config_state.get_role(&role).await.is_none() {
            return Err(ServiceError::Config(format!(
                "Role `{}` not found in config",
                role
            )));
        }

        // Get or create rolegraph
        match self.config_state.roles.get(&role) {
            Some(rolegraph) => {
                let rolegraph = rolegraph.lock().await;
                Ok(rolegraph.clone())
            }
            None => {
                // If role not found in rolegraphs, create a new one
                let thesaurus = self.ensure_thesaurus_loaded(&role).await?;
                let rolegraph = RoleGraph::new(role.clone(), thesaurus).await
                    .map_err(ServiceError::RoleGraph)?;
                
                // Store the new rolegraph
                let rolegraph_sync = RoleGraphSync::from(rolegraph.clone());
                self.config_state.roles.insert(role, rolegraph_sync);
                
                Ok(rolegraph)
            }
        }
    }
}
