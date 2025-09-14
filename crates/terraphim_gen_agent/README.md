# Terraphim GenAgent

OTP GenServer-inspired agent behavior patterns for AI agents in the Terraphim ecosystem.

## Overview

The `terraphim_gen_agent` crate provides a standardized framework for building AI agents that follow the proven Erlang/OTP GenServer pattern. This enables robust, fault-tolerant agent systems with proper state management, message passing, and supervision integration.

## Key Features

- **GenServer Pattern**: Familiar `init`, `handle_call`, `handle_cast`, `handle_info` lifecycle
- **State Management**: Immutable state transitions with persistence and recovery
- **Message Handling**: Type-safe call, cast, and info message patterns
- **Lifecycle Management**: Complete agent lifecycle with statistics and monitoring
- **Hot Code Reloading**: Update agent behavior without stopping
- **Hibernation Support**: Memory-efficient agent hibernation
- **Supervision Integration**: Works seamlessly with `terraphim_agent_supervisor`
- **Fault Tolerance**: Proper error handling and recovery mechanisms

## Architecture

```
┌─────────────────┐    ┌──────────────────┐    ┌─────────────────┐
│   GenAgent      │    │  Runtime         │    │  Lifecycle      │
│   Behavior      │◄──►│  System          │◄──►│  Manager        │
└─────────────────┘    └──────────────────┘    └─────────────────┘
         │                       │                       │
         ▼                       ▼                       ▼
┌─────────────────┐    ┌──────────────────┐    ┌─────────────────┐
│   Message       │    │  State           │    │  Statistics     │
│   Handling      │    │  Management      │    │  & Monitoring   │
└─────────────────┘    └──────────────────┘    └─────────────────┘
```

## Quick Start

### 1. Define Your Agent State

```rust
use serde::{Deserialize, Serialize};
use terraphim_gen_agent::{AgentState, GenAgentResult};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct MyAgentState {
    counter: u64,
    name: String,
    active: bool,
}

impl AgentState for MyAgentState {
    fn serialize(&self) -> GenAgentResult<String> {
        serde_json::to_string(self)
            .map_err(|e| terraphim_gen_agent::GenAgentError::StateSerialization(
                terraphim_gen_agent::AgentPid::new(),
                e.to_string()
            ))
    }
    
    fn deserialize(data: &str) -> GenAgentResult<Self> {
        serde_json::from_str(data)
            .map_err(|e| terraphim_gen_agent::GenAgentError::StateDeserialization(
                terraphim_gen_agent::AgentPid::new(),
                e.to_string()
            ))
    }
    
    fn validate(&self) -> GenAgentResult<()> {
        if self.name.is_empty() {
            return Err(terraphim_gen_agent::GenAgentError::StateTransitionFailed(
                terraphim_gen_agent::AgentPid::new(),
                "Name cannot be empty".to_string()
            ));
        }
        Ok(())
    }
}
```

### 2. Implement Your Agent

```rust
use async_trait::async_trait;
use terraphim_gen_agent::{
    GenAgent, GenAgentInitArgs, GenAgentResult,
    CallContext, CastContext, InfoContext,
};

struct MyAgent {
    name: String,
}

#[derive(Debug, Clone)]
struct CallMessage { request: String }

#[derive(Debug, Clone)]
struct CallReply { response: String }

#[derive(Debug, Clone)]
struct CastMessage { notification: String }

#[derive(Debug, Clone)]
struct InfoMessage { info: String }

#[async_trait]
impl GenAgent<MyAgentState> for MyAgent {
    type CallMessage = CallMessage;
    type CallReply = CallReply;
    type CastMessage = CastMessage;
    type InfoMessage = InfoMessage;

    async fn init(&mut self, args: GenAgentInitArgs) -> GenAgentResult<MyAgentState> {
        Ok(MyAgentState {
            counter: 0,
            name: self.name.clone(),
            active: true,
        })
    }

    async fn handle_call(
        &mut self,
        message: Self::CallMessage,
        context: CallContext,
        mut state: MyAgentState,
    ) -> GenAgentResult<(Self::CallReply, MyAgentState)> {
        state.counter += 1;
        let reply = CallReply {
            response: format!("Processed: {}", message.request),
        };
        Ok((reply, state))
    }

    async fn handle_cast(
        &mut self,
        message: Self::CastMessage,
        context: CastContext,
        mut state: MyAgentState,
    ) -> GenAgentResult<MyAgentState> {
        state.counter += 1;
        println!("Received notification: {}", message.notification);
        Ok(state)
    }

    async fn handle_info(
        &mut self,
        message: Self::InfoMessage,
        context: InfoContext,
        state: MyAgentState,
    ) -> GenAgentResult<MyAgentState> {
        println!("Received info: {}", message.info);
        Ok(state)
    }
}
```

### 3. Create and Run Your Agent

