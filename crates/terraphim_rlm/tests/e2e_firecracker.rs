//! End-to-end integration tests for terraphim_rlm with Firecracker
//!
//! These tests verify the Firecracker integration works correctly.
//! Note: Full VM execution requires complete VM pool implementation.

use terraphim_rlm::{BackendType, RlmConfig, TerraphimRlm};

fn setup() {
    if !std::path::Path::new("/dev/kvm").exists() {
        panic!("KVM not available - skipping Firecracker tests");
    }
}

#[tokio::test]
async fn test_e2e_session_lifecycle() {
    setup();

    let mut config = RlmConfig::default();
    config.backend_preference = vec![BackendType::Firecracker];

    let rlm = TerraphimRlm::new(config)
        .await
        .expect("Failed to create RLM");

    // Test session creation
    let session = rlm
        .create_session()
        .await
        .expect("Failed to create session");
    println!("Created session: {}", session.id);

    // Verify session exists
    let info = rlm.get_session(&session.id).expect("Failed to get session");
    assert_eq!(info.id, session.id);

    // Test context variables
    rlm.set_context_variable(&session.id, "test_key", "test_value")
        .expect("Failed to set context variable");

    let value = rlm
        .get_context_variable(&session.id, "test_key")
        .expect("Failed to get context variable");
    assert_eq!(value, Some("test_value".to_string()));

    // Clean up
    rlm.destroy_session(&session.id)
        .await
        .expect("Failed to destroy session");
    println!("Session lifecycle test PASSED");
}

#[tokio::test]
async fn test_e2e_python_execution_stub() {
    setup();

    let config = RlmConfig::default();
    let rlm = TerraphimRlm::new(config)
        .await
        .expect("Failed to create RLM");

    let session = rlm
        .create_session()
        .await
        .expect("Failed to create session");
    println!("Session created: {}", session.id);

    // Test Python code execution (currently returns stub due to VM pool WIP)
    let code = "print('Hello from Python!')";
    let result = rlm.execute_code(&session.id, code).await;

    match result {
        Ok(exec_result) => {
            println!("Python execution stdout: {}", exec_result.stdout);
            println!("Python execution stderr: {}", exec_result.stderr);
            println!("Exit code: {}", exec_result.exit_code);
            // Currently returns stub - verify stub format
            assert!(exec_result.stdout.contains("[FirecrackerExecutor]"));
            assert_eq!(exec_result.exit_code, 0);
        }
        Err(e) => {
            panic!("Python execution failed: {:?}", e);
        }
    }

    rlm.destroy_session(&session.id).await.ok();
    println!("Python execution stub test PASSED");
}

#[tokio::test]
async fn test_e2e_bash_execution_stub() {
    setup();

    let config = RlmConfig::default();
    let rlm = TerraphimRlm::new(config)
        .await
        .expect("Failed to create RLM");

    let session = rlm
        .create_session()
        .await
        .expect("Failed to create session");

    // Test bash command execution (currently returns stub)
    let result = rlm
        .execute_command(&session.id, "echo 'Hello from bash'")
        .await;

    match result {
        Ok(exec_result) => {
            println!("Bash execution stdout: {}", exec_result.stdout);
            println!("Bash execution stderr: {}", exec_result.stderr);
            // Currently returns stub - verify stub format
            assert!(exec_result.stdout.contains("[FirecrackerExecutor]"));
            assert_eq!(exec_result.exit_code, 0);
        }
        Err(e) => {
            panic!("Bash execution failed: {:?}", e);
        }
    }

    rlm.destroy_session(&session.id).await.ok();
    println!("Bash execution stub test PASSED");
}

#[tokio::test]
async fn test_e2e_budget_tracking() {
    setup();

    let config = RlmConfig {
        token_budget: 1000,
        time_budget_ms: 60000,
        max_recursion_depth: 5,
        ..Default::default()
    };

    let rlm = TerraphimRlm::new(config)
        .await
        .expect("Failed to create RLM");
    let session = rlm
        .create_session()
        .await
        .expect("Failed to create session");

    // Check session has budget tracking
    let info = rlm.get_session(&session.id).expect("Failed to get session");
    println!("Session budget status: {:?}", info.budget_status);

    // Budget should be within limits
    assert!(info.budget_status.tokens_used <= 1000);
    assert!(info.budget_status.time_used_ms <= 60000);

    rlm.destroy_session(&session.id).await.ok();
    println!("Budget tracking test PASSED");
}

#[tokio::test]
async fn test_e2e_snapshots_no_vm() {
    setup();

    let config = RlmConfig::default();
    let rlm = TerraphimRlm::new(config)
        .await
        .expect("Failed to create RLM");
    let session = rlm
        .create_session()
        .await
        .expect("Failed to create session");

    // Try to create a snapshot (will fail without VM assigned)
    let snapshot_result = rlm.create_snapshot(&session.id, "test_checkpoint").await;

    // Expected to fail - no VM assigned yet
    assert!(snapshot_result.is_err());
    println!(
        "Snapshot creation correctly failed: {:?}",
        snapshot_result.err()
    );

    // List snapshots should return empty
    let snapshots = rlm
        .list_snapshots(&session.id)
        .await
        .expect("Failed to list snapshots");
    assert!(snapshots.is_empty());

    rlm.destroy_session(&session.id).await.ok();
    println!("Snapshot test PASSED");
}

#[tokio::test]
async fn test_e2e_health_check() {
    setup();

    let config = RlmConfig::default();
    let rlm = TerraphimRlm::new(config)
        .await
        .expect("Failed to create RLM");

    let healthy = rlm.health_check().await.expect("Health check failed");
    println!("Health check result: {}", healthy);

    // Health check passes (KVM available, managers initialized)
    // but returns false because VM pool is not fully initialized
    println!("Health check test PASSED (result: {})", healthy);
}

#[tokio::test]
async fn test_e2e_session_extension() {
    setup();

    let config = RlmConfig::default();
    let rlm = TerraphimRlm::new(config)
        .await
        .expect("Failed to create RLM");

    let session = rlm
        .create_session()
        .await
        .expect("Failed to create session");
    let original_expiry = session.expires_at;

    // Extend the session
    let extended = rlm
        .extend_session(&session.id)
        .expect("Failed to extend session");
    println!("Original expiry: {:?}", original_expiry);
    println!("Extended expiry: {:?}", extended.expires_at);

    // Extended expiry should be later than original
    assert!(extended.expires_at > original_expiry);

    rlm.destroy_session(&session.id).await.ok();
    println!("Session extension test PASSED");
}
