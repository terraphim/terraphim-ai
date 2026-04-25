# Research Document: Validate terraphim_rlm with All Examples (End-to-End, No Mocks)

## 1. Problem Restatement and Scope

### Problem Statement
The terraphim_rlm crate (Recursive Language Model orchestration) has multiple examples embedded in documentation (rustdoc) and tests throughout the codebase. We need to systematically validate that all these examples work correctly using **real ecosystem dependencies** -- no mocks, no stubs, leveraging actual Firecracker VMs, real LLM services, and real MCP servers.

### IN Scope
- Identify all examples in terraphim_rlm crate (documentation examples, test examples, MCP tool examples)
- Create a validation plan that exercises each example with **real infrastructure**
- Validate end-to-end flows: TerraphimRlm → Session → Executor (Firecracker/Docker/E2B) → LLM Bridge → MCP Tools
- Document validation results with actual execution traces
- Use `gh` CLI for private repository access (firecracker-rust)

### OUT of Scope
- Fixing bugs found during validation (separate task)
- Adding new examples (unless gaps are found)
- Performance benchmarking
- Mock-based testing (this is an end-to-end validation)

## 2. User & Business Outcomes

### User-Visible Outcomes
- Developers can trust that rustdoc examples compile and run correctly with real infrastructure
- All test cases pass against live services, indicating stable API under real conditions
- MCP tools work as documented with real MCP servers
- Clear documentation of what is validated and what infrastructure is required

### Business Outcomes
- Reduced regression risk when making changes to terraphim_rlm
- Improved developer confidence in the RLM system under real-world conditions
- Examples serve as executable specifications against live infrastructure
- Compliance with Rust community standards (documentation tests)
- Validation of private repository integration (firecracker-rust via gh CLI)

## 3. System Elements and Dependencies

### Core Crate Modules

| Module | File | Responsibility | Real Dependency |
|--------|------|----------------|---------------|
| lib.rs | src/lib.rs | Public API exports, crate documentation | All modules |
| rlm.rs | src/rlm.rs | Main TerraphimRlm orchestrator | session, budget, executor, llm_bridge, query_loop |
| session.rs | src/session.rs | Session lifecycle, VM affinity, context variables | config, types |
| budget.rs | src/budget.rs | Token/time budget tracking | config, types |
| config.rs | src/config.rs | RlmConfig, BackendType, KgStrictness | types |
| types.rs | src/types.rs | Shared types (SessionId, ExecutionResult, etc.) | None |
| error.rs | src/error.rs | RlmError definitions | None |

### Execution Environment (Real Backends)

| Module | File | Responsibility | Real Infrastructure |
|--------|------|----------------|---------------------|
| trait.rs | src/executor/trait.rs | ExecutionEnvironment trait | types |
| firecracker.rs | src/executor/firecracker.rs | Firecracker VM executor | **KVM + fcctl-core (private git)** |
| ssh.rs | src/executor/ssh.rs | SSH executor | SSH daemon |
| context.rs | src/executor/context.rs | ExecutionContext | types |
| mod.rs | src/executor/mod.rs | Module exports | All executor modules |

### Query Processing (Real Services)

| Module | File | Responsibility | Real Infrastructure |
|--------|------|----------------|---------------------|
| parser.rs | src/parser.rs | Command parsing | types |
| query_loop.rs | src/query_loop.rs | Query loop orchestration | executor, session, budget, **LLM service** |
| llm_bridge.rs | src/llm_bridge.rs | LLM bridge for VM-to-host calls | **terraphim_service + LLM API** |

### Additional Features (Feature-Gated, Real)

| Module | File | Responsibility | Feature Gate | Real Infrastructure |
|--------|------|----------------|--------------|---------------------|
| logger.rs | src/logger.rs | Trajectory logging | None | Filesystem |
| validator.rs | src/validator.rs | Knowledge graph validation | kg-validation | **terraphim_automata KG** |
| mcp_tools.rs | src/mcp_tools.rs | MCP tool implementations | mcp | **Real MCP server** |

### External Dependencies (Real)

| Dependency | Type | Purpose | Access Method |
|------------|------|---------|---------------|
| terraphim_firecracker | Internal crate | Firecracker VM management | Workspace |
| terraphim_service | Internal crate | LLM service integration | Workspace (llm feature) |
| terraphim_automata | Internal crate | Knowledge graph | Workspace (kg-validation feature) |
| fcctl-core | External git (private) | Firecracker Rust core | **gh CLI + SSH** |
| tokio | External | Async runtime | crates.io |
| rmcp | External | MCP protocol | crates.io (mcp feature) |
| bollard | External | Docker client | crates.io (docker-backend feature) |
| hyper | External | HTTP server | crates.io (llm-bridge feature) |

## 4. Constraints and Their Implications

### Technical Constraints

| Constraint | Why It Matters | Implications for Validation |
|------------|----------------|---------------------------|
| **Feature-gated compilation** | Some features require optional dependencies | Must validate with different feature flags: `default`, `full`, individual features |
| **Firecracker requires KVM** | VM execution needs hardware virtualisation | Requires bigbox with KVM enabled; cannot run in standard CI without bare metal |
| **Async/await throughout** | All APIs are asynchronous | Tests must use tokio test runtime; examples need `#[tokio::test]` or `async fn` |
| **Budget limits (tokens, time, depth)** | Prevents runaway execution | Test with both sufficient and insufficient budgets against real LLM |
| **Session-based state** | Sessions expire, have extensions | Test session lifecycle including expiry and extension with real VMs |
| **Private git dependency (fcctl-core)** | Requires authentication | Use `gh` CLI for access; SSH key or token required |

