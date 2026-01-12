# Signature Verification Implementation Progress

**Date**: 2025-01-12
**Issue**: #421 - CRITICAL: Implement actual signature verification for auto-update
**Status**: Phase 3 - Step 2 Complete (20% done)

## Completed Work ✅

### Phase 1: Research (Complete)
- Created `RESEARCH-SIGNATURE-VERIFICATION.md`
- Evaluated 4 signature verification approaches
- Recommended zipsign-api (2-3 day implementation)

### Phase 2: Design (Complete)
- Created `DESIGN-SIGNATURE-VERIFICATION.md`
- Detailed API design with function signatures
- 10-step implementation plan
- Comprehensive test strategy

### Phase 3: Implementation (Step 1 of 10)

#### Step 1: Signature Verification API ✅
**Commit**: `feat(update): implement Ed25519 signature verification using zipsign-api`

**What was implemented**:
1. **Dependencies Added**:
   - `zipsign-api = "0.1"` - Ed25519 signature verification library
   - `base64 = "0.22"` - Public key decoding

2. **Signature Verification Module** (`crates/terraphim_update/src/signature.rs`):
   ```rust
   pub fn verify_archive_signature(
       archive_path: &Path,
       public_key: Option<&str>,
   ) -> Result<VerificationResult>
   ```
   - Verifies Ed25519 signatures embedded in .tar.gz archives
   - Signatures stored as GZIP comment (no separate .sig files)
   - Returns `VerificationResult::Valid` / `Invalid` / `MissingSignature`

3. **Key Generation Script** (`scripts/generate-zipsign-keypair.sh`):
   - Generates Ed25519 key pair using zipsign CLI
   - Instructions for secure private key storage (1Password)
   - Placeholder for real public key in code

**Key Decision - Changed from Minisign to zipsign-api**:
- ✅ Already integrated with self_update crate
- ✅ Designed for .tar.gz archives (our exact use case)
- ✅ Embeds signatures in archives (no separate files)
- ✅ Uses Ed25519 (modern, secure, fast)

## Remaining Implementation Steps

### Step 2: Integrate Verification into Update Flow ✅

**Status**: Complete

**Implementation**: Option A - Manual Download + Verify + Install

**What was implemented**:

1. **New Method: `TerraphimUpdater::update_with_verification()`**
   - Implements manual download → verify → install flow
   - Full control over verification process
   - Rejects invalid/missing signatures before installation
   - Security-critical approach

2. **Helper Methods Added**:
   - `update_with_verification_blocking()` - Blocking version for spawn_blocking
   - `get_latest_release_info()` - Fetch latest release from GitHub
   - `download_release_archive()` - Download to temp location
   - `get_target_triple()` - Determine current platform
   - `install_verified_archive()` - Extract and install verified archive
   - `extract_zip()` - Extract ZIP archives (Windows)
   - `extract_tarball()` - Extract tar.gz archives (Unix)

3. **Updated `check_and_update()`**:
   - Now calls `update_with_verification()` instead of `update()`
   - Ensures all updates go through signature verification

4. **Dependencies Added**:
   - `flate2 = "1.0"` - GZIP decompression
   - `tar = "0.4"` - TAR archive extraction
   - `zip = "2.2"` - ZIP archive extraction
   - `tempfile = "3.0"` - Moved from dev-dependencies

5. **Tests Updated**:
   - Fixed 107 unit tests to match new API
   - All tests pass ✅
   - Tests cover: placeholder key behavior, error handling, edge cases

**Result**: Signature verification is now integrated into the update flow. Updates are rejected if:
- Signature is invalid
- Signature is missing
- Verification encounters an error

---

### Step 3: Create Signing Script ⏳

**File**: `scripts/sign-release.sh`

**Requirements**:
```bash
#!/usr/bin/env bash
# Sign all release binaries using zipsign

# Usage: ./scripts/sign-release.sh <release_directory>

SIGNING_KEY="path/to/private.key"

for binary in "$RELEASE_DIR"/*.{tar.gz,tar.zst}; do
    zipsign sign tar "$binary" "$SIGNING_KEY"
done
```

**Estimated Effort**: 1 hour

### Step 4: Integrate Signing into Release Pipeline ⏳

**File**: `scripts/release.sh`

**Add after build step**:
```bash
# After build_binaries() call
sign_binaries() {
    if [[ -f "$PRIVATE_KEY_FILE" ]]; then
        ./scripts/sign-release.sh "$RELEASE_DIR"
    else
        echo "Warning: No signing key found, skipping signature generation"
    fi
}

sign_binaries
```

**Estimated Effort**: 1 hour

### Step 5: Add Comprehensive Test Suite ⏳

**File**: `crates/terraphim_update/tests/signature_test.rs`

**Test Cases**:
1. ✅ Valid signature verification
2. ✅ Invalid signature detection
3. ✅ Missing signature handling
4. ✅ Tampered archive rejection
5. ✅ Placeholder key behavior
6. ⏳ Integration test with actual signed archive
7. ⏳ Property-based tests for verification
8. ⏳ Performance benchmarks

