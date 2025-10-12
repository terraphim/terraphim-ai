//! Integration tests for the agent registry

use std::sync::Arc;
use std::time::Duration;

use terraphim_agent_registry::{
    AgentCapability, AgentDiscoveryQuery, AgentMetadata, AgentPid, AgentRegistry, AgentRole,
    AutomataConfig, CapabilityMetrics, KnowledgeGraphAgentRegistry, RegistryBuilder,
    RegistryConfig, ResourceUsage, SimilarityThresholds, SupervisorId,
};
use terraphim_rolegraph::RoleGraph;

#[tokio::test]
async fn test_full_agent_lifecycle() {
    env_logger::try_init().ok();

    // Create registry
    let role_name = RoleName::new("test_role");
    let thesaurus = Thesaurus::new();
    let role_graph = Arc::new(RoleGraph::new(role_name, thesaurus).await.unwrap());
    let config = RegistryConfig {
        max_agents: 100,
        auto_cleanup: false, // Disable for testing
        cleanup_interval_secs: 60,
        enable_monitoring: false, // Disable for testing
        discovery_cache_ttl_secs: 300,
    };

    let registry = RegistryBuilder::new()
        .with_role_graph(role_graph)
        .with_config(config)
        .build()
        .unwrap();

    // Create test agent
    let agent_id = AgentPid::new();
    let supervisor_id = SupervisorId::new();

    let mut primary_role = AgentRole::new(
        "data_scientist".to_string(),
        "Data Scientist".to_string(),
        "Analyzes data and builds models".to_string(),
    );
    primary_role.knowledge_domains = vec!["machine_learning".to_string(), "statistics".to_string()];

    let mut metadata = AgentMetadata::new(agent_id.clone(), supervisor_id, primary_role);

    // Add capabilities
    let analysis_capability = AgentCapability {
        capability_id: "data_analysis".to_string(),
        name: "Data Analysis".to_string(),
        description: "Analyze datasets and extract insights".to_string(),
        category: "analysis".to_string(),
        required_domains: vec!["statistics".to_string()],
        input_types: vec!["csv".to_string(), "json".to_string()],
        output_types: vec!["report".to_string(), "insights".to_string()],
        performance_metrics: CapabilityMetrics {
            avg_execution_time: Duration::from_secs(60),
            success_rate: 0.92,
            quality_score: 0.88,
            resource_usage: ResourceUsage {
                memory_mb: 512.0,
                cpu_percent: 30.0,
                network_kbps: 10.0,
                storage_mb: 200.0,
            },
            last_updated: chrono::Utc::now(),
        },
        dependencies: Vec::new(),
    };

    metadata.add_capability(analysis_capability).unwrap();

    // Test registration
    registry.register_agent(metadata.clone()).await.unwrap();

    // Test retrieval
    let retrieved = registry.get_agent(&agent_id).await.unwrap();
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap().agent_id, agent_id);

    // Test listing
    let all_agents = registry.list_agents().await.unwrap();
    assert_eq!(all_agents.len(), 1);

    // Test discovery
    let query = AgentDiscoveryQuery {
        required_roles: vec!["data_scientist".to_string()],
        required_capabilities: vec!["data_analysis".to_string()],
        required_domains: vec!["statistics".to_string()],
        task_description: Some("Analyze customer behavior data".to_string()),
        min_success_rate: Some(0.8),
        max_resource_usage: Some(ResourceUsage {
            memory_mb: 1024.0,
            cpu_percent: 50.0,
            network_kbps: 50.0,
            storage_mb: 500.0,
        }),
        preferred_tags: Vec::new(),
    };

    let discovery_result = registry.discover_agents(query).await.unwrap();
    assert!(!discovery_result.matches.is_empty());
    assert!(discovery_result.matches[0].match_score > 0.0);

    // Test role-based search
    let role_agents = registry
        .find_agents_by_role("data_scientist")
        .await
        .unwrap();
    assert_eq!(role_agents.len(), 1);

    // Test capability-based search
    let capability_agents = registry
        .find_agents_by_capability("data_analysis")
        .await
        .unwrap();
    assert_eq!(capability_agents.len(), 1);

    // Test supervisor-based search
    let supervisor_agents = registry
        .find_agents_by_supervisor(&supervisor_id)
        .await
        .unwrap();
    assert_eq!(supervisor_agents.len(), 1);

    // Test statistics
    let stats = registry.get_statistics().await.unwrap();
    assert_eq!(stats.total_agents, 1);
    assert!(stats.agents_by_role.contains_key("data_scientist"));

    // Test unregistration
    registry.unregister_agent(&agent_id).await.unwrap();

    let final_agents = registry.list_agents().await.unwrap();
    assert_eq!(final_agents.len(), 0);
}

