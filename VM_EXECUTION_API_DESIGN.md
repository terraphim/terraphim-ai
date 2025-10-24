# VM Execution API Design - Comprehensive Architecture

## Overview

This document outlines the complete VM execution API design for terraphim-ai, leveraging the existing firecracker-rust infrastructure to provide secure, isolated code execution with sub-2-second response times.

## Architecture Components

### 1. Core Infrastructure (Existing)

#### VmManager (`firecracker-rust/fcctl-core/src/vm/manager.rs`)
- **Purpose**: Low-level VM lifecycle management
- **Key Methods**:
  - `create_vm(config, domain)` → Result<String>
  - `start_vm(vm_id)` → Result<()>
  - `stop_vm(vm_id)` → Result<()>
  - `get_vm_status(vm_id)` → Result<VmState>

#### VmPoolManager (`firecracker-rust/fcctl-web/src/vm_pool/mod.rs`)
- **Purpose**: IP allocation and VM-IP mapping
- **IP Range**: 172.26.0.2 - 172.26.0.254 (253 IPs)
- **Key Methods**:
  - `allocate_ip(vm_id)` → Result<String>
  - `deallocate_ip(vm_id)` → Result<()>
  - `get_vm_ip(vm_id)` → Option<String>
  - `restore_from_database()` → Result<()>

#### Session Management (`firecracker-rust/fcctl-repl/src/session.rs`)
- **Purpose**: Command execution with snapshot/rollback
- **Key Methods**:
  - `execute_command(command, timeout)` → Result<ExecutionResult>
  - `create_snapshot(name)` → Result<String>
  - `rollback_to_snapshot(snapshot_id)` → Result<()>

### 2. API Layer (Existing)

#### LLM Execution API (`firecracker-rust/fcctl-web/src/api/llm.rs`)
- **Endpoint**: `POST /api/llm/execute`
- **Request**: `LlmExecuteRequest { code, language, agent_id, vm_id?, timeout_ms? }`
- **Response**: `LlmExecuteResponse { execution_id, vm_id, exit_code, stdout, stderr, duration_ms, timestamps }`

#### VM Pool API (`firecracker-rust/fcctl-web/src/api/vm_pool.rs`)
- **Endpoints**:
  - `GET /api/vm-pool/stats` → Pool statistics
  - `GET /api/vm-pool/list` → VMs with IPs
  - `POST /api/vm-pool/allocate` → Allocate IP for VM

### 3. Enhanced VM Execution Flow Design

#### 3.1 VM Pool Management System

**Pre-warmed VM Pool Strategy**:
```rust
pub struct PrewarmedVmPool {
    // Always-available VMs for instant execution
    prewarmed_vms: Arc<RwLock<VecDeque<PrewarmedVm>>>,
    // VMs currently in use
    active_vms: Arc<RwLock<HashMap<String, ActiveVm>>>,
    // VMs being prepared (warming up)
    warming_vms: Arc<RwLock<HashMap<String, WarmingVm>>>,
}

pub struct PrewarmedVm {
    pub vm_id: String,
    pub ip_address: String,
    pub created_at: DateTime<Utc>,
    pub last_used: Option<DateTime<Utc>>,
    pub readiness_score: f32, // Based on boot time and resource usage
}

pub struct ActiveVm {
    pub vm_id: String,
    pub ip_address: String,
    pub agent_id: String,
    pub execution_count: u32,
    pub last_execution: DateTime<Utc>,
    pub current_snapshot: String, // Base snapshot for rollback
}
```

**Pool Management Algorithm**:
1. **Target Pool Size**: 10 prewarmed VMs (configurable)
2. **Warm-up Strategy**: Maintain 8-12 prewarmed VMs
3. **VM Lifecycle**: 
   - New → Warming (30-45 seconds) → Prewarmed → Active → Cleanup
4. **Resource Recycling**: VMs used >50 executions or >2 hours get recycled

#### 3.2 Fast Execution Flow (<2 seconds)

