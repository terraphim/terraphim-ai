# Research Document: RLM Executor & Agent-Search Hardening

**Status**: Draft
**Author**: Claude Opus 4.7 (review-driven)
**Date**: 2026-05-15
**Reviewers**: Alex (project owner)
**Source**: Structural review of commits `e4d896d3d` → `4442671e9` (RLM executor work + agent fix + plugin example)

---

## Executive Summary

A structural review of the recently-landed RLM executor batch (Docker, Local), the OpenCode/Claude Code plugin example, and the agent's `concepts_matched` server-mode fix surfaced six P1 issues (concurrency race, ignored timeout contract, process leak, broken backend fallback, semantically-wrong thesaurus reconstruction, unportable shell hook) and ten P2 issues (resource limits, lifecycle gaps, dishonest snapshot impls, dead code, hygiene). All issues are independent fixes within already-merged code; none require architectural rework. This phase scopes the problem set, confirms the API surface available for fixes, and identifies the only true unknowns: whether the trait should grow an `end_session` hook and whether the `/thesaurus/{role}` endpoint should be enriched.

## Essential Questions Check

| Question | Answer | Evidence |
|---|---|---|
| Energizing? | Yes | Recently-landed code with named follow-ups; clear "fix what we just shipped" loop. |
| Leverages strengths? | Yes | Touches Rust async/concurrency, executor abstraction, automata search — all areas with established repo patterns to mirror (Firecracker `release_session_vm`, `kill_on_drop` discipline elsewhere). |
| Meets real need? | Yes | Two failure modes (TOCTOU container leak, ignored timeout) are silent reliability bugs that will bite the moment RLM has more than one concurrent session. |

**Proceed**: Yes (3/3 YES).

## Problem Statement

### Description

The RLM executor surface (`crates/terraphim_rlm/src/executor/{docker,local,mod}.rs`), the supporting OpenCode/Claude Code hook example (`examples/opencode-plugin-rlm/`), and the agent's robot-mode `concepts_matched` fix (`crates/terraphim_agent/src/main.rs`) shipped with a set of correctness, lifecycle, and observability defects identified in the structural review of 2026-05-15. They are functionally correct in single-threaded happy-path tests but degrade under concurrency, contract-driven calls, or non-Linux environments.

### Impact

- **DockerExecutor**: container leaks under concurrent sessions; container persists for the full executor lifetime with no per-session release; no resource limits (memory, PIDs, caps), weakening the "container isolation" claim relative to Firecracker; hardcoded `sleep 3600` keepalive becomes a dangling reference after one hour.
- **LocalExecutor**: callers that pass a `timeout_ms` are silently ignored (always 30 s); timed-out child processes become orphans; `create_snapshot` returns a real-looking `SnapshotId` for an op that does nothing.
- **`select_executor`**: `E2b` arm logs "Selected" then falls through; Docker init failure short-circuits the fallback chain, masking `LocalExecutor` even when it is selectable.
- **`concepts_matched` server-mode**: every reconstructed `NormalizedTerm` collides on `id = 1u64`, breaking any downstream code that uses ID for indexing; same logic is duplicated between the offline and server branches.
- **`terraphim-rlm-hook.sh`**: GNU `timeout` (absent on macOS) breaks the hook on a primary developer platform; JSON built via shell interpolation breaks on any prompt containing `"` or `\`.
- **`GiteaSkillRepoConfig`**: `token: Option<String>` will leak via `Debug`; `cache_dir: PathBuf` defaults to empty.

### Success Criteria

1. **Concurrency**: `DockerExecutor::ensure_container` is race-free under concurrent `execute_*` calls with the same `SessionId` (verified by a stress test, no orphan containers after run).
2. **Contract**: `LocalExecutor` honours `ctx.timeout_ms` and reaps timed-out children (verified by a unit test that asserts no zombie process and timely return).
3. **Lifecycle**: Sessions can release their container without tearing down the executor (verified by an inherent-method test mirroring `release_session_vm`).
4. **Selection**: With Docker daemon unavailable, `select_executor` returns `LocalExecutor` instead of erroring (verified by injection test).
5. **Semantic correctness**: server-mode `concepts_matched` returns identical values to offline-mode for the same role + query (verified by parity test).
6. **Portability**: hook script works on macOS without GNU coreutils and survives prompts with quotes (verified by `shellcheck` and a fixture test).
7. **Code hygiene**: zero new clippy warnings, zero `#[allow(dead_code)]`, zero unused fields.

