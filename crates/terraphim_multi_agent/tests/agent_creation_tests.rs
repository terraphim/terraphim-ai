use std::sync::Arc;
use terraphim_multi_agent::{test_utils::*, *};
use tokio_test;

#[tokio::test]
async fn test_agent_creation_with_defaults() {
    let result = create_test_agent().await;
    assert!(result.is_ok(), "Agent creation should succeed");

    let agent = result.unwrap();
    assert_eq!(agent.role_config.name.as_str(), "TestAgent");
    assert_eq!(agent.status, AgentStatus::Initializing);

    // Verify all components are initialized
    assert!(!agent.agent_id.to_string().is_empty());
    assert!(Arc::strong_count(&agent.memory) > 0);
    assert!(Arc::strong_count(&agent.tasks) > 0);
    assert!(Arc::strong_count(&agent.lessons) > 0);
}

#[tokio::test]
async fn test_agent_initialization() {
    let agent = create_test_agent().await.unwrap();

    let result = agent.initialize().await;
    assert!(result.is_ok(), "Agent initialization should succeed");

    assert_eq!(agent.status, AgentStatus::Ready);
}

#[tokio::test]
async fn test_agent_creation_with_role_config() {
    let role = create_test_role();
    let persistence = create_memory_storage().await.unwrap();
    let rolegraph = create_test_rolegraph();
    let automata = create_test_automata();

    let result = TerraphimAgent::new(
        role.clone(),
        persistence,
        Some((rolegraph.clone(), automata.clone())),
    )
    .await;

    assert!(result.is_ok());
    let agent = result.unwrap();

    // Verify role configuration is preserved
    assert_eq!(agent.role_config.name, role.name);
    assert_eq!(
        agent.role_config.relevance_function,
        role.relevance_function
    );
    assert_eq!(agent.role_config.theme, role.theme);

    // Verify knowledge graph components are set
    assert!(Arc::ptr_eq(&agent.rolegraph, &rolegraph));
    assert!(Arc::ptr_eq(&agent.automata, &automata));
}

#[tokio::test]
async fn test_agent_creation_without_knowledge_graph() {
    let role = create_test_role();
    let persistence = create_memory_storage().await.unwrap();

    let result = TerraphimAgent::new(role, persistence, None).await;
    assert!(
        result.is_ok(),
        "Agent creation should work without knowledge graph"
    );

    let agent = result.unwrap();
    // Should have default/empty knowledge graph components
    assert!(Arc::strong_count(&agent.rolegraph) > 0);
    assert!(Arc::strong_count(&agent.automata) > 0);
}

#[tokio::test]
async fn test_agent_memory_initialization() {
    let agent = create_test_agent().await.unwrap();

    // Test memory access
    let memory = agent.memory.read().await;
    assert_eq!(memory.get_current_version(), 1);

    // Test tasks access
    let tasks = agent.tasks.read().await;
    assert_eq!(tasks.get_current_version(), 1);

    // Test lessons access
    let lessons = agent.lessons.read().await;
    assert_eq!(lessons.get_current_version(), 1);
}

#[tokio::test]
async fn test_agent_tracking_initialization() {
    let agent = create_test_agent().await.unwrap();

    // Test token tracker
    let token_tracker = agent.token_tracker.read().await;
    assert_eq!(token_tracker.total_tokens, 0);

    // Test cost tracker
    let cost_tracker = agent.cost_tracker.read().await;
    assert_eq!(cost_tracker.total_cost_usd, 0.0);

    // Test command history
    let history = agent.command_history.read().await;
    assert_eq!(history.commands.len(), 0);
}

#[tokio::test]
async fn test_agent_context_initialization() {
    let agent = create_test_agent().await.unwrap();

    let context = agent.context.read().await;
    assert_eq!(context.items.len(), 0);
    assert_eq!(context.relevance_threshold, 0.5); // Default threshold
}

#[tokio::test]
async fn test_agent_llm_client_initialization() {
    let agent = create_test_agent().await.unwrap();

    // Verify LLM client is properly configured
    assert_eq!(agent.llm_client.get_agent_id(), agent.agent_id);

    // Test that we can get model capabilities
    let capabilities = agent.llm_client.get_model_capabilities();
    assert!(capabilities.context_window > 0);
    assert!(!capabilities.model_name.is_empty());
}

#[tokio::test]
async fn test_concurrent_agent_creation() {
    use tokio::task::JoinSet;

    let mut join_set = JoinSet::new();

    // Create multiple agents concurrently
    for i in 0..5 {
        join_set.spawn(async move {
            let result = create_test_agent().await;
            (i, result)
        });
    }

    let mut results = Vec::new();
    while let Some(result) = join_set.join_next().await {
        results.push(result.unwrap());
    }

    // All agents should be created successfully
    assert_eq!(results.len(), 5);
    for (i, result) in results {
        assert!(result.is_ok(), "Agent {} creation should succeed", i);
    }
}

#[tokio::test]
async fn test_agent_unique_ids() {
    let agent1 = create_test_agent().await.unwrap();
    let agent2 = create_test_agent().await.unwrap();

    // Each agent should have a unique ID
    assert_ne!(agent1.agent_id, agent2.agent_id);

    // But same role configuration
    assert_eq!(agent1.role_config.name, agent2.role_config.name);
}

#[tokio::test]
async fn test_agent_persistence_integration() {
    let agent = create_test_agent().await.unwrap();

    // Test that persistence is properly integrated
    assert!(Arc::strong_count(&agent.persistence) > 0);

    // Test that we can access persistence operations
    let persistence = &agent.persistence;

    // Test basic persistence operations (these should not fail)
    let test_key = format!("test_agent_{}", agent.agent_id);
    let test_data = b"test data";

    let write_result = persistence.fastest_op().write(&test_key, test_data).await;
    assert!(
        write_result.is_ok(),
        "Should be able to write to persistence"
    );

    let read_result = persistence.fastest_op().read(&test_key).await;
    assert!(
        read_result.is_ok(),
        "Should be able to read from persistence"
    );
}
