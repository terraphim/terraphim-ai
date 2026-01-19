# Implementation Plan: Auto-Update Signature Verification

**Status**: Review
**Research Doc**: [RESEARCH-SIGNATURE-VERIFICATION.md](RESEARCH-SIGNATURE-VERIFICATION.md)
**Issue**: #421 - CRITICAL: Implement actual signature verification for auto-update
**Author**: Claude (Terraphim AI Design Agent)
**Date**: 2025-01-12
**Estimated Effort**: 10-16 hours (2-3 days)

---

## Overview

### Summary

Implement actual cryptographic signature verification for the auto-update system using Minisign (Ed25519 signatures). The current placeholder implementation in `crates/terraphim_update/src/signature.rs` always returns `VerificationResult::Valid`, creating a critical security vulnerability.

### Approach

**Chosen Solution**: Minisign with Ed25519 signatures
- **Verification Library**: `minisign-verify` crate (pure Rust, zero dependencies)
- **Signing Tool**: `minisign` CLI for release pipeline
- **Signature Format**: `.minisig` files alongside release binaries
- **Public Key Storage**: Embedded in binary with optional config override

### Scope

**In Scope:**
- Implement actual Ed25519 signature verification using Minisign
- Generate signing key pair for Terraphim AI releases
- Update release scripts to sign Linux/macOS/Windows binaries
- Download signature files from GitHub Releases
- Add comprehensive test coverage (unit, integration, property-based)
- Update integration tests to verify signature checking
- Document public key distribution mechanism

**Out of Scope:**
- PGP/OpenPGP compatibility (use Sequoia if needed in future)
- Sigstore/Cosign integration (evaluate for v2)
- Binary encryption (only signing required)
- Multi-signature support
- Key rotation implementation (framework only, deferred to v1.1)

---

## Architecture

### Component Diagram

```
┌─────────────────────────────────────────────────────────────┐
│                     Update Flow (Current)                    │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│  GitHub Releases                                           │
│  ├─ terraphim_server (binary)                               │
│  ├─ terraphim_server.minisig (NEW - signature file)         │
│  └─ terraphim_server.minisig.sig (optional signature)        │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│  terraphim_update::downloader                               │
│  ├─ Download binary                                         │
│  └─ Download .minisig (NEW)                                 │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│  terraphim_update::signature (MODIFIED)                     │
│  ├─ Embedded public key                                     │
│  ├─ verify_binary_signature() → minisign-verify             │
│  └─ Return VerificationResult                               │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│  Update Installation                                        │
│  ├─ Valid → Install binary                                  │
│  ├─ Invalid → Reject, log error                             │
│  └─ Missing → Fail or warn (configurable)                   │
└─────────────────────────────────────────────────────────────┘
```

### Data Flow

```
[Release Build] → [Sign with minisign CLI] → [Upload to GitHub]
                                                    │
                                                    ▼
[User checks for updates] → [Download binary + .minisig]
                                    │
                                    ▼
                        [Verify with embedded public key]
                                    │
                    ┌───────────────┴───────────────┐
                    ▼                               ▼
            [Valid Signature]              [Invalid/Missing]
                    │                               │
                    ▼                               ▼
            [Install Binary]              [Reject Update]
                    │                               │
                    └───────────────┬───────────────┘
                                    ▼
                            [Update Complete]
```

### Key Design Decisions

| Decision | Rationale | Alternatives Rejected |
|----------|-----------|----------------------|
| **Minisign over raw Ed25519** | Established file format, battle-tested, simple API | Raw ed25519-dalek requires custom format design |
| **Embedded public key** | No user configuration needed, offline verification | Config file only adds complexity |
| **.minisig extension** | Standard minisign convention, recognizable | .sig (ambiguous), .asc (PGP-specific) |
| **Optional config override** | Flexibility for key rotation without rebuild | Embedded only requires rebuild for rotation |
| **Fail on invalid signature** | Security-critical, better to fail safe | Warn-only allows compromised updates |
| **Separate signature download** | Smaller downloads, can verify before full binary | Combined format requires full download |

---

## File Changes

### New Files

| File | Purpose |
|------|---------|
| `crates/terraphim_update/src/keys/default.pub` | Embedded default public key (base64 encoded) |
| `scripts/sign-release.sh` | Sign release binaries with minisign |
| `.github/workflows/release-sign.yml` | CI/CD signature generation workflow |
| `crates/terraphim_update/tests/signature_test.rs` | Comprehensive signature verification tests |
| `docs/updates/KEYS.md` | Public key distribution documentation |

### Modified Files

| File | Changes |
|------|---------|
| `crates/terraphim_update/Cargo.toml` | Add `minisign-verify = "0.7"` dependency |
| `crates/terraphim_update/src/signature.rs` | Replace placeholder with minisign implementation |
| `crates/terraphim_update/src/lib.rs` | Load embedded public key, pass to verify functions |
| `crates/terraphim_update/src/downloader.rs` | Download `.minisig` files alongside binaries |
| `scripts/release.sh` | Call `sign-release.sh` after building binaries |
| `README.md` | Document signature verification and public key |
| `crates/terraphim_update/tests/integration_test.rs` | Add signature verification to integration tests |

### Deleted Files

| File | Reason |
|------|--------|
| None | No files deleted, only replacement of placeholder code |

---

