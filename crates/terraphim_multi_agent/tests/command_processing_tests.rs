use terraphim_multi_agent::{test_utils::*, *};

#[tokio::test]
async fn test_generate_command_processing() {
    let agent = create_test_agent().await.unwrap();
    agent.initialize().await.unwrap();

    let input = CommandInput::new(
        "Write a hello world function in Rust".to_string(),
        CommandType::Generate,
    );
    let result = agent.process_command(input).await;

    assert!(result.is_ok(), "Generate command should succeed");
    let output = result.unwrap();

    // Verify output structure
    assert!(
        !output.text.is_empty(),
        "Output should contain generated text"
    );
    assert!(
        output.quality_score >= 0.0 && output.quality_score <= 1.0,
        "Quality score should be normalized"
    );
    assert!(
        output.processing_duration.as_millis() > 0,
        "Should have processing time"
    );
    assert_eq!(output.command_type, CommandType::Generate);
}

#[tokio::test]
async fn test_answer_command_processing() {
    let agent = create_test_agent().await.unwrap();
    agent.initialize().await.unwrap();

    let input = CommandInput::new(
        "What is Rust programming language?".to_string(),
        CommandType::Answer,
    );
    let result = agent.process_command(input).await;

    assert!(result.is_ok(), "Answer command should succeed");
    let output = result.unwrap();

    assert!(!output.text.is_empty(), "Answer should contain text");
    assert_eq!(output.command_type, CommandType::Answer);
}

#[tokio::test]
async fn test_analyze_command_processing() {
    let agent = create_test_agent().await.unwrap();
    agent.initialize().await.unwrap();

    let input = CommandInput::new(
        "Analyze the performance characteristics of HashMap vs BTreeMap".to_string(),
        CommandType::Analyze,
    );
    let result = agent.process_command(input).await;

    assert!(result.is_ok(), "Analyze command should succeed");
    let output = result.unwrap();

    assert!(!output.text.is_empty(), "Analysis should contain text");
    assert_eq!(output.command_type, CommandType::Analyze);
}

#[tokio::test]
async fn test_create_command_processing() {
    let agent = create_test_agent().await.unwrap();
    agent.initialize().await.unwrap();

    let input = CommandInput::new(
        "Create a new API endpoint design for user authentication".to_string(),
        CommandType::Create,
    );
    let result = agent.process_command(input).await;

    assert!(result.is_ok(), "Create command should succeed");
    let output = result.unwrap();

    assert!(!output.text.is_empty(), "Creation should contain text");
    assert_eq!(output.command_type, CommandType::Create);
}

#[tokio::test]
async fn test_review_command_processing() {
    let agent = create_test_agent().await.unwrap();
    agent.initialize().await.unwrap();

    let input = CommandInput::new(
        "Review this code: fn main() { println!(\"Hello\"); }".to_string(),
        CommandType::Review,
    );
    let result = agent.process_command(input).await;

    assert!(result.is_ok(), "Review command should succeed");
    let output = result.unwrap();

    assert!(!output.text.is_empty(), "Review should contain text");
    assert_eq!(output.command_type, CommandType::Review);
}

#[tokio::test]
async fn test_command_with_context() {
    let agent = create_test_agent().await.unwrap();
    agent.initialize().await.unwrap();

    // First add some context to the agent
    {
        let mut context = agent.context.write().await;
        context
            .add_item(ContextItem::new(
                ContextItemType::Memory,
                "User prefers functional programming patterns".to_string(),
                30, // token count
                0.8,
            ))
            .unwrap();
        context
            .add_item(ContextItem::new(
                ContextItemType::Task,
                "Working on a web API project".to_string(),
                25, // token count
                0.9,
            ))
            .unwrap();
    }

    let input = CommandInput::new(
        "Write a function to handle HTTP requests".to_string(),
        CommandType::Generate,
    );
    let result = agent.process_command(input).await;

    assert!(result.is_ok(), "Command with context should succeed");
    let output = result.unwrap();

    // The mock should have included context in the response
    assert!(!output.text.is_empty());
    assert!(
        output.context_used.len() > 0,
        "Should have used available context"
    );
}

#[tokio::test]
async fn test_command_tracking() {
    let agent = create_test_agent().await.unwrap();
    agent.initialize().await.unwrap();

    let input = CommandInput::new(
        "Test command for tracking".to_string(),
        CommandType::Generate,
    );
    let result = agent.process_command(input).await;
    assert!(result.is_ok());

    // Verify command was tracked
    let history = agent.command_history.read().await;
    assert_eq!(
        history.commands.len(),
        1,
        "Command should be recorded in history"
    );

    let recorded_command = &history.commands[0];
    assert_eq!(recorded_command.command_type, CommandType::Generate);
    assert_eq!(recorded_command.input, "Test command for tracking");
    assert!(!recorded_command.output.is_empty());

    // Verify token tracking
    let token_tracker = agent.token_tracker.read().await;
    assert!(
        token_tracker.total_input_tokens + token_tracker.total_output_tokens > 0,
        "Should have recorded token usage"
    );

    // Verify cost tracking
    let cost_tracker = agent.cost_tracker.read().await;
    assert!(
        cost_tracker.total_cost_usd >= 0.0,
        "Should have recorded costs"
    );
}

