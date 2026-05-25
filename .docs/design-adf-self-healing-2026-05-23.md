# Implementation Plan: ADF Self-Healing -- Decision Tier + pi-rust Alternative Spawner

**Status**: Draft v2 (Phase 2, awaiting approval) -- v1 corrected after bigbox inspection
**Research Doc**: `.docs/research-adf-self-healing-2026-05-23.md` (approved)
**Author**: Claude (this session)
**Date**: 2026-05-23
**Estimated Effort**: 14 hours (revised down from 20h after bigbox inspection -- see "v2 corrections")
**Refs**: epic #1807, #1804 (P0 credentials), #1805 (P1 merge-coordinator), #1797 (fleet CRITICAL), **#1783** (replace opencode runtime with pi-rust), **#1785** (terraphim_pi_agent bridge crate)

## v3 correction: Z.AI is upstream-broken (2026-05-23 evening)

Direct invocation on bigbox revealed:

```
opencode run -m zai-coding-plan/glm-5.1 --format json "say ping"
-> {"type":"step_start",...}
-> (silence; exits with no text/step_finish)
```

Same behaviour across all four `zai-coding-plan/*` models (glm-5.1, glm-5-turbo, glm-4.7, glm-4.5-air). Auth is present in opencode `auth.json`. Other coding-plan providers (kimi-for-coding, minimax-coding-plan) work normally in the same harness. Conclusion: **opencode's Z.AI Coding Plan integration (or the Z.AI subscription itself) is broken upstream**, not a probe-tuning issue. The provider_probe's 60s TIMEOUT classification is **correct**.

**Step 1 revision:**
- Drop "tighten probe to 30s + stderr classification" -- probe is correct
- Instead: **quarantine Z.AI routes in taxonomy** so KG router never selects them as a fallback while Z.AI is upstream-broken. Either remove the `route::` lines, or annotate with `is_unhealthy:: true` if KG router supports it (TBC)
- File a **separate** new issue: "Investigate Z.AI Coding Plan stream truncation (opencode 1.14.48)" -- this is opencode/Z.AI vendor work, not orchestrator work
- pi-rust against Z.AI is separately blocked on `ZHIPU_API_KEY` env not being exported from opencode's `auth.json` to systemd unit -- low priority while Z.AI itself is broken

**Net effect**: Step 1 simplifies further (from "probe fix" to "quarantine routes + file vendor issue") -- estimate drops from 2h to 30 min.

## v2 corrections after bigbox inspection (2026-05-23)

Three factual corrections that simplify the design:

1. **pi-rust 0.1.10 is already installed on bigbox** at `/home/alex/.local/bin/pi-rust` (44 MB, built 2026-03-22). **Step 2 (install) is removed.** No `cargo install` runbook needed.
2. **`fallback_provider` in `conf.d/*.toml` is a filesystem path, not a tier name**. Existing values include `/home/alex/.bun/bin/opencode` and `/home/alex/.local/bin/claude`. Q5's "reuse `bypass_kg_routing` with `fallback_provider="pi-rust"`" was based on a misread; the natural form is `fallback_provider = "/home/alex/.local/bin/pi-rust"`. **No Rust code change needed for the path swap.**
3. **KG taxonomy `action::` templates handle per-CLI argv**. Adding pi-rust as a runtime needs only new `route::` + `action::` lines per tier markdown -- e.g. `action:: /home/alex/.local/bin/pi-rust --provider {{ provider }} --model {{ model }} -p "{{ prompt }}"`. **Step 4 (cross-CLI helper) is removed.**

**Related existing issues this design coordinates with (not duplicates):**
- **#1783** plans a *full* replacement of opencode with pi-rust as ADF's primary runtime. This design ships pi-rust as an *additional* runtime via taxonomy route lines; #1783's full replacement remains separate work.
- **#1785** plans `terraphim_pi_agent` -- a Rust bridge crate with provider inference, skill-chain resolution, and swarm orchestrator. **This design intentionally does NOT build that crate**; it uses pi-rust directly via taxonomy templates so #1785 can supersede cleanly when it lands.

**Net effect on the 10-step plan:**
- Step 2 (install pi-rust): **REMOVED** (-2 h)
- Step 4 (cross-CLI helper): **REMOVED** (-3 h)
- Step 9 install-pi-rust-bigbox script: **REMOVED** (-0.5 h)
- Step 3 (taxonomy edits): unchanged but absorbs the action templates for pi-rust (+0.5 h)
- New step 4 (replaces old 4): **decision-tier `route::` lines + Z.AI route correction in existing tiers** (+1 h)
- Total: **14 h** (was 20 h)

---

## Overview

### Summary