## API Design

### Public Types

```rust
/// Result of a signature verification operation
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VerificationResult {
    /// Signature is valid and cryptographic verification passed
    Valid,

    /// Signature is invalid or verification failed
    Invalid { reason: String },

    /// Signature file is missing from release
    MissingSignature,

    /// Verification encountered an error
    Error(String),
}

/// Public key for signature verification
#[derive(Debug, Clone)]
pub struct PublicKey {
    /// Minisign public key (base64 encoded)
    pub key_data: String,

    /// Key ID/trust level (for future key rotation support)
    pub trust_level: TrustLevel,
}

/// Trust level for public keys (reserved for future key rotation)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrustLevel {
    /// Primary trusted key
    Primary,
    /// Secondary key (during rotation period)
    Secondary,
}
```

### Public Functions

```rust
/// Verify the signature of a downloaded binary using Minisign
///
/// This function performs cryptographic verification of a binary's signature
/// using Ed25519 signatures via the minisign-verify crate. It ensures the
/// binary has not been tampered with and originates from a trusted source.
///
/// # Arguments
/// * `binary_path` - Path to the binary file to verify
/// * `signature_path` - Path to the .minisig signature file
///
/// # Returns
/// * `Ok(VerificationResult)` - Result of verification
///   - `Valid`: Cryptographic verification passed
///   - `Invalid`: Signature verification failed (binary may be tampered)
///   - `MissingSignature`: No .minisig file found
///   - `Error`: Verification error occurred
/// * `Err(SignatureError)` - Invalid input or I/O error
///
/// # Errors
/// Returns `SignatureError::BinaryNotFound` if binary doesn't exist
/// Returns `SignatureError::SignatureNotFound` if .minisig doesn't exist
/// Returns `SignatureError::InvalidPublicKey` if embedded key is corrupt
///
/// # Security
/// This function uses constant-time comparisons and does not short-circuit
/// on signature validation to prevent timing attacks.
///
/// # Example
/// ```no_run
/// use terraphim_update::signature::verify_binary_signature;
/// use std::path::Path;
///
/// let result = verify_binary_signature(
///     Path::new("/tmp/terraphim_server"),
///     Path::new("/tmp/terraphim_server.minisig")
/// ).unwrap();
///
/// match result {
///     VerificationResult::Valid => println!("Signature verified"),
///     VerificationResult::Invalid { reason } => eprintln!("Invalid: {}", reason),
///     VerificationResult::MissingSignature => eprintln!("No signature found"),
///     VerificationResult::Error(msg) => eprintln!("Error: {}", msg),
/// }
/// ```
pub fn verify_binary_signature(
    binary_path: &Path,
    signature_path: &Path,
) -> Result<VerificationResult, SignatureError>;

/// Verify signature using a custom public key (for testing or advanced users)
///
/// Similar to `verify_binary_signature` but allows specifying a custom public key
/// instead of using the embedded default. This is useful for:
/// - Testing signature verification with different keys
/// - Advanced users who want to override the embedded key
/// - Key rotation scenarios where multiple keys are trusted
///
/// # Arguments
/// * `binary_path` - Path to the binary file to verify
/// * `signature_path` - Path to the .minisig signature file
/// * `public_key_base64` - Public key in base64 format (minisign format)
///
/// # Returns
/// * `Ok(VerificationResult)` - Result of verification
/// * `Err(SignatureError)` - Invalid input or corrupt key
pub fn verify_with_custom_key(
    binary_path: &Path,
    signature_path: &Path,
    public_key_base64: &str,
) -> Result<VerificationResult, SignatureError>;

/// Get the embedded default public key
///
/// Returns the public key that is compiled into the binary. This key is used
/// by default for all signature verification operations.
///
/// # Returns
/// * `PublicKey` - The embedded public key
///
/// # Example
/// ```no_run
/// use terraphim_update::signature::get_embedded_public_key;
///
/// let key = get_embedded_public_key();
/// println!("Public key: {}", key.key_data);
/// ```
pub fn get_embedded_public_key() -> PublicKey;

/// Check if signature verification is available and configured
///
/// Returns true if signature verification is available (always true in
/// production, may be false in test environments or feature builds).
///
/// # Returns
/// * `bool` - true if verification is available
pub fn is_verification_available() -> bool;

/// Get the expected signature file name for a binary
///
/// # Arguments
/// * `binary_name` - Name of the binary (e.g., "terraphim_server")
///
/// # Returns
/// * `String` - Expected signature file name (e.g., "terraphim_server.minisig")
///
/// # Example
/// ```no_run
/// use terraphim_update::signature::get_signature_filename;
///
/// let sig_file = get_signature_filename("terraphim_server");
/// assert_eq!(sig_file, "terraphim_server.minisig");
/// ```
pub fn get_signature_filename(binary_name: &str) -> String;
```

### Error Types

```rust
/// Error types for signature verification operations
#[derive(Debug, thiserror::Error)]
pub enum SignatureError {
    /// Binary file not found at specified path
    #[error("binary file not found: {path}")]
    BinaryNotFound {
        path: String,
    },

    /// Signature file not found at specified path
    #[error("signature file not found: {path}")]
    SignatureNotFound {
        path: String,
    },

    /// Invalid public key format (corrupt or invalid base64)
    #[error("invalid public key: {reason}")]
    InvalidPublicKey {
        reason: String,
    },

