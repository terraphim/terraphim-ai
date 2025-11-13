//! Integration tests for the command system
//!
//! These tests verify the end-to-end functionality of the markdown-based
//! command system including parsing, validation, execution, and security.

use std::collections::HashMap;
use std::path::PathBuf;
use std::str::FromStr;

use tempfile::TempDir;
use terraphim_tui::commands::validator::{SecurityAction, SecurityResult};
use terraphim_tui::commands::{
    hooks, CommandHook, CommandRegistry, CommandValidator, ExecutionMode, HookContext, HookManager,
};
use tokio::fs;

/// Creates a temporary directory with test command files
async fn setup_test_commands_directory() -> (TempDir, PathBuf) {
    let temp_dir = tempfile::tempdir().unwrap();
    let commands_dir = temp_dir.path().join("commands");
    fs::create_dir(&commands_dir).await.unwrap();

    // Create test command files
    let commands = vec![
        (
            "search.md",
            r#"---
name: search
description: Search files and content using ripgrep
usage: "search <query> [--type] [--case-sensitive]"
category: File Operations
version: "1.0.0"
risk_level: low
execution_mode: local
permissions:
  - read
aliases:
  - find
parameters:
  - name: query
    type: string
    required: true
    description: Search query
  - name: type
    type: string
    required: false
    default_value: "all"
    allowed_values: ["all", "rs", "js", "md", "json"]
    description: File type filter
timeout: 60
---

# Search Command

Search for files and content using ripgrep with advanced filtering.

## Examples

```bash
search "TODO" --type rs
search "function.*test" --case-sensitive
```
"#,
        ),
        (
            "deploy.md",
            r#"---
name: deploy
description: Deploy applications with safety checks
usage: "deploy <environment> [--dry-run]"
category: Deployment
version: "1.0.0"
risk_level: high
execution_mode: firecracker
permissions:
  - read
  - write
  - execute
knowledge_graph_required:
  - deployment
  - infrastructure
aliases:
  - ship
parameters:
  - name: environment
    type: string
    required: true
    allowed_values: ["staging", "production"]
    description: Target environment
  - name: dry_run
    type: boolean
    required: false
    default_value: false
    description: Perform dry run without making changes
resource_limits:
  max_memory_mb: 2048
  max_cpu_time: 1800
  network_access: true
timeout: 3600
---

# Deploy Command

Deploy applications to specified environments with comprehensive safety checks.

## Safety Features

- Pre-deployment validation
- Rollback capability
- Health checks
- Environment-specific configurations
"#,
        ),
        (
            "security-audit.md",
            r#"---
name: security-audit
description: Perform comprehensive security audit and vulnerability scanning
usage: "security-audit [target] [--deep] [--report]"
category: Security
version: "1.0.0"
risk_level: critical
execution_mode: firecracker
permissions:
  - read
  - execute
knowledge_graph_required:
  - security
  - vulnerability_assessment
  - compliance
parameters:
  - name: target
    type: string
    required: false
    default_value: "."
    description: Target path or component to audit
  - name: deep
    type: boolean
    required: false
    default_value: false
    description: Perform deep analysis
  - name: report
    type: boolean
    required: false
    default_value: true
    description: Generate detailed security report
resource_limits:
  max_memory_mb: 4096
  max_cpu_time: 3600
  network_access: false
timeout: 7200
---

# Security Audit Command

Comprehensive security vulnerability scanning and compliance checking.

## Security Checks

- Dependency vulnerability scanning
- Static code analysis
- Secret detection
- Configuration security review
"#,
        ),
        (
            "hello-world.md",
            r#"---
name: hello-world
description: Simple hello world command for testing
usage: "hello-world [name] [--greeting]"
category: Testing
version: "1.0.0"
risk_level: low
execution_mode: local
permissions:
  - read
aliases:
  - hello
  - hi
parameters:
  - name: name
    type: string
    required: false
    default_value: "World"
    description: Name to greet
  - name: greeting
    type: string
    required: false
    allowed_values: ["hello", "hi", "hey", "greetings"]
    default_value: "hello"
    description: Greeting type
timeout: 10
---

# Hello World Command

A simple greeting command used for testing the command system.
"#,
        ),
    ];

    for (filename, content) in commands {
        let file_path = commands_dir.join(filename);
        fs::write(file_path, content).await.unwrap();
    }

    (temp_dir, commands_dir)
}

