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

## Channel deferral

`required_channels` is the full channel set a release must serve. A downstream
channel may be temporarily deferred with an explicit, reasoned entry:

    "deferred_channels": [
      {"channel": "aur_terraphim_clients_bin", "reason": "AUR registration paused", "deferred_since": "2026-10-03"}
    ]

Rules (enforced by `scripts/validate-release-manifest.py`):

- only `downstream_channels` may be deferred (`homebrew_tap_pr`,
  `aur_terraphim_clients_bin`, `omarchy_terraphim_clients_bin`); the central
  channels are never deferrable;
- each channel appears at most once and must carry a non-blank reason;
- deferral does not remove the channel from `required_channels`; it records a
  time-bound exception. The rehearsal aggregator
  (`scripts/rehearse-managed-release.py`) reads these entries and withholds
  the approval digest while any channel is deferred, so a deferred release
  can never authorize promotion.
