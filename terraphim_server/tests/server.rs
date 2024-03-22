//! Integration tests for the server
//!
//! These tests are meant to be run against a running server.
//! We test the server by sending requests to it and checking the responses.
#[cfg(test)]
mod tests {
    use terraphim_server::axum_server;
    use terraphim_settings::Settings;

    use reqwest::{Client, StatusCode};
    use std::{net::SocketAddr, path::PathBuf, time::Duration};
    use terraphim_config::{Config, ConfigState, Haystack, ServiceType};
    use terraphim_types::Article;

    use serial_test::serial;

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
        let server_addr = start_server().await;
        wait_for_server_ready(server_addr).await;
        server_addr
    }

    // test search article with POST method
    #[tokio::test]
    #[serial]
    async fn test_post_search_article() {
        let server = ensure_server_started().await;
        let client = Client::new();
        let response = client
            .post(format!("http://{server}/articles/search"))
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
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    #[serial]
    async fn test_search_articles() {
        let server = ensure_server_started().await;
        let url = format!("http://{server}/articles/search?search_term=trained%20operators%20and%20maintainers&skip=0&limit=10&role=system%20operator");
        let response = reqwest::get(url).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        // let body = response.text().await.unwrap();
        // assert!(body.contains("expected content"));
    }

    // test search article with POST method
    #[tokio::test]
    #[serial]
    async fn test_post_search_article_lifecycle() {
        let server = ensure_server_started().await;
        let client = Client::new();
        let response = client
            .post(format!("http://{server}/articles/search"))
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
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    #[serial]
    async fn test_search_articles_without_role() {
        let server = ensure_server_started().await;

        // Overwrite config for test
        let config_url = format!("http://{server}/config");
        let response = reqwest::get(&config_url).await.unwrap();
        let mut config: Config = response.json().await.unwrap();
        // For each role, overwrite the haystack path to "test_dir"
        for role in config.roles.values_mut() {
            role.haystacks = vec![Haystack {
                path: PathBuf::from("fixtures/haystack"),
                service: ServiceType::Ripgrep,
            }]
        }

        let client = Client::new();
        let response = client
            .post(&config_url)
            .header("Content-Type", "application/json")
            .header("Accept", "application/json")
            .body(serde_json::to_string(&config).unwrap())
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let url = format!("http://{server}/articles/search?search_term=trained%20operators%20and%20maintainers&skip=0&limit=10");
        let response = reqwest::get(url).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        // The response body should be of this form:
        // [
        //     {
        //         "id": "981a5fdaa157cec9",
        //         "stub": null,
        //         "title": "fixtures/haystack/Transition.md",
        //         "url": "fixtures/haystack/Transition.md",
        //         "body": "Trained operators and maintainers",
        //         "tags": [
        //             "trained operators and maintainers"
        //         ],
        //         "rank": 10
        //     }
        // ]
        let articles: Vec<Article> = response.json().await.unwrap();
        let first_article = &articles[0];
        assert_eq!(first_article.title, "fixtures/haystack/Transition.md");
        assert_eq!(first_article.url, "fixtures/haystack/Transition.md");
        assert!(first_article
            .body
            .contains("Trained operators and maintainers"));
        assert_eq!(
            first_article.tags,
            Some(vec!["trained operators and maintainers".to_string()])
        );
    }

    #[tokio::test]
    #[serial]
    async fn test_search_articles_without_limit() {
        let server = ensure_server_started().await;

        // Overwrite config for test
        let config_url = format!("http://{server}/config");
        let response = reqwest::get(&config_url).await.unwrap();
        let mut config: Config = response.json().await.unwrap();
        // For each role, overwrite the haystack path to "test_dir"
        for role in config.roles.values_mut() {
            role.haystacks = vec![Haystack {
                path: PathBuf::from("fixtures/haystack"),
                service: ServiceType::Ripgrep,
            }]
        }

        let client = Client::new();
        let response = client
            .post(&config_url)
            .header("Content-Type", "application/json")
            .header("Accept", "application/json")
            .body(serde_json::to_string(&config).unwrap())
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let response = reqwest::get(format!(
            "http://{server}/articles/search?search_term=trained%20operators%20and%20maintainers&skip=0",
        ))
        .await
        .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        // The response body should be of this form:
        // [
        //     {
        //         "id": "981a5fdaa157cec9",
        //         "stub": null,
        //         "title": "fixtures/haystack/Transition.md",
        //         "url": "fixtures/haystack/Transition.md",
        //         "body": "Trained operators and maintainers",
        //         "tags": [
        //             "trained operators and maintainers"
        //         ],
        //         "rank": 10
        //     }
        // ]
        let articles: Vec<Article> = response.json().await.unwrap();
        let first_article = &articles[0];
        assert_eq!(first_article.title, "fixtures/haystack/Transition.md");
        assert_eq!(first_article.url, "fixtures/haystack/Transition.md");
        assert!(first_article
            .body
            .contains("Trained operators and maintainers"));
        assert_eq!(
            first_article.tags,
            Some(vec!["trained operators and maintainers".to_string()])
        );
    }

    #[tokio::test]
    #[serial]
    async fn test_get_config() {
        let server = ensure_server_started().await;
        let response = reqwest::get(format!("http://{server}/config"))
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
    #[serial]
    async fn test_post_config() {
        let server = ensure_server_started().await;
        let config_url = format!("http://{server}/config");

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
    #[serial]
    async fn test_post_article() {
        let server = ensure_server_started().await;
        let client = Client::new();
        let response = client.post(format!("http://{server}/article"))
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
