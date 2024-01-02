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
    extract::{Query, State},
    http::StatusCode,
    response::{sse::Event, IntoResponse, Sse, Response},
    routing::{get, post},
    Json, Router, Extension
};
use ulid::Ulid;
use std::sync::Arc;
use tokio::sync::Mutex;
use anyhow::{Result};
use terraphim_config::TerraphimConfig;
use terraphim_settings::Settings;

use terraphim_server::axum_server;
use terraphim_types as types;



/// TODO: Can't get Open API docs to work with axum consistencly,given up for now.

use terraphim_pipeline::RoleGraph;
use portpicker;


#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let server_settings = Settings::load_from_env_and_file(None);
    println!("Device settings hostname: {:?}", server_settings.server_hostname);
    let server_hostname = server_settings.server_hostname.unwrap().parse::<SocketAddr>().unwrap_or_else(|_| {
        let port = portpicker::pick_unused_port().expect("failed to find unused port");
        SocketAddr::from(([127, 0, 0, 1], port))
    });
    let mut config_state= types::ConfigState::new().await?;

    // Add one more for testing local KG

    let addr = server_hostname;
    let role = "system operator".to_string();
    // let automata_url = "https://system-operator.s3.eu-west-2.amazonaws.com/term_to_id.json";
    let automata_url = "./data/term_to_id.json";
    let rolegraph = RoleGraph::new(role.clone(), automata_url).await?;        
    config_state.roles.insert(role, types::RoleGraphState {
        rolegraph: Arc::new(Mutex::new(rolegraph))
    });
    println!("cfg Roles: {:?}", config_state.roles.keys().collect::<Vec<&String>>());
    axum_server(addr, config_state).await;

    Ok(())
}

#[cfg(test)]
mod tests {
    use terraphim_config::TerraphimConfig;
    use tokio::test;
    use reqwest::{StatusCode, Client};
    
    #[test]
    async fn test_search_articles() {
        
            let response = reqwest::get("http://localhost:8000/articles/search?search_term=trained%20operators%20and%20maintainers&skip=0&limit=10&role=system%20operator").await.unwrap();
            assert_eq!(response.status(), StatusCode::OK);
            // You can also test the response body if you want:
            // let body = response.text().await.unwrap();
            // assert!(body.contains("expected content"));
        
    }

    // test search article with POST method 
    #[test]
    async fn test_post_search_article(){

        let client = Client::new();
        let response = client.post("http://localhost:8000/articles/search")
            .header("Content-Type", "application/json")
            .body(r#"
            {
                "search_term": "trained operators and maintainers",
                "skip": 0,
                "limit": 10,
                "role": "system operator"
            }
            "#)
            .send()
            .await
            .unwrap();
        println!("response: {:?}", response);
            assert_eq!(response.status(), StatusCode::OK);
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
        
            let response = reqwest::get("http://localhost:8000/config/").await.unwrap();
            assert_eq!(response.status(), StatusCode::OK);
            // You can also test the response body if you want:
            // let body = response.text().await.unwrap();
            // assert!(body.contains("expected content"));
        
    }

    /// test update config 
    #[test]
    async fn test_post_config(){
        use serde_json::json;
        let response = reqwest::get("http://localhost:8000/config/").await.unwrap();
        let orig_config: TerraphimConfig = response.json().await.unwrap();
        println!("orig_config: {:?}", orig_config);
        let mut new_config = orig_config.clone();
        new_config.default_role = "system operator".to_string();
        new_config.global_shortcut= "Ctrl+X".to_string();
        println!("new_config: {:?}", new_config);
        let client = Client::new();
        let response = client.post("http://localhost:8000/config/")
            .header("Content-Type", "application/json")
            .header("Accept", "application/json")
            .body(serde_json::to_string(&new_config).unwrap())
            .send()
            .await
            .unwrap();
        println!("response: {:?}", response);
        assert_eq!(response.status(), StatusCode::OK);


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
