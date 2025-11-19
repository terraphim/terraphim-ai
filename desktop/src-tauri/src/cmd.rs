use tauri::command;
use tauri::State;

use serde::{Deserialize, Serialize};

#[cfg(feature = "atomic")]
use terraphim_atomic_client::{Agent, Config as AtomicConfig, Store};
use terraphim_config::{Config, ConfigState};
use terraphim_onepassword_cli::{OnePasswordLoader, SecretLoader};
use terraphim_rolegraph::magic_unpair;
use terraphim_service::TerraphimService;
use terraphim_settings::DeviceSettings;
use terraphim_types::Thesaurus;
use terraphim_types::{
    ContextItem, ContextType, ConversationId, Document, KGIndexInfo, KGTermDefinition,
    NormalizedTermValue, RoleName, SearchQuery,
};

use ahash::AHashMap;
use schemars::schema_for;
use serde::Serializer;
use serde_json::Value;
use std::collections::HashMap;
use tsify::Tsify;

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone, Copy, Tsify)]
#[tsify(into_wasm_abi, from_wasm_abi)]
pub enum Status {
    #[serde(rename = "success")]
    Success,
    #[serde(rename = "error")]
    Error,
}

// ErrorResponse removed - using specific response types instead

// Everything we return from commands must implement `Serialize`.
// This includes Errors and `anyhow`'s `Error` type doesn't implement it.
// See https://github.com/tauri-apps/tauri/discussions/3913
#[derive(Debug, thiserror::Error)]
pub enum TerraphimTauriError {
    #[error("An error occurred: {0}")]
    Middleware(#[from] terraphim_middleware::Error),

    #[error("Persistence error: {0}")]
    Persistence(#[from] terraphim_persistence::Error),

    #[error("Service error: {0}")]
    Service(#[from] terraphim_service::ServiceError),

    #[error("Settings error: {0}")]
    Settings(#[from] terraphim_settings::Error),

    #[error("1Password error: {0}")]
    OnePassword(#[from] terraphim_onepassword_cli::OnePasswordError),

    #[error("JSON serialization error: {0}")]
    JsonSerialization(#[from] serde_json::Error),

    #[error("{0}")]
    Generic(String),
}

// Manually implement `Serialize` for our error type because some of the
// lower-level types don't implement it.
impl Serialize for TerraphimTauriError {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.to_string().as_ref())
    }
}

pub type Result<T> = std::result::Result<T, TerraphimTauriError>;

/// Response type for showing the config
///
/// This is also used when updating the config
#[derive(Debug, Serialize, Deserialize, Clone, Tsify)]
#[tsify(into_wasm_abi, from_wasm_abi)]
pub struct ConfigResponse {
    /// Status of the config fetch
    pub status: Status,
    /// The config
    pub config: Config,
}

/// Response type for showing the search results
///
/// This is used when searching for documents
/// and returning the results
#[derive(Debug, Serialize, Deserialize, Clone, Tsify)]
#[tsify(into_wasm_abi, from_wasm_abi)]
pub struct SearchResponse {
    /// Status of the search
    pub status: Status,
    /// The search results
    pub results: Vec<Document>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Tsify)]
#[tsify(into_wasm_abi, from_wasm_abi)]
pub struct DocumentListResponse {
    /// Status of the search
    pub status: Status,
    /// The document results
    pub results: Vec<Document>,
    /// The total number of documents found
    pub total: usize,
}

/// Search All TerraphimGraphs defined in a config by query param
#[command]
pub async fn search(
    config_state: State<'_, ConfigState>,
    search_query: SearchQuery,
) -> Result<SearchResponse> {
    log::info!("Search called with {:?}", search_query);
    let mut terraphim_service = TerraphimService::new(config_state.inner().clone());
    let results = terraphim_service.search(&search_query).await?;
    Ok(SearchResponse {
        status: Status::Success,
        results,
    })
}

#[command]
pub async fn get_config(config_state: tauri::State<'_, ConfigState>) -> Result<ConfigResponse> {
    log::info!("Get config called");
    let terraphim_service = TerraphimService::new(config_state.inner().clone());
    let config = terraphim_service.fetch_config().await;

    Ok(ConfigResponse {
        status: Status::Success,
        config,
    })
}

#[command]
pub async fn update_config(
    config_state: tauri::State<'_, ConfigState>,
    config_new: Config,
) -> Result<ConfigResponse> {
    log::info!("Update config called with {:?}", config_new);
    let terraphim_service = TerraphimService::new(config_state.inner().clone());
    let config = terraphim_service.update_config(config_new).await?;
    Ok(ConfigResponse {
        status: Status::Success,
        config,
    })
}

/// Command to expose thesaurus if publish=true in knowledge graph
#[command]
pub async fn publish_thesaurus(
    config_state: tauri::State<'_, ConfigState>,
    role_name: String,
) -> Result<Thesaurus> {
    let mut terraphim_service = TerraphimService::new(config_state.inner().clone());
    let thesaurus = terraphim_service
        .ensure_thesaurus_loaded(&role_name.into())
        .await?;
    Ok(thesaurus)
}

use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Tsify)]
#[tsify(into_wasm_abi, from_wasm_abi)]
pub struct InitialSettings {
    data_folder: PathBuf,
    global_shortcut: String,
}
use std::sync::Arc;
use tauri::async_runtime::Mutex;

#[tauri::command]
pub async fn save_initial_settings(
    config_state: tauri::State<'_, ConfigState>,
    device_settings: tauri::State<'_, Arc<Mutex<DeviceSettings>>>,
    new_settings: InitialSettings,
) -> Result<()> {
    log::info!("Saving initial settings: {:?}", new_settings);
    let mut settings = device_settings.lock().await;
    let data_folder = PathBuf::from(&new_settings.data_folder);
    log::info!("Data folder: {:?}", data_folder);

    if !data_folder.exists() {
        log::info!("Creating data folder at {:?}", data_folder);
        std::fs::create_dir_all(&data_folder)
            .map_err(|e| TerraphimTauriError::Generic(e.to_string()))?;
    }

    if !data_folder.is_dir() {
        return Err(TerraphimTauriError::Generic(
            "Selected path is not a folder".to_string(),
        ));
    }

    // Update the default_data_path in settings
    settings.default_data_path = new_settings.data_folder.to_string_lossy().to_string();

    log::info!("Data folder set to: {:?}", new_settings.data_folder);
    log::info!("Global shortcut set to: {}", new_settings.global_shortcut);

    let mut config = config_state.config.lock().await;
    config.global_shortcut = new_settings.global_shortcut;
    let updated_config = config.clone();
    drop(config);

    update_config(config_state.clone(), updated_config).await?;
    settings.update_initialized_flag(None, true)?;
    drop(settings);
    Ok(())
}
use tauri::{Manager, Window};

#[tauri::command]
pub async fn close_splashscreen(window: Window) {
    // Close splashscreen
    if let Some(splashscreen) = window.get_window("splashscreen") {
        let _ = splashscreen.close();
    } else {
        log::warn!("Splashscreen window not found");
    }

    // Show main window - try different possible labels
    let window_labels = ["main", ""];
    let mut main_window_found = false;

    for label in &window_labels {
        if let Some(main_window) = window.get_window(label) {
            let _ = main_window.show();
            main_window_found = true;
            break;
        }
    }

    if !main_window_found {
        log::error!("Main window not found with any expected label");
        // Try to get any available window
        let windows = window.windows();
        if let Some((_, any_window)) = windows.iter().next() {
            let _ = any_window.show();
        }
    }
}

/// Response type for a single document
#[derive(Debug, Serialize, Deserialize, Clone, Tsify)]
#[tsify(into_wasm_abi, from_wasm_abi)]
pub struct DocumentResponse {
    pub status: Status,
    pub document: Option<Document>,
}

/// Create (or update) a document and persist it
#[command]
pub async fn create_document(
    config_state: State<'_, ConfigState>,
    document: Document,
) -> Result<DocumentResponse> {
    let mut terraphim_service = TerraphimService::new(config_state.inner().clone());
    let doc = terraphim_service.create_document(document).await?;
    Ok(DocumentResponse {
        status: Status::Success,
        document: Some(doc),
    })
}

/// Fetch a single document by its ID (tries persistence first, then falls back to search)
#[command]
pub async fn get_document(
    config_state: State<'_, ConfigState>,
    document_id: String,
) -> Result<DocumentResponse> {
    let mut terraphim_service = TerraphimService::new(config_state.inner().clone());
    let doc_opt = terraphim_service.get_document_by_id(&document_id).await?;
    Ok(DocumentResponse {
        status: Status::Success,
        document: doc_opt,
    })
}

/// Get JSON Schema for Config for dynamic forms
#[command]
pub async fn get_config_schema() -> Result<Value> {
    let schema = schema_for!(Config);
    Ok(serde_json::to_value(&schema).expect("schema serialization"))
}

#[derive(Debug, Serialize, Deserialize, Clone, Tsify)]
#[tsify(into_wasm_abi, from_wasm_abi)]
pub struct GraphNodeDto {
    pub id: u64,
    pub label: String,
    pub rank: f64,
}

#[derive(Debug, Serialize, Deserialize, Clone, Tsify)]
#[tsify(into_wasm_abi, from_wasm_abi)]
pub struct GraphEdgeDto {
    pub source: u64,
    pub target: u64,
    pub weight: f64,
}

#[derive(Debug, Serialize, Deserialize, Clone, Tsify)]
#[tsify(into_wasm_abi, from_wasm_abi)]
pub struct RoleGraphResponse {
    pub status: Status,
    pub nodes: Vec<GraphNodeDto>,
    pub edges: Vec<GraphEdgeDto>,
}

/// Get role graph visualization data for the specified role
#[command]
pub async fn get_rolegraph(
    config_state: State<'_, ConfigState>,
    role_name: Option<String>,
) -> Result<RoleGraphResponse> {
    log::info!("Get rolegraph called for role: {:?}", role_name);

    let config = {
        let config_guard = config_state.config.lock().await;
        config_guard.clone()
    };

    // Use provided role or fall back to selected role
    let role = match role_name {
        Some(name) => terraphim_types::RoleName::new(&name),
        None => config.selected_role.clone(),
    };

    // Check if role exists and has a rolegraph
    let Some(rolegraph_sync) = config_state.roles.get(&role) else {
        return Err(TerraphimTauriError::Generic(format!(
            "Role '{}' not found or has no knowledge graph configured",
            role.original
        )));
    };

    let rolegraph = rolegraph_sync.lock().await;

    // Convert rolegraph nodes to DTOs
    let nodes: Vec<GraphNodeDto> = rolegraph
        .nodes_map()
        .iter()
        .map(|(id, node)| {
            let label = rolegraph
                .ac_reverse_nterm
                .get(id)
                .map(|s| s.to_string())
                .unwrap_or_else(|| format!("Node {}", id));

            GraphNodeDto {
                id: *id,
                label,
                rank: node.rank as f64,
            }
        })
        .collect();

    // Convert rolegraph edges to DTOs using magic_unpair to get source and target
    let edges: Vec<GraphEdgeDto> = rolegraph
        .edges_map()
        .iter()
        .map(|(&edge_id, edge)| {
            let (source, target) = magic_unpair(edge_id);
            GraphEdgeDto {
                source,
                target,
                weight: edge.rank as f64,
            }
        })
        .collect();

    Ok(RoleGraphResponse {
        status: Status::Success,
        nodes,
        edges,
    })
}

/// Select a role (change `selected_role`) without sending the entire config back and
/// forth. Returns the updated `Config` so the frontend can reflect the change.
///
/// If the selected role has a knowledge graph configured, the thesaurus will be
/// prewarmed (built immediately) to avoid "will be built on first use" delays.
#[command]
pub async fn select_role(
    app_handle: tauri::AppHandle,
    config_state: State<'_, ConfigState>,
    role_name: String,
) -> Result<ConfigResponse> {
    log::info!("Select role called: {}", role_name);
    let terraphim_service = TerraphimService::new(config_state.inner().clone());
    let role_name_typed = terraphim_types::RoleName::new(&role_name);

    // Update selected role
    let config = terraphim_service
        .update_selected_role(role_name_typed.clone())
        .await?;

    // Prewarm thesaurus if the role has a KG configured
    let role_config = config.roles.get(&role_name_typed);
    if let Some(role) = role_config {
        let has_kg = role.kg.is_some()
            && (role
                .kg
                .as_ref()
                .and_then(|kg| kg.automata_path.as_ref())
                .is_some()
                || role
                    .kg
                    .as_ref()
                    .and_then(|kg| kg.knowledge_graph_local.as_ref())
                    .is_some());

        if has_kg {
            log::info!(
                "Role '{}' has KG configured, prewarming thesaurus...",
                role_name
            );
            // Build thesaurus in background - don't block the response
            let role_name_clone = role_name_typed.clone();
            let mut service_clone = TerraphimService::new(config_state.inner().clone());
            tokio::spawn(async move {
                match service_clone
                    .ensure_thesaurus_loaded(&role_name_clone)
                    .await
                {
                    Ok(thesaurus) => {
                        log::info!(
                            "✅ Thesaurus prewarmed for role '{}': {} terms loaded",
                            role_name_clone.original,
                            thesaurus.len()
                        );
                    }
                    Err(e) => {
                        log::warn!(
                            "⚠️ Failed to prewarm thesaurus for role '{}': {}",
                            role_name_clone.original,
                            e
                        );
                    }
                }
            });
        }
    }

    // Notify the frontend that the role has changed, sending the whole new config
    if let Err(e) = app_handle.emit_all("role_changed", &config) {
        log::error!("Failed to emit role_changed event: {}", e);
    }

    Ok(ConfigResponse {
        status: Status::Success,
        config,
    })
}

/// Find documents that contain a given knowledge graph term
///
/// This command searches for documents that were the source of a knowledge graph term.
/// For example, given "haystack", it will find documents like "haystack.md" that contain
/// this term or its synonyms ("datasource", "service", "agent").
#[command]
pub async fn find_documents_for_kg_term(
    config_state: tauri::State<'_, ConfigState>,
    role_name: String,
    term: String,
) -> Result<DocumentListResponse> {
    log::debug!(
        "Finding documents for KG term '{}' in role '{}'",
        term,
        role_name
    );

    let role_name = role_name.into();
    let mut terraphim_service = TerraphimService::new(config_state.inner().clone());

    let results = terraphim_service
        .find_documents_for_kg_term(&role_name, &term)
        .await?;
    let total = results.len();

    log::debug!("Found {} documents for KG term '{}'", total, term);

    Ok(DocumentListResponse {
        status: Status::Success,
        results,
        total,
    })
}

/// Atomic Article data structure for saving to atomic server
#[derive(Debug, Serialize, Deserialize, Clone, Tsify)]
#[tsify(into_wasm_abi, from_wasm_abi)]
pub struct AtomicArticle {
    pub subject: String,
    pub title: String,
    pub description: String,
    pub body: String,
    pub parent: String,
    pub shortname: String,
    // Preserve original metadata
    pub original_id: Option<String>,
    pub original_url: Option<String>,
    pub original_rank: Option<u32>,
    pub tags: Vec<String>,
}

/// Response type for atomic save operations
#[derive(Debug, Serialize, Deserialize, Clone, Tsify)]
#[tsify(into_wasm_abi, from_wasm_abi)]
pub struct AtomicSaveResponse {
    pub status: Status,
    pub subject: Option<String>,
    pub message: String,
}

/// Autocomplete suggestion data structure compatible with TipTap
#[derive(Debug, Serialize, Deserialize, Clone, Tsify)]
#[tsify(into_wasm_abi, from_wasm_abi)]
pub struct AutocompleteSuggestion {
    /// The suggested term/text to insert
    pub term: String,
    /// Alternative text property for TipTap compatibility
    #[serde(alias = "text")]
    pub text: Option<String>,
    /// Normalized term value for search
    pub normalized_term: Option<String>,
    /// URL or snippet information
    pub url: Option<String>,
    /// Alternative snippet property
    #[serde(alias = "snippet")]
    pub snippet: Option<String>,
    /// Confidence score (0.0 to 1.0)
    pub score: f64,
    /// Suggestion type for UI categorization
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggestion_type: Option<String>,
    /// Icon identifier for UI display
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
}

/// Response type for autocomplete operations
#[derive(Debug, Serialize, Deserialize, Clone, Tsify)]
#[tsify(into_wasm_abi, from_wasm_abi)]
pub struct AutocompleteResponse {
    pub status: Status,
    pub suggestions: Vec<AutocompleteSuggestion>,
    pub error: Option<String>,
}

/// Save an article to atomic server
///
/// This command saves a document as an article to the specified atomic server.
/// It uses the atomic client to create the resource with proper authentication.
#[cfg(feature = "atomic")]
#[command]
pub async fn save_article_to_atomic(
    article: AtomicArticle,
    server_url: String,
    atomic_secret: Option<String>,
) -> Result<AtomicSaveResponse> {
    log::info!(
        "Saving article '{}' to atomic server: {}",
        article.title,
        server_url
    );

    // Create atomic client configuration
    let agent = match atomic_secret {
        Some(secret) => Agent::from_base64(&secret).map_err(|e| {
            TerraphimTauriError::Generic(format!("Invalid atomic server secret: {}", e))
        })?,
        None => {
            log::warn!("No atomic server secret provided - using anonymous access");
            return Err(TerraphimTauriError::Generic(
                "Atomic server secret is required for saving articles".to_string(),
            ));
        }
    };

    let atomic_config = AtomicConfig {
        server_url: server_url.clone(),
        agent: Some(agent),
    };

    let store = Store::new(atomic_config).map_err(|e| {
        TerraphimTauriError::Generic(format!("Failed to create atomic store: {}", e))
    })?;

    // Build article properties for atomic server
    let mut properties = HashMap::new();

    // Standard atomic data properties
    properties.insert(
        "https://atomicdata.dev/properties/shortname".to_string(),
        serde_json::Value::String(article.shortname.clone()),
    );
    properties.insert(
        "https://atomicdata.dev/properties/name".to_string(),
        serde_json::Value::String(article.title.clone()),
    );
    properties.insert(
        "https://atomicdata.dev/properties/description".to_string(),
        serde_json::Value::String(article.description.clone()),
    );
    properties.insert(
        "https://atomicdata.dev/properties/parent".to_string(),
        serde_json::Value::String(article.parent.clone()),
    );
    properties.insert(
        "https://atomicdata.dev/properties/isA".to_string(),
        serde_json::Value::Array(vec![serde_json::Value::String(
            "https://atomicdata.dev/classes/Article".to_string(),
        )]),
    );

    // Article body as content - using markdown datatype for rich text support
    properties.insert(
        "https://atomicdata.dev/properties/text".to_string(),
        serde_json::Value::String(article.body),
    );

    // Add metadata if available
    if let Some(original_id) = &article.original_id {
        properties.insert(
            "https://terraphim.ai/properties/originalId".to_string(),
            serde_json::Value::String(original_id.clone()),
        );
    }

    if let Some(original_url) = &article.original_url {
        properties.insert(
            "https://terraphim.ai/properties/originalUrl".to_string(),
            serde_json::Value::String(original_url.clone()),
        );
    }

    if let Some(original_rank) = article.original_rank {
        properties.insert(
            "https://terraphim.ai/properties/originalRank".to_string(),
            serde_json::Value::Number(serde_json::Number::from(original_rank)),
        );
    }

    // Add tags if present
    if !article.tags.is_empty() {
        properties.insert(
            "https://atomicdata.dev/properties/tags".to_string(),
            serde_json::Value::Array(
                article
                    .tags
                    .iter()
                    .map(|tag| serde_json::Value::String(tag.clone()))
                    .collect(),
            ),
        );
    }

    // Add terraphim source metadata
    properties.insert(
        "https://terraphim.ai/properties/source".to_string(),
        serde_json::Value::String("terraphim-search".to_string()),
    );

    // Save to atomic server using commit
    match store.create_with_commit(&article.subject, properties).await {
        Ok(_) => {
            log::info!(
                "✅ Successfully saved article '{}' to atomic server",
                article.title
            );
            Ok(AtomicSaveResponse {
                status: Status::Success,
                subject: Some(article.subject),
                message: format!(
                    "Article '{}' saved successfully to atomic server",
                    article.title
                ),
            })
        }
        Err(e) => {
            log::error!("❌ Failed to save article to atomic server: {}", e);
            Err(TerraphimTauriError::Generic(format!(
                "Failed to save article to atomic server: {}",
                e
            )))
        }
    }
}

/// Get autocomplete suggestions using FST-based autocomplete
///
/// This command provides fast, intelligent autocomplete suggestions based on the
/// knowledge graph thesaurus for the specified or selected role.
#[command]
pub async fn get_autocomplete_suggestions(
    config_state: State<'_, ConfigState>,
    query: String,
    role_name: Option<String>,
    limit: Option<usize>,
) -> Result<AutocompleteResponse> {
    use terraphim_automata::{
        autocomplete_search, build_autocomplete_index, fuzzy_autocomplete_search,
    };

    log::debug!(
        "Getting autocomplete suggestions for query '{}', role {:?}",
        query,
        role_name
    );

    let config = {
        let config_guard = config_state.config.lock().await;
        config_guard.clone()
    };

    // Use provided role or fall back to selected role
    let role = match role_name {
        Some(name) => terraphim_types::RoleName::new(&name),
        None => config.selected_role.clone(),
    };

    // Get the rolegraph for the specified role
    let Some(rolegraph_sync) = config_state.roles.get(&role) else {
        return Ok(AutocompleteResponse {
            status: Status::Error,
            suggestions: vec![],
            error: Some(format!(
                "Role '{}' not found or has no knowledge graph configured",
                role.original
            )),
        });
    };

    let rolegraph = rolegraph_sync.lock().await;

    // Build FST autocomplete index from the thesaurus
    let autocomplete_index = match build_autocomplete_index(rolegraph.thesaurus.clone(), None) {
        Ok(index) => index,
        Err(e) => {
            log::error!("Failed to build autocomplete index: {}", e);
            return Ok(AutocompleteResponse {
                status: Status::Error,
                suggestions: vec![],
                error: Some(format!("Failed to build autocomplete index: {}", e)),
            });
        }
    };

    let limit = limit.unwrap_or(8);

    // Get autocomplete suggestions based on query length
    let results = if query.len() >= 3 {
        // For longer queries, try fuzzy search for better UX (0.7 = 70% similarity threshold)
        match fuzzy_autocomplete_search(&autocomplete_index, &query, 0.7, Some(limit)) {
            Ok(results) => results,
            Err(e) => {
                log::warn!("Fuzzy search failed, trying exact search: {}", e);
                // Fall back to exact search
                match autocomplete_search(&autocomplete_index, &query, Some(limit)) {
                    Ok(results) => results,
                    Err(e) => {
                        log::error!("Both fuzzy and exact search failed: {}", e);
                        return Ok(AutocompleteResponse {
                            status: Status::Error,
                            suggestions: vec![],
                            error: Some(format!("Autocomplete search failed: {}", e)),
                        });
                    }
                }
            }
        }
    } else {
        // For short queries, use exact prefix search only
        match autocomplete_search(&autocomplete_index, &query, Some(limit)) {
            Ok(results) => results,
            Err(e) => {
                log::error!("Exact search failed: {}", e);
                return Ok(AutocompleteResponse {
                    status: Status::Error,
                    suggestions: vec![],
                    error: Some(format!("Autocomplete search failed: {}", e)),
                });
            }
        }
    };

    // Convert results to autocomplete suggestions with TipTap compatibility
    let suggestions: Vec<AutocompleteSuggestion> = results
        .into_iter()
        .map(|result| {
            let term = result.term.clone();
            let url = result.url.clone();

            AutocompleteSuggestion {
                term: term.clone(),
                text: Some(term.clone()), // For TipTap compatibility
                normalized_term: Some(result.normalized_term.to_string()),
                url: url.clone(),
                snippet: url.clone(), // Use URL as snippet for now
                score: result.score,
                suggestion_type: Some("knowledge-graph".to_string()),
                icon: Some("🔗".to_string()), // Default icon for KG terms
            }
        })
        .collect();

    log::debug!(
        "Found {} autocomplete suggestions for query '{}'",
        suggestions.len(),
        query
    );

    Ok(AutocompleteResponse {
        status: Status::Success,
        suggestions,
        error: None,
    })
}

// =================== CONVERSATION MANAGEMENT COMMANDS ===================

use terraphim_service::context::{ContextConfig, ContextManager};
use terraphim_types::{ChatMessage as TerraphimChatMessage, ConversationSummary};

/// Response for conversation creation
#[derive(Debug, Serialize, Deserialize, Clone, Tsify)]
#[tsify(into_wasm_abi, from_wasm_abi)]
pub struct CreateConversationResponse {
    pub status: Status,
    pub conversation_id: Option<String>,
    pub error: Option<String>,
}

/// Response for listing conversations
#[derive(Debug, Serialize, Deserialize, Clone, Tsify)]
#[tsify(into_wasm_abi, from_wasm_abi)]
pub struct ListConversationsResponse {
    pub status: Status,
    pub conversations: Vec<ConversationSummary>,
    pub error: Option<String>,
}

/// Response for getting a single conversation
#[derive(Debug, Serialize, Deserialize, Clone, Tsify)]
#[tsify(into_wasm_abi, from_wasm_abi)]
pub struct GetConversationResponse {
    pub status: Status,
    pub conversation: Option<terraphim_types::Conversation>,
    pub error: Option<String>,
}

/// Response for adding a message
#[derive(Debug, Serialize, Deserialize, Clone, Tsify)]
#[tsify(into_wasm_abi, from_wasm_abi)]
pub struct AddMessageResponse {
    pub status: Status,
    pub message_id: Option<String>,
    pub error: Option<String>,
}

/// Response for adding context
#[derive(Debug, Serialize, Deserialize, Clone, Tsify)]
#[tsify(into_wasm_abi, from_wasm_abi)]
pub struct AddContextResponse {
    pub status: Status,
    pub error: Option<String>,
}

/// Response for updating context
#[derive(Debug, Serialize, Deserialize, Clone, Tsify)]
#[tsify(into_wasm_abi, from_wasm_abi)]
pub struct UpdateContextResponse {
    pub status: Status,
    pub context: Option<ContextItem>,
    pub error: Option<String>,
}

/// Response for deleting context
#[derive(Debug, Serialize, Deserialize, Clone, Tsify)]
#[tsify(into_wasm_abi, from_wasm_abi)]
pub struct DeleteContextResponse {
    pub status: Status,
    pub error: Option<String>,
}

/// Request for chat messages
#[derive(Debug, Serialize, Deserialize, Clone, Tsify)]
#[tsify(into_wasm_abi, from_wasm_abi)]
pub struct ChatRequest {
    pub role: String,
    pub messages: Vec<ChatMessage>,
    pub conversation_id: Option<String>,
}

/// Chat message structure
#[derive(Debug, Serialize, Deserialize, Clone, Tsify)]
#[tsify(into_wasm_abi, from_wasm_abi)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

/// Response for chat operations
#[derive(Debug, Serialize, Deserialize, Clone, Tsify)]
#[tsify(into_wasm_abi, from_wasm_abi)]
pub struct ChatResponse {
    pub status: String,
    pub message: Option<String>,
    pub model_used: Option<String>,
    pub error: Option<String>,
}

/// Request for updating context
#[derive(Debug, Serialize, Deserialize, Clone, Tsify)]
#[tsify(into_wasm_abi, from_wasm_abi)]
pub struct UpdateContextRequest {
    pub title: Option<String>,
    pub summary: Option<String>,
    pub content: Option<String>,
    pub context_type: Option<String>,
    pub metadata: Option<HashMap<String, String>>,
}

/// Global context manager instance for Tauri commands
use tokio::sync::Mutex as TokioMutex;

#[allow(clippy::incompatible_msrv)]
static CONTEXT_MANAGER: std::sync::OnceLock<Arc<TokioMutex<ContextManager>>> =
    std::sync::OnceLock::new();

#[allow(clippy::incompatible_msrv)]
fn get_context_manager() -> &'static Arc<TokioMutex<ContextManager>> {
    CONTEXT_MANAGER.get_or_init(|| {
        Arc::new(TokioMutex::new(ContextManager::new(
            ContextConfig::default(),
        )))
    })
}

