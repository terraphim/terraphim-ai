//! Shared entry point for surfaces that feed the agent loop.

use crate::bus::{InboundMessage, MessageBus};
use std::sync::Arc;

/// Dispatch an inbound message into the same bus consumed by
/// [`crate::agent::agent_loop::ToolCallingLoop`].
pub async fn dispatch_to_agent_loop(
    bus: &Arc<MessageBus>,
    msg: InboundMessage,
) -> anyhow::Result<()> {
    bus.inbound_sender().send(msg).await?;
    Ok(())
}
