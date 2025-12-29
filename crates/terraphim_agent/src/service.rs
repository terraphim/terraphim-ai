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
        // Initialize logging
        terraphim_service::logging::init_logging(
            terraphim_service::logging::detect_logging_config(),
        );

        log::info!("Initializing TUI service with embedded configuration");

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
