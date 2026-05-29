---
stage: design-proposal
issue: 1882
slot: 1
model: opus
provider: claude
timestamp: 2026-05-29T12:13:59+01:00
---

## Problem Statement

This is a **design-for-rescope**, not a design-for-feature: the k=3 research phase for #1882 converged unanimously (all three slots) on the classification `needs-rescope`, and the quality gate passed at 4.5/5. The design's job is therefore to specify *how the rescope is executed* plus the single residual code-level fix -- **not** to re-derive the conceptual architecture, which already exists and is approved in `docs/research/research-adf-real-issue-processing-1882.md` (Status: Approved) and drafted in `docs/research/design-adf-real-issue-processing-1882.md` (Status: Draft).

The concrete situation, verified by direct repository inspection:

1. **#1882 is an empty-bodied umbrella.** The Gitea issue body has length 0 and (per its timeline: 4 label + 3 commit_ref + 1 comment events, no body edit) was never populated. The requirement is not self-contained in the tracker.
2. **One live config/implementation contradiction exists.** `.terraphim/boosting.toml:53-55` declares `[verification.lsp_diagnostics]` with `on_error = "block"`, but `crates/terraphim_lsp/src/lib.rs` is a literal `// placeholder` (verified: the file body after the doc comment is the single line `// placeholder`). The config wires a blocking quality gate that cannot execute.
3. **The remaining named work is heterogeneous and not one acceptance unit.** Verification scripts (`scripts/drift_check.sh`, `scripts/kg_verify.sh`, `scripts/lsp_verify.sh` -- confirmed absent via glob), prompt templates (`.terraphim/prompts/` -- confirmed absent), CI wiring, `terraphim_lsp` implementation, and reusable external-project scaffolding are each separable streams with distinct acceptance criteria.
4. **Labels contradict.** Both `status/research` and `status/in-progress` are present; the decision state is ambiguous.

The design solves: turn the empty umbrella into a state-aware tracker, eliminate the one live contradiction with the cheapest correct change, and decompose the deferred work into independently completable child issues -- so no further "research" spend is wasted on a direction that is already approved.

## Architecture

### What this design does NOT touch (boundary, stated hard)

The approved/draft docs already define `adf-issue-stage`, the `zdp-research.toml`/`zdp-design.toml` flow definitions, `boosting.toml`'s structure, and `contracts/api.toml`. These exist on disk and are functional (the k=3 flow that produced these proposals is first-hand proof). This design adds **no new architecture** to that substrate. It re-shapes the *issue*, not the *system*.

### Vital Few (5/25 Rule)

Considered capabilities, top 5 circled as IN scope; the rest are the **Avoid At All Cost** list.

**IN scope (the vital few):**
1. Resolve the LSP drift in `.terraphim/boosting.toml` (one-line config diff).
2. Populate #1882 body as a state-aware umbrella linking approved docs and listing complete vs pending work.
3. Resolve the contradictory labels (drop `status/research`, keep `status/in-progress`).
4. Create child issues with explicit acceptance criteria for each deferred stream.
5. Cross-link umbrella <-> children and reference the approved research/design docs.

**Avoid At All Cost (dangerous distractions for THIS unit):**
- Implementing `terraphim_lsp` now -- it is a multi-day crate, not a rescope action. (-> child issue)
- Writing the verification scripts now -- each needs its own design/test. (-> child issue)
- Building `.terraphim/prompts/` templates now. (-> child issue)
- CI workflow wiring now. (-> child issue)
- Reusable external-project scaffolding (the "template for other repos" interpretation). (-> child issue)
- Regenerating research or design artefacts for #1882 itself (explicitly forbidden by the research evaluation, recommendation 5).

### Rescope structure (issue graph)

```
#1882  type/initiative  status/in-progress   <- state-aware UMBRELLA (body populated)
  |  links: research-adf-real-issue-processing-1882.md (Approved)
  |         design-adf-real-issue-processing-1882.md   (Draft)
  |  DONE:  adf-issue-stage, zdp-research/design/full flows, boosting.toml, contracts/api.toml,
  |         k=3 flow execution proof (.docs/adf/1882/*)
  |  PENDING (children):
  |
  +-- (A) LSP drift resolution        [IN THIS UNIT -- closes with this work]
  +-- (B) terraphim_lsp diagnostics   child issue, P3, blocks re-enabling block-mode
  +-- (C) verification scripts        child issue, P2  (drift_check, kg_verify, lsp_verify*)
  +-- (D) .terraphim/prompts/ templates child issue, P2
  +-- (E) CI workflow wiring          child issue, P2  (depends on C)
  +-- (F) reusable project template   child issue, P3  (the "external repo" interpretation)
```
`*lsp_verify` depends on (B).

