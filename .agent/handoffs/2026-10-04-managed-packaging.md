# Handover: Managed packaging & Omarchy (2026-10-04)

## Objective

Ship the managed-package / Omarchy distribution work: Omarchy, Arch/AUR, DEB/RPM, Homebrew, and the central release coordinator + rehearsal contract.

## Shipped (merged)

- **Omarchy**: `omacom/omarchy-pkgs#751` open, MERGEABLE, awaiting maintainer `build-approved`. Recipe is `source: local` (no AUR dependency).
- **Arch/AUR recipe**: `terraphim-clients#343` merged (`6d34429b`) -> `pkgbuilds/terraphim-clients-bin/{PKGBUILD,.SRCINFO,verify.sh,README.md}`, `tests/test_pkgbuild_release_contract.py`, `.github/workflows/pkgbuild-contract.yml`. AUR push still blocked.
- **Homebrew outbox**: `terraphim-ai#3407` merged (`62df058f`) -> generator + idempotent outbox + tests. Homebrew formula test bug (stale CLI assertions) fixed in `terraphim-ai#3415` (`ca11e3fb`) and `homebrew-terraphim#4` (`70ea71af94`); `brew test` now passes on macOS for agent and grep (issue #340 closed).
- **Rehearsal (#3382)**: `terraphim-ai#3414` merged (`c60ff9eb`) -> `scripts/rehearse-managed-release.py` (aggregates channel evidence, never copies bytes, withholds approval on deferral).
- **#342 reconciliation**: `.github` + `scripts` made identical between GitHub/Gitea (`clients#344` + `clients#35` + `clients#36`); re-verified after later merges.
- **DEB/RPM**: canonical producer verified locally end-to-end; 4 packages + SHA256SUMS built and installed/removed cleanly on Debian and Fedora, receipts `dpkg`/`rpm`, no shadow binary. Not yet published.

## Decisions taken (2026-10-04)

1. **AUR**: defer `aur_terraphim_clients_bin` in the manifest contract. Implemented in `terraphim-ai#3416` (optional `deferred_channels`; deferral withholds approval).
2. **#342 ci.yml**: rework GitHub `clients#18` on canonical main (it currently fails CI and is dirty); Gitea `#266` is its parallel port.
3. **Mirroring**: GitHub canonical; mirror merges/tags to Gitea via CI (`push` to both) with a drift check.
4. **DEB/RPM publication**: publish to the GitHub release **and** the R2 stable channel.
5. **Stale PRs**: only touch release/CI-related ones (#18/#266).

## Blockers

- **AUR registration paused** (external). `terraphim-clients-bin` unclaimed; no AUR identity available. Do not poll the page.
- **clients#18** fails CI (`check`) and is `mergeable_state: dirty`; needs a rework pass on GitHub before it can land.
- **#3336/#3338** code merged but Gitea refuses to close (open dependents: #3382).
- **omarchy `#751`** and **#249/#245** await the omarchy-pkgs maintainer.

## Open PRs at handover

- `terraphim-ai#3416` - channel deferral contract (open, mergeable).
- `terraphim-clients#18` - V-model CI pipeline (failing/dirty; rework).
- `omacom/omarchy-pkgs#751` - downstream Omarchy package (maintainer).

## Next actions

1. Land `terraphim-ai#3416`, then run a rehearsal with the manifest declaring AUR deferred.
2. Rework `clients#18` on canonical main; get CI green; mirror to Gitea; retire `#266`.
3. Add the CI push-to-both mirror workflow + drift check.
4. Implement DEB/RPM publication to the GitHub release + R2.
5. Add the Homebrew tap-side receiver workflow for the coordinator repository_dispatch.
6. Re-run the rehearsal with a complete candidate once AUR is available.

## Key local artifacts

- `target/rehearsal-1.21.16/` - rehearsal report, evidence bundle, coordinator state.
- `target/managed-packages-1.21.16/` - the four verified DEB/RPM packages + SHA256SUMS.
