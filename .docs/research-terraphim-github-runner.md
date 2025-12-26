# Research Document: Terraphim Agent as GitHub Runner with Firecracker Sandboxing

## 1. Problem Restatement and Scope

### Problem Statement
Design and implement a system where terraphim-agent acts as a self-hosted GitHub Actions runner, executing workflows inside Firecracker microVMs with:
- Webhook-triggered execution from GitHub events
- Firecracker sandbox isolation for security
- Snapshot creation after each successful command
- Command history tracking with success/failure patterns
- Knowledge graph modification to learn from execution patterns and optimize future runs

### IN Scope
- GitHub webhook integration (extending existing `github_webhook` repo)
- Terraphim-agent as workflow executor
- Firecracker VM lifecycle management
- Snapshot management for rollback and state preservation
- Command history tracking and persistence
- Knowledge graph updates for pattern learning
- Error recovery and rollback mechanisms

### OUT of Scope
- GitHub Actions marketplace integration
- Multi-tenant/multi-repository support (initial version)
- Distributed runner architecture
- Container-based execution (Firecracker only)
- Windows/macOS runner support (Linux only initially)

---

## 2. User & Business Outcomes

### Visible Changes
1. **Self-Hosted Runner**: GitHub Actions workflows execute in Firecracker VMs instead of GitHub-hosted runners
2. **Enhanced Security**: Isolated VM execution prevents workflow interference and supply chain attacks
3. **State Persistence**: Successful command states are snapshotted for fast recovery
4. **Learning System**: Failed workflows inform the knowledge graph to prevent repeat failures
5. **Fast Boot**: Sub-2 second VM boot times enable rapid workflow execution

### Business Value
- **Cost Reduction**: Self-hosted execution reduces GitHub Actions minutes usage
- **Security Improvement**: Firecracker isolation provides stronger security guarantees
- **Reliability**: Snapshot-based recovery reduces CI/CD downtime
- **Intelligence**: Knowledge graph learns optimal execution paths over time

---

## 3. System Elements and Dependencies

### Core Components

| Component | Location | Role | Dependencies |
|-----------|----------|------|--------------|
| **github_webhook** | `github.com/terraphim/github_webhook` | Receives GitHub webhook events, triggers agent | Salvo, Octocrab, tokio |
| **terraphim_firecracker** | `terraphim_firecracker/` | VM lifecycle management, snapshots | Firecracker API, tokio |
| **terraphim_multi_agent** | `crates/terraphim_multi_agent/` | VM execution coordination | FcctlBridge, history tracking |
| **FcctlBridge** | `crates/terraphim_multi_agent/src/vm_execution/fcctl_bridge.rs` | VM session management, snapshots | reqwest, HTTP API |
| **CommandHistory** | `crates/terraphim_multi_agent/src/history.rs` | Command tracking and statistics | chrono, serde, uuid |
| **LessonsEvolution** | `crates/terraphim_agent_evolution/src/lessons.rs` | Learning from success/failure patterns | Persistable trait |
| **RoleGraph** | `crates/terraphim_rolegraph/` | Knowledge graph for semantic matching | Aho-Corasick automata |
| **terraphim_tui** | `crates/terraphim_tui/` | REPL interface for agent | rustyline, TuiService |

### Existing Implementations Found

#### 1. GitHub Webhook Handler (github_webhook)
```rust
// Current: Handles PR events, executes bash scripts
#[handler]
async fn handle_webhook(req: &mut Request, res: &mut Response) {
    // Signature verification, event parsing
    // Script execution via std::process::Command
    // Posts results back to PR as comments
}
```
**Limitation**: Executes scripts directly on host, no VM isolation.

#### 2. Firecracker VM Manager (terraphim_firecracker)
```rust
pub struct TerraphimVmManager {
    vm_manager: Arc<dyn VmManager>,
    optimizer: Arc<Sub2SecondOptimizer>,
    pool_manager: Arc<VmPoolManager>,
    performance_monitor: Arc<tokio::sync::RwLock<PerformanceMonitor>>,
}
```
**Capabilities**: VM creation, prewarmed pool, sub-2 second boot optimization.

#### 3. FcctlBridge - History & Snapshots
```rust
pub struct FcctlBridge {
    config: HistoryConfig,
    agent_sessions: Arc<RwLock<HashMap<String, VmSession>>>,
    direct_adapter: Option<Arc<DirectSessionAdapter>>,
}

impl FcctlBridge {
    async fn create_snapshot(&self, vm_id: &str, agent_id: &str) -> Result<String>;
    async fn track_execution(&self, ...) -> Result<Option<String>>;
    async fn auto_rollback_on_failure(&self, vm_id: &str, agent_id: &str);
}
```
**Already Implemented**:
- `snapshot_on_execution`: Create snapshot after every command
- `snapshot_on_failure`: Create snapshot only on failures
- `auto_rollback_on_failure`: Automatic rollback to last successful state
- Session-based history tracking per VM/agent pair