### Operational Constraints

| Constraint | Why It Matters | Implications for Validation |
|------------|----------------|---------------------------|
| **Documentation examples use `rust,ignore`** | Examples don't run via `cargo test --doc` | Must manually extract and run examples, or convert to doctests with real infrastructure |
| **Real VMs needed for full validation** | MockExecutor insufficient for e2e | Requires Firecracker/Docker/E2B infrastructure provisioned |
| **Workspace exclusion** | Crate marked experimental | Must build with `--package terraphim_rlm` explicitly |
| **LLM service endpoint** | query() needs live LLM | Requires API key and network access to LLM provider |
| **MCP server** | MCP tools need server | Requires MCP server running and accessible |

### Security Constraints

| Constraint | Why It Matters | Implications for Validation |
|------------|----------------|---------------------------|
| **VM isolation** | Code runs in sandboxed VM | Validate that executor trait enforces isolation with real VMs |
| **DNS allowlist** | Network access control | Test DNS filtering if DNS features enabled |
| **Budget enforcement** | Prevents resource exhaustion | Verify budgets are actually enforced with real LLM calls |
| **Private repo access** | fcctl-core is private | Use gh CLI with proper authentication; don't leak tokens |

## 5. Risks, Unknowns, and Assumptions

### UNKNOWNS

| Unknown | How to Resolve |
|---------|-----------------|
| How many examples exist across all modules? | Grep for ````rust` and `# Example` in all .rs files |
| Are MCP tool examples functional with real MCP server? | Review mcp_tools.rs for embedded examples |
| Does the full feature set compile with real dependencies? | Run `cargo build --features full` |
| Are there integration tests outside the crate? | Search for `terraphim_rlm` in tests/ directory |
| What LLM service endpoint is configured? | Check terraphim_service configuration |
| Is KVM available on the validation host? | Run `ls /dev/kvm` and check permissions |
| What MCP server should be used for validation? | Check project documentation or ask team |

### ASSUMPTIONS

| Assumption | Risk if Wrong |
|------------|----------------|
| All rustdoc examples can run with real infrastructure | Examples may depend on specific VM states or LLM responses |
| Firecracker VMs can be provisioned on bigbox | May need Docker fallback or E2B cloud option |
| LLM service is available and responsive | May need retries or timeout handling |
| MCP server is running and accessible | May need to start server as part of test setup |
| `gh` CLI is authenticated for private repo access | May need manual token setup |

### RISKS

| Risk | Severity | Mitigation |
|------|----------|-------------|
| **Firecracker not available on validation host** | High | Use Docker backend as fallback; document KVM requirements |
| **LLM service unavailable or rate-limited** | High | Use small models (gpt-3.5-turbo); add retries; test with cached responses |
| **Private repo access fails** | High | Verify gh auth; use SSH keys; document access requirements |
| **Examples have side effects (create snapshots)** | Medium | Clean up after tests; use temporary directories; track resource usage |
| **Budget exhaustion with real LLM** | Medium | Test with intentionally low budgets; monitor token usage |
| **Feature flag combinations untested** | Low | Test at least: no features, default, full |
| **MCP server not running** | Medium | Start MCP server in test setup; skip if unavailable |

## 6. Context Complexity vs. Simplicity Opportunities

### Sources of Complexity

1. **Multiple execution backends** - Firecracker, Docker, E2B each have different capabilities and setup requirements
2. **Feature-gated code** - kg-validation, mcp, llm-bridge change available API surface
3. **Async lifecycle** - Sessions, VMs, and queries have complex state transitions
4. **External dependencies** - LLM services, VM management, network access, private repos
5. **Real infrastructure requirements** - Cannot run in isolation; needs provisioned environment

### Simplification Strategies

1. **Use Docker backend as primary for validation** - More portable than Firecracker; still provides isolation
2. **Feature-based test modules** - Use `#[cfg(feature = "...")]` to organise tests by feature
3. **Categorise examples by dependency** - Group into: no-deps (pure API), docker-deps (use Docker), full-deps (need Firecracker + LLM + MCP)
4. **Extract examples to test file** - Create `tests/e2e_validation.rs` that explicitly tests each documented example with real infrastructure
5. **Use gh CLI for private repo management** - Automate access to firecracker-rust

## 7. Questions for Human Reviewer

1. **What LLM service endpoint should we use for validation?** OpenAI, Anthropic, or local? What API key?

2. **What MCP server should we connect to for MCP tool validation?** Is there a test MCP server available?

3. **Should we use Docker or Firecracker as the primary backend for validation?** Firecracker needs KVM; Docker is more portable.

4. **How should we handle the private firecracker-rust repository access in CI?** Use gh CLI with token, or SSH deploy keys?

5. **What constitutes "validation success" for end-to-end flows?** Is it:
   - All examples compile and run without errors?
   - All examples produce expected output?
   - All features work together in integration?

6. **Should we add `#[ignore]` tests for Firecracker-specific examples?** Since KVM may not be available everywhere.

7. **How should we clean up resources (VMs, snapshots) after validation?** Automatic cleanup or manual?

8. **Should validation be part of `cargo test` or a separate command?** Integration into existing test suite vs separate e2e test runner.

9. **What is the budget for LLM tokens during validation?** Should we limit to prevent runaway costs?

10. **Should we validate the kg-validation feature with a real knowledge graph?** Or skip if KG is not populated?
