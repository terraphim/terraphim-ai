// End-to-end integration tests for Phase 6
//
// Tests the complete workflow across all layers:
// MCP Tool → Validation → Edit → Recovery

use tempfile::TempDir;
use terraphim_automata::{apply_edit, EditStrategy};
use terraphim_mcp_server::recovery::SnapshotManager;
use terraphim_mcp_server::security::{CommandPermission, RepositorySecurityGraph, SecurityConfig};
use terraphim_mcp_server::validation::{ValidationContext, ValidationPipeline};

/// Test complete edit workflow with validation
#[tokio::test]
async fn test_complete_edit_workflow_with_validation() {
    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join("test.rs");

    // Create test file
    tokio::fs::write(
        &test_file,
        r#"fn main() {
    println!("Hello");
}
"#,
    )
    .await
    .unwrap();

    // Step 1: PRE-TOOL VALIDATION
    let pipeline = ValidationPipeline::new();
    let mut context = ValidationContext::new("edit_file_search_replace".to_string());
    context
        .file_paths
        .push(test_file.to_str().unwrap().to_string());

    let pre_validation = pipeline.validate_pre_tool(&context).await.unwrap();
    assert!(pre_validation.passed, "Pre-tool validation should pass");

    // Step 2: EXECUTE EDIT
    let content = tokio::fs::read_to_string(&test_file).await.unwrap();
    let search = r#"println!("Hello");"#;
    let replace = r#"println!("Hello, World!");"#;

    let edit_result = apply_edit(&content, search, replace).unwrap();
    assert!(edit_result.success, "Edit should succeed");

    // Step 3: WRITE FILE
    tokio::fs::write(&test_file, edit_result.modified_content.as_bytes())
        .await
        .unwrap();

    // Step 4: POST-TOOL VALIDATION
    let post_validation = pipeline
        .validate_post_tool(&context, &rmcp::model::CallToolResult::success(vec![]))
        .await
        .unwrap();
    assert!(post_validation.passed, "Post-tool validation should pass");

    // Step 5: VERIFY RESULT
    let final_content = tokio::fs::read_to_string(&test_file).await.unwrap();
    assert!(
        final_content.contains("Hello, World!"),
        "File should be modified"
    );
}

/// Test security validation across layers
#[tokio::test]
async fn test_security_validation_workflow() {
    // Step 1: Create security config
    let temp_dir = TempDir::new().unwrap();
    let config = SecurityConfig::default_for_repo(temp_dir.path());

    // Verify default config
    assert!(config.allowed_commands.contains_key("git"));
    assert!(config.blocked_commands.contains_key("sudo"));

    // Step 2: Build security graph
    let graph = RepositorySecurityGraph::new(config).await.unwrap();

    // Step 3: Test command validation
    let allowed = graph.validate_command("git status").await.unwrap();
    assert_eq!(allowed, CommandPermission::Allow);

    let blocked = graph.validate_command("sudo rm -rf /").await.unwrap();
    assert_eq!(blocked, CommandPermission::Block);

    // Step 4: Test synonym resolution
    let synonym = graph.validate_command("show file").await.unwrap();
    assert_eq!(synonym, CommandPermission::Allow); // Resolves to "cat"
}

/// Test recovery system workflow
#[tokio::test]
async fn test_recovery_system_workflow() {
    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join("test.txt");

    // Initialize git repo (required for GitRecovery)
    std::process::Command::new("git")
        .current_dir(temp_dir.path())
        .args(["init"])
        .output()
        .unwrap();

    std::process::Command::new("git")
        .current_dir(temp_dir.path())
        .args(["config", "user.email", "test@terraphim.ai"])
        .output()
        .unwrap();

    std::process::Command::new("git")
        .current_dir(temp_dir.path())
        .args(["config", "user.name", "Test User"])
        .output()
        .unwrap();

    // Create initial file
    tokio::fs::write(&test_file, "original content")
        .await
        .unwrap();

    // Initial commit
    std::process::Command::new("git")
        .current_dir(temp_dir.path())
        .args(["add", "."])
        .output()
        .unwrap();

    std::process::Command::new("git")
        .current_dir(temp_dir.path())
        .args(["commit", "-m", "Initial commit"])
        .output()
        .unwrap();

    // Step 1: Create snapshot before edit
    let snapshot_dir = temp_dir.path().join(".terraphim/snapshots");
    let mut snapshot_mgr = SnapshotManager::new(snapshot_dir);

    let snapshot_id = snapshot_mgr
        .create_snapshot(
            "Before edit".to_string(),
            vec![test_file.to_str().unwrap().to_string()],
        )
        .await
        .unwrap();

    // Step 2: Modify file
    tokio::fs::write(&test_file, "modified content")
        .await
        .unwrap();

    // Step 3: Verify modification
    assert_eq!(
        tokio::fs::read_to_string(&test_file).await.unwrap(),
        "modified content"
    );

    // Step 4: Restore from snapshot
    snapshot_mgr.restore_snapshot(&snapshot_id).await.unwrap();

    // Step 5: Verify restored
    assert_eq!(
        tokio::fs::read_to_string(&test_file).await.unwrap(),
        "original content"
    );
}

