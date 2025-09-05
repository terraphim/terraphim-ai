use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Extension, Json,
};
use schemars::schema_for;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::broadcast::Sender;
use tokio::sync::Mutex;

use terraphim_config::Config;
use terraphim_config::ConfigState;
use terraphim_persistence::Persistable;
use terraphim_rolegraph::magic_unpair;
use terraphim_rolegraph::RoleGraph;
use terraphim_service::TerraphimService;
use terraphim_types::RoleName;
use terraphim_types::{Document, IndexedDocument, SearchQuery};

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
    log::debug!(
        "Finding documents for KG term '{}' in role '{}'",
        query.term,
        role_name
    );

    let role_name = RoleName::new(&role_name);
    let mut terraphim_service = TerraphimService::new(config_state);

    let results = terraphim_service
        .find_documents_for_kg_term(&role_name, &query.term)
        .await?;
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
    #[allow(dead_code)]
    pub max_length: Option<usize>,
    /// Optional: Force regeneration even if summary exists
    #[allow(dead_code)]
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

// New async queue API types

/// Request for async document summarization
#[derive(Debug, Deserialize)]
pub struct AsyncSummarizeRequest {
    /// Document ID to summarize
    pub document_id: String,
    /// Role to use for summarization
    pub role: String,
    /// Optional: Priority level (low, normal, high, critical)
    pub priority: Option<String>,
    /// Optional: Override max summary length (default: 250 characters)
    pub max_length: Option<usize>,
    /// Optional: Force regeneration even if summary exists
    pub force_regenerate: Option<bool>,
    /// Optional: Callback URL for completion notification
    pub callback_url: Option<String>,
}

/// Response for async summarization request
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AsyncSummarizeResponse {
    /// Status of the request submission
    pub status: Status,
    /// Task ID for tracking progress
    pub task_id: Option<String>,
    /// Position in queue if successfully queued
    pub position_in_queue: Option<usize>,
    /// Estimated wait time in seconds
    pub estimated_wait_seconds: Option<u64>,
    /// Error message if submission failed
    pub error: Option<String>,
}

/// Request for task status
#[derive(Debug, Deserialize)]
pub struct TaskStatusRequest {
    /// Task ID to check
    #[allow(dead_code)] // Task ID comes from URL path, not request body
    pub task_id: String,
}

/// Response for task status
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TaskStatusResponse {
    /// Status of the request
    pub status: Status,
    /// Task ID
    pub task_id: String,
    /// Current task status
    pub task_status: Option<String>,
    /// Progress percentage (0-100) if processing
    pub progress: Option<f32>,
    /// Result summary if completed
    pub summary: Option<String>,
    /// Error message if failed
    pub error: Option<String>,
    /// Processing duration if completed
    pub processing_duration_ms: Option<u64>,
    /// Next retry time if failed and retryable
    pub next_retry_seconds: Option<u64>,
    /// Retry count
    pub retry_count: Option<u32>,
}

/// Request to cancel a task
#[derive(Debug, Deserialize)]
pub struct CancelTaskRequest {
    /// Task ID to cancel
    #[allow(dead_code)] // Task ID comes from URL path, not request body
    pub task_id: String,
    /// Reason for cancellation
    pub reason: Option<String>,
}

/// Response for task cancellation
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CancelTaskResponse {
    /// Status of the cancellation request
    pub status: Status,
    /// Whether the task was successfully cancelled
    pub cancelled: bool,
    /// Error message if cancellation failed
    pub error: Option<String>,
}

/// Response for queue statistics
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct QueueStatsResponse {
    /// Status of the request
    pub status: Status,
    /// Queue statistics
    pub stats: Option<terraphim_service::summarization_queue::QueueStats>,
    /// Error message if retrieval failed
    pub error: Option<String>,
}

/// Request for batch summarization
#[derive(Debug, Deserialize)]
pub struct BatchSummarizeRequest {
    /// List of documents to summarize
    pub documents: Vec<BatchSummarizeItem>,
    /// Role to use for summarization
    pub role: String,
    /// Priority for all tasks (low, normal, high, critical)
    pub priority: Option<String>,
    /// Optional: Callback URL for batch completion notification
    pub callback_url: Option<String>,
}

