# Research Document: Terraphim RLM Integration

## 1. Problem Restatement and Scope

### Problem Statement
The [rig-rlm project](https://github.com/joshua-mo-143/rig-rlm) provides a Rust implementation of Recursive Language Models (RLMs) - an agentic architecture where LLMs return commands that execute in a REPL environment, enabling recursive self-invocation. The project explicitly lists "Set up `impl ExecutionEnvironment` for Firecracker" as a TODO, and terraphim-ai has production-ready Firecracker VM infrastructure.

The opportunity is to integrate rig-rlm patterns into terraphim-ai, leveraging existing crates:
- **terraphim_firecracker**: Sub-2 second VM boot, pool management
- **terraphim_agent_***: Multi-agent supervision and orchestration
- **terraphim_automata**: Text matching for command parsing
- **terraphim_service**: LLM integration and knowledge graph search

### IN Scope
- Implement `ExecutionEnvironment` trait for Firecracker VMs
- Integrate rig-rlm's REPL pattern with terraphim's agent architecture
- Leverage terraphim's existing LLM providers (Ollama, OpenRouter)
- Add knowledge graph validation for code execution safety
- Create structured output using terraphim's RobotResponse patterns
- Support recursive LLM calling within VMs

### OUT of Scope
- PyO3 Python execution (already implemented in rig-rlm)
- OpenAI-only providers (terraphim uses multi-provider approach)
- Replacing rig-core with custom LLM abstraction (use alongside)

### UPDATED: Sandbox Alternatives Now IN Scope
- Docker/gVisor as local fallback (for non-KVM environments)
- Cloud sandboxes (E2B, Modal) as managed execution backends

---

## 2. User & Business Outcomes

### User-Visible Changes
1. **Secure Code Execution**: LLM-generated code runs in isolated Firecracker VMs with sub-2 second boot
2. **Knowledge Graph Validation**: Commands validated against domain knowledge before execution
3. **Recursive AI Workflows**: LLMs can spawn sub-LLM queries within sandboxed environments
4. **Structured Results**: JSON/JSONL output compatible with AI agent tooling
5. **Fault Tolerance**: Supervised agent architecture with automatic recovery

### Business Value
1. **Security**: VM isolation prevents malicious code from affecting host system
2. **Performance**: Pre-warmed VM pools enable sub-500ms allocation
3. **Reliability**: OTP-style supervision ensures continuous operation
4. **Extensibility**: ExecutionEnvironment trait allows pluggable execution backends
5. **Integration**: MCP server exposure enables Claude Code and other AI tools

---

## 3. System Elements and Dependencies

### rig-rlm Components (Source)

| Component | Location | Responsibility | Dependencies |
|-----------|----------|----------------|--------------|
| `RigRlm<T>` | `src/llm.rs` | Main orchestration, query loop | `ExecutionEnvironment`, rig-core Agent |
| `ExecutionEnvironment` | `src/exec.rs` | Trait for code execution backends | None (trait) |
| `Pyo3Executor` | `src/exec.rs` | Python execution via PyO3 | pyo3, Python runtime |
| `REPL<T>` | `src/repl.rs` | Context storage, command dispatch | ExecutionEnvironment |
| `Command` | `src/repl.rs` | RUN/FINAL/RunCode command parsing | None |
| `query_llm()` | `src/exec.rs` | PyFunction for recursive LLM calls | tokio, oneshot channel |

### Terraphim Crates (Target Integration)

| Crate | Responsibility | Integration Point |
|-------|----------------|-------------------|
| `terraphim_firecracker` | VM pool management, sub-2s boot | `impl ExecutionEnvironment` |
| `terraphim_agent_supervisor` | OTP-style agent supervision | RigRlm lifecycle management |
| `terraphim_multi_agent` | Multi-agent coordination | Recursive LLM call orchestration |
| `terraphim_kg_orchestration` | Task scheduling and execution | Command routing |
| `terraphim_service` | LLM integration, search | Replace rig-core for multi-provider |
| `terraphim_automata` | Text matching | Command parsing, validation |
| `terraphim_agent` | CLI robot mode | Structured JSON output |
| `terraphim_github_runner` | Workflow execution | Pattern for CommandExecutor |

### Dependency Flow

```
┌────────────────────────────────────────────────────────────────┐
│                        User Query                               │
└────────────────────────────────────────────────────────────────┘
                               │
                               ▼
┌────────────────────────────────────────────────────────────────┐
│  TerraphimRlm (new crate combining rig-rlm + terraphim)        │
│  ├─ LlmClient (terraphim_service::GenericLlmClient)            │
│  ├─ KnowledgeGraph (terraphim_rolegraph + automata)            │
│  └─ REPL<FirecrackerExecutor>                                  │
└────────────────────────────────────────────────────────────────┘
                               │
              ┌────────────────┼────────────────┐
              ▼                ▼                ▼
┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐
│ Command::Run    │  │ Command::Code   │  │ Command::Final  │
│ (Bash in VM)    │  │ (Python in VM)  │  │ (Return result) │
└─────────────────┘  └─────────────────┘  └─────────────────┘
              │                │
              ▼                ▼
┌────────────────────────────────────────────────────────────────┐
│  FirecrackerExecutor (impl ExecutionEnvironment)               │
│  ├─ VmPoolManager (pre-warmed VMs)                             │
│  ├─ Sub2SecondOptimizer (boot optimization)                    │
│  └─ CommandResult (exit_code, stdout, stderr)                  │
└────────────────────────────────────────────────────────────────┘
                               │
                               ▼
┌────────────────────────────────────────────────────────────────┐
│  AgentSupervisor (terraphim_agent_supervisor)                  │
│  └─ SupervisedAgent lifecycle: init → start → health → stop   │
└────────────────────────────────────────────────────────────────┘
```

### Cross-Cutting Concerns

1. **Error Handling**: Both use Result<T, Error> with structured errors
2. **Async Runtime**: Both use tokio; rig-rlm creates runtime for `query_llm()` - needs adaptation
3. **Logging**: rig-rlm uses tracing; terraphim uses log - standardize on tracing
4. **State Management**: rig-rlm uses HashMap context; terraphim has VersionedMemory

---

## 4. Constraints and Their Implications

### Technical Constraints

| Constraint | Why It Matters | Implication |
|------------|----------------|-------------|
| **Sub-2 second VM boot** | User experience for interactive sessions | Must use pre-warmed pool; cold boots acceptable for batch |
| **Firecracker requires root/sudo** | Installation and operation | Document privilege requirements; provide docker alternative |
| **Python runtime in VM** | Code execution environment | Pre-bake Python in VM images; manage interpreter versions |
| **Recursive LLM creates new runtime** | `query_llm()` calls `tokio::runtime::Runtime::new()` | Refactor to reuse existing runtime via channels |
| **rig-core OpenAI focus** | Provider lock-in | Use terraphim's multi-provider abstraction alongside |

### Security Constraints

| Constraint | Why It Matters | Implication |
|------------|----------------|-------------|
| **VM isolation** | Prevent code escape | No host filesystem mounts; network sandboxing |
| **Knowledge graph validation** | Prevent dangerous commands | Pre-execution hook using terraphim_automata |
| **Resource limits** | Prevent DOS | VM CPU/memory limits; execution timeouts |
| **Snapshot rollback** | Recovery from bad state | Leverage terraphim_github_runner patterns |

### Performance Constraints

| Constraint | Why It Matters | Implication |
|------------|----------------|-------------|
| **Sub-500ms allocation** | Interactive responsiveness | Pool must maintain target_pool_size |
| **LLM latency** | End-to-end user experience | Use fast local models (Ollama) for sub-queries |
| **Context size limits** | RLM handles infinite context | Chunking via knowledge graph; ~500K char sub-LLM limit |

### Compatibility Constraints

| Constraint | Why It Matters | Implication |
|------------|----------------|-------------|
| **Rust edition 2024** | Both projects use latest edition | No compatibility issues |
| **WASM support** | terraphim_automata has WASM target | Executor trait must be Send+Sync, not WASM-only |
| **MCP protocol** | AI tool integration | Expose RLM as MCP tools via terraphim_mcp_server |

---

## 5. Risks, Unknowns, and Assumptions

### UNKNOWNS (Information We Don't Have)

| Unknown | Impact | De-risking Strategy |
|---------|--------|---------------------|
| Firecracker Python image size | Boot time, disk usage | Benchmark minimal vs full images |
| VM-to-host communication latency | Recursive LLM performance | Profile SSH/vsock communication |
| Memory overhead per VM | Pool capacity limits | Measure with real workloads |
| rig-core version compatibility | Breaking changes | Pin version; test with latest |

### ASSUMPTIONS (Explicitly Marked)

| Assumption | If Wrong | Mitigation |
|------------|----------|------------|
| **ASSUMPTION**: Pre-warmed VMs can persist Python interpreter state | Cold restart per execution | Use snapshot restore instead |
| **ASSUMPTION**: Firecracker vsock provides low-latency communication | Need SSH fallback | Abstract communication layer |
| **ASSUMPTION**: rig-core Agent works with custom base_url (Ollama) | Must patch or fork | Test with Ollama before commit |
| **ASSUMPTION**: LLM can parse Command reliably | Command extraction fails | Add retry with reformatting prompt |

### RISKS

#### Technical Risks

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| **VM boot exceeds 2s with Python** | Medium | Defeats pre-warming benefit | Optimize image; use interpreter caching |
| **Recursive LLM creates stack overflow** | Low | Crash | Max recursion depth limit |
| **Command parsing fails on edge cases** | Medium | Incorrect execution | terraphim_automata fuzzy matching |
| **tokio runtime in `query_llm` conflicts** | High | Panic/deadlock | Refactor to channel-based approach |

#### Product/UX Risks

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| **Users expect instant response** | High | Abandonment | Show progress indicators; stream output |
| **Code execution too slow** | Medium | Poor UX | Parallel execution where safe |
| **Error messages unclear** | Medium | User confusion | Structured errors with suggestions |

#### Security Risks

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| **VM escape via kernel exploit** | Very Low | Critical | Regular kernel updates; seccomp |
| **Infinite loop in generated code** | High | Resource exhaustion | Strict timeouts per execution |
| **Malicious file operations** | Medium | Data loss | Read-only rootfs; tmpfs work dir |
| **Network abuse from VM** | Medium | Reputation/cost | Network isolation; egress filtering |

---

## 6. Context Complexity vs. Simplicity Opportunities

### Sources of Complexity

1. **Multiple execution backends**: rig-rlm supports Pyo3, Docker, cloud sandboxes; terraphim has Firecracker, local, hybrid modes
2. **Two LLM abstractions**: rig-core Agent vs terraphim_service GenericLlmClient
3. **Recursive runtime creation**: `query_llm()` creates new tokio runtime inside async context
4. **State management diversity**: HashMap context vs VersionedMemory vs VM snapshots
5. **Historical Python coupling**: rig-rlm is Python-centric; terraphim is Rust-native

### Simplification Opportunities

#### 1. Unified Execution Trait (Clear Boundary)
```rust
// Single trait combining rig-rlm and terraphim patterns
#[async_trait]
pub trait ExecutionEnvironment: Send + Sync {
    type Error: std::error::Error + Send + Sync + 'static;

    async fn execute_code(&self, code: &str, ctx: &ExecutionContext) -> Result<ExecutionResult, Self::Error>;
    async fn execute_command(&self, cmd: &str, ctx: &ExecutionContext) -> Result<ExecutionResult, Self::Error>;
    async fn validate(&self, input: &str) -> Result<ValidationResult, Self::Error>;

    fn capabilities(&self) -> &[Capability];
}
```

#### 2. Smaller Sub-Problem: FirecrackerExecutor First
- Implement `ExecutionEnvironment` for Firecracker without touching LLM layer
- Test with hardcoded commands before integrating with RigRlm
- Add knowledge graph validation as second step

#### 3. Strangler Pattern for LLM Layer
- Keep rig-core Agent working initially
- Gradually introduce terraphim_service endpoints
- Eventually swap backend without changing RigRlm interface

---

## 7. Sandbox Execution Alternatives (Updated Research)

This section documents alternative execution environments beyond self-hosted Firecracker, addressing development environments without KVM and cloud deployment options.

### 7.1 Comparison Matrix

| Backend | Isolation Level | Cold Start | Security | SDK | Cost Model |
|---------|-----------------|------------|----------|-----|------------|
| **Firecracker (self-hosted)** | Hardware VM | 100-200ms (pre-warm) | Strongest | Rust native | Infrastructure |
| **Docker + gVisor** | User-space kernel | 50-100ms | Strong | Rust (bollard) | Local only |
| **Docker + runc** | Shared kernel | <50ms | Weak | Rust (bollard) | Local only |
| **E2B** | Cloud VM | ~150ms | Strongest | Python/JS | $0.10/hr (2 vCPU) |
| **Modal** | Cloud container | Unknown | Strong | Python/JS/Go | Per-second |
| **Prime Intellect** | Docker container | Unknown | Moderate | Python | $0.08/hr |

### 7.2 Docker as Local Fallback

**Use Case**: Development on macOS/Windows, CI environments without KVM, quick testing.

#### Docker with gVisor Runtime

[gVisor](https://gvisor.dev/) provides a user-space kernel (Sentry) that intercepts syscalls, offering container-level convenience with enhanced isolation:

- **Isolation**: Each container gets its own application kernel in Go
- **Syscall coverage**: ~70-80% of Linux syscalls implemented
- **Cold start**: 50-100ms typical
- **Overhead**: Higher for I/O-intensive workloads due to syscall interception
- **Setup**: `runsc install` adds runtime to Docker daemon

```json
// /etc/docker/daemon.json
{
  "runtimes": {
    "runsc": {
      "path": "/usr/local/bin/runsc"
    }
  }
}
```

#### Security Comparison (Docker vs Firecracker)

| Aspect | Docker/runc | Docker/gVisor | Firecracker |
|--------|-------------|---------------|-------------|
| Kernel sharing | Shared host kernel | User-space kernel | Separate guest kernel |
| Escape risk | Container escapes possible | Sentry isolation | Hardware VM boundary |
| Attack surface | Large (full kernel) | Medium (Go kernel) | Minimal (device model) |
| Seccomp/AppArmor | Required for hardening | Built-in via Sentry | Jailer provides |

**ASSUMPTION**: gVisor provides sufficient isolation for development/testing, but Firecracker remains required for production execution of untrusted code.

### 7.3 E2B (Cloud Firecracker)

[E2B](https://e2b.dev/) provides cloud-hosted Firecracker VMs specifically designed for AI code execution:

**Architecture**:
- Isolated VMs with ~150ms cold start
- Per-sandbox filesystem and network isolation
- Pause/resume capability for state persistence

**SDK & API**:
- Python: `pip install e2b`
- JavaScript: `npm install @e2b/code-interpreter`
- **No Rust SDK** - would need REST API wrapper

**Pricing** (per-second billing):
| Resource | Rate |
|----------|------|
| 1 vCPU | $0.000014/s (~$0.05/hr) |
| 2 vCPU (default) | $0.000028/s (~$0.10/hr) |
| Memory | $0.0000045/GiB/s |
| Storage | Free (10-20 GiB) |

**Limits**:
- Free tier: $100 credits, 1hr max session, 20 concurrent
- Pro ($150/mo): 24hr sessions, 100 concurrent
- Enterprise: Custom

**Integration Pattern**:
```rust
// Conceptual - would need REST client
impl ExecutionEnvironment for E2bExecutor {
    async fn execute_code(&self, code: &str, ctx: &ExecutionContext) -> Result<ExecutionResult, Self::Error> {
        // POST /sandboxes/{id}/exec
        // Stream stdout/stderr via SSE
    }
}
```

### 7.4 Modal Sandboxes

[Modal](https://modal.com/docs/guide/sandboxes) provides cloud containers with runtime-defined configuration:

**Architecture**:
- Containers defined at runtime (not pre-built images required)
- 5-minute default lifetime, 24-hour maximum
- Supports volume mounts, secrets, custom images

**SDK & API**:
- Python: `modal.Sandbox.create()`
- JavaScript/Go: Available
- **No Rust SDK** - would need Python subprocess or REST

**Execution Model**:
```python
sb = modal.Sandbox.create(app=app)
p = sb.exec("python", "-c", "print('hello')", timeout=3)
stdout = p.stdout.read()
```

**Pricing**: Per-second compute charges (specific rates not documented in overview)

**Considerations**:
- Stronger ecosystem integration (volumes, secrets)
- Less AI-specific than E2B
- Requires Modal account and deployment

### 7.5 Prime Intellect Sandboxes

[Prime Intellect](https://docs.primeintellect.ai/sandboxes/overview) provides Docker-based sandboxes:

**Architecture**:
- Docker containers (not VMs)
- Network isolation configurable
- Beta status with limited availability

**Pricing**:
- CPU: $0.05/core/hour
- Memory: $0.01/GB/hour
- Disk: $0.001/GB/hour
- Example: 1 core + 2GB + 10GB = $0.08/hr

**Limitations**:
- No GPU support (on roadmap)
- Python SDK only
- Beta with account limits

**Assessment**: Lower security (Docker-only) and limited SDK make this less suitable than E2B/Modal.

### 7.6 Recommended Execution Backend Hierarchy

Based on research, the recommended fallback hierarchy:

```
1. Firecracker (self-hosted)
   └─ Use when: Linux with KVM, production, highest security
   └─ Cold start: 100-200ms (with pre-warming)

2. E2B (cloud)
   └─ Use when: No KVM, cloud deployment, managed infrastructure
   └─ Cold start: ~150ms
   └─ Cost: ~$0.10/hr for 2 vCPU

3. Docker + gVisor (local)
   └─ Use when: Development, CI, macOS/Windows
   └─ Cold start: 50-100ms
   └─ Cost: Local resources only

4. Docker + runc (local)
   └─ Use when: Quick testing only, trusted code
   └─ Cold start: <50ms
   └─ Security: Insufficient for untrusted code
```

### 7.7 ExecutionEnvironment Trait Abstraction

All backends can be unified under the existing trait:

```rust
#[async_trait]
pub trait ExecutionEnvironment: Send + Sync {
    type Error: std::error::Error + Send + Sync + 'static;

    async fn execute_code(&self, code: &str, ctx: &ExecutionContext) -> Result<ExecutionResult, Self::Error>;
    async fn execute_command(&self, cmd: &str, ctx: &ExecutionContext) -> Result<ExecutionResult, Self::Error>;
    async fn validate(&self, input: &str) -> Result<ValidationResult, Self::Error>;

    // New methods for cloud backends
    async fn create_sandbox(&self) -> Result<SandboxId, Self::Error>;
    async fn destroy_sandbox(&self, id: &SandboxId) -> Result<(), Self::Error>;

    fn capabilities(&self) -> ExecutorCapabilities;
    fn backend_type(&self) -> BackendType; // Firecracker | E2B | Docker | Modal
}

pub struct ExecutorCapabilities {
    pub isolation_level: IsolationLevel,  // VM | UserSpaceKernel | Container
    pub supports_snapshots: bool,
    pub supports_network: bool,
    pub max_session_duration: Duration,
    pub cold_start_estimate: Duration,
}
```

### 7.8 New Unknowns from This Research

| Unknown | Impact | De-risking Strategy |
|---------|--------|---------------------|
| E2B Rust SDK feasibility | Integration complexity | Prototype REST client wrapper |
| gVisor syscall coverage for Python ML | May fail on numpy/torch | Test with target workloads |
| E2B latency from terraphim regions | Network overhead | Benchmark from deployment region |
| Modal pricing at scale | Cost predictability | Request detailed pricing |

### 7.9 New Assumptions

| Assumption | If Wrong | Mitigation |
|------------|----------|------------|
| **ASSUMPTION**: E2B REST API is stable and well-documented | Integration fragile | Wrap in abstraction layer |
| **ASSUMPTION**: gVisor works with Python 3.11 and common packages | Development fallback fails | Test matrix in CI |
| **ASSUMPTION**: Docker available on all dev machines | Can't use local fallback | Document setup requirements |

---

## 8. Questions for Human Reviewer

### Critical Path Questions

1. **Should we fork rig-rlm or create terraphim_rlm crate?**
   - *Why it matters*: Fork maintains upstream compatibility; new crate allows deeper terraphim integration

2. **Priority: Firecracker executor vs multi-provider LLM?**
   - *Why it matters*: Determines Phase 2 focus and initial deliverable

3. **What Python version/packages should be pre-baked in VM?**
   - *Why it matters*: Affects boot time, image size, and user capabilities

### Architecture Questions

4. **Should recursive `query_llm()` use the same agent or spawn new?**
   - *Why it matters*: Shared agent = context continuity; new agent = isolation

5. **How should VM-to-host LLM communication work?**
   - *Why it matters*: vsock (fast, complex) vs HTTP (slower, simpler)

6. **Should we expose RLM as MCP tools or standalone service?**
   - *Why it matters*: MCP = Claude Code integration; standalone = broader use

### Security Questions

7. **What network access should VMs have?**
   - *Why it matters*: LLMs often want to fetch URLs; balance capability vs risk

8. **Should knowledge graph validation block or warn?**
   - *Why it matters*: Blocking = safe but restrictive; warning = flexible but risky

### Resource Questions

9. **Target pool size for pre-warmed VMs?**
   - *Why it matters*: Higher = faster allocation; lower = less memory usage

10. **Maximum recursion depth for LLM calls?**
    - *Why it matters*: Prevents infinite loops but may limit complex reasoning

### Sandbox Alternative Questions (New)

11. **Should E2B be the primary cloud backend over Modal?**
    - *Why it matters*: E2B is AI-specific with Firecracker VMs; Modal is more general-purpose

12. **Is Docker + gVisor sufficient for CI testing?**
    - *Why it matters*: Determines if we need real Firecracker in CI (requires nested virtualization)

13. **Should we build a Rust E2B SDK or use REST directly?**
    - *Why it matters*: SDK = better DX, more maintenance; REST = simpler, less ergonomic

14. **What's the budget ceiling for cloud sandbox usage?**
    - *Why it matters*: E2B costs ~$0.10/hr per sandbox; affects default session limits
