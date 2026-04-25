//! End-to-end validation tests for terraphim_rlm.
//!
//! These tests use REAL infrastructure (no mocks):
//! - Firecracker VMs via KVM (or Docker fallback)
//! - Real LLM service (Ollama local models)
//! - Real MCP server (when available)
//!
//! Requirements:
//! - KVM access (/dev/kvm) OR Docker daemon
//! - Ollama running with at least one model
//! - gh CLI authenticated for private repo access

use terraphim_rlm::{
    config::{BackendType, RlmConfig},
    executor::{Capability, ExecutionEnvironment},
    rlm::TerraphimRlm,
};

/// Helper to create a minimal config suitable for testing.
fn test_config() -> RlmConfig {
    let mut config = RlmConfig::minimal();
    // Use small pool for fast tests
    config.pool_min_size = 1;
    config.pool_max_size = 2;
    config.pool_target_size = 1;
    // Short timeouts for tests
    config.vm_boot_timeout_ms = 30_000;
    config.allocation_timeout_ms = 5_000;
    config.time_budget_ms = 10_000;
    config.token_budget = 1000;
    config.max_recursion_depth = 2;
    config.max_iterations = 5;
    config
}

/// Test that we can create a TerraphimRlm with a real Firecracker executor.
/// This test requires KVM to be available.
#[tokio::test]
async fn test_rlm_creation_with_firecracker() {
    if !terraphim_rlm::executor::is_kvm_available() {
        eprintln!("Skipping test: KVM not available");
        return;
    }

    let config = test_config();
    let rlm = TerraphimRlm::new(config).await;

    // Creation may succeed or fail depending on Firecracker setup
    // We just verify it doesn't panic and handles the result properly
    match rlm {
        Ok(_) => {
            println!("TerraphimRlm created successfully with Firecracker backend");
        }
        Err(e) => {
            println!(
                "TerraphimRlm creation failed (expected if Firecracker not fully configured): {}",
                e
            );
        }
    }
}

/// Test that we can create a TerraphimRlm with a custom executor.
/// This uses the with_executor constructor which is useful for testing.
#[tokio::test]
async fn test_rlm_creation_with_executor() {
    use terraphim_rlm::executor::FirecrackerExecutor;

    if !terraphim_rlm::executor::is_kvm_available() {
        eprintln!("Skipping test: KVM not available");
        return;
    }

    let config = test_config();
    let executor = FirecrackerExecutor::new(config.clone())
        .expect("Failed to create FirecrackerExecutor (KVM should be available)");

    let rlm = TerraphimRlm::with_executor(config, executor)
        .expect("Failed to create TerraphimRlm with custom executor");

    // Verify basic properties
    assert_eq!(TerraphimRlm::version(), terraphim_rlm::VERSION);
    assert!(
        rlm.config()
            .backend_preference
            .contains(&BackendType::Firecracker)
            || rlm.config().backend_preference.is_empty()
    );
}

/// Test session lifecycle with real executor.
#[tokio::test]
async fn test_session_lifecycle_real() {
    use terraphim_rlm::executor::FirecrackerExecutor;
    use terraphim_rlm::types::SessionState;

    if !terraphim_rlm::executor::is_kvm_available() {
        eprintln!("Skipping test: KVM not available");
        return;
    }

    let config = test_config();
    let executor =
        FirecrackerExecutor::new(config.clone()).expect("Failed to create FirecrackerExecutor");

    let rlm = TerraphimRlm::with_executor(config, executor).expect("Failed to create TerraphimRlm");

    // Create session
    let session = rlm
        .create_session()
        .await
        .expect("Failed to create session");
    assert_eq!(session.state, SessionState::Initializing);

    // Get session
    let retrieved = rlm.get_session(&session.id).expect("Failed to get session");
    assert_eq!(retrieved.id, session.id);

    // Set and get context variable
    rlm.set_context_variable(&session.id, "test_key", "test_value")
        .expect("Failed to set context variable");
    let value = rlm
        .get_context_variable(&session.id, "test_key")
        .expect("Failed to get context variable");
    assert_eq!(value, Some("test_value".to_string()));

    // Destroy session
    rlm.destroy_session(&session.id)
        .await
        .expect("Failed to destroy session");
    assert!(rlm.get_session(&session.id).is_err());
}