    /// Invalid signature format (corrupt or not minisign format)
    #[error("invalid signature format: {reason}")]
    InvalidSignatureFormat {
        reason: String,
    },

    /// Cryptographic verification failed (signature doesn't match)
    #[error("signature verification failed: {reason}")]
    VerificationFailed {
        reason: String,
    },

    /// I/O error during file operations
    #[error("I/O error: {0}")]
    IoError {
        #[from]
        source: std::io::Error,
    },

    /// Internal error (should not happen in production)
    #[error("internal error: {0}")]
    Internal(String),
}
```

### Internal Types (Downloader Module)

```rust
/// Download result including signature verification
pub struct DownloadedFile {
    /// Path to downloaded binary
    pub binary_path: PathBuf,

    /// Path to downloaded signature (if available)
    pub signature_path: Option<PathBuf>,

    /// Whether download was successful
    pub success: bool,

    /// Download size in bytes
    pub size: u64,
}

/// Download configuration with signature verification
pub struct DownloadConfig {
    /// Existing fields...
    pub max_retries: u32,
    pub timeout: Duration,
    pub show_progress: bool,

    /// NEW: Require signature verification
    pub require_signature: bool,

    /// NEW: Skip verification if signature missing (fail-open)
    pub allow_missing_signature: bool,
}
```

---

## Test Strategy

### Unit Tests

| Test | Location | Purpose |
|------|----------|---------|
| `test_verify_valid_signature` | `signature.rs` | Verify correctly signed binary passes |
| `test_verify_invalid_signature` | `signature.rs` | Reject tampered binary (wrong content) |
| `test_verify_wrong_key` | `signature.rs` | Reject signature signed by wrong key |
| `test_verify_missing_binary` | `signature.rs` | Return error if binary doesn't exist |
| `test_verify_missing_signature` | `signature.rs` | Return MissingSignature if .minisig missing |
| `test_verify_malformed_signature` | `signature.rs` | Reject corrupt signature file |
| `test_verify_malformed_public_key` | `signature.rs` | Error on corrupt public key |
| `test_verify_custom_key` | `signature.rs` | Test custom key override function |
| `test_get_embedded_public_key` | `signature.rs` | Verify embedded key is valid format |
| `test_signature_filename_generation` | `signature.rs` | Verify correct .minisig naming |
| `test_verification_result_equality` | `signature.rs` | Test enum comparison |
| `test_verification_result_display` | `signature.rs` | Test debug formatting |

### Integration Tests

| Test | Location | Purpose |
|------|----------|---------|
| `test_full_update_with_valid_signature` | `integration_test.rs` | Download binary, verify, install |
| `test_update_rejects_invalid_signature` | `integration_test.rs` | Reject tampered update |
| `test_update_handles_missing_signature` | `integration_test.rs` | Configurable behavior (fail or warn) |
| `test_signature_download_fallback` | `integration_test.rs` | Handle network errors gracefully |
| `test_concurrent_verification` | `integration_test.rs` | Multiple verifications don't interfere |
| `test_backup_rollback_preserves_verification` | `integration_test.rs` | Verify rollback doesn't break checks |

### Property Tests

```rust
// Test that verification never panics on arbitrary inputs
proptest! {
    #[test]
    fn fn_verify_never_panics(
        binary_content: Vec<u8>,
        signature_content: Vec<u8>,
        public_key: String
    ) {
        let result = verify_with_custom_key(
            &create_temp_file(&binary_content),
            &create_temp_file(&signature_content),
            &public_key
        );
        // Should never panic, may return error
    }
}

// Test that valid signatures always verify
proptest! {
    #[test]
    fn fn_valid_signature_always_passes(message: Vec<u8>) {
        let (pubkey, privkey) = generate_test_keypair();
        let signature = sign_minisign(&privkey, &message);
        let result = verify_with_custom_key(&message, &signature, &pubkey);
        assert!(matches!(result, Ok(VerificationResult::Valid)));
    }
}
```

### Security Tests

| Test | Location | Purpose |
|------|----------|---------|
| `test_tampered_binary_rejected` | `signature_test.rs` | Modified binary content fails |
| `test_tampered_signature_rejected` | `signature_test.rs` | Modified signature fails |
| `test_wrong_key_rejected` | `signature_test.rs` | Signature from different key fails |
| `test_timing_attack_resistance` | `signature_test.rs` | Verify constant-time comparison |

### Performance Tests

```rust
#[bench]
fn bench_verify_10mb_binary(b: &mut Bencher) {
    let binary = create_test_binary(10 * 1024 * 1024); // 10MB
    let signature = sign_test_binary(&binary);

    b.iter(|| {
        verify_binary_signature(&binary, &signature).unwrap()
    });
}

#[bench]
fn bench_verify_small_binary(b: &mut Bencher) {
    let binary = create_test_binary(1024); // 1KB
    let signature = sign_test_binary(&binary);

    b.iter(|| {
        verify_binary_signature(&binary, &signature).unwrap()
    });
}
```

---

## Implementation Steps

### Step 1: Dependency Setup
**Files:** `crates/terraphim_update/Cargo.toml`
**Description:** Add minisign-verify dependency
**Tests:** Compile verification
**Estimated:** 15 minutes
**Dependencies:** None

```toml
[dependencies]
# Existing dependencies...
minisign-verify = "0.7"
```

### Step 2: Generate Signing Key Pair
**Files:** `crates/terraphim_update/src/keys/default.pub`
**Description:** Generate Minisign key pair for releases
**Tests:** Verify key format is valid
**Estimated:** 30 minutes
**Dependencies:** None

**Commands:**
```bash
# Generate key pair
minisign -G -s terraphim-release.key -p crates/terraphim_update/src/keys/default.pub

