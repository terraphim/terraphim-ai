# Design: TinyClaw Parity Closure

**Date:** 2026-08-12
**Branch:** `task/3143-tinyclaw-parity-closure`
**Scope:** Gitea issues `#3143-#3148` and `#3160-#3167`
**Inputs:** `/tmp/tinyclaw-issues-20260812.json`, issue comments/audit evidence, `docs/plans/*tinyclaw*`, `.docs/adf/3144/*`

## Problem

Prior parity waves landed substantial TinyClaw capabilities, but the audit evidence shows several issue-body acceptance criteria were treated as deviations or partial work. This closure pass must make the original criteria provable without mutating Gitea or committing.

## Acceptance Trace

| Issue | Required closure evidence | Current gap | Implementation/test plan |
|---|---|---|---|
| `#3161` | Every integration test file scrubs env and CI rejects omissions | `slack_integration.rs` fails under all-features; no enforcement gate | Fix nested module import; add a repository script plus CI workflow step that scans `crates/terraphim_tinyclaw/tests/*.rs` for `mod common;` and `common::scrub_env()` |
| `#3144` | Memory capture -> fresh session retrieve/apply -> response flow, with natural-language wiring proof | Closed by `tests/agent_loop_dispatch_e2e.rs::fresh_session_memory_capture_retrieve_apply_response_flow_uat` | Hermetic `terraphim-agent` shim returns memory context; fresh `ToolCallingLoop` injects it into the system prompt; the response path uses the applied memory naturally ("sushi preference"). |
| `#3145` | Production-like subagent isolation, timeout/capacity/cleanup, agent-loop UAT | Partially closed | `tests/subagent_contracts.rs` now uses real `terraphim_spawner` processes with temp working-directory isolation, capacity enforcement, and terminate cleanup. Agent-loop dispatch is covered by `tests/agent_loop_dispatch_e2e.rs`. Full temp git worktree management is not implemented; temp working-directory isolation is the production equivalent currently wired. |
| `#3146` | Recursive query plus Local/Docker backend selection/fallback, timeout/cancellation/output-bound/isolation | Partially closed | `tests/sandbox_contracts.rs` now uses real `SandboxTool::from_config` and production `terraphim_rlm` local backend for execution, backend honor/fallback reporting, timeout, bounded output, and isolated sessions. `recursive_query` is exercised through RLM and returns bounded `success:false` when no real LLM bridge is configured; full recursive LLM success and cancellation remain blocked on a configured real LLM bridge. |
| `#3147` | Orchestrator-backed scheduling or approved design update; durable process restart and unattended fire tests | **BLOCKED** | The issue explicitly requires `terraphim_orchestrator`. This workspace has an excluded residual `crates/terraphim_orchestrator` and the TinyClaw code still documents a CronStore deviation. No approved design update was obtained in this pass, so this criterion is not complete. |
| `#3148` | Browser navigate/click/type/screenshot through `terraphim-agent web_operations`; hermetic browser-driver tests | **BLOCKED** for dependency criterion | `BrowserTool` has local navigate/extract/api/click/type/screenshot coverage, but it is a custom reqwest/local-HTML engine. `crates/terraphim_agent` has no `Cargo.toml` in this workspace and its web operations source is behind `#[cfg(feature = "repl-web")]`; therefore the explicit `terraphim-agent web_operations` dependency is not satisfied. |
| `#3163` | Opt-in pinned live-tier interoperability against reference MCP server: connect/list/call | Existing contract tests prove JSON shape only | Add ignored `LIVE_TINYCLAW_MCP_REFERENCE=1` test that starts/connects to a configured reference stdio command and proves list/call |
| `#3165` | First candidates WhatsApp/Teams with per-channel issues/live-tier | **BLOCKED** for WhatsApp/Teams scope | Existing GitHub/Linear work is useful but does not satisfy the stated WhatsApp/Teams candidate scope. Matrix/WhatsApp is disabled because `matrix-sdk` conflicts with the workspace sqlite stack, and no Teams adapter is present. |
| `#3166` | TUI, dashboard, proxy, ACP all drive shared `AgentLoop` | Closed by `tests/agent_loop_dispatch_e2e.rs::tui_dashboard_proxy_and_acp_dispatch_into_same_agent_loop_entry_bus` | Added `agent::entry::dispatch_to_agent_loop` and routed TUI, dashboard, proxy, and ACP through the same bus entry seam. |
| `#3143/#3160` | Single-conversation and messaging-channel E2E across memory, subagents, sandbox, schedules, browser; README examples/config | Partially closed | `tests/agent_loop_dispatch_e2e.rs` covers one single-conversation AgentLoop tool-dispatch E2E and one messaging-channel dispatch E2E. The combined test uses hermetic static tools for the multi-tool itinerary; individual production-path tests cover memory/subagent/sandbox/browser/schedule contracts. |
| `#3162/#3164/#3167` | Previously merged gates remain intact | No user-listed functional gaps | Preserve behavior; include in final all-features gates |

## Design Decisions

1. **Strict TDD slices.** Each gap gets a focused RED test first, then the smallest production change that makes it pass.
2. **Hermetic default.** New integration tests call `common::scrub_env()` and use local temp dirs, local HTTP servers, or local shims.
3. **Live tier is opt-in only.** External MCP/channel interoperability tests are `#[ignore]` and require explicit `LIVE_*` env flags plus endpoint/credential variables.
4. **No internal mocks.** Tests can use local binaries, temp files, local HTTP servers, and public trait implementations. They must not replace internal production code with fake logic.
5. **Scheduling dependency remains blocked.** CronStore durability is useful, but it does not satisfy the issue criterion that says `via terraphim_orchestrator`.
6. **Browser dependency remains blocked.** The local browser engine is useful, but it does not satisfy the issue criterion that says `via terraphim-agent web_operations`.

## Verification Plan

Focused RED/GREEN commands:

- `cargo test -p terraphim_tinyclaw --all-features --test slack_integration --no-fail-fast`
- `scripts/check-tinyclaw-test-hermeticity.sh`
- Focused tests per changed area: memory, subagent, sandbox, scheduler, browser, MCP, channel, surfaces

Final gates requested by the user:

- `cargo fmt --check`
- `cargo clippy -p terraphim_tinyclaw --all-targets --all-features -- -D warnings`
- `cargo test -p terraphim_tinyclaw --all-features --tests --no-fail-fast`
- `cargo build -p terraphim_tinyclaw --release --all-features`

## Non-Goals

- No Gitea comments, issue changes, commits, pushes, or remote mutation.
- No broad workspace refactors outside TinyClaw, test/CI, and relevant docs.
- No faked live-service evidence. If a live service is unavailable, the test remains opt-in and the blocker is reported.
