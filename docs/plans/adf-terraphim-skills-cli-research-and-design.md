# Research + Design: tsm -- ADF Agent for Multi-CLI Skill Installation and Validation

**Status**: Draft
**Author**: AI Planning Agent
**Date**: 2026-06-13
**Phases**: Phase 1 (Research) + Phase 2 (Design)

---

# PART 1: RESEARCH DOCUMENT

## Executive Summary

The project needs an ADF agent that uses the existing Terraphim Skills manager binary, `tsm`, to install, bridge, and validate Terraphim skills across five agent CLIs: Claude Code, Codex, pi, pi-rust, and Grok. The current `local_skills.rs` module only supports Opencode and Claude; it lacks coverage for Codex, pi, pi-rust, and Grok. Each CLI has a different skill discovery mechanism and directory layout, creating friction for cross-CLI skill management.

## Essential Questions Check

| Question | Answer | Evidence |
|----------|--------|----------|
| Energizing? | Yes | Eliminates manual per-CLI skill setup; unifies skill lifecycle |
| Leverages strengths? | Yes | Builds on existing `local_skills.rs`, `gitea_skill_loader.rs`, and ADF orchestrator |
| Meets real need? | Yes | ADF agents use different CLI backends; skills must be available on all of them |

**Proceed**: Yes (3/3)

## Problem Statement

### Description

There is no unified CLI for installing Terraphim skills across all supported agent backends. Each CLI discovers skills differently:

| CLI | Skill Discovery Mechanism | Native Skill Directory |
|-----|--------------------------|----------------------|
| Claude Code | `.claude/skills/<name>/SKILL.md` | `.claude/skills/` |
| Codex | `AGENTS.md` conventions + `~/.codex/` config | `~/.codex/instructions/` (or project-level) |
| pi (pi_agent_rust) | Installer-managed; `.claude/skills/pi-agent-rust/` | `.claude/skills/` (via installer) |
| pi-rust | Opencode config; `.opencode/skill/` | `.opencode/skill/` |
| Grok | User skills under `~/.grok/skills/`; discovered by `grok --cwd <workspace> inspect --json` | `~/.grok/skills/` |

### Impact

- ADF agents using Codex, pi, pi-rust, or Grok cannot access Terraphim skills
- Manual skill installation is error-prone and non-repeatable
- No validation that skills are correctly loaded after installation
- Skills drift between CLIs over time

### Success Criteria

1. `tsm install` installs selected sentinel skills to all 5 CLIs through `--agent` or `--install-dir`
2. `tsm verify` plus CLI-specific probes verifies each CLI can discover the installed skills
3. An ADF agent (`skills-installer`) automates this process on bigbox
4. Each CLI reports skill availability when queried
5. The ADF runner renders a skills-by-CLI matrix with green/red per-skill test results; current local `tsm` source does not expose a `status` subcommand.
6. ADF `ci-native` validates repository-side changes on PRs before any live host validation runs.

## Current State Analysis

### Existing Implementation

**`local_skills.rs`** (`crates/terraphim_orchestrator/src/local_skills.rs`):
- `SupportedSkillCli` enum: only `Opencode` and `Claude`
- `detect_skill_cli()`: matches `opencode`, `claude`, `claude-code`
- `native_skill_dir()`: maps CLI to `.opencode/skill/` or `.claude/skills/`
- `ensure_native_skill_bridge()`: creates symlink from `.terraphim/skills/` to native dir
- `prepare_local_skill_loading()`: sets `TERRAPHIM_LOCAL_SKILLS_DIR` env var

**`gitea_skill_loader.rs`** (`crates/terraphim_orchestrator/src/gitea_skill_loader.rs`):
- Fetches `SKILL.md` from Gitea repos via curl
- Caches to `skill_data_dir`
- Offline-resilient: falls back to local on network failure

**Skill format**: `SKILL.md` with YAML frontmatter (`name`, `description`)

### Code Locations

