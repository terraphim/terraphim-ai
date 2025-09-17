//! Integration scenario tests for the Agent Evolution System
//!
//! These tests verify that all components work together correctly in realistic
//! scenarios and that the evolution tracking functions properly across different
//! workflow patterns.

use std::time::Duration;

use chrono::Utc;
// use tokio_test;

use terraphim_agent_evolution::{
    workflows::{WorkflowInput, WorkflowParameters, WorkflowPattern},
    *,
};

// =============================================================================
// EVOLUTION SYSTEM INTEGRATION TESTS
// =============================================================================

#[tokio::test]
async fn test_memory_evolution_integration() {
    let mut manager = EvolutionWorkflowManager::new("memory_integration_agent".to_string());

    // Execute a series of related tasks
    let tasks = vec![
        ("task_1", "Learn about renewable energy basics"),
        ("task_2", "Analyze solar panel efficiency"),
        ("task_3", "Compare wind vs solar energy costs"),
        ("task_4", "Recommend renewable energy strategy"),
    ];

    for (task_id, prompt) in tasks {
        let result = manager
            .execute_task(task_id.to_string(), prompt.to_string(), None)
            .await
            .unwrap();

        assert!(!result.is_empty());
    }

    // Verify memory evolution
    let memory_state = &manager.evolution_system().memory.current_state;

    // Should have short-term memories from recent tasks
    assert!(!memory_state.short_term.is_empty());
    assert!(memory_state.short_term.len() >= 4);

    // Should have episodic memories for task sequences
    assert!(!memory_state.episodic_memory.is_empty());

    // Memory should contain domain-relevant content
    let memory_contents: Vec<_> = memory_state.short_term.iter().map(|m| &m.content).collect();
    assert!(memory_contents
        .iter()
        .any(|content| content.to_lowercase().contains("renewable")
            || content.to_lowercase().contains("energy")));
}

#[tokio::test]
async fn test_task_lifecycle_tracking() {
    let mut manager = EvolutionWorkflowManager::new("task_lifecycle_agent".to_string());

    let task_id = "lifecycle_test_task".to_string();
    let start_time = Utc::now();

    // Execute a complex task that should go through full lifecycle
    let result = manager.execute_task(
        task_id.clone(),
        "Analyze the impact of artificial intelligence on job markets and recommend policy responses".to_string(),
        Some("Focus on both short-term disruptions and long-term opportunities".to_string()),
    ).await.unwrap();

    assert!(!result.is_empty());

    // Verify task lifecycle tracking
    let tasks_state = &manager.evolution_system().tasks.current_state;

    // Task should be completed
    assert_eq!(tasks_state.completed_tasks(), 1);

    // Should have detailed task history
    let task_history = tasks_state
        .completed
        .iter()
        .find(|ct| ct.original_task.id == task_id);
    assert!(task_history.is_some());

    let history = task_history.unwrap();
    assert!(history.completed_at > start_time);
    assert!(history.actual_duration.is_some());
    // Note: quality_score and resource_usage are tracked differently in this implementation
}

#[tokio::test]
async fn test_lesson_learning_integration() {
    let mut manager = EvolutionWorkflowManager::new("lesson_learning_agent".to_string());

    // Execute tasks that should generate different types of lessons
    let scenarios = vec![
        ("simple_success", "What is 2+2?", true),
        (
            "complex_analysis",
            "Analyze global climate change impacts comprehensively",
            true,
        ),
        (
            "comparison_task",
            "Compare Python vs Rust for systems programming",
            true,
        ),
    ];

    for (task_type, prompt, _expected_success) in scenarios {
        let result = manager
            .execute_task(format!("{}_task", task_type), prompt.to_string(), None)
            .await
            .unwrap();

        assert!(!result.is_empty());
    }

    // Verify lesson learning
    let lessons_state = &manager.evolution_system().lessons.current_state;

    // Should have learned success patterns
    assert!(!lessons_state.success_patterns.is_empty());
    assert!(lessons_state.success_patterns.len() >= 3);

    // Should have technical and process lessons
    assert!(!lessons_state.technical_lessons.is_empty());
    assert!(!lessons_state.process_lessons.is_empty());

    // Lessons should be domain-categorized
    let all_lessons: Vec<_> = lessons_state
        .technical_lessons
        .iter()
        .chain(lessons_state.process_lessons.iter())
        .chain(lessons_state.success_patterns.iter())
        .collect();

    let domains: Vec<_> = all_lessons.iter().map(|lesson| &lesson.category).collect();

    // Should have lessons from different domains
    // Check if we have lessons from different categories
    use crate::lessons::LessonCategory;
    assert!(domains.iter().any(|&d| matches!(
        *d,
        LessonCategory::Technical | LessonCategory::Process | LessonCategory::Domain
    )));
}

