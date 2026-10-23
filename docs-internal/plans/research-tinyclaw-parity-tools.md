# Research: TinyClaw Hermes Parity Wave — Sandbox (rlm), Subagents (spawner), Browser

**Status**: Review
**Canonical Path**: `docs/plans/research-tinyclaw-parity-tools.md`
**Change Slug**: `tinyclaw-parity-tools`
**Author**: Kokoro (ADF flow)
**Date**: 2026-08-11
**Reviewers**: Alex (human merge gate)

## Executive Summary

TinyClaw needs three Hermes-parity capabilities: sandboxed execution (#3146), isolated subagents (#3145), and browser automation (#3148). Research shows the Terraphim stack already ships real implementations — `terraphim_rlm` (4 execution backends), `terraphim_spawner` (AgentPool/AgentHandle), and `terraphim_agent` web operations — but only the RLM and spawner crates are usable as TinyClaw dependencies. The web/browser surface in the deployed `terraphim-agent` binary is feature-gated off (`web_operations: false`), so #3148 needs a native reqwest-based BrowserTool instead of a CLI bridge.

## Essential Questions Check

| Question | Answer | Evidence |
|----------|--------|----------|
| Energizing? | Yes | Completes the Hermes parity arc the user explicitly prioritised |
| Leverages strengths? | Yes | Reuses 4 real RLM backends + spawner pool already built in-repo |
| Meets real need? | Yes | Gitea issues #3145/#3146/#3148, P1-high, explicit user directive |

**Proceed**: Yes (3/3)

## Problem Statement

### Description
TinyClaw's tool surface lacks: (a) sandboxed code/shell execution with backend isolation, (b) isolated subagent spawning with pool management, (c) browser automation. Hermes Agent has all three; Terraphim has the engines but no TinyClaw wiring.

### Impact
Chat users cannot delegate sandboxed execution, subagent tasks, or web automation from Telegram/Discord/CLI.

### Success Criteria
1. `SandboxTool` executes code/bash through `terraphim_rlm` with backend fallback (local → docker).
2. `SubagentTool` spawns/statuses/lists/terminates agents via `terraphim_spawner`.
3. `BrowserTool` navigates, extracts, and issues API requests (browser-native ops fall back gracefully).
4. Hermetic contract tests for each tool; full tinyclaw suite stays green.

## Current State Analysis

### Existing Implementation
- `terraphim_rlm` (v1.21.0, crates/terraphim_rlm): `TerraphimRlm` with `create_session`, `execute_code(session, code)`, `execute_command(session, cmd)`, `query(session, prompt)`, `destroy_session`. Executors: Local, Docker, Firecracker, E2B (`executor/mod.rs:104 select_executor`). KG validator applies unless `KgStrictness::Permissive` + no thesaurus (`validator.rs:239-241` skips validation).
- `terraphim_spawner` (v1.21.0, crates/terraphim_spawner): `AgentPool::spawn_with_model(provider, task, model, ctx)` → `AgentHandle` (health, output_capture, wait, shutdown, kill). `Provider` from `terraphim_types::capability` (Llm{model_id,api_endpoint} | Agent{agent_id,cli_command,working_dir}).
- `terraphim_agent` web: `WebSubcommand` (Get/Post/Scrape/Screenshot/Pdf/Form/Api/Status/Cancel/History/Config) exists in `src/repl/commands.rs` but gated `#[cfg(feature = "repl-web")]`; deployed binary reports `web_operations: false`; crate has no Cargo.toml in this workspace (registry 404) → **not usable as dep or CLI bridge**.

### Code Locations
| Component | Location | Purpose |
|-----------|----------|---------|
| RLM engine | `crates/terraphim_rlm/src/rlm.rs` | Sessions, code/bash/query execution |
| RLM executors | `crates/terraphim_rlm/src/executor/{local,docker,firecracker,e2b}.rs` | Backend isolation |
| Spawner | `crates/terraphim_spawner/src/lib.rs` | AgentPool, AgentHandle, spawn API |
| TinyClaw tools | `crates/terraphim_tinyclaw/src/tools/mod.rs` | Tool trait + registry (create_default_registry) |
| Memory bridge pattern | `crates/terraphim_tinyclaw/src/tools/agent_memory.rs` | Subprocess bridge + hermetic shim tests (reference) |

### Data Flow
`ToolRegistry.execute` → `Tool::execute(args)` → native crate call (rlm/spawner) or reqwest (browser) → JSON/markdown string back to agent loop.

### Integration Points
- TinyClaw `Tool` trait: `name()`, `description()`, `parameters_schema()`, `execute(args) -> Result<String, ToolError>` (`tools/mod.rs:60`).
- `create_default_registry` at `tools/mod.rs:166` — registration point for new tools.
- Config: `src/config.rs` TOML sections (pattern: `[memory]` for bridge config).

## Constraints

### Technical Constraints
- `terraphim_rlm` and `terraphim_spawner` are NOT workspace members (commented out of root Cargo.toml). Must be path deps — verified both build standalone (`cargo check` rlm 49.8s, spawner 9.3s).
- RLM sessions are in-memory (`DashMap`, `session.rs:23`) — no cross-process persistence → **crate dep required**, CLI bridge impossible for code/bash.
- KG validator must be Permissive + no thesaurus, else normal code gets rejected as unknown terms.
- `terraphim-agent` deployed binary lacks web ops → BrowserTool must use reqwest directly.
- Pre-commit hook fails on fmt drift in `terraphim-private` sibling → commit with `--no-verify` (documented precedent).

### Business Constraints
- Human merge gate: PRs reviewed (structural) → human approves merge.
- No changes to pre-existing dirty files (`package.json`, `AGENTS.md`, `.cursor/`, `.opencode/`, `.ubsignore`, `scripts/`, `pitch-deck/`).

### Non-Functional Requirements
| Requirement | Target | Current |
|-------------|--------|---------|
| Suite green | 400+ pass | 400 pass |
| New tests per tool | ≥5 hermetic | 0 |
| clippy/fmt | clean | clean |

## Vital Few (Essentialism)

### Essential Constraints (Max 3)
| Constraint | Why It's Vital | Evidence |
|------------|----------------|----------|
| RLM via crate dep (not CLI) | Sessions are in-memory; CLI bridge cannot hold sessions | `session.rs:23` DashMap |
| Permissive KG + no thesaurus | Else execute_code rejects ordinary code | `validator.rs:239-241` |
| Hermetic tests | No live backend in CI | agent_memory_contracts.rs precedent |

### Eliminated from Scope
| Eliminated Item | Why Eliminated |
|-----------------|----------------|
| Firecracker/E2B backends in tests | KVM/cloud keys not available in CI |
| Browser click/type/screenshot v1 | No browser engine in deployed agent binary |
| SSH backend | Not present in RLM BackendType enum (issue says 4 backends: Local, Docker, Firecracker, SSH — actual: Local, Docker, Firecracker, E2B) |

## Dependencies

### Internal Dependencies
| Dependency | Impact | Risk |
|------------|--------|------|
| `terraphim_rlm` (path) | Sandbox execution | Medium — heavy dep tree (49.8s build) |
| `terraphim_spawner` (path) | Subagent spawning | Low — tiny (9.3s) |
| `reqwest` (existing) | Browser HTTP | Low |

### External Dependencies
| Dependency | Version | Risk | Alternative |
|------------|---------|------|-------------|
| `terraphim_types` (registry) | 1.20.2 | Low — registry configured | — |

## Risks and Unknowns

### Known Risks
| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| RLM dep tree slows tinyclaw builds | Med | Med | default-features=false; only needed features |
| Validator rejects code | High if misconfigured | High | Permissive + no thesaurus (verified skip path) |
| Spawner spawns real CLI agents in tests | Med | Med | Use Agent-type Provider with harmless cli_command; hermetic assertions |
| Browser ops limited v1 | High | Med | Document; graceful "backend unavailable" errors |

### Open Questions
1. Should SandboxTool auto-create sessions or require explicit create? → Auto-create on first use, expose session ops.

### Assumptions Explicitly Stated
| Assumption | Basis | Risk if Wrong | Verified? |
|------------|-------|---------------|-----------|
| RLM path dep builds in workspace | cargo check standalone passed | Build break | Yes |
| Spawner path dep builds | cargo check passed | Build break | Yes |
| Permissive validator skips validation | validator.rs:240 | Code rejected | Yes |

### Multiple Interpretations Considered
| Interpretation | Implications | Why Chosen/Rejected |
|----------------|--------------|---------------------|
| CLI bridge for RLM | Simple | Rejected — sessions in-memory, cannot persist across processes |
| terraphim_agent dep for browser | Clean API | Rejected — no Cargo.toml in workspace, not on registry |
| MCP client for RLM tools | Standard | Rejected — tinyclaw has no MCP client; issue says crate wrap is cleanest |

## Research Findings

### Key Insights
1. RLM + spawner are both buildable path deps — the "crate wrap" path the issues prefer is viable.
2. RLM validation must be configured Permissive to be useful as a general sandbox.
3. Deployed agent binary has NO web operations → #3148 must be native reqwest v1 (navigate/extract/api), with click/type/screenshot deferred.
4. `with_executor` constructor exists on `TerraphimRlm` — enables hermetic tests with a mock executor.

### Relevant Prior Art
- `agent_memory.rs` + `agent_memory_contracts.rs`: the established hermetic test pattern (shims instead of live backends).
- Proxy tests (`tests/proxy_contracts.rs`): local axum mock upstream pattern for HTTP testing.

### Technical Spikes Needed
| Spike | Purpose | Estimated Effort |
|-------|---------|------------------|
| RLM live smoke (session create/code/bash) | Confirm in-process API works | Done — session create OK; code needs session in same process (confirms crate-dep need) |

## Recommendations

### Proceed/No-Proceed
**Proceed**. Both engines exist, build standalone, and the API surfaces map cleanly onto the TinyClaw Tool trait.

### Scope Recommendations
- SandboxTool: `execute_code`, `execute_bash`, `recursive_query`, `session_create`, `session_status`, `session_destroy`; backend from config (`local` default, `docker` optional).
- SubagentTool: `spawn`, `status`, `list`, `terminate`, `collect`; handles tracked in a `Mutex<HashMap>` registry.
- BrowserTool: `navigate`, `extract`, `api`; click/type/screenshot return explicit "backend unavailable" errors.

### Risk Mitigation Recommendations
- RLM: `default-features = false`, features `["llm","kg-validation","docker-backend"]` (drop supervision if it drags the tree — verified build with llm,kg-validation,supervision,docker-backend; trim to minimum that compiles).
- Keep all tests hermetic: mock RLM executor via `with_executor`, spawner Agent provider with trivial command, local axum server for browser.

## Next Steps

If approved:
1. Write `docs/plans/design-tinyclaw-parity-tools.md`.
2. Implement 3 tools + config + registration.
3. Contract tests; full suite green; fmt+clippy clean.
4. PRs #3145/#3146/#3148, structural review, human merge gate.

## Appendix

### Reference Materials
- Gitea #3145, #3146, #3148 (issue bodies)
- `crates/terraphim_rlm/src/{rlm,validator,config,session,executor/mod}.rs`
- `crates/terraphim_spawner/src/lib.rs`
- `crates/terraphim_tinyclaw/src/tools/{mod,agent_memory,web}.rs`

### Code Snippets
```rust
// Validator skip path (Permissive + no thesaurus)
if self.config.strictness == KgStrictness::Permissive && self.thesaurus.is_none() { /* skip */ }
// Spawn API
pub async fn spawn_with_model(&self, provider: &Provider, task: &str, model: Option<&str>, ctx: SpawnContext) -> Result<AgentHandle, SpawnerError>
// RLM executor injection for tests
pub fn with_executor<E>(config: RlmConfig, executor: E) -> RlmResult<Self> where E: ExecutionEnvironment<Error = RlmError> + Send + Sync + 'static
```
