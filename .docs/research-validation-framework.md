# Research Document: Validation Framework for terraphim-ai

**Status**: Draft
**Author**: Codex CLI (GPT-5)
**Date**: 2026-01-17
**Reviewers**: TBD
**Owner Approval**: Alex Mikhalev (2026-01-17)

## Executive Summary

PR #413 introduces a new **release validation framework** (`crates/terraphim_validation`) with orchestrated validation, performance benchmarking, TUI/desktop UI harnesses, server API validation, and extensive documentation. Separately, terraphim-ai already has **runtime validation hooks** (CLI command hooks, VM execution hooks, and Claude Code pre/post tool hooks). The current hook implementation now includes a **two‑stage guard + replacement** flow (guarding `--no-verify/-n` on git commit/push, then knowledge‑graph replacement). The validation story is therefore split across release validation and runtime validation, with gaps in unification and coverage (notably pre/post LLM hooks in runtime paths).

This research maps both tracks, identifies overlap and gaps, and sets a foundation for a unified validation plan that leverages PR #413 without duplicating or regressing existing runtime safeguards.

## Essential Questions Check

| Question | Answer | Evidence |
|----------|--------|----------|
| Energizing? | Yes | Validation and safety are core to trust and quality. |
| Leverages strengths? | Yes | Existing hooks, KG replacement, and new release framework are strong assets. |
| Meets real need? | Yes | Requirements call for 4‑layer validation and robust release checks. |

**Proceed**: Yes (3/3).

## Problem Statement

### Description
Validation is currently fragmented:
- PR #413 adds a **release validation system** (packaging, install, security, performance).
- Runtime validation remains distributed across **CLI hooks**, **VM execution hooks**, and **Claude Code hooks**.
- Pre/post LLM validation hooks exist in VM execution but are not wired into LLM generation paths.

A proper plan must clarify scope, integrate PR #413 cleanly, and ensure runtime validation coverage without duplicating responsibilities.

### Impact
- Risk of confusing “validation” meaning (release vs runtime).
- Potential duplication of validation logic and inconsistent enforcement.
- Missed coverage for LLM output validation in runtime paths.

### Success Criteria
- PR #413 release validation framework integrated and operational.
- Runtime validation is documented and wired for pre/post LLM/tool stages.
- Clear boundaries and configuration for each validation track.

## Current State Analysis

### Existing Runtime Validation (in-repo)
- **CLI Command Hooks**: `terraphim_agent` `CommandHook` + `HookManager`.
- **VM Execution Hooks**: `terraphim_multi_agent` pre/post tool hooks; pre/post LLM hooks exist but are not invoked around LLM calls.
- **Claude Code Hook Integration**: `terraphim-agent hook` handles `pre-tool-use`, `post-tool-use`, `pre-commit`, `prepare-commit-msg` with knowledge‑graph replacement and connectivity validation.
- **Knowledge‑Graph Replacement**: `terraphim_hooks::ReplacementService`.

### Current Hook Implementation (User Context)
The global Claude hook `~/.claude/hooks/pre_tool_use.sh` now has **two‑stage processing**:
1. **Guard Stage (New)**
   - Extract command from JSON input
   - Strip quoted strings to avoid false positives
   - Check for `--no-verify` or `-n` flags in `git commit/push`
   - If found: return deny decision and exit
2. **Replacement Stage (Existing)**
   - `cd ~/.config/terraphim`
   - Run `terraphim-agent hook` for text replacement
   - Return modified JSON or original

### PR #413: Release Validation Framework
**PR #413 (Open)** adds:
- New crate: `crates/terraphim_validation`
- Orchestrator with config (`validation-config.toml`), categories, artifact manager
- Performance benchmarking, server API tests, TUI/desktop UI testing harnesses
- New CI workflow (`.github/workflows/performance-benchmarking.yml`)
- Extensive design and functional validation docs under `.docs/`

