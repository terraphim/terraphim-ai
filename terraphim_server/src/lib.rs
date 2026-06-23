//! HTTP server for the Terraphim AI backend.
//!
//! Exposes search, configuration, knowledge-graph, conversation-context,
//! and workflow management APIs via Axum.  Optionally serves embedded
//! frontend assets when built with the `embedded-assets` feature.
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;

use axum::{
    Extension, Router,
    http::{StatusCode, Uri, header},
    response::{Html, IntoResponse, Response},
    routing::{delete, get, post},
};
use regex::Regex;
#[cfg(feature = "embedded-assets")]
use rust_embed::RustEmbed;
use tokio::sync::{RwLock, broadcast};

// Pre-compiled regex for normalizing document IDs (performance optimization)
static NORMALIZE_REGEX: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
    Regex::new(r"[^a-zA-Z0-9]+").expect("Failed to create normalize regex")
});

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
    let description = if combined.len() > 400 {
        let safe_end = combined.floor_char_boundary(397);
        format!("{}...", &combined[..safe_end])
    } else {
        combined
    };

    Some(description)
}

mod api;
mod error;

/// HTTP handlers for multi-agent, optimisation, orchestration, parallel, prompt-chain, routing, VM-execution, and WebSocket workflows.
pub mod workflows;

pub use api::{
    AddContextRequest, AddContextResponse, AddMessageRequest, AddMessageResponse,
    AddSearchContextRequest, ConfigResponse, CreateConversationRequest, CreateConversationResponse,
    CreateDocumentResponse, DeleteContextResponse, GetConversationResponse, ListConversationsQuery,
    ListConversationsResponse, RoleGraphResponseDto, SearchResponse, ThesaurusResponse,
    UpdateContextRequest, UpdateContextResponse,
};
use api::{
    create_document, find_documents_by_kg_term, get_rolegraph, health, search_documents,
    search_documents_post,
};
pub use error::{Result, Status};

// use axum_embed::ServeEmbed;
static INDEX_HTML: &str = "index.html";

#[cfg(feature = "embedded-assets")]
#[derive(RustEmbed)]
#[folder = "dist"]
struct Assets;

#[cfg(not(feature = "embedded-assets"))]
mod assets {
    pub struct Asset;
    pub struct EmbeddedFile;

    impl Asset {
        pub fn get(_path: &str) -> Option<EmbeddedFile> {
            None
        }
    }
}

#[cfg(not(feature = "embedded-assets"))]
use assets::Asset as Assets;

/// Shared application state threaded through all Axum route handlers.
#[derive(Clone)]
pub struct AppState {
    /// Configuration and per-role knowledge-graph state.
    pub config_state: ConfigState,
    /// Live map of workflow runs keyed by workflow ID.
    pub workflow_sessions: Arc<workflows::WorkflowSessions>,
    /// Sender half of the WebSocket broadcast channel.
    pub websocket_broadcaster: workflows::WebSocketBroadcaster,
}

