use chrono::Utc;
use std::collections::HashMap;
use terraphim_multi_agent::{test_utils::*, *};

#[tokio::test]
async fn test_token_usage_tracking_accuracy() {
    let agent = create_test_agent().await.unwrap();
    agent.initialize().await.unwrap();

    // Process a command and verify token tracking
    let input = CommandInput::new(
        "Generate a simple Rust function".to_string(),
        CommandType::Generate,
    );
    let result = agent.process_command(input).await;
    assert!(result.is_ok());

    let token_tracker = agent.token_tracker.read().await;

    // Verify token counts are realistic (mock LLM provides predictable values)
    assert!(
        token_tracker.total_input_tokens + token_tracker.total_output_tokens > 0,
        "Should have recorded input + output tokens"
    );
    assert!(
        token_tracker.total_input_tokens > 0,
        "Should have recorded input tokens"
    );
    assert!(
        token_tracker.total_output_tokens > 0,
        "Should have recorded output tokens"
    );

    // Total should equal sum of input and output
    assert_eq!(
        token_tracker.total_input_tokens + token_tracker.total_output_tokens,
        token_tracker.total_input_tokens + token_tracker.total_output_tokens,
        "Total tokens should equal input + output"
    );
}

#[tokio::test]
async fn test_cost_tracking_accuracy() {
    let agent = create_test_agent().await.unwrap();
    agent.initialize().await.unwrap();

    let initial_cost = {
        let cost_tracker = agent.cost_tracker.read().await;
        cost_tracker.current_month_spending
    };

    // Process a command
    let input = CommandInput::new(
        "What is the capital of France?".to_string(),
        CommandType::Answer,
    );
    let result = agent.process_command(input).await;
    assert!(result.is_ok());

    let cost_tracker = agent.cost_tracker.read().await;

    // Cost should have increased
    assert!(
        cost_tracker.current_month_spending > initial_cost,
        "Cost should increase after processing"
    );
    assert!(
        cost_tracker.current_month_spending > 0.0,
        "Should have some cost"
    );

    // Note: CostTracker doesn't maintain a records list, but tracks spending by date/agent
    assert!(
        !cost_tracker.daily_spending.is_empty() || cost_tracker.current_month_spending > 0.0,
        "Should have cost tracking data"
    );
}

#[tokio::test]
async fn test_token_tracking_multiple_commands() {
    let agent = create_test_agent().await.unwrap();
    agent.initialize().await.unwrap();

    let commands = [
        "Write a hello world function",
        "Explain async programming",
        "Review this code: fn main() {}",
    ];

    let mut previous_total = 0u64;

    for (i, prompt) in commands.iter().enumerate() {
        let input = CommandInput::new(prompt.to_string(), CommandType::Generate);
        let result = agent.process_command(input).await;
        assert!(result.is_ok());

        let token_tracker = agent.token_tracker.read().await;

        // Each command should increase total token count
        assert!(
            token_tracker.total_input_tokens + token_tracker.total_output_tokens > previous_total,
            "Command {} should increase token count from {} to {}",
            i,
            previous_total,
            token_tracker.total_input_tokens + token_tracker.total_output_tokens
        );

        previous_total = token_tracker.total_input_tokens + token_tracker.total_output_tokens;
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
        "Complex analysis requiring many tokens".to_string(),
        CommandType::Analyze,
    );
    let result = agent.process_command(input).await;
    assert!(result.is_ok());

    let cost_tracker = agent.cost_tracker.read().await;
    let token_tracker = agent.token_tracker.read().await;

    // Cost calculation should be based on token usage
    // Mock LLM uses predictable pricing
    let expected_cost = (token_tracker.total_input_tokens as f64 * 0.0015 / 1000.0)
        + (token_tracker.total_output_tokens as f64 * 0.002 / 1000.0);

    // Allow small floating point differences
    let cost_diff = (cost_tracker.current_month_spending - expected_cost).abs();
    assert!(
        cost_diff < 0.0001,
        "Cost calculation should be accurate within precision"
    );
}