### Code Locations (Key)
| Component | Location | Purpose |
|-----------|----------|---------|
| CLI Hook Handler | `crates/terraphim_agent/src/main.rs` | Pre/post tool and commit hooks |
| Command Hooks | `crates/terraphim_agent/src/commands/mod.rs` | Pre/post command hooks |
| VM Hooks | `crates/terraphim_multi_agent/src/vm_execution/hooks.rs` | Runtime pre/post tool/LLM hooks |
| LLM Calls | `crates/terraphim_multi_agent/src/agent.rs` | LLM generate (no hooks) |
| Replacement | `crates/terraphim_hooks/src/replacement.rs` | KG replacement |
| Release Validation | `crates/terraphim_validation/*` (PR #413) | Release validation framework |
| Release Config | `crates/terraphim_validation/config/validation-config.toml` (PR #413) | Validation configuration |

### Data Flow (High Level)
**Runtime validation:**
- Claude Code -> `pre_tool_use.sh` (Guard -> Replacement) -> tool execution
- `terraphim_agent` -> CommandExecutor -> pre/post hooks
- `terraphim_multi_agent` -> VM client -> pre/post tool hooks
- `terraphim_multi_agent` -> LLM generate (currently no hooks)

**Release validation (PR #413):**
- `ValidationSystem` -> `ValidationOrchestrator` -> download/install/functionality/security/performance

## Constraints

### Technical Constraints
- Rust workspace with multiple hook abstractions.
- Tests must avoid mocks.
- Hook execution must be low‑latency.

### Business Constraints
- Validation should not block normal workflows.
- Release validation must be automatable in CI.

### Non‑Functional Requirements
| Requirement | Target | Current |
|-------------|--------|---------|
| Runtime validation coverage | 4 layers (pre/post LLM + tool) | Partial |
| Release validation coverage | multi‑platform + security + perf | PR #413 scope |
| Fail behavior | configurable fail‑open/closed | fragmented |

## Vital Few (Essentialism)

### Essential Constraints (Max 3)
| Constraint | Why It's Vital | Evidence |
|------------|----------------|----------|
| Integrate PR #413 release validation | Adds missing release QA | PR #413 scope |
| Wire pre/post LLM hooks | Prevent unchecked LLM output | Existing unused hooks |
| Keep guard stage for git bypass | Protects safety invariants | New hook change |

### Eliminated from Scope
| Eliminated Item | Why Eliminated |
|-----------------|----------------|
| Full LSP auto‑fix pipeline | Not required for validation framework MVP |
| ML anomaly detection | Over‑engineering for Phase 1 |
| Telemetry backend | Nice‑to‑have only |

## Dependencies

### Internal Dependencies
| Dependency | Impact | Risk |
|------------|--------|------|
| terraphim_validation (PR #413) | Core release validation | Medium |
| terraphim_agent | CLI hooks | Medium |
| terraphim_multi_agent | Runtime LLM/VM validation | Medium |
| terraphim_hooks | KG replacement | Low |

### External Dependencies
| Dependency | Version | Risk | Alternative |
|------------|---------|------|-------------|
| config, serde, regex | workspace | Low | n/a |
| docker, gh | tooling | Medium | local alternatives |

## Risks and Unknowns

### Known Risks
| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Validation scope confusion | High | Medium | Document release vs runtime boundaries |
| Performance regressions | Medium | Medium | Benchmarks + minimal default hooks |
| Over‑blocking workflows | Medium | High | Fail‑open defaults for dev |

### Open Questions
1. Should release validation and runtime validation share a common API/config surface?
2. Where should validation config live for runtime hooks vs release validation?
3. Which PR #413 changes are required vs optional for current roadmap?

### Assumptions
1. PR #413 will be merged or cherry‑picked into main.
2. Claude Code hook integration remains the primary runtime guard surface.

## Research Findings

### Key Insights
1. PR #413 provides a solid release validation foundation but does not address runtime validation.
2. Runtime validation hooks exist but are fragmented and partially unwired (LLM).
3. The new guard stage is a critical safety feature and should be preserved and documented.

### Relevant Prior Art
- PR #413 design docs for release validation.
- Existing VM hook system with block/modify/ask decisions.

### Technical Spikes Needed
| Spike | Purpose | Estimated Effort |
|-------|---------|------------------|
| PR #413 integration review | Confirm file changes and conflicts | 0.5–1 day |
| LLM hook wiring prototype | Pre/post LLM validation | 0.5–1 day |

## Recommendations

### Proceed/No‑Proceed
Proceed with a two‑track validation plan: **Release validation** (PR #413) + **Runtime validation** (hooks/LLM/tool).

### Scope Recommendations
- Integrate `terraphim_validation` as release QA framework.
- Wire pre/post LLM hooks in runtime paths.
- Document and test guard+replacement flow.

### Risk Mitigation Recommendations
- Configurable fail‑open for dev; fail‑closed for CI/release.
- Keep hook logic minimal and deterministic.

### Configuration Decision (Proposed)
To avoid coupling release and runtime validation, keep **runtime validation config** separate from PR #413’s release config:
- Runtime config path: `~/.config/terraphim/runtime-validation.toml`
- Environment overrides: `TERRAPHIM_RUNTIME_VALIDATION_*`
- Release validation config remains in `crates/terraphim_validation/config/validation-config.toml`

## Next Steps

If approved:
1. Update implementation plan to align with PR #413 file layout.
2. Define integration steps for runtime validation hooks.

## Appendix

### Reference Materials
- PR #413 summary (GitHub)
- `.docs/code_assistant_requirements.md`
- `crates/terraphim_multi_agent/src/vm_execution/hooks.rs`
- `crates/terraphim_agent/src/main.rs`
- `crates/terraphim_hooks/src/replacement.rs`