**Request Processing Pipeline**:
```rust
pub async fn execute_code_fast(request: LlmExecuteRequest) -> Result<LlmExecuteResponse> {
    // Phase 1: Validation (10ms)
    validate_request(&request)?;
    
    // Phase 2: VM Allocation (50ms)
    let vm = allocate_or_reuse_vm(&request.agent_id).await?;
    
    // Phase 3: Snapshot Creation (100ms)
    let snapshot_id = create_execution_snapshot(&vm.vm_id).await?;
    
    // Phase 4: Code Execution (variable, typically 500-1500ms)
    let execution_result = execute_in_vm(&vm, &request).await?;
    
    // Phase 5: Result Processing (50ms)
    let response = build_response(execution_result, snapshot_id).await?;
    
    // Phase 6: Async Cleanup (background)
    tokio::spawn(async move {
        cleanup_after_execution(vm.vm_id, snapshot_id).await;
    });
    
    Ok(response)
}
```

**VM Allocation Strategy**:
```rust
async fn allocate_or_reuse_vm(agent_id: &str) -> Result<AllocatedVm> {
    // 1. Check if agent has active VM with good performance
    if let Some(active_vm) = get_agent_active_vm(agent_id).await? {
        if active_vm.execution_count < 30 && active_vm.performance_score > 0.8 {
            return Ok(allocate_existing_vm(active_vm).await?);
        }
    }
    
    // 2. Get best prewarmed VM (lowest readiness_score)
    if let Some(prewarmed) = get_best_prewarmed_vm().await? {
        return Ok(allocate_prewarmed_vm(prewarmed, agent_id).await?);
    }
    
    // 3. Wait for warming VM (up to 5 seconds)
    if let Some(warming) = wait_for_warming_vm().await? {
        return Ok(allocate_warming_vm(warming, agent_id).await?);
    }
    
    // 4. Create new VM (fallback, slower path)
    create_and_warm_vm_for_agent(agent_id).await
}
```

#### 3.3 Snapshot and Rollback System

**Execution Snapshots**:
```rust
pub struct ExecutionSnapshot {
    pub snapshot_id: String,
    pub vm_id: String,
    pub agent_id: String,
    pub execution_id: String,
    pub created_at: DateTime<Utc>,
    pub snapshot_type: SnapshotType,
    pub metadata: SnapshotMetadata,
}

pub enum SnapshotType {
    PreExecution,    // Before running code
    PostExecution,   // After successful execution
    Failure,         // On execution failure
    Manual,          // User-requested
}

pub struct SnapshotMetadata {
    pub file_system_changes: Vec<FileChange>,
    pub process_tree: Vec<ProcessInfo>,
    pub network_connections: Vec<NetworkConnection>,
    pub resource_usage: ResourceUsage,
}
```

**Smart Rollback Strategy**:
```rust
pub async fn smart_rollback(
    vm_id: &str, 
    execution_id: &str,
    rollback_strategy: RollbackStrategy
) -> Result<RollbackResult> {
    match rollback_strategy {
        RollbackStrategy::ToLastSuccess => {
            // Find last successful execution snapshot for this agent
            let last_success = find_last_success_snapshot(vm_id, execution_id).await?;
            rollback_to_snapshot(vm_id, &last_success.snapshot_id).await?;
        }
        RollbackStrategy::ToPreExecution => {
            // Rollback to snapshot before this execution
            let pre_exec = get_pre_execution_snapshot(execution_id).await?;
            rollback_to_snapshot(vm_id, &pre_exec.snapshot_id).await?;
        }
        RollbackStrategy::Selective(paths) => {
            // Rollback only specific files/directories
            selective_rollback(vm_id, paths).await?;
        }
    }
}
```

### 4. Performance Optimization

#### 4.1 Sub-2 Second Boot Time Optimization

**Techniques**:
1. **Kernel Caching**: Pre-loaded kernel in memory
2. **Root FS Optimization**: Minimal root filesystem with essential tools
3. **Memory Pre-allocation**: VM memory reserved in host
4. **CPU Pinning**: Dedicated CPU cores for VM pool
5. **Network Pre-configuration**: TAP interfaces pre-created