```rust
use std::sync::Arc;
use std::time::Duration;
use terraphim_gen_agent::{
    GenAgentFactory, RuntimeConfig, StateManager,
    AgentPid, SupervisorId, GenAgentInitArgs,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create state manager and factory
    let state_manager = Arc::new(StateManager::new(false));
    let config = RuntimeConfig::default();
    let factory = GenAgentFactory::new(state_manager, config);
    
    // Create agent
    let agent_id = AgentPid::new();
    let supervisor_id = SupervisorId::new();
    
    let init_args = GenAgentInitArgs {
        agent_id: agent_id.clone(),
        supervisor_id: supervisor_id.clone(),
        config: serde_json::json!({}),
        timeout: Duration::from_secs(30),
    };
    
    let agent = MyAgent {
        name: "my_agent".to_string(),
    };
    
    let runtime = factory.create_agent(
        agent,
        agent_id.clone(),
        supervisor_id,
        init_args,
        None, // Use default behavior spec
        None, // Use default runtime config
    ).await?;
    
    println!("Agent created and running!");
    
    // Agent will run until explicitly stopped
    // factory.stop_agent(&agent_id).await?;
    
    Ok(())
}
```

## Core Concepts

### GenAgent Trait

The `GenAgent` trait is the heart of the framework, providing the standard GenServer callbacks:

- `init`: Initialize the agent with starting state
- `handle_call`: Handle synchronous messages that expect a reply
- `handle_cast`: Handle asynchronous messages (fire-and-forget)
- `handle_info`: Handle system/info messages
- `handle_system`: Handle supervisor messages
- `terminate`: Clean up when the agent stops
- `code_change`: Handle hot code reloading

### State Management

Agent state is managed through the `AgentState` trait and `StateContainer`:

- **Immutable Transitions**: State changes create new state instances
- **Persistence**: States can be serialized and persisted
- **Validation**: States are validated on each transition
- **Versioning**: State containers track version numbers
- **Recovery**: States can be recovered from persistent storage

### Message Types

The framework supports three main message types:

1. **Call Messages**: Synchronous request-reply pattern
2. **Cast Messages**: Asynchronous fire-and-forget messages
3. **Info Messages**: System notifications and events

### Lifecycle Management

The `LifecycleManager` handles the complete agent lifecycle:

- **Phases**: Created → Initializing → Running → Hibernating → Terminating → Terminated
- **Statistics**: Message counts, processing times, error rates
- **Health Monitoring**: Uptime tracking and health checks
- **Hibernation**: Memory-efficient sleep mode

### Runtime System

The `GenAgentRuntime` provides the execution environment:

- **Message Processing**: Concurrent message handling with backpressure
- **Error Handling**: Proper error propagation and recovery
- **Metrics**: Performance monitoring and statistics
- **Configuration**: Tunable runtime parameters

## Advanced Features

### Hot Code Reloading

```rust
async fn code_change(
    &mut self,
    old_version: String,
    state: MyAgentState,
    extra: serde_json::Value,
) -> GenAgentResult<MyAgentState> {
    // Migrate state for new code version
    println!("Upgrading from version: {}", old_version);
    Ok(state)
}
```

### Custom Hibernation Logic

```rust
fn should_hibernate(&self, state: &MyAgentState) -> bool {
    // Custom hibernation logic
    !state.active || state.counter > 1000
}
```

### Debug and Monitoring

```rust
fn format_status(&self, state: &MyAgentState) -> serde_json::Value {
    serde_json::json!({
        "counter": state.counter,
        "active": state.active,
        "status": "healthy"
    })
}
```

## Configuration

### Runtime Configuration

```rust
let config = RuntimeConfig {
    message_buffer_size: 1000,
    max_concurrent_messages: 100,
    message_timeout: Duration::from_secs(30),
    hibernation_timeout: Some(Duration::from_secs(300)),
    enable_tracing: true,
    enable_metrics: true,
};
```

### Behavior Specification

```rust
let behavior_spec = BehaviorSpec {
    name: "my_agent".to_string(),
    version: "1.0.0".to_string(),
    description: "My custom agent".to_string(),
    timeout: Duration::from_secs(30),
    hibernation_after: Some(Duration::from_secs(300)),
    debug_options: DebugOptions {
        trace_calls: true,
        trace_casts: true,
        trace_info: false,
        log_state_changes: true,
        statistics: true,
    },
};
```

## Integration

### With Supervision

```rust
use terraphim_agent_supervisor::{Supervisor, AgentSpec, RestartStrategy};

// Agents created through GenAgentFactory are automatically
// compatible with the supervision system
let supervisor = Supervisor::new(supervisor_id, RestartStrategy::OneForOne);
```

### With Messaging

```rust
use terraphim_agent_messaging::{MessageSystem, AgentMessage};

// GenAgent runtime integrates with the messaging system
// for inter-agent communication
```

## Performance

The framework is designed for high performance:

- **Zero-copy Message Passing**: Efficient message handling
- **Concurrent Processing**: Multiple messages processed concurrently
- **Memory Efficient**: Hibernation and state management optimizations
- **Minimal Allocations**: Careful memory management

See the benchmarks in `benches/genagent_benchmarks.rs` for performance characteristics.

## Testing

Run the test suite:

```bash
cargo test
```

Run integration tests:

```bash
cargo test --test integration_tests
```

Run benchmarks:

```bash
cargo bench --features benchmarks
```

## Examples

See the `tests/` directory for comprehensive examples of:

- Basic agent implementation
- State persistence
- Error handling
- Concurrent operations
- Performance testing

## Contributing

Contributions are welcome! Please see the main Terraphim repository for contribution guidelines.

## License

This project is licensed under the Apache License 2.0 - see the LICENSE file for details.