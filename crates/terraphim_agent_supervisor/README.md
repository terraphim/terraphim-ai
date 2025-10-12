# Terraphim Agent Supervisor

OTP-inspired supervision trees for fault-tolerant AI agent management.

## Overview

This crate provides Erlang/OTP-style supervision patterns for managing AI agents, including automatic restart strategies, fault isolation, and hierarchical supervision. It implements the "let it crash" philosophy with fast failure detection and supervisor recovery.

## Core Concepts

### Supervision Trees
Hierarchical fault tolerance with automatic restart strategies:
- **OneForOne**: Restart only the failed agent
- **OneForAll**: Restart all agents if one fails
- **RestForOne**: Restart the failed agent and all agents started after it

### Agent Lifecycle
Complete agent lifecycle management:
- **Spawn**: Create and initialize new agents
- **Monitor**: Health checks and status monitoring
- **Restart**: Automatic restart on failure with configurable policies
- **Terminate**: Graceful shutdown and cleanup

### Fault Tolerance
Built-in resilience patterns:
- Fast failure detection with supervisor recovery
- Configurable restart intensity limits
- Circuit breaker patterns for cascading failure prevention
- Comprehensive error categorization and recovery strategies

## Quick Start

```rust
use std::sync::Arc;
use terraphim_agent_supervisor::{
    AgentSupervisor, SupervisorConfig, AgentSpec, TestAgentFactory,
    RestartStrategy, RestartPolicy
};
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create supervisor configuration
    let mut config = SupervisorConfig::default();
    config.restart_policy.strategy = RestartStrategy::OneForOne;

    // Create agent factory
    let factory = Arc::new(TestAgentFactory);

    // Create and start supervisor
    let mut supervisor = AgentSupervisor::new(config, factory);
    supervisor.start().await?;

    // Spawn an agent
    let spec = AgentSpec::new("test".to_string(), json!({}))
        .with_name("my-agent".to_string());
    let agent_id = supervisor.spawn_agent(spec).await?;

    println!("Agent {} spawned successfully", agent_id);

    // Simulate agent failure and restart
    supervisor.handle_agent_exit(
        agent_id,
        terraphim_agent_supervisor::ExitReason::Error("test failure".to_string())
    ).await?;

    // Stop supervisor
    supervisor.stop().await?;

    Ok(())
}
```

## Restart Strategies

### OneForOne
Restart only the failed agent. Best for independent agents.

```rust
let mut config = SupervisorConfig::default();
config.restart_policy.strategy = RestartStrategy::OneForOne;
```

### OneForAll
Restart all agents if one fails. Best for tightly coupled agents.

```rust
let mut config = SupervisorConfig::default();
config.restart_policy.strategy = RestartStrategy::OneForAll;
```

### RestForOne
Restart the failed agent and all agents started after it. Best for pipeline-style workflows.

```rust
let mut config = SupervisorConfig::default();
config.restart_policy.strategy = RestartStrategy::RestForOne;
```

## Restart Intensity

Control how aggressively agents are restarted:

```rust
use terraphim_agent_supervisor::{RestartPolicy, RestartIntensity};
use std::time::Duration;

// Lenient policy: 10 restarts in 2 minutes
let lenient = RestartPolicy::new(
    RestartStrategy::OneForOne,
    RestartIntensity::new(10, Duration::from_secs(120))
);

// Strict policy: 3 restarts in 30 seconds
let strict = RestartPolicy::new(
    RestartStrategy::OneForOne,
    RestartIntensity::new(3, Duration::from_secs(30))
);

// Never restart
let never = RestartPolicy::never_restart();
```

## Custom Agents

Implement the `SupervisedAgent` trait for custom agent types:

```rust
use async_trait::async_trait;
use terraphim_agent_supervisor::{
    SupervisedAgent, AgentPid, SupervisorId, AgentStatus,
    TerminateReason, SystemMessage, InitArgs, SupervisionResult
};

struct MyAgent {
    pid: AgentPid,
    supervisor_id: SupervisorId,
    status: AgentStatus,
}

#[async_trait]
impl SupervisedAgent for MyAgent {
    async fn init(&mut self, args: InitArgs) -> SupervisionResult<()> {
        self.pid = args.agent_id;
        self.supervisor_id = args.supervisor_id;
        self.status = AgentStatus::Starting;
        Ok(())
    }

    async fn start(&mut self) -> SupervisionResult<()> {
        self.status = AgentStatus::Running;
        // Start your agent logic here
        Ok(())
    }

    async fn stop(&mut self) -> SupervisionResult<()> {
        self.status = AgentStatus::Stopping;
        // Cleanup logic here
        self.status = AgentStatus::Stopped;
        Ok(())
    }

    // Implement other required methods...
}
```

## Monitoring and Observability

Get supervisor and agent status:

```rust
// Get supervisor status
let status = supervisor.status();
println!("Supervisor status: {:?}", status);

// Get all child agents
let children = supervisor.get_children().await;
for (pid, info) in children {
    println!("Agent {}: {:?} (restarts: {})",
        pid, info.status, info.restart_count);
}

// Get restart history
let history = supervisor.get_restart_history().await;
for entry in history {
    println!("Agent {} restarted at {} due to {:?}",
        entry.agent_id, entry.timestamp, entry.reason);
}
```

## Error Handling

The supervision system provides comprehensive error categorization:

```rust
use terraphim_agent_supervisor::{SupervisionError, ErrorCategory};

match supervisor.spawn_agent(spec).await {
    Ok(agent_id) => println!("Agent spawned: {}", agent_id),
    Err(e) => {
        println!("Error: {}", e);
        println!("Category: {:?}", e.category());
        println!("Recoverable: {}", e.is_recoverable());
    }
}
```

## Features

- **Fault Tolerance**: Automatic restart with configurable strategies
- **Health Monitoring**: Built-in health checks and status tracking
- **Resource Management**: Configurable limits and timeouts
- **Observability**: Comprehensive monitoring and restart history
- **Extensibility**: Custom agent types and factories
- **Performance**: Efficient async implementation with minimal overhead

## Integration

This crate integrates with the broader Terraphim ecosystem:

- **terraphim_persistence**: Agent state persistence
- **terraphim_types**: Common type definitions
- **Future**: Integration with knowledge graph-based agent coordination

## License

Licensed under the Apache License, Version 2.0.