/// Single item in batch summarization request
#[derive(Debug, Deserialize)]
pub struct BatchSummarizeItem {
    /// Document ID to summarize
    pub document_id: String,
    /// Optional: Override max summary length
    pub max_length: Option<usize>,
    /// Optional: Force regeneration even if summary exists
    pub force_regenerate: Option<bool>,
}

/// Response for batch summarization
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BatchSummarizeResponse {
    /// Status of the batch request
    pub status: Status,
    /// List of submitted task IDs
    pub task_ids: Vec<String>,
    /// Number of successfully queued tasks
    pub queued_count: usize,
    /// Number of failed submissions
    pub failed_count: usize,
    /// Errors for failed submissions
    pub errors: Vec<String>,
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

/// Chat message exchanged with the assistant
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ChatMessage {
    pub role: String, // "system" | "user" | "assistant"
    pub content: String,
}

/// Chat request payload
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ChatRequest {
    /// Role to use for chat (determines OpenRouter configuration)
    pub role: String,
    /// Conversation so far
    pub messages: Vec<ChatMessage>,
    /// Optional model override
    pub model: Option<String>,
}

/// Chat response payload
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ChatResponse {
    pub status: Status,
    pub message: Option<String>,
    pub model_used: Option<String>,
    pub error: Option<String>,
}

/// Handle chat completion via generic LLM interface (OpenRouter or Ollama)
pub(crate) async fn chat_completion(
    State(config_state): State<ConfigState>,
    Json(request): Json<ChatRequest>,
) -> Result<Json<ChatResponse>> {
    let role_name = RoleName::new(&request.role);
    let config = config_state.config.lock().await;

    let Some(role_ref) = config.roles.get(&role_name) else {
        return Ok(Json(ChatResponse {
            status: Status::Error,
            message: None,
            model_used: None,
            error: Some(format!("Role '{}' not found", request.role)),
        }));
    };

    // Clone role data to use after releasing the lock
    let role = role_ref.clone();

    drop(config);

    // Try to build LLM client from role configuration
    let llm_client = match terraphim_service::llm::build_llm_from_role(&role) {
        Some(client) => client,
        None => {
            return Ok(Json(ChatResponse {
                status: Status::Error,
                message: None,
                model_used: None,
                error: Some("No LLM provider configured for this role. Please configure either OpenRouter or Ollama.".to_string()),
            }));
        }
    };

    let model_name = llm_client.name();

    // Convert API messages to LLM client format
    let mut messages: Vec<terraphim_service::llm::ChatMessage> = Vec::new();

    // Add system prompt if configured (for OpenRouter compatibility)
    #[cfg(feature = "openrouter")]
    {
        if let Some(system) = &role.openrouter_chat_system_prompt {
            messages.push(terraphim_service::llm::ChatMessage {
                role: "system".to_string(),
                content: system.clone(),
            });
        }
    }

    // Add request messages
    for m in request.messages.iter() {
        messages.push(terraphim_service::llm::ChatMessage {
            role: m.role.clone(),
            content: m.content.clone(),
        });
    }

    // Set up chat options
    let chat_opts = terraphim_service::llm::ChatOptions {
        max_tokens: Some(1024),
        temperature: Some(0.7),
    };

    // Call the generic LLM client
    match llm_client.chat_completion(messages, chat_opts).await {
        Ok(reply) => Ok(Json(ChatResponse {
            status: Status::Success,
            message: Some(reply),
            model_used: Some(model_name.to_string()),
            error: None,
        })),
        Err(e) => Ok(Json(ChatResponse {
            status: Status::Error,
            message: None,
            model_used: Some(model_name.to_string()),
            error: Some(format!("Chat completion failed: {}", e)),
        })),
    }
}

/// Verify OpenRouter API key and fetch available models
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OpenRouterModelsRequest {
    pub role: String,
    pub api_key: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OpenRouterModelsResponse {
    pub status: Status,
    pub models: Vec<String>,
    pub error: Option<String>,
}

#[allow(dead_code)]
pub(crate) async fn list_openrouter_models(
    State(config_state): State<ConfigState>,
    Json(req): Json<OpenRouterModelsRequest>,
) -> Result<Json<OpenRouterModelsResponse>> {
    let role_name = RoleName::new(&req.role);
    let config = config_state.config.lock().await;
    let Some(role) = config.roles.get(&role_name) else {
        return Ok(Json(OpenRouterModelsResponse {
            status: Status::Error,
            models: vec![],
            error: Some(format!("Role '{}' not found", req.role)),
        }));
    };

    #[cfg(feature = "openrouter")]
    {
        // Determine API key preference: request -> role -> env
        let api_key = if let Some(k) = &req.api_key {
            k.clone()
        } else if let Some(k) = &role.openrouter_api_key {
            k.clone()
        } else {
            match std::env::var("OPENROUTER_KEY") {
                Ok(v) => v,
                Err(_) => {
                    return Ok(Json(OpenRouterModelsResponse {
                        status: Status::Error,
                        models: vec![],
                        error: Some("Missing OpenRouter API key".to_string()),
                    }))
                }
            }
        };

        // Any valid model string works for constructing the client
        let seed_model = role
            .openrouter_model
            .clone()
            .unwrap_or_else(|| "openai/gpt-3.5-turbo".to_string());

        drop(config);

        use terraphim_service::openrouter::OpenRouterService;
        match OpenRouterService::new(&api_key, &seed_model) {
            Ok(client) => match client.list_models().await {
                Ok(models) => Ok(Json(OpenRouterModelsResponse {
                    status: Status::Success,
                    models,
                    error: None,
                })),
                Err(e) => Ok(Json(OpenRouterModelsResponse {
                    status: Status::Error,
                    models: vec![],
                    error: Some(format!("Failed to list models: {}", e)),
                })),
            },
            Err(e) => Ok(Json(OpenRouterModelsResponse {
                status: Status::Error,
                models: vec![],
                error: Some(format!("Failed to init OpenRouter client: {}", e)),
            })),
        }
    }

    #[cfg(not(feature = "openrouter"))]
    {
        Ok(Json(OpenRouterModelsResponse {
            status: Status::Error,
            models: vec![],
            error: Some("OpenRouter feature not enabled during compilation".to_string()),
        }))
    }
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
    log::debug!(
        "Summarizing document '{}' with role '{}'",
        request.document_id,
        request.role
    );

    let role_name = RoleName::new(&request.role);
    let config = config_state.config.lock().await;

    // Get the role configuration
    let Some(role_ref) = config.roles.get(&role_name) else {
        return Ok(Json(SummarizeDocumentResponse {
            status: Status::Error,
            document_id: request.document_id,
            summary: None,
            model_used: None,
            from_cache: false,
            error: Some(format!("Role '{}' not found", request.role)),
        }));
    };

    // Clone role to use after dropping lock
    let role = role_ref.clone();
    let max_length = request.max_length.unwrap_or(250);
    let force_regenerate = request.force_regenerate.unwrap_or(false);

    drop(config); // Release the lock before async operations

    // Try to build LLM client from role configuration
    let llm_client = match terraphim_service::llm::build_llm_from_role(&role) {
        Some(client) => client,
        None => {
            return Ok(Json(SummarizeDocumentResponse {
                status: Status::Error,
                document_id: request.document_id,
                summary: None,
                model_used: None,
                from_cache: false,
                error: Some("No LLM provider configured for this role. Please configure either OpenRouter or Ollama.".to_string()),
            }));
        }
    };

    let model_name = llm_client.name();

    let mut terraphim_service = TerraphimService::new(config_state);

    // Try to load existing document first
    let document_opt = match terraphim_service
        .get_document_by_id(&request.document_id)
        .await
    {
        Ok(doc_opt) => doc_opt,
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
    let Some(document) = document_opt else {
        return Ok(Json(SummarizeDocumentResponse {
            status: Status::Error,
            document_id: request.document_id,
            summary: None,
            model_used: None,
            from_cache: false,
            error: Some("Document not found".to_string()),
        }));
    };

    // Check if we already have a summary and don't need to regenerate
    if !force_regenerate {
        if let Some(existing_summary) = &document.description {
            if !existing_summary.trim().is_empty() && existing_summary.len() >= 50 {
                log::debug!(
                    "Using cached summary for document '{}'",
                    request.document_id
                );
                return Ok(Json(SummarizeDocumentResponse {
                    status: Status::Success,
                    document_id: request.document_id,
                    summary: Some(existing_summary.clone()),
                    model_used: Some(model_name.to_string()),
                    from_cache: true,
                    error: None,
                }));
            }
        }
    }

    // Generate new summary using the generic LLM client
    match llm_client
        .summarize(
            &document.body,
            terraphim_service::llm::SummarizeOptions { max_length },
        )
        .await
    {
        Ok(summary) => {
            log::info!(
                "Generated summary for document '{}' using provider '{}'",
                request.document_id,
                model_name
            );

            // Save the updated document with the new summary
            let mut updated_doc = document.clone();
            updated_doc.description = Some(summary.clone());

            if let Err(e) = updated_doc.save().await {
                log::error!(
                    "Failed to save summary for document '{}': {:?}",
                    request.document_id,
                    e
                );
            }

            Ok(Json(SummarizeDocumentResponse {
                status: Status::Success,
                document_id: request.document_id,
                summary: Some(summary),
                model_used: Some(model_name.to_string()),
                from_cache: false,
                error: None,
            }))
        }
        Err(e) => {
            log::error!(
                "Failed to generate summary for document '{}': {:?}",
                request.document_id,
                e
            );
            Ok(Json(SummarizeDocumentResponse {
                status: Status::Error,
                document_id: request.document_id,
                summary: None,
                model_used: Some(model_name.to_string()),
                from_cache: false,
                error: Some(format!("Summarization failed: {}", e)),
            }))
        }
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
        let openrouter_configured =
            role.has_openrouter_config() || std::env::var("OPENROUTER_KEY").is_ok();
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

// New Async Queue API Endpoints

/// Submit a document for async summarization
pub(crate) async fn async_summarize_document(
    State(config_state): State<ConfigState>,
    Extension(summarization_manager): Extension<
        Arc<Mutex<terraphim_service::summarization_manager::SummarizationManager>>,
    >,
    Json(request): Json<AsyncSummarizeRequest>,
) -> Result<Json<AsyncSummarizeResponse>> {
    log::debug!(
        "Async summarizing document '{}' with role '{}'",
        request.document_id,
        request.role
    );

    let role_name = RoleName::new(&request.role);
    let config = config_state.config.lock().await;

    // Get the role configuration
    let Some(role_ref) = config.roles.get(&role_name) else {
        return Ok(Json(AsyncSummarizeResponse {
            status: Status::Error,
            task_id: None,
            position_in_queue: None,
            estimated_wait_seconds: None,
            error: Some(format!("Role '{}' not found", request.role)),
        }));
    };

    let role = role_ref.clone();
    drop(config); // Release the lock

    // Load the document
    let mut terraphim_service = TerraphimService::new(config_state);
    let document = match terraphim_service
        .get_document_by_id(&request.document_id)
        .await
    {
        Ok(Some(doc)) => doc,
        Ok(None) => {
            return Ok(Json(AsyncSummarizeResponse {
                status: Status::Error,
                task_id: None,
                position_in_queue: None,
                estimated_wait_seconds: None,
                error: Some("Document not found".to_string()),
            }));
        }
        Err(e) => {
            log::error!("Failed to load document '{}': {:?}", request.document_id, e);
            return Ok(Json(AsyncSummarizeResponse {
                status: Status::Error,
                task_id: None,
                position_in_queue: None,
                estimated_wait_seconds: None,
                error: Some(format!("Failed to load document: {}", e)),
            }));
        }
    };

    // Parse priority
    let priority = match request.priority.as_deref() {
        Some("low") => Some(terraphim_service::summarization_queue::Priority::Low),
        Some("normal") => Some(terraphim_service::summarization_queue::Priority::Normal),
        Some("high") => Some(terraphim_service::summarization_queue::Priority::High),
        Some("critical") => Some(terraphim_service::summarization_queue::Priority::Critical),
        None => None,
        Some(invalid) => {
            return Ok(Json(AsyncSummarizeResponse {
                status: Status::Error,
                task_id: None,
                position_in_queue: None,
                estimated_wait_seconds: None,
                error: Some(format!(
                    "Invalid priority '{}'. Use: low, normal, high, or critical",
                    invalid
                )),
            }));
        }
    };

    // Submit to queue
    let manager = summarization_manager.lock().await;
    match manager
        .summarize_document(
            document,
            role,
            priority,
            request.max_length,
            request.force_regenerate,
            request.callback_url,
        )
        .await
    {
        Ok(result) => match result {
            terraphim_service::summarization_queue::SubmitResult::Queued {
                task_id,
                position_in_queue,
                estimated_wait_time_seconds,
            } => Ok(Json(AsyncSummarizeResponse {
                status: Status::Success,
                task_id: Some(task_id.to_string()),
                position_in_queue: Some(position_in_queue),
                estimated_wait_seconds: estimated_wait_time_seconds,
                error: None,
            })),
            terraphim_service::summarization_queue::SubmitResult::QueueFull => {
                Ok(Json(AsyncSummarizeResponse {
                    status: Status::Error,
                    task_id: None,
                    position_in_queue: None,
                    estimated_wait_seconds: None,
                    error: Some("Queue is full, please try again later".to_string()),
                }))
            }
            terraphim_service::summarization_queue::SubmitResult::Duplicate(existing_id) => {
                Ok(Json(AsyncSummarizeResponse {
                    status: Status::Success,
                    task_id: Some(existing_id.to_string()),
                    position_in_queue: None,
                    estimated_wait_seconds: None,
                    error: Some("Task already exists for this document".to_string()),
                }))
            }
            terraphim_service::summarization_queue::SubmitResult::ValidationError(err) => {
                Ok(Json(AsyncSummarizeResponse {
                    status: Status::Error,
                    task_id: None,
                    position_in_queue: None,
                    estimated_wait_seconds: None,
                    error: Some(err),
                }))
            }
        },
        Err(e) => {
            log::error!("Failed to submit summarization task: {:?}", e);
            Ok(Json(AsyncSummarizeResponse {
                status: Status::Error,
                task_id: None,
                position_in_queue: None,
                estimated_wait_seconds: None,
                error: Some(format!("Failed to submit task: {}", e)),
            }))
        }
    }
}

/// Get the status of a summarization task
pub(crate) async fn get_task_status(
    Extension(summarization_manager): Extension<
        Arc<Mutex<terraphim_service::summarization_manager::SummarizationManager>>,
    >,
    Path(task_id): Path<String>,
) -> Result<Json<TaskStatusResponse>> {
    let task_id = match task_id.parse() {
        Ok(uuid) => terraphim_service::summarization_queue::TaskId(uuid),
        Err(_) => {
            return Ok(Json(TaskStatusResponse {
                status: Status::Error,
                task_id: task_id.clone(),
                task_status: None,
                progress: None,
                summary: None,
                error: Some("Invalid task ID format".to_string()),
                processing_duration_ms: None,
                next_retry_seconds: None,
                retry_count: None,
            }));
        }
    };

    let manager = summarization_manager.lock().await;
    match manager.get_task_status(&task_id).await {
        Some(status) => {
            let (task_status_str, progress, summary, error, duration_ms, next_retry, retry_count) =
                match status {
                    terraphim_service::summarization_queue::TaskStatus::Pending { .. } => {
                        ("pending".to_string(), None, None, None, None, None, None)
                    }
                    terraphim_service::summarization_queue::TaskStatus::Processing {
                        progress,
                        ..
                    } => (
                        "processing".to_string(),
                        progress,
                        None,
                        None,
                        None,
                        None,
                        None,
                    ),
                    terraphim_service::summarization_queue::TaskStatus::Completed {
                        summary,
                        processing_duration_seconds,
                        ..
                    } => (
                        "completed".to_string(),
                        Some(100.0),
                        Some(summary),
                        None,
                        Some(processing_duration_seconds * 1000),
                        None,
                        None,
                    ),
                    terraphim_service::summarization_queue::TaskStatus::Failed {
                        error,
                        retry_count,
                        next_retry_at,
                        ..
                    } => {
                        let next_retry_seconds = next_retry_at.map(|t| {
                            let duration = t.signed_duration_since(chrono::Utc::now());
                            duration.num_seconds().max(0) as u64
                        });
                        (
                            "failed".to_string(),
                            None,
                            None,
                            Some(error),
                            None,
                            next_retry_seconds,
                            Some(retry_count),
                        )
                    }
                    terraphim_service::summarization_queue::TaskStatus::Cancelled {
                        reason,
                        ..
                    } => (
                        "cancelled".to_string(),
                        None,
                        None,
                        Some(reason),
                        None,
                        None,
                        None,
                    ),
                };

            Ok(Json(TaskStatusResponse {
                status: Status::Success,
                task_id: task_id.to_string(),
                task_status: Some(task_status_str),
                progress,
                summary,
                error,
                processing_duration_ms: duration_ms,
                next_retry_seconds: next_retry,
                retry_count,
            }))
        }
        None => Ok(Json(TaskStatusResponse {
            status: Status::Error,
            task_id: task_id.to_string(),
            task_status: None,
            progress: None,
            summary: None,
            error: Some("Task not found".to_string()),
            processing_duration_ms: None,
            next_retry_seconds: None,
            retry_count: None,
        })),
    }
}

/// Cancel a summarization task
pub(crate) async fn cancel_task(
    Extension(summarization_manager): Extension<
        Arc<Mutex<terraphim_service::summarization_manager::SummarizationManager>>,
    >,
    Path(task_id): Path<String>,
    Json(request): Json<CancelTaskRequest>,
) -> Result<Json<CancelTaskResponse>> {
    let task_id = match task_id.parse() {
        Ok(uuid) => terraphim_service::summarization_queue::TaskId(uuid),
        Err(_) => {
            return Ok(Json(CancelTaskResponse {
                status: Status::Error,
                cancelled: false,
                error: Some("Invalid task ID format".to_string()),
            }));
        }
    };

    let manager = summarization_manager.lock().await;
    match manager
        .cancel_task(
            task_id.clone(),
            request
                .reason
                .unwrap_or_else(|| "Cancelled by user".to_string()),
        )
        .await
    {
        Ok(cancelled) => Ok(Json(CancelTaskResponse {
            status: Status::Success,
            cancelled,
            error: None,
        })),
        Err(e) => {
            log::error!("Failed to cancel task {}: {:?}", task_id, e);
            Ok(Json(CancelTaskResponse {
                status: Status::Error,
                cancelled: false,
                error: Some(format!("Failed to cancel task: {}", e)),
            }))
        }
    }
}

/// Get queue statistics
pub(crate) async fn get_queue_stats(
    Extension(summarization_manager): Extension<
        Arc<Mutex<terraphim_service::summarization_manager::SummarizationManager>>,
    >,
) -> Result<Json<QueueStatsResponse>> {
    let manager = summarization_manager.lock().await;
    match manager.get_stats().await {
        Ok(stats) => Ok(Json(QueueStatsResponse {
            status: Status::Success,
            stats: Some(stats),
            error: None,
        })),
        Err(e) => {
            log::error!("Failed to get queue stats: {:?}", e);
            Ok(Json(QueueStatsResponse {
                status: Status::Error,
                stats: None,
                error: Some(format!("Failed to get stats: {}", e)),
            }))
        }
    }
}

/// Submit multiple documents for batch summarization
pub(crate) async fn batch_summarize_documents(
    State(config_state): State<ConfigState>,
    Extension(summarization_manager): Extension<
        Arc<Mutex<terraphim_service::summarization_manager::SummarizationManager>>,
    >,
    Json(request): Json<BatchSummarizeRequest>,
) -> Result<Json<BatchSummarizeResponse>> {
    log::debug!(
        "Batch summarizing {} documents with role '{}'",
        request.documents.len(),
        request.role
    );

    let role_name = RoleName::new(&request.role);
    let config = config_state.config.lock().await;

    // Get the role configuration
    let Some(role_ref) = config.roles.get(&role_name) else {
        return Ok(Json(BatchSummarizeResponse {
            status: Status::Error,
            task_ids: vec![],
            queued_count: 0,
            failed_count: request.documents.len(),
            errors: vec![format!("Role '{}' not found", request.role)],
        }));
    };

    let role = role_ref.clone();
    drop(config); // Release the lock

    // Parse priority
    let priority = match request.priority.as_deref() {
        Some("low") => Some(terraphim_service::summarization_queue::Priority::Low),
        Some("normal") => Some(terraphim_service::summarization_queue::Priority::Normal),
        Some("high") => Some(terraphim_service::summarization_queue::Priority::High),
        Some("critical") => Some(terraphim_service::summarization_queue::Priority::Critical),
        None => None,
        Some(invalid) => {
            return Ok(Json(BatchSummarizeResponse {
                status: Status::Error,
                task_ids: vec![],
                queued_count: 0,
                failed_count: request.documents.len(),
                errors: vec![format!(
                    "Invalid priority '{}'. Use: low, normal, high, or critical",
                    invalid
                )],
            }));
        }
    };

    let mut terraphim_service = TerraphimService::new(config_state);
    let manager = summarization_manager.lock().await;

    let mut task_ids = Vec::new();
    let mut errors = Vec::new();
    let mut queued_count = 0;
    let mut failed_count = 0;

    for item in request.documents.iter() {
        // Load the document
        let document = match terraphim_service
            .get_document_by_id(&item.document_id)
            .await
        {
            Ok(Some(doc)) => doc,
            Ok(None) => {
                errors.push(format!("Document {} not found", item.document_id));
                failed_count += 1;
                continue;
            }
            Err(e) => {
                log::error!("Failed to load document '{}': {:?}", item.document_id, e);
                errors.push(format!(
                    "Failed to load document {}: {}",
                    item.document_id, e
                ));
                failed_count += 1;
                continue;
            }
        };

        // Submit to queue
        match manager
            .summarize_document(
                document,
                role.clone(),
                priority.clone(),
                item.max_length,
                item.force_regenerate,
                request.callback_url.clone(),
            )
            .await
        {
            Ok(result) => {
                match result {
                    terraphim_service::summarization_queue::SubmitResult::Queued {
                        task_id,
                        ..
                    } => {
                        task_ids.push(task_id.to_string());
                        queued_count += 1;
                    }
                    terraphim_service::summarization_queue::SubmitResult::Duplicate(
                        existing_id,
                    ) => {
                        task_ids.push(existing_id.to_string());
                        queued_count += 1; // Count as success for batch
                    }
                    terraphim_service::summarization_queue::SubmitResult::QueueFull => {
                        errors.push(format!("Queue full for document {}", item.document_id));
                        failed_count += 1;
                    }
                    terraphim_service::summarization_queue::SubmitResult::ValidationError(err) => {
                        errors.push(format!(
                            "Validation error for document {}: {}",
                            item.document_id, err
                        ));
                        failed_count += 1;
                    }
                }
            }
            Err(e) => {
                log::error!(
                    "Failed to submit task for document '{}': {:?}",
                    item.document_id,
                    e
                );
                errors.push(format!(
                    "Failed to submit task for document {}: {}",
                    item.document_id, e
                ));
                failed_count += 1;
            }
        }
    }

    Ok(Json(BatchSummarizeResponse {
        status: if failed_count == 0 {
            Status::Success
        } else {
            Status::PartialSuccess
        },
        task_ids,
        queued_count,
        failed_count,
        errors,
    }))
}

/// Response for thesaurus endpoint
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ThesaurusResponse {
    /// Status of the request
    pub status: Status,
    /// The thesaurus data as a hash map of normalized terms
    pub thesaurus: Option<std::collections::HashMap<String, String>>,
    /// Error message if retrieval failed
    pub error: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct AutocompleteResponse {
    /// Status of the request
    pub status: Status,
    /// Autocomplete suggestions from FST search
    pub suggestions: Vec<AutocompleteSuggestion>,
    /// Error message if search failed
    pub error: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct AutocompleteSuggestion {
    /// The suggested term
    pub term: String,
    /// Alternative text property for TipTap compatibility
    #[serde(alias = "text")]
    pub text: String,
    /// Normalized term value for search
    pub normalized_term: String,
    /// URL associated with the term
    pub url: Option<String>,
    /// Snippet/description for the term
    #[serde(skip_serializing_if = "Option::is_none")]
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

/// Get thesaurus for a specific role
///
/// This endpoint returns the thesaurus (concept mappings) for a given role,
/// which is used for search bar autocomplete functionality in the UI.
pub(crate) async fn get_thesaurus(
    State(config_state): State<ConfigState>,
    Path(role_name): Path<String>,
) -> Result<Json<ThesaurusResponse>> {
    log::debug!("Getting thesaurus for role '{}'", role_name);

    let role_name = RoleName::new(&role_name);

    // Get the role graph for the specified role
    let Some(rolegraph_sync) = config_state.roles.get(&role_name) else {
        return Ok(Json(ThesaurusResponse {
            status: Status::Error,
            thesaurus: None,
            error: Some(format!("Role '{}' not found", role_name)),
        }));
    };

    let rolegraph = rolegraph_sync.lock().await;

    // Convert the thesaurus to a simple HashMap<String, String> format
    // that matches what the UI expects
    let mut thesaurus_map = std::collections::HashMap::new();

    for (key, value) in &rolegraph.thesaurus {
        thesaurus_map.insert(key.as_str().to_string(), value.value.as_str().to_string());
    }

    log::debug!(
        "Found {} thesaurus entries for role '{}'",
        thesaurus_map.len(),
        role_name
    );

    Ok(Json(ThesaurusResponse {
        status: Status::Success,
        thesaurus: Some(thesaurus_map),
        error: None,
    }))
}

/// FST-based autocomplete for a specific role and query
///
/// This endpoint uses the Finite State Transducer (FST) from terraphim_automata
/// to provide fast, intelligent autocomplete suggestions with fuzzy matching.
pub(crate) async fn get_autocomplete(
    State(config_state): State<ConfigState>,
    Path((role_name, query)): Path<(String, String)>,
) -> Result<Json<AutocompleteResponse>> {
    use terraphim_automata::{
        autocomplete_search, build_autocomplete_index, fuzzy_autocomplete_search,
    };

    log::debug!(
        "Getting autocomplete for role '{}', query '{}'",
        role_name,
        query
    );

    let role_name = RoleName::new(&role_name);

    // Get the role graph for the specified role
    let Some(rolegraph_sync) = config_state.roles.get(&role_name) else {
        return Ok(Json(AutocompleteResponse {
            status: Status::Error,
            suggestions: vec![],
            error: Some(format!("Role '{}' not found", role_name)),
        }));
    };

    let rolegraph = rolegraph_sync.lock().await;

    // Build FST autocomplete index from the thesaurus
    let autocomplete_index = match build_autocomplete_index(rolegraph.thesaurus.clone(), None) {
        Ok(index) => index,
        Err(e) => {
            log::error!("Failed to build autocomplete index: {}", e);
            return Ok(Json(AutocompleteResponse {
                status: Status::Error,
                suggestions: vec![],
                error: Some(format!("Failed to build autocomplete index: {}", e)),
            }));
        }
    };

    // Try exact prefix search first
    let results = if query.len() >= 3 {
        // For longer queries, try fuzzy search for better UX (0.7 = 70% similarity threshold)
        match fuzzy_autocomplete_search(&autocomplete_index, &query, 0.7, Some(8)) {
            Ok(results) => results,
            Err(e) => {
                log::warn!("Fuzzy search failed, trying exact search: {}", e);
                // Fall back to exact search
                match autocomplete_search(&autocomplete_index, &query, Some(8)) {
                    Ok(results) => results,
                    Err(e) => {
                        log::error!("Autocomplete search failed: {}", e);
                        return Ok(Json(AutocompleteResponse {
                            status: Status::Error,
                            suggestions: vec![],
                            error: Some(format!("Autocomplete search failed: {}", e)),
                        }));
                    }
                }
            }
        }
    } else {
        // For short queries, use exact prefix search only
        match autocomplete_search(&autocomplete_index, &query, Some(8)) {
            Ok(results) => results,
            Err(e) => {
                log::error!("Autocomplete search failed: {}", e);
                return Ok(Json(AutocompleteResponse {
                    status: Status::Error,
                    suggestions: vec![],
                    error: Some(format!("Autocomplete search failed: {}", e)),
                }));
            }
        }
    };

    // Convert FST results to API response format with TipTap compatibility
    let suggestions: Vec<AutocompleteSuggestion> = results
        .into_iter()
        .map(|result| {
            let term = result.term.clone();
            let url = result.url.clone();

            AutocompleteSuggestion {
                term: term.clone(),
                text: term.clone(), // For TipTap compatibility
                normalized_term: result.normalized_term.as_str().to_string(),
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

    Ok(Json(AutocompleteResponse {
        status: Status::Success,
        suggestions,
        error: None,
    }))
}
