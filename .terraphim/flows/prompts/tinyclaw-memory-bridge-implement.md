You are executing the **disciplined-implementation** skill (Phase 3 of disciplined development) for Gitea issue #3144 in terraphim-ai.

## Task

Implement the memory/learning bridge exactly per `.docs/adf/3144/design.md` (produced by the design phase). Read both `.docs/adf/3144/research.md` and `.docs/adf/3144/design.md` before writing code.

## Hard constraints

1. **Working tree discipline**: you are on branch `task/3144-tinyclaw-memory-bridge` (already created for you). Only modify files under `crates/terraphim_tinyclaw/` and `.docs/adf/3144/`. Do NOT touch pre-existing dirty files (`package.json`, `AGENTS.md`, `.cursor/`, `.opencode/`, `.ubsignore`, `scripts/`, `pitch-deck/`).
2. **Implement**:
   - `crates/terraphim_tinyclaw/src/tools/agent_memory.rs` — 4 tools: `memory_capture`, `memory_retrieve`, `memory_apply`, `learn_capture`, implementing the existing `Tool` trait. Shell bridge via `tokio::process::Command` to `terraphim-agent memory ... --format json --robot` / `terraphim-agent learn ... --format json --robot`. Parse JSON robot output; graceful error if binary missing; graceful empty result on no matches.
   - Register tools in `crates/terraphim_tinyclaw/src/tools/mod.rs`.
   - `crates/terraphim_tinyclaw/src/agent/agent_loop.rs` — prepend `memory_apply` items to the system prompt when memory enabled and items exist; guard against huge dumps (truncate).
   - `crates/terraphim_tinyclaw/src/config.rs` — `[memory]` section: `enabled` (default false), `role` (Option<String>).
   - Add `mod agent_memory;` wiring as required by the module layout.
3. **Tests**: add unit tests for robot-JSON parsing and a hermetic integration test using a scripted `terraphim-agent` shim (a shell script on a temp PATH that returns canned robot JSON) — do NOT depend on the real binary.
4. **Verify before finishing** (all must pass):
   - `cargo fmt -p terraphim_tinyclaw -- --check` (if it fails, run `cargo fmt -p terraphim_tinyclaw`)
   - `cargo clippy -p terraphim_tinyclaw -- -D warnings` (if warnings, fix them)
   - `cargo test -p terraphim_tinyclaw` (full crate suite — existing 365+ tests must stay green)
5. Do NOT commit; do NOT push; do NOT create a PR. The flow's verification phase will run after you. Just leave the working tree with the implementation + tests in place.
6. Report back: list of files changed, test counts, any deviations from the design.

Work carefully; prefer matching existing house patterns over cleverness.
