use std::{error::Error, net::SocketAddr};
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
use terraphim_types as types;

/// health check endpoint
pub(crate) async fn health_axum() -> impl IntoResponse {
    (StatusCode::OK, "OK")
}
use log::{info, warn};

/// Creates index of the article for each rolegraph
pub(crate) async fn create_article(State(config): State<types::ConfigState>,Json(article): Json<types::Article>) -> impl IntoResponse {
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
    pub(crate) async fn search_articles(Extension(tx): Extension<SearchResultsStream>,State(config): State<types::ConfigState>,search_query: Query<types::SearchQuery>) -> Json<Vec<IndexedDocument>>{
    println!("Searching articles with query: {search_query:?}");
    let default_role = config.config.lock().await.default_role.clone();
    // if role is not provided, use the default role in the config
    let role = if search_query.role.is_none() {
        default_role.as_str()
    } else {
        search_query.role.as_ref().unwrap()
    };
    // let role = search_query.role.as_ref().unwrap();
    let rolegraph = config.roles.get(role).unwrap().rolegraph.lock().await;
    let documents: Vec<(&String, IndexedDocument)> =
    match rolegraph.query(&search_query.search_term, search_query.skip, search_query.limit) {
        Ok(docs) => docs,
        Err(e) => {
            log::error!("Error: {}", e);
            return Json(vec![]);
        }
    };
    
    let docs: Vec<IndexedDocument> = documents.into_iter().map(|(_id, doc) | doc).collect();
    println!("Found articles: {docs:?}");
    // send the results to the stream as well (only for testing)
    for doc in docs.iter() {
        if tx.send(doc.clone()).is_err() {
            eprintln!("Record with ID {} was created but nobody's listening to the stream!", doc.id);
        }
    }

    Json(docs)
}

    /// Search All TerraphimGraphs defined in a config by post params.
    /// FIXME: add title, url and body to search output
pub(crate) async fn search_articles_post(Extension(tx): Extension<SearchResultsStream>,State(config): State<types::ConfigState>,search_query: Json<types::SearchQuery>) -> Json<Vec<IndexedDocument>>{
    println!("Searching articles with query: {search_query:?}");
    let default_role = config.config.lock().await.default_role.clone();
    // if role is not provided, use the default role in the config
    let role = if search_query.role.is_none() {
        default_role.as_str()
    } else {
        search_query.role.as_ref().unwrap()
    };
    // let role = search_query.role.as_ref().unwrap();
    let rolegraph = config.roles.get(role).unwrap().rolegraph.lock().await;
    let documents: Vec<(&String, IndexedDocument)> =
    match rolegraph.query(&search_query.search_term, search_query.skip, search_query.limit) {
        Ok(docs) => docs,
        Err(e) => {
            log::error!("Error: {}", e);
            return Json(vec![]);
        }
    };
    
    let docs: Vec<IndexedDocument> = documents.into_iter().map(|(_id, doc) | doc).collect();
    println!("Found articles: {docs:?}");
    // send the results to the stream as well (only for testing)
    for doc in docs.iter() {
        if tx.send(doc.clone()).is_err() {
            eprintln!("Record with ID {} was created but nobody's listening to the stream!", doc.id);
        }
    }

    Json(docs)
}


/// API handler for Terraphim Config
pub(crate) async fn show_config(State(config):State<types::ConfigState>)-> Json<TerraphimConfig> {
    let config=config.config.lock().await;
    Json(config.clone())
}

use persistance::Persistable;
/// API handler for Terraphim Config update
    pub async fn update_config(State(config):State<types::ConfigState>,Json(config_new):Json<TerraphimConfig>)-> Json<TerraphimConfig> {
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


/// Search articles by query params, subscribe to results via SSE.
/// SSE is used to stream results to the client. Code for inspiration from [Shuttle Todo](https://github.com/joshua-mo-143/shuttle-axum-htmx-ex/blob/main/src/main.rs#L7)
// pub(crate) async fn search_articles_stream(Extension(tx): Extension<SearchResultsStream>,State(config): State<types::ConfigState>,search_query: Query<types::SearchQuery>) ->  Sse<impl Stream<Item = Result<Event, Infallible>>> {
    pub(crate) async fn search_articles_stream(Extension(tx): Extension<SearchResultsStream>,State(config): State<types::ConfigState>) ->  Sse<impl Stream<Item = Result<Event, Infallible>>> {
        let rx = tx.subscribe();
    
        let stream = BroadcastStream::new(rx);
        Sse::new(
            stream
                .map(|msg| {
                    let msg = msg.unwrap();
                    let json = format!("<div>{}</div>", json!(msg));
                    Event::default().data(json)
                })
                .map(Ok),
        )
        .keep_alive(
            axum::response::sse::KeepAlive::new()
                .interval(Duration::from_secs(600))
                .text("keep-alive-text"),
        )
    }