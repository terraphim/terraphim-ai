use anyhow::Result;
use std::sync::Arc;
use terraphim_config::{Config, ConfigBuilder, ConfigId, ConfigState};
use terraphim_persistence::Persistable;
use terraphim_service::TerraphimService;
use terraphim_service::llm::{ChatOptions, build_llm_from_role};
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
        // Initialize logging
        terraphim_service::logging::init_logging(
            terraphim_service::logging::detect_logging_config(),
        );

        log::info!("Initializing TUI service with embedded configuration");

        // Load device settings, falling back to embedded defaults when running in sandboxes/tests
        let device_settings = match DeviceSettings::load_from_env_and_file(None) {
            Ok(settings) => settings,
            Err(err) => {
                log::warn!(
                    "Failed to load device settings from env/file: {err:?}; using embedded defaults"
                );
                DeviceSettings::default_embedded()
            }
        };
        log::debug!("Device settings: {:?}", device_settings);

        // Try to load existing configuration, fallback to default embedded config
        let config = match ConfigBuilder::new_with_id(ConfigId::Embedded).build() {
            Ok(mut config) => match config.load().await {
                Ok(config) => {
                    log::debug!("Loaded existing embedded configuration");
                    config
                }
                Err(_) => {
                    // No saved config found is expected on first run - use default
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

    /// Initialize service strictly from the embedded default configuration.
    ///
    /// This constructor avoids touching host-specific config/state and is used by tests.
    pub async fn new_with_embedded_defaults() -> Result<Self> {
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

    /// Update the selected role
    pub async fn update_selected_role(
        &self,
        role_name: RoleName,
    ) -> Result<terraphim_config::Config> {
        let service = self.service.lock().await;
        Ok(service.update_selected_role(role_name).await?)
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

    /// Search documents using the current selected role
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
        Ok(service.search(&query).await?)
    }

    /// Search documents using a complete SearchQuery (supports logical operators)
    pub async fn search_with_query(&self, query: &SearchQuery) -> Result<Vec<Document>> {
        let mut service = self.service.lock().await;
        Ok(service.search(query).await?)
    }

    /// Get thesaurus for a specific role
    pub async fn get_thesaurus(&self, role_name: &RoleName) -> Result<Thesaurus> {
        let mut service = self.service.lock().await;
        Ok(service.ensure_thesaurus_loaded(role_name).await?)
    }

    /// Get the role graph top-k concepts for a specific role
    ///
    /// Returns the top-k concepts sorted by rank (number of co-occurrences) in descending order.
    pub async fn get_role_graph_top_k(
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

    /// Generate chat response using LLM
    pub async fn chat(
        &self,
        role_name: &RoleName,
        prompt: &str,
        _model: Option<String>,
    ) -> Result<String> {
        // Get the role configuration
        let config = self.config_state.config.lock().await;
        let role = config
            .roles
            .get(role_name)
            .ok_or_else(|| anyhow::anyhow!("Role '{}' not found in configuration", role_name))?;

        // Build LLM client from role configuration
        let llm_client = build_llm_from_role(role).ok_or_else(|| {
            anyhow::anyhow!(
                "No LLM configured for role '{}'. Add llm_provider, ollama_model, or llm_model to role's extra config.",
                role_name
            )
        })?;

        log::info!(
            "Using LLM provider: {} for role: {}",
            llm_client.name(),
            role_name
        );

        // Build chat messages
        let messages = vec![serde_json::json!({
            "role": "user",
            "content": prompt
        })];

        // Configure chat options
        let opts = ChatOptions {
            max_tokens: Some(1024),
            temperature: Some(0.7),
        };

        // Call the LLM
        let response = llm_client
            .chat_completion(messages, opts)
            .await
            .map_err(|e| anyhow::anyhow!("LLM chat error: {}", e))?;

        Ok(response)
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

    /// Summarize content using available AI services
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

    /// Check if all matched terms in text are connected by a single path in the knowledge graph
    pub async fn check_connectivity(
        &self,
        role_name: &RoleName,
        text: &str,
    ) -> Result<ConnectivityResult> {
        // Get the RoleGraphSync from config_state.roles
        let rolegraph_sync = self
            .config_state
            .roles
            .get(role_name)
            .ok_or_else(|| anyhow::anyhow!("RoleGraph not loaded for role '{}'", role_name))?;

        // Lock the RoleGraph and check connectivity
        let rolegraph = rolegraph_sync.lock().await;

        // Find matched terms for reporting
        let matched_node_ids = rolegraph.find_matching_node_ids(text);

        if matched_node_ids.is_empty() {
            return Ok(ConnectivityResult {
                connected: true, // Trivially connected if no terms
                matched_terms: vec![],
                message: format!(
                    "No terms from role '{}' knowledge graph found in the provided text.",
                    role_name
                ),
            });
        }

        // Get term names for the matched node IDs
        let matched_terms: Vec<String> = matched_node_ids
            .iter()
            .filter_map(|node_id| {
                rolegraph
                    .ac_reverse_nterm
                    .get(node_id)
                    .map(|nterm| nterm.to_string())
            })
            .collect();

        // Check actual graph connectivity
        let is_connected = rolegraph.is_all_terms_connected_by_path(text);

        let message = if is_connected {
            "All matched terms are connected by a single path in the knowledge graph.".to_string()
        } else {
            "The matched terms are NOT all connected by a single path.".to_string()
        };

        Ok(ConnectivityResult {
            connected: is_connected,
            matched_terms,
            message,
        })
    }

    /// Perform fuzzy autocomplete search
    pub async fn fuzzy_suggest(
        &self,
        role_name: &RoleName,
        query: &str,
        threshold: f64,
        limit: Option<usize>,
    ) -> Result<Vec<FuzzySuggestion>> {
        // Get thesaurus for the role
        let thesaurus = self.get_thesaurus(role_name).await?;

        // Build autocomplete index
        let config = Some(terraphim_automata::AutocompleteConfig {
            max_results: limit.unwrap_or(10),
            min_prefix_length: 1,
            case_sensitive: false,
        });

        let index = terraphim_automata::build_autocomplete_index(thesaurus, config)?;

        // Perform fuzzy search
        let results =
            terraphim_automata::fuzzy_autocomplete_search(&index, query, threshold, limit)?;

        // Convert to FuzzySuggestion
        Ok(results
            .into_iter()
            .map(|r| FuzzySuggestion {
                term: r.term,
                similarity: r.score,
            })
            .collect())
    }

    /// Validate text against a named checklist
    pub async fn validate_checklist(
        &self,
        role_name: &RoleName,
        checklist_name: &str,
        text: &str,
    ) -> Result<ChecklistResult> {
        // Define checklists with their required terms
        // These are the synonyms from the checklist markdown files
        let checklists = std::collections::HashMap::from([
            (
                "code_review",
                vec![
                    "tests",
                    "test",
                    "testing",
                    "unit test",
                    "integration test",
                    "documentation",
                    "docs",
                    "comments",
                    "error handling",
                    "exception handling",
                    "security",
                    "security check",
                    "performance",
                    "optimization",
                ],
            ),
            (
                "security",
                vec![
                    "authentication",
                    "auth",
                    "login",
                    "authorization",
                    "access control",
                    "permissions",
                    "input validation",
                    "sanitization",
                    "encryption",
                    "encrypted",
                    "ssl",
                    "tls",
                    "logging",
                    "audit log",
                ],
            ),
        ]);

        // Get checklist items or return error for unknown checklist
        let checklist_terms = checklists.get(checklist_name).ok_or_else(|| {
            anyhow::anyhow!(
                "Unknown checklist '{}'. Available: {:?}",
                checklist_name,
                checklists.keys().collect::<Vec<_>>()
            )
        })?;

        // Find matches in the text
        let matches = self.find_matches(role_name, text).await?;
        let matched_terms: std::collections::HashSet<String> =
            matches.iter().map(|m| m.term.to_lowercase()).collect();

        // Group checklist items by category (first word is typically the category)
        let categories = vec![
            (
                "tests",
                vec!["tests", "test", "testing", "unit test", "integration test"],
            ),
            ("documentation", vec!["documentation", "docs", "comments"]),
            (
                "error_handling",
                vec!["error handling", "exception handling"],
            ),
            ("security", vec!["security", "security check"]),
            ("performance", vec!["performance", "optimization"]),
            ("authentication", vec!["authentication", "auth", "login"]),
            (
                "authorization",
                vec!["authorization", "access control", "permissions"],
            ),
            ("input_validation", vec!["input validation", "sanitization"]),
            ("encryption", vec!["encryption", "encrypted", "ssl", "tls"]),
            ("logging", vec!["logging", "audit log"]),
        ];

        // Filter categories relevant to this checklist
        let relevant_categories: Vec<_> = categories
            .iter()
            .filter(|(_, terms)| terms.iter().any(|t| checklist_terms.contains(t)))
            .collect();

        let mut satisfied = Vec::new();
        let mut missing = Vec::new();

        for (category, terms) in &relevant_categories {
            // Check if any term in the category is matched
            let found = terms
                .iter()
                .any(|t| matched_terms.contains(&t.to_lowercase()));
            if found {
                satisfied.push(category.to_string());
            } else {
                missing.push(category.to_string());
            }
        }

        let total_items = satisfied.len() + missing.len();
        let passed = missing.is_empty();

        Ok(ChecklistResult {
            checklist_name: checklist_name.to_string(),
            passed,
            total_items,
            satisfied,
            missing,
        })
    }

    /// Add a new role to the configuration
    ///
    /// This adds the role to the existing config and saves it.
    /// If a role with the same name exists, it will be replaced.
    pub async fn add_role(&self, role: terraphim_config::Role) -> Result<()> {
        {
            let mut config = self.config_state.config.lock().await;
            let role_name = role.name.clone();
            config.roles.insert(role_name.clone(), role);
            log::info!("Added role '{}' to configuration", role_name);
        }
        self.save_config().await?;
        Ok(())
    }

    /// Set the configuration to use a single role
    ///
    /// This replaces the current config with a new one containing only this role,
    /// and sets it as the selected role.
    pub async fn set_role(&self, role: terraphim_config::Role) -> Result<()> {
        {
            let mut config = self.config_state.config.lock().await;
            let role_name = role.name.clone();
            config.roles.clear();
            config.roles.insert(role_name.clone(), role);
            config.selected_role = role_name.clone();
            log::info!(
                "Set configuration to role '{}' (cleared other roles)",
                role_name
            );
        }
        self.save_config().await?;
        Ok(())
    }
}

/// Result of connectivity check
#[derive(Debug, Clone, serde::Serialize)]
pub struct ConnectivityResult {
    pub connected: bool,
    pub matched_terms: Vec<String>,
    pub message: String,
}

/// Fuzzy suggestion result
#[derive(Debug, Clone, serde::Serialize)]
pub struct FuzzySuggestion {
    pub term: String,
    pub similarity: f64,
}

/// Checklist validation result
#[derive(Debug, Clone, serde::Serialize)]
pub struct ChecklistResult {
    pub checklist_name: String,
    pub passed: bool,
    pub total_items: usize,
    pub satisfied: Vec<String>,
    pub missing: Vec<String>,
}
