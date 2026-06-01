# Research: ADF Self-Healing with pi-rust (pi_agent_rust) + Codex GPT-5.5 Planning Tier

**Status**: Draft (Phase 1, awaiting approval)
**Author**: Claude (this session)
**Date**: 2026-05-23
**Reviewers**: alex
**Successor design**: `.docs/design-adf-self-healing-2026-05-23.md` (to be produced after approval)
**Refs**: epic #1807, #1804, #1805, #1797

## Executive Summary

Bigbox ADF is up but only 37 % of runs (24 h) succeed; the rest hit wall-clock timeouts, config errors, or rate-limit cascades. The architecture is sound (KG-router fallback fix landed in #1794), but operational hygiene is missing: stuck `merge-coordinator`, no per-step interrupt, no auto-close on PR merge, no quarantine for repeatedly-failing agents, and the planning tier currently routes to a degraded Anthropic. The fix path is short: ship the merge-coordinator Rust rewrite (#1805), redact debug logs (#1804), add a planning tier on **`opencode/gpt-5.5`** (subscription-flat-cost via existing opencode CLI) and a stateful "decision" tier on `pi-rust (pi_agent_rust)` (local binary, session-persistent), and add the four operational guardrails (auto-close, circuit-break, memory watchdog, per-step budgets). Target: 5 + agents reliable overnight by 2026-06-15.

## Essential Questions Check

| Question | Answer | Evidence |
|---|---|---|
| Energizing? | Yes | North Star Q2 P2; "Stalled" today; user explicitly asked for it |
| Leverages strengths? | Yes | KG router, terraphim_orchestrator, pi-rust (pi_agent_rust), codex CLI all in-house or installed |
| Meets real need? | Yes | 37 % success on a fleet that is meant to ship merged PRs overnight |

**Proceed: Yes (3/3).**

## Problem Statement

### Description
ADF orchestrator on bigbox spawns ~10 agents/hour. In the last 24 h: 13 success, 11 unknown, 7 rate_limit, 4 compilation_error, **zero merged PRs**. The orchestrator works, the routing works (post-#1794), but agents are not delivering value because:

1. **`merge-coordinator` config-error loop** -- 15 identical "working directory does not exist" failures retried every cron tick. No circuit breaker.
2. **Wall-clock kill is the dominant non-success exit** -- agents spend their entire budget (20 min – 2 h) before being SIGKILL'd, leaving PRs open.
3. **Anthropic planning tier degraded** -- planning-tier agents (sonnet, opus) cannot route reliably; the fleet has minimal planning capacity (Z.AI probe times out but is *not confirmed dead* -- needs investigation, not exclusion).
4. **No auto-close on PR merge** -- Gitea issues with merged PRs get re-dispatched on the next tick (lesson learned in Symphony already).
5. **No memory watchdog** -- orchestrator at 84.3 G / 90 G high; one bad spawn from OOM-kill.
6. **Debug-derived secrets leak** in tinyclaw, tracker, github-runner configs (#1804, P0).

### Impact
- Zero merged PRs in 48 h. North Star "5 + agents reliable overnight" miss.
- API spend on retries with no output (rate_limit + wall_clock = wasted tokens).
- One bad config can OOM-kill the orchestrator and stop the fleet.
- P0 credential leak shown in compliance-watchdog output (security hygiene risk).

### Success Criteria
| Metric | Today | Target 2026-06-15 |
|---|---|---|
| 24 h success-rate of spawned agents | 37 % | ≥ 70 % |
| Merged PRs from autonomous agents per 24 h | 0 | ≥ 2 |
| Agents quarantined automatically after 3 consecutive config errors | 0 | 100 % of the broken ones |
| Anthropic-only agents (no fallback) on bigbox | 4 | 0 (all routed via planning tier on `opencode/gpt-5.5`) |
| Z.AI probe status investigated | timeout (unclear) | confirmed healthy or marked unhealthy with cause |
| Orchestrator OOM events / week | 0 | 0 (preserve) |
| Secrets visible in Debug output | yes | no |

## Current State Analysis

### Existing Implementation

| Component | Location | Status |
|---|---|---|
| Orchestrator binary | `crates/terraphim_orchestrator/` -> `/usr/local/bin/adf` on bigbox | Detached HEAD at `94ebd7517`, main at `0a2b3a434`; out of sync |
| KG router | `crates/terraphim_orchestrator/src/lib.rs:~6800-6920` | Working; `bypass_kg_routing` + `first_healthy_route` validated live 2026-05-22 |
| Agent definitions | `/opt/ai-dark-factory/conf.d/terraphim.toml` (15 agents) + `digital-twins.toml` | Active; tier scoring intact |
| Spawner | `crates/terraphim_spawner/` | Working; supports_stdin + positional args added |
| merge-coordinator | `scripts/merge-coordinator.py` + `merge-coordinator-gate.sh` | 8/14 spec violations (#1805) |
| Self-healing plan in #1807 | none merged | The issue is a **report** of problems found, not a design |
| GPT-5.5 access | Via existing `opencode` CLI as `opencode/gpt-5.5` and `opencode/gpt-5.5-pro` | **Confirmed live on bigbox via `opencode models`** |
| MiniMax-M2.7-highspeed | Via existing `opencode` CLI as `minimax-coding-plan/MiniMax-M2.7-highspeed` | **Confirmed live on bigbox via `opencode models`**; subscription-backed via `minimax-coding-plan` (same pattern as `kimi-for-coding/k2p6`) |
| pi-rust (pi_agent_rust) | Binary at `/Users/alex/.local/bin/pi-rust` (Mac); source at `/Users/alex/projects/pi_agent_rust`; **not yet installed on bigbox** | Single-binary, `pi-rust -p PROMPT` non-interactive, `--continue` for session memory, 8 built-in tools + `bash` (can call `gtr` directly) |
| pi-rust provider parity with opencode | **Confirmed via `pi-rust --list-providers` 2026-05-23**: `zai-coding-plan`, `minimax-coding-plan`, `kimi-for-coding`, `openai-codex` (aliases: `codex`, `chatgpt-codex`), `opencode` aggregator -- all subscription-backed coding plans present | pi-rust can act as a *drop-in alternative spawner* for the same models, useful as a self-healing fallback if opencode binary itself misbehaves |
| Provider probes | orchestrator-internal | Reports: anthropic FAIL, zai TIMEOUT (**probe issue, not confirmed dead**), openai/kimi/minimax PASS |

### Data Flow (current)

```
cron tick (30 s)
  -> reconcile_loop
     -> for each agent due:
        -> KG router selects provider/model (planning=80, impl=50, review=40)
        -> if primary unhealthy, fall to fallback (validated live)
        -> spawn via cli_tool (claude/opencode)
        -> wait for exit or wall-clock kill (1200/1800/3600/7200 s)
     -> classify exit (success/rate_limit/compilation_error/unknown/wall_clock)
     -> nightwatch alert if threshold breached
```

### Integration Points
- **Gitea API** (`https://git.terraphim.cloud`): issue lifecycle, PR creation, dependency graph
- **Gitea webhook** (`172.18.0.1:9091`): mention dispatch, PR open dispatch
- **Quickwit** (`http://127.0.0.1:7280/api/v1/adf-logs/search`): log indexing for nightwatch
- **systemd**: `adf-orchestrator.service`, `MemoryHigh=90G`, `MemoryMax=115G`
- **Provider CLIs**: claude (Anthropic), opencode (aggregator -- handles OpenAI/Kimi/MiniMax/Z.AI including `opencode/gpt-5.5`), pi (local-or-API stateful via `pi-rust (pi_agent_rust)`). **No standalone `codex` CLI dependency** -- GPT-5.5 routes through opencode like all other openai models.

## Constraints

### Technical Constraints
- **No mocks in tests** (CLAUDE.md). Provider integration must be tested against real CLIs.
- **No bigbox-direct implementation** (North Star). All changes go via prompts -> agents -> review.
- **Memory ceiling 90 G high / 115 G max** (systemd unit). New agents cannot inflate this.
- **bigbox + Mac git histories are unrelated**; deploy via fast-forward only on detached HEAD.
- **British English** (CLAUDE.md). Docs, log messages.
- **No `timeout` command on macOS** (CLAUDE.md). Use `gtimeout` or app-level wall-clock.
- **No `git reset --hard` on bigbox** (memory). Working trees may have agent edits.

### Business Constraints
- **2026-06-15 deadline** for "5 + agents reliable overnight" (North Star).
- **Two outstanding P0/P1 issues** (#1804 credentials, #1805 merge-coordinator) must close inside this work.
- **Sprint focus this week**: Odilo infrastructure + ADF marketing (North Star Current Focus). Cannot consume more than ~2 days of orchestration time.

### Integration Constraints
- **opencode auth for openai/gpt-5.5** uses existing `opencode providers` credential store on bigbox -- no new login lifecycle. Quota = ChatGPT subscription, flat cost.
- **pi-rust (pi_agent_rust)** session JSONLs go to `~/.config/pi/`; per-agent isolation needs distinct `PIAR_SESSION_DIR`.
- **opencode model IDs** for kimi/minimax change quarterly; the conf.d files reference `kimi-for-coding/k2p6` (current). Same drift risk for `opencode/gpt-5.5` -- needs a config-driven model id, not a hard-code.

### Non-Functional Requirements
| Requirement | Target | Current |
|---|---|---|
| Cron-tick reconcile | < 30 s | ~60 - 90 s under load (#1797 P2) |
| Per-agent wall-clock | 1200 - 7200 s | Same; need step-level budget instead |
| Orchestrator memory | < 80 G steady | 84.3 G now |
| Quarantine latency after N failures | < 60 s | infinite (no quarantine) |

## Vital Few (Essentialism)

### Essential Constraints (Max 3)
| Constraint | Why It's Vital | Evidence |
|---|---|---|
| **Close `merge-coordinator` config loop fast** | Until this stops, every gain elsewhere is consumed by retry spend | journal: 15 identical failures/24 h |
| **Add a *healthy* planning tier on `opencode/gpt-5.5`** | Anthropic planning route degraded; without a flat-cost subscription-backed planning model, design tasks fall to implementation-tier models (opencode/kimi) and under-perform | provider_probe; #1797 P0 rate_limit_cascade |
| **Auto-close on PR merge** | Without it, every other improvement still leaks into a re-dispatch loop | Symphony lesson, memory entry |

### Eliminated from Scope (5/25 rule)
| Eliminated | Why |
|---|---|
| Rewriting all 15 agents | Only 1 (merge-coordinator) has a spec violation; the rest work when not starved of capacity |
| Replacing Quickwit / nightwatch | Working; out of scope |
| GPU KG router or new ranking algo | Architecture works; problem is operational not algorithmic |
| Switching off opencode/kimi | Workhorse provider; do not destabilise |
| Multi-region / HA orchestrator | Single bigbox node is the contract; one process is fine |
| New marketing surface for ADF | North Star task; tracked separately; not this design |

## Dependencies

### Internal Dependencies
| Dependency | Impact | Risk |
|---|---|---|
| `crates/terraphim_orchestrator` | All routing + spawn logic. New tier additions land here. | Low: well-tested, recent #1794 fix proved the layering |
| `crates/terraphim_spawner` | New CLIs (codex, pi) need a small `AgentConfig` extension | Low: `supports_stdin` precedent in #1768 |
| `crates/terraphim_kg_agents` | KG tier scores (planning=80, impl=50, review=40). New tier "decision=65" between impl and planning would land here | Medium: must not change existing scores |
| `/opt/ai-dark-factory/conf.d/terraphim.toml` | Live config; one bad TOML kills the fleet | High: deploy via shadow file + reload, never edit in place |

### External Dependencies
| Dependency | Version | Risk | Alternative |
|---|---|---|---|
| `pi-rust (pi_agent_rust)` (`pi`) | Cargo workspace at `/Users/alex/projects/pi_agent_rust` | Local-only binary, no network if offline; provider config required | opencode directly |
| `opencode` | Bun install on bigbox + Mac | Quarterly model ID drift across all models including gpt-5.5 | None; primary aggregator |
| `opencode/gpt-5.5` model | **Confirmed live via `opencode models`** on bigbox 2026-05-23 | Subscription quota exhaustion under fleet-wide planning load | Auto-fallback to `opencode/gpt-5.4` via KG router |

## Risks and Unknowns

### Known Risks
| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| `opencode/gpt-5.5` subscription quota saturates under fleet-wide planning load | Med | Med | Quota probe + safety-floor block already in orchestrator (PR #1794); reuse. Auto-fallback to `opencode/gpt-5.4` then `opencode/gpt-5.3-codex` via KG tier |
| Z.AI probe timeout treated as "dead" hides a real but transient issue | Med | Med | Spike the actual Z.AI probe call; fix the probe before designing the planning tier around its absence |
| pi-rust (pi_agent_rust) session store grows unbounded | Med | Low | Per-agent `PIAR_SESSION_DIR` with rotation; mirror existing `flow-states` dir layout |
| Auto-close on PR merge fires on PRs that did not actually fix the linked issue | Med | High | Only auto-close when PR body contains `Fixes #N` (Gitea-native), not `Refs` |
| Memory watchdog restart races with mid-flight spawn | Low | Med | Drain: stop dispatch first, wait reconcile_tick=0, then restart |
| pi `--continue` session selects wrong session under concurrency | Med | Med | Use deterministic session id = `agent_name + tick_ts`; never `--continue` without explicit id |

### Open Questions
1. ~~Is GPT-5.5 exposed?~~ **RESOLVED**: `opencode/gpt-5.5` and `opencode/gpt-5.5-pro` both listed in `opencode models` on bigbox 2026-05-23. Lock planning-tier model id to `opencode/gpt-5.5`; fallback `opencode/gpt-5.4`; fallback `opencode/gpt-5.3-codex`.
2. **Why does Z.AI probe time out?** Network path? Wrong endpoint? Auth expired? -- needs a spike before declaring it offline.
3. ~~Does pi-rust have Gitea tools?~~ **RESOLVED**: 8 built-ins + `bash` is sufficient -- pi-rust calls `gtr` directly via the `bash` tool. No custom tool implementation needed.
4. **Should the new "decision" tier be a *third* KG score bucket** (planning=80, decision=65, impl=50, review=40) or a sub-attribute on agents? Cleaner to add the bucket; needs ADR.
5. **Is the existing `bypass_kg_routing` flag enough** for fallback to pi when *primary planning tier is down*, or do we need a separate `fallback_to_pi` flag? -- Tentative answer: reuse `bypass_kg_routing` with a new `fallback_provider = "pi"` value.

### Assumptions Explicitly Stated
| Assumption | Basis | Risk if Wrong | Verified? |
|---|---|---|---|
| `opencode/gpt-5.5` is callable on bigbox via existing `opencode` auth, no separate login needed | `opencode models` lists it; same auth surface as `opencode/gpt-5.4` which is already in active config | If gpt-5.5 needs a separate auth (e.g. plan upgrade), planning tier rate-limits | Partial: model id confirmed, runtime call not yet exercised |
| `pi-rust (pi_agent_rust)` is build-clean on bigbox (rust-toolchain.toml + crates.io reachable) | bigbox builds terraphim_orchestrator daily | If pi build fails, fall back to opencode | No (spike) |
| Adding a planning tier on `opencode/gpt-5.5` will not push subscription costs above the user's ChatGPT/opencode plan limit | The fleet today is starved, not over-spending | If we exceed plan, planning agents rate-limit; existing safety-floor catches it | No (monitor) |
| Auto-close-on-PR-merge logic belongs in the new Rust `merge-coordinator`, not the orchestrator | Merge-coordinator is being rewritten anyway (#1805); putting auto-close logic *inside* its Rust replacement is the natural home | If auto-close logic needs to predate merge-coordinator readiness, we need a stop-gap | Partial: design depends on #1805 sequencing |
| `pi --continue` semantics are stable enough that one agent role = one persistent session is safe | pi-rust (pi_agent_rust) README, SKILL.md | If session corruption is real (sqlite/jsonl bug), planning agents lose memory mid-run | No (spike) |
| The North Star "5 + agents reliable overnight" target counts agents that *successfully exit* and *produce a PR* | Spirit of the goal; "reliable" implies value-delivering | If counted only by "process exits 0", we hit it easily but miss the point | Yes (North Star wording supports this) |
| Z.AI is recoverable, not dead | Provider was working previously; only probe times out today | If Z.AI is genuinely permanently offline, this is moot; we still keep the spike to confirm | No (spike) |

### Multiple Interpretations Considered

**Q: Where does the new "planning tier" (Codex GPT-5.5) and "decision tier" (pi) live?**

| Interpretation | Implications | Chosen |
|---|---|---|
| **A**: Add as new `cli_tool` values in existing agents (opt-in per agent) | Minimal code change; flexible per-agent; harder to enforce fleet-wide tier strategy | Rejected: invites config drift |
| **B**: Add new KG tier score buckets (planning=80, decision=65) and let KG router select automatically | Centralised; matches existing design; needs ADR + spawner extension for new CLIs | **Chosen** |
| **C**: Add as a separate "MetaCoordinator" layer that sits above all agents and rewrites tasks | Cleanest abstraction but biggest delta; weeks of work | Rejected: out of time budget |

**Q: Should pi-rust (pi_agent_rust) run on bigbox or only on Mac?**

| Interpretation | Implications | Chosen |
|---|---|---|
| **A**: Mac-only, called via SSH from bigbox orchestrator | Adds network hop + auth surface; violates "never implement on bigbox directly" inverse | Rejected |
| **B**: Build pi on bigbox; spawn as local CLI like opencode/claude | One more provider; matches existing pattern | **Chosen** |
| **C**: Run pi as a sidecar HTTP server on bigbox | Cleaner if pi keeps growing; premature today | Defer |

## Research Findings

### Key Insights

1. **The architecture is not broken.** PR #1794 proved the routing + fallback layering works under live quota exhaustion. The 37 % success rate is operational debt (config loops, no auto-close, missing planning capacity), not an architectural defect.
2. **There is no `merge-coordinator` written to spec** -- the Python/shell predates the 2026-05-19 spec interview. Rewriting it in Rust (#1805) is both the security fix (no token in argv), the lifecycle fix (PID lock, exit codes), AND the natural home for "close issue on PR merge".
3. **The "Anthropic degraded" problem is *not* solved by KG-router fallback alone**: planning-tier scoring (=80) routes to sonnet/opus first. With Anthropic unhealthy, planning falls back to opencode/kimi which is an *implementation* model -- ok for code, weak for design. Adding `opencode/gpt-5.5` at the planning tier (same `opencode` CLI, just a different `--model` argument) is the missing piece, and requires no new auth surface.
4. **pi-rust (pi_agent_rust) adds two capabilities the existing fleet lacks**: (a) a *local, offline-capable, session-persistent* decision agent for nightwatch retrospective + log analysis; (b) **full provider parity with opencode** (`zai-coding-plan`, `minimax-coding-plan`, `kimi-for-coding`, `openai-codex`) -- i.e. pi-rust can act as a drop-in alternative spawner for the same subscription-backed models. This means a failing `opencode` binary is no longer a single point of failure: KG router can route to pi-rust for the same model id and re-spawn the agent.
5. **`opencode/gpt-5.5` is a subscription-flat-cost model** -- predictable cost surface, unlike per-token aggregators. Pushing planning traffic to it is cheaper than the equivalent on opencode/kimi at fleet scale.
6. **Auto-close-on-merge is a 30-line addition to the Rust merge-coordinator** -- it already needs to call Gitea API; one extra `PATCH /repos/.../issues/{idx}` is trivial.

### Relevant Prior Art
- **Symphony**: same auto-close-on-merge problem, same solution. See memory entry "Symphony Lessons (tlaplus-ts runs)".
- **PR #1794 (KG-router fallback)**: layered defence pattern: upstream filter + safety-floor + bypass-flag. Re-use this pattern for planning-tier fallback.
- **`.docs/design-adf-stability-roadmap-2026-05-01.md`**: existing roadmap; this work is its tactical Q2-end materialisation.
- **`.docs/design-adf-fallback-quota-v2.md`**: quota handling we ship as PR #1794 -- extends here for codex/pi.

### Technical Spikes Needed
| Spike | Purpose | Effort |
|---|---|---|
| `opencode run --model opencode/gpt-5.5 "ping"` on bigbox | Verify model is actually callable (not just listed); measure latency | 15 min |
| Investigate Z.AI probe timeout | Determine whether endpoint/auth/network; fix the probe before designing around its absence | 1 h |
| `pi-rust -p "ping"` on bigbox + `--continue` over 3 invocations | Verify build + session stability | 1 h |
| `pi` Gitea tool spike via `bash` tool | Confirm `pi` can run `gtr` via shell call | 30 min |
| KG tier score additions in `terraphim_kg_agents` (new bucket=65) | Confirm no regression on planning/impl/review scoring | 2 h |

Total spike time: ~5 h, parallelisable across one day.

## Recommendations

### Proceed/No-Proceed
**Proceed.** Risk is bounded (no architectural change), value is bounded (close the 37 %-success gap), and the design naturally consumes #1804, #1805, and the operational guardrails the user already flagged.

### Scope Recommendations
- **In scope (this work)**:
  1. Rust `merge-coordinator` per spec (#1805) + auto-close-on-merge
  2. Debug redaction (#1804)
  3. Add planning tier: `opencode/gpt-5.5` (with `opencode/gpt-5.4` then `opencode/gpt-5.3-codex` fallback)
  4. Add decision tier: pi-rust (session-persistent, local) -- for nightwatch/log-analyst/retrospective agents
  4a. **Expand implementation tier** with `minimax-coding-plan/MiniMax-M2.7-highspeed` as a primary option alongside `kimi-for-coding/k2p6`. Higher throughput for fast-turn coding agents (build-runner, pr-reviewer, security-sentinel). Fallback chain: `MiniMax-M2.7-highspeed` -> `kimi-for-coding/k2p6` -> `minimax-coding-plan/MiniMax-M2.5`.
  4b. **Cross-CLI fallback via pi-rust**: because pi-rust supports the same coding-plan providers (`zai-coding-plan`, `minimax-coding-plan`, `kimi-for-coding`, `openai-codex`), wire it as an alternative spawner so a failing opencode binary triggers a re-spawn through pi-rust with the same model id (not a tier downgrade). Adds defence-in-depth without changing the routing tier.
  4c. Investigate + fix Z.AI probe (not exclude it; `zai-coding-plan` is supported in both opencode and pi-rust)
  5. Operational guardrails: circuit-breaker on N config errors, memory watchdog, per-step turn budget pre-empt, bigbox `main` sync runbook
  6. Close #1797 with post-fix evidence
- **Out of scope (separate work)**:
  - MetaCoordinator layer above agents
  - New marketing artefact for ADF (North Star Current Focus, but separate brief)
  - Multi-region orchestrator
  - GPU/CRDT KG router

### Risk Mitigation Recommendations
- Run the five spikes *before* writing design Phase 2.
- Keep all conf.d edits behind a shadow file (`conf.d/terraphim.toml.new` + atomic rename) so a bad TOML never kills live.
- Add a "soft mode" feature flag for the new tiers (default off) so the design can ship merged but disabled, then flip on after manual verification.
- Restrict the auto-close-on-merge to PR bodies that match `Fixes #N` (case-insensitive) -- Gitea-native, false-positive-resistant.

## Next Steps

If approved:
1. Run the five spikes (5 h, half a day on Mac/bigbox)
2. Produce `.docs/design-adf-self-healing-2026-05-23.md` (Phase 2)
3. Open new Gitea issue "ADF self-healing -- planning + decision tiers" depending on #1804, #1805
4. Implementation order (per Phase 2): redact debug -> spawner extension -> KG tier bucket -> merge-coordinator Rust rewrite (with auto-close) -> circuit-breaker -> memory watchdog -> deploy to bigbox -> live verification.

## Appendix

### Reference Materials
- North Star `/Users/alex/cto-executive-system/north-star.md`
- Handover `/Users/alex/projects/terraphim/terraphim-ai/.docs/handover-2026-05-22-kg-router-fix.md`
- Stability roadmap `.docs/design-adf-stability-roadmap-2026-05-01.md`
- pi-rust (pi_agent_rust) skill `/Users/alex/.claude/skills/pi-agent-rust/SKILL.md`
- Codex CLI: `/Users/alex/.bun/bin/codex --help`
- Spec `.docs/spec-merge-coordinator.md` (referenced in #1805)
- Bigbox config `/opt/ai-dark-factory/orchestrator.toml`, `conf.d/terraphim.toml`
- Existing fix PR #1794 (validated KG-router fallback pattern)

### Code Locations
| Component | Path |
|---|---|
| Orchestrator main | `crates/terraphim_orchestrator/src/lib.rs` |
| `AgentDefinition::bypass_kg_routing` | `crates/terraphim_orchestrator/src/config.rs:780` |
| Spawn short-circuit | `crates/terraphim_orchestrator/src/lib.rs:1977` |
| KG tier scoring | `crates/terraphim_kg_agents/src/lib.rs` |
| Spawner config | `crates/terraphim_spawner/src/lib.rs` |
| Merge-coordinator (legacy) | `scripts/merge-coordinator.py` + `scripts/merge-coordinator-gate.sh` |
| Merge-coordinator (new home) | `crates/terraphim_merge_coordinator/` (TBC) |

### Open Question Log (numbered for design import)
- Q1: ~~GPT-5.5 GA?~~ **RESOLVED** -- `opencode/gpt-5.5` available on bigbox.
- Q2: ~~pi tool surface?~~ **RESOLVED** -- 8 built-ins + `bash` calling `gtr` is fine.
- Q3: KG bucket vs per-agent flag?
- Q4: Why does Z.AI probe time out? (network/auth/endpoint) -- spike before excluding
- Q5: `bypass_kg_routing` reuse vs new flag for pi fallback?

---

**Phase 1 gate criteria checklist**

- [x] Problem statement, current state, constraints, dependencies, risks, vital few -- all sections filled
- [x] Essentialism check 3/3
- [x] Multiple interpretations documented (2 forks above)
- [x] Assumptions explicitly listed with `Verified?` column
- [x] Vital few capped at 3
- [x] Eliminated items listed (6)
- [x] Spikes time-boxed (5 h total)
- [ ] **Human approval** (you)
