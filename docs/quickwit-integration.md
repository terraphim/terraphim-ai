# Quickwit Integration Guide

## Overview

Terraphim AI supports Quickwit as a haystack for searching log and observability data. This integration enables unified search across code, documentation, and operational logs.

## Features

- **Hybrid Index Discovery**: Choose explicit configuration (fast) or auto-discovery (convenient)
- **Dual Authentication**: Supports both Bearer tokens and Basic Authentication
- **Glob Pattern Filtering**: Filter auto-discovered indexes with patterns like `logs-*`
- **Graceful Error Handling**: Network failures return empty results without crashing
- **Concurrent Search**: Multiple indexes searched efficiently
- **Compatible**: Works with Quickwit 0.7+ REST API

## Quick Start

### Prerequisites

1. Running Quickwit instance (local or remote)
2. Available indexes with data
3. Optional: Authentication credentials

### Basic Configuration (Explicit Index)

Create or modify your role configuration:

```json
{
  "name": "Quickwit Engineer",
  "relevance_function": "BM25",
  "haystacks": [
    {
      "location": "http://localhost:7280",
      "service": "Quickwit",
      "read_only": true,
      "extra_parameters": {
        "default_index": "workers-logs",
        "max_hits": "100"
      }
    }
  ]
}
```

**Run search:**
```bash
terraphim-agent
# In REPL:
/search error
```

## Configuration Modes

### Mode 1: Explicit Index (Recommended for Production)

Fast, predictable, single index search.

```json
{
  "extra_parameters": {
    "default_index": "workers-logs",
    "max_hits": "100",
    "sort_by": "-timestamp"
  }
}
```

**Pros:**
- Fastest (single API call)
- Predictable results
- Best for production monitoring

### Mode 2: Auto-Discovery (Recommended for Exploration)

Automatically discovers and searches all available indexes.

```json
{
  "extra_parameters": {
    "max_hits": "50"
  }
}
```

**Pros:**
- Zero configuration needed
- Automatically finds new indexes
- Great for exploration

**Cons:**
- Slower (~300ms additional latency)
- Searches all indexes (may return irrelevant results)

### Mode 3: Filtered Auto-Discovery

Best of both worlds - auto-discovery with pattern filtering.

```json
{
  "extra_parameters": {
    "index_filter": "workers-*",
    "max_hits": "50"
  }
}
```

**Glob Patterns:**
- `workers-*` - matches `workers-logs`, `workers-metrics`, etc.
- `*-logs` - matches `workers-logs`, `api-logs`, etc.
- `*logs*` - matches any index containing "logs"
- `*` - matches all indexes (same as auto-discovery)

## Authentication

### Bearer Token

For API token authentication:

```json
{
  "extra_parameters": {
    "auth_token": "Bearer your-token-here",
    "default_index": "logs"
  }
}
```

**Security:** Tokens are redacted in logs (only first 4 characters shown).

### Basic Authentication

For username/password authentication (like try_search):

```json
{
  "extra_parameters": {
    "auth_username": "cloudflare",
    "auth_password": "USE_ENV_VAR",
    "default_index": "workers-logs"
  }
}
```

**Recommended:** Use environment variables or 1Password for passwords:
```bash
export QUICKWIT_PASSWORD=$(op read "op://vault/item/password")
# Update config with password from environment
```

## Configuration Parameters

| Parameter | Required | Default | Description |
|-----------|----------|---------|-------------|
| `location` | Yes | - | Quickwit server base URL |
| `service` | Yes | - | Must be "Quickwit" |
| `default_index` | No | Auto-discover | Specific index to search |
| `index_filter` | No | All indexes | Glob pattern for filtering |
| `auth_token` | No | None | Bearer token (include "Bearer " prefix) |
| `auth_username` | No | None | Basic auth username |
| `auth_password` | No | None | Basic auth password |
| `max_hits` | No | 100 | Maximum results per index |
| `timeout_seconds` | No | 10 | HTTP request timeout |
| `sort_by` | No | -timestamp | Sort order (- for descending) |

## Query Syntax

Quickwit supports powerful query syntax:

```bash
# Simple text search
/search error

# Boolean operators
/search "error AND database"

# Field-specific search
/search "level:ERROR"

# Time range (if timestamp field exists)
/search "timestamp:[2024-01-01 TO 2024-01-31]"

# Combined
/search "level:ERROR AND message:database"
```