## Current State Analysis

### Existing Implementation

The RLM executor uses a `select_executor` factory that walks `RlmConfig::backend_preference` (default: Firecracker → E2B → Docker → Local). Each backend implements the `ExecutionEnvironment` trait (`crates/terraphim_rlm/src/executor/trait.rs`). Snapshot lifecycle is on the trait; per-session resource lifecycle is **not** — `FirecrackerExecutor` exposes an inherent `release_session_vm` method (`firecracker.rs:290`) consumed by the supervisor, but `DockerExecutor` has no equivalent, leaving the trait's snapshot methods to implicitly carry session-cleanup semantics they were not designed for.

### Code Locations

| Component | Location | Purpose |
|---|---|---|
| Trait definition | `crates/terraphim_rlm/src/executor/trait.rs:32-156` | `ExecutionEnvironment` trait |
| Execution context | `crates/terraphim_rlm/src/executor/context.rs:43-92` | `ExecutionContext` (has `timeout_ms`) |
| Firecracker backend | `crates/terraphim_rlm/src/executor/firecracker.rs` | Reference impl with `release_session_vm` |
| Docker backend | `crates/terraphim_rlm/src/executor/docker.rs` | Subject of fixes |
| Local backend | `crates/terraphim_rlm/src/executor/local.rs` | Subject of fixes |
| Backend selector | `crates/terraphim_rlm/src/executor/mod.rs:80-143` | `select_executor` |
| Error variants | `crates/terraphim_rlm/src/error.rs` | Has `NoBackendAvailable`, lacks `NotSupported` |
| Robot search envelope | `crates/terraphim_agent/src/main.rs:2160-2200, 4220-4275` | Two `concepts_matched` populate sites |
| Thesaurus API server | `terraphim_server/src/api.rs:1639-1726` | Only returns `HashMap<String,String>` (lossy) |
| `NormalizedTerm` | `crates/terraphim_types/src/lib.rs:303-345` | Has `with_auto_id` helper (line 349) |
| Hook script | `examples/opencode-plugin-rlm/terraphim-rlm-hook.sh` | Subject of fixes |
| Plugin JS | `examples/opencode-plugin-rlm/terraphim-rlm.js` | Subject of fixes |
| Skill repo config | `crates/terraphim_orchestrator/src/config.rs:222-260` | `GiteaSkillRepoConfig` |

### Data Flow

`Caller → select_executor(config) → DockerExecutor.execute_code(code, ctx)`
  → `ensure_container(session_id)` → (TOCTOU window) → bollard create+start
  → `exec_in_container(container_id, cmd, ctx)` → `tokio::time::timeout(ctx.timeout_ms, stream)`
  → `ExecutionResult { stdout, stderr, exit_code, timed_out }`

`Caller → LocalExecutor.execute_code(code, ctx)`
  → `build_python_command(code, ctx)` → `tokio::time::timeout(30_000ms_HARDCODED, cmd.output())`
  → on timeout: future dropped, child not killed → orphan process.

`Robot-search caller → run_server_command → api.get_thesaurus(role) → ThesaurusResponse{HashMap<String,String>}`
  → reconstruct `Thesaurus` with every `id = 1u64` → `find_matches(query, thesaurus)`.

### Integration Points

