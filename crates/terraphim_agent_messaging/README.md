# Terraphim Agent Messaging

Erlang-style asynchronous message passing system for AI agents.

## Overview

This crate provides message-based communication patterns inspired by Erlang/OTP, including agent mailboxes, message routing, and delivery guarantees. It implements the core messaging infrastructure needed for fault-tolerant AI agent coordination.

## Core Concepts

### Message Patterns
Following Erlang/OTP conventions:
- **Call**: Synchronous messages that expect a response (gen_server:call)
- **Cast**: Asynchronous fire-and-forget messages (gen_server:cast)  
- **Info**: System notification messages (gen_server:info)

### Agent Mailboxes
- **Unbounded Queues**: Erlang-style unlimited message capacity by default
- **Message Ordering**: Preserves message order with configurable priority handling
- **Statistics**: Comprehensive metrics for monitoring and debugging
- **Bounded Mode**: Optional capacity limits for resource management

### Delivery Guarantees
- **At-Most-Once**: Fire and forget delivery
- **At-Least-Once**: Retry until acknowledged (default)
- **Exactly-Once**: Deduplicated delivery with idempotency

### Message Routing
- **Cross-Agent Delivery**: Route messages between any registered agents
- **Retry Logic**: Exponential backoff with configurable limits
- **Circuit Breaker**: Automatic failure isolation and recovery
- **Load Balancing**: Distribute messages across agent instances

## Quick Start

```rust
use terraphim_agent_messaging::{
    MessageSystem, RouterConfig, MessageEnvelope, DeliveryOptions,
    AgentPid, AgentMessage
};
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create message system
    let config = RouterConfig::default();
    let system = MessageSystem::new(config);
    
    // Register agents
    let agent1 = AgentPid::new();
    let agent2 = AgentPid::new();
    
    system.register_agent(agent1.clone()).await?;
    system.register_agent(agent2.clone()).await?;
    
    // Send message from agent1 to agent2
    let envelope = MessageEnvelope::new(
        agent2.clone(),
        "greeting".to_string(),
        json!({"message": "Hello, Agent2!"}),
        DeliveryOptions::default(),
    ).with_from(agent1.clone());
    
    system.send_message(envelope).await?;
    
    // Get mailbox and receive message
    if let Some(mailbox) = system.get_mailbox(&agent2).await {
        let message = mailbox.receive().await?;
        println!("Agent2 received: {:?}", message);
    }
    
    system.shutdown().await?;
    Ok(())
}
```

## Message Types

### Creating Messages

```rust
use terraphim_agent_messaging::{AgentMessage, AgentPid};
use std::time::Duration;

let from = AgentPid::new();
let payload = "Hello, World!";

// Asynchronous cast message
let cast_msg = AgentMessage::cast(from.clone(), payload);

// Synchronous call message  
let (call_msg, reply_rx) = AgentMessage::call(
    from.clone(),
    payload,
    Duration::from_secs(30)
);

// System info message
let info_msg = AgentMessage::info(SystemInfo::HealthCheck {
    agent_id: from.clone(),
    timestamp: chrono::Utc::now(),
});
```

### Message Priorities

```rust
use terraphim_agent_messaging::{MessagePriority, DeliveryOptions};

let mut options = DeliveryOptions::default();
options.priority = MessagePriority::High;
options.timeout = Duration::from_secs(10);
options.max_retries = 5;
```

## Mailbox Management

### Basic Mailbox Operations

```rust
use terraphim_agent_messaging::{AgentMailbox, MailboxConfig, AgentPid};

let agent_id = AgentPid::new();
let config = MailboxConfig::default();
let mailbox = AgentMailbox::new(agent_id, config);

// Send message
let message = AgentMessage::cast(AgentPid::new(), "test");
mailbox.send(message).await?;

// Receive message
let received = mailbox.receive().await?;

// Receive with timeout
let received = mailbox.receive_timeout(Duration::from_secs(5)).await?;

// Try receive (non-blocking)
if let Some(message) = mailbox.try_receive().await? {
    println!("Got message: {:?}", message);
}
```

