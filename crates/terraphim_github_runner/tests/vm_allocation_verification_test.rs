//! Phase 5 Verification: System-Level VM Allocation Testing
//!
//! This test suite empirically verifies that VM allocation happens at the workflow level,
//! not per step. All tests track VM allocations and verify session-VM binding.

#![allow(dead_code)]

use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;
use terraphim_github_runner::error::Result;
use terraphim_github_runner::models::{
    GitHubEvent, GitHubEventType, RepositoryInfo, WorkflowContext,
};
use terraphim_github_runner::session::{SessionManager, SessionManagerConfig, VmProvider};
use terraphim_github_runner::workflow::executor::{
    CommandExecutor, CommandResult, WorkflowExecutor, WorkflowExecutorConfig,
};
use terraphim_github_runner::workflow::parser::{ParsedWorkflow, WorkflowStep};
use tokio::sync::Mutex;
use uuid::Uuid;

/// Tracking information for allocated VMs
#[derive(Debug, Clone)]
struct VmInfo {
    allocation_order: usize,
    vm_type: String,
    allocated_at: chrono::DateTime<chrono::Utc>,
}

/// Test VM provider that tracks all allocations
struct TestVmProvider {
    allocation_count: Arc<AtomicUsize>,
    allocated_vms: Arc<Mutex<HashMap<String, VmInfo>>>,
    release_count: Arc<AtomicUsize>,
    released_vms: Arc<Mutex<Vec<String>>>,
}

impl TestVmProvider {
    fn new() -> Self {
        Self {
            allocation_count: Arc::new(AtomicUsize::new(0)),
            allocated_vms: Arc::new(Mutex::new(HashMap::new())),
            release_count: Arc::new(AtomicUsize::new(0)),
            released_vms: Arc::new(Mutex::new(Vec::new())),
        }
    }

    async fn get_allocation_count(&self) -> usize {
        self.allocation_count.load(Ordering::SeqCst)
    }

    async fn get_release_count(&self) -> usize {
        self.release_count.load(Ordering::SeqCst)
    }

    async fn get_allocated_vms(&self) -> HashMap<String, VmInfo> {
        self.allocated_vms.lock().await.clone()
    }

    async fn get_released_vms(&self) -> Vec<String> {
        self.released_vms.lock().await.clone()
    }

    async fn was_vm_released(&self, vm_id: &str) -> bool {
        self.released_vms.lock().await.contains(&vm_id.to_string())
    }

    fn reset(&self) {
        self.allocation_count.store(0, Ordering::SeqCst);
        self.release_count.store(0, Ordering::SeqCst);
        // Note: We don't clear the maps to preserve history
    }
}

#[async_trait]
impl VmProvider for TestVmProvider {
    async fn allocate(&self, vm_type: &str) -> Result<(String, Duration)> {
        let count = self.allocation_count.fetch_add(1, Ordering::SeqCst);
        let vm_id = format!("test-vm-{}-{}", count, Uuid::new_v4());

        let info = VmInfo {
            allocation_order: count,
            vm_type: vm_type.to_string(),
            allocated_at: chrono::Utc::now(),
        };

        self.allocated_vms.lock().await.insert(vm_id.clone(), info);

        log::info!(
            "[TEST VM PROVIDER] Allocated VM {} (allocation #{})",
            vm_id,
            count
        );

        Ok((vm_id, Duration::from_millis(50)))
    }

    async fn release(&self, vm_id: &str) -> Result<()> {
        let count = self.release_count.fetch_add(1, Ordering::SeqCst);
        self.released_vms.lock().await.push(vm_id.to_string());

        log::info!(
            "[TEST VM PROVIDER] Released VM {} (release #{})",
            vm_id,
            count
        );

        Ok(())
    }
}

/// Tracking command executor that records which VM executes each command
struct TrackingCommandExecutor {
    execution_log: Arc<Mutex<Vec<ExecutionRecord>>>,
}

#[derive(Debug, Clone)]
struct ExecutionRecord {
    vm_id: String,
    session_id: String,
    step_name: String,
    command: String,
    timestamp: chrono::DateTime<chrono::Utc>,
}

impl TrackingCommandExecutor {
    fn new() -> Self {
        Self {
            execution_log: Arc::new(Mutex::new(Vec::new())),
        }
    }

