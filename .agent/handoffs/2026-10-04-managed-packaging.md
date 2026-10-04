# Handover: Managed packaging & Omarchy (2026-10-04, updated)

## Objective

Ship the managed-package / Omarchy distribution work: Omarchy, Arch/AUR, DEB/RPM, Homebrew, central release coordinator + rehearsal contract, and the decisions taken during the session.

## Decisions taken and executed

1. **AUR** - deferred in the release-manifest contract. `terraphim-ai#3416` (merged 6ddb1e19): optional `deferred_channels` ({channel, reason, deferred_since}); only downstream channels may be deferred; deferral withholds approval.
2. **#342 ci.yml** - rework GitHub clients #18 on canonical main; Gitea #266 is its parallel port.
3. **Mirroring** - GitHub canonical; CI mirrors main + tags to Gitea and enforces a drift check. GitHub #37 (e94ff47) + Gitea #346 (69b61ad3).
4. **DEB/RPM publication** - to the GitHub release and the R2 stable channel. Coordinator now signature-exempts deb/rpm/pkg.tar.zst (`terraphim-ai#3421`, merged 0a969a9a) and the operator path is documented (`terraphim-ai#3424`, merged a7683d49).
5. **Stale PRs** - only release/CI ones.

## Shipped (merged)

- **Channel deferral contract** - terraphim-ai#3416 (6ddb1e19).
- **Forge mirroring + drift check** - clients GitHub#37 (e94ff47), Gitea#346 (69b61ad3); invariant verified (`.github`+`scripts` identical).
- **Homebrew tap receiver** - homebrew-terraphim#5 (13d96c96): repository_dispatch receiver + `scripts/build-release-proof.py` (terminal proof artifact).
- **Coordinator publishes deb/rpm** - terraphim-ai#3421 (0a969a9a) + operator doc #3424 (a7683d49); coordinator suite 60/60.
- **PKGBUILD contract fix** - clients GitHub#38 (eaf8def9), Gitea#347 (588073fe): ported `pkgbuilds/` + the test to GitHub (PR #36 had mirrored only the workflow) and switched to `unittest discover`. Workflow now passes on GitHub.
- **Omarchy recipe** - omacom/omarchy-pkgs#751 open, MERGEABLE, awaiting maintainer `build-approved`. `source: local`, no AUR dependency.
- **DEB/RPM producer** - verified locally end-to-end (Debian + Fedora), receipts `dpkg`/`rpm`, no shadow binary.

## In progress

- **clients#18** - rebased onto canonical main, `ci.yml` conflict resolved (canonical jobs + V-model jobs), and the release-CI contract failure fixed by keeping the canonical `build` job name. Head 93eaeb8; CI run 37193484440 in progress. When green: merge, then retire/close Gitea #266.

## Blockers

- **AUR registration paused** (external). `terraphim-clients-bin` unclaimed. The contract deferral path above is the sanctioned workaround; do not poll the AUR page.
- **#3336/#3338** code merged; Gitea refuses to close while #3382 is open.
- **omarchy #751 / #249 / #245** await the omarchy-pkgs maintainer.

## Next actions

1. Confirm clients#18 CI green; merge; close Gitea #266.
2. Run a rehearsal with the manifest declaring AUR deferred (contract now supports it).
3. When AUR registration reopens, claim `terraphim-clients-bin` and clear the deferral.
4. Exercise the Homebrew tap receiver with a live coordinator dispatch.

## Key artifacts

- `target/rehearsal-1.21.16/` - rehearsal report, evidence bundle, coordinator state.
- `target/managed-packages-1.21.16/` - four verified DEB/RPM packages + SHA256SUMS.