| Component | Location | Purpose |
|-----------|----------|---------|
| Local skill bridge | `crates/terraphim_orchestrator/src/local_skills.rs` | Symlink skills to CLI-native dirs |
| Gitea skill fetcher | `crates/terraphim_orchestrator/src/gitea_skill_loader.rs` | Remote skill caching |
| Orchestrator config | `crates/terraphim_orchestrator/orchestrator.example.toml` | ADF agent definitions |
| Project skills | `.terraphim/skills/` | Canonical skill source |
| pi-agent-rust skill | `~/.claude/skills/pi-agent-rust/SKILL.md` | Installer-managed skill |
| ADF orchestrate skill | `~/.agents/skills/adf-orchestrate/SKILL.md` | ADF trigger/monitor docs |

### Integration Points

- ADF orchestrator uses `cli_tool` field per agent (currently all `claude`)
- `prepare_local_skill_loading()` is called at agent spawn time
- Skills are defined in `.terraphim/skills/` and Gitea repos
- Codex stores sessions in `~/.codex/sessions/` (connector exists)

## Constraints

### Technical Constraints

- **C1**: Must not break existing `local_skills.rs` behaviour for Opencode/Claude
- **C2**: Codex does not have a `.claude/skills/` equivalent -- uses `AGENTS.md` conventions
- **C3**: Grok CLI has a concrete user skills directory at `~/.grok/skills`, but current `tsm` has no first-class `Agent::Grok` mapping.
- **C4**: pi_agent_rust has its own installer lifecycle (`install.sh`, `uninstall.sh`) that must not conflict
- **C5**: pi-rust uses opencode configuration under the hood

### Vital Few (Essentialism)

| Constraint | Why Vital | Evidence |
|------------|-----------|----------|
| C1: Backward compatibility | Breaking Opencode/Claude skill loading breaks the entire ADF fleet | All 13 agents use `claude` CLI |
| C2: Codex skill mechanism | Codex has no skills directory -- requires a different approach | `connectors/codex.rs` shows `~/.codex/` only |
| C3: Grok adapter gap | `tsm` cannot currently target Grok with `--agent grok` | Local `grok` is installed, but local `tsm` lacks `Agent::Grok`. |

### Eliminated from Scope

| Eliminated Item | Why Eliminated |
|-----------------|---------------|
| Skill versioning/rollback | Not needed for initial rollout |
| Skill marketplace/discovery UI | YAGNI; Gitea repo is source of truth |
| Hot-reload of skills | ADF agents are long-lived; restart picks up changes |
| Skill dependency resolution | Skills are independent SKILL.md files |

## Dependencies

### Internal Dependencies

| Dependency | Impact | Risk |
|------------|--------|------|
| `local_skills.rs` | Must extend, not replace | Low -- additive enum variants |
| `gitea_skill_loader.rs` | Source of skills to install | Low -- already functional |
| `orchestrator.example.toml` | Must add new agent definition | Low -- TOML config |
| `terraphim_spawner` | SpawnContext used by `prepare_local_skill_loading` | Low |
| ADF `ci-native` | Runs PR-side script, TOML, and dry-run validation without Gitea-native workflows | Medium -- runner capability/configuration must be confirmed. |

### External Dependencies

| Dependency | Version | Risk | Alternative |
|------------|---------|------|-------------|
| Codex CLI | Current | Med -- unclear skill format | Write skills to `AGENTS.md` |
| Grok CLI | `grok 0.2.51` locally observed | Medium -- `tsm` needs `--install-dir` until `Agent::Grok` exists | Install to `~/.grok/skills/` and probe with `grok --cwd <workspace> inspect --json` |
| pi_agent_rust | Current | Low -- documented installer | N/A |

## Risks and Unknowns

### Known Risks

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| `tsm` lacks first-class Grok agent mapping | High | Med | Use `tsm --install-dir "$HOME/.grok/skills" ...` until `Agent::Grok` is added. |
| Codex lacks skills directory | High | Med | Use `AGENTS.md` + `~/.codex/` conventions |
| Symlink conflicts on Windows | Low | Low | Existing `#[cfg(unix)]`/`#[cfg(windows)]` handles this |
| pi installer conflict | Med | High | Check if pi-managed skill exists before bridging |

### Open Questions