- **bollard 0.20** — `create_container`, `start_container`, `create_exec`, `start_exec`, `inspect_exec`, `remove_container`, `ping`. Resource limits live under `ContainerCreateBody.host_config: Option<HostConfig>` (memory, `pids_limit`, `cap_drop`, `network_mode`, `read_only_root_filesystem`).
- **dashmap 6.1** — already a dependency of `terraphim_rlm` (`Cargo.toml:52`); supports `entry().or_insert_with` patterns suitable for per-session locking.
- **tokio 1.x** — `Command::kill_on_drop(true)` exists and is the idiomatic fix for the LocalExecutor child leak.
- **`/thesaurus/{role}` endpoint** — `terraphim_server/src/api.rs:1641` returns lossy `HashMap<String,String>`; agent client mirrors this in `crates/terraphim_agent/src/client.rs:198`.
- **`NormalizedTerm::with_auto_id`** — `crates/terraphim_types/src/lib.rs:349` already provides a unique-id constructor we can use to avoid the `id=1u64` collision.

## Constraints

### Technical Constraints

- **No new dependencies**: the project's disciplined-development culture (and the prior robustness pass plan) explicitly forbids adding crates for these fixes. `dashmap`, `tokio`, `bollard`, `terraphim_automata` are sufficient.
- **No mocks in tests**: per the user's project rule (`CLAUDE.md`). Docker tests must gate on real daemon availability (existing pattern in `docker.rs::tests::is_docker_available`); LocalExecutor tests use real `bash`/`python3`.
- **No `timeout` shell command**: per the user's global rule. Hook script must use a POSIX-portable construct.
- **No dead code**: per the user's global rule. `#[allow(dead_code)]` on `DockerExecutor` struct must go.
- **No emoji in code/docs**: per global rule (already adhered to here).
- **British English**: per global rule (used throughout this doc).
- **Trait stability**: changing `ExecutionEnvironment` is invasive (six implementations: docker, local, firecracker, ssh, e2b stub, mock in `rlm.rs:881`). Prefer inherent methods unless a cross-backend lifecycle hook is genuinely needed.
- **API stability**: extending `ThesaurusResponse` is a public-API schema change with frontend impact (the desktop UI consumes this endpoint). Prefer agent-side mitigation unless the lossy shape blocks other work.

### Business Constraints

- **No breaking changes** to the in-flight Firecracker path; all fixes must coexist with `FirecrackerExecutor` and the supervisor's `release_session_vm` consumer.
- **No timeline pressure**: these are post-merge follow-ups, not release blockers. We can sequence small commits.

### Non-Functional Requirements

| Requirement | Target | Current |
|---|---|---|
| Concurrent session container creation | 0 leaks | 1 leak per concurrent racer |
| `ctx.timeout_ms` honoured (Local) | Yes | No (hardcoded 30 s) |
| Zombie processes from LocalExecutor timeouts | 0 | 1 per timeout |
| Backend fallback after Docker init Err | Falls through to next | Errors out |
| Hook script compatible with macOS | Yes | No (GNU `timeout`) |
| Server-mode `concepts_matched` parity with offline | Identical | Off — id collision, no log on Err |

## Vital Few (Essentialism)

### Essential Constraints (Max 3)

| Constraint | Why It's Vital | Evidence |
|---|---|---|
| **No new public API on `ExecutionEnvironment` trait unless mandatory** | Trait change touches 6 implementations including a mock in `rlm.rs:881`; ripple is large for what is mostly per-backend cleanup. | `grep "impl ExecutionEnvironment"` returns 6 hits. |
| **No new dependencies** | Prior robustness pass (`implementation-plan-docker-executor-robustness.md`, "Avoid At All Cost") set this norm; everything we need is already in the workspace. | Cargo.toml inspection confirms `dashmap`, `tokio`, `bollard`, `terraphim_automata` available. |
| **No regressions to Firecracker path** | Firecracker is the "real" isolation backend; Docker/Local are convenience/dev backends. Don't perturb VM lifecycle semantics or `release_session_vm` contract. | `release_session_vm` is consumed by tests and supervisor; out of scope to change. |

### Eliminated from Scope

