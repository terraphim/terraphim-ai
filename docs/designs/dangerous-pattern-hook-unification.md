# Design & Implementation Plan: DangerousPatternHook Unification

## 1. Summary of Target Behavior

After implementation, terraphim_hooks crate provides a unified pattern guard infrastructure that:
- Blocks dangerous commands via regex patterns with helpful error messages
- Supports allowlist patterns that override blocklist (e.g., `--force-with-lease` overrides `--force`)
- Works in both CLI context (terraphim-agent guard) and VM execution context (DangerousPatternHook)
- Enables future pattern loading from configuration files

The system maintains fail-open semantics for safety while providing clear, actionable blocking messages.

## 2. Key Invariants and Acceptance Criteria

### Invariants

| Category | Guarantee |
|----------|-----------|
| Order | Allowlist patterns are checked BEFORE blocklist patterns |
| Safety | Regex compilation failures are caught at construction, not runtime |
| Fail-open | If PatternGuard fails to initialize, commands pass through |
| No regression | Existing DangerousPatternHook behavior unchanged for VM execution |
| Performance | Pattern checking completes in < 1ms for typical commands |

### Acceptance Criteria

| ID | Criterion | Testable |
|----|-----------|----------|
| AC1 | `git checkout -b branch` allowed (safe pattern) | Yes |
| AC2 | `git checkout -- file` blocked (dangerous pattern) | Yes |
| AC3 | `rm -rf /tmp/test` allowed (temp directory exception) | Yes |
| AC4 | `rm -rf /home/user` blocked | Yes |
| AC5 | terraphim_agent uses PatternGuard from terraphim_hooks | Yes |
| AC6 | terraphim_multi_agent's DangerousPatternHook uses PatternGuard | Yes |
| AC7 | Pattern guard works without async runtime | Yes |

## 3. High-Level Design and Boundaries

### Architecture Diagram

```
                    terraphim_hooks (shared)
                    +---------------------------+
                    | pub mod guard;            |
                    |   PatternGuard            |
                    |   GuardResult             |
                    |   PatternDefinition       |
                    | pub mod replacement;      |
                    |   ReplacementService      |
                    |   HookResult              |
                    +---------------------------+
                         /           \
                        /             \
    terraphim_agent                terraphim_multi_agent
    +------------------+           +------------------------+
    | guard_patterns   |           | vm_execution/hooks.rs  |
    | (thin wrapper)   |           | DangerousPatternHook   |
    |                  |           | (wraps PatternGuard)   |
    | Uses:            |           |                        |
    | - PatternGuard   |           | Uses:                  |
    | - GuardResult    |           | - PatternGuard         |
    +------------------+           | - HookDecision         |
                                   +------------------------+
```

### Component Responsibilities

| Component | Responsibility |
|-----------|----------------|
| `terraphim_hooks::guard::PatternGuard` | Core pattern matching with allowlist/blocklist |
| `terraphim_hooks::guard::GuardResult` | Structured allow/block result with reason |
| `terraphim_hooks::guard::PatternDefinition` | Pattern + reason + priority |
| `terraphim_agent::guard_patterns` | CLI integration, command parsing |
| `terraphim_multi_agent::DangerousPatternHook` | Hook trait impl, VM context handling |

### Boundaries

- **terraphim_hooks** is sync-only (no async dependencies)
- **terraphim_multi_agent** keeps Hook trait and HookDecision (async)
- **terraphim_agent** keeps CLI parsing (sync)
- Pattern storage is internal to PatternGuard initially; future: load from files

## 4. File/Module-Level Change Plan

| File | Action | Before | After | Dependencies |
|------|--------|--------|-------|--------------|
| `crates/terraphim_hooks/src/guard.rs` | Create | - | PatternGuard, GuardResult, patterns | regex |
| `crates/terraphim_hooks/src/lib.rs` | Modify | Exports replacement | Also exports guard | guard.rs |
| `crates/terraphim_hooks/Cargo.toml` | Modify | No regex | Add regex = "1.0" | - |
| `crates/terraphim_agent/src/guard_patterns.rs` | Modify | Full impl | Thin wrapper | terraphim_hooks |
| `crates/terraphim_multi_agent/src/vm_execution/hooks.rs` | Modify | Own patterns | Uses PatternGuard | terraphim_hooks |
| `crates/terraphim_multi_agent/Cargo.toml` | Modify | No terraphim_hooks | Add dependency | terraphim_hooks |

### Detailed Changes