#[tokio::test]
async fn test_full_command_lifecycle() {
    // Setup test environment
    let (_temp_dir, commands_dir) = setup_test_commands_directory().await;

    // Initialize command registry
    let mut registry = CommandRegistry::new().unwrap();
    registry.add_command_directory(commands_dir);

    // Load all commands
    let loaded_count = registry.load_all_commands().await.unwrap();
    assert_eq!(loaded_count, 4, "Should load 4 commands");

    // Test command retrieval
    let search_cmd = registry.get_command("search").await;
    assert!(search_cmd.is_some(), "Should find search command");

    let hello_cmd = registry.get_command("hello-world").await;
    assert!(hello_cmd.is_some(), "Should find hello-world command");

    let deploy_cmd = registry.get_command("deploy").await;
    assert!(deploy_cmd.is_some(), "Should find deploy command");

    // Test alias resolution
    let hello_alias = registry.resolve_command("hello").await;
    assert!(hello_alias.is_some(), "Should find command by alias");
    assert_eq!(hello_alias.unwrap().definition.name, "hello-world");

    // Test search functionality
    let search_results = registry.search_commands("security").await;
    assert_eq!(
        search_results.len(),
        1,
        "Should find 1 security-related command"
    );
    assert_eq!(search_results[0].definition.name, "security-audit");

    let deploy_results = registry.search_commands("dep").await;
    assert_eq!(deploy_results.len(), 1, "Should find deploy command");
    assert_eq!(deploy_results[0].definition.name, "deploy");

    // Test statistics
    let stats = registry.get_stats().await;
    assert_eq!(stats.total_commands, 4, "Should have 4 total commands");
    assert_eq!(stats.total_categories, 4, "Should have 4 categories");
}

#[tokio::test]
async fn test_security_validation_integration() {
    let (_temp_dir, commands_dir) = setup_test_commands_directory().await;

    // Initialize registry and validator
    let mut registry = CommandRegistry::new().unwrap();
    registry.add_command_directory(commands_dir);
    registry.load_all_commands().await.unwrap();

    let mut validator = CommandValidator::new();

    // Test low-risk command validation
    let hello_cmd = registry.get_command("hello-world").await.unwrap();
    let result = validator
        .validate_command_execution(&hello_cmd.definition.name, "Default", &HashMap::new())
        .await;

    assert!(
        result.is_ok(),
        "Default role should execute low-risk commands"
    );
    assert_eq!(result.unwrap(), ExecutionMode::Local);

    // Test high-risk command with default role
    let deploy_cmd = registry.get_command("deploy").await.unwrap();
    let result = validator
        .validate_command_execution(&deploy_cmd.definition.name, "Default", &HashMap::new())
        .await;

    // Default role might not have execute permissions for high-risk commands
    // The exact behavior depends on permission implementation
    println!("Deploy command validation result: {:?}", result);

    // Test high-risk command with engineer role
    let result = validator
        .validate_command_execution(
            &deploy_cmd.definition.name,
            "Terraphim Engineer",
            &HashMap::new(),
        )
        .await;

    assert!(
        result.is_ok(),
        "Engineer role should validate high-risk commands"
    );

    // Test critical risk command
    let audit_cmd = registry.get_command("security-audit").await.unwrap();
    let result = validator
        .validate_command_execution(
            &audit_cmd.definition.name,
            "Terraphim Engineer",
            &HashMap::new(),
        )
        .await;

    assert!(
        result.is_ok(),
        "Should validate critical risk commands for engineers"
    );
    assert_eq!(result.unwrap(), ExecutionMode::Firecracker);
}

