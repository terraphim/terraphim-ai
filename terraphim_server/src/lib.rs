use axum::{
    http::{header,Method,StatusCode,Uri},
    routing::{get, post},
    Extension, Router,
    response::{Html, IntoResponse, Response},
};

use std::net::SocketAddr;
use tower::ServiceExt;
mod api;
use api::{create_article, health_axum, search_articles, search_articles_post};
use portpicker;
use terraphim_pipeline::IndexedDocument;
use terraphim_types as types;
use tokio::sync::broadcast::channel;
use tower_http::cors::{Any, CorsLayer};
use tower_http::{
    services::{ServeDir, ServeFile},
    trace::TraceLayer,
};

mod error;

pub use error::Result;

use rust_embed::RustEmbed;
// use axum_embed::ServeEmbed;
static INDEX_HTML: &str = "index.html";

#[derive(RustEmbed, Clone)]
#[folder = "dist/"]
struct Assets;


pub async fn axum_server(server_hostname: SocketAddr, config_state: types::ConfigState) {

    // let assets = axum_embed::ServeEmbed::<Assets>::with_parameters(Some("index.html".to_owned()),axum_embed::FallbackBehavior::Ok, Some("index.html".to_owned()));
    let (tx, _rx) = channel::<IndexedDocument>(10);

    let app = Router::new()
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

    println!("listening on {server_hostname}");
    let listener = tokio::net::TcpListener::bind(server_hostname).await.unwrap();
    axum::serve(listener, app).await.unwrap();

    // axum::Server::bind(&server_hostname)
    //     .serve(app.into_make_service())
    //     .await
    //     .unwrap_or_else(|_| {
    //         // FIXME: this unwrap is never reached
    //         // the desired behavior is to pick a new port and try again
    //         let port = portpicker::pick_unused_port().expect("failed to find unused port");
    //         let add = SocketAddr::from(([127, 0, 0, 1], port));
    //         println!("listening on {add} with {port}");
    //     });
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
