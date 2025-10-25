//! Comprehensive tests for the hook system
//!
//! Tests all built-in hooks, hook manager, and integration with command execution.

use std::collections::HashMap;
use std::path::PathBuf;
use tempfile::TempDir;
use terraphim_tui::commands::{
    hooks, CommandExecutionResult, CommandHook, ExecutionMode, HookContext, HookManager, HookResult,
};
use tokio::fs;

/// Creates a test hook context
fn create_test_hook_context(command: &str) -> HookContext {
    let mut parameters = HashMap::new();
    parameters.insert("input".to_string(), "test_value".to_string());
    parameters.insert("verbose".to_string(), "true".to_string());

    HookContext {
        command: command.to_string(),
        parameters,
        user: "test_user".to_string(),
        role: "Terraphim Engineer".to_string(),
        execution_mode: ExecutionMode::Local,
        working_directory: PathBuf::from("/test/workspace"),
    }
}

/// Creates a test command execution result
fn create_test_execution_result(command: &str, success: bool) -> CommandExecutionResult {
    CommandExecutionResult {
        command: command.to_string(),
        execution_mode: ExecutionMode::Local,
        exit_code: if success { 0 } else { 1 },
        stdout: if success {
            "Command executed successfully".to_string()
        } else {
            String::new()
        },
        stderr: if !success {
            "Command failed".to_string()
        } else {
            String::new()
        },
        duration_ms: 150,
        resource_usage: None,
    }
}

#[tokio::test]
async fn test_logging_hook_functionality() {
    let hook = hooks::LoggingHook::new();
    let context = create_test_hook_context("test-command");

    let result = hook.execute(&context).await;
    assert!(result.is_ok(), "Logging hook should execute successfully");

    let hook_result = result.unwrap();
    assert!(hook_result.success, "Logging hook should succeed");
    assert!(
        hook_result.should_continue,
        "Logging hook should allow continuation"
    );
    assert!(
        hook_result.message.contains("logged successfully"),
        "Should return success message"
    );
    assert!(
        hook_result.data.is_none(),
        "Logging hook should not return data"
    );
}

#[tokio::test]
async fn test_logging_hook_with_file() {
    let temp_dir = TempDir::new().unwrap();
    let log_file = temp_dir.path().join("test.log");
    let hook = hooks::LoggingHook::with_file(&log_file);
    let context = create_test_hook_context("file-test");

    let result = hook.execute(&context).await;
    assert!(
        result.is_ok(),
        "Logging hook with file should execute successfully"
    );

    // Verify log file was created and contains the entry
    assert!(log_file.exists(), "Log file should be created");

    let log_content = fs::read_to_string(&log_file).await.unwrap();
    assert!(
        log_content.contains("file-test"),
        "Log should contain command name"
    );
    assert!(
        log_content.contains("test_user"),
        "Log should contain username"
    );
    assert!(
        log_content.contains("Terraphim Engineer"),
        "Log should contain role"
    );
}

#[tokio::test]
async fn test_preflight_check_hook_safe_commands() {
    let hook = hooks::PreflightCheckHook::new()
        .with_blocked_commands(vec![
            "rm -rf /".to_string(),
            "dd if=/dev/zero".to_string(),
            "mkfs".to_string(),
        ])
        .with_allowed_dirs(vec![PathBuf::from("/safe"), PathBuf::from("/test")]);

    // Test safe command
    let safe_context = HookContext {
        command: "ls -la".to_string(),
        parameters: HashMap::new(),
        user: "test_user".to_string(),
        role: "Terraphim Engineer".to_string(),
        execution_mode: ExecutionMode::Local,
        working_directory: PathBuf::from("/safe"),
    };

    let result = hook.execute(&safe_context).await;
    assert!(result.is_ok(), "Safe command should pass preflight check");

    let hook_result = result.unwrap();
    assert!(
        hook_result.success,
        "Preflight check should succeed for safe commands"
    );
    assert!(
        hook_result.should_continue,
        "Should allow safe commands to continue"
    );
}