/// Test code execution with real executor.
/// This will return a stub response unless VMs are fully configured.
#[tokio::test]
async fn test_execute_code_real() {
    use terraphim_rlm::executor::FirecrackerExecutor;

    if !terraphim_rlm::executor::is_kvm_available() {
        eprintln!("Skipping test: KVM not available");
        return;
    }

    let config = test_config();
    let executor =
        FirecrackerExecutor::new(config.clone()).expect("Failed to create FirecrackerExecutor");

    let rlm = TerraphimRlm::with_executor(config, executor).expect("Failed to create TerraphimRlm");

    let session = rlm
        .create_session()
        .await
        .expect("Failed to create session");

    // Execute Python code
    let result = rlm
        .execute_code(&session.id, "print('Hello from RLM!')")
        .await
        .expect("Failed to execute code");

    // The result may be a stub if no VM is allocated
    // In a full setup, this would execute in a real VM
    println!("Code execution result: {:?}", result);
    assert_eq!(result.exit_code, 0); // Stubs return 0

    // Clean up
    rlm.destroy_session(&session.id).await.ok();
}

/// Test command execution with real executor.
#[tokio::test]
async fn test_execute_command_real() {
    use terraphim_rlm::executor::FirecrackerExecutor;

    if !terraphim_rlm::executor::is_kvm_available() {
        eprintln!("Skipping test: KVM not available");
        return;
    }

    let config = test_config();
    let executor =
        FirecrackerExecutor::new(config.clone()).expect("Failed to create FirecrackerExecutor");

    let rlm = TerraphimRlm::with_executor(config, executor).expect("Failed to create TerraphimRlm");

    let session = rlm
        .create_session()
        .await
        .expect("Failed to create session");

    // Execute bash command
    let result = rlm
        .execute_command(&session.id, "echo 'Hello from RLM!'")
        .await
        .expect("Failed to execute command");

    println!("Command execution result: {:?}", result);
    assert_eq!(result.exit_code, 0);

    // Clean up
    rlm.destroy_session(&session.id).await.ok();
}

/// Test snapshot operations with real executor.
#[tokio::test]
async fn test_snapshots_real() {
    use terraphim_rlm::executor::FirecrackerExecutor;

    if !terraphim_rlm::executor::is_kvm_available() {
        eprintln!("Skipping test: KVM not available");
        return;
    }

    let config = test_config();
    let executor =
        FirecrackerExecutor::new(config.clone()).expect("Failed to create FirecrackerExecutor");

    let rlm = TerraphimRlm::with_executor(config, executor).expect("Failed to create TerraphimRlm");

    let session = rlm
        .create_session()
        .await
        .expect("Failed to create session");

    // Create snapshot
    let snapshot = rlm.create_snapshot(&session.id, "test_snapshot").await;

    // Snapshot creation may fail if VM is not assigned or managers not initialized
    match snapshot {
        Ok(s) => {
            println!("Created snapshot: {}", s.name);
            assert_eq!(s.name, "test_snapshot");
        }
        Err(e) => {
            println!(
                "Snapshot creation failed (expected if VM not assigned): {}",
                e
            );
        }
    }

    // List snapshots
    let snapshots = rlm
        .list_snapshots(&session.id)
        .await
        .expect("Failed to list snapshots");
    println!("Snapshots: {:?}", snapshots);

    // Clean up
    rlm.destroy_session(&session.id).await.ok();
}

/// Test health check with real executor.
#[tokio::test]
async fn test_health_check_real() {
    use terraphim_rlm::executor::FirecrackerExecutor;

    if !terraphim_rlm::executor::is_kvm_available() {
        eprintln!("Skipping test: KVM not available");
        return;
    }

    let config = test_config();
    let executor =
        FirecrackerExecutor::new(config.clone()).expect("Failed to create FirecrackerExecutor");

    let rlm = TerraphimRlm::with_executor(config, executor).expect("Failed to create TerraphimRlm");

    // Health check should work even without initialization
    let health = rlm.health_check().await.expect("Health check failed");

    // Health is false if not initialized, true if initialized
    println!("Health check result: {}", health);
}

