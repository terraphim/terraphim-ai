# Research Document: macOS Code Signing and Notarization Failure

**Status**: Draft
**Author**: Terraphim AI
**Date**: 2026-04-21
**Reviewers**: [Pending]

## Executive Summary

The `sign-and-notarize-macos` job in `release-comprehensive.yml` failed during a release workflow run on a self-hosted macOS runner (2026-04-21 14:35:51Z). The job loads Apple Developer credentials from 1Password, creates a temporary keychain, signs binaries with `codesign`, and submits them to Apple's notarization service. Without the actual error log, we analyse the failure surface and identify the most probable root causes based on the pipeline design, self-hosted runner state, and documented failure modes.

## Essential Questions Check

| Question | Answer | Evidence |
|----------|--------|----------|
| Energizing? | Yes | Blocking releases; signed binaries are a distribution commitment |
| Leverages strengths? | Yes | Existing 1Password + self-hosted runner infrastructure |
| Meets real need? | Yes | macOS Gatekeeper requires notarization for user trust |

**Proceed**: Yes — 3/3 YES

## Problem Statement

### Description
The `sign-and-notarize-macos` job (lines 368-441 of `.github/workflows/release-comprehensive.yml`) fails after the `create-universal-macos` job succeeds. The job runs on a self-hosted macOS runner (`[self-hosted, macOS]`). The failure prevents signed universal binaries from reaching the release artifacts.

### Impact
- macOS users cannot download signed/notarized binaries
- Homebrew formulas reference unsigned binaries (security warning on first run)
- Release pipeline partially fails (the `create-release` job has `always()` with conditional success, so it may still publish unsigned binaries)

### Success Criteria
- `sign-and-notarize-macos` completes successfully
- `codesign --verify --deep --strict` passes on both binaries
- `xcrun notarytool submit --wait` returns "Accepted"
- Signed binaries are uploaded as artifacts

## Current State Analysis

### Existing Implementation

**Workflow**: `.github/workflows/release-comprehensive.yml`
- Job: `sign-and-notarize-macos` (lines 368-441)
- Depends on: `create-universal-macos`
- Runner: `[self-hosted, macOS]`

**Script**: `scripts/sign-macos-binary.sh`
1. Create temporary keychain (`$RUNNER_TEMP/signing.keychain-db`)
2. Decode base64 p12 certificate → `$RUNNER_TEMP/certificate.p12`
3. Import certificate into keychain with `security import`
4. Set key partition list for `codesign` access
5. Add keychain to search list
6. Find signing identity (`Developer ID Application`)
7. Sign binary with `codesign --sign --options runtime --timestamp`
8. Verify signature with `codesign --verify`
9. Create ZIP with `ditto -c -k`
10. Submit to Apple with `xcrun notarytool submit --wait`
11. Fetch notarisation log with `xcrun notarytool log`
12. Gatekeeper check with `spctl --assess`
13. Clean up keychain and certificate

**Credential Source**: 1Password CLI (`op read`)
- Vault: `TerraphimPlatform`
- Items:
  - `apple.developer.credentials`: `username`, `APPLE_TEAM_ID`, `APPLE_APP_SPECIFIC_PASSWORD`
  - `apple.developer.certificate`: `base64`, `password`
- Secret: `OP_SERVICE_ACCOUNT_TOKEN` (GitHub secret)

**Cleanup** (in `build-binaries` job, lines 132-142):
```bash
find /tmp -name "*.keychain-db" -mmin +60 -delete
find /tmp -name "signing.keychain*" -delete
find /tmp -name "certificate.p12" -delete
rm -rf ~/actions-runner/_work/terraphim-ai/terraphim-ai/target/release/*.zip
```

### Code Locations

| Component | Location | Purpose |
|-----------|----------|---------|
| Workflow job | `.github/workflows/release-comprehensive.yml:368-441` | Orchestrates signing |
| Signing script | `scripts/sign-macos-binary.sh` | Reusable signing logic |
| Release docs | `docs/RELEASE_PROCESS.md:170-186` | Troubleshooting guide |
| Cleanup step | `.github/workflows/release-comprehensive.yml:132-142` | Pre-build runner cleanup |

