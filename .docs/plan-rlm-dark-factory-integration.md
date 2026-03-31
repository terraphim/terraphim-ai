# Plan: Leveraging RLM in AI Dark Factory

**Date**: 2026-03-31
**Status**: Draft
**Author**: Terraphim AI
**Related**: PR #426 (RLM), Dark Factory Orchestrator (`terraphim_orchestrator`)

---

## Executive Summary

The AI Dark Factory orchestrates fleets of CLI-based AI agents (codex, claude, opencode) to autonomously work on codebases. Currently, these agents run directly on the host OS with limited isolation.

The RLM crate provides production-ready Firecracker microVM isolation with sandboxed code execution, snapshot management, and budget tracking.

This plan describes how to integrate RLM into the Dark Factory to provide:
1. **Per-agent VM isolation** - Each agent runs in its own Firecracker VM
2. **Snapshot-based state management** - Agents can save/restore analysis state
3. **Recursive LLM calls** - Agents can spawn sub-LLMs within their VMs
4. **Unified budget enforcement** - Token/time budgets span orchestrator and agent layers

---

## Current Architecture

### Dark Factory (terraphim_orchestrator)

```
OrchestratorConfig (TOML)
        |
        v
AgentOrchestrator
  |-- TimeScheduler -------> cron triggers (Safety/Core/Growth layers)
  |-- NightwatchMonitor ----> drift detection and correction
  |-- CompoundReview -------> nightly multi-agent review swarm
  |
  |-- AgentSpawner ---------> OS processes (codex, claude, opencode)
  |-- RoutingEngine --------> keyword-based task routing
  |-- CostTracker ----------> budget enforcement per agent
  |-- HandoffBuffer --------> inter-agent state transfer
```

**Key limitation**: Agents run as bare CLI processes on the host. They have full filesystem access, share the same resource pool, and cannot be easily sandboxed.

### RLM (terraphim_rlm)

```
TerraphimRlm (public API)
    |-- SessionManager -----> VM affinity, context, snapshots
    |-- BudgetTracker ------> token + time budget enforcement
    |-- QueryLoop ----------> command parsing and execution
    |
    |-- FirecrackerExecutor -> KVM-based microVMs (<500ms boot)
    |-- MockExecutor -------> deterministic testing
    |-- DockerExecutor ------> container fallback
    |
    |-- RlmMcpService ------> 6 MCP tools for AI integration
```

**Key capability**: Isolated code execution in Firecracker VMs with snapshot support and budget tracking.

---

## Integration Architecture

### Vision: Each Dark Factory Agent Gets Its Own RLM VM

```
OrchestratorConfig (TOML)
        |
        v
AgentOrchestrator
  |-- TimeScheduler -------> cron triggers
  |-- NightwatchMonitor ----> drift detection
  |-- CompoundReview -------> nightly review swarm
  |
  |-- RlmManager -----------> NEW: Manages RLM instances per agent
  |     |-- RLM Instance (security-sentinel)
  |     |     |-- Firecracker VM (isolated)
  |     |     |-- Session (budget tracking)
  |     |     |-- Snapshots (state versioning)
  |     |
  |     |-- RLM Instance (code-reviewer)
  |     |     |-- Firecracker VM (isolated)
  |     |     |-- Session (budget tracking)
  |     |     |-- Snapshots (state versioning)
  |     |
  |     |-- RLM Instance (upstream-synchronizer)
  |           |-- Firecracker VM (isolated)
  |           |-- Session (budget tracking)
  |           |-- Snapshots (state versioning)
  |
  |-- AgentSpawner ---------> Spawns CLI tools INSIDE VMs via RLM
  |-- CostTracker ----------> Unified budget (orchestrator + RLM)
  |-- HandoffBuffer --------> JSON handoff between VMs
```

### How It Works

1. **Agent Definition Extension**: Add `isolation: "firecracker"` to `AgentDefinition`
2. **RLM Instance Per Agent**: Orchestrator creates an RLM session for each agent
3. **Code Execution in VM**: Agent commands execute via `rlm.execute_code()` and `rlm.execute_command()`
4. **Snapshot Checkpoints**: Agents can save state before risky operations
5. **Recursive LLM**: Agents can spawn sub-LLMs within their VMs via MCP tools
6. **Unified Budgets**: Token/time budgets enforced at both orchestrator and RLM layers

---

## Implementation Phases

