# Implementation Plan: Harden macOS Code Signing and Notarization

**Status**: Draft
**Research Doc**: [research-macos-signing-failure-2026-04-21.md](./research-macos-signing-failure-2026-04-21.md)
**Author**: Terraphim AI
**Date**: 2026-04-21
**Estimated Effort**: 2-3 hours

## Overview

### Summary
Defensively harden the `sign-and-notarize-macos` job to eliminate the four most common failure modes identified in research: fragile credential loading, insufficient cleanup, brittle notarisation log parsing, and lack of retry for transient Apple API failures. The fix is designed to work regardless of the specific error from the 2026-04-21 failure, while adding diagnostics that make any future failure immediately actionable.

### Approach
Treat the signing script as a critical path that must fail fast with clear messages. Add validation at every step where external state (1Password, keychain, Apple API) is touched. Replace text scraping with structured JSON parsing. Add idempotent cleanup so stale state from previous runs cannot poison new runs.

### Scope

**In Scope:**
- Validate all 1Password credential reads before use
- Unconditional, idempotent keychain cleanup at script start
- JSON-based notarisation log retrieval (replaces `grep | awk`)
- Retry loop for `notarytool submit` with exponential backoff
- Pre-flight validation step (certificate expiry, tool availability)

**Out of Scope:**
- Migrating from self-hosted to GitHub-hosted macOS runners
- Changing from 1Password to another secret manager
- Adding entitlements or custom signing configurations
- Refactoring the broader `release-comprehensive.yml` workflow

**Avoid At All Cost:**
- Complex retry frameworks or external retry crates — three attempts with `sleep` is sufficient
- Re-architecting the entire release pipeline — this is a targeted hardening fix
- Mock-based tests for shell scripts — test by running the actual script

## Architecture

### Component Diagram

```
Workflow Job: sign-and-notarize-macos
├── Step 1: Pre-flight validation
│   ├── Check xcrun notarytool available
│   ├── Check 1Password CLI available
│   └── Check certificate expiry (if provided)
├── Step 2: Load credentials from 1Password
│   ├── op read each field
│   └── Validate non-empty before writing to GITHUB_ENV
├── Step 3: Download universal binaries
├── Step 4: Sign binaries via sign-macos-binary.sh
│   ├── Idempotent cleanup (delete-keychain if exists)
│   ├── Create keychain, import cert
│   ├── codesign --sign --options runtime --timestamp
│   ├── codesign --verify
│   ├── ditto -c -k (ZIP for notarization)
│   ├── notarytool submit --wait (with retry)
│   ├── notarytool log (JSON parsing)
│   └── spctl --assess
└── Step 5: Upload signed binaries
```

### Data Flow

```
1Password (op read)
    ↓
Workflow validates non-empty
    ↓
Env vars passed to sign-macos-binary.sh
    ↓
Script cleans up stale state
    ↓
Keychain created, certificate imported
    ↓
codesign (local)
    ↓
notarytool submit --wait (with retry)
    ↓
JSON response parsed for submission ID
    ↓
notarytool log <id> (structured)
    ↓
Cleanup + artifact upload
```

### Key Design Decisions

| Decision | Rationale | Alternatives Rejected |
|----------|-----------|----------------------|
| Keep 1Password as credential source | Already integrated; moving secrets would be high-risk churn | Migrating to GitHub Secrets or AWS Secrets Manager |
| Use `notarytool --output-format json` | Structured parsing eliminates fragility from text output changes | Keeping `grep | awk` with more regexes |
| Retry only `notarytool submit` | Other steps fail deterministically (bad cert, wrong password); only Apple API is transient | Retry loops around every command |
| Unconditional keychain cleanup | Time-based cleanup (`-mmin +60`) misses recent stale files from rapid re-runs | More complex state tracking |
| Validate credentials in workflow, not script | Fails faster — script doesn't run if credentials are bad | Moving all validation into the script |

### Eliminated Options (Essentialism)