### The one code change (data flow)

```
.terraphim/boosting.toml  [verification.lsp_diagnostics]
   on_error = "block"   --(downgrade)-->   on_error = "report"
        |                                        |
   promises a blocking gate              records diagnostics, never blocks
   over placeholder crate                 -> contradiction eliminated
                                          -> re-enable gated on child issue (B)
```

### Key Design Decisions

| Decision | Rationale | Alternatives Rejected |
|----------|-----------|----------------------|
| Pull the LSP drift fix INTO this unit (in-line) | One-line diff; costs less than filing+tracking a child issue; removes a live contradiction immediately | Defer to child issue (adds tracking overhead for a 1-line change) |
| Downgrade to `report`, do not implement LSP | `terraphim_lsp` is a placeholder; `report` is honest about current capability and unblocks the gate | Implement `terraphim_lsp` now (multi-day, out of rescope); delete the rule (loses intent + future re-enable point) |
| Keep `status/in-progress`, drop `status/research` | Active branch `task/1875-adf-ctl-local-direct-dispatch`, commits, and live flow execution show implementation in progress, not research | Keep `status/research` (research is approved/done); drop both (loses state signal) |
| State-aware umbrella + children, not a rewrite of #1882 into a single task | The work is genuinely heterogeneous; forcing one acceptance unit reproduces the original problem | Close #1882 and open one mega-issue (same un-trackable scope); leave #1882 empty (root cause untouched) |

### Eliminated Options (Essentialism)

| Option Rejected | Why Rejected | Risk of Including |
|-----------------|--------------|-------------------|
| Implement `terraphim_lsp` in this unit | Not in the vital few; a crate, not a rescope | Days of work; couples rescope to a hard dependency; defeats "make the tracker honest cheaply" |
| Author all verification scripts here | Each has its own contract, test, and failure modes | Scope creep recreates the un-reviewable umbrella |
| Build the reusable external-project template | Distinct interpretation; no consumer exists yet | Speculative; YAGNI; large surface for zero current demand |
| Regenerate research/design for #1882 | Approved docs are sufficient; evaluation forbids it | Duplicate effort, gate churn |

### Simplicity Check

**What if this could be easy?** It is: the only file edit is two lines in one TOML; everything else is Gitea tracker hygiene (body text + labels + child issues). No new code, no new files, no migration.

**Senior Engineer Test:** Would a senior engineer call this overcomplicated? No -- a senior engineer would say "the research already concluded; just fix the one drift and split the tracker." That is exactly this.

**Nothing Speculative Checklist:**
- [x] No features the user didn't request (rescope was the convergent research finding)
- [x] No abstractions "in case we need them later"
- [x] No flexibility "just in case"
- [x] No error handling for scenarios that cannot occur (config edit is declarative)
- [x] No premature optimization

## Implementation Plan

### Step 1 -- Resolve LSP config drift (the only code change)
**File:** `.terraphim/boosting.toml`
**Change:** Line 55 `on_error = "block"` -> `on_error = "report"`; add a one-line comment above `[verification.lsp_diagnostics]` pointing at the placeholder crate and child issue (B).
**Verify:** `grep -n 'on_error' .terraphim/boosting.toml` shows `report` at line 55; the file still parses as valid TOML (`adf-issue-stage`/flow loaders read it without error).
**Estimated:** 5 minutes.

### Step 2 -- Populate #1882 body (state-aware umbrella)
**Action:** `tea` / Gitea API: set the issue body to a summary that (a) states the initiative direction in one paragraph, (b) links the Approved research and Draft design docs, (c) lists DONE items (flows, stage script, boosting, contracts, k=3 proof), (d) lists PENDING items as links to child issues created in Step 4.
**Verify:** `GET /repos/terraphim/terraphim-ai/issues/1882` returns `body` length > 0 containing both doc links.
**Estimated:** 20 minutes.

### Step 3 -- Resolve label contradiction
**Action:** Remove `status/research` from #1882; retain `status/in-progress`, `priority/P2-medium`, `type/initiative`. Optionally add an assignee/milestone reflecting the active branch.
**Verify:** issue labels contain exactly one `status/*` label (`status/in-progress`).
**Estimated:** 5 minutes.

### Step 4 -- Create child issues with acceptance criteria
**Action:** Open child issues (B)-(F) below, each with title, body stating scope, and explicit AC. Set `gtr add-dep` so #1882 is blocked-by each child (or children reference #1882 as parent), and `lsp_verify` (within C) notes dependency on (B).