### Phase 1: RLM Integration Layer (Week 1-2)

**Goal**: Add RLM as an optional execution backend for dark factory agents.

#### 1.1 New Crate: `terraphim_orchestrator_rlm`

Create a bridge crate that connects orchestrator to RLM:

```
crates/terraphim_orchestrator_rlm/
  ├── Cargo.toml
  └── src/
      ├── lib.rs           # RlmOrchestratorIntegration
      ├── agent_vm.rs      # Per-agent VM management
      ├── session_bridge.rs # Session ↔ HandoffContext mapping
      └── budget_sync.rs   # Unified budget enforcement
```

**Dependencies**:
```toml
[dependencies]
terraphim_orchestrator = { path = "../terraphim_orchestrator" }
terraphim_rlm = { path = "../terraphim_rlm", features = ["firecracker", "mcp"] }
tokio.workspace = true
serde.workspace = true
```

#### 1.2 Extend AgentDefinition

Add isolation configuration to `AgentDefinition`:

```rust
// In terraphim_orchestrator/src/config.rs

pub struct AgentDefinition {
    // ... existing fields ...

    /// Execution isolation mode
    pub isolation: AgentIsolation,

    /// RLM-specific configuration (only used when isolation = Firecracker)
    pub rlm_config: Option<RlmAgentConfig>,
}

pub enum AgentIsolation {
    /// Run directly on host (current behavior)
    Host,
    /// Run in Firecracker VM via RLM
    Firecracker,
    /// Run in Docker container
    Docker,
}

pub struct RlmAgentConfig {
    /// VM memory limit in MB
    pub memory_mb: u64,
    /// VM vCPU count
    pub vcpus: u8,
    /// Enable snapshot support
    pub enable_snapshots: bool,
    /// Max snapshots per session
    pub max_snapshots: u32,
    /// Enable recursive LLM calls
    pub enable_recursive_llm: bool,
}
```

#### 1.3 RlmOrchestratorIntegration

```rust
pub struct RlmOrchestratorIntegration {
    /// RLM instance shared across all agents
    rlm: Arc<TerraphimRlm>,
    /// Per-agent session tracking
    agent_sessions: HashMap<String, SessionId>,
    /// Budget sync between orchestrator and RLM
    budget_sync: BudgetSync,
}

impl RlmOrchestratorIntegration {
    /// Initialize RLM for an agent
    pub async fn init_agent(
        &mut self,
        agent_def: &AgentDefinition,
    ) -> Result<SessionId, OrchestratorError>;

    /// Execute code in agent's VM
    pub async fn execute_in_vm(
        &self,
        agent_name: &str,
        code: &str,
    ) -> Result<ExecutionResult, OrchestratorError>;

    /// Create snapshot of agent's VM state
    pub async fn snapshot_agent(
        &self,
        agent_name: &str,
        name: &str,
    ) -> Result<SnapshotId, OrchestratorError>;

    /// Restore agent's VM from snapshot
    pub async fn restore_agent(
        &self,
        agent_name: &str,
        snapshot: &SnapshotId,
    ) -> Result<(), OrchestratorError>;
}
```

#### 1.4 Update AgentSpawner

Modify `AgentSpawner` to support RLM-backed execution:

```rust
// Current: spawn CLI process on host
let process = Command::new(&agent.cli_tool)
    .arg(&agent.task)
    .spawn()?;

// New: spawn via RLM when isolation = Firecracker
match agent.isolation {
    AgentIsolation::Host => {
        // Existing behavior
    }
    AgentIsolation::Firecracker => {
        let result = rlm_integration.execute_in_vm(
            &agent.name,
            &format!("{} '{}'", agent.cli_tool, agent.task),
        ).await?;
        // Handle result
    }
    AgentIsolation::Docker => {
        // Future: Docker execution
    }
}
```

#### 1.5 Tests

- `test_rlm_agent_spawn` - Verify Firecracker agent starts
- `test_rlm_code_execution` - Verify code runs in VM
- `test_rlm_snapshot_restore` - Verify state persistence
- `test_budget_sync` - Verify unified budget enforcement

---

### Phase 2: Enhanced Agent Capabilities (Week 3-4)

**Goal**: Leverage RLM's unique features for agent workflows.

#### 2.1 Snapshot-Based Checkpoints

Agents can save state before risky operations:

