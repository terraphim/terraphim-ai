# Verification Report: Native CI Ignore-Policy Coverage Correction

**Status:** LOCAL VERIFICATION PASSED — NATIVE PR VALIDATION PENDING
**Date:** 2026-08-14
**Issue:** https://git.terraphim.cloud/terraphim/terraphim-ai/issues/3222
**Base:** `origin/main` at `64748c58da4d9c97109cce5a2ee473c5ffe2a50d`
**Design/specification:** `docs/plans/design-native-ci-ignore-policy-3222.md`

## Summary

The approved two-file implementation passed strict RED→GREEN TDD, mutation testing, formatting, clippy, the workspace library baseline, and both package-default targeted suites. The live-Docker integration test remained ignored. No dependency, API, job, trigger, status-context, Docker, network-test, or branch-protection change was made.

Native-runner push and pull-request observations remain required before merge.

## Traceability

| Requirement | Implementation | Evidence | Status |
|---|---|---|---|
| Preserve workspace library baseline | Existing `cargo test --workspace --lib --no-fail-fast` remains exact | Final workspace-lib gate `rc=0` | PASS |
| Execute Gitea-runner integration contracts | Targeted package-default workflow step | 53 lib + 21 integration passed; 34s | PASS |
| Execute LLM integration, trybuild, and rustdoc contracts | Targeted package-default workflow step | 38 passed, 1 ignored; 7 doctests passed; 18s | PASS |
| Keep live Docker test excluded | No `--ignored` or `--include-ignored` | Docker test explicitly reported ignored | PASS |
| Prevent targeted-step removal/mutation/duplication | Crate-root lib guard parses embedded workflow and pins exact counts | Initial RED failed `left: 0`, `right: 1`; final GREEN passed | PASS |
| Prevent extra/reordered direct Cargo-test steps | Guard asserts exact ordered direct-Cargo-test list | Temporary fourth-step mutation rejected | PASS |
| Reject scope-widening/ignored flags | Guard rejects `--tests`, `--all-targets`, `--ignored`, `--include-ignored` | Mutation failed on exact `--include-ignored` token | PASS |
| Keep targeted local runtime under 10 minutes | Package-default commands measured independently | 34s runner; 18s companion | PASS locally |
| Preserve workflow/job/context identity | No edits to workflow name, job id, or triggers | Exact diff inspection | PASS |
| Validate on authoritative native runner | Push and PR contexts on immutable PR SHA | Awaiting PR | PENDING |

## TDD and Mutation Evidence

### Initial RED

Command:

```bash
cargo test -p terraphim_gitea_runner --lib --no-fail-fast native_ci_workflow_declares_targeted_package_default_policy_tests -- --exact
```

Observed against the unchanged workflow:

```text
running 1 test
FAILED
assertion `left == right` failed
left: 0
right: 1
0 passed; 1 failed; 52 filtered out
```

### Initial GREEN

After adding the two targeted workflow commands:

```text
running 1 test
1 passed; 0 failed; 52 filtered out
```

### Mutation closure

Temporary fourth workflow step, never executed:

```yaml
- run: cargo test -p terraphim_llm_runner -- --include-ignored
```

- Before remediation: guard incorrectly passed, reproducing D001/D002.
- After remediation: guard failed with:

```text
native CI cargo-test step must not include --include-ignored:
cargo test -p terraphim_llm_runner -- --include-ignored
0 passed; 1 failed; 52 filtered out
```

The workflow was then restored byte-for-byte. Restored SHA-256 matched the pre-mutation backup, and the focused guard returned GREEN.

## Final Local Gates

Executed sequentially on the final strengthened bytes:

```text
RESULT fmt rc=0 elapsed_seconds=1
RESULT clippy rc=0 elapsed_seconds=23
RESULT workspace-lib rc=0 elapsed_seconds=18
RESULT gitea-runner rc=0 elapsed_seconds=34
RESULT llm-runner rc=0 elapsed_seconds=18
ALL_GATES_PASS 2026-08-14T21:20:45Z
```

Non-vacuous package results:

```text
terraphim_gitea_runner:
  53 library passed
  5 + 4 + 1 + 8 + 3 = 21 integration passed
  0 ignored; 0 doctests

terraphim_llm_runner:
  1 library passed
  6 + 19 + 3 + 2 = 30 non-ignored integration passed
  1 live-Docker integration test ignored
  7 rustdoc compile_fail tests passed
  total: 38 passed, 1 ignored
```

## Static Analysis and Review

- `cargo clippy --workspace --all-targets -- -D warnings`: PASS.
- `cargo fmt --all -- --check`: PASS.
- `git diff --check`: PASS.
- `ubs` scanner: unavailable on this host; no result is claimed.
- Independent Claude Opus structural review Round 1: `3/5`, P0=0, P1=1, P2=6, REQUEST_CHANGES.
  - P1 local execution-evidence gap: CLOSED by final gates above; native PR evidence remains a merge gate.
  - D001 `--include-ignored`: CLOSED with mutation evidence.
  - D002 ordered/exhaustive direct Cargo-test list: CLOSED.
  - Documentation status/count/provenance findings: CLOSED by current design status/counts and this retained report.
  - Round 2 on the exact final delta is required before commit.

## Defect Register

| ID | Description | Origin | Severity | Resolution | Status |
|---|---|---|---|---|---|
| D001 | `--include-ignored` bypassed forbidden-token scan | Phase 2.5 specification | P2 | Added token and mutation evidence | CLOSED |
| D002 | Extra/reordered direct Cargo-test commands were not rejected | Phase 2 test strategy | P2 | Assert exact ordered direct-Cargo-test list | CLOSED |

## Remaining Gates

1. Independent structural review Round 2 at confidence 5/5, P0=P1=P2=0.
2. Commit and push the reviewed exact bytes.
3. Open the Gitea PR referencing #3222.
4. Verify branch head SHA equals remote and PR head SHA.
5. Require native-ci push and pull-request contexts green on that exact SHA.
6. Record native per-step durations; each targeted step must remain <=10 minutes.
7. Do not merge until all required statuses and review gates pass.
