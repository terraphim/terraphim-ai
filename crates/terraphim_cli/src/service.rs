//! Service wrapper for CLI operations

use anyhow::Result;
use serde::Serialize;
use std::sync::Arc;
use terraphim_config::{Config, ConfigBuilder, ConfigId, ConfigState};
use terraphim_persistence::Persistable;
use terraphim_service::TerraphimService;
use terraphim_settings::{DeviceSettings, Error as DeviceSettingsError};
use terraphim_types::{
    CoverageSignal, Document, ExtractedEntity, GroundingMetadata, Layer, NormalizationMethod,
    NormalizedTerm, NormalizedTermValue, OntologySchema, RoleName, SchemaSignal, SearchQuery,
    Thesaurus,
};
use tokio::sync::Mutex;

#[derive(Clone)]
pub struct CliService {
    config_state: ConfigState,
    service: Arc<Mutex<TerraphimService>>,
}

impl CliService {
    /// Initialize a new CLI service.
    ///
    /// Config loading priority:
    /// 1. `config_path` (--config CLI flag) -- always loads from JSON, no persistence
    /// 2. `role_config` in settings.toml -- bootstrap-then-persistence (first run loads
    ///    JSON and saves to persistence; subsequent runs use persistence so CLI changes stick)
    /// 3. Persistence layer (SQLite)
    /// 4. Embedded defaults (hardcoded roles)
    pub async fn new(config_path: Option<String>) -> Result<Self> {
        // Initialize logging
        terraphim_service::logging::init_logging(
            terraphim_service::logging::detect_logging_config(),
        );

        log::info!("Initializing CLI service");

        // Priority 1: --config CLI flag (always loads from JSON, no persistence check)
        if let Some(ref path) = config_path {
            log::info!("Loading config from --config flag: '{}'", path);
            match Config::load_from_json_file(path) {
                Ok(config) => {
                    return Self::from_config(config).await;
                }
                Err(e) => {
                    return Err(anyhow::anyhow!(
                        "Failed to load config from '{}': {:?}",
                        path,
                        e
                    ));
                }
            }
        }

        // Load device settings, falling back to embedded defaults when running in sandboxes/tests
        let device_settings = match DeviceSettings::load_from_env_and_file(None) {
            Ok(settings) => settings,
            Err(DeviceSettingsError::IoError(err))
                if err.kind() == std::io::ErrorKind::NotFound =>
            {
                log::warn!(
                    "Device settings not found ({}); using embedded defaults",
                    err
                );
                DeviceSettings::default_embedded()
            }
            Err(err) => {
                log::error!("Failed to load device settings: {err:?}");
                return Err(err.into());
            }
        };
        log::debug!("Device settings: {:?}", device_settings);

        // Priority 2: role_config in settings.toml (bootstrap-then-persistence)
        if let Some(ref role_config_path) = device_settings.role_config {
            log::info!("Found role_config in settings.toml: '{}'", role_config_path);
            return Self::load_with_role_config(role_config_path, &device_settings).await;
        }

        // Priority 3 & 4: Persistence -> embedded defaults (existing behavior)
        log::debug!("No role_config specified, using persistence/embedded defaults");
        let config = match ConfigBuilder::new_with_id(ConfigId::Embedded).build() {
            Ok(mut config) => match config.load().await {
                Ok(config) => {
                    log::debug!("Loaded existing embedded configuration from persistence");
                    config
                }
                Err(_) => {
                    log::debug!("No saved config found, using default embedded");
                    return Self::new_with_embedded_defaults().await;
                }
            },
            Err(e) => {
                log::warn!("Failed to build config: {:?}, using default", e);
                return Self::new_with_embedded_defaults().await;
            }
        };

        Self::from_config(config).await
    }

    /// Load config using bootstrap-then-persistence strategy.
    async fn load_with_role_config(
        role_config_path: &str,
        device_settings: &DeviceSettings,
    ) -> Result<Self> {
        // Try persistence first (preserves runtime changes like `config set`)
        if let Ok(mut empty_config) = ConfigBuilder::new_with_id(ConfigId::Embedded).build() {
            if let Ok(persisted) = empty_config.load().await {
                if !persisted.roles.is_empty() {
                    log::info!(
                        "Loaded {} role(s) from persistence (role_config bootstrap already done)",
                        persisted.roles.len()
                    );
                    return Self::from_config(persisted).await;
                }
            }
        }

        // No persisted config -- bootstrap from JSON file
        log::info!(
            "No persisted config found, bootstrapping from role_config: '{}'",
            role_config_path
        );
        match Config::load_from_json_file(role_config_path) {
            Ok(mut config) => {
                // Apply default_role override from settings.toml
                if let Some(ref default_role) = device_settings.default_role {
                    let role_name = RoleName::new(default_role);
                    if config.roles.contains_key(&role_name) {
                        log::info!(
                            "Setting selected role to '{}' from settings.toml default_role",
                            default_role
                        );
                        config.selected_role = role_name.clone();
                        config.default_role = role_name;
                    } else {
                        log::warn!(
                            "default_role '{}' not found in role_config; available: {:?}",
                            default_role,
                            config
                                .roles
                                .keys()
                                .map(|k| k.to_string())
                                .collect::<Vec<_>>()
                        );
                    }
                }

                // Save to persistence so subsequent runs use persisted config
                if let Err(e) = config.save().await {
                    log::warn!("Failed to save bootstrapped config to persistence: {:?}", e);
                }

                Self::from_config(config).await
            }
            Err(e) => {
                log::error!(
                    "Failed to load role_config '{}': {:?}. Falling back to embedded defaults.",
                    role_config_path,
                    e
                );
                Self::new_with_embedded_defaults().await
            }
        }
    }