```rust
// In compound review workflow
async fn run_compound_review(&self) -> Result<CompoundReviewResult, OrchestratorError> {
    // Snapshot before review
    let pre_review_snapshot = self.rlm.snapshot_agent("code-reviewer", "pre-review").await?;

    // Run review
    let result = self.review_swarm.run().await?;

    // If review made changes, snapshot after
    if result.pr_created {
        self.rlm.snapshot_agent("code-reviewer", "post-review").await?;
    }

    // If review failed catastrophically, restore
    if result.findings.iter().any(|f| f.severity == Critical) {
        self.rlm.restore_agent("code-reviewer", &pre_review_snapshot).await?;
    }

    Ok(result)
}
```

#### 2.2 Recursive LLM for Complex Tasks

Agents can spawn sub-LLMs within their VMs:

```python
# Agent script running inside Firecracker VM
import subprocess

# Main analysis
result = analyze_codebase()

# If complex sub-task needed, spawn sub-LLM
if result.complexity > threshold:
    sub_result = subprocess.run([
        "rlm_query",
        "--prompt", f"Analyze this complex section: {result.section}",
        "--session", os.environ["RLM_SESSION_ID"]
    ], capture_output=True)

    # Incorporate sub-LLM analysis
    result.sub_analysis = sub_result.stdout
```

#### 2.3 Unified Budget Tracking

```rust
pub struct BudgetSync {
    orchestrator_tracker: CostTracker,
    rlm_tracker: BudgetTracker,
}

impl BudgetSync {
    /// Sync budgets between orchestrator and RLM
    pub fn sync(&self) -> Result<(), OrchestratorError> {
        let orchestrator_remaining = self.orchestrator_tracker.remaining_budget();
        let rlm_remaining = self.rlm_tracker.remaining_tokens();

        // Enforce minimum of both
        let effective_budget = min(orchestrator_remaining, rlm_remaining);

        if effective_budget < MINIMUM_BUDGET_THRESHOLD {
            return Err(OrchestratorError::BudgetExhausted);
        }

        Ok(())
    }
}
```

#### 2.4 Nightwatch + RLM Integration

Enhance drift detection with VM-level metrics:

```rust
pub struct NightwatchMonitor {
    // ... existing fields ...

    /// RLM-specific metrics
    rlm_metrics: Option<RlmDriftMetrics>,
}

pub struct RlmDriftMetrics {
    /// VM memory usage trend
    pub memory_trend: MemoryTrend,
    /// Snapshot frequency
    pub snapshot_rate: f64,
    /// Recursive LLM call depth
    pub avg_recursion_depth: f64,
    /// Code execution success rate
    pub execution_success_rate: f64,
}
```

---

### Phase 3: Advanced Features (Week 5-6)

**Goal**: Production-grade RLM integration with full feature set.

#### 3.1 VM Pool Per Agent Layer

Pre-warm VMs based on agent layer:

```rust
pub struct VmPoolManager {
    /// Pre-warmed VMs for Safety layer (always running)
    safety_pool: Vec<FirecrackerVm>,
    /// Pre-warmed VMs for Core layer (cron-scheduled)
    core_pool: Vec<FirecrackerVm>,
    /// Overflow VMs for Growth layer (on-demand)
    overflow_pool: Vec<FirecrackerVm>,
}

impl VmPoolManager {
    /// Maintain pool based on agent schedule
    pub async fn reconcile(&mut self, config: &OrchestratorConfig) {
        // Safety: always maintain min_size VMs
        self.safety_pool.reconcile(config.safety_agents()).await;

        // Core: pre-warm before cron trigger
        self.core_pool.pre_warm(config.core_agents()).await;

        // Growth: spawn on-demand with overflow
        self.overflow_pool.handle_demand(config.growth_agents()).await;
    }
}
```

#### 3.2 Cross-Agent Context Handoff via Snapshots

```rust
impl RlmOrchestratorIntegration {
    /// Hand off context from one agent's VM to another
    pub async fn handoff_between_agents(
        &self,
        from_agent: &str,
        to_agent: &str,
        context: HandoffContext,
    ) -> Result<(), OrchestratorError> {
        // Snapshot source agent's state
        let snapshot = self.snapshot_agent(from_agent, "handoff-source").await?;

        // Serialize context to file in source VM
        self.execute_in_vm(from_agent, &format!(
            "echo '{}' > /tmp/handoff.json",
            serde_json::to_string(&context)?
        )).await?;

        // Copy file to target VM (via shared volume or network)
        // ... implementation depends on VM networking ...

        // Restore target agent from snapshot (if needed)
        self.restore_agent(to_agent, &snapshot).await?;

        Ok(())
    }
}
```