#### 4. Command History Tracking
```rust
pub struct CommandHistoryEntry {
    id: String,
    vm_id: String,
    agent_id: String,
    command: String,
    snapshot_id: Option<String>,
    success: bool,
    exit_code: i32,
    executed_at: DateTime<Utc>,
}
```

#### 5. Lessons Evolution System
```rust
pub struct LessonsEvolution {
    agent_id: AgentId,
    current_state: LessonsState,
    history: BTreeMap<DateTime<Utc>, LessonsState>,
}

pub struct LessonsState {
    technical_lessons: Vec<Lesson>,
    process_lessons: Vec<Lesson>,
    failure_lessons: Vec<Lesson>,
    success_patterns: Vec<Lesson>,
    lesson_index: HashMap<String, Vec<LessonId>>,
}
```

#### 6. RoleGraph Knowledge Graph
```rust
pub struct RoleGraph {
    nodes: AHashMap<u64, Node>,
    edges: AHashMap<u64, Edge>,
    documents: AHashMap<String, IndexedDocument>,
    thesaurus: Thesaurus,
    ac: AhoCorasick, // Fast pattern matching
}
```

---

## 4. Constraints and Their Implications

### Technical Constraints

| Constraint | Why It Matters | Implications |
|------------|---------------|--------------|
| **Firecracker Linux-only** | Firecracker requires KVM support | Must run on Linux hosts with virtualization enabled |
| **Sub-2 second boot target** | Performance requirement for responsive CI | Requires prewarmed VM pools and optimized images |
| **GitHub API rate limits** | 5000 requests/hour for authenticated requests | Must batch operations and implement exponential backoff |
| **Snapshot storage** | Snapshots consume disk space | Implement retention policies and cleanup |
| **Network isolation** | VMs need network for package downloads | Requires NAT/bridge configuration or air-gapped packages |

### Security Constraints

| Constraint | Why It Matters | Implications |
|------------|---------------|--------------|
| **Workflow isolation** | Workflows must not affect host or each other | Each workflow runs in fresh VM from clean snapshot |
| **Secret protection** | GitHub secrets must be secure | Secrets injected at runtime, never persisted to snapshots |
| **Webhook verification** | Prevent unauthorized execution | HMAC-SHA256 signature verification required |
| **Resource limits** | Prevent DoS via resource exhaustion | CPU, memory, and time limits per workflow |

### Operational Constraints

| Constraint | Why It Matters | Implications |
|------------|---------------|--------------|
| **Persistent knowledge** | Learning must survive restarts | Use terraphim_persistence for knowledge graph storage |
| **Graceful degradation** | System must remain operational on failures | Fallback to fresh VM if snapshot restore fails |
| **Observability** | Need visibility into execution | Comprehensive logging and metrics collection |

---

## 5. Risks, Unknowns, and Assumptions

### UNKNOWNS

1. **GitHub Actions YAML Parsing**: How to parse and execute GitHub Actions workflow YAML files
   - Need: Research GitHub Actions syntax specification
   - Mitigation: Start with simple bash-based workflows

2. **Runner Registration Protocol**: GitHub's self-hosted runner registration mechanism
   - Need: Study actions/runner implementation
   - Mitigation: Use webhook approach bypassing registration

3. **Firecracker Snapshot Performance**: Snapshot creation/restore latency at scale
   - Need: Benchmark with realistic workloads
   - Mitigation: Implement incremental snapshots if needed

4. **Knowledge Graph Update Frequency**: How often to update knowledge graph from learnings
   - Need: Balance between freshness and performance
   - Mitigation: Batch updates with periodic sync

### ASSUMPTIONS

1. **A-FIRECRACKER**: Firecracker is installed and KVM is available on the host
2. **A-NETWORK**: VMs have network access for package installation
3. **A-STORAGE**: Sufficient disk space for VM images and snapshots
4. **A-GITHUB**: Valid GitHub webhook secret and API token available
5. **A-PERMISSIONS**: Process has permissions to create/manage VMs
6. **A-SINGLE-REPO**: Initial version targets single repository support

### RISKS

#### Technical Risks

| Risk | Probability | Impact | Mitigation |
|------|-------------|--------|------------|
| **R-SNAPSHOT-CORRUPT** | Medium | High | Verify snapshot integrity before restore, maintain multiple fallbacks |
| **R-VM-LEAK** | Medium | Medium | Implement VM lifecycle timeout and garbage collection |
| **R-KNOWLEDGE-DRIFT** | Low | Medium | Periodic knowledge graph validation and reset mechanism |
| **R-RACE-CONDITIONS** | Medium | High | Use proper locking for concurrent workflow execution |

#### Product/UX Risks

| Risk | Probability | Impact | Mitigation |
|------|-------------|--------|------------|
| **R-SLOW-LEARNING** | Medium | Medium | Start with curated patterns, expand through learning |
| **R-FALSE-POSITIVES** | Medium | Medium | Require multiple failure occurrences before pattern addition |

