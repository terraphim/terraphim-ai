//! Message routing and delivery system
//!
//! Provides message routing between agents with delivery guarantees and retry logic.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use tokio::sync::{Mutex, RwLock};
use tokio::time::{interval, sleep};

use crate::{
    AgentMessage, AgentPid, DeliveryConfig, DeliveryGuarantee, DeliveryManager, MailboxManager,
    MailboxSender, MessageEnvelope, MessagingError, MessagingResult,
};

/// Message router configuration
#[derive(Debug, Clone)]
pub struct RouterConfig {
    pub delivery_config: DeliveryConfig,
    pub retry_interval: Duration,
    pub max_concurrent_deliveries: usize,
    pub enable_metrics: bool,
}

impl Default for RouterConfig {
    fn default() -> Self {
        Self {
            delivery_config: DeliveryConfig::default(),
            retry_interval: Duration::from_secs(5),
            max_concurrent_deliveries: 100,
            enable_metrics: true,
        }
    }
}

/// Router statistics
#[derive(Debug, Default, Clone)]
pub struct RouterStats {
    pub messages_routed: u64,
    pub messages_delivered: u64,
    pub messages_failed: u64,
    pub active_routes: usize,
    pub retry_attempts: u64,
}

/// Message routing trait
#[async_trait]
pub trait MessageRouter: Send + Sync {
    /// Route a message to its destination
    async fn route_message(&self, envelope: MessageEnvelope) -> MessagingResult<()>;

    /// Register an agent for message routing
    async fn register_agent(
        &self,
        agent_id: AgentPid,
        sender: MailboxSender,
    ) -> MessagingResult<()>;

    /// Unregister an agent
    async fn unregister_agent(&self, agent_id: &AgentPid) -> MessagingResult<()>;

    /// Get router statistics
    async fn get_stats(&self) -> RouterStats;

    /// Shutdown the router
    async fn shutdown(&self) -> MessagingResult<()>;
}

/// Default message router implementation
pub struct DefaultMessageRouter {
    config: RouterConfig,
    agents: Arc<RwLock<HashMap<AgentPid, MailboxSender>>>,
    delivery_manager: Arc<DeliveryManager>,
    stats: Arc<Mutex<RouterStats>>,
    shutdown_signal: Arc<tokio::sync::Notify>,
}

impl DefaultMessageRouter {
    /// Create a new message router
    pub fn new(config: RouterConfig) -> Self {
        let delivery_manager = Arc::new(DeliveryManager::new(config.delivery_config.clone()));
        let router = Self {
            config: config.clone(),
            agents: Arc::new(RwLock::new(HashMap::new())),
            delivery_manager,
            stats: Arc::new(Mutex::new(RouterStats::default())),
            shutdown_signal: Arc::new(tokio::sync::Notify::new()),
        };

        // Start retry task
        router.start_retry_task();

        router
    }

    /// Start the retry task for failed messages
    fn start_retry_task(&self) {
        let delivery_manager = Arc::clone(&self.delivery_manager);
        let agents = Arc::clone(&self.agents);
        let stats = Arc::clone(&self.stats);
        let retry_interval = self.config.retry_interval;
        let shutdown_signal = Arc::clone(&self.shutdown_signal);

        tokio::spawn(async move {
            let mut interval = interval(retry_interval);

            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        // Get retry candidates
                        let candidates = delivery_manager.get_retry_candidates().await;

                        for mut envelope in candidates {
                            // Calculate retry delay
                            let delay = delivery_manager.calculate_retry_delay(envelope.attempts);
                            sleep(delay).await;

                            // Attempt retry
                            let agents_guard = agents.read().await;
                            if let Some(sender) = agents_guard.get(&envelope.to) {
                                envelope.increment_attempts();

                                // Mark as in transit
                                if let Err(e) = delivery_manager.mark_in_transit(&envelope.id).await {
                                    log::error!("Failed to mark message {} as in transit: {}", envelope.id, e);
                                    continue;
                                }

                                // Convert envelope to agent message for retry
                                let agent_message = AgentMessage::cast(
                                    envelope.from.clone().unwrap_or_else(|| AgentPid::new()),
                                    envelope.payload.clone()
                                );

                                match sender.send(agent_message).await {
                                    Ok(()) => {
                                        if let Err(e) = delivery_manager.mark_delivered(&envelope.id).await {
                                            log::error!("Failed to mark message {} as delivered: {}", envelope.id, e);
                                        }

                                        // Update stats
                                        {
                                            let mut stats_guard = stats.lock().await;
                                            stats_guard.retry_attempts += 1;
                                            stats_guard.messages_delivered += 1;
                                        }
                                    }
                                    Err(e) => {
                                        if let Err(mark_err) = delivery_manager.mark_failed(&envelope.id, e.to_string()).await {
                                            log::error!("Failed to mark message {} as failed: {}", envelope.id, mark_err);
                                        }

                                        // Update stats
                                        {
                                            let mut stats_guard = stats.lock().await;
                                            stats_guard.retry_attempts += 1;
                                            stats_guard.messages_failed += 1;
                                        }
                                    }
                                }
                            } else {
                                // Agent not found, mark as failed
                                if let Err(e) = delivery_manager.mark_failed(
                                    &envelope.id,
                                    format!("Agent {} not found", envelope.to)
                                ).await {
                                    log::error!("Failed to mark message {} as failed: {}", envelope.id, e);
                                }
                            }
                        }
                    }
                    _ = shutdown_signal.notified() => {
                        log::info!("Retry task shutting down");
                        break;
                    }
                }
            }
        });
    }

    /// Convert message envelope to agent message
    fn envelope_to_agent_message(
        &self,
        envelope: &MessageEnvelope,
    ) -> MessagingResult<AgentMessage> {
        // For now, we'll create a cast message
        // In a real implementation, we'd need to preserve the original message type
        let from = envelope.from.clone().unwrap_or_else(|| AgentPid::new());
        Ok(AgentMessage::cast(from, envelope.payload.clone()))
    }
}

