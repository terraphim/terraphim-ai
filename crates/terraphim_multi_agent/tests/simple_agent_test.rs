use terraphim_multi_agent::{test_utils::*, *};

#[tokio::test]
async fn test_agent_creation_simple() {
    let agent = create_test_agent().await.unwrap();

    // Test basic agent functionality
    assert!(agent.agent_id != uuid::Uuid::nil());
    assert_eq!(*agent.status.read().await, AgentStatus::Initializing);

    // Initialize the agent
    let init_result = agent.initialize().await;
    assert!(init_result.is_ok(), "Agent should initialize successfully");
}

#[tokio::test]
async fn test_agent_command_processing() {
    if std::env::var("RUN_OLLAMA_TESTS").ok().as_deref() != Some("1") {
        eprintln!("Skipping: set RUN_OLLAMA_TESTS=1 and ensure Ollama has model gemma3:270m");
        return;
    }
    let agent = create_test_agent().await.unwrap();
    agent.initialize().await.unwrap();

    // Process a simple command
    let input = CommandInput::new("Hello world".to_string(), CommandType::Generate);
    let result = agent.process_command(input).await;

    // Should succeed with Rig integration
    assert!(result.is_ok(), "Command processing should work with Rig");
}

#[tokio::test]
async fn test_agent_role_config() {
    let agent = create_test_agent().await.unwrap();

    // Test role configuration
    assert_eq!(agent.role_config.name.to_string(), "TestAgent");

    // Should have LLM configuration in extra
    assert!(agent.role_config.extra.contains_key("llm_provider"));
    assert_eq!(
        agent.role_config.extra.get("llm_provider").unwrap(),
        &serde_json::json!("ollama")
    );
}
