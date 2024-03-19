//! Integration tests for the server
//!
//! These tests are meant to be run against a running server.
//! We test the server by sending requests to it and checking the responses.
#[cfg(test)]
mod tests {
    use terraphim_server::axum_server;
    use terraphim_settings::Settings;

    use reqwest::{Client, StatusCode};
    use std::{net::SocketAddr, time::Duration};
    use terraphim_config::{Config, ConfigState};
    use tokio::sync::OnceCell;

    static SERVER: OnceCell<SocketAddr> = OnceCell::const_new();

    async fn start_server() -> SocketAddr {
        let server_settings =
            Settings::load_from_env_and_file(None).expect("Failed to load settings");
        let server_hostname = server_settings
            .server_hostname
            .parse::<SocketAddr>()
            .unwrap_or_else(|_| {
                let port = portpicker::pick_unused_port().expect("Failed to find unused port");
                SocketAddr::from(([127, 0, 0, 1], port))
            });

        let mut config = Config::new();
        let config_state = ConfigState::new(&mut config)
            .await
            .expect("Failed to create config state");

        tokio::spawn(async move {
            axum_server(server_hostname, config_state)
                .await
                .expect("Server failed to start");
        });

        server_hostname
    }

    async fn wait_for_server_ready(address: SocketAddr) {
        let client = Client::new();
        let health_url = format!("http://{}/health", address);

        let mut attempts = 0;
        loop {
            match client.get(&health_url).send().await {
                Ok(response) if response.status() == StatusCode::OK => {
                    println!("Server is ready at {}", address);
                    break;
                }
                _ => {
                    if attempts >= 5 {
                        panic!("Server did not become ready in time at {}", address);
                    }
                    println!("Waiting for server to become ready...");
                    tokio::time::sleep(Duration::from_secs(1)).await;
                    attempts += 1;
                }
            }
        }
    }

    /// Initialize the server once and use it for all tests
    async fn ensure_server_started() -> SocketAddr {
        let server_addr = *SERVER.get_or_init(|| async { start_server().await }).await;
        wait_for_server_ready(server_addr).await;
        server_addr
    }

    // test search article with POST method
    #[tokio::test]
    async fn test_post_search_article() {
        ensure_server_started().await;
        let client = Client::new();
        let response = client
            .post(format!("http://{}/articles/search", SERVER.get().unwrap()))
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

    #[tokio::test]
    async fn test_search_articles() {
        ensure_server_started().await;
        let url = format!("http://{}/articles/search?search_term=trained%20operators%20and%20maintainers&skip=0&limit=10&role=system%20operator", "localhost:8000");
        let response = reqwest::get(url).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        // let body = response.text().await.unwrap();
        // assert!(body.contains("expected content"));
    }

    // test search article with POST method
    #[tokio::test]
    async fn test_post_search_article_lifecycle() {
        ensure_server_started().await;
        let client = Client::new();
        let response = client
            .post(format!("http://{}/articles/search", SERVER.get().unwrap()))
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

    #[tokio::test]
    async fn test_search_articles_without_role() {
        ensure_server_started().await;
        let url = format!("http://{}/articles/search?search_term=trained%20operators%20and%20maintainers&skip=0&limit=10", SERVER.get().unwrap());
        let response = reqwest::get(url).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        // let body = response.text().await.unwrap();
        // assert!(body.contains("expected content"));
    }

    #[tokio::test]
    async fn test_search_articles_without_limit() {
        ensure_server_started().await;
        let response = reqwest::get(format!(
            "http://{}/articles/search?search_term=trained%20operators%20and%20maintainers&skip=0",
            SERVER.get().unwrap()
        ))
        .await
        .unwrap();

        let body = response.text().await.unwrap();
        println!("body: {:?}", body);
        // assert_eq!(response.status(), StatusCode::OK);

        // assert!(body.contains("expected content"));
    }

    #[tokio::test]
    async fn test_get_config() {
        ensure_server_started().await;
        let response = reqwest::get(format!("http://{}/config", SERVER.get().unwrap()))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        // Check that the config is valid JSON and contains the expected roles
        let config: Config = response.json().await.unwrap();
        assert!(config.roles.contains_key("system operator"));
        assert!(config.roles.contains_key("engineer"));
        assert!(config.roles.contains_key("default"));
    }

    /// test update config
    #[tokio::test]
    async fn test_post_config() {
        ensure_server_started().await;

        let config_url = format!("http://{}/config", SERVER.get().unwrap());

        let response = reqwest::get(&config_url).await.unwrap();
        let orig_config: Config = response.json().await.unwrap();
        assert_eq!(orig_config.default_role, "default");
        assert_eq!(orig_config.global_shortcut, "Ctrl+X");

        let mut new_config = orig_config.clone();
        new_config.default_role = "system operator".to_string();
        new_config.global_shortcut = "Ctrl+P".to_string();
        let client = Client::new();
        let response = client
            .post(&config_url)
            .header("Content-Type", "application/json")
            .header("Accept", "application/json")
            .body(serde_json::to_string(&new_config).unwrap())
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let new_config: Config = response.json().await.unwrap();
        assert_eq!(new_config.default_role, "system operator");
        assert_eq!(new_config.global_shortcut, "Ctrl+P");
    }

    #[tokio::test]
    async fn test_post_article() {
        ensure_server_started().await;
        let client = Client::new();
        let response = client.post(format!("http://{}/article", SERVER.get().unwrap()))
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

        // let body = response.text().await.unwrap();
        // assert!(body.contains("expected content"));
    }
}
