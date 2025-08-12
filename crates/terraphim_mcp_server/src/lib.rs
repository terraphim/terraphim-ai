use std::sync::Arc;

use anyhow::Result;
use rmcp::{
    model::{
        CallToolRequestParam, CallToolResult, Content, ErrorData, ListResourcesResult,
        ListToolsResult, ReadResourceRequestParam, ReadResourceResult, ServerInfo, Tool,
    },
    service::RequestContext,
    Error as McpError, RoleServer, ServerHandler,
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
    Mcp(#[from] McpError),
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
    ) -> Result<CallToolResult, McpError> {
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
            limit: limit.map(|l| l as usize),
            skip: skip.map(|s| s as usize),
            role: Some(role_name),
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
    pub async fn update_config_tool(&self, config_str: String) -> Result<CallToolResult, McpError> {
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
    ) -> Result<CallToolResult, McpError> {
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
    ) -> Result<CallToolResult, McpError> {
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
                let line = format!("{}", r.term);
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
    ) -> Result<CallToolResult, McpError> {
        // We only need the terms from automata index, snippets will be pulled from documents when possible
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
                    skip: Some(0),
                    limit: Some(1),
                    role: None,
                };
                let snippet = match service.search(&sq).await {
                    Ok(docs) if !docs.is_empty() => {
                        let d = &docs[0];
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
    ) -> Result<CallToolResult, McpError> {
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
    ) -> Result<CallToolResult, McpError> {
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
}

impl ServerHandler for McpService {
    fn list_tools(
        &self,
        _request: Option<rmcp::model::PaginatedRequestParam>,
        _context: RequestContext<RoleServer>,
    ) -> impl std::future::Future<Output = Result<ListToolsResult, ErrorData>> + Send + '_ {
        async move {
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
                    "limit": { "type": "integer", "description": "Max suggestions (default 10)" }
                },
                "required": ["query"]
            });
            let autocomplete_terms_map = autocomplete_terms_schema.as_object().unwrap().clone();

            let autocomplete_snippets_schema = serde_json::json!({
                "type": "object",
                "properties": {
                    "query": { "type": "string", "description": "Prefix or term for suggestions with snippets" },
                    "limit": { "type": "integer", "description": "Max suggestions (default 10)" }
                },
                "required": ["query"]
            });
            let autocomplete_snippets_map =
                autocomplete_snippets_schema.as_object().unwrap().clone();

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

            let tools = vec![
                Tool {
                    name: "search".into(),
                    description: Some("Search for documents in the Terraphim knowledge graph".into()),
                    input_schema: Arc::new(search_map),
                    annotations: None,
                },
                Tool {
                    name: "update_config_tool".into(),
                    description: Some("Update the Terraphim configuration".into()),
                    input_schema: Arc::new(config_map),
                    annotations: None,
                },
                Tool {
                    name: "build_autocomplete_index".into(),
                    description: Some("Build FST-based autocomplete index from role's knowledge graph. Only available for roles with TerraphimGraph relevance function and configured knowledge graph.".into()),
                    input_schema: Arc::new(build_autocomplete_map),
                    annotations: None,
                },
                Tool {
                    name: "fuzzy_autocomplete_search".into(),
                    description: Some("Perform fuzzy autocomplete search using Jaro-Winkler similarity (default, faster and higher quality)".into()),
                    input_schema: Arc::new(fuzzy_autocomplete_map),
                    annotations: None,
                },
                Tool {
                    name: "autocomplete_terms".into(),
                    description: Some("Autocomplete terms using FST prefix + fuzzy fallback".into()),
                    input_schema: Arc::new(autocomplete_terms_map),
                    annotations: None,
                },
                Tool {
                    name: "autocomplete_with_snippets".into(),
                    description: Some("Autocomplete and return short snippets from matching documents".into()),
                    input_schema: Arc::new(autocomplete_snippets_map),
                    annotations: None,
                },
                Tool {
                    name: "fuzzy_autocomplete_search_levenshtein".into(),
                    description: Some("Perform fuzzy autocomplete search using Levenshtein distance (baseline comparison algorithm)".into()),
                    input_schema: Arc::new(levenshtein_autocomplete_map),
                    annotations: None,
                }
            ];
            Ok(ListToolsResult {
                tools,
                next_cursor: None,
            })
        }
    }

    fn call_tool(
        &self,
        request: CallToolRequestParam,
        _context: RequestContext<RoleServer>,
    ) -> impl std::future::Future<Output = Result<CallToolResult, ErrorData>> + Send + '_ {
        async move {
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
                    self.autocomplete_terms(query, limit)
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
                    self.autocomplete_with_snippets(query, limit)
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
                _ => Err(ErrorData::method_not_found::<
                    rmcp::model::CallToolRequestMethod,
                >()),
            }
        }
    }

    fn list_resources(
        &self,
        _request: Option<rmcp::model::PaginatedRequestParam>,
        _context: RequestContext<RoleServer>,
    ) -> impl std::future::Future<Output = Result<ListResourcesResult, ErrorData>> + Send + '_ {
        async move {
            let mut service = self
                .terraphim_service()
                .await
                .map_err(|e| TerraphimMcpError::Anyhow(e))?;

            // Use a broad search term to find documents instead of empty search
            // We'll try common terms that should match documents in our KG
            let search_terms = vec!["terraphim", "graph", "service", "haystack"];
            let mut all_documents = std::collections::HashSet::new();

            // Perform multiple searches to gather available documents
            for term in search_terms {
                let search_query = terraphim_types::SearchQuery {
                    search_term: terraphim_types::NormalizedTermValue::new(term.to_string()),
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
    }

    fn read_resource(
        &self,
        request: ReadResourceRequestParam,
        _context: RequestContext<RoleServer>,
    ) -> impl std::future::Future<Output = Result<ReadResourceResult, ErrorData>> + Send + '_ {
        async move {
            let doc_id = self
                .resource_mapper
                .uri_to_id(&request.uri)
                .map_err(TerraphimMcpError::Anyhow)?;
            let mut service = self
                .terraphim_service()
                .await
                .map_err(|e| TerraphimMcpError::Anyhow(e))?;
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
    }

    fn get_info(&self) -> ServerInfo {
        let server_info = ServerInfo {
            server_info: rmcp::model::Implementation {
                name: "terraphim-mcp".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
            },
            instructions: Some("This server provides Terraphim knowledge graph search capabilities through the Model Context Protocol. You can search for documents using the search tool and access resources that represent Terraphim documents.".to_string()),
            ..Default::default()
        };
        server_info
    }
}
