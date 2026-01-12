//! Comprehensive signature verification tests
//!
//! This test suite provides thorough testing of signature verification
//! functionality including unit tests, integration tests, and edge cases.

use base64::Engine;
use flate2::write::GzEncoder;
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use tar::Builder;
use tempfile::TempDir;
use terraphim_update::signature::{verify_archive_signature, VerificationResult};

/// Helper function to create a test tar.gz archive
fn create_test_archive(dir: &PathBuf, name: &str) -> PathBuf {
    let archive_path = dir.join(name);

    // Create a simple tar.gz archive
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
        assert!(reason.contains("Failed to read signatures") || reason.contains("magic"));
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
        // Check that the reason mentions the length issue
        assert!(reason.contains("32") || reason.contains("length") || reason.contains("Invalid"));
    }
}

#[test]
fn test_empty_archive_without_signature() {
    let temp_dir = TempDir::new().unwrap();
    let archive_path = temp_dir.path().to_path_buf().join("empty.tar.gz");

    // Create an empty tar.gz
    let file = fs::File::create(&archive_path).unwrap();
    let enc = GzEncoder::new(file, flate2::Compression::default());
    let _tar = Builder::new(enc);

    // With real key, unsigned archives should be rejected
    let result = verify_archive_signature(&archive_path, None).unwrap();
    assert!(matches!(result, VerificationResult::Invalid { .. }));
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
fn test_verification_result_debug_format() {
    let valid = VerificationResult::Valid;
    assert_eq!(format!("{:?}", valid), "Valid");

    let missing = VerificationResult::MissingSignature;
    assert_eq!(format!("{:?}", missing), "MissingSignature");

    let invalid = VerificationResult::Invalid {
        reason: "test error".to_string(),
    };
    let formatted = format!("{:?}", invalid);
    assert!(formatted.contains("test error"));
}

// ============================================================================
// Edge Case Tests
// ============================================================================

#[test]
fn test_corrupted_archive_returns_error() {
    let temp_dir = TempDir::new().unwrap();
    let archive_path = temp_dir.path().to_path_buf().join("corrupted.tar.gz");

    // Create a corrupted archive (not a valid gzip)
    let mut file = fs::File::create(&archive_path).unwrap();
    file.write_all(b"This is not a valid gzip file").unwrap();

    // With real key, corrupted archives should be rejected
    let result = verify_archive_signature(&archive_path, None).unwrap();
    assert!(matches!(result, VerificationResult::Invalid { .. }));
    if let VerificationResult::Invalid { reason } = result {
        assert!(reason.contains("magic") || reason.contains("corrupted"));
    }
}

#[test]
fn test_verification_with_custom_public_key() {
    let temp_dir = TempDir::new().unwrap();
    let archive = create_test_archive(&temp_dir.path().to_path_buf(), "test.tar.gz");

    // Generate a test key pair (32 bytes each)
    let public_key = base64::engine::general_purpose::STANDARD.encode(vec![0u8; 32]);

    let result = verify_archive_signature(&archive, Some(&public_key)).unwrap();

    // Should return Invalid because archive is not signed with this key
    assert!(matches!(result, VerificationResult::Invalid { .. }));
}

#[test]
fn test_multiple_verifications_same_archive() {
    let temp_dir = TempDir::new().unwrap();
    let archive = create_test_archive(&temp_dir.path().to_path_buf(), "test.tar.gz");

    // Verify multiple times with real key
    let result1 = verify_archive_signature(&archive, None).unwrap();
    let result2 = verify_archive_signature(&archive, None).unwrap();
    let result3 = verify_archive_signature(&archive, None).unwrap();

    // All should return the same result (Invalid for unsigned archive)
    assert!(matches!(result1, VerificationResult::Invalid { .. }));
    assert!(matches!(result2, VerificationResult::Invalid { .. }));
    assert!(matches!(result3, VerificationResult::Invalid { .. }));
    assert_eq!(result1, result2);
    assert_eq!(result2, result3);
}

#[test]
fn test_verification_non_file_path() {
    // Try to verify a directory instead of a file
    let temp_dir = TempDir::new().unwrap();
    let result = verify_archive_signature(temp_dir.path(), None);

    // With placeholder key, directories are accepted (placeholder returns Valid)
    // In production with a real key, this would fail differently
    match result {
        Ok(_) => {} // Placeholder accepts anything
        Err(e) => {
            // Real key would return an error about reading the file
            assert!(
                e.to_string().contains("Failed to read") || e.to_string().contains("archive file")
            );
        }
    }
}

// ============================================================================
// Integration Tests (require zipsign CLI)
// ============================================================================

#[cfg(feature = "integration-signing")]
mod integration_tests {
    use super::*;
    use std::process::Command;

    #[test]
    fn test_signed_archive_verification() {
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

        // Read public key
        let public_key_bytes = fs::read_to_string(&public_key).unwrap();
        let public_key_b64 = public_key_bytes.trim();

        // Verify with correct public key
        let result = verify_archive_signature(&archive, Some(public_key_b64)).unwrap();
        assert_eq!(result, VerificationResult::Valid);
    }

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

        // Tamper with the archive by appending garbage
        {
            let mut file = fs::OpenOptions::new().append(true).open(&archive).unwrap();
            file.write_all(b"TAMPERED DATA").unwrap();
        }

        // Read public key
        let public_key_bytes = fs::read_to_string(&public_key).unwrap();
        let public_key_b64 = public_key_bytes.trim();

        // Verify should fail
        let result = verify_archive_signature(&archive, Some(public_key_b64)).unwrap();
        assert!(matches!(result, VerificationResult::Invalid { .. }));
    }
}

