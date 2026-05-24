# Research Document: pi-rust Spawner Integration

**Status**: Draft
**Author**: opencode (GLM-5.1)
**Date**: 2026-05-24

## Executive Summary

The KG router already routes some providers through pi-rust via `action::` templates in taxonomy markdown files. However, the `terraphim_spawner` crate has no awareness of pi-rust as a CLI tool: `infer_args()` returns empty for pi-rust, so no `-p` (non-interactive) flag is passed. This means whenever the KG router selects a pi-rust route, the spawned process starts in interactive mode and either hangs or exits immediately with code 1. The fix requires adding pi-rust as a recognised CLI tool in the spawner, plus updating the orchestrator's model-composition and telemetry-parsing logic to handle pi-rust output.

## Essential Questions Check

| Question | Answer | Evidence |
|----------|--------|----------|
| Energising? | Yes | pi-rust exits 0 on success, fixing the endemic `exit_class=unknown` problem that poisons all implementation-swarm metrics |
| Leverages strengths? | Yes | pi-rust is a Rust binary built in this workspace; we control both sides |
| Meets real need? | Yes | Every opencode and claude agent run today exits code 1, making all exit classification useless |

**Proceed**: Yes (3/3)

## Problem Statement

### Description

All agent runs through opencode and claude exit with code 1 regardless of outcome (success, failure, timeout). The `ExitClassifier` in `agent_run_record.rs` classifies these as `exit_class=unknown` because exit code 1 is non-zero but no error pattern matches in the output. This makes all agent health metrics, circuit breaker decisions, and learning store signals unreliable.

pi-rust (v0.1.16 at `/home/alex/.local/bin/pi-rust`) exits 0 on success, supports `--max-tool-iterations` up to 1000, supports `--provider` and `--model` flags, and supports `--skill` loading. The KG router already routes Z.AI and MiniMax through pi-rust `action::` templates. But the spawner cannot properly invoke pi-rust because it lacks the CLI-specific arg inference.

### Impact

- Every agent run (implementation-swarm, build-runner, pr-reviewer) is classified `exit_class=unknown`
- Circuit breakers cannot distinguish real failures from successes
- Learning store receives no useful exit signals
- Provider health tracking is unreliable

### Success Criteria

1. pi-rust-routed agents exit 0 on success
2. `exit_class=unknown` drops to near zero for pi-rust-invoked runs
3. Telemetry correctly parses pi-rust output for token counting
4. Skill chains can be passed to pi-rust via `--skill` flags

## Current State Analysis

### Evidence from Production Logs (2026-05-24)

All 7 implementation-swarm runs today exited code 1:

| Time (UTC) | Wall Time | CLI | Model | Exit Class |
|------------|-----------|-----|-------|------------|
| 00:16 | 929s | opencode | kimi/k2p6 | unknown |
| 04:23 | 911s | claude | sonnet | unknown |
| 09:09 | 509s | opencode | kimi/k2p6 | unknown |
| 10:00 | 25s | claude | sonnet | unknown |
| 15:16 | 926s | opencode | kimi/k2p6 | timeout |
| 17:04 | 3840s | claude | sonnet | unknown |
| 17:05 | 71s | claude | sonnet | unknown |

### Existing pi-rust Integration Points

pi-rust is ALREADY partially integrated at 3 layers:

1. **KG Router taxonomy** (`docs/taxonomy/routing_scenarios/adf/*.md`):
   - `decision_tier.md`: pi-rust routes for openai-codex and zai-coding-plan
   - `implementation_tier.md`: pi-rust routes for minimax-coding-plan and zai-coding-plan
   - `planning_tier.md`: pi-rust routes for openai-codex and zai-coding-plan
   - `review_tier.md`: pi-rust route for zai-coding-plan

2. **RouteDirective** (`terraphim_types`):
   - `cli_basename()` extracts CLI from `action::` template
   - `route_key()` generates `(cli, provider, model)` probe key
   - Tests for pi-rust basename extraction exist

3. **Provider Probe** (`provider_probe.rs`):
   - pi-rust plaintext output recognised as token-bearing
   - Independent health tracking per (cli, provider, model) triple

### Code Locations