| Eliminated Item | Why Eliminated |
|---|---|
| Container pooling / pre-warm in `DockerExecutor` | Already explicitly out-of-scope per the prior robustness plan; not in the review findings. |
| Image pre-pull in `DockerExecutor` | Same as above. |
| Real snapshot support in Docker/Local | Snapshot story belongs to Firecracker; Docker/Local should honestly say "not supported" (a small fix, not a feature build-out). |
| Extending `/thesaurus/{role}` API to include IDs/URLs | Public-API change with frontend impact. The agent-side mitigation (`with_auto_id` or skip the `Thesaurus` reconstruction entirely) is sufficient for `concepts_matched` correctness. Document as a follow-up if richer client needs emerge. |
| Changing `ExecutionEnvironment` trait | Mostly per-backend resource lifecycle; inherent methods suffice. Re-evaluate only if supervisor needs uniform `end_session(SessionId)` across backends — not currently demonstrated. |
| Refactoring `select_executor` to a capability-driven matcher | Suggested in the review as a follow-up; valuable but out of scope for this defect-driven cycle. Capture as `Open Question 2`. |
| Rewriting `terraphim-rlm.js` plugin | The double-spawn issue is real but limited to an example; flag, fix the most-egregious bits, defer architectural rewrite. |
| Adding `shellcheck` to CI | Useful but a CI/infra change; defer to a separate cycle. Suggested as a follow-up in the review. |

## Dependencies

### Internal Dependencies

| Dependency | Impact | Risk |
|---|---|---|
| `terraphim_agent_supervisor` | Consumes `release_session_vm`; will need to call analogous `release_session_container` if we add it. | Low — additive. |
| `terraphim_automata::find_matches` | Used by both `concepts_matched` paths; signature must accept `Thesaurus`. | None — reusing existing API. |
| `terraphim_types::NormalizedTerm::with_auto_id` | Pre-existing helper that solves the id-collision symptom. | None — already used elsewhere in workspace. |
| `terraphim_orchestrator::config::GiteaSkillRepoConfig` | Token redaction touches struct definition; consumers do `Debug`-print. | Low — `Debug` impl change only. |

### External Dependencies

| Dependency | Version | Risk | Alternative |
|---|---|---|---|
| `bollard` | 0.20 | Low — `HostConfig` is stable across 0.x. | None needed. |
| `dashmap` | 6.1 | Low — already in workspace. | `tokio::sync::Mutex<HashMap<...>>` (slower under contention). |
| `tokio::process::Command::kill_on_drop` | 1.x | Low — stable since 1.0. | Manual `child.kill()` in timeout branch. |

## Risks and Unknowns

### Known Risks

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| Adding `kill_on_drop(true)` masks a legitimate use case (long-running backgrounded process) | Low | Med | LocalExecutor is documented as no-isolation dev/test backend; long-running detached jobs are not its use case. Document the choice in the doc-comment. |
| Per-session DashMap lock leaks an entry per session | Med | Low | Add `release_session_container` that removes the entry. Existing `cleanup()` already drains, so leakage is bounded by session count. |
| Adding `host_config` resource limits breaks existing tests that rely on default permissive container | Low | Low | Existing tests only assert capabilities and health-check; resource limits are additive and don't affect those. |
| `with_auto_id` for server-mode `Thesaurus` reconstruction still loses URL/action/priority | Low | Low | `find_matches` only consumes term values for the automata; URL/action are not needed for `concepts_matched`. Document this. |
| Adding a `release_session_container` inherent method drifts from `release_session_vm` naming | Low | Low | Use `release_session` as the symmetric name on both backends in a follow-up rename (mark as out-of-scope here, name it `release_session_container` to match the `_container` mental model used in this file). |
| Hook script POSIX timeout substitute (`( cmd & PID=$!; sleep 30 && kill $PID )`) misbehaves under `set -e` | Med | Low | Wrap in subshell with explicit error handling; cover with bats-style fixture test. |

### Open Questions

