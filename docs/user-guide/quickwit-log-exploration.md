# Quickwit Log Exploration Guide

This guide covers how to use Terraphim AI with Quickwit to explore and analyze log data from your applications and infrastructure.

## Overview

Quickwit is a cloud-native search engine optimized for log and trace data. When integrated with Terraphim AI, it provides:

- Full-text search across millions of log entries
- Field-specific filtering (level, service, timestamp)
- Multiple index discovery modes for different use cases
- Graceful degradation when services are unavailable

## Quick Start

### Prerequisites

- Quickwit server running (default: `http://localhost:7280`)
- Terraphim server or agent installed
- Logs indexed in Quickwit

### Minimal Configuration

Add a Quickwit haystack to your role configuration:

```json
{
  "haystacks": [
    {
      "location": "http://localhost:7280",
      "service": "Quickwit",
      "read_only": true,
      "extra_parameters": {
        "default_index": "your-logs-index",
        "max_hits": "100"
      }
    }
  ]
}
```

### First Search

```bash
# Start terraphim-agent
terraphim-agent

# In REPL, switch to a role with Quickwit configured
/role QuickwitLogs

# Search for errors
/search "level:ERROR"
```

## Configuration Modes

Quickwit haystack supports three discovery modes, each with different performance characteristics.

### 1. Explicit Index Mode (Fastest)

Best for: Production monitoring, known indexes

```json
{
  "extra_parameters": {
    "default_index": "workers-logs",
    "max_hits": "100",
    "sort_by": "-timestamp"
  }
}
```

| Metric | Value |
|--------|-------|
| API Calls | 1 |
| Latency | ~100ms |
| Use Case | Production monitoring |

### 2. Auto-Discovery Mode (Most Convenient)

Best for: Log exploration, discovering new indexes

```json
{
  "extra_parameters": {
    "max_hits": "50",
    "sort_by": "-timestamp"
  }
}
```

| Metric | Value |
|--------|-------|
| API Calls | N+1 (fetch indexes + search each) |
| Latency | ~300-500ms |
| Use Case | Exploration, unfamiliar instances |

### 3. Filtered Discovery Mode (Balanced)

Best for: Multi-service monitoring with control

```json
{
  "extra_parameters": {
    "index_filter": "workers-*",
    "max_hits": "100",
    "sort_by": "-timestamp"
  }
}
```

| Metric | Value |
|--------|-------|
| API Calls | N+1 (filtered) |
| Latency | ~200-400ms |
| Use Case | Multi-service with patterns |

**Supported Filter Patterns:**
- `workers-*` - Prefix match
- `*-logs` - Suffix match
- `*logs*` - Contains match
- `*` - All indexes

## Authentication

### Bearer Token

For services requiring token authentication:

```json
{
  "extra_parameters": {
    "auth_token": "Bearer your-token-here",
    "default_index": "logs"
  }
}
```

### Basic Authentication

For username/password authentication:

```json
{
  "extra_parameters": {
    "auth_username": "your-username",
    "auth_password": "${QUICKWIT_PASSWORD}"
  }
}
```

### Using 1Password

Securely inject credentials from 1Password:

```bash
# Set password from 1Password
export QUICKWIT_PASSWORD=$(op read "op://Private/Quickwit/password")

# Start agent
terraphim-agent
```

## Query Syntax

Quickwit uses a Lucene-like query syntax.

### Basic Queries

```bash
# Simple text search
/search error

# Phrase search
/search "connection refused"

# Wildcard
/search err*
```

### Field-Specific Queries

```bash
# Log level
/search "level:ERROR"
/search "level:WARN OR level:ERROR"

# Service name
/search "service:api-gateway"

# Combined
/search "level:ERROR AND service:auth"
```

### Time Range Queries

```bash
# After a date
/search "timestamp:[2024-01-01 TO *]"

# Between dates
/search "timestamp:[2024-01-01 TO 2024-01-31]"

# Last hour (relative)
/search "timestamp:[now-1h TO now]"
```

### Boolean Operators

```bash
# AND (both required)
/search "error AND database"

# OR (either matches)
/search "error OR warning"

# NOT (exclude)
/search "error NOT timeout"

# Grouping
/search "(error OR warning) AND database"
```

## Common Workflows

### Incident Investigation

1. **Start with broad search:**
   ```bash
   /search "level:ERROR"
   ```

2. **Narrow by time window:**
   ```bash
   /search "level:ERROR AND timestamp:[2024-01-15T10:00:00Z TO 2024-01-15T11:00:00Z]"
   ```

3. **Focus on specific service:**
   ```bash
   /search "level:ERROR AND service:payment-api"
   ```

4. **Look for patterns:**
   ```bash
   /search "timeout OR connection refused"
   ```

### Error Pattern Analysis

1. **Find all error types:**
   ```bash
   /search "level:ERROR"
   ```

2. **Group by message patterns:**
   ```bash
   /search "level:ERROR AND message:*database*"
   /search "level:ERROR AND message:*timeout*"
   /search "level:ERROR AND message:*authentication*"
   ```

### Performance Troubleshooting

1. **Find slow requests:**
   ```bash
   /search "duration:>1000"
   ```

2. **Identify bottlenecks:**
   ```bash
   /search "level:WARN AND message:*slow*"
   ```

3. **Check specific endpoints:**
   ```bash
   /search "path:/api/users AND duration:>500"
   ```

## Configuration Parameters

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `default_index` | string | none | Explicit index to search |
| `index_filter` | string | none | Glob pattern for auto-discovery |
| `max_hits` | string | "100" | Maximum results per index |
| `sort_by` | string | "-timestamp" | Sort field (- for descending) |
| `timeout_seconds` | string | "10" | HTTP request timeout |
| `auth_token` | string | none | Bearer token (include "Bearer " prefix) |
| `auth_username` | string | none | Basic auth username |
| `auth_password` | string | none | Basic auth password |

## Pre-configured Role

Terraphim AI includes a pre-configured "Quickwit Logs" role in `terraphim_engineer_config.json`:

```bash
# Switch to Quickwit Logs role
/role QuickwitLogs

# Search logs
/search "level:ERROR"
```

**Role Features:**
- Auto-discovery mode (searches all indexes)
- BM25 relevance function
- LLM summarization disabled (faster results)
- Dark theme optimized for log viewing
- Specialized system prompt for log analysis

## Performance Tips

1. **Use explicit index mode** for production monitoring where you know the target index

2. **Limit max_hits** to what you need - 50-100 is usually sufficient for investigation

3. **Add time constraints** to queries to reduce search scope

4. **Use filtered discovery** instead of full auto-discovery when you have many indexes

5. **Enable graceful degradation** - Quickwit haystack returns empty results on network failure rather than crashing

## Integration with Other Haystacks

Combine Quickwit log search with other data sources:

```json
{
  "haystacks": [
    {
      "location": "http://localhost:7280",
      "service": "Quickwit",
      "extra_parameters": { "default_index": "logs" }
    },
    {
      "location": "docs/src",
      "service": "Ripgrep"
    }
  ]
}
```

This allows searching both logs and documentation in a single query.

## Troubleshooting

See [Troubleshooting Guide](troubleshooting.md#quickwit-log-search-issues) for common issues and solutions.

## Related Documentation

- [Quickwit Integration Guide](../quickwit-integration.md)
- [Quickwit Search Skill](../../skills/quickwit-search/skill.md)
- [Example: Log Search Walkthrough](../../examples/quickwit-log-search.md)

---

*Last Updated: January 22, 2026*
*Version: Terraphim AI v1.6.0*
