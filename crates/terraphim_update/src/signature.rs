//! Signature verification for downloaded updates
//!
//! This module provides signature verification capabilities to ensure
//! downloaded binaries are authentic and have not been tampered with.
//! Uses zipsign-api (included via self_update's "signatures" feature)
//! to verify Ed25519 signatures embedded in .tar.gz release archives.

use anyhow::{Context, Result, anyhow};
use base64::Engine;
use chrono::{DateTime, Utc};
use std::fs;
use std::io::Cursor;
use std::path::Path;
use tracing::{debug, info, warn};

// Re-export zipsign-api types for convenience
pub use zipsign_api::ZipsignError;

/// Get the embedded public key for Terraphim AI releases
///
/// This function returns the Ed25519 public key that is embedded in the binary
/// at compile time. This key is used to verify signatures of downloaded updates.
///
/// # Returns
/// Base64-encoded Ed25519 public key bytes
///
/// # Note
/// TODO: Replace with actual Terraphim AI public key after key generation
/// Run: ./scripts/generate-zipsign-keypair.sh
/// Then add the public key here
pub fn get_embedded_public_key() -> &'static str {
    // Ed25519 public key for verifying Terraphim AI release signatures
    // Generated: 2025-01-12
    // Key type: Ed25519 (32 bytes, base64-encoded)
    // Fingerprint: Calculate with: echo -n "1uLjooBMO+HlpKeiD16WOtT3COWeC8J/o2ERmDiEMc4=" | base64 -d | sha256sum
    "1uLjooBMO+HlpKeiD16WOtT3COWeC8J/o2ERmDiEMc4="
}

/// Metadata for cryptographic keys
///
/// This structure provides information about signing keys including
/// validity periods and key identifiers for future key rotation support.
#[derive(Debug, Clone)]
pub struct KeyMetadata {
    /// Unique identifier for this key
    pub key_id: String,
    /// When this key became valid
    pub valid_from: DateTime<Utc>,
    /// When this key expires (None = no expiry set)
    pub valid_until: Option<DateTime<Utc>>,
    /// Base64-encoded Ed25519 public key
    pub public_key: String,
}

/// Get the current active key metadata for Terraphim AI releases
///
/// This function provides metadata about the currently active signing key.
/// In the future, this will support key rotation by maintaining multiple
/// key metadata entries and selecting based on validity periods.
///
/// # Returns
/// Key metadata structure with key information
///
/// # Note
/// This is a basic implementation for v1.5.0. Full key rotation mechanism
/// is deferred to a future release. The current key has no expiration date.
pub fn get_active_key_metadata() -> KeyMetadata {
    KeyMetadata {
        key_id: "terraphim-release-key-2025-01".to_string(),
        valid_from: "2025-01-12T00:00:00Z".parse().unwrap(),
        valid_until: None, // No expiry set yet
        public_key: get_embedded_public_key().to_string(),
    }
}

/// Result of a signature verification operation
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VerificationResult {
    /// Signature is valid
    Valid,

    /// Signature is invalid
    Invalid { reason: String },

    /// Signature file is missing
    MissingSignature,

    /// Verification encountered an error
    Error(String),
}

/// Verify the signature of a downloaded archive
///
/// This function verifies that a .tar.gz archive has a valid Ed25519 signature
/// embedded using zipsign. Signatures are embedded directly in the archive
/// (as GZIP comment for .tar.gz files), not in separate signature files.
///
/// # Arguments
/// * `archive_path` - Path to .tar.gz archive file to verify
/// * `public_key` - Optional public key for verification (base64-encoded).
///
///  If None, uses the embedded public key.
///
/// # Returns
/// * `Ok(VerificationResult)` - Result of verification
/// * `Err(anyhow::Error)` - Error if verification process fails
///
/// # Example
/// ```no_run
/// use terraphim_update::signature::verify_archive_signature;
/// use std::path::Path;
///
/// let result = verify_archive_signature(
///     Path::new("/tmp/terraphim-1.0.0.tar.gz"),
///     None  // Use embedded public key
/// ).unwrap();
/// ```
pub fn verify_archive_signature(
    archive_path: &Path,
    public_key: Option<&str>,
) -> Result<VerificationResult> {
    info!("Starting signature verification for {:?}", archive_path);

    if !archive_path.exists() {
        return Err(anyhow!("Archive file not found: {:?}", archive_path));
    }

    // Use provided key or embedded key
    let key_str = match public_key {
        Some(k) => k,
        None => get_embedded_public_key(),
    };

    // Handle placeholder key - SECURITY: Never allow bypassing signature verification
    if key_str.starts_with("TODO:") {
        return Err(anyhow!(
            "Placeholder public key detected. Signature verification cannot be bypassed. \
            Configure a real Ed25519 public key in get_embedded_public_key()."
        ));
    }

    // Read the archive file
    let archive_bytes = fs::read(archive_path).context("Failed to read archive file")?;

    // Parse the public key (base64-encoded)
    let key_bytes = base64::engine::general_purpose::STANDARD
        .decode(key_str)
        .context("Failed to decode public key base64")?;

    // Public key must be exactly 32 bytes for Ed25519
    if key_bytes.len() != 32 {
        return Ok(VerificationResult::Invalid {
            reason: format!(
                "Invalid public key length: {} bytes (expected 32)",
                key_bytes.len()
            ),
        });
    }

    // Convert to array
    let mut key_array = [0u8; 32];
    key_array.copy_from_slice(&key_bytes);

    // Create verifying key
    let verifying_key = zipsign_api::verify::collect_keys(std::iter::once(Ok(key_array)))
        .context("Failed to parse public key")?;

    // Get the context (file name) for signature verification
    // zipsign uses the file name as context/salt by default
    let context: Option<Vec<u8>> = archive_path
        .file_name()
        .map(|n| n.to_string_lossy().as_bytes().to_vec());

    // Verify the .tar.gz archive signature using verify_tar
    // This function handles the tar.gz format with embedded signatures correctly
    let mut cursor = Cursor::new(archive_bytes);
    let context_ref: Option<&[u8]> = context.as_deref();
    match zipsign_api::verify::verify_tar(&mut cursor, &verifying_key, context_ref) {
        Ok(_index) => {
            info!("Signature verification passed for {:?}", archive_path);
            Ok(VerificationResult::Valid)
        }
        Err(e) => {
            warn!("Signature verification failed: {}", e);
            Ok(VerificationResult::Invalid {
                reason: format!("Signature verification failed: {}", e),
            })
        }
    }
}