1. **Should `ExecutionEnvironment` grow an `end_session(SessionId)` method?** The supervisor would benefit from a uniform hook, but no current call-site demands it. **Recommendation**: defer; add as inherent method on `DockerExecutor` matching `release_session_vm` pattern. **Resolver**: Alex.
2. **Should `select_executor` move to a capability-driven matcher?** The current enum-arm matching makes the E2b-fallthrough bug easy to write. **Recommendation**: defer to a separate cycle; in this cycle, fix the immediate bugs and add `log::warn!` on insecure fallback. **Resolver**: Alex.
3. **Should `/thesaurus/{role_name}` API be enriched to return `Vec<NormalizedTerm>` instead of `HashMap<String,String>`?** Would let the agent reconstruct a faithful `Thesaurus`. **Recommendation**: defer; the immediate fix uses `with_auto_id` (lossless for matching). Track as a separate enhancement if frontend needs richer data. **Resolver**: Alex.
4. **Should `RlmError` grow a `NotSupported { backend, op }` variant?** Cleaner than returning a fake `SnapshotId`. **Recommendation**: yes — this is a small, additive variant that improves caller diagnostics. **Resolver**: Alex.
5. **Should `LocalExecutor`'s `output_dir` field be wired or removed?** Currently dead. **Recommendation**: remove; not used anywhere; if we need an output-spool dir later, reintroduce with consumers. **Resolver**: Alex.

### Assumptions Explicitly Stated

| Assumption | Basis | Risk if Wrong | Verified? |
|---|---|---|---|
| `DockerExecutor` uses bollard 0.20's `HostConfig`-via-`ContainerCreateBody.host_config` field for resource limits | bollard 0.20 changelog, generic Docker REST API knowledge | Need different field path; small fix | No — to verify in Phase 2 by reading bollard docs |
| `dashmap::DashMap<SessionId, Arc<tokio::sync::Mutex<()>>>` is the cheapest way to serialise per-key creation | Established repo pattern; dashmap is already a dep | If contention is high we may want a different scheme; doesn't break correctness | No — accept; benchmark if it bites |
| `tokio::process::Command::kill_on_drop(true)` is sufficient (no `prctl(PR_SET_PDEATHSIG)` style escalation needed) | tokio docs; LocalExecutor is single-process spawns of bash/python | Children of children survive (unlikely for python `-c` snippets) | No — accept; document |
| `find_matches` against a `Thesaurus` built with `with_auto_id` produces identical match output to one built with the original ids | `find_matches` consumes the automata which keys on term values, not ids | Match output diverges; parity test catches | No — to verify with the parity test in Phase 4 |
| Adding `host_config: Some(HostConfig{ memory, pids_limit, cap_drop, ... })` does not regress existing Docker test runs | Tests only assert capabilities and health-check (not resource consumption) | Tests fail in CI; revert and re-tune | No — to verify in Phase 4 |
| The structural-review-flagged `#[allow(dead_code)]` on `DockerExecutor` was added to silence a transient warning during the original PR | Removing it should produce zero warnings now that the struct is fully used | If a field is genuinely unread, will need to investigate | No — to verify in Phase 3 by removing and running clippy |

### Multiple Interpretations Considered

| Interpretation | Implications | Why Chosen/Rejected |
|---|---|---|
| **Fix `concepts_matched` server bug at the API layer (enrich `ThesaurusResponse`)** | Lossless data, no per-caller workaround | **Rejected for now**: schema change with frontend impact; defer to follow-up. The agent-side fix using `with_auto_id` is sufficient for `find_matches`. |
| **Fix `concepts_matched` at the agent layer using `with_auto_id`** | Cheap, local, semantically honest | **Chosen**: minimum viable correctness with no API impact. |
| **Fix `concepts_matched` by skipping `Thesaurus` reconstruction entirely** (call `find_matches` on a vec of strings, e.g., add a thin overload) | Avoids the question of identity altogether | **Considered, deferred**: would need a new `terraphim_automata` API; touches a stable public surface. The `with_auto_id` path uses existing API. |
| **`LocalExecutor` timeout: respect `ctx.timeout_ms` only** | Minimal change | **Chosen**, with the additional `kill_on_drop(true)` to fix the orphan-process issue. The two are coupled because honouring a tight timeout makes leaked children more visible. |
| **Add per-session lock for `DockerExecutor`: `dashmap::DashMap<SessionId, Arc<Mutex<()>>>`** | Localised, no trait change | **Chosen**: matches the existing dashmap dependency; pattern used elsewhere in the repo. |
| **Add per-session lock: replace whole map with single `tokio::sync::Mutex<HashMap<...>>`** | Simpler code | **Rejected**: serialises all session ops, not just creation for a given session. Worse latency under load. |
| **Backend selection: capability matcher rewrite** | Future-proof; can't write the E2b-fallthrough bug | **Deferred**: bigger change; in this cycle just fix the immediate logic. |
| **Hook script: rewrite in Python** | Robust JSON via stdlib | **Rejected**: adds a runtime requirement; users may not have Python in PATH. Fix shell script with `jq -n --arg`. |
| **Hook script: ship two variants (linux/macos)** | Easy maintenance | **Rejected**: one well-written POSIX script is preferable. |

