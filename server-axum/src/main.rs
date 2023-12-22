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

use std::{error::Error, net::SocketAddr, collections::HashMap};
use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router
};
use ulid::Ulid;
use std::sync::Arc;
use tokio::sync::Mutex;
use anyhow::{Result};
use terraphim_config::TerraphimConfig;
use terraphim_settings::Settings;

mod types;

/// TODO: Can't get Open API docs to work with axum consistencly,given up for now.

use utoipa::{
   OpenApi,
};






use terraphim_pipeline::{RoleGraph, IndexedDocument};

#[derive(Debug, Clone)]
pub(crate) struct ConfigState {
    /// Terraphim Config
    pub(crate) config: Arc<Mutex<TerraphimConfig>>,
    pub(crate) roles: HashMap<String, RoleGraphState>
}

#[derive(Debug, Clone)]
pub(crate) struct RoleGraphState {
    /// RoleGraph for ingesting documents
    pub(crate) rolegraph: Arc<Mutex<RoleGraph>>,
}

#[derive(OpenApi, Debug)]
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
/// Creates index of the article for each rolegraph
async fn create_article(State(config): State<ConfigState>,Json(article): Json<types::Article>) -> impl IntoResponse {
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

#[utoipa::path(
    get,
    path = "/article",
    responses(
        (status = 200, description = "List all articles successfully", body = [types::Article]),
    )
)]
async fn list_articles(State(rolegraph): State<Arc<Mutex<RoleGraph>>>) -> impl IntoResponse {

    let rolegraph = rolegraph.lock().await.clone();
    println!("{rolegraph:?}");

    (StatusCode::OK, Json("Ok"))
}

    /// Search All TerraphimGraphs defined in a config by query params.
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
async fn search_articles(State(config): State<ConfigState>,search_query: Query<types::SearchQuery>) -> Json<Vec<IndexedDocument>>{
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
    Json(docs)
}

/// API handler for Terraphim Config
async fn show_config(State(config):State<ConfigState>)-> Json<TerraphimConfig> {
    let config=config.config.lock().await;
    Json(config.clone())
}

/// Search articles by query params, subscribe to results via websocket.
/// 
// async fn ws_handle(
//     Query(search_query): Query<types::SearchQuery>,
//     ws: WebSocketUpgrade,
// ) -> Response {
//     // do something with `params`

//     ws.on_upgrade(handle_socket)
// }


#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let server_settings = Settings::load_from_env_and_file(None);
    println!("Device settings hostname: {:?}", server_settings.server_hostname);
    let server_hostname = server_settings.server_hostname.unwrap().parse::<SocketAddr>().unwrap_or_else(|_| {
        SocketAddr::from(([127, 0, 0, 1], 8000))
    });
    let config=TerraphimConfig::new();
    let mut config_state= ConfigState {
        config: Arc::new(Mutex::new(config.clone())),
        roles: HashMap::new()
    };

    // for each role in a config initialize a rolegraph
    // and add it to the config state
    for (role_name,each_role) in config.roles {
        let automata_url= each_role.kg.automata_url.as_str();
        println!("{} - {}",role_name.clone(),automata_url);
        let rolegraph = RoleGraph::new(role_name.clone(), automata_url).await?;        
        config_state.roles.insert(role_name, RoleGraphState {
            rolegraph: Arc::new(Mutex::new(rolegraph))
        });

    }
    // Add one more for testing local KG
    

    
    let addr = server_hostname;
    let role = "system operator".to_string();
    // let automata_url = "https://system-operator.s3.eu-west-2.amazonaws.com/term_to_id.json";
    let automata_url = "./data/term_to_id.json";
    let rolegraph = RoleGraph::new(role.clone(), automata_url).await?;        
    config_state.roles.insert(role, RoleGraphState {
        rolegraph: Arc::new(Mutex::new(rolegraph))
    });
    println!("cfg Roles: {:?}", config_state.roles.keys().collect::<Vec<&String>>());
    let app = Router::new()
        .route("/", get(health_axum))
        // .route("/articles", get(list_articles))
        .route("/article", post(create_article))
        .route("/articles/search", get(search_articles))
        .route("/config", get(show_config))
        // .merge(Redoc::with_url("/redoc", ApiDoc::openapi()))
        // There is no need to create `RapiDoc::with_openapi` because the OpenApi is served
        // via SwaggerUi instead we only make rapidoc to point to the existing doc.
        // .merge(RapiDoc::new("/api-docs/openapi.json").path("/rapidoc"))
        // .merge(SwaggerUi::new("/swagger-ui").url("/api-doc/openapi.json", ApiDoc::openapi()))
        // .with_state(rolegraph)
        .with_state(config_state);

    println!("listening on {addr}");

    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();

    Ok(())
}

#[cfg(test)]
mod tests {
    use tokio::test;
    use reqwest::{StatusCode, Client};
    use tokio::runtime::Runtime;
    #[test]
    async fn test_search_articles() {
        
            let response = reqwest::get("http://localhost:8000/articles/search?search_term=trained%20operators%20and%20maintainers&skip=0&limit=10&role=system%20operator").await.unwrap();
            assert_eq!(response.status(), StatusCode::OK);
            // You can also test the response body if you want:
            // let body = response.text().await.unwrap();
            // assert!(body.contains("expected content"));
        
    }
    #[test]
    async fn test_search_articles_without_role() {
        
            let response = reqwest::get("http://localhost:8000/articles/search?search_term=trained%20operators%20and%20maintainers&skip=0&limit=10").await.unwrap();
            assert_eq!(response.status(), StatusCode::OK);
            // You can also test the response body if you want:
            // let body = response.text().await.unwrap();
            // assert!(body.contains("expected content"));
        
    }
    #[test]
    async fn test_search_articles_without_limit() {
        
            let response = reqwest::get("http://localhost:8000/articles/search?search_term=trained%20operators%20and%20maintainers&skip=0").await.unwrap();
            assert_eq!(response.status(), StatusCode::OK);
            // You can also test the response body if you want:
            // let body = response.text().await.unwrap();
            // assert!(body.contains("expected content"));
        
    }
    #[test]
    async fn test_get_config() {
        
            let response = reqwest::get("http://localhost:8000/config").await.unwrap();
            assert_eq!(response.status(), StatusCode::OK);
            // You can also test the response body if you want:
            // let body = response.text().await.unwrap();
            // assert!(body.contains("expected content"));
        
    }
    #[test]
    async fn test_post_article() {
        
        let client = Client::new();
        let response = client.post("http://localhost:8000/article")
            .header("Content-Type", "application/json")
            .body(r#"
            {
                "title": "Title of the article",
                "url": "url_of_the_article",
                "body": "I am a text with the word Life cycle concepts and bar and Trained operators and maintainers, some bingo words Paradigm Map and project planning, then again: some bingo words Paradigm Map and project planning, then repeats: Trained operators and maintainers, project direction"
            }
            "#)
            .send()
            .await
            .unwrap();
            assert_eq!(response.status(), StatusCode::CREATED);
            // You can also test the response body if you want:
            // let body = response.text().await.unwrap();
            // assert!(body.contains("expected content"));
        }
}