/// Verify signature using self_update's built-in verification
///
/// This is a convenience wrapper around `verify_archive_signature`.
/// Note: When using `TerraphimUpdater::update()`, signature verification
/// is handled automatically by self_update via `.verifying_keys()`.
///
/// # Arguments
/// * `release_name` - Name of the release (e.g., "terraphim")
/// * `version` - Version string (e.g., "1.0.0")
/// * `archive_path` - Path to the .tar.gz archive to verify
/// * `public_key` - Public key for verification (base64-encoded, or None for embedded key)
///
/// # Returns
/// * `Ok(VerificationResult)` - Result of verification
/// * `Err(anyhow::Error)` - Error if verification fails
///
/// # Note
/// The `release_name` and `version` parameters are kept for API compatibility
/// but are not used in the verification itself. The actual verification uses
/// the archive filename as context (via zipsign).
///
/// # Example
/// ```no_run
/// use terraphim_update::signature::verify_with_self_update;
/// use std::path::Path;
///
/// let result = verify_with_self_update(
///     "terraphim",
///     "1.0.0",
///     Path::new("/tmp/terraphim-1.0.0.tar.gz"),
///     None  // Use embedded public key
/// ).unwrap();
/// ```
pub fn verify_with_self_update(
    _release_name: &str,
    _version: &str,
    archive_path: &Path,
    public_key: Option<&str>,
) -> Result<VerificationResult> {
    info!(
        "Verifying signature for {} v{} at {:?}",
        _release_name, _version, archive_path
    );

    if !archive_path.exists() {
        return Err(anyhow!("Archive file not found: {:?}", archive_path));
    }

    // Delegate to our proven signature verification
    verify_archive_signature(archive_path, public_key)
}

/// Verify signature with detailed error reporting
///
/// Similar to `verify_archive_signature` but provides more detailed error
/// information when verification fails. This is the recommended function
/// for most use cases.
///
/// # Arguments
/// * `archive_path` - Path to the .tar.gz archive file to verify
/// * `public_key` - Optional public key for verification (base64-encoded)
///
/// # Returns
/// * `Ok(VerificationResult)` - Result of verification with details
/// * `Err(anyhow::Error)` - Error if verification process fails
///
/// # Example
/// ```no_run
/// use terraphim_update::signature::{verify_signature_detailed, VerificationResult};
/// use std::path::Path;
///
/// let result = verify_signature_detailed(
///     Path::new("/tmp/terraphim-1.0.0.tar.gz"),
///     None  // Use embedded public key
/// ).unwrap();
///
/// match result {
///     VerificationResult::Valid => println!("Signature valid"),
///     VerificationResult::Invalid { reason } => eprintln!("Invalid: {}", reason),
///     VerificationResult::MissingSignature => eprintln!("No signature found"),
///     VerificationResult::Error(msg) => eprintln!("Error: {}", msg),
/// }
/// ```
pub fn verify_signature_detailed(
    archive_path: &Path,
    public_key: Option<&str>,
) -> Result<VerificationResult> {
    info!("Starting detailed signature verification");

    if !archive_path.exists() {
        return Ok(VerificationResult::Error(format!(
            "Archive file not found: {:?}",
            archive_path
        )));
    }

    debug!("Verifying archive {:?}", archive_path);

    verify_archive_signature(archive_path, public_key)
}

/// Check if signature verification is available
///
/// Returns true if signature verification is available and configured.
/// This can be used to conditionally enable signature verification
/// based on environment or configuration.
///
/// # Returns
/// * `true` - Signature verification is available
/// * `false` - Signature verification is not available
///
/// # Example
/// ```no_run
/// use terraphim_update::signature::is_verification_available;
///
/// if is_verification_available() {
///     println!("Signature verification enabled");
/// } else {
///     println!("Signature verification disabled");
/// }
/// ```
pub fn is_verification_available() -> bool {
    true
}

