# GitHub Secrets Setup Guide

This guide explains how to set up the required GitHub secrets for publishing Terraphim crates.

## Required Secrets

### 1. ONEPASSWORD_SERVICE_ACCOUNT_TOKEN

This token allows GitHub Actions to authenticate with 1Password and retrieve the crates.io publishing token.

#### Setup Steps:

1. **Create a 1Password Service Account**
   - Go to 1Password Business > Integrations > Other > Get a service account token
   - Create a new service account with access to the "TerraphimPlatform" vault
   - Give it read access to the `crates.io.token` item
   - Copy the generated token

2. **Add to GitHub Repository Secrets**
   - Go to your repository on GitHub
   - Navigate to Settings > Secrets and variables > Actions
   - Click "New repository secret"
   - Name: `ONEPASSWORD_SERVICE_ACCOUNT_TOKEN`
   - Value: Paste the service account token from step 1
   - Click "Add secret"

#### Verification:

The service account should have access to:
- Vault: TerraphimPlatform
- Item: crates.io.token
- Field: token

### 2. (Optional) CARGO_REGISTRY_TOKEN

For manual publishing or local testing, you can also store the crates.io token directly:

1. **Get the token from 1Password**
   ```bash
   # First authenticate with 1Password
   op signin

   # Read the token
   op read "op://TerraphimPlatform/crates.io.token/token"
   ```

2. **Add to GitHub Secrets**
   - Name: `CARGO_REGISTRY_TOKEN`
   - Value: Paste the crates.io token

## Local Development Setup

### Option 1: Use the setup script

```bash
# Make sure 1Password CLI is installed and you're signed in
./scripts/setup-crates-token.sh --update-env
```

### Option 2: Manual setup

1. **Authenticate with 1Password**
   ```bash
   op signin <account-shorthand>
   ```

2. **Export the token**
   ```bash
   export CARGO_REGISTRY_TOKEN=$(op read "op://TerraphimPlatform/crates.io.token/token")
   ```

3. **Add to .env file (optional)**
   ```bash
   echo "CARGO_REGISTRY_TOKEN=$(op read \"op://TerraphimPlatform/crates.io.token/token\")" >> .env
   ```

## Security Considerations

### ✅ Good Practices
- Use service accounts with minimal required permissions
- Rotate tokens regularly
- Audit access logs in 1Password
- Use repository-specific secrets, not organization-wide when possible

### ❌ Avoid
- Committing tokens to the repository
- Sharing tokens in plain text
- Using personal tokens for CI/CD
- Giving broader permissions than necessary

## Testing the Setup

### Test Local Setup
```bash
# Test the token works
cargo publish --dry-run --package terraphim_types
```

### Test CI/CD Setup
1. Push a change to trigger the workflow
2. Go to Actions > Publish Rust Crates
3. Run the workflow manually with `dry_run: true`
4. Check that the 1Password authentication succeeds

## Troubleshooting

### Common Issues

1. **"could not read secret" error**
   - Check 1Password authentication: `op account list`
   - Verify the secret path: `op://TerraphimPlatform/crates.io.token/token`
   - Ensure service account has proper permissions

2. **"no token found" error in CI**
   - Verify GitHub secret is correctly named: `ONEPASSWORD_SERVICE_ACCOUNT_TOKEN`
   - Check that the secret is added to the correct repository/environment
   - Ensure the service account has access to the vault

3. **Permission denied when publishing**
   - Verify the crates.io token has publishing permissions
   - Check if the package name conflicts with existing published packages
   - Ensure the token hasn't expired

### Debug Commands

```bash
# Check 1Password status
op account list
op user get --me

# Test secret access
op read "op://TerraphimPlatform/crates.io.token/token"

# Test cargo token
cargo login --dry-run
```

## Workflow Usage

Once set up, you can use the publishing workflow in several ways:

### Manual Publishing (Dry Run)
```bash
gh workflow run "Publish Rust Crates" --field dry_run=true
```

### Manual Publishing (Live)
```bash
gh workflow run "Publish Rust Crates" --field dry_run=false
```

### Publish Specific Crate
```bash
gh workflow run "Publish Rust Crates" --field crate=terraphim_agent --field dry_run=false
```

### Tag-based Publishing
Create and push a tag to automatically trigger publishing:
```bash
git tag v1.0.0
git push origin v1.0.0
```