#### 3.3 Compound Review with RLM Swarms

Each review agent runs in its own isolated VM:

```rust
pub struct RlmReviewSwarm {
    /// Security reviewer VM
    security_vm: SessionId,
    /// Architecture reviewer VM
    architecture_vm: SessionId,
    /// Performance reviewer VM
    performance_vm: SessionId,
    /// Quality reviewer VM
    quality_vm: SessionId,
    /// Domain reviewer VM
    domain_vm: SessionId,
    /// Design quality reviewer VM
    design_vm: SessionId,
}

impl RlmReviewSwarm {
    /// Run all review agents in parallel VMs
    pub async fn run_review(&self, diff: &str) -> Vec<ReviewFinding> {
        tokio::join!(
            self.execute_review("security", diff),
            self.execute_review("architecture", diff),
            self.execute_review("performance", diff),
            self.execute_review("quality", diff),
            self.execute_review("domain", diff),
            self.execute_review("design", diff),
        )
    }

    async fn execute_review(&self, category: &str, diff: &str) -> ReviewFinding {
        let session = self.get_session(category);
        let result = self.rlm.execute_code(session, &format!(
            "review_code(category='{}', diff='{}')",
            category, diff
        )).await?;

        parse_finding(result.stdout)
    }
}
```

---

## Configuration Example

### Updated orchestrator.toml

```toml
# orchestrator.toml - Dark Factory with RLM Integration

working_dir = "/home/alex/projects/terraphim/terraphim-ai"

[rlm]
# Enable RLM integration
enabled = true
# Firecracker binary path
firecracker_bin = "/usr/local/bin/firecracker"
# VM socket base path
socket_base_path = "/var/lib/firecracker"
# Pre-warmed VM pool size
pool_min_size = 2
pool_max_size = 8
pool_target_size = 4

[nightwatch]
eval_interval_secs = 300
minor_threshold = 0.10
moderate_threshold = 0.20
severe_threshold = 0.40
critical_threshold = 0.70

[compound_review]
schedule = "0 2 * * *"
max_duration_secs = 1800
repo_path = "/home/alex/projects/terraphim/terraphim-ai"
create_prs = false
# Use RLM VMs for review agents
use_rlm_vms = true

# --- Safety Layer (always running in VMs) ---

[[agents]]
name = "security-sentinel"
layer = "Safety"
cli_tool = "codex"
task = "Scan for CVEs and security vulnerabilities"
isolation = "firecracker"
capabilities = ["security", "vulnerability-scanning"]

[agents.rlm_config]
memory_mb = 512
vcpus = 1
enable_snapshots = true
max_snapshots = 5
enable_recursive_llm = true

# --- Core Layer (scheduled, pre-warmed VMs) ---

[[agents]]
name = "upstream-synchronizer"
layer = "Core"
cli_tool = "claude"
task = "Sync with upstream repositories"
schedule = "0 3 * * *"
isolation = "firecracker"
capabilities = ["sync", "dependency-management"]

[agents.rlm_config]
memory_mb = 256
vcpus = 1
enable_snapshots = false

# --- Growth Layer (on-demand, host execution) ---

[[agents]]
name = "code-reviewer"
layer = "Growth"
cli_tool = "claude"
task = "Review PRs for code quality"
isolation = "host"  # No VM needed for simple review
capabilities = ["code-review", "architecture"]
```

---

## Migration Path

### Step 1: Feature Flag (Week 1)
- Add `rlm.enabled = false` default to config
- RLM integration is opt-in, zero impact on existing deployments

### Step 2: Safety Agents First (Week 2)
- Migrate Safety layer agents to Firecracker VMs
- These are always running, so easiest to test and monitor

### Step 3: Core Agents (Week 3)
- Migrate Core layer agents with pre-warmed VM pools
- Add cron-based VM lifecycle management

### Step 4: Growth Agents (Week 4)
- Migrate Growth layer agents to on-demand VMs
- Implement overflow VM spawning

### Step 5: Compound Review (Week 5)
- Migrate review swarm to RLM VMs
- Enable snapshot-based review state management

### Step 6: Full Integration (Week 6)
- Enable unified budget tracking
- Enable cross-agent context handoff
- Enable Nightwatch RLM metrics

