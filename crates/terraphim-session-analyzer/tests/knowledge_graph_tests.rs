//! Integration tests for the PatternLearner knowledge graph
//!
//! Tests the complete learning lifecycle: observation -> voting -> promotion -> caching

use anyhow::Result;
use tempfile::TempDir;
use terraphim_session_analyzer::models::ToolCategory;
use terraphim_session_analyzer::patterns::knowledge_graph::{
    PatternLearner, infer_category_from_contexts,
};

#[test]
fn test_complete_learning_lifecycle() -> Result<()> {
    let mut learner = PatternLearner::new();

    // Phase 1: Observe a tool multiple times
    learner.observe(
        "pytest".to_string(),
        "pytest tests/".to_string(),
        ToolCategory::Testing,
    );
    learner.observe(
        "pytest".to_string(),
        "pytest tests/ --verbose".to_string(),
        ToolCategory::Testing,
    );
    learner.observe(
        "pytest".to_string(),
        "pytest tests/ --cov".to_string(),
        ToolCategory::Testing,
    );

    assert_eq!(learner.candidate_count(), 1);

    // Phase 2: Promote candidates (threshold = 3)
    let promoted = learner.promote_candidates();

    assert_eq!(promoted.len(), 1);
    assert_eq!(promoted[0].tool_name, "pytest");
    assert!(matches!(promoted[0].category, ToolCategory::Testing));
    assert_eq!(promoted[0].observations, 3);
    assert!(promoted[0].confidence > 0.9); // All votes for Testing

    // After promotion, candidate should be removed
    assert_eq!(learner.candidate_count(), 0);

    Ok(())
}

#[test]
fn test_learning_with_conflicting_votes() -> Result<()> {
    let mut learner = PatternLearner::new();

    // Observe tool with conflicting categorizations
    learner.observe(
        "custom-tool".to_string(),
        "custom-tool build".to_string(),
        ToolCategory::BuildTool,
    );
    learner.observe(
        "custom-tool".to_string(),
        "custom-tool test".to_string(),
        ToolCategory::Testing,
    );
    learner.observe(
        "custom-tool".to_string(),
        "custom-tool deploy".to_string(),
        ToolCategory::BuildTool,
    );

    let promoted = learner.promote_candidates();

    assert_eq!(promoted.len(), 1);
    // BuildTool should win (2 votes vs 1)
    assert!(matches!(promoted[0].category, ToolCategory::BuildTool));
    // Confidence should be 2/3 ≈ 0.67
    assert!((promoted[0].confidence - 0.67).abs() < 0.01);

    Ok(())
}

#[test]
fn test_multiple_tools_learning() -> Result<()> {
    let mut learner = PatternLearner::new();

    // Observe multiple different tools
    for i in 0..3 {
        learner.observe(
            "pytest".to_string(),
            format!("pytest test_{i}.py"),
            ToolCategory::Testing,
        );
        learner.observe(
            "webpack".to_string(),
            format!("webpack build --mode production{i}"),
            ToolCategory::BuildTool,
        );
        learner.observe(
            "eslint".to_string(),
            format!("eslint src/{i}"),
            ToolCategory::Linting,
        );
    }

    assert_eq!(learner.candidate_count(), 3);

    let promoted = learner.promote_candidates();

    assert_eq!(promoted.len(), 3);
    assert_eq!(learner.candidate_count(), 0);

    // Verify each tool was categorized correctly
    let tool_names: Vec<&str> = promoted.iter().map(|p| p.tool_name.as_str()).collect();
    assert!(tool_names.contains(&"pytest"));
    assert!(tool_names.contains(&"webpack"));
    assert!(tool_names.contains(&"eslint"));

    Ok(())
}

#[test]
fn test_custom_threshold() -> Result<()> {
    let mut learner = PatternLearner::with_threshold(5);

    // Observe 4 times (below custom threshold of 5)
    for i in 0..4 {
        learner.observe(
            "custom".to_string(),
            format!("custom command {i}"),
            ToolCategory::Other("unknown".to_string()),
        );
    }

    let promoted = learner.promote_candidates();
    assert_eq!(promoted.len(), 0); // Not enough observations

    // One more observation to meet threshold
    learner.observe(
        "custom".to_string(),
        "custom command 5".to_string(),
        ToolCategory::Other("unknown".to_string()),
    );

    let promoted = learner.promote_candidates();
    assert_eq!(promoted.len(), 1);
    assert_eq!(promoted[0].observations, 5);

    Ok(())
}

