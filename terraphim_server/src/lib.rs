use axum::{
    http::{header, Method, StatusCode, Uri},
    response::{Html, IntoResponse, Response},
    routing::{get, post},
    Extension, Router,
};
use std::net::SocketAddr;
mod api;
use api::{create_document, health_axum, search_documents, search_documents_post};
use rust_embed::RustEmbed;
use terraphim_config::ConfigState;
use terraphim_types::IndexedDocument;
use tokio::sync::broadcast::channel;
use tower_http::cors::{Any, CorsLayer};

mod error;

pub use error::Result;

// use axum_embed::ServeEmbed;
static INDEX_HTML: &str = "index.html";

#[derive(RustEmbed, Clone)]
#[folder = "dist/"]
struct Assets;

pub async fn axum_server(server_hostname: SocketAddr, config_state: ConfigState) -> Result<()> {
    log::info!("Starting axum server");
    // let assets = axum_embed::ServeEmbed::<Assets>::with_parameters(Some("index.html".to_owned()),axum_embed::FallbackBehavior::Ok, Some("index.html".to_owned()));
    let (tx, _rx) = channel::<IndexedDocument>(10);

    let app = Router::new()
        .route("/health", get(health_axum))
        // .route("/articles", get(list_articles))
        .route("/article", post(create_document))
        .route("/article/", post(create_document))
        .route("/articles/search", get(search_documents))
        .route("/articles/search", post(search_documents_post))
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