### Data Flow

```
1Password (op read)
    ↓
GitHub Actions env vars (APPLE_ID, TEAM_ID, APP_PASS, CERT_BASE64, CERT_PASS)
    ↓
scripts/sign-macos-binary.sh
    ↓
Temporary keychain + imported certificate
    ↓
codesign --sign (local signing)
    ↓
xcrun notarytool submit --wait (Apple notarization API)
    ↓
Signed binary artifact upload
```

### Integration Points

- **1Password**: Service account token required; CLI must be installed on runner
- **Apple Developer**: Valid Developer ID Application certificate; active Apple ID; valid app-specific password
- **Self-hosted runner**: macOS with Xcode Command Line Tools; `codesign`, `security`, `xcrun notarytool` available

## Constraints

### Technical Constraints
- **Self-hosted runner state**: Previous runs may leave stale keychains, certificates, or build artifacts. Cleanup is best-effort (`-mmin +60` may miss recent stale files).
- **Apple notarization API**: Network-dependent; can reject binaries for policy reasons (hardened runtime, entitlements, unsigned dependencies).
- `xcrun notarytool` requires macOS 12+ or Xcode 13+.
- Certificate must be of type **Developer ID Application** (not Mac Development or Distribution).

### Business Constraints
- Apple Developer Program membership must be active ($99/year).
- Developer ID certificates expire after 5 years (renewal required).
- App-specific passwords do not expire but can be revoked.

### Non-Functional Requirements

| Requirement | Target | Current (unknown) |
|-------------|--------|-------------------|
| Signing latency | < 2 min | Unknown |
| Notarization wait | < 10 min | Unknown |
| Job total time | < 15 min | Fails before completion |

## Vital Few (Essentialism)

### Essential Constraints (Max 3)

| Constraint | Why It's Vital | Evidence |
|------------|----------------|----------|
| Valid Apple Developer ID certificate | Without this, `codesign` and notarization both fail | Certificate is 5-year expiry; last generated Dec 2025 |
| 1Password service account token + CLI | All credentials flow through this; if `op read` fails, everything is empty | `OP_SERVICE_ACCOUNT_TOKEN` is a GitHub secret |
| Self-hosted runner environment | `codesign`, `security`, `xcrun notarytool` must exist and work | Runner version: 2.333.1 |

### Eliminated from Scope

| Eliminated Item | Why Eliminated |
|-----------------|----------------|
| Switch to GitHub-hosted macOS runner | Self-hosted is required for custom hardware/access; not a quick fix |
| Replace notarytool with notarytool + keychain profile | Would require storing credentials in macOS keychain, not 1Password; more complex |
| Add entitlements file | Hardened runtime is already used (`--options runtime`); no entitlement customisations needed yet |

## Dependencies

### Internal Dependencies
| Dependency | Impact | Risk |
|------------|--------|------|
| `create-universal-macos` | Must produce valid universal binaries | Low — succeeded in this run |
| `build-binaries` (macOS) | Must produce x86_64 and aarch64 binaries | Low — succeeded |

### External Dependencies
| Dependency | Version | Risk | Alternative |
|------------|---------|------|-------------|
| Apple Developer Program | Active | High if expired | Re-enrol |
| 1Password CLI | v2+ | Medium if outdated | Manual secret injection |
| Xcode / CLT | 13+ | Medium if missing | Install via `xcode-select --install` |
| Self-hosted runner | 2.333.1 | Low | GitHub-hosted (limited) |

## Risks and Unknowns

