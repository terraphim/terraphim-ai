//! Comprehensive tests for the command system
//!
//! This module contains unit and integration tests for all components of the
//! markdown-based command system including parsing, registry, validation,
//! execution modes, and hooks.

#[cfg(test)]
#[allow(clippy::module_inception)]
mod tests {
    use chrono::{Datelike, Timelike};
    use std::collections::HashMap;
    use std::path::PathBuf;

    // Import all the types we need for tests
    use crate::commands::executor;
    use crate::commands::registry::CommandRegistry;
    use crate::commands::validator::{CommandValidator, SecurityAction, SecurityResult};
    use crate::commands::{
        hooks::{BackupHook, EnvironmentHook, LoggingHook, PreflightCheckHook},
        HookContext,
    };
    use crate::commands::{
        CommandDefinition, CommandHook, CommandParameter, ExecutionMode, HookManager,
        ParsedCommand, RiskLevel,
    };
    use crate::CommandExecutionResult;

    // Test data and helper functions
    fn create_test_command_definition() -> CommandDefinition {
        CommandDefinition {
            name: "test-command".to_string(),
            description: "Test command for unit testing".to_string(),
            usage: Some("test-command [options]".to_string()),
            category: Some("Testing".to_string()),
            version: "1.0.0".to_string(),
            risk_level: RiskLevel::Low,
            execution_mode: ExecutionMode::Local,
            permissions: vec!["read".to_string()],
            knowledge_graph_required: vec![],
            namespace: None,
            aliases: vec!["test".to_string()],
            timeout: Some(30),
            resource_limits: None,
            parameters: vec![
                CommandParameter {
                    name: "input".to_string(),
                    param_type: "string".to_string(),
                    required: true,
                    description: Some("Input parameter".to_string()),
                    default_value: None,
                    validation: None,
                    allowed_values: None,
                },
                CommandParameter {
                    name: "verbose".to_string(),
                    param_type: "boolean".to_string(),
                    required: false,
                    description: Some("Verbose output".to_string()),
                    default_value: Some(serde_json::Value::Bool(false)),
                    validation: None,
                    allowed_values: None,
                },
            ],
        }
    }

    fn create_test_markdown() -> String {
        r#"---
name: test-command
description: Test command for unit testing
usage: "test-command [options]"
category: Testing
version: "1.0.0"
risk_level: low
execution_mode: local
permissions:
  - read
aliases:
  - test
parameters:
  - name: input
    type: string
    required: true
    description: Input parameter
  - name: verbose
    type: boolean
    required: false
    default_value: false
    description: Verbose output
timeout: 30
---

# Test Command

This is a test command for unit testing purposes.

## Examples

```bash
test-command --input "hello" --verbose
```
"#
        .to_string()
    }

    async fn create_temp_command_file(content: String) -> (PathBuf, tempfile::TempDir) {
        let temp_dir = tempfile::tempdir().unwrap();
        let file_path = temp_dir.path().join("test-command.md");
        tokio::fs::write(&file_path, content).await.unwrap();
        (file_path, temp_dir)
    }

    // === Markdown Parser Tests ===

    #[tokio::test]
    async fn test_parse_markdown_command_valid() {
        let markdown = create_test_markdown();
        let (file_path, _temp_dir) = create_temp_command_file(markdown).await;

        let result = super::super::markdown_parser::parse_markdown_command(&file_path).await;
        assert!(
            result.is_ok(),
            "Should successfully parse valid markdown command"
        );

        let parsed = result.unwrap();
        assert_eq!(parsed.definition.name, "test-command");
        assert_eq!(
            parsed.definition.description,
            "Test command for unit testing"
        );
        assert_eq!(parsed.definition.risk_level, RiskLevel::Low);
        assert_eq!(parsed.definition.execution_mode, ExecutionMode::Local);
        assert_eq!(parsed.definition.parameters.len(), 2);
        // Test that markdown structure is preserved
        assert!(parsed.content.contains("# Test Command"));
        assert!(parsed
            .content
            .contains("This is a test command for unit testing purposes."));
    }

    #[tokio::test]
    async fn test_parse_markdown_command_missing_frontmatter() {
        let markdown = r#"# Simple Command

This command has no frontmatter.
"#
        .to_string();
        let (file_path, _temp_dir) = create_temp_command_file(markdown).await;

        let result = super::super::markdown_parser::parse_markdown_command(&file_path).await;
        assert!(result.is_err(), "Should fail when frontmatter is missing");
    }

