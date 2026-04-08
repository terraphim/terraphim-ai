# NightwatchMonitor in Bigbox ADF Orchestrator

## Overview

The ADF (Autonomous Development Factory) orchestrator running on **bigbox** (`ssh bigbox`) uses the `NightwatchMonitor` system for behavioral drift detection and cost-aware agent management. This document describes the configuration, capabilities, and operational procedures.

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    ADF Orchestrator (bigbox)                │
│                                                             │
│  ┌──────────────┐    ┌──────────────┐    ┌──────────────┐  │
│  │   Agents     │───▶│  Nightwatch  │───▶│   Alerts     │  │
│  │   (Core/     │    │   Monitor    │    │  (Drift)     │  │
│  │   Safety)    │    │              │    │              │  │
│  └──────────────┘    └──────┬───────┘    └──────────────┘  │
│                             │                               │
│                             ▼                               │
│                    ┌──────────────┐                        │
│                    │ CostTracker  │                        │
│                    │ (Budget/     │                        │
│                    │  Token Use)  │                        │
│                    └──────────────┘                        │
└─────────────────────────────────────────────────────────────┘
```

## Configuration

Nightwatch is configured in the orchestrator's TOML file (typically `/home/alex/.config/terraphim/adf-orchestrator.toml` on bigbox):

```toml
[nightwatch]
eval_interval_secs = 300          # Evaluate drift every 5 minutes
minor_threshold = 0.10            # 10% drift = minor warning
moderate_threshold = 0.20         # 20% drift = moderate (restart agent)
severe_threshold = 0.40           # 40% drift = severe (restart agent)
critical_threshold = 0.70         # 70% drift = critical (pause + escalate)
active_start_hour = 0             # Monitor 24/7 (midnight start)
active_end_hour = 24              # Monitor 24/7 (midnight end)

# Drift calculation weights (configurable)
error_weight = 0.35               # Error rate contribution
success_weight = 0.25             # Command success rate contribution
health_weight = 0.20              # Health check contribution
budget_weight = 0.20              # Budget exhaustion contribution
```

## Drift Detection Metrics

Nightwatch monitors four dimensions of agent behavior:

### 1. Error Rate (`error_weight = 0.35`)
- Tracks stderr output containing error keywords
- Keywords: "error", "panic", "fatal", "failed"
- Normal stderr (e.g., bun init output) is not counted

### 2. Command Success Rate (`success_weight = 0.25`)
- Derived from error rate: `1.0 - error_rate`
- Represents proportion of successful operations

### 3. Health Score (`health_weight = 0.20`)
- Based on periodic health checks via `HealthStatus`
- Values: `Healthy`, `Degraded`, `Unhealthy`

### 4. Budget Exhaustion (`budget_weight = 0.20`)
- Tracks spend against monthly budget caps
- Triggers when >80% of budget consumed
- Integrates with `CostTracker` for token usage data

## Cost Tracking Integration

The ADF orchestrator extracts token usage from CLI tool outputs and feeds it into nightwatch:

### Token Parsing
Located in `crates/terraphim_orchestrator/src/flow/token_parser.rs`:

```rust
pub struct TokenUsage {
    pub input_tokens: Option<u64>,
    pub output_tokens: Option<u64>,
    pub total_tokens: Option<u64>,
    pub cost_usd: Option<f64>,
}
```

Supported output formats:
- OpenAI/Anthropic style: `Usage: Input: 1234, Output: 567`
- JSON style: `{"input_tokens": 1500, "output_tokens": 600}`
- Cost format: `Cost: $0.023`

### Flow Integration
When flows complete, token data is fed to nightwatch:

```rust
// In reconcile_tick()
for envelope in &state.step_envelopes {
    if let (Some(cost), Some(input), Some(output)) = (
        envelope.cost_usd,
        envelope.input_tokens,
        envelope.output_tokens,
    ) {
        self.nightwatch.observe_cost(
            &format!("flow-{}", name),
            cost,
            input,
            output,
            None,
        );
    }
}
```

## Drift Alert Levels

| Level | Threshold | Action | Notification |
|-------|-----------|--------|--------------|
| Normal | < 10% | None | None |
| Minor | 10-20% | Log warning | Console |
| Moderate | 20-40% | Restart agent | Console + Log |
| Severe | 40-70% | Restart agent | Console + Log |
| Critical | > 70% | Pause + escalate | All channels |

## Operational Commands

### Check Current Drift Scores
```bash
# Via orchestrator REPL (if available)
/orchestrator drift-scores

# Via logs
tail -f /var/log/terraphim/adf-orchestrator.log | grep "drift"
```

### View Nightwatch Configuration
```bash
# On bigbox
cat /home/alex/.config/terraphim/adf-orchestrator.toml | grep -A 20 "\[nightwatch\]"
```

### Restart Orchestrator with New Weights
```bash
# On bigbox
sudo systemctl restart terraphim-adf-orchestrator
# or
pkill -f "terraphim-orchestrator" && ./target/release/terraphim-orchestrator --config ~/.config/terraphim/adf-orchestrator.toml
```

## Monitoring Integration

### Symphony Integration
The Symphony multi-agent system on bigbox uses nightwatch for:
- Pre-dispatch drift checks (agents with high drift are not dispatched)
- Post-execution cost accounting
- Fleet-wide budget enforcement

### Gitea Robot API
Nightwatch drift scores are exposed via the Gitea Robot API:
```bash
curl -H "Authorization: token $GITEA_TOKEN" \
  https://git.terraphim.cloud/api/v1/robot/triage?owner=terraphim&repo=symphony
```

## Troubleshooting

### High Drift Without Errors
If agents show high drift scores without error output:
- Check budget exhaustion: `budget_exhaustion_rate > 0.8` contributes to drift
- Verify health checks are passing
- Review `budget_weight` configuration

### Missing Cost Data
If token usage is not appearing:
- Verify CLI tools output usage data in supported formats
- Check `token_parser.rs` regex patterns match your CLI output
- Enable debug logging: `RUST_LOG=debug` to see parsed tokens

### Adjusting Sensitivity
To make nightwatch more/less sensitive:
```toml
# More sensitive to errors
[nightwatch]
error_weight = 0.50
success_weight = 0.20
health_weight = 0.15
budget_weight = 0.15
```

## References

- `crates/terraphim_orchestrator/src/nightwatch.rs` - Core implementation
- `crates/terraphim_orchestrator/src/cost_tracker.rs` - Budget tracking
- `crates/terraphim_orchestrator/src/flow/token_parser.rs` - Token extraction
- `.docs/design-dark-factory-orchestration.md` - Architecture design