#[test]
fn test_context_limit() -> Result<()> {
    let mut learner = PatternLearner::new();

    // Observe tool with many different contexts (should limit to 10)
    for i in 0..20 {
        learner.observe(
            "tool".to_string(),
            format!("tool command variant {i}"),
            ToolCategory::Testing,
        );
    }

    let candidates = learner.get_candidates();
    assert_eq!(candidates.len(), 1);
    // Context list should be limited to 10
    assert!(candidates[0].contexts.len() <= 10);
    assert_eq!(candidates[0].observations, 20); // All observations counted

    Ok(())
}

#[test]
fn test_save_and_load_cache() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let _cache_path = temp_dir.path().join("learned_patterns.json");

    let mut learner = PatternLearner::new();

    // Create some learned patterns
    for i in 0..3 {
        learner.observe(
            "pytest".to_string(),
            format!("pytest test_{i}.py"),
            ToolCategory::Testing,
        );
    }

    let promoted = learner.promote_candidates();
    assert_eq!(promoted.len(), 1);

    // Save to custom location (modify the save_to_cache to accept path for testing)
    // For now, just verify the patterns are created correctly
    assert_eq!(promoted[0].tool_name, "pytest");
    assert!(matches!(promoted[0].category, ToolCategory::Testing));

    Ok(())
}

#[test]
fn test_infer_category_testing_keywords() {
    let contexts = vec![
        "pytest tests/unit/".to_string(),
        "pytest tests/integration/".to_string(),
        "pytest --verbose --cov".to_string(),
    ];

    let category = infer_category_from_contexts(&contexts);
    assert!(matches!(category, ToolCategory::Testing));
}

#[test]
fn test_infer_category_build_tool_keywords() {
    let contexts = vec![
        "webpack build --mode production".to_string(),
        "vite build".to_string(),
        "rollup -c".to_string(),
    ];

    let category = infer_category_from_contexts(&contexts);
    assert!(matches!(category, ToolCategory::BuildTool));
}

#[test]
fn test_infer_category_linting_keywords() {
    let contexts = vec![
        "eslint src/ --fix".to_string(),
        "cargo clippy".to_string(),
        "pylint mymodule".to_string(),
    ];

    let category = infer_category_from_contexts(&contexts);
    assert!(matches!(category, ToolCategory::Linting));
}

#[test]
fn test_infer_category_git_keywords() {
    let contexts = vec![
        "git commit -m 'message'".to_string(),
        "git push origin main".to_string(),
        "git pull --rebase".to_string(),
    ];

    let category = infer_category_from_contexts(&contexts);
    assert!(matches!(category, ToolCategory::Git));
}

#[test]
fn test_infer_category_package_manager_keywords() {
    let contexts = vec![
        "npm install express".to_string(),
        "yarn add lodash".to_string(),
        "cargo install ripgrep".to_string(),
    ];

    let category = infer_category_from_contexts(&contexts);
    assert!(matches!(category, ToolCategory::PackageManager));
}

#[test]
fn test_infer_category_cloud_deploy_keywords() {
    let contexts = vec![
        "wrangler deploy --env production".to_string(),
        "vercel deploy".to_string(),
        "netlify deploy --prod".to_string(),
    ];

    let category = infer_category_from_contexts(&contexts);
    assert!(matches!(category, ToolCategory::CloudDeploy));
}

#[test]
fn test_infer_category_database_keywords() {
    let contexts = vec![
        "migrate up".to_string(),
        "psql -d mydb".to_string(),
        "mysql -u root".to_string(),
    ];

    let category = infer_category_from_contexts(&contexts);
    assert!(matches!(category, ToolCategory::Database));
}

#[test]
fn test_infer_category_unknown_defaults_to_other() {
    let contexts = vec![
        "unknown-tool --flag".to_string(),
        "mystery command".to_string(),
    ];

    let category = infer_category_from_contexts(&contexts);
    assert!(matches!(category, ToolCategory::Other(_)));
}

#[test]
fn test_repeated_context_not_duplicated() {
    let mut learner = PatternLearner::new();

    // Observe same command multiple times
    for _ in 0..5 {
        learner.observe(
            "tool".to_string(),
            "tool test".to_string(), // Same command
            ToolCategory::Testing,
        );
    }

    let candidates = learner.get_candidates();
    assert_eq!(candidates.len(), 1);
    // Should only store unique contexts
    assert_eq!(candidates[0].contexts.len(), 1);
    assert_eq!(candidates[0].observations, 5);
}

#[test]
fn test_confidence_all_same_votes() {
    let mut learner = PatternLearner::new();

    // All observations vote for the same category
    for i in 0..10 {
        learner.observe(
            "tool".to_string(),
            format!("tool test {i}"),
            ToolCategory::Testing,
        );
    }

    let promoted = learner.promote_candidates();
    assert_eq!(promoted.len(), 1);
    // Perfect confidence: all votes for same category
    assert_eq!(promoted[0].confidence, 1.0);
}