## Research Findings

### Key Insights

1. **The "robustness pass" already established the right normative pattern** for DockerExecutor (Drop guards, `tokio::time::timeout` honouring `ctx.timeout_ms`, rollback-on-start-fail). The remaining work is to extend that discipline to (a) `LocalExecutor` and (b) container lifecycle/resource concerns that the prior pass declared out-of-scope but the structural review re-raised.
2. **The trait does not need to grow**. Both Firecracker (`release_session_vm`) and Docker (need `release_session_container`) carry per-session resource lifecycle as inherent methods consumed directly by the supervisor. Adding `end_session` to the trait is tempting but premature without a uniform call-site.
3. **The `concepts_matched` server bug is an artefact of a lossy API**, not a logic bug. The fix can be local (use `with_auto_id`) or systemic (enrich the API). Local is correct for this cycle; systemic is a separate enhancement.
4. **The hook script's bugs are 100% recoverable in shell** — `jq -n --arg` for safe JSON encoding, a portable timeout pattern, and `shellcheck` cleanup. No language switch required.
5. **The `Local` backend is honestly named "no isolation"** in its own doc-comment. Logging `info!` on selection is wrong — fall-back to no-isolation is a security-posture downgrade and should be `warn!`.
6. **`with_auto_id`** (`types/lib.rs:349`) already exists for this exact "I have a string, I need a `NormalizedTerm`" use case. The bug is using `NormalizedTerm::new(1u64, ...)` instead.

### Relevant Prior Art

- `.docs/research-docker-executor-robustness.md` — established the small-fix discipline for `docker.rs` and informs scope boundaries.
- `.docs/implementation-plan-docker-executor-robustness.md` — confirms the "no new deps, no trait changes, smallest correct fix" norm.
- `crates/terraphim_rlm/src/executor/firecracker.rs:290` — `release_session_vm` reference pattern.
- `crates/terraphim_rlm/src/executor/local.rs:89` — current hardcoded timeout (the bug).
- `crates/terraphim_types/src/lib.rs:349` — `with_auto_id` helper.

### Technical Spikes Needed

| Spike | Purpose | Estimated Effort |
|---|---|---|
| Verify bollard 0.20 `HostConfig` field path & required builder pattern | Confirm `cap_drop: Vec<String>`, `memory: i64`, `pids_limit: i64`, `network_mode: String`, `readonly_rootfs: bool` shape | 15 min |
| Confirm `dashmap::Entry::or_insert_with` works inside `async fn` | Pattern check; dashmap entries are sync but the value can hold an async-friendly type | 10 min |
| `shellcheck` on `terraphim-rlm-hook.sh` to enumerate any issues beyond the two we know | Defence in depth | 5 min |
| Reproduce TOCTOU container leak with a test that fires N concurrent `execute_command` calls | Convert review claim into a regression test | 30 min |

## Recommendations

### Proceed/No-Proceed

**Proceed.** All findings are well-scoped, all needed APIs exist in the workspace, no new dependencies are required, and the prior robustness pass set the normative pattern for small-and-correct fixes in this area.

### Scope Recommendations

Group the work into five independent commits, ordered by dependency and risk:

