//! HMAC-SHA256 webhook payload signing.
//!
//! Provides secure signing and verification of webhook payloads.

use hmac::{Hmac, Mac};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

/// Sign a webhook payload using HMAC-SHA256.
///
/// # Arguments
/// * `payload` - The raw payload bytes to sign
/// * `secret` - The shared secret key
///
/// # Returns
/// Hex-encoded signature string
pub fn sign_payload(payload: &[u8], secret: &str) -> String {
    let mut mac =
        HmacSha256::new_from_slice(secret.as_bytes()).expect("HMAC can take key of any size");
    mac.update(payload);
    let result = mac.finalize();
    hex::encode(result.into_bytes())
}

/// Verify a webhook signature.
///
/// Uses constant-time comparison to prevent timing attacks.
///
/// # Arguments
/// * `payload` - The raw payload bytes
/// * `signature` - The hex-encoded signature to verify
/// * `secret` - The shared secret key
///
/// # Returns
/// true if the signature is valid
pub fn verify_signature(payload: &[u8], signature: &str, secret: &str) -> bool {
    // Decode the hex signature
    let signature_bytes = match hex::decode(signature) {
        Ok(bytes) => bytes,
        Err(_) => return false,
    };

    let mut mac =
        HmacSha256::new_from_slice(secret.as_bytes()).expect("HMAC can take key of any size");
    mac.update(payload);

    // Use verify_slice which performs constant-time comparison
    mac.verify_slice(&signature_bytes).is_ok()
}

/// Format signature for HTTP header.
///
/// Returns the signature in the format: `sha256=<hex_signature>`
pub fn format_signature_header(signature: &str) -> String {
    format!("sha256={}", signature)
}

/// Parse signature from HTTP header.
///
/// Extracts the hex signature from `sha256=<hex_signature>` format.
pub fn parse_signature_header(header: &str) -> Option<&str> {
    header.strip_prefix("sha256=")
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_SECRET: &str = "test-webhook-secret-key-12345";

    #[test]
    fn test_hmac_signing() {
        let payload = b"test payload";
        let signature = sign_payload(payload, TEST_SECRET);

        // Signature should be 64 hex characters (256 bits = 32 bytes = 64 hex chars)
        assert_eq!(signature.len(), 64);

        // Signature should be valid hex
        assert!(hex::decode(&signature).is_ok());
    }

    #[test]
    fn test_hmac_verify() {
        let payload = b"test payload";
        let signature = sign_payload(payload, TEST_SECRET);

        assert!(verify_signature(payload, &signature, TEST_SECRET));
    }

    #[test]
    fn test_hmac_verify_wrong_payload() {
        let payload = b"test payload";
        let signature = sign_payload(payload, TEST_SECRET);

        assert!(!verify_signature(b"wrong payload", &signature, TEST_SECRET));
    }

    #[test]
    fn test_hmac_verify_wrong_secret() {
        let payload = b"test payload";
        let signature = sign_payload(payload, TEST_SECRET);

        assert!(!verify_signature(payload, &signature, "wrong-secret"));
    }

    #[test]
    fn test_hmac_verify_wrong_signature() {
        let payload = b"test payload";

        // Invalid hex
        assert!(!verify_signature(payload, "not-hex", TEST_SECRET));

        // Valid hex but wrong signature
        assert!(!verify_signature(
            payload,
            "0000000000000000000000000000000000000000000000000000000000000000",
            TEST_SECRET
        ));
    }

    #[test]
    fn test_hmac_deterministic() {
        let payload = b"test payload";

        let sig1 = sign_payload(payload, TEST_SECRET);
        let sig2 = sign_payload(payload, TEST_SECRET);

        assert_eq!(sig1, sig2);
    }

    #[test]
    fn test_hmac_different_payloads() {
        let sig1 = sign_payload(b"payload1", TEST_SECRET);
        let sig2 = sign_payload(b"payload2", TEST_SECRET);

        assert_ne!(sig1, sig2);
    }

    #[test]
    fn test_hmac_different_secrets() {
        let payload = b"test payload";

        let sig1 = sign_payload(payload, "secret1");
        let sig2 = sign_payload(payload, "secret2");

        assert_ne!(sig1, sig2);
    }

    #[test]
    fn test_format_signature_header() {
        let signature = "abc123";
        let header = format_signature_header(signature);
        assert_eq!(header, "sha256=abc123");
    }

    #[test]
    fn test_parse_signature_header() {
        let result = parse_signature_header("sha256=abc123");
        assert_eq!(result, Some("abc123"));

        let invalid = parse_signature_header("invalid=abc123");
        assert_eq!(invalid, None);

        let no_prefix = parse_signature_header("abc123");
        assert_eq!(no_prefix, None);
    }

    #[test]
    fn test_json_payload_signing() {
        let json = serde_json::json!({
            "event": "test_event",
            "data": {
                "key": "value"
            }
        });
        let payload = serde_json::to_vec(&json).unwrap();

        let signature = sign_payload(&payload, TEST_SECRET);
        assert!(verify_signature(&payload, &signature, TEST_SECRET));
    }

    #[test]
    fn test_empty_payload() {
        let signature = sign_payload(b"", TEST_SECRET);
        assert!(verify_signature(b"", &signature, TEST_SECRET));
    }

    #[test]
    fn test_empty_secret() {
        let signature = sign_payload(b"payload", "");
        assert!(verify_signature(b"payload", &signature, ""));
    }

    #[test]
    fn test_unicode_payload() {
        let payload = "Hello, 世界! 🎉".as_bytes();
        let signature = sign_payload(payload, TEST_SECRET);
        assert!(verify_signature(payload, &signature, TEST_SECRET));
    }

    #[test]
    fn test_large_payload() {
        let payload = vec![0u8; 1_000_000]; // 1MB payload
        let signature = sign_payload(&payload, TEST_SECRET);
        assert!(verify_signature(&payload, &signature, TEST_SECRET));
    }
}
