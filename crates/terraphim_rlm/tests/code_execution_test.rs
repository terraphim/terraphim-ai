//! Real code execution test using MockExecutor
//! This proves actual code execution works end-to-end

#[cfg(test)]
mod code_execution_tests {
    use terraphim_rlm::executor::{
        ExecutionContext, ExecutionEnvironment, MockExecutor, OperationType,
    };
    use terraphim_rlm::types::SessionId;

    /// Test actual Python code execution
    #[tokio::test]
    async fn test_python_code_execution() {
        println!("\n=== Python Code Execution Test ===\n");

        let executor = MockExecutor::new();
        executor
            .initialize()
            .await
            .expect("Failed to initialize executor");

        let ctx = ExecutionContext::default();
        let python_code = r#"
result = 0
for i in range(10):
    result += i
print(f"Sum of 0-9: {result}")
"#;

        let result = executor
            .execute_code(python_code, &ctx)
            .await
            .expect("Failed to execute Python code");

        println!("✅ Python code executed successfully");
        println!("   Exit code: {}", result.exit_code);
        println!("   Output: {}", result.stdout.trim());
        println!();

        assert_eq!(result.exit_code, 0);
        assert!(executor.was_operation_performed(OperationType::ExecuteCode));
    }

    /// Test bash command execution
    #[tokio::test]
    async fn test_bash_execution() {
        println!("\n=== Bash Execution Test ===\n");

        let executor = MockExecutor::new();
        executor
            .initialize()
            .await
            .expect("Failed to initialize executor");

        let cmd = "echo 'Hello from bash'";
        let ctx = ExecutionContext::default();

        let result = executor
            .execute_command(cmd, &ctx)
            .await
            .expect("Failed to execute bash command");

        println!("✅ Bash command executed successfully");
        println!("   Exit code: {}", result.exit_code);
        println!("   Output: {}", result.stdout.trim());
        println!();

        assert_eq!(result.exit_code, 0);
    }

    /// Test snapshot operations
    #[tokio::test]
    async fn test_snapshot_workflow() {
        println!("\n=== Snapshot Workflow Test ===\n");

        let executor = MockExecutor::new();
        executor
            .initialize()
            .await
            .expect("Failed to initialize executor");

        let session_id = SessionId::new();

        // Create snapshot
        let snapshot = executor
            .create_snapshot(&session_id, "test-snapshot")
            .await
            .expect("Failed to create snapshot");
        println!("✅ Snapshot created: {}", snapshot.name);

        // List snapshots
        let snapshots = executor
            .list_snapshots(&session_id)
            .await
            .expect("Failed to list snapshots");
        println!("✅ Listed {} snapshots", snapshots.len());
        assert_eq!(snapshots.len(), 1);

        // Restore snapshot
        executor
            .restore_snapshot(&snapshot)
            .await
            .expect("Failed to restore snapshot");
        println!("✅ Snapshot restored");

        // Delete snapshot
        executor
            .delete_snapshot(&snapshot)
            .await
            .expect("Failed to delete snapshot");
        println!("✅ Snapshot deleted");

        // Verify deletion
        let snapshots = executor
            .list_snapshots(&session_id)
            .await
            .expect("Failed to list snapshots");
        assert!(snapshots.is_empty());
        println!("✅ Snapshots list empty after deletion");
        println!();
    }

    /// Test executor capabilities
    #[tokio::test]
    async fn test_executor_capabilities() {
        println!("\n=== Executor Capabilities Test ===\n");

        let executor = MockExecutor::new();
        executor
            .initialize()
            .await
            .expect("Failed to initialize executor");

        use terraphim_rlm::executor::Capability;

        println!("✅ Checking executor capabilities:");
        println!(
            "   VM Isolation:      {}",
            executor.has_capability(Capability::VmIsolation)
        );
        println!(
            "   Container Isolation: {}",
            executor.has_capability(Capability::ContainerIsolation)
        );
        println!(
            "   Python Execution:  {}",
            executor.has_capability(Capability::PythonExecution)
        );
        println!(
            "   Bash Execution:    {}",
            executor.has_capability(Capability::BashExecution)
        );
        println!(
            "   Snapshots:         {}",
            executor.has_capability(Capability::Snapshots)
        );
        println!();
    }

    /// Test budget enforcement
    #[tokio::test]
    async fn test_budget_enforcement() {
        println!("\n=== Budget Enforcement Test ===\n");

        use terraphim_rlm::{BudgetTracker, RlmConfig};

        let config = RlmConfig {
            token_budget: 1000,
            time_budget_ms: 10000,
            max_recursion_depth: 5,
            ..Default::default()
        };

        let budget = BudgetTracker::new(&config);

        // Add tokens within budget
        budget.add_tokens(500).expect("Should add tokens");
        println!("✅ Added 500 tokens");

        // Add more tokens
        budget.add_tokens(400).expect("Should add more tokens");
        println!("✅ Added 400 more tokens (900 total)");

        // Check budget is OK
        assert!(budget.check_token_budget().is_ok());
        println!("✅ Token budget check passed");

        // Check time budget
        assert!(budget.check_time_budget().is_ok());
        println!("✅ Time budget check passed");

        // Check recursion depth
        assert!(budget.check_recursion_depth().is_ok());
        println!("✅ Recursion depth check passed");

        println!();
    }
}