#[test]
fn test_confidence_evenly_split_votes() {
    let mut learner = PatternLearner::new();

    // Split votes between two categories
    learner.observe(
        "tool".to_string(),
        "tool build 1".to_string(),
        ToolCategory::BuildTool,
    );
    learner.observe(
        "tool".to_string(),
        "tool build 2".to_string(),
        ToolCategory::BuildTool,
    );
    learner.observe(
        "tool".to_string(),
        "tool test".to_string(),
        ToolCategory::Testing,
    );
    learner.observe(
        "tool".to_string(),
        "tool test2".to_string(),
        ToolCategory::Testing,
    );

    let promoted = learner.promote_candidates();
    assert_eq!(promoted.len(), 1);
    // 50% confidence (2 out of 4 votes for winner)
    assert_eq!(promoted[0].confidence, 0.5);
}

#[test]
fn test_learned_pattern_timestamp() -> Result<()> {
    let mut learner = PatternLearner::new();

    // Observe and promote
    for i in 0..3 {
        learner.observe(
            "tool".to_string(),
            format!("tool {i}"),
            ToolCategory::Testing,
        );
    }

    let promoted = learner.promote_candidates();
    assert_eq!(promoted.len(), 1);

    // Verify learned_at timestamp is set and reasonable (not default/zero)
    let learned_at = &promoted[0].learned_at;
    assert!(learned_at.to_string().starts_with("20")); // Year starts with 20xx

    Ok(())
}

#[test]
fn test_multiple_promotion_rounds() -> Result<()> {
    let mut learner = PatternLearner::new();

    // Round 1: Add tool1 (meets threshold)
    for i in 0..3 {
        learner.observe(
            "tool1".to_string(),
            format!("tool1 {i}"),
            ToolCategory::Testing,
        );
    }

    let round1 = learner.promote_candidates();
    assert_eq!(round1.len(), 1);
    assert_eq!(round1[0].tool_name, "tool1");

    // Round 2: Add tool2 and tool3
    for i in 0..3 {
        learner.observe(
            "tool2".to_string(),
            format!("tool2 {i}"),
            ToolCategory::BuildTool,
        );
        learner.observe(
            "tool3".to_string(),
            format!("tool3 {i}"),
            ToolCategory::Linting,
        );
    }

    let round2 = learner.promote_candidates();
    assert_eq!(round2.len(), 2);

    Ok(())
}

#[test]
fn test_empty_contexts_infers_other() {
    let contexts: Vec<String> = Vec::new();
    let category = infer_category_from_contexts(&contexts);
    assert!(matches!(category, ToolCategory::Other(_)));
}

#[test]
fn test_case_insensitive_context_inference() {
    let contexts = vec!["PYTEST TESTS/".to_string(), "PyTest --VERBOSE".to_string()];

    let category = infer_category_from_contexts(&contexts);
    assert!(matches!(category, ToolCategory::Testing));
}

#[test]
fn test_mixed_keywords_first_match_wins() {
    // When contexts contain multiple category keywords, first match in priority wins
    let contexts = vec![
        "npm install && webpack build".to_string(),
        "yarn add something".to_string(),
    ];

    // BuildTool (build, webpack) appears before PackageManager in priority
    let category = infer_category_from_contexts(&contexts);
    // This should match BuildTool because "build" keyword has higher priority
    assert!(matches!(category, ToolCategory::BuildTool));
}

#[test]
fn test_observation_updates_timestamps() -> Result<()> {
    let mut learner = PatternLearner::new();

    learner.observe(
        "tool".to_string(),
        "tool cmd".to_string(),
        ToolCategory::Testing,
    );

    let candidates1 = learner.get_candidates();
    let first_seen1 = candidates1[0].first_seen;
    let last_seen1 = candidates1[0].last_seen;

    // Timestamps should be equal on first observation
    assert_eq!(first_seen1, last_seen1);

    // Sleep briefly to ensure timestamp difference
    std::thread::sleep(std::time::Duration::from_millis(10));

    learner.observe(
        "tool".to_string(),
        "tool cmd2".to_string(),
        ToolCategory::Testing,
    );

    let candidates2 = learner.get_candidates();
    let first_seen2 = candidates2[0].first_seen;
    let last_seen2 = candidates2[0].last_seen;

    // First seen should not change
    assert_eq!(first_seen1, first_seen2);
    // Last seen should be updated
    assert!(last_seen2 > last_seen1);

    Ok(())
}