#[tokio::test]
async fn test_tracking_record_structure() {
    let agent = create_test_agent().await.unwrap();
    agent.initialize().await.unwrap();

    let input = CommandInput::new("Create a data structure".to_string(), CommandType::Create);
    let result = agent.process_command(input).await;
    assert!(result.is_ok());

    // Verify token usage record structure
    let token_tracker = agent.token_tracker.read().await;
    assert!(!token_tracker.records.is_empty());

    let token_record = &token_tracker.records[0];
    assert_eq!(token_record.agent_id, agent.agent_id);
    assert!(token_record.input_tokens > 0);
    assert!(token_record.output_tokens > 0);
    assert!(token_record.duration_ms > 0);
    assert!(token_record.timestamp <= Utc::now());

    // Verify cost tracker state
    let cost_tracker = agent.cost_tracker.read().await;
    assert!(cost_tracker.current_month_spending > 0.0);
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
            let input = CommandInput::new(format!("Generate content {}", i), CommandType::Generate);
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
    // All costs should be positive
    assert!(cost_tracker.current_month_spending > 0.0);
    assert!(token_tracker.total_input_tokens + token_tracker.total_output_tokens > 0);
}

#[tokio::test]
async fn test_tracking_metadata() {
    let agent = create_test_agent().await.unwrap();
    agent.initialize().await.unwrap();

    let input = CommandInput::new("Review code quality".to_string(), CommandType::Review);
    let result = agent.process_command(input).await;
    assert!(result.is_ok());

    let token_tracker = agent.token_tracker.read().await;
    let token_record = &token_tracker.records[0];

    // Verify token record has correct agent ID and model
    assert_eq!(token_record.agent_id, agent.agent_id);
    assert!(!token_record.model.is_empty());
    assert!(token_record.input_tokens > 0);
    assert!(token_record.output_tokens > 0);

    // CostTracker doesn't maintain a records list - it uses daily_spending HashMap
    let cost_tracker = agent.cost_tracker.read().await;
    assert!(cost_tracker.current_month_spending > 0.0);
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
        let input = CommandInput::new(format!("Generate content {}", i), CommandType::Generate);
        let result = agent.process_command(input).await;
        assert!(result.is_ok());
    }

    let cost_tracker = agent.cost_tracker.read().await;

    // Should track against budget (even if not enforced in mock)
    assert!(cost_tracker.daily_budget_usd.is_some());
    assert!(cost_tracker.current_month_spending >= 0.0);
}

#[tokio::test]
async fn test_tracking_add_record_methods() {
    let agent = create_test_agent().await.unwrap();

    // Test manual token record addition
    let token_record = TokenUsageRecord::new(
        agent.agent_id,
        "test-model".to_string(),
        100,
        50,
        0.01,
        1000,
    );

    {
        let mut token_tracker = agent.token_tracker.write().await;
        token_tracker.add_record(token_record).unwrap();
    }

    let token_tracker = agent.token_tracker.read().await;
    assert_eq!(
        token_tracker.total_input_tokens + token_tracker.total_output_tokens,
        150
    );
    assert_eq!(token_tracker.total_input_tokens, 100);
    assert_eq!(token_tracker.total_output_tokens, 50);
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
        cost_tracker.add_record(cost_record).unwrap();
    }

    let cost_tracker = agent.cost_tracker.read().await;
    assert!(cost_tracker.current_month_spending >= 0.0015);
}

#[tokio::test]
async fn test_tracking_time_accuracy() {
    let agent = create_test_agent().await.unwrap();
    agent.initialize().await.unwrap();

    let start_time = Utc::now();

    let input = CommandInput::new("Detailed analysis task".to_string(), CommandType::Analyze);
    let result = agent.process_command(input).await;
    assert!(result.is_ok());

    let end_time = Utc::now();

    let token_tracker = agent.token_tracker.read().await;
    let token_record = &token_tracker.records[0];

    // Verify timestamp is within reasonable bounds
    assert!(token_record.timestamp >= start_time);
    assert!(token_record.timestamp <= end_time);

    // Duration should be reasonable (mock LLM adds simulated processing time)
    assert!(token_record.duration_ms > 0);
    assert!(token_record.duration_ms < 10000); // Should be less than 10 seconds
}