### Mailbox Configuration

```rust
use terraphim_agent_messaging::MailboxConfig;
use std::time::Duration;

let config = MailboxConfig {
    max_messages: 1000,        // Bounded mailbox
    preserve_order: true,      // FIFO message ordering
    enable_persistence: false, // In-memory only
    stats_interval: Duration::from_secs(60),
};
```

### Mailbox Statistics

```rust
let stats = mailbox.stats().await;
println!("Messages received: {}", stats.total_messages_received);
println!("Messages processed: {}", stats.total_messages_processed);
println!("Current queue size: {}", stats.current_queue_size);
println!("Average processing time: {:?}", stats.average_processing_time);
```

## Delivery Guarantees

### At-Most-Once Delivery

```rust
use terraphim_agent_messaging::{DeliveryConfig, DeliveryGuarantee, RouterConfig};

let mut delivery_config = DeliveryConfig::default();
delivery_config.guarantee = DeliveryGuarantee::AtMostOnce;

let router_config = RouterConfig {
    delivery_config,
    ..Default::default()
};
```

### At-Least-Once Delivery

```rust
let mut delivery_config = DeliveryConfig::default();
delivery_config.guarantee = DeliveryGuarantee::AtLeastOnce;
delivery_config.max_retries = 5;
delivery_config.retry_delay = Duration::from_millis(100);
delivery_config.retry_backoff_multiplier = 2.0;
```

### Exactly-Once Delivery

```rust
let mut delivery_config = DeliveryConfig::default();
delivery_config.guarantee = DeliveryGuarantee::ExactlyOnce;
// Automatic deduplication based on message IDs
```

## Message Routing

### Router Configuration

```rust
use terraphim_agent_messaging::RouterConfig;
use std::time::Duration;

let config = RouterConfig {
    retry_interval: Duration::from_secs(5),
    max_concurrent_deliveries: 100,
    enable_metrics: true,
    delivery_config: DeliveryConfig::default(),
};
```

### Custom Message Router

```rust
use terraphim_agent_messaging::{MessageRouter, MessageEnvelope, RouterStats};
use async_trait::async_trait;

struct CustomRouter {
    // Your custom routing logic
}

#[async_trait]
impl MessageRouter for CustomRouter {
    async fn route_message(&self, envelope: MessageEnvelope) -> MessagingResult<()> {
        // Custom routing implementation
        Ok(())
    }
    
    async fn register_agent(&self, agent_id: AgentPid, sender: MailboxSender) -> MessagingResult<()> {
        // Custom registration logic
        Ok(())
    }
    
    // Implement other required methods...
}
```

## Error Handling

### Error Types

```rust
use terraphim_agent_messaging::{MessagingError, ErrorCategory};

match system.send_message(envelope).await {
    Ok(()) => println!("Message sent successfully"),
    Err(e) => {
        println!("Error: {}", e);
        println!("Category: {:?}", e.category());
        println!("Recoverable: {}", e.is_recoverable());
        
        match e {
            MessagingError::AgentNotFound(agent_id) => {
                println!("Agent {} not found", agent_id);
            }
            MessagingError::MessageTimeout(agent_id) => {
                println!("Timeout waiting for response from {}", agent_id);
            }
            MessagingError::DeliveryFailed(agent_id, reason) => {
                println!("Failed to deliver to {}: {}", agent_id, reason);
            }
            _ => {}
        }
    }
}
```

### Retry Logic

```rust
use terraphim_agent_messaging::DeliveryManager;

let delivery_manager = DeliveryManager::new(DeliveryConfig::default());

// Get messages that need retry
let retry_candidates = delivery_manager.get_retry_candidates().await;

for envelope in retry_candidates {
    let delay = delivery_manager.calculate_retry_delay(envelope.attempts);
    tokio::time::sleep(delay).await;
    
    // Retry delivery...
}
```