1. **Codex**: Does Codex support a skills-like mechanism, or only AGENTS.md? Check Codex docs
2. **pi-rust**: Is pi-rust purely opencode-based, or does it have its own skill layer?
3. **pi**: What is badlogic Pi's canonical skills directory and non-interactive probe command?

### Assumptions Explicitly Stated

| Assumption | Basis | Risk if Wrong | Verified? |
|------------|-------|---------------|-----------|
| pi-rust uses opencode skill dir | pi-rust-terraphim-router skill references opencode.json | pi-rust has separate skill dir | No |
| Codex uses AGENTS.md for skills | AGENTS.md is the standard convention | Codex has separate skill mechanism | No |
| Grok has `~/.grok/skills/` user skill dir | `which grok`, `grok --version`, `~/.grok/skills`, and `grok --cwd ... inspect` checked locally | ADF host differs from local machine | Locally verified |
| Skills are format-compatible across CLIs | All use SKILL.md markdown | Some CLIs need different format | Partially |

---

# PART 2: IMPLEMENTATION PLAN

## Overview

### Summary

Build an ADF validation agent around the existing `tsm` binary that:
1. Installs selected sentinel skills through the marketplace download path
2. Targets first-class `tsm --agent` mappings where they exist and `--install-dir` where they do not
3. Validates that each CLI can discover the installed skills
4. Renders a red/green skills-versus-CLI status matrix for every validated skill
5. Runs as an ADF agent for automated fleet-wide skill management evidence

### Approach

Use `tsm` as the skills manager, add a shell-driven ADF validation agent `skills-installer` or `skills-cli-validator`, and only extend `local_skills.rs` after validation proves an orchestrator-native bridge is required.

### Scope

**In Scope:**
- Use the existing `tsm` binary (`list`, `install`, `verify`, `validate`) and render status in the ADF runner
- Create ADF agent `skills-installer` in `conf.d/skills-installer.toml`
- Create or configure an ADF `ci-native` validation path for PR-side checks
- Create `scripts/skill-installer-validation.sh` to run real `tsm` installs/verifications and CLI probes
- Validate each CLI using the least invasive supported path: `--agent` where available, `--install-dir` where not
- Red/green skills-by-CLI status matrix for release evidence

**Out of Scope:**
- Skill authoring/editing tools
- Skill versioning
- Marketplace integration
- Hot-reload

**Avoid At All Cost:**
- Re-implementing Gitea skill fetching (use existing `gitea_skill_loader.rs`)
- Building a TUI for skill management
- Per-skill enable/disable flags
- Skill dependency graph resolution
- Extending `local_skills.rs` before validation proves a native bridge is required
- Adding `.gitea/workflows/*` or using Gitea-native CI. This plan must use ADF `ci-native`, not Forgejo/Gitea Actions.

## Architecture

### Component Diagram

```
                          tsm
                   /        |         |          \
          install      verify       list      validate
             |            |          |          |
     .terraphim/skills/   |    Gitea cache    Report
             |            |
     +-------+-------+----+----+----------+
     |       |       |    |    |          |
   Claude  Codex   pi  pi-rust  Grok   (Opencode)
      |       |       |    |    |
.claude/  AGENTS  PI_DIR   .opencode  .grok/
skills/   .md              /skill/    skills/
```

### Data Flow

```
[.terraphim/skills/] + [Gitea cache]
         |
          tsm install
         |
    +----+----+----+----+----+
    |    |    |    |    |    |
 tsm/agent AGENTS PI_DIR tsm/agent inspect
    |    |    |    |    |    |
    v    v    v    v    v    v
  Claude Codex pi pi-rust Grok Opencode
         |
     tsm verify + CLI probes
         |
    Verify each CLI sees the skills
         |
    Report: PASS/FAIL per skill per CLI

Pull request / branch update
         |
     ADF ci-native
         |
  bash syntax + script dry-run + ADF TOML validation + repository inventory
         |
  ADF commit status / PR evidence, separate from Gitea-native workflows
```

### Key Design Decisions

