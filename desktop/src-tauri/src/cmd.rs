use tauri::command;
use tauri::State;

use serde::{Deserialize, Serialize};

use terraphim_atomic_client::{Agent, Config as AtomicConfig, Store};
use terraphim_config::{Config, ConfigState};
use terraphim_rolegraph::magic_unpair;
use terraphim_service::TerraphimService;
use terraphim_settings::DeviceSettings;
use terraphim_types::Thesaurus;
use terraphim_types::{Document, SearchQuery};

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

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub status: Status,
    pub message: String,
}

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
#[command]
pub async fn select_role(
    app_handle: tauri::AppHandle,
    config_state: State<'_, ConfigState>,
    role_name: String,
) -> Result<ConfigResponse> {
    log::info!("Select role called: {}", role_name);
    let terraphim_service = TerraphimService::new(config_state.inner().clone());
    let config = terraphim_service
        .update_selected_role(terraphim_types::RoleName::new(&role_name))
        .await?;

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
