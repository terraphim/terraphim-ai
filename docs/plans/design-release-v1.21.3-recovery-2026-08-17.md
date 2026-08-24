# Research and Design: v1.21.3 Release Recovery

**Status**: Approved
**Issue**: Gitea #3255, `release: recover complete v1.21.3 server and distribution matrix`
**Author**: Codex
**Date**: 2026-08-17
**Scope**: Phase 1/2 only. No implementation, no commit.
**Approval Basis**: User instruction on 2026-08-17 to use disciplined engineering skills to complete actions; KLS evaluation passed with Physical 4, Empirical 5, Syntactic 4, Semantic 4, Pragmatic 4, Social 4, average 4.17.

## Executive Summary

The `v1.21.3` tag already resolves to immutable source commit `4a1d9f24c99f1504fdb2476667aa1087b698d33c`, and failed release run evidence shows the tag checkout itself worked. Recovery failed because the current comprehensive workflow has no validated manual release-tag input, so manual recovery from canonical `main` cannot target existing tag source without retagging, and the original run lost required Linux artifacts to runner/tooling failures before the critical asset gate correctly stopped the release.

The surgical recovery design is to run the fixed workflow from canonical `main`, accept a strictly validated `release_tag` and required manual `expected_source_sha`, resolve and verify that tag to the expected immutable commit, checkout that resolved source SHA in every source-dependent job, sanitize inherited Rust wrappers, make `cross` detection side-effect-free, add bounded Docker retry for transient BuildKit EOFs, and preserve the critical server asset verification gate unchanged in intent.

## Essential Questions

| Question | Answer | Evidence |
|---|---:|---|
| Does this solve a significant validated need? | Yes | Gitea #3255 says the `v1.21.3` release object exists but run `31942995782` did not publish the complete required server/package/container surface. |
| Does it leverage project strengths? | Yes | The workflow already has a complete release graph, artifact naming, signing, checksums, Homebrew update, and asset verification; recovery only needs a controlled dispatch/source contract. |
| Is it essential now? | Yes | Existing installation surfaces are incomplete: asset verification reported three missing Linux server patterns in `/tmp/ai-asset-verify.log:250-255`. |

**Proceed**: Yes, 3/3.

## Problem Statement

Issue #3255 requires recovering the existing `v1.21.3` release without moving or recreating its tag. Success means a manual recovery dispatch from canonical `main` builds source at `v1.21.3^{commit}` = `4a1d9f24c99f1504fdb2476667aa1087b698d33c`, publishes the missing required server artifacts to the existing release, and leaves critical asset enforcement intact.

## Exact Current-State/Data-Flow Map

Current trigger surface in `.github/workflows/release-comprehensive.yml`:

1. Tag pushes for `v*` and component tags trigger the workflow (`release-comprehensive.yml:3-10`).
2. Manual dispatch only accepts `test_run`; there is no `release_tag` input (`release-comprehensive.yml:11-17`).
3. `verify-versions` checks out the event ref with default `actions/checkout@v6`, extracts version from `github.ref_name`, and fails non-tag names (`release-comprehensive.yml:33-61`).
4. Build jobs depend on `verify-versions`, then each performs default checkout of the event ref (`release-comprehensive.yml:97-165`, `507-516`, `638-646`, `718-719`).
5. Linux server artifacts are expected from the `build-binaries` matrix for `x86_64-unknown-linux-gnu`, `x86_64-unknown-linux-musl`, and `aarch64-unknown-linux-musl` (`release-comprehensive.yml:106-117`, `314-340`).
6. macOS universal binaries are created from the two macOS artifacts and signed/notarized (`release-comprehensive.yml:369-505`).
7. Debian packaging is best-effort (`continue-on-error: true`) and uses the same default source checkout (`release-comprehensive.yml:507-565`).
8. Client and desktop workflows are cross-repo dispatches, only on tag refs (`release-comprehensive.yml:567-624`).
9. Docker builds use the reusable local workflow `./.github/workflows/docker-multiarch.yml`, passing `github.ref_name` as the Docker tag (`release-comprehensive.yml:626-636`).
10. `verify-release-assets` downloads `binaries-*` artifacts and fails the release if the three macOS server files or three Linux server archive patterns are missing (`release-comprehensive.yml:638-703`).
11. `create-release` only runs when version verification, signing, and asset verification succeed, then overwrites/updates the release for `github.ref_name` (`release-comprehensive.yml:705-808`).
12. Homebrew waits for client binaries and then consumes checksums from the GitHub release (`release-comprehensive.yml:859-1103`).

