# Decision: Approve Native CI Ignore-Policy Design

**Date:** 2026-08-14
**Status:** APPROVED — PHASE 2.5 SPECIFICATION AUTHORIZED
**Issue:** https://git.terraphim.cloud/terraphim/terraphim-ai/issues/3222

## Approved artifact

- Document: `docs/plans/design-native-ci-ignore-policy-3222.md`
- SHA-256: `632acff377eff7053a397b11ce589c03b43b9241ab7880c10f374ab8dfbb83ca`

## Quality gate

Independent Claude Opus final design KLS result: **PASS**.

- Average: 5.00/5
- Minimum dimension: 5/5
- Essentialism: all checks passed

## Human decision

Alex reviewed the plan description and responded: “perfect, proceed.” This approves the frozen Phase 2 design and authorizes Phase 2.5 specification only.

## Approved policy interpretation

The approved design bans `--tests`, `--all-targets`, and `--ignored` across every native-CI Cargo test step. A future legitimate exception requires an explicit reviewed update to the guard; unrelated future steps are not silently exempted.

## Boundary

This decision does not itself authorize implementation. Phase 2.5 findings and their quality gate must complete first.
