//! Comprehensive signature verification tests
//!
//! This test suite provides thorough testing of signature verification
//! functionality including unit tests, integration tests, and edge cases.

use base64::Engine;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::TempDir;
use terraphim_update::signature::{
    get_signature_filename, is_verification_available, verify_archive_signature,
    verify_signature_detailed, verify_with_self_update, VerificationResult,
};

/// Helper function to create a test tar.gz archive
fn create_test_archive(dir: &PathBuf, name: &str) -> PathBuf {
    let archive_path = dir.join(name);

    // Create test file content
    let test_file = dir.join("test.txt");
    fs::write(&test_file, "Hello World\n").unwrap();

    // Use system tar command on Unix (produces standard format for zipsign)
    #[cfg(unix)]
    {
        let status = Command::new("tar")
            .args([
                "-czf",
                archive_path.to_str().unwrap(),
                "-C",
                dir.to_str().unwrap(),
                "test.txt",
            ])
            .status()
            .unwrap();
        assert!(
            status.success(),
            "Failed to create tar archive with system tar"
        );
        return archive_path;
    }

    // Non-Unix fallback: Create a simple tar.gz archive programmatically
    #[cfg(not(unix))]
    {
        let file = fs::File::create(&archive_path).unwrap();
        let enc = GzEncoder::new(file, flate2::Compression::default());
        let mut tar = Builder::new(enc);

        // Add some test files
        let mut header = tar::Header::new_gnu();
        header.set_path("test.txt").unwrap();
        header.set_size(12);
        header.set_mode(0o644);
        header.set_cksum();

        let mut data = "Hello World\n".as_bytes();
        tar.append(&header, &mut data).unwrap();
        tar.into_inner().unwrap().finish().unwrap();

        archive_path
    }
}