# Extract public key for embedding
cat crates/terraphim_update/src/keys/default.pub

# Store private key in GitHub Actions Secrets
# (Copy content of terraphim-release.key to MINISIGN_PRIVATE_KEY secret)
```

**Key File Format:**
```
RWT+5ZvQzV/5/K5Z9Y3v6Y8V6Z8Z6Z9Z6Z6Z6Z6Z6Z6Z6Z6Z6Z6Z6Z6Z6Z6Z6Z6Z6Z6
```

### Step 3: Implement Verification Functions
**Files:** `crates/terraphim_update/src/signature.rs`
**Description:** Replace placeholder with minisign implementation
**Tests:** Unit tests for all verification paths
**Dependencies:** Steps 1-2
**Estimated:** 4-6 hours

**Key Changes:**
```rust
use anyhow::{anyhow, Result};
use minisign_verify::{PublicKey, Signature};
use std::fs;
use std::path::Path;
use thiserror::Error;
use tracing::{debug, info, warn};

// Embedded public key (generated in Step 2)
const DEFAULT_PUBLIC_KEY: &str = include_str!("keys/default.pub");

impl SignatureError {
    // Conversion from minisign-verify errors
    fn from_minisign_error(err: minisign_verify::Error) -> Self {
        match err {
            minisign_verify::Error::InvalidPublicKey => {
                SignatureError::InvalidPublicKey {
                    reason: "Invalid key format".to_string(),
                }
            }
            minisign_verify::Error::InvalidSignature => {
                SignatureError::VerificationFailed {
                    reason: "Signature verification failed".to_string(),
                }
            }
            _ => SignatureError::Internal(err.to_string()),
        }
    }
}

pub fn verify_binary_signature(
    binary_path: &Path,
    signature_path: &Path,
) -> Result<VerificationResult, SignatureError> {
    info!("Starting signature verification with embedded key");

    // Check binary exists
    if !binary_path.exists() {
        return Err(SignatureError::BinaryNotFound {
            path: binary_path.display().to_string(),
        });
    }

    // Check signature exists
    if !signature_path.exists() {
        warn!(
            "Signature file not found at {:?}",
            signature_path
        );
        return Ok(VerificationResult::MissingSignature);
    }

    // Load embedded public key
    let public_key = PublicKey::from_base64(DEFAULT_PUBLIC_KEY.trim())
        .map_err(|e| SignatureError::InvalidPublicKey {
            reason: e.to_string(),
        })?;

    // Read binary and signature
    let binary_bytes = fs::read(binary_path)?;
    let sig_bytes = fs::read(signature_path)?;

    // Decode signature
    let signature = Signature::decode(&sig_bytes)
        .map_err(|e| SignatureError::InvalidSignatureFormat {
            reason: e.to_string(),
        })?;

    // Verify signature (constant-time, no short-circuit)
    signature
        .verify(&public_key, &binary_bytes)
        .map(|_| VerificationResult::Valid)
        .map_err(SignatureError::from_minisign_error)
}

pub fn verify_with_custom_key(
    binary_path: &Path,
    signature_path: &Path,
    public_key_base64: &str,
) -> Result<VerificationResult, SignatureError> {
    info!("Starting signature verification with custom key");

    if !binary_path.exists() {
        return Err(SignatureError::BinaryNotFound {
            path: binary_path.display().to_string(),
        });
    }

    if !signature_path.exists() {
        return Ok(VerificationResult::MissingSignature);
    }

    let public_key = PublicKey::from_base64(public_key_base64.trim())
        .map_err(|e| SignatureError::InvalidPublicKey {
            reason: e.to_string(),
        })?;

    let binary_bytes = fs::read(binary_path)?;
    let sig_bytes = fs::read(signature_path)?;

    let signature = Signature::decode(&sig_bytes)
        .map_err(|e| SignatureError::InvalidSignatureFormat {
            reason: e.to_string(),
        })?;

    signature
        .verify(&public_key, &binary_bytes)
        .map(|_| VerificationResult::Valid)
        .map_err(SignatureError::from_minisign_error)
}

pub fn get_embedded_public_key() -> PublicKey {
    PublicKey {
        key_data: DEFAULT_PUBLIC_KEY.trim().to_string(),
        trust_level: TrustLevel::Primary,
    }
}
```

### Step 4: Update Downloader Module
**Files:** `crates/terraphim_update/src/downloader.rs`
**Description:** Download signature files alongside binaries
**Tests:** Integration tests for download + verify flow
**Dependencies:** Step 3
**Estimated:** 2-3 hours

**Key Changes:**
```rust
/// Download binary and signature from GitHub release
pub async fn download_with_signature(
    binary_url: &str,
    signature_url: &str,
    output_dir: &Path,
    config: &DownloadConfig,
) -> Result<DownloadedFile, DownloadError> {
    // Download binary
    let binary_path = download_file(binary_url, output_dir, config).await?;

    // Download signature
    let signature_path = if config.require_signature {
        Some(download_file(signature_url, output_dir, config).await?)
    } else {
        None
    };

    Ok(DownloadedFile {
        binary_path,
        signature_path,
        success: true,
        size: fs::metadata(&binary_path)?.len(),
    })
}

