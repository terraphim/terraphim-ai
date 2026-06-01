You are executing the **disciplined-design** skill (Phase 2 of disciplined development). You have already read the Phase 1 research at `.docs/adf/1806/research.md`. Read it now if you need to refresh.

## Principles

- **Essentialism**: Eliminate before adding. The minimum viable fix.
- **Nothing speculative**: No features the user did not request.
- **Interface-first**: Define the change before implementing.

## Task

Produce a minimal implementation plan to fix the spawn-context isolation bug confirmed in Phase 1.

## Design Constraints

- Modify **only** `crates/terraphim_orchestrator/src/lib.rs`
- The fix must layer `agent_working_dir` into `spawn_ctx` after it is computed (around line 2404) but before `spawner.spawn_with_fallback()` is called
- Maximum 3 lines of code changed
- Must not break existing tests
- Must add one assertion test

## Output

Write to `.docs/adf/1806/design.md` in this format:

```markdown
# Phase 2 Design: Spawn-Context Isolation Fix

**Skill**: disciplined-design
**Issue**: 1806

## Problem Summary

[One paragraph from research]

## Architecture

[ASCII diagram of the fix - before and after]

## File Changes

| File | Change | Lines |
|------|--------|-------|
| lib.rs | [description] | ~3 |

## Proposed Change

```rust
// Before (line 2404):
let mut spawn_ctx =
    build_spawn_context_for_agent(&self.config, def, self.output_poster.as_ref());

// After:
let mut spawn_ctx =
    build_spawn_context_for_agent(&self.config, def, self.output_poster.as_ref());
spawn_ctx.working_dir = Some(agent_working_dir.clone());
```

## Test Plan

| Test | What it verifies |
|------|-----------------|
| [test name] | spawn_ctx.working_dir is set to worktree path after isolation |

## Eliminated Options

| Option | Why rejected |
|--------|-------------|
| [option] | [reason] |

## Rollback

Revert the 1-2 line addition.
```

**Skill evidence requirement**: Begin your output with "Skill: disciplined-design invoked". Include the 5/25 rule analysis - list at least 3 things you explicitly chose NOT to fix in this PR. Do NOT modify any code.