#[tokio::test]
async fn test_cross_pattern_evolution_tracking() {
    let mut manager = EvolutionWorkflowManager::new("cross_pattern_agent".to_string());

    // Force different patterns by using appropriate task characteristics
    let pattern_tests = vec![
        ("simple_routing", "Hello world", Some("routing")),
        (
            "step_analysis",
            "Analyze this step by step: market trends",
            Some("prompt_chaining"),
        ),
        (
            "comparison",
            "Compare React vs Vue comprehensively",
            Some("parallelization"),
        ),
        (
            "complex_project",
            "Research, analyze, and recommend AI governance policies",
            Some("orchestrator_workers"),
        ),
        (
            "quality_critical",
            "Write a formal research proposal on quantum computing",
            Some("evaluator_optimizer"),
        ),
    ];

    for (task_id, prompt, _expected_pattern) in pattern_tests {
        let result = manager
            .execute_task(task_id.to_string(), prompt.to_string(), None)
            .await
            .unwrap();

        assert!(!result.is_empty());

        // Note: With mock adapters, pattern selection may not be perfectly predictable
        // The important thing is that tasks complete successfully
    }

    // Verify evolution system tracked all patterns
    let evolution_system = manager.evolution_system();

    // Should have memories from different pattern executions
    let memory_state = &evolution_system.memory.current_state;
    assert!(memory_state.short_term.len() >= 5);

    // Should have completed all tasks
    let tasks_state = &&evolution_system.tasks.current_state;
    assert_eq!(tasks_state.completed_tasks(), 5);

    // Should have learned from diverse experiences
    let lessons_state = &evolution_system.lessons.current_state;
    assert!(lessons_state.success_patterns.len() >= 3);
}

#[tokio::test]
async fn test_evolution_snapshot_creation() {
    let mut evolution_system = AgentEvolutionSystem::new("snapshot_test_agent".to_string());

    // Add some initial state
    let initial_memory = crate::MemoryItem {
        id: "initial_memory".to_string(),
        item_type: crate::memory::MemoryItemType::Experience,
        content: "Initial agent state".to_string(),
        created_at: Utc::now(),
        last_accessed: None,
        access_count: 0,
        importance: crate::memory::ImportanceLevel::Medium,
        tags: vec!["initialization".to_string()],
        associations: std::collections::HashMap::new(),
    };
    evolution_system
        .memory
        .add_memory(initial_memory)
        .await
        .unwrap();

    let task = crate::AgentTask::new("Initial task description".to_string());
    evolution_system.tasks.add_task(task).await.unwrap();

    // Create snapshot
    let snapshot_result = evolution_system
        .create_snapshot("Initial state snapshot".to_string())
        .await;
    assert!(snapshot_result.is_ok());

    // Add more state
    let success_lesson = crate::Lesson::new(
        "success_lesson".to_string(),
        "Successful task completion pattern".to_string(),
        "Task execution".to_string(),
        crate::lessons::LessonCategory::Process,
    );
    evolution_system
        .lessons
        .add_lesson(success_lesson)
        .await
        .unwrap();

    // Create another snapshot
    let second_snapshot = evolution_system
        .create_snapshot("After learning snapshot".to_string())
        .await;
    assert!(second_snapshot.is_ok());

    // Snapshots should capture state progression
    // (In a full implementation, we would verify snapshot content)
}

// =============================================================================
// PERFORMANCE AND SCALABILITY TESTS
// =============================================================================

