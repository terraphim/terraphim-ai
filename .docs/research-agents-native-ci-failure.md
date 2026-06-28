# Research Document: terraphim-agents native-ci failure (PR #33)

**Status**: Resolved (native-ci run 73 green on `6e668e84`)
**Author**: Terraphim AI
**Date**: 2026-06-10
**Reviewers**: Alex

## Executive Summary

terraphim-agents PR #33 fails `native-ci / build (push)` in ~1 second. Local reproduction shows `cargo fmt --all -- --check` fails on two files introduced by the Phase 0 PrGateResult port. Main-branch CI on the same repo succeeded 7 seconds earlier (run 68), ruling out a broken runner instance as the primary cause for this PR.

## Essential Questions Check

| Question | Answer | Evidence |
|----------|--------|----------|
| Does this problem energize us to solve it? | Yes | Blocks Phase 0 exit: agents PR merge + ADF deploy |
| Does solving this leverage our unique capabilities? | Yes | We own the port diff and BUILD.md contract |
| Does this meet a significant, validated need? | Yes | Branch protection requires `native-ci / build (push)` |

## Problem Understanding

### What problem are we solving?

PR #33 cannot merge because Gitea Actions posts `native-ci / build (push)` **failure** on head `98af0ab5`, with description "Failing after 1s".

### Impact of not solving

- PrGateResult implementation stays off `terraphim-agents` main
- ADF binary on bigbox remains pre-#33
- Phase 1 auto-merge work cannot deploy against production main

### Success criteria

- `cargo fmt --all -- --check` passes locally on the PR branch
- Gitea workflow run 71+ posts `native-ci / build (push)` **success**
- PR #33 becomes mergeable

## Existing System Analysis

### Workflow (`.gitea/workflows/native-ci.yml`)

Identical in terraphim-agents and terraphim-ai:

```yaml
jobs:
  build:
    runs-on: terraphim-native
    steps:
      - run: cargo fmt --all -- --check   # step 1 — fails fast
      - run: cargo clippy ...
      - run: cargo build ...
      - run: cargo test ...
```

No explicit `actions/checkout` step; the `terraphim-native` runner uses a persistent workspace checkout (proven by 7s green runs on main).

### Recent run history (terraphim-agents)

| Run | Head | Branch | Duration | Result |
|-----|------|--------|----------|--------|
| 68 | `7b68d2b0` | main (publish merge) | ~7s | **success** |
| 69 | `1df21795` | task/2301-pr-gate-result-contract | ~1s | failure |
| 70 | `98af0ab5` | task/2301-pr-gate-result-contract | ~1s | failure |

### Prior art (same failure signature)

`docs/handovers/2026-06-05-adf-repolocal-rollout-doc-churn-runner3.md` documents an identical symptom ("failing step `cargo fmt`, <1s") with two root causes:

1. **Code**: rustfmt drift on the pushed branch
2. **Infra**: runner instance online but missing `~/.cargo-runner-N/bin` toolchain

## Constraint Identification

| Constraint | Detail |
|------------|--------|
| CI gate | Branch protection requires `native-ci / build (push)` success |
| Command contract | `BUILD.md` mandates fmt as first step |
| Runner | `runs-on: terraphim-native` — shared across 6 polyrepos |
| No mocks | Fix must be real fmt compliance, not status spoofing |

## Hypothesis Evaluation

### H1: rustfmt violations on PR branch (PRIMARY)

**Evidence for:**
- Local `cargo fmt --check` exits 1 with diffs in:
  - `crates/terraphim_orchestrator/src/pr_handlers_impl.rs` (tuple destructuring line width)
  - `crates/terraphim_orchestrator/src/reconcile_impl.rs` (import order, HashMap type wrap, `if/else` collapse)
- Failure at step 1 explains ~1s wall time
- Runs 69–70 correlate exactly with the PrGateResult port commits

**Evidence against:** none

### H2: Broken runner instance (SECONDARY — ruled out for this PR)

**Evidence for (historical):** instance-3 missing toolchain caused identical 1s fmt failures org-wide (2026-06-05)

**Evidence against:**
- Run 68 on **same repo** succeeded 2026-06-09 on main
- Failure is branch-specific (PR branch only), not org-wide
- Local fmt reproduces without involving the runner

## Risk Assessment

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| Clippy/test failures after fmt | Low | Run full BUILD.md locally before push |
| Runner regression during fix | Low | Re-check run 71 duration (>3s implies real cargo execution) |
| Duplicate pending+failure statuses | Known Gitea quirk | Judge by latest `status: success` entry |

## Open Questions

None — root cause chain confirmed through CI step 4.

## Follow-up Findings (multi-stage failure chain)

| Stage | Run | Duration | Root cause |
|-------|-----|----------|------------|
| 1 fmt | 69–70 | ~1s | rustfmt drift in `pr_handlers_impl.rs`, `reconcile_impl.rs` |
| 2 clippy | (local) | — | 3× `collapsible_if` in ported code (`pr_gate_context.rs:259`, `reconcile_impl.rs:1110,1608`) |
| 4 test | 71 | ~54s | 6–8 `lib_tests` fail under parallel load: `SpawnFailed` broken pipe on PR gate agents |

### H3: PR gate stdin + `echo` stub race (PRIMARY for run 71)

**Evidence for:**
- Phase 0 port changed `dispatch_pr_reviewer_for_pr` to `SpawnRequest::new(..., gate_prompt).with_stdin()` (`pr_handlers_impl.rs:304`)
- Failing tests use `review_pr_config("echo")` / fanout helpers with `echo` CLI stubs
- Error: `failed to write prompt to stdin: Broken pipe (os error 32)`
- Reproduces locally with `RUST_TEST_THREADS=32`; passes at default thread count (flake)
- Existing precedent: `test_spawn_agent_with_persona_composes_prompt` documents `cat` not `echo` for stdin paths (`lib_tests.rs:884`)

**Evidence against:** none

**Fix:** Map gate-agent test stubs from `echo` → `cat`; keep `echo` for `build-runner` (no stdin). No production code change.

## Recommendation

1. Apply fmt + clippy fixes (commit `8d361169` — done)
2. Fix ReviewPr `lib_tests` gate stubs (`cat` for stdin-consuming agents)
3. Push and verify workflow run 72+ posts `native-ci / build (push)` success
4. Merge PR #33

No workflow or runner changes required.