/// Get the expected signature file name for a binary
///
/// # Arguments
/// * `binary_name` - Name of the binary (e.g., "terraphim")
///
/// # Returns
/// * `String` - Expected signature file name (e.g., "terraphim.sig")
///
/// # Example
/// ```no_run
/// use terraphim_update::signature::get_signature_filename;
///
/// let sig_file = get_signature_filename("terraphim");
/// assert_eq!(sig_file, "terraphim.sig");
/// ```
pub fn get_signature_filename(binary_name: &str) -> String {
    format!("{}.sig", binary_name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_real_key_rejects_unsigned_file() {
        // With real public key, unsigned files should be rejected
        let temp_file = tempfile::NamedTempFile::new().unwrap();

        // Create a simple test file (not a signed archive)
        let result = verify_archive_signature(temp_file.path(), None).unwrap();

        // Real key rejects unsigned files
        assert!(matches!(result, VerificationResult::Invalid { .. }));
    }

    #[test]
    fn test_nonexistent_file_returns_error() {
        let result = verify_archive_signature(Path::new("/nonexistent/file.tar.gz"), None);

        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_base64_key_returns_error() {
        let temp_file = tempfile::NamedTempFile::new().unwrap();

        // Invalid base64 key - should return Err during decode
        let result = verify_archive_signature(temp_file.path(), Some("not-valid-base64!!!"));

        // Base64 decoding fails, so we get an error
        assert!(result.is_err());
    }

    #[test]
    fn test_wrong_length_key_returns_invalid() {
        let temp_file = tempfile::NamedTempFile::new().unwrap();

        // Valid base64 but wrong length (not 32 bytes)
        let result = verify_archive_signature(temp_file.path(), Some("VGVzdGluZw==")).unwrap();

        assert!(matches!(result, VerificationResult::Invalid { .. }));
    }

    #[test]
    fn test_is_verification_available() {
        let available = is_verification_available();
        assert!(available);
    }

    #[test]
    fn test_get_signature_filename() {
        assert_eq!(get_signature_filename("terraphim"), "terraphim.sig");
        assert_eq!(get_signature_filename("test"), "test.sig");
        assert_eq!(get_signature_filename("my-binary"), "my-binary.sig");
    }

    #[test]
    fn test_verification_result_equality() {
        let valid1 = VerificationResult::Valid;
        let valid2 = VerificationResult::Valid;
        assert_eq!(valid1, valid2);

        let invalid1 = VerificationResult::Invalid {
            reason: "test".to_string(),
        };
        let invalid2 = VerificationResult::Invalid {
            reason: "test".to_string(),
        };
        assert_eq!(invalid1, invalid2);

        let missing1 = VerificationResult::MissingSignature;
        let missing2 = VerificationResult::MissingSignature;
        assert_eq!(missing1, missing2);

        assert_ne!(valid1, missing1);
        assert_ne!(invalid1, missing1);
    }

    #[test]
    fn test_verification_result_display() {
        let valid = VerificationResult::Valid;
        let missing = VerificationResult::MissingSignature;
        let invalid = VerificationResult::Invalid {
            reason: "test error".to_string(),
        };
        let error = VerificationResult::Error("test error".to_string());

        assert_eq!(format!("{:?}", valid), "Valid");
        assert_eq!(format!("{:?}", missing), "MissingSignature");
        assert_eq!(
            format!("{:?}", invalid),
            "Invalid { reason: \"test error\" }"
        );
        assert_eq!(format!("{:?}", error), "Error(\"test error\")");
    }

    #[test]
    fn test_verify_signature_detailed_with_real_key() {
        let temp_file = tempfile::NamedTempFile::new().unwrap();

        let result = verify_signature_detailed(temp_file.path(), None).unwrap();

        // Real key rejects unsigned files
        assert!(matches!(result, VerificationResult::Invalid { .. }));
    }

    #[test]
    fn test_verify_signature_detailed_nonexistent() {
        let result =
            verify_signature_detailed(Path::new("/nonexistent/file.tar.gz"), None).unwrap();

        assert!(matches!(result, VerificationResult::Error(_)));
    }

    #[test]
    fn test_verify_with_self_update() {
        let temp_file = tempfile::NamedTempFile::new().unwrap();

        // Use a valid 32-byte base64-encoded test key (not a real signing key)
        // This key is just for testing the verification function works
        let test_key = "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA="; // 32 bytes of zeros, base64-encoded

        let result =
            verify_with_self_update("terraphim", "1.0.0", temp_file.path(), Some(test_key))
                .unwrap();

        // Unsigned file should be rejected with Invalid result
        assert!(matches!(result, VerificationResult::Invalid { .. }));
    }

    #[test]
    fn test_verify_with_self_update_missing_binary() {
        let result = verify_with_self_update(
            "terraphim",
            "1.0.0",
            Path::new("/nonexistent/binary"),
            Some("test-key"),
        );

        assert!(result.is_err());
    }
}
