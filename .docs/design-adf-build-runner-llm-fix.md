# Design: ADF Build-Runner-LLM Fix

**Status**: Draft
**Research Doc**: `.docs/research-adf-build-runner-llm-fix.md`
**Date**: 2026-05-19
**Estimated Effort**: 3 hours (Phase 1 + 2), 2 hours (Phase 3)

---

## 1. Summary of Target Behavior

Fix the ADF build-runner-llm script to:
1. Pass clippy with `--all-targets` (test targets currently fail)
2. Successfully transform commands via terraphim-agent (role issue resolved)
3. Reduce auto-merge failure issue spam by 80% within 48 hours
4. Optionally enable rch remote compilation for faster builds

### Key Invariants

| Invariant | Description |
|-----------|-------------|
| I1 | All CI builds must pass `cargo clippy --workspace --all-targets -- -D warnings` |
| I2 | No new `[ADF] Auto-merge failed` duplicate issues within 24h per PR |
| I3 | Build cost remains under $0.01 per build (no expensive model calls) |
| I4 | Fallback to deterministic commands when LLM/KG unavailable |

### Acceptance Criteria

| AC | Criterion | Test Method |
|----|-----------|-------------|
| AC1 | `evolution.rs` compiles with 0 clippy warnings | `cargo clippy -p terraphim_orchestrator --all-targets` |
| AC2 | `build-runner-llm.sh` transforms commands successfully | `terraphim-agent replace "cargo clippy" --role "AI Engineer"` returns transformed command |
| AC3 | No duplicate auto-merge issues within 24h window | Gitea API query for issue deduplication |
| AC4 | Build time < 5 minutes via rch transformation | Timed build execution |

---

## 2. Key Invariants and Acceptance Criteria

### Phase 1: Unblock Test Compilation (30 minutes)

**Root Cause**: 5 test functions in `evolution.rs` use `config.enabled = true` pattern after `Default::default()`, triggering clippy `field_reassign_with_default` lint.

**Fix Pattern**:
```rust
// BEFORE (lines 430, 448, 463, 482, 496)
let mut config = EvolutionConfig::default();
config.enabled = true;

// AFTER
let config = EvolutionConfig {
    enabled: true,
    ..Default::default()
};
```

### Phase 2: Fix Command Transformation (25 minutes)

**Root Cause**: Script calls `--role "DevOpsRunner"` which doesn't exist.

**Fix Options**:
- Option A (fastest): Use existing `AI Engineer` role
- Option B (correct): Create `DevOpsRunner` role with cargo/rch synonyms

### Phase 3: Reduce Auto-Merge Spam (45 minutes)

**Root Cause**: Dedup cache not recording on Gitea API failure path.

**Fix**: Move `record()` call before issue creation attempt.

---

## 3. High-Level Design and Boundaries

### Component Diagram

```
┌─────────────────────────────────────────────────────────────┐
│                build-runner-llm.sh                          │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  ┌──────────────┐   ┌──────────────┐   ┌──────────────┐   │
│  │   yq/GH     │──▶│  terraphim-  │──▶│  whitelist   │   │
│  │  extraction │   │  agent replace│   │  validation │   │
│  └──────────────┘   └──────────────┘   └──────────────┘   │
│                             │                               │
│                             ▼                               │
│                    ┌─────────────────┐                      │
│                    │  rch executor   │                      │
│                    │ (if available) │                      │
│                    └────────┬────────┘                      │
│                             │                               │
│                             ▼                               │
│                    ┌─────────────────┐                      │
│                    │  POST_STATUS    │                      │
│                    │  (Gitea API)    │                      │
│                    └─────────────────┘                      │
└─────────────────────────────────────────────────────────────┘
```

### Phase 3 Components

```
┌─────────────────────────────────────────────────────────────┐
│              AutoMergeFailureDedupe                          │
├─────────────────────────────────────────────────────────────┤
│  ┌──────────────┐   ┌──────────────┐   ┌──────────────┐   │
│  │ pr_poller.rs │──▶│  dedupe.rs    │──▶│  Gitea API  │   │
│  │              │   │  (cache)     │   │  (issue cr)  │   │
│  └──────────────┘   └──────────────┘   └──────────────┘   │
│         │                  │                               │
│         │                  ▼                               │
│         │         record(pr, sha) BEFORE API call         │
│         │                  │                               │
│         │                  ▼                               │
│         │         dedupe check on next tick               │
│         └─────────────────────────────────────────────────┘
└─────────────────────────────────────────────────────────────┘
```

