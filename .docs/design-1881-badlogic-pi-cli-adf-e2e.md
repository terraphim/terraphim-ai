# Implementation and Evidence Plan: badlogic/pi CLI Support via ADF Local Dispatch

**Status**: Review - requires approval before implementation
**Research Doc**: `.docs/research-1881-badlogic-pi-cli-adf-flow.md`
**Issue**: terraphim/terraphim-ai#1881
**Skills evidenced**: disciplined-design, disciplined-implementation, structural-pr-review, disciplined-verification, disciplined-validation

## Overview

### Summary

Implement first-class badlogic/pi CLI support in `terraphim_spawner`, then prove the complete issue lifecycle through `adf-ctl --local trigger --direct` from `.terraphim/adf.toml`.

### Corrected Approach

The proof must not hardcode issue #1881 into reusable scripts or agent config. The target issue is supplied at runtime as explicit dispatch context:

```bash
./target/debug/adf-ctl --local trigger disciplined-research-agent --direct \
  --context 'issue=1881 stage=disciplined-research skill=disciplined-research'
```

Each ADF-spawned stage parses `issue=...` and `stage=...` from the task/context, posts progress to that Gitea issue, and writes/validates the expected artefact.

## Scope

### In Scope

- `AgentConfig::infer_args("pi")` uses badlogic/pi shape: `prompt` plus model name when configured.
- `AgentConfig::model_args("pi", model)` appends model name for `pi prompt <model> <task>`.
- `pi-rust` behaviour remains unchanged.
- Tests cover `pi`, `pi-rust`, full-path binary names, and command argument construction via a temporary executable.
- ADF local direct dispatch runs all lifecycle stages and posts evidence to issue #1881.

### Out of Scope

- GPU pod provisioning.
- Real vLLM model startup.
- Changing production agent defaults to badlogic/pi.
- Replacing `pi-rust` routing.

### Avoid At All Cost

- Hardcoded issue IDs in `.terraphim/adf.toml` or reusable stage scripts.
- Treating badlogic `pi` and `pi-rust` as aliases.
- Proof that only invokes scripts directly instead of through `adf-ctl` and the daemon.

## Architecture

### Components

```text
Gitea Issue #1881
  ^ comments/evidence via gtr
  |
ADF-spawned stage process
  ^ spawned by AgentSpawner
  |
AgentOrchestrator direct dispatch handler
  ^ WebhookDispatch::SpawnAgent
  |
Unix socket /tmp/adf-ctl.sock
  ^ JSON command { agent, context }
  |
adf-ctl --local trigger --direct
```

### badlogic/pi Command Shape

For configured model `phi3` and task `hello`:

```text
pi prompt phi3 hello
```

For no configured model, design will fail validation or use documented default only if upstream supports it. The preferred path is to require/provide a model for badlogic/pi agents because upstream README examples require a model alias for `prompt`.

## File Changes

### Modified Files

| File | Changes |
|------|---------|
| `crates/terraphim_spawner/src/config.rs` | Split `pi` from `pi-rust` in `infer_args`, `model_args`, tests. |
| `crates/terraphim_spawner/src/lib.rs` | Add or adjust integration test using a temporary `pi` executable to prove argv shape without mocks. |
| `.terraphim/adf.toml` | Keep only generic ADF proof agents; no issue-specific hardcoding. |
| `.terraphim/bin/adf-e2e-stage` | Parse `issue=` and `stage=` from context dynamically; no default issue. |
| `.docs/*1881*` | Research, implementation plan, review, verification, validation evidence. |

## API Design

No public Rust API changes are required. Internal CLI inference rules change as follows:

```rust
fn infer_args(cli_command: &str) -> Vec<String> {
    match cli_name(cli_command) {
        "pi-rust" => vec!["-p", "--mode", "json"],
        "pi" => vec!["prompt"],
        // existing arms unchanged
    }
}

fn model_args(cli_command: &str, model: &str) -> Vec<String> {
    match cli_name(cli_command) {
        "pi-rust" => provider_model_flags(model),
        "pi" => vec![model.to_string()],
        // existing arms unchanged
    }
}
```

## Test Strategy

### Unit Tests

| Test | Location | Purpose |
|------|----------|---------|
| `test_infer_args_pi_badlogic` | `terraphim_spawner/src/config.rs` | `pi` maps to `prompt`. |
| `test_model_args_pi_badlogic` | `terraphim_spawner/src/config.rs` | model becomes positional model alias. |
| Existing `test_infer_args_pi_rust` | `terraphim_spawner/src/config.rs` | prove no regression. |
| Existing `test_model_args_pi_rust_*` | `terraphim_spawner/src/config.rs` | prove provider/model flags remain. |

