use anyhow::Result;
use std::sync::Arc;
use terraphim_config::{ConfigBuilder, ConfigId, ConfigState};
use terraphim_persistence::Persistable;
use terraphim_service::TerraphimService;
use terraphim_settings::DeviceSettings;
use terraphim_types::{Document, NormalizedTermValue, RoleName, SearchQuery, Thesaurus};
use tokio::sync::Mutex;

#[derive(Clone)]
pub struct TuiService {
    config_state: ConfigState,
    service: Arc<Mutex<TerraphimService>>,
}

impl TuiService {
    /// Initialize a new TUI service with embedded configuration
    pub async fn new() -> Result<Self> {
        Self::new_internal(None).await
    }

    /// Initialize a new TUI service with a custom configuration file
    pub async fn new_with_config_file(config_path: &str) -> Result<Self> {
        // Validate config file exists and is readable before proceeding
        Self::validate_config_file(config_path)?;
        Self::new_internal(Some(config_path)).await
    }

    /// Validate that the config file exists and is readable
    fn validate_config_file(config_path: &str) -> Result<()> {
        use std::fs;
        
        // Check if file exists
        if !std::path::Path::new(config_path).exists() {
            return Err(anyhow::anyhow!(
                "Configuration file not found: '{}'\n\
                Please ensure the file exists and the path is correct.\n\
                Example: terraphim-tui --config /path/to/config.json search query",
                config_path
            ));
        }

        // Check if file is readable
        match fs::metadata(config_path) {
            Ok(metadata) => {
                if metadata.is_dir() {
                    return Err(anyhow::anyhow!(
                        "Configuration path is a directory, not a file: '{}'\n\
                        Please provide a path to a JSON configuration file.",
                        config_path
                    ));
                }
                
                if metadata.len() == 0 {
                    return Err(anyhow::anyhow!(
                        "Configuration file is empty: '{}'\n\
                        Please provide a valid JSON configuration file.",
                        config_path
                    ));
                }
            }
            Err(e) => {
                return Err(anyhow::anyhow!(
                    "Cannot read configuration file '{}': {}\n\
                    Please check file permissions and ensure the file is accessible.",
                    config_path, e
                ));
            }
        }

        Ok(())
    }

    /// Validate the loaded configuration content for required fields and structure
    fn validate_config_content(config: &terraphim_config::Config) -> Result<()> {
        // Check if selected_role exists in roles
        if !config.roles.contains_key(&config.selected_role) {
            return Err(anyhow::anyhow!(
                "Invalid configuration: selected_role '{}' not found in roles.\n\
                Available roles: {:?}\n\
                Please ensure the selected_role matches one of the defined role names.",
                config.selected_role,
                config.roles.keys().collect::<Vec<_>>()
            ));
        }

        // Check if roles is not empty
        if config.roles.is_empty() {
            return Err(anyhow::anyhow!(
                "Invalid configuration: no roles defined.\n\
                At least one role must be configured in the 'roles' section."
            ));
        }

        // Validate each role has required fields
        for (role_name, role) in &config.roles {
            if role.haystacks.is_empty() {
                return Err(anyhow::anyhow!(
                    "Invalid configuration: role '{}' has no haystacks defined.\n\
                    Each role must have at least one haystack configured.",
                    role_name
                ));
            }

            // Validate each haystack has required fields
            for (i, haystack) in role.haystacks.iter().enumerate() {
                if haystack.location.is_empty() {
                    return Err(anyhow::anyhow!(
                        "Invalid configuration: haystack {} in role '{}' has empty location.\n\
                        Each haystack must specify a valid location path.",
                        i + 1, role_name
                    ));
                }
            }
        }

        log::debug!("Configuration validation passed for config with {} roles", config.roles.len());
        Ok(())
    }

