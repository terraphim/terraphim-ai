# Quickwit Log Search with Terraphim Agent - Complete Example

This example demonstrates how to use Terraphim AI with Quickwit to search and analyze application logs.

## Scenario

You're investigating a production incident where the API service started returning 500 errors around 10:30 AM on January 13, 2024. You need to:
1. Find error logs from the incident timeframe
2. Identify the root cause
3. Check if other services were affected

## Prerequisites

1. **Quickwit Server Running:**
   ```bash
   # Option 1: Docker
   docker run -p 7280:7280 quickwit/quickwit:0.7

   # Option 2: Docker Compose
   docker-compose up -d quickwit

   # Verify
   curl http://localhost:7280/health
   ```

2. **Terraphim Agent Built:**
   ```bash
   cargo build --release -p terraphim_agent --features repl-full
   ```

3. **Sample Data** (optional - for testing):
   ```bash
   # Ingest some test logs to Quickwit
   # See Quickwit documentation for ingestion
   ```

---

## Step-by-Step Walkthrough

### Step 1: Configure Quickwit Haystack

Create a role configuration for Quickwit searching:

**File:** `my-quickwit-config.json`
```json
{
  "name": "Incident Investigator",
  "shortname": "IncidentInvestigator",
  "relevance_function": "BM25",
  "theme": "observability",
  "haystacks": [
    {
      "location": "http://localhost:7280",
      "service": "Quickwit",
      "read_only": true,
      "fetch_content": false,
      "extra_parameters": {
        "default_index": "api-logs",
        "max_hits": "100",
        "sort_by": "-timestamp"
      }
    }
  ],
  "llm_enabled": false
}
```

---

### Step 2: Start Terraphim Agent

```bash
./target/release/terraphim-agent --config my-quickwit-config.json
```

**Expected Output:**
```
Terraphim Agent v1.5.0
Loading configuration from: my-quickwit-config.json
Role: Incident Investigator
Haystacks configured: 1
  - Quickwit: http://localhost:7280 (index: api-logs)

Ready for commands. Type /help for available commands.
>
```

---

### Step 3: Search for Errors Around Incident Time

```bash
> /search "level:ERROR AND timestamp:[2024-01-13T10:20:00Z TO 2024-01-13T10:40:00Z]"
```

**Expected Results:**
```
Found 23 documents from Quickwit:

1. [ERROR] Database connection pool exhausted
   Source: api-logs (2024-01-13T10:30:15Z)
   Service: api-server
   Tags: quickwit, logs, ERROR, api-server

2. [ERROR] Failed to acquire database connection: timeout after 30s
   Source: api-logs (2024-01-13T10:30:14Z)
   Service: api-server
   Tags: quickwit, logs, ERROR, api-server

3. [ERROR] HTTP 500: Internal Server Error
   Source: api-logs (2024-01-13T10:30:13Z)
   Service: api-gateway
   Tags: quickwit, logs, ERROR, api-gateway

...
```

---

### Step 4: Narrow Down to Specific Service

```bash
> /search "service:api-server AND level:ERROR"
```

**Results Show:**
- Database connection pool issues
- Timeout errors
- Connection refused errors

**Root Cause Identified:** Database connection pool exhaustion

---

### Step 5: Check Timeline Leading to Incident

```bash
# Find warnings before the errors
> /search "service:api-server AND level:WARN AND timestamp:[2024-01-13T10:00:00Z TO 2024-01-13T10:30:00Z]"
```

**Expected Output:**
```
Found 45 documents:

1. [WARN] Database connection pool at 80% capacity
   2024-01-13T10:25:00Z

2. [WARN] Slow query detected: 5.2s
   2024-01-13T10:20:00Z

3. [WARN] Database connection pool at 90% capacity
   2024-01-13T10:29:00Z
```

**Pattern:** Pool capacity warnings preceded the failure.

---

### Step 6: Check if Other Services Affected

Switch to auto-discovery mode to search all services:

**Update Config:** Change to `quickwit_autodiscovery_config.json`

```bash
# Restart agent with auto-discovery config
./target/release/terraphim-agent --config terraphim_server/default/quickwit_autodiscovery_config.json
```

```bash
> /search "database AND (error OR timeout)"
```

**Results from Multiple Indexes:**
```
Found 67 documents across 3 indexes:

From api-logs (23 docs):
  - Database connection errors

From worker-logs (15 docs):
  - Database timeout warnings

From payment-service-logs (29 docs):
  - Database connection refused errors
```

**Conclusion:** Database issue affected multiple services.

---

## Advanced Use Cases

### Use Case 1: Performance Investigation

**Goal:** Find all slow queries in the last 24 hours

```bash
> /search "slow OR performance OR latency AND timestamp:[2024-01-12T10:00:00Z TO *]"
```

Filter by severity:
```bash
> /search "level:WARN AND (slow OR latency > 1000ms)"
```

---

### Use Case 2: Security Audit

**Goal:** Find authentication failures and suspicious activity

```bash
# Failed login attempts
> /search "authentication failed OR login failed OR invalid credentials"

# Suspicious IP patterns
> /search "level:WARN AND (rate_limit OR blocked OR suspicious)"

# Security events
> /search "security OR unauthorized OR forbidden"
```

---

### Use Case 3: Deployment Correlation

**Goal:** Check logs after deployment at 14:00

```bash
# Errors after deployment
> /search "level:ERROR AND timestamp:[2024-01-13T14:00:00Z TO 2024-01-13T15:00:00Z]"

# Compare with pre-deployment baseline
> /search "level:ERROR AND timestamp:[2024-01-13T13:00:00Z TO 2024-01-13T14:00:00Z]"
```

---

