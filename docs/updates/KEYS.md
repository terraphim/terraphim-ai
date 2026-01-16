# Terraphim AI Update Signing Keys

This document describes how Terraphim AI release binaries are cryptographically signed and how to verify the authenticity of downloaded updates.

## Overview

Terraphim AI uses **Ed25519** signatures to verify that downloaded update binaries are authentic and have not been tampered with. Ed25519 is a modern elliptic curve signature scheme that provides:

- **Strong security**: 128-bit security level (equivalent to 3000-bit RSA)
- **Fast verification**: Optimized for quick verification on all platforms
- **Small signatures**: 64-byte signatures (much smaller than RSA/PGP)
- **No key confusion**: Ed25519 prevents signature ambiguity attacks

## Public Key Distribution

### Primary Method: Embedded Public Key

The Ed25519 public key is embedded directly in the Terraphim AI binary at compile time:

**Location**: `crates/terraphim_update/src/signature.rs`

```rust
fn get_embedded_public_key() -> &'static str {
    // Base64-encoded Ed25519 public key (32 bytes)
    "BASE64_ENCODED_PUBLIC_KEY_HERE"
}
```

**How it works**:
1. The public key is compiled into the binary during the build process
2. When checking for updates, the binary verifies signatures using this embedded key
3. No external key files or configuration needed
4. Key cannot be modified without recompiling the binary

### Alternative Methods (for advanced users)

#### 1. Environment Variable Override

For testing or emergency key rotation, you can override the embedded key:

```bash
export TERRAPHIM_UPDATE_PUBLIC_KEY="base64-encoded-key-here"
```

#### 2. Configuration File

Some installations may support specifying a custom public key in the configuration file (check your specific deployment documentation).

## Key Generation Process