    async fn get_execution_log(&self) -> Vec<ExecutionRecord> {
        self.execution_log.lock().await.clone()
    }

    async fn count_executions_for_vm(&self, vm_id: &str) -> usize {
        self.execution_log
            .lock()
            .await
            .iter()
            .filter(|r| r.vm_id == vm_id)
            .count()
    }

    async fn get_unique_vms_used(&self) -> Vec<String> {
        let mut vms = self
            .execution_log
            .lock()
            .await
            .iter()
            .map(|r| r.vm_id.clone())
            .collect::<Vec<_>>();
        vms.sort();
        vms.dedup();
        vms
    }
}

#[async_trait]
impl CommandExecutor for TrackingCommandExecutor {
    async fn execute(
        &self,
        session: &terraphim_github_runner::session::Session,
        command: &str,
        _timeout: Duration,
        _working_dir: &str,
    ) -> Result<CommandResult> {
        let record = ExecutionRecord {
            vm_id: session.vm_id.clone(),
            session_id: session.id.to_string(),
            step_name: "step".to_string(), // Will be updated by test wrapper
            command: command.to_string(),
            timestamp: chrono::Utc::now(),
        };

        self.execution_log.lock().await.push(record);

        Ok(CommandResult {
            exit_code: 0,
            stdout: format!("Executed in VM {}", session.vm_id),
            stderr: String::new(),
            duration: Duration::from_millis(10),
        })
    }

    async fn create_snapshot(
        &self,
        _session: &terraphim_github_runner::session::Session,
        name: &str,
    ) -> Result<terraphim_github_runner::models::SnapshotId> {
        Ok(terraphim_github_runner::models::SnapshotId::new(format!(
            "snap-{}",
            name
        )))
    }

    async fn rollback(
        &self,
        _session: &terraphim_github_runner::session::Session,
        _snapshot_id: &terraphim_github_runner::models::SnapshotId,
    ) -> Result<()> {
        Ok(())
    }
}

/// Helper to create test workflows
fn create_test_workflow(name: &str, step_count: usize) -> ParsedWorkflow {
    let steps: Vec<WorkflowStep> = (0..step_count)
        .map(|i| WorkflowStep {
            name: format!("Step {}", i + 1),
            command: format!("echo 'Step {}'", i + 1),
            working_dir: "/workspace".to_string(),
            continue_on_error: false,
            timeout_seconds: 300,
        })
        .collect();

    ParsedWorkflow {
        name: name.to_string(),
        trigger: "push".to_string(),
        environment: HashMap::new(),
        setup_commands: vec![format!("echo 'Setup for {}'", name)],
        steps,
        cleanup_commands: vec![format!("echo 'Cleanup for {}'", name)],
        cache_paths: vec![],
    }
}

/// Helper to create test GitHub event
fn create_test_event() -> GitHubEvent {
    GitHubEvent {
        event_type: GitHubEventType::Push,
        action: Some("opened".to_string()),
        repository: RepositoryInfo {
            full_name: "test/repo".to_string(),
            clone_url: None,
            default_branch: Some("main".to_string()),
        },
        pull_request: None,
        git_ref: None,
        sha: Some(Uuid::new_v4().to_string()),
        extra: HashMap::new(),
    }
}

/// Helper to create workflow context
fn create_test_context() -> WorkflowContext {
    WorkflowContext::new(create_test_event())
}

