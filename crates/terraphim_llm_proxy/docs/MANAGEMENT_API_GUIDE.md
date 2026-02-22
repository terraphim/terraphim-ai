# Management API Guide

**Version:** 1.0
**Last Updated:** 2026-01-13

Complete guide to using the Management API for runtime control of the Terraphim LLM Proxy.

---

## Table of Contents

1. [Overview](#overview)
2. [Authentication](#authentication)
3. [Endpoints](#endpoints)
4. [Configuration Management](#configuration-management)
5. [API Key Management](#api-key-management)
6. [Provider Management](#provider-management)
7. [Logging & Monitoring](#logging--monitoring)
8. [Health & Metrics](#health--metrics)
9. [Best Practices](#best-practices)
10. [Troubleshooting](#troubleshooting)

---

## Overview

The Management API provides runtime control over the proxy without requiring restarts. All changes take effect immediately for new requests.

### Key Features

- **Runtime configuration**: Update settings without restart
- **API key management**: Add, list, and revoke keys instantly
- **Provider management**: Add/remove providers dynamically
- **Logging control**: Change log levels at runtime
- **Health monitoring**: Check provider health and performance
- **Usage metrics**: Track request statistics and costs

### Base URL

All Management API endpoints are prefixed with `/v0/management/`:

```
http://localhost:3456/v0/management/{endpoint}
```

---

## Authentication

All Management API endpoints require authentication using the `X-Management-Key` header.

### Setup

Configure the management secret in `config.toml`:

```toml
[management]
enabled = true
secret_key = "your-secret-key-here"
allow_remote = false  # Only allow localhost (recommended)
```

### Using Environment Variables

For better security, use environment variables:

```bash
export MANAGEMENT_SECRET="your-secret-key-here"
```

Reference in config:
```toml
[management]
secret_key = "$MANAGEMENT_SECRET"
```

### Making Authenticated Requests

```bash
curl -H "X-Management-Key: your-secret-key-here" \
  http://localhost:3456/v0/management/config
```

### Security Notes

- **Use a strong secret**: Minimum 32 characters, randomly generated
- **Separate from API keys**: Management secret is different from proxy API keys
- **Limit remote access**: Set `allow_remote = false` unless needed
- **Use HTTPS**: In production, use HTTPS with TLS certificates
- **Rotate regularly**: Change management secrets periodically

---

## Endpoints

### Endpoint Summary

| Category | Endpoints |
|----------|-----------|
| **Configuration** | `GET /config`, `PUT /config`, `POST /config/reload` |
| **API Keys** | `GET /api-keys`, `POST /api-keys`, `DELETE /api-keys:{key_id}` |
| **Providers** | `GET /providers`, `POST /providers`, `DELETE /providers:{name}` |
| **OAuth Tokens** | `GET /oauth/tokens`, `DELETE /oauth/tokens:{provider}:{account_id}` |
| **Logging** | `GET /logs`, `GET /logs/level`, `PUT /logs/level` |
| **Monitoring** | `GET /health`, `GET /metrics` |

---

## Configuration Management

### Get Current Configuration

Retrieve the current proxy configuration with secrets redacted.

```http
GET /v0/management/config
```

**Request**:
```bash
curl -H "X-Management-Key: secret" \
  http://localhost:3456/v0/management/config
```

**Response**:
```json
{
  "proxy": {
    "host": "0.0.0.0",
    "port": 3456,
    "api_key": "[REDACTED]",
    "timeout_ms": 60000
  },
  "router": {
    "default": "openrouter,anthropic/claude-opus-4.5",
    "model_mappings": [...],
    "model_exclusions": [...],
    "strategy": "FillFirst"
  },
  "providers": [...],
  "oauth": {
    "claude": {
      "enabled": true,
      "client_id": "[REDACTED]",
      "client_secret": "[REDACTED]"
    }
  },
  "management": {
    "enabled": true,
    "secret_key": "[REDACTED]",
    "allow_remote": false
  }
}
```

### Update Configuration

Update configuration atomically. Changes take effect immediately.

```http
PUT /v0/management/config
```

**Request**:
```bash
curl -X PUT \
  -H "X-Management-Key: secret" \
  -H "Content-Type: application/json" \
  -d '{
    "router": {
      "strategy": "RoundRobin"
    }
  }' \
  http://localhost:3456/v0/management/config
```

**Response**:
```json
{
  "status": "success",
  "message": "Configuration updated successfully",
  "timestamp": "2026-01-13T15:30:00Z"
}
```

**Important Notes**:
- Partial updates are supported (only send fields you want to change)
- In-flight requests complete with original configuration
- New requests use updated configuration
- Changes are not persisted to disk (use `reload` to revert)

### Reload Configuration from Disk

Reload configuration from the config file, discarding any runtime changes.

```http
POST /v0/management/config/reload
```

**Request**:
```bash
curl -X POST \
  -H "X-Management-Key: secret" \
  http://localhost:3456/v0/management/config/reload
```

**Response**:
```json
{
  "status": "success",
  "message": "Configuration reloaded from disk",
  "config_file": "/path/to/config.toml",
  "timestamp": "2026-01-13T15:30:00Z"
}
```

**Use Cases**:
- Revert runtime changes
- Apply manual config file edits
- Recover from invalid configuration changes

---

## API Key Management

### List All API Keys

Get all configured API keys (prefixes only, for security).

```http
GET /v0/management/api-keys
```

**Request**:
```bash
curl -H "X-Management-Key: secret" \
  http://localhost:3456/v0/management/api-keys
```

**Response**:
```json
{
  "keys": [
    {
      "id": "key-001",
      "key_prefix": "sk-proj_abc1...",
      "created_at": "2026-01-13T10:00:00Z",
      "last_used": "2026-01-13T15:25:00Z",
      "request_count": 1234
    },
    {
      "id": "key-002",
      "key_prefix": "sk-proj_xyz9...",
      "created_at": "2026-01-13T12:00:00Z",
      "last_used": null,
      "request_count": 0
    }
  ]
}
```

### Create New API Key

Generate a new random API key.

```http
POST /v0/management/api-keys
```

**Request**:
```bash
curl -X POST \
  -H "X-Management-Key: secret" \
  -H "Content-Type: application/json" \
  -d '{
    "description": "Production API key",
    "expires_at": "2026-12-31T23:59:59Z"
  }' \
  http://localhost:3456/v0/management/api-keys
```

**Response**:
```json
{
  "id": "key-003",
  "key": "sk-proj_MrN9jK8vP3qR2xT5wY7zA1bC4dE6fG8h",
  "key_prefix": "sk-proj_MrN9...",
  "description": "Production API key",
  "created_at": "2026-01-13T15:30:00Z",
  "expires_at": "2026-12-31T23:59:59Z"
}
```

**Important**: Copy the key immediately. It won't be shown again.

### Revoke API Key

Immediately revoke an API key. Revoked keys stop working instantly.

```http
DELETE /v0/management/api-keys/{key_id}
```

**Request**:
```bash
curl -X DELETE \
  -H "X-Management-Key: secret" \
  http://localhost:3456/v0/management/api-keys/key-001
```

**Response**:
```json
{
  "status": "success",
  "message": "API key revoked successfully",
  "key_id": "key-001",
  "revoked_at": "2026-01-13T15:30:00Z"
}
```

**Key Deletion Behavior**:
- **Immediate**: Key stops working instantly
- **No grace period**: In-flight requests with revoked key will fail
- **Permanent**: Cannot undo key revocation
- **Audit**: Key deletion is logged

---

## Provider Management

### List All Providers

Get all configured LLM providers with health status.

```http
GET /v0/management/providers
```

**Request**:
```bash
curl -H "X-Management-Key: secret" \
  http://localhost:3456/v0/management/providers
```

**Response**:
```json
{
  "providers": [
    {
      "name": "openrouter",
      "api_base_url": "https://openrouter.ai/api/v1/chat/completions",
      "models": ["anthropic/claude-opus-4.5", "google/gemini-pro"],
      "health_status": "healthy",
      "last_check": "2026-01-13T15:29:00Z",
      "uptime_percent": 99.95,
      "avg_latency_ms": 245
    },
    {
      "name": "deepseek",
      "api_base_url": "https://api.deepseek.com/v1/chat/completions",
      "models": ["deepseek-chat"],
      "health_status": "degraded",
      "last_check": "2026-01-13T15:29:00Z",
      "uptime_percent": 98.50,
      "avg_latency_ms": 512
    }
  ]
}
```

### Add New Provider

Add a new LLM provider at runtime.

```http
POST /v0/management/providers
```

**Request**:
```bash
curl -X POST \
  -H "X-Management-Key: secret" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "anthropic",
    "api_base_url": "https://api.anthropic.com/v1/messages",
    "api_key": "sk-ant-...",
    "models": ["claude-opus-4.5", "claude-sonnet-4.5"],
    "transformers": ["anthropic"]
  }' \
  http://localhost:3456/v0/management/providers
```

**Response**:
```json
{
  "status": "success",
  "message": "Provider added successfully",
  "provider": {
    "name": "anthropic",
    "api_base_url": "https://api.anthropic.com/v1/messages",
    "models": ["claude-opus-4.5", "claude-sonnet-4.5"],
    "added_at": "2026-01-13T15:30:00Z"
  }
}
```

### Remove Provider

Remove a provider from the configuration.

```http
DELETE /v0/management/providers/{provider_name}
```

**Request**:
```bash
curl -X DELETE \
  -H "X-Management-Key: secret" \
  http://localhost:3456/v0/management/providers/deepseek
```

**Response**:
```json
{
  "status": "success",
  "message": "Provider removed successfully",
  "provider_name": "deepseek",
  "removed_at": "2026-01-13T15:30:00Z"
}
```

**Warning**: Removing a provider that's actively used will cause requests to fail. Update routing configuration first.

---

## Logging & Monitoring

### Get Recent Logs

Fetch recent log entries with optional filtering.

```http
GET /v0/management/logs?lines=100&level=info
```

**Parameters**:
- `lines`: Number of log lines to return (default: 100, max: 1000)
- `level`: Filter by log level (`trace`, `debug`, `info`, `warn`, `error`)

**Request**:
```bash
curl -H "X-Management-Key: secret" \
  "http://localhost:3456/v0/management/logs?lines=50&level=warn"
```

**Response**:
```json
{
  "logs": [
    {
      "timestamp": "2026-01-13T15:29:45Z",
      "level": "WARN",
      "target": "terraphim_llm_proxy::router",
      "message": "Provider 'deepseek' is degraded"
    },
    {
      "timestamp": "2026-01-13T15:29:30Z",
      "level": "INFO",
      "target": "terraphim_llm_proxy::server",
      "message": "Request completed in 245ms"
    }
  ],
  "count": 50
}
```

### Get Current Log Level

Get the current logging level.

```http
GET /v0/management/logs/level
```

**Request**:
```bash
curl -H "X-Management-Key: secret" \
  http://localhost:3456/v0/management/logs/level
```

**Response**:
```json
{
  "level": "info",
  "timestamp": "2026-01-13T15:30:00Z"
}
```

### Change Log Level

Change the logging level at runtime without restart.

```http
PUT /v0/management/logs/level
```

**Request**:
```bash
curl -X PUT \
  -H "X-Management-Key: secret" \
  -H "Content-Type: application/json" \
  -d '{"level": "debug"}' \
  http://localhost:3456/v0/management/logs/level
```

**Response**:
```json
{
  "status": "success",
  "message": "Log level changed",
  "old_level": "info",
  "new_level": "debug",
  "timestamp": "2026-01-13T15:30:00Z"
}
```

**Log Levels**:
- `trace`: Most verbose logging
- `debug`: Detailed debugging information
- `info`: General informational messages (default)
- `warn`: Warning messages
- `error`: Error messages only

---

## Health & Metrics

### Get Detailed Health Status

Get comprehensive health information for all providers.

```http
GET /v0/management/health
```

**Request**:
```bash
curl -H "X-Management-Key: secret" \
  http://localhost:3456/v0/management/health
```

**Response**:
```json
{
  "status": "healthy",
  "timestamp": "2026-01-13T15:30:00Z",
  "uptime_seconds": 86400,
  "providers": [
    {
      "name": "openrouter",
      "status": "healthy",
      "uptime_percent": 99.95,
      "avg_latency_ms": 245,
      "request_count": 50000,
      "error_count": 25,
      "last_error": null
    },
    {
      "name": "deepseek",
      "status": "degraded",
      "uptime_percent": 98.50,
      "avg_latency_ms": 512,
      "request_count": 25000,
      "error_count": 375,
      "last_error": "Connection timeout"
    }
  ],
  "system": {
    "memory_usage_mb": 256.5,
    "cpu_usage_percent": 15.2,
    "disk_usage_percent": 45.8
  }
}
```

### Get Usage Metrics

Get detailed usage statistics and costs.

```http
GET /v0/management/metrics
```

**Request**:
```bash
curl -H "X-Management-Key: secret" \
  http://localhost:3456/v0/management/metrics
```

**Response**:
```json
{
  "period": {
    "start": "2026-01-13T00:00:00Z",
    "end": "2026-01-13T15:30:00Z"
  },
  "requests": {
    "total": 75000,
    "successful": 74600,
    "failed": 400,
    "avg_latency_ms": 285
  },
  "tokens": {
    "input_tokens": 125000000,
    "output_tokens": 45000000,
    "total_tokens": 170000000
  },
  "costs": {
    "total_usd": 1250.50,
    "by_provider": [
      {
        "provider": "openrouter",
        "cost_usd": 950.25
      },
      {
        "provider": "deepseek",
        "cost_usd": 300.25
      }
    ]
  },
  "models": [
    {
      "model": "anthropic/claude-opus-4.5",
      "request_count": 45000,
      "input_tokens": 85000000,
      "output_tokens": 32000000,
      "cost_usd": 850.00
    }
  ]
}
```

---

## OAuth Token Management

### List OAuth Tokens

List all stored OAuth tokens.

```http
GET /v0/management/oauth/tokens
```

**Request**:
```bash
curl -H "X-Management-Key: secret" \
  http://localhost:3456/v0/management/oauth/tokens
```

**Response**:
```json
{
  "tokens": [
    {
      "provider": "claude",
      "account_id": "user@example.com",
      "created_at": "2026-01-13T10:00:00Z",
      "last_refresh": "2026-01-13T15:00:00Z",
      "expires_at": "2026-01-14T03:00:00Z"
    },
    {
      "provider": "gemini",
      "account_id": "user@gmail.com",
      "created_at": "2026-01-13T12:00:00Z",
      "last_refresh": "2026-01-13T15:00:00Z",
      "expires_at": "2026-01-14T04:00:00Z"
    }
  ]
}
```

### Revoke OAuth Token

Revoke a specific OAuth token.

```http
DELETE /v0/management/oauth/tokens/{provider}/{account_id}
```

**Request**:
```bash
curl -X DELETE \
  -H "X-Management-Key: secret" \
  http://localhost:3456/v0/management/oauth/tokens/claude/user@example.com
```

**Response**:
```json
{
  "status": "success",
  "message": "OAuth token revoked",
  "provider": "claude",
  "account_id": "user@example.com",
  "revoked_at": "2026-01-13T15:30:00Z"
}
```

---

## Best Practices

### 1. Configuration Changes

**Do**:
- Test configuration changes in development first
- Use `reload` to revert bad changes
- Monitor health after changes
- Update model mappings before adding new providers

**Don't**:
- Change multiple settings at once
- Remove providers that are actively used
- Set `allow_remote = true` unnecessarily

### 2. API Key Management

**Do**:
- Generate unique keys per application/environment
- Revoke unused keys regularly
- Monitor key usage for anomalies
- Set expiration dates for temporary keys

**Don't****:
- Share API keys via email/chat
- Use the same key across environments
- Forget to copy generated keys immediately

### 3. Monitoring

**Do**:
- Check health endpoint regularly
- Monitor error rates and latency
- Set up alerts for degraded providers
- Review metrics daily

**Don't**:
- Ignore warning signs in logs
- Wait for outages to investigate
- Skip log level adjustments during debugging

### 4. Security

**Do**:
- Use strong, unique management secrets
- Rotate secrets regularly (30-90 days)
- Limit remote access to trusted networks
- Use HTTPS in production
- Audit API key usage

**Don't****:
- Use default or weak secrets
- Store secrets in config files
- Allow remote access unnecessarily
- Share management credentials

---

## Troubleshooting

### Authentication Failures

**Problem**: `401 Unauthorized` on all endpoints

**Solutions**:
1. Verify management secret is set correctly
2. Check header format: `X-Management-Key: secret`
3. Ensure `[management]` section has `enabled = true`
4. Verify secret isn't expired (if using token-based auth)

### Configuration Updates Not Working

**Problem**: Changes to config don't take effect

**Solutions**:
1. Check for validation errors in response
2. Verify config file is writable
3. Use `reload` endpoint to force reload
4. Check logs for config parsing errors

### Provider Health Checks Failing

**Problem**: Provider shows as "unhealthy"

**Solutions**:
1. Test provider API directly: `curl $api_base_url`
2. Check API key is valid
3. Verify network connectivity
4. Review provider-specific rate limits

### Cannot Add New Provider

**Problem**: `POST /providers` returns error

**Solutions**:
1. Verify provider name is unique
2. Check API base URL is reachable
3. Ensure models list is not empty
4. Validate transformers are supported

### Log Level Changes Not Persisting

**Problem**: Log level reverts after restart

**Solutions**:
1. Log level changes are runtime-only
2. Set default level in config file:
   ```toml
   [management.logging]
   default_level = "debug"
   ```

### High Memory Usage

**Problem**: Memory usage growing over time

**Solutions**:
1. Check for token storage bloat: `ls -la ~/.terraphim-llm-proxy/tokens/`
2. Clear old OAuth tokens
3. Review log retention settings
4. Monitor for memory leaks in provider connections

---

## Examples

### Example 1: Rotate Management Secret

```bash
# 1. Update config file
vim config.toml
# Change: secret_key = "new-secret-here"

# 2. Reload configuration
curl -X POST \
  -H "X-Management-Key: old-secret" \
  http://localhost:3456/v0/management/config/reload

# 3. Verify with new secret
curl -H "X-Management-Key: new-secret" \
  http://localhost:3456/v0/management/config
```

### Example 2: Switch Routing Strategy

```bash
# Change from FillFirst to RoundRobin
curl -X PUT \
  -H "X-Management-Key: secret" \
  -H "Content-Type: application/json" \
  -d '{"router": {"strategy": "RoundRobin"}}' \
  http://localhost:3456/v0/management/config
```

### Example 3: Enable Debug Logging Temporarily

```bash
# Set debug level
curl -X PUT \
  -H "X-Management-Key: secret" \
  -H "Content-Type: application/json" \
  -d '{"level": "debug"}' \
  http://localhost:3456/v0/management/logs/level

# ... reproduce issue ...

# Reset to info
curl -X PUT \
  -H "X-Management-Key: secret" \
  -H "Content-Type: application/json" \
  -d '{"level": "info"}' \
  http://localhost:3456/v0/management/logs/level
```

### Example 4: Monitor Provider Health

```bash
# Watch health endpoint every 5 seconds
watch -n 5 'curl -s -H "X-Management-Key: secret" \
  http://localhost:3456/v0/management/health | jq .'
```

---

## API Reference Summary

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/v0/management/config` | Get current config |
| PUT | `/v0/management/config` | Update config |
| POST | `/v0/management/config/reload` | Reload from disk |
| GET | `/v0/management/api-keys` | List API keys |
| POST | `/v0/management/api-keys` | Create API key |
| DELETE | `/v0/management/api-keys/:id` | Revoke API key |
| GET | `/v0/management/providers` | List providers |
| POST | `/v0/management/providers` | Add provider |
| DELETE | `/v0/management/providers/:name` | Remove provider |
| GET | `/v0/management/oauth/tokens` | List OAuth tokens |
| DELETE | `/v0/management/oauth/tokens/:provider/:account` | Revoke token |
| GET | `/v0/management/logs` | Get recent logs |
| GET | `/v0/management/logs/level` | Get log level |
| PUT | `/v0/management/logs/level` | Change log level |
| GET | `/v0/management/health` | Health status |
| GET | `/v0/management/metrics` | Usage metrics |

---

## Further Reading

- [Features Guide](./FEATURES.md) - Complete feature overview
- [OAuth Guide](./OAUTH_GUIDE.md) - OAuth authentication setup
- [Configuration Reference](./CONFIGURATION.md) - Full config options
- [Security Guide](./SECURITY.md) - Security best practices
