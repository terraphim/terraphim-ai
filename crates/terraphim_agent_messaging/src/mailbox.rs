//! Agent mailbox implementation
//!
//! Provides unbounded message queues with delivery guarantees for agents.

use std::sync::Arc;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc, Mutex, Notify};

use crate::{AgentMessage, AgentPid, MessagingError, MessagingResult};

/// Configuration for agent mailboxes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MailboxConfig {
    /// Maximum number of messages in the mailbox (0 = unbounded)
    pub max_messages: usize,
    /// Whether to preserve message order
    pub preserve_order: bool,
    /// Whether to enable message persistence
    pub enable_persistence: bool,
    /// Mailbox statistics collection interval
    pub stats_interval: std::time::Duration,
}

impl Default for MailboxConfig {
    fn default() -> Self {
        Self {
            max_messages: 0, // Unbounded by default (Erlang-style)
            preserve_order: true,
            enable_persistence: false,
            stats_interval: std::time::Duration::from_secs(60),
        }
    }
}

/// Mailbox statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MailboxStats {
    pub agent_id: AgentPid,
    pub total_messages_received: u64,
    pub total_messages_processed: u64,
    pub current_queue_size: usize,
    pub max_queue_size_reached: usize,
    pub last_message_received: Option<DateTime<Utc>>,
    pub last_message_processed: Option<DateTime<Utc>>,
    pub average_processing_time: std::time::Duration,
}

impl MailboxStats {
    pub fn new(agent_id: AgentPid) -> Self {
        Self {
            agent_id,
            total_messages_received: 0,
            total_messages_processed: 0,
            current_queue_size: 0,
            max_queue_size_reached: 0,
            last_message_received: None,
            last_message_processed: None,
            average_processing_time: std::time::Duration::ZERO,
        }
    }

    pub fn record_message_received(&mut self) {
        self.total_messages_received += 1;
        self.current_queue_size += 1;
        self.max_queue_size_reached = self.max_queue_size_reached.max(self.current_queue_size);
        self.last_message_received = Some(Utc::now());
    }

    pub fn record_message_processed(&mut self, processing_time: std::time::Duration) {
        self.total_messages_processed += 1;
        self.current_queue_size = self.current_queue_size.saturating_sub(1);
        self.last_message_processed = Some(Utc::now());

        // Update average processing time (simple moving average)
        if self.total_messages_processed == 1 {
            self.average_processing_time = processing_time;
        } else {
            let total_time = self.average_processing_time.as_nanos() as f64
                * (self.total_messages_processed - 1) as f64;
            let new_average = (total_time + processing_time.as_nanos() as f64)
                / self.total_messages_processed as f64;
            self.average_processing_time = std::time::Duration::from_nanos(new_average as u64);
        }
    }
}

/// Agent mailbox for message queuing and delivery
pub struct AgentMailbox {
    agent_id: AgentPid,
    config: MailboxConfig,
    sender: mpsc::UnboundedSender<AgentMessage>,
    receiver: Arc<Mutex<mpsc::UnboundedReceiver<AgentMessage>>>,
    stats: Arc<Mutex<MailboxStats>>,
    shutdown_notify: Arc<Notify>,
}

impl AgentMailbox {
    /// Create a new agent mailbox
    pub fn new(agent_id: AgentPid, config: MailboxConfig) -> Self {
        let (sender, receiver) = mpsc::unbounded_channel();
        let stats = MailboxStats::new(agent_id.clone());

        Self {
            agent_id: agent_id.clone(),
            config,
            sender,
            receiver: Arc::new(Mutex::new(receiver)),
            stats: Arc::new(Mutex::new(stats)),
            shutdown_notify: Arc::new(Notify::new()),
        }
    }

    /// Get the agent ID
    pub fn agent_id(&self) -> &AgentPid {
        &self.agent_id
    }

    /// Get mailbox configuration
    pub fn config(&self) -> &MailboxConfig {
        &self.config
    }

    /// Send a message to this mailbox
    pub async fn send(&self, message: AgentMessage) -> MessagingResult<()> {
        // Check if mailbox is full (if bounded)
        if self.config.max_messages > 0 {
            let stats = self.stats.lock().await;
            if stats.current_queue_size >= self.config.max_messages {
                return Err(MessagingError::MailboxFull(self.agent_id.clone()));
            }
        }

        // Send message
        self.sender
            .send(message)
            .map_err(|_| MessagingError::ChannelClosed(self.agent_id.clone()))?;

        // Update statistics
        {
            let mut stats = self.stats.lock().await;
            stats.record_message_received();
        }

        Ok(())
    }

    /// Receive a message from this mailbox
    pub async fn receive(&self) -> MessagingResult<AgentMessage> {
        let start_time = std::time::Instant::now();

        let message = {
            let mut receiver = self.receiver.lock().await;
            receiver
                .recv()
                .await
                .ok_or_else(|| MessagingError::ChannelClosed(self.agent_id.clone()))?
        };

        // Update statistics
        {
            let mut stats = self.stats.lock().await;
            stats.record_message_processed(start_time.elapsed());
        }

        Ok(message)
    }

