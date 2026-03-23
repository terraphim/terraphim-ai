# crates/terraphim_agent_messaging

## Purpose
Erlang-style agent mailbox system with delivery guarantees, priority messaging, and cross-agent routing.

## Status: Production-Ready
- ~2,174 LOC (non-test)
- 26/26 tests passing (100%)

## Key Types

### AgentMailbox
Unbounded message queues with delivery guarantees using tokio::mpsc channels.
- `new()` - Creates unbounded channel pair
- `send()` / `receive()` - Async message passing
- `receive_timeout()` - Timed receive
- `try_receive()` - Non-blocking receive
- Statistics tracking (total messages, queue size, processing time)

### AgentMessage (enum)
Erlang-style patterns:
- `Call` - Synchronous request/response
- `Cast` - Fire-and-forget
- `Info` - Informational
- `Reply` - Response to Call
- `Ack` - Acknowledgment

### MessageRouter
Cross-agent message routing with timeout handling.

### DeliveryManager
At-least-once delivery with retry logic and deduplication.

### MessagePriority (enum)
Low, Normal, High, Critical

### MailboxManager
Multi-agent mailbox coordination.

### MailboxSender
Cloneable sender handles for fire-and-forget pattern.

## Integration Points
- Used by terraphim_multi_agent for agent communication
- Used by terraphim_agent_supervisor for health checks
- Bounded mailbox support with backpressure
- Configurable delivery options (timeouts, retries, acknowledgments)

## Relevance to TinyClaw Rebuild
Maps to PicoClaw's MessageBus (Go channels) but with stronger guarantees. The mailbox system can replace PicoClaw's simple buffered channels while adding delivery guarantees, priority routing, and deduplication.
