# Private-source LSP binary releases

This control path releases six frozen LSP binaries from reviewed private source. It does not mirror private source or history. Existing same-source publication remains unchanged in `publish_lsp_release.py`; `lsp_handoff.py` explicitly separates the public control commit from the private build commit.

## One-time bindings

First create the organization runner group `lsp-private`, restricted to this repository and exactly `terraphim/terraphim-ai/.github/workflows/lsp-producer.yml@refs/heads/main`. Run its Linux/macOS agents under dedicated OS accounts/workspaces inaccessible to untrusted PR jobs; a custom label alone does not provide that isolation. Existing physical hosts may be reused only after this isolation is verified. Independently read back group policy and runner account/workspace configuration before binding source credentials. The workflows require this group and contain no generic-runner fallback. The initial metadata probe returned HTTP 403 for the current account, so the group and this organization's selected-workflow feature availability are **not verified**. [GitHub runner-group access documentation](https://docs.github.com/en/enterprise-cloud@latest/actions/how-tos/manage-runners/self-hosted-runners/manage-access) describes selected-workflow restrictions.

Create the GitHub environment `lsp-producer` with a custom deployment policy containing exactly the `main` branch. Bind `GITEA_LSP_SOURCE_TOKEN` to a dedicated read-only token covering Git/API reads for `terraphim/terraphim-ai` and the pinned `terraphim/terraphim-editor` dependency. Bind `CARGO_REGISTRIES_TERRAPHIM_TOKEN` with its required `Bearer ` prefix. The workflows never fall back to broad Gitea or 1Password credentials. The producer runtime verifies source/tag/required Gitea CI and the private build-recipe hash. Environment and credential availability must be verified before a real run; checked-in code is not evidence those bindings exist.

Create `lsp-releases` with a custom deployment policy containing exactly the `terraphim_lsp-v*` tag pattern. Only its hosted publisher has `contents: write`. Its `actions: read` permission reads producer and artifact metadata. The publisher receives no private-source credentials. The producer's resolve job checks the environment policy with `actions: read`; build jobs have only `contents: read`.

The producer runs only from reviewed public main using `workflow_dispatch`, on isolated self-hosted Linux and macOS agents in `lsp-private`. No PR-triggered job reads private source. The initial recipe deliberately builds unsigned macOS binaries; it does not claim Developer ID signing or notarization.

## Reviewed request, build and handoff

1. Normal-merge the private build recipe, including its tests. Choose a new version absent from private core tags, internal LSP releases and public releases. `rc.3` already identifies an older internal build; do not reuse it. Create the exact private source tag only after source review and required CI. This document creates no tag.
2. Add `release/lsp/requests/<tag>.json` through a normal public PR, following `docs/contracts/schemas/lsp-source-request.schema.json`. It pins the private source SHA/tag, toolchain, CI contexts and the exact build-recipe path/hash. No production request or invented source identity is included with this framework.
3. Dispatch **Build private-source LSP artifact** at public main using that tag. Qualification runs the private recipe tests, pinned Rust formatting/Clippy/core and LSP tests before any binary build lane. Each lane fetches the reviewed private source, verifies the recipe hash, uses an isolated Cargo cache, and produces only expected binaries plus a strict receipt. The supported matrix is Linux GNU x86_64, Linux musl x86_64/aarch64, and macOS x86_64/aarch64/universal. Native architectures verify `--version`; cross architectures receive explicit architecture checks.
4. Raw checkout, compiler/test logs and private failure receipts stay under the runner owner's mode-0700 `~/.local/state/terraphim-lsp-private/producer-*`. The ephemeral askpass script is removed. Public outcomes use fixed error codes; only hashes and known receipt fields leave the private lane. These logs need deliberate owner retention/cleanup after release review. They are not uploaded as public Actions artifacts.
5. Each public lane receipt binds the producer repository/workflow/control commit/run/attempt. Assembly refuses mixed attempts, extra files, altered bytes or failed qualification. Re-run **all jobs** after a failure; artifacts from earlier attempts cannot satisfy later attempts. The final immutable artifact contains exactly six raw binaries, `checksums.txt` and `lsp-release-manifest.json`.
6. Download and independently review that artifact, then add `release/lsp/handoffs/<tag>.json` in a separate normal PR. The handoff pins its artifact ID/digest, producer run/attempt/control commit, and manifest hash. `requestSha256` is SHA-256 of canonical JSON (sorted keys, compact separators); duplicate JSON keys are rejected. The artifact digest is GitHub's ZIP SHA-256. The manifest and every binary also have independent byte hashes. Metadata fixtures in tests are not a claim of successful remote publishing.
7. Normal-merge the handoff, then create the public release tag on that reviewed public main commit. The tag-only publisher requires exact main ancestry, tag target, workflow identity, completed successful producer and exact artifact. Expired artifacts, reruns, moved tags and mismatched hashes fail closed. It downloads the frozen ZIP with bounded extraction, executes no private source, and uses the unchanged draft/resume publisher. Existing published bytes are never replaced.
8. Inspect the publication receipt and anonymously download every public asset. Only exact byte readback is a successful release. Install that exact released LSP in stock Zed and rerun host acceptance before updating the extension's immutable provisioning pin and preparing its upstream catalog handoff.

## Verification

Use the existing pinned requirements in `scripts/requirements-release-manifest.txt` and `scripts/requirements-release-workflow-tests.txt`. Run:

```sh
python -m unittest -v tests.lsp_handoff_test tests.lsp_private_producer_test tests.workflow_lsp_handoff_test tests.workflow_release_recovery_contract_test
actionlint .github/workflows/lsp-producer.yml .github/workflows/lsp-publish.yml .github/workflows/release-manifest-contract.yml
```

These tests use real Git histories, files, subprocesses and ZIP archives. Metadata inputs are explicit fixtures. They prove refusal and orchestration contracts; six-target compilation, source credential access, native host acceptance and public release readback remain separate actual-output gates.
