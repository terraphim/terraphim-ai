// End-to-end test for GitHub runner with Firecracker VM and knowledge graph learning
//
// This test demonstrates:
// 1. Creating a Firecracker VM session
// 2. Executing commands in the VM
// 3. LearningCoordinator tracking success/failure
// 4. Knowledge graph integration recording patterns

use chrono::Utc;
use std::sync::Arc;
use terraphim_github_runner::{
    CommandExecutor, CommandKnowledgeGraph, GitHubEvent, GitHubEventType,
    InMemoryLearningCoordinator, LearningCoordinator, ParsedWorkflow, RepositoryInfo, SessionId,
    SessionManager, SessionManagerConfig, VmCommandExecutor, WorkflowContext, WorkflowExecutor,
    WorkflowExecutorConfig, WorkflowStep,
};
use uuid::Uuid;

/// Helper to create a test GitHub event
fn create_test_event() -> GitHubEvent {
    GitHubEvent {
        event_type: GitHubEventType::Push,
        action: None,
        repository: RepositoryInfo {
            full_name: "testuser/test-repo".to_string(),
            clone_url: Some("https://github.com/testuser/test-repo.git".to_string()),
            default_branch: Some("main".to_string()),
        },
        pull_request: None,
        git_ref: Some("refs/heads/main".to_string()),
        sha: Some(Uuid::new_v4().to_string()),
        extra: Default::default(),
    }
}

/// Helper to create a test workflow
#[allow(dead_code)]
fn create_test_workflow() -> ParsedWorkflow {
    ParsedWorkflow {
        name: "Test Rust CI Workflow".to_string(),
        trigger: "push".to_string(),
        environment: Default::default(),
        setup_commands: vec!["echo 'Setting up environment'".to_string()],
        steps: vec![
            WorkflowStep {
                name: "Build Project".to_string(),
                command: "cargo build --release".to_string(),
                working_dir: "/workspace".to_string(),
                continue_on_error: false,
                timeout_seconds: 60,
            },
            WorkflowStep {
                name: "Run Tests".to_string(),
                command: "cargo test".to_string(),
                working_dir: "/workspace".to_string(),
                continue_on_error: false,
                timeout_seconds: 60,
            },
        ],
        cleanup_commands: vec!["echo 'Cleanup complete'".to_string()],
        cache_paths: vec![],
    }
}