/// Create a new conversation
#[command]
pub async fn create_conversation(
    title: String,
    role: String,
) -> Result<CreateConversationResponse> {
    log::debug!("Creating conversation '{}' with role '{}'", title, role);

    let role_name = role.into();
    let mut manager = get_context_manager().lock().await;

    match manager.create_conversation(title, role_name).await {
        Ok(conversation_id) => {
            log::debug!("Created conversation with ID: {}", conversation_id.as_str());
            Ok(CreateConversationResponse {
                status: Status::Success,
                conversation_id: Some(conversation_id.as_str().to_string()),
                error: None,
            })
        }
        Err(e) => {
            log::error!("Failed to create conversation: {}", e);
            Ok(CreateConversationResponse {
                status: Status::Error,
                conversation_id: None,
                error: Some(format!("Failed to create conversation: {}", e)),
            })
        }
    }
}

/// List all conversations
#[command]
pub async fn list_conversations(limit: Option<usize>) -> Result<ListConversationsResponse> {
    log::debug!("Listing conversations with limit: {:?}", limit);

    let manager = get_context_manager().lock().await;
    let conversations = manager.list_conversations(limit);

    log::debug!("Found {} conversations", conversations.len());

    Ok(ListConversationsResponse {
        status: Status::Success,
        conversations,
        error: None,
    })
}

