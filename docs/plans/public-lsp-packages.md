# Public LSP packages

The language server and its shared WASM analysis core use a bounded workspace
at `crates/terraphim_lsp/Cargo.toml`. The root workspace excludes only these two
packages; all other members and their dependency configuration are preserved.
This keeps the public LSP dependency graph independent of unrelated packages
and private registry patches in the main application workspace.

## Source and package boundary

The source snapshot comes from Terraphim core commit
`440f320e6447337aa9b58827719f3406402f9490`, limited to `terraphim_lsp` and
`terraphim_lsp_core`. It brings the existing graph export, saved alternatives,
Lab, Trim, re-anchoring and clean Markdown behaviour into the public source
repository. Existing analysis and returned-edit behaviour is preserved. Package preparation
changes metadata and fixture include paths; the additive
`terraphim.alternative.add_and_apply` command also supports generic command
pickers by requesting a versioned edit and confirming the matching document
change. The original `alternative.add` result remains unchanged.

- `terraphim_lsp_core` is version `1.23.1`, with public dependencies on types
  `1.22.3`, automata `2.1.0`, alternatives `0.1.1` and Lab `0.1.1`.
- `terraphim_lsp` is the `1.22.0-rc.4` candidate and depends on core `1.23.1`.
- Both packages restrict publication to crates.io and include the Apache-2.0
  licence notice and their own test fixtures. No private Git history, registry
  credentials or operational files are part of the snapshot.

The LSP fixtures copied from the shared core are byte-identical. Keeping them
inside each crate allows the tests to run after a `.crate` is extracted; a
successful source-workspace build alone cannot establish that property.

## Verification

Run from the repository root:

```sh
cargo test --manifest-path crates/terraphim_lsp/Cargo.toml --workspace
cargo clippy --manifest-path crates/terraphim_lsp/Cargo.toml --workspace --all-targets -- -D warnings
cargo fmt --manifest-path crates/terraphim_lsp/Cargo.toml --all -- --check
cargo build --manifest-path crates/terraphim_lsp/Cargo.toml -p terraphim_lsp_core --target wasm32-unknown-unknown
cargo package --manifest-path crates/terraphim_lsp/Cargo.toml -p terraphim_lsp_core
cargo package --manifest-path crates/terraphim_lsp/Cargo.toml -p terraphim_lsp
```

Publish and verify the dependency packages before qualifying the server with
crates.io alone. Local development may use an external Cargo `--config` with
explicit path patches to those public-source candidates; keep that evidence
separate from registry-only qualification. Never commit a lockfile generated
with local path patches as the release lockfile. Generate the final lockfile
from the public registry, review it, then verify and publish the exact reviewed
source through the release workflow.

Qualification must include tests from extracted package files, a real stdio
server session, and saved synonym graph output. Publication is only complete
after public download checksums and released-binary behaviour are verified.

## Release automation

`release/lsp-crates.json` is the ordered package allowlist. The
`publish-lsp-crates.yml` workflow runs on an exact package tag already contained
in public `main`. It qualifies a clean checkout, checks the standalone lockfile
for public registry sources, builds the package, and tests the extracted bytes
outside the source checkout. A second job reproduces the qualified archive
before requesting publication credentials.

Publish `terraphim_lsp_core-v1.23.1` first, then
`terraphim_lsp-v1.22.0-rc.4`. Existing crates use the `crates-io` environment and
crates.io trusted publishing. A first-ever crate name uses a deliberate manual
run on its exact tag with `auth_mode=bootstrap` and the `crates-io-bootstrap`
environment. This temporary path reads the existing `CARGO_REGISTRY_TOKEN`
secret; failed trusted publishing never falls back automatically.

Both paths compare the published registry checksum and anonymously downloaded
`.crate` bytes with the qualified archive. The reusable `lsp-publish.yml`
binary workflow is called only after successful server-package readback. It
builds the released public crate and retains the established six binary asset
names. The shared immutable release publisher refuses tag moves, conflicting
assets and changes to a published release.

The workflow definitions are preparation until their exact source commits,
public registry outputs and downloadable binaries have passed the recorded
release checks. Local candidate tests do not substitute for those readbacks.

## Pull-request qualification

`lsp-public-ci.yml` qualifies the standalone Rust workspace on pull requests
and matching source pushes with read-only permissions and public registry
access. It checks the reviewed lock, formatting, all-target native tests,
doctests, Clippy and the shared core's WASM target. It packages the core with
Cargo verification enabled and runs tests again from the extracted archive.

The server's package file list is checked before merge. Its fully verified
registry package follows the core publication: Cargo must resolve the
normalized `terraphim_lsp_core` dependency from crates.io at that point. The
server release workflow performs that final packaging and extracted test gate.
Neither a skipped verification nor a local dependency patch is a substitute.

## Existing release paths

The comprehensive server release retains its bundled LSP artifact names and
root `target` output directory, and now builds the nested LSP manifest explicitly.
It refuses LSP-specific release tags. The legacy arbitrary-crate publisher also
refuses both LSP packages before reading publication credentials; neither is an
alternative route around the package allowlist and registry readback.

The earlier private-source producer remains manually dispatchable for its
separate provenance work, but it does not publish releases. The public binary
publisher accepts only the reviewed crate workflow call and no longer consumes
private producer artifacts. The RC4 release uses public package provenance.
