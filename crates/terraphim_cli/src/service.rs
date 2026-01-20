//! Service wrapper for CLI operations

use anyhow::Result;
use std::sync::Arc;
use terraphim_config::{ConfigBuilder, ConfigId, ConfigState};
use terraphim_persistence::Persistable;
use terraphim_service::TerraphimService;
use terraphim_settings::DeviceSettings;
use terraphim_types::{Document, NormalizedTermValue, RoleName, SearchQuery, Thesaurus};
use tokio::sync::Mutex;

#[derive(Clone)]
pub struct CliService {
    config_state: ConfigState,
    service: Arc<Mutex<TerraphimService>>,
}

impl CliService {
    /// Initialize a new CLI service
    pub async fn new() -> Result<Self> {
        // Initialize logging
        terraphim_service::logging::init_logging(
            terraphim_service::logging::detect_logging_config(),
        );

        log::info!("Initializing CLI service");

        // Load device settings
        let device_settings = DeviceSettings::load_from_env_and_file(None)?;
        log::debug!("Device settings: {:?}", device_settings);

        // Try to load existing configuration, fallback to default embedded config
        let mut config = match ConfigBuilder::new_with_id(ConfigId::Embedded).build() {
            Ok(mut config) => match config.load().await {
                Ok(config) => {
                    log::info!("Loaded existing embedded configuration");
                    config
                }
                Err(e) => {
                    log::info!("Failed to load config: {:?}, using default embedded", e);
                    ConfigBuilder::new_with_id(ConfigId::Embedded)
                        .build_default_embedded()
                        .build()?
                }
            },
            Err(e) => {
                log::warn!("Failed to build config: {:?}, using default", e);
                ConfigBuilder::new_with_id(ConfigId::Embedded)
                    .build_default_embedded()
                    .build()?
            }
        };

        // Create config state
        let config_state = ConfigState::new(&mut config).await?;

        // Create service
        let service = TerraphimService::new(config_state.clone());

        Ok(Self {
            config_state,
            service: Arc::new(Mutex::new(service)),
        })
    }

    /// Get the current configuration
    pub async fn get_config(&self) -> terraphim_config::Config {
        let config = self.config_state.config.lock().await;
        config.clone()
    }

    /// Get the current selected role
    pub async fn get_selected_role(&self) -> RoleName {
        let config = self.config_state.config.lock().await;
        config.selected_role.clone()
    }

    /// List all available roles
    pub async fn list_roles(&self) -> Vec<String> {
        let config = self.config_state.config.lock().await;
        config.roles.keys().map(|r| r.to_string()).collect()
    }

    /// Search documents with a specific role
    pub async fn search(
        &self,
        search_term: &str,
        role: &RoleName,
        limit: Option<usize>,
    ) -> Result<Vec<Document>> {
        let query = SearchQuery {
            search_term: NormalizedTermValue::from(search_term),
            search_terms: None,
            operator: None,
            skip: Some(0),
            limit,
            role: Some(role.clone()),
        };

        let mut service = self.service.lock().await;
        Ok(service.search(&query).await?)
    }

    /// Get thesaurus for a specific role
    pub async fn get_thesaurus(&self, role_name: &RoleName) -> Result<Thesaurus> {
        let mut service = self.service.lock().await;
        Ok(service.ensure_thesaurus_loaded(role_name).await?)
    }

    /// Get the role graph top-k concepts for a specific role
    ///
    /// Returns the top-k concepts sorted by rank (number of co-occurrences) in descending order.
    pub async fn get_top_concepts(
        &self,
        role_name: &RoleName,
        top_k: usize,
    ) -> Result<Vec<String>> {
        log::info!("Getting top {} concepts for role {}", top_k, role_name);

        // Get the role graph for this role
        if let Some(rolegraph_sync) = self.config_state.roles.get(role_name) {
            let rolegraph = rolegraph_sync.lock().await;

            // Get nodes and sort by rank (descending)
            let mut nodes: Vec<_> = rolegraph.nodes_map().iter().collect();
            nodes.sort_by(|a, b| b.1.rank.cmp(&a.1.rank));

            // Map node IDs to term names and collect top-k
            let top_concepts: Vec<String> = nodes
                .into_iter()
                .take(top_k)
                .filter_map(|(node_id, _node)| {
                    rolegraph
                        .ac_reverse_nterm
                        .get(node_id)
                        .map(|term| term.to_string())
                })
                .collect();

            log::debug!(
                "Found {} concepts for role {} (requested {})",
                top_concepts.len(),
                role_name,
                top_k
            );
            Ok(top_concepts)
        } else {
            log::warn!("Role graph not found for role {}", role_name);
            Ok(Vec::new())
        }
    }

    /// Find matches in text using thesaurus
    pub async fn find_matches(
        &self,
        role_name: &RoleName,
        text: &str,
    ) -> Result<Vec<terraphim_automata::Matched>> {
        // Get thesaurus for the role
        let thesaurus = self.get_thesaurus(role_name).await?;

        // Find matches
        Ok(terraphim_automata::find_matches(text, thesaurus, true)?)
    }

    /// Replace matches in text with links using thesaurus
    pub async fn replace_matches(
        &self,
        role_name: &RoleName,
        text: &str,
        link_type: terraphim_automata::LinkType,
    ) -> Result<String> {
        // Get thesaurus for the role
        let thesaurus = self.get_thesaurus(role_name).await?;

        // Replace matches
        let result = terraphim_automata::replace_matches(text, thesaurus, link_type)?;
        Ok(String::from_utf8(result).unwrap_or_else(|_| text.to_string()))
    }
}
