# GitHub Secret Setup for Signed Tauri Releases

## Required Action: Set TAURI_PRIVATE_KEY Secret

To enable signed Tauri releases and auto-updates, you need to add the private key as a GitHub secret.

### Steps:

1. **Extract Private Key from `.reports/tauri_keys.txt`**:
   - Open the file and copy the SECRET KEY (starts with `dW50cnVzdGVkIGNvbW1lbnQ...`)
   - This is the long base64 string after "Your secret key was generated successfully"

2. **Add to GitHub Secrets**:
   ```bash
   gh secret set TAURI_PRIVATE_KEY --repo terraphim/terraphim-ai
   # Paste the private key when prompted
   ```

   Or manually:
   - Go to https://github.com/terraphim/terraphim-ai/settings/secrets/actions
   - Click "New repository secret"
   - Name: `TAURI_PRIVATE_KEY`
   - Value: Paste the private key from tauri_keys.txt
   - Click "Add secret"

3. **Verify Secret is Set**:
   ```bash
   gh secret list --repo terraphim/terraphim-ai | grep TAURI
   ```

### Security Notes

- ⚠️ **NEVER commit the private key to git**
- ⚠️ **Keep `.reports/tauri_keys.txt` secure** - it's gitignored
- ✅ The public key is already configured in `tauri.conf.json`
- ✅ GitHub workflows will use the secret automatically

### What Happens After Setting the Secret

1. Push a new tag (e.g., `v1.0.1`)
2. GitHub Actions will trigger the release workflow
3. Tauri will build and sign the installers
4. Users' apps will verify updates using the public key
5. Auto-update will work securely

### Workflows That Use This Secret

- `.github/workflows/publish-tauri.yml`
- `.github/workflows/release-comprehensive.yml`
- `.github/workflows/tauri-build.yml`

---

**Current Status**: Secret needs to be set before publishing v1.0.0 release.
