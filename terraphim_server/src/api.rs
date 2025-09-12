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
    /// Role to use for chat (determines LLM configuration)
    pub role: String,
    /// Conversation so far
    pub messages: Vec<ChatMessage>,
    /// Optional model override
    pub model: Option<String>,
    /// Optional conversation ID to include context from
    pub conversation_id: Option<String>,
    /// Optional maximum tokens for the response
    pub max_tokens: Option<u32>,
    /// Optional temperature for response randomness (0.0-1.0)
    pub temperature: Option<f32>,
}

/// Chat response payload
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ChatResponse {
    pub status: Status,
    pub message: Option<String>,
    pub model_used: Option<String>,
    pub error: Option<String>,
}

/// Handle chat completion via generic LLM client (OpenRouter, Ollama, etc.)
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

    // Try to build an LLM client from the role configuration
    use terraphim_service::llm;
    let Some(llm_client) = llm::build_llm_from_role(&role) else {
        return Ok(Json(ChatResponse {
            status: Status::Error,
            message: None,
            model_used: None,
            error: Some("No LLM provider configured for this role. Please configure OpenRouter or Ollama in the role's 'extra' settings.".to_string()),
        }));
    };

    // Build messages array; optionally inject system prompt and context
    let mut messages_json: Vec<serde_json::Value> = Vec::new();

    // Start with system prompt if available (support both OpenRouter and generic formats)
    let system_prompt = {
        #[cfg(feature = "openrouter")]
        {
            role.openrouter_chat_system_prompt.or_else(|| {
                // Try generic system prompt from extra
                role.extra
                    .get("system_prompt")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
            })
        }
        #[cfg(not(feature = "openrouter"))]
        {
            // Try generic system prompt from extra
            role.extra
                .get("system_prompt")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
        }
    };

    if let Some(system) = &system_prompt {
        messages_json.push(serde_json::json!({"role":"system","content":system}));
    }

    // Inject context from conversation if provided
    if let Some(conversation_id) = &request.conversation_id {
        let conv_id = ConversationId::from_string(conversation_id.clone());
        let manager = CONTEXT_MANAGER.lock().await;

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

                // Add context as a system message after the main system prompt
                messages_json
                    .push(serde_json::json!({"role": "system", "content": context_content}));
            }
        }
    }

    // Add user messages from the request
    for m in request.messages.iter() {
        messages_json.push(serde_json::json!({"role": m.role, "content": m.content}));
    }

    // Determine model name for response
    let model_name = request
        .model
        .or_else(|| {
            #[cfg(feature = "openrouter")]
            {
                role.openrouter_chat_model
                    .clone()
                    .or_else(|| role.openrouter_model.clone())
            }
            #[cfg(not(feature = "openrouter"))]
            {
                None
            }
        })
        .or_else(|| {
            role.extra
                .get("llm_model")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
        })
        .or_else(|| {
            role.extra
                .get("ollama_model")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
        })
        .unwrap_or_else(|| format!("{} (default)", llm_client.name()));

    // Configure chat options
    let chat_opts = llm::ChatOptions {
        max_tokens: request.max_tokens.or(Some(1024)),
        temperature: request.temperature.or(Some(0.7)),
    };

    // Call the LLM client
    match llm_client.chat_completion(messages_json, chat_opts).await {
        Ok(reply) => Ok(Json(ChatResponse {
            status: Status::Success,
            message: Some(reply),
            model_used: Some(model_name),
            error: None,
        })),
        Err(e) => Ok(Json(ChatResponse {
            status: Status::Error,
            message: None,
            model_used: Some(model_name),
            error: Some(format!("Chat failed: {}", e)),
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

    // Check if OpenRouter is enabled and configured for this role
    #[cfg(feature = "openrouter")]
    {
        if !role_ref.has_openrouter_config() {
            return Ok(Json(SummarizeDocumentResponse {
                status: Status::Error,
                document_id: request.document_id,
                summary: None,
                model_used: None,
                from_cache: false,
                error: Some("OpenRouter not properly configured for this role".to_string()),
            }));
        }

        // Clone role to use after dropping lock
        let role = role_ref.clone();

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

        let model = role
            .openrouter_model
            .as_deref()
            .unwrap_or("openai/gpt-3.5-turbo");
        let max_length = request.max_length.unwrap_or(250);
        let force_regenerate = request.force_regenerate.unwrap_or(false);

        drop(config); // Release the lock before async operations

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
                        model_used: Some(model.to_string()),
                        from_cache: true,
                        error: None,
                    }));
                }
            }
        }

        // Generate new summary using OpenRouter
        match terraphim_service
            .generate_document_summary(&document, &api_key, model, max_length)
            .await
        {
            Ok(summary) => {
                log::info!(
                    "Generated summary for document '{}' using model '{}'",
                    request.document_id,
                    model
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
                    model_used: Some(model.to_string()),
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

#[derive(Debug, Serialize, Clone)]
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
/// Results are cached for 10 minutes to improve performance.
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
    let cache_key = format!("{}:{}", role_name, query);

    // Check cache first
    {
        let cache = AUTOCOMPLETE_CACHE.lock().await;
        if let Some(cached) = cache.get(&cache_key) {
            if cached.is_valid() {
                log::debug!("Returning cached autocomplete results for '{}'", cache_key);
                return Ok(Json(AutocompleteResponse {
                    status: Status::Success,
                    suggestions: cached.suggestions.clone(),
                    error: None,
                }));
            }
        }
    }

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

    // Cache the results
    {
        let mut cache = AUTOCOMPLETE_CACHE.lock().await;
        let cached_result = CachedAutocompleteResult::new(suggestions.clone());
        cache.insert(cache_key.clone(), cached_result);
        log::debug!("Cached autocomplete results for '{}'", cache_key);
    }

    Ok(Json(AutocompleteResponse {
        status: Status::Success,
        suggestions,
        error: None,
    }))
}

// =================== CONVERSATION MANAGEMENT API ===================

use terraphim_service::context::{ContextConfig, ContextManager};
use terraphim_types::{ConversationId, ConversationSummary};
use std::collections::HashMap;
use chrono::{DateTime, Duration, Utc};

/// Cached autocomplete result with expiration
#[derive(Debug, Clone)]
struct CachedAutocompleteResult {
    suggestions: Vec<AutocompleteSuggestion>,
    created_at: DateTime<Utc>,
    expires_at: DateTime<Utc>,
}

impl CachedAutocompleteResult {
    fn new(suggestions: Vec<AutocompleteSuggestion>) -> Self {
        let now = Utc::now();
        Self {
            suggestions,
            created_at: now,
            expires_at: now + Duration::minutes(10), // 10-minute cache
        }
    }

    fn is_valid(&self) -> bool {
        Utc::now() < self.expires_at
    }
}

/// Global autocomplete cache
pub static AUTOCOMPLETE_CACHE: std::sync::LazyLock<tokio::sync::Mutex<HashMap<String, CachedAutocompleteResult>>> =
    std::sync::LazyLock::new(|| {
        tokio::sync::Mutex::new(HashMap::new())
    });

/// Global context manager instance
pub static CONTEXT_MANAGER: std::sync::LazyLock<tokio::sync::Mutex<ContextManager>> =
    std::sync::LazyLock::new(|| {
        tokio::sync::Mutex::new(ContextManager::new(ContextConfig::default()))
    });

/// Request to create a new conversation
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CreateConversationRequest {
    pub title: String,
    pub role: String,
}

/// Response for conversation creation
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CreateConversationResponse {
    pub status: Status,
    pub conversation_id: Option<String>,
    pub error: Option<String>,
}

/// Request to list conversations
#[derive(Debug, Deserialize)]
pub struct ListConversationsQuery {
    pub limit: Option<usize>,
}

/// Response for listing conversations
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ListConversationsResponse {
    pub status: Status,
    pub conversations: Vec<ConversationSummary>,
    pub error: Option<String>,
}

/// Response for getting a single conversation
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GetConversationResponse {
    pub status: Status,
    pub conversation: Option<terraphim_types::Conversation>,
    pub error: Option<String>,
}

/// Request to add a message to a conversation
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AddMessageRequest {
    pub content: String,
    pub role: Option<String>, // Default to "user"
}

/// Response for adding a message
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AddMessageResponse {
    pub status: Status,
    pub message_id: Option<String>,
    pub error: Option<String>,
}

/// Request to add context to a conversation
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AddContextRequest {
    pub context_type: String, // "document" | "search_result" | "user_input"
    pub title: String,
    pub summary: Option<String>,
    pub content: String,
    pub metadata: Option<std::collections::HashMap<String, String>>,
}

/// Response for adding context
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AddContextResponse {
    pub status: Status,
    pub error: Option<String>,
}

/// Request to add search results as context
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AddSearchContextRequest {
    pub query: String,
    pub documents: Vec<Document>,
    pub limit: Option<usize>,
}

/// Request to update context in a conversation
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UpdateContextRequest {
    pub context_type: Option<String>,
    pub title: Option<String>,
    pub summary: Option<String>,
    pub content: Option<String>,
    pub metadata: Option<std::collections::HashMap<String, String>>,
}

/// Response for updating context
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UpdateContextResponse {
    pub status: Status,
    pub context: Option<terraphim_types::ContextItem>,
    pub error: Option<String>,
}

/// Response for deleting context
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DeleteContextResponse {
    pub status: Status,
    pub error: Option<String>,
}

/// Create a new conversation
pub(crate) async fn create_conversation(
    Json(request): Json<CreateConversationRequest>,
) -> Result<Json<CreateConversationResponse>> {
    let role_name = RoleName::new(&request.role);

    let mut manager = CONTEXT_MANAGER.lock().await;
    match manager.create_conversation(request.title, role_name).await {
        Ok(conversation_id) => Ok(Json(CreateConversationResponse {
            status: Status::Success,
            conversation_id: Some(conversation_id.as_str().to_string()),
            error: None,
        })),
        Err(e) => Ok(Json(CreateConversationResponse {
            status: Status::Error,
            conversation_id: None,
            error: Some(format!("Failed to create conversation: {}", e)),
        })),
    }
}

/// List conversations
pub(crate) async fn list_conversations(
    Query(params): Query<ListConversationsQuery>,
) -> Result<Json<ListConversationsResponse>> {
    let manager = CONTEXT_MANAGER.lock().await;
    let conversations = manager.list_conversations(params.limit);
    Ok(Json(ListConversationsResponse {
        status: Status::Success,
        conversations,
        error: None,
    }))
}

/// Get a specific conversation
pub(crate) async fn get_conversation(
    Path(conversation_id): Path<String>,
) -> Result<Json<GetConversationResponse>> {
    let conv_id = ConversationId::from_string(conversation_id);

    let manager = CONTEXT_MANAGER.lock().await;
    match manager.get_conversation(&conv_id) {
        Some(conversation) => Ok(Json(GetConversationResponse {
            status: Status::Success,
            conversation: Some((*conversation).clone()),
            error: None,
        })),
        None => Ok(Json(GetConversationResponse {
            status: Status::Error,
            conversation: None,
            error: Some("Conversation not found".to_string()),
        })),
    }
}

/// Add a message to a conversation
pub(crate) async fn add_message_to_conversation(
    Path(conversation_id): Path<String>,
    Json(request): Json<AddMessageRequest>,
) -> Result<Json<AddMessageResponse>> {
    let conv_id = ConversationId::from_string(conversation_id);
    let role = request.role.unwrap_or_else(|| "user".to_string());

    let message = if role == "user" {
        terraphim_types::ChatMessage::user(request.content)
    } else if role == "assistant" {
        terraphim_types::ChatMessage::assistant(request.content, None)
    } else if role == "system" {
        terraphim_types::ChatMessage::system(request.content)
    } else {
        return Ok(Json(AddMessageResponse {
            status: Status::Error,
            message_id: None,
            error: Some(format!("Invalid role: {}", role)),
        }));
    };

    let mut manager = CONTEXT_MANAGER.lock().await;
    match manager.add_message(&conv_id, message) {
        Ok(message_id) => Ok(Json(AddMessageResponse {
            status: Status::Success,
            message_id: Some(message_id.as_str().to_string()),
            error: None,
        })),
        Err(e) => Ok(Json(AddMessageResponse {
            status: Status::Error,
            message_id: None,
            error: Some(format!("Failed to add message: {}", e)),
        })),
    }
}

/// Add context to a conversation
pub(crate) async fn add_context_to_conversation(
    Path(conversation_id): Path<String>,
    Json(request): Json<AddContextRequest>,
) -> Result<Json<AddContextResponse>> {
    let conv_id = ConversationId::from_string(conversation_id);

    let context_type = match request.context_type.as_str() {
        "document" => terraphim_types::ContextType::Document,
        "search_result" => terraphim_types::ContextType::Document, // Changed from SearchResult to Document
        "user_input" => terraphim_types::ContextType::UserInput,
        "system" => terraphim_types::ContextType::System,
        "external" => terraphim_types::ContextType::External,
        _ => {
            return Ok(Json(AddContextResponse {
                status: Status::Error,
                error: Some(format!("Invalid context type: {}", request.context_type)),
            }))
        }
    };

    let context_item = terraphim_types::ContextItem {
        id: ulid::Ulid::new().to_string(),
        context_type,
        title: request.title,
        summary: request.summary,
        content: request.content,
        metadata: request.metadata.unwrap_or_default().into_iter().collect(),
        created_at: chrono::Utc::now(),
        relevance_score: None,
    };

    let mut manager = CONTEXT_MANAGER.lock().await;
    match manager.add_context(&conv_id, context_item) {
        Ok(()) => Ok(Json(AddContextResponse {
            status: Status::Success,
            error: None,
        })),
        Err(e) => Ok(Json(AddContextResponse {
            status: Status::Error,
            error: Some(format!("Failed to add context: {}", e)),
        })),
    }
}

/// Add search results as context to a conversation
pub(crate) async fn add_search_context_to_conversation(
    Path(conversation_id): Path<String>,
    Json(request): Json<AddSearchContextRequest>,
) -> Result<Json<AddContextResponse>> {
    let conv_id = ConversationId::from_string(conversation_id);

    let mut manager = CONTEXT_MANAGER.lock().await;
    let context_item =
        manager.create_search_context(&request.query, &request.documents, request.limit);

    match manager.add_context(&conv_id, context_item) {
        Ok(()) => Ok(Json(AddContextResponse {
            status: Status::Success,
            error: None,
        })),
        Err(e) => Ok(Json(AddContextResponse {
            status: Status::Error,
            error: Some(format!("Failed to add search context: {}", e)),
        })),
    }
}

/// Delete context from a conversation
pub(crate) async fn delete_context_from_conversation(
    Path((conversation_id, context_id)): Path<(String, String)>,
) -> Result<Json<DeleteContextResponse>> {
    let conv_id = ConversationId::from_string(conversation_id);

    let mut manager = CONTEXT_MANAGER.lock().await;
    match manager.delete_context(&conv_id, &context_id) {
        Ok(()) => Ok(Json(DeleteContextResponse {
            status: Status::Success,
            error: None,
        })),
        Err(e) => Ok(Json(DeleteContextResponse {
            status: Status::Error,
            error: Some(format!("Failed to delete context: {}", e)),
        })),
    }
}

/// Update context in a conversation
pub(crate) async fn update_context_in_conversation(
    Path((conversation_id, context_id)): Path<(String, String)>,
    Json(request): Json<UpdateContextRequest>,
) -> Result<Json<UpdateContextResponse>> {
    let conv_id = ConversationId::from_string(conversation_id.clone());

    // Get the existing context item first
    let manager = CONTEXT_MANAGER.lock().await;
    let conversation = match manager.get_conversation(&conv_id) {
        Some(conv) => conv,
        None => {
            return Ok(Json(UpdateContextResponse {
                status: Status::Error,
                context: None,
                error: Some(format!("Conversation {} not found", conversation_id)),
            }))
        }
    };

    // Find the existing context item
    let existing_context = conversation
        .global_context
        .iter()
        .find(|item| item.id == context_id);

    let existing_context = match existing_context {
        Some(ctx) => ctx,
        None => {
            return Ok(Json(UpdateContextResponse {
                status: Status::Error,
                context: None,
                error: Some(format!("Context item {} not found", context_id)),
            }))
        }
    };

    // Build the updated context item
    let context_type = if let Some(ref type_str) = request.context_type {
        match type_str.as_str() {
            "document" => terraphim_types::ContextType::Document,
            "search_result" => terraphim_types::ContextType::Document, // Changed from SearchResult to Document
            "user_input" => terraphim_types::ContextType::UserInput,
            "system" => terraphim_types::ContextType::System,
            "external" => terraphim_types::ContextType::External,
            _ => existing_context.context_type.clone(),
        }
    } else {
        existing_context.context_type.clone()
    };

    let updated_context = terraphim_types::ContextItem {
        id: context_id.clone(),
        context_type,
        title: request
            .title
            .unwrap_or_else(|| existing_context.title.clone()),
        summary: request.summary.or_else(|| existing_context.summary.clone()),
        content: request
            .content
            .unwrap_or_else(|| existing_context.content.clone()),
        metadata: request
            .metadata
            .map(|m| m.into_iter().collect())
            .unwrap_or_else(|| existing_context.metadata.clone()),
        created_at: existing_context.created_at,
        relevance_score: existing_context.relevance_score,
    };

    drop(manager); // Release the lock before re-acquiring it
    let mut manager = CONTEXT_MANAGER.lock().await;
    match manager.update_context(&conv_id, &context_id, updated_context.clone()) {
        Ok(context) => Ok(Json(UpdateContextResponse {
            status: Status::Success,
            context: Some(context),
            error: None,
        })),
        Err(e) => Ok(Json(UpdateContextResponse {
            status: Status::Error,
            context: None,
            error: Some(format!("Failed to update context: {}", e)),
        })),
    }
}

// =================== KG CONTEXT MANAGEMENT API ===================

/// Request to search KG terms with autocomplete
#[derive(Debug, Deserialize)]
pub struct KGSearchRequest {
    pub query: String,
    pub limit: Option<usize>,
    pub min_similarity: Option<f64>,
}

/// Response for KG search
#[derive(Debug, Serialize)]
pub struct KGSearchResponse {
    pub status: Status,
    pub suggestions: Vec<AutocompleteSuggestion>,
    pub error: Option<String>,
}

/// Request to add KG term definition to context
#[derive(Debug, Deserialize)]
pub struct AddKGTermContextRequest {
    pub term: String,
    pub role: String,
}

/// Request to add complete KG index to context
#[derive(Debug, Deserialize)]
pub struct AddKGIndexContextRequest {
    pub role: String,
}

/// Search KG terms with autocomplete and caching
pub(crate) async fn search_kg_terms(
    State(config_state): State<ConfigState>,
    Path(conversation_id): Path<String>,
    Query(request): Query<KGSearchRequest>,
) -> Result<Json<KGSearchResponse>> {
    log::debug!(
        "Searching KG terms for conversation {} with query '{}'",
        conversation_id,
        request.query
    );

    let role_name = RoleName::new(&request.query); // Use query as role for now, should be passed in request
    let cache_key = format!("kg_search:{}:{}", role_name, request.query);

    // Check cache first
    {
        let cache = AUTOCOMPLETE_CACHE.lock().await;
        if let Some(cached) = cache.get(&cache_key) {
            if cached.is_valid() {
                log::debug!("Returning cached KG search results for '{}'", cache_key);
                return Ok(Json(KGSearchResponse {
                    status: Status::Success,
                    suggestions: cached.suggestions.clone(),
                    error: None,
                }));
            }
        }
    }

    // Use existing autocomplete functionality
    let autocomplete_response = get_autocomplete(
        State(config_state),
        Path((role_name.to_string(), request.query.clone())),
    ).await?;

    match autocomplete_response {
        Json(response) => {
            // Cache the results
            {
                let mut cache = AUTOCOMPLETE_CACHE.lock().await;
                let cached_result = CachedAutocompleteResult::new(response.suggestions.clone());
                cache.insert(cache_key.clone(), cached_result);
                log::debug!("Cached KG search results for '{}'", cache_key);
            }

            Ok(Json(KGSearchResponse {
                status: response.status,
                suggestions: response.suggestions,
                error: response.error,
            }))
        }
    }
}

/// Add KG term definition to conversation context
pub(crate) async fn add_kg_term_context(
    State(config_state): State<ConfigState>,
    Path(conversation_id): Path<String>,
    Json(request): Json<AddKGTermContextRequest>,
) -> Result<Json<AddContextResponse>> {
    log::debug!(
        "Adding KG term '{}' to conversation {} context",
        request.term,
        conversation_id
    );

    let conv_id = ConversationId::from_string(conversation_id);
    let role_name = RoleName::new(&request.role);

    // Use existing KG search to find documents for the term
    let mut terraphim_service = TerraphimService::new(config_state);
    let documents = terraphim_service
        .find_documents_for_kg_term(&role_name, &request.term)
        .await
        .map_err(|e| crate::error::ApiError(
            StatusCode::INTERNAL_SERVER_ERROR,
            anyhow::anyhow!("Failed to find KG documents: {}", e),
        ))?;

    if documents.is_empty() {
        return Ok(Json(AddContextResponse {
            status: Status::Error,
            error: Some(format!("No documents found for KG term '{}'", request.term)),
        }));
    }

    // Create KG term definition from the first document
    let doc = &documents[0];
    let kg_term = terraphim_types::KGTermDefinition {
        term: request.term.clone(),
        normalized_term: terraphim_types::NormalizedTermValue::new(request.term.clone()),
        id: 1, // TODO: Get actual term ID from thesaurus
        definition: Some(doc.description.clone().unwrap_or_default()),
        synonyms: Vec::new(), // TODO: Extract from thesaurus
        related_terms: Vec::new(), // TODO: Extract from thesaurus
        usage_examples: Vec::new(), // TODO: Extract from thesaurus
        url: if doc.url.is_empty() { None } else { Some(doc.url.clone()) },
        metadata: {
            let mut meta = ahash::AHashMap::new();
            meta.insert("source_document".to_string(), doc.id.clone());
            meta.insert("document_title".to_string(), doc.title.clone());
            meta
        },
        relevance_score: doc.rank.map(|r| r as f64),
    };

    let context_item = terraphim_types::ContextItem::from_kg_term_definition(&kg_term);

    let mut manager = CONTEXT_MANAGER.lock().await;
    match manager.add_context(&conv_id, context_item) {
        Ok(()) => Ok(Json(AddContextResponse {
            status: Status::Success,
            error: None,
        })),
        Err(e) => Ok(Json(AddContextResponse {
            status: Status::Error,
            error: Some(format!("Failed to add KG term context: {}", e)),
        })),
    }
}

/// Add complete KG index to conversation context
pub(crate) async fn add_kg_index_context(
    State(config_state): State<ConfigState>,
    Path(conversation_id): Path<String>,
    Json(request): Json<AddKGIndexContextRequest>,
) -> Result<Json<AddContextResponse>> {
    log::debug!(
        "Adding KG index for role '{}' to conversation {} context",
        request.role,
        conversation_id
    );

    let conv_id = ConversationId::from_string(conversation_id);
    let role_name = RoleName::new(&request.role);

    // Get rolegraph to extract KG index information
    let rolegraph_sync = config_state.roles.get(&role_name)
        .ok_or_else(|| crate::error::ApiError(
            StatusCode::NOT_FOUND,
            anyhow::anyhow!("Role '{}' not found", role_name),
        ))?;

    let rolegraph = rolegraph_sync.lock().await;
    
    let kg_index = terraphim_types::KGIndexInfo {
        name: format!("KG Index for {}", role_name),
        total_terms: rolegraph.thesaurus.len(),
        total_nodes: rolegraph.nodes_map().len(),
        total_edges: rolegraph.edges_map().len(),
        last_updated: chrono::Utc::now(), // TODO: Get actual last updated time
        source: "local".to_string(),
        version: Some("1.0".to_string()),
    };

    let context_item = terraphim_types::ContextItem::from_kg_index(&kg_index);

    let mut manager = CONTEXT_MANAGER.lock().await;
    match manager.add_context(&conv_id, context_item) {
        Ok(()) => Ok(Json(AddContextResponse {
            status: Status::Success,
            error: None,
        })),
        Err(e) => Ok(Json(AddContextResponse {
            status: Status::Error,
            error: Some(format!("Failed to add KG index context: {}", e)),
        })),
    }
}
