# Implementation Plan: PR #529 Gap Coverage

**Status**: Draft  
**Research Doc**: `docs/plans/pr529-gap-analysis-research.md`  
**Author**: AI Agent Analysis  
**Date**: 2026-02-20  
**Estimated Effort**: 16 hours

## Overview

### Summary
This plan addresses 15 identified gaps in PR #529, prioritized by criticality. P0 gaps must be resolved before production deployment. P1 gaps should be addressed in the immediate follow-up sprint. P2/P3 gaps are documented for future sprints.

### Approach
Focus on security (token leakage), reliability (gateway testing), and operational readiness (health checks, documentation). Each gap is addressed with minimal, focused changes.

### Scope

**In Scope (P0/P1):**
- GAP-003: Gateway outbound dispatch tests (P0 - Critical)
- GAP-001/GAP-002: Security - Remove token logging (P1)
- GAP-004: Channel error recovery (P1)
- GAP-009: Health check endpoint (P1)
- GAP-011/GAP-012: Gateway and channel documentation (P1)

**Out of Scope (P2/P3):**
- GAP-005: Session cleanup (deferred - P2)
- GAP-006: Rate limiting (deferred - P2)
- GAP-007: Matrix adapter (blocked by upstream - P3)
- GAP-008: ToolRegistry error handling (acceptable - P3)
- GAP-010: Config validation (nice-to-have - P3)
- GAP-013: Load testing (future work - P3)
- GAP-014: Degradation spec (docs - P2)
- GAP-015: CLI parity (intentional - P3)

**Avoid At All Cost:**
- Complex retry logic with exponential backoff (over-engineering)
- Distributed session storage (Redis, etc.) - premature optimization
- Web dashboard for monitoring - scope creep
- Custom metrics pipeline - use existing logging

## Architecture

### Component Diagram
```
                    +------------------+
                    |   Health Check   |
                    |    Endpoint      |
                    +--------+---------+
                             |
+------------+      +--------v---------+      +------------------+
|  Telegram  |<---->|                  |----->|  Agent Loop      |
|  Channel   |      |   MessageBus     |      |  (ToolCalling)   |
+-----^------+      |                  |<-----+------------------+
      |             +--------^---------+
      |                      |
+-----v------+      +--------v---------+      +------------------+
|  Discord   |<---->|   ChannelMgr     |----->|  SkillExecutor   |
|  Channel   |      |   (Recovery)     |      |  (with registry) |
+------------+      +------------------+      +------------------+
```

### Data Flow
```
Gateway Mode:
Telegram/Discord -> MessageBus.inbound -> AgentLoop -> MessageBus.outbound -> ChannelManager.send()

Health Check:
HTTP GET /health -> HealthCheckHandler -> Status Response
```

### Key Design Decisions

| Decision | Rationale | Alternatives Rejected |
|----------|-----------|----------------------|
| Remove token logging entirely | Security over debugging | Token hashing, partial logging |
| Test outbound dispatch via integration | Real behavior verification | Unit test with mocks |
| Health check as simple HTTP | Operational standard | Complex health aggregation |
| Channel panic isolation | Prevent gateway crashes | Process per channel (too heavy) |

### Eliminated Options

| Option Rejected | Why Rejected | Risk of Including |
|-----------------|--------------|-------------------|
| Token masking/hashing | Still leaks token length and format | Complexity without security benefit |
| Full circuit breaker pattern | Over-engineering for current scale | Premature optimization |
| Distributed session storage | Not needed for single-node deployment | Infrastructure complexity |
| Prometheus metrics | Overkill for initial release | Operational burden |

### Simplicity Check

**What if this could be easy?**
- Token logging: Just don't log it (easiest)
- Health check: Return HTTP 200 with process status (simplest)
- Error recovery: Catch panics, log error, continue (simplest)
- Gateway tests: Spawn channels, send message, verify delivery (simplest)

**Senior Engineer Test**: This design is minimal and direct. A senior engineer would approve.

**Nothing Speculative Checklist**:
- [x] No features the user didn't request
- [x] No abstractions "in case we need them later"
- [x] No flexibility "just in case"
- [x] No error handling for scenarios that cannot occur
- [x] No premature optimization

## File Changes

### New Files
| File | Purpose |
|------|---------|
| `crates/terraphim_tinyclaw/src/health.rs` | Health check endpoint handler |
| `crates/terraphim_tinyclaw/tests/gateway_integration.rs` | Gateway mode integration tests |
| `crates/terraphim_tinyclaw/tests/channel_recovery.rs` | Channel error recovery tests |
| `docs/src/gateway-deployment.md` | Gateway deployment guide |
| `docs/src/channel-setup.md` | Bot setup instructions |

