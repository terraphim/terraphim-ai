use chrono::Utc;
use std::collections::HashMap;
use terraphim_multi_agent::{test_utils::*, *};
use tokio_test;

#[tokio::test]
async fn test_token_usage_tracking_accuracy() {
    let agent = create_test_agent().await.unwrap();
    agent.initialize().await.unwrap();

    // Process a command and verify token tracking
    let input = CommandInput::new(CommandType::Generate, "Generate a simple Rust function");
    let result = agent.process_command(input).await;
    assert!(result.is_ok());

    let token_tracker = agent.token_tracker.read().await;

    // Verify token counts are realistic (mock LLM provides predictable values)
    assert!(
        token_tracker.total_tokens > 0,
        "Should have recorded input + output tokens"
    );
    assert!(
        token_tracker.input_tokens > 0,
        "Should have recorded input tokens"
    );
    assert!(
        token_tracker.output_tokens > 0,
        "Should have recorded output tokens"
    );

    // Total should equal sum of input and output
    assert_eq!(
        token_tracker.total_tokens,
        token_tracker.input_tokens + token_tracker.output_tokens,
        "Total tokens should equal input + output"
    );
}

#[tokio::test]
async fn test_cost_tracking_accuracy() {
    let agent = create_test_agent().await.unwrap();
    agent.initialize().await.unwrap();

    let initial_cost = {
        let cost_tracker = agent.cost_tracker.read().await;
        cost_tracker.total_cost_usd
    };

    // Process a command
    let input = CommandInput::new(CommandType::Answer, "What is the capital of France?");
    let result = agent.process_command(input).await;
    assert!(result.is_ok());

    let cost_tracker = agent.cost_tracker.read().await;

    // Cost should have increased
    assert!(
        cost_tracker.total_cost_usd > initial_cost,
        "Cost should increase after processing"
    );
    assert!(cost_tracker.total_cost_usd > 0.0, "Should have some cost");

    // Should have at least one cost record
    assert!(cost_tracker.records.len() > 0, "Should have cost records");

    let latest_record = cost_tracker.records.last().unwrap();
    assert_eq!(latest_record.agent_id, agent.agent_id);
    assert!(latest_record.cost_usd > 0.0);
    assert_eq!(latest_record.operation_type, "process_command");
}

#[tokio::test]
async fn test_token_tracking_multiple_commands() {
    let agent = create_test_agent().await.unwrap();
    agent.initialize().await.unwrap();

    let commands = vec![
        "Write a hello world function",
        "Explain async programming",
        "Review this code: fn main() {}",
    ];

    let mut previous_total = 0u32;

    for (i, prompt) in commands.iter().enumerate() {
        let input = CommandInput::new(CommandType::Generate, prompt);
        let result = agent.process_command(input).await;
        assert!(result.is_ok());

        let token_tracker = agent.token_tracker.read().await;

        // Each command should increase total token count
        assert!(
            token_tracker.total_tokens > previous_total,
            "Command {} should increase token count from {} to {}",
            i,
            previous_total,
            token_tracker.total_tokens
        );

        previous_total = token_tracker.total_tokens;
    }

    // Should have multiple records
    let token_tracker = agent.token_tracker.read().await;
    assert_eq!(
        token_tracker.records.len(),
        3,
        "Should have records for all commands"
    );
}

#[tokio::test]
async fn test_cost_calculation_by_model() {
    let agent = create_test_agent().await.unwrap();
    agent.initialize().await.unwrap();

    let input = CommandInput::new(
        CommandType::Analyze,
        "Complex analysis requiring many tokens",
    );
    let result = agent.process_command(input).await;
    assert!(result.is_ok());

    let cost_tracker = agent.cost_tracker.read().await;
    let token_tracker = agent.token_tracker.read().await;

    // Cost calculation should be based on token usage
    // Mock LLM uses predictable pricing
    let expected_cost = (token_tracker.input_tokens as f64 * 0.0015 / 1000.0)
        + (token_tracker.output_tokens as f64 * 0.002 / 1000.0);

    // Allow small floating point differences
    let cost_diff = (cost_tracker.total_cost_usd - expected_cost).abs();
    assert!(
        cost_diff < 0.0001,
        "Cost calculation should be accurate within precision"
    );
}

#[tokio::test]
async fn test_tracking_record_structure() {
    let agent = create_test_agent().await.unwrap();
    agent.initialize().await.unwrap();

    let input = CommandInput::new(CommandType::Create, "Create a data structure");
    let result = agent.process_command(input).await;
    assert!(result.is_ok());

    // Verify token usage record structure
    let token_tracker = agent.token_tracker.read().await;
    assert!(token_tracker.records.len() > 0);

    let token_record = &token_tracker.records[0];
    assert_eq!(token_record.agent_id, agent.agent_id);
    assert!(token_record.input_tokens > 0);
    assert!(token_record.output_tokens > 0);
    assert!(token_record.duration.as_millis() > 0);
    assert!(!token_record.operation_type.is_empty());
    assert!(token_record.timestamp <= Utc::now());

    // Verify cost record structure
    let cost_tracker = agent.cost_tracker.read().await;
    assert!(cost_tracker.records.len() > 0);

    let cost_record = &cost_tracker.records[0];
    assert_eq!(cost_record.agent_id, agent.agent_id);
    assert!(cost_record.cost_usd > 0.0);
    assert_eq!(cost_record.operation_type, "process_command");
    assert!(cost_record.timestamp <= Utc::now());
}