| Component | Location | Purpose |
|-----------|----------|---------|
| Spawner config | `crates/terraphim_spawner/src/config.rs` | `infer_args()`, `model_args()`, `infer_supports_stdin()`, `infer_api_keys()` |
| Spawner process | `crates/terraphim_spawner/src/lib.rs:650` | `spawn_process()` - builds Command, applies args |
| Orchestrator CLI override | `crates/terraphim_orchestrator/src/lib.rs:2047-2052` | Extracts CLI from `action::` template |
| Orchestrator model composition | `crates/terraphim_orchestrator/src/lib.rs:2093-2104` | Composes `provider/model` for opencode only |
| Orchestrator supports_model_flag | `crates/terraphim_orchestrator/src/lib.rs:1937` | `matches!(cli_name, "claude" \| "claude-code" \| "opencode")` |
| Telemetry parsing | `crates/terraphim_orchestrator/src/lib.rs:7672` | `parse_stdout_for_telemetry` - opencode/claude only |
| Exit classification | `crates/terraphim_orchestrator/src/agent_run_record.rs:250-378` | `EXIT_CLASS_PATTERNS` |
| KG Router | `crates/terraphim_orchestrator/src/kg_router.rs` | Loads `action::` templates from markdown |
| RouteDirective type | `crates/terraphim_types/src/lib.rs:482` | `cli_basename()`, `route_key()` |
| Taxonomy files | `docs/taxonomy/routing_scenarios/adf/*.md` | Route definitions with pi-rust actions |

### Data Flow (Current, Broken)

```
1. Scheduler fires cron for implementation-swarm
2. KG router matches task → implementation_tier concept
3. first_healthy_route() picks provider (e.g., zai-coding-plan)
4. action template: "/home/alex/.local/bin/pi-rust --provider zai-coding-plan --model {{ model }} -p {{ prompt }}"
5. Orchestrator extracts CLI basename: "pi-rust"
6. effective_cli = "/home/alex/.local/bin/pi-rust" (overrides def.cli_tool)
7. Spawner receives Provider with cli_command="/home/alex/.local/bin/pi-rust"
8. Spawner calls infer_args("pi-rust") → returns [] (NO MATCH)
9. Command: pi-rust <task_string>  (missing -p, --provider, --model)
10. pi-rust starts INTERACTIVE mode → hangs or exits 1
```

### The Gap

The spawner's `infer_args()` knows about `codex`, `claude`, `opencode`, `bash` but NOT `pi-rust`. The `model_args()` knows about `codex`, `claude`, `opencode` but NOT `pi-rust`. When the orchestrator routes through a pi-rust action template, the spawner receives a raw pi-rust binary path but cannot generate proper non-interactive arguments.

## Constraints

### Technical Constraints

- **C1**: pi-rust must use subscription-only providers (kimi-for-coding, minimax-coding-plan, zai-coding-plan, anthropic)
- pi-rust `--provider` flag requires the canonical provider name (e.g., `zai-coding-plan`, not `pi-rust-zai`)
- pi-rust `-p` is required for non-interactive mode (default is interactive TUI)
- pi-rust `--max-tool-iterations` default is 50, max 1000
- pi-rust `--skill` takes a file path (can be specified multiple times)
- pi-rust does NOT hang on stdin like opencode (supports stdin for large tasks)
- Model names: pi-rust uses canonical names like `kimi-for-coding/k2p6`, `glm-5.1`, `MiniMax-M2.5-highspeed`

### Non-Functional Requirements

| Requirement | Target | Current |
|-------------|--------|---------|
| exit_class accuracy | >90% classified | ~0% (all unknown) |
| pi-rust spawn reliability | exit 0 on success | exits 1 (broken args) |

## Vital Few (Essentialism)

### Essential Constraints (Max 3)

| Constraint | Why It's Vital | Evidence |
|------------|----------------|----------|
| Spawner must pass `-p` to pi-rust | Without it, pi-rust starts interactive mode and hangs/exit-1 | `pi-rust --help` confirms `-p` is non-interactive mode |
| Spawner must pass `--provider` and `--model` | pi-rust needs explicit provider selection; cannot infer from model string alone | `pi-rust --list-providers` shows provider names are distinct from model names |
| Orchestrator must parse pi-rust output for telemetry | Without parsing, cost tracking and token counting are blind | `parse_stdout_for_telemetry` only handles opencode/claude today |

### Eliminated from Scope

| Eliminated Item | Why Eliminated |
|-----------------|----------------|
| Skill chain → `--skill` flag wiring | Requires orchestrator changes to pass skill file paths; defer to follow-up |
| `--max-tool-iterations` in spawner args | Default 50 is sufficient for now; configurable later via env var `PI_MAX_TOOL_ITERATIONS` |
| pi-rust JSON output format | pi-rust streams plaintext in `-p` mode; telemetry can parse plain text |
| Changing implementation-swarm primary cli_tool to pi-rust | KG routing already handles this via action templates; no TOML change needed |
| Adding pi-rust patterns to EXIT_CLASS_PATTERNS | Not needed if pi-rust exits 0 on success (handled by exit_code==0 path) |