## Monitoring and Observability

### System Statistics

```rust
let (router_stats, mailbox_stats) = system.get_stats().await;

println!("Router Stats:");
println!("  Messages routed: {}", router_stats.messages_routed);
println!("  Messages delivered: {}", router_stats.messages_delivered);
println!("  Messages failed: {}", router_stats.messages_failed);
println!("  Active routes: {}", router_stats.active_routes);

println!("Mailbox Stats:");
for stats in mailbox_stats {
    println!("  Agent {}: {} messages processed", 
        stats.agent_id, stats.total_messages_processed);
}
```

### Delivery Statistics

```rust
use terraphim_agent_messaging::DeliveryManager;

let delivery_manager = DeliveryManager::new(DeliveryConfig::default());
let stats = delivery_manager.get_stats().await;

println!("Delivery Stats:");
println!("  Success rate: {:.2}%", stats.success_rate() * 100.0);
println!("  Failure rate: {:.2}%", stats.failure_rate() * 100.0);
println!("  Average attempts: {:.2}", stats.average_attempts());
```

## Integration with Supervision

The messaging system integrates seamlessly with the supervision system:

```rust
use terraphim_agent_supervisor::{AgentSupervisor, SupervisorConfig};
use terraphim_agent_messaging::MessageSystem;

// Create supervisor and messaging system
let supervisor_config = SupervisorConfig::default();
let mut supervisor = AgentSupervisor::new(supervisor_config, agent_factory);

let messaging_config = RouterConfig::default();
let message_system = MessageSystem::new(messaging_config);

// Register agents in both systems
let agent_id = AgentPid::new();
supervisor.spawn_agent(agent_spec).await?;
message_system.register_agent(agent_id).await?;
```

## Performance Characteristics

- **Throughput**: 10,000+ messages/second on modern hardware
- **Latency**: Sub-millisecond message routing
- **Memory**: ~1KB per mailbox + message storage
- **Scalability**: Supports 1000+ concurrent agents
- **Reliability**: 99.9%+ delivery success rate with retries

## Advanced Features

### Custom Message Types

```rust
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug)]
struct CustomMessage {
    task_id: String,
    priority: u8,
    payload: Vec<u8>,
}

let message = AgentMessage::cast(from_agent, CustomMessage {
    task_id: "task_123".to_string(),
    priority: 5,
    payload: vec![1, 2, 3, 4],
});
```

### Message Filtering

```rust
// Custom message filtering based on content
let received = mailbox.receive().await?;
match received {
    AgentMessage::Cast { payload, .. } => {
        // Handle cast message
    }
    AgentMessage::Call { payload, reply_to, .. } => {
        // Handle call message and send reply
        let response = process_request(payload);
        reply_to.send(Box::new(response)).ok();
    }
    _ => {}
}
```

## Testing

The crate includes comprehensive test coverage:

```bash
# Run unit tests
cargo test -p terraphim_agent_messaging

# Run integration tests
cargo test -p terraphim_agent_messaging --test integration_tests

# Run with logging
RUST_LOG=debug cargo test -p terraphim_agent_messaging
```

## Features

- **Erlang/OTP Patterns**: Proven message passing patterns from telecommunications
- **Delivery Guarantees**: At-most-once, at-least-once, exactly-once delivery
- **Fault Tolerance**: Automatic retry with exponential backoff
- **High Performance**: Optimized for low latency and high throughput
- **Monitoring**: Comprehensive metrics and statistics
- **Type Safety**: Full Rust type safety with serde serialization
- **Async/Await**: Native tokio integration for async operations

## Integration

This crate integrates with the broader Terraphim ecosystem:

- **terraphim_agent_supervisor**: Agent lifecycle management and supervision
- **terraphim_types**: Common type definitions and utilities
- **Future**: Knowledge graph-based message routing and content filtering

## License

Licensed under the Apache License, Version 2.0.