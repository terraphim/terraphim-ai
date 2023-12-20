//! terraphim API (AXUM) server
#![warn(clippy::all, clippy::pedantic)]
#![warn(
    absolute_paths_not_starting_with_crate,
    rustdoc::invalid_html_tags,
    missing_copy_implementations,
    missing_debug_implementations,
    semicolon_in_expressions_from_macros,
    unreachable_pub,
    unused_extern_crates,
    variant_size_differences,
    clippy::missing_const_for_fn
)]
#![deny(anonymous_parameters, macro_use_extern_crate, pointer_structural_match)]
// #![deny(missing_docs)]

use std::{error::Error, net::SocketAddr};
use axum::{
    error_handling::HandleErrorLayer,
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router
};
use ulid::Ulid;
use std::sync::Arc;
use tokio::sync::Mutex;
use anyhow::{Context, Result};


mod types;

use utoipa::{
   OpenApi,
};

use utoipa_rapidoc::RapiDoc;
use utoipa_redoc::{Redoc, Servable};
use utoipa_swagger_ui::SwaggerUi;


use terraphim_pipeline::{RoleGraph, IndexedDocument};



pub(crate) struct RoleGraphState {
    /// RoleGraph for ingesting documents
    pub(crate) rolegraph: Mutex<RoleGraph>,
}

#[derive(OpenApi)]
#[openapi(paths(health_axum, create_article, search_articles), components(schemas(types::Article)))]
pub struct ApiDoc;

#[utoipa::path(
    get,
    path = "/health",
    responses(
        (status = 200, description = "Health Check")
    )
)]
pub async fn health_axum() -> impl IntoResponse {
    (StatusCode::OK, "OK")
}

#[utoipa::path(
    post,
    path = "/article",
    request_body = Article,
    responses(
        (status = 201, description = "Article item created successfully", body = types::Article),
        (status = 409, description = "Create failed")
    )
)]
async fn create_article(State(rolegraph): State<Arc<Mutex<RoleGraph>>>,Json(article): Json<types::Article>) -> impl IntoResponse {
    log::warn!("create_article");

    log::warn!("create document");
    let id = Ulid::new().to_string();

    let mut rolegraph = rolegraph.lock().await;
    rolegraph.parse_document(id.clone(), article.clone());

    log::warn!("send response");
    let response= Json(article);
    (StatusCode::CREATED, response)
}

#[utoipa::path(
    get,
    path = "/article",
    responses(
        (status = 200, description = "List all articles successfully", body = [types::Article]),
    )
)]
async fn list_articles(State(rolegraph): State<Arc<Mutex<RoleGraph>>>) -> impl IntoResponse {

    let rolegraph = rolegraph.lock().await.clone();
    println!("{:?}",rolegraph);

    (StatusCode::OK, Json("Ok"))
}

    /// Search Todos by query params.
    ///
    /// Search `Todo`s by query params and return matching `Todo`s.
    #[utoipa::path(
        get,
        path = "/articles/search",
        params(
            types::SearchQuery
        ),
        responses(
            (status = 200, description = "List matching articles by query", body = [types::Article]),
        )
    )]
async fn search_articles(State(rolegraph): State<Arc<Mutex<RoleGraph>>>,search_query: Query<types::SearchQuery>) -> Json<Vec<IndexedDocument>>{
    // Here you would normally search the articles in the database
    println!("Searching articles with query: {:?}", search_query);
    let rolegraph = rolegraph.lock().await;
    let documents: Vec<(&String, IndexedDocument)> =
    match rolegraph.query(&search_query.search_term) {
        Ok(docs) => docs,
        Err(e) => {
            log::error!("Error: {}", e);
            return Json(vec![]);
        }
    };
    let docs: Vec<IndexedDocument> = documents.into_iter().map(|(_id, doc) | doc).collect();
    println!("Found articles: {:?}", docs);
    Json(docs)
}


#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let addr = SocketAddr::from(([127, 0, 0, 1], 8000));
    let role = "system operator".to_string();
    // let automata_url = "https://system-operator.s3.eu-west-2.amazonaws.com/term_to_id.json";
    let automata_url = "./data/term_to_id.json";
    let rolegraph = RoleGraph::new(role, automata_url)
        .context(format!("Failed to create rolegraph from {automata_url}"))?;
    let rolegraph = Mutex::new(rolegraph);
    let rolegraph = Arc::new(rolegraph);

    let app = Router::new()
        .route("/", get(health_axum))
        .route("/articles", get(list_articles))
        .route("/article", post(create_article))
        .route("/articles/search", get(search_articles))
        // .route("/articles/search", routing::get(search_articles))
        .merge(Redoc::with_url("/redoc", ApiDoc::openapi()))
        // There is no need to create `RapiDoc::with_openapi` because the OpenApi is served
        // via SwaggerUi instead we only make rapidoc to point to the existing doc.
        .merge(RapiDoc::new("/api-docs/openapi.json").path("/rapidoc"))
        .merge(SwaggerUi::new("/swagger-ui").url("/api-doc/openapi.json", ApiDoc::openapi()))
        .with_state(rolegraph);

    println!("listening on {}", addr);

    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();

    Ok(())
}