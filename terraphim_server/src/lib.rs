use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;

use axum::{
    http::{header, StatusCode, Uri},
    response::{Html, IntoResponse, Response},
    routing::{delete, get, post},
    Extension, Router,
};
use regex::Regex;
use rust_embed::RustEmbed;
use tokio::sync::{broadcast, RwLock};

// Pre-compiled regex for normalizing document IDs (performance optimization)
static NORMALIZE_REGEX: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
    Regex::new(r"[^a-zA-Z0-9]+").expect("Failed to create normalize regex")
});

/// Find the largest byte index <= `index` that is a valid UTF-8 char boundary.
/// Polyfill for str::floor_char_boundary (stable since Rust 1.91).
fn floor_char_boundary(s: &str, index: usize) -> usize {
    if index >= s.len() {
        return s.len();
    }
    let mut i = index;
    while i > 0 && !s.is_char_boundary(i) {
        i -= 1;
    }
    i
}
use terraphim_automata::builder::{Logseq, ThesaurusBuilder};
use terraphim_config::ConfigState;
use terraphim_persistence::Persistable;
use terraphim_rolegraph::{RoleGraph, RoleGraphSync};
use terraphim_service::summarization_manager::SummarizationManager;
use terraphim_service::summarization_queue::QueueConfig;
use terraphim_types::IndexedDocument;
use terraphim_types::{Document, RelevanceFunction};
use tokio::sync::broadcast::channel;
use tower_http::cors::{Any, CorsLayer};
use walkdir::WalkDir;

/// Find the largest index that is a valid UTF-8 char boundary at or before `index`.
/// Compatible with Rust < 1.91 (replaces `str::floor_char_boundary`).
#[inline]
fn floor_char_boundary(s: &str, index: usize) -> usize {
    if index >= s.len() {
        s.len()
    } else {
        let mut idx = index;
        while idx > 0 && !s.is_char_boundary(idx) {
            idx -= 1;
        }
        idx
    }
}

/// Create a proper description from document content
/// Collects multiple meaningful sentences to create informative descriptions
fn create_document_description(content: &str) -> Option<String> {
    let lines: Vec<&str> = content.lines().collect();
    let mut description_parts = Vec::new();
    let mut found_header = false;
    let mut content_lines = 0;

    for line in lines {
        let trimmed = line.trim();

        // Skip empty lines, frontmatter, and comments
        if trimmed.is_empty() || trimmed.starts_with("---") || trimmed.starts_with("<!--") {
            continue;
        }

        // Check if this is a markdown header
        if trimmed.starts_with('#') {
            if !found_header {
                // Include the first header (remove # symbols and clean up)
                let header_text = trimmed.trim_start_matches('#').trim();
                if !header_text.is_empty() && header_text.len() > 3 {
                    description_parts.push(header_text.to_string());
                    found_header = true;
                }
            }
            continue;
        }

        // Handle synonyms specially for KG files
        if trimmed.starts_with("synonyms::") {
            let synonym_text = trimmed.trim_start_matches("synonyms::").trim();
            if !synonym_text.is_empty() {
                description_parts.push(format!("synonyms: {}", synonym_text));
            }
            continue;
        }

        // Found a meaningful paragraph - collect multiple lines for better context
        if trimmed.len() > 20 && content_lines < 3 {
            // Get up to 3 meaningful content lines
            // Skip lines that are just metadata or formatting
            if !trimmed.starts_with("tags::") &&
               !trimmed.starts_with("![") && // Skip image references
               !trimmed.starts_with("```")
            {
                // Skip code blocks
                description_parts.push(trimmed.to_string());
                content_lines += 1;
            }
        }

        // Stop if we have enough content
        if description_parts.len() >= 4 || content_lines >= 3 {
            break;
        }
    }

    if description_parts.is_empty() {
        return None;
    }

    // Combine all parts intelligently
    let combined = if description_parts.len() == 1 {
        description_parts[0].clone()
    } else {
        // Join header with content using appropriate separators
        let mut result = description_parts[0].clone();
        for (i, part) in description_parts.iter().skip(1).enumerate() {
            if i == 0 {
                result.push_str(" - ");
            } else {
                result.push(' ');
            }
            result.push_str(part);
        }
        result
    };

    // Limit total length to 400 characters for more informative descriptions
    // Use floor_char_boundary to safely truncate at a valid UTF-8 boundary
    let description = if combined.len() > 400 {
        let safe_end = floor_char_boundary(&combined, 397);
        format!("{}...", &combined[..safe_end])
    } else {
        combined
    };

    Some(description)
}

