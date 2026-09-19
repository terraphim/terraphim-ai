# Release Coordinator

**Workflow file**: `.github/workflows/release-coordinator.yml`
**CLI**: `.github/scripts/release/release_coordinator.py`
**Tests**: `tests/release_coordinator_test.py`, `tests/workflow_release_coordinator_contract_test.py`
**Schema**: `.release/release-manifest.schema.json` (validated via `scripts/validate-release-manifest.py`)

Implements Gitea issue [terraphim/terraphim-ai#3336](https://git.terraphim.cloud) (`[MP1] Implement
central release coordinator and approval-gated publish workflow`) and the
pre-approval rehearsal / zero-mutation contract from issue #3382.

## Purpose

Terraphim releases two central channels (a GitHub release and an R2 stable
manifest) built from artifacts produced by two separate repos
(`terraphim-ai` and `terraphim-clients`), then hand off to three downstream
packaging channels (Homebrew tap PR, AUR, Omarchy). Before this coordinator,
nothing owned that lifecycle end to end: every job independently decided
what to trust, so a partial failure or a re-run risked silently downloading
a different tag/run, re-uploading over validated bytes, or dispatching
downstream channels before the central artifacts had actually landed.

The coordinator is the **single source of truth** for a release. Its coarse
cursor remains:

```
planned -> staged -> verified -> promoting -> promoted
                                          \-> failed
{planned, staged, verified, promoting, failed} -> superseded
```

It never builds anything. It only stages already-downloaded, immutable
artifacts from two *frozen* producer workflow runs, validates them against
the canonical manifest schema and business invariants, and gates every
central mutation on an **approval bound to the exact verified manifest
SHA-256 digest**. The durable record also keeps source landing, CI
verification, release creation, package production/reconciliation,
promotion, publication, and terminal verification as separate `phases`;
dispatch acceptance is never treated as downstream publication success.

## Operator flow

### 1. Rehearsal (default, zero mutation)

Dispatch `release-coordinator.yml` with `promote: false` (the default) and:

- `release_version`, `release_tag`
- `terraphim_ai_run_id`, `terraphim_ai_sha`, `terraphim_ai_tree_sha` -- the
  exact completed `terraphim-ai` producer workflow run to stage from, and
  the source commit/tree it built
- `terraphim_clients_run_id`, `terraphim_clients_sha`, `terraphim_clients_tree_sha`
  -- same, for `terraphim-clients`
- `manifest_artifact_name` -- name of a workflow artifact (uploaded ahead of
  dispatch by operator tooling) containing the candidate `manifest.json`
  describing both producer artifact sets

This runs the `plan-stage-verify` job only:

1. Verifies both release tags peel to the exact frozen commit SHAs (fail
   closed on drift -- this is the immutable-tag check).
2. `plan` -- freezes version/tag/sources into coordinator state.
3. Downloads both producer artifact directories (already built, never
   rebuilt).
4. `stage` -- copies asset bytes into the state directory, checking each
   file's actual SHA-256/size against the manifest's declared values.
5. `verify` -- runs `scripts/validate-release-manifest.py` against
   `.release/release-manifest.schema.json`, cross-checks sources/version/tag
   against the frozen plan, confirms every staged file is accounted for
   (no missing/extra/duplicate), and freezes the manifest's SHA-256 digest.
6. Uploads the frozen state (`state.json` + `manifest.json` + `assets/`) as
   the `release-coordinator-state-<tag>` artifact.

**Nothing outside this artifact is touched.** No GitHub release is created,
nothing is written to R2, and no downstream repo is dispatched. The job
summary prints the frozen `manifest_sha256` -- this is the value an approver
must supply to promote.

If anything is wrong, fix the inputs and re-run rehearsal, or call `fail`
via the CLI directly against a downloaded state artifact to record why the
release was abandoned before any mutation happened.

### 2. Approval

An approver reviews the rehearsal run (or the state artifact + job summary)
and confirms the `manifest_sha256` matches the artifacts they intend to
ship. There is no separate "approve" UI: the approval **is** re-dispatching
this workflow with `promote: true` and `approval: <that exact digest>`.
Any other value fails closed in the `central-promote` job before any
mutation:

```
release-coordinator: approval not bound to current manifest digest: ...
```

### 3. Promotion

Re-dispatch with the same freeze inputs, `promote: true`, and
`approval: <manifest_sha256>`. This runs `plan-stage-verify` again
(idempotent -- staged bytes are content-addressed and are never
re-copied or overwritten if unchanged), then `central-promote`:

1. `approve` -- records the approval, bound to the digest.
2. `promote-begin` -- fails closed unless an approval bound to the *current*
   verified digest exists; transitions `verified -> promoting`.
3. Creates or recovers a coordinator-owned GitHub draft. An existing object
   is accepted only with the exact approval marker; lookup transport failures
   are not treated as absence.
4. Reconciles the draft to exactly the approved assets plus `manifest.json`
   and its detached Ed25519 `manifest.json.sig`. Extra or different bytes
   fail closed. The draft is then published as the atomic visibility point.
5. Records GitHub publication and terminal readback verification separately.
6. Publishes the detached signature to R2, verifies it, then conditionally
   creates the immutable stable `manifest.json` commit object. Existing
   identical bytes are an idempotent retry; different bytes are rejected.
7. Records R2 publication and terminal signed readback verification
   separately. Only both verified central channels transition to `promoted`.

Only once `central-promote` succeeds does `downstream-handoff` run:

8. `handoff-downstream` -- fails closed unless status is `promoted`; emits
   `downstream-handoff.json` listing all three downstream channels and the
   full asset list.
9. Dispatches each downstream repo with `schema_version: "1.0.0"`, the exact
   `correlation_id`, release tag, manifest digest, channel, and asset digests.
   A 204 is recorded only as `accepted`, never as terminal success.
10. Each downstream run must emit
    `terraphim-release-proof-<correlation-id>-<channel>/proof.json`; a resume
    supplies it through `homebrew_proof_run_id`, `aur_proof_run_id`, or
    `omarchy_proof_run_id` and verifies the proof against the frozen handoff.

The `repository_dispatch` payload is exactly:

```json
{
  "schema_version": "1.0.0",
  "release_tag": "vX.Y.Z",
  "manifest_sha256": "<64 lowercase hex>",
  "correlation_id": "<sha256(release_tag + ':' + manifest_sha256)>",
  "channel": "<homebrew_tap_pr|aur_terraphim_clients_bin|omarchy_terraphim_clients_bin>",
  "assets": [{"name": "<basename>", "sha256": "<64 lowercase hex>"}]
}
```

The downstream `proof.json` must use the same first five fields, set
`outcome` to `"success"`, and contain the exact same sorted asset name/digest
set. Its artifact name is
`terraphim-release-proof-<correlation_id>-<channel>`. A proof is rejected
unless an accepted dispatch for that channel is already recorded.

Cross-repository tag lookup, artifact download, dispatch, and proof download
require `TERRAPHIM_ORG_GITHUB_TOKEN` (or the documented repository-specific
fallback) to have **Actions: read** and **Contents: read** on each target
repository, plus **Contents: write** where `repository_dispatch` is sent.
The run-scoped `github.token` cannot read artifacts in another repository.

### 4. Recovery: partial failure

If, say, the GitHub release publishes but the R2 upload fails transiently,
`central-promote` records the failed R2 publication/verification and the job
fails (status stays `promoting`, GitHub publication and verification stay
successful). **Nothing
needs to be rebuilt.** Re-dispatch with the same inputs and the same
`approval` digest, adding `resume_run_id: <the failed run's ID>` so
`plan-stage-verify` restores the exact frozen state instead of re-planning.
successful records are immutable/idempotent; the R2 step retries only the
same bytes and terminal verification then transitions to `promoted`. A
`promoted` state may likewise be resumed to collect missing downstream proof
run IDs without repeating central publication.

### 5. Recovery: abandon before promotion

If rehearsal or review surfaces a problem, no undo is needed -- nothing was
mutated. To make that explicit in the record, run the CLI directly against
the downloaded state artifact:

```bash
python3 .github/scripts/release/release_coordinator.py fail \
  --state-dir ./state \
  --reason "wrong terraphim-clients run id; re-planning under a fresh tag"
```

### 6. Superseding an abandoned release

If a release is abandoned in favor of a later one, mark it explicitly
rather than leaving it dangling:

```bash
python3 .github/scripts/release/release_coordinator.py supersede \
  --state-dir ./state --by v1.2.4 --reason "re-cut with fixed manifest"
```

`supersede` is allowed from any non-terminal state or from `failed`; it is
**not** allowed from `promoted` (a shipped release cannot be retroactively
superseded by this record).

## Direct CLI usage (resume / inspect without re-running the workflow)

Download the `release-coordinator-state-<tag>` artifact and:

```bash
# Read-only: current status, generation, approval, channel outcomes.
python3 .github/scripts/release/release_coordinator.py inspect --state-dir ./state

# Same, plus a hint of valid next commands for the current status.
python3 .github/scripts/release/release_coordinator.py resume --state-dir ./state
```

Every mutating subcommand accepts `--expected-generation N` for
compare-and-swap: pass the generation from `inspect`/`resume` to guard
against a concurrent or duplicate invocation silently double-applying a
transition. A stale generation fails closed (exit code 3) without touching
state. All mutating commands also take an exclusive lock on
`<state-dir>/.lock`; a second concurrent invocation fails closed (exit code
4) rather than corrupting state.

## Design notes

- **Dependency-light**: the coordinator module itself imports only the
  Python standard library. Schema validation is delegated to the existing
  `scripts/validate-release-manifest.py` via `subprocess`, so the
  coordinator never imports `jsonschema` directly.
- **No rebuilds**: the coordinator only reads an already-downloaded,
  immutable producer artifact directory and copies bytes into its state
  directory. It has no code path that invokes a compiler, package manager,
  or build tool (`tests/release_coordinator_test.py::NoRebuildContract`
  greps the source for exactly this).
- **Staged bytes are never overwritten**: if a `stage` call presents a file
  with the same name as an already-staged asset but different bytes, the
  command fails closed and the original staged file is left untouched.
  Retrying `stage` with byte-for-byte identical input is a safe no-op.
- **Single publisher**: for standard `vX.Y.Z` tags,
  `release-comprehensive.yml` is a producer only and `release-sign.yml` is a
  read-only verifier. `publish-pypi.yml` may still publish the Python package
  for a bare tag, but only `python-v*`/`pypi-v*` tags may create its
  component GitHub release; npm is restricted to `nodejs-v*`, and the
  deprecated Tauri workflow is restricted to `app-v*`/`desktop-v*`. None can
  create, clobber, or augment the shared standard release inventory or update
  Homebrew.
- **Signed BOM**: the final manifest has a detached zipsign Ed25519 signature
  with a fixed context. Both GitHub and R2 publish the manifest and signature;
  R2 writes `manifest.json` last as the stable commit point.
- **Conditional-write preflight**: R2 publication checks that the installed
  AWS CLI exposes `s3api put-object --if-none-match` before the first object
  lookup or mutation (AWS CLI 2.17 or newer). Runners lacking that capability
  fail closed.
- **Atomicity**: every write (`state.json`, `manifest.json`, staged assets,
  `downstream-handoff.json`) uses temp-write + `fsync` + `os.replace` +
  directory `fsync`, so a crash mid-write cannot leave a partially written
  file on disk.
- **Path safety**: asset names from the manifest must be bare basenames
  (no `/`, no `..`, not absolute); the corresponding artifact-dir file must
  be a regular file, not a symlink.
- **Signing scope is `tar.gz` and `zip` only**: `sign-assets`/`verify`
  embed/check a real zipsign Ed25519 signature, but zipsign itself only
  understands those two container formats. A manifest asset declared with
  `format: deb`, `rpm`, `pkg.tar.zst`, `exe`, or `dmg` fails closed at
  `sign-assets`/`verify` with an explicit "no signing/verification support
  for format" error rather than being routed through `zipsign sign tar`,
  which does not validate its input is a real gzipped tar and would
  otherwise silently produce a corrupted asset that zipsign itself reports
  as successfully signed (confirmed: a real `.deb` signed this way still
  opens under `dpkg-deb` but `ar t` reports it as a malformed archive).
  Shipping a manifest with a non-`rb`, non-tar.gz/zip asset therefore
  requires either a format-appropriate signing mechanism to be added to
  the coordinator first, or that format to be added to
  `SIGNATURE_EXEMPT_FORMATS` with its own out-of-band provenance story.
