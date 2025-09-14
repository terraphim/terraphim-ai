//! Message delivery guarantees and reliability features

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::sync::{Mutex, RwLock};
use tokio::time::interval;

use crate::{AgentPid, MessageEnvelope, MessageId, MessagingError, MessagingResult};

/// Delivery guarantee levels
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeliveryGuarantee {
    /// At most once - fire and forget
    AtMostOnce,
    /// At least once - retry until acknowledged
    AtLeastOnce,
    /// Exactly once - deduplicated delivery
    ExactlyOnce,
}

impl Default for DeliveryGuarantee {
    fn default() -> Self {
        DeliveryGuarantee::AtLeastOnce
    }
}

/// Delivery status for tracking message delivery
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeliveryStatus {
    Pending,
    InTransit,
    Delivered,
    Acknowledged,
    Failed(String),
    Expired,
}

/// Delivery record for tracking message delivery
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeliveryRecord {
    pub message_id: MessageId,
    pub from: Option<AgentPid>,
    pub to: AgentPid,
    pub status: DeliveryStatus,
    pub attempts: u32,
    pub created_at: DateTime<Utc>,
    pub last_attempt: Option<DateTime<Utc>>,
    pub delivered_at: Option<DateTime<Utc>>,
    pub acknowledged_at: Option<DateTime<Utc>>,
    pub error_message: Option<String>,
}

impl DeliveryRecord {
    pub fn new(message_id: MessageId, from: Option<AgentPid>, to: AgentPid) -> Self {
        Self {
            message_id,
            from,
            to,
            status: DeliveryStatus::Pending,
            attempts: 0,
            created_at: Utc::now(),
            last_attempt: None,
            delivered_at: None,
            acknowledged_at: None,
            error_message: None,
        }
    }

    pub fn mark_in_transit(&mut self) {
        self.status = DeliveryStatus::InTransit;
        self.attempts += 1;
        self.last_attempt = Some(Utc::now());
    }

    pub fn mark_delivered(&mut self) {
        self.status = DeliveryStatus::Delivered;
        self.delivered_at = Some(Utc::now());
    }

    pub fn mark_acknowledged(&mut self) {
        self.status = DeliveryStatus::Acknowledged;
        self.acknowledged_at = Some(Utc::now());
    }

    pub fn mark_failed(&mut self, error: String) {
        self.status = DeliveryStatus::Failed(error.clone());
        self.error_message = Some(error);
    }

    pub fn mark_expired(&mut self) {
        self.status = DeliveryStatus::Expired;
    }

    pub fn is_final_state(&self) -> bool {
        matches!(
            self.status,
            DeliveryStatus::Acknowledged | DeliveryStatus::Failed(_) | DeliveryStatus::Expired
        )
    }
}

/// Configuration for delivery guarantees
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeliveryConfig {
    pub guarantee: DeliveryGuarantee,
    pub max_retries: u32,
    pub retry_delay: Duration,
    pub retry_backoff_multiplier: f64,
    pub max_retry_delay: Duration,
    pub acknowledgment_timeout: Duration,
    pub cleanup_interval: Duration,
    pub max_delivery_records: usize,
}

impl Default for DeliveryConfig {
    fn default() -> Self {
        Self {
            guarantee: DeliveryGuarantee::AtLeastOnce,
            max_retries: 3,
            retry_delay: Duration::from_millis(100),
            retry_backoff_multiplier: 2.0,
            max_retry_delay: Duration::from_secs(30),
            acknowledgment_timeout: Duration::from_secs(30),
            cleanup_interval: Duration::from_secs(300), // 5 minutes
            max_delivery_records: 10000,
        }
    }
}

/// Delivery manager for handling message delivery guarantees
pub struct DeliveryManager {
    config: DeliveryConfig,
    delivery_records: Arc<RwLock<HashMap<MessageId, DeliveryRecord>>>,
    pending_deliveries: Arc<Mutex<HashMap<MessageId, MessageEnvelope>>>,
    deduplication_cache: Arc<RwLock<HashMap<MessageId, DateTime<Utc>>>>,
}

impl DeliveryManager {
    /// Create a new delivery manager
    pub fn new(config: DeliveryConfig) -> Self {
        let manager = Self {
            config,
            delivery_records: Arc::new(RwLock::new(HashMap::new())),
            pending_deliveries: Arc::new(Mutex::new(HashMap::new())),
            deduplication_cache: Arc::new(RwLock::new(HashMap::new())),
        };

        // Start background cleanup task
        manager.start_cleanup_task();

        manager
    }

