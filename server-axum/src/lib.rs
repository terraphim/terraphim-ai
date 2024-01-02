use std::{error::Error, net::SocketAddr};
use axum::{
    extract::{Query, State},
    http::StatusCode,
    http::Method,
    response::{sse::Event, IntoResponse, Sse, Response},
    routing::{get, post},
    Json, Router, Extension
};

mod api;
use terraphim_types as types;
use api::{create_article,search_articles,show_config,health_axum, search_articles_stream, search_articles_post};
use terraphim_pipeline::IndexedDocument;
use tokio::sync::broadcast::channel;
use portpicker;
use tower_http::cors::{Any, CorsLayer};


pub async fn axum_server(server_hostname:SocketAddr, config_state:types::ConfigState) {
    let (tx, _rx) = channel::<IndexedDocument>(10);
    let app = Router::new()
    .route("/", get(health_axum))
    .route("/health", get(health_axum))
    // .route("/articles", get(list_articles))
    .route("/article", post(create_article))
    .route("/article/", post(create_article))
    .route("/articles/search", get(search_articles))
    .route("/articles/search", post(search_articles_post))
    .route("/config", get(api::show_config))
    .route("/config/", get(api::show_config))
    .route("/config", post(api::update_config))
    .route("/config/", post(api::update_config))
    .route("/articles/search/stream", get(search_articles_stream))
    .with_state(config_state)
    .layer(Extension(tx))
    .layer(CorsLayer::new().allow_origin(Any).allow_headers(Any).allow_methods(vec![
        Method::GET,
        Method::POST,
        Method::PUT,
        Method::PATCH,
        Method::DELETE,
    ]));

     println!("listening on {server_hostname}");
     axum::Server::bind(&server_hostname)
     .serve(app.into_make_service())
     .await
     .unwrap_or_else(|_| {
        // FIXME: this unwrap is never reached
        // the desired behavior is to pick a new port and try again
        let port = portpicker::pick_unused_port().expect("failed to find unused port");
        let add=SocketAddr::from(([127, 0, 0, 1], port));
        println!("listening on {add} with {port}");
    });
     
    
}
