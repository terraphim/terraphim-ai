# Research Document: Validate terraphim_rlm with All Examples

## 1. Problem Restatement and Scope

### Problem Statement
The terraphim_rlm crate (Recursive Language Model orchestration) has multiple examples embedded in documentation (rustdoc) and tests throughout the codebase. We need to systematically validate that all these examples work correctly to ensure the crate's reliability and serve as living documentation.

### IN Scope
- Identify all examples in terraphim_rlm crate (documentation examples, test examples, MCP tool examples)
- Create a validation plan that exercises each example
- Design test infrastructure to run examples (potentially with mock executors where real VMs are unavailable)
- Document validation results

### OUT of Scope
- Fixing bugs found during validation (separate task)
- Adding new examples (unless gaps are found)
- Validating external dependencies (firecracker, Docker, E2B)
- Performance benchmarking

## 2. User & Business Outcomes

### User-Visible Outcomes
- Developers can trust that rustdoc examples compile and run correctly
- All test cases pass, indicating stable API
- MCP tools work as documented
- Clear documentation of what is validated and what is not

### Business Outcomes
- Reduced regression risk when making changes to terraphim_rlm
- Improved developer confidence in the RLM system
- Examples serve as executable specifications
- Compliance with Rust community standards (documentation tests)

## 3. System Elements and Dependencies

### Core Crate Modules

| Module | File | Responsibility | Dependencies |
|--------|------|-----------------|--------------|
| lib.rs | src/lib.rs | Public API exports, crate documentation | All modules |
| rlm.rs | src/rlm.rs | Main TerraphimRlm orchestrator | session, budget, executor, llm_bridge, query_loop |
| session.rs | src/session.rs | Session lifecycle, VM affinity, context variables | config, types |
| budget.rs | src/budget.rs | Token/time budget tracking | config, types |
| config.rs | src/config.rs | RlmConfig, BackendType, KgStrictness | types |
| types.rs | src/types.rs | Shared types (SessionId, ExecutionResult, etc.) | None |
| error.rs | src/error.rs | RlmError definitions | None |

### Execution Environment

| Module | File | Responsibility | Dependencies |
|--------|------|-----------------|--------------|
| trait.rs | src/executor/trait.rs | ExecutionEnvironment trait | types |
| firecracker.rs | src/executor/firecracker.rs | Firecracker VM executor | fcctl-core, terraphim_firecracker |
| ssh.rs | src/executor/ssh.rs | SSH executor | - |
| context.rs | src/executor/context.rs | ExecutionContext | types |
| mod.rs | src/executor/mod.rs | Module exports | All executor modules |

### Query Processing

| Module | File | Responsibility | Dependencies |
|--------|------|-----------------|--------------|
| parser.rs | src/parser.rs | Command parsing | types |
| query_loop.rs | src/query_loop.rs | Query loop orchestration | executor, session, budget, llm_bridge |
| llm_bridge.rs | src/llm_bridge.rs | LLM bridge for VM-to-host calls | terraphim_service (optional) |

### Additional Features (Feature-Gated)

| Module | File | Responsibility | Feature Gate |
|--------|------|-----------------|--------------|
| logger.rs | src/logger.rs | Trajectory logging | None (always compiled) |
| validator.rs | src/validator.rs | Knowledge graph validation | kg-validation |
| mcp_tools.rs | src/mcp_tools.rs | MCP tool implementations | mcp |

### External Dependencies

| Dependency | Type | Purpose | Optional |
|------------|------|---------|----------|
| terraphim_firecracker | Internal crate | Firecracker VM management | No (direct dependency) |
| terraphim_service | Internal crate | LLM service integration | Yes (llm feature) |
| terraphim_automata | Internal crate | Knowledge graph | Yes (kg-validation feature) |
| fcctl-core | External git | Firecracker Rust core | No |
| tokio | External | Async runtime | No |
| rmcp | External | MCP protocol | Yes (mcp feature) |
| bollard | External | Docker client | Yes (docker-backend feature) |
| hyper | External | HTTP server | Yes (llm-bridge feature) |

## 4. Constraints and Their Implications

### Technical Constraints

| Constraint | Why It Matters | Implications for Validation |
|------------|----------------|---------------------------|
| **Feature-gated compilation** | Some features require optional dependencies | Must validate with different feature flags: `default`, `full`, individual features |
| **Firecracker requires KVM** | VM execution needs hardware virtualisation | Use mock executors for CI; document real VM requirements |
| **Async/await throughout** | All APIs are asynchronous | Tests must use tokio test runtime; examples need `#[tokio::test]` or `async fn` |
| **Budget limits (tokens, time, depth)** | Prevents runaway execution | Test with both sufficient and insufficient budgets |
| **Session-based state** | Sessions expire, have extensions | Test session lifecycle including expiry and extension |

