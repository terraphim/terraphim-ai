# Tauri Signing Keys - 1Password Integration

## Overview

Tauri signing keys have been securely migrated to 1Password for enhanced security and team access management. The keys are stored in the TerraphimPlatform vault and can be accessed via the 1Password CLI.

## Key Storage Details

- **Vault**: TerraphimPlatform
- **Vault ID**: `6fsizn2h5rrs5mp3e4phudjab4`
- **Item ID**: `3k2d5ycxeagdazexivgomi2gpy`
- **Item Title**: TauriSigning

## Stored Credentials

The following credentials are stored in the 1Password item:

1. **TAURI_PRIVATE_KEY**: The private signing key for Tauri updates (concealed)
2. **TAURI_PUBLIC_KEY**: The public key for verification (visible)
3. **TAURI_KEY_PASSWORD**: Password for the private key (empty/concealed)

## Usage

### Prerequisites

Ensure you have the 1Password CLI installed and authenticated:

```bash
# Check installation
op --version

# Authenticate if needed
op signin
```

### Method 1: Direct Environment Variable Export

Export the keys as environment variables for use in build scripts:

```bash
export TAURI_PRIVATE_KEY=$(op read "op://6fsizn2h5rrs5mp3e4phudjab4/3k2d5ycxeagdazexivgomi2gpy/TAURI_PRIVATE_KEY")
export TAURI_PUBLIC_KEY=$(op read "op://6fsizn2h5rrs5mp3e4phudjab4/3k2d5ycxeagdazexivgomi2gpy/TAURI_PUBLIC_KEY")
export TAURI_KEY_PASSWORD=$(op read "op://6fsizn2h5rrs5mp3e4phudjab4/3k2d5ycxeagdazexivgomi2gpy/TAURI_KEY_PASSWORD")
```

### Method 2: Using op run with Environment File

The project includes a `.env.tauri-release` file with 1Password references:

```bash
# Run build with automatic secret injection
op run --env-file=.env.tauri-release -- npm run build

# Or for Rust builds
op run --env-file=.env.tauri-release -- cargo build --release
```

### Method 3: GitHub Actions Integration

For CI/CD pipelines, configure a 1Password service account:

1. Create a service account in 1Password with read access to the TerraphimPlatform vault
2. Add the service account token as a GitHub secret: `OP_SERVICE_ACCOUNT_TOKEN`
3. Use the 1Password GitHub Action in your workflow:

```yaml
- name: Load secrets from 1Password
  uses: 1password/load-secrets-action@v1
  with:
    export-env: true
  env:
    OP_SERVICE_ACCOUNT_TOKEN: ${{ secrets.OP_SERVICE_ACCOUNT_TOKEN }}
    TAURI_PRIVATE_KEY: op://6fsizn2h5rrs5mp3e4phudjab4/3k2d5ycxeagdazexivgomi2gpy/TAURI_PRIVATE_KEY
    TAURI_KEY_PASSWORD: op://6fsizn2h5rrs5mp3e4phudjab4/3k2d5ycxeagdazexivgomi2gpy/TAURI_KEY_PASSWORD
```

## File References

### .env.tauri-release

This file contains 1Password references for the Tauri signing keys:

```bash
TAURI_PRIVATE_KEY="op://6fsizn2h5rrs5mp3e4phudjab4/3k2d5ycxeagdazexivgomi2gpy/TAURI_PRIVATE_KEY"
TAURI_KEY_PASSWORD="op://6fsizn2h5rrs5mp3e4phudjab4/3k2d5ycxeagdazexivgomi2gpy/TAURI_KEY_PASSWORD"
```

### docs/artifacts (historical)

Historically, release tracking documents lived under `.reports/`. Those have been migrated to `docs/artifacts/` for publication and long-term reference.

The actual signing key references should live in `.env.tauri-release` (1Password reference URIs only) and in this document.

## Security Benefits

1. **No Plain Text Keys**: Sensitive keys are never stored in plain text in the repository
2. **Access Control**: Team members need 1Password vault access to retrieve keys
3. **Audit Trail**: All key access is logged in 1Password
4. **Rotation Support**: Keys can be updated in 1Password without changing code
5. **Service Account Integration**: CI/CD can access keys securely without exposing them

## Troubleshooting

### Multiple Vaults with Same Name

If you encounter an error about multiple vaults matching "TerraphimPlatform", use the vault ID directly:
- Vault ID: `6fsizn2h5rrs5mp3e4phudjab4`

### Permission Errors

Ensure your 1Password account has access to the TerraphimPlatform vault. Contact your 1Password administrator if you need access.

### CLI Authentication Issues

If `op` commands fail, re-authenticate:

```bash
op signout --all
op signin
```

## Migration Notes

- **Migration Date**: November 7, 2025
- **Previous Storage**: Keys were previously documented under `.reports/`
- **Vault Created**: A new TerraphimPlatform vault was created (ID: deahm4nag52derwyq2frgy3pda)
- **Item Updated**: Existing item `3k2d5ycxeagdazexivgomi2gpy` was updated with Tauri keys

## Related Documentation

- [1Password CLI Documentation](https://developer.1password.com/docs/cli)
- [Tauri Code Signing](https://tauri.app/v1/guides/distribution/sign)
- [1Password Service Accounts](https://developer.1password.com/docs/service-accounts)
