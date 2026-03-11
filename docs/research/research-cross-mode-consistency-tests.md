# Research Document: Cross-Mode Consistency Test Failures

**Status**: Draft
**Author**: Claude (Disciplined Research)
**Date**: 2026-02-05
**Reviewers**: User

## Executive Summary

The cross-mode consistency integration tests fail with "Server failed to start within 60s" because the test uses a heavyweight configuration (`terraphim_engineer_config.json`) that requires building thesaurus from 58 markdown files and indexing 131+ documents at startup. The server only binds to the port AFTER all initialization completes, causing the health check to timeout.

## Essential Questions Check

| Question | Answer | Evidence |
|----------|--------|----------|
| Energizing? | Yes | These tests validate critical server/CLI/REPL consistency |
| Leverages strengths? | Yes | Validates core Terraphim search functionality |
| Meets real need? | Yes | Broken tests = broken CI, unknown reliability |

**Proceed**: Yes - 3/3 YES

## Problem Statement

### Description
Three integration tests in `crates/terraphim_agent/tests/cross_mode_consistency_test.rs` consistently fail:
- `test_cross_mode_consistency`
- `test_mode_specific_verification`
- `test_role_consistency_across_modes`

All fail with: `Error: Server failed to start within 60s`

### Impact
- CI pipeline fails on these tests
- Cannot validate cross-mode search consistency
- Blocks verification of server/CLI/REPL parity

### Success Criteria
- All three tests pass reliably
- Tests complete in reasonable time (<30s)
- Tests validate actual cross-mode consistency

## Current State Analysis

### Existing Implementation

**Test Location**: `crates/terraphim_agent/tests/cross_mode_consistency_test.rs`

**Server Startup Flow** (`start_test_server()`):
1. Pick unused port via `portpicker`
2. Ensure server binary compiled (debug build)
3. Spawn server with config: `terraphim_server/default/terraphim_engineer_config.json`
4. Loop for 60s waiting for `/health` to respond
5. Timeout if health never responds

**Server Initialization Flow** (`terraphim_server/src/lib.rs`):
```
axum_server() called
  |
  v
For each role with TerraphimGraph relevance:
  - Read markdown files from kg_local.path
  - Build thesaurus via Logseq builder
  - Create RoleGraph
  - Index documents from KG files
  - Process haystack files recursively
  |
  v
Create AppState with workflow/websocket support
  |
  v
Build Router with routes
  |
  v
TcpListener::bind() <-- Health only available AFTER this
  |
  v
axum::serve()
```

### Code Locations

| Component | Location | Purpose |
|-----------|----------|---------|
| Failing tests | `crates/terraphim_agent/tests/cross_mode_consistency_test.rs` | Cross-mode validation |
| Server startup | `terraphim_server/src/lib.rs:167-593` | axum_server function |
| Health endpoint | `terraphim_server/src/api.rs:27-29` | Simple OK response |
| Heavy config | `terraphim_server/default/terraphim_engineer_config.json` | 3 roles, KG, haystacks |
| Light config | `terraphim_server/fixtures/test_config.json` | Smaller test config |
| Working tests | `terraphim_server/tests/server.rs` | Uses inline sample_config |

### Data Flow

```
Test starts server with terraphim_engineer_config.json
  |
  v
Config specifies: docs/src/kg (58 .md files) + docs/src haystack (131 .md files)
  |
  v
Server builds thesaurus from 58 files (SLOW)
  |
  v
Server indexes 131 documents (SLOW)
  |
  v
Only THEN does TcpListener::bind() happen
  |
  v
Health check finally available
  |
  v
But 60s timeout already expired!
```

## Constraints

### Technical Constraints
- **Server Architecture**: Health endpoint only available after full initialization
- **Thesaurus Building**: Must process all KG markdown files to build automata
- **Document Indexing**: Each document saved to persistence + indexed in rolegraph
- **No Pre-built Automata**: Config has `automata_path: null`

### Performance Data
| Config | KG Files | Haystack Files | Has Pre-built Automata | Expected Startup |
|--------|----------|----------------|------------------------|------------------|
| terraphim_engineer_config.json | 58 | 131 | No | >60s |
| fixtures/test_config.json | 0 | 18 | No | Fast |
| server.rs sample_config() | 0 | 18 | Yes (fixtures/term_to_id.json) | <5s |

## Vital Few (Essentialism)

### Essential Constraints (Max 3)

| Constraint | Why It's Vital | Evidence |
|------------|----------------|----------|
| Tests must use lightweight config | Heavy config causes timeout | 60s not enough for 189 files |
| Config must include all needed roles | Tests verify roles: Terraphim Engineer, Default, Quickwit Logs | Test code references these |
| Tests must have predictable data | Consistent test data = reproducible results | Current config uses live docs/ |

