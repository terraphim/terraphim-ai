# Decision: Approve Native CI Ignore-Policy Specification

**Date:** 2026-08-14
**Status:** APPROVED — PHASE 3 IMPLEMENTATION AUTHORIZED
**Issue:** https://git.terraphim.cloud/terraphim/terraphim-ai/issues/3222

## Approved artifact

- Combined design/specification: `docs/plans/design-native-ci-ignore-policy-3222.md`
- Approval SHA-256: `ce6417e176bbf59480e5341d4488b8281e6dbf8b98916dfeaf700e2e2ea1d419`
- Current post-review SHA-256: `2f40a11b3236c13769bd07976e9e06e07e52b3513fde0fa4039118ea6dcce143`
- Frozen Phase 2 body SHA-256: `632acff377eff7053a397b11ce589c03b43b9241ab7880c10f374ab8dfbb83ca`

## Quality gate

Independent Claude Opus Phase 2.5 KLS: **PASS**, 5.00/5 in every dimension; essentialism passed; convergence complete.

## Authorization basis

Alex approved the design with “perfect, proceed.” Phase 2.5 introduced no new implementation files or requirements and retained the approved global forbidden-flag policy. Its two factual clarifications were resolved and independently accepted.

## Authorized implementation scope

Only:

1. `.gitea/workflows/native-ci.yml`
2. `crates/terraphim_gitea_runner/src/lib.rs`

Strict RED→GREEN TDD is required. No dependency, API, branch-protection, status-context, Docker, network-test, or ignored-test changes are authorized.

## Post-approval defect loop-back

Independent structural review Round 1 identified two under-enforced parts of
the approved policy: `--include-ignored` was not rejected, and the guard did not
enforce the exact ordered direct-Cargo-test set. D001/D002 in the specification
appendix clarify those existing invariants without expanding the two-file
implementation scope. Mutation-first evidence and final gate results are
retained in `docs/plans/verification-native-ci-ignore-policy-3222.md`.