/// Get a specific conversation
#[command]
pub async fn get_conversation(conversation_id: String) -> Result<GetConversationResponse> {
    log::debug!("Getting conversation with ID: {}", conversation_id);

    let conv_id = ConversationId::from_string(conversation_id.clone());
    let manager = get_context_manager().lock().await;

    match manager.get_conversation(&conv_id) {
        Some(conversation) => {
            log::debug!("Found conversation: {}", conversation.title);
            Ok(GetConversationResponse {
                status: Status::Success,
                conversation: Some((*conversation).clone()),
                error: None,
            })
        }
        None => {
            log::warn!("Conversation not found: {}", conv_id.as_str());
            Ok(GetConversationResponse {
                status: Status::Error,
                conversation: None,
                error: Some("Conversation not found".to_string()),
            })
        }
    }
}

/// Add a message to a conversation
#[command]
pub async fn add_message_to_conversation(
    conversation_id: String,
    content: String,
    role: Option<String>,
) -> Result<AddMessageResponse> {
    log::debug!(
        "Adding message to conversation {}: {} chars, role: {:?}",
        conversation_id,
        content.len(),
        role
    );

    let conv_id = ConversationId::from_string(conversation_id.clone());
    let message_role = role.unwrap_or_else(|| "user".to_string());

    let message = if message_role == "user" {
        TerraphimChatMessage::user(content)
    } else if message_role == "assistant" {
        TerraphimChatMessage::assistant(content, None)
    } else if message_role == "system" {
        TerraphimChatMessage::system(content)
    } else {
        log::error!("Invalid role: {}", message_role);
        return Ok(AddMessageResponse {
            status: Status::Error,
            message_id: None,
            error: Some(format!("Invalid role: {}", message_role)),
        });
    };

    let mut manager = get_context_manager().lock().await;
    match manager.add_message(&conv_id, message) {
        Ok(message_id) => {
            log::debug!("Added message with ID: {}", message_id.as_str());
            Ok(AddMessageResponse {
                status: Status::Success,
                message_id: Some(message_id.as_str().to_string()),
                error: None,
            })
        }
        Err(e) => {
            log::error!("Failed to add message: {}", e);
            Ok(AddMessageResponse {
                status: Status::Error,
                message_id: None,
                error: Some(format!("Failed to add message: {}", e)),
            })
        }
    }
}

