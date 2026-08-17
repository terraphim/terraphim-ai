You are executing the **disciplined-research** skill (Phase 1 of disciplined development) for Gitea issue #3144 in terraphim-ai.

## Task

Research how to wire `terraphim-agent` memory/learning lifecycle into TinyClaw's agent loop. Do NOT write code — produce a research report.

## Issue spec (from #3144)

Hermes Agent parity: TinyClaw needs persistent memory + learning. `terraphim-agent memory` already provides capture/distill/retrieve/apply/validate/retire + reliability rubric; `terraphim-agent learn` captures failed commands. TinyClaw keeps per-chat JSONL sessions but has no cross-session memory. The `ToolRegistry` in `crates/terraphim_tinyclaw/src/tools/mod.rs` is the extension point.

Scope:
- 4 tools implementing the existing `Tool` trait: `memory_capture`, `memory_retrieve`, `memory_apply`, `learn_capture`
- Shell bridge via `tokio::process::Command` calling `terraphim-agent memory capture|retrieve|apply` and `terraphim-agent learn capture` with `--format json --robot`
- Parse JSON output → structured Markdown to the agent loop
- In `src/agent/agent_loop.rs`, prepend retrieved memories to the system prompt when `memory_apply` returns items
- `[memory]` config section in `Config` with `enabled` and `role` overrides

## Research steps

1. Read `crates/terraphim_tinyclaw/src/tools/mod.rs` — understand the `Tool` trait (signature, async or not, schema format), how `ToolRegistry::register` works, how existing tools (filesystem, shell, web) are implemented, and how the agent loop invokes tools.
2. Read `crates/terraphim_tinyclaw/src/agent/agent_loop.rs` — how the system prompt is built, where tool results are fed back, where memory context should be injected.
3. Read `crates/terraphim_tinyclaw/src/config.rs` — how config sections are declared/parsed, where a `[memory]` section fits.
4. Inspect the `terraphim-agent` binary's memory/learn CLI surface (it is on PATH as `terraphim-agent`): run `terraphim-agent memory --help` and `terraphim-agent learn --help` (or read `crates/terraphim_agent/src/commands/` source) to document exact subcommand names, flags (`--format json --robot`), and JSON output shapes.
5. Check `crates/terraphim_tinyclaw/tests/` for existing contract-test patterns (e.g. credentials_pool_tests.rs, proxy_contracts.rs) so new tests match house style.
6. Note the existing `memory/` module in tinyclaw (jsonl.rs, sqlite.rs — Wave 6) — determine whether the new MemoryTool should sit alongside it (separate file `src/tools/agent_memory.rs`) and how it differs (shell bridge vs backend trait).

## Output

Write your findings to `.docs/adf/3144/research.md` in this format:

```markdown
# Research: #3144 TinyClaw memory/learning bridge

## Tool trait contract
<signature, register pattern, schema format>

## Existing tool examples
<how filesystem/shell tools are structured>

## agent_loop integration point
<where system prompt is built; where memory context goes>

## terraphim-agent CLI surface
<exact subcommands + flags + JSON shapes, verified>

## Config addition
<where [memory] goes>

## Test patterns
<house style for contract tests>

## Risks / edge cases
<missing binary, empty store, token budget, JSON parse failures>

## Recommendation
<concrete file-level plan>
```

Be precise and cite file:line references. Output ONLY the report.