| Option Rejected | Why Rejected | Risk of Including |
|-----------------|--------------|-------------------|
| GitHub-hosted runners for signing | Self-hosted is required for custom hardware; would not solve credential/cert issues | High complexity, no guarantee of fixing the actual problem |
| `notarytool` keychain profile | Requires storing credentials in macOS keychain, contradicts 1Password-first approach | Adds credential sprawl, harder to rotate |
| Entitlements file | No current need; binaries run fine with `--options runtime` alone | Premature complexity, unnecessary files |
| Parallel signing of both binaries | Minimal time savings; adds complexity for error attribution | Marginal benefit, harder to debug |

### Simplicity Check

> "Minimum code that solves the problem. Nothing speculative."

**What if this could be easy?**

The easiest fix is: validate credentials, clean up unconditionally, parse JSON, retry Apple API. That's exactly what this plan does. No new dependencies, no new infrastructure, no architectural changes.

**Senior Engineer Test**: A senior engineer would recognise this as boring, defensive infrastructure work — which is exactly what CI scripts should be.

**Nothing Speculative Checklist**:
- [x] No features the user didn't request
- [x] No abstractions "in case we need them later"
- [x] No flexibility "just in case"
- [x] No error handling for scenarios that cannot occur
- [x] No premature optimization

## File Changes

### Modified Files

| File | Changes |
|------|---------|
| `.github/workflows/release-comprehensive.yml` | Add pre-flight step; validate `op read` outputs; pass `RUNNER_TEMP` explicitly |
| `scripts/sign-macos-binary.sh` | Idempotent cleanup; JSON-based log parsing; retry loop; better error messages |

### Deleted Files

None.

## API Design

This is shell script infrastructure — no Rust APIs. The interface is the script's command-line contract:

```bash
scripts/sign-macos-binary.sh \
  <binary_path> \
  <apple_id> \
  <team_id> \
  <app_password> \
  <cert_base64> \
  <cert_password>
```

**Exit codes**:
- `0` — Success (signed, notarised, verified)
- `1` — Failure (any step failed; error message printed to stderr)

**Environment variables read**:
- `RUNNER_TEMP` — Temporary directory for keychain and certificate

## Test Strategy

### Local Validation Tests

| Test | Command | Purpose |
|------|---------|---------|
| Script syntax | `bash -n scripts/sign-macos-binary.sh` | Ensure no syntax errors |
| Workflow syntax | `act -W .github/workflows/release-comprehensive.yml -l` | Validate workflow YAML |
| Local signing (dry run) | Build a binary, run script with real credentials | End-to-end validation |

### CI Validation Tests

| Test | Trigger | Purpose |
|------|---------|---------|
| Workflow dispatch | `workflow_dispatch` with `test_run: true` | Run full pipeline without creating a release |
| Credential validation | Pre-flight step in workflow | Ensure `op read` succeeds before attempting signing |

### Manual Verification Checklist

- [ ] `codesign --verify --deep --strict --verbose=2` passes on signed binary
- [ ] `spctl --assess --type execute --verbose` passes or warns (not errors)
- [ ] `xcrun notarytool log <id>` shows "status: Accepted"
- [ ] Binary downloads from release and runs without Gatekeeper blocking

## Implementation Steps

### Step 1: Harden Credential Loading in Workflow
**Files:** `.github/workflows/release-comprehensive.yml`
**Description:** Add validation after each `op read` to ensure credentials are non-empty before proceeding.
**Tests:** Workflow dispatch with `test_run: true`
**Estimated:** 30 minutes

**Key code to write**:
```yaml
- name: Load signing credentials from 1Password
  env:
    OP_SERVICE_ACCOUNT_TOKEN: ${{ secrets.OP_SERVICE_ACCOUNT_TOKEN }}
  run: |
    set -euo pipefail
    echo "Loading credentials from 1Password..."

    load_credential() {
      local field="$1"
      local value
      value=$(op read "$field" --no-newline)
      if [ -z "$value" ]; then
        echo "Error: Failed to read credential from $field" >&2
        exit 1
      fi
      echo "$value"
    }

    APPLE_ID=$(load_credential 'op://TerraphimPlatform/apple.developer.credentials/username')
    APPLE_TEAM_ID=$(load_credential 'op://TerraphimPlatform/apple.developer.credentials/APPLE_TEAM_ID')
    APPLE_APP_PASSWORD=$(load_credential 'op://TerraphimPlatform/apple.developer.credentials/APPLE_APP_SPECIFIC_PASSWORD')
    CERT_BASE64=$(load_credential 'op://TerraphimPlatform/apple.developer.certificate/base64')
    CERT_PASSWORD=$(load_credential 'op://TerraphimPlatform/apple.developer.certificate/password')

    echo "APPLE_ID=$APPLE_ID" >> $GITHUB_ENV
    echo "APPLE_TEAM_ID=$APPLE_TEAM_ID" >> $GITHUB_ENV
    echo "APPLE_APP_PASSWORD=$APPLE_APP_PASSWORD" >> $GITHUB_ENV
    echo "CERT_BASE64=$CERT_BASE64" >> $GITHUB_ENV
    echo "CERT_PASSWORD=$CERT_PASSWORD" >> $GITHUB_ENV
    echo "Credentials loaded successfully"
```

