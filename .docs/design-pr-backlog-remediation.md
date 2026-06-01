# Implementation Plan: Systematic PR Backlog Remediation

**Status**: Draft (Pending Human Approval)
**Author**: Root (orchestrator agent)
**Date**: 2026-05-31
**Based on**: Research Document `.docs/research-pr-backlog-remediation.md`

## Goal

Implement fixes for all 14 real unfixed issues identified during PR backlog triage, starting with security fixes, then core features, then CI/quality improvements.

## Plan Overview

### Phase 1: Security (Immediate)
| Priority | Issue | PR | Title | Estimated Effort |
|----------|-------|-----|-------|-----------------|
| P0 | #1313 | Fresh PR | Bind Redis to 127.0.0.1 | 15 min |
| P1 | #446 | Fresh PR | Exempt environment errors from circuit breaker penalties | 2 hours |

### Phase 2: Core Features (This Week)
| Priority | Issue | PR | Title | Estimated Effort |
|----------|-------|-----|-------|-----------------|
| P1 | #1443 | Fresh PR | Context rot token budget detection | 4 hours |
| P1 | #851 | Fresh PR | Thesaurus matching in robot mode | 4 hours |
| P1 | #1488 | Fresh PR | RLM executor hardening | 6 hours |

### Phase 3: CI/Quality (This Week)
| Priority | Issue | PR | Title | Estimated Effort |
|----------|-------|-----|-------|-----------------|
| P2 | #1362 | Fresh PR | Add RUSTDOCFLAGS=-D warnings to CI | 1 hour |
| P2 | #1347 | Fresh PR | Use correct test role names | 30 min |
| P2 | #1511 | Fresh PR | Enforce strict file permissions | 2 hours |

### Phase 4: Medium Priority (Next Week)
| Priority | Issue | PR | Title | Estimated Effort |
|----------|-------|-----|-------|-----------------|
| P2 | #1597 | Fresh PR | Complete robot envelope specification | 3 hours |
| P2 | #248 | Fresh PR | Circuit breaker environment error exemption | 2 hours |
| P2 | #251 | Fresh PR | Enforce RetryBound in Symphony | 3 hours |
| P2 | #1596 | Fresh PR | Isolate shared_resource tests | 2 hours |
| P2 | #1612 | Fresh PR | Implement LearningStore persistence | 4 hours |
| P2 | #1601 | Fresh PR | Matrix dimension expansion tests | 3 hours |
| P3 | #784 | Fresh PR | Fill robot specification gaps | 2 hours |

## Detailed Design

### Phase 1.1: Redis Binding Fix

**Files to modify**:
- `docker/docker-compose.yml:55` - Change port binding

**Changes**:
```yaml
# BEFORE
    ports:
      - "6379:6379"

# AFTER
    ports:
      - "127.0.0.1:6379:6379"
```

**Test strategy**:
1. Run `docker-compose up redis` and verify it binds to 127.0.0.1
2. Run `ss -tlnp | grep 6379` and confirm it's on 127.0.0.1 not 0.0.0.0
3. Run existing tests to ensure no regression

**Acceptance criteria**:
- [ ] Redis container only accepts connections from localhost
- [ ] All existing tests pass
- [ ] Docker Compose health check still works

---

### Phase 1.2: Circuit Breaker Environment Error Exemption

**Files to modify**:
- `crates/terraphim_orchestrator/src/provider_probe.rs` - Add `is_environment_error()` predicate
- `crates/terraphim_orchestrator/src/circuit_breaker.rs` - Check predicate before penalising

**Changes**:
```rust
// In provider_probe.rs
pub fn is_environment_error(error: &ProviderError) -> bool {
    matches!(error, 
        ProviderError::EnvVarMissing(_) |
        ProviderError::TokenUnavailable |
        ProviderError::NetworkTimeout { .. }
    )
}

// In circuit_breaker.rs (in record_failure)
if is_environment_error(&error) {
    // Don't increment failure count or only increment partially
    return;
}
```

**Test strategy**:
1. Unit test: `test_environment_error_not_penalised()`
2. Unit test: `test_real_failure_still_penalised()`
3. Integration test: Simulate env var missing, verify circuit stays closed