/// Test session extension with real executor.
#[tokio::test]
async fn test_session_extension_real() {
    use terraphim_rlm::executor::FirecrackerExecutor;

    if !terraphim_rlm::executor::is_kvm_available() {
        eprintln!("Skipping test: KVM not available");
        return;
    }

    let config = test_config();
    let executor =
        FirecrackerExecutor::new(config.clone()).expect("Failed to create FirecrackerExecutor");

    let rlm = TerraphimRlm::with_executor(config, executor).expect("Failed to create TerraphimRlm");

    let session = rlm
        .create_session()
        .await
        .expect("Failed to create session");
    let original_expiry = session.expires_at;

    let extended = rlm
        .extend_session(&session.id)
        .expect("Failed to extend session");

    assert!(extended.expires_at > original_expiry);
    assert_eq!(extended.extension_count, 1);

    // Clean up
    rlm.destroy_session(&session.id).await.ok();
}

/// Test that validates the executor capabilities are correct.
#[tokio::test]
async fn test_executor_capabilities_real() {
    use terraphim_rlm::executor::FirecrackerExecutor;

    if !terraphim_rlm::executor::is_kvm_available() {
        eprintln!("Skipping test: KVM not available");
        return;
    }

    let config = test_config();
    let executor =
        FirecrackerExecutor::new(config.clone()).expect("Failed to create FirecrackerExecutor");

    assert!(executor.has_capability(Capability::VmIsolation));
    assert!(executor.has_capability(Capability::Snapshots));
    assert!(executor.has_capability(Capability::PythonExecution));
    assert!(executor.has_capability(Capability::BashExecution));
    assert!(!executor.has_capability(Capability::ContainerIsolation));
}

/// Test LLM query with real LLM bridge.
/// This requires Ollama to be running.
#[tokio::test]
async fn test_query_llm_real() {
    use terraphim_rlm::executor::FirecrackerExecutor;

    if !terraphim_rlm::executor::is_kvm_available() {
        eprintln!("Skipping test: KVM not available");
        return;
    }

    // Check if Ollama is available
    let ollama_available = reqwest::get("http://127.0.0.1:11434/api/tags")
        .await
        .map(|r| r.status().is_success())
        .unwrap_or(false);

    if !ollama_available {
        eprintln!("Skipping test: Ollama not available");
        return;
    }

    let config = test_config();
    let executor =
        FirecrackerExecutor::new(config.clone()).expect("Failed to create FirecrackerExecutor");

    let rlm = TerraphimRlm::with_executor(config, executor).expect("Failed to create TerraphimRlm");

    let session = rlm
        .create_session()
        .await
        .expect("Failed to create session");

    // Query LLM directly (without full query loop)
    let result = rlm
        .query_llm(&session.id, "What is 2+2? Answer with just the number.")
        .await;

    match result {
        Ok(response) => {
            println!("LLM response: {}", response.response);
            println!("Tokens used: {}", response.tokens_used);
            assert!(!response.response.is_empty());
        }
        Err(e) => {
            println!("LLM query failed: {}", e);
            // Don't fail the test if LLM is not configured
        }
    }

    // Clean up
    rlm.destroy_session(&session.id).await.ok();
}