**Boot Time Breakdown**:
```
Total Target: <2000ms
├── VM Allocation: 50ms
├── Snapshot Creation: 100ms  
├── Code Transfer: 50ms
├── Execution Setup: 100ms
├── Code Execution: 1000-1500ms (variable)
├── Result Collection: 100ms
└── Response Building: 50ms
```

#### 4.2 Resource Management

**Memory Management**:
```rust
pub struct MemoryManager {
    // Total memory pool for VMs
    total_memory_gb: u32,
    // Memory per VM (default 2GB)
    memory_per_vm: u32,
    // Current memory usage
    used_memory: Arc<RwLock<u32>>,
    // Memory pressure threshold
    pressure_threshold: f32,
}

impl MemoryManager {
    pub async fn can_allocate_vm(&self) -> bool {
        let used = self.used_memory.read().await;
        *used < (self.total_memory_gb as f32 * (1.0 - self.pressure_threshold)) as u32
    }
    
    pub async fn allocate_memory(&self) -> Result<MemoryAllocation> {
        if !self.can_allocate_vm().await {
            return Err(Error::MemoryPressure);
        }
        // Allocate memory for new VM
    }
}
```

**CPU Management**:
```rust
pub struct CpuManager {
    // Available CPU cores
    available_cores: Vec<usize>,
    // Core to VM mapping
    core_vm_map: Arc<RwLock<HashMap<usize, String>>>,
    // CPU usage tracking
    cpu_usage: Arc<RwLock<HashMap<String, f32>>>,
}
```

### 5. Security Model

#### 5.1 VM Isolation

**Network Isolation**:
- Each VM in isolated network namespace
- Only outbound HTTPS allowed (port 443)
- No inbound connections from external networks
- DNS filtering to prevent exfiltration

**File System Isolation**:
- Read-only root filesystem
- Temporary writable overlay for execution
- No access to host filesystem
- Automatic cleanup on VM destruction

**Process Isolation**:
- No privileged processes allowed
- Resource limits (CPU, memory, disk)
- Process monitoring and termination
- No access to host system calls

#### 5.2 Code Validation

**Input Validation Pipeline**:
```rust
pub struct CodeValidator {
    // Language-specific validators
    validators: HashMap<String, Box<dyn LanguageValidator>>,
    // Security pattern detectors
    security_scanners: Vec<Box<dyn SecurityScanner>>,
    // Resource usage estimators
    resource_estimators: HashMap<String, Box<dyn ResourceEstimator>>,
}

impl CodeValidator {
    pub async fn validate(&self, code: &str, language: &str) -> Result<ValidationResult> {
        // 1. Language syntax validation
        self.validators.get(language)
            .ok_or(Error::UnsupportedLanguage)?
            .validate_syntax(code)?;
        
        // 2. Security pattern scanning
        for scanner in &self.security_scanners {
            scanner.scan_code(code, language)?;
        }
        
        // 3. Resource usage estimation
        let estimate = self.resource_estimators.get(language)
            .ok_or(Error::UnsupportedLanguage)?
            .estimate_resources(code)?;
        
        if estimate.memory_mb > 1024 || estimate.duration_secs > 30 {
            return Err(Error::ResourceLimitsExceeded);
        }
        
        Ok(ValidationResult { estimate, warnings })
    }
}
```

**Security Patterns Detected**:
- File system access outside sandbox
- Network connections to non-HTTPS endpoints
- System calls for privilege escalation
- Infinite loops and resource exhaustion
- Cryptocurrency mining patterns
- Data exfiltration attempts

### 6. Monitoring and Observability

#### 6.1 Metrics Collection

**Execution Metrics**:
```rust
pub struct ExecutionMetrics {
    pub execution_id: String,
    pub vm_id: String,
    pub agent_id: String,
    pub language: String,
    pub code_length: usize,
    pub duration_ms: u64,
    pub memory_peak_mb: u32,
    pub cpu_percent: f32,
    pub exit_code: i32,
    pub success: bool,
    pub error_type: Option<String>,
}
```

