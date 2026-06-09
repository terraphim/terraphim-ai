# Research & Design: Fix PR Gate Producer Agents (#2334)

**Status**: Draft
**Author**: opencode session
**Date**: 2026-06-09
**Issue**: terraphim-ai#2334
**Parent**: terraphim-ai#2301, PR #2318

---

## Phase 1: Research

### Executive Summary

PR gate producer agents (`pr-reviewer`, `pr-validator`, `pr-verifier`) emit unusable output because the orchestrator dispatches them via `pi-rust` with a short `task_string` from `build_review_task()`, while their TOML `task` field contains a full bash script meant for `claude -p`. The orchestrator ignores the TOML task and passes `task_string` as stdin, so `pi-rust` receives only a one-line summary and goes into an unbounded thinking loop trying to read skills and diffs it cannot access. The fix is to make the orchestrator pass the TOML `task` field (the bash script) to PR gate agents, not the routing summary.

### Problem Statement

When a PR webhook arrives, the orchestrator:
1. Builds a short `task_string` via `pr_dispatch::build_review_task()` (one line: "Structural review of PR #N: title...")
2. Creates a `SpawnRequest` with that `task_string` as the prompt
3. Passes `def.task` (the full bash script) via `SpawnRequest::new(provider, &task_string)` -- but only `task_string` becomes the prompt, not `def.task`

In `pr_handlers_impl.rs:269`:
```rust
let mut request = SpawnRequest::new(primary_provider, &task_string);
```

For `implementation-swarm` (non-PR agent), the spawn path at `spawn_impl.rs` correctly passes `&def.task` as the task. For PR gate agents, the `task_string` overrides it.

This means `pi-rust` receives "Structural review of PR #2318: ..." as its only prompt. The agent has no PR diff, no skill instructions, no review template. It tries to figure out what to do by reading skills it cannot access (symlinks point outside the allowed working directory), and enters an infinite thinking loop generating 7000+ lines of streaming JSONL without producing a final answer.

### Root Cause Analysis

There are **two independent bugs** that combine to produce the observed failure:

#### Bug 1: Orchestrator passes routing summary instead of TOML task to PR gate agents

In `pr_handlers_impl.rs:269`:
```rust
let mut request = SpawnRequest::new(primary_provider, &task_string);
```

`task_string` is built by `pr_dispatch::build_review_task()` which produces:
```
"Structural review of PR #2318: Fix #2301: add PrGateResult contract ... (project=terraphim-ai, size=42 LOC, author=..., head=...)"
```

The TOML `task` field contains the full bash script that:
- Exports env vars
- Checks idempotency
- Fetches the PR diff
- Constructs a REVIEW_PROMPT with the diff embedded
- Pipes it through `claude -p --output-format text`
- Posts the comment via `gtr comment`
- Emits the canonical `adf:gate-result` block

This bash script never reaches `pi-rust`. The agent receives only the one-line summary.

**Contrast with `implementation-swarm`**: The normal spawn path in `spawn_impl.rs` passes `&def.task` (the TOML task body) to the spawner. The PR gate path overrides this with `task_string`.

#### Bug 2: pi-rust cannot access skills outside the allowed working directory

The `pi-rust` binary is an ELF executable that appears to use skills from a configured path. The skill symlinks on bigbox point to `/home/alex/terraphim-skills/skills/` and `/home/alex/zestic-ai-development-process/zestic-ai-skills/skills/`. When the agent is spawned in its working directory (e.g. `/data/projects/terraphim/terraphim-ai`), it may not have permission to read skill files outside the agent directory.

Evidence: drain logs show the agent spending thousands of tokens trying to read skill files and encountering access errors.

### Current State Analysis

#### Data Flow (Current - Broken)

```
PR webhook
  -> handle_review_pr()
    -> build_review_task(req) -> "Structural review of PR #2318: ..."
    -> SpawnRequest::new(provider, &task_string)  // BUG: should be &def.task
    -> pi-rust receives one-line summary as prompt
    -> pi-rust enters thinking loop, reads no diff, accesses no skills
    -> 300s timeout kills agent
    -> Orchestrator fails closed with canonical failure comment
```

#### Data Flow (Expected - Fixed)

```
PR webhook
  -> handle_review_pr()
    -> build_review_task(req) -> stored as ADF_TASK_SUMMARY env
    -> SpawnRequest::new(provider, &def.task)      // Use TOML bash script
    -> pi-rust/bash receives full script
    -> Script fetches PR diff, constructs REVIEW_PROMPT
    -> LLM reviews diff, posts comment, emits gate-result block
    -> Agent exits cleanly within 300s
    -> Orchestrator parses gate-result, posts terminal status
```