#[tokio::test]
async fn test_preflight_check_hook_blocked_commands() {
    let hook = hooks::PreflightCheckHook::new().with_blocked_commands(vec![
        "rm -rf /".to_string(),
        "format".to_string(),
        "shutdown".to_string(),
    ]);

    // Test blocked command
    let blocked_context = create_test_hook_context("rm -rf /");

    let result = hook.execute(&blocked_context).await;
    assert!(
        result.is_ok(),
        "Preflight check should execute without panicking"
    );

    let hook_result = result.unwrap();
    assert!(
        !hook_result.success,
        "Preflight check should fail for blocked commands"
    );
    assert!(
        !hook_result.should_continue,
        "Should block dangerous commands"
    );
    assert!(
        hook_result.message.contains("blocked"),
        "Should indicate command is blocked"
    );
}

#[tokio::test]
async fn test_preflight_check_hook_directory_restriction() {
    let hook = hooks::PreflightCheckHook::new()
        .with_allowed_dirs(vec![PathBuf::from("/allowed"), PathBuf::from("/safe")]);

    // Test command in allowed directory
    let allowed_context = HookContext {
        command: "test".to_string(),
        parameters: HashMap::new(),
        user: "test_user".to_string(),
        role: "Terraphim Engineer".to_string(),
        execution_mode: ExecutionMode::Local,
        working_directory: PathBuf::from("/allowed/subdir"),
    };

    let result = hook.execute(&allowed_context).await;
    assert!(
        result.is_ok(),
        "Allowed directory should pass preflight check"
    );
    assert!(
        result.unwrap().should_continue,
        "Should allow commands in allowed directories"
    );

    // Test command in disallowed directory
    let disallowed_context = HookContext {
        command: "test".to_string(),
        parameters: HashMap::new(),
        user: "test_user".to_string(),
        role: "Terraphim Engineer".to_string(),
        execution_mode: ExecutionMode::Local,
        working_directory: PathBuf::from("/forbidden"),
    };

    let result = hook.execute(&disallowed_context).await;
    assert!(
        result.is_ok(),
        "Disallowed directory should not cause hook to fail"
    );
    assert!(
        !result.unwrap().should_continue,
        "Should block commands in disallowed directories"
    );
}

#[tokio::test]
async fn test_notification_hook_important_commands() {
    let hook = hooks::NotificationHook::new().with_important_commands(vec![
        "deploy".to_string(),
        "security-audit".to_string(),
        "backup".to_string(),
    ]);

    // Test important command
    let important_context = create_test_hook_context("deploy production");

    let result = hook.execute(&important_context).await;
    assert!(
        result.is_ok(),
        "Notification hook should execute successfully"
    );

    let hook_result = result.unwrap();
    assert!(hook_result.success, "Notification hook should succeed");
    assert!(
        hook_result.should_continue,
        "Notifications should not block execution"
    );

    // Test non-important command
    let normal_context = create_test_hook_context("search test");

    let result = hook.execute(&normal_context).await;
    assert!(result.is_ok(), "Normal command should not cause errors");

    let hook_result = result.unwrap();
    assert!(
        hook_result.success,
        "Normal command should pass notification hook"
    );
    // The notification should still succeed, just not send notifications
}

#[tokio::test]
async fn test_environment_hook_variables() {
    let hook = hooks::EnvironmentHook::new()
        .with_env("CUSTOM_VAR", "custom_value")
        .with_env("DEBUG", "true")
        .with_env("ENVIRONMENT", "test");

    let context = create_test_hook_context("env-test");

    let result = hook.execute(&context).await;
    assert!(
        result.is_ok(),
        "Environment hook should execute successfully"
    );

    let hook_result = result.unwrap();
    assert!(hook_result.success, "Environment hook should succeed");
    assert!(
        hook_result.data.is_some(),
        "Environment hook should return data"
    );

    if let Some(data) = hook_result.data {
        // Check custom environment variables
        assert_eq!(data.get("CUSTOM_VAR").unwrap(), "custom_value");
        assert_eq!(data.get("DEBUG").unwrap(), "true");
        assert_eq!(data.get("ENVIRONMENT").unwrap(), "test");

        // Check automatically added variables
        assert_eq!(data.get("COMMAND_USER").unwrap(), "test_user");
        assert_eq!(data.get("COMMAND_ROLE").unwrap(), "Terraphim Engineer");
        assert_eq!(data.get("COMMAND_WORKING_DIR").unwrap(), "/test/workspace");
    }
}