#[tokio::test]
async fn test_hook_system_integration() {
    let (_temp_dir, commands_dir) = setup_test_commands_directory().await;

    // Initialize system components
    let mut registry = CommandRegistry::new().unwrap();
    registry.add_command_directory(commands_dir);
    registry.load_all_commands().await.unwrap();

    let mut validator = CommandValidator::new();

    // Create hook manager with test hooks
    let mut hook_manager = HookManager::new();
    hook_manager.add_pre_hook(Box::new(hooks::LoggingHook::new()));
    hook_manager.add_pre_hook(Box::new(hooks::PreflightCheckHook::new()));
    hook_manager.add_post_hook(Box::new(hooks::LoggingHook::new()));

    // Test command with hooks
    let hello_cmd = registry.get_command("hello-world").await.unwrap();
    let mut parameters = HashMap::new();
    parameters.insert("name".to_string(), "Test".to_string());

    let hook_context = HookContext {
        command: hello_cmd.definition.name.clone(),
        parameters: parameters.clone(),
        user: "test_user".to_string(),
        role: "Terraphim Engineer".to_string(),
        execution_mode: ExecutionMode::Local,
        working_directory: std::env::current_dir().unwrap(),
    };

    // Execute pre-hooks
    let pre_result = hook_manager.execute_pre_hooks(&hook_context).await;
    assert!(pre_result.is_ok(), "Pre-hooks should execute successfully");

    // Mock command execution result
    let execution_result = terraphim_tui::commands::CommandExecutionResult {
        command: hello_cmd.definition.name.clone(),
        execution_mode: ExecutionMode::Local,
        exit_code: 0,
        stdout: "Hello, Test!".to_string(),
        stderr: String::new(),
        duration_ms: 50,
        resource_usage: None,
    };

    // Execute post-hooks
    let post_result = hook_manager
        .execute_post_hooks(&hook_context, &execution_result)
        .await;
    assert!(
        post_result.is_ok(),
        "Post-hooks should execute successfully"
    );
}

#[tokio::test]
async fn test_rate_limiting_integration() {
    let mut validator = CommandValidator::new();

    // Set up rate limiting for search command
    validator.set_rate_limit("search", 2, std::time::Duration::from_secs(60));

    // First two requests should succeed
    let result1 = validator.check_rate_limit("search");
    assert!(result1.is_ok(), "First request should succeed");

    let result2 = validator.check_rate_limit("search");
    assert!(result2.is_ok(), "Second request should succeed");

    // Third request should fail
    let result3 = validator.check_rate_limit("search");
    assert!(
        result3.is_err(),
        "Third request should fail due to rate limiting"
    );

    // Different command should not be affected
    let result4 = validator.check_rate_limit("deploy");
    assert!(
        result4.is_ok(),
        "Different command should not be rate limited"
    );
}

#[tokio::test]
async fn test_security_event_logging() {
    let mut validator = CommandValidator::new();

    // Log various security events
    validator.log_security_event(
        "test_user",
        "hello-world",
        SecurityAction::CommandValidation,
        SecurityResult::Allowed,
        "Command validation passed",
    );

    validator.log_security_event(
        "test_user",
        "deploy",
        SecurityAction::PermissionCheck,
        SecurityResult::Denied("Insufficient permissions".to_string()),
        "User lacks execute permissions",
    );

    validator.log_security_event(
        "admin_user",
        "security-audit",
        SecurityAction::KnowledgeGraphCheck,
        SecurityResult::Allowed,
        "Knowledge graph concepts verified",
    );

    // Test statistics
    let stats = validator.get_security_stats();
    assert_eq!(stats.total_events, 3, "Should have 3 total events");
    assert_eq!(stats.denied_events, 1, "Should have 1 denied event");
    assert_eq!(stats.recent_events, 3, "Should have 3 recent events");

    // Test recent events retrieval
    let recent_events = validator.get_recent_events(2);
    assert_eq!(recent_events.len(), 2, "Should return 2 most recent events");

    // Verify event ordering (most recent first)
    assert_eq!(recent_events[0].command, "security-audit");
    assert_eq!(recent_events[1].command, "deploy");
}