Observed run data flow:

1. Failed jobs on the tag event did fetch and checkout `v1.21.3`; logs show `rev-parse refs/tags/v1.21.3^{commit}` returned `4a1d9f24c99f1504fdb2476667aa1087b698d33c` in GNU, Debian, musl ARM, and asset verification logs (`/tmp/ai-linux-gnu.log:109-127`, `/tmp/ai-deb.log:183-198`, `/tmp/ai-musl-arm.log:117-135`, `/tmp/ai-asset-verify.log:102-136`).
2. macOS and Windows artifacts were produced and downloaded by asset verification, but no Linux server artifacts were present (`/tmp/ai-asset-verify.log:150-180`, `/tmp/ai-asset-verify.log:231-245`).
3. Critical asset verification then failed exactly because the three Linux server archive patterns were missing (`/tmp/ai-asset-verify.log:247-255`).

## Ranked Root Causes With Log Line Evidence

1. **Missing recovery dispatch contract forces retagging or wrong source/version on manual runs.**
   - Workflow evidence: manual dispatch has only `test_run`, no `release_tag` (`release-comprehensive.yml:11-17`).
   - Workflow evidence: version extraction uses `github.ref_name` and rejects non-tag refs (`release-comprehensive.yml:39-61`).
   - Impact: a manual run from `main` would see `github.ref_name=main`, fail version extraction, or publish with the wrong tag unless the tag is moved/recreated.

2. **Self-hosted Linux GNU and Debian lanes inherited an unavailable `RUSTC_WRAPPER=kache`.**
   - GNU failed before compilation: `could not execute process 'kache ... rustc -vV'` and `No such file or directory` (`/tmp/ai-linux-gnu.log:1540-1558`).
   - Debian failed the same way during `cargo deb` (`/tmp/ai-deb.log:1633-1648`).
   - Impact: required Linux GNU server archive and Debian package never uploaded.

3. **ARM musl lane treats `cross --version` as a hard preflight, but installed `cross` can return non-zero after metadata probing.**
   - The job runs `cross --version` and exits from the install step when `cross` exists (`release-comprehensive.yml:183-191`).
   - Log shows `cross 0.2.5`, then metadata warning, then `Errors encountered before cross compilation, aborting`, exit 1 (`/tmp/ai-musl-arm.log:331-349`).
   - Impact: required `aarch64-unknown-linux-musl` server archive never uploaded.

4. **Docker BuildKit hit a transient transport EOF during Ubuntu 20.04 multiarch build/push.**
   - Reusable workflow executed at tag source (`/tmp/ai-docker-2004.log:22-30`).
   - BuildKit failed with `rpc error: code = Unavailable ... error reading from server: EOF ... graceful_stop` (`/tmp/ai-docker-2004.log:3909-3917`).
   - Impact: container publication is incomplete or unverified, but this should be bounded-retry recoverable without weakening release asset checks.

5. **Critical server asset enforcement correctly stopped release creation after upstream artifacts were absent.**
   - Asset verification downloaded five `binaries-*` artifacts but no Linux artifacts (`/tmp/ai-asset-verify.log:150-180`, `/tmp/ai-asset-verify.log:231-245`).
   - It then reported missing Linux GNU, x86_64 musl, and aarch64 musl archive patterns and exited 1 (`/tmp/ai-asset-verify.log:250-256`).
   - Impact: this is not a bug to remove; it is a control to preserve.

## Constraints

### Technical Constraints

- Recovery workflow must execute the current fixed workflow from canonical `main`, but build source from existing tag `v1.21.3` peeled to `4a1d9f24c99f1504fdb2476667aa1087b698d33c`.
- GitHub Actions `workflow_dispatch` runs workflow logic from a branch/ref; source checkout must therefore be explicit in each source-dependent job. The workflow ref and source ref are separate values and must remain explicitly named and routed separately.
- `actions/checkout@v6` defaults to the event ref and shallow fetch; immutable tag resolution needs explicit fetch and checkout behavior.
- Existing release asset verification must remain blocking for the critical server matrix.
- `v1.21.3` must not be moved, recreated, force-updated, or replaced.
- No product code changes are required.

