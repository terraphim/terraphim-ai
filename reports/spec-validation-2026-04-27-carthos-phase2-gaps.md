# Spec Validation Report — Phase 2 Design Brief Gaps

**Date:** 2026-04-27 17:24 CEST
**Agent:** Carthos (Domain Architect / spec-validator)
**Scope:** Three reported gaps — subscription validation handshake, PATCH 204 semantics, `list_by_prefix` pattern
**Mandate:** Confirm capture in Phase 2 design brief before implementation begins
**Verdict:** FAIL — none of the three gaps appear in any active Phase 2 design brief

---

## Executive Summary

A systematic search across all design documents (`.docs/design-*.md`, `plans/*.md`),
spec files (`docs/specifications/`), issue tracker comments, and codebase found
**zero occurrences** of the three gap names. They are not captured. Each gap maps
to a real, currently unspecified boundary in the ADF orchestrator's Phase 2 work.
All three must be added to the relevant Phase 2 design brief before implementation.

---

## Search Evidence

Artefacts searched:

| Source | Files | Result |
|--------|-------|--------|
| `.docs/design-*.md` (34 files) | All design briefs | No match |
| `plans/` (6 files) | Active plans | No match |
| `docs/specifications/` | Spec files | No match |
| `reports/spec-validation-*.md` (8 files) | Previous validations | No match |
| Gitea issue #672 comments (28) | PR/issue comments | No match |
| Gitea repo-wide recent comments | Last 100 | No match |
| `crates/` Rust source | All `.rs` files | No match |

---

## Gap Analysis

### Gap 1: Subscription Validation Handshake

**Bounded context:** ADF Control-Plane Routing — `crates/terraphim_orchestrator/src/control_plane/routing.rs` (to be created per `.docs/design-adf-control-plane-routing.md` §4)

**What the gap is:**
The control-plane routing design (`.docs/design-adf-control-plane-routing.md`) specifies
"subscription-liveness-aware ranking" (Step 6) and states that "the primary operational
goal is choosing the most suitable currently-live subscription lane". However, the design
brief contains no specification of:
- What a subscription validation handshake looks like at runtime
- When it is triggered (startup, post-cooldown, first dispatch, periodic)
- What constitutes a valid response (HTTP 200, non-empty content, latency threshold)
- How handshake failure propagates (route exclusion, fallback, retry cadence)
- Whether handshake is per-provider, per-model, or per-agent

**Invariant at risk:** AC3 (unhealthy routes are never preferred) cannot be verified
without a defined liveness check mechanism. The current design relies on "real dispatch
outcomes" as the liveness signal but defines no bootstrap path for a provider that has
never been successfully dispatched in this orchestrator instance.

**Required addition to Phase 2 brief:**
> Sub-section: Subscription Validation Handshake Protocol
> Define trigger condition, request format, success/failure criteria, retry policy,
> and the mapping from handshake outcome to route health state.

---

### Gap 2: PATCH 204 Semantics

**Bounded context:** ADF Orchestrator → Gitea API integration — `crates/terraphim_orchestrator/src/lib.rs` (`post_pending_status` and any PATCH call sites)