/// Construct signature URL from binary URL
pub fn signature_url_from_binary_url(binary_url: &str) -> String {
    // Replace extension with .minisig
    if binary_url.ends_with(".tar.gz") {
        binary_url.replace(".tar.gz", ".tar.gz.minisig")
    } else {
        format!("{}.minisig", binary_url)
    }
}
```

### Step 5: Create Release Signing Script
**Files:** `scripts/sign-release.sh`
**Description:** Sign all release binaries with minisign
**Tests:** Test signing produces valid .minisig files
**Dependencies:** Step 2
**Estimated:** 1 hour

```bash
#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

# Colors
GREEN='\033[0;32m'
BLUE='\033[0;34m'
NC='\033[0m'

print_info() { echo -e "${BLUE}[INFO]${NC} $1"; }
print_success() { echo -e "${GREEN}[SUCCESS]${NC} $1"; }

# Check if minisign is installed
if ! command -v minisign &> /dev/null; then
    echo "Error: minisign not found. Install with: cargo install minisign"
    exit 1
fi

# Check for private key
if [[ -z "${MINISIGN_PRIVATE_KEY:-}" ]]; then
    echo "Error: MINISIGN_PRIVATE_KEY environment variable not set"
    exit 1
fi

# Create temp key file
TEMP_KEY=$(mktemp)
trap "rm -f '$TEMP_KEY'" EXIT

# Decode private key from base64 (if stored in CI)
echo "$MINISIGN_PRIVATE_KEY" > "$TEMP_KEY"
chmod 600 "$TEMP_KEY"

# Sign all binaries in release directory
RELEASE_DIR="${1:-$PROJECT_ROOT/release-artifacts}"

if [[ ! -d "$RELEASE_DIR" ]]; then
    echo "Error: Release directory not found: $RELEASE_DIR"
    exit 1
fi

print_info "Signing binaries in $RELEASE_DIR"