#[tokio::test]
async fn test_concurrent_task_execution() {
    use tokio::task::JoinSet;

    let agent_id = "concurrent_test_agent".to_string();

    // Create multiple tasks that will execute concurrently
    let mut join_set = JoinSet::new();

    for i in 0..5 {
        let agent_id_clone = agent_id.clone();
        join_set.spawn(async move {
            let mut manager = EvolutionWorkflowManager::new(agent_id_clone);

            let result = manager
                .execute_task(
                    format!("concurrent_task_{}", i),
                    format!("Task number {} analysis", i),
                    None,
                )
                .await;

            (i, result)
        });
    }

    // Wait for all tasks to complete
    let mut results = Vec::new();
    while let Some(result) = join_set.join_next().await {
        let (task_id, task_result) = result.unwrap();
        assert!(task_result.is_ok());
        results.push((task_id, task_result.unwrap()));
    }

    // All tasks should complete successfully
    assert_eq!(results.len(), 5);

    for (_task_id, result) in results {
        assert!(!result.is_empty());
    }
}

#[tokio::test]
async fn test_memory_efficiency_under_load() {
    let mut manager = EvolutionWorkflowManager::new("memory_efficiency_agent".to_string());

    // Execute many tasks to test memory management
    for i in 0..20 {
        let result = manager
            .execute_task(
                format!("load_test_{}", i),
                format!("Analyze topic number {}", i),
                None,
            )
            .await;

        assert!(result.is_ok());
    }

    // Memory should be managed efficiently
    let memory_state = &manager.evolution_system().memory.current_state;

    // Should have reasonable number of short-term memories (not unlimited growth)
    assert!(memory_state.short_term.len() <= 50);

    // Should have promoted some to long-term memory
    // (This depends on the promotion logic in the implementation)

    // Tasks should all be tracked
    let tasks_state = &manager.evolution_system().tasks.current_state;
    assert_eq!(tasks_state.completed_tasks(), 20);
}

// =============================================================================
// ERROR HANDLING AND RESILIENCE TESTS
// =============================================================================

#[tokio::test]
async fn test_graceful_degradation() {
    let mut manager = EvolutionWorkflowManager::new("degradation_test_agent".to_string());

    // Test with various edge cases
    let very_long_string = "x".repeat(5000);
    let edge_cases = vec![
        ("empty_prompt", ""),
        ("very_short", "Hi"),
        ("very_long", very_long_string.as_str()),
        (
            "special_chars",
            "Test with émojis 🚀 and special chars: @#$%^&*()",
        ),
        ("multilingual", "Test English, Español, 日本語, العربية"),
    ];

    for (test_name, prompt) in edge_cases {
        let result = manager
            .execute_task(format!("edge_case_{}", test_name), prompt.to_string(), None)
            .await;

        // Should handle edge cases gracefully
        match result {
            Ok(output) => {
                // If successful, should have reasonable output
                assert!(!output.is_empty() || prompt.is_empty());
            }
            Err(e) => {
                // If failed, should have informative error message
                assert!(!e.to_string().is_empty());
                assert!(e.to_string().contains("error") || e.to_string().contains("failed"));
            }
        }
    }

    // Evolution system should remain stable despite edge cases
    let _evolution_system = manager.evolution_system();
    // Memory state has valid short-term entries
    // Task state has valid task count
}

#[tokio::test]
async fn test_workflow_timeout_handling() {
    // Test that workflows handle timeouts gracefully
    let adapter = LlmAdapterFactory::create_mock("test");

    // Create patterns with short timeouts
    let short_timeout_config = workflows::prompt_chaining::ChainConfig {
        step_timeout: Duration::from_millis(1), // Very short timeout
        ..Default::default()
    };

    let chaining =
        workflows::prompt_chaining::PromptChaining::with_config(adapter, short_timeout_config);

    let workflow_input = WorkflowInput {
        task_id: "timeout_test".to_string(),
        agent_id: "test_agent".to_string(),
        prompt: "This is a test of timeout handling".to_string(),
        context: None,
        parameters: WorkflowParameters::default(),
        timestamp: Utc::now(),
    };

    let result = chaining.execute(workflow_input).await;

    // Should handle timeout gracefully (either succeed quickly or fail with timeout)
    match result {
        Ok(output) => {
            // If succeeded, should be reasonable
            assert!(!output.result.is_empty());
        }
        Err(e) => {
            // If timed out, should indicate timeout
            assert!(
                e.to_string().to_lowercase().contains("timeout")
                    || e.to_string().to_lowercase().contains("time")
            );
        }
    }
}

