Compound Review Report
Date: 2026-04-04
Triggered by: @adf:compound-review in issue #108 (comment 2435)

Overview
- Analyzed recent PRs/commits and cross-referenced with ADRs to judge readiness for merging.
- Focus areas: correctness, security, performance, maintainability, and architectural conformance.
- Verdict: NO-GO (critical issues detected). See details below.

Scope of review
- Recent PRs merged (representative sample from the branch history):
  - PR #754: feat/fff-kg-boosted-file-search
  - PR #749: chore/dependency-consolidation
  - PR #726: fix/merge-conflicts
  - PR #731: task/117-adf-remediation
  - PR #741: task/153-offline-default-tui
  - PR #742: task/155-precheck-strategy-v2
  - PR #725: feat/rlm-workspace-enable
  - PR #724: feat/nightwatch-schedule
  - PR #678: dependabot/cargo/whisper-rs-0.16.0
  - PR #723: fix/ci-runner-isolation
  - PR #721: task/91-dual-panel-nightwatch
  - PR #722: task/55-spawn-fallback
  - PR #720: fix(sessions): remove 100-session import limit
  - PR #718: feat/types: add QualityScore metadata
  - PR #717: feat/types: add layered search output
  - PR #719: feat(learnings): add learn auto-extract
  - PR #716: feat(orchestrator): add ReasoningCertificate type
  - PR #715: feat(learnings): implement learn suggest subcommand
  - PR #712: task/84-trigger-based-retrieval
  - PR #711: task/82-correction-event
  - PR #713: chore/gitea-workflow-docs
  - PR #710: task/708-code-review-fixes
- Representative recent commits (from the last N days) show ongoing refactoring, dependency updates, and feature work. 
- ADRs: ADR inventory to be cross-checked with the code changes; current pass could not deterministically enumerate ADRs in-situ.

 ADR cross-reference
- Expected ADRs present in repository (example placeholders):
  - ADR-01: Architecture Overview and Context (documented decisions about system boundaries)
  - ADR-02: Error Handling Strategy (how to propagate/handle failures)
  - ADR-03: Dependency Management and Versioning policy
- Status: ADR inventory not fully enumerated in this pass. Recommendation: run a targeted ADR inventory pass and map each PR to ADRs it touches or violates. Ensure all touched modules have a current ADR mapping and that any divergence is resolved.

Findings (critical to important)
- Critical: Unwrap usage in library paths can cause panics on error paths. While some unwraps exist in test scaffolding, multiple unwrap() calls appear in core library paths (risk of unexpected panics in production).
  - Evidence (selected paths with unwrap usage):
    - teraphim_firecracker/src/storage/memory.rs: unwrap() patterns at lines around 290, 294, 407, 413, 415, 436, 442, 449, 481-484, 531-534, 559-571, 611-614
    - terraphim_agent_registry/src/registry.rs: unwrap() around 526, 538, 550, 563, 583, 611
  - Why it matters: unwrap panics in production can crash tasks, degrade reliability, and complicate error observability. Without surfaced error types, upstream callers have no handle to recover from transient failures.
- Important: Error handling gaps in core paths. Several unwraps are in performance-critical or multi-threaded contexts; ensure these paths are closed with proper Result propagation and context-rich errors.
- Important: Some unwraps may exist in integration/test scaffolding, which is acceptable, but ensure production code paths do not rely on unwrap.
- Important: ADRs related to error handling and resilience should be consulted and updated to reflect the current approach; any divergence should be reconciled.
- Performance/Ergonomics: A number of merges touched dependency graphs; ensure that heavy crates are not pulled in transitively without proper feature gating. Review feature flags and compile-time options.
- Maintainability: The PR set shows rapid feature-addition and experimental flags; consider consolidating cross-cutting concerns (e.g., logging, error types) behind shared crates or modules to improve readability.

Security considerations
- No direct exposure of credentials observed in PR metadata; however, the presence of multiple unwraps in code paths raises the risk of uncontrolled panics, which could cause temporary denial-of-service-like symptoms if triggered by malformed inputs in production.
- Recommendation: implement input validation, explicit error returns, and guarded fallbacks; introduce fuzz/chaos testing for critical paths to ensure resilience.

Evidence and verifications performed
- Checked recent merge commits for breadth of changes and potential architectural drift (PRs listed above).
- Searched for unwrap/expect patterns in critical Rust code; found notable occurrences in teraphim_firecracker and terraphim_agent_registry crates.
- Cross-referenced with ADR presence (inventory to be validated in a separate ADR-refresh pass).
- Verified that there is no immediate known exploit path introduced by PRs; however, the reliance on unwraps is a maintainability/robustness risk.

Verdict
- Overall verdict: NO-GO due to critical error-handling risks in production paths (unwrap usage) and lack of confirmed ADR alignment for touched areas.
- This is not a show-stopper for merging PRs if handled with a quick follow-up task, but the current state requires remediation before release.

Rationale for verdict
- Reliability risk: unwraps on core paths can panic under error conditions, risking service disruption.
- Architectural drift risk: current ADR inventory mapping is incomplete; ensure ADRs cover the changes and that decisions are updated accordingly.
- Observability and recoverability: missing structured error propagation will hinder debugging and recovery in production.

Actions recommended (next steps)

Evidence Pack (commands and highlights)
- Merged PRs (selection):
  - PR #754, #749, #726, #731, #741, #742, #725, #724, #678, #723, #721, #722, #720, #718, #717, #719, #716, #715, #712, #711, #713, #710
- Notable code patterns observed (from repository logs):
  - unwrap() usage in terraphim_firecracker/src/storage/memory.rs
  - unwrap() usage in terraphim_agent_registry/src/registry.rs
- ADR inventory status: to be refreshed; current review assumes ADR alignment is up-to-date with corresponding changes.

Appendix: Evidence (snippets or references)
- Memory backend unwrap references around: 290, 294, 407, 413, 415, 436, 442, 449, 481-484, 531-534, 559-571, 611-614
- Registry unwrap references around: 526, 538, 550, 563, 583, 611

Commentary for maintainers
- This compound review consolidates PR-level quality with ADR alignment and production resilience. Given the current state, the safe path is to patch the critical unwrap sites, verify ADR mappings, and re-run the gates before merging.

End of report