| Decision | Rationale | Alternatives Rejected |
|----------|-----------|----------------------|
| Defer `local_skills.rs` changes | Validation can prove actual CLI behaviour first | Premature native bridge expansion. |
| Use existing `tsm` binary | Local source already exists in `terraphim-skills.md`; avoids inventing a second command surface | New orchestrator CLI binary. |
| Use `tsm --agent` where available | Exercises the real skills manager path | Hand-written symlink-only bridge. |
| Use `--install-dir` for unsupported first-class agents | Handles Pi/Grok without pretending mappings exist | Add guessed `Agent::Pi`/`Agent::Grok` immediately. |
| Install Grok skills to `~/.grok/skills/` | Verified local Grok user skills directory and `inspect` output | Write to guessed `~/.grok/instructions/`. |
| ADF agent uses `/bin/bash` | The agent is an operational validator, not an LLM reasoning task | Run under `claude` and ask it to execute shell steps. |
| Use ADF `ci-native` for PR validation | Keeps CI under the ADF system that already owns build/review statuses | Add `.gitea/workflows/*` or rely on Gitea Actions. |
| Status matrix is the primary evidence view | Per-CLI summary can hide a single missing skill | Plain aggregate pass/fail summary only |

### Eliminated Options

| Option Rejected | Why Rejected | Risk of Including |
|-----------------|--------------|-------------------|
| Separate orchestrator skills CLI | `tsm` already exists as the Terraphim Skills manager | Maintenance burden, version drift |
| Per-skill enable/disable | Not needed; all-or-nothing is fine | Feature creep, config complexity |
| Web UI for skill management | ADF agents manage skills; humans read reports | Frontend maintenance, scope explosion |
| Bypassing skill signing/verification | Release evidence must prove BLAKE3 and Ed25519 checks are active | Faster but false-positive install validation. |
| Cross-CLI format translation | All CLIs accept SKILL.md markdown | Per-CLI parsers, format drift |

### Simplicity Check

**What if this could be easy?**

The simplest approach: use `tsm` directly, configure per-target install paths, run deterministic CLI probes, and register an ADF agent. No new crates, no new public command surface, no guessed Grok path.

**Senior Engineer Test**: Would this be called overcomplicated? No -- it wraps the existing `tsm` binary with a validation script and only changes orchestrator code if validation proves a real bridge gap.

**Nothing Speculative Checklist**:
- [x] No features the user didn't request
- [x] No abstractions "in case we need them later"
- [x] No flexibility "just in case"
- [x] No error handling for scenarios that cannot occur
- [x] No premature optimization

## File Changes

### New Files

| File | Purpose |
|------|---------|
| `crates/terraphim_orchestrator/conf.d/skills-installer.toml` | ADF agent definition using `/bin/bash`, `task`, `max_cpu_seconds`, and `gitea_issue`. |
| `crates/terraphim_orchestrator/conf.d/skills-cli-ci-native.toml` | ADF `ci-native` job definition for PR-side validation, if the repository does not already provide a central ci-native config. |
| `scripts/skill-installer-validation.sh` | Validation script for ADF agent; invokes `tsm` and renders JSON/Markdown evidence. |
| `scripts/skill-installer-ci-native.sh` | Non-secret ADF ci-native wrapper for syntax checks, dry-run matrix rendering, TOML validation, and repository ownership inventory. |

### Modified Files

| File | Changes |
|------|---------|
| `crates/terraphim_orchestrator/src/local_skills.rs` | Modify only if validation proves native bridges are required; preserve existing Claude/opencode behaviour. |
| `crates/terraphim_orchestrator/Cargo.toml` | No change expected for validation-only implementation. |
| `crates/terraphim_orchestrator/src/lib.rs` | No change expected for validation-only implementation. |

### Deleted Files

None.

### ADF ci-native Contract

ADF `ci-native` must validate repository-side artefacts without requiring paid model calls or live subscription-gated downloads by default. It must not create `.gitea/workflows/*` and must not depend on Forgejo/Gitea Actions.

Required checks:

