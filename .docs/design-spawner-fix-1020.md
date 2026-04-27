# Design: Pass TOML `task` body to spawner on PR/push fan-out (Gitea #1020)

## Approach

Mirror the working mention-dispatch and `flow/executor.rs` shape: pass `def.task` as the task body to the spawner, layering the runtime informational string as an additional environment variable (`ADF_TASK_SUMMARY`) so future TOML scripts can opt in to it without a new code change.

## Step 1 -- Spawner unit test (TDD)

**Files**:
- New `crates/terraphim_spawner/tests/fixtures/echo_args_env.sh` -- bash fixture that writes argv joined by `|`, stdin, and selected env vars to `$RECORD_FILE`.
- New `crates/terraphim_spawner/tests/spawn_task_body.rs` -- two `#[tokio::test]` cases:
  1. `bash_provider_receives_task_body_via_dash_c` -- builds a `Provider` with `cli_command=/bin/bash`, spawns with a multi-line task containing a `printf` pipeline, asserts the fixture recorded the expected output (proves `bash -c "<script>"` shape).
  2. `env_overrides_are_visible_to_spawned_process` -- asserts that `SpawnContext::with_env("ADF_TASK_SUMMARY", "summary")` is visible to the child.

These tests use the real spawner, no mocks. The fixture script is small (~10 lines) and self-contained.

**Gate**: `cargo test -p terraphim_spawner --test spawn_task_body` red, then green after Step 2.

## Step 2 -- Fix the three orchestrator dispatch sites

**File**: `crates/terraphim_orchestrator/src/lib.rs`

For each of the three sites:

1. `dispatch_pr_reviewer_for_pr` (line 2004): replace `SpawnRequest::new(primary_provider, &task_string)` with `SpawnRequest::new(primary_provider, &def.task)`. Layer `spawn_ctx = spawn_ctx.with_env("ADF_TASK_SUMMARY", task_string.clone())`.
2. `dispatch_build_runner_for_pr` (line 2179): same swap. The existing `spawn_ctx.with_env("ADF_PUSH_*", ...)` chain extends with `.with_env("ADF_TASK_SUMMARY", task_string.clone())`.
3. `handle_push` (line 2442): same swap and env layering.

`task_string` becomes the value of `ADF_TASK_SUMMARY` (informational only; existing TOML scripts ignore it). The behaviour change:
- bash agents now run their multi-line script body via `bash -c`.
- LLM agents now receive their TOML task body as the prompt for `claude -p`.

**Gate**: `cargo test -p terraphim_orchestrator` green, `cargo clippy -p terraphim_orchestrator -p terraphim_spawner --all-targets -- -D warnings` clean.

## Step 3 -- Phase 1 visibility

Add a `tracing::debug!` inside the `post_pr_reviewer_pending_status` gated branch so future runs reveal whether the gate is the cause of missing `pending` statuses.

**File**: `crates/terraphim_orchestrator/src/lib.rs` near line 2039.

**Gate**: same as Step 2.

## Risk assessment

- **Behaviour change blast radius**: all three sites are exercised exclusively by PR-open / push events. Mention-dispatch (`lib.rs:1686`) is unchanged and continues to use `composed_task`.
- **Compile-time safety**: `def.task` is `String` (orchestrator config), `task_string` is `String`; both produce `&str` via deref. No type signature change.
- **No new dependencies**, no new feature flags.
- **Disk discipline**: build/test scoped to two crates only; no `--workspace`.

## Commit plan

1. `test(spawner): cover task body delivery via bash -c invocation Refs #1020` -- fixture + tests (red without fix).
2. `fix(orchestrator): pass TOML task body to spawner on PR/push fan-out Refs #1020` -- the three `SpawnRequest::new` swaps + `ADF_TASK_SUMMARY` env layering.
3. `feat(orchestrator): trace missing pr-reviewer pending status gate Refs #1020` -- one `tracing::debug!` in the gated branch.

Each commit independently passes `cargo test -p <crate>` and `cargo clippy -p <crate> -- -D warnings`.
