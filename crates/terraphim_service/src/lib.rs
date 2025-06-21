use ahash::AHashMap;
use terraphim_automata::load_thesaurus;
use terraphim_config::{ConfigState, Role};
use terraphim_middleware::thesaurus::{self, build_thesaurus_from_haystack};
use terraphim_persistence::Persistable;
use terraphim_rolegraph::{RoleGraph, RoleGraphSync};
use terraphim_types::{
    Document, Index, IndexedDocument, NormalizedTermValue,RelevanceFunction, RoleName, SearchQuery, Thesaurus,
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
            let role = config_state.get_role(role_name).await.unwrap();
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
                Ok(thesaurus)
            } else {
                Err(ServiceError::Config("Automata path not found".into()))
            }
        }
        log::debug!("Loading thesaurus for role: {}", role_name);
        log::debug!("Role keys {:?}", self.config_state.roles.keys());
        let mut rolegraphs = self.config_state.roles.clone();
        if let Some(rolegraph_value) = rolegraphs.get(role_name) {
            let thesaurus_result = rolegraph_value.lock().await.thesaurus.clone().load().await;
            match thesaurus_result {
                Ok(thesaurus) => {
                    log::debug!("Thesaurus loaded: {:?}", thesaurus);
                    log::info!("Rolegraph loaded: for role name {:?}", role_name);
                    Ok(thesaurus)
                }
                Err(e) => {
                    log::error!("Failed to load thesaurus: {:?}", e);
                    load_thesaurus_from_automata_path(
                        &self.config_state,
                        role_name,
                        &mut rolegraphs,
                    )
                    .await
                }
            }
        } else {
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
        // Persist the document using the fastest available Operator. The document becomes
        // available on all profiles/devices thanks to the Persistable implementation.
        document.save().await?;

        // Index the freshly-saved document inside all role graphs so it can be discovered via
        // search immediately.
        self.config_state.add_to_roles(&document).await?;

        Ok(document)
    }

    /// Get document by ID
    pub async fn get_document_by_id(&mut self, document_id: &str) -> Result<Option<Document>> {
        // 1️⃣ Try to load the document directly from the persistence layer.
        let mut placeholder = Document::default();
        placeholder.id = document_id.to_string();
        match placeholder.load().await {
            Ok(doc) => return Ok(Some(doc)),
            Err(e) => {
                log::debug!("Document {} not found in persistence layer: {:?}. Falling back to search", document_id, e);
            }
        }

        // 2️⃣ Fallback: search the haystacks/graphs – this covers the case where the document
        // is not yet persisted but is already indexed in memory.
        let search_query = SearchQuery {
            search_term: NormalizedTermValue::new(document_id.to_string()),
            limit: Some(1),
            skip: None,
            role: None,
        };

        let documents = self.search(&search_query).await?;

        Ok(documents.into_iter().find(|doc| doc.id == document_id))
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
    
    /// search for documents in the haystacks with selected role from the config
    /// and return the documents sorted by relevance
    pub async fn search_documents_selected_role(
        &mut self,
        search_term: &NormalizedTermValue,
    ) -> Result<Vec<Document>> {
        let role = self.config_state.get_selected_role().await;
        let documents = self.search(&SearchQuery { search_term: search_term.clone(), role: Some(role), skip: None, limit: None }).await?;
        Ok(documents)
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
        current_config.save().await?;
        log::info!("Config updated");
        Ok(config)
    }
}


#[cfg(test)]
mod tests {
    use super::*;


    //test get config
    #[tokio::test]
    async fn test_get_config() {
        use anyhow::Context;
        use terraphim_settings::DeviceSettings;
        use terraphim_config::{ConfigBuilder, ConfigId};
        let device_settings =
        DeviceSettings::load_from_env_and_file(None).context("Failed to load settings").unwrap();
        log::debug!("Device settings: {:?}", device_settings);
      
          let mut config = match ConfigBuilder::new_with_id(ConfigId::Desktop).build() {
            Ok(mut config) => match config.load().await {
                Ok(config) => config,
                Err(e) => {
                    log::warn!("Failed to load config: {:?}", e);
                    let config = ConfigBuilder::new().build_default_desktop().build().unwrap();
                    config
                },
            },
            Err(e) => panic!("Failed to build config: {:?}", e),
        };
        let config_state = ConfigState::new(&mut config).await.unwrap();
        let terraphim_service = TerraphimService::new(config_state);
        let config = terraphim_service.fetch_config().await;
        log::debug!("Config: {:?}", config);
    }


    // test search documents with selected role
    #[tokio::test]
    async fn test_search_documents_selected_role() {
        use anyhow::Context;
        use terraphim_settings::DeviceSettings;
        use terraphim_config::{ConfigBuilder, ConfigId};
        let device_settings =
        DeviceSettings::load_from_env_and_file(None).context("Failed to load settings").unwrap();
        log::debug!("Device settings: {:?}", device_settings);
      
          let mut config = match ConfigBuilder::new_with_id(ConfigId::Desktop).build() {
            Ok(mut config) => match config.load().await {
                Ok(config) => config,
                Err(e) => {
                    log::warn!("Failed to load config: {:?}", e);
                    let config = ConfigBuilder::new().build_default_desktop().build().unwrap();
                    config
                },
            },
            Err(e) => panic!("Failed to build config: {:?}", e),
        };
        let config_state = ConfigState::new(&mut config).await.unwrap();
        let mut terraphim_service = TerraphimService::new(config_state);
        let documents = terraphim_service.search_documents_selected_role(&NormalizedTermValue::new("agent".to_string())).await.unwrap();
        log::debug!("Documents: {:?}", documents);
    }     
}  