#### Key Files

| Component | Location | Purpose |
|-----------|----------|---------|
| `pr_handlers_impl.rs` | `crates/terraphim_orchestrator/src/` | PR fan-out spawn; line 269 is the bug |
| `pr_dispatch.rs` | `crates/terraphim_orchestrator/src/` | `build_review_task()` builds routing summary |
| `spawn_impl.rs` | `crates/terraphim_orchestrator/src/` | Normal agent spawn (correctly uses `def.task`) |
| `reconcile_impl.rs` | `crates/terraphim_orchestrator/src/` | Consumes gate results from drain logs |
| `pr_gate_result.rs` | `crates/terraphim_orchestrator/src/` | Canonical gate result parser |
| `terraphim.toml` | `/opt/ai-dark-factory/conf.d/` | Agent configs with bash task scripts |
| `orchestrator.toml` | `/opt/ai-dark-factory/` | ADF orchestrator config |

#### How Each Producer Agent Works

All three agents (`pr-reviewer`, `pr-validator`, `pr-verifier`) share the same structure:

1. **Env validation**: Check `ADF_PR_NUMBER` and `ADF_PR_PROJECT`
2. **Idempotency**: Skip if a comment for the same head SHA exists within 2 hours
3. **Diff fetch**: `git fetch gitea "pull/$N/head:pr-$N"` then `git diff main...pr-$N`
4. **Prompt construction**: Build `REVIEW_PROMPT` with diff, metadata, output template
5. **LLM invocation**: Pipe `REVIEW_PROMPT` into `claude -p --output-format text`
6. **Output filter**: Python script strips JSON tool-use events
7. **Verdict extraction**: Grep for "Verdict: pass|concerns|fail"
8. **Gate result emission**: The prompt instructs the LLM to emit `<!-- adf:gate-result { ... } -->`
9. **Exit code**: Map verdict to exit 0/1

The TOML `cli_tool` is `pi-rust`, but the bash script inside `task` invokes `claude -p` directly. This means:
- `pi-rust` is the outer wrapper that manages the process
- The bash script is the actual task that runs
- The bash script calls `claude` explicitly

### Constraints

1. **No orchestrator code change for the bash scripts**: The bash scripts in TOML `task` fields are config, not code. They are updated by editing `terraphim.toml` on bigbox.
2. **pi-rust binary is compiled**: Cannot modify pi-rust behaviour without a build cycle.
3. **Working directory isolation**: Agents run in the project working directory; skill symlinks must be accessible from there.
4. **300s wall-clock timeout**: Hard limit from PR #2318; producers must complete well within this.
5. **Fail-closed contract**: Orchestrator already fails closed; producers just need to produce valid output.

### Risks

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Spawner ignores `def.task` for PR agents | Low (code is clear) | High | Verify in spawn_impl.rs |
| `pi-rust` does not handle bash scripts as input | Medium | High | Test manually on bigbox |
| Bash script `claude` invocation conflicts with pi-rust wrapper | Medium | High | May need to change cli_tool |
| Skills still inaccessible after task fix | Low | Medium | Skills are read by `claude`, not pi-rust |
| 2000-line diff cap too small for large PRs | Low | Low | Increase to 4000 or make configurable |

### Assumptions

| Assumption | Basis | Risk if Wrong |
|------------|-------|---------------|
| The spawner passes the `SpawnRequest` task string as stdin to the cli_tool | Spawner code pattern | pi-rust ignores stdin and does its own thing |
| `pi-rust` can execute bash scripts | It's an ELF binary; unknown if it handles shell | Need to verify or change cli_tool |
| The TOML bash scripts work when run directly | They were designed for this | They may have bit-rotted |
| `claude` is available on bigbox | `/home/alex/.local/bin/claude` exists in PATH | Claude CLI may not be authenticated |

### Key Insight

The fundamental issue is that `dispatch_pr_reviewer_for_pr()` builds a `SpawnRequest` with `task_string` (the routing summary) instead of `def.task` (the bash script). Comment at line 265 says:

> Bug #2450 fix: pr-reviewer agent was receiving `def.task` ("review") instead of `task_string`

This comment is misleading. The original bug was that `def.task` was the placeholder string "review" rather than the full script. The fix switched to `task_string`, but `task_string` is also wrong -- it's the routing summary, not the bash script. The correct fix is to ensure `def.task` contains the actual bash script (which it does in the live config) and pass it to the spawner.

---

## Phase 2: Design

### Approach

**Switch PR gate agents from `pi-rust` to `claude` as cli_tool and pass the TOML `task` field as the spawn task.**

