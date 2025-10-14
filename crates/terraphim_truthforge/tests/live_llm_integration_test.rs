//! Live LLM integration tests using free OpenRouter models
//!
//! These tests require OPENROUTER_API_KEY environment variable to be set.
//! Run with: cargo test --test live_llm_integration_test -- --ignored

use chrono::Utc;
use std::sync::Arc;
use terraphim_multi_agent::GenAiLlmClient;
use terraphim_truthforge::{
    AnalysisCostTracker, AudienceType, NarrativeContext, NarrativeInput, StakeType,
    TwoPassDebateWorkflow, UrgencyLevel,
};
use uuid::Uuid;

/// Test using free models from OpenRouter (google/gemma-2-9b-it:free)
#[tokio::test]
#[ignore] // Run only when OPENROUTER_API_KEY is set
async fn test_full_workflow_with_free_model() {
    // Check for API key
    let _api_key = std::env::var("OPENROUTER_API_KEY").expect(
        "OPENROUTER_API_KEY environment variable must be set for live tests. \
         Get a free key at https://openrouter.ai/",
    );

    // Create LLM client with free model
    let client = Arc::new(
        GenAiLlmClient::new_openrouter(Some("google/gemma-2-9b-it:free".to_string()))
            .expect("Failed to create OpenRouter client"),
    );

    // Create a simple test narrative
    let narrative = NarrativeInput {
        session_id: Uuid::new_v4(),
        text: "We achieved a 40% cost reduction this quarter through process optimization. \
               This will improve our operational efficiency and deliver value to shareholders."
            .to_string(),
        context: NarrativeContext {
            urgency: UrgencyLevel::Low,
            stakes: vec![StakeType::Financial],
            audience: AudienceType::Internal,
        },
        submitted_at: Utc::now(),
    };

    // Create workflow with LLM client
    let workflow = TwoPassDebateWorkflow::new().with_llm_client(client);

    // Execute workflow
    let result = workflow
        .execute(&narrative)
        .await
        .expect("Workflow execution should succeed");

    // Verify basic results
    assert_eq!(result.session_id, narrative.session_id);
    assert!(
        !result.omission_catalog.omissions.is_empty(),
        "Should detect some omissions"
    );
    assert_eq!(
        result.response_strategies.len(),
        3,
        "Should generate 3 strategies"
    );

    // Print results for manual inspection
    println!("\n=== Live LLM Integration Test Results ===");
    println!("Session: {}", result.session_id);
    println!(
        "\nOmissions Detected: {}",
        result.omission_catalog.omissions.len()
    );
    println!("Response Strategies: {}", result.response_strategies.len());
    println!("\nExecutive Summary:\n{}", result.executive_summary);
    println!("\nProcessing Time: {}ms", result.processing_time_ms);
}

/// Test Pass One agents with free model
#[tokio::test]
#[ignore]
async fn test_pass_one_with_free_model() {
    std::env::var("OPENROUTER_API_KEY").expect("OPENROUTER_API_KEY must be set");

    let client = Arc::new(
        GenAiLlmClient::new_openrouter(Some("google/gemma-2-9b-it:free".to_string()))
            .expect("Failed to create client"),
    );

    let narrative = NarrativeInput {
        session_id: Uuid::new_v4(),
        text: "Company announces layoffs affecting 500 employees.".to_string(),
        context: NarrativeContext {
            urgency: UrgencyLevel::High,
            stakes: vec![StakeType::Reputational, StakeType::Financial],
            audience: AudienceType::PublicMedia,
        },
        submitted_at: Utc::now(),
    };

    let workflow = TwoPassDebateWorkflow::new().with_llm_client(client);
    let result = workflow.execute(&narrative).await.expect("Should execute");

    println!("\n=== Pass One Results ===");
    println!("Omissions: {}", result.omission_catalog.omissions.len());

    for (i, omission) in result.omission_catalog.omissions.iter().take(3).enumerate() {
        println!(
            "{}. {} (severity: {:.2}, exploitability: {:.2})",
            i + 1,
            omission.description,
            omission.severity,
            omission.exploitability
        );
    }
}

