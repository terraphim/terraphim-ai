use axum::{
    extract::{Query, State, Path},
    http::StatusCode,
    response::IntoResponse,
    Extension, Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::broadcast::Sender;
use tokio::sync::Mutex;
use schemars::schema_for;
use serde_json::Value;

use terraphim_config::Config;
use terraphim_config::ConfigState;
use terraphim_persistence::Persistable;
use terraphim_rolegraph::RoleGraph;
use terraphim_service::TerraphimService;
use terraphim_types::{Document, IndexedDocument, SearchQuery};
use terraphim_rolegraph::magic_unpair;
use terraphim_types::RoleName;

use crate::error::{Result, Status};
pub type SearchResultsStream = Sender<IndexedDocument>;

/// Health check endpoint
pub(crate) async fn health() -> impl IntoResponse {
    (StatusCode::OK, "OK")
}

/// Response for creating a document
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CreateDocumentResponse {
    /// Status of the document creation
    pub status: Status,
    /// The id of the document that was successfully created
    pub id: String,
}

/// Creates index of the document for each rolegraph
pub(crate) async fn create_document(
    State(config): State<ConfigState>,
    Json(document): Json<Document>,
) -> Result<Json<CreateDocumentResponse>> {
    log::debug!("create_document");
    let mut terraphim_service = TerraphimService::new(config.clone());
    let document = terraphim_service.create_document(document).await?;
    Ok(Json(CreateDocumentResponse {
        status: Status::Success,
        id: document.id,
    }))
}

// TODO: Is this still needed now that we have search?
pub(crate) async fn _list_documents(
    State(rolegraph): State<Arc<Mutex<RoleGraph>>>,
) -> impl IntoResponse {
    let rolegraph = rolegraph.lock().await.clone();
    log::debug!("{rolegraph:?}");

    (StatusCode::OK, Json("Ok"))
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SearchResponse {
    /// Status of the search
    pub status: Status,
    /// Vector of results which matched the query
    pub results: Vec<Document>,
    /// The number of documents that match the search query
    pub total: usize,
}

/// Search for documents in all Terraphim graphs defined in the config via GET params
pub(crate) async fn search_documents(
    Extension(_tx): Extension<SearchResultsStream>,
    State(config_state): State<ConfigState>,
    search_query: Query<SearchQuery>,
) -> Result<Json<SearchResponse>> {
    log::debug!("search_document called with {:?}", search_query);

    let mut terraphim_service = TerraphimService::new(config_state);
    let results = terraphim_service.search(&search_query.0).await?;
    let total = results.len();

    Ok(Json(SearchResponse {
        status: Status::Success,
        results,
        total,
    }))
}

/// Search for documents in all Terraphim graphs defined in the config via POST body
pub(crate) async fn search_documents_post(
    Extension(_tx): Extension<SearchResultsStream>,
    State(config_state): State<ConfigState>,
    search_query: Json<SearchQuery>,
) -> Result<Json<SearchResponse>> {
    log::debug!("POST Searching documents with query: {search_query:?}");

    let mut terraphim_service = TerraphimService::new(config_state);
    let results = terraphim_service.search(&search_query).await?;
    let total = results.len();

    if total == 0 {
        log::debug!("No documents found");
    } else {
        log::debug!("Found {total} documents");
    }

    Ok(Json(SearchResponse {
        status: Status::Success,
        results,
        total,
    }))
}

/// Response type for showing the config
///
/// This is also used when updating the config
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ConfigResponse {
    /// Status of the config fetch
    pub status: Status,
    /// The config
    pub config: Config,
}

/// API handler for Terraphim Config
pub(crate) async fn get_config(State(config): State<ConfigState>) -> Result<Json<ConfigResponse>> {
    log::debug!("Called API endpoint get_config");
    let terraphim_service = TerraphimService::new(config);
    let config = terraphim_service.fetch_config().await;
    Ok(Json(ConfigResponse {
        status: Status::Success,
        config,
    }))
}

/// API handler for Terraphim Config update
/// 
/// This function updates the configuration both in-memory and persists it to disk
/// so that the changes survive server restarts.
pub(crate) async fn update_config(
    State(config_state): State<ConfigState>,
    Json(config_new): Json<Config>,
) -> Result<Json<ConfigResponse>> {
    log::info!("Updating configuration and persisting to disk");
    
    // Update in-memory configuration
    let mut config = config_state.config.lock().await;
    *config = config_new.clone();
    drop(config); // Release the lock before async save operation
    
    // Persist the configuration to disk
    match config_new.save().await {
        Ok(()) => {
            log::info!("Configuration successfully updated and persisted");
            Ok(Json(ConfigResponse {
                status: Status::Success,
                config: config_new,
            }))
        }
        Err(e) => {
            log::error!("Failed to persist configuration: {:?}", e);
            // The configuration was updated in memory but not persisted
            // This is still partially successful, so we return the new config
            // but log the persistence error
            Ok(Json(ConfigResponse {
                status: Status::Success,
                config: config_new,
            }))
        }
    }
}

/// Returns JSON Schema for Terraphim Config
pub(crate) async fn get_config_schema() -> Json<Value> {
    let schema = schema_for!(Config);
    Json(serde_json::to_value(&schema).expect("schema serialization"))
}

/// Request body for updating the selected role only
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SelectedRoleRequest {
    pub selected_role: terraphim_types::RoleName,
}

/// Update only the selected role without replacing the whole config
pub(crate) async fn update_selected_role(
    State(config_state): State<ConfigState>,
    Json(payload): Json<SelectedRoleRequest>,
) -> Result<Json<ConfigResponse>> {
    let terraphim_service = TerraphimService::new(config_state.clone());
    let config = terraphim_service
        .update_selected_role(payload.selected_role)
        .await?;

    Ok(Json(ConfigResponse {
        status: Status::Success,
        config,
    }))
}

// NOTE: RoleGraph visualisation DTOs
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GraphNodeDto {
    pub id: u64,
    pub label: String,
    pub rank: u64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GraphEdgeDto {
    pub source: u64,
    pub target: u64,
    pub rank: u64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RoleGraphResponseDto {
    pub status: Status,
    pub nodes: Vec<GraphNodeDto>,
    pub edges: Vec<GraphEdgeDto>,
}

#[derive(Debug, Deserialize)]
pub struct RoleGraphQuery {
    role: Option<String>,
}

/// Return nodes and edges for the RoleGraph of the requested role (or currently selected role if omitted)
pub(crate) async fn get_rolegraph(
    State(config_state): State<ConfigState>,
    Query(query): Query<RoleGraphQuery>,
) -> Result<Json<RoleGraphResponseDto>> {
    // Determine which role we should use
    let role_name: RoleName = if let Some(role_str) = query.role {
        RoleName::new(&role_str)
    } else {
        config_state.get_selected_role().await
    };

    // Retrieve the rolegraph for the role
    let Some(rolegraph_sync) = config_state.roles.get(&role_name) else {
        return Err(crate::error::ApiError(
            StatusCode::NOT_FOUND,
            anyhow::anyhow!(format!("Rolegraph not found for role: {role_name}")),
        ));
    };

    let rolegraph = rolegraph_sync.lock().await;

    // Build node DTOs
    let nodes: Vec<GraphNodeDto> = rolegraph
        .nodes_map()
        .iter()
        .map(|(&id, node)| {
            let label = rolegraph
                .ac_reverse_nterm
                .get(&id)
                .map(|v| v.as_str().to_string())
                .unwrap_or_else(|| id.to_string());
            GraphNodeDto {
                id,
                label,
                rank: node.rank,
            }
        })
        .collect();

    // Build edge DTOs
    let edges: Vec<GraphEdgeDto> = rolegraph
        .edges_map()
        .iter()
        .map(|(&edge_id, edge)| {
            let (source, target) = magic_unpair(edge_id);
            GraphEdgeDto {
                source,
                target,
                rank: edge.rank,
            }
        })
        .collect();

    Ok(Json(RoleGraphResponseDto {
        status: Status::Success,
        nodes,
        edges,
    }))
}

/// Query parameters for KG term search
#[derive(Debug, Deserialize)]
pub struct KgSearchQuery {
    /// The knowledge graph term to search for
    pub term: String,
}

/// Find documents that contain a given knowledge graph term
/// 
/// This endpoint searches for documents that were the source of a knowledge graph term.
/// For example, given "haystack", it will find documents like "haystack.md" that contain
/// this term or its synonyms ("datasource", "service", "agent").
pub(crate) async fn find_documents_by_kg_term(
    State(config_state): State<ConfigState>,
    axum::extract::Path(role_name): axum::extract::Path<String>,
    Query(query): Query<KgSearchQuery>,
) -> Result<Json<SearchResponse>> {
    log::debug!("Finding documents for KG term '{}' in role '{}'", query.term, role_name);
    
    let role_name = RoleName::new(&role_name);
    let mut terraphim_service = TerraphimService::new(config_state);
    
    let results = terraphim_service.find_documents_for_kg_term(&role_name, &query.term).await?;
    let total = results.len();
    
    log::debug!("Found {} documents for KG term '{}'", total, query.term);
    
    Ok(Json(SearchResponse {
        status: Status::Success,
        results,
        total,
    }))
}

/// Request for document summarization
#[derive(Debug, Deserialize)]
pub struct SummarizeDocumentRequest {
    /// Document ID to summarize
    pub document_id: String,
    /// Role to use for summarization (determines OpenRouter configuration)
    pub role: String,
    /// Optional: Override max summary length (default: 250 characters)
    pub max_length: Option<usize>,
    /// Optional: Force regeneration even if summary exists
    pub force_regenerate: Option<bool>,
}

/// Response for document summarization
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SummarizeDocumentResponse {
    /// Status of the summarization request
    pub status: Status,
    /// The document ID that was summarized
    pub document_id: String,
    /// The generated summary
    pub summary: Option<String>,
    /// The OpenRouter model used for summarization
    pub model_used: Option<String>,
    /// Whether this summary was newly generated or retrieved from cache
    pub from_cache: bool,
    /// Error message if summarization failed
    pub error: Option<String>,
}

/// Query parameters for summarization status
#[derive(Debug, Deserialize)]
pub struct SummarizationStatusQuery {
    /// Role to check summarization status for
    pub role: String,
}

/// Response for summarization status
#[derive(Debug, Serialize, Clone)]
pub struct SummarizationStatusResponse {
    /// Status of the request
    pub status: Status,
    /// Whether OpenRouter is enabled for this role
    pub openrouter_enabled: bool,
    /// Whether OpenRouter is properly configured for this role
    pub openrouter_configured: bool,
    /// The model that would be used for summarization
    pub model: Option<String>,
    /// Number of documents with existing summaries for this role
    pub cached_summaries_count: u32,
}

/// Generate or retrieve a summary for a document using OpenRouter
/// 
/// This endpoint generates AI-powered summaries for documents using the OpenRouter service.
/// It requires the role to have OpenRouter properly configured (enabled, API key, model).
/// Summaries are cached in the persistence layer to avoid redundant API calls.
pub(crate) async fn summarize_document(
    State(config_state): State<ConfigState>,
    Json(request): Json<SummarizeDocumentRequest>,
) -> Result<Json<SummarizeDocumentResponse>> {
    log::debug!("Summarizing document '{}' with role '{}'", request.document_id, request.role);
    
    let role_name = RoleName::new(&request.role);
    let config = config_state.config.lock().await;
    
    // Get the role configuration
    let Some(role) = config.roles.get(&role_name) else {
        return Ok(Json(SummarizeDocumentResponse {
            status: Status::Error,
            document_id: request.document_id,
            summary: None,
            model_used: None,
            from_cache: false,
            error: Some(format!("Role '{}' not found", request.role)),
        }));
    };
    
    // Check if OpenRouter is enabled and configured for this role
    #[cfg(feature = "openrouter")]
    {
        if !role.has_openrouter_config() {
            return Ok(Json(SummarizeDocumentResponse {
                status: Status::Error,
                document_id: request.document_id,
                summary: None,
                model_used: None,
                from_cache: false,
                error: Some("OpenRouter not properly configured for this role".to_string()),
            }));
        }
        
        // Get the API key from environment variable if not set in role
        let api_key = if let Some(key) = &role.openrouter_api_key {
            key.clone()
        } else {
            std::env::var("OPENROUTER_KEY").map_err(|_| {
                crate::error::ApiError(
                    StatusCode::BAD_REQUEST,
                    anyhow::anyhow!("OpenRouter API key not found in role configuration or OPENROUTER_KEY environment variable"),
                )
            })?
        };
        
        let model = role.openrouter_model.as_deref().unwrap_or("openai/gpt-3.5-turbo");
        let max_length = request.max_length.unwrap_or(250);
        let force_regenerate = request.force_regenerate.unwrap_or(false);
        
        drop(config); // Release the lock before async operations
        
        let mut terraphim_service = TerraphimService::new(config_state);
        
        // Try to load existing document first
        let document = match terraphim_service.get_document_by_id(&request.document_id, &role_name).await {
            Ok(doc) => doc,
            Err(e) => {
                log::error!("Failed to load document '{}': {:?}", request.document_id, e);
                return Ok(Json(SummarizeDocumentResponse {
                    status: Status::Error,
                    document_id: request.document_id,
                    summary: None,
                    model_used: None,
                    from_cache: false,
                    error: Some(format!("Document not found: {}", e)),
                }));
            }
        };
        
        // Check if we already have a summary and don't need to regenerate
        if !force_regenerate {
            if let Some(existing_summary) = &document.description {
                if !existing_summary.trim().is_empty() && existing_summary.len() >= 50 {
                    log::debug!("Using cached summary for document '{}'", request.document_id);
                    return Ok(Json(SummarizeDocumentResponse {
                        status: Status::Success,
                        document_id: request.document_id,
                        summary: Some(existing_summary.clone()),
                        model_used: Some(model.to_string()),
                        from_cache: true,
                        error: None,
                    }));
                }
            }
        }
        
        // Generate new summary using OpenRouter
        match terraphim_service.generate_document_summary(&document, &api_key, model, max_length).await {
            Ok(summary) => {
                log::info!("Generated summary for document '{}' using model '{}'", request.document_id, model);
                
                // Save the updated document with the new summary
                let mut updated_doc = document.clone();
                updated_doc.description = Some(summary.clone());
                
                if let Err(e) = updated_doc.save().await {
                    log::error!("Failed to save summary for document '{}': {:?}", request.document_id, e);
                }
                
                Ok(Json(SummarizeDocumentResponse {
                    status: Status::Success,
                    document_id: request.document_id,
                    summary: Some(summary),
                    model_used: Some(model.to_string()),
                    from_cache: false,
                    error: None,
                }))
            }
            Err(e) => {
                log::error!("Failed to generate summary for document '{}': {:?}", request.document_id, e);
                Ok(Json(SummarizeDocumentResponse {
                    status: Status::Error,
                    document_id: request.document_id,
                    summary: None,
                    model_used: Some(model.to_string()),
                    from_cache: false,
                    error: Some(format!("Summarization failed: {}", e)),
                }))
            }
        }
    }
    
    #[cfg(not(feature = "openrouter"))]
    {
        Ok(Json(SummarizeDocumentResponse {
            status: Status::Error,
            document_id: request.document_id,
            summary: None,
            model_used: None,
            from_cache: false,
            error: Some("OpenRouter feature not enabled during compilation".to_string()),
        }))
    }
}

/// Check summarization status and capabilities for a role
pub(crate) async fn get_summarization_status(
    State(config_state): State<ConfigState>,
    Query(query): Query<SummarizationStatusQuery>,
) -> Result<Json<SummarizationStatusResponse>> {
    let role_name = RoleName::new(&query.role);
    let config = config_state.config.lock().await;
    
    let Some(role) = config.roles.get(&role_name) else {
        return Err(crate::error::ApiError(
            StatusCode::NOT_FOUND,
            anyhow::anyhow!(format!("Role '{}' not found", query.role)),
        ));
    };
    
    #[cfg(feature = "openrouter")]
    {
        let openrouter_enabled = role.openrouter_enabled;
        let openrouter_configured = role.has_openrouter_config() || std::env::var("OPENROUTER_KEY").is_ok();
        let model = role.get_openrouter_model().map(|s| s.to_string());
        
        // Note: For now, we'll set cached_summaries_count to 0
        // In the future, this could query the persistence layer for documents with summaries
        let cached_summaries_count = 0;
        
        Ok(Json(SummarizationStatusResponse {
            status: Status::Success,
            openrouter_enabled,
            openrouter_configured,
            model,
            cached_summaries_count,
        }))
    }
    
    #[cfg(not(feature = "openrouter"))]
    {
        Ok(Json(SummarizationStatusResponse {
            status: Status::Success,
            openrouter_enabled: false,
            openrouter_configured: false,
            model: None,
            cached_summaries_count: 0,
        }))
    }
}