### Operational Constraints

- Self-hosted runners may inherit environment variables like `RUSTC_WRAPPER`; the workflow must sanitize unavailable wrappers locally.
- Existing macOS signing/notarization and 1Password secret flow should remain unchanged.
- Docker retry should be bounded and limited to transient build/push failure, not an infinite loop or a substitute for artifact verification.

## Vital Few

| Vital Item | Why It Matters | Evidence |
|---|---|---|
| Validated `release_tag` dispatch | Enables recovery without retagging and rejects hostile/manual misuse before checkout or mutation. | Issue #3255 acceptance criteria; current dispatch lacks this input (`release-comprehensive.yml:11-17`). |
| Immutable source checkout | Ensures every source-dependent job builds exactly `v1.21.3^{commit}` = `4a1d9f24...` while workflow logic comes from `main`. | Logs prove the tag commit (`/tmp/ai-linux-gnu.log:114-127`, `/tmp/ai-asset-verify.log:107-136`). |
| Preserve critical asset gate | Prevents partial release publication when Linux server artifacts are absent. | Existing gate correctly failed (`/tmp/ai-asset-verify.log:250-256`). |

## Explicit Assumptions/Unknowns

### Assumptions

1. `v1.21.3` must resolve to `4a1d9f24c99f1504fdb2476667aa1087b698d33c`; any different SHA is a hard failure.
2. Existing macOS/Windows behavior is acceptable and should not be refactored.
3. The recovered release should update the existing GitHub release using the same `softprops/action-gh-release@v2` overwrite behavior, with `tag_name` set to validated `release_tag`.
4. Debian remains non-critical because the current job is `continue-on-error: true`; Linux server archives remain critical.

### Unknowns

1. Whether Docker EOF was runner-local or service-side; mitigation is bounded retry plus explicit classification in the final recovery report.
2. Whether `kache` should be installed long term; recovery should not depend on it.
3. Whether all Linux musl targets will complete after side-effect-free `cross` detection; artifact verification will decide.

## Rejected Alternatives

| Alternative | Rejection Reason | Risk If Included |
|---|---|---|
| Move or recreate `v1.21.3` | Explicitly out of scope and breaks immutable release provenance. | Release consumers see a changed tag for the same version. |
| Disable or relax critical asset verification | Contradicts issue scope and would allow another incomplete release. | Silent distribution gaps. |
| Backport workflow changes onto the tag | Mutates the source/release tag path and complicates provenance. | Blurs workflow logic and released source. |
| Full release workflow rewrite | Not needed; existing graph is close and evidence points to a few failures. | High regression risk in signing, release, and Homebrew steps. |
| Product code or Cargo dependency changes | Failures are workflow/runner dispatch problems, not product behavior. | Larger blast radius before recovery. |
| Infinite Docker retries | Masks real failures and can burn runner capacity. | Hung release runs and unclear status. |

## Validated Recovery-Dispatch Contract

### Inputs

Add to `workflow_dispatch.inputs`:

- `release_tag`
  - Required for recovery runs.
  - Type: string.
  - Allowed format: strict SemVer release tag only, `^v[0-9]+\.[0-9]+\.[0-9]+$`.
  - For issue #3255, accepted value is exactly `v1.21.3`.
- `expected_source_sha`
  - Required for manual recovery dispatch.
  - Type: string.
  - Allowed format: exactly 40 lowercase hexadecimal characters, `^[0-9a-f]{40}$`.
  - For issue #3255, required value is `4a1d9f24c99f1504fdb2476667aa1087b698d33c`.
  - Not required for tag-push runs; tag-push runs derive the trusted event tag and source SHA from the GitHub event ref without accepting a manual expected SHA input.
- `test_run`
  - Existing boolean retained.

### Validation Step

Create an early `resolve-release-source` job before `verify-versions`:

