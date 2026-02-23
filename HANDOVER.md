# Handover: 2026-02-23 - v1.10.0 Release Fully Published

**Branch**: `main` (at `dd2ee59c`)
**Release**: v1.10.0 -- https://github.com/terraphim/terraphim-ai/releases/tag/v1.10.0

---

## Session Summary

Fixed the remaining crates.io publish failure for v1.10.0. The publish script's `update_versions` function had an overly broad `sed` pattern that corrupted dependency versions in multi-line `[dependencies.X]` blocks.

---

## What Was Done

### 1. Fixed crates.io Publish Script (PR #574, merged)

**Problem**: The `update_versions()` function in `scripts/publish-crates.sh` used `sed -i "s/^version = \".*\"/version = \"$VERSION\"/"` which replaced ALL lines starting with `version = "` -- not just the `[package]` version. This changed `notify`'s version from `"6.1"` to `"1.10.0"` in `terraphim_router/Cargo.toml`, causing `cargo publish` to fail with:
```
error: failed to select a version for the requirement `notify = "^1.10.0"`
```

**Fix**: Changed the `sed` command to use range addressing (`0,/pattern/` on GNU, `1,/pattern/` on BSD) so only the first occurrence (the `[package]` version line) gets replaced.

**File changed**: `scripts/publish-crates.sh` (lines 196-203)

### 2. Re-tagged v1.10.0 and Triggered Release

- Deleted remote tag `v1.10.0`
- Re-tagged at `dd2ee59c` (includes all three CI fixes)
- Deleted orphaned draft release
- Triggered workflow run 22303128846

### 3. Verified Full Release Success

All critical jobs in run 22303128846 completed successfully:

| Job | Status | Duration |
|-----|--------|----------|
| Verify version consistency | PASS | 23s |
| Build binaries (all 7 targets) | PASS | 3m36s - 9m54s |
| Build Debian packages | PASS | 5m0s |
| Build Tauri desktop (all 3 platforms) | PASS | 9m40s - 17m31s |
| Create macOS universal binaries | PASS | 35s |
| Sign and notarize macOS binaries | PASS | 2m48s |
| Create GitHub release | PASS | 25s |
| **Publish Rust crates to crates.io** | **PASS** | **8m34s** |
| Update Homebrew formulas | PASS | 5s |
| Docker 20.04 | Still running | Self-hosted runner |
| Docker 22.04 | Still running | Self-hosted runner |

---

## PRs Merged This Session (cumulative across both sessions)

| PR | Title | Fix |
|----|-------|-----|
| #558 | fix(docker): use pre-built frontend assets | Replaced in-container Node.js build with CI-built assets |
| #565 | fix(ci): use 7z instead of zip for Windows | Replaced `zip -j` with `7z a -tzip` |
| #573 | fix(ci): add missing version fields for crates.io | Added `version` to path deps, added new crates to publish list |
| #574 | fix(ci): scope publish script version sed to package section only | Scoped sed to first `version =` line only |

---

## Current State

```
Branch: main
HEAD:   dd2ee59c fix(ci): scope publish script version sed to package section only (#574)
Working tree: modified HANDOVER.md, lessons-learned.md (uncommitted)
```

---

## Blockers and Known Issues

1. **Docker builds still running**: Both Docker 20.04 and 22.04 builds are pending on self-hosted runners. Not a code problem -- runner availability issue.

2. **Tauri desktop bundles not produced**: All three Tauri builds pass, but no `.dmg`, `.AppImage`, `.deb`, `.msi`, or `.nsis` artifacts are uploaded. The Tauri bundle configuration may need updating.

3. **Diagnostics warnings** (non-blocking, pre-existing):
   - `learning_via_negativa.rs`: unused imports, unused variables, dead code
   - `advanced_routing.rs`: unused import `Latency`
   - `unified_routing.rs`: unused variable `spawner`

---

## Open Issues (Priority Order)

| Issue | Title | Priority |
|-------|-------|----------|
| #544 | Dynamic ontology extraction (parent) | High -- new feature |
| #545 | CLI extract --schema outputs SchemaSignal | High -- child of #544 |
| #546 | CLI coverage --schema --threshold --json | High -- child of #544 |
| #547 | Define ontology schema file format | High -- child of #544 |
| #548 | CLI extract includes GroundingMetadata | High -- child of #544 |
| #566 | Cross-compile Windows binaries with cargo-xwin | Medium -- CI optimization |
| #560 | TinyClaw: agent spawning via terraphim_spawner | Medium |
| #561 | TinyClaw: Document standalone architecture | Low |

---

## Next Steps (Recommended Priority)

1. **Monitor Docker builds** -- Check if self-hosted runners complete the Docker 20.04/22.04 builds.

2. **Implement issue #544** -- Design plan is complete and posted. Start with #547 (schema file format), then #545 (extract --schema), #546 (coverage --schema), #548 (grounding metadata).

3. **Fix Tauri bundle packaging** -- Investigate why no desktop bundles (dmg/AppImage/msi) are produced despite successful builds.

4. **Implement issue #566** (cargo-xwin) -- Lower priority, but will save CI costs long-term.

---

## Key Patterns Discovered This Session

- **`sed` first-match-only addressing**: Use `0,/pattern/` (GNU) or `1,/pattern/` (BSD) to replace only the first occurrence. Critical when Cargo.toml files have `version = "..."` in both `[package]` and `[dependencies.X]` sections.
- **Publish script version corruption**: The `update_versions()` function should never use a global `sed` replacement on `version = "..."` because multi-line dependency blocks have their own version lines.
- **crates.io publish takes ~8-10 minutes**: 16 crates with 60-second delays between each.
