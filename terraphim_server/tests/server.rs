//! Integration tests for the server
//!
//! These tests are meant to be run against a running server.
//! We test the server by sending requests to it and checking the responses.
#[cfg(test)]
mod tests {
    use ahash::AHashMap;
    use terraphim_automata::AutomataPath;
    use terraphim_server::{axum_server, CreateDocumentResponse, SearchResponse, Status};
    use terraphim_settings::DeviceSettings;

    use std::{net::SocketAddr, path::PathBuf, time::Duration};
    use terraphim_config::{
        Config, ConfigBuilder, ConfigState, Haystack, KnowledgeGraph, KnowledgeGraphLocal, Role,
        ServiceType,
    };
    use terraphim_types::{KnowledgeGraphInputType, RelevanceFunction, RoleName};

    use terraphim_server::ConfigResponse;

    use serial_test::serial;

    // Sample config for testing
    fn sample_config() -> Config {
        let automata_path = AutomataPath::from_local("fixtures/term_to_id.json");
        let haystack = "fixtures/haystack".to_string();

        ConfigBuilder::new()
            .global_shortcut("Ctrl+X")
            .add_role(
                "Default",
                Role {
                    shortname: Some("Default".to_string()),
                    name: "Default".into(),
                    relevance_function: RelevanceFunction::TitleScorer,
                    theme: "spacelab".to_string(),
                    kg: None,
                    haystacks: vec![Haystack {
                        location: haystack.clone(),
                        service: ServiceType::Ripgrep,
                        read_only: false,
                        atomic_server_secret: None,
                        extra_parameters: std::collections::HashMap::new(),
                        fetch_content: false,
                    }],
                    terraphim_it: false,
                    ..Default::default()
                },
            )
            .add_role(
                "Engineer",
                Role {
                    shortname: Some("Engineer".into()),
                    name: "Engineer".into(),
                    relevance_function: RelevanceFunction::TitleScorer,
                    theme: "lumen".to_string(),
                    kg: Some(KnowledgeGraph {
                        automata_path: Some(automata_path.clone()),
                        knowledge_graph_local: Some(KnowledgeGraphLocal {
                            input_type: KnowledgeGraphInputType::Markdown,
                            path: PathBuf::from("/tmp/system_operator/pages/"),
                        }),
                        public: true,
                        publish: true,
                    }),
                    haystacks: vec![Haystack {
                        location: haystack.clone(),
                        service: ServiceType::Ripgrep,
                        read_only: false,
                        fetch_content: false,
                        atomic_server_secret: None,
                        extra_parameters: std::collections::HashMap::new(),
                    }],
                    terraphim_it: false,
                    ..Default::default()
                },
            )
            .add_role(
                "System Operator",
                Role {
                    shortname: Some("operator".to_string()),
                    name: "System Operator".into(),
                    relevance_function: RelevanceFunction::TerraphimGraph,
                    theme: "superhero".to_string(),
                    kg: Some(KnowledgeGraph {
                        automata_path: Some(automata_path),
                        knowledge_graph_local: Some(KnowledgeGraphLocal {
                            input_type: KnowledgeGraphInputType::Markdown,
                            path: PathBuf::from("/tmp/system_operator/pages/"),
                        }),
                        public: true,
                        publish: true,
                    }),
                    haystacks: vec![Haystack {
                        location: haystack.clone(),
                        service: ServiceType::Ripgrep,
                        read_only: false,
                        atomic_server_secret: None,
                        extra_parameters: std::collections::HashMap::new(),
                        fetch_content: false,
                    }],
                    terraphim_it: false,
                    ..Default::default()
                },
            )
            .build()
            .unwrap()
    }