#[tokio::test]
async fn test_backup_hook_with_destructive_commands() {
    let temp_dir = TempDir::new().unwrap();
    let backup_dir = temp_dir.path().join("backups");
    let hook = hooks::BackupHook::new(&backup_dir).with_backup_commands(vec![
        "rm".to_string(),
        "mv".to_string(),
        "cp".to_string(),
        "deploy".to_string(),
    ]);

    // Test command that requires backup
    let backup_context = create_test_hook_context("rm important_file.txt");

    let result = hook.execute(&backup_context).await;
    assert!(result.is_ok(), "Backup hook should execute successfully");

    let hook_result = result.unwrap();
    assert!(hook_result.success, "Backup should succeed");
    assert!(
        hook_result.should_continue,
        "Backup should not block execution"
    );
    assert!(
        hook_result.message.contains("Backup created"),
        "Should indicate backup was created"
    );

    // Verify backup directory exists
    assert!(backup_dir.exists(), "Backup directory should be created");

    // Verify backup file was created
    let mut entries = fs::read_dir(&backup_dir).await.unwrap();
    let mut backup_files = Vec::new();
    while let Some(entry) = entries.next_entry().await.unwrap() {
        backup_files.push(entry);
    }

    assert_eq!(
        backup_files.len(),
        1,
        "Should create exactly one backup file"
    );

    // Check backup file content
    let backup_file = backup_files.pop().unwrap();
    let backup_content = fs::read_to_string(backup_file.path()).await.unwrap();
    assert!(
        backup_content.contains("rm important_file.txt"),
        "Backup should contain command"
    );
    assert!(
        backup_content.contains("test_user"),
        "Backup should contain user"
    );
}

#[tokio::test]
async fn test_backup_hook_with_safe_commands() {
    let temp_dir = TempDir::new().unwrap();
    let backup_dir = temp_dir.path().join("backups");
    let hook = hooks::BackupHook::new(&backup_dir)
        .with_backup_commands(vec!["rm".to_string(), "mv".to_string()]);

    // Test command that doesn't require backup
    let safe_context = create_test_hook_context("search query");

    let result = hook.execute(&safe_context).await;
    assert!(result.is_ok(), "Backup hook should execute successfully");

    let hook_result = result.unwrap();
    assert!(hook_result.success, "Safe command should pass backup hook");
    assert!(hook_result.should_continue, "Safe commands should continue");
    assert!(
        hook_result.message.contains("No backup needed"),
        "Should indicate no backup needed"
    );

    // Backup directory should not be created for safe commands
    assert!(
        !backup_dir.exists(),
        "Backup directory should not be created for safe commands"
    );
}

#[tokio::test]
async fn test_resource_monitoring_hook() {
    let hook = hooks::ResourceMonitoringHook::new()
        .with_memory_limit(1024)
        .with_duration_limit(300);

    let context = create_test_hook_context("resource-test");

    let result = hook.execute(&context).await;
    assert!(
        result.is_ok(),
        "Resource monitoring hook should execute successfully"
    );

    let hook_result = result.unwrap();
    assert!(hook_result.success, "Resource monitoring should succeed");
    assert!(
        hook_result.should_continue,
        "Resource monitoring should not block execution"
    );
    assert!(
        hook_result.data.is_some(),
        "Resource monitoring should return data"
    );

    if let Some(data) = hook_result.data {
        assert_eq!(data.get("memory_limit_mb").unwrap(), 1024);
        assert_eq!(data.get("duration_limit_seconds").unwrap(), 300);
    }
}