### Modified Files
| File | Changes |
|------|---------|
| `crates/terraphim_tinyclaw/src/channels/telegram.rs` | Remove token logging |
| `crates/terraphim_tinyclaw/src/channels/discord.rs` | Remove token logging |
| `crates/terraphim_tinyclaw/src/channel.rs` | Add panic recovery wrapper |
| `crates/terraphim_tinyclaw/src/main.rs` | Add health check endpoint, fix logging |
| `crates/terraphim_tinyclaw/src/lib.rs` | Export health module |
| `crates/terraphim_tinyclaw/Cargo.toml` | Add optional axum for health endpoint |

### Deleted Files
| File | Reason |
|------|--------|
| None | All changes are additive or modifications |

## API Design

### Health Check Response
```rust
/// Health check response
#[derive(Debug, Serialize)]
pub struct HealthResponse {
    /// Overall status
    pub status: HealthStatus,
    /// Service version
    pub version: String,
    /// Timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Component statuses
    pub components: ComponentHealth,
}

#[derive(Debug, Serialize)]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
}

#[derive(Debug, Serialize)]
pub struct ComponentHealth {
    pub message_bus: bool,
    pub session_storage: bool,
    pub telegram: Option<bool>,
    pub discord: Option<bool>,
}
```

### Error Recovery Types
```rust
/// Result of channel execution with recovery
pub type ChannelResult<T> = Result<T, ChannelError>;

#[derive(Debug, Error)]
pub enum ChannelError {
    #[error("Channel panicked: {0}")]
    Panicked(String),
    #[error("Channel failed: {0}")]
    Failed(String),
    #[error("Channel unavailable")]
    Unavailable,
}
```

## Test Strategy

### Unit Tests
| Test | Location | Purpose |
|------|----------|---------|
| `test_telegram_no_token_log` | `telegram.rs` | Verify token not logged |
| `test_discord_no_token_log` | `discord.rs` | Verify token not logged |
| `test_health_response_format` | `health.rs` | Verify health check format |
| `test_channel_panic_recovery` | `channel.rs` | Verify panic isolation |

### Integration Tests
| Test | Location | Purpose |
|------|----------|---------|
| `test_gateway_message_flow` | `tests/gateway_integration.rs` | End-to-end message flow |
| `test_outbound_dispatch` | `tests/gateway_integration.rs` | GAP-003 verification |
| `test_channel_failure_isolation` | `tests/channel_recovery.rs` | GAP-004 verification |
| `test_health_endpoint` | `tests/gateway_integration.rs` | GAP-009 verification |

## Implementation Steps

### Step 1: Security - Remove Token Logging
**Files:** `crates/terraphim_tinyclaw/src/channels/telegram.rs`, `discord.rs`
**Description:** Remove all token logging including partial tokens
**Tests:** Unit tests verifying no token in logs
**Estimated:** 1 hour
**Priority:** P1

```rust
// BEFORE (remove this):
log::info!(
    "Telegram bot starting (token: {}...)",
    &self.config.token[..self.config.token.len().min(10)]
);

// AFTER:
log::info!("Telegram bot starting");
```

### Step 2: Gateway Integration Tests
**Files:** `crates/terraphim_tinyclaw/tests/gateway_integration.rs`
**Description:** Test that outbound messages are dispatched to channels
**Tests:** GAP-003 verification
**Estimated:** 4 hours
**Priority:** P0
**Depends on:** None

```rust
#[tokio::test]
async fn test_outbound_message_dispatch() {
    // Create message bus and channels
    let bus = Arc::new(MessageBus::new());
    let mock_channel = Arc::new(MockChannel::new());
    
    // Start gateway dispatch loop
    let bus_clone = bus.clone();
    tokio::spawn(async move {
        run_gateway_dispatch(bus_clone).await;
    });
    
    // Send outbound message
    let msg = OutboundMessage::new("mock", "chat1", "Hello".to_string());
    bus.outbound_sender().send(msg).await.unwrap();
    
    // Verify message received by channel
    tokio::time::timeout(Duration::from_secs(5), async {
        mock_channel.wait_for_message().await
    }).await.expect("Message should be dispatched");
}
```

### Step 3: Channel Error Recovery
**Files:** `crates/terraphim_tinyclaw/src/channel.rs`
**Description:** Wrap channel operations in catch_unwind to prevent gateway crashes
**Tests:** Unit and integration tests
**Estimated:** 3 hours
**Priority:** P1