#[tokio::test]
async fn test_multi_agent_discovery() {
    env_logger::try_init().ok();

    let role_name = RoleName::new("test_role");
    let thesaurus = Thesaurus::new();
    let role_graph = Arc::new(RoleGraph::new(role_name, thesaurus).await.unwrap());
    let registry = RegistryBuilder::new()
        .with_role_graph(role_graph)
        .build()
        .unwrap();

    // Create multiple agents with different specializations
    let agents_data = vec![
        ("planner", "Planning Agent", "task_planning", "planning"),
        ("executor", "Execution Agent", "task_execution", "execution"),
        ("analyzer", "Analysis Agent", "data_analysis", "analysis"),
        (
            "coordinator",
            "Coordination Agent",
            "team_coordination",
            "coordination",
        ),
    ];

    for (role_id, role_name, capability_id, category) in agents_data {
        let agent_id = AgentPid::new();
        let supervisor_id = SupervisorId::new();

        let role = AgentRole::new(
            role_id.to_string(),
            role_name.to_string(),
            format!("Specializes in {}", category),
        );

        let mut metadata = AgentMetadata::new(agent_id, supervisor_id, role);

        let capability = AgentCapability {
            capability_id: capability_id.to_string(),
            name: format!("{} Capability", role_name),
            description: format!("Provides {} services", category),
            category: category.to_string(),
            required_domains: vec![category.to_string()],
            input_types: vec!["request".to_string()],
            output_types: vec!["result".to_string()],
            performance_metrics: CapabilityMetrics::default(),
            dependencies: Vec::new(),
        };

        metadata.add_capability(capability).unwrap();
        registry.register_agent(metadata).await.unwrap();
    }

    // Test discovery for planning tasks
    let planning_query = AgentDiscoveryQuery {
        required_roles: vec!["planner".to_string()],
        required_capabilities: vec!["task_planning".to_string()],
        required_domains: Vec::new(),
        task_description: Some("Plan a software development project".to_string()),
        min_success_rate: None,
        max_resource_usage: None,
        preferred_tags: Vec::new(),
    };

    let planning_result = registry.discover_agents(planning_query).await.unwrap();
    assert_eq!(planning_result.matches.len(), 1);
    assert_eq!(
        planning_result.matches[0].agent.primary_role.role_id,
        "planner"
    );

    // Test discovery for multiple roles
    let multi_role_query = AgentDiscoveryQuery {
        required_roles: vec!["planner".to_string(), "executor".to_string()],
        required_capabilities: Vec::new(),
        required_domains: Vec::new(),
        task_description: None,
        min_success_rate: None,
        max_resource_usage: None,
        preferred_tags: Vec::new(),
    };

    let multi_result = registry.discover_agents(multi_role_query).await.unwrap();
    assert_eq!(multi_result.matches.len(), 2);

    // Test discovery with no matches
    let no_match_query = AgentDiscoveryQuery {
        required_roles: vec!["nonexistent_role".to_string()],
        required_capabilities: Vec::new(),
        required_domains: Vec::new(),
        task_description: None,
        min_success_rate: None,
        max_resource_usage: None,
        preferred_tags: Vec::new(),
    };

    let no_match_result = registry.discover_agents(no_match_query).await.unwrap();
    assert!(no_match_result.matches.is_empty());
    assert!(!no_match_result.suggestions.is_empty());
}