    /// Try to receive a message without blocking
    pub async fn try_receive(&self) -> MessagingResult<Option<AgentMessage>> {
        let start_time = std::time::Instant::now();

        let message = {
            let mut receiver = self.receiver.lock().await;
            match receiver.try_recv() {
                Ok(message) => Some(message),
                Err(mpsc::error::TryRecvError::Empty) => None,
                Err(mpsc::error::TryRecvError::Disconnected) => {
                    return Err(MessagingError::ChannelClosed(self.agent_id.clone()));
                }
            }
        };

        if let Some(_message) = &message {
            // Update statistics
            let mut stats = self.stats.lock().await;
            stats.record_message_processed(start_time.elapsed());
        }

        Ok(message)
    }

    /// Receive a message with timeout
    pub async fn receive_timeout(
        &self,
        timeout: std::time::Duration,
    ) -> MessagingResult<AgentMessage> {
        let start_time = std::time::Instant::now();

        let message = tokio::time::timeout(timeout, async {
            let mut receiver = self.receiver.lock().await;
            receiver
                .recv()
                .await
                .ok_or_else(|| MessagingError::ChannelClosed(self.agent_id.clone()))
        })
        .await
        .map_err(|_| MessagingError::MessageTimeout(self.agent_id.clone()))??;

        // Update statistics
        {
            let mut stats = self.stats.lock().await;
            stats.record_message_processed(start_time.elapsed());
        }

        Ok(message)
    }

    /// Get current mailbox statistics
    pub async fn stats(&self) -> MailboxStats {
        self.stats.lock().await.clone()
    }

    /// Check if mailbox is empty
    pub async fn is_empty(&self) -> bool {
        let stats = self.stats.lock().await;
        stats.current_queue_size == 0
    }

    /// Get current queue size
    pub async fn queue_size(&self) -> usize {
        let stats = self.stats.lock().await;
        stats.current_queue_size
    }

    /// Close the mailbox
    pub fn close(&self) {
        // Dropping the sender will close the channel
        // The receiver will get None on next recv()
        self.shutdown_notify.notify_waiters();
    }

    /// Wait for mailbox shutdown
    pub async fn wait_for_shutdown(&self) {
        self.shutdown_notify.notified().await;
    }

    /// Create a sender handle for this mailbox
    pub fn sender(&self) -> MailboxSender {
        MailboxSender {
            agent_id: self.agent_id.clone(),
            sender: self.sender.clone(),
        }
    }
}

/// Sender handle for a mailbox
#[derive(Clone)]
pub struct MailboxSender {
    agent_id: AgentPid,
    sender: mpsc::UnboundedSender<AgentMessage>,
}

impl MailboxSender {
    /// Send a message through this sender
    pub async fn send(&self, message: AgentMessage) -> MessagingResult<()> {
        self.sender
            .send(message)
            .map_err(|_| MessagingError::ChannelClosed(self.agent_id.clone()))
    }

    /// Get the target agent ID
    pub fn agent_id(&self) -> &AgentPid {
        &self.agent_id
    }

    /// Check if the sender is closed
    pub fn is_closed(&self) -> bool {
        self.sender.is_closed()
    }
}

/// Mailbox manager for handling multiple agent mailboxes
pub struct MailboxManager {
    mailboxes: Arc<Mutex<std::collections::HashMap<AgentPid, AgentMailbox>>>,
    default_config: MailboxConfig,
}

impl MailboxManager {
    /// Create a new mailbox manager
    pub fn new(default_config: MailboxConfig) -> Self {
        Self {
            mailboxes: Arc::new(Mutex::new(std::collections::HashMap::new())),
            default_config,
        }
    }

    /// Create a mailbox for an agent
    pub async fn create_mailbox(&self, agent_id: AgentPid) -> MessagingResult<MailboxSender> {
        let mut mailboxes = self.mailboxes.lock().await;

        if mailboxes.contains_key(&agent_id) {
            return Err(MessagingError::DuplicateAgent(agent_id));
        }

        let mailbox = AgentMailbox::new(agent_id.clone(), self.default_config.clone());
        let sender = mailbox.sender();

        mailboxes.insert(agent_id, mailbox);

        Ok(sender)
    }

    /// Get a mailbox sender for an agent
    pub async fn get_mailbox_sender(&self, agent_id: &AgentPid) -> Option<MailboxSender> {
        let mailboxes = self.mailboxes.lock().await;
        mailboxes.get(agent_id).map(|mailbox| mailbox.sender())
    }

    /// Get a mailbox for an agent (for receiving - this creates a new receiver)
    pub async fn get_mailbox(&self, agent_id: &AgentPid) -> Option<AgentMailbox> {
        let mailboxes = self.mailboxes.lock().await;
        // Note: This clones the entire mailbox, which creates a new receiver
        // This is not ideal for production - we'd want a different approach
        mailboxes.get(agent_id).cloned()
    }