---

## 4. File/Module-Level Change Plan

### Phase 1: Clippy Fixes

| File | Action | Lines | Change |
|------|--------|-------|--------|
| `crates/terraphim_orchestrator/src/evolution.rs` | Modify | 430, 448, 463, 482, 496 | Struct update syntax |

### Phase 2: Role Fixes

| File | Action | Change |
|------|--------|--------|
| `scripts/build-runner-llm.sh` | Modify | Line 129: `DevOpsRunner` → `AI Engineer` |

### Phase 3: Dedup Fixes

| File | Action | Location | Change |
|------|--------|----------|--------|
| `crates/terraphim_orchestrator/src/lib.rs` | Modify | ~4802-4809 | Move `record()` before issue creation |

---

## 5. Step-by-Step Implementation Sequence

### Step 1: Fix Clippy Violations (evolution.rs)
**Purpose**: Unblock test compilation
**Files**: `crates/terraphim_orchestrator/src/evolution.rs`
**Deployable**: Yes (test-only change)

```
1. Change line 430: config.enabled = true → EvolutionConfig { enabled: true, ..Default::default() }
2. Change line 448: same pattern
3. Change line 463: same pattern
4. Change line 482: same pattern
5. Change line 496: same pattern
6. Run: cargo clippy -p terraphim_orchestrator --all-targets -- -D warnings
7. Verify: 0 warnings
```

### Step 2: Fix Role in build-runner-llm.sh
**Purpose**: Enable command transformation
**Files**: `scripts/build-runner-llm.sh`
**Deployable**: Yes (script change)

```
1. Change line 129: --role "DevOpsRunner" → --role "AI Engineer"
2. Test: echo "cargo clippy" | terraphim-agent replace --role "AI Engineer"
3. Verify transformation occurs
```

### Step 3: Fix Dedup API Failure Path
**Purpose**: Prevent duplicate issues on transient failures
**Files**: `crates/terraphim_orchestrator/src/lib.rs`
**Deployable**: Yes (after testing)

```
1. Find open_failure_issue() call around line 4802
2. Move record() call BEFORE the API call
3. Verify: on next tick, duplicate is detected
```

### Step 4: Optional - rch Integration
**Purpose**: Faster builds via remote compilation
**Files**: `~/.config/terraphim/docs/src/kg/rch.md` (new)
**Deployable**: After KG rebuild

```
1. Create rch.md KG mapping
2. Rebuild knowledge graph
3. Verify: cargo clippy → rch exec -- cargo clippy
```

---

## 6. Testing & Verification Strategy

| AC | Test Type | Command | Expected |
|----|----------|---------|----------|
| AC1 | Unit | `cargo clippy -p terraphim_orchestrator --all-targets -- -D warnings` | 0 warnings |
| AC2 | Manual | `echo "cargo clippy" \| terraphim-agent replace --role "AI Engineer"` | Non-empty output |
| AC3 | Integration | Gitea API query | No duplicate issues within 24h |
| AC4 | Performance | Timed build | < 5 minutes |

---

## 7. Risk & Complexity Review

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| Test change affects production | Low | Medium | Tests are feature-gated with `#[cfg(feature = "evolution")]` |
| AI Engineer role lacks DevOps synonyms | Medium | Low | Script falls back to original command |
| Dedup fix creates new race condition | Low | Medium | Move record() to BEFORE API call (idempotent) |

---

## 8. Open Questions / Decisions

1. **Use Option A (AI Engineer) or Option B (DevOpsRunner role)?**
   - Option A: 5 minutes, works immediately
   - Option B: 30 minutes, better semantic separation

2. **Should we also fix existing 40+ duplicate issues?**
   - Yes: One-time batch close script
   - No: Let them auto-close when PR merges

3. **Is rch integration in scope for this fix?**
   - Yes: Complete the design doc implementation
   - No: Track separately as #2378 subtask

---

## References

- Issue #2378: `fix(ci): adf/build build-runner failing on clippy violations`
- Issue #2181: `feat(adf): dedupe [ADF] Auto-merge failed issue creation`
- Issue #285: `[ADF] compliance-watchdog 5x timeout in 12h`
- Research: `.docs/research-adf-build-runner-llm-fix.md`
- Script: `scripts/build-runner-llm.sh`