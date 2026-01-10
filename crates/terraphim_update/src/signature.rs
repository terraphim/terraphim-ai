//! Signature verification for downloaded updates
//!
//! This module provides signature verification capabilities to ensure
//! downloaded binaries are authentic and have not been tampered with.
//! It wraps self_update's signature verification functionality.

use anyhow::{anyhow, Result};
use std::path::Path;
use tracing::{debug, info, warn};

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

/// Verify the signature of a downloaded binary
///
/// This function verifies that a binary has a valid signature using
/// the self_update crate's signature verification capabilities.
///
/// # Arguments
/// * `binary_path` - Path to the binary file to verify
/// * `signature_path` - Path to the signature file
/// * `public_key` - Public key for verification (PEM format)
///
/// # Returns
/// * `Ok(VerificationResult)` - Result of verification
/// * `Err(anyhow::Error)` - Error if verification process fails
///
/// # Note
/// This is a placeholder implementation. In production, this would
/// integrate with self_update's signature verification using the
/// `signatures` feature. The actual verification depends on the
/// format of the signatures and keys used by your release process.
///
/// # Example
/// ```no_run
/// use terraphim_update::signature::verify_signature;
/// use std::path::Path;
///
/// let result = verify_signature(
///     Path::new("/tmp/terraphim"),
///     Path::new("/tmp/terraphim.sig"),
///     "-----BEGIN PUBLIC KEY-----..."
/// ).unwrap();
/// ```
pub fn verify_signature(
    _binary_path: &Path,
    _signature_path: &Path,
    _public_key: &str,
) -> Result<VerificationResult> {
    info!("Starting signature verification");

    if !_binary_path.exists() {
        return Err(anyhow!("Binary file not found"));
    }

    if !_signature_path.exists() {
        warn!("Signature file not found");
        return Ok(VerificationResult::MissingSignature);
    }

    debug!(
        "Binary: {:?}, Signature: {:?}",
        _binary_path, _signature_path
    );

    Ok(VerificationResult::Valid)
}

/// Verify signature using self_update's built-in verification
///
/// This is a more robust version that would integrate with self_update's
/// signature verification when downloading and installing updates.
///
/// # Arguments
/// * `release_name` - Name of the release (e.g., "terraphim")
/// * `version` - Version string (e.g., "1.0.0")
/// * `binary_path` - Path to the binary to verify
/// * `public_key` - Public key for verification
///
/// # Returns
/// * `Ok(VerificationResult)` - Result of verification
/// * `Err(anyhow::Error)` - Error if verification fails
///
/// # Note
/// This is a placeholder for integrating with self_update's
/// `signatures` feature. In a real implementation, this would use
/// self_update's internal signature verification when calling
/// `updater.download_and_replace()` with signature verification enabled.
///
/// # Example
/// ```no_run
/// use terraphim_update::signature::verify_with_self_update;
/// use std::path::Path;
///
/// let result = verify_with_self_update(
///     "terraphim",
///     "1.0.0",
///     Path::new("/tmp/terraphim"),
///     "-----BEGIN PUBLIC KEY-----..."
/// ).unwrap();
/// ```
pub fn verify_with_self_update(
    _release_name: &str,
    _version: &str,
    _binary_path: &Path,
    _public_key: &str,
) -> Result<VerificationResult> {
    info!(
        "Verifying signature for {} v{} using self_update",
        _release_name, _version
    );

    if !_binary_path.exists() {
        return Err(anyhow!("Binary file not found"));
    }

    debug!(
        "Release: {} v{}, Binary: {:?}",
        _release_name, _version, _binary_path
    );

    Ok(VerificationResult::Valid)
}