**VM Pool Metrics**:
```rust
pub struct VmPoolMetrics {
    pub total_vms: usize,
    pub prewarmed_vms: usize,
    pub active_vms: usize,
    pub warming_vms: usize,
    pub average_boot_time_ms: u64,
    pub pool_utilization_percent: f32,
    pub memory_utilization_percent: f32,
    pub cpu_utilization_percent: f32,
}
```

#### 6.2 Alerting

**Alert Conditions**:
- VM pool size < 5 prewarmed VMs
- Average execution time > 5 seconds
- Memory utilization > 80%
- VM failure rate > 10%
- Security pattern detection rate > 1%

### 7. API Endpoints Specification

#### 7.1 Core Execution Endpoints

**Execute Code**:
```
POST /api/v2/execute
Content-Type: application/json
Authorization: Bearer <token>

{
  "code": "print('Hello, World!')",
  "language": "python",
  "agent_id": "agent-123",
  "vm_id": "vm-456",  // optional
  "timeout_ms": 10000,  // optional, default 10000
  "snapshot_strategy": "pre-execution",  // optional
  "resource_limits": {  // optional
    "memory_mb": 512,
    "timeout_secs": 10
  }
}

Response:
{
  "execution_id": "exec-789",
  "vm_id": "vm-456",
  "agent_id": "agent-123",
  "status": "success",
  "exit_code": 0,
  "stdout": "Hello, World!\n",
  "stderr": "",
  "duration_ms": 1234,
  "memory_peak_mb": 64,
  "snapshot_id": "snap-101",
  "started_at": "2025-10-18T10:30:00Z",
  "completed_at": "2025-10-18T10:30:01.234Z"
}
```

**Execute Multiple Code Blocks**:
```
POST /api/v2/execute/batch
Content-Type: application/json

{
  "code_blocks": [
    {
      "code": "x = 1",
      "language": "python"
    },
    {
      "code": "print(x)",
      "language": "python"
    }
  ],
  "agent_id": "agent-123",
  "execution_mode": "sequential"  // or "parallel"
}

Response:
{
  "batch_id": "batch-456",
  "execution_id": "exec-789",
  "results": [
    {
      "block_index": 0,
      "status": "success",
      "stdout": "",
      "stderr": "",
      "exit_code": 0,
      "duration_ms": 45
    },
    {
      "block_index": 1,
      "status": "success", 
      "stdout": "1\n",
      "stderr": "",
      "exit_code": 0,
      "duration_ms": 67
    }
  ],
  "total_duration_ms": 156,
  "vm_id": "vm-456",
  "snapshot_id": "snap-102"
}
```

#### 7.2 VM Management Endpoints

**Get VM Pool Status**:
```
GET /api/v2/vm-pool/status

Response:
{
  "pool_stats": {
    "total_vms": 15,
    "prewarmed_vms": 8,
    "active_vms": 5,
    "warming_vms": 2,
    "utilization_percent": 33,
    "average_boot_time_ms": 1800
  },
  "resource_stats": {
    "memory_total_gb": 32,
    "memory_used_gb": 10,
    "memory_utilization_percent": 31,
    "cpu_cores_total": 16,
    "cpu_cores_used": 8,
    "cpu_utilization_percent": 50
  }
}
```

**Allocate VM for Agent**:
```
POST /api/v2/vms/allocate
Content-Type: application/json

{
  "agent_id": "agent-123",
  "vm_config": {
    "memory_mb": 2048,
    "vcpus": 2,
    "root_fs_size_gb": 10
  }
}

Response:
{
  "vm_id": "vm-456",
  "ip_address": "172.26.0.42",
  "status": "prewarmed",
  "estimated_ready_time_ms": 500,
  "allocated_at": "2025-10-18T10:30:00Z"
}
```

#### 7.3 Snapshot Management Endpoints

**Create Snapshot**:
```
POST /api/v2/vms/{vm_id}/snapshots
Content-Type: application/json

{
  "name": "before-risky-operation",
  "description": "Snapshot before executing untrusted code",
  "snapshot_type": "manual"
}

Response:
{
  "snapshot_id": "snap-103",
  "vm_id": "vm-456",
  "name": "before-risky-operation",
  "created_at": "2025-10-18T10:30:00Z",
  "size_mb": 256,
  "creation_time_ms": 234
}
```

