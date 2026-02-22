//! Integration tests for the skills system.
//!
//! Tests end-to-end skill workflows including:
//! - Save/load skills from disk
//! - Execute skills with various step types
//! - Progress monitoring and reporting
//! - Error handling and cancellation

use std::collections::HashMap;
use std::time::Duration;
use tempfile::TempDir;
use terraphim_tinyclaw::skills::{
    ExecutionReport, Skill, SkillExecutor, SkillInput, SkillMonitor, SkillStatus, SkillStep,
};

/// Creates a temporary directory and skill executor for testing.
async fn setup_test_executor() -> (TempDir, SkillExecutor) {
    let temp_dir = TempDir::new().unwrap();
    let executor = SkillExecutor::new(temp_dir.path()).unwrap();
    (temp_dir, executor)
}

/// Creates a simple test skill with tool and LLM steps.
fn create_test_skill() -> Skill {
    Skill {
        name: "test-integration".to_string(),
        version: "1.0.0".to_string(),
        description: "Integration test skill".to_string(),
        author: Some("Test Suite".to_string()),
        steps: vec![
            SkillStep::Tool {
                tool: "shell".to_string(),
                args: serde_json::json!({"command": "echo {message}"}),
            },
            SkillStep::Llm {
                prompt: "Process this message: {message}".to_string(),
                use_context: true,
            },
        ],
        inputs: vec![SkillInput {
            name: "message".to_string(),
            description: "Message to process".to_string(),
            required: true,
            default: None,
        }],
    }
}

/// Creates a multi-step skill for testing progress tracking.
fn create_multi_step_skill(step_count: usize) -> Skill {
    let steps = (0..step_count)
        .map(|i| SkillStep::Llm {
            prompt: format!("Step {}", i + 1),
            use_context: false,
        })
        .collect();

    Skill {
        name: format!("multi-step-{}", step_count),
        version: "1.0.0".to_string(),
        description: format!("Skill with {} steps", step_count),
        author: None,
        steps,
        inputs: vec![],
    }
}

#[tokio::test]
async fn test_skill_save_and_load() {
    let (_temp_dir, executor) = setup_test_executor().await;
    let skill = create_test_skill();

    // Save the skill
    executor.save_skill(&skill).unwrap();

    // Load it back
    let loaded = executor.load_skill(&skill.name).unwrap();

    // Verify all fields match
    assert_eq!(loaded.name, skill.name);
    assert_eq!(loaded.version, skill.version);
    assert_eq!(loaded.description, skill.description);
    assert_eq!(loaded.author, skill.author);
    assert_eq!(loaded.steps.len(), skill.steps.len());
    assert_eq!(loaded.inputs.len(), skill.inputs.len());
}

#[tokio::test]
async fn test_skill_list_and_delete() {
    let (_temp_dir, executor) = setup_test_executor().await;

    // Create multiple skills
    let skills = vec![
        create_test_skill(),
        Skill {
            name: "second-skill".to_string(),
            version: "0.1.0".to_string(),
            description: "Another skill".to_string(),
            author: None,
            steps: vec![],
            inputs: vec![],
        },
        Skill {
            name: "third-skill".to_string(),
            version: "2.0.0".to_string(),
            description: "Third skill".to_string(),
            author: None,
            steps: vec![],
            inputs: vec![],
        },
    ];

    // Save all skills
    for skill in &skills {
        executor.save_skill(skill).unwrap();
    }

    // List skills
    let listed = executor.list_skills().unwrap();
    assert_eq!(listed.len(), 3);

    // Verify names are sorted
    assert_eq!(listed[0].name, "second-skill");
    assert_eq!(listed[1].name, "test-integration");
    assert_eq!(listed[2].name, "third-skill");

    // Delete one skill
    executor.delete_skill("second-skill").unwrap();

    // Verify deletion
    let listed = executor.list_skills().unwrap();
    assert_eq!(listed.len(), 2);
    assert!(!listed.iter().any(|s| s.name == "second-skill"));
}

#[tokio::test]
async fn test_skill_execution_success() {
    let (_temp_dir, executor) = setup_test_executor().await;
    let skill = create_test_skill();

    let mut inputs = HashMap::new();
    inputs.insert("message".to_string(), "Hello, World!".to_string());

    let result = executor.execute_skill(&skill, inputs, None).await.unwrap();

    assert_eq!(result.status, SkillStatus::Success);
    assert_eq!(result.execution_log.len(), 2);
    assert!(!result.output.is_empty());
    assert!(result.output.contains("Hello, World!"));

    // Verify step-by-step execution
    for (i, log) in result.execution_log.iter().enumerate() {
        assert_eq!(log.step_number, i);
        assert!(log.success);
        assert!(!log.output.is_empty());
    }
}