/// =============================================================================
/// TEST 1: Single Workflow with Multiple Steps
/// =============================================================================
#[tokio::test]
async fn test_single_workflow_multiple_steps_one_vm() {
    // Setup: Initialize test providers with tracking
    let vm_provider = Arc::new(TestVmProvider::new());
    let command_executor = Arc::new(TrackingCommandExecutor::new());
    let session_manager = Arc::new(SessionManager::with_provider(
        vm_provider.clone(),
        SessionManagerConfig::default(),
    ));

    // Create workflow with 5 steps
    let workflow = create_test_workflow("test-workflow-1", 5);
    let context = create_test_context();

    // Execute workflow
    let executor = WorkflowExecutor::with_executor(
        command_executor.clone(),
        session_manager,
        WorkflowExecutorConfig::default(),
    );

    let result = executor
        .execute_workflow(&workflow, &context)
        .await
        .expect("Workflow should execute successfully");

    // Verify results
    assert!(result.success, "Workflow should succeed");
    assert_eq!(result.steps.len(), 5, "Should execute all 5 steps");

    // CRITICAL VERIFICATION: Exactly ONE VM allocated
    let allocation_count = vm_provider.get_allocation_count().await;
    assert_eq!(
        allocation_count, 1,
        "Expected 1 VM allocation for workflow with 5 steps, got {}",
        allocation_count
    );

    // Verify all steps used the same VM
    let unique_vms = command_executor.get_unique_vms_used().await;

    assert_eq!(
        unique_vms.len(),
        1,
        "All steps should use same VM, but found {} unique VMs",
        unique_vms.len()
    );

    // Verify execution count
    let vm_id = &unique_vms[0];
    let executions_in_vm = command_executor.count_executions_for_vm(vm_id).await;

    // Should have: 1 setup + 5 steps + 1 cleanup = 7 executions
    assert_eq!(
        executions_in_vm, 7,
        "Expected 7 command executions (1 setup + 5 steps + 1 cleanup) in VM {}, got {}",
        vm_id, executions_in_vm
    );

    // Verify VM was allocated for the correct session
    let allocated_vms = vm_provider.get_allocated_vms().await;
    assert_eq!(
        allocated_vms.len(),
        1,
        "Should have 1 VM in allocation history"
    );

    let (allocated_vm_id, _) = allocated_vms.iter().next().unwrap();
    assert_eq!(
        allocated_vm_id, vm_id,
        "Allocated VM ID should match execution VM ID"
    );

    log::info!(
        "✅ TEST 1 PASSED: 1 workflow with 5 steps allocated exactly 1 VM ({})",
        vm_id
    );
}

/// =============================================================================
/// TEST 2: Multiple Workflows in Parallel
/// =============================================================================
#[tokio::test]
async fn test_multiple_workflows_multiple_vms() {
    // Setup: Initialize test providers with tracking
    let vm_provider = Arc::new(TestVmProvider::new());
    let session_manager = Arc::new(SessionManager::with_provider(
        vm_provider.clone(),
        SessionManagerConfig::default(),
    ));

    // Create 3 workflows with different step counts
    let workflow_a = create_test_workflow("workflow-a", 5);
    let workflow_b = create_test_workflow("workflow-b", 3);
    let workflow_c = create_test_workflow("workflow-c", 7);

    let context_a = create_test_context();
    let context_b = create_test_context();
    let context_c = create_test_context();

    // Execute workflows in parallel
    let command_executor_a = Arc::new(TrackingCommandExecutor::new());
    let command_executor_b = Arc::new(TrackingCommandExecutor::new());
    let command_executor_c = Arc::new(TrackingCommandExecutor::new());

    // Keep references for verification after execution
    let executor_a_ref = command_executor_a.clone();
    let executor_b_ref = command_executor_b.clone();
    let executor_c_ref = command_executor_c.clone();

    let executor_a = WorkflowExecutor::with_executor(
        command_executor_a,
        session_manager.clone(),
        WorkflowExecutorConfig::default(),
    );
    let executor_b = WorkflowExecutor::with_executor(
        command_executor_b,
        session_manager.clone(),
        WorkflowExecutorConfig::default(),
    );
    let executor_c = WorkflowExecutor::with_executor(
        command_executor_c,
        session_manager,
        WorkflowExecutorConfig::default(),
    );

    // Run workflows in parallel
    let (result_a, result_b, result_c) = tokio::join!(
        executor_a.execute_workflow(&workflow_a, &context_a),
        executor_b.execute_workflow(&workflow_b, &context_b),
        executor_c.execute_workflow(&workflow_c, &context_c)
    );

    // Verify all workflows succeeded
    assert!(result_a.unwrap().success, "Workflow A should succeed");
    assert!(result_b.unwrap().success, "Workflow B should succeed");
    assert!(result_c.unwrap().success, "Workflow C should succeed");

    // CRITICAL VERIFICATION: Exactly THREE VMs allocated (one per workflow)
    let allocation_count = vm_provider.get_allocation_count().await;
    assert_eq!(
        allocation_count, 3,
        "Expected 3 VM allocations for 3 parallel workflows, got {}",
        allocation_count
    );

    // Verify we have 3 unique VMs
    let _allocated_vms = vm_provider.get_allocated_vms().await;
    assert_eq!(
        _allocated_vms.len(),
        3,
        "Should have 3 unique VMs in allocation history"
    );

    // Verify each workflow used a unique VM
    let vms_a = executor_a_ref.get_unique_vms_used().await;
    let vms_b = executor_b_ref.get_unique_vms_used().await;
    let vms_c = executor_c_ref.get_unique_vms_used().await;

    assert_eq!(vms_a.len(), 1, "Workflow A should use 1 VM");
    assert_eq!(vms_b.len(), 1, "Workflow B should use 1 VM");
    assert_eq!(vms_c.len(), 1, "Workflow C should use 1 VM");

    // Verify VMs are different across workflows
    let vm_a = &vms_a[0];
    let vm_b = &vms_b[0];
    let vm_c = &vms_c[0];

    assert_ne!(vm_a, vm_b, "Workflow A and B should use different VMs");
    assert_ne!(vm_b, vm_c, "Workflow B and C should use different VMs");
    assert_ne!(vm_a, vm_c, "Workflow A and C should use different VMs");

    log::info!(
        "✅ TEST 2 PASSED: 3 parallel workflows allocated 3 unique VMs: A={}, B={}, C={}",
        vm_a,
        vm_b,
        vm_c
    );
}