    /// Record a message for delivery tracking
    pub async fn record_message(&self, envelope: &MessageEnvelope) -> MessagingResult<()> {
        let message_id = envelope.id.clone();

        // Check for duplicates if exactly-once delivery is required
        if self.config.guarantee == DeliveryGuarantee::ExactlyOnce {
            let cache = self.deduplication_cache.read().await;
            if cache.contains_key(&message_id) {
                return Ok(()); // Already processed
            }
        }

        // Create delivery record
        let record = DeliveryRecord::new(
            message_id.clone(),
            envelope.from.clone(),
            envelope.to.clone(),
        );

        // Store record and envelope
        {
            let mut records = self.delivery_records.write().await;
            records.insert(message_id.clone(), record);
        }

        {
            let mut pending = self.pending_deliveries.lock().await;
            pending.insert(message_id.clone(), envelope.clone());
        }

        // Add to deduplication cache if needed
        if self.config.guarantee == DeliveryGuarantee::ExactlyOnce {
            let mut cache = self.deduplication_cache.write().await;
            cache.insert(message_id, Utc::now());
        }

        Ok(())
    }

    /// Mark a message as in transit
    pub async fn mark_in_transit(&self, message_id: &MessageId) -> MessagingResult<()> {
        let mut records = self.delivery_records.write().await;
        if let Some(record) = records.get_mut(message_id) {
            record.mark_in_transit();
            Ok(())
        } else {
            Err(MessagingError::InvalidMessage(format!(
                "Message {} not found in delivery records",
                message_id
            )))
        }
    }

    /// Mark a message as delivered
    pub async fn mark_delivered(&self, message_id: &MessageId) -> MessagingResult<()> {
        let mut records = self.delivery_records.write().await;
        if let Some(record) = records.get_mut(message_id) {
            record.mark_delivered();

            // For at-most-once delivery, we're done
            if self.config.guarantee == DeliveryGuarantee::AtMostOnce {
                record.mark_acknowledged();
            }

            Ok(())
        } else {
            Err(MessagingError::InvalidMessage(format!(
                "Message {} not found in delivery records",
                message_id
            )))
        }
    }

    /// Mark a message as acknowledged
    pub async fn mark_acknowledged(&self, message_id: &MessageId) -> MessagingResult<()> {
        let mut records = self.delivery_records.write().await;
        if let Some(record) = records.get_mut(message_id) {
            record.mark_acknowledged();
            Ok(())
        } else {
            Err(MessagingError::InvalidMessage(format!(
                "Message {} not found in delivery records",
                message_id
            )))
        }
    }

    /// Mark a message as failed
    pub async fn mark_failed(&self, message_id: &MessageId, error: String) -> MessagingResult<()> {
        let mut records = self.delivery_records.write().await;
        if let Some(record) = records.get_mut(message_id) {
            record.mark_failed(error);
            Ok(())
        } else {
            Err(MessagingError::InvalidMessage(format!(
                "Message {} not found in delivery records",
                message_id
            )))
        }
    }

    /// Get messages that need retry
    pub async fn get_retry_candidates(&self) -> Vec<MessageEnvelope> {
        let mut candidates = Vec::new();
        let records = self.delivery_records.read().await;
        let pending = self.pending_deliveries.lock().await;

        for (message_id, record) in records.iter() {
            if self.should_retry(record) {
                if let Some(envelope) = pending.get(message_id) {
                    let mut retry_envelope = envelope.clone();
                    retry_envelope.attempts = record.attempts;
                    candidates.push(retry_envelope);
                }
            }
        }

        candidates
    }

    /// Check if a message should be retried
    fn should_retry(&self, record: &DeliveryRecord) -> bool {
        match record.status {
            DeliveryStatus::Failed(_) => record.attempts < self.config.max_retries,
            DeliveryStatus::InTransit => {
                // Check if acknowledgment timeout has passed
                if let Some(last_attempt) = record.last_attempt {
                    let elapsed = Utc::now() - last_attempt;
                    elapsed.to_std().unwrap_or(Duration::ZERO) > self.config.acknowledgment_timeout
                } else {
                    false
                }
            }
            _ => false,
        }
    }

