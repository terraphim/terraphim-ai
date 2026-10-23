# Decision: Approve Native CI Ignore-Policy Research

**Date:** 2026-08-14
**Status:** APPROVED — PHASE 2 DESIGN AUTHORIZED
**Issue:** https://git.terraphim.cloud/terraphim/terraphim-ai/issues/3222

## Approved artifact

- Document: `docs/plans/research-native-ci-ignore-policy-3222.md`
- SHA-256: `3dd5f2b069e34fd5a361f204d520c11405b734513c84fe98a230a21e21b9429b`
- Size: 377 lines / 21,297 bytes

## Quality gate

Independent Claude Opus KLS R2 result: **PASS**.

- Average: 4.43/5
- Minimum dimension: 4/5
- Physical: 5/5
- Empirical: 4/5
- Syntactic: 4/5
- Semantic: 5/5
- Pragmatic: 4/5
- Social: 4/5
- Governance: 5/5
- Essentialism: all checks PASS

## Human decision

Alex approved the frozen research artifact as the root-cause basis and authorized proceeding to Phase 2 design.

## Boundaries

This decision authorizes design only. It does not authorize implementation, workflow mutation, commit, push, or deployment. Phase 2 must preserve workspace `--lib`, avoid workspace-wide `--all-targets`, cover package-default runner and companion contracts including rustdoc tests, and define a RED-capable workflow-policy test before implementation.