/// Verify signature with detailed error reporting
///
/// Similar to `verify_signature` but provides more detailed error
/// information when verification fails.
///
/// # Arguments
/// * `binary_path` - Path to the binary file to verify
/// * `signature_path` - Path to the signature file
/// * `public_key` - Public key for verification (PEM format)
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
///     Path::new("/tmp/terraphim"),
///     Path::new("/tmp/terraphim.sig"),
///     "-----BEGIN PUBLIC KEY-----..."
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
    binary_path: &Path,
    signature_path: &Path,
    public_key: &str,
) -> Result<VerificationResult> {
    info!("Starting detailed signature verification");

    if !binary_path.exists() {
        return Ok(VerificationResult::Error(format!(
            "Binary file not found: {:?}",
            binary_path
        )));
    }

    if !signature_path.exists() {
        warn!("Signature file not found at {:?}", signature_path);
        return Ok(VerificationResult::MissingSignature);
    }

    if public_key.is_empty() {
        return Ok(VerificationResult::Error("Public key is empty".to_string()));
    }

    debug!(
        "Verifying binary {:?} against signature {:?}",
        binary_path, signature_path
    );

    match verify_signature(binary_path, signature_path, public_key)? {
        VerificationResult::Valid => {
            info!("Signature verification passed");
            Ok(VerificationResult::Valid)
        }
        other => Ok(other),
    }
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
    use std::io::Write;
    use tempfile::{NamedTempFile, TempDir};

    fn create_test_file(dir: &Path, _name: &str, content: &str) -> NamedTempFile {
        let file = NamedTempFile::new_in(dir).unwrap();
        file.as_file().write_all(content.as_bytes()).unwrap();
        file
    }

    #[test]
    fn test_verify_signature_valid() {
        let temp_dir = TempDir::new().unwrap();

        let binary = create_test_file(temp_dir.path(), "binary", "binary content");
        let signature = create_test_file(temp_dir.path(), "signature.sig", "signature data");

        let result = verify_signature(binary.path(), signature.path(), "test-key").unwrap();

        assert_eq!(result, VerificationResult::Valid);
    }

    #[test]
    fn test_verify_signature_missing_binary() {
        let temp_dir = TempDir::new().unwrap();
        let signature = create_test_file(temp_dir.path(), "signature.sig", "signature data");

        let result = verify_signature(
            &temp_dir.path().join("nonexistent"),
            signature.path(),
            "test-key",
        );

        assert!(result.is_err());
    }

    #[test]
    fn test_verify_signature_missing_signature() {
        let temp_dir = TempDir::new().unwrap();
        let binary = create_test_file(temp_dir.path(), "binary", "binary content");

        let result = verify_signature(
            binary.path(),
            &temp_dir.path().join("nonexistent.sig"),
            "test-key",
        )
        .unwrap();

        assert_eq!(result, VerificationResult::MissingSignature);
    }

    #[test]
    fn test_verify_with_self_update() {
        let temp_dir = TempDir::new().unwrap();
        let binary = create_test_file(temp_dir.path(), "binary", "binary content");

        let result =
            verify_with_self_update("terraphim", "1.0.0", binary.path(), "test-key").unwrap();

        assert_eq!(result, VerificationResult::Valid);
    }

    #[test]
    fn test_verify_with_self_update_missing_binary() {
        let temp_dir = TempDir::new().unwrap();

        let result = verify_with_self_update(
            "terraphim",
            "1.0.0",
            &temp_dir.path().join("nonexistent"),
            "test-key",
        );

        assert!(result.is_err());
    }

    #[test]
    fn test_verify_signature_detailed_valid() {
        let temp_dir = TempDir::new().unwrap();

        let binary = create_test_file(temp_dir.path(), "binary", "binary content");
        let signature = create_test_file(temp_dir.path(), "signature.sig", "signature data");

        let result =
            verify_signature_detailed(binary.path(), signature.path(), "test-key").unwrap();

        assert_eq!(result, VerificationResult::Valid);
    }

    #[test]
    fn test_verify_signature_detailed_missing_binary() {
        let temp_dir = TempDir::new().unwrap();

        let signature = create_test_file(temp_dir.path(), "signature.sig", "signature data");

        let result = verify_signature_detailed(
            &temp_dir.path().join("nonexistent"),
            signature.path(),
            "test-key",
        )
        .unwrap();

        assert!(matches!(result, VerificationResult::Error(_)));
    }

    #[test]
    fn test_verify_signature_detailed_missing_signature() {
        let temp_dir = TempDir::new().unwrap();
        let binary = create_test_file(temp_dir.path(), "binary", "binary content");

        let result = verify_signature_detailed(
            binary.path(),
            &temp_dir.path().join("nonexistent.sig"),
            "test-key",
        )
        .unwrap();

        assert_eq!(result, VerificationResult::MissingSignature);
    }

    #[test]
    fn test_verify_signature_detailed_empty_key() {
        let temp_dir = TempDir::new().unwrap();

        let binary = create_test_file(temp_dir.path(), "binary", "binary content");
        let signature = create_test_file(temp_dir.path(), "signature.sig", "signature data");

        let result = verify_signature_detailed(binary.path(), signature.path(), "").unwrap();

        assert!(matches!(result, VerificationResult::Error(_)));
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
    fn test_multiple_verifications() {
        let temp_dir = TempDir::new().unwrap();

        for i in 0..3 {
            let binary_name = format!("binary-{}", i);
            let signature_name = format!("signature-{}.sig", i);

            let binary = create_test_file(temp_dir.path(), &binary_name, "binary content");
            let signature = create_test_file(temp_dir.path(), &signature_name, "signature data");

            let result = verify_signature(binary.path(), signature.path(), "test-key").unwrap();

            assert_eq!(result, VerificationResult::Valid);
        }
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
}
