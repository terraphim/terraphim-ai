# TLA+ Specifications for Terraphim Multi-Agent System

Formal verification specs for the core multi-agent protocols.

## Specs

| Spec | Validates | Maps To |
|------|-----------|---------|
| `AgentSupervisor.tla` | OTP-style supervision, restart strategies | `crates/terraphim_agent_supervisor/` |
| `MessageDelivery.tla` | At-least-once delivery, retry bounds | `crates/terraphim_agent_messaging/` |

## Running

```bash
# Install TLC model checker (via tla+ toolbox or CLI)
# https://github.com/tlaplus/tlaplus/releases

# Check supervisor spec (OneForOne strategy)
tlc AgentSupervisor.tla -config AgentSupervisor.cfg

# Check message delivery spec
tlc MessageDelivery.tla -config MessageDelivery.cfg

# Check with different strategy (edit .cfg or override):
# Change Strategy = "OneForAll" or "RestForOne" in AgentSupervisor.cfg
```

## Properties Verified

### AgentSupervisor
- **RestartBoundSafety**: No agent exceeds `MaxRestarts` within time window
- **TypeOK**: All variables stay within declared types
- **EventualRecovery**: Failed agents eventually restart or supervisor escalates

### MessageDelivery
- **RetryBound**: No message retried more than `MaxRetries` times
- **NoDeliveredAndFailed**: A message can't be both delivered and failed
- **MessageStatePartition**: Messages are in exactly one logical state
- **EventualResolution**: Every pending message eventually resolves

## Future Specs to Add

1. **WorkflowOrchestration.tla** — RoleChaining, RoleParallelization fork-join, RoleWithReview convergence
2. **GoalAlignment.tla** — Multi-level goal hierarchy consistency, conflict detection
3. **TaskDecomposition.tla** — DAG execution with dependency ordering
4. **AgentEvolution.tla** — Learning/adaptation without state corruption