| Check | Command shape | Purpose |
|-------|---------------|---------|
| Shell syntax | `bash -n scripts/skill-installer-validation.sh` and `bash -n scripts/skill-installer-ci-native.sh` | Catches syntax regressions without executing live installs. |
| Shell lint | `shellcheck ...` when available | Catches quoting and control-flow defects; report as actionable evidence if unavailable. |
| ADF TOML parse | `adf agent validate-all --config crates/terraphim_orchestrator/conf.d/skills-installer.toml --format json --skip-model-probe` | Ensures schema fields bind correctly. |
| Dry-run matrix | `TERRAPHIM_SKILLS_DRY_RUN=1 scripts/skill-installer-validation.sh` | Proves matrix rendering and target enumeration without secrets. |
| Repository inventory | `gtr list-repos --org terraphim` or equivalent ADF-host API check | Ensures Terraphim Skills dependencies stay under the `terraphim` organisation. |

ADF `ci-native` may post commit status / PR evidence, but it must not be confused with Gitea-native CI. Branch protection checks such as `adf/build` and `adf/pr-reviewer` remain ADF-posted statuses.

## Runner Contract

### `tsm` Interface

```bash
# List registry/catalogue entries supported by local `tsm`
tsm list

# Install and verify skills for first-class targets
tsm --agent claude-code install <skill>
tsm --agent claude-code verify <skill>
tsm --agent codex install <skill>
tsm --agent codex verify <skill>
tsm --agent pi-rust install <skill>
tsm --agent pi-rust verify <skill>

# Install and verify skills for targets not yet represented in local Agent enum
tsm --install-dir "$HOME/.grok/skills" install <skill>
tsm --install-dir "$HOME/.grok/skills" verify <skill>
tsm --install-dir "$TERRAPHIM_SKILLS_PI_DIR" install <skill>
tsm --install-dir "$TERRAPHIM_SKILLS_PI_DIR" verify <skill>

# Probe Grok inventory without model invocation
grok --cwd <workspace> inspect --json
```

### Status Matrix Output

The ADF runner must render a skills-by-CLI matrix. Rows are skills; columns are target CLIs. Each cell includes a text label, colour, and short evidence string so the result remains accessible when colour is stripped from logs.

Status semantics:

| Status | Colour | Meaning | Required Target Gate |
|--------|--------|---------|----------------------|
| `PASS` | Green | Skill is installed and discoverable for that CLI | Passes |
| `FAIL` | Red | Skill was expected but missing or invalid for that CLI | Fails |
| `SKIP` | Grey | CLI or probe was intentionally skipped | Neutral only when target is optional |
| `UNSUPPORTED` | Amber | CLI exists but required mapping/configuration is missing | Fails until explicitly waived |

Markdown/Gitea example:

```markdown
| Skill | Claude Code | Codex | pi | pi-rust | Grok |
|-------|-------------|-------|----|---------|------|
| code-review | <span style="color:#15803d">PASS</span> `tsm --agent claude-code verify` | <span style="color:#b91c1c">FAIL</span> missing from Codex probe | <span style="color:#b45309">UNSUPPORTED</span> Pi dir not configured | <span style="color:#15803d">PASS</span> `tsm --agent pi-rust verify` | <span style="color:#15803d">PASS</span> `grok inspect` lists skill |
```

JSON example:

```json
{
  "skills": ["code-review", "testing"],
  "clis": ["claude", "codex", "pi", "pi-rust", "grok"],
  "matrix": [
    {
      "skill": "code-review",
      "claude": { "status": "PASS", "colour": "green", "evidence": ".claude/skills" },
      "codex": { "status": "FAIL", "colour": "red", "evidence": "missing AGENTS.md section" },
      "pi": { "status": "PASS", "colour": "green", "evidence": "shared Claude dir" },
      "pi-rust": { "status": "PASS", "colour": "green", "evidence": ".opencode/skill" },
      "grok": { "status": "PASS", "colour": "green", "evidence": "grok inspect lists skill from ~/.grok/skills" }
    }
  ],
  "overall": "FAIL"
}
```

Roll-up rules:

1. Overall status is `PASS` only when every required skill is `PASS` for every required CLI.
2. Any `FAIL` cell on a required CLI makes the overall status `FAIL`.
3. Any `UNSUPPORTED` cell on a required CLI makes the overall status `FAIL` until waived by config.
4. `SKIP` is neutral only when the CLI or probe is explicitly optional.