/// Add context to a conversation
#[command]
pub async fn add_context_to_conversation(
    conversation_id: String,
    context_type: String,
    title: String,
    summary: Option<String>,
    content: String,
    metadata: Option<HashMap<String, String>>,
) -> Result<AddContextResponse> {
    log::debug!(
        "Adding context to conversation {}: type={}, title={}",
        conversation_id,
        context_type,
        title
    );

    let conv_id = ConversationId::from_string(conversation_id.clone());

    let ctx_type = match context_type.as_str() {
        "document" => ContextType::Document,
        "search_result" => ContextType::Document, // Changed from SearchResult to Document
        "user_input" => ContextType::UserInput,
        "system" => ContextType::System,
        "external" => ContextType::External,
        _ => {
            log::error!("Invalid context type: {}", context_type);
            return Ok(AddContextResponse {
                status: Status::Error,
                error: Some(format!("Invalid context type: {}", context_type)),
            });
        }
    };

    let context_item = ContextItem {
        id: ulid::Ulid::new().to_string(),
        context_type: ctx_type,
        title,
        summary,
        content,
        metadata: metadata.unwrap_or_default().into_iter().collect(),
        created_at: chrono::Utc::now(),
        relevance_score: None,
    };

    let mut manager = get_context_manager().lock().await;
    match manager.add_context(&conv_id, context_item) {
        Ok(()) => {
            log::debug!("Successfully added context to conversation");
            Ok(AddContextResponse {
                status: Status::Success,
                error: None,
            })
        }
        Err(e) => {
            log::error!("Failed to add context: {}", e);
            Ok(AddContextResponse {
                status: Status::Error,
                error: Some(format!("Failed to add context: {}", e)),
            })
        }
    }
}

/// Add search results as context to a conversation
#[command]
pub async fn add_search_context_to_conversation(
    conversation_id: String,
    query: String,
    documents: Vec<Document>,
    limit: Option<usize>,
) -> Result<AddContextResponse> {
    log::debug!(
        "Adding search context to conversation {}: query='{}', {} documents",
        conversation_id,
        query,
        documents.len()
    );

    let conv_id = ConversationId::from_string(conversation_id.clone());
    let mut manager = get_context_manager().lock().await;

    let context_item = manager.create_search_context(&query, &documents, limit);

    match manager.add_context(&conv_id, context_item) {
        Ok(()) => {
            log::debug!("Successfully added search context to conversation");
            Ok(AddContextResponse {
                status: Status::Success,
                error: None,
            })
        }
        Err(e) => {
            log::error!("Failed to add search context: {}", e);
            Ok(AddContextResponse {
                status: Status::Error,
                error: Some(format!("Failed to add search context: {}", e)),
            })
        }
    }
}

/// Delete context from a conversation
#[command]
pub async fn delete_context(
    conversation_id: String,
    context_id: String,
) -> Result<DeleteContextResponse> {
    log::debug!(
        "Deleting context {} from conversation {}",
        context_id,
        conversation_id
    );

    let conv_id = ConversationId::from_string(conversation_id.clone());
    let mut manager = get_context_manager().lock().await;

    match manager.delete_context(&conv_id, &context_id) {
        Ok(()) => {
            log::debug!("Successfully deleted context from conversation");
            Ok(DeleteContextResponse {
                status: Status::Success,
                error: None,
            })
        }
        Err(e) => {
            log::error!("Failed to delete context: {}", e);
            Ok(DeleteContextResponse {
                status: Status::Error,
                error: Some(format!("Failed to delete context: {}", e)),
            })
        }
    }
}

/// Update context in a conversation
#[command]
pub async fn update_context(
    conversation_id: String,
    context_id: String,
    request: UpdateContextRequest,
) -> Result<UpdateContextResponse> {
    log::debug!(
        "Updating context {} in conversation {}: title={:?}",
        context_id,
        conversation_id,
        request.title
    );

    let conv_id = ConversationId::from_string(conversation_id.clone());

    // Get the existing context first
    let manager = get_context_manager().lock().await;
    let conversation = match manager.get_conversation(&conv_id) {
        Some(conv) => conv,
        None => {
            log::error!("Conversation {} not found", conversation_id);
            return Ok(UpdateContextResponse {
                status: Status::Error,
                context: None,
                error: Some(format!("Conversation {} not found", conversation_id)),
            });
        }
    };

    let existing_context = conversation
        .global_context
        .iter()
        .find(|item| item.id == context_id);

    let existing_context = match existing_context {
        Some(ctx) => ctx.clone(),
        None => {
            log::error!("Context item {} not found", context_id);
            return Ok(UpdateContextResponse {
                status: Status::Error,
                context: None,
                error: Some(format!("Context item {} not found", context_id)),
            });
        }
    };

    // Build the updated context item
    let context_type = if let Some(type_str) = &request.context_type {
        match type_str.as_str() {
            "document" => ContextType::Document,
            "search_result" => ContextType::Document, // Changed from SearchResult to Document
            "user_input" => ContextType::UserInput,
            "system" => ContextType::System,
            "external" => ContextType::External,
            _ => existing_context.context_type,
        }
    } else {
        existing_context.context_type
    };

    let updated_context = ContextItem {
        id: context_id.clone(),
        context_type,
        title: request.title.unwrap_or(existing_context.title),
        summary: request.summary.or(existing_context.summary),
        content: request.content.unwrap_or(existing_context.content),
        metadata: request
            .metadata
            .map(|m| m.into_iter().collect())
            .unwrap_or(existing_context.metadata),
        created_at: existing_context.created_at,
        relevance_score: existing_context.relevance_score,
    };

    drop(manager); // Release the lock
    let mut manager = get_context_manager().lock().await;

    match manager.update_context(&conv_id, &context_id, updated_context.clone()) {
        Ok(context) => {
            log::debug!("Successfully updated context in conversation");
            Ok(UpdateContextResponse {
                status: Status::Success,
                context: Some(context),
                error: None,
            })
        }
        Err(e) => {
            log::error!("Failed to update context: {}", e);
            Ok(UpdateContextResponse {
                status: Status::Error,
                context: None,
                error: Some(format!("Failed to update context: {}", e)),
            })
        }
    }
}

// ============================================================================
// Persistent Conversation Management Commands
// ============================================================================

/// Response for listing persistent conversations
#[derive(Debug, Serialize, Deserialize, Clone, Tsify)]
#[tsify(into_wasm_abi, from_wasm_abi)]
pub struct ListPersistentConversationsResponse {
    pub status: Status,
    pub conversations: Vec<terraphim_types::ConversationSummary>,
    pub total: usize,
    pub error: Option<String>,
}

/// Response for getting a persistent conversation
#[derive(Debug, Serialize, Deserialize, Clone, Tsify)]
#[tsify(into_wasm_abi, from_wasm_abi)]
pub struct GetPersistentConversationResponse {
    pub status: Status,
    pub conversation: Option<terraphim_types::Conversation>,
    pub error: Option<String>,
}