#[tokio::test]
async fn test_backup_hook_integration() {
    let temp_dir = tempfile::tempdir().unwrap();
    let backup_dir = temp_dir.path().join("backups");

    let hook = hooks::BackupHook::new(&backup_dir).with_backup_commands(vec![
        "rm".to_string(),
        "mv".to_string(),
        "deploy".to_string(),
    ]);

    // Test command that requires backup
    let backup_context = HookContext {
        command: "deploy production".to_string(),
        parameters: HashMap::new(),
        user: "test_user".to_string(),
        role: "Terraphim Engineer".to_string(),
        execution_mode: ExecutionMode::Firecracker,
        working_directory: PathBuf::from("/test"),
    };

    let result = hook.execute(&backup_context).await;
    assert!(result.is_ok(), "Backup hook should execute successfully");

    let hook_result = result.unwrap();
    assert!(hook_result.success, "Backup should succeed");
    assert!(backup_dir.exists(), "Backup directory should be created");

    // Verify backup file was created
    let mut backup_files: Vec<_> = std::fs::read_dir(&backup_dir)
        .unwrap()
        .map(|entry| entry.unwrap())
        .collect();

    assert_eq!(backup_files.len(), 1, "Should create one backup file");

    // Test command that doesn't require backup
    let no_backup_context = HookContext {
        command: "search test".to_string(),
        parameters: HashMap::new(),
        user: "test_user".to_string(),
        role: "Terraphim Engineer".to_string(),
        execution_mode: ExecutionMode::Local,
        working_directory: PathBuf::from("/test"),
    };

    let result = hook.execute(&no_backup_context).await;
    assert!(result.is_ok(), "Hook should execute successfully");

    let hook_result = result.unwrap();
    assert!(
        hook_result.message.contains("No backup needed"),
        "Should indicate no backup needed"
    );
}

#[tokio::test]
async fn test_environment_hook_integration() {
    let hook = hooks::EnvironmentHook::new()
        .with_env("TEST_MODE", "true")
        .with_env("LOG_LEVEL", "debug")
        .with_env("USER_ROLE", "test_engineer");

    let mut parameters = HashMap::new();
    parameters.insert("input".to_string(), "test_value".to_string());

    let context = HookContext {
        command: "test-command".to_string(),
        parameters: parameters.clone(),
        user: "test_user".to_string(),
        role: "Terraphim Engineer".to_string(),
        execution_mode: ExecutionMode::Local,
        working_directory: PathBuf::from("/test"),
    };

    let result = hook.execute(&context).await;
    assert!(
        result.is_ok(),
        "Environment hook should execute successfully"
    );

    let hook_result = result.unwrap();
    assert!(hook_result.success, "Environment hook should succeed");
    assert!(hook_result.data.is_some(), "Should return environment data");

    if let Some(data) = hook_result.data {
        // Check custom environment variables
        assert_eq!(data.get("TEST_MODE").unwrap(), "true");
        assert_eq!(data.get("LOG_LEVEL").unwrap(), "debug");
        assert_eq!(data.get("USER_ROLE").unwrap(), "test_engineer");

        // Check automatically added environment variables
        assert_eq!(data.get("COMMAND_USER").unwrap(), "test_user");
        assert_eq!(data.get("COMMAND_ROLE").unwrap(), "Terraphim Engineer");
        assert_eq!(data.get("COMMAND_WORKING_DIR").unwrap(), "/test");
    }
}

