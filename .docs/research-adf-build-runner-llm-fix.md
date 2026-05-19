# Research: ADF build-runner-llm Fix

**Date**: 2026-05-19
**Issues**: #2378, #2181, #285
**Status**: Root cause identified

---

## 1. Problem Restatement

The `build-runner-llm.sh` script is failing on clippy violations across PR branches. The failure manifests as:

1. **Test compilation failures**: 5 `field_reassign_with_default` clippy errors in `evolution.rs` test functions (lines 430, 447, 463, 482, 496)
2. **Command transformation failures**: Script calls `terraphim-agent replace --role "DevOpsRunner"` but this role does not exist
3. **rch integration missing**: No KG mapping exists to transform `cargo clippy` → `rch exec -- cargo clippy`
4. **Auto-merge spam**: Despite issue #2181 (dedupe implementation), 40+ duplicate `[ADF] Auto-merge failed` issues exist as noise

---

## 2. System Elements Mapping

### 2.1 Script Architecture

```
build-runner-llm.sh (scripts/build-runner-llm.sh)
├── Priority 1: GitHub Actions workflow extraction (yq)
├── Priority 2: BUILD.md parsing
├── Priority 3: Cargo workspace fallback
│   ├── cargo fmt --all -- --check
│   ├── cargo clippy --workspace --all-targets -- -D warnings
│   ├── cargo build --workspace
│   └── cargo test --workspace --no-fail-fast
└── Priority 4-6: Makefile, Earthfile, package.json
```

### 2.2 Command Transformation Pipeline

```
extract command
    ↓
transform_command() → terraphim-agent replace --role "DevOpsRunner"
    ↓ (if transformed)
execute via eval
    ↓
POST_STATUS to Gitea
```

### 2.3 Key Files

| File | Purpose | Status |
|------|---------|--------|
| `scripts/build-runner-llm.sh` | Main build runner script | **Exists, needs role fix** |
| `crates/terraphim_orchestrator/src/evolution.rs` | Evolution manager (504 lines) | **Test code has 5 clippy violations** |
| `crates/terraphim_orchestrator/src/pr_poller.rs` | Auto-merge dedupe cache | **Dedup implemented** |
| `crates/terraphim_orchestrator/src/lib.rs` | Orchestrator with dedupe fields | **Dedup integrated** |
| `crates/terraphim_orchestrator/src/config.rs` | Orchestrator config (1390+ lines) | **Provider allowlist enforced** |
| `~/.config/terraphim/docs/src/kg/` | Knowledge graph terms | **No DevOpsRunner role** |

### 2.4 Agent Configuration

```toml
# terraphim.toml (line 962)
[[agents]]
name = "build-runner"
layer = "Growth"
cli_tool = "/bin/bash"
event_only = true
```

The agent uses `/bin/bash` with `scripts/build-runner-llm.sh` as task.

### 2.5 Known KG Roles

```
$ terraphim-agent roles list
* AI Engineer (ai)
  Frontend Developer (fedev)
```

**No "DevOpsRunner" role exists.** The script calls a non-existent role.

---

## 3. Constraints Analysis

### 3.1 Provider Constraints (from config.rs)

```rust
// Allowed subscription providers (C1)
const ALLOWED_PROVIDER_PREFIXES: &[&str] = &[
    "claude-code", "opencode-go", "kimi-for-coding",
    "minimax-coding-plan", "zai-coding-plan", "openai",
];

// Explicitly banned (pay-per-use prevention)
const BANNED_PROVIDER_PREFIXES: &[&str] = &[
    "opencode", "github-copilot", "google", "huggingface", "minimax",
];

// Bare models routed through claude-code CLI
const CLAUDE_CLI_BARE_MODELS: &[&str] = &["sonnet", "opus", "haiku"];
```

### 3.2 Model Routing

The script hardcodes `"haiku"` as the model in cost tracking:
```bash
"model": "haiku",
```

This is allowed (bare model, routes through claude-code CLI).

### 3.3 Feature Gates

```rust
// evolution feature must be enabled
#[cfg(feature = "evolution")]
use terraphim_agent_evolution::{...};

// Without feature, all methods are no-op
#[cfg(not(feature = "evolution"))]
pub fn ensure_agent(&mut self, _agent_id: &str) {}
```

