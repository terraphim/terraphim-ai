# Implementation Report: Project-Local and Server ADF from `.terraphim/adf.toml`

Date: 2026-05-20
Status: **COMPLETED**
Tracking issue: `terraphim/terraphim-ai#1764`

## Summary

Implemented project-local ADF configuration discovery and local agent execution. Agents can now be defined in `.terraphim/adf.toml` within any project and executed locally via `adf --local --agent NAME`.

## Changes

### New Files

- `crates/terraphim_orchestrator/src/project_adf.rs` (601 lines)
  - `ProjectAdfConfig` - parsed project-local ADF configuration
  - `TomlProjectAdfConfig` / `TomlAdfAgent` / `TomlPrDispatchEntry` - TOML schema
  - `discover_and_load()` - discovers `.terraphim/adf.toml` via upward search
  - `FromStr` impl for `AgentLayer`
  - `TryFrom<&ProjectAdfConfig>` impl for `(Project, Vec<AgentDefinition>)`
  - 14 unit tests

### Modified Files

- `crates/terraphim_orchestrator/src/bin/adf.rs` (+325 lines)
  - `Cli::LocalAgent { agent_name }` variant
  - `parse_args()` handles `--local --agent NAME`
  - `run_local_agent()` - discovers config, builds `OrchestratorConfig`, spawns agent, streams output
  - `build_local_spawn_context()` helper

- `crates/terraphim_orchestrator/tests/adf_check_tests.rs` (+119 lines)
  - 13 integration tests covering all acceptance criteria

## CLI Interface

```bash
# Validate project-local config
adf --local --check

# Run named agent locally
adf --local --agent <AGENT_NAME>
```

## Schema

```toml
project_id = "my-project"
name = "My Project"

[[agents]]
name = "safety-bot"
layer = "Safety"          # Safety | Compound | Inference
cli_tool = "echo"
task = "Run safety checks"
model = "kimi-for-coding/k2p6"  # optional
capabilities = ["code-generation"]  # optional
budget_monthly_cents = 10000      # optional
provider = "claude"              # optional

[[pr_dispatch]]  # optional
name = "pr-reviewer"
context = "terraphim/ci"
```

## Key Design Decisions

1. **No second runtime model** - `.terraphim/adf.toml` is compiled into existing `OrchestratorConfig` structures
2. **Reuse `.terraphim` discovery** - upward search from CWD
3. **Env var expansion** - `${VAR}` placeholders supported
4. **Output streaming** - spawned task for async output, `wait()` + 100ms sleep + `abort()` pattern for graceful exit

## Tests

All 13 tests pass:
- `adf_local_check_fails_when_no_adf_toml`
- `adf_local_check_succeeds_on_valid_adf_toml`
- `adf_local_check_fails_on_invalid_layer`
- `adf_local_check_requires_both_flags`
- `adf_check_fails_on_missing_file`
- `adf_check_succeeds_on_valid_inline_config`
- `adf_check_fails_on_banned_provider_with_nonzero_exit`
- `adf_check_expands_include_and_prints_merged_agents`
- `adf_check_table_is_sorted_by_project_then_agent`
- `adf_local_agent_spawns_echo_agent`
- `adf_local_agent_not_found`
- `adf_local_agent_no_terraphim_dir`
- `adf_local_agent_requires_agent_name`

## Quality Checks

- UBS: 0 critical, 195 warnings (test unwraps - expected)
- Sentrux: quality_signal 5258
- Clippy: clean
- Format: clean
- Tests: 13/13 pass

## Commits

```
e7b892944 chore: remove tmux temp file
33a35df6b feat(adf): implement adf --local --agent NAME for local agent execution
919b86a5a fix(spawner): use write_all with newline for oom_score_adj write
a0406dd50 feat(spawner): add OOM diagnostic logging and set_oom_score_adj helper
```

## Remaining Work

- Step 3 (future): Server import - server-side fleet picks up project-local agent definitions
- Documentation in user-facing docs (not internal `.docs/`)