**Acceptance criteria**:
- [ ] Missing env var does not increment circuit breaker failure count
- [ ] Real API errors still increment failure count
- [ ] Circuit breaker transitions to OPEN after real failures

---

### Phase 2.1: Context Rot Detection

**Files to modify**:
- `crates/terraphim_orchestrator/src/context.rs` - Add rot detection fields
- `crates/terraphim_orchestrator/src/agent.rs` - Check rot before using context
- `crates/terraphim_orchestrator/src/config.rs` - Add rot threshold config

**Changes**:
```rust
// In context.rs
pub struct ContextHealth {
    pub token_count: usize,
    pub last_accessed: Instant,
    pub rot_score: f32, // 0.0 = fresh, 1.0 = rotten
}

impl Context {
    pub fn check_rot(&self, budget: usize) -> RotStatus {
        let ratio = self.token_count as f32 / budget as f32;
        if ratio > 0.9 {
            RotStatus::Critical
        } else if ratio > 0.75 {
            RotStatus::Warning
        } else {
            RotStatus::Fresh
        }
    }
}
```

**Test strategy**:
1. Unit test: Context at 50% budget = Fresh
2. Unit test: Context at 80% budget = Warning
3. Unit test: Context at 95% budget = Critical
4. Integration test: Agent switches context when Critical

**Acceptance criteria**:
- [ ] Contexts exceeding 90% of token budget are flagged Critical
- [ ] Agent selects fresh context when available
- [ ] Warning logged when context exceeds 75%

---

### Phase 2.2: Thesaurus Matching in Robot Mode

**Files to modify**:
- `crates/terraphim_agent/src/robot/mod.rs` - Add Thesaurus matching
- `crates/terraphim_agent/src/robot/search.rs` - Populate `Thesaurus_matched` field

**Changes**:
```rust
// In robot/search.rs
pub struct RobotSearchResult {
    pub concept_id: String,
    pub label: String,
    pub confidence: f32,
    pub Thesaurus_matched: Vec<String>, // NEW FIELD
}

// After concept matching
let Thesaurus_matches = thesaurus.expand(&query);
result.Thesaurus_matched = Thesaurus_matches.into_iter().map(|t| t.term).collect();
```

**Test strategy**:
1. Unit test: Query "AI" returns "artificial intelligence" as Thesaurus match
2. Unit test: Query without Thesaurus matches returns empty vec
3. Integration test: Robot mode search includes Thesaurus field in output

**Acceptance criteria**:
- [ ] Robot search output includes `Thesaurus_matched` array
- [ ] Thesaurus expansion uses existing Thesaurus crate
- [ ] Performance impact < 10ms per query

---

### Phase 2.3: RLM Executor Hardening

**Files to modify**:
- `crates/terraphim_rlm/src/executor/mod.rs` - Create executor abstraction
- `crates/terraphim_rlm/src/executor/local.rs` - Local process executor
- `crates/terraphim_rlm/src/executor/docker.rs` - Docker executor
- `crates/terraphim_rlm/src/executor/sandbox.rs` - Sandbox boundaries

**Changes**:
```rust
// In executor/mod.rs
pub trait RlmExecutor: Send + Sync {
    async fn execute(&self, task: RlmTask) -> Result<RlmOutput, RlmError>;
    fn supports_tool(&self, tool_id: &str) -> bool;
}

pub struct SandboxConfig {
    pub max_memory_mb: usize,
    pub max_cpu_percent: f32,
    pub allowed_paths: Vec<PathBuf>,
    pub network_access: bool,
}
```

**Test strategy**:
1. Unit test: Local executor runs process and captures output
2. Unit test: Docker executor runs container with resource limits
3. Unit test: Sandbox prevents access to disallowed paths
4. Integration test: RLM task executes in sandbox

**Acceptance criteria**:
- [ ] Local executor runs tools in subprocess
- [ ] Docker executor runs tools in containers
- [ ] Sandbox prevents filesystem escape
- [ ] Resource limits (memory, CPU) are enforced

---

### Phase 3.1: Rustdoc CI Gate

**Files to modify**:
- `.github/workflows/ci-pr.yml` - Add rustdoc job