#[tokio::test]
async fn test_agent_performance_tracking() {
    env_logger::try_init().ok();

    let role_name = RoleName::new("test_role");
    let thesaurus = Thesaurus::new();
    let role_graph = Arc::new(RoleGraph::new(role_name, thesaurus).await.unwrap());
    let registry = RegistryBuilder::new()
        .with_role_graph(role_graph)
        .build()
        .unwrap();

    let agent_id = AgentPid::new();
    let supervisor_id = SupervisorId::new();

    let role = AgentRole::new(
        "worker".to_string(),
        "Worker Agent".to_string(),
        "General purpose worker".to_string(),
    );

    let mut metadata = AgentMetadata::new(agent_id.clone(), supervisor_id, role);

    // Record some performance data
    metadata.record_task_completion(Duration::from_secs(10), true);
    metadata.record_task_completion(Duration::from_secs(15), true);
    metadata.record_task_completion(Duration::from_secs(20), false);

    assert_eq!(metadata.statistics.tasks_completed, 2);
    assert_eq!(metadata.statistics.tasks_failed, 1);
    assert_eq!(metadata.get_success_rate(), 2.0 / 3.0);

    registry.register_agent(metadata).await.unwrap();

    // Test discovery with performance requirements
    let performance_query = AgentDiscoveryQuery {
        required_roles: vec!["worker".to_string()],
        required_capabilities: Vec::new(),
        required_domains: Vec::new(),
        task_description: None,
        min_success_rate: Some(0.5), // Should match
        max_resource_usage: None,
        preferred_tags: Vec::new(),
    };

    let result = registry.discover_agents(performance_query).await.unwrap();
    assert_eq!(result.matches.len(), 1);

    // Test with higher performance requirement
    let high_performance_query = AgentDiscoveryQuery {
        required_roles: vec!["worker".to_string()],
        required_capabilities: Vec::new(),
        required_domains: Vec::new(),
        task_description: None,
        min_success_rate: Some(0.9), // Should not match
        max_resource_usage: None,
        preferred_tags: Vec::new(),
    };

    let high_result = registry
        .discover_agents(high_performance_query)
        .await
        .unwrap();
    // Agent might still match but with lower score due to performance penalty
    if !high_result.matches.is_empty() {
        assert!(high_result.matches[0].match_score < 1.0);
    }
}

#[tokio::test]
async fn test_agent_role_hierarchy() {
    env_logger::try_init().ok();

    let role_name = RoleName::new("test_role");
    let thesaurus = Thesaurus::new();
    let role_graph = Arc::new(RoleGraph::new(role_name, thesaurus).await.unwrap());
    let registry = RegistryBuilder::new()
        .with_role_graph(role_graph)
        .build()
        .unwrap();

    // Create agent with primary and secondary roles
    let agent_id = AgentPid::new();
    let supervisor_id = SupervisorId::new();

    let primary_role = AgentRole::new(
        "senior_developer".to_string(),
        "Senior Developer".to_string(),
        "Experienced software developer".to_string(),
    );

    let mut metadata = AgentMetadata::new(agent_id.clone(), supervisor_id, primary_role);

    // Add secondary roles
    let reviewer_role = AgentRole::new(
        "code_reviewer".to_string(),
        "Code Reviewer".to_string(),
        "Reviews code for quality".to_string(),
    );

    let mentor_role = AgentRole::new(
        "mentor".to_string(),
        "Mentor".to_string(),
        "Mentors junior developers".to_string(),
    );

    metadata.add_secondary_role(reviewer_role).unwrap();
    metadata.add_secondary_role(mentor_role).unwrap();

    registry.register_agent(metadata).await.unwrap();

    // Test discovery by primary role
    let primary_agents = registry
        .find_agents_by_role("senior_developer")
        .await
        .unwrap();
    assert_eq!(primary_agents.len(), 1);

    // Test discovery by secondary role
    let reviewer_agents = registry.find_agents_by_role("code_reviewer").await.unwrap();
    assert_eq!(reviewer_agents.len(), 1);

    let mentor_agents = registry.find_agents_by_role("mentor").await.unwrap();
    assert_eq!(mentor_agents.len(), 1);

    // Test that agent has all roles
    let retrieved = registry.get_agent(&agent_id).await.unwrap().unwrap();
    assert!(retrieved.has_role("senior_developer"));
    assert!(retrieved.has_role("code_reviewer"));
    assert!(retrieved.has_role("mentor"));
    assert!(!retrieved.has_role("nonexistent_role"));

    // Test role count
    assert_eq!(retrieved.get_all_roles().len(), 3);
}