#[async_trait]
impl MessageRouter for DefaultMessageRouter {
    async fn route_message(&self, envelope: MessageEnvelope) -> MessagingResult<()> {
        // Record message for delivery tracking
        self.delivery_manager.record_message(&envelope).await?;

        // Get target agent
        let agents = self.agents.read().await;
        let sender = agents
            .get(&envelope.to)
            .ok_or_else(|| MessagingError::AgentNotFound(envelope.to.clone()))?;

        // Mark as in transit
        self.delivery_manager.mark_in_transit(&envelope.id).await?;

        // Convert envelope to agent message
        let agent_message = self.envelope_to_agent_message(&envelope)?;

        // Send message
        match sender.send(agent_message).await {
            Ok(()) => {
                // Mark as delivered
                self.delivery_manager.mark_delivered(&envelope.id).await?;

                // For at-most-once delivery, also mark as acknowledged
                if self.config.delivery_config.guarantee == DeliveryGuarantee::AtMostOnce {
                    self.delivery_manager
                        .mark_acknowledged(&envelope.id)
                        .await?;
                }

                // Update stats
                {
                    let mut stats = self.stats.lock().await;
                    stats.messages_routed += 1;
                    stats.messages_delivered += 1;
                }

                Ok(())
            }
            Err(e) => {
                // Mark as failed
                self.delivery_manager
                    .mark_failed(&envelope.id, e.to_string())
                    .await?;

                // Update stats
                {
                    let mut stats = self.stats.lock().await;
                    stats.messages_routed += 1;
                    stats.messages_failed += 1;
                }

                Err(e)
            }
        }
    }

    async fn register_agent(
        &self,
        agent_id: AgentPid,
        sender: MailboxSender,
    ) -> MessagingResult<()> {
        let mut agents = self.agents.write().await;

        if agents.contains_key(&agent_id) {
            return Err(MessagingError::DuplicateAgent(agent_id));
        }

        agents.insert(agent_id.clone(), sender);

        // Update stats
        {
            let mut stats = self.stats.lock().await;
            stats.active_routes = agents.len();
        }

        log::info!("Registered agent {} for message routing", agent_id);
        Ok(())
    }

    async fn unregister_agent(&self, agent_id: &AgentPid) -> MessagingResult<()> {
        let mut agents = self.agents.write().await;

        if agents.remove(agent_id).is_none() {
            return Err(MessagingError::AgentNotFound(agent_id.clone()));
        }

        // Update stats
        {
            let mut stats = self.stats.lock().await;
            stats.active_routes = agents.len();
        }

        log::info!("Unregistered agent {} from message routing", agent_id);
        Ok(())
    }

    async fn get_stats(&self) -> RouterStats {
        self.stats.lock().await.clone()
    }

    async fn shutdown(&self) -> MessagingResult<()> {
        log::info!("Shutting down message router");

        // Signal shutdown to background tasks
        self.shutdown_signal.notify_waiters();

        // Clear all routes
        {
            let mut agents = self.agents.write().await;
            agents.clear();
        }

        // Reset stats
        {
            let mut stats = self.stats.lock().await;
            *stats = RouterStats::default();
        }

        Ok(())
    }
}

/// High-level message system that combines routing and mailbox management
pub struct MessageSystem {
    router: Arc<dyn MessageRouter>,
    mailbox_manager: Arc<MailboxManager>,
}

impl MessageSystem {
    /// Create a new message system
    pub fn new(router_config: RouterConfig) -> Self {
        let router = Arc::new(DefaultMessageRouter::new(router_config));
        let mailbox_config = crate::MailboxConfig::default();
        let mailbox_manager = Arc::new(MailboxManager::new(mailbox_config));

        Self {
            router,
            mailbox_manager,
        }
    }

    /// Register an agent in the message system
    pub async fn register_agent(&self, agent_id: AgentPid) -> MessagingResult<()> {
        // Create mailbox
        let sender = self
            .mailbox_manager
            .create_mailbox(agent_id.clone())
            .await?;

        // Register with router
        self.router.register_agent(agent_id, sender).await?;

        Ok(())
    }