**What the gap is:**
The Gitea API returns `204 No Content` on successful PATCH operations (issue edits,
label updates, commit status updates). The Phase 2 design brief for PR fan-out wiring
(issue #944 / PR #999) introduces `post_pending_status` which calls:

```
POST /repos/{owner}/{repo}/statuses/{sha}
```

POST returns `201 Created` with a body. However, any subsequent status transition
via PATCH must not attempt to deserialise a body — 204 carries none. The design brief
for #944 (and its successors #950–#955) contains **no specification** of how the
orchestrator handles 204 responses from Gitea PATCH endpoints. The P2 note in the
pr-reviewer verdict for PR #999 (`skip_deserializing` on `pr_dispatch_per_project`)
reveals that this class of serialisation-contract gap is already present; 204 semantics
is the HTTP-layer analogue.

**Invariant at risk:** Any PATCH call site that unwraps a response body on 204
will silently fail or panic. New Phase 2 agents (spec-validator, security-sentinel,
compliance-watchdog, test-guardian) each post verdicts via commit status API; if
a status context already exists and is being updated, the update path is PATCH + 204.

**Required addition to Phase 2 brief:**
> Sub-section: Gitea API Response Contract
> Define expected status codes per verb: POST→201, GET→200, PATCH→204, DELETE→204.
> Specify that all Gitea call sites must treat 204 as success without deserialising
> a response body. Include test case: updating an existing commit status returns 204.

---

### Gap 3: `list_by_prefix` Pattern

**Bounded context:** ADF Config Layer — `crates/terraphim_orchestrator/src/config.rs` (`pr_dispatch_per_project` and `agents_on_pr_open_for_project`)

**What the gap is:**
PR #999 introduced `pr_dispatch_per_project: HashMap<String, PrDispatchConfig>` keyed
by project name string (e.g. `"terraphim"`, `"odilo"`). The lookup is exact:
`pr_dispatch_per_project.get(project)`. This works for the current two-project fleet.

The design brief for #944 and its successors does not specify how project keys are
normalised, nor how the config system handles prefix-based lookups where a repo
(`terraphim/terraphim-ai`) needs to resolve against a project key (`terraphim`).
The three-tier cascade described in the pr-reviewer comment for PR #999 performs
exact-key lookup, with no defined behaviour for partial matches or namespace prefixes.

As additional projects are onboarded (a stated goal of the Phase 2–2e series),
the absence of a `list_by_prefix` contract creates two failure modes:
1. Dispatch silently falls through to the legacy default when a new project's key
   doesn't match exactly (key normalisation not specified)
2. A project named `terraphim-staging` could shadow `terraphim` or miss its dispatch
   config entirely depending on unspecified key comparison rules

**Required addition to Phase 2 brief:**
> Sub-section: Project Key Normalisation and Prefix Semantics
> Define the canonical project key format, key normalisation rules (lowercase, strip
> path separator), and the `list_by_prefix` lookup semantics — either: (a) exact match
> only (document limitation), or (b) prefix match with defined precedence when multiple
> keys share a prefix. Include a test case for each semantic.

---

## Phase 2 Design Brief Coverage Map

| Gap | Relevant Design Brief | Currently Captured | Required Action |
|-----|-----------------------|--------------------|-----------------|
| Subscription validation handshake | `.docs/design-adf-control-plane-routing.md` §6 | ❌ Absent | Add handshake protocol sub-section |
| PATCH 204 semantics | Issue #944 / #950–#955 design | ❌ Absent | Add Gitea API response contract |
| `list_by_prefix` pattern | PR #999 / `.docs/design-pr-dispatch-per-project.md` | ❌ Absent | Add key normalisation sub-section |

---

## Verdict

**FAIL** — Phase 2 design brief does not capture any of the three identified gaps.

Implementation must not proceed until each Phase 2 brief receives the required additions
documented above. The gaps are not cosmetic: each represents a missing invariant that
will cause silent routing errors, API failures, or config-resolution ambiguity under
the multi-project fleet conditions that Phase 2 is designed to enable.

---

## Recommended Resolution

1. **Subscription validation handshake** — Add to `.docs/design-adf-control-plane-routing.md`
   before implementing `RoutingDecisionEngine` (Gitea #524 / Step 6).

2. **PATCH 204 semantics** — Add to the design document for whichever Phase 2b–2e
   issue is next in implementation order (recommend #950 or #953 as they introduce
   the most new commit-status PATCH paths).

3. **`list_by_prefix` pattern** — Add to `.docs/design-pr-dispatch-per-project.md`
   (referenced in PR #999) or to the config layer design before any new project is
   onboarded to the multi-project fleet.

All three additions are prose-only changes to existing design briefs — no Rust code
required. Combined effort: ~45 minutes.