#[tokio::test]
async fn test_concurrent_tracking() {
    let agent = create_test_agent().await.unwrap();
    agent.initialize().await.unwrap();

    use tokio::task::JoinSet;
    let mut join_set = JoinSet::new();

    // Process multiple commands concurrently
    for i in 0..5 {
        let agent_clone = agent.clone();
        join_set.spawn(async move {
            let input =
                CommandInput::new(CommandType::Generate, &format!("Generate content {}", i));
            agent_clone.process_command(input).await
        });
    }

    let mut results = Vec::new();
    while let Some(result) = join_set.join_next().await {
        results.push(result.unwrap());
    }

    // All should succeed
    for result in results {
        assert!(result.is_ok());
    }

    // Verify tracking handled concurrency correctly
    let token_tracker = agent.token_tracker.read().await;
    let cost_tracker = agent.cost_tracker.read().await;

    assert_eq!(
        token_tracker.records.len(),
        5,
        "Should have 5 token usage records"
    );
    assert_eq!(cost_tracker.records.len(), 5, "Should have 5 cost records");

    // All costs should be positive
    assert!(cost_tracker.total_cost_usd > 0.0);
    assert!(token_tracker.total_tokens > 0);
}

#[tokio::test]
async fn test_tracking_metadata() {
    let agent = create_test_agent().await.unwrap();
    agent.initialize().await.unwrap();

    let input = CommandInput::new(CommandType::Review, "Review code quality");
    let result = agent.process_command(input).await;
    assert!(result.is_ok());

    let token_tracker = agent.token_tracker.read().await;
    let token_record = &token_tracker.records[0];

    // Verify metadata contains useful information
    assert!(token_record.metadata.contains_key("command_type"));
    assert_eq!(token_record.metadata.get("command_type").unwrap(), "Review");

    let cost_tracker = agent.cost_tracker.read().await;
    let cost_record = &cost_tracker.records[0];

    // Cost metadata should include model information
    assert!(cost_record.metadata.contains_key("model_name"));
    assert!(cost_record.metadata.get("model_name").unwrap().len() > 0);
}

#[tokio::test]
async fn test_budget_tracking() {
    let agent = create_test_agent().await.unwrap();
    agent.initialize().await.unwrap();

    // Set a budget limit
    {
        let mut cost_tracker = agent.cost_tracker.write().await;
        cost_tracker.daily_budget_usd = Some(0.10); // 10 cents
    }

    // Process several commands
    for i in 0..3 {
        let input = CommandInput::new(CommandType::Generate, &format!("Generate content {}", i));
        let result = agent.process_command(input).await;
        assert!(result.is_ok());
    }

    let cost_tracker = agent.cost_tracker.read().await;

    // Should track against budget (even if not enforced in mock)
    assert!(cost_tracker.daily_budget_usd.is_some());
    assert!(cost_tracker.total_cost_usd >= 0.0);
}

#[tokio::test]
async fn test_tracking_add_record_methods() {
    let agent = create_test_agent().await.unwrap();

    // Test manual token record addition
    let token_record = TokenUsageRecord {
        timestamp: Utc::now(),
        agent_id: agent.agent_id,
        operation_type: "test_operation".to_string(),
        input_tokens: 100,
        output_tokens: 50,
        total_tokens: 150,
        duration: std::time::Duration::from_millis(500),
        metadata: HashMap::new(),
    };

    {
        let mut token_tracker = agent.token_tracker.write().await;
        token_tracker.add_record(token_record);
    }

    let token_tracker = agent.token_tracker.read().await;
    assert_eq!(token_tracker.total_tokens, 150);
    assert_eq!(token_tracker.input_tokens, 100);
    assert_eq!(token_tracker.output_tokens, 50);
    assert_eq!(token_tracker.records.len(), 1);

    // Test manual cost record addition
    let cost_record = CostRecord {
        timestamp: Utc::now(),
        agent_id: agent.agent_id,
        operation_type: "test_operation".to_string(),
        cost_usd: 0.0015,
        metadata: HashMap::new(),
    };

    {
        let mut cost_tracker = agent.cost_tracker.write().await;
        cost_tracker.add_record(cost_record);
    }

    let cost_tracker = agent.cost_tracker.read().await;
    assert_eq!(cost_tracker.total_cost_usd, 0.0015);
    assert_eq!(cost_tracker.records.len(), 1);
}

#[tokio::test]
async fn test_tracking_time_accuracy() {
    let agent = create_test_agent().await.unwrap();
    agent.initialize().await.unwrap();

    let start_time = Utc::now();

    let input = CommandInput::new(CommandType::Analyze, "Detailed analysis task");
    let result = agent.process_command(input).await;
    assert!(result.is_ok());

    let end_time = Utc::now();

    let token_tracker = agent.token_tracker.read().await;
    let token_record = &token_tracker.records[0];

    // Verify timestamp is within reasonable bounds
    assert!(token_record.timestamp >= start_time);
    assert!(token_record.timestamp <= end_time);

    // Duration should be reasonable (mock LLM adds simulated processing time)
    assert!(token_record.duration.as_millis() > 0);
    assert!(token_record.duration.as_millis() < 10000); // Should be less than 10 seconds
}
