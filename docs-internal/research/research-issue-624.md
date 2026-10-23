# Research Document: Issue #624 - Remove terraphim_repl, Consolidate CLIs

**Status**: Approved
**Author**: Claude Code
**Date**: 2026-03-11
**Reviewers**: Engineering Team

## Executive Summary

Issue #624 requests removing the terraphim_repl crate and consolidating CLIs. Research found that terraphim_repl is already excluded from the workspace but the directory still exists. No nested terraphim_settings duplicates were found. The remaining work is to delete the terraphim_repl directory and verify no references remain.

---

## Essential Questions Check

| Question | Answer | Evidence |
|----------|--------|----------|
| Energizing? | Yes | Cleanup removes dead code |
| Leverages strengths? | Yes | terraphim_agent provides REPL via `--features repl-full` |
| Meets real need? | Yes | Issue #624 explicitly requests this |

**Proceed**: Yes - 3/3 YES

---

## Problem Statement

### Description
Remove the 87-LOC terraphim_repl crate (subset of terraphim_agent) and consolidate CLI functionality into terraphim_agent.

### Impact
Removes redundant code and simplifies the codebase.

### Success Criteria
1. terraphim_repl directory removed
2. `cargo build -p terraphim_agent --features repl-full` succeeds
3. `cargo build -p terraphim_cli` succeeds
4. `cargo test --workspace` passes

---

## Current State Analysis

### Existing Implementation

**Root Cargo.toml exclude list**:
```toml
exclude = [
    # ... other excludes ...
    # Superseded by terraphim_agent
    "crates/terraphim_repl",
    # ...
]
```

**terraphim_repl directory**: EXISTS at `crates/terraphim_repl/`
- Cargo.toml (1.9 KB)
- CHANGELOG.md
- README.md
- src/ directory
- tests/ directory
- assets/ directory

**terraphim_agent**: Provides REPL via `repl-full` feature
```bash
cargo build -p terraphim_agent --features repl-full
```

### Code Locations

| Component | Location | Status |
|-----------|----------|--------|
| terraphim_repl | crates/terraphim_repl/ | **EXISTS but excluded** |
| terraphim_agent | crates/terraphim_agent/ | Active, has REPL feature |
| terraphim_cli | crates/terraphim_cli/ | Active CLI |

### Dependencies

**terraphim_repl dependencies** (from its Cargo.toml):
- tokio
- terraphim_types
- terraphim_settings
- terraphim_config
- terraphim_automata
- terraphim_rolegraph
- terraphim_service
- terraphim_persistence

**Note**: These are all provided by terraphim_agent with `--features repl-full`.

### Nested terraphim_settings Search

```bash
find crates -type d -name "terraphim_settings" -path "*/crates/*/crates/*"
```
**Result**: No nested terraphim_settings directories found.

---

## Constraints

### Technical Constraints
- Must ensure terraphim_agent REPL feature provides equivalent functionality
- Cannot break existing CLI builds

### Business Constraints
- No breaking changes to user-facing functionality
- terraphim_repl is already excluded (no users affected)

---

## Vital Few

### Essential Constraints
| Constraint | Why It's Vital | Evidence |
|------------|----------------|----------|
| terraphim_agent REPL works | Must replace terraphim_repl | Issue requirements |
| No CLI breakage | terraphim_cli must still work | Issue requirements |

### Eliminated from Scope
| Eliminated Item | Why Eliminated |
|-----------------|----------------|
| Nested settings cleanup | No nested directories found |
| .release-plz.toml update | terraphim_repl not in config |

---

## Risks and Unknowns

### Known Risks
| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| terraphim_agent REPL missing features | Low | Low | terraphim_repl was subset, terraphim_agent superset |
| Documentation references | Medium | Low | Search and update docs |

### Open Questions
1. Are there any documentation references to terraphim_repl? - Need to check

---

## Research Findings

### Key Insights
1. **Already Excluded**: terraphim_repl is already in workspace exclude list
2. **No Nested Settings**: No nested terraphim_settings directories found
3. **No .release-plz.toml Entry**: terraphim_repl not in release config (nothing to update)
4. **Directory Exists**: The actual crates/terraphim_repl/ directory still needs removal

### terraphim_repl Content Analysis

**Size**: ~87 LOC (as stated in issue)
**Functionality**: Basic REPL loop using rustyline
**Superseded By**: terraphim_agent with `--features repl-full`

### Verification Commands

```bash
# Verify terraphim_agent REPL builds
cargo build -p terraphim_agent --features repl-full

# Verify terraphim_cli builds
cargo build -p terraphim_cli

# Run tests
cargo test --workspace
```

---

## Recommendations

### Proceed/No-Proceed
**Proceed** - Straightforward cleanup of already-excluded crate.

### Scope
1. Remove `crates/terraphim_repl/` directory entirely
2. Verify workspace builds without it
3. Update any documentation references (if found)

### Risk Mitigation
- Run full workspace check before committing
- Verify terraphim_agent REPL feature works

---

## Next Steps

1. Create design document with implementation steps
2. Remove terraphim_repl directory
3. Run cargo check --workspace
4. Run cargo test --workspace
5. Commit and PR

---

## Appendix

### terraphim_repl Directory Structure
```
crates/terraphim_repl/
├── Cargo.toml
├── CHANGELOG.md
├── README.md
├── assets/
├── src/
│   └── main.rs (87 LOC)
└── tests/
```

### References to Check
- Any documentation mentioning terraphim_repl
- README files
- CHANGELOG entries
