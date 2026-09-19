# Release Domain

This section covers release processes, signing/notarization, and changelog history.

## What belongs here

- Release pipeline and tooling
- Artifact signing and verification
- Versioning and changelog

## Release coordinator

The central release coordinator owns the single `planned -> staged ->
verified -> promoting -> promoted|failed|superseded` lifecycle across the
GitHub release / R2 stable manifest central channels and the
Homebrew/AUR/Omarchy downstream channels. Its durable record separates
source landing, CI verification, release creation, package production,
approval/promotion, publication, and terminal verification. Standard release
assets are published only by the coordinator, and its detached-signed manifest
is the sole machine-readable BOM. See
[`.github/workflows/README_RELEASE_COORDINATOR.md`](../../../../.github/workflows/README_RELEASE_COORDINATOR.md)
for the operator flow and recovery procedures, and
[`.release/release-manifest.schema.json`](../../../../.release/release-manifest.schema.json)
for the manifest contract it validates against.

Downstream repositories receive a versioned, correlation-bound dispatch and
must return a terminal proof artifact bound to the same tag, manifest digest,
channel, correlation ID, and exact asset set. Cross-repository tokens need
Actions/Contents read access plus repository-dispatch write access on their
targets; the workflow fails closed when those scopes or proofs are absent.

## Reports

- [Final Validation Status (v1.0.0)](./reports/final-validation-status.md)

## Case studies

- [v1.0.0 CI + Validation Case Study](https://terraphim.ai/posts/v1-0-0-ci-validation-case-study/)
- [Case Studies Index](../../case-studies/README.md)