    /// Internal implementation for service initialization
    async fn new_internal(config_path: Option<&str>) -> Result<Self> {
        // Initialize logging
        terraphim_service::logging::init_logging(
            terraphim_service::logging::detect_logging_config(),
        );

        if let Some(path) = config_path {
            log::info!("Initializing TUI service with custom configuration from: {}", path);
        } else {
            log::info!("Initializing TUI service with embedded configuration");
        }

        // Load device settings
        let device_settings = DeviceSettings::load_from_env_and_file(None)?;
        log::debug!("Device settings: {:?}", device_settings);

        // Load configuration from custom file or use embedded config
        let mut config = if let Some(config_file) = config_path {
            // Load from custom config file
            match std::fs::read_to_string(config_file) {
                Ok(config_content) => {
                    match serde_json::from_str::<terraphim_config::Config>(&config_content) {
                        Ok(config) => {
                            log::info!("Successfully loaded custom configuration from: {}", config_file);
                            // Validate the loaded configuration
                            Self::validate_config_content(&config)?;
                            config
                        }
                        Err(e) => {
                            log::error!("Failed to parse config file {}: {}", config_file, e);
                            return Err(anyhow::anyhow!(
                                "Failed to parse configuration file '{}': {}\n\n\
                                Common issues:\n\
                                • Check that the file contains valid JSON syntax\n\
                                • Ensure all required fields are present (id, selected_role, roles)\n\
                                • Verify that role configurations are properly formatted\n\n\
                                Example valid configuration:\n\
                                {{\n  \
                                  \"id\": \"Custom\",\n  \
                                  \"selected_role\": \"MyRole\",\n  \
                                  \"roles\": {{\n    \
                                    \"MyRole\": {{\n      \
                                      \"name\": \"MyRole\",\n      \
                                      \"relevance_function\": \"title-scorer\",\n      \
                                      \"haystacks\": [...]\n    \
                                    }}\n  \
                                  }}\n\
                                }}",
                                config_file, e
                            ));
                        }
                    }
                }
                Err(e) => {
                    log::error!("Failed to read config file {}: {}", config_file, e);
                    return Err(anyhow::anyhow!(
                        "Failed to read configuration file '{}': {}\n\
                        Please ensure the file exists and is readable.",
                        config_file, e
                    ));
                }
            }
        } else {
            // Use embedded configuration (existing logic)
            match ConfigBuilder::new_with_id(ConfigId::Embedded).build() {
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

    /// Update the selected role
    pub async fn update_selected_role(
        &self,
        role_name: RoleName,
    ) -> Result<terraphim_config::Config> {
        let service = self.service.lock().await;
        Ok(service.update_selected_role(role_name).await?)
    }

    /// List all available roles
    pub async fn list_roles(&self) -> Vec<String> {
        let config = self.config_state.config.lock().await;
        config.roles.keys().map(|r| r.to_string()).collect()
    }

    /// Search documents using the current selected role
    #[allow(dead_code)]
    pub async fn search(&self, search_term: &str, limit: Option<usize>) -> Result<Vec<Document>> {
        let selected_role = self.get_selected_role().await;
        self.search_with_role(search_term, &selected_role, limit)
            .await
    }

    /// Search documents with a specific role
    pub async fn search_with_role(
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
        Ok(service.search(&query).await?.documents)
    }

    /// Search documents using a complete SearchQuery (supports logical operators)
    pub async fn search_with_query(&self, query: &SearchQuery) -> Result<Vec<Document>> {
        let mut service = self.service.lock().await;
        Ok(service.search(query).await?.documents)
    }

    /// Get thesaurus for a specific role
    pub async fn get_thesaurus(&self, role_name: &RoleName) -> Result<Thesaurus> {
        let mut service = self.service.lock().await;
        Ok(service.ensure_thesaurus_loaded(role_name).await?)
    }

    /// Get the role graph top-k concepts for a specific role
    pub async fn get_role_graph_top_k(
        &self,
        role_name: &RoleName,
        top_k: usize,
    ) -> Result<Vec<String>> {
        // For now, return placeholder data since role graph access needs proper implementation
        // TODO: Implement actual role graph integration
        log::info!("Getting top {} concepts for role {}", top_k, role_name);
        Ok((0..std::cmp::min(top_k, 10))
            .map(|i| format!("concept_{}_for_role_{}", i + 1, role_name))
            .collect())
    }

    /// Generate chat response using LLM
    pub async fn chat(
        &self,
        role_name: &RoleName,
        prompt: &str,
        model: Option<String>,
    ) -> Result<String> {
        // Check if role has LLM configuration
        let config = self.config_state.config.lock().await;
        if let Some(role) = config.roles.get(role_name) {
            // Check for various LLM providers in the role's extra config
            if let Some(llm_provider) = role.extra.get("llm_provider") {
                if let Some(provider_str) = llm_provider.as_str() {
                    log::info!("Using LLM provider: {}", provider_str);
                    // Use the service's LLM capabilities
                    let _service = self.service.lock().await;
                    // For now, return a placeholder response
                    // TODO: Implement actual LLM integration when service supports it
                    return Ok(format!(
                        "Chat response from {} with model {:?}: {}",
                        provider_str, model, prompt
                    ));
                }
            }
        }

        // Fallback response
        Ok(format!(
            "No LLM configured for role {}. Prompt was: {}",
            role_name, prompt
        ))
    }

    /// Extract paragraphs from text using thesaurus
    pub async fn extract_paragraphs(
        &self,
        role_name: &RoleName,
        text: &str,
        exclude_term: bool,
    ) -> Result<Vec<(String, String)>> {
        // Get thesaurus for the role
        let thesaurus = self.get_thesaurus(role_name).await?;

        // Use automata to extract paragraphs
        let results = terraphim_automata::matcher::extract_paragraphs_from_automata(
            text,
            thesaurus,
            !exclude_term, // include_term is opposite of exclude_term
        )?;

        // Convert to string tuples
        let string_results = results
            .into_iter()
            .map(|(matched, paragraph)| (matched.normalized_term.value.to_string(), paragraph))
            .collect();

        Ok(string_results)
    }

    /// Perform autocomplete search using thesaurus for a role
    #[allow(dead_code)]
    pub async fn autocomplete(
        &self,
        role_name: &RoleName,
        query: &str,
        limit: Option<usize>,
    ) -> Result<Vec<terraphim_automata::AutocompleteResult>> {
        // Get thesaurus for the role
        let thesaurus = self.get_thesaurus(role_name).await?;

        // Build autocomplete index
        let config = Some(terraphim_automata::AutocompleteConfig {
            max_results: limit.unwrap_or(10),
            min_prefix_length: 1,
            case_sensitive: false,
        });

        let index = terraphim_automata::build_autocomplete_index(thesaurus, config)?;

        // Perform search
        Ok(terraphim_automata::autocomplete_search(
            &index, query, limit,
        )?)
    }

    /// Find matches in text using thesaurus
    #[allow(dead_code)]
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
    #[allow(dead_code)]
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

    /// Summarize content using available AI services
    #[allow(dead_code)]
    pub async fn summarize(&self, role_name: &RoleName, content: &str) -> Result<String> {
        // For now, use the chat method with a summarization prompt
        let prompt = format!("Please summarize the following content:\n\n{}", content);
        self.chat(role_name, &prompt, None).await
    }

    /// Save configuration changes
    pub async fn save_config(&self) -> Result<()> {
        let config = self.config_state.config.lock().await;
        config.save().await?;
        Ok(())
    }
}