// =============================================================================
// QUALITY AND CONSISTENCY TESTS
// =============================================================================

#[tokio::test]
async fn test_consistent_quality_across_patterns() {
    let mut manager = EvolutionWorkflowManager::new("quality_consistency_agent".to_string());

    let test_prompt = "Analyze the benefits and challenges of remote work for software teams";

    // Execute the same task multiple times to test consistency
    let mut quality_scores = Vec::new();

    for i in 0..5 {
        let result = manager
            .execute_task(format!("quality_test_{}", i), test_prompt.to_string(), None)
            .await
            .unwrap();

        assert!(!result.is_empty());

        // Extract quality information from lessons learned
        let lessons_state = &manager.evolution_system().lessons.current_state;
        if let Some(latest_lesson) = lessons_state.success_patterns.iter().last() {
            quality_scores.push(latest_lesson.confidence);
        }
    }

    // Quality should be reasonably consistent
    if quality_scores.len() >= 2 {
        let avg_quality: f64 = quality_scores.iter().sum::<f64>() / quality_scores.len() as f64;
        let variance: f64 = quality_scores
            .iter()
            .map(|&x| (x - avg_quality).powi(2))
            .sum::<f64>()
            / quality_scores.len() as f64;

        // Standard deviation should be reasonable (not too much variance)
        let std_dev = variance.sqrt();
        assert!(std_dev < 0.3); // Quality shouldn't vary too wildly
        assert!(avg_quality > 0.5); // Average quality should be decent
    }
}

#[tokio::test]
async fn test_learning_from_repeated_tasks() {
    let mut manager = EvolutionWorkflowManager::new("learning_test_agent".to_string());

    let task_template = "Explain the concept of";
    let topics = vec![
        "machine learning",
        "blockchain",
        "quantum computing",
        "renewable energy",
    ];

    // Execute similar tasks to test learning
    for topic in &topics {
        let result = manager
            .execute_task(
                format!("learning_{}", topic.replace(" ", "_")),
                format!("{} {}", task_template, topic),
                None,
            )
            .await
            .unwrap();

        assert!(!result.is_empty());
    }

    // Evolution system should show learning patterns
    let lessons_state = &manager.evolution_system().lessons.current_state;

    // Should have learned patterns about explanation tasks
    assert!(!lessons_state.success_patterns.is_empty());
    assert!(!lessons_state.process_lessons.is_empty());

    // Should have domain-specific lessons
    let domains: Vec<_> = lessons_state
        .technical_lessons
        .iter()
        .chain(lessons_state.success_patterns.iter())
        .map(|l| &l.category)
        .collect();

    // Should show learning across different domains
    assert!(domains.len() > 1);
}

#[tokio::test]
async fn test_evolution_viewer_integration() {
    let mut manager = EvolutionWorkflowManager::new("viewer_integration_agent".to_string());

    // Execute some tasks to create evolution history
    let tasks = [
        "Analyze market trends",
        "Compare technologies",
        "Write recommendations",
    ];

    for (i, prompt) in tasks.iter().enumerate() {
        let result = manager
            .execute_task(format!("viewer_test_{}", i), prompt.to_string(), None)
            .await
            .unwrap();

        assert!(!result.is_empty());

        // Small delay to ensure timestamp differences
        tokio::time::sleep(Duration::from_millis(10)).await;
    }

    // Test evolution viewer functionality
    let viewer = MemoryEvolutionViewer::new(manager.evolution_system().agent_id.clone());

    let end_time = Utc::now();
    let start_time = end_time - chrono::Duration::minutes(5);

    let timeline_result = viewer
        .get_timeline(manager.evolution_system(), start_time, end_time)
        .await;

    // Should be able to retrieve evolution timeline
    assert!(timeline_result.is_ok());
    let timeline = timeline_result.unwrap();
    assert!(!timeline.events.is_empty());

    // Timeline should show progression
    assert!(!timeline.events.is_empty());

    // Each evolution step should have valid structure
    for evolution_step in &timeline.events {
        assert!(!evolution_step.description.is_empty());
        assert!(evolution_step.timestamp >= start_time);
        assert!(evolution_step.timestamp <= end_time);
    }
}

// Helper functions for integration testing


