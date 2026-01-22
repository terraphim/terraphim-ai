# Quickwit Search Skill

Search log and observability data using Quickwit search engine integration for Terraphim AI.

## When to Use

- Searching application logs for errors, warnings, or specific patterns
- Investigating production incidents and debugging issues
- Analyzing observability data across multiple indexes
- Correlating log entries with code changes or deployments
- Exploring log data without knowing exact index names

## Features

- **Hybrid Index Discovery** - Explicit configuration or auto-discovery
- **Dual Authentication** - Bearer token and Basic Auth support
- **Pattern Filtering** - Glob patterns like `logs-*`, `*-workers`
- **Multi-Index Search** - Search across multiple log indexes simultaneously
- **Time-Sorted Results** - Most recent logs first by default

## Quick Start

### Basic Search (Explicit Index)

```bash
# Configure Quickwit haystack
export QUICKWIT_CONFIG='{
  "location": "http://localhost:7280",
  "service": "Quickwit",
  "extra_parameters": {
    "default_index": "workers-logs"
  }
}'

# Search for errors
terraphim-agent --config quickwit_engineer_config.json
> /search error
```

### Auto-Discovery Mode

```bash
# Search all available indexes
terraphim-agent --config quickwit_autodiscovery_config.json
> /search "level:ERROR"
```

### Filtered Discovery

```bash
# Search only indexes matching pattern
terraphim-agent --config quickwit_production_config.json  # with index_filter: "workers-*"
> /search "database connection failed"
```

## Configuration Modes

### Mode 1: Explicit Index (Fast - Recommended for Production)

**Use When:**
- You know the exact index name
- Performance is critical
- Production monitoring

**Configuration:**
```json
{
  "name": "Quickwit Production Monitor",
  "haystacks": [
    {
      "location": "http://localhost:7280",
      "service": "Quickwit",
      "extra_parameters": {
        "default_index": "workers-logs",
        "max_hits": "100"
      }
    }
  ]
}
```

**Performance:** ~100-200ms per search (1 API call)

---

### Mode 2: Auto-Discovery (Convenient - Recommended for Exploration)

**Use When:**
- Exploring unfamiliar Quickwit instance
- Don't know available index names
- Want comprehensive search across all logs

**Configuration:**
```json
{
  "name": "Quickwit Explorer",
  "haystacks": [
    {
      "location": "http://localhost:7280",
      "service": "Quickwit",
      "extra_parameters": {
        "max_hits": "50"
      }
    }
  ]
}
```

**Performance:** ~300-500ms per search (N+1 API calls for N indexes)

---

### Mode 3: Filtered Discovery (Balanced)

**Use When:**
- Want convenience of auto-discovery
- Need to limit scope to specific indexes
- Multi-tenant or multi-service environments

**Configuration:**
```json
{
  "name": "Quickwit Service Monitor",
  "haystacks": [
    {
      "location": "http://localhost:7280",
      "service": "Quickwit",
      "extra_parameters": {
        "index_filter": "workers-*",
        "max_hits": "100"
      }
    }
  ]
}
```

**Glob Patterns:**
- `workers-*` - Matches: workers-logs, workers-metrics
- `*-logs` - Matches: api-logs, service-logs, workers-logs
- `*logs*` - Matches any index containing "logs"
- `*` - Matches all indexes (same as auto-discovery)

**Performance:** ~200-400ms per search (depends on matching indexes)

---

## Authentication

### Bearer Token

For API token-based authentication:

```json
{
  "extra_parameters": {
    "auth_token": "Bearer your-api-token-here",
    "default_index": "logs"
  }
}
```

**Security:** Store token in environment variable:
```bash
export QUICKWIT_TOKEN="Bearer $(cat ~/.quickwit/token)"
# Update config to use ${QUICKWIT_TOKEN}
```

---

### Basic Authentication

<!-- For username/password authentication, set auth_username and auth_password in extra_parameters -->

**Secure Password Management:**
```bash
# Using 1Password CLI
export QUICKWIT_PASSWORD=$(op read "op://Private/Quickwit/password")

# Or use existing environment variable if set
# export QUICKWIT_PASSWORD

# Then start terraphim-agent
terraphim-agent --config quickwit_production_config.json
```

---

## Query Syntax

Quickwit supports powerful query syntax:

### Simple Text Search
```
/search error
/search "database connection"
/search timeout
```

### Field-Specific Search
```
/search "level:ERROR"
/search "service:api-server"
/search "message:database"
```

### Boolean Operators
```
/search "error AND database"
/search "error OR timeout"
/search "level:ERROR AND service:api"
```

### Range Queries
```
/search "timestamp:[2024-01-01 TO 2024-01-31]"
/search "level:ERROR AND timestamp:[2024-01-13T00:00:00Z TO *]"
```

### Complex Queries
```
/search "level:ERROR AND (message:database OR message:connection)"
/search "service:api-server AND NOT level:INFO"
```

---

## Configuration Parameters

