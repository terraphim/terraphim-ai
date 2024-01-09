use axum::{
    http::Method,
    routing::{get, post},
    Extension, Router,
};
use std::net::SocketAddr;

mod api;
use api::{create_article, health_axum, search_articles, search_articles_post};
use portpicker;
use terraphim_pipeline::IndexedDocument;
use terraphim_types as types;
use tokio::sync::broadcast::channel;
use tower_http::cors::{Any, CorsLayer};

mod error;

pub use error::Result;

pub async fn axum_server(server_hostname: SocketAddr, config_state: types::ConfigState) {
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

    println!("listening on {server_hostname}");
    axum::Server::bind(&server_hostname)
        .serve(app.into_make_service())
        .await
        .unwrap_or_else(|_| {
            // FIXME: this unwrap is never reached
            // the desired behavior is to pick a new port and try again
            let port = portpicker::pick_unused_port().expect("failed to find unused port");
            let add = SocketAddr::from(([127, 0, 0, 1], port));
            println!("listening on {add} with {port}");
        });
}
