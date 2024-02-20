//! terraphim API (AXUM) server
#![warn(
    clippy::all,
    clippy::pedantic,
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
// Optional: #![deny(missing_docs)]

use anyhow::Context;
use clap::Parser;
use std::net::SocketAddr;
use std::sync::Arc;
use terraphim_config::{ConfigState, ServiceType, TerraphimConfig};
use terraphim_pipeline::RoleGraphSync;
use terraphim_server::{axum_server, Result};
use terraphim_settings::Settings;
use terraphim_types as types;
use tokio::sync::Mutex;

/// TODO: Can't get Open API docs to work with axum consistently, given up for now.
use terraphim_pipeline::RoleGraph;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// String to search for
    #[arg(short, long)]
    search_term: Option<String>,

    /// Role to use for search
    #[arg(short, long)]
    role: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    println!("args: {args:?}");
    let server_settings =
        Settings::load_from_env_and_file(None).context("Failed to load settings")?;
    println!(
        "Device settings hostname: {:?}",
        server_settings.server_hostname
    );

    let server_hostname = server_settings
        .server_hostname
        .parse::<SocketAddr>()
        .unwrap_or_else(|_| {
            let port = portpicker::pick_unused_port().expect("Failed to find unused port");
            SocketAddr::from(([127, 0, 0, 1], port))
        });

    // TODO: make the service type configurable
    // For now, we only support passing in the service type as an argument
    let mut config = TerraphimConfig::new(ServiceType::Logseq);
    let mut config_state = ConfigState::new(&mut config)
        .await
        .context("Failed to load config")?;

    // Example of adding a role for testing
    let role = "system operator2".to_string();
    let automata_url = "https://system-operator.s3.eu-west-2.amazonaws.com/term_to_id.json";
    let rolegraph = RoleGraph::new(role.clone(), automata_url).await?;
    config_state
        .roles
        .insert(role, RoleGraphSync::from(rolegraph));
    println!(
        "cfg Roles: {:?}",
        config_state.roles.keys().collect::<Vec<&String>>()
    );

    axum_server(server_hostname, config_state).await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use reqwest::{Client, StatusCode};
    use tokio::sync::OnceCell;

    static SERVER: OnceCell<()> = OnceCell::const_new();

    async fn start_server() {
        let server_settings =
            Settings::load_from_env_and_file(None).expect("Failed to load settings");
        let server_hostname = server_settings
            .server_hostname
            .parse::<SocketAddr>()
            .unwrap_or_else(|_| {
                let port = portpicker::pick_unused_port().expect("Failed to find unused port");
                SocketAddr::from(([127, 0, 0, 1], port))
            });
        let config_state = types::ConfigState::new()
            .await
            .expect("Failed to create config state");

        tokio::spawn(async move {
            axum_server(server_hostname, config_state)
                .await
                .expect("Server failed to start");
        });
    }

    async fn ensure_server_started() {
        SERVER.get_or_init(|| async { start_server().await }).await;
    }

    // Example test function
    #[tokio::test]
    async fn test_search_articles() {
        ensure_server_started().await;
        let url = format!("http://{}/articles/search?search_term=trained%20operators%20and%20maintainers&skip=0&limit=10&role=system%20operator", "localhost:8000");
        println!("url: {:?}", url);
        let response = reqwest::get(url).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        // You can also test the response body if you want:
        // let body = response.text().await.unwrap();
        // assert!(body.contains("expected content"));
    }

    // test search article with POST method
    #[tokio::test]
    async fn test_post_search_article() {
        ensure_server_started().await;
        let client = Client::new();
        let response = client
            .post("http://localhost:8000/articles/search")
            .header("Content-Type", "application/json")
            .body(
                r#"
            {
                "search_term": "trained operators and maintainers",
                "skip": 0,
                "limit": 10,
                "role": "system operator"
            }
            "#,
            )
            .send()
            .await
            .unwrap();
        println!("response: {response:?}");
        assert_eq!(response.status(), StatusCode::OK);
    }
    // test search article with POST method
    #[tokio::test]
    async fn test_post_search_article_lifecycle() {
        ensure_server_started().await;
        let client = Client::new();
        let response = client
            .post("http://localhost:8000/articles/search")
            .header("Content-Type", "application/json")
            .body(
                r#"
                {
                    "search_term": "life cycle framework",
                    "skip": 0,
                    "limit": 10,
                    "role": "system operator"
                }
                "#,
            )
            .send()
            .await
            .unwrap();
        println!("response: {response:?}");
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_search_articles_without_role() {
        ensure_server_started().await;
        let response = reqwest::get("http://localhost:8000/articles/search?search_term=trained%20operators%20and%20maintainers&skip=0&limit=10").await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        // You can also test the response body if you want:
        // let body = response.text().await.unwrap();
        // assert!(body.contains("expected content"));
    }

    #[tokio::test]
    async fn test_search_articles_without_limit() {
        ensure_server_started().await;
        let response = reqwest::get("http://localhost:8000/articles/search?search_term=trained%20operators%20and%20maintainers&skip=0").await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        // You can also test the response body if you want:
        // let body = response.text().await.unwrap();
        // assert!(body.contains("expected content"));
    }

    #[tokio::test]
    async fn test_get_config() {
        ensure_server_started().await;
        let url = "http://localhost:8000/config/";
        let response = reqwest::get(url).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        // You can also test the response body if you want:
        // let body = response.text().await.unwrap();
        // assert!(body.contains("expected content"));
    }

    /// test update config
    #[tokio::test]
    async fn test_post_config() {
        ensure_server_started().await;
        use terraphim_config::TerraphimConfig;
        let response = reqwest::get("http://localhost:8000/config/").await.unwrap();
        let orig_config: TerraphimConfig = response.json().await.unwrap();
        println!("orig_config: {orig_config:?}");
        let mut new_config = orig_config.clone();
        new_config.default_role = "system operator".to_string();
        new_config.global_shortcut = "Ctrl+X".to_string();
        println!("new_config: {new_config:?}");
        let client = Client::new();
        let response = client
            .post("http://localhost:8000/config/")
            .header("Content-Type", "application/json")
            .header("Accept", "application/json")
            .body(serde_json::to_string(&new_config).unwrap())
            .send()
            .await
            .unwrap();
        println!("response: {response:?}");
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_post_article() {
        ensure_server_started().await;
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
