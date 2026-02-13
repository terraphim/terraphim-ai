# Implementation Plan: Terraphim Agent & CLI Status Assessment

**Status**: Draft
**Research Doc**: `.docs/research-terraphim-agent-cli-status.md`
**Author**: pi
**Date**: 2026-02-10
**Estimated Effort**: 1-2 days

## Overview

### Summary
Plan a focused status assessment for `terraphim_agent` and `terraphim_cli` to validate current behavior, document gaps, and produce a prioritized list of follow-up issues. This plan is documentation- and verification-focused; it does not include new features or code changes unless explicitly approved later.

### Approach
- Confirm current behavior through targeted smoke tests.
- Compare feature parity between the two CLIs where overlap is expected.
- Document findings and create follow-up issues for gaps/risks.

### Scope
**In Scope:**
- Update documentation with a status/health report.
- Run minimal smoke tests for core commands (search, roles, config, replace).
- Identify and log risks (server dependency, config fallback differences, update path).

**Out of Scope:**
- Implementing feature changes.
- UI/UX redesign or refactor.
- Performance benchmarking beyond quick smoke checks.

**Avoid At All Cost:**
- Expanding scope to unrelated crates.
- Introducing new configuration formats.

## Architecture

### Component Diagram
```
[CLI Entry] -> [Service Wrapper] -> [TerraphimService] -> [Config/Rolegraph/Automata]
```

### Data Flow
```
User Input -> clap -> Service wrapper -> terraphim_service -> Output JSON/Text
```

### Key Design Decisions
| Decision | Rationale | Alternatives Rejected |
|----------|-----------|----------------------|
| Focus on verification + docs | Minimizes risk; aligns with status check request | Implementing new features without approval |
| Use smoke tests only | Time-boxed, avoids heavy CI | Full regression testing |

### Eliminated Options (Essentialism)
| Option Rejected | Why Rejected | Risk of Including |
|----------------|--------------|-------------------|
| Full feature parity refactor | Not requested | Scope creep |
| New CLI subcommands | No requirements | Added maintenance burden |

### Simplicity Check
**What if this could be easy?**
Keep the plan to documentation plus minimal command checks. No code changes unless findings are severe and approved.

## File Changes

### New Files
| File | Purpose |
|------|---------|
| `.docs/status-terraphim-agent.md` | Status findings for `terraphim_agent` |
| `.docs/status-terraphim-cli.md` | Status findings for `terraphim_cli` |

### Modified Files
| File | Changes |
|------|---------|
| `.docs/summary.md` | Link to new status docs |

### Deleted Files
| File | Reason |
|------|--------|
| None | N/A |

## API Design

No new APIs. Status assessment is documentation + verification only.

## Test Strategy

### Smoke Tests
| Test | Location | Purpose |
|------|----------|---------|
| `terraphim-agent --help` | CLI | Ensure command parsing available |
| `terraphim-agent roles list` | CLI | Confirm role listing in offline mode |
| `terraphim-cli search <query>` | CLI | Validate search output JSON |
| `terraphim-cli roles list` | CLI | Confirm role listing |

### Integration Tests
None planned for this status check unless smoke tests reveal regressions.

## Implementation Steps

### Step 1: Gather Command Inventory
**Files:** `.docs/status-terraphim-agent.md`, `.docs/status-terraphim-cli.md`
**Description:** Document current command surface and dependencies.
**Tests:** N/A
**Estimated:** 2 hours

### Step 2: Run Smoke Tests
**Files:** `.docs/status-terraphim-agent.md`, `.docs/status-terraphim-cli.md`
**Description:** Execute minimal commands to confirm behavior and capture output summaries.
**Tests:** Manual CLI runs
**Estimated:** 2-3 hours

### Step 3: Risk and Gap Analysis
**Files:** `.docs/status-terraphim-agent.md`, `.docs/status-terraphim-cli.md`
**Description:** Capture gaps, risks, and follow-up issues.
**Tests:** N/A
**Estimated:** 1-2 hours

### Step 4: Update Summary
**Files:** `.docs/summary.md`
**Description:** Link to status docs and summarize outcomes.
**Tests:** N/A
**Estimated:** 1 hour

## Rollback Plan

No code changes planned. If documentation is incorrect, revert docs only.

## Dependencies

### New Dependencies
None.

### Dependency Updates
None.

## Performance Considerations

No performance changes planned. Smoke tests may be used to spot obvious regressions.

## Open Items

| Item | Status | Owner |
|------|--------|-------|
| Approval to proceed with smoke tests | Pending | User |

## Approval

- [ ] Technical review complete
- [ ] Test strategy approved
- [ ] Human approval received