### Known Risks

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Certificate expired | Medium | High | Check expiry with `security find-certificate -c "Developer ID Application" -p | openssl x509 -noout -dates` |
| 1Password `op read` fails (token expired, network, item moved) | Medium | High | Add `set -e` + explicit error checking on every `op read` |
| Stale keychain from previous run | Medium | Medium | Clean up ALL keychains in `/tmp`, not just >60min old |
| App-specific password revoked | Low | High | Regenerate in Apple ID settings |
| Notarization rejected (policy) | Low | Medium | Check notarization log for specific rejection reason |
| Self-hosted runner missing `notarytool` | Low | High | Verify `xcrun notarytool --version` |

### Open Questions

1. **What is the exact error message?** — The user only reported job name + "Failed". We need the step-level error log to narrow to certificate, signing, or notarisation phase.
2. **Does `op read` succeed?** — If 1Password CLI fails, env vars are empty and all subsequent steps fail silently or with confusing errors.
3. **Is the certificate still valid?** — Generated Dec 2025; could be revoked or the base64 corrupted in 1Password.
4. **Is the self-hosted runner's macOS version compatible?** — `notarytool` requires macOS 12+ / Xcode 13+.
5. **Were there previous failed runs leaving state?** — Cleanup only removes keychains >60 minutes old; a recent failure could leave a conflicting keychain.

### Assumptions Explicitly Stated

| Assumption | Basis | Risk if Wrong | Verified? |
|------------|-------|---------------|-----------|
| `OP_SERVICE_ACCOUNT_TOKEN` is valid and not expired | No rotation mentioned; last successful signing was Dec 2025 | If expired, all `op read` calls fail silently | No |
| Developer ID certificate in 1Password is valid | Last generated Dec 2025; Apple certs are 5-year | If expired/revoked, `security import` or `codesign` fails | No |
| Self-hosted runner has Xcode CLT installed | Runner was used for successful builds | If `notarytool` missing, notarization fails | No |
| Base64 certificate has no newlines | Fixed in commit `a2c23a9c` with `tr -d '\n'` | If 1Password item was re-saved with newlines, decode fails | No |
| Apple ID and app-specific password are valid | Last used Dec 2025 | If password revoked or Apple ID locked, notarization fails | No |

### Multiple Interpretations Considered

| Interpretation | Implications | Why Chosen/Rejected |
|----------------|--------------|---------------------|
| **A. Certificate or credential expired** | Requires renewing Apple Developer cert or regenerating app-specific password | Most likely — 5-month gap since last use; Apple credentials are common failure point |
| **B. Stale self-hosted runner state** | Requires better cleanup or runner restart | Likely — cleanup is time-based (`-mmin +60`), not exhaustive |
| **C. 1Password service account token expired** | Requires regenerating token in 1Password and updating GitHub secret | Possible — service account tokens can expire or be revoked |
| **D. Notarization policy rejection** | Requires fixing binary (entitlements, dependencies) | Less likely — binaries haven't changed structurally; would need notarization log |
| **E. Network/connectivity to Apple** | Transient; retry would succeed | Unlikely — Apple notarization is generally reliable |

## Research Findings

### Key Insights

1. **The failure point is unknown without the log**. The job has 5 distinct steps (checkout, download artifacts, load credentials, sign server, sign agent). Any of these could fail.

2. **Credential loading is the most fragile step**. Five separate `op read` calls feed into env vars. The workflow does not validate that env vars are non-empty before proceeding. Empty `CERT_BASE64` would cause `base64 --decode` to produce an empty file, leading to a cryptic `security import` error. Empty `APPLE_ID`/`TEAM_ID`/`APP_PASS` would cause `notarytool submit` to fail with authentication errors.

3. **Cleanup is insufficient for rapid re-runs**. The `build-binaries` cleanup uses `-mmin +60`, meaning a keychain from a failed run 30 minutes ago would persist. The signing script itself uses `set -euo pipefail` and creates a keychain at `$RUNNER_TEMP/signing.keychain-db`. If a previous run was killed mid-script (e.g., runner disconnect, timeout), the keychain may still exist, causing `security create-keychain` to fail with "A keychain with the same name already exists".