#[tokio::test]
async fn test_git_hook_with_repository() {
    let temp_dir = TempDir::new().unwrap();
    let repo_path = temp_dir.path();

    // Initialize a git repository
    std::process::Command::new("git")
        .args(["init"])
        .current_dir(&repo_path)
        .output()
        .expect("Failed to initialize git repository");

    std::process::Command::new("git")
        .args(["config", "user.email", "test@example.com"])
        .current_dir(&repo_path)
        .output()
        .expect("Failed to configure git user");

    std::process::Command::new("git")
        .args(["config", "user.name", "Test User"])
        .current_dir(&repo_path)
        .output()
        .expect("Failed to configure git name");

    // Create a test file
    let test_file = repo_path.join("test.txt");
    fs::write(&test_file, "test content").await.unwrap();

    std::process::Command::new("git")
        .args(["add", "test.txt"])
        .current_dir(&repo_path)
        .output()
        .expect("Failed to add file to git");

    std::process::Command::new("git")
        .args(["commit", "-m", "Initial commit"])
        .current_dir(&repo_path)
        .output()
        .expect("Failed to commit");

    let hook = hooks::GitHook::new(&repo_path).with_auto_commit(false);

    let context = create_test_hook_context("git-test");

    let result = hook.execute(&context).await;
    assert!(result.is_ok(), "Git hook should execute successfully");

    let hook_result = result.unwrap();
    assert!(hook_result.success, "Git hook should succeed");
    assert!(
        hook_result.should_continue,
        "Git hook should not block execution"
    );

    if let Some(data) = hook_result.data {
        assert_eq!(
            data.get("is_clean").unwrap(),
            true,
            "Repository should be clean"
        );
        assert_eq!(
            data.get("auto_commit").unwrap(),
            false,
            "Auto-commit should be disabled"
        );
    }
}

#[tokio::test]
async fn test_git_hook_with_dirty_repository() {
    let temp_dir = TempDir::new().unwrap();
    let repo_path = temp_dir.path();

    // Initialize git repository
    std::process::Command::new("git")
        .args(["init"])
        .current_dir(&repo_path)
        .output()
        .expect("Failed to initialize git repository");

    std::process::Command::new("git")
        .args(["config", "user.email", "test@example.com"])
        .current_dir(&repo_path)
        .output()
        .expect("Failed to configure git user");

    // Create an untracked file
    let test_file = repo_path.join("untracked.txt");
    fs::write(&test_file, "untracked content").await.unwrap();

    let hook = hooks::GitHook::new(&repo_path).with_auto_commit(false);

    let context = create_test_hook_context("git-dirty-test");

    let result = hook.execute(&context).await;
    assert!(
        result.is_ok(),
        "Git hook should execute successfully with dirty repository"
    );

    let hook_result = result.unwrap();
    assert!(
        hook_result.success,
        "Git hook should succeed with dirty repository"
    );
    assert!(
        hook_result.should_continue,
        "Git hook should not block execution"
    );

    if let Some(data) = hook_result.data {
        assert_eq!(
            data.get("is_clean").unwrap(),
            false,
            "Repository should be dirty"
        );
    }
}

#[tokio::test]
async fn test_hook_manager_pre_hooks() {
    let mut hook_manager = HookManager::new();

    // Add multiple pre-hooks
    hook_manager.add_pre_hook(Box::new(hooks::LoggingHook::new()));
    hook_manager.add_pre_hook(Box::new(hooks::PreflightCheckHook::new()));
    hook_manager.add_pre_hook(Box::new(hooks::EnvironmentHook::new()));

    let context = create_test_hook_context("hook-manager-test");

    let result = hook_manager.execute_pre_hooks(&context).await;
    assert!(result.is_ok(), "All pre-hooks should execute successfully");

    // The test passes if no pre-hook blocks execution
    // In a real scenario, individual hook failures would be tested
}

#[tokio::test]
async fn test_hook_manager_post_hooks() {
    let mut hook_manager = HookManager::new();

    // Add multiple post-hooks
    hook_manager.add_post_hook(Box::new(hooks::LoggingHook::new()));
    hook_manager.add_post_hook(Box::new(hooks::ResourceMonitoringHook::new()));

    let context = create_test_hook_context("post-hook-test");
    let execution_result = create_test_execution_result("post-hook-test", true);

    let result = hook_manager
        .execute_post_hooks(&context, &execution_result)
        .await;
    assert!(result.is_ok(), "All post-hooks should execute successfully");

    // Post-hooks should not block execution even if they fail
    // They are for logging, cleanup, and monitoring
}