#### Security Risks

| Risk | Probability | Impact | Mitigation |
|------|-------------|--------|------------|
| **R-VM-ESCAPE** | Low | Critical | Keep Firecracker updated, monitor security advisories |
| **R-SECRET-LEAK** | Low | Critical | Never persist secrets to snapshots, audit logging |

---

## 6. Context Complexity vs. Simplicity Opportunities

### Sources of Complexity

1. **Multi-Crate Architecture**: 10+ crates involved in execution path
2. **Async Coordination**: Multiple concurrent VMs and workflows
3. **State Management**: VM state, snapshots, history, knowledge graph
4. **External Dependencies**: Firecracker, GitHub API, fcctl-web

### Simplification Strategies

#### Strategy 1: Layered Architecture
```
┌─────────────────────────────────────────┐
│  GitHub Webhook Handler (Entry Point)   │
├─────────────────────────────────────────┤
│  Workflow Executor (New Component)      │
├─────────────────────────────────────────┤
│  VM Session Manager (FcctlBridge)       │
├─────────────────────────────────────────┤
│  Firecracker VM Manager (Existing)      │
├─────────────────────────────────────────┤
│  Knowledge Graph + Lessons (Learning)   │
└─────────────────────────────────────────┘
```

#### Strategy 2: Event-Driven Design
```
Webhook → Event → Executor → VM → Result → Learning → Response
                    ↓
              Snapshot Points
```

#### Strategy 3: Phased Implementation
1. **Phase 1**: Basic webhook → VM execution → result posting
2. **Phase 2**: Snapshot on success, history tracking
3. **Phase 3**: Knowledge graph integration, pattern learning
4. **Phase 4**: Advanced features (parallel workflows, caching)

---

## 7. Questions for Human Reviewer

### Critical Decisions

1. **Q: GitHub Actions Compatibility Level**
   - Should we parse full GitHub Actions YAML or use simplified bash-only execution?
   - Full compatibility is significantly more complex but more useful.

2. **Q: Snapshot Strategy**
   - Create snapshots after EVERY successful command, or only at workflow boundaries?
   - Per-command is safer but storage-intensive.

3. **Q: Knowledge Graph Scope**
   - Should the knowledge graph be shared across repositories or per-repository?
   - Sharing enables cross-project learning but risks contamination.

4. **Q: Failure Classification**
   - What failure categories should influence the knowledge graph?
   - Transient errors (network timeouts) vs. deterministic failures (missing dependencies).

5. **Q: Integration Mode**
   - Use existing `fcctl-web` HTTP API or implement direct Firecracker integration?
   - HTTP is simpler but adds latency; direct is faster but more complex.

### Architecture Questions

6. **Q: Runner vs. Webhook Model**
   - Register as official self-hosted runner or continue with webhook-based execution?
   - Runner model requires implementing GitHub's protocol but enables better integration.

7. **Q: Multi-Repository Support**
   - Should initial design account for multiple repositories or single-repo only?
   - Multi-repo requires tenant isolation and resource allocation.

### Operational Questions

8. **Q: Snapshot Retention Policy**
   - How long to retain snapshots? How many per workflow?
   - Affects storage costs and recovery capabilities.

9. **Q: Learning Threshold**
   - How many failures before a pattern is added to knowledge graph?
   - Balance between responsiveness and noise filtering.

10. **Q: Monitoring Integration**
    - Which observability stack (Prometheus, OpenTelemetry, custom)?
    - Affects debugging and operations visibility.

---

## Appendix: Existing Code References

### Key Files for Implementation

| File | Purpose | Line Reference |
|------|---------|----------------|
| `github_webhook/src/main.rs` | Webhook handler to extend | Full file |
| `terraphim_firecracker/src/manager.rs` | VM management patterns | L36-89 |
| `crates/terraphim_multi_agent/src/vm_execution/fcctl_bridge.rs` | Snapshot/history implementation | L51-119 |
| `crates/terraphim_multi_agent/src/vm_execution/models.rs` | Data models for VM execution | L30-62 (HistoryConfig) |
| `crates/terraphim_multi_agent/src/history.rs` | Command history tracking | L11-127 |
| `crates/terraphim_agent_evolution/src/lessons.rs` | Lessons learning system | L14-128 |
| `crates/terraphim_rolegraph/src/lib.rs` | Knowledge graph implementation | L86-277 |

### Configuration Already Available

```rust
// HistoryConfig in models.rs
pub struct HistoryConfig {
    pub enabled: bool,
    pub snapshot_on_execution: bool,
    pub snapshot_on_failure: bool,
    pub auto_rollback_on_failure: bool,
    pub max_history_entries: usize,
    pub persist_history: bool,
    pub integration_mode: String, // "http" or "direct"
}
```

---

*Research completed: 2025-12-23*
*Phase 1 Disciplined Development*