1. **`fix(rlm-local): honour ctx.timeout_ms and reap timed-out children`** (LocalExecutor) — smallest, no concurrency risk, sets up `kill_on_drop` discipline.
2. **`fix(rlm-docker): per-session lock in ensure_container; resource limits; release_session_container`** (DockerExecutor) — concurrency + lifecycle + resource limits in one logical change because they share container-creation and cleanup paths.
3. **`fix(rlm-selector): correct E2B fallthrough; degrade to Local on Docker init Err; warn on insecure fallback`** (`mod.rs`) — backend selection bugs.
4. **`fix(agent): de-duplicate concepts_matched and use with_auto_id in server path`** (`crates/terraphim_agent/src/main.rs` + tests) — agent fix.
5. **`fix(hook): portable timeout, safe JSON via jq -n --arg; redact GiteaSkillRepoConfig.token`** (hook script + orchestrator config) — peripheral hardening.

### Risk Mitigation Recommendations

- **TOCTOU regression test must run under release profile** — debug builds are slow enough to obscure the race.
- **All Docker tests gated on `is_docker_available()`** — already the convention, retain it.
- **Add a `wrk`-style integration test for concurrent sessions** in a follow-up cycle; for this cycle a `tokio::join!`-based stress test in unit tests is sufficient.
- **Do not change the `ExecutionEnvironment` trait** — it changes 6 impls and one mock. Use inherent methods.
- **Do not change the `/thesaurus/{role}` HTTP contract** — would touch desktop UI. Use `with_auto_id` agent-side.

## Next Steps

If approved:

1. Run the four small spikes (bollard `HostConfig` shape, dashmap-in-async, shellcheck baseline, TOCTOU repro test) — ~1 hour total.
2. Produce `.docs/implementation-plan-rlm-executor-hardening.md` (Phase 2) with file-level changes, function signatures, and step ordering matching the five-commit grouping above.
3. Submit the design for human approval before any code changes.

## Appendix

### Reference Materials

- `.docs/research-docker-executor-robustness.md` (prior pass)
- `.docs/implementation-plan-docker-executor-robustness.md` (prior pass)
- `.docs/verification-report-docker-executor-robustness.md` (prior pass)
- Structural review: in-conversation output of 2026-05-15 (this session)
- bollard docs: <https://docs.rs/bollard/0.20> (to validate in spike 1)

### Code Snippets

**Current TOCTOU window (`docker.rs:77-88`)**:
```rust
async fn ensure_container(&self, session_id: &SessionId) -> RlmResult<String> {
    if let Some(container_id) = self.session_to_container.read().get(session_id) {
        return Ok(container_id.clone());
    }
    // <-- TOCTOU window: other task can pass the read check here
    let container_id = self.create_container(session_id).await?;
    self.session_to_container
        .write()
        .insert(*session_id, container_id.clone());
    Ok(container_id)
}
```

**Current LocalExecutor timeout (`local.rs:89`)**:
```rust
let output = timeout(Duration::from_millis(30000), cmd.output()).await;
//                            ^^^^^ ignores ctx.timeout_ms
```

**Current concepts_matched server-mode (`main.rs:4240-4263`)**:
```rust
for value in entries.values() {
    let normalized_term = terraphim_types::NormalizedTerm::new(
        1u64,                                              // <-- collision
        terraphim_types::NormalizedTermValue::from(value.clone()),
    );
    thesaurus.insert(NormalizedTermValue::from(value.clone()), normalized_term);
}
```

**Available helper (`types/lib.rs:349`)**:
```rust
pub fn with_auto_id(value: NormalizedTermValue) -> Self {
    Self {
        id: get_int_id(),  // unique
        value,
        ...
    }
}
```

---

## Gate Checklist

### Standard Gates
- [x] Research document completed
- [x] All sections filled in
- [x] Risks identified and categorized
- [ ] Human approval received
- [x] Open questions captured (resolution recommendations included)

### Essentialism Gates
- [x] Essential Questions Check completed (3/3 YES)
- [x] Vital Few section completed (3 essential constraints)
- [x] Eliminated Items documented
- [x] Passes 90% rule: a HELL YES — fix-list for code we just shipped, with clear, bounded scope.

### Quality Evaluation
Proceed to Phase 2 once Open Questions 1–5 receive direction and human approves the scope grouping. Quality-evaluation skill can be invoked between Phase 1 and Phase 2 if desired.
