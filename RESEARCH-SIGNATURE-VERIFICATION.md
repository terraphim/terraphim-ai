# Research Document: Auto-Update Signature Verification

**Status**: Draft
**Author**: Claude (Terraphim AI Research Agent)
**Date**: 2025-01-12
**Issue**: #421 - CRITICAL: Implement actual signature verification for auto-update
**Related**: DESIGN-AUTO-UPDATE.md, RESEARCH-AUTO-UPDATE.md

---

## Executive Summary

The auto-update system currently has a **critical security vulnerability**: the signature verification module (`crates/terraphim_update/src/signature.rs`) contains a placeholder implementation that **always returns `VerificationResult::Valid`**, exposing users to tampered or malicious binaries. This research document analyzes the problem, identifies constraints, evaluates solutions, and provides recommendations for implementing proper cryptographic signature verification.

### Key Findings
1. **Critical Security Risk**: Placeholder verification accepts any binary without validation
2. **Multiple Mature Solutions Available**: Minisign, ed25519-dalek, and Sigstore are viable options
3. **Existing Signing Infrastructure**: macOS code signing exists; Linux signatures missing
4. **Clear Integration Path**: Can leverage existing release scripts and CI/CD workflows
5. **Testing Strategy Established**: Property-based testing, test vectors, and integration tests available

---

## Problem Statement

### Description

The `terraphim_update` crate provides auto-update functionality for Terraphim AI binaries using GitHub Releases as a distribution channel. The system includes signature verification infrastructure, but the implementation is a **placeholder that always returns success**:

```rust
// Current implementation in crates/terraphim_update/src/signature.rs
pub fn verify_binary_signature(
    _binary_path: &Path,
    _signature_path: &Path,
    _public_key: &str,
) -> Result<VerificationResult, SignatureError> {
    Ok(VerificationResult::Valid) // ALWAYS RETURNS VALID!
}
```

This means:
- Malicious actors can serve tampered binaries
- No protection against supply chain attacks
- Users falsely believe binaries are cryptographically verified
- Violates security requirements from design documents

### Impact

**Who is affected:**
- All Terraphim AI users running auto-update
- Users downloading release binaries from GitHub
- Organizations deploying Terraphim AI (supply chain risk)

**Consequences:**
- **Immediate**: No actual security despite appearance of verification
- **Potential**: Supply chain attack if release infrastructure compromised
- **Compliance**: Violates security best practices for software distribution

### Success Criteria

1. **Functional Requirements**:
   - Reject binaries without valid signatures
   - Reject binaries with invalid/tampered signatures
   - Verify signatures using embedded public keys
   - Support multiple signature algorithms (Ed25519 priority)

2. **Non-Functional Requirements**:
   - Verification time: < 100ms per binary
   - No external dependencies at runtime
   - Cross-platform compatibility (Linux, macOS, Windows)
   - Clear error messages for verification failures

3. **Security Requirements**:
   - Constant-time signature comparison
   - Secure public key storage mechanism
   - Key rotation support
   - Compromise recovery procedures

---

## Current State Analysis

### Existing Implementation

**Component: `crates/terraphim_update/src/signature.rs`**

| Function | Status | Purpose |
|----------|--------|---------|
| `verify_binary_signature` | Placeholder | Verifies binary signature (always returns Valid) |
| `VerificationResult` enum | Defined | Valid/Invalid/NotFound variants |
| `SignatureError` enum | Defined | Error types for failures |
| `verify_release_signature` | Placeholder | Verifies GitHub Release signatures |

**Current Code Locations**:
- `crates/terraphim_update/src/signature.rs` - Placeholder verification
- `crates/terraphim_update/src/lib.rs:253` - Update flow (no verification calls)
- `crates/terraphim_update/tests/integration_test.rs` - No signature tests

### Data Flow