### Per-CLI Validation Strategy

| CLI | Strategy | Details |
|-----|----------|---------|
| Claude Code | `tsm --agent claude-code install/verify <skill>` | Probe installed files and, if needed, a non-interactive Claude inventory command. |
| Codex | `tsm --agent codex install/verify <skill>` | Confirm actual Codex discovery mechanism in Step 0 before treating AGENTS.md as sufficient. |
| Pi | `tsm --install-dir "$TERRAPHIM_SKILLS_PI_DIR" install/verify <skill>` | Required until badlogic Pi path and/or `Agent::Pi` exists. |
| pi-rust | `tsm --agent pi-rust install/verify <skill>` | Confirm whether runtime also needs opencode visibility. |
| Grok | `tsm --install-dir "$HOME/.grok/skills" install/verify <skill>` plus `grok --cwd <workspace> inspect --json` | Verified locally; recheck on ADF host. |

## Test Strategy

### Script Tests

| Test | Location | Purpose |
|------|----------|---------|
| `script_requires_tsm` | `scripts/skill-installer-validation.sh` | Fails clearly when `tsm` is missing. |
| `script_requires_token_for_live_download` | `scripts/skill-installer-validation.sh` | Fails without `TSM_TOKEN` or `~/.terraphim/token` in live mode. |
| `script_renders_matrix` | `scripts/skill-installer-validation.sh` | Emits Markdown and JSON rows for every sentinel skill and target CLI. |
| `script_marks_missing_cli_skip_or_fail` | `scripts/skill-installer-validation.sh` | Distinguishes missing optional CLI from required target failure. |
| `script_uses_grok_inspect_json` | `scripts/skill-installer-validation.sh` | Uses `grok --cwd <workspace> inspect --json`, not `grok inspect --cwd`. |

### Integration Tests

| Test | Location | Purpose |
|------|----------|---------|
| `adf_config_validates` | ADF validation command | Confirms TOML fields bind correctly and no ignored `[agents.command]`/`max_cpu_secs` fields exist. |
| `adf_ci_native_dry_run` | ADF ci-native job | Confirms PR-side ADF CI runs shell syntax, TOML validation, dry-run matrix, and repository inventory checks. |
| `live_tsm_install_verify_round_trip` | ADF/manual run | Installs and verifies sentinel skills through live `tsm` with a dedicated token. |
| `grok_inventory_round_trip` | ADF/manual run | Confirms Grok lists installed sentinel skills from `~/.grok/skills`. |

### Validation Script

```bash
#!/bin/bash
# scripts/skill-installer-validation.sh
# Used by the ADF skills-installer agent

set -euo pipefail

SKILLS_DIR="${1:-.terraphim/skills}"
PASS=0
FAIL=0

for CLI in claude codex pi pi-rust grok; do
    echo "=== Validating $CLI ==="
    if scripts/probe-one-skill-target.sh "$CLI" "$SKILLS_DIR"; then
        echo "PASS: $CLI skills validated"
        ((PASS++))
    else
        echo "FAIL: $CLI skills missing or invalid"
        ((FAIL++))
    fi
done

echo ""
echo "=== Summary ==="
echo "Passed: $PASS / $((PASS + FAIL))"
echo "Failed: $FAIL / $((PASS + FAIL))"
echo ""
echo "=== Skills x CLI matrix ==="
scripts/render-skills-cli-status-matrix.sh "$SKILLS_DIR"

[ "$FAIL" -eq 0 ] || exit 1
```

## Implementation Steps

### Step 0: Technical Spike -- COMPLETE

**Verified locally 2026-06-14:**

| CLI | Binary | Version | Skills Dir | Non-interactive Probe | Status |
|-----|--------|---------|------------|----------------------|--------|
| Claude Code | `claude` | 2.1.177 | `~/.claude/skills/` | `ls ~/.claude/skills/<skill>` | Confirmed |
| Codex | `codex` | 0.122.0 | `~/.codex/skills/` | `ls ~/.codex/skills/<skill>`, `codex exec` (headless) | Confirmed |
| Pi (badlogic) | `pi` | 0.1.16 | `~/.pi/agent/skills/` | `pi list` | Confirmed |
| pi-rust | `pi-rust` | 0.1.15 | `~/.pi/agent/skills/` and `~/.agents/skills/` | `pi-rust list` (returns empty; shares Pi config at `~/.pi/agent/`) | Confirmed |
| Grok | `grok` | 0.2.51 | `~/.grok/skills/` | `grok --cwd ~/.grok/skills inspect --json` (`.skills[]` key) | Confirmed |