### 3.4 Auto-merge Dedup Constraints

The `AutoMergeFailureDedupe` uses:
- **TTL**: 24 hours (configured but actual value needs verification)
- **Key**: `(project, pr_number, head_sha)`
- **Location**: `pr_poller.rs:379-419`

---

## 4. Root Cause Analysis

### 4.1 Root Cause #1: Non-existent DevOpsRunner Role

**Location**: `build-runner-llm.sh:129`

```bash
transformed=$(echo "$cmd" | "$HOME/.cargo/bin/terraphim-agent" replace \
  --role "DevOpsRunner" 2>/dev/null || echo "")
```

**Error**: Role 'DevOpsRunner' not found in config

**Available roles**: `AI Engineer (ai)`, `Frontend Developer (fedev)`

**Impact**: All command transformations silently fail (return empty string), script falls back to original commands.

**Mitigation**: Use existing `AI Engineer` role, or create `DevOpsRunner` role in KG.

---

### 4.2 Root Cause #2: evolution.rs Test Code Pattern Violation

**Location**: `crates/terraphim_orchestrator/src/evolution.rs`

```rust
// Lines 428-430, 446-448, 462-464, 481-483, 495-497
#[tokio::test(flavor = "multi_thread")]
async fn test_lesson_with_feature() {
    let mut config = EvolutionConfig::default();
    config.enabled = true;  // <-- field_reassign_with_default VIOLATION
    let mut mgr = EvolutionManager::new(config);
    ...
}
```

**Lint**: `field assignment outside of initializer for an instance created with Default::default()`

**Why it fails tests only**: The `--lib` target compiles clean. Only `--test '*'` targets fail because they include test code with this pattern.

**Correct pattern**:
```rust
let mut config = EvolutionConfig {
    enabled: true,
    ..Default::default()
};
```

---

### 4.3 Root Cause #3: Missing rch KG Mapping

**Location**: Design doc `design-build-runner-llm-v4-leverage-existing.md:46-56`

The design specifies creating:
```markdown
# ~/.config/terraphim/docs/src/kg/rch.md
# rch
Remote compilation helper for Rust projects.
synonyms:: cargo build, cargo test, cargo check, cargo clippy
```

**Status**: Not created. No rch → `rch exec --` transformation occurs.

**Current state**: `rch` IS installed at `/home/alex/.local/bin/rch`, but commands are NOT transformed.

---

### 4.4 Root Cause #4: Auto-merge Dedup Not Fully Effective

**Location**: `lib.rs:4776-4801`

The dedupe cache `auto_merge_failure_dedupe` is correctly integrated, but:
- It only prevents duplicates within the TTL window (24h)
- 40+ existing duplicate issues were created BEFORE the dedupe was implemented
- The dedupe only applies when `tracker.open_failure_issue()` succeeds

**Gap**: If Gitea API fails during issue creation, the `record()` call never executes, so retry on next tick creates another issue.

---

## 5. Risks and Assumptions

### 5.1 Confirmed Risks

| Risk | Severity | Evidence |
|------|----------|----------|
| Test-only code blocks production deploy | Medium | 5 test functions fail clippy, lib compiles clean |
| DevOpsRunner role doesn't exist | High | `terraphim-agent` returns error on role lookup |
| rch not integrated | Medium | Design doc not implemented |
| Duplicate auto-merge issues still being created | Medium | Gitea API failure path doesn't record dedupe |

### 5.2 Assumptions

| Assumption | Confidence | Notes |
|------------|------------|-------|
| Fixing tests won't break production | High | Tests are isolated with `#[cfg(feature = "evolution")]` |
| AI Engineer role can substitute for DevOpsRunner | Medium | Roles are project-specific; need to verify synonym coverage |
| The 40+ duplicate issues predate dedupe implementation | High | Issue #2181 was specifically about implementing dedupe |
| rch installation is correct | High | Binary exists at `/home/alex/.local/bin/rch` |

### 5.3 Unknowns