```
[GitHub Release] -> [Download] -> [Placeholder Verify] -> [Install Binary]
                                        ↓
                                  Always Valid!
                                        ↓
                                  [SECURITY VULNERABILITY]
```

**Missing Steps**:
1. No signature generation in release pipeline
2. No signature download from GitHub Releases
3. No actual cryptographic verification
4. No public key distribution mechanism

### Integration Points

**Release Pipeline**:
- `scripts/release.sh` - Creates releases, packages, GitHub releases
- `scripts/build-release.sh` - Builds optimized release binaries
- `.github/workflows/release*.yml` - CI/CD release automation

**Existing Signing**:
- `scripts/sign-macos-binary.sh` - Apple code signing for macOS
- `scripts/build-with-signing.sh` - Tauri app signing with 1Password
- **Missing**: Linux/Windows binary signature generation

**Update System**:
- `terraphim_update::downloader` - Downloads binaries
- `terraphim_update::platform` - Platform-specific paths
- `terraphim_update::rollback` - Backup/restore functionality

---

## Constraints

### Technical Constraints

| Constraint | Description | Impact |
|------------|-------------|--------|
| Rust Edition 2024 | Must use compatible cryptographic crates | Limited to crates supporting latest Rust |
| No External Runtime Deps | Verification must work offline | Embed public keys in binary |
| Cross-Platform | Support Linux/macOS/Windows | Algorithm must work everywhere |
- | Release Artifacts | Binaries distributed via GitHub Releases | Must generate signatures during release |

### Business Constraints

- **Timeline**: Critical security issue, should be addressed ASAP
- **Resources**: Small team, need simple, maintainable solution
- **Compliance**: Should follow open source security best practices
- **User Experience**: Verification failures must be clear and actionable

### Non-Functional Requirements

| Requirement | Target | Rationale |
|-------------|--------|-----------|
| Verification Time | < 100ms | Fast update checks |
| Binary Size Overhead | < 1MB | Public key + verification code |
| Key Rotation | Supported | Security best practice |
| FIPS Compliance | Optional | Enterprise requirements |

---

## Dependencies

### Internal Dependencies

| Dependency | Impact | Risk |
|------------|--------|------|
| `terraphim_update::downloader` | Must download signature files | Low - clear extension point |
| `terraphim_update::platform` | Must store public keys securely | Low - can embed in binary |
| Release scripts | Must generate signatures | Medium - requires script changes |

### External Dependencies (Rust Crates)

| Crate | Version | Purpose | Risk | Alternative |
|-------|---------|---------|------|-------------|
| **minisign** | 0.7+ | Ed25519 signatures | Low | ed25519-dalek |
| **minisign-verify** | 0.7+ | Verification-only (smaller) | Low | ed25519-dalek |
| **ed25519-dalek** | 2.x | Low-level Ed25519 | Low | ring |
| **sigstore** | pre-1.0 | Container/binary signing | High | minisign |

**Recommended**: `minisign-verify` for verification, `minisign` CLI for signing

---

## Risks and Unknowns

### Known Risks

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Key compromise in repository | Low | High | Use subkey signing, rotate keys |
| Integration breaks update flow | Medium | Medium | Comprehensive testing, gradual rollout |
| Performance degradation | Low | Low | Benchmark verification, optimize if needed |
| Public key distribution confusion | Medium | Medium | Document clearly in README |
| Signature file missing from releases | Medium | Low | CI checks, release verification |

### Open Questions

1. **Public Key Storage**: Should keys be embedded in binary or loaded from config file?
   - **Embedded**: Simpler, no user configuration needed
   - **Config File**: More flexible for key rotation
   - **Recommendation**: Embedded with fallback to config

2. **Signature Format**: Minisign vs. custom Ed25519 vs. Sigstore?
   - **Minisign**: Simple, widely used, has Rust implementation
   - **Custom Ed25519**: More control, more complexity
   - **Sigstore**: Overkill for current needs, evolving
   - **Recommendation**: Minisign for v1, consider Sigstore for v2

