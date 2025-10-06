# 1Password Setup for Terraphim AI Auto-Update

This document provides step-by-step instructions for setting up 1Password integration with Terraphim AI's auto-update system.

## Overview

Terraphim AI uses 1Password to securely manage:
- Tauri signing keys for desktop application updates
- GitHub release tokens for CI/CD
- All deployment secrets without exposing them in code

## Prerequisites

1. **1Password CLI installed**:
   ```bash
   # macOS
   brew install --cask 1password-cli

   # Linux
   curl -sS https://downloads.1password.com/linux/keys/1password.asc | \
     sudo gpg --dearmor --output /usr/share/keyrings/1password-archive-keyring.gpg
   ```

2. **1Password account with admin access**

3. **GitHub repository with admin permissions**

## Step 1: Run the Setup Script

The easiest way to set up 1Password integration is to use the automated setup script:

```bash
./scripts/setup-1password-secrets.sh
```

This script will:
- Create the "Terraphim-Deployment" vault
- Generate Tauri signing keys
- Store secrets in 1Password
- Update configuration files
- Provide next steps

## Step 2: Manual Setup (Alternative)

If you prefer manual setup or need to troubleshoot:

### 2.1 Create 1Password Vault

```bash
# Sign in to 1Password CLI
op signin

# Create dedicated vault for deployment secrets
op vault create "Terraphim-Deployment"
```

### 2.2 Generate Tauri Signing Keys

```bash
cd desktop
npm run tauri signer generate -- -w tauri-private.key

# Extract public key
npm run tauri signer show-public-key < tauri-private.key
```

### 2.3 Store Secrets in 1Password

```bash
# Store Tauri signing keys
op item create \
  --category "API Credential" \
  --title "Tauri Update Signing" \
  --vault "Terraphim-Deployment" \
  --field "label=TAURI_PRIVATE_KEY,type=concealed,value=$(cat tauri-private.key)" \
  --field "label=TAURI_KEY_PASSWORD,type=concealed,value=$(openssl rand -base64 32)" \
  --field "label=TAURI_PUBLIC_KEY,type=text,value=$(npm run tauri signer show-public-key < tauri-private.key)"

# Store GitHub token
op item create \
  --category "API Credential" \
  --title "GitHub Release Token" \
  --vault "Terraphim-Deployment" \
  --field "label=GITHUB_TOKEN,type=concealed,value=YOUR_GITHUB_TOKEN"

# Clean up temporary key file
rm tauri-private.key
```

## Step 3: Create Service Account for CI/CD

### 3.1 Web Interface Setup

1. Go to [1Password web interface](https://start.1password.com/)
2. Navigate to **Developer Tools > Service Accounts**
3. Click **"Create Service Account"**
4. Name: **"Terraphim CI/CD"**
5. Description: **"Service account for Terraphim AI automated deployments"**

### 3.2 Grant Vault Access

1. In the service account settings, add vault access:
   - **Vault**: Terraphim-Deployment
   - **Permissions**: Read

### 3.3 Copy Service Account Token

1. Copy the service account token (starts with 'ops_...')
2. Add to GitHub repository secrets:
   - Go to repository Settings > Secrets and variables > Actions
   - Click **"New repository secret"**
   - **Name**: `OP_SERVICE_ACCOUNT_TOKEN`
   - **Value**: [paste the copied token]

## Step 4: Test the Setup

### 4.1 Local Testing

```bash
# Test 1Password CLI access
op whoami

# Test vault access
op vault get "Terraphim-Deployment"

# Test secret retrieval
op item get "Tauri Update Signing" --vault "Terraphim-Deployment" --field "TAURI_PUBLIC_KEY"

# Test environment injection
op run --env-file=.env.tauri-release -- echo "Secrets loaded successfully"
```

### 4.2 Build Testing

```bash
# Test local build with signing
./scripts/build-with-signing.sh

# Test release script (dry run)
./scripts/release-all.sh 0.2.1 --dry-run
```

### 4.3 CI/CD Testing

Test the GitHub Actions workflow by creating a test release:

```bash
# Create test tag
git tag -a "test-v0.2.0-alpha" -m "Test auto-update setup"
git push origin "test-v0.2.0-alpha"
```

Monitor the GitHub Actions workflow to ensure:
- 1Password CLI authenticates successfully
- Secrets are injected properly
- Tauri builds and signs correctly
- Release artifacts are created

## Step 5: Verify Auto-Update Functionality

### 5.1 Desktop Application

1. Build and install the desktop app locally
2. Create a new release
3. Launch the app and check for updates via the menu
4. Verify update process works end-to-end

### 5.2 CLI Binaries

```bash
# Test CLI update check
./target/release/terraphim_server --update-check

# Test CLI update installation
./target/release/terraphim_server --update
```

## Security Best Practices

### Least Privilege Access
- Service accounts have read-only access to specific vaults
- No personal credentials in CI/CD environments
- Regular key rotation schedule

### Audit and Monitoring
- Monitor 1Password access logs
- Review service account usage regularly
- Set up alerts for unusual access patterns

### Key Rotation

Rotate signing keys every 6 months or if compromised:

```bash
# Generate new keys
./scripts/setup-1password-secrets.sh

# Update GitHub secrets if needed
# Test with a pre-release build
# Deploy new keys with next release
```

## Troubleshooting

### Common Issues

1. **"Not authenticated with 1Password"**
   ```bash
   op signin
   ```

2. **"Cannot access vault 'Terraphim-Deployment'"**
   ```bash
   # Check vault exists
   op vault list

   # Verify permissions
   op vault get "Terraphim-Deployment"
   ```

3. **"Failed to inject secrets"**
   ```bash
   # Check template file exists
   ls desktop/src-tauri/tauri.conf.json.template

   # Verify secret references
   op item get "Tauri Update Signing" --vault "Terraphim-Deployment"
   ```

4. **"GitHub Actions failing"**
   - Verify `OP_SERVICE_ACCOUNT_TOKEN` is set in repository secrets
   - Check service account has proper vault access
   - Review GitHub Actions logs for specific errors

### Debug Commands

```bash
# Check 1Password CLI version
op --version

# List all vaults
op vault list

# List items in deployment vault
op item list --vault "Terraphim-Deployment"

# Test service account locally
export OP_SERVICE_ACCOUNT_TOKEN="ops_..."
op item get "Tauri Update Signing" --vault "Terraphim-Deployment"
```

## Additional Resources

- [1Password CLI Documentation](https://developer.1password.com/docs/cli)
- [1Password Service Accounts](https://developer.1password.com/docs/service-accounts)
- [Tauri Updater Guide](https://tauri.app/v1/guides/distribution/updater)
- [GitHub Actions with 1Password](https://github.com/1password/install-cli-action)

## Support

If you encounter issues with the 1Password setup:

1. Check the troubleshooting section above
2. Review the GitHub Actions logs
3. Verify all prerequisites are met
4. Create an issue in the repository with:
   - Steps to reproduce
   - Error messages (without sensitive data)
   - Environment details (OS, 1Password CLI version, etc.)
