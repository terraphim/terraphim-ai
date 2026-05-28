# Research Document: badlogic/pi CLI Support via ADF Local Dispatch

**Status**: Review
**Issue**: terraphim/terraphim-ai#1881
**Skills evidenced**: disciplined-research

## Executive Summary

The requested proof is not just adding a `pi` argument mapping. It must prove that `adf-ctl --local trigger --direct` can orchestrate the complete implementation lifecycle for issue #1881 through ADF-dispatched agents: research, detailed plan, implementation, structured PR review, verification, and validation.

The previous hardcoded issue-number proof is insufficient because it demonstrates a fixed demo script rather than a reusable ADF CLI workflow. The correct proof must use repository-local `.terraphim/adf.toml`, dynamically pass or discover the target issue, and record evidence back to Gitea from each ADF-spawned stage.

## Essential Questions Check

| Question | Answer | Evidence |
|----------|--------|----------|
| Energising? | Yes | This proves ADF can execute real local implementation workflows rather than smoke tests. |
| Leverages strengths? | Yes | The repo already contains ADF direct dispatch, spawner, Gitea workflow, and disciplined development artefacts. |
| Meets real need? | Yes | The user explicitly requires proof that `adf-ctl` can run the whole flow end to end for issue #1881. |

**Proceed**: Yes, 3/3.

## Problem Statement

### Description

Add support for the `pi` CLI from <https://github.com/badlogic/pi> in ADF agent spawning, then prove the change through a full ADF local direct-dispatch lifecycle.

### Impact

- Without correct `pi` support, ADF cannot use badlogic/pi as a managed CLI tool.
- Without an ADF-driven proof, local dispatch remains only partially validated.
- Without Gitea progress evidence, there is no auditable task lifecycle.

### Success Criteria

1. `adf-ctl --local trigger --direct` dispatches each lifecycle stage from `.terraphim/adf.toml`.
2. Each stage posts skill-specific evidence to Gitea issue #1881.
3. Implementation changes support badlogic/pi without regressing existing `pi-rust` support.
4. Tests verify `pi` argument construction and model handling.
5. Structured PR review, verification, and validation reports are produced and linked.
6. No hardcoded issue ID exists in reusable agent config or scripts; target issue is supplied via dispatch context or selected by Gitea query.

## Current State Analysis

### Existing Implementation

| Component | Location | Current Behaviour |
|-----------|----------|-------------------|
| Direct dispatch client | `crates/terraphim_orchestrator/src/bin/adf-ctl.rs` | Sends `{ agent, context }` over Unix socket. |
| Direct dispatch daemon | `crates/terraphim_orchestrator/src/direct_dispatch.rs` | Validates agent name and emits `WebhookDispatch::SpawnAgent`. |
| Direct dispatch handler | `crates/terraphim_orchestrator/src/lib.rs` | Appends context to the agent task and calls `spawn_agent`. |
| Spawner config | `crates/terraphim_spawner/src/config.rs` | Already recognises `pi-rust` and `pi`, but currently treats both as `pi-rust` style. |
| Spawner process | `crates/terraphim_spawner/src/lib.rs` | Appends the task as the final positional argument unless stdin is used. |

### Upstream badlogic/pi Contract

From upstream README:

```bash
pi prompt phi3 "What is 2+2?"
pi start microsoft/Phi-3-mini-128k-instruct --name phi3 --memory 20%
pi list
```

The badlogic/pi command named `pi` is a GPU pod/model manager. For prompt execution, the non-interactive form is `pi prompt <model-name> <message>`, not `pi -p --mode json <message>`.

### Important Distinction

`pi-rust` and badlogic `pi` are different CLIs:

| CLI | Expected Prompt Shape | Current Code |
|-----|-----------------------|--------------|
| `pi-rust` | `pi-rust -p --mode json [--provider P --model M] <prompt>` | Existing tests expect this. |
| badlogic `pi` | `pi prompt <model-name> <prompt>` | Current code incorrectly maps `pi` to `pi-rust` style. |

## Risks and Unknowns

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Regressing `pi-rust` | Medium | High | Separate match arms and explicit tests for both CLIs. |
| Upstream `pi` not installed locally | Medium | Medium | Unit-test command construction; validation can use a temporary executable named `pi` for E2E without mocks. |
| ADF proof devolves into scripted fake proof | High | High | Require daemon logs, Gitea comments, and stage artefacts produced by ADF-spawned processes. |
| Hardcoded issue IDs in reusable config | Already occurred | High | Agent config must be generic; issue ID comes from context or `gtr ready` selection. |

## Assumptions Explicitly Stated

| Assumption | Basis | Risk if Wrong | Verification |
|------------|-------|---------------|--------------|
| badlogic/pi prompt invocation is `pi prompt <model> <message>` | Upstream README | Incorrect args | Unit tests encode README contract; local fake pi validates argv shape. |
| `pi-rust` must remain unchanged | Prior repo research and tests | Regression of existing integration | Existing and new tests for `pi-rust`. |
| ADF stage identity can be passed via context | `direct_dispatch.rs` supports context and `handle_direct_dispatch` appends it to task | Ambiguous parsing | Use explicit key-value context (`issue=1881 stage=...`). |

## Vital Few

| Constraint | Why Vital | Evidence |
|------------|-----------|----------|
| Separate `pi` from `pi-rust` | They are different CLIs with incompatible flags. | Upstream README vs existing pi-rust tests. |
| Dynamic issue targeting | Proves reusable ADF CLI flow, not hardcoded demo. | User correction. |
| ADF-spawned evidence | Proves `adf-ctl` executed the flow. | Daemon logs + Gitea comments. |

## Eliminated from Scope

| Eliminated Item | Why Eliminated |
|-----------------|----------------|
| Provisioning GPU pods | badlogic/pi supports this, but issue is CLI spawn support. |
| Running real remote GPU inference | Requires external GPU pod and model setup; command-contract proof is sufficient for spawner support. |
| Replacing `pi-rust` with badlogic/pi | They serve different purposes and must coexist. |

## Recommendation

Proceed to disciplined design with a two-part plan:

1. Implement minimal, test-backed badlogic/pi argument support while preserving `pi-rust`.
2. Prove end to end through ADF local dispatch stages that dynamically target #1881 and update Gitea.