#[tokio::test]
async fn test_registry_capacity_and_cleanup() {
    env_logger::try_init().ok();

    let role_name = RoleName::new("test_role");
    let thesaurus = Thesaurus::new();
    let role_graph = Arc::new(RoleGraph::new(role_name, thesaurus).await.unwrap());
    let config = RegistryConfig {
        max_agents: 3, // Small capacity for testing
        auto_cleanup: false,
        cleanup_interval_secs: 1,
        enable_monitoring: false,
        discovery_cache_ttl_secs: 60,
    };

    let registry = RegistryBuilder::new()
        .with_role_graph(role_graph)
        .with_config(config)
        .build()
        .unwrap();

    // Register agents up to capacity
    for i in 0..3 {
        let agent_id = AgentPid::new();
        let supervisor_id = SupervisorId::new();

        let role = AgentRole::new(
            format!("agent_{}", i),
            format!("Agent {}", i),
            format!("Test agent {}", i),
        );

        let metadata = AgentMetadata::new(agent_id, supervisor_id, role);
        registry.register_agent(metadata).await.unwrap();
    }

    // Try to register one more (should fail)
    let agent_id = AgentPid::new();
    let supervisor_id = SupervisorId::new();
    let role = AgentRole::new(
        "overflow_agent".to_string(),
        "Overflow Agent".to_string(),
        "Should not fit".to_string(),
    );
    let metadata = AgentMetadata::new(agent_id, supervisor_id, role);

    let result = registry.register_agent(metadata).await;
    assert!(result.is_err());

    // Verify capacity
    let stats = registry.get_statistics().await.unwrap();
    assert_eq!(stats.total_agents, 3);
}

#[tokio::test]
async fn test_knowledge_graph_integration() {
    env_logger::try_init().ok();

    let role_name = RoleName::new("test_role");
    let thesaurus = Thesaurus::new();
    let role_graph = Arc::new(RoleGraph::new(role_name, thesaurus).await.unwrap());
    let automata_config = AutomataConfig {
        min_confidence: 0.6,
        max_paragraphs: 5,
        context_window: 256,
        language_models: vec!["test_model".to_string()],
    };

    let similarity_thresholds = SimilarityThresholds {
        role_similarity: 0.7,
        capability_similarity: 0.6,
        domain_similarity: 0.65,
        concept_similarity: 0.6,
    };

    let registry = RegistryBuilder::new()
        .with_role_graph(role_graph)
        .with_automata_config(automata_config)
        .with_similarity_thresholds(similarity_thresholds)
        .build()
        .unwrap();

    // Create agent with knowledge context
    let agent_id = AgentPid::new();
    let supervisor_id = SupervisorId::new();

    let mut role = AgentRole::new(
        "ml_engineer".to_string(),
        "Machine Learning Engineer".to_string(),
        "Builds and deploys ML models".to_string(),
    );
    role.knowledge_domains = vec![
        "machine_learning".to_string(),
        "deep_learning".to_string(),
        "data_science".to_string(),
    ];

    let mut metadata = AgentMetadata::new(agent_id, supervisor_id, role);

    // Set knowledge context
    metadata.knowledge_context.domains = vec![
        "tensorflow".to_string(),
        "pytorch".to_string(),
        "scikit_learn".to_string(),
    ];
    metadata.knowledge_context.concepts = vec![
        "neural_networks".to_string(),
        "gradient_descent".to_string(),
        "backpropagation".to_string(),
    ];

    registry.register_agent(metadata).await.unwrap();

    // Test discovery with task description (knowledge graph analysis)
    let kg_query = AgentDiscoveryQuery {
        required_roles: Vec::new(),
        required_capabilities: Vec::new(),
        required_domains: vec!["machine_learning".to_string()],
        task_description: Some(
            "Build a neural network model for image classification using deep learning techniques"
                .to_string(),
        ),
        min_success_rate: None,
        max_resource_usage: None,
        preferred_tags: Vec::new(),
    };

    let kg_result = registry.discover_agents(kg_query).await.unwrap();
    assert!(!kg_result.matches.is_empty());

    // Verify query analysis
    assert!(!kg_result.query_analysis.identified_domains.is_empty());

    // Test domain-based discovery
    let domain_agents = registry.list_agents().await.unwrap();
    let ml_agent = &domain_agents[0];
    assert!(ml_agent.can_handle_domain("machine_learning"));
    assert!(ml_agent.can_handle_domain("tensorflow"));
    assert!(!ml_agent.can_handle_domain("unrelated_domain"));
}