#[tokio::test]
#[ignore] // Requires Firecracker VM running locally
async fn end_to_end_real_firecracker_vm() {
    // Initialize logging
    let _ = env_logger::try_init();

    println!("\n=== END-TO-END TEST: Real Firecracker VM ===\n");

    // Check if Firecracker API is available
    let health_url = "http://127.0.0.1:8080/health";
    let response = reqwest::get(health_url).await;
    if response.is_err() || !response.unwrap().status().is_success() {
        panic!("⚠️  Firecracker API not available at http://127.0.0.1:8080");
    }

    // Get JWT token from environment
    let jwt_token = std::env::var("FIRECRACKER_AUTH_TOKEN")
        .expect("FIRECRACKER_AUTH_TOKEN must be set for real Firecracker test");

    // Step 1: Create knowledge graph and learning coordinator
    println!("📊 Step 1: Initializing Knowledge Graph and LearningCoordinator...");
    let _knowledge_graph = CommandKnowledgeGraph::new()
        .await
        .expect("Failed to create knowledge graph");

    let coordinator = InMemoryLearningCoordinator::with_knowledge_graph("test-agent")
        .await
        .expect("Failed to create learning coordinator");

    println!("✅ Knowledge graph and learning coordinator initialized");

    // Step 2: Get or create a real VM
    println!("\n🎯 Step 2: Getting real Firecracker VM...");

    // Try to get existing VM, or create a new one
    let vm_id: String = {
        let client = reqwest::Client::new();
        let list_response = client
            .get("http://127.0.0.1:8080/api/vms")
            .bearer_auth(&jwt_token)
            .send()
            .await
            .expect("Failed to list VMs");

        let vms: serde_json::Value = list_response.json().await.expect("Failed to parse VM list");

        if let Some(vms_array) = vms["vms"].as_array() {
            if !vms_array.is_empty() {
                // Use first running VM
                if let Some(vm) = vms_array.iter().find(|v| v["status"] == "running") {
                    println!("✅ Using existing VM: {}", vm["id"]);
                    vm["id"].as_str().unwrap().to_string()
                } else {
                    // Create new VM
                    println!("Creating new VM...");
                    let create_response = client
                        .post("http://127.0.0.1:8080/api/vms")
                        .bearer_auth(&jwt_token)
                        .json(&serde_json::json!({"name": "test-runner", "vm_type": "bionic-test"}))
                        .send()
                        .await
                        .expect("Failed to create VM");

                    let new_vm: serde_json::Value = create_response
                        .json()
                        .await
                        .expect("Failed to parse create VM response");

                    let vm_id = new_vm["id"].as_str().unwrap().to_string();
                    println!("✅ Created new VM: {}", vm_id);

                    // Wait for VM to boot
                    println!("⏳ Waiting 10 seconds for VM to boot...");
                    tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
                    vm_id
                }
            } else {
                // Create new VM
                println!("Creating new VM...");
                let create_response = client
                    .post("http://127.0.0.1:8080/api/vms")
                    .bearer_auth(&jwt_token)
                    .json(&serde_json::json!({"name": "test-runner", "vm_type": "bionic-test"}))
                    .send()
                    .await
                    .expect("Failed to create VM");

                let new_vm: serde_json::Value = create_response
                    .json()
                    .await
                    .expect("Failed to parse create VM response");

                let vm_id = new_vm["id"].as_str().unwrap().to_string();
                println!("✅ Created new VM: {}", vm_id);

                // Wait for VM to boot
                println!("⏳ Waiting 10 seconds for VM to boot...");
                tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
                vm_id
            }
        } else {
            panic!("No VMs found and failed to create new VM");
        }
    };

    // Step 3: Create workflow executor with REAL Firecracker VM
    println!("\n🔧 Step 3: Creating WorkflowExecutor with REAL Firecracker VM...");
    let executor = Arc::new(VmCommandExecutor::with_auth(
        "http://127.0.0.1:8080",
        jwt_token,
    ));
    let config = WorkflowExecutorConfig::default();

    // Create session manager with mock provider
    let session_config = SessionManagerConfig::default();
    let session_manager = Arc::new(SessionManager::new(session_config));

    let _workflow_executor =
        WorkflowExecutor::with_executor(executor.clone(), session_manager.clone(), config);
    println!("✅ WorkflowExecutor created with real Firecracker VM");

    // Create a manual session with the real VM ID for testing
    use terraphim_github_runner::session::{Session, SessionState};

    let session_id = SessionId::new();
    let test_session = Session {
        id: session_id.clone(),
        vm_id: vm_id.clone(),
        vm_type: "bionic-test".to_string(),
        started_at: Utc::now(),
        state: SessionState::Executing,
        snapshots: vec![],
        last_activity: Utc::now(),
    };

    // Step 4: Create workflow and context
    println!("\n📝 Step 4: Creating workflow context...");
    let event = create_test_event();
    let context = WorkflowContext::new(event);

    // Create a simple workflow for testing
    let workflow = ParsedWorkflow {
        name: "Firecracker Test Workflow".to_string(),
        trigger: "push".to_string(),
        environment: Default::default(),
        setup_commands: vec![],
        steps: vec![
            WorkflowStep {
                name: "Echo Test".to_string(),
                command: "echo 'Hello from Firecracker VM'".to_string(),
                working_dir: "/workspace".to_string(),
                continue_on_error: false,
                timeout_seconds: 5,
            },
            WorkflowStep {
                name: "List Root".to_string(),
                command: "ls -la /".to_string(),
                working_dir: "/".to_string(),
                continue_on_error: false,
                timeout_seconds: 5,
            },
            WorkflowStep {
                name: "Check Username".to_string(),
                command: "whoami".to_string(),
                working_dir: "/".to_string(),
                continue_on_error: false,
                timeout_seconds: 5,
            },
        ],
        cleanup_commands: vec![],
        cache_paths: vec![],
    };

    println!("✅ Workflow created with {} steps", workflow.steps.len());

    // Step 5: Execute commands directly using real VM session
    println!("\n▶️  Step 5: Executing commands in REAL Firecracker VM...");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    let mut all_success = true;
    let mut executed_count = 0;

    for (i, step) in workflow.steps.iter().enumerate() {
        println!("\n📤 Step {}: {}", i + 1, step.name);
        println!("   Command: {}", step.command);
        println!("   Working Dir: {}", step.working_dir);

        let timeout = std::time::Duration::from_secs(step.timeout_seconds);

        match executor
            .execute(&test_session, &step.command, timeout, &step.working_dir)
            .await
        {
            Ok(result) => {
                let success = result.exit_code == 0;
                if success {
                    println!("   ✅ Exit Code: {}", result.exit_code);
                    if !result.stdout.is_empty() {
                        println!("   stdout:");
                        for line in result.stdout.lines().take(5) {
                            println!("      {}", line);
                        }
                        if result.stdout.lines().count() > 5 {
                            println!("      ... ({} lines total)", result.stdout.lines().count());
                        }
                    }
                    if !result.stderr.is_empty() && result.stderr.lines().count() < 5 {
                        println!("   stderr: {}", result.stderr.trim());
                    }

                    // Record success in learning coordinator
                    let _ = coordinator
                        .record_success(&step.command, result.duration.as_millis() as u64, &context)
                        .await;

                    executed_count += 1;
                } else {
                    println!("   ❌ Exit Code: {}", result.exit_code);
                    if !result.stderr.is_empty() {
                        println!("   stderr: {}", result.stderr.trim());
                    }
                    all_success = false;

                    // Record failure in learning coordinator
                    let _ = coordinator
                        .record_failure(&step.command, &result.stderr, &context)
                        .await;

                    if !step.continue_on_error {
                        break;
                    }
                }
            }
            Err(e) => {
                println!("   ❌ Error: {}", e);
                all_success = false;

                // Record failure in learning coordinator
                let _ = coordinator
                    .record_failure(&step.command, &e.to_string(), &context)
                    .await;

                if !step.continue_on_error {
                    break;
                }
            }
        }
    }

    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("\n✅ Command execution completed:");
    println!("   - Success: {}", all_success);
    println!(
        "   - Commands executed: {}/{}",
        executed_count,
        workflow.steps.len()
    );

    // Verify the test expectations
    assert!(all_success, "All commands should execute successfully");
    assert_eq!(
        executed_count,
        workflow.steps.len(),
        "Should execute all {} commands",
        workflow.steps.len()
    );

    // Step 6: Verify learning coordinator stats
    println!("\n📈 Step 6: Verifying LearningCoordinator Statistics...");
    let learning_stats = coordinator.get_stats();
    println!("✅ Learning Coordinator Statistics:");
    println!("   - Total successes: {}", learning_stats.total_successes);
    println!("   - Total failures: {}", learning_stats.total_failures);
    println!(
        "   - Unique success patterns: {}",
        learning_stats.unique_success_patterns
    );
    println!(
        "   - Unique failure patterns: {}",
        learning_stats.unique_failure_patterns
    );
    println!("   - Lessons created: {}", learning_stats.lessons_created);

    assert!(
        learning_stats.total_successes >= 3,
        "Should have recorded at least 3 successful executions"
    );

    println!("\n=== END-TO-END TEST WITH REAL FIRECRACKER VM PASSED ===\n");
    println!("✅ GitHub hook integration verified:");
    println!("   ✅ Commands execute in real Firecracker VM sandbox");
    println!("   ✅ LearningCoordinator records execution results");
    println!("   ✅ Real VM output captured and returned");
}

/// Main function to run tests manually
fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Running end-to-end test for GitHub runner with real Firecracker VM...\n");
    println!(
        "Use: cargo test -p terraphim_github_runner end_to_end_real_firecracker_vm -- --ignored --nocapture"
    );
    Ok(())
}