**Rollback to Snapshot**:
```
POST /api/v2/vms/{vm_id}/snapshots/{snapshot_id}/rollback

Response:
{
  "vm_id": "vm-456",
  "snapshot_id": "snap-103",
  "rollback_time_ms": 567,
  "files_affected": 12,
  "processes_terminated": 3,
  "rolled_back_at": "2025-10-18T10:30:05Z"
}
```

### 8. Integration with Terraphim AI

#### 8.1 Agent Integration

**Agent VM Assignment**:
```rust
pub struct AgentVmManager {
    // Persistent VM assignment for agents
    agent_vm_assignments: Arc<RwLock<HashMap<String, String>>>,
    // VM performance tracking
    vm_performance: Arc<RwLock<HashMap<String, VmPerformance>>>,
}

impl AgentVmManager {
    pub async fn get_or_assign_vm(&self, agent_id: &str) -> Result<String> {
        // 1. Check existing assignment
        if let Some(vm_id) = self.agent_vm_assignments.read().await.get(agent_id) {
            if self.is_vm_healthy(vm_id).await? {
                return Ok(vm_id.clone());
            }
        }
        
        // 2. Assign new VM from pool
        let vm_id = allocate_vm_for_agent(agent_id).await?;
        self.agent_vm_assignments.write().await.insert(agent_id.to_string(), vm_id.clone());
        Ok(vm_id)
    }
}
```

#### 8.2 LLM Proxy Integration

**LLM Model Configuration**:
```rust
pub struct LlmVmConfig {
    pub model_provider: String,  // "openrouter", "anthropic", etc.
    pub model_name: String,
    pub api_key_ref: String,     // Reference to 1Password secret
    pub max_tokens: u32,
    pub temperature: f32,
    pub vm_requirements: VmRequirements,
}

pub struct VmRequirements {
    pub memory_mb: u32,
    pub vcpus: u32,
    pub disk_gb: u32,
    pub network_access: bool,
    pub timeout_secs: u32,
}
```

### 9. Implementation Roadmap

#### Phase 1: Core VM Execution (Weeks 1-2)
- [ ] Implement PrewarmedVmPool with basic allocation
- [ ] Add fast execution flow with snapshot support
- [ ] Integrate with existing firecracker-rust infrastructure
- [ ] Basic performance monitoring and metrics

#### Phase 2: Advanced Features (Weeks 3-4)  
- [ ] Smart rollback system with selective restore
- [ ] Resource management with memory/CPU limits
- [ ] Enhanced security validation and scanning
- [ ] Batch execution support for multiple code blocks

#### Phase 3: Performance Optimization (Weeks 5-6)
- [ ] Sub-2 second boot time optimization
- [ ] VM prewarming with predictive allocation
- [ ] Advanced caching and memory management
- [ ] Load balancing across multiple host machines

#### Phase 4: Production Readiness (Weeks 7-8)
- [ ] Comprehensive monitoring and alerting
- [ ] Disaster recovery and backup procedures  
- [ ] Performance tuning and capacity planning
- [ ] Security audit and penetration testing

### 10. Success Metrics

**Performance Targets**:
- VM allocation time: <50ms (from pool)
- Code execution setup: <200ms
- Total execution time: <2000ms (95th percentile)
- VM boot time: <1800ms (cold start)
- Snapshot creation: <100ms
- Rollback time: <500ms

**Reliability Targets**:
- VM pool availability: 99.9%
- Execution success rate: >95%
- Snapshot success rate: >99%
- System uptime: 99.95%

**Security Targets**:
- Zero VM escape incidents
- 100% code validation coverage
- <1% false positive security alerts
- Complete audit trail for all executions

This comprehensive VM execution API design provides the foundation for secure, high-performance code execution in terraphim-ai, leveraging the existing firecracker-rust infrastructure while adding advanced features for performance, security, and reliability.