#[tokio::test]
async fn test_concurrent_command_processing() {
    let agent = create_test_agent().await.unwrap();
    agent.initialize().await.unwrap();

    use tokio::task::JoinSet;
    let mut join_set = JoinSet::new();

    // Process multiple commands concurrently
    let commands = vec![
        ("Generate", "Write hello world"),
        ("Answer", "What is Rust?"),
        ("Analyze", "Compare Vec and LinkedList"),
        ("Create", "Design a REST API"),
        ("Review", "fn test() {}"),
    ];

    for (i, (cmd_type, prompt)) in commands.into_iter().enumerate() {
        let agent_clone = agent.clone();
        join_set.spawn(async move {
            let cmd_type = match cmd_type {
                "Generate" => CommandType::Generate,
                "Answer" => CommandType::Answer,
                "Analyze" => CommandType::Analyze,
                "Create" => CommandType::Create,
                "Review" => CommandType::Review,
                _ => CommandType::Generate,
            };
            let input = CommandInput::new(prompt.to_string(), cmd_type);
            (i, agent_clone.process_command(input).await)
        });
    }

    let mut results = Vec::new();
    while let Some(result) = join_set.join_next().await {
        results.push(result.unwrap());
    }

    // All commands should succeed
    assert_eq!(results.len(), 5);
    for (i, result) in results {
        assert!(result.is_ok(), "Concurrent command {} should succeed", i);
    }

    // Verify all commands were tracked
    let history = agent.command_history.read().await;
    assert_eq!(history.commands.len(), 5, "All commands should be tracked");
}

#[tokio::test]
async fn test_command_input_validation() {
    let agent = create_test_agent().await.unwrap();
    agent.initialize().await.unwrap();

    // Test empty prompt
    let input = CommandInput::new("".to_string(), CommandType::Generate);
    let result = agent.process_command(input).await;

    // Should still work (mock LLM will handle empty input)
    assert!(result.is_ok(), "Empty prompt should be handled gracefully");
}

#[tokio::test]
async fn test_command_quality_scoring() {
    let agent = create_test_agent().await.unwrap();
    agent.initialize().await.unwrap();

    let input = CommandInput::new(
        "Write excellent Rust code".to_string(),
        CommandType::Generate,
    );
    let result = agent.process_command(input).await;
    assert!(result.is_ok());

    let output = result.unwrap();

    // Quality score should be within valid range
    assert!(
        output.quality_score >= 0.0,
        "Quality score should be non-negative"
    );
    assert!(
        output.quality_score <= 1.0,
        "Quality score should not exceed 1.0"
    );
}

#[tokio::test]
async fn test_context_injection() {
    let agent = create_test_agent().await.unwrap();
    agent.initialize().await.unwrap();

    // Add relevant context
    {
        let mut context = agent.context.write().await;
        context
            .add_item(ContextItem::new(
                ContextItemType::Memory,
                "User is working on performance optimization".to_string(),
                35, // token count
                0.95,
            ))
            .unwrap();
    }

    let input = CommandInput::new(
        "How to optimize this code?".to_string(),
        CommandType::Analyze,
    );
    let result = agent.process_command(input).await;
    assert!(result.is_ok());

    let output = result.unwrap();
    assert!(
        output.context_used.len() > 0,
        "High-relevance context should be included"
    );
}

#[tokio::test]
async fn test_command_temperature_control() {
    let agent = create_test_agent().await.unwrap();
    agent.initialize().await.unwrap();

    // Different command types should use appropriate temperatures
    // (This is mainly testing that the commands execute with different configs)

    let creative_input =
        CommandInput::new("Write creative content".to_string(), CommandType::Generate);
    let creative_result = agent.process_command(creative_input).await;
    assert!(creative_result.is_ok());

    let analytical_input = CommandInput::new("Analyze this data".to_string(), CommandType::Analyze);
    let analytical_result = agent.process_command(analytical_input).await;
    assert!(analytical_result.is_ok());

    // Both should succeed (temperature differences handled by LLM client)
    assert!(!creative_result.unwrap().text.is_empty());
    assert!(!analytical_result.unwrap().text.is_empty());
}