| ID | Title | Priority | Acceptance Criteria (summary) |
|----|-------|----------|-------------------------------|
| B | `terraphim_lsp`: implement KG-markdown diagnostics | P3 | `crates/terraphim_lsp/src/lib.rs` provides diagnostics over KG markdown; on completion, `boosting.toml` LSP rule may return to `on_error = "block"` |
| C | ADF verification scripts (drift_check, kg_verify, lsp_verify) | P2 | `scripts/drift_check.sh` (terraphim_grep over `.terraphim/contracts`), `scripts/kg_verify.sh` (`terraphim-agent validate`), `scripts/lsp_verify.sh` (gated on B) each exist, are executable, exit 0 on clean input and non-zero on a seeded violation, and have a smoke test |
| D | ADF prompt templates under `.terraphim/prompts/` | P2 | Per-phase prompt templates (research/design/implementation/review) exist and are referenced by `adf-issue-stage` or the flow TOMLs |
| E | CI workflow: drift_check + kg_verify + tests gate | P2 | A workflow runs the Step-C scripts and `cargo test` on PRs; merge blocked on failure; depends on C |
| F | Reusable project-template scaffolding | P3 | A documented scaffold (the external-repo interpretation) distinct from terraphim-ai's own `.terraphim/`, with a generation mechanism and a smoke-generated sample |

**Verify:** five child issues exist; each has non-empty AC; dependency edges recorded.
**Estimated:** 45 minutes.

### Step 5 -- Cross-link and record decision
**Action:** Comment on #1882 summarising the rescope decision and linking children; reference this design proposal path. Comment on each child linking back to #1882 and the approved docs.
**Verify:** #1882 comment references all five children; each child references #1882.
**Estimated:** 15 minutes.

## File Changes

### Modified Files
| File | Changes |
|------|---------|
| `.terraphim/boosting.toml` | Line 55: `on_error = "block"` -> `on_error = "report"`; add explanatory comment referencing the `terraphim_lsp` placeholder and child issue (B) |

### New Files
None. (This is a rescope; deliverables B-F are deferred to child issues that each carry their own file changes.)

### Deleted Files
None.

### Non-file (Gitea tracker) changes
| Object | Change |
|--------|--------|
| Issue #1882 body | Empty -> state-aware umbrella summary with doc + child links |
| Issue #1882 labels | Remove `status/research`; keep `status/in-progress` |
| Child issues B-F | Created with titles, scope bodies, AC, and dependency edges |

## Risks and Mitigations

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Downgrading LSP to `report` silently weakens a future quality gate | Medium | Low | Child issue (B) is the explicit re-enable point; comment in `boosting.toml` records the intent so it is not lost |
| Child-issue proliferation recreates an un-trackable umbrella | Low | Medium | Exactly 5 children, each with concrete AC and a clear owner-stream; dependency graph keeps order explicit |
| Editing `boosting.toml` breaks flow/stage TOML parsing | Low | High | Change is a single string value; verify by re-running a loader (`adf-issue-stage` or flow parse) before closing Step 1 |
| Rescope is read as "abandon the initiative" | Low | Medium | Umbrella body explicitly marks DONE work and keeps `type/initiative`; direction is preserved, only decomposed |
| As slot 1 of k=3, this proposal diverges from slots 2/3 on the in-line LSP fix | Medium | Low | Decision is stated hard with rationale so the downstream judge can compare cleanly; the alternative (defer to child) is named and rejected with reason |

## Acceptance Criteria

1. **LSP drift eliminated:** `.terraphim/boosting.toml` line 55 reads `on_error = "report"`; the file parses as valid TOML and is loaded without error by the existing stage/flow tooling.
2. **Umbrella is state-aware:** `GET /repos/terraphim/terraphim-ai/issues/1882` returns a `body` of length > 0 that links both `docs/research/research-adf-real-issue-processing-1882.md` and `docs/research/design-adf-real-issue-processing-1882.md`, and lists DONE vs PENDING work.
3. **Label state unambiguous:** #1882 carries exactly one `status/*` label, `status/in-progress`; `status/research` removed.
4. **Children exist with AC:** five child issues (B-F) are open, each with a non-empty acceptance-criteria section; dependency edges recorded (`lsp_verify` in C depends on B; E depends on C).
5. **Cross-linked:** #1882 comment references all five children; each child references #1882 and the approved research/design docs.
6. **No research/design regeneration:** no new `research-*.md` or `design-*.md` is produced under `docs/research/` for #1882 itself (the approved docs remain authoritative).