### Use Case 4: Multi-Index Pattern Filtering

**Goal:** Search only production worker logs

**Configuration:**
```json
{
  "extra_parameters": {
    "index_filter": "prod-workers-*",
    "max_hits": "50"
  }
}
```

**Search:**
```bash
> /search "error OR failed"
```

Automatically searches: `prod-workers-logs`, `prod-workers-metrics`, `prod-workers-events`

---

## Integration with Code Search

Combine Quickwit (logs) with Ripgrep (code) for full context:

**Configuration:**
```json
{
  "name": "Full Stack Investigator",
  "haystacks": [
    {
      "location": "/path/to/codebase",
      "service": "Ripgrep"
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

**Unified Search:**
```bash
> /search "DatabaseConnectionPool"
```

**Results:**
- **From Code (Ripgrep):** Implementation of DatabaseConnectionPool class
- **From Logs (Quickwit):** Runtime errors and warnings from the pool

---

## Performance Comparison

### Explicit Index (Fastest)
```bash
# Configuration: default_index specified
# Time: ~100ms
# API Calls: 1
# Best for: Production monitoring, known indexes
```

### Auto-Discovery (Most Convenient)
```bash
# Configuration: No default_index
# Time: ~300-500ms
# API Calls: N+1 (list + search each)
# Best for: Exploration, unknown indexes
```

### Filtered Discovery (Balanced)
```bash
# Configuration: index_filter specified
# Time: ~200-400ms
# API Calls: 1 (list) + M (matched indexes)
# Best for: Multi-service monitoring
```

---

## Real-World Example: Production Setup

**Scenario:** Monitor production Quickwit at `https://logs.terraphim.cloud/api`

**Configuration:** `production-monitoring.json`
```json
{
  "name": "Production Monitor",
  "shortname": "ProdMonitor",
  "relevance_function": "BM25",
  "theme": "observability",
  "haystacks": [
    {
      "location": "https://logs.terraphim.cloud/api",
      "service": "Quickwit",
      "read_only": true,
      "fetch_content": false,
      "extra_parameters": {
        "auth_username": "cloudflare",
        "auth_password": "${QUICKWIT_PASSWORD}",
        "index_filter": "workers-*",
        "max_hits": "100",
        "sort_by": "-timestamp"
      }
    }
  ],
  "llm_enabled": false,
  "llm_auto_summarize": false,
  "llm_chat_enabled": false
}
```

**Usage:**
```bash
# Set password from 1Password
export QUICKWIT_PASSWORD=$(op read "op://Private/Quickwit/password")

# Start agent
terraphim-agent --config production-monitoring.json

# Monitor for errors
> /search "level:ERROR"

# Check specific time window
> /search "timestamp:[2024-01-13T10:00:00Z TO *]"

# Find database issues
> /search "database AND (error OR timeout OR failed)"
```

---

## Tips and Best Practices

### 1. Start Broad, Then Narrow

```bash
# Start with broad search
> /search error

# Narrow by service
> /search "service:api-server AND error"

# Narrow by time
> /search "service:api-server AND error AND timestamp:[recent]"
```

### 2. Use Wildcards Wisely

```bash
# Too broad (slow)
> /search "*"

# Better (specific field)
> /search "level:*"

# Best (specific value)
> /search "level:ERROR"
```

### 3. Combine Multiple Conditions

```bash
> /search "(level:ERROR OR level:WARN) AND service:critical-service"
```

### 4. Save Frequent Queries

Create role configs for common investigations:
- `error-monitor.json` - Only ERROR level
- `slow-query-detective.json` - Performance issues
- `security-audit.json` - Security events

### 5. Time Range Syntax

```bash
# Last hour (if Quickwit supports relative times)
> /search "timestamp:[now-1h TO now]"

# Specific window
> /search "timestamp:[2024-01-13T10:00:00Z TO 2024-01-13T11:00:00Z]"

# Everything after point
> /search "timestamp:[2024-01-13T10:30:00Z TO *]"
```

---

## Common Patterns

### Pattern 1: Error Spike Investigation
```bash
# Find when errors started
> /search "level:ERROR" --sort-by timestamp

# Group by service
> /search "level:ERROR" --group-by service

# Find common messages
> /search "level:ERROR" --field message
```

### Pattern 2: Service Health Check
```bash
# Current error rate
> /search "level:ERROR AND timestamp:[now-5m TO now]"

# Warning trends
> /search "level:WARN AND timestamp:[now-1h TO now]"
```

### Pattern 3: Deployment Validation
```bash
# Pre-deployment baseline
> /search "level:ERROR AND timestamp:[DEPLOY_TIME-1h TO DEPLOY_TIME]"

# Post-deployment check
> /search "level:ERROR AND timestamp:[DEPLOY_TIME TO DEPLOY_TIME+1h]"
```

---

## Limitations

1. **Time Ranges:** Require timestamp field in logs
2. **Aggregations:** Not supported (Quickwit feature not exposed)
3. **Streaming:** Search-only, no real-time log tailing
4. **Client Timeout:** Fixed at 10s (config parameter not yet wired)

---

## Next Steps

1. **Try It:** Follow Step 1-4 above with your Quickwit instance
2. **Customize:** Create role configs for your common searches
3. **Integrate:** Add Quickwit alongside existing haystacks
4. **Monitor:** Watch logs and optimize configuration

---

## Support

- **Documentation:** `docs/quickwit-integration.md`
- **Issues:** https://github.com/terraphim/terraphim-ai/issues
- **Examples:** `terraphim_server/default/quickwit_*.json`

---

**Status:** Production Ready
**Last Updated:** 2026-01-13
**Quickwit Version:** 0.7+