---

## Benefits

| Benefit | Current | With RLM |
|---------|---------|----------|
| **Isolation** | None (host processes) | Firecracker VMs |
| **State Management** | File-based | VM snapshots |
| **Budget Tracking** | Orchestrator only | Orchestrator + RLM |
| **Recursive LLM** | Not supported | MCP tools |
| **Security** | Process-level | Kernel-level |
| **Recovery** | Restart agent | Restore snapshot |
| **Resource Limits** | rlimits only | VM memory/CPU limits |
| **Testing** | Mock processes | MockExecutor |

---

## Risks and Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| KVM not available | High | Fallback to Docker/host execution |
| VM boot time >500ms | Medium | Pre-warmed VM pools |
| fcctl-core API changes | Medium | Pin git dependency to specific commit |
| Memory overhead (8 VMs) | Low | Configurable VM memory limits |
| Network isolation | Medium | Firecracker provides network namespace |
| Snapshot storage growth | Low | Configurable max snapshots per session |

---

## Testing Strategy

### Unit Tests
- `test_rlm_agent_spawn` - VM creation and initialization
- `test_rlm_code_execution` - Python/bash in VM
- `test_rlm_snapshot_lifecycle` - Create/restore/delete
- `test_budget_sync` - Unified budget enforcement
- `test_handoff_between_vms` - Cross-agent context transfer

### Integration Tests
- `test_safety_agents_in_vms` - Safety layer VM management
- `test_compound_review_swarm` - Parallel review in VMs
- `test_drift_detection_with_rlm` - Nightwatch + RLM metrics
- `test_cron_pre_warm` - Core layer VM pre-warming

### Performance Tests
- VM boot time <500ms
- Snapshot creation <100ms
- Code execution latency <1s
- Memory overhead per VM <256MB

---

## Dependencies

### New Dependencies
- `terraphim_rlm` (existing crate, new dependency for orchestrator)
- `fcctl-core` (via terraphim_rlm)
- `terraphim-firecracker` (via terraphim_rlm)

### Existing Dependencies
- `tokio`, `serde`, `serde_json`, `thiserror`, `tracing`
- `cron` (scheduling)
- `uuid` (session IDs)

---

## Open Questions

1. **VM Networking**: How do VMs communicate with each other for handoff?
   - Option A: Shared volume via host filesystem
   - Option B: Firecracker virtio-net with bridge
   - Option C: Unix domain sockets via vsock

2. **Snapshot Storage**: Where are snapshots stored?
   - Option A: `/var/lib/firecracker/snapshots/`
   - Option B: S3-compatible object storage
   - Option C: Local filesystem with rotation

3. **VM Image Management**: How are VM images built and updated?
   - Option A: Pre-built rootfs with all CLI tools
   - Option B: Dynamic image creation per agent
   - Option C: Base image + overlay per agent

4. **Cost Tracking**: How to track Firecracker VM costs?
   - Option A: CPU time + memory usage
   - Option B: Fixed cost per VM-hour
   - Option C: API call counting only (current approach)

---

## Next Steps

1. **Review this plan** with team
2. **Answer open questions** above
3. **Create GitHub issue** for Phase 1 implementation
4. **Start with `terraphim_orchestrator_rlm` crate scaffold**
5. **Implement Phase 1** (RLM integration layer)
6. **Test with Safety agents** first
7. **Iterate through phases** based on feedback

---

## Appendix: File Changes

### New Files
- `crates/terraphim_orchestrator_rlm/Cargo.toml`
- `crates/terraphim_orchestrator_rlm/src/lib.rs`
- `crates/terraphim_orchestrator_rlm/src/agent_vm.rs`
- `crates/terraphim_orchestrator_rlm/src/session_bridge.rs`
- `crates/terraphim_orchestrator_rlm/src/budget_sync.rs`
- `crates/terraphim_orchestrator_rlm/tests/integration_test.rs`

### Modified Files
- `crates/terraphim_orchestrator/src/config.rs` (add isolation fields)
- `crates/terraphim_orchestrator/src/lib.rs` (add RLM integration)
- `Cargo.toml` (workspace members)
- `orchestrator.example.toml` (add RLM config section)

### Estimated Effort
- **Phase 1**: 3-4 days
- **Phase 2**: 4-5 days
- **Phase 3**: 5-6 days
- **Total**: ~2 weeks