## Dependencies

### Internal Dependencies

| Dependency | Impact | Risk |
|------------|--------|------|
| terraphim_spawner | Must add pi-rust CLI recognition | Low - additive change |
| terraphim_orchestrator lib.rs | Must add pi-rust to supports_model_flag, model composition, telemetry | Low - additive match arms |
| terraphim_types | No changes needed (RouteDirective already handles pi-rust) | None |

### External Dependencies

| Dependency | Version | Risk | Alternative |
|------------|---------|------|-------------|
| pi-rust binary | 0.1.16 | Low - already installed at /home/alex/.local/bin/pi-rust | opencode (broken for some providers) |

## Risks and Unknowns

### Known Risks

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| pi-rust `-p` output format differs from opencode JSON | Medium | Medium | pi-rust streams plaintext; telemetry parser already handles non-JSON CLIs |
| pi-rust `--provider` name mismatch with KG route provider | Low | High | Use canonical names already in taxonomy files |
| pi-rust hangs on large task stdin | Low | Medium | pi-rust supports stdin (unlike opencode); infer_supports_stdin returns true |

### Assumptions Explicitly Stated

| Assumption | Basis | Risk if Wrong | Verified? |
|------------|-------|---------------|-----------|
| pi-rust exits 0 on success in `-p` mode | Tested manually: `pi-rust -p "echo hello"` → exit 0 | If exits non-zero, same problem as opencode | Yes |
| pi-rust accepts `--provider <name>` for all subscription providers | `--list-providers` shows kimi-for-coding, minimax-coding-plan, zai-coding-plan | Provider name mismatch would break routing | Yes |
| pi-rust `--model <model>` works with canonical model names | `--list-models` shows full model IDs | Model name format mismatch | Yes |
| The KG router's `action::` template is NOT used for arg construction | The spawner builds its own args via `infer_args()`; the `action::` template is only used for CLI path extraction and probing | If `action::` args were used, spawner args would be redundant | Yes (verified in code) |

### Multiple Interpretations Considered

| Interpretation | Implications | Why Chosen/Rejected |
|----------------|--------------|---------------------|
| Add pi-rust args to spawner `infer_args()` (like opencode/claude) | Minimal change; spawner handles all CLI-specific args | **Chosen**: matches existing pattern; smallest diff |
| Use the `action::` template directly as the spawn command | Bypasses spawner arg inference entirely; template already has correct flags | **Rejected**: requires spawner refactor; template is for probing, not spawning; task string injection is complex |
| Make pi-rust primary cli_tool in TOML | No code changes needed in spawner; just config | **Rejected**: loses the benefit of KG routing selecting best CLI per provider; also still needs spawner recognition for when KG routes to pi-rust |

## Research Findings

### Key Insights

1. **pi-rust is already routed to by the KG router** for Z.AI and MiniMax, but the spawner cannot invoke it properly (missing `-p` flag)
2. **All opencode and claude agents exit 1**, making exit classification useless for the entire fleet
3. **pi-rust exits 0 on success**, which would immediately fix the exit classification problem
4. **The spawner pattern is well-established**: adding a new CLI tool is a known, additive change (4 functions to extend)
5. **The orchestrator has 3 places that hard-code CLI tool names**: `supports_model_flag`, model composition, and telemetry parsing

### Relevant Prior Art

- Commit `16678f34`: Added opencode stdin handling to spawner
- Commit `cc15f19b`: Added bash/sh support to spawner (`-c` flag)
- Commit `11e5cbbe`: Added pi-rust routes to KG taxonomy
- Commit `1813416c`: Swapped Z.AI from opencode to pi-rust

## Recommendations

### Proceed: Yes

The change is purely additive, follows established patterns, and addresses a fleet-wide observability gap.

### Scope

Minimum viable: extend spawner config for pi-rust args + update orchestrator CLI name matches.

### Risk Mitigation

1. Add pi-rust unit tests to spawner config (same pattern as opencode/claude tests)
2. Add pi-rust to orchestrator `supports_model_flag` match
3. Add pi-rust plaintext handling to telemetry parser

## Next Steps

1. Proceed to Phase 2 (Design) for detailed implementation plan
2. No external dependencies or approvals needed