**Key findings:**
- Codex uses `~/.codex/skills/` (confirmed by tsm agent mapping and filesystem)
- Badlogic Pi skills live at `~/.pi/agent/skills/` (discovered via `pi config` → Packages path)
- pi-rust shares Pi's config directory at `~/.pi/agent/`; `pi-rust list` returns no packages but skills are present
- Grok `inspect --json` has a structured `.skills[]` array with name/description/source
- `tsm verify` requires `skill.toml` metadata (only present for marketplace-installed skills)
- `pi list` and `pi-rust list` work non-interactively (no stdin required)
**Tests:** All CLI probes confirmed.

### Step 1: Confirm `tsm` Runtime Surface
**Files:** N/A, or `terraphim-skills.md` for source confirmation
**Description:** Verify `tsm --version`, `tsm list`, `tsm --agent <agent> install <skill>`, `tsm --agent <agent> verify <skill>`, `tsm --install-dir <path> install <skill>`, and `tsm --install-dir <path> verify <skill>` on the ADF host. Do not assume `tsm status`, `--json`, `--format`, or `--offline` unless local source or runtime help confirms them.
**Tests:** Command smoke checks against a non-secret sentinel skill
**Dependencies:** Step 0 findings
**Estimated:** 30 min

### Step 2: Create Validation Script
**Files:** `scripts/skill-installer-validation.sh`
**Description:** Shell script that expands sentinel skills and target CLIs, invokes `tsm` with `--agent` where supported and `--install-dir` where required, runs CLI-specific probes, and renders JSON plus Markdown matrix evidence. Default Grok install path is `$HOME/.grok/skills`.
**Tests:** Shellcheck, local dry run with download disabled, live run with `TSM_TOKEN` when approved
**Dependencies:** Step 1
**Estimated:** 30 min

### Step 3: Create ADF Agent Definition
**Files:** `crates/terraphim_orchestrator/conf.d/skills-installer.toml`
**Description:** TOML agent definition using `cli_tool = "/bin/bash"`, `task = """..."""`, `max_cpu_seconds`, `project`, and `gitea_issue`. Avoid `[agents.command]`, `max_cpu_secs`, and `model = "none"`.
**Tests:** `adf agent validate-all --config <cfg> --format json --skip-model-probe` plus JSON inspection that expected fields bind correctly
**Dependencies:** Step 2
**Estimated:** 1 hour

### Step 4: Add ADF ci-native Validation
**Files:** `crates/terraphim_orchestrator/conf.d/skills-cli-ci-native.toml`, `scripts/skill-installer-ci-native.sh`
**Description:** Add or configure an ADF `ci-native` job that runs on PR/branch updates and validates shell syntax, optional shellcheck, ADF TOML parsing, dry-run matrix rendering, and repository ownership inventory. Do not add `.gitea/workflows/*`.
**Tests:** Trigger through ADF ci-native or run the same commands locally before commit.
**Dependencies:** Step 2 and Step 3
**Estimated:** 1 hour

### Step 5: Full Round Trip Validation
**Files:** ADF logs and Gitea issue evidence
**Description:** Run the agent against Claude Code, Codex, Pi, pi-rust, and Grok. The report must show `PASS`, `FAIL`, `SKIP`, or `UNSUPPORTED` for every sentinel skill and CLI cell, with evidence text.
**Tests:** Live ADF run, `grok --cwd <workspace> inspect --json`, and target-specific probes
**Dependencies:** Step 4
**Estimated:** 1 hour

### Deferred: Native Orchestrator Bridges
**Files:** `crates/terraphim_orchestrator/src/local_skills.rs`
**Description:** Extend `local_skills.rs` only if validation proves a native bridge is required. Preserve existing Claude/opencode behaviour.
**Tests:** Add targeted unit tests only for any bridge that is actually implemented.
**Dependencies:** Step 5 findings
**Estimated:** 30 min