/// Helper function to sign an archive with zipsign
#[cfg(feature = "integration-signing")]
fn sign_archive(
    archive_path: &PathBuf,
    private_key: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    use std::process::Command;

    let output = Command::new("zipsign")
        .args(["sign", "tar", archive_path.to_str().unwrap(), private_key])
        .output()?;

    if !output.status.success() {
        return Err(format!(
            "zipsign failed: {}",
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }

    Ok(())
}

// ============================================================================
// Unit Tests
// ============================================================================

#[test]
fn test_real_key_rejects_unsigned_archive() {
    let temp_dir = TempDir::new().unwrap();
    let archive = create_test_archive(&temp_dir.path().to_path_buf(), "test.tar.gz");

    // With real embedded key, unsigned archives should be rejected
    let result = verify_archive_signature(&archive, None).unwrap();
    assert!(matches!(result, VerificationResult::Invalid { .. }));
    if let VerificationResult::Invalid { reason } = result {
        // verify_tar returns various error messages for unsigned archives
        assert!(
            reason.contains("Failed to read signatures")
                || reason.contains("magic")
                || reason.contains("no matching")
                || reason.contains("NoMatch")
                || reason.contains("could not find read signatures")
                || reason.contains("find data start"),
            "Unexpected error message: {}",
            reason
        );
    }
}

#[test]
fn test_nonexistent_archive_returns_error() {
    let result = verify_archive_signature(&PathBuf::from("/nonexistent/archive.tar.gz"), None);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("not found"));
}

#[test]
fn test_invalid_base64_key_returns_error() {
    let temp_dir = TempDir::new().unwrap();
    let archive = create_test_archive(&temp_dir.path().to_path_buf(), "test.tar.gz");

    let result = verify_archive_signature(&archive, Some("not-valid-base64!!!"));
    assert!(result.is_err());
}

#[test]
fn test_wrong_length_key_returns_invalid() {
    let temp_dir = TempDir::new().unwrap();
    let archive = create_test_archive(&temp_dir.path().to_path_buf(), "test.tar.gz");

    // Valid base64 but wrong length (not 32 bytes)
    let short_key = base64::engine::general_purpose::STANDARD.encode(b"short");
    let result = verify_archive_signature(&archive, Some(&short_key)).unwrap();

    assert!(matches!(result, VerificationResult::Invalid { .. }));
    if let VerificationResult::Invalid { reason } = result {
        // Check that reason mentions length issue
        assert!(reason.contains("32") || reason.contains("length") || reason.contains("Invalid"));
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

    let error1 = VerificationResult::Error("test error".to_string());
    let error2 = VerificationResult::Error("test error".to_string());
    assert_eq!(error1, error2);

    assert_ne!(valid1, missing1);
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
    let result = verify_signature_detailed(Path::new("/nonexistent/file.tar.gz"), None).unwrap();

    assert!(matches!(result, VerificationResult::Error(_)));
}

#[test]
fn test_verify_with_self_update() {
    let temp_file = tempfile::NamedTempFile::new().unwrap();

    // Use a valid 32-byte base64-encoded test key (not a real signing key)
    // This key is just for testing the verification function works
    let test_key = "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA="; // 32 bytes of zeros, base64-encoded

    let result =
        verify_with_self_update("terraphim", "1.0.0", temp_file.path(), Some(test_key)).unwrap();

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

// ============================================================================
// Integration Tests (require zipsign CLI)
// ============================================================================

#[cfg(feature = "integration-signing")]
mod integration_tests {
    use super::*;
    use std::process::Command;

    #[test]
    fn test_wrong_key_rejects_signed_archive() {
        // Skip if zipsign not installed
        if !Command::new("zipsign")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
        {
            println!("Skipping test: zipsign not installed");
            return;
        }

        let temp_dir = TempDir::new().unwrap();
        let key_dir = temp_dir.path().to_path_buf().join("keys");
        fs::create_dir(&key_dir).unwrap();

        let private_key = key_dir.join("private.key");
        let public_key = key_dir.join("public.key");

        // Generate key pair
        let output = Command::new("zipsign")
            .args([
                "gen-key",
                private_key.to_str().unwrap(),
                public_key.to_str().unwrap(),
            ])
            .output()
            .unwrap();

        assert!(output.status.success(), "Failed to generate key pair");

        // Create and sign archive
        let archive = create_test_archive(&temp_dir.path().to_path_buf(), "signed.tar.gz");
        sign_archive(&archive, private_key.to_str().unwrap()).unwrap();

        // Use a different public key
        let wrong_key = base64::engine::general_purpose::STANDARD.encode(vec![255u8; 32]);

        // Verify with wrong public key
        let result = verify_archive_signature(&archive, Some(&wrong_key)).unwrap();
        assert!(matches!(result, VerificationResult::Invalid { .. }));
    }

    #[test]
    fn test_tampered_archive_rejected() {
        // Skip if zipsign not installed
        if !Command::new("zipsign")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
        {
            println!("Skipping test: zipsign not installed");
            return;
        }

        let temp_dir = TempDir::new().unwrap();
        let key_dir = temp_dir.path().to_path_buf().join("keys");
        fs::create_dir(&key_dir).unwrap();

        let private_key = key_dir.join("private.key");
        let public_key = key_dir.join("public.key");

        // Generate key pair
        let output = Command::new("zipsign")
            .args([
                "gen-key",
                private_key.to_str().unwrap(),
                public_key.to_str().unwrap(),
            ])
            .output()
            .unwrap();

        assert!(output.status.success(), "Failed to generate key pair");

        // Create and sign archive
        let archive = create_test_archive(&temp_dir.path().to_path_buf(), "tampered.tar.gz");
        sign_archive(&archive, private_key.to_str().unwrap()).unwrap();

        // Tamper with archive by appending garbage
        {
            let mut file = fs::OpenOptions::new().append(true).open(&archive).unwrap();
            file.write_all(b"TAMPERED DATA").unwrap();
        }

        // Read public key (binary) and convert to base64
        // zipsign gen-key creates binary key files, not text
        let public_key_bytes = fs::read(&public_key).unwrap();
        let public_key_b64 = base64::engine::general_purpose::STANDARD.encode(&public_key_bytes);

        // Verify should fail
        let result = verify_archive_signature(&archive, Some(&public_key_b64)).unwrap();
        assert!(matches!(result, VerificationResult::Invalid { .. }));
    }

    #[test]
    fn test_signed_archive_verification() {
        // Skip if zipsign not installed
        let zipsign_check = Command::new("zipsign")
            .arg("--version")
            .output()
            .map(|o| {
                (
                    o.status.success(),
                    String::from_utf8_lossy(&o.stdout).trim().to_string(),
                )
            })
            .unwrap_or((false, String::new()));

        if !zipsign_check.0 {
            println!("Skipping test: zipsign not installed");
            return;
        }

        let zipsign_version = zipsign_check.1;
        println!("zipsign CLI version: {}", zipsign_version);

        // zipsign-api 0.2.x should be compatible with zipsign CLI 0.2.x
        // Extract version number from string like "zipsign 0.2.0"
        let version_num = zipsign_version.replace("zipsign ", "");
        if !version_num.starts_with("0.2") {
            println!(
                "Skipping test: zipsign CLI {} is not compatible with zipsign-api 0.2.x",
                zipsign_version
            );
            println!("To run this test, install zipsign v0.2.x");
            return;
        }

        let temp_dir = TempDir::new().unwrap();
        let key_dir = temp_dir.path().to_path_buf().join("keys");
        fs::create_dir(&key_dir).unwrap();

        let private_key = key_dir.join("private.key");
        let public_key = key_dir.join("public.key");

        // Generate key pair
        let output = Command::new("zipsign")
            .args([
                "gen-key",
                private_key.to_str().unwrap(),
                public_key.to_str().unwrap(),
            ])
            .output()
            .unwrap();

        assert!(output.status.success(), "Failed to generate key pair");

        // Create and sign archive
        let archive = create_test_archive(&temp_dir.path().to_path_buf(), "signed.tar.gz");
        sign_archive(&archive, private_key.to_str().unwrap()).unwrap();

        // Read public key (binary) and convert to base64
        let public_key_bytes = fs::read(&public_key).unwrap();
        let public_key_b64 = base64::engine::general_purpose::STANDARD.encode(&public_key_bytes);

        // Verify with correct public key - should pass
        let result = verify_archive_signature(&archive, Some(&public_key_b64)).unwrap();
        assert!(
            matches!(result, VerificationResult::Valid),
            "Expected Valid, got {:?}",
            result
        );
    }
}
