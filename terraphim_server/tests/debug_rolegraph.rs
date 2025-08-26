//! Debug test for RoleGraph visualization API

#[cfg(test)]
mod tests {
    use ahash::AHashMap;

    use terraphim_server::{axum_server, Status};
    use terraphim_settings::DeviceSettings;

    use std::{net::SocketAddr, time::Duration};
    use terraphim_config::{
        Config, ConfigBuilder, ConfigState, Haystack, KnowledgeGraph, KnowledgeGraphLocal, Role,
        ServiceType,
    };
    use terraphim_types::{KnowledgeGraphInputType, RelevanceFunction};

    use serial_test::serial;

    // Sample config with knowledge graphs for testing visualization
    fn sample_config_with_kg() -> Config {
        // Use absolute path to docs/src/kg
        let kg_path =
            std::path::PathBuf::from("/Users/alex/projects/terraphim/terraphim-ai/docs/src/kg");
        let haystack = kg_path.parent().unwrap().to_path_buf();

        ConfigBuilder::new()
            .global_shortcut("Ctrl+X")
            .add_role(
                "Engineer".into(),
                Role {
                    terraphim_it: true,
                    shortname: Some("Engineer".into()),
                    name: "Engineer".into(),
                    relevance_function: RelevanceFunction::TerraphimGraph,
                    theme: "lumen".to_string(),
                    kg: Some(KnowledgeGraph {
                        automata_path: None, // Will be built from local files
                        knowledge_graph_local: Some(KnowledgeGraphLocal {
                            input_type: KnowledgeGraphInputType::Markdown,
                            path: kg_path,
                        }),
                        public: true,
                        publish: true,
                    }),
                    haystacks: vec![Haystack {
                        location: haystack.to_string_lossy().to_string(),
                        service: ServiceType::Ripgrep,
                        read_only: false,
                        atomic_server_secret: None,
                        extra_parameters: std::collections::HashMap::new(),
                    }],
                    extra: AHashMap::new(),
                },
            )
            .default_role("Engineer")
            .unwrap()
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

        let mut config = sample_config_with_kg();
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
                    if attempts >= 30 {
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

    #[derive(Debug, serde::Deserialize)]
    struct GraphNodeDto {
        pub id: u64,
        pub label: String,
        pub rank: u64,
    }

    #[derive(Debug, serde::Deserialize)]
    struct GraphEdgeDto {
        pub source: u64,
        pub target: u64,
        pub rank: u64,
    }

    #[derive(Debug, serde::Deserialize)]
    struct RoleGraphResponseDto {
        pub status: Status,
        pub nodes: Vec<GraphNodeDto>,
        pub edges: Vec<GraphEdgeDto>,
    }

    /// Debug test to understand why rolegraph visualization is empty
    #[tokio::test]
    #[serial]
    async fn debug_rolegraph_visualization() {
        let server = ensure_server_started().await;
        let client = terraphim_service::http_client::create_default_client()
        .expect("Failed to create HTTP client");

        // Test Engineer role
        let response = client
            .get(format!("http://{server}/rolegraph?role=Engineer"))
            .send()
            .await
            .unwrap();

        println!("Response status: {}", response.status());

        if response.status() == 200 {
            let rolegraph_data: RoleGraphResponseDto = response.json().await.unwrap();

            println!("Response status: {:?}", rolegraph_data.status);
            println!("Number of nodes: {}", rolegraph_data.nodes.len());
            println!("Number of edges: {}", rolegraph_data.edges.len());

            if !rolegraph_data.nodes.is_empty() {
                println!("First few nodes:");
                for (i, node) in rolegraph_data.nodes.iter().take(5).enumerate() {
                    println!(
                        "  Node {}: id={}, label='{}', rank={}",
                        i, node.id, node.label, node.rank
                    );
                }
            }

            if !rolegraph_data.edges.is_empty() {
                println!("First few edges:");
                for (i, edge) in rolegraph_data.edges.iter().take(5).enumerate() {
                    println!(
                        "  Edge {}: source={}, target={}, rank={}",
                        i, edge.source, edge.target, edge.rank
                    );
                }
            }
        } else {
            let error_text = response.text().await.unwrap();
            println!("Error response: {}", error_text);
        }
    }

    /// Test config endpoint to see what roles are available
    #[tokio::test]
    #[serial]
    async fn debug_config() {
        let server = ensure_server_started().await;
        let client = terraphim_service::http_client::create_default_client()
        .expect("Failed to create HTTP client");

        let response = client
            .get(format!("http://{server}/config"))
            .send()
            .await
            .unwrap();

        println!("Config response status: {}", response.status());

        if response.status() == 200 {
            let config_text = response.text().await.unwrap();
            println!("Config response: {}", config_text);
        }
    }
}
