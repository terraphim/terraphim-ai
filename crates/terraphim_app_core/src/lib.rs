//! Shared app-core service facade.
//!
//! This crate exists to reduce duplication between the various Terraphim
//! binaries (`terraphim-agent`, `terraphim-cli`, etc.). It provides a single
//! canonical initialization path (embedded config + persisted overrides) and
//! shared high-level operations.

use anyhow::Result;
use std::sync::Arc;
use terraphim_config::{ConfigBuilder, ConfigId, ConfigState};
use terraphim_persistence::Persistable;
use terraphim_service::llm::{build_llm_from_role, ChatOptions};
use terraphim_service::TerraphimService;
use terraphim_settings::DeviceSettings;
use terraphim_types::{Document, NormalizedTermValue, RoleName, SearchQuery, Thesaurus};
use tokio::sync::Mutex;

#[derive(Clone)]
pub struct AppService {
    config_state: ConfigState,
    service: Arc<Mutex<TerraphimService>>,
}

impl AppService {
    /// Initialize a new service using embedded defaults and persisted overrides.
    pub async fn new_embedded() -> Result<Self> {
        // Initialize logging
        terraphim_service::logging::init_logging(terraphim_service::logging::detect_logging_config());

        log::info!("Initializing app service with embedded configuration");

        // Load device settings (currently used for side effects / future extension)
        let device_settings = DeviceSettings::load_from_env_and_file(None)?;
        log::debug!("Device settings: {:?}", device_settings);

        // Try to load existing configuration, fallback to default embedded config
        let mut config = match ConfigBuilder::new_with_id(ConfigId::Embedded).build() {
            Ok(mut config) => match config.load().await {
                Ok(config) => {
                    log::debug!("Loaded existing embedded configuration");
                    config
                }
                Err(_) => {
                    // No saved config found is expected on first run - use default
                    log::debug!("No saved config found, using default embedded");
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

    /// Get the current configuration.
    pub async fn get_config(&self) -> terraphim_config::Config {
        let config = self.config_state.config.lock().await;
        config.clone()
    }

    /// Get the currently selected role.
    pub async fn get_selected_role(&self) -> RoleName {
        let config = self.config_state.config.lock().await;
        config.selected_role.clone()
    }

    /// List all available roles.
    pub async fn list_roles(&self) -> Vec<String> {
        let config = self.config_state.config.lock().await;
        config.roles.keys().map(|r| r.to_string()).collect()
    }

    /// List all available roles with their shortnames.
    pub async fn list_roles_with_info(&self) -> Vec<(String, Option<String>)> {
        let config = self.config_state.config.lock().await;
        config
            .roles
            .iter()
            .map(|(name, role)| (name.to_string(), role.shortname.clone()))
            .collect()
    }

    /// Find a role by name or shortname (case-insensitive).
    pub async fn find_role_by_name_or_shortname(&self, query: &str) -> Option<RoleName> {
        let config = self.config_state.config.lock().await;
        let query = query.trim();
        // Allow selection from CLI list outputs which may prefix the selected
        // role with a marker like "* ".
        let query = query.trim_start_matches('*').trim();
        let query = if let Some((name, _rest)) = query.rsplit_once(" (") {
            // Allow selecting roles from display strings like "RoleName (short)"
            // which is what some UIs/tests print.
            if query.ends_with(')') {
                name.trim()
            } else {
                query
            }
        } else {
            query
        };
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

    /// Update the selected role.
    pub async fn update_selected_role(&self, role_name: RoleName) -> Result<terraphim_config::Config> {
        let service = self.service.lock().await;
        Ok(service.update_selected_role(role_name).await?)
    }

    /// Set the selected role without validating it exists.
    ///
    /// This is used for workflows/tests that want to persist a role name first
    /// and define/populate it later.
    pub async fn set_selected_role_unchecked(&self, role_name: RoleName) -> Result<()> {
        let mut config = self.config_state.config.lock().await;
        config.selected_role = role_name;
        config.save().await?;
        Ok(())
    }

    /// Save configuration changes.
    pub async fn save_config(&self) -> Result<()> {
        let config = self.config_state.config.lock().await;
        config.save().await?;
        Ok(())
    }

    /// Search documents using the current selected role.
    pub async fn search(&self, search_term: &str, limit: Option<usize>) -> Result<Vec<Document>> {
        let selected_role = self.get_selected_role().await;
        self.search_with_role(search_term, &selected_role, limit).await
    }

    /// Search documents with a specific role.
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

    /// Search documents using a complete SearchQuery (supports logical operators).
    pub async fn search_with_query(&self, query: &SearchQuery) -> Result<Vec<Document>> {
        let mut service = self.service.lock().await;
        Ok(service.search(query).await?)
    }

    /// Get thesaurus for a specific role.
    pub async fn get_thesaurus(&self, role_name: &RoleName) -> Result<Thesaurus> {
        let mut service = self.service.lock().await;
        Ok(service.ensure_thesaurus_loaded(role_name).await?)
    }

    /// Get the role graph top-k concepts for a specific role.
    ///
    /// Returns the top-k concepts sorted by rank (number of co-occurrences) in descending order.
    pub async fn get_role_graph_top_k(&self, role_name: &RoleName, top_k: usize) -> Result<Vec<String>> {
        log::info!("Getting top {} concepts for role {}", top_k, role_name);

        if let Some(rolegraph_sync) = self.config_state.roles.get(role_name) {
            let rolegraph = rolegraph_sync.lock().await;

            let mut nodes: Vec<_> = rolegraph.nodes_map().iter().collect();
            nodes.sort_by(|a, b| b.1.rank.cmp(&a.1.rank));

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

            Ok(top_concepts)
        } else {
            log::warn!("Role graph not found for role {}", role_name);
            Ok(Vec::new())
        }
    }

    /// Generate chat response using LLM.
    pub async fn chat(&self, role_name: &RoleName, prompt: &str, _model: Option<String>) -> Result<String> {
        let config = self.config_state.config.lock().await;
        let Some(role) = config.roles.get(role_name) else {
            // Fail-open UX: allow users to set arbitrary roles (e.g. in tests)
            // without hard-failing chat.
            return Ok(format!(
                "Role '{}' not found in configuration",
                role_name
            ));
        };

        let Some(llm_client) = build_llm_from_role(role) else {
            // Fail-open UX: chat is optional and may be unconfigured.
            return Ok(format!(
                "No LLM configured for role '{}'. Add llm_provider, ollama_model, or llm_model to role's extra config.",
                role_name
            ));
        };

        let messages = vec![serde_json::json!({
            "role": "user",
            "content": prompt
        })];

        let opts = ChatOptions {
            max_tokens: Some(1024),
            temperature: Some(0.7),
        };

        let response = llm_client
            .chat_completion(messages, opts)
            .await
            .map_err(|e| anyhow::anyhow!("LLM chat error: {}", e))?;

        Ok(response)
    }

    /// Extract paragraphs from text using thesaurus.
    pub async fn extract_paragraphs(
        &self,
        role_name: &RoleName,
        text: &str,
        exclude_term: bool,
    ) -> Result<Vec<(String, String)>> {
        let thesaurus = self.get_thesaurus(role_name).await?;

        let results = terraphim_automata::matcher::extract_paragraphs_from_automata(
            text,
            thesaurus,
            !exclude_term,
        )?;

        Ok(results
            .into_iter()
            .map(|(matched, paragraph)| (matched.normalized_term.value.to_string(), paragraph))
            .collect())
    }

    /// Perform autocomplete search using thesaurus for a role.
    pub async fn autocomplete(
        &self,
        role_name: &RoleName,
        query: &str,
        limit: Option<usize>,
    ) -> Result<Vec<terraphim_automata::AutocompleteResult>> {
        let thesaurus = self.get_thesaurus(role_name).await?;

        let config = Some(terraphim_automata::AutocompleteConfig {
            max_results: limit.unwrap_or(10),
            min_prefix_length: 1,
            case_sensitive: false,
        });

        let index = terraphim_automata::build_autocomplete_index(thesaurus, config)?;
        Ok(terraphim_automata::autocomplete_search(&index, query, limit)?)
    }

    /// Find matches in text using thesaurus.
    pub async fn find_matches(
        &self,
        role_name: &RoleName,
        text: &str,
    ) -> Result<Vec<terraphim_automata::Matched>> {
        let thesaurus = self.get_thesaurus(role_name).await?;
        Ok(terraphim_automata::find_matches(text, thesaurus, true)?)
    }

    /// Replace matches in text with links using thesaurus.
    pub async fn replace_matches(
        &self,
        role_name: &RoleName,
        text: &str,
        link_type: terraphim_automata::LinkType,
    ) -> Result<String> {
        let thesaurus = self.get_thesaurus(role_name).await?;
        let result = terraphim_automata::replace_matches(text, thesaurus, link_type)?;
        Ok(String::from_utf8(result).unwrap_or_else(|_| text.to_string()))
    }

    /// Summarize content using available AI services.
    pub async fn summarize(&self, role_name: &RoleName, content: &str) -> Result<String> {
        let prompt = format!("Please summarize the following content:\n\n{}", content);
        self.chat(role_name, &prompt, None).await
    }

    /// Check if all matched terms in text are connected by a single path in the knowledge graph.
    pub async fn check_connectivity(
        &self,
        role_name: &RoleName,
        text: &str,
    ) -> Result<ConnectivityResult> {
        let rolegraph_sync = self
            .config_state
            .roles
            .get(role_name)
            .ok_or_else(|| anyhow::anyhow!("RoleGraph not loaded for role '{}'", role_name))?;

        let rolegraph = rolegraph_sync.lock().await;
        let matched_node_ids = rolegraph.find_matching_node_ids(text);

        if matched_node_ids.is_empty() {
            return Ok(ConnectivityResult {
                connected: true,
                matched_terms: vec![],
                message: format!(
                    "No terms from role '{}' knowledge graph found in the provided text.",
                    role_name
                ),
            });
        }

        let matched_terms: Vec<String> = matched_node_ids
            .iter()
            .filter_map(|node_id| {
                rolegraph
                    .ac_reverse_nterm
                    .get(node_id)
                    .map(|nterm| nterm.to_string())
            })
            .collect();

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

    /// Perform fuzzy autocomplete search.
    pub async fn fuzzy_suggest(
        &self,
        role_name: &RoleName,
        query: &str,
        threshold: f64,
        limit: Option<usize>,
    ) -> Result<Vec<FuzzySuggestion>> {
        let thesaurus = self.get_thesaurus(role_name).await?;

        let config = Some(terraphim_automata::AutocompleteConfig {
            max_results: limit.unwrap_or(10),
            min_prefix_length: 1,
            case_sensitive: false,
        });

        let index = terraphim_automata::build_autocomplete_index(thesaurus, config)?;
        let results = terraphim_automata::fuzzy_autocomplete_search(&index, query, threshold, limit)?;

        Ok(results
            .into_iter()
            .map(|r| FuzzySuggestion {
                term: r.term,
                similarity: r.score,
            })
            .collect())
    }

    /// Validate text against a named checklist.
    pub async fn validate_checklist(
        &self,
        role_name: &RoleName,
        checklist_name: &str,
        text: &str,
    ) -> Result<ChecklistResult> {
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

        let checklist_terms = checklists.get(checklist_name).ok_or_else(|| {
            anyhow::anyhow!(
                "Unknown checklist '{}'. Available: {:?}",
                checklist_name,
                checklists.keys().collect::<Vec<_>>()
            )
        })?;

        let matches = self.find_matches(role_name, text).await?;
        let matched_terms: std::collections::HashSet<String> =
            matches.iter().map(|m| m.term.to_lowercase()).collect();

        let categories = vec![
            (
                "tests",
                vec!["tests", "test", "testing", "unit test", "integration test"],
            ),
            ("documentation", vec!["documentation", "docs", "comments"]),
            ("error_handling", vec!["error handling", "exception handling"]),
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

        let relevant_categories: Vec<_> = categories
            .iter()
            .filter(|(_, terms)| terms.iter().any(|t| checklist_terms.contains(t)))
            .collect();

        let mut satisfied = Vec::new();
        let mut missing = Vec::new();

        for (category, terms) in &relevant_categories {
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

/// Result of connectivity check.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ConnectivityResult {
    pub connected: bool,
    pub matched_terms: Vec<String>,
    pub message: String,
}

/// Fuzzy suggestion result.
#[derive(Debug, Clone, serde::Serialize)]
pub struct FuzzySuggestion {
    pub term: String,
    pub similarity: f64,
}

/// Checklist validation result.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ChecklistResult {
    pub checklist_name: String,
    pub passed: bool,
    pub total_items: usize,
    pub satisfied: Vec<String>,
    pub missing: Vec<String>,
}
