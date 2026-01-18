# Secure Key Storage Instructions

## CRITICAL: Store Private Key in 1Password

The private signing key MUST be stored securely before deleting it from the filesystem.

### 1Password Item Reference

**Item ID**: `jbhgblc7m2pluxe6ahqdfr5b6a`
**Vault**: `TerraphimPlatform`
**Title**: "Terraphim AI Release Signing Key (Ed25519)"

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

The signing scripts now support automatic 1Password integration. There are two ways to use the key:

**Method 1: Use 1Password Item ID (Recommended)**

```bash
# Set the 1Password item ID
export ZIPSIGN_OP_ITEM=jbhgblc7m2pluxe6ahqdfr5b6a

# Run the signing script - it will automatically retrieve the key from 1Password
./scripts/sign-release.sh release/0.2.5/

# Or for full release:
export ZIPSIGN_OP_ITEM=jbhgblc7m2pluxe6ahqdfr5b6a
./scripts/release.sh 0.2.5
```

**Method 2: Trigger via Environment Variable**

```bash
# Set to trigger 1Password retrieval
export ZIPSIGN_PRIVATE_KEY=op://

# The script will use the default item ID (jbhgblc7m2pluxe6ahqdfr5b6a)
./scripts/sign-release.sh release/0.2.5/
```

**Method 3: Manual Key Retrieval (Fallback)**

```bash
# For custom scripts or manual signing:
export ZIPSIGN_PRIVATE_KEY=$(op item get jbhgblc7m2pluxe6ahqdfr5b6a --reveal --fields note | \
  sed -n '/Key contents:/,/Public Key/p' | \
  tail -n +3 | \
  head -n -3)

# Use the key
zipsign sign tar archive.tar.gz "$ZIPSIGN_PRIVATE_KEY"

# Clean up the key from environment
unset ZIPSIGN_PRIVATE_KEY
```

### GitHub Actions Integration

For CI/CD signing, you can use either method:

**Option 1: 1Password GitHub Action (Recommended)**

Use the official 1Password GitHub Action to inject the signing key during CI/CD:

```yaml
# In .github/workflows/release-sign.yml
- name: Load signing key from 1Password
  uses: 1Password/load-secrets-action@v2
  with:
    export-env: ZIPSIGN_OP_ITEM
  env:
    ZIPSIGN_OP_ITEM: "op://TerraphimPlatform/jbhgblc7m2pluxe6ahqdfr5b6a/note"

- name: Sign release archives
  run: ./scripts/sign-release.sh release/${{ github.ref_name }}/
  env:
    ZIPSIGN_OP_ITEM: jbhgblc7m2pluxe6ahqdfr5b6a
```

**Option 2: GitHub Secret (Alternative)**

Store the key directly in GitHub Secrets:

```bash
# Retrieve from 1Password
op item get jbhgblc7m2pluxe6ahqdfr5b6a --reveal --fields note | \
  sed -n '/Key contents:/,/Public Key/p' | \
  tail -n +3 | \
  head -n -3 | \
  pbcopy

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