### Operational Constraints

| Constraint | Why It Matters | Implications for Validation |
|------------|----------------|---------------------------|
| **Documentation examples use `rust,ignore`** | Examples don't run via `cargo test --doc` | Must manually extract and run examples, or convert to doctests |
| **MockExecutor for unit tests** | Real executor needs infrastructure | Use MockExecutor pattern for fast, reliable tests |
| **Workspace exclusion** | Crate marked experimental | Must build with `--package terraphim_rlm` explicitly |

### Security Constraints

| Constraint | Why It Matters | Implications for Validation |
|------------|----------------|---------------------------|
| **VM isolation** | Code runs in sandboxed VM | Validate that executor trait enforces isolation |
| **DNS allowlist** | Network access control | Test DNS filtering if DNS features enabled |
| **Budget enforcement** | Prevents resource exhaustion | Verify budgets are actually enforced |

## 5. Risks, Unknowns, and Assumptions

### UNKNOWNS

| Unknown | How to Resolve |
|---------|-----------------|
| How many examples exist across all modules? | Grep for ````rust` and `# Example` in all .rs files |
| Are MCP tool examples functional or just documentation? | Review mcp_tools.rs for embedded examples |
| Does the full feature set compile correctly? | Run `cargo build --features full` |
| Are there integration tests outside the crate? | Search for `terraphim_rlm` in tests/ directory |

### ASSUMPTIONS

| Assumption | Risk if Wrong |
|------------|----------------|
| All rustdoc examples can be converted to runnable tests | Examples may depend on external state (running VMs) |
| MockExecutor is sufficient for validation | May miss real executor bugs |
| Feature flags work correctly | Some features may have broken dependencies |

### RISKS

| Risk | Severity | Mitigation |
|------|----------|-------------|
| **Firecracker not available in CI** | High | Use MockExecutor; document real VM requirements separately |
| **LLM bridge needs running service** | Medium | Mock the LLM bridge or use `dep:terraphim_service` feature gate |
| **Examples have side effects (create snapshots)** | Medium | Clean up after tests; use temporary directories |
| **Budget exhaustion not properly handled** | Medium | Test with intentionally low budgets |
| **Feature flag combinations untested** | Low | Test at least: no features, default, full |

## 6. Context Complexity vs. Simplicity Opportunities

### Sources of Complexity

1. **Multiple execution backends** - Firecracker, Docker, E2B each have different capabilities
2. **Feature-gated code** - kg-validation, mcp, llm-bridge change available API surface
3. **Async lifecycle** - Sessions, VMs, and queries have complex state transitions
4. **External dependencies** - LLM services, VM management, network access

### Simplification Strategies

1. **Use MockExecutor pattern consistently** - Already established in existing tests; follow this pattern
2. **Feature-based test modules** - Use `#[cfg(feature = "...")]` to organise tests by feature
3. **Categorise examples by dependency** - Group into: no-deps (pure API), mock-deps (use MockExecutor), real-deps (need full infrastructure)
4. **Extract examples to test file** - Create `examples_validation.rs` that explicitly tests each documented example

## 7. Questions for Human Reviewer

1. **Should we convert rustdoc examples to doctests?** Currently marked `ignore` which means they don't run automatically. Converting them would provide continuous validation but requires ensuring they can run in test environment.

2. **What is the target validation environment?** Should we:
   - (a) Only validate with mocks (fast, reliable, no infrastructure needed)
   - (b) Validate with real Firecracker VMs (slow, needs KVM, most realistic)
   - (c) Both: mocks in CI, real VMs in nightly tests

3. **Should we validate MCP tools?** The MCP tools (rlm_code, rlm_bash, etc.) are feature-gated. Should the validation plan include them?

4. **How should we handle the LLM bridge?** The `query()` and `query_llm()` methods need an LLM service. Should we:
   - Mock the LLM bridge
   - Require a test LLM service
   - Skip these tests if service unavailable

5. **Should validation be part of `cargo test`?** Or should it be a separate validation script/command?

6. **What constitutes "validation success"?** Is it:
   - All examples compile?
   - All examples run to completion?
   - All examples produce expected output?

7. **Should we add `#[should_panic]` or `#[ignore]` tests?** Some examples may demonstrate error conditions. How should these be handled?

8. **Version compatibility concerns?** The crate depends on `fcctl-core` from git. Should we pin to a specific commit for validation?

9. **Documentation validation tools?** Rust has `cargo test --doc` for doctests. Should we use this, or create custom validation?

10. **Reporting format?** Should validation results be:
    - Simple pass/fail with list of failures
    - Detailed report with execution times and output
    - JUnit XML for CI integration
