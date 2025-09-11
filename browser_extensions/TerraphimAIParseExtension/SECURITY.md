# Terraphim AI Parse Extension - Security Guide

## Overview

This extension has been enhanced with security features to prevent hardcoded API credentials and protect against accidental credential leakage.

## Secure Credential Management

### Setting Up Cloudflare API Credentials

1. **Open Extension Options**:
   - Right-click the extension icon and select "Options"
   - Or go to `chrome://extensions/`, find "Terraphim AI parse it", and click "Options"

2. **Configure Cloudflare API Settings**:
   - Enter your **Cloudflare Account ID** in the designated field
   - Enter your **Cloudflare API Token** with AI:Read permissions
   - Click "Test Cloudflare API" to verify your credentials
   - Click "Save Settings" to store them securely

3. **Credentials Storage**:
   - Credentials are stored securely using Chrome's `storage.sync` API
   - They are encrypted and synchronized across your Chrome profile
   - Never hardcode credentials in source code

### Finding Your Cloudflare Credentials

#### Account ID
1. Log in to the [Cloudflare dashboard](https://dash.cloudflare.com/)
2. Select any domain or go to the right sidebar
3. Your Account ID is displayed in the right sidebar under "API"

#### API Token
1. Go to [Cloudflare API Tokens](https://dash.cloudflare.com/profile/api-tokens)
2. Click "Create Token"
3. Use the "AI" template or create a custom token with:
   - **Permissions**: `Cloudflare AI:Read`
   - **Account Resources**: Include your account
4. Copy the generated token (you'll only see it once)

## Security Features

### API Key Detection Pre-commit Hook

The repository includes a comprehensive pre-commit hook that prevents API keys from being committed:

#### Installation
```bash
# Install the pre-commit hook
./scripts/install-pre-commit-hook.sh
```

#### What It Detects
- Cloudflare Account IDs and API tokens
- AWS access keys and secrets
- GitHub tokens
- Google API keys
- Generic API keys, secrets, and tokens
- Hardcoded credential patterns

#### Manual Testing
```bash
# Test the API key detection script
./scripts/check-api-keys.sh
```

### Error Handling

The extension will show clear error messages if:
- Cloudflare credentials are not configured
- API calls fail due to invalid credentials
- Network issues prevent API access

## Security Best Practices

1. **Never Hardcode Credentials**:
   - Always use the extension's settings UI
   - Store credentials in Chrome storage, not source code

2. **Use Minimal Permissions**:
   - Create API tokens with only the permissions needed
   - Use read-only permissions when possible

3. **Regular Rotation**:
   - Rotate your API tokens periodically
   - Revoke unused tokens from the Cloudflare dashboard

4. **Environment Separation**:
   - Use different API tokens for development and production
   - Never share tokens via email, chat, or version control

## Troubleshooting

### Common Issues

1. **"Cloudflare credentials not configured" Error**:
   - Go to extension options and set up your credentials
   - Verify both Account ID and API Token are entered

2. **API Call Failures**:
   - Test your credentials using the "Test Cloudflare API" button
   - Check that your API token has AI:Read permissions
   - Verify your account has AI features enabled

3. **Pre-commit Hook Failures**:
   - Review the detected patterns in the output
   - Remove any hardcoded credentials
   - Use environment variables or secure storage instead

### Getting Help

If you encounter security-related issues:
1. Check the browser console for detailed error messages
2. Verify your API token permissions in Cloudflare dashboard
3. Test your credentials using the extension options page
4. Review this security guide for best practices

## Development Notes

For developers working on this extension:

1. **Testing with Credentials**:
   - Never commit real API credentials
   - Use test/mock credentials for development
   - Document any test credential patterns in `.gitignore`

2. **Adding New API Integrations**:
   - Follow the same pattern: store credentials in Chrome storage
   - Add appropriate detection patterns to the pre-commit hook
   - Update this security guide with new credential setup instructions

3. **Pre-commit Hook Updates**:
   - Test any changes to the detection patterns
   - Ensure the hook catches various credential formats
   - Document new patterns in the script comments