    async fn start_server() -> SocketAddr {
        let server_settings =
            DeviceSettings::load_from_env_and_file(None).expect("Failed to load settings");
        let server_hostname = server_settings
            .server_hostname
            .parse::<SocketAddr>()
            .unwrap_or_else(|_| {
                let port = portpicker::pick_unused_port().expect("Failed to find unused port");
                SocketAddr::from(([127, 0, 0, 1], port))
            });

        let mut config = sample_config();
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
        let client = terraphim_service::http_client::create_default_client()
            .expect("Failed to create HTTP client");
        let health_url = format!("http://{}/health", address);

        let mut attempts = 0;
        loop {
            match client.get(&health_url).send().await {
                Ok(response) if response.status() == 200 => {
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

    // test search document with POST method
    #[tokio::test]
    #[serial]
    async fn test_post_search_document() {
        let server = ensure_server_started().await;
        let client = terraphim_service::http_client::create_default_client()
            .expect("Failed to create HTTP client");
        let response = client
            .post(format!("http://{server}/documents/search"))
            .header("Content-Type", "application/json")
            .body(
                r#"
            {
                "search_term": "trained operators and maintainers",
                "skip": 0,
                "limit": 10,
                "role": "System Operator"
            }
            "#,
            )
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), 200);
    }

    #[tokio::test]
    #[serial]
    async fn test_search_documents() {
        let server = ensure_server_started().await;
        let response = reqwest::get(format!("http://{server}/documents/search?search_term=trained%20operators%20and%20maintainers&skip=0&limit=10&role=System%20Operator")).await.unwrap();
        assert_eq!(response.status(), 200);
    }

    #[tokio::test]
    #[serial]
    async fn test_search_documents_without_role() {
        let server = ensure_server_started().await;

        let url = format!("http://{server}/documents/search?search_term=system&skip=0&limit=10");
        let response = reqwest::get(url).await.unwrap();
        assert_eq!(response.status(), 200);

        // The response body should be of this form:
        // {
        //     "status": "success",
        //     "total": 6,
        //     "results": [
        //       {
        //           "id": "981a5fdaa157cec9",
        //           "stub": null,
        //           "title": "fixtures/haystack/Transition.md",
        //           "url": "fixtures/haystack/Transition.md",
        //           "body": "Trained operators and maintainers",
        //           "tags": [
        //               "trained operators and maintainers"
        //           ],
        //           "rank": 10
        //       },
        //       ...
        //     ]
        // }
        let response: SearchResponse = response.json().await.unwrap();
        println!("{:#?}", response);
        assert!(matches!(response.status, Status::Success));
        assert!(response.total > 0);
        assert_eq!(response.total, response.results.len());
        let documents = response.results;

        // Check that all documents contain the search term and are located in the haystack
        for document in documents {
            println!("{:#?}", document);
            assert!(document.body.to_lowercase().contains("system"));
            //TODO: tags are not populated for default role, only for KG based roles
            // assert_eq!(
            //     document.tags,
            //     Some(vec!["trained operators and maintainers".to_string()])
            // );
            assert!(document.url.contains("fixtures/haystack/"));
        }
    }

    #[tokio::test]
    #[serial]
    async fn test_search_documents_without_limit() {
        let server = ensure_server_started().await;

        let response = reqwest::get(format!(
            "http://{server}/documents/search?search_term=system&skip=0",
        ))
        .await
        .unwrap();
        assert_eq!(response.status(), 200);

        let response: SearchResponse = response.json().await.unwrap();
        println!("{:#?}", response);
        assert!(matches!(response.status, Status::Success));
        assert!(response.total > 0);
        assert_eq!(response.total, response.results.len());
        let documents = response.results;

        // Check that all documents contain the search term and are located in the haystack
        for document in documents {
            println!("{:#?}", document);
            assert!(document.body.to_lowercase().contains("system"));
            assert!(document.url.contains("fixtures/haystack/"));
        }
    }

    #[tokio::test]
    #[serial]
    async fn test_get_config() {
        let server = ensure_server_started().await;
        let response = reqwest::get(format!("http://{server}/config"))
            .await
            .unwrap();
        assert_eq!(response.status(), 200);

        // Check that the config is valid JSON and contains the expected roles
        let response: ConfigResponse = response.json().await.unwrap();
        assert!(matches!(response.status, Status::Success));
        assert!(response
            .config
            .roles
            .contains_key(&RoleName::new("System Operator")));
        assert!(response
            .config
            .roles
            .contains_key(&RoleName::new("Engineer")));
    }

    /// test update config
    #[tokio::test]
    #[serial]
    async fn test_update_config() {
        let server = ensure_server_started().await;
        let config_url = format!("http://{server}/config");

        let response = reqwest::get(&config_url).await.unwrap();
        let orig_config: ConfigResponse = response.json().await.unwrap();
        assert!(matches!(orig_config.status, Status::Success));
        assert_eq!(orig_config.config.default_role, "Default".into());
        assert_eq!(orig_config.config.global_shortcut, "Ctrl+X");

        let mut new_config = orig_config.config.clone();
        new_config.default_role = "Engineer".to_string().into();
        new_config.global_shortcut = "Ctrl+P".to_string();
        let client = terraphim_service::http_client::create_default_client()
            .expect("Failed to create HTTP client");
        let response = client
            .post(&config_url)
            .header("Content-Type", "application/json")
            .header("Accept", "application/json")
            .body(serde_json::to_string(&new_config).unwrap())
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), 200);

        let new_config: ConfigResponse = response.json().await.unwrap();
        assert!(matches!(orig_config.status, Status::Success));
        assert_eq!(new_config.config.default_role, "Engineer".into());
        assert_eq!(new_config.config.global_shortcut, "Ctrl+P");
    }

    #[tokio::test]
    #[serial]
    async fn test_create_document() {
        let server = ensure_server_started().await;
        let client = terraphim_service::http_client::create_default_client()
            .expect("Failed to create HTTP client");
        let response = client.post(format!("http://{server}/documents"))
            .header("Content-Type", "application/json")
            // TODO: Do we want to set the ID here or want the server to
            // generate it?
            .body(r#"
            {
                "id": "Title of the document",
                "title": "Title of the document",
                "url": "url_of_the_document",
                "body": "I am a text with the word Life cycle concepts and bar and Trained operators and maintainers, some bingo words Paradigm Map and project planning, then again: some bingo words Paradigm Map and project planning, then repeats: Trained operators and maintainers, project direction"
            }
            "#)
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), 200);
        let response: CreateDocumentResponse = response.json().await.unwrap();
        assert!(matches!(response.status, Status::Success));
        assert_eq!(response.id, "Title of the document");
    }
}