### Step 2: Harden Signing Script
**Files:** `scripts/sign-macos-binary.sh`
**Description:** Add idempotent cleanup, JSON-based notarization log parsing, retry loop, and explicit error handling.
**Tests:** `bash -n` syntax check; local run with test binary
**Dependencies:** Step 1
**Estimated:** 60 minutes

**Key changes**:
1. Add `cleanup()` function called at start and on EXIT trap
2. Replace `grep | awk` parsing with `notarytool --output-format json`
3. Wrap `notarytool submit` in retry loop (3 attempts, 30s backoff)
4. Add `set -euo pipefail` and explicit error messages

```bash
#!/bin/bash
set -euo pipefail

BINARY_PATH="$1"
APPLE_ID="$2"
TEAM_ID="$3"
APP_PASS="$4"
CERT_BASE64="$5"
CERT_PASS="$6"

KEYCHAIN_PATH="${RUNNER_TEMP:-/tmp}/signing.keychain-db"
KEYCHAIN_PASS=""
CERT_PATH="${RUNNER_TEMP:-/tmp}/certificate.p12"

cleanup() {
  echo "==> Cleaning up"
  rm -f "$CERT_PATH"
  security delete-keychain "$KEYCHAIN_PATH" 2>/dev/null || true
}
trap cleanup EXIT

echo "==> Pre-flight: removing stale keychain if present"
security delete-keychain "$KEYCHAIN_PATH" 2>/dev/null || true

echo "==> Creating temporary keychain"
KEYCHAIN_PASS=$(openssl rand -base64 32)
security create-keychain -p "$KEYCHAIN_PASS" "$KEYCHAIN_PATH"
security set-keychain-settings -lut 21600 "$KEYCHAIN_PATH"
security unlock-keychain -p "$KEYCHAIN_PASS" "$KEYCHAIN_PATH"

echo "==> Importing certificate"
echo "$CERT_BASE64" | tr -d '\n' | base64 --decode > "$CERT_PATH"
security import "$CERT_PATH" -k "$KEYCHAIN_PATH" -P "$CERT_PASS" \
  -T /usr/bin/codesign -T /usr/bin/security

security set-key-partition-list -S apple-tool:,apple: -s -k "$KEYCHAIN_PASS" "$KEYCHAIN_PATH"
security list-keychains -d user -s "$KEYCHAIN_PATH" $(security list-keychains -d user | sed 's/"//g')

SIGNING_IDENTITY=$(security find-identity -v -p codesigning "$KEYCHAIN_PATH" | grep "Developer ID Application" | head -1 | awk -F'"' '{print $2}')
if [ -z "$SIGNING_IDENTITY" ]; then
  echo "Error: No Developer ID Application certificate found in keychain" >&2
  exit 1
fi
echo "==> Found signing identity: $SIGNING_IDENTITY"

echo "==> Signing binary"
codesign --sign "$SIGNING_IDENTITY" --options runtime --timestamp --verbose "$BINARY_PATH"

echo "==> Verifying signature"
codesign --verify --deep --strict --verbose=2 "$BINARY_PATH"

ZIP_PATH="${BINARY_PATH}.zip"
echo "==> Creating ZIP for notarization"
ditto -c -k --keepParent "$BINARY_PATH" "$ZIP_PATH"

echo "==> Submitting for notarization"
SUBMISSION_OUTPUT=""
for attempt in 1 2 3; do
  echo "Attempt $attempt..."
  if SUBMISSION_OUTPUT=$(xcrun notarytool submit "$ZIP_PATH" \
      --apple-id "$APPLE_ID" \
      --team-id "$TEAM_ID" \
      --password "$APP_PASS" \
      --wait \
      --output-format json 2>&1); then
    echo "$SUBMISSION_OUTPUT"
    break
  fi
  echo "Notarization submission failed on attempt $attempt" >&2
  echo "$SUBMISSION_OUTPUT" >&2
  if [ "$attempt" -eq 3 ]; then
    echo "Error: Notarization submission failed after 3 attempts" >&2
    exit 1
  fi
  sleep 30
done

SUBMISSION_ID=$(echo "$SUBMISSION_OUTPUT" | python3 -c "import sys, json; print(json.load(sys.stdin)['id'])")
echo "==> Submission ID: $SUBMISSION_ID"

echo "==> Fetching notarization log"
xcrun notarytool log "$SUBMISSION_ID" \
  --apple-id "$APPLE_ID" \
  --team-id "$TEAM_ID" \
  --password "$APP_PASS"

echo "==> Verifying Gatekeeper acceptance"
spctl --assess --type execute --verbose "$BINARY_PATH" || true

echo "Successfully signed and notarized: $(basename "$BINARY_PATH")"
```