### Eliminated from Scope

| Eliminated Item | Why Eliminated |
|-----------------|----------------|
| Refactoring server to bind before init | Major architectural change, out of scope |
| Parallel thesaurus building | Optimization, not root cause fix |
| Increasing timeout to 120s | Treats symptom, not cause |

## Dependencies

### Internal Dependencies

| Dependency | Impact | Risk |
|------------|--------|------|
| terraphim_server binary | Must be compiled before test | Low - ensure_server_binary() handles |
| ApiClient | Used for server mode search | Low - stable |
| Config files | Test behavior depends on config | **HIGH** - current config causes failure |

### External Dependencies

| Dependency | Version | Risk | Alternative |
|------------|---------|------|-------------|
| portpicker | Current | Low | None needed |
| reqwest | Current | Low | Already used |

## Risks and Unknowns

### Known Risks

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Test config doesn't match production behavior | Medium | Medium | Create minimal config with same roles |
| Fixture data gets stale | Low | Low | Use stable fixtures/ directory |
| Pre-built automata require regeneration | Low | Medium | Document regeneration process |

### Open Questions

1. Should tests use pre-built automata or build on-the-fly with small dataset? - **Recommend pre-built for speed**
2. What's the minimum test data needed to validate cross-mode consistency? - **Small fixture set sufficient**
3. Should we mark these as `#[ignore]` for CI and run separately? - **No, fix the root cause**

### Assumptions Explicitly Stated

| Assumption | Basis | Risk if Wrong | Verified? |
|------------|-------|---------------|-----------|
| 60s timeout insufficient for full config | Test consistently fails at 60s | None - empirically proven | Yes |
| Smaller config will be fast enough | server.rs tests pass in <5s | Could still be slow | No |
| Roles in test match roles in heavy config | Test code references same role names | Tests would fail differently | Yes |

## Research Findings

### Key Insights

1. **Root Cause Identified**: Server health endpoint only available after ALL initialization (thesaurus building, document indexing) completes

2. **Working Pattern Found**: `terraphim_server/tests/server.rs` tests work because they:
   - Use `sample_config()` with pre-built automata (`fixtures/term_to_id.json`)
   - Use small haystack (`fixtures/haystack/` with 18 files)
   - Wait only 5 seconds for server readiness

3. **Config Mismatch**: Test uses heavyweight production-like config while other working tests use lightweight test config

4. **File Counts**:
   - `docs/src/kg/`: 58 markdown files
   - `docs/src/`: 131 markdown files total
   - `fixtures/haystack/`: 18 markdown files

### Relevant Prior Art

- `terraphim_server/tests/server.rs`: Uses lightweight inline config successfully
- `terraphim_server/fixtures/test_config.json`: Pre-existing lightweight config (but needs role updates)
- `terraphim_server/fixtures/term_to_id.json`: Pre-built automata available

## Recommendations

### Proceed/No-Proceed
**PROCEED** - Clear fix path identified

### Scope Recommendations

**Primary Fix**: Create dedicated test config for cross-mode tests that:
1. Uses pre-built automata from `fixtures/term_to_id.json`
2. Uses small haystack from `fixtures/haystack/`
3. Includes roles needed by tests: "Terraphim Engineer", "Default", "Quickwit Logs"
4. Avoids building thesaurus at runtime

**Alternative**: Modify tests to use `sample_config()` pattern from server.rs tests

### Risk Mitigation Recommendations

1. Keep test fixtures stable and versioned
2. Document that cross-mode tests use test fixtures, not production data
3. Consider adding a `#[serial]` attribute to prevent test interference

## Next Steps

If approved:
1. Create `fixtures/cross_mode_test_config.json` with required roles and pre-built automata
2. Update `cross_mode_consistency_test.rs` to use new config
3. Verify tests pass in <30s
4. Update CI if needed

## Appendix

### Reference Materials
- `terraphim_server/tests/server.rs` - Working test pattern
- `terraphim_server/src/lib.rs:167-593` - Server initialization code
- `terraphim_server/default/terraphim_engineer_config.json` - Current (slow) config

### Code Snippets

**Current test config loading** (line 84-88):
```rust
let mut server = Command::new(&binary_path)
    .args([
        "--config",
        "terraphim_server/default/terraphim_engineer_config.json",  // SLOW CONFIG
    ])
```

**Working pattern from server.rs** (line 25-109):
```rust
fn sample_config() -> Config {
    let automata_path = AutomataPath::from_local("fixtures/term_to_id.json");  // PRE-BUILT
    let haystack = "fixtures/haystack".to_string();  // SMALL
    // ... builds lightweight config
}
```