1. **Actual TTL value for AutoMergeFailureDedupe**: Need to verify the configured duration
2. **Whether compliance-watchdog timeout (Issue #285) is related**: 5x timeout in 12h suggests rate limiting or resource exhaustion
3. **Why 200+ stale task branches exist**: Likely ADF agents creating branches but never cleaning up

---

## 6. Questions for Reviewer

### 6.1 Immediate Action Questions

1. **Should we use "AI Engineer" role as DevOpsRunner fallback, or create a new DevOpsRunner role?**
   - Creating a new role takes ~30 minutes and requires KG rebuild
   - Using existing role is faster but may lack DevOps-specific synonyms

2. **Should the 5 test violations be fixed with struct update syntax or by disabling the lint in test modules?**
   - Struct update (`:=true`) is more idiomatic but changes test readability
   - `#[allow(field_reassign_with_default)]` is quicker but masks the issue

3. **Should rch integration be completed as part of this fix?**
   - The design doc specifies it but it's a separate implementation effort
   - Current script works without rch (just less efficient)

### 6.2 Architecture Questions

4. **Is the dedupe cache TTL configurable, and what is the current value?**
   - If hardcoded to 24h and that's too short, transient failures create issues
   - If too long, legitimate new failures wait unnecessarily

5. **Should duplicate issue cleanup be automated?**
   - 40+ existing issues need manual closing
   - Could write a one-time script to dedupe by (pr_number, head_sha)

### 6.3 Relationship Questions

6. **Is Issue #285 (compliance-watchdog timeout) caused by build-runner failures?**
   - If build-runner fails, compliance-watchdog may be retrying excessively
   - Check if timeout coincides with build-runner execution windows

7. **Are the 200+ stale branches from ADF agents that failed to complete?**
   - If so, branch cleanup should be part of agent lifecycle
   - Could be tracked separately in issue #2378 or a new issue

---

## 7. Fastest Path to Fix

### Phase 1: Unblock Build (30 minutes)

1. **Fix evolution.rs test code** (5 minutes):
   ```rust
   // Change 5 instances of:
   config.enabled = true;
   // To:
   config = EvolutionConfig {
       enabled: true,
       ..Default::default()
   };
   ```

2. **Fix DevOpsRunner role** (25 minutes):
   - Option A (fastest): Change script to use `AI Engineer` role
   - Option B (correct): Create `DevOpsRunner` role in KG with cargo/rch synonyms

### Phase 2: Reduce Auto-merge Spam (45 minutes)

3. **Verify dedupe TTL** (15 minutes):
   - Find `AutoMergeFailureDedupe::new(ttl)` call site
   - Confirm TTL is reasonable (recommend 24h minimum)

4. **Fix Gitea API failure path** (20 minutes):
   - In `lib.rs:4802-4809`, record dedupe BEFORE attempting issue creation
   - Or record on either success OR failure (failure creates no issue anyway)

5. **Close existing duplicate issues** (10 minutes):
   - One-time script to find and close duplicates by (pr_number, head_sha)

### Phase 3: Complete rch Integration (2 hours, optional)

6. **Create KG mappings** (30 minutes):
   - `~/.config/terraphim/docs/src/kg/rch.md`
   - `~/.config/terraphim/docs/src/kg/cargo fmt.md` (exception)

7. **Test transformation** (30 minutes):
   - Verify `cargo clippy` → `rch exec -- cargo clippy`

8. **Update build-runner-llm.sh** (60 minutes):
   - Add rch pre-execution check
   - Fall back to local if rch unavailable

---

## 8. Deliverables Not in Scope

This is a research document. The following are NOT included:
- Code changes (implementation)
- Test modifications
- KG configuration updates
- Issue creation or closing
- Branch cleanup scripts

---

## 9. References

- Issue #2378: `fix(ci): adf/build build-runner failing on clippy violations across PR branches`
- Issue #2181: `feat(adf): dedupe [ADF] Auto-merge failed issue creation`
- Issue #285: `[ADF] compliance-watchdog 5x timeout in 12h`
- Design doc: `.docs/design-build-runner-llm-v4-leverage-existing.md`
- Test file: `crates/terraphim_orchestrator/src/evolution.rs:328-504`
- Script: `scripts/build-runner-llm.sh`
- Dedupe: `crates/terraphim_orchestrator/src/pr_poller.rs:373-419`