/// Test cost tracking with real LLM calls
#[tokio::test]
#[ignore]
async fn test_cost_tracking_integration() {
    std::env::var("OPENROUTER_API_KEY").expect("OPENROUTER_API_KEY must be set");

    let client = Arc::new(
        GenAiLlmClient::new_openrouter(Some("google/gemma-2-9b-it:free".to_string()))
            .expect("Failed to create client"),
    );

    let narrative = NarrativeInput {
        session_id: Uuid::new_v4(),
        text: "We reduced operational costs by 30%.".to_string(),
        context: NarrativeContext {
            urgency: UrgencyLevel::Low,
            stakes: vec![StakeType::Financial],
            audience: AudienceType::Internal,
        },
        submitted_at: Utc::now(),
    };

    // Note: In a real implementation, you would track costs during execution
    // For now, we're just validating the cost tracking infrastructure
    let mut cost_tracker = AnalysisCostTracker::new(narrative.session_id);

    let workflow = TwoPassDebateWorkflow::new().with_llm_client(client);
    let result = workflow.execute(&narrative).await.expect("Should execute");

    // Complete tracking
    cost_tracker.complete();

    println!("\n=== Cost Tracking Test ===");
    println!("Session: {}", result.session_id);
    println!("\nNote: Actual cost tracking during workflow execution will be");
    println!("implemented when LLM responses include token usage data.");
    println!("\nCurrent infrastructure ready for:");
    println!("- Per-agent cost tracking");
    println!("- Stage-based cost aggregation");
    println!("- Budget limit enforcement");
    println!("- Cost breakdown reporting");
}

/// Test response strategies with free model
#[tokio::test]
#[ignore]
async fn test_response_strategies_with_free_model() {
    std::env::var("OPENROUTER_API_KEY").expect("OPENROUTER_API_KEY must be set");

    let client = Arc::new(
        GenAiLlmClient::new_openrouter(Some("google/gemma-2-9b-it:free".to_string()))
            .expect("Failed to create client"),
    );

    let narrative = NarrativeInput {
        session_id: Uuid::new_v4(),
        text: "Company pivots to sustainable energy solutions.".to_string(),
        context: NarrativeContext {
            urgency: UrgencyLevel::Low,
            stakes: vec![StakeType::Reputational],
            audience: AudienceType::PublicMedia,
        },
        submitted_at: Utc::now(),
    };

    let workflow = TwoPassDebateWorkflow::new().with_llm_client(client);
    let result = workflow.execute(&narrative).await.expect("Should execute");

    println!("\n=== Response Strategies ===");
    for strategy in &result.response_strategies {
        println!(
            "\n{:?} Strategy ({:?}):",
            strategy.strategy_type, strategy.tone_guidance
        );
        println!("Rationale: {}", strategy.strategic_rationale);
        println!(
            "Media Risk: {:.2}",
            strategy.risk_assessment.media_amplification_risk
        );
        println!(
            "Vulnerabilities Addressed: {}",
            strategy.vulnerabilities_addressed.len()
        );
    }

    assert_eq!(
        result.response_strategies.len(),
        3,
        "Should have 3 strategies"
    );
}

/// Test with minimal narrative to keep costs low
#[tokio::test]
#[ignore]
async fn test_minimal_narrative_free_model() {
    std::env::var("OPENROUTER_API_KEY").expect("OPENROUTER_API_KEY must be set");

    let client = Arc::new(
        GenAiLlmClient::new_openrouter(Some("google/gemma-2-9b-it:free".to_string()))
            .expect("Failed to create client"),
    );

    let narrative = NarrativeInput {
        session_id: Uuid::new_v4(),
        text: "Product launch successful.".to_string(),
        context: NarrativeContext {
            urgency: UrgencyLevel::Low,
            stakes: vec![StakeType::Financial],
            audience: AudienceType::Internal,
        },
        submitted_at: Utc::now(),
    };

    let workflow = TwoPassDebateWorkflow::new().with_llm_client(client);
    let result = workflow.execute(&narrative).await;

    assert!(
        result.is_ok(),
        "Even minimal narratives should process successfully"
    );

    let result = result.unwrap();
    println!("\n=== Minimal Narrative Test ===");
    println!("Input: '{}'", narrative.text);
    println!("Omissions: {}", result.omission_catalog.omissions.len());
    println!("Processing: {}ms", result.processing_time_ms);
}
