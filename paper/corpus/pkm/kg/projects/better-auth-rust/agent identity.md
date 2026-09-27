# Agent Identity

First-class AI agent identity in better-auth-rust. Agents are distinct from human users with their own authentication, capabilities, and token management.

## Entity Structure

```rust
pub struct Agent {
    pub id: Uuid,
    pub name: String,
    pub agent_type: AgentType, // "ai_agent", "service", "bot"
    pub created_by: Uuid,     // User ID who created
    pub organization_id: Option<Uuid>,
    pub enabled: bool,
    pub capabilities: Vec<Capability>,
}
```

## Key Features

- Agent entities with typed agent_type (ai_agent, service, bot)
- Agents belong to users and optionally to organisations
- API keys scoped to agents for M2M authentication
- Capability-based access control
- Token vault for storing external service credentials
- SPIFFE/SPIRE workload identity integration

## Business Scenario: BS-002

AI agents authenticate via API keys, use capabilities to determine allowed actions, and store tokens in the vault for downstream service access. This enables agent-to-agent and agent-to-service auth patterns.

## Related

- x402 payments protocol for agent-to-agent economic transactions
- SPIFFE for workload identity verification
- Organization membership for multi-tenant agent management