### Integration Tests

| Test | Location | Purpose |
|------|----------|---------|
| `test_spawn_pi_receives_prompt_model_and_task` | `terraphim_spawner/src/lib.rs` | Temporary executable named `pi` records argv; spawner must call `pi prompt <model> <task>`. |

No mocks are used. The integration test uses a real temporary executable and filesystem output.

### ADF CLI Evidence Tests

Each step is run through `adf-ctl`:

```bash
./target/debug/adf .terraphim/adf.toml
./target/debug/adf-ctl --local trigger disciplined-research-agent --direct --context 'issue=1881 stage=disciplined-research skill=disciplined-research'
./target/debug/adf-ctl --local trigger implementation-plan-agent --direct --context 'issue=1881 stage=implementation-plan skill=disciplined-design'
./target/debug/adf-ctl --local trigger disciplined-implementation-agent --direct --context 'issue=1881 stage=disciplined-implementation skill=disciplined-implementation'
./target/debug/adf-ctl --local trigger structured-pr-review-agent --direct --context 'issue=1881 stage=structured-pr-review skill=structural-pr-review'
./target/debug/adf-ctl --local trigger disciplined-verification-agent --direct --context 'issue=1881 stage=disciplined-verification skill=disciplined-verification'
./target/debug/adf-ctl --local trigger disciplined-validation-agent --direct --context 'issue=1881 stage=disciplined-validation skill=disciplined-validation'
```

Required daemon evidence for every stage:

- `direct dispatch: spawning agent agent=<stage-agent>`
- isolated worktree creation
- `AgentSpawned`
- `agent exit classified ... exit_class=success`
- `core agent completed ... exit status: 0`
- worktree cleanup

Required Gitea evidence for every stage:

- A comment on #1881 naming the stage.
- The exact skill used.
- The artefact path or command output produced.
- The ADF command used for the stage.

## Implementation Steps

### Step 1: Correct ADF Evidence Harness

**Skill**: disciplined-design / disciplined-implementation
**Files**: `.terraphim/adf.toml`, `.terraphim/bin/adf-e2e-stage`
**Goal**: remove hardcoded issue IDs and make the stage script reject missing `issue=`.
**Verification**: trigger one stage with `issue=1881`, confirm Gitea comment; trigger without issue in dry run, confirm non-zero failure.

### Step 2: Implement badlogic/pi Argument Support

**Skill**: disciplined-implementation
**Files**: `crates/terraphim_spawner/src/config.rs`
**Goal**: separate `pi` from `pi-rust`.
**Verification**: unit tests for both CLIs.

### Step 3: Add Real Spawn Integration Test

**Skill**: disciplined-implementation
**Files**: `crates/terraphim_spawner/src/lib.rs`
**Goal**: prove actual process argv uses `pi prompt <model> <task>`.
**Verification**: cargo test for the new integration test.

### Step 4: Structured PR Review

**Skill**: structural-pr-review
**Goal**: review the local diff against architecture and regressions.
**Evidence**: `.docs/pr-review-1881-badlogic-pi-cli.md`, Gitea issue comment.

### Step 5: Verification

**Skill**: disciplined-verification
**Commands**:

```bash
cargo test -p terraphim_spawner
cargo test -p terraphim_orchestrator --bin adf-ctl
cargo fmt --check
cargo clippy -p terraphim_spawner
ubs crates/terraphim_spawner/src/config.rs crates/terraphim_spawner/src/lib.rs
```

Coverage evidence should be recorded if the project coverage tooling is available and responsive.

### Step 6: Validation

**Skill**: disciplined-validation
**Goal**: run the full ADF local direct-dispatch lifecycle and confirm #1881 has all stage evidence.
**Evidence**: `.docs/validation-1881-badlogic-pi-cli.md`, daemon logs, issue comments.

## Rollback Plan

- Revert changes to `AgentConfig::infer_args` and `model_args` for `pi` only.
- Leave `pi-rust` tests as guardrails.
- Restore `.terraphim` proof harness to generic state or remove it if not intended to persist.

## Approval Gate

Implementation must not proceed until this plan is approved.

- [ ] Research reviewed
- [ ] Design reviewed
- [ ] Dynamic ADF evidence plan accepted
- [ ] Human approval to implement
