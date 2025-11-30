use std::sync::Arc;

use anyhow::Result;
use base64::Engine;
use rmcp::{
    model::{
        CallToolRequestParam, CallToolResult, Content, ErrorData, ListResourcesResult,
        ListToolsResult, ReadResourceRequestParam, ReadResourceResult, ServerInfo, Tool,
    },
    service::RequestContext,
    RoleServer, ServerHandler,
};
use terraphim_automata::builder::json_decode;
use terraphim_automata::matcher::{
    extract_paragraphs_from_automata, find_matches, replace_matches,
};
use terraphim_automata::{AutocompleteConfig, AutocompleteIndex, AutocompleteResult};
use terraphim_config::{Config, ConfigState};
use terraphim_service::TerraphimService;
use terraphim_types::{NormalizedTermValue, RoleName, SearchQuery};
use thiserror::Error;
use tracing::{error, info};

pub mod resource_mapper;

use crate::resource_mapper::TerraphimResourceMapper;

#[derive(Error, Debug)]
pub enum TerraphimMcpError {
    #[error("Service error: {0}")]
    Service(#[from] terraphim_service::ServiceError),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("MCP error: {0}")]
    Mcp(#[from] ErrorData),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Anyhow error: {0}")]
    Anyhow(#[from] anyhow::Error),
}

impl From<TerraphimMcpError> for ErrorData {
    fn from(err: TerraphimMcpError) -> Self {
        ErrorData::internal_error(err.to_string(), None)
    }
}

/// The main service type for the Terraphim MCP server
#[derive(Clone)]
pub struct McpService {
    config_state: Arc<ConfigState>,
    resource_mapper: Arc<TerraphimResourceMapper>,
    autocomplete_index: Arc<tokio::sync::RwLock<Option<AutocompleteIndex>>>,
}

impl McpService {
    /// Create a new service instance
    pub fn new(config_state: Arc<ConfigState>) -> Self {
        Self {
            config_state,
            resource_mapper: Arc::new(TerraphimResourceMapper::new()),
            autocomplete_index: Arc::new(tokio::sync::RwLock::new(None)),
        }
    }

    /// Initialize autocomplete index for the currently selected role, if possible
    pub async fn init_autocomplete_default(&self) {
        if let Ok(result) = self.build_autocomplete_index(None).await {
            if result.is_error == Some(true) {
                tracing::debug!("Autocomplete init skipped: {:?}", result.content);
            } else {
                tracing::info!("Autocomplete index initialized by default");
            }
        } else {
            tracing::debug!("Autocomplete init failed");
        }
    }

    /// Create a Terraphim service instance from the current configuration
    pub async fn terraphim_service(&self) -> Result<TerraphimService, anyhow::Error> {
        // Instead of cloning the old ConfigState (which has stale roles),
        // create a fresh ConfigState from the current config to ensure roles are up-to-date
        let config = self.config_state.config.clone();
        let current_config = config.lock().await;
        let mut fresh_config = current_config.clone();
        drop(current_config);

        let fresh_config_state = terraphim_config::ConfigState::new(&mut fresh_config)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to create fresh ConfigState: {}", e))?;

        Ok(TerraphimService::new(fresh_config_state))
    }

    /// Update the configuration
    pub async fn update_config(&self, new_config: Config) -> Result<()> {
        let config = self.config_state.config.clone();
        let mut current_config = config.lock().await;
        *current_config = new_config;
        Ok(())
    }

    /// Search for documents in the Terraphim knowledge graph
    pub async fn search(
        &self,
        query: String,
        role: Option<String>,
        limit: Option<i32>,
        skip: Option<i32>,
    ) -> Result<CallToolResult, ErrorData> {
        let mut service = self
            .terraphim_service()
            .await
            .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;

        // Determine which role to use (provided role or selected role)
        let role_name = if let Some(role_str) = role {
            RoleName::from(role_str)
        } else {
            self.config_state.get_selected_role().await
        };

        let search_query = SearchQuery {
            search_term: NormalizedTermValue::from(query),
            search_terms: None,
            operator: None,
            role: Some(role_name),
            limit: limit.map(|l| l as usize),
            skip: skip.map(|s| s as usize),
        };

        match service.search(&search_query).await {
            Ok(documents) => {
                let mut contents = Vec::new();
                let summary = format!("Found {} documents matching your query.", documents.len());
                contents.push(Content::text(summary));

                let limit = limit.unwrap_or(documents.len() as i32) as usize;
                for (idx, doc) in documents.iter().enumerate() {
                    if idx >= limit {
                        break;
                    }

                    let resource_contents = self
                        .resource_mapper
                        .document_to_resource_contents(doc)
                        .unwrap();
                    contents.push(Content::resource(resource_contents));
                }

                Ok(CallToolResult::success(contents))
            }
            Err(e) => {
                error!("Search failed: {}", e);
                let error_content = Content::text(format!("Search failed: {}", e));
                Ok(CallToolResult::error(vec![error_content]))
            }
        }
    }

    /// Update the Terraphim configuration
    pub async fn update_config_tool(
        &self,
        config_str: String,
    ) -> Result<CallToolResult, ErrorData> {
        match serde_json::from_str::<Config>(&config_str) {
            Ok(new_config) => match self.update_config(new_config).await {
                Ok(()) => {
                    let content = Content::text("Configuration updated successfully".to_string());
                    Ok(CallToolResult::success(vec![content]))
                }
                Err(e) => {
                    error!("Failed to update configuration: {}", e);
                    let error_content =
                        Content::text(format!("Failed to update configuration: {}", e));
                    Ok(CallToolResult::error(vec![error_content]))
                }
            },
            Err(e) => {
                error!("Failed to parse config: {}", e);
                let error_content = Content::text(format!("Invalid configuration JSON: {}", e));
                Ok(CallToolResult::error(vec![error_content]))
            }
        }
    }

    /// Build autocomplete index from current role's thesaurus
    pub async fn build_autocomplete_index(
        &self,
        role: Option<String>,
    ) -> Result<CallToolResult, ErrorData> {
        let mut service = self
            .terraphim_service()
            .await
            .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;

        // Determine which role to use (provided role or selected role)
        let role_name = if let Some(role_str) = role {
            RoleName::from(role_str)
        } else {
            self.config_state.get_selected_role().await
        };

        // Check if the role exists and has proper knowledge graph configuration
        let role_config = self.config_state.get_role(&role_name).await;
        if let Some(role_cfg) = role_config {
            // Check if role uses TerraphimGraph relevance function (required for knowledge graph)
            if role_cfg.relevance_function != terraphim_types::RelevanceFunction::TerraphimGraph {
                let error_content = Content::text(format!(
                    "Role '{}' does not use knowledge graph ranking (TerraphimGraph). Autocomplete is only available for roles with knowledge graph-based ranking. Current relevance function: {:?}",
                    role_name, role_cfg.relevance_function
                ));
                return Ok(CallToolResult::error(vec![error_content]));
            }

            // Check if role has knowledge graph configuration
            let kg_is_properly_configured = role_cfg
                .kg
                .as_ref()
                .map(|kg| kg.automata_path.is_some() || kg.knowledge_graph_local.is_some())
                .unwrap_or(false);

            if !kg_is_properly_configured {
                let error_content = Content::text(format!(
                    "Role '{}' does not have a properly configured knowledge graph. Autocomplete requires a role with defined automata_path or local knowledge graph.",
                    role_name
                ));
                return Ok(CallToolResult::error(vec![error_content]));
            }
        } else {
            let error_content = Content::text(format!(
                "Role '{}' not found in configuration. Available roles: {:?}",
                role_name,
                self.config_state.roles.keys().collect::<Vec<_>>()
            ));
            return Ok(CallToolResult::error(vec![error_content]));
        }

        // Load thesaurus for the role
        match service.ensure_thesaurus_loaded(&role_name).await {
            Ok(thesaurus_data) => {
                if thesaurus_data.is_empty() {
                    let error_content = Content::text(format!(
                        "No thesaurus data available for role '{}'. Please ensure the role has a properly configured and loaded knowledge graph.",
                        role_name
                    ));
                    return Ok(CallToolResult::error(vec![error_content]));
                }

                info!(
                    "Building autocomplete index from {} thesaurus entries for role: {}",
                    thesaurus_data.len(),
                    role_name
                );

                let config = AutocompleteConfig::default();
                match terraphim_automata::build_autocomplete_index(
                    thesaurus_data.clone(),
                    Some(config),
                ) {
                    Ok(index) => {
                        // Store the index with role context
                        let mut autocomplete_lock = self.autocomplete_index.write().await;
                        *autocomplete_lock = Some(index);

                        let content = Content::text(format!(
                            "Autocomplete index built successfully with {} terms for role '{}'",
                            thesaurus_data.len(),
                            role_name
                        ));
                        Ok(CallToolResult::success(vec![content]))
                    }
                    Err(e) => {
                        error!("Failed to build autocomplete index: {}", e);
                        let error_content =
                            Content::text(format!("Failed to build autocomplete index: {}", e));
                        Ok(CallToolResult::error(vec![error_content]))
                    }
                }
            }
            Err(e) => {
                error!("Failed to load thesaurus for role '{}': {}", role_name, e);
                let error_content = Content::text(format!(
                    "Failed to load thesaurus for role '{}': {}. Please ensure the role has a valid knowledge graph configuration with accessible automata_path.",
                    role_name, e
                ));
                Ok(CallToolResult::error(vec![error_content]))
            }
        }
    }

    /// Autocomplete terms (prefix + fuzzy) returning structured results
    pub async fn autocomplete_terms(
        &self,
        query: String,
        limit: Option<usize>,
        role: Option<String>,
    ) -> Result<CallToolResult, ErrorData> {
        // Determine which role to use (provided role or selected role)
        let _role_name = if let Some(role_str) = role {
            RoleName::from(role_str)
        } else {
            self.config_state.get_selected_role().await
        };

        // Check if we need to rebuild the autocomplete index for this role
        // For now, we'll use the existing index, but in the future we could implement
        // role-specific indices or rebuild when the role changes
        let autocomplete_lock = self.autocomplete_index.read().await;
        if let Some(ref index) = *autocomplete_lock {
            let max_results = limit.unwrap_or(10);
            let mut combined: Vec<AutocompleteResult> = Vec::new();
            // Prefer exact/prefix first
            if let Ok(mut exact) =
                terraphim_automata::autocomplete_search(index, &query, Some(max_results))
            {
                combined.append(&mut exact);
            }
            // Fill remaining with fuzzy
            if combined.len() < max_results {
                let remaining = max_results - combined.len();
                if let Ok(mut fuzzy) = terraphim_automata::fuzzy_autocomplete_search(
                    index,
                    &query,
                    0.6,
                    Some(remaining),
                ) {
                    combined.append(&mut fuzzy);
                }
            }
            // Graph embeddings-style expansion: include additional synonyms sharing the same concept id
            if combined.len() < max_results {
                use std::collections::HashSet;
                let mut seen_terms: HashSet<String> =
                    combined.iter().map(|r| r.term.clone()).collect();
                let concept_ids: HashSet<u64> = combined.iter().map(|r| r.id).collect();
                let mut candidates: Vec<AutocompleteResult> = Vec::new();
                // Iterate over all metadata and pick terms with same concept id
                for (term, meta) in terraphim_automata::autocomplete_helpers::iter_metadata(index) {
                    if concept_ids.contains(&meta.id) && !seen_terms.contains(term) {
                        candidates.push(AutocompleteResult {
                            term: meta.original_term.clone(),
                            normalized_term: meta.normalized_term.clone(),
                            id: meta.id,
                            url: meta.url.clone(),
                            score: meta.id as f64,
                        });
                    }
                }
                // Prefer shorter terms, stable order
                candidates.sort_by(|a, b| {
                    a.term
                        .len()
                        .cmp(&b.term.len())
                        .then_with(|| a.term.cmp(&b.term))
                });
                for cand in candidates {
                    if combined.len() >= max_results {
                        break;
                    }
                    if seen_terms.insert(cand.term.clone()) {
                        combined.push(cand);
                    }
                }
            }
            let mut contents = Vec::new();
            contents.push(Content::text(format!(
                "Found {} suggestions",
                combined.len()
            )));
            for r in combined.into_iter().take(max_results) {
                let line = r.term.to_string();
                contents.push(Content::text(line));
            }
            return Ok(CallToolResult::success(contents));
        }
        let error_content = Content::text(
            "Autocomplete index not built. Please run 'build_autocomplete_index' first."
                .to_string(),
        );
        Ok(CallToolResult::error(vec![error_content]))
    }

    /// Autocomplete with snippets: returns term and a short snippet (stub/description) when available
    pub async fn autocomplete_with_snippets(
        &self,
        query: String,
        limit: Option<usize>,
        role: Option<String>,
    ) -> Result<CallToolResult, ErrorData> {
        // Determine which role to use (provided role or selected role)
        let _role_name = if let Some(role_str) = role {
            RoleName::from(role_str)
        } else {
            self.config_state.get_selected_role().await
        };

        // We only need the terms from automata index, snippets will be pulled from documents when possible
        // For now, we'll use the existing index, but in the future we could implement
        // role-specific indices or rebuild when the role changes
        let autocomplete_lock = self.autocomplete_index.read().await;
        if let Some(ref index) = *autocomplete_lock {
            let max_results = limit.unwrap_or(10);
            let mut results: Vec<AutocompleteResult> = Vec::new();
            if let Ok(mut prefix) =
                terraphim_automata::autocomplete_search(index, &query, Some(max_results))
            {
                results.append(&mut prefix);
            }
            if results.len() < max_results {
                let remaining = max_results - results.len();
                if let Ok(mut fuzzy) = terraphim_automata::fuzzy_autocomplete_search(
                    index,
                    &query,
                    0.6,
                    Some(remaining),
                ) {
                    results.append(&mut fuzzy);
                }
            }

            // Build snippets by searching for each term to fetch a related document and use its stub/description/body excerpt
            let mut service = self
                .terraphim_service()
                .await
                .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
            let mut contents = Vec::new();
            contents.push(Content::text(format!(
                "Found {} suggestions",
                results.len()
            )));
            for r in results.into_iter().take(max_results) {
                let sq = SearchQuery {
                    search_term: NormalizedTermValue::from(r.term.clone()),
                    search_terms: None,
                    operator: None,
                    role: None,
                    limit: Some(1),
                    skip: Some(0),
                };
                let snippet = match service.search(&sq).await {
                    Ok(documents) if !documents.is_empty() => {
                        let d = &documents[0];
                        if let Some(stub) = &d.stub {
                            stub.clone()
                        } else if let Some(desc) = &d.description {
                            desc.clone()
                        } else {
                            let body = d.body.as_str();
                            let snip = body.chars().take(160).collect::<String>();
                            snip
                        }
                    }
                    _ => String::new(),
                };
                let line = if snippet.is_empty() {
                    r.term
                } else {
                    format!("{} — {}", r.term, snippet)
                };
                contents.push(Content::text(line));
            }
            return Ok(CallToolResult::success(contents));
        }
        let error_content = Content::text(
            "Autocomplete index not built. Please run 'build_autocomplete_index' first."
                .to_string(),
        );
        Ok(CallToolResult::error(vec![error_content]))
    }

    /// Fuzzy autocomplete search using Jaro-Winkler similarity (default, faster)
    pub async fn fuzzy_autocomplete_search(
        &self,
        query: String,
        similarity: Option<f64>,
        limit: Option<usize>,
    ) -> Result<CallToolResult, ErrorData> {
        let autocomplete_lock = self.autocomplete_index.read().await;

        if let Some(ref index) = *autocomplete_lock {
            let min_similarity = similarity.unwrap_or(0.6);
            let max_results = limit.unwrap_or(10);

            match terraphim_automata::fuzzy_autocomplete_search(
                index,
                &query,
                min_similarity,
                Some(max_results),
            ) {
                Ok(results) => {
                    let mut contents = Vec::new();
                    let summary = format!(
                        "Found {} autocomplete suggestions for '{}'",
                        results.len(),
                        query
                    );
                    contents.push(Content::text(summary));

                    for result in results {
                        let suggestion = format!("• {} (score: {:.3})", result.term, result.score);
                        contents.push(Content::text(suggestion));
                    }

                    Ok(CallToolResult::success(contents))
                }
                Err(e) => {
                    error!("Autocomplete search failed: {}", e);
                    let error_content = Content::text(format!("Autocomplete search failed: {}", e));
                    Ok(CallToolResult::error(vec![error_content]))
                }
            }
        } else {
            let error_content = Content::text(
                "Autocomplete index not built. Please run 'build_autocomplete_index' first."
                    .to_string(),
            );
            Ok(CallToolResult::error(vec![error_content]))
        }
    }

    /// Fuzzy autocomplete search using Levenshtein distance (baseline comparison)
    pub async fn fuzzy_autocomplete_search_levenshtein(
        &self,
        query: String,
        max_edit_distance: Option<usize>,
        limit: Option<usize>,
    ) -> Result<CallToolResult, ErrorData> {
        let autocomplete_lock = self.autocomplete_index.read().await;

        if let Some(ref index) = *autocomplete_lock {
            let max_distance = max_edit_distance.unwrap_or(2);
            let max_results = limit.unwrap_or(10);

            match terraphim_automata::fuzzy_autocomplete_search_levenshtein(
                index,
                &query,
                max_distance,
                Some(max_results),
            ) {
                Ok(results) => {
                    let mut contents = Vec::new();
                    let summary = format!(
                        "Found {} Levenshtein autocomplete suggestions for '{}'",
                        results.len(),
                        query
                    );
                    contents.push(Content::text(summary));

                    for result in results {
                        let suggestion = format!("• {} (score: {:.3})", result.term, result.score);
                        contents.push(Content::text(suggestion));
                    }

                    Ok(CallToolResult::success(contents))
                }
                Err(e) => {
                    error!("Levenshtein autocomplete search failed: {}", e);
                    let error_content =
                        Content::text(format!("Levenshtein autocomplete search failed: {}", e));
                    Ok(CallToolResult::error(vec![error_content]))
                }
            }
        } else {
            let error_content = Content::text(
                "Autocomplete index not built. Please run 'build_autocomplete_index' first."
                    .to_string(),
            );
            Ok(CallToolResult::error(vec![error_content]))
        }
    }

    /// Fuzzy autocomplete search using Jaro-Winkler similarity (explicit)
    pub async fn fuzzy_autocomplete_search_jaro_winkler(
        &self,
        query: String,
        similarity: Option<f64>,
        limit: Option<usize>,
    ) -> Result<CallToolResult, ErrorData> {
        let autocomplete_lock = self.autocomplete_index.read().await;

        if let Some(ref index) = *autocomplete_lock {
            let min_similarity = similarity.unwrap_or(0.6);
            let max_results = limit.unwrap_or(10);

            match terraphim_automata::fuzzy_autocomplete_search(
                index,
                &query,
                min_similarity,
                Some(max_results),
            ) {
                Ok(results) => {
                    let mut contents = Vec::new();
                    let summary = format!(
                        "Found {} Jaro-Winkler autocomplete suggestions for '{}'",
                        results.len(),
                        query
                    );
                    contents.push(Content::text(summary));

                    for result in results {
                        let suggestion = format!("• {} (score: {:.3})", result.term, result.score);
                        contents.push(Content::text(suggestion));
                    }

                    Ok(CallToolResult::success(contents))
                }
                Err(e) => {
                    error!("Jaro-Winkler autocomplete search failed: {}", e);
                    let error_content =
                        Content::text(format!("Jaro-Winkler autocomplete search failed: {}", e));
                    Ok(CallToolResult::error(vec![error_content]))
                }
            }
        } else {
            let error_content = Content::text(
                "Autocomplete index not built. Please run 'build_autocomplete_index' first."
                    .to_string(),
            );
            Ok(CallToolResult::error(vec![error_content]))
        }
    }

    /// Serialize autocomplete index to bytes for storage/transmission
    pub async fn serialize_autocomplete_index(&self) -> Result<CallToolResult, ErrorData> {
        let autocomplete_lock = self.autocomplete_index.read().await;

        if let Some(ref index) = *autocomplete_lock {
            match terraphim_automata::serialize_autocomplete_index(index) {
                Ok(bytes) => {
                    let mut contents = Vec::new();
                    contents.push(Content::text(format!(
                        "Successfully serialized autocomplete index to {} bytes",
                        bytes.len()
                    )));

                    // Convert bytes to base64 for text representation
                    let base64_data = base64::engine::general_purpose::STANDARD.encode(&bytes);
                    contents.push(Content::text("Base64 encoded data:".to_string()));
                    contents.push(Content::text(base64_data));

                    Ok(CallToolResult::success(contents))
                }
                Err(e) => {
                    error!("Serialize autocomplete index failed: {}", e);
                    let error_content =
                        Content::text(format!("Serialize autocomplete index failed: {}", e));
                    Ok(CallToolResult::error(vec![error_content]))
                }
            }
        } else {
            let error_content = Content::text(
                "Autocomplete index not built. Please run 'build_autocomplete_index' first."
                    .to_string(),
            );
            Ok(CallToolResult::error(vec![error_content]))
        }
    }

    /// Deserialize autocomplete index from bytes
    pub async fn deserialize_autocomplete_index(
        &self,
        base64_data: String,
    ) -> Result<CallToolResult, ErrorData> {
        // Decode base64 data
        let bytes = match base64::engine::general_purpose::STANDARD.decode(&base64_data) {
            Ok(data) => data,
            Err(e) => {
                let error_content = Content::text(format!("Invalid base64 data: {}", e));
                return Ok(CallToolResult::error(vec![error_content]));
            }
        };

        match terraphim_automata::deserialize_autocomplete_index(&bytes) {
            Ok(index) => {
                // Store the deserialized index
                let mut autocomplete_lock = self.autocomplete_index.write().await;
                *autocomplete_lock = Some(index);

                let mut contents = Vec::new();
                contents.push(Content::text(format!(
                    "Successfully deserialized autocomplete index with {} terms",
                    autocomplete_lock.as_ref().unwrap().len()
                )));

                Ok(CallToolResult::success(contents))
            }
            Err(e) => {
                error!("Deserialize autocomplete index failed: {}", e);
                let error_content =
                    Content::text(format!("Deserialize autocomplete index failed: {}", e));
                Ok(CallToolResult::error(vec![error_content]))
            }
        }
    }

    /// Find all term matches in text using Aho-Corasick algorithm
    pub async fn find_matches(
        &self,
        text: String,
        role: Option<String>,
        return_positions: Option<bool>,
    ) -> Result<CallToolResult, ErrorData> {
        let mut service = self
            .terraphim_service()
            .await
            .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;

        // Determine which role to use (provided role or selected role)
        let role_name = if let Some(role_str) = role {
            RoleName::from(role_str)
        } else {
            self.config_state.get_selected_role().await
        };

        // Load thesaurus for the role
        match service.ensure_thesaurus_loaded(&role_name).await {
            Ok(thesaurus_data) => {
                if thesaurus_data.is_empty() {
                    let error_content = Content::text(format!(
                        "No thesaurus data available for role '{}'. Please ensure the role has a properly configured and loaded knowledge graph.",
                        role_name
                    ));
                    return Ok(CallToolResult::error(vec![error_content]));
                }

                let return_pos = return_positions.unwrap_or(false);

                match find_matches(&text, thesaurus_data, return_pos) {
                    Ok(matches) => {
                        let mut contents = Vec::new();
                        let summary = format!(
                            "Found {} term matches in text for role '{}'",
                            matches.len(),
                            role_name
                        );
                        contents.push(Content::text(summary));

                        for matched in matches.iter() {
                            let match_info = if return_pos {
                                if let Some((start, end)) = matched.pos {
                                    format!("• {} (pos: {}-{})", matched.term, start, end)
                                } else {
                                    format!("• {} (no position)", matched.term)
                                }
                            } else {
                                format!("• {}", matched.term)
                            };
                            contents.push(Content::text(match_info));
                        }

                        Ok(CallToolResult::success(contents))
                    }
                    Err(e) => {
                        error!("Find matches failed: {}", e);
                        let error_content = Content::text(format!("Find matches failed: {}", e));
                        Ok(CallToolResult::error(vec![error_content]))
                    }
                }
            }
            Err(e) => {
                error!("Failed to load thesaurus for role '{}': {}", role_name, e);
                let error_content = Content::text(format!(
                    "Failed to load thesaurus for role '{}': {}. Please ensure the role has a valid knowledge graph configuration.",
                    role_name, e
                ));
                Ok(CallToolResult::error(vec![error_content]))
            }
        }
    }

    /// Replace matched terms in text with links using specified format
    pub async fn replace_matches(
        &self,
        text: String,
        role: Option<String>,
        link_type: String,
    ) -> Result<CallToolResult, ErrorData> {
        let mut service = self
            .terraphim_service()
            .await
            .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;

        // Determine which role to use (provided role or selected role)
        let role_name = if let Some(role_str) = role {
            RoleName::from(role_str)
        } else {
            self.config_state.get_selected_role().await
        };

        // Parse link type
        let link_type_enum = match link_type.to_lowercase().as_str() {
            "wiki" | "wikilinks" => terraphim_automata::LinkType::WikiLinks,
            "html" | "htmllinks" => terraphim_automata::LinkType::HTMLLinks,
            "markdown" | "md" => terraphim_automata::LinkType::MarkdownLinks,
            _ => {
                let error_content = Content::text(format!(
                    "Invalid link type '{}'. Supported types: wiki, html, markdown",
                    link_type
                ));
                return Ok(CallToolResult::error(vec![error_content]));
            }
        };

        // Load thesaurus for the role
        match service.ensure_thesaurus_loaded(&role_name).await {
            Ok(thesaurus_data) => {
                if thesaurus_data.is_empty() {
                    let error_content = Content::text(format!(
                        "No thesaurus data available for role '{}'. Please ensure the role has a properly configured and loaded knowledge graph.",
                        role_name
                    ));
                    return Ok(CallToolResult::error(vec![error_content]));
                }

                match replace_matches(&text, thesaurus_data, link_type_enum) {
                    Ok(replaced_bytes) => {
                        let replaced_text = String::from_utf8(replaced_bytes)
                            .unwrap_or_else(|_| "Binary output (non-UTF8)".to_string());

                        let mut contents = Vec::new();
                        contents.push(Content::text(format!(
                            "Successfully replaced terms in text for role '{}' using {} format",
                            role_name, link_type
                        )));
                        contents.push(Content::text("Replaced text:".to_string()));
                        contents.push(Content::text(replaced_text));

                        Ok(CallToolResult::success(contents))
                    }
                    Err(e) => {
                        error!("Replace matches failed: {}", e);
                        let error_content = Content::text(format!("Replace matches failed: {}", e));
                        Ok(CallToolResult::error(vec![error_content]))
                    }
                }
            }
            Err(e) => {
                error!("Failed to load thesaurus for role '{}': {}", role_name, e);
                let error_content = Content::text(format!(
                    "Failed to load thesaurus for role '{}': {}. Please ensure the role has a valid knowledge graph configuration.",
                    role_name, e
                ));
                Ok(CallToolResult::error(vec![error_content]))
            }
        }
    }

    /// Extract paragraphs containing matched terms from text
    pub async fn extract_paragraphs_from_automata(
        &self,
        text: String,
        role: Option<String>,
        include_term: Option<bool>,
    ) -> Result<CallToolResult, ErrorData> {
        let mut service = self
            .terraphim_service()
            .await
            .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;

        // Determine which role to use (provided role or selected role)
        let role_name = if let Some(role_str) = role {
            RoleName::from(role_str)
        } else {
            self.config_state.get_selected_role().await
        };

        // Load thesaurus for the role
        match service.ensure_thesaurus_loaded(&role_name).await {
            Ok(thesaurus_data) => {
                if thesaurus_data.is_empty() {
                    let error_content = Content::text(format!(
                        "No thesaurus data available for role '{}'. Please ensure the role has a properly configured and loaded knowledge graph.",
                        role_name
                    ));
                    return Ok(CallToolResult::error(vec![error_content]));
                }

                let include_term_bool = include_term.unwrap_or(true);

                match extract_paragraphs_from_automata(&text, thesaurus_data, include_term_bool) {
                    Ok(paragraphs) => {
                        let mut contents = Vec::new();
                        let summary = format!(
                            "Extracted {} paragraphs containing matched terms for role '{}'",
                            paragraphs.len(),
                            role_name
                        );
                        contents.push(Content::text(summary));

                        for (idx, (matched, paragraph)) in paragraphs.iter().enumerate() {
                            let match_info = format!("Match {}: {}", idx + 1, matched.term);
                            contents.push(Content::text(match_info));
                            contents.push(Content::text(format!("Paragraph: {}", paragraph)));
                            contents.push(Content::text("---".to_string()));
                        }

                        Ok(CallToolResult::success(contents))
                    }
                    Err(e) => {
                        error!("Extract paragraphs failed: {}", e);
                        let error_content =
                            Content::text(format!("Extract paragraphs failed: {}", e));
                        Ok(CallToolResult::error(vec![error_content]))
                    }
                }
            }
            Err(e) => {
                error!("Failed to load thesaurus for role '{}': {}", role_name, e);
                let error_content = Content::text(format!(
                    "Failed to load thesaurus for role '{}': {}. Please ensure the role has a valid knowledge graph configuration.",
                    role_name, e
                ));
                Ok(CallToolResult::error(vec![error_content]))
            }
        }
    }

    /// Parse Logseq JSON output using terraphim_automata
    pub async fn json_decode(&self, jsonlines: String) -> Result<CallToolResult, ErrorData> {
        match json_decode(&jsonlines) {
            Ok(messages) => {
                let mut contents = Vec::new();
                let summary = format!("Successfully parsed {} Logseq messages", messages.len());
                contents.push(Content::text(summary));

                for (idx, message) in messages.iter().enumerate() {
                    let message_info = format!("Message {}: {:?}", idx + 1, message);
                    contents.push(Content::text(message_info));
                }

                Ok(CallToolResult::success(contents))
            }
            Err(e) => {
                error!("JSON decode failed: {}", e);
                let error_content = Content::text(format!("JSON decode failed: {}", e));
                Ok(CallToolResult::error(vec![error_content]))
            }
        }
    }

    /// Load thesaurus from automata path (local file or remote URL)
    pub async fn load_thesaurus(&self, automata_path: String) -> Result<CallToolResult, ErrorData> {
        // Parse the automata path
        let path = if automata_path.starts_with("http://") || automata_path.starts_with("https://")
        {
            terraphim_automata::AutomataPath::from_remote(&automata_path)
                .map_err(|e| ErrorData::internal_error(e.to_string(), None))?
        } else {
            terraphim_automata::AutomataPath::from_local(&automata_path)
        };

        match terraphim_automata::load_thesaurus(&path).await {
            Ok(thesaurus) => {
                let mut contents = Vec::new();
                let summary = format!(
                    "Successfully loaded thesaurus from '{}' with {} terms",
                    automata_path,
                    thesaurus.len()
                );
                contents.push(Content::text(summary));

                // Show first few terms as preview
                let preview_terms: Vec<_> = thesaurus.keys().take(10).collect();
                if !preview_terms.is_empty() {
                    contents.push(Content::text("Preview of terms:".to_string()));
                    for term in preview_terms {
                        let normalized = thesaurus.get(term).unwrap();
                        let term_info = format!("• {} -> {}", term, normalized.value);
                        contents.push(Content::text(term_info));
                    }
                    if thesaurus.len() > 10 {
                        contents.push(Content::text(format!(
                            "... and {} more terms",
                            thesaurus.len() - 10
                        )));
                    }
                }

                Ok(CallToolResult::success(contents))
            }
            Err(e) => {
                error!("Load thesaurus failed: {}", e);
                let error_content = Content::text(format!("Load thesaurus failed: {}", e));
                Ok(CallToolResult::error(vec![error_content]))
            }
        }
    }

    /// Load thesaurus from JSON string
    pub async fn load_thesaurus_from_json(
        &self,
        json_str: String,
    ) -> Result<CallToolResult, ErrorData> {
        match terraphim_automata::load_thesaurus_from_json(&json_str) {
            Ok(thesaurus) => {
                let mut contents = Vec::new();
                let summary = format!(
                    "Successfully loaded thesaurus from JSON with {} terms",
                    thesaurus.len()
                );
                contents.push(Content::text(summary));

                // Show first few terms as preview
                let preview_terms: Vec<_> = thesaurus.keys().take(10).collect();
                if !preview_terms.is_empty() {
                    contents.push(Content::text("Preview of terms:".to_string()));
                    for term in preview_terms {
                        let normalized = thesaurus.get(term).unwrap();
                        let term_info = format!("• {} -> {}", term, normalized.value);
                        contents.push(Content::text(term_info));
                    }
                    if thesaurus.len() > 10 {
                        contents.push(Content::text(format!(
                            "... and {} more terms",
                            thesaurus.len() - 10
                        )));
                    }
                }

                Ok(CallToolResult::success(contents))
            }
            Err(e) => {
                error!("Load thesaurus from JSON failed: {}", e);
                let error_content =
                    Content::text(format!("Load thesaurus from JSON failed: {}", e));
                Ok(CallToolResult::error(vec![error_content]))
            }
        }
    }

    /// Check if all matched terms in text can be connected by a single path in the knowledge graph
    pub async fn is_all_terms_connected_by_path(
        &self,
        text: String,
        role: Option<String>,
    ) -> Result<CallToolResult, ErrorData> {
        let mut service = self
            .terraphim_service()
            .await
            .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;

        // Determine which role to use (provided role or selected role)
        let role_name = if let Some(role_str) = role {
            RoleName::from(role_str)
        } else {
            self.config_state.get_selected_role().await
        };

        // Check if the role exists and has proper knowledge graph configuration
        let role_config = self.config_state.get_role(&role_name).await;
        if let Some(role_cfg) = role_config {
            // Check if role uses TerraphimGraph relevance function (required for knowledge graph)
            if role_cfg.relevance_function != terraphim_types::RelevanceFunction::TerraphimGraph {
                let error_content = Content::text(format!(
                    "Role '{}' does not use knowledge graph ranking (TerraphimGraph). Graph connectivity check is only available for roles with knowledge graph-based ranking. Current relevance function: {:?}",
                    role_name, role_cfg.relevance_function
                ));
                return Ok(CallToolResult::error(vec![error_content]));
            }

            // Check if role has knowledge graph configuration
            let kg_is_properly_configured = role_cfg
                .kg
                .as_ref()
                .map(|kg| kg.automata_path.is_some() || kg.knowledge_graph_local.is_some())
                .unwrap_or(false);

            if !kg_is_properly_configured {
                let error_content = Content::text(format!(
                    "Role '{}' does not have a properly configured knowledge graph. Graph connectivity check requires a role with defined automata_path or local knowledge graph.",
                    role_name
                ));
                return Ok(CallToolResult::error(vec![error_content]));
            }
        } else {
            let error_content = Content::text(format!(
                "Role '{}' not found in configuration. Available roles: {:?}",
                role_name,
                self.config_state.roles.keys().collect::<Vec<_>>()
            ));
            return Ok(CallToolResult::error(vec![error_content]));
        }

        // Load thesaurus for the role to find matches
        match service.ensure_thesaurus_loaded(&role_name).await {
            Ok(thesaurus_data) => {
                if thesaurus_data.is_empty() {
                    let error_content = Content::text(format!(
                        "No thesaurus data available for role '{}'. Please ensure the role has a properly configured and loaded knowledge graph.",
                        role_name
                    ));
                    return Ok(CallToolResult::error(vec![error_content]));
                }

                // Find all term matches in the text
                match terraphim_automata::find_matches(&text, thesaurus_data, false) {
                    Ok(matches) => {
                        if matches.is_empty() {
                            let content = Content::text(format!(
                                "No terms from role '{}' found in the provided text. Cannot check graph connectivity.",
                                role_name
                            ));
                            return Ok(CallToolResult::success(vec![content]));
                        }

                        // Extract matched terms
                        let matched_terms: Vec<String> =
                            matches.iter().map(|m| m.term.clone()).collect();

                        // Create a RoleGraph instance to check connectivity
                        // For now, we'll use a simple approach by checking if we can build a graph
                        // In a full implementation, you might want to load the actual graph structure
                        let mut contents = Vec::new();
                        contents.push(Content::text(format!(
                            "Found {} matched terms in text for role '{}': {:?}",
                            matched_terms.len(),
                            role_name,
                            matched_terms
                        )));

                        // Note: This is a placeholder implementation
                        // The actual RoleGraph::is_all_terms_connected_by_path would need the graph structure
                        contents.push(Content::text("Note: Graph connectivity check requires full graph structure loading. This is a preview of matched terms."));

                        Ok(CallToolResult::success(contents))
                    }
                    Err(e) => {
                        error!("Find matches failed: {}", e);
                        let error_content = Content::text(format!("Find matches failed: {}", e));
                        Ok(CallToolResult::error(vec![error_content]))
                    }
                }
            }
            Err(e) => {
                error!("Failed to load thesaurus for role '{}': {}", role_name, e);
                let error_content = Content::text(format!(
                    "Failed to load thesaurus for role '{}': {}. Please ensure the role has a valid knowledge graph configuration.",
                    role_name, e
                ));
                Ok(CallToolResult::error(vec![error_content]))
            }
        }
    }
}

impl ServerHandler for McpService {
    async fn initialize(
        &self,
        request: rmcp::model::InitializeRequestParam,
        context: RequestContext<RoleServer>,
    ) -> Result<rmcp::model::InitializeResult, ErrorData> {
        if context.peer.peer_info().is_none() {
            context.peer.set_peer_info(request);
        }
        Ok(self.get_info())
    }

    async fn list_tools(
        &self,
        _request: Option<rmcp::model::PaginatedRequestParam>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, ErrorData> {
        tracing::debug!("list_tools function called!");

        // Convert JSON values to Arc<Map<String, Value>> for input_schema
        let search_schema = serde_json::json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "The search query"
                },
                "role": {
                    "type": "string",
                    "description": "Optional role to filter by"
                },
                "limit": {
                    "type": "integer",
                    "description": "Maximum number of results to return"
                },
                "skip": {
                    "type": "integer",
                    "description": "Number of results to skip"
                }
            },
            "required": ["query"]
        });
        let search_map = search_schema.as_object().unwrap().clone();

        let config_schema = serde_json::json!({
            "type": "object",
            "properties": {
                "config_str": {
                    "type": "string",
                    "description": "JSON configuration string"
                }
            },
            "required": ["config_str"]
        });
        let config_map = config_schema.as_object().unwrap().clone();

        let build_autocomplete_schema = serde_json::json!({
            "type": "object",
            "properties": {
                "role": {
                    "type": "string",
                    "description": "Optional role name to build autocomplete index for. If not provided, uses the currently selected role."
                }
            },
            "required": []
        });
        let build_autocomplete_map = build_autocomplete_schema.as_object().unwrap().clone();

        let autocomplete_terms_schema = serde_json::json!({
            "type": "object",
            "properties": {
                "query": { "type": "string", "description": "Prefix or term for suggestions" },
                "limit": { "type": "integer", "description": "Max suggestions (default 10)" },
                "role": { "type": "string", "description": "Optional role name to use for autocomplete. If not provided, uses the currently selected role." }
            },
            "required": ["query"]
        });
        let autocomplete_terms_map = autocomplete_terms_schema.as_object().unwrap().clone();

        let autocomplete_snippets_schema = serde_json::json!({
            "type": "object",
            "properties": {
                "query": { "type": "string", "description": "Prefix or term for suggestions with snippets" },
                "limit": { "type": "integer", "description": "Max suggestions (default 10)" },
                "role": { "type": "string", "description": "Optional role name to use for autocomplete. If not provided, uses the currently selected role." }
            },
            "required": ["query"]
        });
        let autocomplete_snippets_map = autocomplete_snippets_schema.as_object().unwrap().clone();

        let fuzzy_autocomplete_schema = serde_json::json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "The text to get autocomplete suggestions for"
                },
                "similarity": {
                    "type": "number",
                    "description": "Minimum Jaro-Winkler similarity threshold (0.0-1.0, default: 0.6)"
                },
                "limit": {
                    "type": "integer",
                    "description": "Maximum number of suggestions to return (default: 10)"
                }
            },
            "required": ["query"]
        });
        let fuzzy_autocomplete_map = fuzzy_autocomplete_schema.as_object().unwrap().clone();

        let levenshtein_autocomplete_schema = serde_json::json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "The text to get autocomplete suggestions for"
                },
                "max_edit_distance": {
                    "type": "integer",
                    "description": "Maximum Levenshtein edit distance allowed (default: 2)"
                },
                "limit": {
                    "type": "integer",
                    "description": "Maximum number of suggestions to return (default: 10)"
                }
            },
            "required": ["query"]
        });
        let levenshtein_autocomplete_map =
            levenshtein_autocomplete_schema.as_object().unwrap().clone();

        let find_matches_schema = serde_json::json!({
            "type": "object",
            "properties": {
                "text": { "type": "string", "description": "The text to search in" },
                "role": { "type": "string", "description": "Optional role to filter by" },
                "return_positions": { "type": "boolean", "description": "Whether to return positions (default: false)" }
            },
            "required": ["text"]
        });
        let find_matches_map = find_matches_schema.as_object().unwrap().clone();

        let replace_matches_schema = serde_json::json!({
            "type": "object",
            "properties": {
                "text": { "type": "string", "description": "The text to replace terms in" },
                "role": { "type": "string", "description": "Optional role to filter by" },
                "link_type": { "type": "string", "description": "The type of link to use (wiki, html, markdown)" }
            },
            "required": ["text", "link_type"]
        });
        let replace_matches_map = replace_matches_schema.as_object().unwrap().clone();

        let extract_paragraphs_schema = serde_json::json!({
            "type": "object",
            "properties": {
                "text": { "type": "string", "description": "The text to extract paragraphs from" },
                "role": { "type": "string", "description": "Optional role to filter by" },
                "include_term": { "type": "boolean", "description": "Whether to include the matched term in the paragraph (default: true)" }
            },
            "required": ["text"]
        });
        let extract_paragraphs_map = extract_paragraphs_schema.as_object().unwrap().clone();

        let json_decode_schema = serde_json::json!({
            "type": "object",
            "properties": {
                "jsonlines": { "type": "string", "description": "The JSON lines string to decode" }
            },
            "required": ["jsonlines"]
        });
        let json_decode_map = json_decode_schema.as_object().unwrap().clone();

        let load_thesaurus_schema = serde_json::json!({
            "type": "object",
            "properties": {
                "automata_path": { "type": "string", "description": "The path to the automata file (local or remote URL)" }
            },
            "required": ["automata_path"]
        });
        let load_thesaurus_map = load_thesaurus_schema.as_object().unwrap().clone();

        let load_thesaurus_json_schema = serde_json::json!({
            "type": "object",
            "properties": {
                "json_str": { "type": "string", "description": "The JSON string to load thesaurus from" }
            },
            "required": ["json_str"]
        });
        let load_thesaurus_json_map = load_thesaurus_json_schema.as_object().unwrap().clone();

        let is_all_terms_connected_schema = serde_json::json!({
            "type": "object",
            "properties": {
                "text": { "type": "string", "description": "The text to check for term connectivity" },
                "role": { "type": "string", "description": "Optional role to use for thesaurus and graph" }
            },
            "required": ["text"]
        });
        let is_all_terms_connected_map = is_all_terms_connected_schema.as_object().unwrap().clone();

        let tools = vec![
            Tool {
                name: "search".into(),
                title: Some("Search Knowledge Graph".into()),
                description: Some("Search for documents in Terraphim knowledge graph".into()),
                input_schema: Arc::new(search_map),
                output_schema: None,
                annotations: None,
                icons: None,
                meta: None,
            },
            Tool {
                name: "update_config_tool".into(),
                title: Some("Update Configuration".into()),
                description: Some("Update the Terraphim configuration".into()),
                input_schema: Arc::new(config_map),
                output_schema: None,
                annotations: None,
                icons: None,
                meta: None,
            },
            Tool {
                name: "build_autocomplete_index".into(),
                title: Some("Build Autocomplete Index".into()),
                description: Some("Build FST-based autocomplete index from role's knowledge graph. Only available for roles with TerraphimGraph relevance function and configured knowledge graph.".into()),
                input_schema: Arc::new(build_autocomplete_map),
                output_schema: None,
                annotations: None,
                icons: None,
                meta: None,
            },
            Tool {
                name: "fuzzy_autocomplete_search".into(),
                title: Some("Fuzzy Autocomplete Search".into()),
                description: Some("Perform fuzzy autocomplete search using Jaro-Winkler similarity (default, faster and higher quality)".into()),
                input_schema: Arc::new(fuzzy_autocomplete_map),
                output_schema: None,
                annotations: None,
                icons: None,
                meta: None,
            },
            Tool {
                name: "autocomplete_terms".into(),
                title: Some("Autocomplete Terms".into()),
                description: Some("Autocomplete terms using FST prefix + fuzzy fallback".into()),
                input_schema: Arc::new(autocomplete_terms_map),
                output_schema: None,
                annotations: None,
                icons: None,
                meta: None,
            },
            Tool {
                name: "autocomplete_with_snippets".into(),
                title: Some("Autocomplete With Snippets".into()),
                description: Some("Autocomplete and return short snippets from matching documents".into()),
                input_schema: Arc::new(autocomplete_snippets_map),
                output_schema: None,
                annotations: None,
                icons: None,
                meta: None,
            },
            Tool {
                name: "fuzzy_autocomplete_search_levenshtein".into(),
                title: Some("Fuzzy Search (Levenshtein)".into()),
                description: Some("Perform fuzzy autocomplete search using Levenshtein distance (baseline comparison algorithm)".into()),
                input_schema: Arc::new(levenshtein_autocomplete_map),
                output_schema: None,
                annotations: None,
                icons: None,
                meta: None,
            },
            Tool {
                name: "fuzzy_autocomplete_search_jaro_winkler".into(),
                title: Some("Fuzzy Search (Jaro-Winkler)".into()),
                description: Some("Perform fuzzy autocomplete search using Jaro-Winkler similarity (explicit)".into()),
                input_schema: Arc::new(fuzzy_autocomplete_schema.as_object().unwrap().clone()),
                output_schema: None,
                annotations: None,
                icons: None,
                meta: None,
            },
            Tool {
                name: "serialize_autocomplete_index".into(),
                title: Some("Serialize Index".into()),
                description: Some("Serialize the current autocomplete index to a base64-encoded string for storage/transmission".into()),
                input_schema: Arc::new(serde_json::json!({
                    "type": "object",
                    "properties": {},
                    "required": []
                }).as_object().unwrap().clone()),
                output_schema: None,
                annotations: None,
                icons: None,
                meta: None,
            },
            Tool {
                name: "deserialize_autocomplete_index".into(),
                title: Some("Deserialize Index".into()),
                description: Some("Deserialize an autocomplete index from a base64-encoded string".into()),
                input_schema: Arc::new(serde_json::json!({
                    "type": "object",
                    "properties": {
                        "base64_data": { "type": "string", "description": "The base64-encoded string of the serialized index" }
                    },
                    "required": ["base64_data"]
                }).as_object().unwrap().clone()),
                output_schema: None,
                annotations: None,
                icons: None,
                meta: None,
            },
            Tool {
                name: "find_matches".into(),
                title: Some("Find Matches".into()),
                description: Some("Find all term matches in text using Aho-Corasick algorithm".into()),
                input_schema: Arc::new(find_matches_map),
                output_schema: None,
                annotations: None,
                icons: None,
                meta: None,
            },
            Tool {
                name: "replace_matches".into(),
                title: Some("Replace Matches".into()),
                description: Some("Replace matched terms in text with links using specified format".into()),
                input_schema: Arc::new(replace_matches_map),
                output_schema: None,
                annotations: None,
                icons: None,
                meta: None,
            },
            Tool {
                name: "extract_paragraphs_from_automata".into(),
                title: Some("Extract Paragraphs".into()),
                description: Some("Extract paragraphs containing matched terms from text".into()),
                input_schema: Arc::new(extract_paragraphs_map),
                output_schema: None,
                annotations: None,
                icons: None,
                meta: None,
            },
            Tool {
                name: "json_decode".into(),
                title: Some("JSON Decode".into()),
                description: Some("Parse Logseq JSON output using terraphim_automata".into()),
                input_schema: Arc::new(json_decode_map),
                output_schema: None,
                annotations: None,
                icons: None,
                meta: None,
            },
            Tool {
                name: "load_thesaurus".into(),
                title: Some("Load Thesaurus".into()),
                description: Some("Load thesaurus from a local file or remote URL".into()),
                input_schema: Arc::new(load_thesaurus_map.clone()),
                output_schema: None,
                annotations: None,
                icons: None,
                meta: None,
            },
            Tool {
                name: "load_thesaurus_from_json".into(),
                title: Some("Load Thesaurus from JSON".into()),
                description: Some("Load thesaurus from a JSON string".into()),
                input_schema: Arc::new(load_thesaurus_json_map),
                output_schema: None,
                annotations: None,
                icons: None,
                meta: None,
            },
            Tool {
                name: "is_all_terms_connected_by_path".into(),
                title: Some("Check Terms Connectivity".into()),
                description: Some("Check if all matched terms in text can be connected by a single path in the knowledge graph".into()),
                input_schema: Arc::new(is_all_terms_connected_map),
                output_schema: None,
                annotations: None,
                icons: None,
                meta: None,
            }
        ];

        tracing::debug!("Created {} tools", tools.len());
        tracing::debug!("First tool name: {:?}", tools.first().map(|t| &t.name));

        let result = ListToolsResult {
            tools,
            next_cursor: None,
        };

        tracing::debug!(
            "Returning ListToolsResult with {} tools",
            result.tools.len()
        );

        Ok(result)
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParam,
        _context: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, ErrorData> {
        match request.name.as_ref() {
            "search" => {
                let arguments = request.arguments.unwrap_or_default();
                let query = arguments
                    .get("query")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| {
                        ErrorData::invalid_params("Missing 'query' parameter".to_string(), None)
                    })?
                    .to_string();

                let role = arguments
                    .get("role")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());

                let limit = arguments
                    .get("limit")
                    .and_then(|v| v.as_i64())
                    .map(|i| i as i32);

                let skip = arguments
                    .get("skip")
                    .and_then(|v| v.as_i64())
                    .map(|i| i as i32);

                self.search(query, role, limit, skip)
                    .await
                    .map_err(TerraphimMcpError::Mcp)
                    .map_err(ErrorData::from)
            }
            "update_config_tool" => {
                let arguments = request.arguments.unwrap_or_default();
                let config_str = arguments
                    .get("config_str")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| {
                        ErrorData::invalid_params(
                            "Missing 'config_str' parameter".to_string(),
                            None,
                        )
                    })?
                    .to_string();

                self.update_config_tool(config_str)
                    .await
                    .map_err(TerraphimMcpError::Mcp)
                    .map_err(ErrorData::from)
            }
            "build_autocomplete_index" => {
                let arguments = request.arguments.unwrap_or_default();
                let role = arguments
                    .get("role")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());

                self.build_autocomplete_index(role)
                    .await
                    .map_err(TerraphimMcpError::Mcp)
                    .map_err(ErrorData::from)
            }
            "fuzzy_autocomplete_search" => {
                let arguments = request.arguments.unwrap_or_default();
                let query = arguments
                    .get("query")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| {
                        ErrorData::invalid_params("Missing 'query' parameter".to_string(), None)
                    })?
                    .to_string();

                let similarity = arguments.get("similarity").and_then(|v| v.as_f64());

                let limit = arguments
                    .get("limit")
                    .and_then(|v| v.as_i64())
                    .map(|i| i as usize);

                self.fuzzy_autocomplete_search(query, similarity, limit)
                    .await
                    .map_err(TerraphimMcpError::Mcp)
                    .map_err(ErrorData::from)
            }
            "autocomplete_terms" => {
                let arguments = request.arguments.unwrap_or_default();
                let query = arguments
                    .get("query")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| {
                        ErrorData::invalid_params("Missing 'query' parameter".to_string(), None)
                    })?
                    .to_string();
                let limit = arguments
                    .get("limit")
                    .and_then(|v| v.as_i64())
                    .map(|i| i as usize);
                let role = arguments
                    .get("role")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                self.autocomplete_terms(query, limit, role)
                    .await
                    .map_err(TerraphimMcpError::Mcp)
                    .map_err(ErrorData::from)
            }
            "autocomplete_with_snippets" => {
                let arguments = request.arguments.unwrap_or_default();
                let query = arguments
                    .get("query")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| {
                        ErrorData::invalid_params("Missing 'query' parameter".to_string(), None)
                    })?
                    .to_string();
                let limit = arguments
                    .get("limit")
                    .and_then(|v| v.as_i64())
                    .map(|i| i as usize);
                let role = arguments
                    .get("role")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                self.autocomplete_with_snippets(query, limit, role)
                    .await
                    .map_err(TerraphimMcpError::Mcp)
                    .map_err(ErrorData::from)
            }
            "fuzzy_autocomplete_search_levenshtein" => {
                let arguments = request.arguments.unwrap_or_default();
                let query = arguments
                    .get("query")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| {
                        ErrorData::invalid_params("Missing 'query' parameter".to_string(), None)
                    })?
                    .to_string();

                let max_edit_distance = arguments
                    .get("max_edit_distance")
                    .and_then(|v| v.as_i64())
                    .map(|i| i as usize);

                let limit = arguments
                    .get("limit")
                    .and_then(|v| v.as_i64())
                    .map(|i| i as usize);

                self.fuzzy_autocomplete_search_levenshtein(query, max_edit_distance, limit)
                    .await
                    .map_err(TerraphimMcpError::Mcp)
                    .map_err(ErrorData::from)
            }
            "fuzzy_autocomplete_search_jaro_winkler" => {
                let arguments = request.arguments.unwrap_or_default();
                let query = arguments
                    .get("query")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| {
                        ErrorData::invalid_params("Missing 'query' parameter".to_string(), None)
                    })?
                    .to_string();

                let similarity = arguments.get("similarity").and_then(|v| v.as_f64());

                let limit = arguments
                    .get("limit")
                    .and_then(|v| v.as_i64())
                    .map(|i| i as usize);

                self.fuzzy_autocomplete_search_jaro_winkler(query, similarity, limit)
                    .await
                    .map_err(TerraphimMcpError::Mcp)
                    .map_err(ErrorData::from)
            }
            "serialize_autocomplete_index" => self
                .serialize_autocomplete_index()
                .await
                .map_err(TerraphimMcpError::Mcp)
                .map_err(ErrorData::from),
            "deserialize_autocomplete_index" => {
                let arguments = request.arguments.unwrap_or_default();
                let base64_data = arguments
                    .get("base64_data")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| {
                        ErrorData::invalid_params(
                            "Missing 'base64_data' parameter".to_string(),
                            None,
                        )
                    })?
                    .to_string();

                self.deserialize_autocomplete_index(base64_data)
                    .await
                    .map_err(TerraphimMcpError::Mcp)
                    .map_err(ErrorData::from)
            }
            "find_matches" => {
                let arguments = request.arguments.unwrap_or_default();
                let text = arguments
                    .get("text")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| {
                        ErrorData::invalid_params("Missing 'text' parameter".to_string(), None)
                    })?
                    .to_string();
                let role = arguments
                    .get("role")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                let return_positions = arguments
                    .get("return_positions")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);

                self.find_matches(text, role, Some(return_positions))
                    .await
                    .map_err(TerraphimMcpError::Mcp)
                    .map_err(ErrorData::from)
            }
            "replace_matches" => {
                let arguments = request.arguments.unwrap_or_default();
                let text = arguments
                    .get("text")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| {
                        ErrorData::invalid_params("Missing 'text' parameter".to_string(), None)
                    })?
                    .to_string();
                let role = arguments
                    .get("role")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                let link_type = arguments
                    .get("link_type")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| {
                        ErrorData::invalid_params("Missing 'link_type' parameter".to_string(), None)
                    })?
                    .to_string();

                self.replace_matches(text, role, link_type)
                    .await
                    .map_err(TerraphimMcpError::Mcp)
                    .map_err(ErrorData::from)
            }
            "extract_paragraphs_from_automata" => {
                let arguments = request.arguments.unwrap_or_default();
                let text = arguments
                    .get("text")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| {
                        ErrorData::invalid_params("Missing 'text' parameter".to_string(), None)
                    })?
                    .to_string();
                let role = arguments
                    .get("role")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                let include_term = arguments
                    .get("include_term")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(true);

                self.extract_paragraphs_from_automata(text, role, Some(include_term))
                    .await
                    .map_err(TerraphimMcpError::Mcp)
                    .map_err(ErrorData::from)
            }
            "json_decode" => {
                let arguments = request.arguments.unwrap_or_default();
                let jsonlines = arguments
                    .get("jsonlines")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| {
                        ErrorData::invalid_params("Missing 'jsonlines' parameter".to_string(), None)
                    })?
                    .to_string();

                self.json_decode(jsonlines)
                    .await
                    .map_err(TerraphimMcpError::Mcp)
                    .map_err(ErrorData::from)
            }
            "load_thesaurus" => {
                let arguments = request.arguments.unwrap_or_default();
                let automata_path = arguments
                    .get("automata_path")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| {
                        ErrorData::invalid_params(
                            "Missing 'automata_path' parameter".to_string(),
                            None,
                        )
                    })?
                    .to_string();

                self.load_thesaurus(automata_path)
                    .await
                    .map_err(TerraphimMcpError::Mcp)
                    .map_err(ErrorData::from)
            }
            "load_thesaurus_from_json" => {
                let arguments = request.arguments.unwrap_or_default();
                let json_str = arguments
                    .get("json_str")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| {
                        ErrorData::invalid_params("Missing 'json_str' parameter".to_string(), None)
                    })?
                    .to_string();

                self.load_thesaurus_from_json(json_str)
                    .await
                    .map_err(TerraphimMcpError::Mcp)
                    .map_err(ErrorData::from)
            }
            "is_all_terms_connected_by_path" => {
                let arguments = request.arguments.unwrap_or_default();
                let text = arguments
                    .get("text")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| {
                        ErrorData::invalid_params("Missing 'text' parameter".to_string(), None)
                    })?
                    .to_string();
                let role = arguments
                    .get("role")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());

                self.is_all_terms_connected_by_path(text, role)
                    .await
                    .map_err(TerraphimMcpError::Mcp)
                    .map_err(ErrorData::from)
            }
            _ => Err(ErrorData::method_not_found::<
                rmcp::model::CallToolRequestMethod,
            >()),
        }
    }

    async fn list_resources(
        &self,
        _request: Option<rmcp::model::PaginatedRequestParam>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListResourcesResult, ErrorData> {
        let mut service = self
            .terraphim_service()
            .await
            .map_err(TerraphimMcpError::Anyhow)?;

        // Use a broad search term to find documents instead of empty search
        // We'll try common terms that should match documents in our KG
        let search_terms = vec!["terraphim", "graph", "service", "haystack"];
        let mut all_documents = std::collections::HashSet::new();

        // Perform multiple searches to gather available documents
        for term in search_terms {
            let search_query = terraphim_types::SearchQuery {
                search_term: terraphim_types::NormalizedTermValue::new(term.to_string()),
                search_terms: None,
                operator: None,
                limit: Some(50), // Reasonable limit per search
                skip: None,
                role: None,
            };

            match service.search(&search_query).await {
                Ok(documents) => {
                    for doc in documents {
                        all_documents.insert(doc.id.clone());
                    }
                }
                Err(_) => {
                    // Continue with other terms if one fails
                    continue;
                }
            }
        }

        // If we still have no documents, try a final broad search
        if all_documents.is_empty() {
            let fallback_query = terraphim_types::SearchQuery {
                search_term: terraphim_types::NormalizedTermValue::new("*".to_string()),
                search_terms: None,
                operator: None,
                limit: Some(100),
                skip: None,
                role: None,
            };

            let documents = service
                .search(&fallback_query)
                .await
                .map_err(TerraphimMcpError::Service)?;

            let resources = self
                .resource_mapper
                .documents_to_resources(&documents)
                .map_err(TerraphimMcpError::Anyhow)?;

            return Ok(ListResourcesResult {
                resources,
                next_cursor: None,
            });
        }

        // Convert unique document IDs back to documents for resource mapping
        // For now, we'll do individual searches to get full document objects
        let mut final_documents = Vec::new();
        for doc_id in all_documents.iter().take(50) {
            // Limit to 50 resources
            if let Ok(Some(doc)) = service.get_document_by_id(doc_id).await {
                final_documents.push(doc);
            }
        }

        let resources = self
            .resource_mapper
            .documents_to_resources(&final_documents)
            .map_err(TerraphimMcpError::Anyhow)?;

        Ok(ListResourcesResult {
            resources,
            next_cursor: None,
        })
    }

    async fn read_resource(
        &self,
        request: ReadResourceRequestParam,
        _context: RequestContext<RoleServer>,
    ) -> Result<ReadResourceResult, ErrorData> {
        let doc_id = self
            .resource_mapper
            .uri_to_id(&request.uri)
            .map_err(TerraphimMcpError::Anyhow)?;
        let mut service = self
            .terraphim_service()
            .await
            .map_err(TerraphimMcpError::Anyhow)?;
        let document = service
            .get_document_by_id(&doc_id)
            .await
            .map_err(TerraphimMcpError::Service)?;
        if let Some(doc) = document {
            let contents = self
                .resource_mapper
                .document_to_resource_contents(&doc)
                .map_err(TerraphimMcpError::Anyhow)?;
            Ok(ReadResourceResult {
                contents: vec![contents],
            })
        } else {
            Err(ErrorData::resource_not_found(
                format!("Document not found: {}", doc_id),
                None,
            ))
        }
    }

    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            server_info: rmcp::model::Implementation {
                name: "terraphim-mcp".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
                title: Some("Terraphim MCP Server".to_string()),
                icons: None,
                website_url: None,
            },
            instructions: Some("This server provides Terraphim knowledge graph search capabilities through the Model Context Protocol. You can search for documents using the search tool and access resources that represent Terraphim documents.".to_string()),
            ..Default::default()
        }
    }
}