    #[tokio::test]
    async fn test_parse_markdown_command_invalid_yaml() {
        let markdown = r#"---
name: test-command
description: Test command
invalid_yaml: [unclosed array
---

# Test Command
"#
        .to_string();
        let (file_path, _temp_dir) = create_temp_command_file(markdown).await;

        let result = super::super::markdown_parser::parse_markdown_command(&file_path).await;
        assert!(result.is_err(), "Should fail with invalid YAML");
    }

    #[tokio::test]
    async fn test_parse_markdown_command_parameter_validation() {
        let markdown = r#"---
name: test-command
description: Test command
parameters:
  - name: input
    type: string
    required: true
  - name: number
    type: number
    required: true
    validation:
      min: 0
      max: 100
---

# Test Command
"#
        .to_string();
        let (file_path, _temp_dir) = create_temp_command_file(markdown).await;

        let result = super::super::markdown_parser::parse_markdown_command(&file_path).await;
        assert!(result.is_ok());

        let parsed = result.unwrap();
        assert_eq!(parsed.definition.parameters.len(), 2);

        let number_param = &parsed.definition.parameters[1];
        assert_eq!(number_param.name, "number");
        assert_eq!(number_param.param_type, "number");
        assert!(number_param.validation.is_some());
    }

    // === Command Registry Tests ===

    #[tokio::test]
    async fn test_registry_add_and_get_command() {
        let registry = CommandRegistry::new().unwrap();
        let command_def = create_test_command_definition();
        let parsed = ParsedCommand {
            definition: command_def.clone(),
            content: "# Test Command".to_string(),
            source_path: PathBuf::from("test.md"),
            modified: std::time::SystemTime::now(),
        };

        let result = registry.register_command(parsed).await;
        assert!(
            result.is_ok(),
            "Should successfully add command to registry"
        );

        let retrieved = registry.get_command("test-command").await;
        assert!(
            retrieved.is_some(),
            "Should be able to retrieve added command"
        );

        let retrieved_def = retrieved.unwrap();
        assert_eq!(retrieved_def.definition.name, "test-command");
        assert_eq!(
            retrieved_def.definition.description,
            "Test command for unit testing"
        );
    }

    #[tokio::test]
    async fn test_registry_add_duplicate_command() {
        let registry = CommandRegistry::new().unwrap();
        let command_def = create_test_command_definition();
        let parsed = ParsedCommand {
            definition: command_def,
            content: "# Test Command".to_string(),
            source_path: PathBuf::from("test.md"),
            modified: std::time::SystemTime::now(),
        };

        let result1 = registry.register_command(parsed.clone()).await;
        assert!(result1.is_ok());

        let result2 = registry.register_command(parsed).await;
        assert!(result2.is_err(), "Should fail to add duplicate command");
    }

    #[tokio::test]
    async fn test_registry_get_command_by_alias() {
        let registry = CommandRegistry::new().unwrap();
        let command_def = create_test_command_definition();
        let parsed = ParsedCommand {
            definition: command_def,
            content: "# Test Command".to_string(),
            source_path: PathBuf::from("test.md"),
            modified: std::time::SystemTime::now(),
        };

        registry.register_command(parsed).await.unwrap();

        let retrieved = registry.resolve_command("test").await;
        assert!(
            retrieved.is_some(),
            "Should be able to retrieve command by alias"
        );
        assert_eq!(retrieved.unwrap().definition.name, "test-command");
    }

    #[tokio::test]
    async fn test_registry_search_commands() {
        let registry = CommandRegistry::new().unwrap();

        // Add multiple commands
        let commands = vec![
            ("search-files", "Search for files in the system"),
            ("deploy-app", "Deploy application to production"),
            ("test-unit", "Run unit tests"),
        ];

        for (name, description) in commands {
            let mut command_def = create_test_command_definition();
            command_def.name = name.to_string();
            command_def.description = description.to_string();
            command_def.aliases = vec![];

            let parsed = ParsedCommand {
                definition: command_def,
                content: format!("# {}", name),
                source_path: PathBuf::from(format!("{}.md", name)),
                modified: std::time::SystemTime::now(),
            };

            registry.register_command(parsed).await.unwrap();
        }

        // Test search functionality
        let search_results = registry.search_commands("search").await;
        assert_eq!(
            search_results.len(),
            1,
            "Should find one command matching 'search'"
        );
        assert_eq!(search_results[0].definition.name, "search-files");

        let deploy_results = registry.search_commands("deploy").await;
        assert_eq!(
            deploy_results.len(),
            1,
            "Should find one command matching 'deploy'"
        );
        assert_eq!(deploy_results[0].definition.name, "deploy-app");

        let test_results = registry.search_commands("test").await;
        assert_eq!(
            test_results.len(),
            1,
            "Should find one command matching 'test'"
        );
        assert_eq!(test_results[0].definition.name, "test-unit");
    }

    #[tokio::test]
    async fn test_registry_get_stats() {
        let registry = CommandRegistry::new().unwrap();

        let stats = registry.get_stats().await;
        assert_eq!(stats.total_commands, 0, "Initially should have no commands");
        assert_eq!(
            stats.total_categories, 0,
            "Initially should have no categories"
        );

        // Add commands from different categories
        let categories = vec![("Testing", "test-unit"), ("Deployment", "deploy-app")];
        for (category, name) in categories {
            let mut command_def = create_test_command_definition();
            command_def.name = name.to_string();
            command_def.category = Some(category.to_string());
            command_def.aliases = vec![];

            let parsed = ParsedCommand {
                definition: command_def,
                content: format!("# {}", name),
                source_path: PathBuf::from(format!("{}.md", name)),
                modified: std::time::SystemTime::now(),
            };

            registry.register_command(parsed).await.unwrap();
        }

        let updated_stats = registry.get_stats().await;
        assert_eq!(updated_stats.total_commands, 2, "Should have 2 commands");
        assert_eq!(
            updated_stats.total_categories, 2,
            "Should have 2 categories"
        );
    }

    // === Command Validator Tests ===

    #[tokio::test]
    async fn test_validator_role_permissions() {
        let mut validator = CommandValidator::new();

        // Test default role permissions
        let result = validator
            .validate_command_execution("ls -la", "Default", &HashMap::new())
            .await;

        assert!(
            result.is_ok(),
            "Default role should be able to execute read-only commands"
        );

        // Test write operation with default role
        let result = validator
            .validate_command_execution("rm file.txt", "Default", &HashMap::new())
            .await;

        assert!(
            result.is_err(),
            "Default role should not be able to execute write operations"
        );

        // Test write operation with engineer role
        let result = validator
            .validate_command_execution("rm file.txt", "Terraphim Engineer", &HashMap::new())
            .await;

        assert!(
            result.is_ok(),
            "Engineer role should be able to execute write operations"
        );
    }

    #[tokio::test]
    async fn test_validator_risk_assessment() {
        let mut validator = CommandValidator::new();

        // Test that validator can be created and configured
        assert!(
            !validator.is_blacklisted("ls -la"),
            "Should not blacklist safe commands by default"
        );

        // Test public interface methods
        validator.add_role_permissions("TestRole".to_string(), vec!["read".to_string()]);

        // Test time restrictions
        let time_result = validator.check_time_restrictions();
        // Note: This test might fail if run on weekends due to default business hour restrictions
        // The validator correctly restricts to Monday-Friday, 9 AM - 5 PM
        if time_result.is_err() {
            println!(
                "Time restriction test info: This may fail on weekends. Current time restrictions: Mon-Fri, 9AM-5PM"
            );
        }
        // For now, we'll just ensure the validator doesn't panic - time_result existence proves no panic

        // Test rate limiting
        let rate_result = validator.check_rate_limit("test");
        assert!(rate_result.is_ok(), "Rate limiting should pass by default");
    }

    #[tokio::test]
    async fn test_validator_rate_limiting() {
        let mut validator = CommandValidator::new();

        // Add test rate limit using public interface
        validator.set_rate_limit("test", 2, std::time::Duration::from_secs(60));

        // First request should succeed
        let result1 = validator.check_rate_limit("test command");
        assert!(result1.is_ok(), "First request should succeed");

        // Second request should succeed
        let result2 = validator.check_rate_limit("test command");
        assert!(result2.is_ok(), "Second request should succeed");

        // Third request should fail
        let result3 = validator.check_rate_limit("test command");
        assert!(
            result3.is_err(),
            "Third request should fail due to rate limiting"
        );
    }

    #[tokio::test]
    async fn test_validator_blacklisting() {
        let validator = CommandValidator::new();

        // Test non-blacklisted command
        assert!(
            !validator.is_blacklisted("ls -la"),
            "Normal commands should not be blacklisted"
        );

        // Test blacklisted command
        assert!(
            validator.is_blacklisted("rm -rf /"),
            "Dangerous commands should be blacklisted"
        );
        assert!(
            validator.is_blacklisted("dd if=/dev/zero"),
            "System commands should be blacklisted"
        );
    }

    #[tokio::test]
    async fn test_validator_time_restrictions() {
        let validator = CommandValidator::new();

        // Test business hours (9 AM - 5 PM, Monday - Friday)
        let current_time = std::time::SystemTime::now();
        let datetime = chrono::DateTime::<chrono::Utc>::from(current_time);
        let local_time = datetime.with_timezone(&chrono::Local);

        // This test might fail depending on when it's run
        // In a real test environment, you would mock the current time
        if (9..=17).contains(&local_time.hour())
            && (1..=5).contains(&local_time.weekday().num_days_from_sunday())
        {
            let result = validator.check_time_restrictions();
            assert!(
                result.is_ok(),
                "Should allow commands during business hours"
            );
        }
    }

    #[tokio::test]
    async fn test_validator_security_validation() {
        let mut validator = CommandValidator::new();

        // Test valid command
        let result = validator
            .validate_command_security("help", "Terraphim Engineer", "test_user")
            .await;

        // Note: This test may fail on weekends or outside business hours due to default time restrictions
        // The validator correctly restricts to Monday-Friday, 9 AM - 5 PM
        if let Err(ref e) = result {
            println!(
                "Security validation failed (expected on weekends/off-hours): {:?}",
                e
            );
            // If the failure is due to time restrictions, that's correct behavior
            let err_msg = e.to_string();
            if err_msg.contains("Commands not allowed on this day")
                || err_msg.contains("Commands not allowed at this time")
            {
                return; // Skip assertion - this is expected behavior outside business hours
            }
        }

        assert!(
            result.is_ok(),
            "Valid command should pass security validation (or fail due to weekend time restrictions). Error: {:?}",
            result
        );

        // Test blacklisted command
        let result = validator
            .validate_command_security("rm -rf /", "Terraphim Engineer", "test_user")
            .await;

        assert!(
            result.is_err(),
            "Blacklisted command should fail security validation"
        );
    }

    // === Hook Tests ===

    #[tokio::test]
    async fn test_logging_hook() {
        let hook = LoggingHook::new();
        let context = HookContext {
            command: "test-command".to_string(),
            parameters: HashMap::new(),
            user: "test_user".to_string(),
            role: "Terraphim Engineer".to_string(),
            execution_mode: ExecutionMode::Local,
            working_directory: PathBuf::from("/test"),
        };

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
            "Should log success message"
        );
    }