Close the 37 % -> 70 % success-rate gap on bigbox ADF by:
1. Adding a **decision tier** (priority 65) between planning (80) and implementation (50)
2. Expanding existing tiers with `opencode/gpt-5.5` (planning) and `minimax-coding-plan/MiniMax-M2.7-highspeed` (implementation)
3. Wiring **pi-rust as an alternative spawner** for the same subscription providers (reuses `bypass_kg_routing` + new `fallback_provider="pi-rust"` value)
4. Investigating + fixing the Z.AI probe (the model is already in the taxonomy; probe is broken, not the provider)
5. Rust-rewriting `merge-coordinator` per spec (#1805) with **auto-close-on-PR-merge**
6. Redacting Debug derive secret leaks (#1804)
7. Adding three operational guardrails: circuit-breaker on repeated config errors, memory watchdog, bigbox sync runbook

### Approach

Minimal additive changes to a working architecture. PR #1794 proved the layered fallback works; this plan extends it along three axes (more models, alternative CLI, better hygiene) without restructuring routing.

### Scope

**In Scope (5):**
1. KG taxonomy: new `decision_tier.md`; expand `planning_tier.md` (+ `opencode/gpt-5.5`) and `implementation_tier.md` (+ `MiniMax-M2.7-highspeed`)
2. `pi-rust` as alternative spawner via `fallback_provider="pi-rust"`
3. Z.AI probe fix in `provider_probe.rs`
4. Rust `crates/terraphim_merge_coordinator/` per `.docs/spec-merge-coordinator.md` (#1805) including auto-close-on-merge
5. Operational guardrails: Debug redaction (#1804), config-error circuit-breaker, memory watchdog, bigbox sync runbook

**Out of Scope:**
- MetaCoordinator above all agents
- New ranking algo / KG re-architecture
- New marketing artefact for ADF (separate brief)
- Persistent VMs / multi-region
- Codex CLI install (opencode covers GPT-5.5)
- New Gitea client crate (use existing `terraphim_tracker`)

**Avoid At All Cost** (from 5/25 analysis):
1. Replacing opencode -- workhorse provider, do not destabilise
2. Restructuring KG tier scoring (planning=80/impl=50/review=40 stays untouched; only add decision=65)
3. Custom Gitea API client (use existing `terraphim_tracker::gitea`)
4. New systemd units (extend `adf-orchestrator.service`; do not add siblings)
5. Sub-agent spawning inside merge-coordinator (it shells out only)
6. Asynchronous merge-coordinator pipeline (spec says sequential)
7. Cross-repo dependency graph rewrite
8. Quickwit/log-analyst overhaul
9. New auth/token rotation mechanism
10. UI for ADF status (CLI + journal is enough)
11. Persistent agent process pool
12. Database for merge-coordinator state (file-based PID lock per spec)
13. New telemetry pipeline (existing `tracing` spans suffice)
14. ADF "soft mode" feature flag with separate state (use `enabled = false` in conf.d)
15. Generic "MetaCoordinator" agent (out of time budget; epic #1807 separate)
16. New ADR proliferation -- one ADR covers decision-tier + pi-rust spawner (single decision, single doc)
17. Schema migrations on `flow-states`
18. K8s / containerisation of any agent
19. IDE plugin / VSCode helper for ADF observability
20. Custom Aho-Corasick replacement -- KG router works

## Architecture

### Component Diagram

```
                +-------------------------------+
                |   /opt/ai-dark-factory/       |
                |   conf.d/*.toml (agents)      |
                +---------------+---------------+
                                |
                                v
+-------------------------------+--------------------------------+
|              terraphim_orchestrator (adf binary)               |
|                                                                |
|   reconcile_loop (30s tick)                                    |
|     -> KgRouter::route_agent(task)                             |
|         reads docs/taxonomy/routing_scenarios/adf/*.md         |
|         tiers: planning(80) decision(65*new) impl(50) review(40)|
|     -> first_healthy_route(unhealthy=[claude, zai_if_probe_ok])|
|     -> spawn_agent(def, route, fallback_provider)              |
|         if fallback_provider == "pi-rust":                     |
|             cli_tool = /usr/local/bin/pi-rust                  |
|         else:                                                  |
|             cli_tool = def.cli_tool (opencode/claude)          |
|                                                                |
|   provider_probe (TTL=1800s)                                   |
|     - claude     (existing)                                    |
|     - opencode/*  (existing)                                   |
|     - zai-coding-plan/glm-5.1  (* fix probe)                   |
|     - pi-rust  (* new health probe)                            |
+----------------------+----------------+-----------------------+
                       |                |
        +--------------v-----+   +------v---------+
        | opencode (Bun)     |   | pi-rust (Rust) |  <-- new
        | OpenAI/Kimi/MiniMax |   | same providers |
        | Z.AI/Claude        |   | + sessions     |
        +--------------------+   +----------------+

  (merge-coordinator)
  cron 0 */2 * * *  -> /usr/local/bin/merge-coordinator (Rust, new)
    - PID lock /tmp/merge-coordinator.lock (30s timeout)
    - eval each open PR sequentially
    - if mergeable + PR body has "Fixes #N" -> merge -> auto-close issue N
    - structured JSON logs to stdout
    - exit 0=success, 1=eval-failures, 2=critical
```

### Data Flow (after change)

```
cron tick (30s)
  -> reconcile_loop
     -> for each agent due:
        -> KG router selects (planning/decision/impl/review tier)
        -> first_healthy_route skips unhealthy providers
        -> spawn via cli_tool (opencode OR /usr/local/bin/pi-rust if fallback_provider="pi-rust")
        -> wait for exit OR wall-clock OR config-error circuit-breaker (NEW)
        -> classify exit
     -> if 3 consecutive Config-validation-errors for one agent name -> set enabled=false (NEW)
     -> if RSS > 80 % MemoryHigh -> graceful restart timer (NEW)

cron 0 */2 * * * (every 2 h)
  -> /usr/local/bin/merge-coordinator (NEW Rust binary)
     -> pid lock
     -> list open PRs via terraphim_tracker::gitea
     -> for each mergeable PR with "Fixes #N":
        -> merge (with retry/backoff 1s/2s/4s)
        -> close issue N (PATCH /repos/.../issues/{N})
     -> exit 0/1/2
```

### Key Design Decisions

| Decision | Rationale | Alternatives Rejected |
|---|---|---|
| **New KG tier bucket "decision" priority=65** | Cleaner than per-agent flag; matches existing pattern; opt-in by task synonyms | Per-agent opt-in flag (config drift) |
| **`fallback_provider="pi-rust"` reuses `bypass_kg_routing`** | No new flag; one switch statement in 3 fallback respawn sites | New `fallback_to_pi: bool` field (duplicates intent) |
| **Investigate Z.AI probe, do not exclude** | `zai-coding-plan/glm-5.1` is already in `planning_tier.md` + `implementation_tier.md`; probe failure is operational not architectural | Mark Z.AI dead in router (loses a free provider) |
| **Rust merge-coordinator as a separate binary, not a tokio task in orchestrator** | Cron-driven, sequential, needs PID lock -- different lifecycle from orchestrator | In-orchestrator task (couples failure modes) |
| **Auto-close lives inside merge-coordinator, gated on `Fixes #N` in PR body** | merge-coordinator already calls Gitea API; `Fixes` is Gitea-native; `Refs` is *not* a close-triggering pattern | Orchestrator-side webhook listener (more code, same effect) |
| **Debug redaction via manual `impl Debug`** (not derive macro) | Pattern already used in tinyclaw `LinearConfig`; minimal new code | `derive_redact` crate (new dep for 6 structs) |
| **Circuit-breaker as in-orchestrator counter, not external** | One process owns the state; no need for shared store | Persisted breaker state in sqlite (premature) |
| **Memory watchdog via systemd `MemoryHigh=80G` + custom `OnFailure=` unit** | Native systemd; no new code | Periodic RSS poll in orchestrator (adds CPU overhead) |

### Eliminated Options

| Option Rejected | Why | Risk if Included |
|---|---|---|
| Add `decision_tier` *and* `synthesis_tier` *and* `triage_tier` | Vital few = 1 new tier | Tier explosion; ranking ambiguity |
| Wire pi-rust as primary spawner | Premature; opencode works for the live fleet | Single-point-of-failure swap |
| Restructure conf.d with new `[tier]` sections | Existing flat `[[agents]]` works | Migration cost dwarfs gain |
| Add a UI for ADF circuit-breaker state | Journal output is enough | Scope creep |
| Persist provider_probe results across restarts | TTL=1800s + restart is rare | Extra disk write on every probe |
| Tighten wall-clock to 600s for all agents | Some agents (product-development) need 1 h+ | Mass failure |
| Replace `flow-states` with a database | Filesystem works | Migration risk |
| Run merge-coordinator continuously (event-driven) | Cron + PID lock per spec; simplicity | Race conditions |
| Add ADF agent to recommend ADF improvements | Recursive; out of scope | Token spend with no gain |

### Simplicity Check

> "What if this could be easy?"

Easy version: **add one markdown file (`decision_tier.md`), one match arm (`fallback_provider="pi-rust"` -> use pi-rust CLI), one Rust crate (`merge-coordinator`), six Debug impls, and three systemd directives.**

That is the design. No new abstractions, no traits, no async coordination primitives beyond what `terraphim_orchestrator` already has. The work is mostly mechanical.

**Senior Engineer Test**: ✅ A senior engineer reading this would say "this is basically taking what already works and adding more of the same."

**Nothing Speculative Checklist:**
- [x] No features the user didn't request
- [x] No abstractions "in case we need them later"
- [x] No flexibility "just in case"
- [x] No error handling for scenarios that cannot occur (KG-router already has the fallback layering)
- [x] No premature optimization (one probe fix, one circuit-breaker; nothing more)

## File Changes

### New Files

| File | Purpose |
|---|---|
| `docs/taxonomy/routing_scenarios/adf/decision_tier.md` | KG taxonomy: priority 65; tier for analysis, log triage, retrospective; routes to pi-rust + opencode |
| `crates/terraphim_merge_coordinator/Cargo.toml` | New crate |
| `crates/terraphim_merge_coordinator/src/lib.rs` | Library: PID lock, evaluation loop, Gitea API client |
| `crates/terraphim_merge_coordinator/src/main.rs` | Binary entry (cron-invoked) |
| `crates/terraphim_merge_coordinator/src/types.rs` | `EvalVerdict`, `MergeOutcome`, `ExitCode` |
| `crates/terraphim_merge_coordinator/tests/integration.rs` | End-to-end with real Gitea against a test issue |
| `adr/ADR-007-decision-tier-pi-rust-spawner.md` | One ADR for both decisions |
| `scripts/bigbox-sync.sh` | Idempotent ff-only sync of bigbox to `main` |
| `scripts/install-pi-rust-bigbox.sh` | `cargo install --path /data/projects/.../pi_agent_rust` |
| `systemd/adf-orchestrator-restart.service` | `OnFailure=` graceful restart |
| `systemd/adf-orchestrator.service.d/memory.conf` | drop-in: `MemoryHigh=80G` |

### Modified Files

| File | Changes |
|---|---|
| `docs/taxonomy/routing_scenarios/adf/planning_tier.md` | Add `route:: openai, opencode/gpt-5.5`; add `route:: openai, opencode/gpt-5.4`; add `route:: pi-rust-openai-codex, openai-codex/gpt-5.5` |
| `docs/taxonomy/routing_scenarios/adf/implementation_tier.md` | Add `route:: minimax, minimax-coding-plan/MiniMax-M2.7-highspeed`; add `route:: pi-rust-minimax, minimax-coding-plan/MiniMax-M2.7-highspeed` (cross-CLI parity) |
| `crates/terraphim_orchestrator/src/lib.rs` | At 3 fallback respawn sites (1977, ~6304, ~6876, ~6921, ~6930): branch on `fallback_provider == "pi-rust"` and substitute CLI path; add config-error circuit-breaker counter to `AgentRunRecord` |
| `crates/terraphim_orchestrator/src/config.rs` | Document `fallback_provider="pi-rust"` as a reserved value in AgentDefinition Rustdoc; no struct change |
| `crates/terraphim_orchestrator/src/provider_probe.rs` | Add `zai-coding-plan` probe path (fix endpoint/timeout); add `pi-rust` health probe (binary present + `pi-rust --version` exit 0) |
| `crates/terraphim_orchestrator/src/kg_router.rs` | Sanity test: ensure decision_tier (priority 65) routes between planning and implementation |
| `crates/terraphim_orchestrator/Cargo.toml` | Add workspace dep on `terraphim_merge_coordinator` (lib, not bin) for shared Gitea types |
| `crates/terraphim_tinyclaw/src/config.rs` | `impl Debug` for `TelegramConfig`, `DiscordConfig`, `SlackConfig`, `MatrixConfig` -- redact secrets (#1804) |
| `crates/terraphim_tracker/src/gitea.rs` | `impl Debug` for `GiteaConfig` -- redact token (#1804) |
| `crates/terraphim_github_runner_server/src/config/mod.rs` | `impl Debug` for `Settings` -- redact `github_webhook_secret`, `github_token`, `firecracker_auth_token` (#1804) |
| `Cargo.toml` (workspace) | Add `crates/terraphim_merge_coordinator` to `members` |
| `crates/terraphim_orchestrator/src/agent_run_record.rs` | Add `consecutive_config_errors: u32` field |
| `/etc/cron.d/merge-coordinator` (on bigbox, runbook) | `0 */2 * * * alex /usr/local/bin/merge-coordinator >> /var/log/merge-coordinator.log 2>&1` |

### Deleted Files

| File | Reason |
|---|---|
| `scripts/merge-coordinator.py` (bigbox only) | Replaced by Rust binary |
| `scripts/merge-coordinator-gate.sh` (bigbox only) | Replaced by Rust binary |

## API Design

### `crates/terraphim_merge_coordinator/src/types.rs`

```rust
//! Exit code semantics per spec (Operational-1)
#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExitCode {
    Success = 0,
    EvaluationFailures = 1,
    Critical = 2,
}

/// One evaluation of one open PR.
#[derive(Debug, Clone)]
pub struct PrEvaluation {
    pub pr_index: u64,
    pub repo: String,            // "terraphim/terraphim-ai"
    pub mergeable: bool,
    pub fixes_issues: Vec<u64>,  // parsed from PR body "Fixes #N"
    pub verdict: EvalVerdict,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EvalVerdict {
    Merge,        // ready, will merge + close fixes
    Hold(String), // reason
    Conflicting,  // multiple subagent verdicts disagree (Edge-2)
}

#[derive(Debug)]
pub enum MergeOutcome {
    Merged { closed_issues: Vec<u64> },
    PartialFailure { merged: bool, close_errors: Vec<u64> },
    Skipped(String),
}

#[derive(Debug, thiserror::Error)]
pub enum MergeCoordinatorError {
    #[error("PID lock held by another instance (pid={pid}, age={age_secs}s)")]
    LockHeld { pid: i32, age_secs: u64 },

    #[error("Gitea API failure after 3 retries: {0}")]
    GiteaApi(String),

    #[error("partial failure: merge ok, close failed for issues {0:?}")]
    PartialFailure(Vec<u64>),

    #[error(transparent)]
    Io(#[from] std::io::Error),
}
```

### `crates/terraphim_merge_coordinator/src/lib.rs` (key signatures)

```rust
/// Acquire file-based PID lock per spec (Concurrency-1).
/// Returns `LockHeld` if another instance holds the lock and hasn't aged
/// past `stale_after_secs`.
pub fn acquire_pid_lock(path: &Path, stale_after_secs: u64)
    -> Result<PidLockGuard, MergeCoordinatorError>;

/// Parse a PR body and extract `Fixes #N` (case-insensitive) references.
/// `Refs #N` is intentionally **not** matched (no auto-close).
pub fn extract_fixes(body: &str) -> Vec<u64>;

/// Evaluate all open PRs in `repo`, sequentially (Concurrency-2).
/// Returns one PrEvaluation per PR. Does not perform merges.
pub async fn evaluate_all(
    gitea: &GiteaClient,
    repo: &str,
) -> Result<Vec<PrEvaluation>, MergeCoordinatorError>;

/// Merge a PR and close referenced issues, with retry/backoff 1s/2s/4s
/// (Failure-3). Returns `PartialFailure` if merge succeeds but any issue
/// close fails (Failure-1: stop, do not return success).
pub async fn merge_and_close(
    gitea: &GiteaClient,
    eval: &PrEvaluation,
) -> Result<MergeOutcome, MergeCoordinatorError>;

/// Structured JSON log emitter (Observability-1). One JSON object per line.
pub fn log_event(event: &str, fields: &[(&str, &dyn erased_serde::Serialize)]);
```

### KG router taxonomy: `decision_tier.md`

```markdown
# Decision Tier

Analytical decisions over execution data: log triage, retrospective,
quality evaluation, conflict resolution between agent verdicts, post-merge
gate assessment. Lower priority than strategic planning but above
implementation -- requires reasoning, not just code.

Maps to ZDP phase: disciplined-verification + disciplined-validation
(retrospective and quality dimensions).

priority:: 65

synonyms:: analyse logs, log triage, root cause analysis, incident review
synonyms:: post-merge gate, merge verdict, conflict resolution
synonyms:: nightwatch retrospective, quality evaluation, KLS rating
synonyms:: spec validation gap analysis, decision audit
synonyms:: pattern detection, anomaly review, fleet health

trigger:: analytical decisions over execution data with session continuity

route:: pi-rust-openai-codex, openai-codex/gpt-5.5
action:: /usr/local/bin/pi-rust -p "{{ prompt }}" --provider openai-codex --model gpt-5.5 --continue {{ session_id }}

route:: openai, opencode/gpt-5.5
action:: /home/alex/.bun/bin/opencode run -m {{ model }} --format json "{{ prompt }}"

route:: kimi, kimi-for-coding/k2p6
action:: /home/alex/.bun/bin/opencode run -m {{ model }} --format json "{{ prompt }}"
```

### Modified `planning_tier.md` (additions only)

```markdown
route:: openai, opencode/gpt-5.5
action:: /home/alex/.bun/bin/opencode run -m {{ model }} --format json "{{ prompt }}"

route:: pi-rust-openai-codex, openai-codex/gpt-5.5
action:: /usr/local/bin/pi-rust -p "{{ prompt }}" --provider openai-codex --model gpt-5.5
```

### Modified `implementation_tier.md` (additions only)

```markdown
route:: minimax, minimax-coding-plan/MiniMax-M2.7-highspeed
action:: /home/alex/.bun/bin/opencode run -m {{ model }} --format json "{{ prompt }}"

route:: pi-rust-minimax, minimax-coding-plan/MiniMax-M2.7-highspeed
action:: /usr/local/bin/pi-rust -p "{{ prompt }}" --provider minimax-coding-plan --model MiniMax-M2.7-highspeed
```

### Orchestrator change: cross-CLI substitution

```rust
// crates/terraphim_orchestrator/src/lib.rs around line 1977 (existing
// `bypass_kg_routing` short-circuit) and the 3 fallback respawn sites
// (~6304, ~6876, ~6921, ~6930). Centralise in a helper:

/// Returns the CLI tool path to use for spawn, honouring the special
/// `fallback_provider="pi-rust"` cross-CLI substitution.
fn resolved_cli_tool(def: &AgentDefinition) -> &str {
    match def.fallback_provider.as_deref() {
        Some("pi-rust") => "/usr/local/bin/pi-rust",
        _ => def.cli_tool.as_str(),
    }
}
```

### Debug redaction pattern (one example; same for 5 other structs)

```rust
// crates/terraphim_tinyclaw/src/config.rs

#[derive(Clone, Deserialize)]  // drop derive(Debug)
pub struct TelegramConfig {
    pub token: String,
    pub chat_id: i64,
}

impl std::fmt::Debug for TelegramConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TelegramConfig")
            .field("token", &"***REDACTED***")
            .field("chat_id", &self.chat_id)
            .finish()
    }
}
```

### Circuit-breaker (orchestrator-side)

```rust
// crates/terraphim_orchestrator/src/agent_run_record.rs

#[derive(Default)]
pub struct AgentRunRecord {
    pub last_exit_class: Option<ExitClass>,
    pub consecutive_config_errors: u32,   // NEW
    // ... existing fields
}

impl AgentRunRecord {
    /// Returns true if the agent should be quarantined (enabled=false).
    /// Threshold: 3 consecutive Config validation errors.
    pub fn should_quarantine(&self) -> bool {
        self.consecutive_config_errors >= 3
    }
}
```

### Probe extension (Z.AI fix + pi-rust health)

```rust
// crates/terraphim_orchestrator/src/provider_probe.rs

/// Probe `zai-coding-plan` via `opencode run -m zai-coding-plan/glm-5.1 "ping"`
/// with 30s wall-clock instead of the existing 60s default. Failures here
/// indicate either ZHIPU_API_KEY missing or endpoint unreachable -- both
/// distinguishable from "model unhealthy" by exit code + stderr inspection.
async fn probe_zai_coding_plan(...) -> ProbeOutcome { ... }

/// Probe `pi-rust` health: binary present + `pi-rust --version` exit 0
/// within 5s. No model call (would cost a token).
async fn probe_pi_rust(...) -> ProbeOutcome { ... }
```

## Test Strategy

### Unit Tests

| Test | Location | Purpose |
|---|---|---|
| `kg_router::decision_tier_routes_between_planning_and_impl` | `kg_router.rs` | Priority 65 lands between 80 and 50 |
| `kg_router::synonym_log_triage_routes_to_decision_tier` | `kg_router.rs` | "analyse logs" hits decision tier |
| `kg_router::planning_tier_includes_gpt_5_5` | `kg_router.rs` | New route present |
| `kg_router::impl_tier_includes_m2_7_highspeed` | `kg_router.rs` | New route present |
| `resolved_cli_tool_substitutes_pi_rust` | `lib.rs` | `fallback_provider="pi-rust"` -> `/usr/local/bin/pi-rust` |
| `resolved_cli_tool_default_unchanged` | `lib.rs` | Other fallback_provider values unaffected |
| `should_quarantine_after_three_config_errors` | `agent_run_record.rs` | Threshold |
| `extract_fixes_matches_fixes_not_refs` | `merge_coordinator/lib.rs` | `Refs #N` does NOT trigger close |
| `extract_fixes_handles_case_insensitive_multiple` | `merge_coordinator/lib.rs` | `fixes #1 closes #2 FIXES #3` -> `[1, 2, 3]` |
| `acquire_pid_lock_returns_lock_held_when_active` | `merge_coordinator/lib.rs` | Concurrency-1 |
| `acquire_pid_lock_steals_stale_lock_after_timeout` | `merge_coordinator/lib.rs` | 30s stale timeout |
| `merge_and_close_partial_failure_returns_error` | `merge_coordinator/lib.rs` | Failure-1: merge ok + close fail -> err |
| `merge_and_close_retries_with_backoff_1_2_4` | `merge_coordinator/lib.rs` | Failure-3 |
| `telegram_config_debug_redacts_token` | `tinyclaw/config.rs` | #1804 |
| `gitea_config_debug_redacts_token` | `tracker/gitea.rs` | #1804 |
| `settings_debug_redacts_all_secrets` | `github_runner_server/config/mod.rs` | #1804 |

### Integration Tests

| Test | Location | Purpose |
|---|---|---|
| `merge_coordinator_e2e_against_live_gitea` | `crates/terraphim_merge_coordinator/tests/integration.rs` | Real Gitea test repo; create PR with `Fixes #N`; verify merge + close |
| `pi_rust_alternative_spawner_substitution` | `crates/terraphim_orchestrator/tests/spawner.rs` | Set `fallback_provider="pi-rust"`, spawn, assert pi-rust path used |
| `zai_probe_recovers_after_endpoint_change` | `crates/terraphim_orchestrator/tests/probe.rs` | Mock the underlying CLI invocation via temp script; probe distinguishes "endpoint timeout" from "API key missing" |
| `quarantine_after_three_consecutive_config_errors` | `crates/terraphim_orchestrator/tests/quarantine.rs` | Inject 3 config-error exit classifications; assert `enabled=false` on the in-memory def |

### Property Tests

```rust
// crates/terraphim_merge_coordinator/tests/proptest.rs
proptest! {
    /// extract_fixes never panics on arbitrary PR body
    #[test]
    fn extract_fixes_never_panics(body: String) {
        let _ = extract_fixes(&body);
    }

    /// extract_fixes is case-insensitive
    #[test]
    fn extract_fixes_case_insensitive(n: u32) {
        let n = n % 10_000;
        let upper = format!("FIXES #{}", n);
        let lower = format!("fixes #{}", n);
        prop_assert_eq!(extract_fixes(&upper), extract_fixes(&lower));
    }
}
```

### Live verification (post-deploy on bigbox)

```bash
# 1. After deploy, journal should show new routes
ssh bigbox 'sudo journalctl -u adf-orchestrator --since "5 min ago"' \
  | rg -i 'decision_tier|gpt-5.5|MiniMax-M2.7-highspeed|pi-rust'

# 2. Trigger a decision-tier agent (log triage)
gtr comment --owner terraphim --repo terraphim-ai --index 1797 \
  --body "@adf:log-analyst analyse logs since restart"
# expect journal: "model selected via KG tier routing ... tier=decision ... model=opencode/gpt-5.5"

# 3. Force opencode failure to test pi-rust substitution
ssh bigbox 'sudo mv /home/alex/.bun/bin/opencode /home/alex/.bun/bin/opencode.disabled'
# trigger another decision-tier agent; expect pi-rust spawn
# restore: ssh bigbox 'sudo mv /home/alex/.bun/bin/opencode.disabled /home/alex/.bun/bin/opencode'

# 4. Verify merge-coordinator
ssh bigbox 'sudo -u alex /usr/local/bin/merge-coordinator --dry-run'
# expect: one JSON object per evaluated PR; exit 0; no merges; no closes
```

## Implementation Steps

### Step 1: Investigate + fix Z.AI probe
**Files:** `crates/terraphim_orchestrator/src/provider_probe.rs`, `tests/probe.rs`
**Description:** Reproduce the timeout on bigbox; trace into probe code; either (a) tighten timeout to 30s + improved stderr classification, or (b) fix endpoint URL/auth env var. Add unit test that distinguishes "auth missing" from "endpoint timeout" from "model unhealthy".
**Tests:** `zai_probe_recovers_after_endpoint_change`, plus targeted unit tests on the parse helpers
**Estimated:** 2 hours
**Dependencies:** none -- can run first as it is the lowest-risk + highest-information spike

### Step 2: ~~Install pi-rust on bigbox~~ **REMOVED**
Already installed at `/home/alex/.local/bin/pi-rust` (v0.1.10, 2026-03-22, 44 MB). Smoke-test inline with Step 3 below by calling `pi-rust --list-providers` and `pi-rust -p "ping" --provider openai-codex --model gpt-5.5 --max-turns 1`.

### Step 3: Add decision_tier.md + extend planning + impl tier taxonomy
**Files:** `docs/taxonomy/routing_scenarios/adf/decision_tier.md` (new), `planning_tier.md`, `implementation_tier.md`
**Description:** Markdown-only changes. KG router picks them up automatically on load.
**Tests:** `decision_tier_routes_between_planning_and_impl`, `planning_tier_includes_gpt_5_5`, `impl_tier_includes_m2_7_highspeed`, `synonym_log_triage_routes_to_decision_tier`
**Estimated:** 1 hour
**Dependencies:** Step 2 (pi-rust available so action lines are valid)

### Step 4: ~~Cross-CLI substitution in orchestrator~~ **REPLACED** with: taxonomy route lines for pi-rust
**Files:** `docs/taxonomy/routing_scenarios/adf/decision_tier.md`, `planning_tier.md`, `implementation_tier.md`; `adr/ADR-007-decision-tier-pi-rust-routes.md`
**Description:** No Rust change. Add `route::` + `action::` pairs that invoke pi-rust directly:
```
route:: pi-rust-openai-codex, gpt-5.5
action:: /home/alex/.local/bin/pi-rust --provider openai-codex --model {{ model }} -p "{{ prompt }}"

route:: pi-rust-minimax, MiniMax-M2.7-highspeed
action:: /home/alex/.local/bin/pi-rust --provider minimax-coding-plan --model {{ model }} -p "{{ prompt }}"
```
Optional per-agent: in conf.d/*.toml, set `fallback_provider = "/home/alex/.local/bin/pi-rust"` for agents where pi-rust is the fallback path; the orchestrator already invokes whatever path is in that field (lib.rs:6313).
ADR-007 records the decision: pi-rust integrates via taxonomy templates, not via a Rust helper; the `terraphim_pi_agent` bridge crate (#1785) supersedes when it lands.
**Tests:** `pi_rust_route_in_decision_tier`, `pi_rust_action_template_renders` (in kg_router unit tests)
**Estimated:** 1 hour
**Dependencies:** Step 3

### Step 5: Debug redaction (#1804)
**Files:** `crates/terraphim_tinyclaw/src/config.rs`, `crates/terraphim_tracker/src/gitea.rs`, `crates/terraphim_github_runner_server/src/config/mod.rs`
**Description:** Drop `#[derive(Debug)]`; add manual `impl Debug` per the pattern in `LinearConfig`. 6 structs total.
**Tests:** `telegram_config_debug_redacts_token`, `gitea_config_debug_redacts_token`, `settings_debug_redacts_all_secrets`
**Estimated:** 2 hours
**Dependencies:** none (parallel-safe)

### Step 6: Config-error circuit-breaker
**Files:** `crates/terraphim_orchestrator/src/agent_run_record.rs`, `lib.rs`, `tests/quarantine.rs`
**Description:** Add `consecutive_config_errors: u32` to `AgentRunRecord`. On exit classification, increment if `ExitClass::ConfigError`, reset to 0 on any other class. When threshold (3) reached, set `def.enabled = false` in-memory and emit a `WARN` event. Persist `enabled=false` to `conf.d/<file>.toml` only behind an explicit flag (not default).
**Tests:** `should_quarantine_after_three_config_errors`, `quarantine_after_three_consecutive_config_errors`
**Estimated:** 2 hours
**Dependencies:** Step 4 (test ergonomics)

### Step 7: Rust merge-coordinator crate (#1805)
**Files:** entire `crates/terraphim_merge_coordinator/` (types, lib, main, tests, Cargo.toml); workspace `Cargo.toml`
**Description:** Implement per `.docs/spec-merge-coordinator.md`:
- PID lock (Concurrency-1, 30s stale timeout)
- Sequential PR evaluation (Concurrency-2)
- Partial failure -> CRITICAL log + exit 2 (Failure-1, Operational-1)
- Remediation atomicity (Failure-2): if close fails, do NOT continue
- Retry/backoff 1s/2s/4s (Failure-3)
- Structured JSON log per spec (Observability-1)
- Exit codes 0/1/2 (Operational-1)
- Token never in argv (Security-2)
- Edge-2: conflicting verdicts logged
- **Auto-close on merge**: parse `Fixes #N`, PATCH issue to `state=closed` after successful merge
**Tests:** all unit tests in §Test Strategy; one e2e integration test against a throwaway Gitea repo
**Estimated:** 5 hours
**Dependencies:** none (can start in parallel; lands behind feature flag in cron)

### Step 8: Memory watchdog
**Files:** `systemd/adf-orchestrator.service.d/memory.conf` (drop-in), `systemd/adf-orchestrator-restart.service`
**Description:** Set `MemoryHigh=80G`; add `OnFailure=adf-orchestrator-restart.service` and a unit that does `systemctl restart adf-orchestrator` after a 60s grace.
**Tests:** runbook step on bigbox (synthetic memory pressure check); no Rust tests
**Estimated:** 1 hour
**Dependencies:** none

### Step 9: Bigbox sync runbook
**Files:** `scripts/bigbox-sync.sh`, `.docs/runbook-bigbox-sync-2026-05-23.md`
**Description:** `git fetch && git checkout main && git merge --ff-only`; fail loudly if working tree dirty (memory rule: no `--hard`). Document the deploy: rebuild binary, `install /usr/local/bin/adf`, `systemctl restart adf-orchestrator`. **pi-rust install no longer needed** (already at `/home/alex/.local/bin/pi-rust`).
**Tests:** manual; runbook reviewed
**Estimated:** 1 hour
**Dependencies:** Steps 1, 3-8 (deploys the cumulative result)

### Step 10: Deploy + live verification
**Files:** N/A (operational)
**Description:** Run `bigbox-sync.sh`; install merge-coordinator binary + cron; install pi-rust; verify via the 4 live verification steps in §Test Strategy. Close #1804, #1805, #1797 with evidence.
**Tests:** live journal traces; manual scoring against success-criteria table
**Estimated:** 1 hour
**Dependencies:** all prior steps

**Total: 14 hours** (was 20h before bigbox inspection). Parallelisable: Steps 1, 5, 7, 8 can run concurrently; critical path is Step 7 (merge-coordinator crate, 5h) + Step 10 (deploy, 1h) ≈ 6h wall-clock.

## Rollback Plan

If issues discovered:
1. **Quick rollback**: `git checkout <pre-deploy-sha>` on bigbox + restart -- restores the validated `94ebd7517` state.
2. **Selective rollback** (one tier): delete `decision_tier.md`, restart orchestrator. KG router silently drops the rule.
3. **Disable pi-rust spawner**: set `fallback_provider=None` on affected agents in `conf.d/terraphim.toml`. Falls back to opencode.
4. **Disable merge-coordinator**: comment out the cron line in `/etc/cron.d/merge-coordinator`. No effect on orchestrator.
5. **Re-enable quarantined agent**: edit conf.d, set `enabled = true`, restart.

No DB migration to roll back. No persistent state change.

Feature gating: the new merge-coordinator binary is invoked only by cron; not installing the cron file is the kill switch.

## Migration

None. No schema, no persistent state changes.

The legacy `scripts/merge-coordinator.py` + `merge-coordinator-gate.sh` on bigbox are deleted in Step 10 after the Rust binary is verified for one cron cycle.

## Dependencies

### New Dependencies (`crates/terraphim_merge_coordinator/Cargo.toml`)

| Crate | Version | Justification |
|---|---|---|
| `tokio` | workspace | async runtime |
| `serde` + `serde_json` | workspace | structured logs |
| `thiserror` | workspace | error type |
| `reqwest` | workspace | Gitea API client |
| `tracing` | workspace | structured logs |
| `fs2` | "0.4" | file-based PID lock (per spec); already in workspace lockfile |
| `proptest` (dev) | workspace | extract_fixes property tests |
| `terraphim_tracker` | workspace | reuse Gitea client (not a new HTTP client) |

### Dependency Updates

None.

## SRD Traceability

N/A -- this is an ADF stability change, not a formal SRD'd release.

## Performance Considerations

### Expected Performance

| Metric | Target | Measurement |
|---|---|---|
| KG router load with 4 tier files (was 3) | < 50 ms | `kg_router::load()` unit timer |
| Resolved CLI tool helper | O(1), no allocation | inspection |
| Merge-coordinator per-PR evaluation | < 5 s | end-to-end test |
| Memory overhead from `consecutive_config_errors` field | +4 bytes per agent | static analysis |
| Probe latency (zai-coding-plan, fixed) | < 30 s | probe unit test |

### Benchmarks to Add

```rust
// crates/terraphim_merge_coordinator/benches/extract_fixes.rs
#[bench]
fn bench_extract_fixes_typical_body(b: &mut Bencher) {
    let body = "Fixes #1234. Refs #5678. Closes #9012.";
    b.iter(|| extract_fixes(body));
}
```

(Only added if extract_fixes shows up in a profile; otherwise skip.)

## Open Items

| Item | Status | Owner |
|---|---|---|
| Bigbox `cargo install` of pi-rust succeeds on Linux | Pending Step 2 | alex |
| `pi-rust --continue <session-id>` flag syntax confirmation | Pending Step 2 | alex |
| `merge-coordinator --dry-run` flag spec (not in #1805 body) | Pending Step 7 | alex |
| Whether to persist `enabled=false` to conf.d when quarantining | Pending Step 6 | alex (default: in-memory only, journal logs) |

## Approval

- [ ] Technical review complete
- [ ] Test strategy approved
- [ ] Performance targets agreed
- [ ] **Human approval received**

---

## Gate Criteria Checklist

### Standard
- [x] All file changes listed (11 new files, 12 modified, 2 deleted)
- [x] All public APIs defined (`ExitCode`, `PrEvaluation`, `EvalVerdict`, `MergeOutcome`, `MergeCoordinatorError`, `acquire_pid_lock`, `extract_fixes`, `evaluate_all`, `merge_and_close`, `resolved_cli_tool`, `AgentRunRecord::should_quarantine`)
- [x] Test strategy complete (15 unit, 4 integration, 2 proptest, 4 live verification)
- [x] Steps sequenced (10 steps, dependencies marked)
- [x] Performance targets set
- [ ] Human approval received

### Essentialism
- [x] 5 or fewer major components in scope (5 exactly)
- [x] Eliminated Options populated (9 entries)
- [x] Avoid At All Cost list (20 entries -- full 5/25 rule)
- [x] Simplicity Check answered (effortless, mechanical)
- [x] 5/25 Rule applied

**Awaiting human approval to proceed to Phase 2.5 (disciplined-specification) or Phase 3 (disciplined-implementation).**