/// Build the application router: all API routes, shared `AppState`, and layers.
///
/// Single source of truth for route composition (ADR-005) so the production
/// server and the test harness can never drift. `serve_static` toggles the
/// static-asset fallback used by the production binary (the test harness omits
/// it so unmatched paths surface as 404s).
pub fn build_router(
    app_state: AppState,
    tx: tokio::sync::broadcast::Sender<IndexedDocument>,
    summarization_manager: Arc<SummarizationManager>,
    serve_static: bool,
) -> Router {
    let router = Router::new()
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
        .merge(workflows::create_router());

    let router = if serve_static {
        router.fallback(static_handler)
    } else {
        router
    };

    router
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

/// Starts the Axum HTTP server, builds all routes, and serves until shutdown.
pub async fn axum_server(server_hostname: SocketAddr, mut config_state: ConfigState) -> Result<()> {
    log::info!("Starting axum server");

    let mut config = config_state.config.lock().await.clone();
    let mut local_rolegraphs = ahash::AHashMap::new();

    for (role_name, role) in &mut config.roles {
        if role.relevance_function == RelevanceFunction::TerraphimGraph
            && let Some(kg) = &role.kg
            && let (None, Some(kg_local)) = (&kg.automata_path, &kg.knowledge_graph_local)
        {
            log::info!(
                "Building rolegraph for role '{}' from local files",
                role_name
            );

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
                    log::info!(
                        "Successfully built and indexed rolegraph for role '{}' with {} terms and {} documents",
                        role_name,
                        thesaurus.len(),
                        files.len()
                    );
                    // Create rolegraph
                    let rolegraph = RoleGraph::new(role_name.clone(), thesaurus).await?;
                    log::info!("Successfully created rolegraph for role '{}'", role_name);

                    // Index documents from knowledge graph files into the rolegraph
                    let mut rolegraph_with_docs = rolegraph;

                    // Index the knowledge graph markdown files as documents
                    if let Ok(entries) = std::fs::read_dir(&kg_local.path) {
                        for entry in entries.filter_map(|e| e.ok()) {
                            if let Some(ext) = entry.path().extension()
                                && (ext == "md" || ext == "markdown")
                                && let Ok(content) = tokio::fs::read_to_string(&entry.path()).await
                            {
                                // Create a proper description from the document content
                                let description = create_document_description(&content);

                                // Use normalized ID to match what persistence layer uses
                                let filename = entry.file_name().to_string_lossy().to_string();
                                let normalized_id =
                                    { NORMALIZE_REGEX.replace_all(&filename, "").to_lowercase() };

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
                                    doc_type: terraphim_types::DocumentType::KgEntry,
                                    synonyms: None,
                                    route: None,
                                    priority: None,
                                    quality_score: None,
                                };

                                // Save document to persistence layer first
                                if let Err(e) = document.save().await {
                                    log::error!(
                                        "Failed to save document '{}' to persistence: {}",
                                        document.id,
                                        e
                                    );
                                } else {
                                    log::info!(
                                        "✅ Saved document '{}' to persistence layer",
                                        document.id
                                    );
                                }

                                // Validate document has content before indexing into rolegraph
                                if document.body.is_empty() {
                                    log::warn!(
                                        "Document '{}' has empty body, cannot properly index into rolegraph",
                                        filename
                                    );
                                } else {
                                    log::debug!(
                                        "Document '{}' has {} chars of body content",
                                        filename,
                                        document.body.len()
                                    );
                                }

                                // Then add to rolegraph for KG indexing using the same normalized ID
                                let document_clone = document.clone();
                                rolegraph_with_docs.insert_document(&normalized_id, document);

                                // Log rolegraph statistics after insertion
                                let node_count = rolegraph_with_docs.get_node_count();
                                let edge_count = rolegraph_with_docs.get_edge_count();
                                let doc_count = rolegraph_with_docs.get_document_count();

                                log::info!(
                                    "✅ Indexed document '{}' into rolegraph (body: {} chars, nodes: {}, edges: {}, docs: {})",
                                    filename,
                                    document_clone.body.len(),
                                    node_count,
                                    edge_count,
                                    doc_count
                                );
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
                                if let Some(ext) = entry.path().extension()
                                    && (ext == "md" || ext == "markdown")
                                    && let Ok(content) =
                                        tokio::fs::read_to_string(&entry.path()).await
                                {
                                    // Create a proper description from the document content
                                    let description = create_document_description(&content);

                                    // Use normalized ID to match what persistence layer uses
                                    let filename = entry.file_name().to_string_lossy().to_string();
                                    let normalized_id = {
                                        NORMALIZE_REGEX.replace_all(&filename, "").to_lowercase()
                                    };

                                    // Skip if this is already a KG document (avoid duplicates)
                                    if let Some(kg_local) = &kg.knowledge_graph_local
                                        && entry.path().starts_with(&kg_local.path)
                                    {
                                        continue; // Skip KG files, already processed above
                                    }

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
                                        doc_type: terraphim_types::DocumentType::KgEntry,
                                        synonyms: None,
                                        route: None,
                                        priority: None,
                                        quality_score: None,
                                    };

                                    // Save document to persistence layer
                                    if let Err(e) = document.save().await {
                                        log::debug!(
                                            "Failed to save haystack document '{}' to persistence: {}",
                                            document.id,
                                            e
                                        );
                                    } else {
                                        log::debug!(
                                            "✅ Saved haystack document '{}' to persistence layer",
                                            document.id
                                        );
                                        processed_count += 1;
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
                    local_rolegraphs
                        .insert(role_name.clone(), RoleGraphSync::from(rolegraph_with_docs));
                    log::info!("Stored rolegraph in local map for role '{}'", role_name);
                }
                Err(e) => {
                    log::error!("Failed to build thesaurus for role '{}': {}", role_name, e);
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

    let app = build_router(app_state, tx, summarization_manager, true);

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

#[cfg(test)]
mod tests {
    use super::*;

    // --- floor_char_boundary ---

    #[test]
    fn floor_char_boundary_index_beyond_len_returns_len() {
        let s = "hello";
        assert_eq!(s.floor_char_boundary(100), s.len());
    }

    #[test]
    fn floor_char_boundary_index_equal_to_len_returns_len() {
        let s = "hello";
        assert_eq!(s.floor_char_boundary(5), 5);
    }

    #[test]
    fn floor_char_boundary_index_at_valid_ascii_boundary() {
        let s = "hello";
        assert_eq!(s.floor_char_boundary(3), 3);
    }

    #[test]
    fn floor_char_boundary_zero_index_returns_zero() {
        let s = "hello";
        assert_eq!(s.floor_char_boundary(0), 0);
    }

    #[test]
    fn floor_char_boundary_empty_string_returns_zero() {
        assert_eq!("".floor_char_boundary(0), 0);
        assert_eq!("".floor_char_boundary(5), 0);
    }

    #[test]
    fn floor_char_boundary_multibyte_char_at_boundary() {
        // "é" encodes as 2 bytes: [0xC3, 0xA9]; "aé".len() == 3
        let s = "aé";
        assert_eq!(s.floor_char_boundary(1), 1); // byte 1 = start of 'é' — valid
        assert_eq!(s.floor_char_boundary(2), 1); // byte 2 = inside 'é' — retreat to 1
        assert_eq!(s.floor_char_boundary(3), 3); // index >= len — returns len
    }

    #[test]
    fn floor_char_boundary_inside_multibyte_char_retreats() {
        // CJK character '中' encodes as 3 bytes: [0xE4, 0xB8, 0xAD]
        let s = "a中b";
        // byte index 2 and 3 are inside '中', so floor should return 1 (the 'a' boundary)
        assert_eq!(s.floor_char_boundary(2), 1);
        assert_eq!(s.floor_char_boundary(3), 1);
        // byte index 4 is the start of 'b', valid boundary
        assert_eq!(s.floor_char_boundary(4), 4);
    }

    #[test]
    fn floor_char_boundary_emoji_inside_retreats() {
        // "🦀" encodes as 4 bytes
        let s = "x🦀y";
        // byte indices 2 and 3 are inside the emoji (starts at 1)
        assert_eq!(s.floor_char_boundary(2), 1);
        assert_eq!(s.floor_char_boundary(3), 1);
        // byte index 5 is the start of 'y'
        assert_eq!(s.floor_char_boundary(5), 5);
    }

    // --- create_document_description ---

    #[test]
    fn create_document_description_empty_content_returns_none() {
        assert_eq!(create_document_description(""), None);
    }

    #[test]
    fn create_document_description_only_whitespace_returns_none() {
        assert_eq!(create_document_description("   \n\n  "), None);
    }

    #[test]
    fn create_document_description_only_frontmatter_returns_none() {
        let content = "---\ntitle: My Doc\nauthor: Alice\n---\n";
        assert_eq!(create_document_description(content), None);
    }

    #[test]
    fn create_document_description_simple_paragraph() {
        let content = "This is a simple paragraph that is long enough to be included.";
        let result = create_document_description(content);
        assert!(result.is_some());
        assert!(result.unwrap().contains("simple paragraph"));
    }

    #[test]
    fn create_document_description_header_included() {
        let content = "# My Great Document\n\nSome body text here that is informative.";
        let result = create_document_description(content);
        assert!(result.is_some());
        let desc = result.unwrap();
        assert!(
            desc.contains("My Great Document"),
            "Expected header in description, got: {desc}"
        );
    }

    #[test]
    fn create_document_description_synonyms_line_included() {
        let content = "synonyms:: foo, bar, baz";
        let result = create_document_description(content);
        assert!(result.is_some());
        let desc = result.unwrap();
        assert!(desc.contains("synonyms: foo, bar, baz"), "Got: {desc}");
    }

    #[test]
    fn create_document_description_tags_line_skipped() {
        let content = "tags:: rust, programming\n\nActual description text that is long enough.";
        let result = create_document_description(content);
        assert!(result.is_some());
        let desc = result.unwrap();
        assert!(
            !desc.contains("tags::"),
            "tags:: line should be excluded, got: {desc}"
        );
    }

    #[test]
    fn create_document_description_image_references_skipped() {
        let content = "![logo](img/logo.png)\n\nActual description text that is long enough.";
        let result = create_document_description(content);
        assert!(result.is_some());
        let desc = result.unwrap();
        assert!(
            !desc.contains("!["),
            "Image reference should be excluded, got: {desc}"
        );
    }

    #[test]
    fn create_document_description_code_block_skipped() {
        let content = "```rust\nfn main() {}\n```\n\nActual description text that is long enough.";
        let result = create_document_description(content);
        // code block lines starting with ``` are skipped
        if let Some(desc) = result {
            assert!(
                !desc.contains("```"),
                "Code fence should be excluded, got: {desc}"
            );
        }
    }

    #[test]
    fn create_document_description_html_comment_skipped() {
        let content = "<!-- This is a comment -->\n\nReal content that is long enough here.";
        let result = create_document_description(content);
        assert!(result.is_some());
        let desc = result.unwrap();
        assert!(
            !desc.contains("<!--"),
            "HTML comment should be excluded, got: {desc}"
        );
    }

    #[test]
    fn create_document_description_long_content_truncated_at_valid_boundary() {
        // Build content that exceeds 400 characters
        let long_line = "a".repeat(500);
        let result = create_document_description(&long_line);
        assert!(result.is_some());
        let desc = result.unwrap();
        assert!(
            desc.len() <= 403,
            "Truncated description should be <=403 chars, got {}",
            desc.len()
        );
        assert!(
            desc.ends_with("..."),
            "Truncated description should end with '...', got: {desc}"
        );
    }

    #[test]
    fn create_document_description_long_unicode_truncated_at_char_boundary() {
        // Build a string of 200 multibyte chars — each '中' is 3 bytes, so 200*3=600 bytes > 400
        let content = "中".repeat(200);
        let result = create_document_description(&content);
        assert!(result.is_some());
        let desc = result.unwrap();
        // Must be valid UTF-8 (would panic if not)
        let _ = desc.len();
        assert!(
            desc.ends_with("..."),
            "Should be truncated with '...', got: {desc}"
        );
    }

    #[test]
    fn create_document_description_short_lines_ignored() {
        // Lines shorter than 20 chars are not included as content
        let content = "short\nalso short\n\nThis line is long enough to qualify as content for the description.";
        let result = create_document_description(content);
        assert!(result.is_some());
        let desc = result.unwrap();
        assert!(
            !desc.contains("short"),
            "Short lines should be ignored, got: {desc}"
        );
    }

    #[test]
    fn create_document_description_combines_header_and_body() {
        let content = "# Knowledge Graph\n\nThis document describes the core concepts in the knowledge graph system.";
        let result = create_document_description(content);
        assert!(result.is_some());
        let desc = result.unwrap();
        assert!(desc.contains("Knowledge Graph"), "Got: {desc}");
        assert!(desc.contains("core concepts"), "Got: {desc}");
    }

    #[test]
    fn create_document_description_at_most_four_parts() {
        // Provide many qualifying lines; ensure we stop collecting after 4 parts
        let content = "# Header\n".to_string()
            + &"This is a meaningful content line that is long enough. ".repeat(10);
        let result = create_document_description(&content);
        assert!(result.is_some());
        // Result should not be unbounded — capped by the length limit or content_lines<3
        let desc = result.unwrap();
        assert!(!desc.is_empty());
    }
}

/// Constructs a minimal Axum router suitable for integration tests.
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

    build_router(app_state, tx, summarization_manager, false)
}