3. **Key Rotation**: How to handle compromised keys?
   - **Document**: Need compromise response procedure
   - **Implementation**: Support multiple trusted keys
   - **Recommendation**: Start with single key, add rotation in v1.1

4. **CI/CD Integration**: Where to generate signatures?
   - **Options**: Local signing, CI signing, hybrid
   - **Recommendation**: CI signing with GitHub Actions secrets

### Assumptions

1. GitHub Releases can store `.sig` files alongside binaries
2. Users trust the initial binary installation (bootstrapping problem)
3. Ed25519 provides sufficient security for binary signing
4. Signature verification is fast enough for interactive updates
5. Public keys can be securely stored in source code repository

---

## Research Findings

### Key Insights

1. **Ed25519 is Modern Standard**: Preferred over RSA/DSA for new implementations
   - Smaller keys (32 bytes vs. 256+ bytes)
   - Faster verification (single integer multiplication)
   - Better security properties (deterministic, no timing attacks)

2. **Minisign is Best Fit**: Purpose-built for file signing, simple Rust API
   - Created by Frank Denis (知名密码学家)
   - Compatible with OpenBSD signify
   - Zero-dependency verification crate available

3. **Sigstore is Future-Ready**: Industry standard for supply chain security
   - Used by major projects (Kubernetes, etcd)
   - Integrates with transparency logs (Rekor)
   - Pre-1.0 but rapidly maturing

4. **Testing Infrastructure Exists**: Multiple testing approaches available
   - Wycheproof test vectors (Google)
   - Property-based testing (QuickCheck)
   - NIST CAVP validation program

### Relevant Prior Art

| Project | Signing Method | Relevance |
|---------|---------------|-----------|
| **ripgrep** | Minisign | Similar Rust CLI, same release patterns |
| **rustup** | GPG | Official Rust toolchain, complex PGP |
| **Debian APT** | GPG | Package manager, migrating to Sequoia PGP |
| **TUF** | Ed25519 | The Update Framework (academic research) |
| **Sigstore** | Cosign | Cloud-native standard |

### Technical Spikes Needed

| Spike | Purpose | Estimated Effort |
|-------|---------|------------------|
| Prototype minisign integration | Verify API works with our build system | 2-4 hours |
| Key generation workflow | Generate test signing key pair | 1 hour |
| CI/CD integration test | Test signature generation in GitHub Actions | 2-3 hours |
| Performance benchmarking | Measure verification time on target platforms | 2 hours |

---

## Signature Verification Approaches Analysis

### 1. Minisign (Recommended)

**Description**: Simple, modern file signing tool using Ed25519

**Pros**:
- Pure Rust implementation available
- Zero-dependency verification crate (`minisign-verify`)
- Simple key format (base64 encoded)
- Compatible with OpenBSD signify
- Battle-tested in production

**Cons**:
- Smaller ecosystem than PGP
- No built-in key expiration
- Newer than GPG (less tooling)

**Implementation Effort**: Low (2-3 days)

