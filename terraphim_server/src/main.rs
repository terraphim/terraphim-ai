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
use anyhow::Context;
// #![deny(missing_docs)]
use clap::Parser;
use std::net::SocketAddr;
use std::sync::Arc;
use terraphim_settings::Settings;
use tokio::sync::Mutex;

use terraphim_server::{axum_server, Result};
use terraphim_types as types;

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
    println!("args: {:?}", args);
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
            let port = portpicker::pick_unused_port().expect("failed to find unused port");
            SocketAddr::from(([127, 0, 0, 1], port))
        });

    let mut config_state = types::ConfigState::new().await?;

    // Add one more for testing local KG

    let addr = server_hostname;
    let role = "system operator2".to_string();
    let automata_url = "https://system-operator.s3.eu-west-2.amazonaws.com/term_to_id.json";
    // let automata_url = "./data/term_to_id.json";
    let rolegraph = RoleGraph::new(role.clone(), automata_url).await?;
    config_state.roles.insert(
        role,
        types::RoleGraphState {
            rolegraph: Arc::new(Mutex::new(rolegraph)),
        },
    );
    println!(
        "cfg Roles: {:?}",
        config_state.roles.keys().collect::<Vec<&String>>()
    );
    axum_server(addr, config_state).await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use reqwest::{Client, StatusCode};
    use terraphim_types as types;
    use tokio::test;
    use std::net::SocketAddr;
    use terraphim_server::axum_server;

    lazy_static::lazy_static! {
        static ref SERVER: String = {
            // let port = portpicker::pick_unused_port().expect("failed to find unused port");
            let port = 8000;
            let addr = SocketAddr::from(([127, 0, 0, 1], port));

            tokio::spawn(async move {
                let config_state = types::ConfigState::new().await.unwrap();
                if let Err(e) = axum_server(addr, config_state).await {
                    println!("Failed to start axum server: {e:?}");
                }
                std::thread::sleep(std::time::Duration::from_secs(2));
                println!("Server started");
            });
    
            // Wait for the server to start
            
    
            // Store the server address in an environment variable so the tests can use it
            std::env::set_var("TEST_SERVER_URL", format!("http://{}", addr));
            addr.to_string()
        };
    }
    #[test]
    async fn test_search_articles() {
        let url = format!("http://{}/articles/search?search_term=trained%20operators%20and%20maintainers&skip=0&limit=10&role=system%20operator",&*SERVER);
        println!("url: {:?}", url);
        let response = reqwest::get(url).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        // You can also test the response body if you want:
        // let body = response.text().await.unwrap();
        // assert!(body.contains("expected content"));
    }

    // test search article with POST method
    #[test]
    async fn test_post_search_article() {
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
        println!("response: {:?}", response);
        assert_eq!(response.status(), StatusCode::OK);
    }
    // test search article with POST method
    #[test]
    async fn test_post_search_article_lifecycle() {
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
        let _ = &*SERVER;
        let url = format!("http://{}/api/config/",&*SERVER);
        println!("url: {:?}", url);
        let response = reqwest::get(url).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        // You can also test the response body if you want:
        // let body = response.text().await.unwrap();
        // assert!(body.contains("expected content"));
    }

    /// test update config
    #[test]
    async fn test_post_config() {
        use terraphim_config::TerraphimConfig;
        let response = reqwest::get("http://localhost:8000/config/").await.unwrap();
        let orig_config: TerraphimConfig = response.json().await.unwrap();
        println!("orig_config: {:?}", orig_config);
        let mut new_config = orig_config.clone();
        new_config.default_role = "system operator".to_string();
        new_config.global_shortcut = "Ctrl+X".to_string();
        println!("new_config: {:?}", new_config);
        let client = Client::new();
        let response = client
            .post("http://localhost:8000/config/")
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