    /// Calculate retry delay with exponential backoff
    pub fn calculate_retry_delay(&self, attempts: u32) -> Duration {
        let base_delay = self.config.retry_delay.as_millis() as f64;
        let multiplier = self.config.retry_backoff_multiplier.powi(attempts as i32);
        let delay_ms = (base_delay * multiplier) as u64;

        let delay = Duration::from_millis(delay_ms);
        std::cmp::min(delay, self.config.max_retry_delay)
    }

    /// Get delivery statistics
    pub async fn get_stats(&self) -> DeliveryStats {
        let records = self.delivery_records.read().await;
        let mut stats = DeliveryStats::default();

        for record in records.values() {
            stats.total_messages += 1;

            match &record.status {
                DeliveryStatus::Pending => stats.pending += 1,
                DeliveryStatus::InTransit => stats.in_transit += 1,
                DeliveryStatus::Delivered => stats.delivered += 1,
                DeliveryStatus::Acknowledged => stats.acknowledged += 1,
                DeliveryStatus::Failed(_) => stats.failed += 1,
                DeliveryStatus::Expired => stats.expired += 1,
            }

            stats.total_attempts += record.attempts as u64;
        }

        stats
    }

    /// Get delivery record for a message
    pub async fn get_delivery_record(&self, message_id: &MessageId) -> Option<DeliveryRecord> {
        let records = self.delivery_records.read().await;
        records.get(message_id).cloned()
    }

    /// Start background cleanup task
    fn start_cleanup_task(&self) {
        let records = Arc::clone(&self.delivery_records);
        let pending = Arc::clone(&self.pending_deliveries);
        let dedup_cache = Arc::clone(&self.deduplication_cache);
        let cleanup_interval = self.config.cleanup_interval;
        let max_records = self.config.max_delivery_records;

        tokio::spawn(async move {
            let mut interval = interval(cleanup_interval);

            loop {
                interval.tick().await;

                // Clean up completed delivery records
                {
                    let mut records_guard = records.write().await;
                    let mut pending_guard = pending.lock().await;

                    // Remove completed records
                    let mut to_remove = Vec::new();
                    for (message_id, record) in records_guard.iter() {
                        if record.is_final_state() {
                            // Keep records for a while for debugging, but limit total count
                            if records_guard.len() > max_records {
                                to_remove.push(message_id.clone());
                            }
                        }
                    }

                    for message_id in to_remove {
                        records_guard.remove(&message_id);
                        pending_guard.remove(&message_id);
                    }
                }

                // Clean up old deduplication cache entries
                {
                    let mut cache_guard = dedup_cache.write().await;
                    let cutoff = Utc::now() - chrono::Duration::hours(24); // Keep for 24 hours

                    cache_guard.retain(|_, timestamp| *timestamp > cutoff);
                }
            }
        });
    }
}

/// Delivery statistics
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct DeliveryStats {
    pub total_messages: u64,
    pub pending: u64,
    pub in_transit: u64,
    pub delivered: u64,
    pub acknowledged: u64,
    pub failed: u64,
    pub expired: u64,
    pub total_attempts: u64,
}

impl DeliveryStats {
    pub fn success_rate(&self) -> f64 {
        if self.total_messages == 0 {
            0.0
        } else {
            (self.acknowledged as f64) / (self.total_messages as f64)
        }
    }

    pub fn failure_rate(&self) -> f64 {
        if self.total_messages == 0 {
            0.0
        } else {
            (self.failed as f64) / (self.total_messages as f64)
        }
    }

