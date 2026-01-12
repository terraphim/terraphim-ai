# Secure Key Storage Instructions

## CRITICAL: Store Private Key in 1Password

The private signing key MUST be stored securely before deleting it from the filesystem.

### 1Password Storage Instructions

**Vault**: `TerraphimPlatform`

**Steps to store the private key**:

```bash
# Install 1Password CLI if not already installed
# See: https://developer.1password.com/docs/cli/get-started/

# Create a new secure note in TerraphimPlatform vault
op item create \
  --vault="TerraphimPlatform" \
  --category="Secure Note" \
  --title="Terraphim AI Release Signing Key (Ed25519)" \
  --tags="signing-key,ed25519,release" \
  note="Ed25519 private key for signing Terraphim AI release binaries.

Generated: 2025-01-12
Key Type: Ed25519
Purpose: Sign .tar.gz release archives

Key contents:
$(cat /home/alex/projects/terraphim/terraphim-ai/keys/private.key)

Security Notes:
- This key signs all official Terraphim AI releases
- If compromised, rotate immediately and announce security advisory
- Never share this key outside trusted maintainers
- Store only in 1Password or air-gapped storage

Fingerprint (for verification):
1c78db3c8e1afa3af4fcbaf32ccfa30988c82f9e7d383dfb127ae202732b631a"
```

### Alternative: Manual Storage

If the CLI command above doesn't work, manually create the item:

1. **Open 1Password app** and unlock
2. **Select vault**: `TerraphimPlatform`
3. **Create new item**: Secure Note
4. **Title**: "Terraphim AI Release Signing Key (Ed25519)"
5. **Contents**: Copy entire contents of `/home/alex/projects/terraphim/terraphim-ai/keys/private.key`
6. **Tags**: `signing-key`, `ed25519`, `release`
7. **Save** and verify the note is accessible

### After Secure Storage

Once the key is safely stored in 1Password:

```bash
# Verify the key is retrievable
op item list --vault="TerraphimPlatform" --tags="signing-key" | grep "Terraphim AI Release Signing"

# Then delete from filesystem
shred -vfz -n 3 /home/alex/projects/terraphim/terraphym-ai/keys/private.key
# Or if shred is not available:
rm -P /home/alex/projects/terraphim/terraphym-ai/keys/private.key

# Verify the file is gone
ls -la /home/alex/projects/terraphim/terraphim-ai/keys/
```

### Using the Key for Signing

When signing releases, retrieve the key from 1Password:

```bash
# For release.sh script:
export ZIPSIGN_PRIVATE_KEY=$(op item get --vault="TerraphimPlatform" --fields="note" "Terraphim AI Release Signing Key" | grep -A 1000 "Key contents:" | tail -n +3)

# Or use the op run command for automatic injection
op run --env-file=/path/to/.env -- ./scripts/release.sh
```

### GitHub Actions Integration

For CI/CD signing, add the private key to GitHub Secrets:

```bash
# Retrieve from 1Password
op item get --vault="TerraphimPlatform" --fields="note" "Terraphim AI Release Signing Key" | grep -A 1000 "Key contents:" | tail -n +3 | pbcopy

# Then in GitHub:
# 1. Go to repository Settings
# 2. Secrets and variables -> Actions
# 3. New repository secret
# 4. Name: ZIPSIGN_PRIVATE_KEY
# 5. Value: [paste from clipboard]
# 6. Add secret
```

### Security Checklist

- [ ] Private key stored in 1Password vault "TerraphimPlatform"
- [ ] Private key deleted from filesystem
- [ ] Public key embedded in `crates/terraphim_update/src/signature.rs`
- [ ] Key fingerprint documented in `docs/updates/KEYS.md`
- [ ] `keys/` directory added to `.gitignore`
- [ ] Only trusted maintainers have 1Password access
- [ ] GitHub secret `ZIPSIGN_PRIVATE_KEY` configured (for CI/CD)
- [ ] Backup procedure documented (e.g., export 1Password item to secure location)

### Emergency Key Rotation

If the private key is compromised:

1. **Immediately**: Revoke key in next release (embed new public key)
2. **Generate**: Create new key pair using secure environment
3. **Store**: Store new private key in 1Password
4. **Update**: Embed new public key in codebase
5. **Sign**: Sign new release with new key
6. **Announce**: Publish security advisory with old and new key fingerprints
7. **Audit**: Review all releases signed with compromised key

### Verification

To verify the key is correctly stored:

```bash
# Test signing with retrieved key
op item get --vault="TerraphimPlatform" --fields="note" "Terraphim AI Release Signing Key" > /tmp/test-private.key
zipsign sign tar /tmp/test.tar.gz /tmp/test-private.key
shred -vfz -n 3 /tmp/test-private.key
```

---

**Last Updated**: 2025-01-12
**Key Version**: v1.0
**Vault**: TerraphimPlatform