| Parameter | Required | Default | Description |
|-----------|----------|---------|-------------|
| `location` | Yes | - | Quickwit server URL (http://localhost:7280) |
| `service` | Yes | - | Must be "Quickwit" |
| `default_index` | No | Auto-discover | Specific index to search |
| `index_filter` | No | All | Glob pattern for filtering |
| `auth_token` | No | None | Bearer token (include "Bearer " prefix) |
| `auth_username` | No | None | Basic auth username |
| `auth_password` | No | None | Basic auth password |
| `max_hits` | No | 100 | Maximum results per index |
| `timeout_seconds` | No | 10 | HTTP request timeout |
| `sort_by` | No | -timestamp | Sort order (- for descending) |

---

## Usage Examples

### Example 1: Debug Production Error

```bash
terraphim-agent --config quickwit_engineer_config.json

# In REPL:
/search "level:ERROR AND timestamp:[2024-01-13T10:00:00Z TO *]"
/search "message:NullPointerException"
/search "service:payment-processor AND level:ERROR"
```

### Example 2: Incident Investigation

```bash
# Search across all service logs
terraphim-agent --config quickwit_autodiscovery_config.json

# Find all errors around incident time
/search "timestamp:[2024-01-13T09:50:00Z TO 2024-01-13T10:10:00Z]"

# Correlate with specific service
/search "service:api-gateway AND (error OR timeout OR 5**)"
```

### Example 3: Performance Analysis

```bash
# Search for slow queries
/search "message:slow AND level:WARN"

# Find timeout patterns
/search "timeout OR timed_out OR deadline_exceeded"

# Analyze specific endpoint
/search "path:/api/users AND (slow OR error)"
```

### Example 4: Multi-Service Correlation

Configure multiple Quickwit haystacks for different services:

```json
{
  "name": "Multi-Service Monitor",
  "haystacks": [
    {
      "location": "http://localhost:7280",
      "service": "Quickwit",
      "extra_parameters": {
        "default_index": "api-logs"
      }
    },
    {
      "location": "http://localhost:7280",
      "service": "Quickwit",
      "extra_parameters": {
        "default_index": "worker-logs"
      }
    }
  ]
}
```

Then search returns results from both indexes unified.

---

## Troubleshooting

### Connection Issues

**Problem:** "Failed to connect to Quickwit"

**Solutions:**
1. Verify Quickwit is running:
   ```bash
   curl http://localhost:7280/health
   ```
2. Check firewall/network connectivity
3. Verify URL in configuration

### Authentication Failures

**Problem:** Status 401 or 403

**Solutions:**
1. Verify credentials are correct
2. Check Bearer token includes "Bearer " prefix
3. Test auth with curl:
   ```bash
   curl -H "Authorization: Bearer token" http://localhost:7280/api/v1/indexes
   ```

### No Results

**Problem:** Search returns empty

**Possible Causes:**
1. Index is empty - verify:
   ```bash
   curl "http://localhost:7280/api/v1/workers-logs/search?query=*&max_hits=10"
   ```
2. Query doesn't match any logs - try broader query
3. Auto-discovery found no indexes - check logs for warnings

### Auto-Discovery Not Working

**Problem:** "No indexes discovered"

**Solutions:**
1. Test endpoint directly:
   ```bash
   curl http://localhost:7280/api/v1/indexes
   ```
2. Check authentication if required
3. Use explicit `default_index` as workaround

---

## Performance Tips

### For Fast Searches
- Use explicit `default_index` (avoids discovery overhead)
- Reduce `max_hits` to minimum needed
- Use specific field queries: `level:ERROR` faster than text search

### For Comprehensive Searches
- Use auto-discovery or `index_filter: "*"`
- Increase `max_hits` if needed
- Combine with time ranges to limit scope

---

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
        "index_filter": "logs-*"
      }
    }
  ]
}
```

Search query returns unified results from both code/docs and logs.

---

## Docker Setup for Testing

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

**Start Quickwit:**
```bash
docker-compose up -d
curl http://localhost:7280/health
```

---

## Best Practices

1. **Production:** Use explicit index configuration for predictable performance
2. **Development:** Use auto-discovery for exploration and debugging
3. **Security:** Store credentials in environment variables or 1Password
4. **Performance:** Limit max_hits and use specific queries
5. **Monitoring:** Watch terraphim-agent logs for Quickwit connectivity warnings

---

## Related Documentation

- **User Guide:** `docs/quickwit-integration.md` - Complete integration guide
- **Design Docs:** `.docs/design-quickwit-haystack-integration.md` - Technical specification
- **Examples:** `terraphim_server/default/quickwit_*.json` - Configuration templates

---

## Supported Quickwit Versions

- Quickwit 0.7+
- REST API v1
- Compatible with both open-source and cloud deployments

---

## Skill Metadata

**Skill Type:** Data Integration
**Complexity:** Medium
**Dependencies:** Running Quickwit instance
**Performance:** 100-500ms per search (depending on mode)
**Status:** Production Ready ✅
