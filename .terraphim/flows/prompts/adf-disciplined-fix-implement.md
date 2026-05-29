You are executing the **disciplined-implementation** skill (Phase 3 of disciplined development). You have already produced:
- Phase 1 research: `.docs/adf/1806/research.md`
- Phase 2 design: `.docs/adf/1806/design.md`

Read both files now to understand what to implement.

## Principles

- Follow the design exactly -- no scope creep
- Each step is independently reviewable
- All tests pass before reporting success
- Zero clippy warnings on new code

## Task

Implement the fix specified in the design document, run tests, and verify correctness.

## Implementation Steps

### Step 1: Apply the code change

Edit `crates/terraphim_orchestrator/src/lib.rs`. Find the line where `build_spawn_context_for_agent` is called (around line 2404). After that line, add a line that sets `spawn_ctx.working_dir` to `Some(agent_working_dir.clone())`.

### Step 2: Add a unit test

Add a test to the existing test module in `lib.rs` that verifies `agent_working_dir` is correctly set on the spawn context after worktree creation. The test should NOT require a real git worktree -- it should verify the logic, not the I/O.

### Step 3: Run quality gates

```bash
cargo fmt
cargo test -p terraphim_orchestrator --lib
cargo clippy -p terraphim_orchestrator
```

### Step 4: Report results

Write to `.docs/adf/1806/verification.md`:

```markdown
# Phase 3 Verification: Spawn-Context Isolation Fix

**Skill**: disciplined-implementation
**Issue**: 1806

## Change Applied

[What file, what line, what the diff looks like]

## Test Results

[Output of cargo test]

## Clippy

[Output of cargo clippy]

## Self-Check

- [ ] Design followed exactly
- [ ] All tests pass
- [ ] No new clippy warnings
- [ ] No scope creep
```

**Skill evidence requirement**: Begin your output with "Skill: disciplined-implementation invoked". Include the exact `git diff` of your change in the verification report. If any test fails, fix the code and retry before writing the report.
