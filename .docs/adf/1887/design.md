---
stage: design
issue: 1887
timestamp: 2026-05-29T21:00:00Z
agent: disciplined-design-agent
---

## Problem Statement
The `adf-ctl` CLI lacks visibility into the ADF pipeline artefact lifecycle. Users cannot quickly check which stage artefacts (research, design, implementation, review) have been produced for a given issue. This feature adds a `pipeline-status` subcommand that scans `.docs/adf/<issue>/` and reports completion status, file size, and timestamp for each expected artefact.

## Research Summary
- The `AdfSub` enum at `crates/terraphim_orchestrator/src/bin/adf-ctl.rs:49-121` defines 5 subcommands via `clap`'s `Subcommand` derive
- The `run` function at `crates/terraphim_orchestrator/src/bin/adf-ctl.rs:128-166` pattern-matches on `AdfSub` variants and delegates to handler functions (`cmd_status`, `cmd_agents`, `cmd_flow`, etc.)
- The `OutputFormat` enum at lines 41-47 supports `Human` and `Json` output modes
- Existing handler functions like `cmd_agents` accept `format: OutputFormat` for dual-mode output
- The `.docs/adf/` directory exists at the repo root and contains issue subdirectories (e.g. `.docs/adf/1882/`, `.docs/adf/1887/`)
- The existing test module at lines 1030-1322 uses `#[cfg(test)]` with `tempfile` for directory/file fixtures
- `Cargo.toml` already includes `clap`, `jiff`, `anyhow`, `serde` -- no new dependencies needed
- Error handling uses `anyhow::{bail, Result}`; `bail!` produces exit code 1

## Design Overview
Add a `PipelineStatus` variant to the `AdfSub` enum with a positional `issue` argument and an optional `--format` flag. Implement a `cmd_pipeline_status` handler that reads `.docs/adf/{issue}/`, checks for the 4 standard stage files (`research.md`, `design.md`, `implementation.md`, `review.md`), collects file metadata (size in bytes, modified time via `std::fs::metadata`), and prints formatted output. Returns `Ok(())` when the directory exists (even with 0/4 artefacts), and `bail!` when the issue directory is missing.

## File Changes

### New Files
| File | Purpose |
|------|---------|
| (none) | All changes fit within the existing `adf-ctl.rs` binary |

### Modified Files
| File | Change Description |
|------|--------------------|
| `crates/terraphim_orchestrator/src/bin/adf-ctl.rs:49-121` | Add `PipelineStatus` variant to `AdfSub` enum with `issue: String` positional arg and `#[arg(long, value_enum, default_value_t)] format: OutputFormat` |
| `crates/terraphim_orchestrator/src/bin/adf-ctl.rs:128-166` | Add match arm `AdfSub::PipelineStatus { issue, format } => cmd_pipeline_status(&issue, format)` in `run()` function |
| `crates/terraphim_orchestrator/src/bin/adf-ctl.rs` | Implement `cmd_pipeline_status(issue: &str, format: OutputFormat) -> Result<()>` handler function after `cmd_flow` (around line 980) |
| `crates/terraphim_orchestrator/src/bin/adf-ctl.rs` | Define `PipelineStatusReport` struct with `serde::Serialize` for JSON output, containing `issue`, `directory`, `artefacts: Vec<ArtefactInfo>`, and `summary: String` |
| `crates/terraphim_orchestrator/src/bin/adf-ctl.rs:1030-1322` | Add 5 unit tests in the `#[cfg(test)]` module: `test_pipeline_status_all_present`, `test_pipeline_status_missing_dir`, `test_pipeline_status_partial`, `test_pipeline_status_json_shape`, `test_pipeline_status_empty_dir` |

## Implementation Steps
1. Add `PipelineStatus` variant to `AdfSub` enum with doc comment `/// Show ADF pipeline artefacts for an issue`, positional `issue: String`, and optional `format: OutputFormat` flag following the `Agents` pattern
2. Add match arm in `run()` function to call `cmd_pipeline_status(&issue, format)`
3. Define `ArtefactInfo` struct (fields: `stage: String`, `filename: String`, `status: String`, `size: Option<u64>`, `timestamp: Option<String>`) and `PipelineStatusReport` struct with `#[derive(Serialize)]`
4. Implement `cmd_pipeline_status` function: construct `PathBuf` from `.docs/adf/{issue}/`, check `path.exists()` and `path.is_dir()`, bail if missing, iterate over `["research.md", "design.md", "implementation.md", "review.md"]`, call `std::fs::metadata` for each, build `Vec<ArtefactInfo>`, print human table or JSON via `serde_json::to_string_pretty`
5. Add unit tests using `tempfile::tempdir()` to create fixture directories, write sample files, assert output shape and exit behaviour

## Test Strategy
- `test_pipeline_status_all_present`: Creates `.docs/adf/9999/` with all 4 artefacts, asserts `cmd_pipeline_status` returns `Ok(())` and human output contains "4/4 artefacts complete"
- `test_pipeline_status_missing_dir`: Calls `cmd_pipeline_status` with a non-existent issue, asserts `is_err()` and error message contains the missing path
- `test_pipeline_status_partial`: Creates directory with only `research.md` and `design.md`, asserts output shows "2/4 artefacts complete" and MISSING for the other two
- `test_pipeline_status_json_shape`: Creates full fixture, calls with `OutputFormat::Json`, asserts JSON has `issue`, `directory`, `artefacts` array, and `summary` fields
- `test_pipeline_status_empty_dir`: Creates empty issue directory, asserts `Ok(())` and output shows "0/4 artefacts complete"

## Acceptance Criteria
- [ ] `cargo run -p terrraphim_orchestrator --bin adf-ctl -- pipeline-status 1887` prints a table with stage names and completion status
- [ ] `cargo run -p terrraphim_orchestrator --bin adf-ctl -- pipeline-status 99999` exits with code 1 and prints an error
- [ ] `cargo run -p terrraphim_orchestrator --bin adf-ctl -- pipeline-status 1887 --format json` prints valid JSON with the expected schema
- [ ] `cargo test -p terrraphim_orchestrator pipeline_status` runs 5 tests and all pass
- [ ] No new dependencies are added to `Cargo.toml`

## Risks
- **File naming mismatch**: Real `.docs/adf/` directories use `research-proposal-1.md` rather than `research.md`. The design hard-codes the 4 standard names from the issue acceptance criteria. If the naming convention changes, the command will report all MISSING. Mitigation: the issue explicitly specifies these 4 filenames; future enhancement can add glob matching.
- **Workspace root resolution**: The command assumes the current working directory is the repo root (or an ancestor) so `.docs/adf/` resolves correctly. Mitigation: use `std::env::current_dir()` and construct a relative path; document that the command should be run from the repo root.