```rust
/// Run channel with panic recovery
pub async fn run_with_recovery<F, Fut>(
    channel_name: &str,
    f: F,
) -> ChannelResult<()>
where
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = anyhow::Result<()>>,
{
    match std::panic::AssertUnwindSafe(f()).catch_unwind().await {
        Ok(Ok(())) => Ok(()),
        Ok(Err(e)) => {
            log::error!("Channel {} error: {}", channel_name, e);
            Err(ChannelError::Failed(e.to_string()))
        }
        Err(_) => {
            log::error!("Channel {} panicked", channel_name);
            Err(ChannelError::Panicked(channel_name.to_string()))
        }
    }
}
```

### Step 4: Health Check Endpoint
**Files:** `crates/terraphim_tinyclaw/src/health.rs`, `main.rs`
**Description:** Add HTTP health check endpoint for load balancers
**Tests:** Integration test
**Estimated:** 3 hours
**Priority:** P1

```rust
pub async fn health_check(
    bus: Arc<MessageBus>,
    config: &Config,
) -> impl axum::response::IntoResponse {
    let response = HealthResponse {
        status: HealthStatus::Healthy,
        version: env!("CARGO_PKG_VERSION").to_string(),
        timestamp: chrono::Utc::now(),
        components: ComponentHealth {
            message_bus: bus.is_healthy(),
            session_storage: check_session_storage(&config.agent.workspace).await,
            telegram: config.channels.telegram.as_ref().map(|_| true),
            discord: config.channels.discord.as_ref().map(|_| true),
        },
    };
    
    (StatusCode::OK, Json(response))
}
```

### Step 5: Documentation
**Files:** `docs/src/gateway-deployment.md`, `docs/src/channel-setup.md`
**Description:** User-facing documentation for gateway deployment
**Tests:** Doc tests, manual verification
**Estimated:** 4 hours
**Priority:** P1
**Depends on:** Steps 1-4

**Outline for gateway-deployment.md:**
1. Overview of gateway mode
2. Prerequisites (server, tokens)
3. Configuration file format
4. Docker deployment
5. systemd service setup
6. Health check usage
7. Troubleshooting

**Outline for channel-setup.md:**
1. Creating a Telegram bot (@BotFather)
2. Getting Discord bot token
3. Allowlist configuration
4. Testing channel connectivity

### Step 6: Cargo.toml Updates
**Files:** `crates/terraphim_tinyclaw/Cargo.toml`
**Description:** Add optional axum dependency for health endpoint
**Tests:** Build verification
**Estimated:** 1 hour
**Priority:** P1

```toml
[dependencies]
# Add for health endpoint
axum = { version = "0.7", optional = true, features = ["tokio"] }
tower = { version = "0.4", optional = true }

[features]
default = ["telegram", "discord", "health"]
health = ["dep:axum", "dep:tower"]
```

## Rollback Plan

If issues discovered:
1. **Token logging removal**: Revert specific commits, but tokens in logs are security risk
2. **Health endpoint**: Can be disabled by not building with `health` feature
3. **Channel recovery**: If causing issues, revert wrapper and accept crash risk
4. **Gateway tests**: Test-only changes, safe to keep

## Dependencies

### New Dependencies
| Crate | Version | Justification |
|-------|---------|---------------|
| axum | 0.7 | Health check HTTP endpoint |
| tower | 0.4 | Axum middleware support |

### Dependency Updates
None - all optional additions.

## Performance Considerations

### Expected Performance
| Metric | Target | Measurement |
|--------|--------|-------------|
| Health check latency | < 10ms | HTTP request |
| Channel recovery time | < 100ms | Panic to restart |
| Memory overhead | < 1MB | Recovery wrapper |

### No Benchmarks Required
Changes are operational, not performance-critical.

## Open Items

| Item | Status | Owner |
|------|--------|-------|
| Review security fix approach | Pending | Security review |
| Confirm health endpoint port | Pending | DevOps input |
| Validate documentation accuracy | Pending | Manual testing |

## Approval Checklist

- [ ] Security review of token logging removal
- [ ] Technical review of panic recovery approach
- [ ] DevOps approval of health endpoint design
- [ ] Test strategy approved
- [ ] Documentation review scheduled
- [ ] Human approval received

## Next Steps

1. Review this implementation plan
2. Approve P0 (GAP-003) test implementation
3. Prioritize P1 items based on deployment timeline
4. Schedule implementation sprints
5. Create GitHub issues for each gap

## Traceability Matrix

| Gap ID | Priority | Step | Test | Status |
|--------|----------|------|------|--------|
| GAP-001 | P1 | Step 1 | Unit test | Planned |
| GAP-002 | P1 | Step 1 | Unit test | Planned |
| GAP-003 | P0 | Step 2 | Integration test | Planned |
| GAP-004 | P1 | Step 3 | Integration test | Planned |
| GAP-009 | P1 | Step 4 | Integration test | Planned |
| GAP-011 | P1 | Step 5 | Doc review | Planned |
| GAP-012 | P1 | Step 5 | Doc review | Planned |
