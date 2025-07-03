use std::net::SocketAddr;

use axum::{
    http::{header, Method, StatusCode, Uri},
    response::{Html, IntoResponse, Response},
    routing::{get, post},
    Extension, Router,
};
use rust_embed::RustEmbed;
use terraphim_automata::builder::{Logseq, ThesaurusBuilder};
use terraphim_config::ConfigState;
use terraphim_types::IndexedDocument;
use tokio::sync::broadcast::channel;
use tower_http::cors::{Any, CorsLayer};
use terraphim_rolegraph::{RoleGraph, RoleGraphSync};
use terraphim_types::{RelevanceFunction, Document};

mod api;
mod error;

use api::{create_document, health, search_documents, search_documents_post, get_rolegraph};
pub use api::{ConfigResponse, CreateDocumentResponse, SearchResponse};
pub use error::{Result, Status};

// use axum_embed::ServeEmbed;
static INDEX_HTML: &str = "index.html";

#[derive(RustEmbed)]
#[folder = "../desktop/dist"]
struct Assets;

pub async fn axum_server(server_hostname: SocketAddr, mut config_state: ConfigState) -> Result<()> {
    log::info!("Starting axum server");
    
    let mut config = config_state.config.lock().await.clone();
    let mut local_rolegraphs = ahash::AHashMap::new();
    
    for (role_name, role) in &mut config.roles {
        if role.relevance_function == RelevanceFunction::TerraphimGraph {
            if let Some(kg) = &role.kg {
                if kg.automata_path.is_none() && kg.knowledge_graph_local.is_some() {
                    log::info!("Building rolegraph for role '{}' from local files", role_name);
                    
                    let kg_local = kg.knowledge_graph_local.as_ref().unwrap();
                    log::info!("Knowledge graph path: {:?}", kg_local.path);
                    
                    // Check if the directory exists
                    if !kg_local.path.exists() {
                        log::warn!("Knowledge graph directory does not exist: {:?}", kg_local.path);
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
                    
                    log::info!("Found {} markdown files in {:?}", files.len(), kg_local.path);
                    for file in &files {
                        log::info!("  - {:?}", file.path());
                    }
                    
                    // Build thesaurus using Logseq builder
                    let builder = Logseq::default();
                    log::info!("Created Logseq builder for path: {:?}", kg_local.path);
                    
                    match builder.build(role_name.to_string(), kg_local.path.clone()).await {
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
                                            if let Ok(content) = tokio::fs::read_to_string(&entry.path()).await {
                                                let document = Document {
                                                    id: entry.file_name().to_string_lossy().to_string(),
                                                    url: entry.path().to_string_lossy().to_string(),
                                                    title: entry.file_name().to_string_lossy().to_string(),
                                                    body: content,
                                                    description: None,
                                                    stub: None,
                                                    tags: None,
                                                    rank: None,
                                                };
                                                
                                                rolegraph_with_docs.insert_document(&document.id.clone(), document);
                                                log::info!("Indexed document '{}' into rolegraph", entry.file_name().to_string_lossy());
                                            }
                                        }
                                    }
                                }
                            }
                            
                            // Store in local rolegraphs map
                            local_rolegraphs.insert(role_name.clone(), RoleGraphSync::from(rolegraph_with_docs));
                            log::info!("Stored rolegraph in local map for role '{}'", role_name);
                        }
                        Err(e) => {
                            log::error!("Failed to build thesaurus for role '{}': {}", role_name, e);
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

    let app = Router::new()
        .route("/health", get(health))
        // .route("/documents", get(list_documents))
        .route("/documents", post(create_document))
        .route("/documents/", post(create_document))
        .route("/documents/search", get(search_documents))
        .route("/documents/search", post(search_documents_post))
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
        .fallback(static_handler)
        .with_state(config_state)
        .layer(Extension(tx))
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_headers(Any)
                .allow_methods(vec![
                    Method::GET,
                    Method::POST,
                    Method::PUT,
                    Method::PATCH,
                    Method::DELETE,
                ]),
        );

    // Note: Prefixing the host with `http://` makes the URL clickable in some terminals
    println!("listening on http://{server_hostname}");

    // This is the new way to start the server
    // However, we can't use it yet, because some crates have not updated
    // to `http` 1.0.0 yet.
    // let listener = tokio::net::TcpListener::bind(server_hostname).await?;
    // axum::serve(listener, app).await?;

    axum::Server::bind(&server_hostname)
        .serve(app.into_make_service())
        .await?;

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