**Rust Crate**: [`minisign`](https://crates.io/crates/minisign) / [`minisign-verify`](https://docs.rs/minisign-verify)

**Example**:
```rust
use minisign_verify::{Signature, PublicKey};

// Load public key
let public_key = PublicKey::from_base64("RWT+5...")?;

// Load signature
let signature = Signature::decode(&sig_bytes)?;

// Verify
signature.verify(&public_key, &binary_bytes)?;
```

**Sources**:
- [Minisign GitHub](https://github.com/jedisct1/minisign)
- [Minisign crates.io](https://crates.io/crates/minisign)
- [Minisign verify docs](https://docs.rs/minisign-verify)

### 2. ed25519-dalek

**Description**: Low-level Ed25519 signature library

**Pros**:
- Most popular Ed25519 implementation in Rust
- High performance, well-audited
- Flexible (build custom formats)
- No dependencies on system tools

**Cons**:
- Requires custom signature format design
- More error-prone than using established format
- Need to handle key serialization yourself

**Implementation Effort**: Medium (3-5 days)

**Rust Crate**: [`ed25519-dalek`](https://docs.rs/ed25519-dalek)

**Sources**:
- [ed25519-dalek docs](https://docs.rs/ed25519-dalek)
- [Ed25519 signatures info](https://ianix.com/pub/ed25519-deployment.html)

### 3. Sequoia PGP

**Description**: Modern OpenPGP implementation in pure Rust

**Pros**:
- OpenPGP compatibility (standard format)
- Used by Debian APT (2025 migration)
- Supports key expiration, multiple signatures
- Comprehensive feature set

**Cons**:
- Heavy dependency tree
- Overkill for binary signing
- Complex API (PGP is complex)
- Slower verification than Ed25519

**Implementation Effort**: High (5-7 days)

**Rust Crate**: [`Sequoia PGP`](https://sequoia-pgp.org/)

**Sources**:
- [Sequoia PGP website](https://sequoia-pgp.org/)
- [Debian APT Sequoia discussion](https://github.com/freedomofpress/securedrop/issues/6399)

### 4. Sigstore/Cosign

**Description**: Cloud-native supply chain security standard

**Pros**:
- Industry standard for containers/binaries
- Transparent log integration (Rekor)
- Supports keyless signing (Fulcio)
- SLSA provenance support

**Cons**:
- Pre-1.0 (evolving rapidly)
- External service dependencies
- Complex for simple binary signing
- Overkill for current needs

**Implementation Effort**: High (7-10 days)

**Rust Crate**: [`sigstore`](https://docs.rs/sigstore), [`sigstore-verification`](https://crates.io/crates/sigstore-verification)

**Sources**:
- [Sigstore Rust crate](https://github.com/sigstore/sigstore-rs)
- [Sigstore docs](https://docs.sigstore.dev/language_clients/rust/)
- [Cosign introduction](https://edu.chainguard.dev/open-source/sigstore/cosign/an-introduction-to-cosign/)

### Comparison Matrix

| Approach | Implementation Time | Dependencies | Maturity | Flexibility | Recommendation |
|----------|-------------------|--------------|----------|-------------|----------------|
| **Minisign** | 2-3 days | Low | High | Medium | **PRIMARY CHOICE** |
| ed25519-dalek | 3-5 days | Low | High | High | Alternative |
| Sequoia PGP | 5-7 days | High | High | Low | For PGP compatibility |
| Sigstore | 7-10 days | Medium | Medium | High | Future consideration |

---

## Public Key Distribution Strategy

### Recommended Approach: Multi-Modal Distribution

**1. Embedded in Binary** (Primary)
- Store public key in source code
- Compile into binary during build
- Pros: No user configuration, offline verification
- Cons: Key rotation requires rebuild

**2. GitHub Repository** (Secondary)
- Publish public key in `docs/keys/` directory
- Document in README
- Pros: Transparency, easy to verify
- Cons: Requires download/trust of GitHub

**3. Key Servers** (Optional, for PGP)
- Upload to pgp.mit.edu, keyserver.ubuntu.com
- Only needed if using PGP format
- Pros: Standard distribution method
- Cons: Key server ecosystem issues

### Key Distribution Best Practices

Based on research ([security.stackexchange](https://security.stackexchange.com/questions/406/how-should-i-distribute-my-public-key)):

1. **Document the Process**: Clearly explain how users obtain and verify keys
2. **Multiple Channels**: Distribute keys through multiple independent channels
3. **Fingerprint Verification**: Publish key fingerprints in secure locations (website, documentation)
4. **Key Signing**: Consider web-of-trust or developer key signing for higher security

### Key Storage Locations

| Location | Purpose | Access Method |
|----------|---------|---------------|
| `crates/terraphim_update/src/keys/default.pub` | Embedded default key | Compiled into binary |
| `~/.config/terraphim/update-key.pub` | User-specified override | Config file |
| `docs/keys/release-public-key.pub` | Documentation transparency | Downloaded separately |
| GitHub Releases `KEYS` file | Release-specific keys | Downloaded with release |

---

## Testing Strategies for Cryptographic Verification

### 1. Unit Testing with Test Vectors

**Source**: [Wycheproof Project](https://appsec.guide/docs/crypto/wycheproof/) (Google)

Use standardized test vectors for Ed25519 signatures:
- Valid signatures (should pass)
- Invalid signatures (should fail)
- Edge cases (wrong message, wrong key, malformed signatures)

**Example**:
```rust
#[test]
fn test_verify_valid_signature() {
    let public_key = PublicKey::from_base64(TEST_PUBLIC_KEY).unwrap();
    let signature = Signature::decode(TEST_SIGNATURE_BYTES).unwrap();
    let message = b"Test message";

    assert!(signature.verify(&public_key, message).is_ok());
}
```

**Test Vector Sources**:
- [Wycheproof Ed25519 tests](https://appsec.guide/docs/crypto/wycheproof/)
- [ed25519-speccheck](https://github.com/novifinancial/ed25519-speccheck)
- [NIST ACVP Digital Signatures](https://csrc.nist.gov/projects/cryptographic-algorithm-validation-program/digital-signatures)

### 2. Property-Based Testing

**Framework**: [`quickcheck`](https://crates.io/crates/quickcheck) or [`proptest`](https://crates.io/crates/proptest)

Test properties:
- Valid signatures always verify
- Invalid signatures never verify
- Verification is deterministic
- Message modification breaks signature

**Example**:
```rust
#[quickcheck]
fn fn_verify_valid_always_passes(message: Vec<u8>) -> bool {
    let (pubkey, privkey) = generate_keypair();
    let signature = sign(&privkey, &message);
    verify(&pubkey, &message, &signature).is_ok()
}
```

### 3. Integration Testing

**Scenarios**:
- Download and verify real release binaries
- Test with missing signatures
- Test with corrupted signatures
- Test with wrong public key
- Test backup/rollback after failed verification

**See**: `crates/terraphim_update/tests/integration_test.rs` for existing tests

### 4. Fuzzing (Advanced)

**Tools**: [`cargo-fuzz`](https://github.com/rust-fuzz/cargo-fuzz)

Find edge cases:
- Malformed signature data
- Unexpected message lengths
- Integer overflow in verification
- Timing attack vulnerabilities

**Research**: [CLFuzz: Vulnerability Detection](https://dl.acm.org/doi/10.1145/3628160)

### 5. Security Auditing

**Approaches**:
- Manual code review for constant-time comparisons
- Static analysis with [`cargo-audit`](https://crates.io/crates/cargo-audit)
- Dependency review for known vulnerabilities

---

## Key Rotation and Compromise Recovery

### Key Rotation Strategy

**Based on research**: [Encryption Key Rotation for Data Security](https://cpl.thalesgroup.com/blog/data-protection/encryption-key-rotation-data-security)

**Recommendations**:
1. **Regular Rotation**: Rotate signing keys annually (even if not compromised)
2. **Versioned Keys**: Support multiple trusted keys simultaneously
3. **Grace Period**: Keep old key trusted for 2-3 release cycles
4. **Key Expiration**: Embed expiration date in key metadata

### Implementation Approach

**Key Structure**:
```rust
struct TrustedKeys {
    primary: PublicKey,
    secondary: Option<PublicKey>,
    rotation_date: DateTime<Utc>,
}
```

**Verification Logic**:
```rust
fn verify_with_any_key(signature: &Signature, message: &[u8], keys: &TrustedKeys) -> bool {
    // Try primary key first
    if signature.verify(&keys.primary, message).is_ok() {
        return true;
    }

    // Try secondary key (for rotation period)
    if let Some(secondary) = &keys.secondary {
        if signature.verify(secondary, message).is_ok() {
            return true;
        }
    }

    false
}
```

### Compromise Response Procedure

**Based on**: [Survivable Key Compromise in Software Update Systems](https://freehaven.net/~arma/tuf-ccs2010.pdf) and [Handle Breached Certificate And Key](https://www.encryptionconsulting.com/education-center/handle-breached-certificate-and-key)

**Immediate Actions** (if key compromised):
1. **Revoke Compromised Key**: Add to revocation list in code
2. **Generate New Key**: Create new signing key pair offline
3. **Emergency Release**: Release new version signed with new key
4. **Security Advisory**: Publish disclosure about compromise
5. **Update Verification**: Push update with new trusted key

**Long-term Actions**:
1. **Post-Mortem**: Analyze how compromise occurred
2. **Improve Procedures**: Strengthen key storage and access controls
3. **Consider TUF**: Evaluate The Update Framework for more robust security

---

## Integration with Release Pipeline

### Proposed Signature Generation Workflow

**Option 1: CI/CD Signing** (Recommended)
```yaml
# .github/workflows/release-sign.yml
- name: Generate signing key
  run: |
    if [ ! -f "$SECRETS_DIR/signing.key" ]; then
      minisign -G -s "$SECRETS_DIR/signing.key" -p "$SECRETS_DIR/signing.pub"
    fi

- name: Sign release binaries
  env:
    MINISIGN_SIGNING_KEY: ${{ secrets.MINISIGN_PRIVATE_KEY }}
  run: |
    for binary in target/release/terraphim_*; do
      minisign -S -s "$MINISIGN_SIGNING_KEY" -m "$binary" -x "$binary.minisig"
    done

- name: Upload signatures
  run: |
    gh release upload $TAG *.minisig
```

**Option 2: Local Signing**
```bash
# scripts/sign-release.sh
for binary in release-artifacts/*; do
    minisign -S -m "$binary" -x "$binary.minisig"
done
```

### Required Changes to Release Scripts

**`scripts/release.sh`** additions:
```bash
# After building binaries
sign_binaries() {
    print_status "Signing release binaries with minisign"

    for binary in "$RELEASE_DIR"/*; do
        if [[ -f "$binary" ]]; then
            print_status "Signing $(basename "$binary")"
            minisign -S -m "$binary" -x "$binary.minisig"
        fi
    done
}

# Add to main() after build_binaries
sign_binaries
```

### Integration Points Summary

| Component | Change Required | Effort |
|-----------|----------------|--------|
| `.github/workflows/release*.yml` | Add signature generation step | 1-2 hours |
| `scripts/release.sh` | Integrate signing commands | 1 hour |
| `crates/terraphim_update/Cargo.toml` | Add minisign-verify dependency | 15 minutes |
| `crates/terraphim_update/src/signature.rs` | Implement verification | 4-6 hours |
| `crates/terraphim_update/src/downloader.rs` | Download signature files | 1-2 hours |
| `crates/terraphim_update/tests/` | Add verification tests | 2-3 hours |

**Total Estimated Effort**: 10-16 hours (1.5-2 days)

---

## Recommendations

### Proceed/No-Proceed

**DECISION: PROCEED** with implementing Minisign-based signature verification

**Justification**:
1. **Critical Security Issue**: Current vulnerability is unacceptable
2. **Mature Solution Available**: Minisign is battle-tested and simple
3. **Low Implementation Risk**: Well-understood problem, clear path forward
4. **Minimal Disruption**: Can be added without breaking existing functionality
5. **Strong ROI**: 2-day effort for major security improvement

### Scope Recommendations

**Phase 1: MVP** (2-3 days)
- Implement Minisign verification in `signature.rs`
- Generate signing key pair
- Update release scripts to sign binaries
- Add basic unit tests for verification
- Embed public key in binary
- Document for users

**Phase 2: Production Hardening** (1-2 days)
- Comprehensive test coverage
- Integration tests with real releases
- Performance benchmarking
- Error message refinement
- Key rotation framework (data structure only)

**Phase 3: Advanced Features** (Future)
- Key rotation implementation
- Multiple trusted keys support
- Configurable public keys
- Consider Sigstore integration

### Out of Scope (Deferred)
- PGP compatibility (use Sequoia if needed)
- Sigstore/Cosign integration (evaluate for v2)
- Binary encryption (only signing needed)
- Multi-signature support

---

## Risk Mitigation Recommendations

### Implementation Risks

| Risk | Mitigation |
|------|------------|
| Integration breaks updates | Comprehensive integration tests, feature flag |
| Performance degradation | Benchmark before/after, optimize if needed |
| Key management complexity | Start simple, add rotation later |
| User confusion | Clear documentation, helpful error messages |

### Operational Risks

| Risk | Mitigation |
|------|------------|
| Private key leaked | Store in GitHub Actions secrets, access logs |
| Key rotation downtime | Support multiple keys during transition |
| Signature generation fails | CI checks prevent releases without signatures |

### Security Risks

| Risk | Mitigation |
|------|------------|
| Weak random number generation | Use minisign (proper entropy handling) |
| Timing attacks | Use constant-time comparison in ed25519-dalek |
| Key compromise | Document incident response procedure |

---

## Next Steps

### Immediate Actions (Phase 1)

1. **Create GitHub Issue** for tracking implementation
   - Break down into subtasks
   - Assign to developer
   - Set milestone

2. **Generate Signing Key Pair**
   ```bash
   # Generate minisign key pair
   minisign -G -s terraphim-release.key -p terraphim-release.pub

   # Store private key in GitHub Actions secrets
   # Store public key in repository
   ```

3. **Update Dependencies**
   ```toml
   # crates/terraphim_update/Cargo.toml
   [dependencies]
   minisign-verify = "0.7"
   ```

4. **Implement Verification**
   - Replace placeholder in `signature.rs`
   - Add signature download to `downloader.rs`
   - Call verification in update flow

5. **Update Release Pipeline**
   - Modify `scripts/release.sh`
   - Update GitHub Actions workflows
   - Test signature generation

### If Approved

1. **Design Document**: Create detailed design (Phase 2)
2. **Implementation**: Execute Phase 1 tasks
3. **Testing**: Comprehensive test coverage
4. **Documentation**: Update README and security docs
5. **Release**: Deploy signed binaries

### Open Questions for Stakeholders

1. **Key Storage**: Should we use GitHub Actions secrets or local signing?
   - **Recommendation**: GitHub Actions secrets for automation

2. **Key Rotation Frequency**: Annual or bi-annual?
   - **Recommendation**: Start with annual, evaluate based on risk

3. **Rollback Strategy**: What if verification breaks legitimate updates?
   - **Recommendation**: Implement --skip-verification flag with warning

---

## Appendix

### Reference Materials

**Signature Verification Libraries**:
- [Minisign GitHub](https://github.com/jedisct1/minisign)
- [Minisign crates.io](https://crates.io/crates/minisign)
- [Minisign verify docs](https://docs.rs/minisign-verify)
- [ed25519-dalek docs](https://docs.rs/ed25519-dalek)
- [Sequoia PGP](https://sequoia-pgp.org/)
- [Sigstore Rust](https://github.com/sigstore/sigstore-rs)
- [Sigstore docs](https://docs.sigstore.dev/language_clients/rust/)

**Best Practices**:
- [Security Best Practices for Open Source](https://opensource.guide/de/security-best-practices-for-your-project/)
- [How should I distribute my public key?](https://security.stackexchange.com/questions/406/how-should-i-distribute-my-public-key)
- [Core Infrastructure Best Practices Badge](https://github.com/coreinfrastructure/best-practices-badge)

**Testing Resources**:
- [Wycheproof Project](https://appsec.guide/docs/crypto/wycheproof/)
- [ed25519-speccheck](https://github.com/novifinancial/ed25519-speccheck)
- [NIST Digital Signatures Validation](https://csrc.nist.gov/projects/cryptographic-algorithm-validation-program/digital-signatures)
- [Automated Cryptographic Validation Protocol](https://pages.nist.gov/ACVP/)

**Key Management**:
- [Encryption Key Rotation for Data Security](https://cpl.thalesgroup.com/blog/data-protection/encryption-key-rotation-data-security)
- [Handle Breached Certificate And Key](https://www.encryptionconsulting.com/education-center/handle-breached-certificate-and-key)
- [Survivable Key Compromise in Software Update Systems](https://freehaven.net/~arma/tuf-ccs2010.pdf)
- [Managing Cryptographic Keys and Secrets](https://www.cyber.gov.au/business-government/secure-design/secure-by-design/managing-cryptographic-keys-and-secrets)

### Code Snippets

**Minisign Verification Example**:
```rust
use minisign_verify::{PublicKey, Signature};

pub fn verify_binary_signature(
    binary_path: &Path,
    signature_path: &Path,
    public_key: &str,
) -> Result<VerificationResult, SignatureError> {
    // Load public key from base64
    let pk = PublicKey::from_base64(public_key)
        .map_err(|e| SignatureError::InvalidPublicKey(e.to_string()))?;

    // Read binary and signature
    let binary_bytes = std::fs::read(binary_path)
        .map_err(|e| SignatureError::ReadError(e.to_string()))?;
    let sig_bytes = std::fs::read(signature_path)
        .map_err(|e| SignatureError::ReadError(e.to_string()))?;

    // Decode signature
    let signature = Signature::decode(&sig_bytes)
        .map_err(|e| SignatureError::InvalidSignature(e.to_string()))?;

    // Verify
    signature.verify(&pk, &binary_bytes)
        .map(|_| VerificationResult::Valid)
        .map_err(|e| SignatureError::VerificationFailed(e.to_string()))
}
```

**Key Generation**:
```bash
# Generate new key pair
minisign -G -s release.key -p release.pub

# Sign a binary
minisign -S -s release.key -m terraphim_server -x terraphim_server.minisig

# Verify a binary
minisign -V -p release.pub -m terraphim_server -x terraphim_server.minisig
```

### Security Considerations

**Constant-Time Comparison**:
- Ed25519 (used by minisign) uses constant-time operations
- Avoid `==` comparison on signature bytes
- Use `subtle` crate if implementing custom comparison

**Key Storage**:
- **NEVER commit private keys to repository**
- Use environment variables or secret managers
- Consider hardware security modules (HSM) for production
- Encrypt keys at rest

**Timing Attacks**:
- Ed25519 is designed to prevent timing attacks
- Use `constant_time_eq` from `subtle` crate if needed
- Avoid early returns on byte-level comparisons

---

## Conclusion

This research document identifies a **critical security vulnerability** in the auto-update system and provides a **clear, actionable path forward** using the Minisign signature verification library. The recommended approach balances security, simplicity, and maintainability while providing a foundation for future enhancements.

**Key Takeaways**:
1. **Immediate Action Required**: Current placeholder is a security risk
2. **Mature Solutions Available**: Minisign is production-ready
3. **Low Implementation Risk**: 2-3 day effort, well-understood problem
4. **Clear Integration Path**: Can leverage existing release infrastructure
5. **Foundation for Future**: Supports key rotation and advanced features

**Next Phase**: Proceed to **Phase 2: Disciplined Design** to create detailed implementation plans.

---

**Status**: Ready for Review
**Next Review**: Design phase approval
**Completion Date**: 2025-01-12