This is the simplest fix because:
1. The TOML `task` field is already a bash script that invokes `claude -p` directly.
2. Using `pi-rust` as the outer wrapper adds no value when the inner script already calls `claude`.
3. The orchestrator code change is minimal: use `&def.task` instead of `&task_string` in the PR gate spawn path.
4. The TOML config change is minimal: change `cli_tool` from `pi-rust` to `/bin/bash` for the three PR gate agents (or use `claude` directly).

Wait -- there's a subtlety. The spawner runs `cli_tool` with the task string as stdin or argument. Let me check how the spawner invokes the tool.

### Spawner Investigation Needed

Need to verify: does `terraphim_spawner` pass the `SpawnRequest` task as:
- (a) stdin to the cli_tool process?
- (b) as a `-p` argument?
- (c) as `bash -c "$task"`?

The answer determines whether we can pass a bash script as the task string.

If (a) stdin: The bash script will be piped into `pi-rust`, which may or may not execute it.
If (c) bash -c: The script will execute directly.

### Design Option A: Fix the task string (Minimal orchestrator code change)

Change `pr_handlers_impl.rs:269` from:
```rust
let mut request = SpawnRequest::new(primary_provider, &task_string);
```
to:
```rust
let mut request = SpawnRequest::new(primary_provider, &def.task);
```

This passes the TOML bash script as the task. The routing summary is still available via `ADF_TASK_SUMMARY` env var (already set at line 292).

**Risk**: If the spawner passes the task as stdin to `pi-rust`, pi-rust may not execute the bash script correctly.

### Design Option B: Change cli_tool to bash for PR gate agents

In the TOML config, change:
```toml
cli_tool = "/home/alex/.local/bin/pi-rust"
```
to:
```toml
cli_tool = "/bin/bash"
```

And keep using `&def.task` as the spawn task. Bash will execute the script directly.

**Risk**: The spawner may pass the task as a `-c` argument, which works for bash. Need to verify.

### Design Option C: Keep pi-rust but embed the REVIEW_PROMPT in the task string

Instead of the bash script, construct a direct prompt for pi-rust that includes:
- The PR diff
- The review template
- The gate-result emission instructions
- The env metadata

This bypasses the bash script entirely and lets pi-rust handle the review directly.

**Risk**: pi-rust may not handle large prompts well (we saw it enter thinking loops with small prompts). The bash script approach with explicit `claude -p` invocation is more predictable.

### Recommended Approach: Option A + B

1. **Orchestrator code change**: Use `&def.task` instead of `&task_string` for PR gate agents.
2. **Config change**: Change `cli_tool` to `/bin/bash` for `pr-reviewer`, `pr-validator`, `pr-verifier`.
3. **No new dependencies**: The bash scripts already work; they just need to be passed to the right executor.

### Simplicity Check

> "What if this could be easy?"

The simplest fix is a one-line code change in `pr_handlers_impl.rs` and three `cli_tool` edits in `terraphim.toml`. No new modules, no new types, no new tests beyond verifying the spawner handles bash scripts correctly.

**Senior Engineer Test**: This is a bug fix, not a redesign. The system was designed to work this way; it just has the wrong task string.

### File Changes

#### Modified Files (Orchestrator)

| File | Changes |
|------|---------|
| `crates/terraphim_orchestrator/src/pr_handlers_impl.rs:269` | Change `&task_string` to `&def.task` |

#### Modified Files (Config on bigbox)

| File | Changes |
|------|---------|
| `/opt/ai-dark-factory/conf.d/terraphim.toml` | Change `cli_tool` to `/bin/bash` for pr-reviewer, pr-validator, pr-verifier |

#### No New Files

This is a two-line fix (one code, one config pattern applied three times).

### Test Strategy

#### Unit Tests

| Test | Purpose |
|------|---------|
| Verify `def.task` reaches `SpawnRequest` | Ensure the task field is not overridden |

#### Integration Test (Manual on bigbox)

1. Build the fix in the isolated worktree
2. Deploy to bigbox with backup
3. Restart `adf-orchestrator.service`
4. Trigger synthetic PR webhook
5. Verify agents complete within 300s
6. Verify canonical `adf:gate-result` blocks are emitted
7. Verify terminal commit statuses are posted

### Implementation Steps

#### Step 1: Verify spawner behaviour
**Files**: `crates/terraphim_spawner/src/`
**Description**: Read the spawner code to confirm how `SpawnRequest` task string reaches the child process. Determine if it's stdin, `-c` argument, or something else.
**Tests**: None needed (read-only investigation)
**Estimated**: 15 minutes

