# Document Quality Evaluation Report

## Metadata

- **Document**: `.docs/design/adf-otp-orchestrator-implementation-plan-2026-05-21.md`
- **Type**: Phase 2 Design
- **Evaluated**: 2026-05-21 14:25 BST
- **Evaluator**: disciplined-quality-evaluation

## Decision: GO

**Average Score**: 4.5 / 5.0
**Weighted Average**: 4.6 / 5.0
**Blocking Dimensions**: None

## Dimension Scores

| Dimension | Score | Status |
|-----------|-------|--------|
| Syntactic | 4/5 | Pass |
| Semantic | 5/5 | Pass |
| Pragmatic | 5/5 | Pass |
| Social | 4/5 | Pass |
| Physical | 5/5 | Pass |
| Empirical | 4/5 | Pass |

## Detailed Findings

### Syntactic Quality (4/5)

**Strengths:**

- Sections 1-9 use consistent terms: `AgentRegistry`, `RunSupervisor`, `BuildDispatchService`, `ProjectController`, and `ProviderHealthActor`.
- Section 3 clearly defines boundaries and explicitly resolves the Phase 1 ambiguity between ADF `RunSupervisor` and `terraphim_agent_supervisor`.
- Section 9 maps the user's explicit constraints back to concrete implementation slices.
- The updated plan consistently treats TLA+ as a formal validation lane, not as an implementation component.

**Weaknesses:**

- Section 3 introduces `FleetKernel`, but Section 5 starts with `AgentRegistry` and does not schedule a concrete `FleetKernel` implementation step until indirectly through later actor/controller work.

**Suggested Revisions:**

- [ ] In Phase 3 or a revision, clarify whether `FleetKernel` is a named module in the first implementation slice or an architectural label for the existing `AgentOrchestrator` shell.

### Semantic Quality (5/5)

**Strengths:**

- The design accurately reflects current code: `OrchestratorConfig::from_file`, `merge_project_sources`, project-scoped duplicate validation, build-runner push/PR fan-out paths, Symphony state, OTP supervisor primitives, and spawner usage.
- The design correctly identifies the global `active_agents.contains_key("build-runner")` risk and treats project-scoped identity as a first-order invariant.
- The `adf/build` flow preserves current push and PR semantics, including `ADF_PUSH_*` environment injection and pending/terminal status handling.
- The new formal validation boundary accurately leverages the existing TLA+ research by modelling message-passing state transitions rather than Rust `await` points.
- The bounded models target the right risk areas for this refactor: registry merge, project-scoped lookup, `adf/build` lifecycle, retry bounds, terminal status, and provider probe isolation.

**Weaknesses:**

- Section 7 still mentions build-runner workspace `cargo clippy --workspace` failures from PR #1782, but does not yet define whether Phase 3 should diagnose that separate build gate before or after the registry/build-dispatch slice.

**Suggested Revisions:**

- [ ] Add a Phase 3 pre-flight decision: either fix the existing `adf/build` workspace gate first, or explicitly proceed with lifecycle refactoring while preserving the visible failure.

### Pragmatic Quality (5/5)

**Strengths:**

- Section 5 is directly implementable, with 14 ordered, reversible steps and deployability/feature-flag guidance for each step.
- Section 6 maps every acceptance criterion to test type and test location.
- Section 4 provides file-level changes with before/after responsibilities and dependencies.
- The first Phase 3 slice is small enough to implement without rewriting the daemon and now starts with formal model pre-flight checks.

**Weaknesses:**

- None blocking.

**Suggested Revisions:**

- [ ] Optional: split the 14-step sequence into milestones if implementation ownership will be shared across agents.

### Social Quality (4/5)

**Strengths:**

- Section 8 presents explicit recommended decisions rather than leaving major choices ambiguous.
- The plan states that this is an in-place strangler refactor, avoiding the likely stakeholder misunderstanding that ADF will be replaced immediately.
- The user-requested requirements are explicitly traced in Section 9.

**Weaknesses:**

- The phrase "OTP orchestrator" could still be interpreted by different readers as either a new daemon, a module inside `terraphim_orchestrator`, or direct adoption of the existing OTP crate.

**Suggested Revisions:**

- [ ] Add a short glossary entry defining "OTP orchestrator" as the set of `AgentRegistry`, `ProjectController`, `RunSupervisor`, and supervised actors inside the existing daemon during the first migration phase.

### Physical Quality (5/5)

**Strengths:**

- All required Phase 2 sections are present, with one useful extra Section 9 for Phase 3 starting point.
- Tables are used effectively for invariants, acceptance criteria, boundaries, file changes, tests, risks, and decisions.
- The ASCII component diagram is simple and enough to communicate system boundaries without requiring external tooling.

**Weaknesses:**

- None blocking.

**Suggested Revisions:**

- [ ] Optional: add links from each file path in Section 4 to existing source locations if the document is later published in rendered docs.

### Empirical Quality (4/5)

**Strengths:**

- The document is readable despite broad scope because it chunks concepts into tables and ordered slices.
- Section 9 provides a concise implementation starting point after the detailed plan.
- Complex migration risks are kept concrete and tied to current code behaviour.

**Weaknesses:**

- The 15-step implementation sequence is long; a reader starting Phase 3 may need to re-read Section 5 and Section 9 together to determine the exact first task boundary.

**Suggested Revisions:**

- [ ] Before Phase 3 execution, copy Section 9 into a short task brief or Gitea issue checklist for the first vertical slice.

## Revision Checklist

Priority order based on impact:

- [ ] Clarify whether `FleetKernel` is a first-slice module or an architectural label for the existing shell.
- [ ] Add a pre-flight decision about the existing PR #1782 `adf/build` workspace gate failure.
- [ ] Define "OTP orchestrator" in a one-paragraph glossary before implementation begins.
- [ ] Optionally split Section 5 into implementation milestones if multiple agents will work in parallel.
- [ ] Confirm TLA+/TLC tooling availability in CI before making `tla_validation_tests` blocking.

## Next Steps

Document approved for Phase 3 planning/implementation. Proceed with human approval, then start the first vertical slice from Section 9: TLA+ pre-flight models, `AgentRegistry`, registry-backed `build-runner` lookup, project-scoped active key, and `BuildDispatchService` extraction.

## JSON Summary

```json
{
  "metadata": {
    "document_path": ".docs/design/adf-otp-orchestrator-implementation-plan-2026-05-21.md",
    "document_type": "phase2-design",
    "evaluated_at": "2026-05-21T14:25:00+01:00",
    "evaluator": "disciplined-quality-evaluation"
  },
  "dimensions": {
    "syntactic": {"score": 4},
    "semantic": {"score": 5},
    "pragmatic": {"score": 5},
    "social": {"score": 4},
    "physical": {"score": 5},
    "empirical": {"score": 4}
  },
  "decision": {
    "verdict": "GO",
    "blocking_dimensions": [],
    "average_score": 4.5,
    "weighted_average": 4.6
  }
}
```