    /// Unregister an agent from the message system
    pub async fn unregister_agent(&self, agent_id: &AgentPid) -> MessagingResult<()> {
        // Unregister from router
        self.router.unregister_agent(agent_id).await?;

        // Remove mailbox
        self.mailbox_manager.remove_mailbox(agent_id).await?;

        Ok(())
    }

    /// Send a message through the system
    pub async fn send_message(&self, envelope: MessageEnvelope) -> MessagingResult<()> {
        self.router.route_message(envelope).await
    }

    /// Get a mailbox for an agent (for receiving messages)
    pub async fn get_mailbox(&self, agent_id: &AgentPid) -> Option<crate::AgentMailbox> {
        self.mailbox_manager.get_mailbox(agent_id).await
    }

    /// Get system statistics
    pub async fn get_stats(&self) -> (RouterStats, Vec<crate::MailboxStats>) {
        let router_stats = self.router.get_stats().await;
        let mailbox_stats = self.mailbox_manager.get_all_stats().await;
        (router_stats, mailbox_stats)
    }

    /// Shutdown the message system
    pub async fn shutdown(&self) -> MessagingResult<()> {
        self.router.shutdown().await?;

        // Shutdown all mailboxes
        let agents = self.mailbox_manager.list_agents().await;
        for agent_id in agents {
            let _ = self.mailbox_manager.remove_mailbox(&agent_id).await;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DeliveryOptions, MessagePriority};

    #[tokio::test]
    async fn test_router_registration() {
        let config = RouterConfig::default();
        let router = DefaultMessageRouter::new(config);

        let agent_id = AgentPid::new();
        let mailbox_config = crate::MailboxConfig::default();
        let mailbox = crate::AgentMailbox::new(agent_id.clone(), mailbox_config);
        let sender = mailbox.sender();

        // Register agent
        router
            .register_agent(agent_id.clone(), sender)
            .await
            .unwrap();

        let stats = router.get_stats().await;
        assert_eq!(stats.active_routes, 1);

        // Unregister agent
        router.unregister_agent(&agent_id).await.unwrap();

        let stats = router.get_stats().await;
        assert_eq!(stats.active_routes, 0);
    }

    #[tokio::test]
    async fn test_message_routing() {
        let config = RouterConfig::default();
        let router = DefaultMessageRouter::new(config);

        let agent_id = AgentPid::new();
        let mailbox_config = crate::MailboxConfig::default();
        let mailbox = crate::AgentMailbox::new(agent_id.clone(), mailbox_config);
        let sender = mailbox.sender();

        // Register agent
        router
            .register_agent(agent_id.clone(), sender)
            .await
            .unwrap();

        // Create message envelope
        let envelope = MessageEnvelope::new(
            agent_id.clone(),
            "test_message".to_string(),
            serde_json::json!({"data": "test"}),
            DeliveryOptions::default(),
        );

        // Route message
        router.route_message(envelope).await.unwrap();

        // Check stats
        let stats = router.get_stats().await;
        assert_eq!(stats.messages_routed, 1);
        assert_eq!(stats.messages_delivered, 1);

        // Check message was received
        let received = mailbox.receive().await.unwrap();
        assert!(received.from().is_some());
    }

    #[tokio::test]
    async fn test_message_system() {
        let config = RouterConfig::default();
        let system = MessageSystem::new(config);

        let agent_id = AgentPid::new();

        // Register agent
        system.register_agent(agent_id.clone()).await.unwrap();

        // Send message
        let envelope = MessageEnvelope::new(
            agent_id.clone(),
            "test_message".to_string(),
            serde_json::json!({"data": "test"}),
            DeliveryOptions::default(),
        );

        system.send_message(envelope).await.unwrap();

        // Check stats (message should be delivered)
        let (router_stats, mailbox_stats) = system.get_stats().await;
        assert_eq!(router_stats.messages_delivered, 1);
        assert_eq!(mailbox_stats.len(), 1);
        // Note: We can't easily test message reception due to mailbox cloning issues
        // In a real implementation, we'd use proper handles or references
    }

    #[tokio::test]
    async fn test_agent_not_found() {
        let config = RouterConfig::default();
        let router = DefaultMessageRouter::new(config);

        let agent_id = AgentPid::new();

        // Try to route message to non-existent agent
        let envelope = MessageEnvelope::new(
            agent_id.clone(),
            "test_message".to_string(),
            serde_json::json!({"data": "test"}),
            DeliveryOptions::default(),
        );

        let result = router.route_message(envelope).await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            MessagingError::AgentNotFound(_)
        ));
    }

    #[tokio::test]
    async fn test_duplicate_registration() {
        let config = RouterConfig::default();
        let router = DefaultMessageRouter::new(config);

        let agent_id = AgentPid::new();
        let mailbox_config = crate::MailboxConfig::default();
        let mailbox = crate::AgentMailbox::new(agent_id.clone(), mailbox_config);
        let sender = mailbox.sender();

        // Register agent
        router
            .register_agent(agent_id.clone(), sender.clone())
            .await
            .unwrap();

        // Try to register again
        let result = router.register_agent(agent_id, sender).await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            MessagingError::DuplicateAgent(_)
        ));
    }
}