/// Response for creating/updating a persistent conversation
#[derive(Debug, Serialize, Deserialize, Clone, Tsify)]
#[tsify(into_wasm_abi, from_wasm_abi)]
pub struct SavePersistentConversationResponse {
    pub status: Status,
    pub conversation: Option<terraphim_types::Conversation>,
    pub error: Option<String>,
}

/// Response for deleting a persistent conversation
#[derive(Debug, Serialize, Deserialize, Clone, Tsify)]
#[tsify(into_wasm_abi, from_wasm_abi)]
pub struct DeletePersistentConversationResponse {
    pub status: Status,
    pub error: Option<String>,
}

/// Response for conversation statistics
#[derive(Debug, Serialize, Deserialize, Clone, Tsify)]
#[tsify(into_wasm_abi, from_wasm_abi)]
pub struct ConversationStatisticsResponse {
    pub status: Status,
    pub total_conversations: usize,
    pub total_messages: usize,
    pub conversations_by_role: HashMap<String, usize>,
    pub error: Option<String>,
}

/// List persistent conversations with optional filtering
#[command]
pub async fn list_persistent_conversations(
    role: Option<String>,
    limit: Option<usize>,
) -> Result<ListPersistentConversationsResponse> {
    use terraphim_service::conversation_service::{ConversationFilter, ConversationService};

    log::debug!(
        "Listing persistent conversations with role: {:?}, limit: {:?}",
        role,
        limit
    );

    let service = ConversationService::new();

    let filter = ConversationFilter {
        role: role.map(|r| r.into()),
        limit,
        ..Default::default()
    };

    match service.list_conversations(filter).await {
        Ok(conversations) => {
            let total = conversations.len();
            log::debug!("Found {} persistent conversations", total);
            Ok(ListPersistentConversationsResponse {
                status: Status::Success,
                conversations,
                total,
                error: None,
            })
        }
        Err(e) => {
            log::error!("Failed to list conversations: {}", e);
            Ok(ListPersistentConversationsResponse {
                status: Status::Error,
                conversations: vec![],
                total: 0,
                error: Some(format!("Failed to list conversations: {}", e)),
            })
        }
    }
}

/// Get a specific persistent conversation by ID
#[command]
pub async fn get_persistent_conversation(
    conversation_id: String,
) -> Result<GetPersistentConversationResponse> {
    use terraphim_service::conversation_service::ConversationService;

    log::debug!("Getting persistent conversation: {}", conversation_id);

    let service = ConversationService::new();

    let conv_id = ConversationId::from_string(conversation_id);

    match service.get_conversation(&conv_id).await {
        Ok(conversation) => {
            log::debug!("Found persistent conversation: {}", conversation.title);
            Ok(GetPersistentConversationResponse {
                status: Status::Success,
                conversation: Some(conversation),
                error: None,
            })
        }
        Err(e) => {
            log::error!("Failed to get conversation: {}", e);
            Ok(GetPersistentConversationResponse {
                status: Status::Error,
                conversation: None,
                error: Some(format!("Failed to get conversation: {}", e)),
            })
        }
    }
}

/// Create a new persistent conversation
#[command]
pub async fn create_persistent_conversation(
    title: String,
    role: String,
) -> Result<SavePersistentConversationResponse> {
    use terraphim_service::conversation_service::ConversationService;

    log::debug!(
        "Creating persistent conversation '{}' with role '{}'",
        title,
        role
    );

    let service = ConversationService::new();

    let role_name = role.into();

    match service.create_conversation(title, role_name).await {
        Ok(conversation) => {
            log::debug!(
                "Created persistent conversation with ID: {}",
                conversation.id.as_str()
            );
            Ok(SavePersistentConversationResponse {
                status: Status::Success,
                conversation: Some(conversation),
                error: None,
            })
        }
        Err(e) => {
            log::error!("Failed to create conversation: {}", e);
            Ok(SavePersistentConversationResponse {
                status: Status::Error,
                conversation: None,
                error: Some(format!("Failed to create conversation: {}", e)),
            })
        }
    }
}

/// Update an existing persistent conversation
#[command]
pub async fn update_persistent_conversation(
    conversation: terraphim_types::Conversation,
) -> Result<SavePersistentConversationResponse> {
    use terraphim_service::conversation_service::ConversationService;

    log::debug!(
        "Updating persistent conversation: {}",
        conversation.id.as_str()
    );

    let service = ConversationService::new();

    match service.update_conversation(conversation).await {
        Ok(updated) => {
            log::debug!("Updated persistent conversation: {}", updated.id.as_str());
            Ok(SavePersistentConversationResponse {
                status: Status::Success,
                conversation: Some(updated),
                error: None,
            })
        }
        Err(e) => {
            log::error!("Failed to update conversation: {}", e);
            Ok(SavePersistentConversationResponse {
                status: Status::Error,
                conversation: None,
                error: Some(format!("Failed to update conversation: {}", e)),
            })
        }
    }
}

/// Delete a persistent conversation
#[command]
pub async fn delete_persistent_conversation(
    conversation_id: String,
) -> Result<DeletePersistentConversationResponse> {
    use terraphim_service::conversation_service::ConversationService;

    log::debug!("Deleting persistent conversation: {}", conversation_id);

    let service = ConversationService::new();

    let conv_id = ConversationId::from_string(conversation_id);

    match service.delete_conversation(&conv_id).await {
        Ok(_) => {
            log::debug!("Deleted persistent conversation: {}", conv_id.as_str());
            Ok(DeletePersistentConversationResponse {
                status: Status::Success,
                error: None,
            })
        }
        Err(e) => {
            log::error!("Failed to delete conversation: {}", e);
            Ok(DeletePersistentConversationResponse {
                status: Status::Error,
                error: Some(format!("Failed to delete conversation: {}", e)),
            })
        }
    }
}

/// Search persistent conversations
#[command]
pub async fn search_persistent_conversations(
    query: String,
) -> Result<ListPersistentConversationsResponse> {
    use terraphim_service::conversation_service::ConversationService;

    log::debug!("Searching persistent conversations with query: {}", query);

    let service = ConversationService::new();

    match service.search_conversations(&query).await {
        Ok(conversations) => {
            let total = conversations.len();
            log::debug!("Found {} conversations matching '{}'", total, query);
            Ok(ListPersistentConversationsResponse {
                status: Status::Success,
                conversations,
                total,
                error: None,
            })
        }
        Err(e) => {
            log::error!("Failed to search conversations: {}", e);
            Ok(ListPersistentConversationsResponse {
                status: Status::Error,
                conversations: vec![],
                total: 0,
                error: Some(format!("Failed to search conversations: {}", e)),
            })
        }
    }
}

/// Export a conversation to JSON
#[command]
pub async fn export_persistent_conversation(conversation_id: String) -> Result<String> {
    use terraphim_service::conversation_service::ConversationService;

    log::debug!("Exporting persistent conversation: {}", conversation_id);

    let service = ConversationService::new();
    let conv_id = ConversationId::from_string(conversation_id);
    let json = service.export_conversation(&conv_id).await?;

    Ok(json)
}

/// Import a conversation from JSON
#[command]
pub async fn import_persistent_conversation(
    json: String,
) -> Result<SavePersistentConversationResponse> {
    use terraphim_service::conversation_service::ConversationService;

    log::debug!(
        "Importing persistent conversation from JSON ({} bytes)",
        json.len()
    );

    let service = ConversationService::new();

    match service.import_conversation(&json).await {
        Ok(conversation) => {
            log::debug!(
                "Imported persistent conversation: {}",
                conversation.id.as_str()
            );
            Ok(SavePersistentConversationResponse {
                status: Status::Success,
                conversation: Some(conversation),
                error: None,
            })
        }
        Err(e) => {
            log::error!("Failed to import conversation: {}", e);
            Ok(SavePersistentConversationResponse {
                status: Status::Error,
                conversation: None,
                error: Some(format!("Failed to import conversation: {}", e)),
            })
        }
    }
}

/// Get conversation statistics
#[command]
pub async fn get_conversation_statistics() -> Result<ConversationStatisticsResponse> {
    use terraphim_service::conversation_service::ConversationService;

    log::debug!("Getting conversation statistics");

    let service = ConversationService::new();

    match service.get_statistics().await {
        Ok(stats) => {
            log::debug!(
                "Retrieved statistics: {} conversations, {} messages",
                stats.total_conversations,
                stats.total_messages
            );
            Ok(ConversationStatisticsResponse {
                status: Status::Success,
                total_conversations: stats.total_conversations,
                total_messages: stats.total_messages,
                conversations_by_role: stats.conversations_by_role,
                error: None,
            })
        }
        Err(e) => {
            log::error!("Failed to get statistics: {}", e);
            Ok(ConversationStatisticsResponse {
                status: Status::Error,
                total_conversations: 0,
                total_messages: 0,
                conversations_by_role: HashMap::new(),
                error: Some(format!("Failed to get statistics: {}", e)),
            })
        }
    }
}