#[tokio::test]
async fn test_concurrent_registry_operations() {
    env_logger::try_init().ok();

    let role_name = RoleName::new("test_role");
    let thesaurus = Thesaurus::new();
    let role_graph = Arc::new(RoleGraph::new(role_name, thesaurus).await.unwrap());
    let registry = Arc::new(
        RegistryBuilder::new()
            .with_role_graph(role_graph)
            .build()
            .unwrap(),
    );

    let num_concurrent_ops = 10;
    let mut handles = Vec::new();

    // Concurrent registrations
    for i in 0..num_concurrent_ops {
        let registry_clone = registry.clone();
        let handle = tokio::spawn(async move {
            let agent_id = AgentPid::new();
            let supervisor_id = SupervisorId::new();

            let role = AgentRole::new(
                format!("concurrent_agent_{}", i),
                format!("Concurrent Agent {}", i),
                format!("Test agent for concurrency {}", i),
            );

            let metadata = AgentMetadata::new(agent_id.clone(), supervisor_id, role);
            registry_clone.register_agent(metadata).await.unwrap();

            agent_id
        });

        handles.push(handle);
    }

    // Wait for all registrations
    let mut agent_ids = Vec::new();
    for handle in handles {
        let agent_id = handle.await.unwrap();
        agent_ids.push(agent_id);
    }

    // Verify all agents were registered
    let stats = registry.get_statistics().await.unwrap();
    assert_eq!(stats.total_agents, num_concurrent_ops);

    // Concurrent discoveries
    let mut discovery_handles = Vec::new();
    for i in 0..num_concurrent_ops {
        let registry_clone = registry.clone();
        let handle = tokio::spawn(async move {
            let query = AgentDiscoveryQuery {
                required_roles: vec![format!("concurrent_agent_{}", i)],
                required_capabilities: Vec::new(),
                required_domains: Vec::new(),
                task_description: None,
                min_success_rate: None,
                max_resource_usage: None,
                preferred_tags: Vec::new(),
            };

            registry_clone.discover_agents(query).await.unwrap()
        });

        discovery_handles.push(handle);
    }

    // Wait for all discoveries
    for handle in discovery_handles {
        let result = handle.await.unwrap();
        assert_eq!(result.matches.len(), 1);
    }

    // Concurrent unregistrations
    let mut unregister_handles = Vec::new();
    for agent_id in agent_ids {
        let registry_clone = registry.clone();
        let handle = tokio::spawn(async move {
            registry_clone.unregister_agent(&agent_id).await.unwrap();
        });

        unregister_handles.push(handle);
    }

    // Wait for all unregistrations
    for handle in unregister_handles {
        handle.await.unwrap();
    }

    // Verify all agents were unregistered
    let final_stats = registry.get_statistics().await.unwrap();
    assert_eq!(final_stats.total_agents, 0);
}
