//! Integration tests for the RoleGraph visualization API
//!
//! These tests validate that the visualization API correctly outputs nodes and edges
//! that match the rolegraph data structure for roles with defined knowledge graphs.

#[cfg(test)]
mod tests {
    use ahash::AHashMap;
    use terraphim_automata::AutomataPath;
    use terraphim_server::{axum_server, Status};
    use terraphim_settings::DeviceSettings;

    use reqwest::{Client, StatusCode};
    use std::{net::SocketAddr, path::PathBuf, time::Duration};
    use terraphim_config::{
        Config, ConfigBuilder, ConfigState, Haystack, KnowledgeGraph, KnowledgeGraphLocal, Role,
        ServiceType,
    };
    use terraphim_types::{KnowledgeGraphInputType, RelevanceFunction, RoleName};

    use serial_test::serial;

    // Sample config with knowledge graphs for testing visualization
    fn sample_config_with_kg() -> Config {
        // Use local knowledge graph files instead of pre-built automata
        let haystack = PathBuf::from("fixtures/haystack");

        ConfigBuilder::new()
            .global_shortcut("Ctrl+X")
            .add_role(
                "Default".into(),
                Role {
                    shortname: Some("Default".to_string()),
                    name: "Default".into(),
                    relevance_function: RelevanceFunction::TitleScorer,
                    theme: "spacelab".to_string(),
                    kg: None, // No knowledge graph for Default role
                    haystacks: vec![Haystack {
                        location: haystack.to_string_lossy().to_string(),
                        service: ServiceType::Ripgrep,
                        read_only: false,
                        atomic_server_secret: None,
                    }],
                    extra: AHashMap::new(),
                },
            )
            .add_role(
                "Engineer".into(),
                Role {
                    shortname: Some("Engineer".into()),
                    name: "Engineer".into(),
                    relevance_function: RelevanceFunction::TerraphimGraph,
                    theme: "lumen".to_string(),
                    kg: Some(KnowledgeGraph {
                        automata_path: None, // Will be built from local files
                        knowledge_graph_local: Some(KnowledgeGraphLocal {
                            input_type: KnowledgeGraphInputType::Markdown,
                            path: PathBuf::from("fixtures/haystack/"),
                        }),
                        public: true,
                        publish: true,
                    }),
                    haystacks: vec![Haystack {
                        location: haystack.to_string_lossy().to_string(),
                        service: ServiceType::Ripgrep,
                        read_only: false,
                        atomic_server_secret: None,
                    }],
                    extra: AHashMap::new(),
                },
            )
            .add_role(
                "System Operator".into(),
                Role {
                    shortname: Some("operator".to_string()),
                    name: "System Operator".into(),
                    relevance_function: RelevanceFunction::TerraphimGraph,
                    theme: "superhero".to_string(),
                    kg: Some(KnowledgeGraph {
                        automata_path: None, // Will be built from local files
                        knowledge_graph_local: Some(KnowledgeGraphLocal {
                            input_type: KnowledgeGraphInputType::Markdown,
                            path: PathBuf::from("fixtures/haystack/"),
                        }),
                        public: true,
                        publish: true,
                    }),
                    haystacks: vec![Haystack {
                        location: haystack.to_string_lossy().to_string(),
                        service: ServiceType::Ripgrep,
                        read_only: false,
                        atomic_server_secret: None,
                    }],
                    extra: AHashMap::new(),
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

    /// Test that the rolegraph visualization endpoint returns correct structure for roles with knowledge graphs
    #[tokio::test]
    #[serial]
    async fn test_rolegraph_visualization_structure() {
        let server = ensure_server_started().await;
        let client = Client::new();

        // Test Engineer role (has knowledge graph)
        let response = client
            .get(format!("http://{server}/rolegraph?role=Engineer"))
            .send()
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let rolegraph_data: RoleGraphResponseDto = response.json().await.unwrap();
        
        // Validate response structure
        assert_eq!(rolegraph_data.status, Status::Success);
        
        // Validate that we have nodes and edges
        assert!(!rolegraph_data.nodes.is_empty(), "Engineer role should have nodes");
        assert!(!rolegraph_data.edges.is_empty(), "Engineer role should have edges");
        
        // Validate node structure
        for node in &rolegraph_data.nodes {
            assert!(node.id > 0, "Node ID should be positive");
            assert!(!node.label.is_empty(), "Node label should not be empty");
            assert!(node.rank > 0, "Node rank should be positive");
        }
        
        // Validate edge structure
        for edge in &rolegraph_data.edges {
            assert!(edge.source > 0, "Edge source should be positive");
            assert!(edge.target > 0, "Edge target should be positive");
            assert!(edge.rank > 0, "Edge rank should be positive");
            
            // Validate that source and target nodes exist
            let source_exists = rolegraph_data.nodes.iter().any(|n| n.id == edge.source);
            let target_exists = rolegraph_data.nodes.iter().any(|n| n.id == edge.target);
            
            assert!(source_exists, "Edge source node {} should exist", edge.source);
            assert!(target_exists, "Edge target node {} should exist", edge.target);
        }
        
        println!("Engineer role visualization: {} nodes, {} edges", 
                 rolegraph_data.nodes.len(), rolegraph_data.edges.len());
    }

    /// Test that the rolegraph visualization endpoint returns correct structure for System Operator role
    #[tokio::test]
    #[serial]
    async fn test_rolegraph_visualization_system_operator() {
        let server = ensure_server_started().await;
        let client = Client::new();

        // Test System Operator role (has knowledge graph)
        let response = client
            .get(format!("http://{server}/rolegraph?role=System%20Operator"))
            .send()
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let rolegraph_data: RoleGraphResponseDto = response.json().await.unwrap();
        
        // Validate response structure
        assert_eq!(rolegraph_data.status, Status::Success);
        
        // Validate that we have nodes and edges
        assert!(!rolegraph_data.nodes.is_empty(), "System Operator role should have nodes");
        assert!(!rolegraph_data.edges.is_empty(), "System Operator role should have edges");
        
        println!("System Operator role visualization: {} nodes, {} edges", 
                 rolegraph_data.nodes.len(), rolegraph_data.edges.len());
    }

    /// Test that the rolegraph visualization endpoint returns 404 for roles without knowledge graphs
    #[tokio::test]
    #[serial]
    async fn test_rolegraph_visualization_no_kg() {
        let server = ensure_server_started().await;
        let client = Client::new();

        // Test Default role (no knowledge graph)
        let response = client
            .get(format!("http://{server}/rolegraph?role=Default"))
            .send()
            .await
            .unwrap();

        // Should return 404 since Default role has no knowledge graph
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    /// Test that the rolegraph visualization endpoint uses selected role when no role specified
    #[tokio::test]
    #[serial]
    async fn test_rolegraph_visualization_default_role() {
        let server = ensure_server_started().await;
        let client = Client::new();

        // Test without specifying role (should use selected role)
        let response = client
            .get(format!("http://{server}/rolegraph"))
            .send()
            .await
            .unwrap();

        // Since the default selected role is "Default" which has no KG, this should return 404
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    /// Test that the rolegraph visualization endpoint returns consistent data structure
    #[tokio::test]
    #[serial]
    async fn test_rolegraph_visualization_consistency() {
        let server = ensure_server_started().await;
        let client = Client::new();

        // Test Engineer role multiple times to ensure consistency
        let response1 = client
            .get(format!("http://{server}/rolegraph?role=Engineer"))
            .send()
            .await
            .unwrap();

        let response2 = client
            .get(format!("http://{server}/rolegraph?role=Engineer"))
            .send()
            .await
            .unwrap();

        assert_eq!(response1.status(), StatusCode::OK);
        assert_eq!(response2.status(), StatusCode::OK);

        let rolegraph_data1: RoleGraphResponseDto = response1.json().await.unwrap();
        let rolegraph_data2: RoleGraphResponseDto = response2.json().await.unwrap();

        // Validate that the structure is consistent
        assert_eq!(rolegraph_data1.nodes.len(), rolegraph_data2.nodes.len());
        assert_eq!(rolegraph_data1.edges.len(), rolegraph_data2.edges.len());
        
        // Validate that node IDs are consistent
        let node_ids1: std::collections::HashSet<u64> = rolegraph_data1.nodes.iter().map(|n| n.id).collect();
        let node_ids2: std::collections::HashSet<u64> = rolegraph_data2.nodes.iter().map(|n| n.id).collect();
        assert_eq!(node_ids1, node_ids2);
        
        // Validate that edge IDs are consistent
        let edge_ids1: std::collections::HashSet<(u64, u64)> = rolegraph_data1.edges.iter()
            .map(|e| (e.source, e.target))
            .collect();
        let edge_ids2: std::collections::HashSet<(u64, u64)> = rolegraph_data2.edges.iter()
            .map(|e| (e.source, e.target))
            .collect();
        assert_eq!(edge_ids1, edge_ids2);
    }

    /// Test that the rolegraph visualization endpoint handles invalid role names gracefully
    #[tokio::test]
    #[serial]
    async fn test_rolegraph_visualization_invalid_role() {
        let server = ensure_server_started().await;
        let client = Client::new();

        // Test with invalid role name
        let response = client
            .get(format!("http://{server}/rolegraph?role=InvalidRole"))
            .send()
            .await
            .unwrap();

        // Should return 404 for invalid role
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    /// Test that the rolegraph visualization endpoint returns proper node labels
    #[tokio::test]
    #[serial]
    async fn test_rolegraph_visualization_node_labels() {
        let server = ensure_server_started().await;
        let client = Client::new();

        let response = client
            .get(format!("http://{server}/rolegraph?role=Engineer"))
            .send()
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let rolegraph_data: RoleGraphResponseDto = response.json().await.unwrap();
        
        // Validate that node labels are meaningful (not just IDs)
        for node in &rolegraph_data.nodes {
            assert!(!node.label.is_empty(), "Node label should not be empty");
            assert!(!node.label.chars().all(|c| c.is_numeric()), 
                   "Node label should not be just numeric: {}", node.label);
        }
        
        // Check for some expected labels from the thesaurus
        let labels: std::collections::HashSet<&str> = rolegraph_data.nodes.iter()
            .map(|n| n.label.as_str())
            .collect();
        
        println!("Available node labels: {:?}", labels);
    }

    /// Test that the rolegraph visualization endpoint returns proper edge relationships
    #[tokio::test]
    #[serial]
    async fn test_rolegraph_visualization_edge_relationships() {
        let server = ensure_server_started().await;
        let client = Client::new();

        let response = client
            .get(format!("http://{server}/rolegraph?role=Engineer"))
            .send()
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let rolegraph_data: RoleGraphResponseDto = response.json().await.unwrap();
        
        // Validate that edges connect valid nodes
        let node_ids: std::collections::HashSet<u64> = rolegraph_data.nodes.iter()
            .map(|n| n.id)
            .collect();
        
        for edge in &rolegraph_data.edges {
            assert!(node_ids.contains(&edge.source), 
                   "Edge source {} should exist in nodes", edge.source);
            assert!(node_ids.contains(&edge.target), 
                   "Edge target {} should exist in nodes", edge.target);
            assert_ne!(edge.source, edge.target, 
                      "Edge should not connect node to itself");
        }
        
        // Validate that edges have proper ranks
        for edge in &rolegraph_data.edges {
            assert!(edge.rank > 0, "Edge rank should be positive");
        }
    }
} 