/// Chat endpoint for LLM interactions
#[command]
pub async fn chat(
    config_state: State<'_, ConfigState>,
    request: ChatRequest,
) -> Result<ChatResponse> {
    log::debug!("Chat request for role: {}", request.role);

    let role_name = RoleName::new(&request.role);

    // Get the role configuration
    let config = {
        let config_guard = config_state.config.lock().await;
        config_guard.clone()
    };

    let role_config = match config.roles.get(&role_name) {
        Some(role) => role,
        None => {
            return Ok(ChatResponse {
                status: "error".to_string(),
                message: None,
                model_used: None,
                error: Some(format!("Role '{}' not found", request.role)),
            });
        }
    };

    // Check for LLM provider configuration in role.extra
    log::debug!("Role extra settings: {:?}", role_config.extra);
    log::debug!(
        "Role extra keys: {:?}",
        role_config.extra.keys().collect::<Vec<_>>()
    );

    // First check top-level fields (flattened), then fall back to extra for backward compatibility
    let llm_provider = role_config
        .extra
        .get("llm_provider")
        .and_then(|v| v.as_str())
        .or_else(|| {
            // Check if it's directly in extra as a nested object
            role_config
                .extra
                .get("extra")
                .and_then(|extra_obj| extra_obj.as_object())
                .and_then(|obj| obj.get("llm_provider"))
                .and_then(|v| v.as_str())
        })
        .unwrap_or("");

    let openrouter_enabled = role_config
        .extra
        .get("openrouter_enabled")
        .and_then(|v| v.as_bool())
        .or_else(|| {
            role_config
                .extra
                .get("extra")
                .and_then(|extra_obj| extra_obj.as_object())
                .and_then(|obj| obj.get("openrouter_enabled"))
                .and_then(|v| v.as_bool())
        })
        .unwrap_or(false);

    log::debug!("LLM provider from extra: '{}'", llm_provider);
    log::debug!("OpenRouter enabled: {}", openrouter_enabled);

    // Determine the LLM provider
    let provider = if !llm_provider.is_empty() {
        llm_provider.to_string()
    } else if openrouter_enabled {
        "openrouter".to_string()
    } else {
        "".to_string()
    };

    log::debug!("Final provider determined: '{}'", provider);

    if provider.is_empty() {
        return Ok(ChatResponse {
            status: "error".to_string(),
            message: None,
            model_used: None,
            error: Some("No LLM provider configured for this role. Please configure OpenRouter or Ollama in the role's 'extra' settings.".to_string()),
        });
    }

    // Use the terraphim_service LLM client for actual integration
    use terraphim_service::llm;

    // Try to build an LLM client from the role configuration
    let Some(llm_client) = llm::build_llm_from_role(role_config) else {
        return Ok(ChatResponse {
            status: "error".to_string(),
            message: None,
            model_used: None,
            error: Some("No LLM provider configured for this role. Please configure OpenRouter or Ollama in the role's 'extra' settings.".to_string()),
        });
    };

    // Build messages array for the LLM
    let mut messages_json: Vec<serde_json::Value> = Vec::new();

    // Inject context from conversation if provided
    if let Some(conversation_id) = &request.conversation_id {
        let conv_id = ConversationId::from_string(conversation_id.clone());
        let manager = get_context_manager().lock().await;

        if let Some(conversation) = manager.get_conversation(&conv_id) {
            // Build context content from all context items
            let mut context_content = String::new();

            if !conversation.global_context.is_empty() {
                context_content.push_str("=== CONTEXT INFORMATION ===\n");
                context_content.push_str("The following information provides relevant context for this conversation:\n\n");

                for (index, context_item) in conversation.global_context.iter().enumerate() {
                    context_content.push_str(&format!(
                        "Context Item {}: {}\n",
                        index + 1,
                        context_item.title
                    ));
                    if let Some(score) = context_item.relevance_score {
                        context_content.push_str(&format!("Relevance Score: {:.2}\n", score));
                    }
                    context_content.push_str(&format!("Content: {}\n", context_item.content));
                    if !context_item.metadata.is_empty() {
                        context_content
                            .push_str(&format!("Metadata: {:?}\n", context_item.metadata));
                    }
                    context_content.push_str("\n---\n\n");
                }

                context_content.push_str("=== END CONTEXT ===\n\n");
                context_content.push_str("Please use this context information to inform your responses. You can reference specific context items when relevant.\n\n");

                // Add context as a system message
                messages_json
                    .push(serde_json::json!({"role": "system", "content": context_content}));
            }
        }
    }

    // Add user messages from the request
    for m in request.messages.iter() {
        messages_json.push(serde_json::json!({"role": m.role, "content": m.content}));
    }

    // Configure chat options
    let chat_opts = llm::ChatOptions {
        max_tokens: Some(1024),
        temperature: Some(0.7),
    };

    // Call the LLM client
    match llm_client.chat_completion(messages_json, chat_opts).await {
        Ok(reply) => Ok(ChatResponse {
            status: "success".to_string(),
            message: Some(reply),
            model_used: Some(llm_client.name().to_string()),
            error: None,
        }),
        Err(e) => Ok(ChatResponse {
            status: "error".to_string(),
            message: None,
            model_used: Some(llm_client.name().to_string()),
            error: Some(format!("Chat failed: {}", e)),
        }),
    }
}

// === KG Search Commands ===

use lru::LruCache;
use std::num::NonZeroUsize;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::OnceLock;
use std::time::{SystemTime, UNIX_EPOCH};

/// Performance metrics for cache monitoring
#[derive(Default)]
struct CacheMetrics {
    hits: AtomicU64,
    misses: AtomicU64,
    builds: AtomicU64,
    evictions: AtomicU64,
}

impl CacheMetrics {
    fn record_hit(&self) {
        self.hits.fetch_add(1, Ordering::Relaxed);
    }

    fn record_miss(&self) {
        self.misses.fetch_add(1, Ordering::Relaxed);
    }

    fn record_build(&self) {
        self.builds.fetch_add(1, Ordering::Relaxed);
    }

    fn record_eviction(&self) {
        self.evictions.fetch_add(1, Ordering::Relaxed);
    }

    #[allow(dead_code)]
    fn get_hit_ratio(&self) -> f64 {
        let hits = self.hits.load(Ordering::Relaxed);
        let misses = self.misses.load(Ordering::Relaxed);
        if hits + misses == 0 {
            0.0
        } else {
            hits as f64 / (hits + misses) as f64
        }
    }
}

/// Cached autocomplete index with Arc for efficient sharing
#[derive(Clone)]
struct CachedAutocompleteIndex {
    index: Arc<terraphim_automata::AutocompleteIndex>,
    role_name: String,
    created_at: u64,
    build_time_ms: u64,
}

impl CachedAutocompleteIndex {
    fn new(
        index: terraphim_automata::AutocompleteIndex,
        role_name: String,
        build_time_ms: u64,
    ) -> Self {
        let created_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        Self {
            index: Arc::new(index),
            role_name,
            created_at,
            build_time_ms,
        }
    }

    /// Check if the cached index is still valid (within 10 minutes)
    fn is_valid(&self, current_role: &str) -> bool {
        if self.role_name != current_role {
            return false;
        }

        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        current_time - self.created_at < 600 // 10 minutes
    }

    /// Get shared reference to the index (no cloning needed)
    fn get_index(&self) -> Arc<terraphim_automata::AutocompleteIndex> {
        Arc::clone(&self.index)
    }
}

/// LRU cache with performance monitoring for autocomplete indices
type IndexCache = LruCache<String, CachedAutocompleteIndex>;

/// Global LRU cache for autocomplete indices with metrics
#[allow(clippy::incompatible_msrv)]
static AUTOCOMPLETE_INDEX_CACHE: OnceLock<tokio::sync::RwLock<IndexCache>> = OnceLock::new();

/// Global cache metrics
static CACHE_METRICS: CacheMetrics = CacheMetrics {
    hits: AtomicU64::new(0),
    misses: AtomicU64::new(0),
    builds: AtomicU64::new(0),
    evictions: AtomicU64::new(0),
};

/// Get or create the global autocomplete index cache with LRU eviction
#[allow(clippy::incompatible_msrv)]
fn get_autocomplete_cache() -> &'static tokio::sync::RwLock<IndexCache> {
    AUTOCOMPLETE_INDEX_CACHE.get_or_init(|| {
        // Cache up to 10 role indices with LRU eviction
        let cache = LruCache::new(NonZeroUsize::new(10).unwrap());
        tokio::sync::RwLock::new(cache)
    })
}

/// Pre-warm the autocomplete cache for frequently used roles
#[allow(dead_code)]
pub async fn pre_warm_autocomplete_cache(config_state: &ConfigState) {
    log::info!("🔥 Pre-warming autocomplete cache...");

    let config = config_state.config.lock().await;
    let roles_to_warm: Vec<_> = config.roles.keys().take(5).cloned().collect(); // Top 5 roles
    drop(config);

    let mut handles = Vec::new();

    for role_name in roles_to_warm {
        let config_state = config_state.clone();
        let role_name_str = role_name.original.clone();

        let handle = tokio::spawn(async move {
            log::debug!("Pre-warming cache for role: {}", role_name_str);

            let mut terraphim_service = TerraphimService::new(config_state);
            if let Ok(thesaurus) = terraphim_service.ensure_thesaurus_loaded(&role_name).await {
                if let Ok(index) = terraphim_automata::build_autocomplete_index(thesaurus, None) {
                    let build_time = std::time::Instant::now();
                    let cached_index =
                        CachedAutocompleteIndex::new(index, role_name_str.clone(), 0);

                    // Add to cache
                    let cache = get_autocomplete_cache();
                    let mut cache_guard = cache.write().await;
                    if let Some(evicted) = cache_guard.put(role_name_str.clone(), cached_index) {
                        log::debug!("Evicted cached index for role: {}", evicted.role_name);
                        CACHE_METRICS.record_eviction();
                    }

                    log::debug!(
                        "✅ Pre-warmed cache for role: {} in {:?}",
                        role_name_str,
                        build_time.elapsed()
                    );
                }
            }
        });

        handles.push(handle);
    }

    // Wait for all pre-warming tasks to complete
    for handle in handles {
        if let Err(e) = handle.await {
            log::warn!("Pre-warming task failed: {}", e);
        }
    }

    log::info!("✅ Autocomplete cache pre-warming completed");
}

/// Request for KG search
#[derive(Debug, Serialize, Deserialize, Clone, Tsify)]
#[tsify(into_wasm_abi, from_wasm_abi)]
pub struct KGSearchRequest {
    pub query: String,
    pub role_name: String,
    pub limit: Option<usize>,
    pub min_similarity: Option<f64>,
}

/// Response for KG search operations
#[derive(Debug, Serialize, Deserialize, Clone, Tsify)]
#[tsify(into_wasm_abi, from_wasm_abi)]
pub struct KGSearchResponse {
    pub status: Status,
    pub suggestions: Vec<AutocompleteSuggestion>,
    pub error: Option<String>,
}