## Examples

### Example 1: Local Development

```json
{
  "location": "http://localhost:7280",
  "service": "Quickwit",
  "extra_parameters": {
    "default_index": "dev-logs"
  }
}
```

### Example 2: Production with Authentication

```json
{
  "location": "https://logs.terraphim.cloud/api",
  "service": "Quickwit",
  "extra_parameters": {
    "auth_username": "cloudflare",
    "auth_password": "${QUICKWIT_PASSWORD}",
    "index_filter": "workers-*",
    "max_hits": "50"
  }
}
```

### Example 3: Multiple Indexes (Multi-Haystack)

Search multiple specific indexes:

```json
{
  "haystacks": [
    {
      "location": "http://localhost:7280",
      "service": "Quickwit",
      "extra_parameters": {
        "default_index": "workers-logs"
      }
    },
    {
      "location": "http://localhost:7280",
      "service": "Quickwit",
      "extra_parameters": {
        "default_index": "api-logs"
      }
    }
  ]
}
```

## Troubleshooting

### Connection Refused

**Error:** "Failed to connect to Quickwit"

**Solutions:**
1. Verify Quickwit is running: `curl http://localhost:7280/health`
2. Check `location` URL is correct
3. Ensure no firewall blocking connection

### Authentication Failed

**Error:** Status 401 or 403

**Solutions:**
1. Verify credentials are correct
2. Check token includes "Bearer " prefix
3. Ensure password doesn't have special characters issues

### No Results

**Possible causes:**
1. Index is empty - verify with: `curl http://localhost:7280/api/v1/{index}/search?query=*`
2. Query doesn't match any logs
3. Auto-discovery found no indexes - check logs for warnings

### Auto-Discovery Not Working

**Error:** "No indexes discovered"

**Solutions:**
1. Verify `/api/v1/indexes` endpoint works: `curl http://localhost:7280/api/v1/indexes`
2. Check authentication if required
3. Try explicit `default_index` instead

## Performance Tuning

### For Fast Searches (Production)
- Use explicit `default_index` configuration
- Reduce `max_hits` to minimum needed (e.g., 20)
- Use specific index names, avoid auto-discovery

### For Comprehensive Searches (Development)
- Use auto-discovery or `index_filter: "*"`
- Increase `max_hits` if needed
- Search multiple indexes concurrently

## Integration with Other Haystacks

Quickwit works alongside other Terraphim haystacks:

```json
{
  "haystacks": [
    {
      "location": "/path/to/docs",
      "service": "Ripgrep"
    },
    {
      "location": "http://localhost:7280",
      "service": "Quickwit",
      "extra_parameters": {
        "default_index": "logs"
      }
    }
  ]
}
```

Searches return unified results from all configured haystacks.

## Docker Setup (Development)

```yaml
# docker-compose.yml
version: '3.8'
services:
  quickwit:
    image: quickwit/quickwit:0.7
    ports:
      - "7280:7280"
    command: ["quickwit", "run", "--service", "searcher"]
    volumes:
      - ./quickwit-data:/quickwit/qwdata
```

**Start:**
```bash
docker-compose up -d
# Verify: curl http://localhost:7280/health
```

## Reference Implementation

This integration is based on the try_search project at `/Users/alex/projects/zestic-ai/charm/try_search` which demonstrates:
- Quickwit REST API usage
- Multi-index support
- Basic Authentication
- Dynamic table rendering

## Supported Quickwit Versions

- Quickwit 0.7+
- REST API v1

## Limitations

1. **Time Range Queries:** Not yet supported in v1 (planned for v2)
2. **Aggregations:** Not supported (Quickwit feature not exposed)
3. **Real-time Streaming:** Not supported (search-only, no tail/streaming)
4. **Custom Timeout:** Client timeout fixed at 10s (config parameter not yet wired)

## Next Steps

1. Set up Quickwit instance (local or cloud)
2. Create indexes and ingest log data
3. Configure Terraphim role with Quickwit haystack
4. Search and explore logs from terraphim-agent CLI

## Support

For issues or questions:
- GitHub: https://github.com/terraphim/terraphim-ai/issues
- Documentation: https://terraphim.ai
