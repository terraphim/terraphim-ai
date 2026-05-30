---
stage: research
issue: 1887
classification: valid
timestamp: 2026-05-29T20:00:00Z
agent: disciplined-research-agent
---

## Issue Title
feat(adf-ctl): add pipeline-status subcommand for ADF artefact visibility

## Issue Body
## Acceptance Criteria

1. Running `adf-ctl pipeline-status <issue>` prints a summary of all ADF stage artefacts for the given issue
2. The command reads artefacts from `.docs/adf/<issue>/` directory
3. Output includes: stage name, completion status, file size, and timestamp for each artefact
4. Returns exit code 0 when artefacts exist, exit code 1 when the issue directory is missing
5. Unit tests cover: existing artefacts, missing directory, partial artefacts

## Implementation Details

This is a small, self-contained feature for the `adf-ctl` binary:

**New file:** `crates/terraphim_orchestrator/src/bin/adf_ctl/pipeline_status.rs` (or inline in `adf-ctl.rs`)

**Modified file:** `crates/terraphim_orchestrator/src/bin/adf-ctl.rs` - add `PipelineStatus` subcommand

### Command Interface

```
adf-ctl pipeline-status <issue>

# Example output:
Issue: #1886
Directory: .docs/adf/1886/

Stage Artefacts:
  research.md      | COMPLETE | 45 lines  | 2026-05-29T12:21:00Z
  design.md        | COMPLETE | 183 lines | 2026-05-29T12:13:59Z
  implementation.md| MISSING  | -         | -
  review.md        | MISSING  | -         | -

Summary: 2/4 artefacts complete
```

### Files to Change

1. `crates/terraphim_orchestrator/src/bin/adf-ctl.rs` - add `PipelineStatus` variant to the `Command` enum and handler
2. New test module or inline tests for the handler logic

### Constraints

- Do NOT add new dependencies
- Do NOT modify any files outside `crates/terraphim_orchestrator/src/bin/`
- Use only `std::fs` for file operations (no tokio needed)
- Follow existing code patterns in `adf-ctl.rs` for subcommand handling

## Labels

type/feature, status/ready

## Classification: valid
This is a well-specified, self-contained feature with clear acceptance criteria, no new dependencies required, and direct alignment with the existing ADF workflow. The `.docs/adf/` directory already exists and contains issue subdirectories, confirming the infrastructure is in place.

## Current State
The `adf-ctl` binary currently has 5 subcommands (Trigger, Status, Cancel, Agents, Flow) defined in `AdfSub` enum at `crates/terraphim_orchestrator/src/bin/adf-ctl.rs:49-121`. The `.docs/adf/` directory exists at the repo root and already contains subdirectories for tracked issues (e.g., `.docs/adf/1882/`, `.docs/adf/1887/`). The existing code uses `clap` for CLI parsing, `anyhow` for error handling, and `std::fs` for file operations (e.g., `discover_local_config` at line 187, `parse_agent_names_from_toml` at line 203). There is no existing pipeline-status functionality.

## Key Findings
- The `AdfSub` enum at `crates/terraphim_orchestrator/src/bin/adf-ctl.rs:49-121` uses `clap`'s `Subcommand` derive with named variants and doc comments (e.g., `/// Trigger an agent or persona by name` at line 51)
- The `run` function at `crates/terraphim_orchestrator/src/bin/adf-ctl.rs:128-166` pattern-matches on `AdfSub` variants and delegates to handler functions like `cmd_status`, `cmd_agents`, `cmd_flow`
- Existing handler functions like `cmd_status` at line 631 and `cmd_agents` at line 851 accept a `format: OutputFormat` parameter for human/JSON output -- the pipeline-status command should follow this pattern
- The `OutputFormat` enum at `crates/terraphim_orchestrator/src/bin/adf-ctl.rs:41-47` already supports `Human` and `Json` variants with `#[default]` and `ValueEnum` derives
- The `.docs/adf/` directory contains real issue directories: `.docs/adf/1882/` with files `research-proposal-1.md`, `design-proposal-1.md`, etc., and `.docs/adf/1887/` with `e2e-results.txt` and `flow-output.log`
- The existing test module at `crates/terraphim_orchestrator/src/bin/adf-ctl.rs:1030-1322` uses `#[cfg(test)]` and includes tests for payload building, signature verification, agent list parsing, journal activity parsing, and process parsing
- The `Cargo.toml` at `crates/terraphim_orchestrator/Cargo.toml:74-76` already includes `clap = { version = "4", features = ["derive"] }`, `jiff = { version = "0.2" }`, and `anyhow = { workspace = true }` -- no new dependencies needed

## Recommendations
1. Add a `PipelineStatus` variant to the `AdfSub` enum accepting `<issue>` as a positional argument and `--format` as an optional flag, following the exact pattern of `AdfSub::Agents`
2. Implement `cmd_pipeline_status(issue: &str, format: OutputFormat) -> Result<()>` that reads `.docs/adf/{issue}/`, checks for the 4 standard stage files (`research.md`, `design.md`, `implementation.md`, `review.md`), reports file metadata (size, modified time), and prints human-readable or JSON output
3. Return `Ok(())` (exit 0) when the directory exists, and use `bail!` (exit 1) when the directory is missing, matching the existing error handling patterns in `cmd_trigger` at line 511
4. Add unit tests in the existing `#[cfg(test)]` module covering: all 4 artefacts present, missing directory, partial artefacts (2/4 present), and JSON output shape verification

## Files Affected
- `crates/terraphim_orchestrator/src/bin/adf-ctl.rs`: Add `PipelineStatus` variant to `AdfSub` enum (line ~122), add match arm in `run()` function (line ~165), implement `cmd_pipeline_status()` handler function, and add unit tests in the `#[cfg(test)]` module