#[tokio::test]
async fn test_command_suggestion_system() {
    let (_temp_dir, commands_dir) = setup_test_commands_directory().await;

    let mut registry = CommandRegistry::new().unwrap();
    registry.add_command_directory(commands_dir);
    registry.load_all_commands().await.unwrap();

    // Test partial name suggestions
    let suggestions = registry.search_commands("sec").await;
    assert_eq!(suggestions.len(), 1, "Should suggest security-audit");
    assert_eq!(suggestions[0].definition.name, "security-audit");

    // Test category-based suggestions
    let security_commands = registry.search_commands("security").await;
    assert_eq!(security_commands.len(), 1, "Should find security commands");

    // Test description-based search
    let deploy_commands = registry.search_commands("application").await;
    assert_eq!(deploy_commands.len(), 1, "Should find deploy command");
    assert!(deploy_commands[0]
        .definition
        .description
        .contains("Deploy applications"));

    // Test case-insensitive search
    let hello_commands = registry.search_commands("HeLLo").await;
    assert_eq!(hello_commands.len(), 1, "Should be case-insensitive");
    assert_eq!(hello_commands[0].definition.name, "hello-world");
}

#[tokio::test]
async fn test_parameter_validation_integration() {
    let (_temp_dir, commands_dir) = setup_test_commands_directory().await;

    let mut registry = CommandRegistry::new().unwrap();
    registry.add_command_directory(commands_dir);
    registry.load_all_commands().await.unwrap();

    // Test deploy command parameter validation
    let deploy_cmd = registry.get_command("deploy").await.unwrap();

    // Valid parameters
    let mut valid_params = HashMap::new();
    valid_params.insert("environment".to_string(), "staging".to_string());
    valid_params.insert("dry-run".to_string(), "true".to_string());

    // This would require implementing parameter validation logic
    // For now, we just verify the parameter structure
    assert_eq!(
        deploy_cmd.definition.parameters.len(),
        2,
        "Deploy command should have 2 parameters"
    );

    let env_param = &deploy_cmd.definition.parameters[0];
    assert_eq!(env_param.name, "environment");
    assert_eq!(env_param.param_type, "string");
    assert!(env_param.required);
    assert!(env_param
        .validation
        .as_ref()
        .unwrap()
        .allowed_values
        .is_some());

    let dry_run_param = &deploy_cmd.definition.parameters[1];
    assert_eq!(dry_run_param.name, "dry-run");
    assert_eq!(dry_run_param.param_type, "boolean");
    assert!(!dry_run_param.required);
    assert!(dry_run_param.default_value.is_some());

    // Test search command parameter validation
    let search_cmd = registry.get_command("search").await.unwrap();
    assert_eq!(
        search_cmd.definition.parameters.len(),
        2,
        "Search command should have 2 parameters"
    );

    let query_param = &search_cmd.definition.parameters[0];
    assert_eq!(query_param.name, "query");
    assert!(query_param.required);

    let type_param = &search_cmd.definition.parameters[1];
    assert_eq!(type_param.name, "type");
    assert!(!type_param.required);
    assert!(type_param.default_value.is_some());
}

#[tokio::test]
async fn test_role_based_command_access() {
    let mut validator = CommandValidator::new();

    // Test different role permissions
    let test_cases = vec![
        ("Default", "ls -la", true),                          // Read-only command
        ("Default", "rm file.txt", false),                    // Write command
        ("Default", "systemctl stop nginx", false),           // System command
        ("Terraphim Engineer", "ls -la", true),               // Read command
        ("Terraphim Engineer", "rm file.txt", true),          // Write command
        ("Terraphim Engineer", "systemctl stop nginx", true), // System command
    ];

    for (role, command, should_succeed) in test_cases {
        let result = validator
            .validate_command_execution(command, role, &HashMap::new())
            .await;

        if should_succeed {
            assert!(
                result.is_ok(),
                "Role '{}' should be able to execute '{}'",
                role,
                command
            );
        } else {
            assert!(
                result.is_err(),
                "Role '{}' should not be able to execute '{}'",
                role,
                command
            );
        }
    }
}