## ADF Agent Definition

```toml
# conf.d/skills-installer.toml
# ADF Agent: installs and validates Terraphim skills across all agent CLIs

[[agents]]
name = "skills-installer"
layer = "Safety"
cli_tool = "/bin/bash"
schedule = "0 0 * * 1"
capabilities = ["skill-management", "multi-cli", "validation"]
max_cpu_seconds = 900
gitea_issue = 9
task = """You are the skills-installer agent. Your job is to ensure all Terraphim
skills are installed and validated across all agent CLIs.

## Step 1 -- Install skills
Run:
  tsm --agent claude-code install <skill>
  tsm --agent codex install <skill>
  tsm --agent pi-rust install <skill>
  tsm --install-dir "$HOME/.grok/skills" install <skill>

## Step 2 -- Validate skills
Run:
  tsm --agent claude-code verify <skill>
  tsm --agent codex verify <skill>
  tsm --agent pi-rust verify <skill>
  tsm --install-dir "$HOME/.grok/skills" verify <skill>
  grok --cwd <workspace> inspect --json

## Step 3 -- Render status matrix
Run:
  scripts/skill-installer-validation.sh

This output must show every skill as a row and every CLI as a column with green PASS or red FAIL status text.

## Step 4 -- Report results
ADF should capture stdout and post it through the built-in `gitea_issue` field.

## Step 5 -- Handle failures
If any CLI fails validation:
1. Investigate the cause (missing directory, wrong format, etc.)
2. Attempt remediation
3. If unfixable, flag as a Gitea issue

## Domain Knowledge Search
When you need context, use:
  terraphim-agent search "skill installation CLI bridge" --role "Mneme Knowledge"
"""

```

## Rollback Plan

1. **Revert TOML**: Remove `conf.d/skills-installer.toml` -- ADF agent stops.
2. **Revert ci-native config**: Remove `conf.d/skills-cli-ci-native.toml` or the equivalent ADF ci-native registration.
3. **Revert scripts**: Remove `scripts/skill-installer-validation.sh` and `scripts/skill-installer-ci-native.sh`.
4. **Clean validation directories**: Remove only temporary validation workspaces created by the runner; do not remove existing user skill directories.

## Dependencies

### New Dependencies

| Crate | Version | Justification |
|-------|---------|---------------|
| None expected | N/A | Validation-only implementation should not require Rust dependency changes. |

### Dependency Updates

None.

## Performance Considerations

### Expected Performance

| Metric | Target | Measurement |
|--------|--------|-------------|
| Install time (all CLIs) | < 5s | CLI stopwatch |
| Validate time (all CLIs) | < 2s | CLI stopwatch |
| Status matrix render time | < 1s for 100 skills x 5 CLIs | CLI stopwatch |
| Memory | < 10MB | Process profiler |

No benchmarks needed -- this is a CLI tool, not a hot path.

## Open Items

| Item | Status | Owner |
|------|--------|-------|
| Grok ADF-host parity with local `~/.grok/skills` | Pending | ADF agent / human |
| Codex AGENTS.md convention confirmation | Pending | ADF agent / human |
| pi-rust vs opencode relationship | Pending | ADF agent / human |
| Dedicated `TSM_TOKEN` or pre-provisioned `~/.terraphim/token` for unattended validation | Pending | Human |
| ADF ci-native runner availability and exact config shape | Pending | ADF agent / human |

## Approval

- [ ] Technical review complete
- [ ] Test strategy approved
- [ ] Step 0 spike results reviewed
- [ ] Human approval received

---

## Summary

| Phase | Steps | Est. Time |
|-------|-------|-----------|
| Spike | Step 0 | 1 hour |
| Runtime confirmation | Step 1 | 30 min |
| Script + ADF agent | Steps 2-3 | 2.5 hours |
| ADF ci-native | Step 4 | 1 hour |
| Full validation | Step 5 | 1 hour |
| **Total** | **6 steps + matrix rendering** | **~6 hours** |