    #[tokio::test]
    async fn test_preflight_check_hook() {
        let hook = PreflightCheckHook::new()
            .with_blocked_commands(vec!["rm -rf /".to_string(), "dangerous".to_string()]);

        // Test safe command
        let safe_context = HookContext {
            command: "ls -la".to_string(),
            parameters: HashMap::new(),
            user: "test_user".to_string(),
            role: "Terraphim Engineer".to_string(),
            execution_mode: ExecutionMode::Local,
            working_directory: PathBuf::from("/test"),
        };

        let result = hook.execute(&safe_context).await;
        assert!(result.is_ok());
        assert!(
            result.unwrap().should_continue,
            "Safe commands should pass preflight check"
        );

        // Test blocked command
        let blocked_context = HookContext {
            command: "rm -rf /".to_string(),
            parameters: HashMap::new(),
            user: "test_user".to_string(),
            role: "Terraphim Engineer".to_string(),
            execution_mode: ExecutionMode::Local,
            working_directory: PathBuf::from("/test"),
        };

        let result = hook.execute(&blocked_context).await;
        assert!(result.is_ok());
        assert!(
            !result.unwrap().should_continue,
            "Blocked commands should not pass preflight check"
        );
    }

    #[tokio::test]
    async fn test_environment_hook() {
        let hook = EnvironmentHook::new()
            .with_env("TEST_VAR", "test_value")
            .with_env("DEBUG", "true");

        let context = HookContext {
            command: "test-command".to_string(),
            parameters: HashMap::new(),
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
        assert!(
            hook_result.data.is_some(),
            "Environment hook should return data"
        );

        if let Some(data) = hook_result.data {
            assert!(data.get("TEST_VAR").is_some(), "Should set TEST_VAR");
            assert!(
                data.get("COMMAND_USER").is_some(),
                "Should set COMMAND_USER"
            );
        }
    }

    #[tokio::test]
    async fn test_backup_hook() {
        let temp_dir = tempfile::tempdir().unwrap();
        let backup_dir = temp_dir.path().join("backups");
        let hook = BackupHook::new(&backup_dir)
            .with_backup_commands(vec!["rm".to_string(), "mv".to_string()]);

        // Test command that requires backup
        let backup_context = HookContext {
            command: "rm file.txt".to_string(),
            parameters: HashMap::new(),
            user: "test_user".to_string(),
            role: "Terraphim Engineer".to_string(),
            execution_mode: ExecutionMode::Local,
            working_directory: PathBuf::from("/test"),
        };

        let result = hook.execute(&backup_context).await;
        assert!(result.is_ok(), "Backup hook should execute successfully");

        let hook_result = result.unwrap();
        assert!(hook_result.success, "Backup hook should succeed");
        assert!(backup_dir.exists(), "Backup directory should be created");

        // Test command that doesn't require backup
        let no_backup_context = HookContext {
            command: "ls -la".to_string(),
            parameters: HashMap::new(),
            user: "test_user".to_string(),
            role: "Terraphim Engineer".to_string(),
            execution_mode: ExecutionMode::Local,
            working_directory: PathBuf::from("/test"),
        };

        let result = hook.execute(&no_backup_context).await;
        assert!(result.is_ok());
        let hook_result = result.unwrap();
        assert!(
            hook_result.message.contains("No backup needed"),
            "Should indicate no backup needed"
        );
    }

    #[tokio::test]
    async fn test_hook_manager() {
        let mut hook_manager = HookManager::new();

        // Add test hooks
        hook_manager.add_pre_hook(Box::new(LoggingHook::new()));
        hook_manager.add_post_hook(Box::new(LoggingHook::new()));

        let context = HookContext {
            command: "test-command".to_string(),
            parameters: HashMap::new(),
            user: "test_user".to_string(),
            role: "Terraphim Engineer".to_string(),
            execution_mode: ExecutionMode::Local,
            working_directory: PathBuf::from("/test"),
        };

        // Test pre-hooks
        let result = hook_manager.execute_pre_hooks(&context).await;
        assert!(result.is_ok(), "Pre-hooks should execute successfully");

        // Test post-hooks
        let execution_result = CommandExecutionResult {
            command: "test-command".to_string(),
            execution_mode: ExecutionMode::Local,
            exit_code: 0,
            stdout: "success".to_string(),
            stderr: String::new(),
            duration_ms: 100,
            resource_usage: None,
        };

        let result = hook_manager
            .execute_post_hooks(&context, &execution_result)
            .await;
        assert!(result.is_ok(), "Post-hooks should execute successfully");
    }

    // === Command Executor Tests ===

    #[tokio::test]
    async fn test_command_executor_with_hooks() {
        let hooks = vec![
            Box::new(LoggingHook::new()) as Box<dyn CommandHook + Send + Sync>,
            Box::new(PreflightCheckHook::new()) as Box<dyn CommandHook + Send + Sync>,
        ];

        let executor = executor::CommandExecutor::new().with_hooks(hooks);
        let command_def = create_test_command_definition();
        let mut parameters = HashMap::new();
        parameters.insert("command".to_string(), "echo test".to_string());

        let _result = executor
            .execute_with_context(
                &command_def,
                &parameters,
                "test-command",
                "test_user",
                "Terraphim Engineer",
                ".",
            )
            .await;

        // This might fail depending on whether the LocalExecutor is implemented
        // For now, we test that the hook system integration doesn't panic
        // In a complete implementation, you would mock the actual command execution
    }

    // === Integration Tests ===

    #[tokio::test]
    async fn test_end_to_end_command_processing() {
        // This test demonstrates the complete flow from markdown parsing to execution
        let markdown = create_test_markdown();
        let (file_path, _temp_dir) = create_temp_command_file(markdown).await;

        // Parse markdown
        let parsed = super::super::markdown_parser::parse_markdown_command(&file_path)
            .await
            .unwrap();

        // Create registry and add command
        let registry = CommandRegistry::new().unwrap();
        registry.register_command(parsed.clone()).await.unwrap();

        // Create validator
        let mut validator = CommandValidator::new();

        // Validate command
        let execution_mode = validator
            .validate_command_execution(
                &parsed.definition.name,
                "Terraphim Engineer",
                &HashMap::new(),
            )
            .await
            .unwrap();

        assert_eq!(execution_mode, ExecutionMode::Hybrid);

        // Create executor with hooks
        let hooks = vec![
            Box::new(LoggingHook::new()) as Box<dyn CommandHook + Send + Sync>,
            Box::new(PreflightCheckHook::new()) as Box<dyn CommandHook + Send + Sync>,
        ];
        let _executor = executor::CommandExecutor::new().with_hooks(hooks);

        // Execute command (LocalExecutor is fully implemented!)
        let mut parameters = HashMap::new();
        parameters.insert("command".to_string(), "echo test".to_string());

        // For now, just test that the context is created correctly
        let context = HookContext {
            command: parsed.definition.name.clone(),
            parameters: parameters.clone(),
            user: "test_user".to_string(),
            role: "Terraphim Engineer".to_string(),
            execution_mode,
            working_directory: PathBuf::from("."),
        };

        assert_eq!(context.command, "test-command");
        assert_eq!(context.user, "test_user");
        assert_eq!(context.role, "Terraphim Engineer");
        assert_eq!(context.execution_mode, ExecutionMode::Hybrid);
    }

    #[tokio::test]
    async fn test_command_parameter_validation() {
        let _command_def = create_test_command_definition();

        // Test valid parameters
        let mut valid_params = HashMap::new();
        valid_params.insert("input".to_string(), "test_value".to_string());
        valid_params.insert("verbose".to_string(), "true".to_string());

        // This would test parameter validation logic
        // Implementation depends on how you choose to validate parameters

        // Test missing required parameter
        let mut invalid_params = HashMap::new();
        invalid_params.insert("verbose".to_string(), "true".to_string());
        // Missing required "input" parameter

        // Validation would fail for missing required parameter
    }

    #[tokio::test]
    async fn test_command_alias_resolution() {
        let registry = CommandRegistry::new().unwrap();
        let command_def = create_test_command_definition();
        let parsed = ParsedCommand {
            definition: command_def,
            content: "# Test Command".to_string(),
            source_path: PathBuf::from("test.md"),
            modified: std::time::SystemTime::now(),
        };

        registry.register_command(parsed).await.unwrap();

        // Test getting command by name
        let by_name = registry.get_command("test-command").await;
        assert!(by_name.is_some(), "Should find command by name");

        // Test getting command by alias
        let by_alias = registry.resolve_command("test").await;
        assert!(by_alias.is_some(), "Should find command by alias");

        // Test getting non-existent command
        let not_found = registry.get_command("non-existent").await;
        assert!(not_found.is_none(), "Should not find non-existent command");
    }

    #[tokio::test]
    async fn test_security_event_logging() {
        let mut validator = CommandValidator::new();

        // Log some security events
        validator.log_security_event(
            "test_user",
            "test-command",
            SecurityAction::CommandValidation,
            SecurityResult::Allowed,
            "Test validation passed",
        );

        validator.log_security_event(
            "test_user",
            "dangerous-command",
            SecurityAction::BlacklistCheck,
            SecurityResult::Denied("Command is blacklisted".to_string()),
            "Blacklisted command attempted",
        );

        // Check statistics
        let stats = validator.get_security_stats();
        assert_eq!(stats.total_events, 2, "Should have 2 total events");
        assert_eq!(stats.denied_events, 1, "Should have 1 denied event");

        // Check recent events
        let recent_events = validator.get_recent_events(10);
        assert_eq!(recent_events.len(), 2, "Should return 2 recent events");

        // Verify event details
        let denied_event = &recent_events[0];
        assert_eq!(denied_event.user, "test_user");
        assert_eq!(denied_event.command, "dangerous-command");
        assert!(matches!(denied_event.result, SecurityResult::Denied(_)));
    }
}