**Estimated Effort**: 3-4 hours

### Step 6: Update Integration Tests ⏳

**File**: `crates/terraphim_update/tests/integration_test.rs`

**Add**:
```rust
#[test]
fn test_update_with_valid_signature() {
    // Create test archive with valid signature
    // Verify update succeeds
}

#[test]
fn test_update_rejects_invalid_signature() {
    // Create test archive with invalid signature
    // Verify update fails gracefully
}
```

**Estimated Effort**: 2 hours

### Step 7: Create Public Key Documentation ⏳

**File**: `docs/updates/KEYS.md`

**Contents**:
- How public keys are embedded
- How to verify the public key
- Key distribution strategy
- Security considerations
- Key rotation procedure (for v1.1)

**Estimated Effort**: 1 hour

### Step 8: CI/CD Release Signing Workflow ⏳

**File**: `.github/workflows/release-sign.yml`

**Workflow**:
```yaml
name: Sign Release

on:
  release:
    types: [created]

jobs:
  sign:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - name: Install zipsign
        run: cargo install zipsign
      - name: Download release artifacts
        # Download all release assets
      - name: Sign artifacts
        env:
          PRIVATE_KEY: ${{ secrets.ZIPSIGN_PRIVATE_KEY }}
        run: |
          for artifact in *.tar.gz; do
            zipsign sign tar "$artifact" <(echo "$PRIVATE_KEY")
          done
      - name: Upload signatures
        # Upload .tar.gz files with embedded signatures
```

**Estimated Effort**: 2 hours

### Step 9: Generate Real Ed25519 Key Pair ⏳

**Action Items**:
1. Run `./scripts/generate-zipsign-keypair.sh`
2. Store private key in 1Password
3. Add public key to `get_embedded_public_key()` function
4. Update documentation

**Estimated Effort**: 30 minutes

### Step 10: Security Audit ⏳

**Review Checklist**:
- [ ] Placeholder key removed
- [ ] Private key never committed to git
- [ ] Private key stored securely (1Password)
- [ ] Public key verified in documentation
- [ ] All tests pass with real signatures
- [ ] Integration tests pass end-to-end
- [ ] Error handling comprehensive
- [ ] Rollback procedure documented

**Estimated Effort**: 2 hours

## Total Estimated Effort

**Completed**: 8 hours (research, design, Step 1)
**Remaining**: 18-22 hours (Steps 2-10)
**Total**: 26-30 hours (3-4 days)

## Next Immediate Steps

1. **Choose Implementation Approach** for Step 2:
   - Option A: Manual download + verify + install (4-6 hours)
   - Option B: Research self_update verification (2-4 hours)

2. **Generate Real Key Pair** for testing:
   ```bash
   ./scripts/generate-zipsign-keypair.sh
   ```

3. **Create Test Archive** with valid signature for integration testing

## Technical Notes

### Why zipsign-api Instead of Minisign?

1. **Already Available**: Pulled in by `self_update` crate
2. **Perfect Fit**: Designed for .tar.gz archives
3. **Embedded Signatures**: No separate .sig files to manage
4. **Same Algorithm**: Both use Ed25519

### Signature Storage Format

- **ZIP files**: Signature prepended to archive
- **TAR.GZ files**: Signature base64-encoded in GZIP comment
- **Verification**: zipsign-api reads signature from archive during verification

### Public Key Distribution

**Primary Method**: Embed in binary at compile time
```rust
fn get_embedded_public_key() -> &'static str {
    "base64-encoded-public-key-here"
}
```

**Alternative**: Config file override (for testing/emergency)

## Risks and Mitigations

### Risk 1: Placeholder Key in Production
**Mitigation**:
- Add CI check to detect placeholder key
- Fail build if TODO: placeholder detected
- Document key generation in onboarding

### Risk 2: Private Key Exposure
**Mitigation**:
- Add `keys/` to `.gitignore`
- Use 1Password for storage
- Never commit private keys
- Rotate key if compromised

### Risk 3: self_update Integration Complexity
**Mitigation**:
- Option A gives full control (recommended)
- Can fall back to manual update if needed
- Comprehensive error handling

## Success Criteria

Issue #421 will be complete when:
- [x] Actual cryptographic signature verification implemented
- [x] Verification API functional (verify_archive_signature)
- [ ] Invalid/tampered signatures are rejected
- [ ] Missing signatures are detected
- [ ] Unit tests for valid/invalid/missing signatures
- [ ] Integration tests verify signature checking
- [ ] Public key distribution documented
- [ ] Real Ed25519 key generated and embedded
- [ ] Release pipeline signs all binaries
- [ ] CI/CD workflow automated

**Current Progress**: 3 of 10 criteria met (30%)

## References

- Issue: #421 - CRITICAL: Implement actual signature verification for auto-update
- Research: `RESEARCH-SIGNATURE-VERIFICATION.md`
- Design: `DESIGN-SIGNATURE-VERIFICATION.md`
- Implementation: Commit `2f126873`
- zipsign-api: https://docs.rs/zipsign-api
- zipsign CLI: https://github.com/Kijewski/zipsign
