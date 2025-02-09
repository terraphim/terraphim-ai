use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
    Extension, Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::broadcast::Sender;
use tokio::sync::Mutex;

use terraphim_config::Config;
use terraphim_config::ConfigState;
use terraphim_rolegraph::RoleGraph;
use terraphim_service::TerraphimService;
use terraphim_types::{RankedNode, Rank, Document, IndexedDocument, SearchQuery};

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

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ListNodesQuery {
    /// Whether to expand nodes to include linked documents
    #[serde(default)]
    pub expand: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ListDocumentsResponse {
    /// Status of the document listing
    pub status: Status,
    /// Vector of ranked nodes from the RoleGraph
    pub nodes: Vec<RankedNode>,
    /// Vector of documents with their IDs and ranks, only present when expand=true
    #[serde(skip_serializing_if = "Option::is_none")]
    pub documents: Option<Vec<(String, IndexedDocument, Rank)>>,
    /// Total number of nodes
    pub total: usize,
}

pub(crate) async fn list_ranked_nodes(
    State(config): State<ConfigState>,
    Query(query): Query<ListNodesQuery>,
) -> Result<Json<ListDocumentsResponse>> {
    let mut terraphim_service = TerraphimService::new(config);
    let rolegraph = terraphim_service.get_rolegraph().await?;
    let nodes = rolegraph.list_ranked_nodes()?;
    let total = nodes.len();

    log::debug!("Found {total} ranked nodes");
    
    let documents = if query.expand {
        let docs = rolegraph.get_ranked_documents(&nodes)?;
        log::debug!("Expanded to {} documents", docs.len());
        Some(docs)
    } else {
        None
    };

    Ok(Json(ListDocumentsResponse {
        status: Status::Success,
        nodes,
        documents,
        total,
    }))
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
pub(crate) async fn update_config(
    State(config_state): State<ConfigState>,
    Json(config_new): Json<Config>,
) -> Json<ConfigResponse> {
    let mut config = config_state.config.lock().await;
    *config = config_new.clone();
    Json(ConfigResponse {
        status: Status::Success,
        config: config_new,
    })
}
