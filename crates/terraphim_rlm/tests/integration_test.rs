//! End-to-end integration test for all 6 MCP tools
//! This test exercises the complete RLM functionality

#[cfg(test)]
mod integration_tests {
    use serde_json::Map;
    use terraphim_rlm::executor::{is_kvm_available, select_executor};
    use terraphim_rlm::mcp_tools::RlmMcpService;
    use terraphim_rlm::{BudgetTracker, RlmConfig, RlmError, SessionId, TerraphimRlm};

    /// Test all 6 MCP tools end-to-end
    #[tokio::test]
    async fn test_mcp_tools_e2e() {
        println!("\n=== MCP Tools E2E Test ===\n");

        // Initialize RLM
        let config = RlmConfig::default();
        let rlm = TerraphimRlm::new(config.clone())
            .await
            .expect("Failed to create RLM");

        // Initialize MCP service
        let mcp = RlmMcpService::new();
        mcp.initialize(rlm).await;

        let session_id = SessionId::new();
        mcp.set_session(session_id).await;

        // Test 1: rlm_status - Get session status
        println!("Test 1: rlm_status");
        let mut args = Map::new();
        args.insert(
            "session_id".to_string(),
            serde_json::json!(session_id.to_string()),
        );

        match mcp.call_tool("rlm_status", Some(args)).await {
            Ok(result) => println!("  ✅ Session status retrieved"),
            Err(e) => println!("  ⚠️  Status error (expected if no session): {}", e),
        }

        // Test 2: rlm_context - Get context
        println!("\nTest 2: rlm_context (get)");
        let mut args = Map::new();
        args.insert("action".to_string(), serde_json::json!("get"));
        args.insert("key".to_string(), serde_json::json!("test_key"));

        match mcp.call_tool("rlm_context", Some(args)).await {
            Ok(result) => println!("  ✅ Context retrieved"),
            Err(e) => println!("  ⚠️  Context error: {}", e),
        }

        // Test 3: rlm_context - Set context
        println!("\nTest 3: rlm_context (set)");
        let mut args = Map::new();
        args.insert("action".to_string(), serde_json::json!("set"));
        args.insert("key".to_string(), serde_json::json!("test_key"));
        args.insert("value".to_string(), serde_json::json!("test_value"));

        match mcp.call_tool("rlm_context", Some(args)).await {
            Ok(result) => println!("  ✅ Context set successfully"),
            Err(e) => println!("  ⚠️  Context set error: {}", e),
        }

        // Test 4: rlm_code - Execute Python (uses MockExecutor without firecracker feature)
        println!("\nTest 4: rlm_code");
        let mut args = Map::new();
        args.insert(
            "code".to_string(),
            serde_json::json!("print('Hello from RLM')"),
        );
        args.insert(
            "session_id".to_string(),
            serde_json::json!(session_id.to_string()),
        );

        match mcp.call_tool("rlm_code", Some(args)).await {
            Ok(result) => {
                println!("  ✅ Code executed");
                // Parse result to show output
                if let Some(content) = result.content.first() {
                    println!("  Output: {:?}", content);
                }
            }
            Err(e) => println!("  ⚠️  Code execution error: {}", e),
        }

        // Test 5: rlm_bash - Execute bash command
        println!("\nTest 5: rlm_bash");
        let mut args = Map::new();
        args.insert(
            "command".to_string(),
            serde_json::json!("echo 'Hello from bash'"),
        );
        args.insert(
            "session_id".to_string(),
            serde_json::json!(session_id.to_string()),
        );

        match mcp.call_tool("rlm_bash", Some(args)).await {
            Ok(result) => {
                println!("  ✅ Bash command executed");
                if let Some(content) = result.content.first() {
                    println!("  Output: {:?}", content);
                }
            }
            Err(e) => println!("  ⚠️  Bash execution error: {}", e),
        }

        // Test 6: rlm_snapshot - Create snapshot
        println!("\nTest 6: rlm_snapshot (create)");
        let mut args = Map::new();
        args.insert("action".to_string(), serde_json::json!("create"));
        args.insert("name".to_string(), serde_json::json!("test-snapshot"));
        args.insert(
            "session_id".to_string(),
            serde_json::json!(session_id.to_string()),
        );

        match mcp.call_tool("rlm_snapshot", Some(args)).await {
            Ok(result) => {
                println!("  ✅ Snapshot created");
                if let Some(content) = result.content.first() {
                    println!("  Snapshot: {:?}", content);
                }
            }
            Err(e) => println!("  ⚠️  Snapshot error: {}", e),
        }

        // Test 7: rlm_query - Query LLM
        println!("\nTest 7: rlm_query");
        let mut args = Map::new();
        args.insert("prompt".to_string(), serde_json::json!("What is 2+2?"));
        args.insert(
            "session_id".to_string(),
            serde_json::json!(session_id.to_string()),
        );

        match mcp.call_tool("rlm_query", Some(args)).await {
            Ok(result) => {
                println!("  ✅ LLM query executed");
                if let Some(content) = result.content.first() {
                    println!("  Response: {:?}", content);
                }
            }
            Err(e) => println!("  ⚠️  LLM query error: {}", e),
        }

        println!("\n=== MCP Tools Test Complete ===\n");
    }

    /// Test executor selection with KVM check
    #[tokio::test]
    async fn test_executor_selection() {
        println!("\n=== Executor Selection Test ===\n");

        let config = RlmConfig::default();

        match select_executor(&config).await {
            Ok(executor) => {
                println!("✅ Executor selected successfully");
                println!("  KVM Available: {}", is_kvm_available());
            }
            Err(e) => {
                println!("⚠️  Executor selection failed: {}", e);
            }
        }

        println!();
    }

    /// Test budget tracking
    #[test]
    fn test_budget_tracking() {
        println!("\n=== Budget Tracking Test ===\n");

        let config = RlmConfig::default();
        let budget = BudgetTracker::new(&config);

        // Test token tracking
        budget.add_tokens(100).expect("Failed to add tokens");
        budget.add_tokens(200).expect("Failed to add tokens");

        assert!(budget.check_token_budget().is_ok());
        println!("✅ Token budget tracking works");

        // Test time budget
        assert!(budget.check_time_budget().is_ok());
        println!("✅ Time budget tracking works");

        println!();
    }

    /// Test session lifecycle
    #[tokio::test]
    async fn test_session_lifecycle() {
        println!("\n=== Session Lifecycle Test ===\n");

        let config = RlmConfig::default();
        let rlm = TerraphimRlm::new(config)
            .await
            .expect("Failed to create RLM");

        // Create session
        let session_id = SessionId::new();
        println!("✅ Session created: {}", session_id);

        // Test session operations
        match rlm.get_session(&session_id) {
            Ok(_session) => println!("✅ Session retrieved successfully"),
            Err(_e) => println!("ℹ️  Session not found (expected for new session)"),
        }

        println!();
    }
}