/// =============================================================================
/// TEST 3: VM Reuse After Workflow Completion
/// =============================================================================
#[tokio::test]
async fn test_vm_reuse_after_completion() {
    // Setup: Initialize test providers with tracking
    let vm_provider = Arc::new(TestVmProvider::new());
    let command_executor = Arc::new(TrackingCommandExecutor::new());
    let session_manager = Arc::new(SessionManager::with_provider(
        vm_provider.clone(),
        SessionManagerConfig::default(),
    ));

    // Execute workflow 1
    let workflow_1 = create_test_workflow("workflow-1", 2);
    let context_1 = create_test_context();

    let executor = WorkflowExecutor::with_executor(
        command_executor.clone(),
        session_manager.clone(),
        WorkflowExecutorConfig::default(),
    );

    let result_1 = executor
        .execute_workflow(&workflow_1, &context_1)
        .await
        .expect("Workflow 1 should execute");

    assert!(result_1.success);
    let allocation_count_after_1 = vm_provider.get_allocation_count().await;
    assert_eq!(
        allocation_count_after_1, 1,
        "Should allocate 1 VM for workflow 1"
    );

    // Get the VM ID from first workflow
    let vms_after_1 = command_executor.get_unique_vms_used().await;
    assert_eq!(vms_after_1.len(), 1, "Should have 1 VM after workflow 1");
    let vm_id_1 = vms_after_1[0].clone();

    // Release the session
    session_manager
        .release_session(&result_1.session_id)
        .await
        .expect("Should release session 1");

    let release_count_after_1 = vm_provider.get_release_count().await;
    assert_eq!(
        release_count_after_1, 1,
        "Should release 1 VM after workflow 1"
    );

    // Verify VM was released
    assert!(
        vm_provider.was_vm_released(&vm_id_1).await,
        "VM {} should be marked as released",
        vm_id_1
    );

    // Execute workflow 2 (should potentially reuse or allocate new VM)
    let workflow_2 = create_test_workflow("workflow-2", 2);
    let context_2 = create_test_context();

    let result_2 = executor
        .execute_workflow(&workflow_2, &context_2)
        .await
        .expect("Workflow 2 should execute");

    assert!(result_2.success);

    // Note: Current implementation doesn't reuse VM IDs, but we verify proper lifecycle
    let allocation_count_after_2 = vm_provider.get_allocation_count().await;
    assert_eq!(
        allocation_count_after_2, 2,
        "Should allocate new VM for workflow 2"
    );

    // Verify both VMs are in allocation history
    let allocated_vms = vm_provider.get_allocated_vms().await;
    assert_eq!(allocated_vms.len(), 2, "Should have 2 VMs in history");

    // Verify first VM was released
    let released_vms = vm_provider.get_released_vms().await;
    assert_eq!(released_vms.len(), 1, "Should have 1 released VM");

    log::info!("✅ TEST 3 PASSED: VMs properly released and new allocations work correctly");
}