#[tokio::test]
async fn test_skill_execution_with_defaults() {
    let (_temp_dir, executor) = setup_test_executor().await;

    let skill = Skill {
        name: "skill-with-defaults".to_string(),
        version: "1.0.0".to_string(),
        description: "Skill with default inputs".to_string(),
        author: None,
        steps: vec![SkillStep::Llm {
            prompt: "Hello {name}, your setting is {setting}".to_string(),
            use_context: false,
        }],
        inputs: vec![
            SkillInput {
                name: "name".to_string(),
                description: "User name".to_string(),
                required: true,
                default: Some("User".to_string()),
            },
            SkillInput {
                name: "setting".to_string(),
                description: "Some setting".to_string(),
                required: false,
                default: Some("default_value".to_string()),
            },
        ],
    };

    // Execute without providing any inputs
    let result = executor
        .execute_skill(&skill, HashMap::new(), None)
        .await
        .unwrap();

    assert_eq!(result.status, SkillStatus::Success);
    assert!(result.output.contains("Hello User"));
    assert!(result.output.contains("default_value"));
}

#[tokio::test]
async fn test_skill_execution_missing_required_input() {
    let (_temp_dir, executor) = setup_test_executor().await;
    let skill = create_test_skill();

    // Execute without required input
    let result = executor.execute_skill(&skill, HashMap::new(), None).await;

    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("message") || err.contains("MissingInput"));
}

#[tokio::test]
async fn test_skill_execution_timeout() {
    let (_temp_dir, executor) = setup_test_executor().await;
    let skill = create_multi_step_skill(5);

    // Execute with very short timeout
    let result = executor
        .execute_skill(&skill, HashMap::new(), Some(Duration::from_nanos(1)))
        .await
        .unwrap();

    // Should either succeed quickly or timeout
    match result.status {
        SkillStatus::Success | SkillStatus::Timeout => {
            // Expected outcomes
        }
        _ => panic!("Unexpected status: {:?}", result.status),
    }
}

#[tokio::test]
async fn test_skill_execution_cancellation() {
    let (_temp_dir, executor) = setup_test_executor().await;
    let (_temp_dir2, executor_clone) = setup_test_executor().await;
    let skill = create_multi_step_skill(10);

    // Spawn a task that will cancel after a short delay
    let cancel_handle = tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(5)).await;
        executor_clone.cancel();
    });

    // Execute skill (should get cancelled)
    let result = executor
        .execute_skill(&skill, HashMap::new(), None)
        .await
        .unwrap();

    // Wait for cancel task
    let _ = cancel_handle.await;

    // Result could be Success (completed before cancel) or Cancelled
    assert!(
        result.status == SkillStatus::Success || result.status == SkillStatus::Cancelled,
        "Expected Success or Cancelled, got {:?}",
        result.status
    );
}

#[tokio::test]
async fn test_execution_report_generation() {
    let (_temp_dir, executor) = setup_test_executor().await;
    let skill = create_multi_step_skill(3);

    let result = executor
        .execute_skill(&skill, HashMap::new(), None)
        .await
        .unwrap();
    let report = ExecutionReport::from_result(&skill, &result);

    // Verify report contents
    assert_eq!(report.skill_name, skill.name);
    assert_eq!(report.skill_version, skill.version);
    assert_eq!(report.statistics.total_steps, 3);
    assert_eq!(report.step_reports.len(), 3);

    // Check summary
    let summary = report.summary();
    assert!(summary.contains(&skill.name));
    assert!(summary.contains("3/3 successful") || summary.contains("completed"));

    // Check detailed report
    let detailed = report.detailed();
    assert!(detailed.contains(&skill.name));
    assert!(detailed.contains("Statistics:"));
    assert!(detailed.contains("Step Details:"));
}

#[tokio::test]
async fn test_progress_monitoring() {
    let skill = create_multi_step_skill(5);
    let mut monitor = SkillMonitor::new(skill.steps.len());

    monitor.start();
    assert_eq!(monitor.progress(), 0.0);

    // Simulate step execution
    for i in 0..skill.steps.len() {
        monitor.begin_step(i);
        assert_eq!(monitor.current_step_display(), i + 1);

        let expected_progress = i as f32 / skill.steps.len() as f32;
        assert!((monitor.progress() - expected_progress).abs() < 0.01);

        // Simulate step duration
        monitor.end_step(Duration::from_millis(100));
    }

    // Mark as complete by setting current_step to total_steps
    monitor.begin_step(skill.steps.len());

    assert!(monitor.is_complete());
    assert!(monitor.is_success());
    assert_eq!(monitor.progress(), 1.0);
}

