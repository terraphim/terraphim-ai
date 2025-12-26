use anyhow::Result;
use hmac::{Hmac, Mac};
use sha2::Sha256;
use subtle::ConstantTimeEq;

/// Verify GitHub webhook signature using HMAC-SHA256
///
/// # Arguments
/// * `secret` - The webhook secret configured in GitHub
/// * `signature` - The value from X-Hub-Signature-256 header (includes "sha256=" prefix)
/// * `body` - The raw request body bytes
///
/// # Returns
/// * `Ok(true)` if signature is valid
/// * `Ok(false)` if signature doesn't match
/// * `Err` if verification fails
pub fn verify_signature(secret: &str, signature: &str, body: &[u8]) -> Result<bool> {
    // Strip prefix without allocation
    let signature_bytes = signature
        .strip_prefix("sha256=")
        .ok_or_else(|| anyhow::anyhow!("Invalid signature format: missing sha256= prefix"))?;

    // Decode expected signature to bytes (handle invalid hex gracefully)
    let expected = match hex::decode(signature_bytes) {
        Ok(bytes) => bytes,
        Err(_) => return Ok(false), // Invalid hex means signature doesn't match
    };

    // Compute HMAC
    let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes())?;
    mac.update(body);
    let result = mac.finalize().into_bytes();

    // Constant-time comparison of bytes (no hex encoding needed)
    Ok(result.as_slice().ct_eq(&expected).into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_verify_signature_valid() {
        let secret = "test_secret";
        let body = b"test payload";

        // Generate valid signature
        let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes()).unwrap();
        mac.update(body);
        let signature = format!("sha256={}", hex::encode(mac.finalize().into_bytes()));

        let result = verify_signature(secret, &signature, body);

        assert!(result.unwrap());
    }

    #[test]
    fn test_verify_signature_invalid() {
        let secret = "test_secret";
        let body = b"test payload";

        let result = verify_signature(secret, "sha256=invalid", body);

        assert!(!result.unwrap());
    }

    #[test]
    fn test_verify_signature_wrong_secret() {
        let secret1 = "secret1";
        let secret2 = "secret2";
        let body = b"test payload";

        // Generate signature with secret1
        let mut mac = Hmac::<Sha256>::new_from_slice(secret1.as_bytes()).unwrap();
        mac.update(body);
        let signature = format!("sha256={}", hex::encode(mac.finalize().into_bytes()));

        // Verify with secret2
        let result = verify_signature(secret2, &signature, body);

        assert!(!result.unwrap());
    }
}