    /// Initialize service strictly from the embedded default configuration.
    async fn new_with_embedded_defaults() -> Result<Self> {
        let config = ConfigBuilder::new_with_id(ConfigId::Embedded)
            .build_default_embedded()
            .build()?;
        Self::from_config(config).await
    }

    async fn from_config(mut config: Config) -> Result<Self> {
        let config_state = ConfigState::new(&mut config).await?;
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

    /// List all available roles with their shortnames
    pub async fn list_roles_with_info(&self) -> Vec<(String, Option<String>)> {
        let config = self.config_state.config.lock().await;
        config
            .roles
            .iter()
            .map(|(name, role)| (name.to_string(), role.shortname.clone()))
            .collect()
    }

    /// Find a role by name or shortname (case-insensitive)
    pub async fn find_role_by_name_or_shortname(&self, query: &str) -> Option<RoleName> {
        let config = self.config_state.config.lock().await;
        let query_lower = query.to_lowercase();

        // First try exact match on name
        for (name, _role) in config.roles.iter() {
            if name.to_string().to_lowercase() == query_lower {
                return Some(name.clone());
            }
        }

        // Then try match on shortname
        for (name, role) in config.roles.iter() {
            if let Some(ref shortname) = role.shortname {
                if shortname.to_lowercase() == query_lower {
                    return Some(name.clone());
                }
            }
        }

        None
    }

    /// Update the selected role
    pub async fn update_selected_role(&self, role_name: RoleName) -> Result<()> {
        let service = self.service.lock().await;
        service.update_selected_role(role_name).await?;
        Ok(())
    }

    /// Save the current configuration
    pub async fn save_config(&self) -> Result<()> {
        let config = self.config_state.config.lock().await;
        config.save().await?;
        Ok(())
    }

    /// Search documents with a specific role
    pub async fn search(
        &self,
        search_term: &str,
        role: &RoleName,
        limit: Option<usize>,
    ) -> Result<Vec<Document>> {
        self.search_with_options(search_term, role, limit, false)
            .await
    }

    /// Search documents with full options including include_pinned
    pub async fn search_with_options(
        &self,
        search_term: &str,
        role: &RoleName,
        limit: Option<usize>,
        include_pinned: bool,
    ) -> Result<Vec<Document>> {
        let query = SearchQuery {
            search_term: NormalizedTermValue::from(search_term),
            search_terms: None,
            operator: None,
            skip: Some(0),
            limit,
            role: Some(role.clone()),
            layer: Layer::default(),
            include_pinned,
        };

        let mut service = self.service.lock().await;
        Ok(service.search(&query).await?)
    }

    /// List KG entries for a role, optionally filtered to pinned entries only.
    ///
    /// Returns a list of `(node_id, term)` pairs. When `pinned_only` is true,
    /// only entries whose node ID appears in the role graph's pinned list are returned.
    pub async fn list_kg_entries(
        &self,
        role_name: &RoleName,
        pinned_only: bool,
    ) -> Result<Vec<serde_json::Value>> {
        if let Some(rolegraph_sync) = self.config_state.roles.get(role_name) {
            let rolegraph = rolegraph_sync.lock().await;
            let pinned_ids: std::collections::HashSet<u64> =
                rolegraph.get_pinned_node_ids().iter().copied().collect();

            let entries: Vec<serde_json::Value> = rolegraph
                .ac_reverse_nterm
                .iter()
                .filter(|(node_id, _)| !pinned_only || pinned_ids.contains(node_id))
                .map(|(node_id, term)| {
                    serde_json::json!({
                        "node_id": node_id,
                        "term": term.to_string(),
                        "pinned": pinned_ids.contains(node_id),
                    })
                })
                .collect();

            Ok(entries)
        } else {
            Ok(Vec::new())
        }
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

    /// Extract matches with grounding metadata
    pub async fn extract_with_grounding(
        &self,
        role_name: &RoleName,
        text: &str,
    ) -> Result<Vec<ExtractedEntity>> {
        let thesaurus = self.get_thesaurus(role_name).await?;
        let matches = terraphim_automata::find_matches(text, thesaurus, true)?;

        let entities: Vec<ExtractedEntity> = matches
            .iter()
            .map(|m| ExtractedEntity {
                entity_type: "term".to_string(),
                raw_value: m.term.clone(),
                normalized_value: Some(m.normalized_term.value.to_string()),
                grounding: Some(GroundingMetadata::new(
                    m.normalized_term
                        .url
                        .clone()
                        .unwrap_or_else(|| format!("kg://{}", m.normalized_term.value)),
                    m.normalized_term.value.to_string(),
                    "terraphim".to_string(),
                    1.0,
                    NormalizationMethod::Exact,
                )),
            })
            .collect();

        Ok(entities)
    }

    /// Extract entities using ontology schema, returning SchemaSignal
    pub fn extract_with_schema(&self, schema: &OntologySchema, text: &str) -> Result<SchemaSignal> {
        let thesaurus = Self::build_thesaurus_from_schema(schema);
        let matches = terraphim_automata::find_matches(text, thesaurus, true)?;

        // Build a lookup from NormalizedTermValue -> entity_type_id
        let entry_lookup: std::collections::HashMap<String, String> = schema
            .to_thesaurus_entries()
            .into_iter()
            .map(|(id, term, _)| (term.to_lowercase(), id))
            .collect();

        let entities: Vec<ExtractedEntity> = matches
            .iter()
            .map(|m| {
                let entity_type_id = entry_lookup
                    .get(&m.normalized_term.value.to_string())
                    .cloned()
                    .unwrap_or_else(|| "unknown".to_string());
                ExtractedEntity {
                    entity_type: entity_type_id.clone(),
                    raw_value: m.term.clone(),
                    normalized_value: Some(m.normalized_term.value.to_string()),
                    grounding: Some(GroundingMetadata::new(
                        schema
                            .uri_for(&entity_type_id)
                            .unwrap_or_else(|| format!("kg://{}", entity_type_id)),
                        m.normalized_term.value.to_string(),
                        schema.name.clone(),
                        1.0,
                        NormalizationMethod::Exact,
                    )),
                }
            })
            .collect();

        Ok(SchemaSignal {
            entities,
            relationships: Vec::new(),
            confidence: if matches.is_empty() { 0.0 } else { 1.0 },
        })
    }

    /// Build a Thesaurus from an OntologySchema's entity types and aliases
    fn build_thesaurus_from_schema(schema: &OntologySchema) -> Thesaurus {
        let entries = schema.to_thesaurus_entries();
        let mut thesaurus = Thesaurus::new(schema.name.clone());
        for (idx, (_id, term, url)) in entries.into_iter().enumerate() {
            let nterm_value = NormalizedTermValue::new(term);
            let mut nterm = NormalizedTerm::new(idx as u64, nterm_value.clone());
            if let Some(url) = url {
                nterm = nterm.with_url(url);
            }
            thesaurus.insert(nterm_value, nterm);
        }
        thesaurus
    }

    /// Calculate coverage of schema categories in text
    pub fn calculate_coverage(
        &self,
        schema: &OntologySchema,
        text: &str,
        threshold: f32,
    ) -> Result<CoverageResult> {
        let signal = self.extract_with_schema(schema, text)?;

        let all_categories = schema.category_ids();
        let matched_ids: std::collections::HashSet<String> = signal
            .entities
            .iter()
            .map(|e| e.entity_type.clone())
            .collect();

        let matched: Vec<String> = all_categories
            .iter()
            .filter(|id| matched_ids.contains(*id))
            .cloned()
            .collect();
        let missing: Vec<String> = all_categories
            .iter()
            .filter(|id| !matched_ids.contains(*id))
            .cloned()
            .collect();

        let coverage = CoverageSignal::compute(&all_categories, matched.len(), threshold);

        Ok(CoverageResult {
            signal: coverage,
            matched_categories: matched,
            missing_categories: missing,
            schema_name: schema.name.clone(),
        })
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

/// Result of ontology coverage analysis
#[derive(Debug, Clone, Serialize)]
pub struct CoverageResult {
    /// Coverage signal with ratio and threshold
    pub signal: CoverageSignal,
    /// Entity type IDs that were matched in the text
    pub matched_categories: Vec<String>,
    /// Entity type IDs that were NOT matched in the text
    pub missing_categories: Vec<String>,
    /// Name of the schema used
    pub schema_name: String,
}