4. **The signing script has a brittle notarization log retrieval**. Lines 79-88 parse `xcrun notarytool history` output with `grep -m1 "id:" | awk '{print $2}'`. If the history output format changes or is empty, `SUBMISSION_ID` is empty and the subsequent `notarytool log` command fails. This is a secondary failure after successful submission.

5. **No retry logic for notarization**. Apple's notarization service occasionally returns transient errors (503, timeout). The script uses `--wait` but does not retry on failure.

### Relevant Prior Art

- **Commit `a2c23a9c`** (Dec 2025): Fixed base64 newline handling — evidence that 1Password certificate storage was already problematic.
- **Commit `2529a878`** (Jan 2026): "fix(release): restore self-hosted macOS x86_64 runner" — evidence of runner instability.
- **Docs `RELEASE_PROCESS.md:170-186`**: Documents known issues (stale keychain, base64 input, notarization rejection).

### Technical Spikes Needed

| Spike | Purpose | Estimated Effort |
|-------|---------|------------------|
| **Spike 1: Reproduce signing locally** | Run `scripts/sign-macos-binary.sh` with current 1Password credentials on a macOS machine to identify which step fails | 15 min |
| **Spike 2: Check certificate validity** | Inspect certificate expiry and validity with OpenSSL | 5 min |
| **Spike 3: Inspect runner state** | SSH to self-hosted runner, check for stale keychains, verify `xcrun notarytool --version`, check 1Password CLI version | 10 min |
| **Spike 4: Review actual workflow log** | Find the specific step error in GitHub Actions UI or runner logs | 5 min |

## Recommendations

### Proceed/No-Proceed
**Proceed** — This is a blocking release issue with a well-understood failure surface. The fix is likely small (credentials or cleanup).

### Scope Recommendations
1. **Immediate**: Diagnose the exact failure via workflow log + local reproduction.
2. **Short-term**: Harden the signing script with validation, better cleanup, and retry logic.
3. **Long-term**: Consider migrating to GitHub-hosted macOS runners for signing (eliminates state drift), or use `notarytool` keychain profiles to reduce credential exposure.

### Risk Mitigation Recommendations
- Add `set -e` and explicit empty-check after every `op read` in the workflow.
- Change cleanup from `-mmin +60` to unconditional deletion of known keychain paths.
- Add `security delete-keychain` at the start of the signing script (idempotent cleanup).
- Cache `notarytool` submission ID more robustly (use `--output-format json` and `jq`).
- Add a 1-minute sleep + retry loop for notarization submission.

## Next Steps

1. **Retrieve the actual error log** from the failed workflow run (GitHub Actions UI or runner logs).
2. **Run Spike 1** (local reproduction) if a macOS machine with 1Password access is available.
3. **Run Spike 3** (runner inspection) via SSH to the self-hosted runner.
4. Based on findings, proceed to **Phase 2 (Design)** for the specific fix.

## Appendix

### Reference Materials
- `.github/workflows/release-comprehensive.yml` — workflow definition
- `scripts/sign-macos-binary.sh` — signing script
- `docs/RELEASE_PROCESS.md` — troubleshooting guide
- Apple Notarization Guide: https://developer.apple.com/documentation/security/notarizing_macos_software_before_distribution

### Code Snippets

**Fragile credential loading (current)**:
```yaml
- name: Load signing credentials from 1Password
  env:
    OP_SERVICE_ACCOUNT_TOKEN: ${{ secrets.OP_SERVICE_ACCOUNT_TOKEN }}
  run: |
    echo "APPLE_ID=$(op read 'op://.../username' --no-newline)" >> $GITHUB_ENV
    # No validation that op read succeeded
```

**Fragile notarization log parsing (current)**:
```bash
SUBMISSION_ID=$(xcrun notarytool history ... | grep -m1 "id:" | awk '{print $2}')
# Fails silently if output format changes or history is empty
```

**Incomplete cleanup (current)**:
```bash
find /tmp -name "*.keychain-db" -mmin +60 -delete  # Misses recent stale files
find /tmp -name "signing.keychain*" -delete         # Only /tmp, not $RUNNER_TEMP
```