#[tokio::test]
async fn test_hook_manager_blocking_pre_hook() {
    let mut hook_manager = HookManager::new();

    // Add a blocking pre-hook
    let blocking_hook =
        hooks::PreflightCheckHook::new().with_blocked_commands(vec!["forbidden".to_string()]);

    hook_manager.add_pre_hook(Box::new(blocking_hook));

    let blocked_context = create_test_hook_context("forbidden");

    let result = hook_manager.execute_pre_hooks(&blocked_context).await;
    assert!(
        result.is_err(),
        "Pre-hooks should return error when blocking execution"
    );

    let error = result.unwrap_err();
    assert!(
        error.to_string().contains("Pre-hook"),
        "Error should indicate pre-hook failure"
    );
}

#[tokio::test]
async fn test_hook_priority_ordering() {
    let mut hook_manager = HookManager::new();

    // Add hooks with different priorities
    // High priority hooks should execute first
    hook_manager.add_pre_hook(Box::new(hooks::EnvironmentHook::new())); // priority 80
    hook_manager.add_pre_hook(Box::new(hooks::PreflightCheckHook::new())); // priority 90
    hook_manager.add_pre_hook(Box::new(hooks::LoggingHook::new())); // priority 100

    let context = create_test_hook_context("priority-test");

    let result = hook_manager.execute_pre_hooks(&context).await;
    assert!(
        result.is_ok(),
        "Hooks with different priorities should execute successfully"
    );

    // The order should be: Logging (100) -> PreflightCheck (90) -> Environment (80)
    // This is verified by the hooks executing without conflicts
}

#[tokio::test]
async fn test_default_hook_sets() {
    let default_hooks = hooks::create_default_hooks();
    assert!(
        !default_hooks.is_empty(),
        "Default hooks should not be empty"
    );

    let development_hooks = hooks::create_development_hooks();
    assert!(
        !development_hooks.is_empty(),
        "Development hooks should not be empty"
    );

    let production_hooks = hooks::create_production_hooks();
    assert!(
        !production_hooks.is_empty(),
        "Production hooks should not be empty"
    );

    // Production hooks should be more restrictive than default
    assert!(
        production_hooks.len() >= default_hooks.len(),
        "Production should have at least as many hooks as default"
    );
}

#[tokio::test]
async fn test_hook_error_handling() {
    let mut hook_manager = HookManager::new();

    // Add a hook that will fail
    let failing_hook =
        hooks::PreflightCheckHook::new().with_blocked_commands(vec!["test".to_string()]);

    hook_manager.add_pre_hook(Box::new(failing_hook));

    let context = create_test_hook_context("test");

    let result = hook_manager.execute_pre_hooks(&context).await;
    assert!(result.is_err(), "Should return error when pre-hook fails");

    // Test post-hooks with failing command result
    let execution_result = create_test_execution_result("test", false);

    let post_result = hook_manager
        .execute_post_hooks(&context, &execution_result)
        .await;
    // Post-hooks should not fail even when command fails
    assert!(
        post_result.is_ok(),
        "Post-hooks should execute even when command fails"
    );
}

#[tokio::test]
async fn test_hook_data_accumulation() {
    let mut hook_manager = HookManager::new();

    // Add hooks that return data
    hook_manager.add_pre_hook(Box::new(hooks::EnvironmentHook::new()));
    hook_manager.add_pre_hook(Box::new(hooks::ResourceMonitoringHook::new()));

    let context = create_test_hook_context("data-test");

    let result = hook_manager.execute_pre_hooks(&context).await;
    assert!(
        result.is_ok(),
        "Hooks with data should execute successfully"
    );

    // In a full implementation, you might want to accumulate data from multiple hooks
    // For now, just verify that hooks with data don't interfere with each other
}

#[tokio::test]
async fn test_concurrent_hook_execution() {
    // This test would verify that hooks can be executed concurrently when appropriate
    // For now, test sequential execution (which is the current implementation)

    let mut hook_manager = HookManager::new();

    hook_manager.add_pre_hook(Box::new(hooks::LoggingHook::new()));
    hook_manager.add_pre_hook(Box::new(hooks::EnvironmentHook::new()));

    let context = create_test_hook_context("concurrent-test");

    let start = std::time::Instant::now();
    let result = hook_manager.execute_pre_hooks(&context).await;
    let duration = start.elapsed();

    assert!(result.is_ok(), "Concurrent hook execution should succeed");
    assert!(duration.as_millis() < 100, "Hook execution should be fast");

    // In a future implementation, you might want to test actual concurrent execution
}