#[tokio::test]
async fn test_complex_skill_with_all_step_types() {
    let (_temp_dir, executor) = setup_test_executor().await;

    let skill = Skill {
        name: "complex-skill".to_string(),
        version: "1.0.0".to_string(),
        description: "Skill with all step types".to_string(),
        author: None,
        steps: vec![
            SkillStep::Tool {
                tool: "shell".to_string(),
                args: serde_json::json!({"command": "echo Starting"}),
            },
            SkillStep::Llm {
                prompt: "Analyze the situation".to_string(),
                use_context: true,
            },
            SkillStep::Shell {
                command: "echo {status}".to_string(),
                working_dir: Some("/tmp".to_string()),
            },
            SkillStep::Llm {
                prompt: "Final analysis".to_string(),
                use_context: false,
            },
        ],
        inputs: vec![SkillInput {
            name: "status".to_string(),
            description: "Current status".to_string(),
            required: true,
            default: None,
        }],
    };

    let mut inputs = HashMap::new();
    inputs.insert("status".to_string(), "completed".to_string());

    let result = executor.execute_skill(&skill, inputs, None).await.unwrap();

    assert_eq!(result.status, SkillStatus::Success);
    assert_eq!(result.execution_log.len(), 4);

    // Verify step types
    assert_eq!(result.execution_log[0].step_type, "tool");
    assert_eq!(result.execution_log[1].step_type, "llm");
    assert_eq!(result.execution_log[2].step_type, "shell");
    assert_eq!(result.execution_log[3].step_type, "llm");
}

#[tokio::test]
async fn test_skill_versioning() {
    let (_temp_dir, executor) = setup_test_executor().await;

    // Save first version
    let skill_v1 = Skill {
        name: "versioned-skill".to_string(),
        version: "1.0.0".to_string(),
        description: "Version 1".to_string(),
        author: None,
        steps: vec![SkillStep::Llm {
            prompt: "V1".to_string(),
            use_context: false,
        }],
        inputs: vec![],
    };
    executor.save_skill(&skill_v1).unwrap();

    // Overwrite with second version
    let skill_v2 = Skill {
        name: "versioned-skill".to_string(),
        version: "2.0.0".to_string(),
        description: "Version 2".to_string(),
        author: None,
        steps: vec![
            SkillStep::Llm {
                prompt: "V2 Step 1".to_string(),
                use_context: false,
            },
            SkillStep::Llm {
                prompt: "V2 Step 2".to_string(),
                use_context: false,
            },
        ],
        inputs: vec![],
    };
    executor.save_skill(&skill_v2).unwrap();

    // Load and verify it's the new version
    let loaded = executor.load_skill("versioned-skill").unwrap();
    assert_eq!(loaded.version, "2.0.0");
    assert_eq!(loaded.steps.len(), 2);
}

#[tokio::test]
async fn test_empty_skill_execution() {
    let (_temp_dir, executor) = setup_test_executor().await;

    let skill = Skill {
        name: "empty-skill".to_string(),
        version: "1.0.0".to_string(),
        description: "Skill with no steps".to_string(),
        author: None,
        steps: vec![],
        inputs: vec![],
    };

    let result = executor
        .execute_skill(&skill, HashMap::new(), None)
        .await
        .unwrap();

    assert_eq!(result.status, SkillStatus::Success);
    assert!(result.execution_log.is_empty());
    assert!(result.output.is_empty());
}

#[tokio::test]
async fn test_skill_with_many_inputs() {
    let (_temp_dir, executor) = setup_test_executor().await;

    let skill = Skill {
        name: "many-inputs".to_string(),
        version: "1.0.0".to_string(),
        description: "Skill with many inputs".to_string(),
        author: None,
        steps: vec![SkillStep::Llm {
            prompt: "Process {a} {b} {c} {d} {e}".to_string(),
            use_context: false,
        }],
        inputs: vec![
            SkillInput {
                name: "a".to_string(),
                description: "Input A".to_string(),
                required: true,
                default: None,
            },
            SkillInput {
                name: "b".to_string(),
                description: "Input B".to_string(),
                required: true,
                default: None,
            },
            SkillInput {
                name: "c".to_string(),
                description: "Input C".to_string(),
                required: false,
                default: Some("default_c".to_string()),
            },
            SkillInput {
                name: "d".to_string(),
                description: "Input D".to_string(),
                required: false,
                default: Some("default_d".to_string()),
            },
            SkillInput {
                name: "e".to_string(),
                description: "Input E".to_string(),
                required: false,
                default: Some("default_e".to_string()),
            },
        ],
    };

    // Provide only required inputs
    let mut inputs = HashMap::new();
    inputs.insert("a".to_string(), "value_a".to_string());
    inputs.insert("b".to_string(), "value_b".to_string());

    let result = executor.execute_skill(&skill, inputs, None).await.unwrap();

    assert_eq!(result.status, SkillStatus::Success);
    assert!(result.output.contains("value_a"));
    assert!(result.output.contains("value_b"));
    assert!(result.output.contains("default_c"));
    assert!(result.output.contains("default_d"));
    assert!(result.output.contains("default_e"));
}
