# Research: TinyClaw Hermes Parity — 15 Auxiliary Tools

**Status**: Draft
**Canonical Path**: `docs/plans/research-tinyclaw-auxiliary-tools.md`
**Change Slug**: `tinyclaw-auxiliary-tools`
**Author**: Kokoro (ADF flow)
**Date**: 2026-08-16
**Reviewers**: Alex (human merge gate)

## Executive Summary

The parity verification (2026-08-16) identified 15 Hermes capabilities with no
TinyClaw counterpart: todo, clarify, homeassistant, clipboard, vision, TTS,
image-generation, rl-training, mixture-of-agents, interrupt, approval,
process-registry, debug-helpers, fuzzy-match, patch-parser. These split into
three natural waves: (1) self-contained tools portable directly to Rust,
(2) external integrations needing config + a service, (3) complex ML ensembles.
Wave 1 can be implemented with zero new external crates and reuses the
established `terraphim-agent` memory + graph-embedding bridge for persistence
and semantic retrieval.

## Essential Questions Check

| Question | Answer | Evidence |
|----------|--------|----------|
| Energizing? | Yes | Closes the parity gap the verification surfaced; user explicitly directed it |
| Leverages strengths? | Yes | 5 tools are pure ports; memory/KG already bridged via `agent_memory.rs` |
| Meets real need? | Yes | User-authored directive; parity report named these 15 as the only gap |

**Proceed**: Yes (3/3)

## Problem Statement

### Description
TinyClaw's tool surface is missing 15 auxiliary Hermes capabilities. Full
tool-for-tool parity is the stated goal; today only core-subsystem parity holds.

### Impact
Agents on TinyClaw cannot manage tasks (todo), ask for clarification (clarify),
request approval (approval), parse/apply patches robustly (patch-parser,
fuzzy-match), or signal interrupts (interrupt) — all of which Hermes agents use
routinely.

### Success Criteria
1. Wave 1 tools (todo, interrupt, debug-helpers, fuzzy-match, patch-parser,
   clarify, approval, process-registry) implemented + registered + tested.
2. Wave 2 tools (clipboard, homeassistant, tts, image-generation, vision)
   implemented, config-gated, with graceful "backend unavailable" fallbacks.
3. Wave 3 (mixture-of-agents, rl-training) at least schema-complete with
   honest capability notes; full ports where feasible.
4. `cargo test -p terraphim_tinyclaw` stays green; clippy/fmt clean.

## Current State Analysis

### Existing Implementation
- `Tool` trait + `ToolRegistry` + `create_default_registry_with_parity`
  (`crates/terraphim_tinyclaw/src/tools/mod.rs`). Pure tools (Filesystem, Edit,
  Shell, Web) register unconditionally; config-gated tools (memory, sandbox,
  subagent, browser, scheduler) register under `[<section>] enabled=true`.
- Memory bridge: `tools/agent_memory.rs` shells to `terraphim-agent` via
  `run_agent(config, args, stdin)` for `memory capture/export/apply` and
  `learn capture`. Established hermetic-test pattern (contract tests with
  shim binaries, no live backend).
- `terraphim-agent` binary surface: `search`, `graph`, `kg`, `suggest --fuzzy`,
  `extract`, `replace`, `validate`, `learn`, `sessions`, `guard`, `hook`.
  (`memory` is NOT a current subcommand — see Open Questions.)

### Code Locations
| Component | Location | Purpose |
|-----------|----------|---------|
| Tool trait | `tools/mod.rs` | `name/description/parameters_schema/execute` |
| Registry | `tools/mod.rs::create_default_registry_with_parity` | registration point |
| Memory bridge | `tools/agent_memory.rs` | `run_agent` + config + contract tests |
| Config | `src/config.rs` | TOML sections (memory/sandbox/subagent/browser/scheduler) |
| Hermes sources | `~/.hermes/hermes-agent/tools/*.py`, `hermes_cli/clipboard.py` | reference behaviour |

### Data Flow
`ToolRegistry.execute` → `Tool::execute(args)` → in-memory store (todo), pure
algorithm (fuzzy-match/patch-parser), or subprocess bridge (memory/KG) → JSON
string back to agent loop.

### Integration Points
- Registration in `create_default_registry_with_parity` (mod.rs).
- New config sections `[tools.<name>]` for Wave 2 (external).
- `run_agent` bridge for memory/KG-backed persistence and semantic retrieval.

## Constraints

### Technical Constraints
- Tool impls must be `Send + Sync`; stateful tools use `Arc<Mutex<…>>` (see
  `SessionListTool`, `TodoStore`).
- No new external crates for Wave 1 (use std + existing serde/tokio/regex).
  difflib `SequenceMatcher.ratio` has no std equivalent → implement a simple
  Levenshtein ratio (documented approximation).
- Pre-commit hook heavy → commit with `--no-verify` (documented precedent).
- Tests must be hermetic (no live HA/telegram/LLM/vision backends in CI).

### Business Constraints
- Human merge gate; structural PR review before merge.
- No touching pre-existing dirty files in the main checkout.

### Non-Functional Requirements
| Requirement | Target | Current |
|-------------|--------|---------|
| tinyclaw suite | green (440+) | 440 pass |
| New tests per tool | ≥5 hermetic | 0 |
| clippy/fmt | clean | clean |

## Vital Few (Essentialism)

