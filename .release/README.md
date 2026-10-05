# `.release/`

Canonical, machine-checked release contracts. These files are consumed
directly by tooling and tests -- treat any change here as a breaking-change
review, not a docs edit.

- [`release-manifest.schema.json`](./release-manifest.schema.json) -- the
  JSON Schema (Draft 2020-12) every release manifest must satisfy. Validated
  by `scripts/validate-release-manifest.py` and exercised by
  `tests/release_manifest_validator_test.py`.

Operator-facing documentation lives in
[`docs/src/domains/release/README.md`](../docs/src/domains/release/README.md).
The central release coordinator that stages, verifies, and approval-gates
promotion against this schema is documented in
[`.github/workflows/README_RELEASE_COORDINATOR.md`](../.github/workflows/README_RELEASE_COORDINATOR.md).

## Downstream Homebrew outbox

The `homebrew_tap_pr` downstream channel is served by an idempotent outbox:

- [`scripts/generate-homebrew-formulas.py`](../scripts/generate-homebrew-formulas.py)
  renders the tap formulas from a validated release manifest. It is a pure,
  network-free function: every asset is resolved by exact name and must exist
  exactly once, be non-empty, carry a lowercase 64-hex SHA-256, and match the
  declared target/os/arch. The same manifest always renders the same bytes.
- [`scripts/homebrew-outbox.sh`](../scripts/homebrew-outbox.sh) applies the
  rendered formulas to a tap checkout and, only with `--dispatch`, commits
  them on a branch and opens/updates a PR in `terraphim/homebrew-terraphim`.
  It never touches the central GitHub/R2 publication state, and a re-run
  against an already-current tap is a no-op.
- Formula specs and templates live in
  [`config/homebrew/`](../config/homebrew/). Add a formula by adding a spec
  entry and a template; components without a spec are ignored.
