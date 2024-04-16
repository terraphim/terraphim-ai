use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
    Extension, Json,
};
use std::sync::Arc;
use tokio::sync::broadcast::Sender;
use tokio::sync::Mutex;

use service::TerraphimService;
use terraphim_config::Config;
use terraphim_config::ConfigState;
use terraphim_rolegraph::RoleGraph;
use terraphim_types::{Document, IndexedDocument, SearchQuery};

use crate::error::Result;

pub type SearchResultsStream = Sender<IndexedDocument>;

pub(crate) async fn health_axum() -> impl IntoResponse {
    (StatusCode::OK, "OK")
}

/// Creates index of the document for each rolegraph
pub(crate) async fn create_document(
    State(config): State<ConfigState>,
    Json(document): Json<Document>,
) -> impl IntoResponse {
    log::info!("create_document");
    let mut terraphim_service = TerraphimService::new(config.clone());
    let document = terraphim_service
        .create_document(document)
        .await
        .expect("Failed to create document");
    log::info!("send response");
    let response = Json(document);
    (StatusCode::CREATED, response)
}

pub(crate) async fn _list_documents(
    State(rolegraph): State<Arc<Mutex<RoleGraph>>>,
) -> impl IntoResponse {
    let rolegraph = rolegraph.lock().await.clone();
    log::debug!("{rolegraph:?}");

    (StatusCode::OK, Json("Ok"))
}

/// Search All TerraphimGraphs defined in a config by query params.
pub(crate) async fn search_documents(
    Extension(_tx): Extension<SearchResultsStream>,
    State(config_state): State<ConfigState>,
    search_query: Query<SearchQuery>,
) -> Result<Json<Vec<Document>>> {
    log::info!("Search called with {:?}", search_query);
    let terraphim_service = TerraphimService::new(config_state);
    let documents = terraphim_service.search_documents(&search_query.0).await?;

    Ok(Json(documents))
}

/// Search All TerraphimGraphs defined in a config by post params.
/// FIXME: add title, url and body to search output
pub(crate) async fn search_documents_post(
    Extension(_tx): Extension<SearchResultsStream>,
    State(config_state): State<ConfigState>,
    search_query: Json<SearchQuery>,
) -> Result<Json<Vec<Document>>> {
    log::info!("POST Searching documents with query: {search_query:?}");

    let terraphim_service = TerraphimService::new(config_state);
    let documents = terraphim_service.search_documents(&search_query.0).await?;

    // Check if log level is debug:
    if log::log_enabled!(log::Level::Debug) {
        log::debug!("Documents found:");
        for document in &documents {
            log::debug!("{} -> {}", document.id, document.rank.unwrap());
        }
    }

    Ok(Json(documents))
}

/// API handler for Terraphim Config
pub(crate) async fn show_config(State(config): State<ConfigState>) -> Json<Config> {
    let terraphim_service = TerraphimService::new(config);
    let config = terraphim_service.fetch_config().await;

    Json(config)
}

/// API handler for Terraphim Config update
pub async fn update_config(
    State(config): State<ConfigState>,
    Json(config_new): Json<Config>,
) -> Result<Json<Config>> {
    let terraphim_service = TerraphimService::new(config);
    let config_state = terraphim_service.update_config(config_new).await?;

    Ok(Json(config_state.clone()))
}
