//! @mention routing for agent coordination
//!
//! This module enables agents to route messages to other agents via @mentions
//! in their output, enabling decentralized coordination without a central orchestrator.

use std::collections::HashMap;
use tokio::sync::mpsc;

use terraphim_types::capability::ProcessId;

use crate::{AgentHandle, AgentSpawner, SpawnerError};
use crate::output::OutputEvent;

/// Router for @mention messages between agents
#[derive(Debug)]
pub struct MentionRouter {
    /// Map of agent IDs to their process IDs
    agents: HashMap<String, ProcessId>,
    /// Event receiver for @mentions
    mention_receiver: mpsc::Receiver<MentionEvent>,
    /// Event sender (cloned to agents)
    mention_sender: mpsc::Sender<MentionEvent>,
}

/// An @mention event
#[derive(Debug, Clone)]
pub struct MentionEvent {
    /// Source agent/process
    pub from: ProcessId,
    /// Target agent (without @)
    pub target: String,
    /// Message content
    pub message: String,
}

impl MentionRouter {
    /// Create a new mention router
    pub fn new() -> Self {
        let (mention_sender, mention_receiver) = mpsc::channel(100);
        
        Self {
            agents: HashMap::new(),
            mention_receiver,
            mention_sender,
        }
    }
    
    /// Register an agent with the router
    pub fn register_agent(
        &mut self,
        agent_id: String,
        process_id: ProcessId,
    ) {
        self.agents.insert(agent_id, process_id);
    }
    
    /// Get the sender for agents to use
    pub fn sender(&self) -> mpsc::Sender<MentionEvent> {
        self.mention_sender.clone()
    }
    
    /// Route mentions to their targets
    pub async fn route_mentions(
        &mut self,
        spawner: &AgentSpawner,
    ) -> Result<(), SpawnerError> {
        while let Some(event) = self.mention_receiver.recv().await {
            log::info!(
                "Routing mention from {} to @{}: {}",
                event.from,
                event.target,
                event.message
            );
            
            // Check if target agent is registered
            if let Some(&target_pid) = self.agents.get(&event.target) {
                log::debug!("Target agent {} found with PID {}", event.target, target_pid);
                // In a real implementation, this would forward the message
                // to the target agent's input channel
            } else {
                log::warn!("Target agent @{} not found, message dropped", event.target);
                // Could spawn a new agent here if configured
            }
        }
        
        Ok(())
    }
}

impl Default for MentionRouter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mention_router() {
        let mut router = MentionRouter::new();
        let sender = router.sender();
        
        // Register an agent
        router.register_agent("kimiko".to_string(), ProcessId::new());
        
        // Send a mention
        let event = MentionEvent {
            from: ProcessId::new(),
            target: "kimiko".to_string(),
            message: "Hello!".to_string(),
        };
        
        sender.send(event).await.unwrap();
        
        // Route should receive it
        let received = router.mention_receiver.recv().await;
        assert!(received.is_some());
        let received = received.unwrap();
        assert_eq!(received.target, "kimiko");
    }
}
