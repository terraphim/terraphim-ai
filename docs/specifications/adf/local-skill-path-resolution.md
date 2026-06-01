# Specification: Local Project Skill Path Resolution

**Status**: Authoritative
**Source**: `crates/terraphim_orchestrator/src/local_skills.rs`
**Issue**: #1924 (re-scoped from PR #1788 Slice 8)
**Date**: 2026-06-01

---

## Overview

When the ADF orchestrator runs an agent under `adf --local`, it resolves project-local
skills from a well-known directory tree under the project root. This specification
describes the discovery algorithm, native bridge wiring, and invariants that hold
across all supported CLI tools.

---

## Behaviour

### Discovery

The orchestrator looks for a directory at:

```
<project_root>/.terraphim/skills/
```

If the directory exists it is considered the canonical project skill store.
If it is absent, skill loading is a no-op: the agent spawn proceeds unchanged.

### Native Bridge Wiring

For CLI tools that have their own skill directory conventions, the orchestrator
creates a platform symlink from the native location into `.terraphim/skills`:

| CLI tool | Native skill path | Symlink created? |
|----------|------------------|-----------------|
| `opencode` | `.opencode/skill` | Yes (if absent) |
| `claude` or `claude-code` | `.claude/skills` | Yes (if absent) |
| Any other binary | n/a | No |

The symlink is created with `std::os::unix::fs::symlink` on Unix and
`std::os::windows::fs::symlink_dir` on Windows.

**Non-overwrite invariant**: if the native skill directory already exists, the
orchestrator leaves it untouched. It never replaces an existing native bridge.

### Environment Variable Export

Regardless of CLI type, the orchestrator sets:

```
TERRAPHIM_LOCAL_SKILLS_DIR=<project_root>/.terraphim/skills
```

on the spawn context so the spawned process can locate skills itself if it reads
this variable.

---

## Invariants

| # | Invariant | Source |
|---|-----------|--------|
| I1 | If `.terraphim/skills/` does not exist, `SpawnContext` is returned unmodified. | `prepare_local_skill_loading` early return |
| I2 | `TERRAPHIM_LOCAL_SKILLS_DIR` is always set when `.terraphim/skills/` exists. | `ctx.with_env(...)` unconditional |
| I3 | Existing native skill directories are never overwritten. | `if native_dir.exists() { return Ok(()); }` |
| I4 | Symlink creation failure is logged as WARN but does not abort the agent spawn. | `tracing::warn!(...)` in `prepare_local_skill_loading` |
| I5 | CLI tool detection uses the basename of the binary path, not the full path. | `cli_name()` uses `Path::file_name()` |

---

## Failure Modes

| Failure | Observable Effect | Recovery |
|---------|-------------------|---------|
| `.terraphim/skills/` missing | No skill loading; env var absent | Create directory |
| Symlink creation IO error | WARN logged; bridge absent; env var still set | Investigate fs permissions |
| Unsupported CLI binary | No symlink; env var set | Agent reads `TERRAPHIM_LOCAL_SKILLS_DIR` if it honours the variable |
| Native dir is a file not a dir | Exists check returns true; no symlink | No recovery needed — env var still points to `.terraphim/skills` |

---

## Verification Note

All invariants above are covered by the unit tests in the source file. The following
tests were verified to pass on `gitea/main` as of 2026-06-01:

```bash
cargo test -p terraphim_orchestrator local_skills -- --nocapture
```

Tests verified:
- `discover_local_skills_returns_none_when_missing`
- `discover_local_skills_finds_project_skills_dir`
- `detect_skill_cli_handles_supported_names_and_paths`
- `prepare_local_skill_loading_is_noop_when_skills_missing`
- `prepare_local_skill_loading_bridges_opencode_project_skills`
- `prepare_local_skill_loading_bridges_claude_project_skills`
- `prepare_local_skill_loading_does_not_overwrite_existing_native_dir`
- `unsupported_cli_exports_terraphim_skill_dir_without_native_bridge`
