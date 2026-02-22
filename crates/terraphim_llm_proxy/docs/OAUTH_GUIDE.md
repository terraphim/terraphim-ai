# OAuth Authentication Guide

**Version:** 1.0
**Last Updated:** 2026-01-13

Complete guide to configuring and using OAuth authentication with the Terraphim LLM Proxy.

---

## Table of Contents

1. [Overview](#overview)
2. [Supported Providers](#supported-providers)
3. [Configuration](#configuration)
4. [Authentication Flows](#authentication-flows)
5. [Token Storage](#token-storage)
6. [CLI Integration](#cli-integration)
7. [Security Best Practices](#security-best-practices)
8. [Troubleshooting](#troubleshooting)

---

## Overview

OAuth authentication allows the proxy to authenticate with LLM providers using browser-based login flows instead of static API keys. This provides:

- **Enhanced security**: No static API keys in configuration files
- **User-friendly authentication**: Browser-based login with familiar OAuth flows
- **Automatic token refresh**: Tokens are refreshed transparently when expired
- **Multi-account support**: Authenticate multiple accounts per provider

### How It Works

```
1. Client starts OAuth flow
   ↓
2. Proxy generates authorization URL and state token
   ↓
3. User opens browser to authorization URL
   ↓
4. User completes authentication with provider
   ↓
5. Provider redirects to proxy callback endpoint
   ↓
6. Proxy exchanges authorization code for tokens
   ↓
7. Tokens stored securely and used for API requests
   ↓
8. Background task refreshes tokens when expired
```

---

## Supported Providers

| Provider | OAuth Flow | Client ID Required | Status |
|----------|------------|-------------------|--------|
| **Claude (Anthropic)** | PKCE with browser callback | Yes | Fully Supported |
| **Gemini (Google)** | OAuth2 with browser callback | Yes | Fully Supported |
| **GitHub Copilot** | Device code flow | No | Fully Supported |

---

## Configuration

### Basic Configuration

Add OAuth settings to your `config.toml`:

```toml
[oauth]
# Token storage backend: "file" (default) or "redis"
storage_backend = "file"

# Optional: Redis URL for distributed deployments
# redis_url = "redis://localhost:6379"

# Claude (Anthropic) OAuth
[oauth.claude]
enabled = true
callback_port = 54545
client_id = "your-client-id"
client_secret = "your-client-secret"

# Gemini (Google) OAuth
[oauth.gemini]
enabled = false
callback_port = 54546
client_id = "your-gemini-client-id"
client_secret = "your-gemini-client-secret"

# GitHub Copilot OAuth
[oauth.copilot]
enabled = false
callback_port = 54547
```

### Provider-Specific Setup

#### Claude (Anthropic) OAuth

1. **Create an OAuth Application**:
   - Visit [Anthropic Console](https://console.anthropic.com/)
   - Navigate to OAuth → Applications → Create Application
   - Set callback URL: `http://localhost:54545/oauth/claude/callback`
   - Note your Client ID and Client Secret

2. **Configure the Proxy**:
   ```toml
   [oauth.claude]
   enabled = true
   callback_port = 54545
   client_id = "your-anthropic-client-id"
   client_secret = "your-anthropic-client-secret"
   ```

3. **Enable OAuth for Requests**:
   - Configure your provider to use OAuth tokens instead of API keys
   - The proxy will automatically use stored OAuth tokens for requests

#### Gemini (Google) OAuth

1. **Create OAuth 2.0 Credentials**:
   - Visit [Google Cloud Console](https://console.cloud.google.com/)
   - Create a new OAuth 2.0 Client ID
   - Add `http://localhost:54546/oauth/gemini/callback` to authorized redirect URIs
   - Download credentials JSON

2. **Configure the Proxy**:
   ```toml
   [oauth.gemini]
   enabled = true
   callback_port = 54546
   client_id = "your-google-client-id"
   client_secret = "your-google-client-secret"
   ```

#### GitHub Copilot OAuth

GitHub Copilot uses device code flow (no callback URL needed):

```toml
[oauth.copilot]
enabled = true
callback_port = 54547
```

No client credentials required for Copilot device flow.

---

## Authentication Flows

### Browser-Based Flow (Claude, Gemini)

**Step 1: Start Authentication**

```bash
# The proxy generates an authorization URL
curl -X POST http://localhost:3456/oauth/claude/start
```

Response:
```json
{
  "authorization_url": "https://anthropic.com/oauth/authorize?...",
  "state": "random-state-token",
  "callback_port": 54545
}
```

**Step 2: Open Browser**

Open the `authorization_url` in your browser. Complete the sign-in process.

**Step 3: Callback Handling**

After successful authentication, the provider redirects to:
```
http://localhost:54545/oauth/claude/callback?code=...&state=...
```

The proxy automatically:
- Validates the state token
- Exchanges the authorization code for tokens
- Stores tokens securely
- Displays a success page (browser auto-closes after 3 seconds)

**Step 4: Poll for Status**

```bash
curl "http://localhost:3456/oauth/claude/status?state=random-state-token"
```

Response (while pending):
```json
{
  "status": "pending",
  "account_id": null
}
```

Response (when complete):
```json
{
  "status": "completed",
  "account_id": "user@example.com",
  "error": null
}
```

### Device Code Flow (GitHub Copilot)

**Step 1: Start Device Flow**

```bash
curl -X POST http://localhost:3456/oauth/copilot/start
```

Response:
```json
{
  "device_code": "ABCD-1234-EFGH",
  "user_code": "WDJM-MQHT",
  "verification_uri": "https://github.com/login/device",
  "expires_in": 900,
  "interval": 5
}
```

**Step 2: Visit GitHub**

1. Open `https://github.com/login/device` in your browser
2. Enter the user code: `WDJM-MQHT`
3. Authorize the device

**Step 3: Proxy Polls**

The proxy automatically polls GitHub every 5 seconds for completion. When you authorize:
- Tokens are exchanged and stored
- Your CLI/application is notified of completion

---

## Token Storage

### File-Based Storage (Default)

Tokens are stored in `~/.terraphim-llm-proxy/tokens/`:

```
~/.terraphim-llm-proxy/tokens/
├── claude/
│   ├── user@example.com.json
│   └── another@example.com.json
├── gemini/
│   └── user@gmail.com.json
└── copilot/
    └── github-username.json
```

Each token file contains:
```json
{
  "access_token": "eyJhbGciOi...",
  "refresh_token": "refresh_token_value",
  "token_type": "Bearer",
  "expires_at": "2026-01-14T03:00:00Z",
  "provider": "claude",
  "account_id": "user@example.com",
  "created_at": "2026-01-13T15:00:00Z",
  "last_refresh": "2026-01-13T18:00:00Z"
}
```

**File Permissions**: Token files are created with `0600` permissions (read/write by owner only).

### Redis Storage (Optional)

For distributed deployments, use Redis for token sharing:

```toml
[oauth]
storage_backend = "redis"
redis_url = "redis://localhost:6379"
```

**Redis Key Pattern**:
```
terraphim:oauth:{provider}:{account_id}
```

**Benefits**:
- Share tokens across multiple proxy instances
- Automatic token synchronization
- Built-in TTL for expired tokens

---

## CLI Integration

### Using OAuth with CLI Tools

The OAuth callback server integrates seamlessly with CLI tools:

```rust
use terraphim_llm_proxy::oauth::{ClaudeOAuthProvider, OAuthProvider};

let provider = ClaudeOAuthProvider::new(
    client_id,
    client_secret,
    redirect_url,
)?;

// Start OAuth flow
let (auth_url, flow_state) = provider.start_auth(54545).await?;

println!("Visit: {}", auth_url);

// Poll for completion
loop {
    tokio::time::sleep(Duration::from_secs(2)).await;
    match provider.exchange_code(&code, &flow_state).await {
        Ok(token) => break,
        Err(OAuthError::Pending) => continue,
        Err(e) => return Err(e),
    }
}
```

### Example: OAuth Flow in CLI

```bash
# 1. Start OAuth (prints URL to visit)
terraphim-proxy oauth login claude

# Output:
# Visit: https://anthropic.com/oauth/authorize?...
# Waiting for authentication...

# 2. User opens browser, completes auth

# 3. CLI detects completion
# ✓ Authenticated as user@example.com
# ✓ Tokens stored in ~/.terraphim-llm-proxy/tokens/

# 4. Make requests with OAuth token
terraphim-proxy chat "Hello, Claude!"
```

---

## Security Best Practices

### 1. Separate Management Secrets

Use a different secret key for Management API vs OAuth:

```toml
[management]
secret_key = "$MANAGEMENT_SECRET"  # Environment variable

[oauth.claude]
client_secret = "$CLAUDE_OAUTH_SECRET"  # Different environment variable
```

### 2. Environment Variables

Store secrets in environment variables, not config files:

```bash
export CLAUDE_CLIENT_ID="your-client-id"
export CLAUDE_CLIENT_SECRET="your-client-secret"
export MANAGEMENT_SECRET="your-management-secret"
```

Reference in config:
```toml
[oauth.claude]
client_id = "$CLAUDE_CLIENT_ID"
client_secret = "$CLAUDE_CLIENT_SECRET"
```

### 3. Token File Permissions

- Token files are created with `0600` permissions (owner read/write only)
- Verify permissions: `ls -la ~/.terraphim-llm-proxy/tokens/`
- On Linux/Mac: Only the file owner can read tokens
- On Windows: ACLs restrict access to the user

### 4. HTTPS in Production

For production deployments:
- Use HTTPS for all callback URLs
- Configure proper TLS certificates
- Use `allow_remote = false` in Management API settings

### 5. Token Rotation

OAuth tokens are automatically refreshed when expired:
- Access tokens typically expire after 1-2 hours
- Refresh tokens last longer (days to weeks)
- Background task refreshes tokens before expiration
- No manual intervention required

---

## Troubleshooting

### OAuth Callback Not Working

**Problem**: Browser shows "Connection refused" after authentication

**Solutions**:
1. Verify callback port is not in use: `netstat -an | grep 54545`
2. Check firewall allows localhost connections
3. Ensure OAuth is enabled in config: `[oauth.claude.enabled = true]`

### State Token Mismatch

**Problem**: Callback shows "State mismatch" error

**Cause**: State token expired or incorrect

**Solution**:
- State tokens expire after 10 minutes
- Restart the OAuth flow from the beginning
- Ensure you're using the latest authorization URL

### Token Not Found

**Problem**: Requests fail with "Token not found" error

**Solutions**:
1. Check token storage location: `ls -la ~/.terraphim-llm-proxy/tokens/`
2. Verify authentication completed successfully
3. Re-authenticate: `terraphim-proxy oauth login <provider>`

### Token Expired

**Problem**: Requests fail with "Token expired" error

**Cause**: Token expired and refresh failed

**Solution**:
1. Check network connectivity (refresh requires internet)
2. Verify refresh token is valid
3. Re-authenticate if refresh token is also expired

### Redis Connection Failed

**Problem**: Tokens not stored in Redis

**Solutions**:
1. Verify Redis is running: `redis-cli ping`
2. Check connection string: `redis://localhost:6379`
3. Test Redis connection: `redis-cli -h localhost -p 6379`

### Port Already in Use

**Problem**: "Address already in use" on callback port

**Solutions**:
1. Change callback port in config: `callback_port = 54550`
2. Find and kill process using the port: `lsof -i :54545`
3. Use different ports per provider to avoid conflicts

---

## Advanced Configuration

### Multiple OAuth Accounts

Authenticate multiple accounts per provider:

```bash
# Authenticate first account
terraphim-proxy oauth login claude
# → user1@example.com

# Authenticate second account
terraphim-proxy oauth login claude
# → user2@example.com
```

Tokens are stored separately:
```
~/.terraphim-llm-proxy/tokens/claude/
├── user1@example.com.json
└── user2@example.com.json
```

### Custom Token Storage Path

Override default token storage location:

```toml
[oauth]
storage_backend = "file"
token_path = "/custom/path/tokens"
```

### Token Cleanup

Remove expired or unused tokens:

```bash
# List all stored tokens
terraphim-proxy oauth list

# Revoke specific token
terraphim-proxy oauth revoke claude user@example.com

# Clear all tokens for provider
terraphim-proxy oauth clear claude
```

---

## API Reference

### OAuth Endpoints

#### Start OAuth Flow
```http
POST /oauth/{provider}/start
```

Response:
```json
{
  "authorization_url": "https://...",
  "state": "random-state-token",
  "callback_port": 54545,
  "expires_in": 600
}
```

#### OAuth Callback
```http
GET /oauth/{provider}/callback?code=...&state=...
```

Automatically handled by the proxy. Shows success/error page in browser.

#### Poll OAuth Status
```http
GET /oauth/{provider}/status?state={state_token}
```

Response:
```json
{
  "status": "pending|completed|error",
  "account_id": "user@example.com",
  "error": null
}
```

---

## Further Reading

- [Features Guide](./FEATURES.md) - Complete feature overview
- [Management API Guide](./MANAGEMENT_API_GUIDE.md) - Runtime configuration
- [Configuration Reference](./CONFIGURATION.md) - Full config options
- [Security Guide](./SECURITY.md) - Security best practices