#### Step 2: Fix orchestrator code
**Files**: `crates/terraphim_orchestrator/src/pr_handlers_impl.rs`
**Description**: Change line 269 from `&task_string` to `&def.task`. Update the comment to explain why.
**Tests**: Existing `pr_handlers_impl` tests should still pass; the task string is used for routing only.
**Dependencies**: Step 1 (must understand spawner first)
**Estimated**: 15 minutes

#### Step 3: Fix agent configs
**Files**: `/opt/ai-dark-factory/conf.d/terraphim.toml` (on bigbox)
**Description**: Change `cli_tool` from `pi-rust` to `/bin/bash` for the three PR gate agents. Back up config first.
**Tests**: Manual verification
**Dependencies**: Step 1
**Estimated**: 10 minutes

#### Step 4: Build and deploy
**Files**: Isolated worktree
**Description**: Build the orchestrator binary, deploy to bigbox, restart service.
**Tests**: Build passes, service starts.
**Dependencies**: Steps 2, 3
**Estimated**: 30 minutes (includes remote build)

#### Step 5: Live verification
**Description**: Trigger synthetic PR webhook, monitor agent output, verify gate results.
**Tests**: Agents complete within 300s, emit valid gate-result blocks, post terminal statuses.
**Dependencies**: Step 4
**Estimated**: 15 minutes

### Rollback Plan

1. Revert `pr_handlers_impl.rs` change
2. Restore `terraphim.toml` from backup
3. Restore `/usr/local/bin/adf` from backup
4. Restart service

### Open Items

| Item | Status | Owner |
|------|--------|-------|
| Verify spawner task-passing mechanism | Pending | opencode session |
| Verify `claude` CLI is authenticated on bigbox | Pending | Manual check |
| Consider if pi-rust should be kept for non-bash agents | Deferred | Post-fix decision |

---

## Appendix: Spawner Task Passing (Confirmed)

### How `SpawnRequest.task` reaches the child process

**Confirmed**: `terraphim_spawner/src/lib.rs:681-699` and `config.rs:159-178`.

When `use_stdin` is false (the default), the spawner constructs:

```rust
let mut cmd = Command::new(&config.cli_command);
cmd.current_dir(working_dir).args(&config.args);  // args from infer_args()
cmd.arg(task);                                     // task as positional argument
cmd.stdin(Stdio::null());
```

The `infer_args()` function returns CLI-specific arguments based on the binary name:

| cli_tool binary | Inferred args | Final command |
|-----------------|---------------|---------------|
| `pi-rust` | `["-p", "--mode", "json"]` | `pi-rust -p --mode json "<task>"` |
| `claude` | `["-p", "--allowedTools=..."]` | `claude -p --allowedTools=... "<task>"` |
| `bash` | `["-c"]` | `bash -c "<task>"` |
| `opencode` | `["run", "--format", "json"]` | `opencode run --format json "<task>"` |

**Key finding**: `bash` is already a first-class supported cli_tool. When `cli_tool = "/bin/bash"`, the spawner runs `bash -c "<task>"`, which executes the task string as a bash script.

This confirms the recommended approach:
1. Change `cli_tool` from `pi-rust` to `/bin/bash` in the TOML config
2. Pass `&def.task` (the bash script) instead of `&task_string` (the routing summary)
3. The spawner will execute `/bin/bash -c "<full bash script>"`

No code changes needed in the spawner -- it already handles this case.

### Why the current setup fails

With `cli_tool = "pi-rust"` and `task_string = "Structural review of PR #2318: ..."`:

1. Spawner runs: `pi-rust -p --mode json --provider zai-coding-plan --model glm-5.1 "Structural review of PR #2318: ..."`
2. `pi-rust` receives the one-line routing summary as its prompt
3. `pi-rust` has no PR diff, no skill instructions, no review template
4. The model enters an extended thinking loop trying to understand what to do
5. It reads skills it cannot access, generating errors
6. 7000+ lines of streaming JSONL output with no useful content
7. 300s timeout kills the agent
8. Orchestrator fails closed with canonical failure envelope

### Why the fix works

With `cli_tool = "/bin/bash"` and `def.task = "<bash script>"`:

1. Spawner runs: `bash -c "export GITEA_URL=... && ... && REVIEW_OUTPUT=$(echo \"$REVIEW_PROMPT\" | claude -p ...)"`
2. The bash script executes:
   - Validates env vars
   - Checks idempotency
   - Fetches the PR diff
   - Constructs the REVIEW_PROMPT with embedded diff
   - Pipes it through `claude -p` explicitly
   - Filters JSON tool-use events
   - Extracts verdict
   - Posts comment via `gtr comment`
   - Emits canonical `adf:gate-result` block
3. Agent exits cleanly (typically in 60-180s depending on diff size)
4. Orchestrator parses the gate-result block
5. Terminal commit status reflects the actual review outcome