mod api;
mod error;

pub mod workflows;

use api::{
    create_document, find_documents_by_kg_term, get_rolegraph, health, search_documents,
    search_documents_post,
};
pub use api::{
    AddContextRequest, AddContextResponse, AddMessageRequest, AddMessageResponse,
    AddSearchContextRequest, ConfigResponse, CreateConversationRequest, CreateConversationResponse,
    CreateDocumentResponse, DeleteContextResponse, GetConversationResponse, ListConversationsQuery,
    ListConversationsResponse, SearchResponse, UpdateContextRequest, UpdateContextResponse,
};
pub use error::{Result, Status};

// use axum_embed::ServeEmbed;
static INDEX_HTML: &str = "index.html";

#[derive(RustEmbed)]
#[folder = "../desktop/dist"]
struct Assets;

// Extended application state that includes workflow management
#[derive(Clone)]
pub struct AppState {
    pub config_state: ConfigState,
    pub workflow_sessions: Arc<workflows::WorkflowSessions>,
    pub websocket_broadcaster: workflows::WebSocketBroadcaster,
}

pub async fn axum_server(server_hostname: SocketAddr, mut config_state: ConfigState) -> Result<()> {
    log::info!("Starting axum server");

    let mut config = config_state.config.lock().await.clone();
    let mut local_rolegraphs = ahash::AHashMap::new();

    for (role_name, role) in &mut config.roles {
        if role.relevance_function == RelevanceFunction::TerraphimGraph {
            if let Some(kg) = &role.kg {
                if kg.automata_path.is_none() && kg.knowledge_graph_local.is_some() {
                    log::info!(
                        "Building rolegraph for role '{}' from local files",
                        role_name
                    );

                    let kg_local = kg.knowledge_graph_local.as_ref().unwrap();
                    log::info!("Knowledge graph path: {:?}", kg_local.path);

                    // Check if the directory exists
                    if !kg_local.path.exists() {
                        log::warn!(
                            "Knowledge graph directory does not exist: {:?}",
                            kg_local.path
                        );
                        continue;
                    }

                    // List files in the directory
                    let files: Vec<_> = if let Ok(entries) = std::fs::read_dir(&kg_local.path) {
                        entries
                            .filter_map(|entry| entry.ok())
                            .filter(|entry| {
                                if let Some(ext) = entry.path().extension() {
                                    ext == "md" || ext == "markdown"
                                } else {
                                    false
                                }
                            })
                            .collect()
                    } else {
                        Vec::new()
                    };

                    log::info!(
                        "Found {} markdown files in {:?}",
                        files.len(),
                        kg_local.path
                    );
                    for file in &files {
                        log::info!("  - {:?}", file.path());
                    }

                    // Build thesaurus using Logseq builder
                    let builder = Logseq::default();
                    log::info!("Created Logseq builder for path: {:?}", kg_local.path);

                    match builder
                        .build(role_name.to_string(), kg_local.path.clone())
                        .await
                    {
                        Ok(thesaurus) => {
                            log::info!("Successfully built and indexed rolegraph for role '{}' with {} terms and {} documents", role_name, thesaurus.len(), files.len());
                            // Create rolegraph
                            let rolegraph = RoleGraph::new(role_name.clone(), thesaurus).await?;
                            log::info!("Successfully created rolegraph for role '{}'", role_name);

                            // Index documents from knowledge graph files into the rolegraph
                            let mut rolegraph_with_docs = rolegraph;

                            // Index the knowledge graph markdown files as documents
                            if let Ok(entries) = std::fs::read_dir(&kg_local.path) {
                                for entry in entries.filter_map(|e| e.ok()) {
                                    if let Some(ext) = entry.path().extension() {
                                        if ext == "md" || ext == "markdown" {
                                            if let Ok(content) =
                                                tokio::fs::read_to_string(&entry.path()).await
                                            {
                                                // Create a proper description from the document content
                                                let description =
                                                    create_document_description(&content);

                                                // Use normalized ID to match what persistence layer uses
                                                let filename =
                                                    entry.file_name().to_string_lossy().to_string();
                                                let normalized_id = {
                                                    NORMALIZE_REGEX
                                                        .replace_all(&filename, "")
                                                        .to_lowercase()
                                                };

                                                let document = Document {
                                                    id: normalized_id.clone(),
                                                    url: entry.path().to_string_lossy().to_string(),
                                                    title: filename.clone(), // Keep original filename as title for display
                                                    body: content,
                                                    description,
                                                    summarization: None,
                                                    stub: None,
                                                    tags: None,
                                                    rank: None,
                                                    source_haystack: None,
                                                };

                                                // Save document to persistence layer first
                                                if let Err(e) = document.save().await {
                                                    log::error!("Failed to save document '{}' to persistence: {}", document.id, e);
                                                } else {
                                                    log::info!("✅ Saved document '{}' to persistence layer", document.id);
                                                }

                                                // Validate document has content before indexing into rolegraph
                                                if document.body.is_empty() {
                                                    log::warn!("Document '{}' has empty body, cannot properly index into rolegraph", filename);
                                                } else {
                                                    log::debug!("Document '{}' has {} chars of body content", filename, document.body.len());
                                                }

                                                // Then add to rolegraph for KG indexing using the same normalized ID
                                                let document_clone = document.clone();
                                                rolegraph_with_docs
                                                    .insert_document(&normalized_id, document);

                                                // Log rolegraph statistics after insertion
                                                let node_count =
                                                    rolegraph_with_docs.get_node_count();
                                                let edge_count =
                                                    rolegraph_with_docs.get_edge_count();
                                                let doc_count =
                                                    rolegraph_with_docs.get_document_count();

                                                log::info!(
                                                    "✅ Indexed document '{}' into rolegraph (body: {} chars, nodes: {}, edges: {}, docs: {})",
                                                    filename, document_clone.body.len(), node_count, edge_count, doc_count
                                                );
                                            }
                                        }
                                    }
                                }
                            }

                            // Also process and save all documents from haystack directories (recursively)
                            for haystack in &role.haystacks {
                                if haystack.service == terraphim_config::ServiceType::Ripgrep {
                                    log::info!(
                                        "Processing haystack documents from: {} (recursive)",
                                        haystack.location
                                    );

                                    let mut processed_count = 0;

                                    // Use walkdir for recursive directory traversal
                                    for entry in WalkDir::new(&haystack.location)
                                        .into_iter()
                                        .filter_map(|e| e.ok())
                                        .filter(|e| e.file_type().is_file())
                                    {
                                        if let Some(ext) = entry.path().extension() {
                                            if ext == "md" || ext == "markdown" {
                                                if let Ok(content) =
                                                    tokio::fs::read_to_string(&entry.path()).await
                                                {
                                                    // Create a proper description from the document content
                                                    let description =
                                                        create_document_description(&content);

                                                    // Use normalized ID to match what persistence layer uses
                                                    let filename = entry
                                                        .file_name()
                                                        .to_string_lossy()
                                                        .to_string();
                                                    let normalized_id = {
                                                        NORMALIZE_REGEX
                                                            .replace_all(&filename, "")
                                                            .to_lowercase()
                                                    };

                                                    // Skip if this is already a KG document (avoid duplicates)
                                                    if let Some(kg_local) =
                                                        &kg.knowledge_graph_local
                                                    {
                                                        if entry.path().starts_with(&kg_local.path)
                                                        {
                                                            continue; // Skip KG files, already processed above
                                                        }
                                                    }

                                                    let document = Document {
                                                        id: normalized_id.clone(),
                                                        url: entry
                                                            .path()
                                                            .to_string_lossy()
                                                            .to_string(),
                                                        title: filename.clone(), // Keep original filename as title for display
                                                        body: content,
                                                        description,
                                                        summarization: None,
                                                        stub: None,
                                                        tags: None,
                                                        rank: None,
                                                        source_haystack: None,
                                                    };

                                                    // Save document to persistence layer
                                                    if let Err(e) = document.save().await {
                                                        log::debug!("Failed to save haystack document '{}' to persistence: {}", document.id, e);
                                                    } else {
                                                        log::debug!("✅ Saved haystack document '{}' to persistence layer", document.id);
                                                        processed_count += 1;
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    log::info!(
                                        "✅ Processed {} documents from haystack: {} (recursive)",
                                        processed_count,
                                        haystack.location
                                    );
                                }
                            }

                            // Store in local rolegraphs map
                            local_rolegraphs.insert(
                                role_name.clone(),
                                RoleGraphSync::from(rolegraph_with_docs),
                            );
                            log::info!("Stored rolegraph in local map for role '{}'", role_name);
                        }
                        Err(e) => {
                            log::error!(
                                "Failed to build thesaurus for role '{}': {}",
                                role_name,
                                e
                            );
                        }
                    }
                }
            }
        }
    }

    // Merge local rolegraphs with existing ones
    for (role_name, rolegraph) in local_rolegraphs {
        config_state.roles.insert(role_name, rolegraph);
    }

    // let assets = axum_embed::ServeEmbed::<Assets>::with_parameters(Some("index.html".to_owned()),axum_embed::FallbackBehavior::Ok, Some("index.html".to_owned()));
    let (tx, _rx) = channel::<IndexedDocument>(10);

    // Initialize summarization manager
    let summarization_manager = Arc::new(SummarizationManager::new(QueueConfig::default()));
    log::info!("Initialized summarization manager with default configuration");

    // Initialize workflow management components
    let workflow_sessions = Arc::new(RwLock::new(HashMap::new()));
    let (websocket_broadcaster, _) = broadcast::channel(1000);
    log::info!("Initialized workflow management system with WebSocket support");

    // Create extended application state
    let app_state = AppState {
        config_state,
        workflow_sessions,
        websocket_broadcaster,
    };

    let app = Router::new()
        .route("/health", get(health))
        // .route("/documents", get(list_documents))
        .route("/documents", post(create_document))
        .route("/documents/", post(create_document))
        .route("/documents/search", get(search_documents))
        .route("/documents/search", post(search_documents_post))
        .route("/documents/summarize", post(api::summarize_document))
        .route("/documents/summarize/", post(api::summarize_document))
        .route("/summarization/status", get(api::get_summarization_status))
        .route("/summarization/status/", get(api::get_summarization_status))
        // New async summarization endpoints
        .route(
            "/documents/async_summarize",
            post(api::async_summarize_document),
        )
        .route(
            "/documents/async_summarize/",
            post(api::async_summarize_document),
        )
        .route("/summarization/batch", post(api::batch_summarize_documents))
        .route(
            "/summarization/batch/",
            post(api::batch_summarize_documents),
        )
        .route(
            "/summarization/task/{task_id}/status",
            get(api::get_task_status),
        )
        .route(
            "/summarization/task/{task_id}/status/",
            get(api::get_task_status),
        )
        .route(
            "/summarization/task/{task_id}/cancel",
            post(api::cancel_task),
        )
        .route(
            "/summarization/task/{task_id}/cancel/",
            post(api::cancel_task),
        )
        .route("/summarization/queue/stats", get(api::get_queue_stats))
        .route("/summarization/queue/stats/", get(api::get_queue_stats))
        .route("/chat", post(api::chat_completion))
        .route("/chat/", post(api::chat_completion))
        .route("/config", get(api::get_config))
        .route("/config/", get(api::get_config))
        .route("/config", post(api::update_config))
        .route("/config/", post(api::update_config))
        .route("/config/schema", get(api::get_config_schema))
        .route("/config/schema/", get(api::get_config_schema))
        .route("/config/selected_role", post(api::update_selected_role))
        .route("/config/selected_role/", post(api::update_selected_role))
        .route("/rolegraph", get(get_rolegraph))
        .route("/rolegraph/", get(get_rolegraph))
        .route(
            "/roles/{role_name}/kg_search",
            get(find_documents_by_kg_term),
        )
        .route("/thesaurus/{role_name}", get(api::get_thesaurus))
        .route(
            "/autocomplete/{role_name}/{query}",
            get(api::get_autocomplete),
        )
        // Conversation management routes
        .route("/conversations", post(api::create_conversation))
        .route("/conversations", get(api::list_conversations))
        .route("/conversations/", post(api::create_conversation))
        .route("/conversations/", get(api::list_conversations))
        .route("/conversations/{id}", get(api::get_conversation))
        .route("/conversations/{id}/", get(api::get_conversation))
        .route(
            "/conversations/{id}/messages",
            post(api::add_message_to_conversation),
        )
        .route(
            "/conversations/{id}/messages/",
            post(api::add_message_to_conversation),
        )
        .route(
            "/conversations/{id}/context",
            post(api::add_context_to_conversation),
        )
        .route(
            "/conversations/{id}/context/",
            post(api::add_context_to_conversation),
        )
        .route(
            "/conversations/{id}/search-context",
            post(api::add_search_context_to_conversation),
        )
        .route(
            "/conversations/{id}/search-context/",
            post(api::add_search_context_to_conversation),
        )
        .route(
            "/conversations/{id}/context/{context_id}",
            delete(api::delete_context_from_conversation).put(api::update_context_in_conversation),
        )
        // Add workflow management routes
        .merge(workflows::create_router())
        .fallback(static_handler)
        .with_state(app_state)
        .layer(Extension(tx))
        .layer(Extension(summarization_manager))
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_headers(Any)
                .allow_methods(Any),
        );

    // Note: Prefixing the host with `http://` makes the URL clickable in some terminals
    println!("listening on http://{server_hostname}");

    // This is the new way to start the server
    // However, we can't use it yet, because some crates have not updated
    // to `http` 1.0.0 yet.
    // let listener = tokio::net::TcpListener::bind(server_hostname).await?;
    // axum::serve(listener, app).await?;

    let listener = tokio::net::TcpListener::bind(&server_hostname).await?;
    axum::serve(listener, app.into_make_service()).await?;

    Ok(())
}

async fn static_handler(uri: Uri) -> impl IntoResponse {
    let path = uri.path().trim_start_matches('/');

    if path.is_empty() || path == INDEX_HTML {
        return index_html().await;
    }

    match Assets::get(path) {
        Some(content) => {
            let mime = mime_guess::from_path(path).first_or_octet_stream();

            ([(header::CONTENT_TYPE, mime.as_ref())], content.data).into_response()
        }
        None => {
            if path.contains('.') {
                return not_found().await;
            }

            index_html().await
        }
    }
}

async fn index_html() -> Response {
    match Assets::get(INDEX_HTML) {
        Some(content) => Html(content.data).into_response(),
        None => not_found().await,
    }
}

async fn not_found() -> Response {
    (StatusCode::NOT_FOUND, "404").into_response()
}

pub async fn build_router_for_tests() -> Router {
    use terraphim_config::ConfigBuilder;

    // Create minimal test configuration
    let mut config = ConfigBuilder::new()
        .build_default_embedded()
        .build()
        .expect("Failed to build test config");

    let config_state = ConfigState::new(&mut config)
        .await
        .expect("Failed to create ConfigState");

    let (tx, _rx) = channel::<IndexedDocument>(10);

    // Initialize summarization manager
    let summarization_manager = Arc::new(SummarizationManager::new(QueueConfig::default()));

    // Initialize workflow management components for tests
    let workflow_sessions = Arc::new(RwLock::new(HashMap::new()));
    let (websocket_broadcaster, _) = broadcast::channel(100);

    // Create extended application state for tests
    let app_state = AppState {
        config_state,
        workflow_sessions,
        websocket_broadcaster,
    };

    Router::new()
        .route("/health", get(health))
        .route("/documents", post(create_document))
        .route("/documents/", post(create_document))
        .route("/documents/search", get(search_documents))
        .route("/documents/search", post(search_documents_post))
        .route("/documents/summarize", post(api::summarize_document))
        .route("/documents/summarize/", post(api::summarize_document))
        .route("/summarization/status", get(api::get_summarization_status))
        .route("/summarization/status/", get(api::get_summarization_status))
        .route(
            "/documents/async_summarize",
            post(api::async_summarize_document),
        )
        .route(
            "/documents/async_summarize/",
            post(api::async_summarize_document),
        )
        .route("/summarization/batch", post(api::batch_summarize_documents))
        .route(
            "/summarization/batch/",
            post(api::batch_summarize_documents),
        )
        .route(
            "/summarization/task/{task_id}/status",
            get(api::get_task_status),
        )
        .route(
            "/summarization/task/{task_id}/status/",
            get(api::get_task_status),
        )
        .route(
            "/summarization/task/{task_id}/cancel",
            post(api::cancel_task),
        )
        .route(
            "/summarization/task/{task_id}/cancel/",
            post(api::cancel_task),
        )
        .route("/summarization/queue/stats", get(api::get_queue_stats))
        .route("/summarization/queue/stats/", get(api::get_queue_stats))
        .route("/chat", post(api::chat_completion))
        .route("/chat/", post(api::chat_completion))
        .route("/config", get(api::get_config))
        .route("/config/", get(api::get_config))
        .route("/config", post(api::update_config))
        .route("/config/", post(api::update_config))
        .route("/config/schema", get(api::get_config_schema))
        .route("/config/schema/", get(api::get_config_schema))
        .route("/config/selected_role", post(api::update_selected_role))
        .route("/config/selected_role/", post(api::update_selected_role))
        .route("/rolegraph", get(get_rolegraph))
        .route("/rolegraph/", get(get_rolegraph))
        .route(
            "/roles/{role_name}/kg_search",
            get(find_documents_by_kg_term),
        )
        .route("/thesaurus/{role_name}", get(api::get_thesaurus))
        .route(
            "/autocomplete/{role_name}/{query}",
            get(api::get_autocomplete),
        )
        // Conversation management routes
        .route("/conversations", post(api::create_conversation))
        .route("/conversations", get(api::list_conversations))
        .route("/conversations/", post(api::create_conversation))
        .route("/conversations/", get(api::list_conversations))
        .route("/conversations/{id}", get(api::get_conversation))
        .route("/conversations/{id}/", get(api::get_conversation))
        .route(
            "/conversations/{id}/messages",
            post(api::add_message_to_conversation),
        )
        .route(
            "/conversations/{id}/messages/",
            post(api::add_message_to_conversation),
        )
        .route(
            "/conversations/{id}/context",
            post(api::add_context_to_conversation),
        )
        .route(
            "/conversations/{id}/context/",
            post(api::add_context_to_conversation),
        )
        .route(
            "/conversations/{id}/search-context",
            post(api::add_search_context_to_conversation),
        )
        .route(
            "/conversations/{id}/search-context/",
            post(api::add_search_context_to_conversation),
        )
        .route(
            "/conversations/{id}/context/{context_id}",
            delete(api::delete_context_from_conversation).put(api::update_context_in_conversation),
        )
        // Add workflow management routes for tests
        .merge(workflows::create_router())
        .with_state(app_state)
        .layer(Extension(tx))
        .layer(Extension(summarization_manager))
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_headers(Any)
                .allow_methods(Any),
        )
}