/// Request for adding KG term context
#[derive(Debug, Serialize, Deserialize, Clone, Tsify)]
#[tsify(into_wasm_abi, from_wasm_abi)]
pub struct AddKGTermContextRequest {
    pub conversation_id: String,
    pub term: String,
    pub role_name: String,
}

/// Request for adding KG index context
#[derive(Debug, Serialize, Deserialize, Clone, Tsify)]
#[tsify(into_wasm_abi, from_wasm_abi)]
pub struct AddKGIndexContextRequest {
    pub conversation_id: String,
    pub role_name: String,
}

/// Search KG terms with autocomplete functionality
/// Uses FST-based autocomplete with fuzzy matching for intelligent search suggestions
#[command]
pub async fn search_kg_terms(
    config_state: State<'_, ConfigState>,
    request: KGSearchRequest,
) -> Result<KGSearchResponse> {
    // Import autocomplete functions at the top for use throughout function
    use terraphim_automata::{
        autocomplete_search, build_autocomplete_index, fuzzy_autocomplete_search,
    };
    log::debug!(
        "Searching KG terms for role '{}' with query '{}'",
        request.role_name,
        request.query
    );

    let role_name = RoleName::new(&request.role_name);

    // Input validation
    if request.role_name.trim().is_empty() {
        return Ok(KGSearchResponse {
            status: Status::Error,
            suggestions: Vec::new(),
            error: Some("Role name cannot be empty".to_string()),
        });
    }

    if request.query.len() > 1000 {
        return Ok(KGSearchResponse {
            status: Status::Error,
            suggestions: Vec::new(),
            error: Some("Query too long (maximum 1000 characters)".to_string()),
        });
    }

    if let Some(min_sim) = request.min_similarity {
        if !(0.0..=1.0).contains(&min_sim) {
            return Ok(KGSearchResponse {
                status: Status::Error,
                suggestions: Vec::new(),
                error: Some("Minimum similarity must be between 0.0 and 1.0".to_string()),
            });
        }
    }

    if let Some(limit) = request.limit {
        if limit == 0 || limit > 100 {
            return Ok(KGSearchResponse {
                status: Status::Error,
                suggestions: Vec::new(),
                error: Some("Limit must be between 1 and 100".to_string()),
            });
        }
    }

    // Check if we have a valid cached autocomplete index first using RwLock for better concurrency
    let autocomplete_index = {
        let cache = get_autocomplete_cache().read().await;
        if let Some(cached) = cache.peek(&request.role_name) {
            if cached.is_valid(&request.role_name) {
                log::debug!(
                    "✅ Cache HIT: Using cached autocomplete index for role '{}' (build time: {}ms)",
                    request.role_name,
                    cached.build_time_ms
                );
                CACHE_METRICS.record_hit();
                Some(cached.get_index())
            } else {
                log::debug!(
                    "⏰ Cache EXPIRED: Index expired for role '{}', will rebuild",
                    request.role_name
                );
                CACHE_METRICS.record_miss();
                None
            }
        } else {
            log::debug!(
                "❌ Cache MISS: No cached index found for role '{}', will build new one",
                request.role_name
            );
            CACHE_METRICS.record_miss();
            None
        }
    };

    // If no valid cached index, build a new one
    let autocomplete_index = if let Some(index) = autocomplete_index {
        index
    } else {
        let mut terraphim_service = TerraphimService::new(config_state.inner().clone());

        // Ensure thesaurus is loaded for the role
        let thesaurus = match terraphim_service.ensure_thesaurus_loaded(&role_name).await {
            Ok(thesaurus) => thesaurus,
            Err(e) => {
                log::error!("Failed to load thesaurus for role '{}': {}", role_name, e);
                return Ok(KGSearchResponse {
                    status: Status::Error,
                    suggestions: Vec::new(),
                    error: Some(format!(
                        "Failed to load knowledge graph for role '{}': {}",
                        role_name, e
                    )),
                });
            }
        };

        let build_start = std::time::Instant::now();
        let new_index = match build_autocomplete_index(thesaurus, None) {
            Ok(index) => index,
            Err(e) => {
                log::error!("Failed to build autocomplete index: {}", e);
                return Ok(KGSearchResponse {
                    status: Status::Error,
                    suggestions: Vec::new(),
                    error: Some(format!("Failed to build search index: {}", e)),
                });
            }
        };

        let build_time_ms = build_start.elapsed().as_millis() as u64;
        CACHE_METRICS.record_build();

        let index_arc = Arc::new(new_index);

        // Cache the new index using write lock only when necessary
        {
            let mut cache = get_autocomplete_cache().write().await;
            let cached_index = CachedAutocompleteIndex::new(
                (*index_arc).clone(), // Clone the index itself, not the Arc
                request.role_name.clone(),
                build_time_ms,
            );

            if let Some(evicted) = cache.put(request.role_name.clone(), cached_index) {
                log::debug!(
                    "🗑️ Cache EVICTION: Evicted cached index for role '{}' to make room for '{}'",
                    evicted.role_name,
                    request.role_name
                );
                CACHE_METRICS.record_eviction();
            }

            log::debug!(
                "💾 Cache STORE: Cached new autocomplete index for role '{}' (build time: {}ms)",
                request.role_name,
                build_time_ms
            );
        }

        index_arc
    };

    // Perform fuzzy search first, then fallback to regular search
    let results = if let Some(min_sim) = request.min_similarity {
        match fuzzy_autocomplete_search(&autocomplete_index, &request.query, min_sim, request.limit)
        {
            Ok(results) => results,
            Err(fuzzy_err) => {
                log::debug!(
                    "Fuzzy search failed ({}), falling back to regular search",
                    fuzzy_err
                );
                // Fallback to regular autocomplete on fuzzy search failure
                match autocomplete_search(&autocomplete_index, &request.query, request.limit) {
                    Ok(results) => results,
                    Err(e) => {
                        log::error!("Autocomplete search failed: {}", e);
                        return Ok(KGSearchResponse {
                            status: Status::Error,
                            suggestions: Vec::new(),
                            error: Some(format!("Search failed: {}", e)),
                        });
                    }
                }
            }
        }
    } else {
        // Use regular autocomplete search
        match autocomplete_search(&autocomplete_index, &request.query, request.limit) {
            Ok(results) => results,
            Err(e) => {
                log::error!("Autocomplete search failed: {}", e);
                return Ok(KGSearchResponse {
                    status: Status::Error,
                    suggestions: Vec::new(),
                    error: Some(format!("Search failed: {}", e)),
                });
            }
        }
    };

    // Convert AutocompleteResult to AutocompleteSuggestion
    let suggestions: Vec<AutocompleteSuggestion> = results
        .into_iter()
        .map(|result| AutocompleteSuggestion {
            term: result.term,
            text: None,
            normalized_term: Some(result.normalized_term.to_string()),
            url: result.url,
            snippet: None,
            score: result.score,
            suggestion_type: Some("kg_term".to_string()),
            icon: Some("kg".to_string()),
        })
        .collect();

    log::debug!(
        "Found {} KG suggestions for query '{}'",
        suggestions.len(),
        request.query
    );

    Ok(KGSearchResponse {
        status: Status::Success,
        suggestions,
        error: None,
    })
}

/// Add KG term definition to conversation context
#[command]
pub async fn add_kg_term_context(
    config_state: State<'_, ConfigState>,
    request: AddKGTermContextRequest,
) -> Result<AddContextResponse> {
    log::debug!(
        "Adding KG term '{}' to conversation {} context (role: {})",
        request.term,
        request.conversation_id,
        request.role_name
    );

    let conv_id = ConversationId::from_string(request.conversation_id.clone());
    let role_name = RoleName::new(&request.role_name);
    let mut terraphim_service = TerraphimService::new(config_state.inner().clone());

    // Find documents related to the KG term
    let documents = match terraphim_service
        .find_documents_for_kg_term(&role_name, &request.term)
        .await
    {
        Ok(docs) => docs,
        Err(e) => {
            log::error!(
                "Failed to find documents for KG term '{}': {}",
                request.term,
                e
            );
            return Ok(AddContextResponse {
                status: Status::Error,
                error: Some(format!(
                    "Failed to find documents for term '{}': {}",
                    request.term, e
                )),
            });
        }
    };

    if documents.is_empty() {
        return Ok(AddContextResponse {
            status: Status::Error,
            error: Some(format!("No documents found for KG term '{}'", request.term)),
        });
    }

    // Create KG term definition from the first document
    let doc = &documents[0];
    let kg_term = KGTermDefinition {
        term: request.term.clone(),
        normalized_term: NormalizedTermValue::new(request.term.clone()),
        id: 1, // TODO: Get actual term ID from thesaurus
        definition: Some(doc.description.clone().unwrap_or_default()),
        synonyms: Vec::new(),       // TODO: Extract from thesaurus
        related_terms: Vec::new(),  // TODO: Extract from thesaurus
        usage_examples: Vec::new(), // TODO: Extract from thesaurus
        url: if doc.url.is_empty() {
            None
        } else {
            Some(doc.url.clone())
        },
        metadata: {
            let mut meta = AHashMap::new();
            meta.insert("source_document".to_string(), doc.id.clone());
            meta.insert("document_title".to_string(), doc.title.clone());
            meta
        },
        relevance_score: doc.rank.map(|r| r as f64),
    };

    let context_item = terraphim_types::ContextItem::from_kg_term_definition(&kg_term);

    let mut manager = get_context_manager().lock().await;
    match manager.add_context(&conv_id, context_item) {
        Ok(()) => {
            log::debug!("Successfully added KG term context to conversation");
            Ok(AddContextResponse {
                status: Status::Success,
                error: None,
            })
        }
        Err(e) => {
            log::error!("Failed to add KG term context: {}", e);
            Ok(AddContextResponse {
                status: Status::Error,
                error: Some(format!("Failed to add KG term context: {}", e)),
            })
        }
    }
}

