//! Firecracker VM execution

use crate::{RunnerResult, RunnerError};
use ahash::AHashMap;

/// Result from VM execution
pub struct VmExecutionResult {
    /// Exit code
    pub exit_code: i32,
    /// Standard output
    pub stdout: String,
    /// Standard error
    pub stderr: String,
    /// Captured outputs
    pub outputs: AHashMap<String, String>,
    /// VM snapshot ID (if captured)
    pub snapshot_id: Option<String>,
}

/// Executor for running commands in Firecracker VMs
pub struct VmExecutor {
    /// VM pool size
    pool_size: usize,
    /// Available VMs
    available_vms: tokio::sync::Mutex<Vec<String>>,
}

impl VmExecutor {
    /// Create a new VM executor
    pub fn new(pool_size: usize) -> Self {
        Self {
            pool_size,
            available_vms: tokio::sync::Mutex::new(Vec::new()),
        }
    }

    /// Initialize the VM pool
    pub async fn initialize_pool(&self) -> RunnerResult<()> {
        log::info!("Initializing VM pool with {} VMs", self.pool_size);

        let mut vms = self.available_vms.lock().await;
        for i in 0..self.pool_size {
            let vm_id = format!("vm-{}", i);
            // In real implementation, would boot Firecracker VMs
            vms.push(vm_id);
        }

        Ok(())
    }

    /// Execute commands in a VM
    pub async fn execute(
        &self,
        commands: &[String],
        env: &AHashMap<String, String>,
        timeout_minutes: Option<u32>,
    ) -> RunnerResult<VmExecutionResult> {
        // Acquire a VM from pool
        let vm_id = self.acquire_vm().await?;

        log::debug!("Executing {} commands in VM {}", commands.len(), vm_id);

        // Set up execution context
        let timeout = timeout_minutes.unwrap_or(360) as u64 * 60 * 1000; // Convert to ms

        // Execute commands
        let result = self.execute_in_vm(&vm_id, commands, env, timeout).await;

        // Release VM back to pool
        self.release_vm(vm_id).await;

        result
    }

    /// Acquire a VM from the pool
    async fn acquire_vm(&self) -> RunnerResult<String> {
        let mut vms = self.available_vms.lock().await;

        if let Some(vm_id) = vms.pop() {
            Ok(vm_id)
        } else {
            // No VMs available, create one on demand
            let vm_id = format!("vm-dynamic-{}", uuid::Uuid::new_v4());
            log::info!("No VMs in pool, creating on-demand: {}", vm_id);
            Ok(vm_id)
        }
    }

    /// Release a VM back to the pool
    async fn release_vm(&self, vm_id: String) {
        let mut vms = self.available_vms.lock().await;

        // Only return to pool if we haven't exceeded pool size
        if vms.len() < self.pool_size {
            vms.push(vm_id);
        } else {
            log::debug!("Pool full, discarding VM: {}", vm_id);
            // In real implementation, would destroy the VM
        }
    }

    /// Execute commands in a specific VM
    async fn execute_in_vm(
        &self,
        vm_id: &str,
        commands: &[String],
        env: &AHashMap<String, String>,
        timeout_ms: u64,
    ) -> RunnerResult<VmExecutionResult> {
        // In a real implementation, this would:
        // 1. Connect to Firecracker VM via vsock or API
        // 2. Set up environment variables
        // 3. Execute each command
        // 4. Capture stdout/stderr
        // 5. Extract outputs (GITHUB_OUTPUT format)
        // 6. Optionally snapshot the VM

        log::debug!("VM {} executing with timeout {}ms", vm_id, timeout_ms);
        log::debug!("Environment: {:?}", env.keys().collect::<Vec<_>>());

        // Placeholder implementation
        let mut stdout = String::new();
        let mut stderr = String::new();
        let mut exit_code = 0;

        for (idx, cmd) in commands.iter().enumerate() {
            log::debug!("Command {}: {}", idx, cmd);

            // Simulate execution
            if cmd.contains("Unknown") || cmd.contains("# ") {
                // Comment or unknown command
                stdout.push_str(&format!("# Skipped: {}\n", cmd));
            } else {
                // "Execute" the command
                stdout.push_str(&format!("Executed: {}\n", cmd));

                // Check for known failure patterns
                if cmd.contains("exit 1") || cmd.contains("false") {
                    exit_code = 1;
                    stderr.push_str(&format!("Command failed: {}\n", cmd));
                    break;
                }
            }
        }

        Ok(VmExecutionResult {
            exit_code,
            stdout,
            stderr,
            outputs: AHashMap::new(),
            snapshot_id: None,
        })
    }

    /// Snapshot a VM state
    pub async fn snapshot_vm(&self, vm_id: &str) -> RunnerResult<String> {
        let snapshot_id = format!("snapshot-{}-{}", vm_id, uuid::Uuid::new_v4());
        log::info!("Creating snapshot {} for VM {}", snapshot_id, vm_id);
        // In real implementation, would use Firecracker snapshot API
        Ok(snapshot_id)
    }

    /// Restore a VM from snapshot
    pub async fn restore_vm(&self, snapshot_id: &str) -> RunnerResult<String> {
        let vm_id = format!("vm-restored-{}", uuid::Uuid::new_v4());
        log::info!("Restoring VM {} from snapshot {}", vm_id, snapshot_id);
        // In real implementation, would use Firecracker restore API
        Ok(vm_id)
    }

    /// Shutdown all VMs
    pub async fn shutdown(&self) -> RunnerResult<()> {
        let mut vms = self.available_vms.lock().await;
        log::info!("Shutting down {} VMs", vms.len());
        vms.clear();
        Ok(())
    }

    /// Get pool status
    pub async fn pool_status(&self) -> (usize, usize) {
        let vms = self.available_vms.lock().await;
        (vms.len(), self.pool_size)
    }
}