1. Determine `RELEASE_TAG`:
   - Tag push: use `github.ref_name`.
   - Manual dispatch: use `inputs.release_tag`.
2. Determine `EXPECTED_SOURCE_SHA`:
   - Manual dispatch: require `inputs.expected_source_sha`, validate it with `^[0-9a-f]{40}$`, and compare it to the resolved source SHA.
   - Tag push: do not accept or require a manual expected input; derive the trusted event tag/source SHA from `github.ref` after checkout/fetch.
3. Reject empty tag, component tags, branch names, shell metacharacters, path-like refs, spaces, and partial SHAs.
4. Validate `RELEASE_TAG` with `^v[0-9]+\.[0-9]+\.[0-9]+$`.
5. Fetch only `refs/tags/$RELEASE_TAG:refs/tags/$RELEASE_TAG` from origin.
6. Resolve immutable source commit by recursively peeling annotated tags to the commit object, for example with `git rev-parse "refs/tags/$RELEASE_TAG^{commit}"`.
7. Compare the recursively peeled source SHA against `EXPECTED_SOURCE_SHA` for manual recovery; fail before any build, Docker push, release mutation, or cross-repo dispatch if it differs.
8. Output:
   - `release_tag`
   - `version` = tag without leading `v`
   - `source_sha`
   - `source_ref` = resolved SHA, not a mutable tag name.
   - `workflow_ref` = event workflow ref, kept separate from `source_ref`.

### Source Checkout Contract

Every source-dependent job must use:

```yaml
with:
  ref: ${{ needs.resolve-release-source.outputs.source_sha }}
  fetch-depth: 1
```

This applies to:

- `verify-versions`
- `build-binaries`
- `create-universal-macos` only if repository scripts/source are needed; artifact-only downloads do not need source checkout except signing scripts currently do.
- `sign-and-notarize-macos`
- `build-debian-packages`
- `verify-release-assets` if it reads workflow-side scripts or release metadata from source
- `create-release`
- Docker reusable workflow source checkout, via an explicit input or wrapper behavior in `.github/workflows/docker-multiarch.yml` if needed

Release naming, release lookup, release upload, client wait, and Homebrew checksum URLs must use `needs.resolve-release-source.outputs.release_tag`, not `github.ref_name`, during manual recovery.

Workflow logic continues to come from the dispatch/tag event workflow ref, while repository source checkout for builds comes from `source_sha`/`source_ref`. Do not collapse these into a single ref variable.

## Exact Workflow/File Changes

### New Files

| File | Purpose |
|---|---|
| None | Implementation should remain workflow-only. |

### Modified Files

| File | Exact Change |
|---|---|
| `.github/workflows/release-comprehensive.yml` | Add `workflow_dispatch.inputs.release_tag` and required manual `expected_source_sha`; add early `resolve-release-source` job; make `verify-versions` consume resolved `version` and checkout `source_sha`; replace release-tag/version references from `github.ref_name`/`github.ref` where release mutation or waits require the target tag; update all source checkouts to `source_sha`; sanitize `RUSTC_WRAPPER`/`RUSTC_WORKSPACE_WRAPPER` when the configured executable is absent; make `cross` detection non-fatal; keep workflow ref and source ref as distinct values; either call an implementable command-level Docker retry path or document manual rerun for the classified transient EOF. |
| `.github/workflows/docker-multiarch.yml` | Only if needed: accept `source_ref` or use caller-provided tag/source checkout explicitly; add bounded command-level retry around the Buildx build/push command for classified transient EOF, including cleanup between attempts. Keep image tags derived from validated `release_tag`. Do not claim GitHub reusable workflow syntax can automatically retry a called job. |
| `docs/plans/design-release-v1.21.3-recovery-2026-08-17.md` | This Phase 1/2 document. |

### No Changes

- No Rust, TypeScript, Svelte, Dockerfile, Cargo, or release tag changes.
- No suppression of `verify-release-assets`.
- No changes to artifact naming beyond routing version/tag values through resolved outputs.

## Test-First Strategy

### Static Workflow Contract Tests

Add static tests before changing behavior, preferably under an existing workflow-test location or a new minimal script checked by CI. These tests must parse YAML and structured values where possible instead of merely grepping comments or free text.