Terraphim AI maintainers generate Ed25519 key pairs using the [zipsign](https://github.com/Kijewski/zipsign) tool:

### Generating a New Key Pair

```bash
# Run the provided key generation script
./scripts/generate-zipsign-keypair.sh
```

This generates:
- `keys/private.key` - **SECRET** signing key (store securely!)
- `keys/public.key` - Public verification key (embed in code)

### Private Key Storage

**IMPORTANT**: The private signing key is stored securely using 1Password or equivalent password manager.

**Security practices**:
- Private key is **never** committed to git
- `keys/` directory is in `.gitignore`
- Only trusted maintainers have access to the signing key
- Key is rotated if compromised or periodically (e.g., annually)

## Signature Format

Terraphim AI uses **embedded signatures** rather than separate signature files:

### Archive Signatures

- **TAR.GZ files**: Signature stored in GZIP comment field
- **TAR.ZST files**: Signature stored in Zstandard comment field
- **ZIP files**: Signature prepended to the archive

**Advantages**:
- No separate `.sig` files to download
- Signatures travel with the archive
- Cannot accidentally download archive without signature
- Simpler distribution process

### Verification Process

When you download a Terraphim AI update:

1. Binary downloads the release archive (`.tar.gz`)
2. Signature verification reads the embedded signature from the archive
3. Verification uses the embedded Ed25519 public key
4. Archive is installed **only if** signature is valid

**Failure modes**:
- ❌ Invalid signature → Update rejected, security warning logged
- ❌ Missing signature → Update rejected
- ❌ Verification error → Update rejected

## Verifying Downloaded Archives Manually

You can manually verify a downloaded archive using the zipsign CLI:

### Installing zipsign

```bash
# Install from crates.io
cargo install zipsign

# Or build from source
cargo install --git https://github.com/Kijewski/zipsign
```

### Extracting the Public Key

The public key is available in the source code:

```bash
# Extract from source code
grep -A 2 'fn get_embedded_public_key' crates/terraphim_update/src/signature.rs
```

### Verifying an Archive

```bash
# Verify a downloaded archive
zipsign verify tar terraphim-ai-1.0.0.tar.gz public.key

# Expected output:
# Signature by KEY_ID verified successfully
```

## Key Rotation

### Planned Rotation (v1.1+)

Future versions will support multiple trusted public keys to enable smooth key rotation:

```rust
fn get_trusted_public_keys() -> &'static [&'static str] {
    &[
        "CURRENT_KEY_BASE64",
        "PREVIOUS_KEY_BASE64",  // Accept for grace period
    ]
}
```

### Emergency Rotation

If the signing key is compromised:

1. **Immediate**: Revoke compromised key in next release
2. **Generate**: Create new key pair using secure environment
3. **Update**: Embed new public key in code
4. **Release**: Sign new release with new key
5. **Announce**: Publish security advisory with key fingerprint

### Key Fingerprint

Each Ed25519 public key has a unique fingerprint (SHA-256 hash):

```
# Calculate fingerprint
echo -n "PUBLIC_KEY_BASE64" | base64 -d | sha256sum
```

**Terraphim AI Official Keys**:

| Key Version | Fingerprint (SHA-256) | Valid From | Status |
|-------------|----------------------|------------|--------|
| v1.0        | `1c78db3c8e1afa3af4fcbaf32ccfa30988c82f9e7d383dfb127ae202732b631a` | 2025-01-12 | Active |

**Public Key (base64-encoded)**:
```
1uLjooBMO+HlpKeiD16WOtT3COWeC8J/o2ERmDiEMc4=
```

**Key Location**: Embedded in `crates/terraphim_update/src/signature.rs`

## Security Considerations

### Threat Model

**Signature verification protects against**:
- ✅ Man-in-the-middle attacks on downloads
- ✅ Compromised download servers/CDNs
- ✅ Malicious actors modifying binaries
- ✅ Supply chain attacks during distribution

**Signature verification does NOT protect against**:
- ❌ Vulnerabilities in the binary itself
- ❌ Compromised build systems (signs malicious code)
- ❌ Developer account compromise (if they have signing key access)

### Best Practices for Users

1. **Always** verify signatures before installing updates
2. **Check** key fingerprints match official announcements
3. **Report** suspicious signature failures to security team
4. **Keep** your Terraphim AI binary updated to get latest keys
5. **Never** disable signature verification in production

### Best Practices for Maintainers

1. **Generate** keys on air-gapped or secure systems
2. **Store** private keys in password managers (1Password, etc.)
3. **Rotate** keys periodically or immediately after compromise
4. **Document** all key rotations with proper announcements
5. **Audit** signing scripts and CI/CD pipelines regularly
6. **Use** hardware security modules (HSMs) for production signing

## Trust Model

### Developer Trust

Users trust that:
1. Terraphim AI developers have secured the signing private key
2. Build systems are not compromised
3. Signed binaries match source code (reproducible builds)

### Verification Trust

Users verify that:
1. Downloaded binaries have valid signatures
2. Signatures match the embedded public key
3. Public key is from official Terraphim AI sources

## Troubleshooting

### Signature Verification Fails

**Error**: "Signature verification failed"

**Possible causes**:
1. Archive was corrupted during download
2. Archive was modified after signing
3. Wrong public key (embedded key mismatch)
4. Expired key (if rotation implemented)

**Solutions**:
1. Re-download the archive
2. Verify your binary is from official sources
3. Check for security advisories about key rotation
4. Report the issue if problem persists

### Missing Signature

**Error**: "No signature found in archive"

**Possible causes**:
1. Downloaded unsigned development build
2. Archive from unofficial source
3. Incomplete download

**Solutions**:
1. Download official release from GitHub releases
2. Verify you're using the correct download URL
3. Check release notes for signature availability

## References

- [Ed25519 Paper](https://ed25519.cr.yp.to/) - Cryptography paper by Bernstein et al.
- [zipsign Documentation](https://github.com/Kijewski/zipsign) - Signing tool used
- [Issue #421](https://github.com/terraphim/terraphim-ai/issues/421) - Original implementation issue
- [SIGNATURE_VERIFICATION_PROGRESS.md](../SIGNATURE_VERIFICATION_PROGRESS.md) - Implementation progress

## Contact

For security-related questions about signature verification:
- **Security Issues**: security@terraphim.ai
- **General Questions**: GitHub Discussions
- **Report Verification Failures**: GitHub Issues with "security" label

---

**Last Updated**: 2025-01-12
**Document Version**: 1.0