    /// Remove a mailbox
    pub async fn remove_mailbox(&self, agent_id: &AgentPid) -> MessagingResult<()> {
        let mut mailboxes = self.mailboxes.lock().await;

        if let Some(mailbox) = mailboxes.remove(agent_id) {
            mailbox.close();
            Ok(())
        } else {
            Err(MessagingError::AgentNotFound(agent_id.clone()))
        }
    }

    /// Get all agent IDs with mailboxes
    pub async fn list_agents(&self) -> Vec<AgentPid> {
        let mailboxes = self.mailboxes.lock().await;
        mailboxes.keys().cloned().collect()
    }

    /// Get statistics for all mailboxes
    pub async fn get_all_stats(&self) -> Vec<MailboxStats> {
        let mailboxes = self.mailboxes.lock().await;
        let mut stats = Vec::new();

        for mailbox in mailboxes.values() {
            stats.push(mailbox.stats().await);
        }

        stats
    }
}

// Note: We can't derive Clone for AgentMailbox because of the receiver
// This is intentional - each mailbox should have a single receiver
impl Clone for AgentMailbox {
    fn clone(&self) -> Self {
        // Create a new mailbox with the same configuration
        // This is used by the MailboxManager for get_mailbox
        // In production, we'd want to return handles instead
        AgentMailbox::new(self.agent_id.clone(), self.config.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[tokio::test]
    async fn test_mailbox_creation() {
        let agent_id = AgentPid::new();
        let config = MailboxConfig::default();
        let mailbox = AgentMailbox::new(agent_id.clone(), config);

        assert_eq!(mailbox.agent_id(), &agent_id);
        assert!(mailbox.is_empty().await);
        assert_eq!(mailbox.queue_size().await, 0);
    }

    #[tokio::test]
    async fn test_message_send_receive() {
        let agent_id = AgentPid::new();
        let config = MailboxConfig::default();
        let mailbox = AgentMailbox::new(agent_id.clone(), config);

        // Send a message
        let message = AgentMessage::cast(agent_id.clone(), "test payload");
        mailbox.send(message).await.unwrap();

        assert!(!mailbox.is_empty().await);
        assert_eq!(mailbox.queue_size().await, 1);

        // Receive the message
        let received = mailbox.receive().await.unwrap();
        assert_eq!(received.from(), Some(&agent_id));

        assert!(mailbox.is_empty().await);
        assert_eq!(mailbox.queue_size().await, 0);
    }

    #[tokio::test]
    async fn test_mailbox_stats() {
        let agent_id = AgentPid::new();
        let config = MailboxConfig::default();
        let mailbox = AgentMailbox::new(agent_id.clone(), config);

        // Send and receive a message
        let message = AgentMessage::cast(agent_id.clone(), "test");
        mailbox.send(message).await.unwrap();
        let _received = mailbox.receive().await.unwrap();

        let stats = mailbox.stats().await;
        assert_eq!(stats.total_messages_received, 1);
        assert_eq!(stats.total_messages_processed, 1);
        assert_eq!(stats.current_queue_size, 0);
        assert!(stats.last_message_received.is_some());
        assert!(stats.last_message_processed.is_some());
    }

    #[tokio::test]
    async fn test_mailbox_timeout() {
        let agent_id = AgentPid::new();
        let config = MailboxConfig::default();
        let mailbox = AgentMailbox::new(agent_id.clone(), config);

        // Try to receive with timeout (should timeout)
        let result = mailbox.receive_timeout(Duration::from_millis(100)).await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            MessagingError::MessageTimeout(_)
        ));
    }

    #[tokio::test]
    async fn test_mailbox_manager() {
        let config = MailboxConfig::default();
        let manager = MailboxManager::new(config);

        let agent_id = AgentPid::new();

        // Create mailbox
        let sender = manager.create_mailbox(agent_id.clone()).await.unwrap();
        assert_eq!(sender.agent_id(), &agent_id);

        // Check agent is listed
        let agents = manager.list_agents().await;
        assert_eq!(agents.len(), 1);
        assert_eq!(agents[0], agent_id);

        // Remove mailbox
        manager.remove_mailbox(&agent_id).await.unwrap();
        let agents = manager.list_agents().await;
        assert_eq!(agents.len(), 0);
    }

    #[tokio::test]
    async fn test_bounded_mailbox() {
        let agent_id = AgentPid::new();
        let mut config = MailboxConfig::default();
        config.max_messages = 2; // Limit to 2 messages

        let mailbox = AgentMailbox::new(agent_id.clone(), config);

        // Send messages up to limit
        let msg1 = AgentMessage::cast(agent_id.clone(), "msg1");
        let msg2 = AgentMessage::cast(agent_id.clone(), "msg2");

        mailbox.send(msg1).await.unwrap();
        mailbox.send(msg2).await.unwrap();

        // Third message should fail
        let msg3 = AgentMessage::cast(agent_id.clone(), "msg3");
        let result = mailbox.send(msg3).await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            MessagingError::MailboxFull(_)
        ));
    }
}