1. Parse `.github/workflows/release-comprehensive.yml` as YAML and assert `workflow_dispatch.inputs.release_tag` exists and is required.
2. Parse `.github/workflows/release-comprehensive.yml` as YAML and assert manual `workflow_dispatch.inputs.expected_source_sha` exists, is required, and has validation coverage for `^[0-9a-f]{40}$`.
3. Assert the strict tag regex is present in executable validation logic and does not accept component tags or branches.
4. Assert there is a `resolve-release-source` job and that source-dependent jobs need it.
5. Assert checkouts in source-dependent jobs use `needs.resolve-release-source.outputs.source_sha`.
6. Assert workflow-ref values and source-ref/source-SHA values are represented separately in job inputs and outputs.
7. Assert `softprops/action-gh-release` uses the resolved release tag, not raw `github.ref_name`, for manual recovery.
8. Assert critical patterns for Linux GNU, x86_64 musl, and aarch64 musl server archives remain present in executable verification logic.
9. Assert Docker retry is either an implementable bounded command-level Buildx retry with attempt cleanup, or the workflow clearly documents manual rerun for the classified transient EOF; do not assert impossible automatic retry semantics for reusable jobs.
10. Assert no workflow step contains a tag-moving command such as `git tag -f`, `git push --force`, or deletion of `refs/tags/v1.21.3`.

### Hostile Dispatch Inputs

Use unit-style shell validation tests for the tag resolver:

| Input | Expected Result |
|---|---|
| `v1.21.3` | Accepted and resolved to `4a1d9f24c99f1504fdb2476667aa1087b698d33c`. |
| `main` | Rejected before checkout. |
| `release/v1.21.3` | Rejected before checkout. |
| `v1.21.3;echo bad` | Rejected before checkout. |
| `v1.21.3 $(echo bad)` | Rejected before checkout. |
| `../v1.21.3` | Rejected before checkout. |
| `terraphim_server-v1.21.3` | Rejected for recovery dispatch. |
| `v1.21` | Rejected. |
| `v1.21.3-rc.1` | Rejected unless a later issue explicitly broadens recovery semantics. |
| `4a1d9f24c99f1504fdb2476667aa1087b698d33c` | Rejected as tag input. |

Expected source SHA validation must separately reject uppercase hex, short SHAs, non-hex characters, empty values, and shell/path metacharacters for manual recovery dispatch.

### Runtime Verification

1. Dry-run dispatch from `main` with `release_tag=v1.21.3`, `expected_source_sha=4a1d9f24c99f1504fdb2476667aa1087b698d33c`, and `test_run=true`; verify resolver logs show the expected recursively peeled SHA and no release mutation occurs.
2. Recovery dispatch with `release_tag=v1.21.3`, `expected_source_sha=4a1d9f24c99f1504fdb2476667aa1087b698d33c`, and `test_run=false`.
3. Verify Linux GNU, x86_64 musl, and aarch64 musl server artifacts exist in run artifacts and in the GitHub release.
4. Verify archive checks:
   - each required `.tar.gz` lists `terraphim_server`;
   - extracted binary is executable;
   - GNU binary `--version` contains `1.21.3` where executable on runner.
5. Verify checksums include all critical assets.
6. Verify `gh release view v1.21.3` shows assets attached to the existing release and tag SHA remains unchanged.
7. Obtain independent different-model review and require `5/5` with `Findings: P0=0 P1=0 P2=0` before merge, per issue acceptance criteria.
8. If Docker BuildKit EOF recurs and the design selected manual rerun rather than command-level retry, classify it as the known transient EOF and rerun the Docker job manually; do not represent that behavior as automatic reusable-job retry.

## Rollback

1. Because the tag is not moved, rollback does not require Git history repair.
2. If the workflow change is bad before release mutation, cancel the run and revert the workflow commit on `main`.
3. If incorrect assets are uploaded to the existing release, remove only those assets from the GitHub release, leave `v1.21.3` tag untouched, revert the workflow commit, and rerun after fix.
4. If Docker images are incorrectly pushed, repush corrected images for the same validated tag after source SHA verification; do not delete or move the Git tag.
5. Preserve failed run logs as evidence for post-recovery review.