**Changes**:
```yaml
# Add to ci-pr.yml
  rustdoc:
    name: Rustdoc Warnings
    runs-on: [self-hosted, Linux, X64, bigbox]
    steps:
      - uses: actions/checkout@v4
      - name: Check rustdoc
        env:
          RUSTDOCFLAGS: "-D warnings"
        run: cargo doc --no-deps --workspace --all-features
```

**Test strategy**:
1. Verify CI fails when rustdoc warning introduced
2. Verify CI passes when no warnings

**Acceptance criteria**:
- [ ] CI job `rustdoc` runs on every PR
- [ ] Job fails if any rustdoc warning exists
- [ ] Job passes on current main (after fixing existing warnings)

---

### Phase 3.2: Test Role Name Fix

**Files to modify**:
- Search for "test_agent" in test files and rename to "test_user"

**Changes**:
```rust
// BEFORE
let role = Role::new("test_agent");

// AFTER
let role = Role::new("test_user");
```

**Test strategy**:
1. Run `cargo test` and verify all tests pass
2. Verify no remaining "test_agent" references in test code

**Acceptance criteria**:
- [ ] All test role names use "test_user"
- [ ] No test failures due to role name mismatch

---

### Phase 3.3: Strict File Permissions

**Files to modify**:
- `crates/terraphim_orchestrator/src/config.rs` - Add permission check on config load

**Changes**:
```rust
// In config.rs
pub fn load_config(path: &Path) -> Result<Config, ConfigError> {
    let metadata = std::fs::metadata(path)?;
    let permissions = metadata.permissions().mode();
    
    // Check if group/others have write access
    if permissions & 0o022 != 0 {
        return Err(ConfigError::InsecurePermissions {
            path: path.to_path_buf(),
            mode: permissions,
        });
    }
    
    // Continue loading...
}
```

**Test strategy**:
1. Unit test: Config with 0o644 permissions loads successfully
2. Unit test: Config with 0o666 permissions fails with error
3. Integration test: `warn_if_world_readable()` already handles this

**Acceptance criteria**:
- [ ] Config files with group/other write permissions are rejected
- [ ] Informative error message shows expected vs actual permissions
- [ ] Existing tests still pass

---

## Test Strategy

### Unit Tests
- Every new function gets unit tests
- Target 90% line coverage for new code
- Use `tokio::test` for async functions

### Integration Tests
- Security fixes: Verify behaviour in Docker Compose environment
- Feature fixes: Verify end-to-end workflow
- CI fixes: Verify CI pipeline changes in fork/branch

### Regression Tests
- Run `cargo test --workspace --all-features` before each PR
- Run `cargo clippy --workspace --all-features` before each PR
- Run `cargo fmt --check` before each PR

## Risk Analysis

| Risk | Mitigation |
|------|-----------|
| Security fix breaks remote Redis access | Document change in CHANGELOG; provide migration guide |
| Context rot detection too aggressive | Start with conservative thresholds (90% Critical, 75% Warning) |
| RLM hardening breaks existing tools | Maintain backward-compatible executor selection |
| CI rustdoc gate blocks unrelated PRs | Fix all existing warnings before enabling gate |

## Rollback Plan

Each fix is an independent PR. If any fix causes issues:
1. Revert the specific PR via `git revert`
2. Investigate root cause
3. Create revised PR

## Definition of Done

- [ ] All 14 issues have fresh focused PRs
- [ ] All 19 stale PRs are closed with documented rationale
- [ ] Security fixes merged to main
- [ ] Core feature fixes merged to main
- [ ] CI gates operational
- [ ] No regression in existing tests
- [ ] CHANGELOG updated

## Human Approval Required

Before starting implementation, please confirm:
1. Is the prioritisation correct? (Security > Core Features > CI/Quality > Medium)
2. Should any of the 14 issues be deprioritised or removed?
3. Is the scope appropriate (14 independent PRs vs. grouped PRs)?

## Next Steps After Approval

1. Start Phase 1.1 (Redis binding) - 15 minutes
2. Create PR, run CI, merge
3. Proceed to Phase 1.2 (Circuit breaker) - 2 hours
4. Continue through phases in order