/// =============================================================================
/// TEST 4: Concurrent Workflow Limit Enforcement
/// =============================================================================
#[tokio::test]
async fn test_concurrent_workflow_limit() {
    // Setup: Session manager with max 2 concurrent sessions
    let vm_provider = Arc::new(TestVmProvider::new());
    let config = SessionManagerConfig {
        max_concurrent_sessions: 2,
        ..Default::default()
    };
    let session_manager = Arc::new(SessionManager::with_provider(vm_provider.clone(), config));

    // Try to create 3 sessions concurrently
    let context_1 = create_test_context();
    let context_2 = create_test_context();
    let context_3 = create_test_context();

    // Create first 2 sessions (should succeed)
    let session_1 = session_manager.create_session(&context_1).await;
    let session_2 = session_manager.create_session(&context_2).await;

    assert!(session_1.is_ok(), "First session should be created");
    assert!(session_2.is_ok(), "Second session should be created");

    // Try to create third session (should fail)
    let session_3 = session_manager.create_session(&context_3).await;
    assert!(session_3.is_err(), "Third session should be rejected");

    // Verify only 2 VMs were allocated
    let allocation_count = vm_provider.get_allocation_count().await;
    assert_eq!(
        allocation_count, 2,
        "Should only allocate 2 VMs due to concurrent limit"
    );

    // Release one session
    let session_1 = session_1.unwrap();
    session_manager
        .release_session(&session_1.id)
        .await
        .expect("Should release session 1");

    // Now third session should succeed
    let context_4 = create_test_context();
    let session_4 = session_manager.create_session(&context_4).await;
    assert!(
        session_4.is_ok(),
        "Fourth session should succeed after release"
    );

    // Verify still only 3 VMs total (2 original + 1 new)
    let allocation_count_final = vm_provider.get_allocation_count().await;
    assert_eq!(
        allocation_count_final, 3,
        "Should have 3 total allocations (2 original + 1 after release)"
    );

    log::info!("✅ TEST 4 PASSED: Concurrent session limit enforced correctly (max 2)");
}

/// =============================================================================
/// TEST 5: Step Execution VM Consistency
/// =============================================================================
#[tokio::test]
async fn test_step_execution_vm_consistency() {
    // This test verifies that within a single workflow, every step
    // executes in the exact same VM instance

    let vm_provider = Arc::new(TestVmProvider::new());
    let command_executor = Arc::new(TrackingCommandExecutor::new());
    let session_manager = Arc::new(SessionManager::with_provider(
        vm_provider.clone(),
        SessionManagerConfig::default(),
    ));

    // Create workflow with 10 steps
    let workflow = create_test_workflow("consistency-test", 10);
    let context = create_test_context();

    let executor = WorkflowExecutor::with_executor(
        command_executor.clone(),
        session_manager,
        WorkflowExecutorConfig::default(),
    );

    let result = executor
        .execute_workflow(&workflow, &context)
        .await
        .expect("Workflow should execute");

    assert!(result.success);
    assert_eq!(result.steps.len(), 10, "Should execute all 10 steps");

    // Get the unique VM used
    let unique_vms = command_executor.get_unique_vms_used().await;
    assert_eq!(unique_vms.len(), 1, "Should use exactly 1 VM");

    let vm_id = &unique_vms[0];

    // Verify all executions happened in this VM
    let execution_log = command_executor.get_execution_log().await;

    // Total executions: 1 setup + 10 steps + 1 cleanup = 12
    assert_eq!(execution_log.len(), 12, "Should have 12 total executions");

    // Verify every execution used the same VM
    for (i, record) in execution_log.iter().enumerate() {
        assert_eq!(
            &record.vm_id, vm_id,
            "Execution {} should use VM {}, but used {}",
            i, vm_id, record.vm_id
        );
    }

    // Verify VM was allocated exactly once
    let allocation_count = vm_provider.get_allocation_count().await;
    assert_eq!(allocation_count, 1, "Should allocate VM exactly once");

    log::info!(
        "✅ TEST 5 PASSED: All 12 executions (setup + 10 steps + cleanup) used same VM {}",
        vm_id
    );
}
