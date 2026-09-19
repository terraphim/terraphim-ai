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