/// Test multi-strategy edit with all fallbacks
#[tokio::test]
async fn test_multi_strategy_edit_integration() {
    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join("test.rs");

    let code = r#"fn calculate(a: i32, b: i32) -> i32 {
    let result = a + b;
    return result;
}
"#;

    tokio::fs::write(&test_file, code).await.unwrap();

    // Test 1: Exact match strategy
    let content = tokio::fs::read_to_string(&test_file).await.unwrap();
    let result = apply_edit(&content, "let result = a + b;", "let result = a * b;");
    assert!(result.is_ok());
    let edit = result.unwrap();
    assert!(
        edit.success
            || edit.strategy_used == "exact"
            || edit.strategy_used == "whitespace-flexible"
    );

    // Test 2: Whitespace-flexible strategy
    let result2 = apply_edit(
        &content,
        "    let result = a + b;",
        "    let result = a - b;",
    );
    assert!(result2.unwrap().success);

    // Test 3: Fuzzy strategy for typo handling
    use terraphim_automata::apply_edit_with_strategy;
    let search_with_typo = r#"fn calcuate(a: i32, b: i32) -> i32 {
    let result = a + b;
    return result;
}"#;

    let result3 = apply_edit_with_strategy(
        &content,
        search_with_typo,
        "fn calculate(a: i32, b: i32) -> i32 { a + b }",
        EditStrategy::Fuzzy,
    );
    assert!(result3.is_ok());
    let fuzzy_edit = result3.unwrap();
    assert!(
        fuzzy_edit.similarity_score >= 0.8,
        "Fuzzy match should have high similarity"
    );
}

/// Test validation pipeline prevents invalid operations
#[tokio::test]
async fn test_validation_prevents_invalid_operations() {
    let pipeline = ValidationPipeline::new();

    // Test with non-existent file
    let mut context = ValidationContext::new("edit_file_search_replace".to_string());
    context.file_paths.push("/nonexistent/file.txt".to_string());

    let result = pipeline.validate_pre_tool(&context).await.unwrap();
    assert!(
        !result.passed,
        "Validation should fail for non-existent file"
    );
    assert!(result.message.contains("does not exist"));
}

/// Test complete MCP tool execution with validation
#[tokio::test]
async fn test_mcp_tool_with_validation_integration() {
    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join("integration.rs");

    tokio::fs::write(
        &test_file,
        r#"fn test() {
    let x = 1;
}
"#,
    )
    .await
    .unwrap();

    // Simulate MCP call_tool request
    let mut context = ValidationContext::new("edit_file_search_replace".to_string());
    context
        .file_paths
        .push(test_file.to_str().unwrap().to_string());

    // Pre-validation
    let pipeline = ValidationPipeline::new();
    let pre_result = pipeline.validate_pre_tool(&context).await.unwrap();
    assert!(pre_result.passed);

    // Execute edit
    let content = tokio::fs::read_to_string(&test_file).await.unwrap();
    let edit_result = apply_edit(&content, "let x = 1;", "let x = 10;").unwrap();
    assert!(edit_result.success);

    tokio::fs::write(&test_file, edit_result.modified_content.as_bytes())
        .await
        .unwrap();

    // Post-validation
    let post_result = pipeline
        .validate_post_tool(&context, &rmcp::model::CallToolResult::success(vec![]))
        .await
        .unwrap();
    assert!(post_result.passed);

    // Verify final state
    let final_content = tokio::fs::read_to_string(&test_file).await.unwrap();
    assert!(final_content.contains("let x = 10"));
}

/// Test backward compatibility - existing features still work
#[tokio::test]
async fn test_backward_compatibility() {
    // This test ensures that adding new features didn't break existing functionality

    // Test 1: Existing validation still works
    let pipeline = ValidationPipeline::new();
    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join("compat.txt");
    tokio::fs::write(&test_file, "content").await.unwrap();

    let mut context = ValidationContext::new("test_tool".to_string());
    context
        .file_paths
        .push(test_file.to_str().unwrap().to_string());

    let result = pipeline.validate_pre_tool(&context).await.unwrap();
    assert!(result.passed);

    // Test 2: Existing security config works
    let config = SecurityConfig::default_for_repo(temp_dir.path());
    assert!(config.allowed_commands.contains_key("cat"));
    assert!(config.blocked_commands.contains_key("rm"));
}