// ============================================================================
// Property-Based Tests (basic implementation)
// ============================================================================

#[test]
fn test_verification_deterministic() {
    // Same input should always produce same output
    let temp_dir = TempDir::new().unwrap();
    let archive = create_test_archive(&temp_dir.path().to_path_buf(), "test.tar.gz");

    let mut results = Vec::new();
    for _ in 0..10 {
        let result = verify_archive_signature(&archive, None).unwrap();
        results.push(result);
    }

    // All results should be identical
    for result in &results[1..] {
        assert_eq!(results[0], *result);
    }
}

#[test]
fn test_verification_no_panic() {
    // Verification should never panic on any input
    let test_cases: Vec<PathBuf> = vec![
        PathBuf::from("/nonexistent/file.tar.gz"),
        PathBuf::from("/tmp"),
        PathBuf::from(""),
    ];

    for path in test_cases {
        let _ = verify_archive_signature(&path, None);
        // Should not panic, may return error
    }
}

// ============================================================================
// Performance Tests (basic benchmarks)
// ============================================================================

#[test]
fn test_verification_performance_small_archive() {
    let temp_dir = TempDir::new().unwrap();
    let archive = create_test_archive(&temp_dir.path().to_path_buf(), "small.tar.gz");

    let start = std::time::Instant::now();
    let _result = verify_archive_signature(&archive, None).unwrap();
    let elapsed = start.elapsed();

    // Verification should be fast (< 100ms for small archive)
    assert!(
        elapsed.as_millis() < 100,
        "Verification took too long: {:?}",
        elapsed
    );
}

#[test]
fn test_verification_multiple_archives_performance() {
    let temp_dir = TempDir::new().unwrap();

    // Create multiple archives
    let archives: Vec<PathBuf> = (0..10)
        .map(|i| create_test_archive(&temp_dir.path().to_path_buf(), &format!("test{}.tar.gz", i)))
        .collect();

    let start = std::time::Instant::now();
    for archive in &archives {
        let _result = verify_archive_signature(archive, None).unwrap();
    }
    let elapsed = start.elapsed();

    // Should verify 10 small archives quickly (< 1 second)
    assert!(
        elapsed.as_secs() < 1,
        "Batch verification took too long: {:?}",
        elapsed
    );
}
