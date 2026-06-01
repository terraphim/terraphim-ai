You are executing the **disciplined-research** skill (Phase 1 of disciplined development). Your output will be evaluated by the disciplined-quality-evaluation skill before the next phase can proceed.

## Task

Research the spawn-context isolation bug in the ADF orchestrator. The hypothesis: `spawn_agent_with_event()` creates an isolated git worktree but the spawned agent process may run in the project root because `SpawnContext.working_dir` takes priority over the provider's `working_dir` in `AgentSpawner::spawn_process()`.

## Research Steps

1. Read `crates/terraphim_orchestrator/src/lib.rs` lines 2288-2418. Trace:
   - How `agent_working_dir` is computed (line 2329)
   - How the provider `working_dir` receives it (line 2338)
   - How `build_spawn_context_for_agent` is called (line 2404)
   - Whether `spawn_ctx` carries its own `working_dir` and whether it matches the worktree path

2. Read `build_spawn_context_for_agent` at `crates/terraphim_orchestrator/src/lib.rs` around line 621-643. What directory does it use?

3. Read `crates/terraphim_spawner/src/lib.rs` lines 649-665 (`spawn_process`). Confirm the priority order: `ctx.working_dir` > `config.working_dir` > `spawner.default_working_dir`.

4. Determine: **is the bug real?** Does `spawn_ctx.working_dir` (set to the project root by `build_spawn_context_for_agent`) override `agent_working_dir` (set on the provider)?

## Output

Write your findings to `.docs/adf/1806/research.md` in this format:

```markdown
# Phase 1 Research: Spawn-Context Isolation Bug

**Skill**: disciplined-research
**Issue**: 1806
**Date**: [today]

## Bug Confirmation

[YES/NO - with line-number evidence]

## Root Cause

[Specific lines and why the override happens]

## Impact

[What actually happens vs what should happen]

## Existing Safeguards

[What currently mitigates this, if anything]

## Recommendation

[What should change - single sentence]
```

**Skill evidence requirement**: Begin your output with "Skill: disciplined-research invoked" and reference specific code locations by file:line. Do NOT modify any code.
