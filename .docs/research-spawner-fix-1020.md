# Research: Spawner drops TOML `task` body on PR/push fan-out (Gitea #1020)

## Symptom (PR #999, head e870df71)

```
spawner.spawn{provider_id="build-runner" task_len=123}: Agent spawned
spawner.spawn{provider_id="pr-reviewer"  task_len=186}: Agent spawned
... 5 min wall clock ...
agent exit classified agent=pr-reviewer  exit_code=Some(0)   exit_class=success    confidence=1.0
agent exit classified agent=build-runner exit_code=Some(127) exit_class=unknown    confidence=0.0
```

`task_len=123` and `task_len=186` correspond to the two short formatted summaries built by `handle_review_pr` / `handle_push` -- not the multi-line bash scripts in the agent TOML files (70 / 142 lines).

## Root cause

Three sites in `crates/terraphim_orchestrator/src/lib.rs` build a `SpawnRequest` from the runtime informational `task_string` instead of the TOML `def.task` script body:

1. **`lib.rs:2004`** -- `dispatch_pr_reviewer_for_pr`
   ```rust
   let task_string = pr_dispatch::build_review_task(req);
   // ...
   let mut request = SpawnRequest::new(primary_provider, &task_string);
   ```

2. **`lib.rs:2179`** -- `dispatch_build_runner_for_pr`
   ```rust
   let task_string = format!("Build/test verdict for PR #{} ...", ...);
   let mut request = SpawnRequest::new(primary_provider, &task_string);
   ```

3. **`lib.rs:2442`** -- `handle_push`
   ```rust
   let task_string = format!("Build/test verdict for push to {} ...", ...);
   let mut request = SpawnRequest::new(primary_provider, &task_string);
   ```

For comparison, the **working** mention path at `lib.rs:1686` and `flow/executor.rs:260` both pass `def.task` (or its `composed_task` derivative built from `def.task`). Those agents (e.g. `meta-coordinator`, `security-sentinel`) work today.

## Why each agent symptom matches

`crates/terraphim_spawner/src/config.rs:115` -- `infer_args` for `bash`/`sh` returns `["-c"]`. So the spawner runs:
```
/bin/bash -c "<task>"
```

- **build-runner** (`cli_tool=/bin/bash`): receives `bash -c "Build/test verdict for push to refs/..."`. The first token `Build/test` is interpreted as a command and is not on PATH -> exit 127. Matches observed.
- **pr-reviewer** (`cli_tool=/home/alex/.local/bin/claude`): receives `claude -p "Build/test verdict for PR #999 ..."`. Claude treats it as a bare prompt, has nothing meaningful to do, exits 0 without invoking the curl-status-post workflow. Matches observed.

Once `def.task` (the actual bash script) is passed:
- build-runner runs the 70-line script that does `git fetch / git checkout / rch exec / curl status post`.
- pr-reviewer runs the 142-line bash script (under `claude -p`) which itself shells out to claude / curl as designed.

## Phase 1 missing-status (item 4)

`post_pr_reviewer_pending_status` (`lib.rs` ~line 2039) is gated on `get_or_init_pre_check_tracker()` returning `Some(...)`. The branch logs nothing on the skip path, so silent gating is invisible to the operator. Fix: add a `tracing::debug!` inside the gated branch.

## Files to change

- `crates/terraphim_orchestrator/src/lib.rs` -- three `SpawnRequest::new(_, &task_string)` -> `SpawnRequest::new(_, &def.task)`; expose `task_string` as `ADF_TASK_SUMMARY` env var for parity / future hooks.
- `crates/terraphim_orchestrator/src/lib.rs` -- one `tracing::debug!` inside the missing-status gate.
- `crates/terraphim_spawner/tests/fixtures/echo_args_env.sh` -- new fixture that records argv/stdin/env to a temp file.
- `crates/terraphim_spawner/tests/spawn_task_body.rs` -- new integration test: spawn the fixture via the spawner with a Provider whose cli_tool is `/bin/bash`, assert the toml-task content reaches the script.

## Out of scope

- Per-project pr_dispatch (PR #999 / sibling design `.docs/design-pr-dispatch-per-project.md`).
- Phases 2c/2d/2e (operator follow-up only).
- Bigbox deploy (operator step).
