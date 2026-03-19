# PR #426 Implementation Plan Summary

## Executive Summary

PR #426 implements the `terraphim_rlm` crate for Recursive Language Model (RLM) orchestration. The implementation is substantial (5,681 additions, 108 tests) but has **critical security vulnerabilities**, **race conditions**, and **external dependency issues** blocking merge.

Both research and design documents have passed quality evaluation and are **approved for implementation**.

---

## Outstanding Issues from PR #426

### Critical Security Issues (Must Fix)

| Issue | Location | Risk | Fix Strategy |
|-------|----------|------|--------------|
| Path traversal vulnerability | firecracker.rs:726 | HIGH | Validate snapshot names |
| No input size limits | mcp_tools.rs:2625-2628 | HIGH | Add MAX_CODE_SIZE constant |
| Missing session validation | mcp_tools.rs:2630 | HIGH | Validate session exists |
| Race condition | firecracker.rs:692-693 | HIGH | Atomic snapshot counter |

### Critical Dependency Issues

| Issue | Impact | Fix Strategy |
|-------|--------|--------------|
| fcctl-core unavailable | CI/CD breaks | Create ExecutionEnvironment trait + MockExecutor |
| Firecracker-rust PRs #14-19 | Assumed but not merged | Proceed with abstraction layer |

### Resource Management Issues

| Issue | Location | Fix Strategy |
|-------|----------|--------------|
| Memory leak | logger.rs:1638-1640 | Add MAX_MEMORY_EVENTS limit |
| No timeout handling | query_loop.rs | Add tokio::time::timeout |
| Parser limits | parser.rs | Add size/depth limits |

---

## Implementation Phases

### Phase A: Security Hardening (Priority 1)
**5 steps - Critical for merge**

1. Create `validation.rs` - Centralised input validation
2. Fix snapshot naming - Apply path traversal validation
3. Fix race condition - Atomic snapshot counter
4. Add input validation to MCP tools
5. Add session validation to MCP tools

### Phase B: Resource Management (Priority 2)
**3 steps - HIGH priority**

6. Fix MemoryBackend memory leak
7. Add timeout to query loop
8. Add parser limits

### Phase C: CI Compatibility (Priority 3)
**4 steps - HIGH priority**

9. Create ExecutionEnvironment trait
10. Implement MockExecutor
11. Refactor firecracker.rs with feature gates
12. Update Cargo.toml

### Phase D: Error Handling (Priority 4)
**1 step - MEDIUM priority**

13. Enhance error types with `#[source]`

### Phase E: Testing (Priority 5)
**2 steps - MEDIUM priority**

14. Add integration test framework
15. Add unit tests for validation

---

## Files to Modify

| File | Action | Phase |
|------|--------|-------|
| `src/validation.rs` | Create | A |
| `src/executor/mod.rs` | Create | C |
| `src/executor/mock.rs` | Create | C |
| `src/executor/firecracker.rs` | Modify | A, C |
| `src/parser.rs` | Modify | B |
| `src/query_loop.rs` | Modify | B |
| `src/session.rs` | Modify | A |
| `src/logger.rs` | Modify | B |
| `src/mcp_tools.rs` | Modify | A |
| `Cargo.toml` | Modify | C |

---

## Configuration Constants

| Constant | Proposed Value | Usage |
|----------|---------------|-------|
| MAX_CODE_SIZE | 1MB | Input validation |
| MAX_MEMORY_EVENTS | 10,000 | Memory leak prevention |
| MAX_INPUT_SIZE | 10KB | Parser limit |
| MAX_RECURSION_DEPTH | 100 | Parser limit |
| Query timeout | 5 minutes | Query loop timeout |
| max_snapshots_per_session | 50 | Resource limit |

---

## Quality Gate Status

| Document | Status | Score | Verdict |
|----------|--------|-------|---------|
| Research | Approved | 4.2/5.0 | GO |
| Design | Approved | 4.5/5.0 | GO |

---

## Next Steps

1. **Await human approval** on research and design documents
2. **Set up bigbox environment** for implementation
3. **Execute Phase A** (Security) immediately upon approval
4. **Run tests** after each phase
5. **Quality gate review** before Phase C

---

## Documents

- **Research**: `.docs/research-pr426.md`
- **Design**: `.docs/design-pr426.md`
- **Quality Evaluation (Research)**: `.docs/quality-evaluation-pr426-research.md`
- **Quality Evaluation (Design)**: `.docs/quality-evaluation-pr426-design.md`
- **This Summary**: `.docs/summary-pr426-plan.md`

---

## Implementation Command

```bash
# Run on bigbox via SSH
ssh bigbox "cd /workspace/terraphim-ai && ./scripts/implement-pr426.sh"
```

Or use terraphim symphony/dark factory orchestrator for distributed execution.