## Traceability From Issue Acceptance Criteria to Verification

| Acceptance Criterion | Design/Test Coverage |
|---|---|
| Invalid dispatch tags fail before checkout or mutation. | Strict regex and hostile input tests; resolver job precedes build, Docker, release, and dispatch jobs. |
| Manual dispatch requires immutable expected source SHA. | `expected_source_sha` is required for manual recovery, validated as strict 40 lowercase hex, and compared after recursive annotated-tag peeling. |
| Tag-push runs do not depend on manual expected SHA. | Resolver derives trusted event tag/source SHA from the tag-push event ref and keeps manual expected input out of that path. |
| Workflow ref and source ref remain explicitly separate. | Resolver emits both values; source-dependent checkouts consume `source_sha`/`source_ref`, while workflow execution remains tied to the event workflow ref. |
| Dispatch `release_tag=v1.21.3` resolves and checks out commit `4a1d9f24c99f1504fdb2476667aa1087b698d33c`. | Resolver compares recursively peeled SHA; all source checkouts use `source_sha`; dry-run verifies resolver log. |
| Linux GNU, x86_64 musl, and aarch64 musl server artifacts exist and pass archive checks. | Preserve critical patterns; add runtime archive listing/extract/version checks. |
| Required recovery jobs are green; transient/optional lanes classified. | Required jobs are resolver, version verification, binary matrix, macOS signing, asset verification, create release; Debian remains best-effort unless reclassified; Docker has bounded retry and explicit final status. |
| Release assets attached to existing GitHub release without moving tag. | `action-gh-release` uses resolved release tag; static tests reject tag force/deletion commands; post-run verifies tag SHA. |
| Independent different-model review reports 5/5 and no P0/P1/P2 findings. | Required pre-merge verification step in runtime checklist. |

## Implementation Steps

1. Add failing static workflow contract tests for recovery dispatch, immutable source checkout, preserved critical asset checks, no tag-moving commands, separate workflow/source refs, implementable Docker retry semantics, and structured YAML/value parsing where possible.
2. Add `release_tag` and required manual `expected_source_sha` inputs to `workflow_dispatch`.
3. Add `resolve-release-source` job with strict tag validation, strict 40-lowercase-hex manual expected SHA validation, targeted tag fetch, recursive annotated-tag peeling to a commit, manual expected SHA comparison, tag-push derivation of trusted event tag/source SHA, separate workflow/source ref outputs, and early failure before mutation.
4. Change `verify-versions` to depend on the resolver, checkout `source_sha`, and use resolver `version` instead of deriving from `github.ref_name`.
5. Update every source-dependent checkout in `release-comprehensive.yml` to `source_sha`.
6. Replace release mutation, release wait, Homebrew checksum, Docker tag, and client dispatch tag/version references with resolver outputs where manual dispatch would otherwise use `main`.
7. Sanitize unavailable `RUSTC_WRAPPER` and `RUSTC_WORKSPACE_WRAPPER` before Cargo/Cross/Cargo Deb commands on self-hosted Linux lanes.
8. Change `cross` detection so `cross --version` cannot fail the install/preflight step after printing a metadata warning; if `cross` exists, continue, otherwise install.
9. Add either bounded command-level Buildx retry for classified transient Docker BuildKit EOF with attempt cleanup, or document manual job rerun for that specific transient failure; do not model automatic retry as reusable workflow syntax.
10. Run static tests and hostile input validation locally.
11. Run `test_run=true` manual dispatch from `main` for `release_tag=v1.21.3` and `expected_source_sha=4a1d9f24c99f1504fdb2476667aa1087b698d33c`; confirm source SHA and no release mutation.
12. Run recovery dispatch for `v1.21.3`.
13. Verify critical artifacts, archives, release assets, checksums, Docker status, and immutable tag SHA.
14. Obtain independent different-model review and attach review evidence to #3255 before merge/closure.

## Approval

Approved on 2026-08-17 by user instruction to use disciplined engineering skills to complete actions. KLS evaluation result: Physical 4, Empirical 5, Syntactic 4, Semantic 4, Pragmatic 4, Social 4; average 4.17, PASS.