    pub fn average_attempts(&self) -> f64 {
        if self.total_messages == 0 {
            0.0
        } else {
            (self.total_attempts as f64) / (self.total_messages as f64)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DeliveryOptions, MessagePriority};

    #[tokio::test]
    async fn test_delivery_record_lifecycle() {
        let message_id = MessageId::new();
        let from = AgentPid::new();
        let to = AgentPid::new();

        let mut record = DeliveryRecord::new(message_id.clone(), Some(from), to);

        assert_eq!(record.status, DeliveryStatus::Pending);
        assert_eq!(record.attempts, 0);
        assert!(!record.is_final_state());

        record.mark_in_transit();
        assert_eq!(record.status, DeliveryStatus::InTransit);
        assert_eq!(record.attempts, 1);
        assert!(record.last_attempt.is_some());

        record.mark_delivered();
        assert_eq!(record.status, DeliveryStatus::Delivered);
        assert!(record.delivered_at.is_some());

        record.mark_acknowledged();
        assert_eq!(record.status, DeliveryStatus::Acknowledged);
        assert!(record.acknowledged_at.is_some());
        assert!(record.is_final_state());
    }

    #[tokio::test]
    async fn test_delivery_manager_basic_flow() {
        let config = DeliveryConfig::default();
        let manager = DeliveryManager::new(config);

        let envelope = MessageEnvelope::new(
            AgentPid::new(),
            "test_message".to_string(),
            serde_json::json!({"data": "test"}),
            DeliveryOptions::default(),
        );

        // Record message
        manager.record_message(&envelope).await.unwrap();

        // Mark as in transit
        manager.mark_in_transit(&envelope.id).await.unwrap();

        // Mark as delivered
        manager.mark_delivered(&envelope.id).await.unwrap();

        // Mark as acknowledged
        manager.mark_acknowledged(&envelope.id).await.unwrap();

        // Check final state
        let record = manager.get_delivery_record(&envelope.id).await.unwrap();
        assert_eq!(record.status, DeliveryStatus::Acknowledged);
        assert!(record.is_final_state());
    }

    #[tokio::test]
    async fn test_retry_logic() {
        let config = DeliveryConfig::default();
        let manager = DeliveryManager::new(config);

        let envelope = MessageEnvelope::new(
            AgentPid::new(),
            "test_message".to_string(),
            serde_json::json!({"data": "test"}),
            DeliveryOptions::default(),
        );

        // Record and mark as failed
        manager.record_message(&envelope).await.unwrap();
        manager.mark_in_transit(&envelope.id).await.unwrap();
        manager
            .mark_failed(&envelope.id, "network error".to_string())
            .await
            .unwrap();

        // Should be a retry candidate
        let candidates = manager.get_retry_candidates().await;
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].id, envelope.id);
    }

    #[tokio::test]
    async fn test_deduplication() {
        let mut config = DeliveryConfig::default();
        config.guarantee = DeliveryGuarantee::ExactlyOnce;
        let manager = DeliveryManager::new(config);

        let envelope = MessageEnvelope::new(
            AgentPid::new(),
            "test_message".to_string(),
            serde_json::json!({"data": "test"}),
            DeliveryOptions::default(),
        );

        // Record message twice
        manager.record_message(&envelope).await.unwrap();
        manager.record_message(&envelope).await.unwrap(); // Should be deduplicated

        let stats = manager.get_stats().await;
        assert_eq!(stats.total_messages, 1); // Only one message recorded
    }

    #[tokio::test]
    async fn test_retry_delay_calculation() {
        let config = DeliveryConfig::default();
        let manager = DeliveryManager::new(config);

        // Test exponential backoff
        let delay1 = manager.calculate_retry_delay(0);
        let delay2 = manager.calculate_retry_delay(1);
        let delay3 = manager.calculate_retry_delay(2);

        assert!(delay2 > delay1);
        assert!(delay3 > delay2);

        // Test max delay cap
        let max_delay = manager.calculate_retry_delay(100);
        assert_eq!(max_delay, Duration::from_secs(30)); // Should be capped
    }

    #[tokio::test]
    async fn test_delivery_stats() {
        let config = DeliveryConfig::default();
        let manager = DeliveryManager::new(config);

        // Create some test messages
        for i in 0..5 {
            let envelope = MessageEnvelope::new(
                AgentPid::new(),
                "test_message".to_string(),
                serde_json::json!({"data": i}),
                DeliveryOptions::default(),
            );

            manager.record_message(&envelope).await.unwrap();
            manager.mark_in_transit(&envelope.id).await.unwrap();

            if i < 3 {
                manager.mark_delivered(&envelope.id).await.unwrap();
                manager.mark_acknowledged(&envelope.id).await.unwrap();
            } else {
                manager
                    .mark_failed(&envelope.id, "test error".to_string())
                    .await
                    .unwrap();
            }
        }

        let stats = manager.get_stats().await;
        assert_eq!(stats.total_messages, 5);
        assert_eq!(stats.acknowledged, 3);
        assert_eq!(stats.failed, 2);
        assert_eq!(stats.success_rate(), 0.6);
        assert_eq!(stats.failure_rate(), 0.4);
    }
}