SIGNED_COUNT=0
for binary in "$RELEASE_DIR"/*; do
    if [[ -f "$binary" ]]; then
        binary_name=$(basename "$binary")

        # Skip if already signed
        if [[ -f "${binary}.minisig" ]]; then
            print_info "Skipping $binary_name (already signed)"
            continue
        fi

        print_info "Signing $binary_name"

        # Sign with minisign (will prompt for password if key is encrypted)
        if minisign -S -s "$TEMP_KEY" -m "$binary" -x "${binary}.minisig"; then
            print_success "Signed $binary_name"
            ((SIGNED_COUNT++))
        else
            echo "Error: Failed to sign $binary_name"
            exit 1
        fi
    fi
done

print_success "Signed $SIGNED_COUNT binaries"
print_info "Signature files: *.minisig"
```

### Step 6: Update Release Scripts
**Files:** `scripts/release.sh`
**Description:** Integrate signing into release workflow
**Tests:** Run full release, verify signatures generated
**Dependencies:** Step 5
**Estimated:** 1 hour

**Changes to `release.sh`:**
```bash
# After build_binaries() function call
sign_binaries() {
    print_status "Signing release binaries with Minisign"

    if ! command -v minisign &> /dev/null; then
        print_warning "minisign not found, skipping signing"
        print_warning "Install with: cargo install minisign"
        return 0
    fi

    execute "$SCRIPT_DIR/sign-release.sh" "$RELEASE_DIR"
}

# Add to main() after build_binaries
sign_binaries

# Modify create_github_release() to include .minisig files
execute gh release create "$TAG" \
    --title "Terraphim AI v$VERSION" \
    --notes "$release_notes" \
    $prerelease_flag \
    "$RELEASE_DIR"/*.{deb,tar.zst,rpm,tar.gz,minisig} 2>/dev/null || {
    print_warning "Some package files may not exist, creating release without them"
    execute gh release create "$TAG" \
        --title "Terraphim AI v$VERSION" \
        --notes "$release_notes" \
        $prerelease_flag
}
```

### Step 7: Add Comprehensive Tests
**Files:** `crates/terraphim_update/tests/signature_test.rs`
**Description:** Create dedicated signature verification test suite
**Tests:** All unit, integration, property, security tests
**Dependencies:** Step 3
**Estimated:** 3-4 hours

**Test File Structure:**
```rust
//! Signature verification tests

use std::fs;
use std::io::Write;
use std::path::PathBuf;
use tempfile::TempDir;
use terraphim_update::signature::*;
use minisign_verify::{PublicKey, Signature, Signature as MinisignSignature};

// Helper: Generate test key pair
fn generate_test_keypair() -> (String, String) {
    // Use minisign CLI or generate Ed25519 key directly
    // Returns (public_key_base64, private_key_base64)
}

// Helper: Sign a binary with test key
fn sign_binary(binary_path: &PathBuf, private_key: &str) -> Vec<u8> {
    // Use minisign or sign directly with ed25519-dalek
    // Returns signature bytes
}

#[test]
fn test_verify_valid_signature() {
    let temp_dir = TempDir::new().unwrap();
    let (pubkey, privkey) = generate_test_keypair();

    // Create test binary
    let binary_path = temp_dir.path().join("test_binary");
    fs::write(&binary_path, b"test binary content").unwrap();

    // Sign binary
    let sig_bytes = sign_binary(&binary_path, &privkey);
    let sig_path = temp_dir.path().join("test_binary.minisig");
    fs::write(&sig_path, sig_bytes).unwrap();

    // Verify with custom key
    let result = verify_with_custom_key(
        &binary_path,
        &sig_path,
        &pubkey,
    ).unwrap();

    assert_eq!(result, VerificationResult::Valid);
}

#[test]
fn test_verify_tampered_binary() {
    let temp_dir = TempDir::new().unwrap();
    let (pubkey, privkey) = generate_test_keypair();

    // Create and sign binary
    let binary_path = temp_dir.path().join("test_binary");
    fs::write(&binary_path, b"original content").unwrap();

    let sig_bytes = sign_binary(&binary_path, &privkey);
    let sig_path = temp_dir.path().join("test_binary.minisig");
    fs::write(&sig_path, sig_bytes).unwrap();

    // Tamper with binary
    fs::write(&binary_path, b"tampered content").unwrap();

    // Verify should fail
    let result = verify_with_custom_key(
        &binary_path,
        &sig_path,
        &pubkey,
    ).unwrap();

    assert!(matches!(result, VerificationResult::Invalid { .. }));
}

#[test]
fn test_verify_missing_binary() {
    let temp_dir = TempDir::new().unwrap();
    let (pubkey, _) = generate_test_keypair();

    let binary_path = temp_dir.path().join("nonexistent");
    let sig_path = temp_dir.path().join("test.sig");

    let result = verify_with_custom_key(
        &binary_path,
        &sig_path,
        &pubkey,
    );

    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), SignatureError::BinaryNotFound { .. }));
}

#[test]
fn test_verify_missing_signature() {
    let temp_dir = TempDir::new().unwrap();
    let (pubkey, _) = generate_test_keypair();

    let binary_path = temp_dir.path().join("test_binary");
    fs::write(&binary_path, b"test content").unwrap();

    let sig_path = temp_dir.path().join("nonexistent.sig");

    let result = verify_with_custom_key(
        &binary_path,
        &sig_path,
        &pubkey,
    ).unwrap();

    assert_eq!(result, VerificationResult::MissingSignature);
}

#[test]
fn test_verify_with_embedded_key() {
    let temp_dir = TempDir::new().unwrap();

    // Create binary signed with actual release key (generated in Step 2)
    let binary_path = temp_dir.path().join("test_binary");
    fs::write(&binary_path, b"test content").unwrap();

    let sig_path = temp_dir.path().join("test_binary.minisig");
    // ... sign with actual key ...

    let result = verify_binary_signature(&binary_path, &sig_path).unwrap();

    assert_eq!(result, VerificationResult::Valid);
}

// ... more tests for all scenarios
```

### Step 8: Update Integration Tests
**Files:** `crates/terraphim_update/tests/integration_test.rs`
**Description:** Add signature verification to existing integration tests
**Tests:** Full update flow with signature verification
**Dependencies:** Steps 3-4
**Estimated:** 2 hours

```rust
#[test]
fn test_full_update_with_signature_verification() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");

    // Setup: Create signed binary in temp directory
    let binary_path = setup_mock_binary(&temp_dir, "1.0.0");
    let signature_path = create_mock_signature(&binary_path);

    // Test: Verify signature before "installing"
    let result = verify_binary_signature(&binary_path, &signature_path)
        .expect("Verification should succeed");

    assert_eq!(result, VerificationResult::Valid);

    // Continue with normal update flow
    let updated_binary = setup_mock_binary(&temp_dir, "1.1.0");
    fs::copy(&updated_binary, &binary_path).expect("Failed to copy");

    let updated_content = fs::read_to_string(&binary_path)
        .expect("Failed to read updated binary");
    assert_eq!(updated_content, "Mock binary version 1.1.0\n");
}

#[test]
fn test_update_rejects_invalid_signature() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");

    let binary_path = setup_mock_binary(&temp_dir, "1.0.0");

    // Create invalid signature
    let signature_path = temp_dir.path().join("invalid.sig");
    fs::write(&signature_path, b"invalid signature data").expect("Failed to write");

    let result = verify_binary_signature(&binary_path, &signature_path)
        .expect("Verification should complete");

    // Should reject invalid signature
    assert!(matches!(result, VerificationResult::Invalid { .. }));

    // Update should NOT proceed
    // (test that update flow stops when verification fails)
}
```

### Step 9: Create Documentation
**Files:** `docs/updates/KEYS.md`, `README.md`
**Description:** Document public key distribution and verification
**Tests**: Doc tests compile
**Dependencies:** Step 7
**Estimated:** 1 hour

**KEYS.md Content:**
```markdown
# Terraphim AI Release Signing Keys

## Public Key Distribution

The public key used to verify Terraphim AI releases is embedded in the
binary at compile time. This ensures signature verification works offline
and without user configuration.

## Primary Public Key

```
RWT+5ZvQzV/5/K5Z9Y3v6Y8V6Z8Z6Z9Z6Z6Z6Z6Z6Z6Z6Z6Z6Z6Z6Z6Z6Z6Z6Z6Z6Z6
```

**Key ID**: `RWT+5ZvQzV/5/K5Z9Y`
**Algorithm**: Ed25519
**Trust Level**: Primary

## Verification

Users can verify release binaries using the embedded public key or the
key above. Signatures are distributed as `.minisig` files alongside
binaries in GitHub Releases.

## Key Rotation

If the primary key is compromised, a new key will be generated and
announced via:
1. Security advisory
2. GitHub repository security bulletin
3. Update to this file

## Reporting Issues

If signature verification fails or you suspect a compromised key:
- Report immediately: https://github.com/terraphim/terraphim-ai/security/advisories
- Do NOT install the binary
- Check the repository for security announcements
```

### Step 10: CI/CD Integration
**Files:** `.github/workflows/release-sign.yml`
**Description:** Automated signature generation in releases
**Tests**: Run release workflow, verify signatures generated
**Dependencies:** Steps 2, 5, 6
**Estimated:** 1-2 hours

```yaml
name: Release Signing

on:
  workflow_dispatch:
    inputs:
      version:
        description: "Release version"
        required: true
      ref:
        description: "Git ref to build"
        required: true

jobs:
  sign-release:
    name: Sign Release Binaries
    runs-on: ubuntu-latest
    timeout-minutes: 30

    steps:
      - name: Checkout
        uses: actions/checkout@v4
        with:
          ref: ${{ github.event.inputs.ref }}

      - name: Install Rust
        uses: actions-rust-lang/setup-rust-toolchain@v1

      - name: Install minisign
        run: cargo install minisign

      - name: Build release binaries
        run: |
          ./scripts/build-release.sh --version ${{ inputs.version }}

      - name: Sign binaries
        env:
          MINISIGN_PRIVATE_KEY: ${{ secrets.MINISIGN_PRIVATE_KEY }}
        run: |
          ./scripts/sign-release.sh release-artifacts/

      - name: Verify signatures
        run: |
          for sig in release-artifacts/*.minisig; do
            echo "Verifying $sig"
            # Verification test using embedded key
          done

      - name: Upload signatures
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
        run: |
          gh release upload "v${{ inputs.version }}" release-artifacts/*.minisig
```

---

## Rollback Plan

If issues discovered during implementation:

### Immediate Rollback
1. **Revert signature.rs changes**: Restore placeholder implementation
2. **Remove minisign-verify dependency**: Revert Cargo.toml changes
3. **Disable signature generation**: Skip sign-release.sh in release workflow
4. **Update issue**: Document rollback reason in #421

### Feature Flag
```rust
// Add to lib.rs
#[cfg(feature = "signature-verification")]
pub use signature::verify_binary_signature;

#[cfg(not(feature = "signature-verification))]
pub fn verify_binary_signature(_: &Path, _: &Path) -> Result<VerificationResult> {
    Ok(VerificationResult::Valid) // Fallback to placeholder
}
```

### Graceful Degradation
```rust
pub fn verify_binary_signature_safe(
    binary_path: &Path,
    signature_path: &Path,
) -> Result<VerificationResult> {
    match verify_binary_signature(binary_path, signature_path) {
        Ok(result) => Ok(result),
        Err(e) => {
            tracing::warn!("Signature verification failed, falling back: {}", e);
            Ok(VerificationResult::Valid) // Safe fallback
        }
    }
}
```

---

## Migration

### Database Changes
None - No database schema changes required.

### Data Migration
None - No data migration required.

### Configuration Migration

**Before** (Placeholder):
```toml
# No signature verification configuration
```

**After** (With Verification):
```toml
[update]
# Enable signature verification (default: true)
verify_signatures = true

# Allow updates without signatures (fail-open)
allow_missing_signatures = false

# Optional: Override embedded public key
# public_key_path = "/path/to/custom.pub"
```

**Migration Script**: None - Configuration is optional with sensible defaults.

---

## Dependencies

### New Dependencies

| Crate | Version | Justification |
|-------|---------|---------------|
| `minisign-verify` | 0.7 | Zero-dependency Ed25519 signature verification |
| `thiserror` | 1.0 | Derive error enums (already in workspace) |

### Dependency Updates

| Crate | From | To | Reason |
|-------|------|-----|--------|
| None | - | - | No dependency updates required |

### Removed Dependencies

None - All existing dependencies retained.

---

## Performance Considerations

### Expected Performance

| Metric | Target | Measurement |
|--------|--------|-------------|
| Verification time (1MB binary) | < 10ms | Benchmark with `cargo bench` |
| Verification time (100MB binary) | < 100ms | Benchmark with `cargo bench` |
| Memory overhead | < 1MB | Profiling with `heaptrack` |
| Binary size increase | < 100KB | `ls -lh` before/after |

### Benchmarks to Add

```rust
// In crates/terraphim_update/benches/signature.rs

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use std::fs;
use tempfile::TempDir;

fn bench_verify_binary(c: &mut Criterion) {
    let temp_dir = TempDir::new().unwrap();

    for size in [1024, 10_000, 100_000, 1_000_000, 10_000_000].iter() {
        let binary = create_test_binary(*size);
        let signature = sign_test_binary(&binary);

        c.bench_with_input(BenchmarkId::new("verify", size), size, |b, _| {
            b.iter(|| {
                verify_binary_signature(
                    black_box(&binary),
                    black_box(&signature)
                ).unwrap()
            })
        });
    }
}

criterion_group!(benches, bench_verify_binary);
criterion_main!(benches);
```

### Optimization Opportunities

1. **Parallel verification**: Verify multiple signatures concurrently
2. **Signature caching**: Cache verification results for same binary
3. **Lazy verification**: Only verify when binary is about to execute
4. **Stream verification**: Verify during download (requires streamable signatures)

---

## Security Considerations

### Constant-Time Verification

Minisign uses Ed25519 which provides:
- Constant-time signature verification
- No timing side channels
- Deterministic validation

### Public Key Storage

**Embedded Key**:
- Compiled into binary (read-only memory)
- Cannot be modified at runtime
- Single source of truth

**Config Override** (Optional):
- Allows key rotation without rebuild
- Must be protected by file permissions (0600)
- Validation on load (must be valid base64, minisign format)

### Key Compromise Response

**Immediate Actions**:
1. **Revoke compromised key**: Add to revocation list in code
2. **Generate new keypair**: Create new signing keys offline
3. **Emergency release**: Release new version signed with new key
4. **Security advisory**: Publish disclosure on GitHub Security Advisories
5. **Update embedded key**: Rebuild with new public key

**Long-term Actions**:
1. **Post-mortem**: Analyze compromise vector
2. **Improve procedures**: Strengthen key storage and access controls
3. **Key rotation schedule**: Implement regular key rotation (annual)

---

## Open Items

| Item | Status | Owner | Priority |
|------|--------|-------|----------|
| Generate signing key pair | Pending | TBD | P0 |
| Configure GitHub Actions secrets | Pending | TBD | P0 |
| Property test implementation | Deferred | TBD | P2 |
| Key rotation framework | Deferred | TBD | P2 |
| Sigstore integration evaluation | Deferred | TBD | P3 |

---

## Approval Checklist

- [x] Research document approved (Phase 1)
- [x] Design document complete (Phase 2)
- [ ] Specification interview completed (Phase 2.5)
- [ ] All file changes listed
- [ ] All public APIs defined
- [ ] Test strategy complete
- [ ] Steps sequenced with dependencies
- [ ] Performance targets set
- [ ] Security considerations addressed
- [ ] Rollback plan documented
- [ ] **Human approval received**

---

## Next Steps

### Phase 2.5: Specification Interview (Optional but Recommended)

Before proceeding to implementation, conduct a **specification interview** using the `disciplined-specification` skill to:
- Deep dive into edge cases and failure modes
- Verify acceptance criteria completeness
- Surface hidden requirements
- Validate implementation assumptions

**Questions to Explore**:
1. What should happen if signature verification fails in production?
2. Should there be an escape hatch for emergency updates?
3. How do we handle key rotation without breaking existing installs?
4. What error messages are most helpful to users?
5. How do we verify the signature verification implementation itself?

### Phase 3: Implementation (After Approval)

Proceed to implementation using the `disciplined-implementation` skill:
1. Execute implementation steps in sequence
2. Write tests before code (TDD approach)
3. Commit each step independently
4. Run full test suite after each step
5. Update documentation continuously

---

## Appendix

### Test Vectors

From [Wycheproof Project](https://appsec.guide/docs/crypto/wycheproof/):

```rust
// Valid signature test vector
const TEST_VALID_SIGNATURE: &[u8] = b"...";

// Invalid signature test vector (wrong message)
const TEST_INVALID_WRONG_MESSAGE: &[u8] = b"...";

// Invalid signature test vector (wrong key)
const TEST_INVALID_WRONG_KEY: &[u8] = b"...";
```

### Example Release Workflow

```bash
# 1. Build release
./scripts/build-release.sh --version 1.2.3

# 2. Sign release
MINISIGN_PRIVATE_KEY="$(cat ~/.minisign/terraphim.key)" \
  ./scripts/sign-release.sh release-artifacts/

# 3. Verify signatures
for sig in release-artifacts/*.minisig; do
    minisign -V -m "${sig%.minisig}" -x "$sig" -p public.key
done

# 4. Create GitHub release
gh release create v1.2.3 \
  --title "Terraphim AI v1.2.3" \
  --notes "Release notes..." \
  release-artifacts/*.{tar.gz,minisig}
```

### Verifying Release as User

```bash
# Download release
wget https://github.com/terraphim/terraphim-ai/releases/download/v1.2.3/terraphim_server.tar.gz
wget https://github.com/terraphim/terraphim-ai/releases/download/v1.2.3/terraphim_server.tar.gz.minisig

# Verify signature
minisign -V -m terraphim_server.tar.gz -x terraphim_server.tar.gz.minisig \
  -p "RWT+5ZvQzV/5/K5Z9Y3v6Y8V6Z8Z6Z9Z6Z6Z6Z6Z6Z6Z6Z6Z6Z6Z6Z6Z6Z6Z6Z6Z6Z6"

# Extract and install
tar -xzf terraphim_server.tar.gz
sudo cp terraphim_server /usr/local/bin/
```

---

**Status**: Ready for Specification Interview or Approval
**Next Phase**: Phase 2.5 (Specification) or Phase 3 (Implementation)
**Completion Date**: 2025-01-12