### Step 3: Add Pre-flight Validation Step
**Files:** `.github/workflows/release-comprehensive.yml`
**Description:** Add a step before credential loading that checks tool availability and optionally validates the certificate.
**Tests:** Workflow dispatch
**Dependencies:** Step 2
**Estimated:** 30 minutes

```yaml
- name: Pre-flight validation
  run: |
    set -euo pipefail
    echo "Checking tool availability..."

    if ! command -v op &> /dev/null; then
      echo "Error: 1Password CLI (op) not found" >&2
      exit 1
    fi
    echo "1Password CLI: $(op --version)"

    if ! xcrun notarytool --version &> /dev/null; then
      echo "Error: notarytool not available" >&2
      exit 1
    fi
    echo "notarytool: available"

    if ! command -v codesign &> /dev/null; then
      echo "Error: codesign not available" >&2
      exit 1
    fi
    echo "codesign: available"

    echo "Pre-flight checks passed"
```

### Step 4: Test with Workflow Dispatch
**Description:** Trigger the release workflow with `test_run: true` to validate the hardened pipeline without publishing.
**Tests:** Full workflow run
**Dependencies:** Steps 1-3
**Estimated:** 30 minutes

**Command**:
```bash
gh workflow run release-comprehensive.yml --repo terraphim/terraphim-ai -f test_run=true
```

### Step 5: Verify and Document
**Description:** Confirm the fix works, update `docs/RELEASE_PROCESS.md` with any new troubleshooting notes.
**Tests:** None (documentation)
**Dependencies:** Step 4
**Estimated:** 15 minutes

## Rollback Plan

If issues discovered:
1. Revert commits modifying `.github/workflows/release-comprehensive.yml` and `scripts/sign-macos-binary.sh`
2. The previous script and workflow are preserved in git history
3. No database or persistent state changes — rollback is immediate

## Dependencies

### New Dependencies

None.

### Dependency Updates

None.

## Performance Considerations

### Expected Performance

| Metric | Target | Measurement |
|--------|--------|-------------|
| Signing latency | < 2 min | Workflow step timer |
| Notarization wait | < 10 min | `notarytool submit --wait` |
| Retry overhead | +60s max (2 retries x 30s) | Worst case |

The retry loop adds at most 60 seconds of overhead in the worst case (two 30-second retries). This is acceptable for a release pipeline where reliability matters more than speed.

## Open Items

| Item | Status | Owner |
|------|--------|-------|
| Actual error log from 2026-04-21 failure | Pending — will inform if fix needs adjustment | Alex |
| Certificate expiry check | Optional — can be added in Step 3 if desired | Alex |
| Test on self-hosted runner | Pending Step 4 | Alex |

## Approval

- [ ] Technical review complete
- [ ] Test strategy approved
- [ ] Human approval received