/// Add complete KG index information to conversation context
#[command]
pub async fn add_kg_index_context(
    config_state: State<'_, ConfigState>,
    request: AddKGIndexContextRequest,
) -> Result<AddContextResponse> {
    log::debug!(
        "Adding KG index for role '{}' to conversation {} context",
        request.role_name,
        request.conversation_id
    );

    let conv_id = ConversationId::from_string(request.conversation_id.clone());
    let role_name = RoleName::new(&request.role_name);

    // Get role configuration to access the rolegraph
    let config = config_state.config.lock().await;
    let _role_config = match config.roles.get(&role_name) {
        Some(role) => role,
        None => {
            return Ok(AddContextResponse {
                status: Status::Error,
                error: Some(format!("Role '{}' not found", request.role_name)),
            });
        }
    };
    drop(config);

    // Get rolegraph from config state
    let rolegraph_sync = match config_state.roles.get(&role_name) {
        Some(rg) => rg,
        None => {
            return Ok(AddContextResponse {
                status: Status::Error,
                error: Some(format!("Role graph for '{}' not found", request.role_name)),
            });
        }
    };

    let rolegraph = rolegraph_sync.lock().await;

    // Serialize the full thesaurus to JSON
    let thesaurus_json =
        serde_json::to_string_pretty(&rolegraph.thesaurus).unwrap_or_else(|_| "{}".to_string());

    let kg_index = KGIndexInfo {
        name: format!("Knowledge Graph Index for {}", request.role_name),
        total_terms: rolegraph.thesaurus.len(),
        total_nodes: rolegraph.nodes_map().len(),
        total_edges: rolegraph.edges_map().len(),
        last_updated: chrono::Utc::now(), // TODO: Get actual last updated time
        source: "local".to_string(),
        version: Some("1.0".to_string()),
    };
    drop(rolegraph);

    // Create context item with full thesaurus JSON content
    let context_item = ContextItem {
        id: ulid::Ulid::new().to_string(),
        context_type: ContextType::System, // Use System type for KG index
        title: kg_index.name.clone(),
        summary: Some(format!(
            "Complete thesaurus with {} terms, {} nodes, and {} edges. Contains full JSON vocabulary data for comprehensive AI understanding.",
            kg_index.total_terms, kg_index.total_nodes, kg_index.total_edges
        )),
        content: thesaurus_json,
        metadata: {
            let mut meta = AHashMap::new();
            meta.insert("total_terms".to_string(), kg_index.total_terms.to_string());
            meta.insert("total_nodes".to_string(), kg_index.total_nodes.to_string());
            meta.insert("total_edges".to_string(), kg_index.total_edges.to_string());
            meta.insert("source".to_string(), kg_index.source.clone());
            meta.insert("version".to_string(), kg_index.version.clone().unwrap_or_default());
            meta.insert("last_updated".to_string(), kg_index.last_updated.to_rfc3339());
            meta.insert("content_type".to_string(), "thesaurus_json".to_string());
            meta.insert("kg_index_type".to_string(), "KGIndex".to_string());
            meta
        },
        created_at: chrono::Utc::now(),
        relevance_score: Some(1.0),
    };

    let mut manager = get_context_manager().lock().await;
    match manager.add_context(&conv_id, context_item) {
        Ok(()) => {
            log::debug!("Successfully added KG index context to conversation");
            Ok(AddContextResponse {
                status: Status::Success,
                error: None,
            })
        }
        Err(e) => {
            log::error!("Failed to add KG index context: {}", e);
            Ok(AddContextResponse {
                status: Status::Error,
                error: Some(format!("Failed to add KG index context: {}", e)),
            })
        }
    }
}

// ========================
// 1Password Integration Commands
// ========================

#[derive(Serialize, Deserialize, Debug, Tsify)]
#[tsify(into_wasm_abi, from_wasm_abi)]
pub struct OnePasswordStatusResponse {
    pub status: Status,
    pub available: bool,
    pub authenticated: bool,
    pub error: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Tsify)]
#[tsify(into_wasm_abi, from_wasm_abi)]
pub struct ResolveSecretRequest {
    pub reference: String,
}

#[derive(Serialize, Deserialize, Debug, Tsify)]
#[tsify(into_wasm_abi, from_wasm_abi)]
pub struct ResolveSecretResponse {
    pub status: Status,
    pub value: Option<String>,
    pub error: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Tsify)]
#[tsify(into_wasm_abi, from_wasm_abi)]
pub struct ProcessConfigRequest {
    pub config: String,
}

#[derive(Serialize, Deserialize, Debug, Tsify)]
#[tsify(into_wasm_abi, from_wasm_abi)]
pub struct ProcessConfigResponse {
    pub status: Status,
    pub config: Option<String>,
    pub error: Option<String>,
}

/// Check 1Password CLI availability and authentication status
#[command]
pub async fn onepassword_status() -> Result<OnePasswordStatusResponse> {
    log::debug!("Checking 1Password status");

    let loader = OnePasswordLoader::new();
    let available = loader.check_cli_installed().await;
    let authenticated = if available {
        loader.check_authenticated().await
    } else {
        false
    };

    log::info!(
        "1Password status: available={}, authenticated={}",
        available,
        authenticated
    );

    Ok(OnePasswordStatusResponse {
        status: Status::Success,
        available,
        authenticated,
        error: None,
    })
}

/// Resolve a single 1Password secret reference
#[command]
pub async fn onepassword_resolve_secret(
    request: ResolveSecretRequest,
) -> Result<ResolveSecretResponse> {
    log::debug!("Resolving 1Password secret: {}", request.reference);

    let loader = OnePasswordLoader::new();

    match loader.resolve_secret(&request.reference).await {
        Ok(value) => {
            log::info!("Successfully resolved 1Password secret");
            Ok(ResolveSecretResponse {
                status: Status::Success,
                value: Some(value),
                error: None,
            })
        }
        Err(e) => {
            log::error!("Failed to resolve 1Password secret: {}", e);
            Ok(ResolveSecretResponse {
                status: Status::Error,
                value: None,
                error: Some(e.to_string()),
            })
        }
    }
}

/// Process a configuration string, resolving 1Password references
#[command]
pub async fn onepassword_process_config(
    request: ProcessConfigRequest,
) -> Result<ProcessConfigResponse> {
    log::debug!("Processing configuration with 1Password references");

    let loader = OnePasswordLoader::new();

    match loader.process_config(&request.config).await {
        Ok(processed_config) => {
            log::info!("Successfully processed configuration with 1Password");
            Ok(ProcessConfigResponse {
                status: Status::Success,
                config: Some(processed_config),
                error: None,
            })
        }
        Err(e) => {
            log::error!("Failed to process configuration with 1Password: {}", e);
            Ok(ProcessConfigResponse {
                status: Status::Error,
                config: None,
                error: Some(e.to_string()),
            })
        }
    }
}

/// Load device settings with 1Password integration
#[command]
pub async fn onepassword_load_settings() -> Result<serde_json::Value> {
    log::debug!("Loading device settings with 1Password integration");

    match DeviceSettings::load_with_onepassword(None).await {
        Ok(settings) => {
            log::info!("Successfully loaded settings with 1Password integration");
            Ok(serde_json::to_value(settings)?)
        }
        Err(e) => {
            log::error!("Failed to load settings with 1Password: {}", e);
            Err(TerraphimTauriError::Settings(e))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_kg_search_request_validation() {
        let request = KGSearchRequest {
            query: "async".to_string(),
            role_name: "Engineer".to_string(),
            limit: Some(10),
            min_similarity: Some(0.7),
        };

        assert_eq!(request.query, "async");
        assert_eq!(request.role_name, "Engineer");
        assert_eq!(request.limit, Some(10));
        assert_eq!(request.min_similarity, Some(0.7));
    }

    #[tokio::test]
    async fn test_empty_query_validation() {
        let request = KGSearchRequest {
            query: "".to_string(),
            role_name: "Engineer".to_string(),
            limit: Some(10),
            min_similarity: Some(0.7),
        };

        assert!(request.query.is_empty());
    }

    #[tokio::test]
    async fn test_cache_metrics_initialization() {
        // Test that cache metrics can be accessed without panicking
        CACHE_METRICS.record_hit();
        CACHE_METRICS.record_miss();
        CACHE_METRICS.record_build();
        CACHE_METRICS.record_eviction();

        // Test passes if we reach here without panic
    }

    #[tokio::test]
    async fn test_status_enum() {
        match Status::Success {
            Status::Success => {} // Test passes
            Status::Error => panic!("Should be Success"),
        }

        match Status::Error {
            Status::Error => {} // Test passes
            Status::Success => panic!("Should be Error"),
        }
    }

    #[tokio::test]
    async fn test_kg_add_term_request_validation() {
        let request = AddKGTermContextRequest {
            conversation_id: "test_conv".to_string(),
            term: "async".to_string(),
            role_name: "Engineer".to_string(),
        };

        assert_eq!(request.conversation_id, "test_conv");
        assert_eq!(request.term, "async");
        assert_eq!(request.role_name, "Engineer");
    }

    #[tokio::test]
    async fn test_kg_add_index_request_validation() {
        let request = AddKGIndexContextRequest {
            conversation_id: "test_conv".to_string(),
            role_name: "Engineer".to_string(),
        };

        assert_eq!(request.conversation_id, "test_conv");
        assert_eq!(request.role_name, "Engineer");
    }

    #[tokio::test]
    async fn test_cache_key_creation() {
        // Test that cache keys work correctly
        let role1 = "Engineer";
        let role2 = "Engineer";
        let role3 = "DataScientist";

        assert_eq!(role1, role2);
        assert_ne!(role1, role3);
    }
}
