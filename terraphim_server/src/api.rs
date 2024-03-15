use anyhow::Context;
use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
    Extension, Json,
};
use std::ops::Deref;
use std::sync::Arc;
use terraphim_config::Config;
use terraphim_config::ConfigState;
use terraphim_middleware::search_haystacks;
use terraphim_rolegraph::RoleGraph;
use terraphim_types::{merge_and_serialize, Article, IndexedDocument, SearchQuery};
use tokio::sync::broadcast::Sender;
use tokio::sync::Mutex;

use crate::error::Result;

pub type SearchResultsStream = Sender<IndexedDocument>;

/// health check endpoint
pub(crate) async fn health_axum() -> impl IntoResponse {
    (StatusCode::OK, "OK")
}
/// Creates index of the article for each rolegraph
pub(crate) async fn create_article(
    State(mut config): State<ConfigState>,
    Json(article): Json<Article>,
) -> impl IntoResponse {
    log::warn!("create_article");
    config
        .index_article(&article)
        .await
        .expect("Failed to index article");
    log::warn!("send response");
    let response = Json(article);
    (StatusCode::CREATED, response)
}

pub(crate) async fn _list_articles(
    State(rolegraph): State<Arc<Mutex<RoleGraph>>>,
) -> impl IntoResponse {
    let rolegraph = rolegraph.lock().await.clone();
    println!("{rolegraph:?}");

    let articles = rolegraph.articles();


    (StatusCode::OK, Json(articles))
}

/// Search All TerraphimGraphs defined in a config by query params.
#[axum::debug_handler]
pub(crate) async fn search_articles(
    Extension(_tx): Extension<SearchResultsStream>,
    State(config_state): State<ConfigState>,
    search_query: Query<SearchQuery>,
) -> Result<Json<Vec<Article>>> {
    println!("Searching articles with query: {search_query:?}");
    let search_query = search_query.deref().clone();
    // Return on empty search term
    if search_query.search_term.is_empty() {
        log::debug!("Empty search term. Returning early");
        return Ok(Json(vec![]));
    }

    let cached_articles = search_haystacks(config_state.clone(), search_query.clone())
        .await
        .context(format!(
            "Failed to query haystack for `{}`",
            search_query.search_term
        ))?;
    let docs: Vec<IndexedDocument> = config_state.search_articles(search_query).await;
    let articles = merge_and_serialize(cached_articles, docs);
    log::trace!("Final articles: {articles:?}");
    Ok(Json(articles))
}

/// Search All TerraphimGraphs defined in a config by post params.
/// FIXME: add title, url and body to search output
pub(crate) async fn search_articles_post(
    Extension(_tx): Extension<SearchResultsStream>,
    State(config_state): State<ConfigState>,
    search_query: Json<SearchQuery>,
) -> Result<Json<Vec<Article>>> {
    log::debug!("Searching articles with query: {search_query:?}");
    let search_query = search_query.deref().clone();
    let cached_articles = search_haystacks(config_state.clone(), search_query.clone())
        .await
        .context(format!(
            "Failed to query haystack for `{}`",
            search_query.search_term
        ))?;
    let docs: Vec<IndexedDocument> = config_state.search_articles(search_query).await;
    let articles = merge_and_serialize(cached_articles, docs);
    log::trace!("Final articles: {articles:?}");
    Ok(Json(articles))
}

/// API handler for Terraphim Config
pub(crate) async fn show_config(State(config): State<ConfigState>) -> Json<Config> {
    let config = config.config.lock().await;
    Json(config.clone())
}

use persistence::Persistable;

/// API handler for Terraphim Config update
pub async fn update_config(
    State(config): State<ConfigState>,
    Json(config_new): Json<Config>,
) -> Result<Json<Config>> {
    println!("Updating config: {config_new:?}");
    // let config = TerraphimConfig::new();
    let mut config_state = config.config.lock().await;
    println!("Lock acquired");
    config_state.update(config_new.clone());
    config_state.save().await.context("Failed to save config")?;
    println!("Config updated");
    println!("Config: {config_state:?}");
    Ok(Json(config_state.clone()))
}
