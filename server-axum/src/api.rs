use std::{error::Error, net::SocketAddr, ops::Deref};
use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{sse::Event, IntoResponse, Sse, Response},
    routing::{get, post},
    Json, Router, Extension
};
use serde_json::json;
use ulid::Ulid;
use std::sync::Arc;
use tokio::sync::Mutex;
use anyhow::{Result};
use terraphim_config::TerraphimConfig;
use terraphim_settings::Settings;
use terraphim_pipeline::{RoleGraph, IndexedDocument};
use terraphim_types::{ConfigState, SearchQuery, Article, merge_and_serialize};
use std::collections::HashMap;
use terraphim_middleware::search_haystacks;

/// health check endpoint
pub(crate) async fn health_axum() -> impl IntoResponse {
    (StatusCode::OK, "OK")
}
use log::{info, warn};

/// Creates index of the article for each rolegraph
pub(crate) async fn create_article(State(config): State<ConfigState>,Json(article): Json<Article>) -> impl IntoResponse {
    log::warn!("create_article");
    let mut article = article.clone();
    let id = Ulid::new().to_string();
    let id = if article.id.is_none() {
        article.id = Some(id.clone());
        id
    } else {
        article.id.clone().unwrap()
    };
    for rolegraph_state in config.roles.values() {
        let mut rolegraph = rolegraph_state.rolegraph.lock().await;
        rolegraph.parse_document(id.clone(), article.clone());
    }
    log::warn!("send response");
    let response= Json(article.clone());
    (StatusCode::CREATED, response)
}


pub(crate) async fn list_articles(State(rolegraph): State<Arc<Mutex<RoleGraph>>>) -> impl IntoResponse {

    let rolegraph = rolegraph.lock().await.clone();
    println!("{rolegraph:?}");

    (StatusCode::OK, Json("Ok"))
}

    /// Search All TerraphimGraphs defined in a config by query params.
    pub(crate) async fn search_articles(Extension(tx): Extension<SearchResultsStream>,State(config_state): State<ConfigState>,search_query: Query<SearchQuery>) -> Json<Vec<Article>>{
    println!("Searching articles with query: {search_query:?}");
    let search_query = search_query.deref().clone();
    let articles_cached= search_haystacks(config_state.clone(),search_query.clone()).await;
    let docs: Vec<IndexedDocument> = config_state.search_articles(search_query).await.expect("Failed to search articles");
    let articles = merge_and_serialize(articles_cached, docs);
    println!("Articles: {articles:?}");
    Json(articles)
}

    /// Search All TerraphimGraphs defined in a config by post params.
    /// FIXME: add title, url and body to search output
pub(crate) async fn search_articles_post(Extension(tx): Extension<SearchResultsStream>,State(config_state): State<ConfigState>,search_query: Json<SearchQuery>) -> Json<Vec<Article>>{
    println!("Searching articles with query: {search_query:?}");
    let search_query = search_query.deref().clone();
    let articles_cached= search_haystacks(config_state.clone(),search_query.clone()).await;
    let docs: Vec<IndexedDocument> = config_state.search_articles(search_query).await.expect("Failed to search articles");
    let articles = merge_and_serialize(articles_cached, docs);
    println!("Articles: {articles:?}");
    Json(articles)
}


/// API handler for Terraphim Config
pub(crate) async fn show_config(State(config):State<ConfigState>)-> Json<TerraphimConfig> {
    let config=config.config.lock().await;
    Json(config.clone())
}

use persistance::Persistable;
/// API handler for Terraphim Config update
    pub async fn update_config(State(config):State<ConfigState>,Json(config_new):Json<TerraphimConfig>)-> Json<TerraphimConfig> {
    println!("Updating config: {config_new:?}");
    // let config = TerraphimConfig::new();
    let mut config_state=config.config.lock().await;
    println!("Lock acquired");
    config_state.update(config_new.clone());
    config_state.save().await.unwrap();
    println!("Config updated");
    println!("Config: {config_state:?}");
    Json(config_state.clone())
}

use std::convert::Infallible;
use std::time::Duration;

use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::{Stream, StreamExt as _};
use tokio::sync::broadcast::{channel, Sender, Receiver};

pub type SearchResultsStream = Sender<IndexedDocument>;