**terraphim_hooks/src/guard.rs (NEW)**
- `PatternDefinition` struct: regex pattern string, reason, priority
- `SafePattern` struct: regex pattern string (allowlist)
- `PatternGuard` struct: compiled patterns, check() method
- `GuardResult` struct: decision, reason, command, pattern (moved from guard_patterns.rs)
- `PatternGuardBuilder`: builder pattern for configuration
- Default patterns: all git/fs patterns from guard_patterns.rs

**terraphim_agent/src/guard_patterns.rs (MODIFY)**
- Remove `DestructivePattern`, `SafePattern`, `CommandGuard` implementations
- Keep only re-exports: `pub use terraphim_hooks::guard::{PatternGuard as CommandGuard, GuardResult};`
- Or keep thin wrapper if CLI-specific logic needed

**terraphim_multi_agent/hooks.rs (MODIFY)**
- `DangerousPatternHook::new()` creates internal `PatternGuard`
- `pre_tool()` delegates to `PatternGuard::check()`, maps to `HookDecision`
- Remove hardcoded regex patterns (moved to terraphim_hooks)

## 5. Step-by-Step Implementation Sequence

| Step | Action | Purpose | Deployable? |
|------|--------|---------|-------------|
| 1 | Add `regex = "1.0"` to terraphim_hooks/Cargo.toml | Enable regex | Yes |
| 2 | Create `terraphim_hooks/src/guard.rs` with types | Core module | Yes |
| 3 | Move patterns from guard_patterns.rs to guard.rs | Centralize | Yes |
| 4 | Implement `PatternGuard::check()` | Core logic | Yes |
| 5 | Add tests in terraphim_hooks | Verify | Yes |
| 6 | Export guard module from terraphim_hooks/lib.rs | Public API | Yes |
| 7 | Update terraphim_agent guard_patterns.rs to use import | Integration | Yes |
| 8 | Run terraphim_agent tests | Verify no regression | Yes |
| 9 | Add terraphim_hooks dependency to terraphim_multi_agent | Enable | Yes |
| 10 | Refactor DangerousPatternHook to use PatternGuard | Unification | Yes |
| 11 | Run terraphim_multi_agent tests | Verify no regression | Yes |
| 12 | Remove duplicate patterns from DangerousPatternHook | Cleanup | Yes |

## 6. Testing & Verification Strategy

| Acceptance Criteria | Test Type | Test Location |
|---------------------|-----------|---------------|
| AC1: git checkout -b allowed | Unit | terraphim_hooks/src/guard.rs |
| AC2: git checkout -- blocked | Unit | terraphim_hooks/src/guard.rs |
| AC3: rm -rf /tmp allowed | Unit | terraphim_hooks/src/guard.rs |
| AC4: rm -rf /home blocked | Unit | terraphim_hooks/src/guard.rs |
| AC5: terraphim_agent integration | Unit | terraphim_agent/src/guard_patterns.rs |
| AC6: DangerousPatternHook integration | Unit | terraphim_multi_agent hooks.rs |
| AC7: No async required | Unit | terraphim_hooks (no tokio in deps) |

### Test Categories

**Unit tests in terraphim_hooks/src/guard.rs:**
- All pattern matching scenarios
- Allowlist precedence over blocklist
- Edge cases (empty command, very long command)
- Invalid regex handling during construction

**Integration tests:**
- CLI guard command still works
- VM execution hook still blocks dangerous patterns

## 7. Risk & Complexity Review

| Risk | Mitigation | Residual Risk |
|------|------------|---------------|
| Breaking DangerousPatternHook | Step-by-step migration, run tests at each step | Low |
| Regex performance regression | Compile patterns once at construction | Low |
| Missing patterns in migration | Diff patterns before/after, 100% coverage tests | Low |
| Circular dependencies | terraphim_hooks has no deps on agent/multi_agent | None |
| API breakage | Keep existing public types, add new ones | Low |

## 8. Open Questions / Decisions for Human Review

1. **Pattern loading from files**: Should Phase 1 include TOML/JSON pattern loading, or defer to Phase 2?
   - Recommendation: Defer - keep hardcoded patterns initially

2. **Pattern priority**: Should patterns have explicit priority, or is order-of-definition sufficient?
   - Recommendation: Order-based - simpler, matches current behavior

3. **Error handling**: Should PatternGuard::new() return Result or panic on bad regex?
   - Recommendation: Return Result - callers decide how to handle

4. **Async trait**: Should PatternGuard implement the async Hook trait?
   - Recommendation: No - keep sync, DangerousPatternHook wraps it

5. **Pattern categories**: Should patterns be organized by category (git, fs, etc.)?
   - Recommendation: Yes - helps with documentation and future filtering

---

**Do you approve this plan as-is, or would you like to adjust any part?**