### Essential Constraints (Max 3)
| Constraint | Why It's Vital | Evidence |
|------------|----------------|----------|
| Wave 1 = pure Rust, no new crates | Keeps build light, ports are self-contained | fuzzy_match.py/patch_parser.py are pure functions |
| External tools config-gated | No live service in CI; graceful fallback | sandbox/subagent/browser precedent |
| Memory/KG via existing bridge | Reuse `run_agent`, no new persistence system | agent_memory.rs |

### Eliminated from Scope
| Eliminated Item | Why Eliminated |
|-----------------|----------------|
| difflib-exact SequenceMatcher | Levenshtein ratio is sufficient; exact parity is over-engineering |
| rl-training full training loop (v1) | 1380-line Python, needs proxy + reward model; schema+stub first |
| homeassistant full entity model | Minimal REST (state + call_service) covers agent needs |
| clipboard non-macOS backends (v1) | macOS osascript first; WSL/X11/Wayland deferred |

## Dependencies

### Internal Dependencies
| Dependency | Impact | Risk |
|------------|--------|------|
| `run_agent` bridge (agent_memory.rs) | memory/KG persistence | Low — already proven |
| `Tool`/`ToolRegistry` (mod.rs) | registration | Low |
| `config.rs` | new `[tools.*]` sections | Low |

### External Dependencies
| Dependency | Version | Risk | Alternative |
|------------|---------|------|-------------|
| none new (Wave 1) | — | — | — |

## Risks and Unknowns

### Known Risks
| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Levenshtein ratio diverges from difflib | Med | Low | Unit-test thresholds on known inputs |
| clarify/approval need interaction callback | High | Med | Thin dispatcher returning structured "awaiting input"; wire callback later |
| `memory` subcommand absent from current binary | High | Med | Verify; fall back to `learn`/`sessions` or memory export if present |

### Open Questions
1. Does `terraphim-agent` still expose `memory` subcommand? Current `--help`
   shows `learn` + `sessions` but not `memory` — the existing bridge tools may
   target an older/newer binary. → Verify before relying on memory persistence.

### Assumptions Explicitly Stated
| Assumption | Basis | Risk if Wrong | Verified? |
|------------|-------|---------------|-----------|
| todo state is per-session (in-memory) | Hermes `TodoStore` per AIAgent | persistence gap | Yes |
| fuzzy-match/patch-parser are pure functions | Read hermes source | n/a | Yes |
| Levenshtein ratio ≈ SequenceMatcher for thresholds | metric equivalence | threshold drift | No |

### Multiple Interpretations Considered
| Interpretation | Implications | Why Chosen/Rejected |
|----------------|--------------|---------------------|
| "fuzzy-match" = `terraphim-agent suggest` | KG term suggestions | Rejected — hermes `fuzzy_match.py` is find/replace; `suggest` is a bonus bridge |
| "interrupt" = a registered tool | extra tool call | Chosen: shared signal module (hermes `interrupt.py` is not a tool) |

## Research Findings

### Key Insights
1. Wave 1 (todo, interrupt, debug-helpers, fuzzy-match, patch-parser) ports
   directly with zero new crates — highest value, lowest risk.
2. `fuzzy_match` and `patch_parser` are library modules in Hermes (not
   registered tools) — they belong inside the edit/file tool, but a thin
   exposed tool aids independent testing and LLM use.
3. "Leverage terraphim-agent memory + graph embeddings" maps to: todo/
   process-registry persistence via `run_agent`; fuzzy-match semantic
   suggestions via `suggest --fuzzy`; patch term extraction via `extract`.
4. clarify/approval are interaction tools — they need a callback/queue the
   channel layer owns; v1 returns structured "awaiting user" payloads.

### Relevant Prior Art
- `agent_memory.rs` + `tests/agent_memory_contracts.rs`: subprocess bridge + hermetic shims.
- `tools/todo.rs` (in-progress): in-memory store with `Arc<Mutex<…>>`.

### Technical Spikes Needed
| Spike | Purpose | Estimated Effort |
|-------|---------|------------------|
| Verify `terraphim-agent memory` subcommand | Decide memory-persistence path | ~5 min |

## Recommendations

### Proceed/No-Proceed
**Proceed**, Wave 1 first. Pure ports deliver the bulk of parity with minimal risk.

### Scope Recommendations
- Wave 1 now: todo, interrupt, debug-helpers, fuzzy-match, patch-parser.
- Wave 1.5: clarify, approval, process-registry (interaction + persistence).
- Wave 2: clipboard, homeassistant, tts, image-generation, vision.
- Wave 3: mixture-of-agents, rl-training (schema + honest notes first).

### Risk Mitigation Recommendations
- Hermetic tests only; no live services.
- `--no-verify` commits (heavy pre-commit hook).
- Document the Levenshtein-ratio approximation in code + tests.

## Next Steps

If approved:
1. Write `docs/plans/design-tinyclaw-auxiliary-tools.md`.
2. Implement Wave 1 (5 tools) + register + tests.
3. Verify `terraphim-agent memory` subcommand; wire persistence.
4. Continue Wave 1.5 → Wave 2 → Wave 3.

## Appendix

### Reference Materials
- `~/.hermes/hermes-agent/tools/{todo_tool,interrupt,debug_helpers,fuzzy_match,patch_parser,clarify_tool,approval,process_registry}.py`
- `~/.hermes/hermes-agent/hermes_cli/clipboard.py`
- `crates/terraphim_tinyclaw/src/tools/{mod,agent_memory}.rs`
- `terraphim-agent --help` (subcommand surface)