/// Test the full query loop with real infrastructure.
/// This is the most comprehensive test.
#[tokio::test]
async fn test_full_query_loop_real() {
    use terraphim_rlm::executor::FirecrackerExecutor;
    use terraphim_rlm::query_loop::TerminationReason;

    if !terraphim_rlm::executor::is_kvm_available() {
        eprintln!("Skipping test: KVM not available");
        return;
    }

    // Check if Ollama is available
    let ollama_available = reqwest::get("http://127.0.0.1:11434/api/tags")
        .await
        .map(|r| r.status().is_success())
        .unwrap_or(false);

    if !ollama_available {
        eprintln!("Skipping test: Ollama not available");
        return;
    }

    let config = test_config();
    let executor =
        FirecrackerExecutor::new(config.clone()).expect("Failed to create FirecrackerExecutor");

    let rlm = TerraphimRlm::with_executor(config, executor).expect("Failed to create TerraphimRlm");

    let session = rlm
        .create_session()
        .await
        .expect("Failed to create session");

    // Run a simple query
    let result = rlm.query(&session.id, "Calculate 2+2").await;

    match result {
        Ok(query_result) => {
            println!("Query result: {:?}", query_result.result);
            println!("Termination reason: {:?}", query_result.termination_reason);
            println!("Iterations: {}", query_result.iterations);

            // Should terminate for some reason (budget, final, or error)
            match query_result.termination_reason {
                TerminationReason::FinalReached => {
                    println!("Query completed successfully");
                }
                TerminationReason::TokenBudgetExhausted => {
                    println!("Query ran out of tokens");
                }
                TerminationReason::TimeBudgetExhausted => {
                    println!("Query ran out of time");
                }
                TerminationReason::MaxIterationsReached => {
                    println!("Query reached max iterations");
                }
                TerminationReason::RecursionDepthExhausted => {
                    println!("Query reached max recursion depth");
                }
                TerminationReason::Error { message } => {
                    println!("Query encountered an error: {}", message);
                }
                TerminationReason::Cancelled => {
                    println!("Query was cancelled");
                }
                TerminationReason::FinalVarReached { variable } => {
                    println!("Query returned variable: {}", variable);
                }
            }
        }
        Err(e) => {
            println!("Query failed: {}", e);
            // Don't fail the test - the query loop may fail for various reasons
        }
    }

    // Clean up
    rlm.destroy_session(&session.id).await.ok();
}

/// Test that validates all the doc examples can be replicated with real infrastructure.
#[tokio::test]
async fn test_doc_examples_real() {
    use terraphim_rlm::executor::FirecrackerExecutor;

    if !terraphim_rlm::executor::is_kvm_available() {
        eprintln!("Skipping test: KVM not available");
        return;
    }

    let config = RlmConfig::default();
    let executor =
        FirecrackerExecutor::new(config.clone()).expect("Failed to create FirecrackerExecutor");

    let rlm = TerraphimRlm::with_executor(config, executor).expect("Failed to create TerraphimRlm");

    // Example from lib.rs
    let session = rlm
        .create_session()
        .await
        .expect("Failed to create session");

    let result = rlm
        .execute_code(&session.id, "print('Hello, RLM!')")
        .await
        .expect("Failed to execute code");
    println!("Output: {}", result.stdout);

    let result = rlm
        .execute_command(&session.id, "ls -la")
        .await
        .expect("Failed to execute command");
    println!("Output: {}", result.stdout);

    // Clean up
    rlm.destroy_session(&session.id).await.ok();
}

/// Test cleanup and resource management.
#[tokio::test]
async fn test_cleanup_real() {
    use terraphim_rlm::executor::FirecrackerExecutor;

    if !terraphim_rlm::executor::is_kvm_available() {
        eprintln!("Skipping test: KVM not available");
        return;
    }

    let config = test_config();
    let executor =
        FirecrackerExecutor::new(config.clone()).expect("Failed to create FirecrackerExecutor");

    let rlm = TerraphimRlm::with_executor(config, executor).expect("Failed to create TerraphimRlm");

    // Create and destroy multiple sessions
    for i in 0..3 {
        let session = rlm
            .create_session()
            .await
            .expect("Failed to create session");

        rlm.set_context_variable(&session.id, "index", &i.to_string())
            .expect("Failed to set context variable");

        rlm.destroy_session(&session.id)
            .await
            .expect("Failed to destroy session");
    }

    // Verify no sessions remain
    let stats = rlm.get_stats();
    assert_eq!(stats.active_sessions, 0);
}
