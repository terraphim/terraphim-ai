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
use terraphim_types::{Article, IndexedDocument, SearchQuery};

use crate::error::Result;

pub type SearchResultsStream = Sender<IndexedDocument>;

/// health check endpoint
pub(crate) async fn health_axum() -> impl IntoResponse {
    (StatusCode::OK, "OK")
}
/// Creates index of the article for each rolegraph
pub(crate) async fn create_article(
    State(config): State<ConfigState>,
    Json(article): Json<Article>,
) -> impl IntoResponse {
    log::info!("create_article");
    let mut terraphim_service = TerraphimService::new(config.clone());
    let article = terraphim_service
        .create_article(article)
        .await
        .expect("Failed to create article");
    log::info!("send response");
    let response = Json(article);
    (StatusCode::CREATED, response)
}

pub(crate) async fn _list_articles(
    State(rolegraph): State<Arc<Mutex<RoleGraph>>>,
) -> impl IntoResponse {
    let rolegraph = rolegraph.lock().await.clone();
    println!("{rolegraph:?}");

    (StatusCode::OK, Json("Ok"))
}

/// Search All TerraphimGraphs defined in a config by query params.
#[axum::debug_handler]
pub(crate) async fn search_articles(
    Extension(_tx): Extension<SearchResultsStream>,
    State(config_state): State<ConfigState>,
    search_query: Query<SearchQuery>,
) -> Result<Json<Vec<Article>>> {
    log::info!("Search called with {:?}", search_query);
    let terraphim_service = TerraphimService::new(config_state);
    let articles = terraphim_service.search_articles(&search_query.0).await?;

    Ok(Json(articles))
}

/// Search All TerraphimGraphs defined in a config by post params.
/// FIXME: add title, url and body to search output
pub(crate) async fn search_articles_post(
    Extension(_tx): Extension<SearchResultsStream>,
    State(config_state): State<ConfigState>,
    search_query: Json<SearchQuery>,
) -> Result<Json<Vec<Article>>> {
    log::debug!("POST Searching articles with query: {search_query:?}");

    let terraphim_service = TerraphimService::new(config_state);
    let articles = terraphim_service.search_articles(&search_query.0).await?;
    log::trace!("Final articles: {articles:?}");

    Ok(Json(articles))
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
