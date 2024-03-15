//! Integration tests for the server
//!
//! These tests are meant to be run against a running server.
//! We test the server by sending requests to it and checking the responses.
#[cfg(test)]
mod tests {
    use terraphim_server::axum_server;
    use terraphim_settings::Settings;

    use reqwest::{Client, StatusCode};
    use std::net::SocketAddr;
    use terraphim_config::{Config, ConfigState};
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

        let mut config = Config::new();
        let config_state = ConfigState::new(&mut config)
            .await
            .expect("Failed to create config state");

        tokio::spawn(async move {
            axum_server(server_hostname, config_state)
                .await
                .expect("Server failed to start");
        });
    }

    /// Initialize the server once and use it for all tests
    async fn ensure_server_started() {
        SERVER.get_or_init(|| async { start_server().await }).await;
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
        println!("response: {:?}", response);
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_search_articles() {
        ensure_server_started().await;
        let url = format!("http://{}/articles/search?search_term=trained%20operators%20and%20maintainers&skip=0&limit=10&role=system%20operator", "localhost:8000");
        println!("url: {:?}", url);
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

    #[tokio::test]
    async fn test_search_articles_without_role() {
        ensure_server_started().await;
        let response = reqwest::get("http://localhost:8000/articles/search?search_term=trained%20operators%20and%20maintainers&skip=0&limit=10").await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        // let body = response.text().await.unwrap();
        // assert!(body.contains("expected content"));
    }

    #[tokio::test]
    async fn test_search_articles_without_limit() {
        ensure_server_started().await;
        let response = reqwest::get("http://localhost:8000/articles/search?search_term=trained%20operators%20and%20maintainers&skip=0").await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        // let body = response.text().await.unwrap();
        // assert!(body.contains("expected content"));
    }

    #[tokio::test]
    async fn test_get_config() {
        ensure_server_started().await;
        let url = "http://localhost:8000/config/";
        let response = reqwest::get(url).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        // let body = response.text().await.unwrap();
        // assert!(body.contains("expected content"));
    }

    /// test update config
    #[tokio::test]
    async fn test_post_config() {
        use terraphim_config::Config;

        ensure_server_started().await;

        let response = reqwest::get("http://localhost:8000/config/").await.unwrap();
        let orig_config: Config = response.json().await.unwrap();
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

        // let body = response.text().await.unwrap();
        // assert!(body.contains("expected content"));
    }
}
