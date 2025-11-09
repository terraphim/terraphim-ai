/*!
# Centralized Security Monitoring and Alerts System

This module provides a comprehensive security monitoring and alerting system
that consolidates security events from multiple sources and provides intelligent alerting.
*/

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::{SecurityEvent, SecurityEventType, SecuritySeverity};

/// Alert severity levels
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum AlertSeverity {
    Info,
    Warning,
    Error,
    Critical,
}

/// Alert status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AlertStatus {
    Active,
    Acknowledged,
    Resolved,
    Suppressed,
}

/// Security alert
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityAlert {
    /// Unique alert identifier
    pub id: Uuid,
    /// Alert title
    pub title: String,
    /// Detailed description
    pub description: String,
    /// Alert severity
    pub severity: AlertSeverity,
    /// Current status
    pub status: AlertStatus,
    /// When alert was created
    pub created_at: DateTime<Utc>,
    /// When alert was last updated
    pub updated_at: DateTime<Utc>,
    /// Related security events
    pub related_events: Vec<Uuid>,
    /// Alert source (e.g., "1password", "audit", "monitoring")
    pub source: String,
    /// Alert category
    pub category: AlertCategory,
    /// Additional metadata
    pub metadata: HashMap<String, String>,
    /// Suggested actions
    pub recommendations: Vec<String>,
}

/// Alert categories
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AlertCategory {
    /// Unauthorized access attempts
    UnauthorizedAccess,
    /// Suspicious behavior patterns
    SuspiciousActivity,
    /// System security issues
    SystemSecurity,
    /// Data protection concerns
    DataProtection,
    /// Configuration issues
    Configuration,
    /// Performance anomalies
    Performance,
    /// Compliance violations
    Compliance,
}

/// Alert rule configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertRule {
    /// Rule identifier
    pub id: String,
    /// Rule name
    pub name: String,
    /// Rule description
    pub description: String,
    /// Rule is enabled
    pub enabled: bool,
    /// Event types to monitor
    pub event_types: Vec<SecurityEventType>,
    /// Minimum severity to trigger
    pub min_severity: SecuritySeverity,
    /// Time window in minutes
    pub time_window_minutes: u32,
    /// Threshold count
    pub threshold: u32,
    /// Alert severity when triggered
    pub alert_severity: AlertSeverity,
    /// Alert category
    pub alert_category: AlertCategory,
    /// Custom conditions
    pub conditions: Vec<AlertCondition>,
}

/// Alert conditions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AlertCondition {
    /// Event count exceeds threshold
    EventCount { threshold: u32 },
    /// Specific event type detected
    EventType { event_type: SecurityEventType },
    /// Severity level detected
    Severity { min_severity: SecuritySeverity },
    /// Time-based condition
    TimeWindow { minutes: u32 },
    /// Metadata-based condition
    Metadata { key: String, value: String },
    /// Custom condition
    Custom { expression: String },
}

/// Alert notification channel
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NotificationChannel {
    /// In-app notification
    InApp,
    /// Email notification
    Email { recipients: Vec<String> },
    /// Webhook notification
    Webhook { url: String },
    /// System log
    SystemLog,
    /// External monitoring system
    External { system: String, endpoint: String },
}

/// Notification message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationMessage {
    /// Message ID
    pub id: Uuid,
    /// Alert ID
    pub alert_id: Uuid,
    /// Channel used
    pub channel: NotificationChannel,
    /// Message content
    pub content: String,
    /// When notification was sent
    pub sent_at: DateTime<Utc>,
    /// Delivery status
    pub status: NotificationStatus,
}

/// Notification status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum NotificationStatus {
    Pending,
    Sent,
    Failed,
    Retrying,
}

/// Centralized security monitoring system
#[derive(Debug, Clone)]
pub struct SecurityMonitoringSystem {
    /// Active alerts
    alerts: Arc<RwLock<HashMap<Uuid, SecurityAlert>>>,
    /// Security events
    events: Arc<RwLock<Vec<SecurityEvent>>>,
    /// Alert rules
    rules: Arc<RwLock<Vec<AlertRule>>>,
    /// Notification channels
    notification_channels: Arc<RwLock<Vec<NotificationChannel>>>,
    /// Notification history
    notifications: Arc<RwLock<Vec<NotificationMessage>>>,
}

impl SecurityMonitoringSystem {
    /// Create a new security monitoring system
    pub fn new() -> Self {
        let system = Self {
            alerts: Arc::new(RwLock::new(HashMap::new())),
            events: Arc::new(RwLock::new(Vec::new())),
            rules: Arc::new(RwLock::new(Vec::new())),
            notification_channels: Arc::new(RwLock::new(Vec::new())),
            notifications: Arc::new(RwLock::new(Vec::new())),
        };

        // Initialize default rules
        tokio::spawn({
            let system = system.clone();
            async move {
                system.initialize_default_rules().await;
            }
        });

        system
    }

    /// Initialize default alert rules
    async fn initialize_default_rules(&self) {
        let default_rules = vec![
            AlertRule {
                id: "critical_immediate".to_string(),
                name: "Critical Security Events".to_string(),
                description: "Immediate alert for any critical security event".to_string(),
                enabled: true,
                event_types: vec![],
                min_severity: SecuritySeverity::Critical,
                time_window_minutes: 1,
                threshold: 1,
                alert_severity: AlertSeverity::Critical,
                alert_category: AlertCategory::SystemSecurity,
                conditions: vec![AlertCondition::Severity {
                    min_severity: SecuritySeverity::Critical,
                }],
            },
            AlertRule {
                id: "auth_failure_burst".to_string(),
                name: "Authentication Failure Burst".to_string(),
                description: "Multiple authentication failures in short time".to_string(),
                enabled: true,
                event_types: vec![SecurityEventType::AuthenticationFailure],
                min_severity: SecuritySeverity::Low,
                time_window_minutes: 5,
                threshold: 3,
                alert_severity: AlertSeverity::Error,
                alert_category: AlertCategory::UnauthorizedAccess,
                conditions: vec![
                    AlertCondition::EventCount { threshold: 3 },
                    AlertCondition::TimeWindow { minutes: 5 },
                ],
            },
            AlertRule {
                id: "suspicious_access_pattern".to_string(),
                name: "Suspicious Access Pattern".to_string(),
                description: "High frequency access to sensitive resources".to_string(),
                enabled: true,
                event_types: vec![SecurityEventType::SuspiciousAccess],
                min_severity: SecuritySeverity::Medium,
                time_window_minutes: 10,
                threshold: 1,
                alert_severity: AlertSeverity::Warning,
                alert_category: AlertCategory::SuspiciousActivity,
                conditions: vec![AlertCondition::EventType {
                    event_type: SecurityEventType::SuspiciousAccess,
                }],
            },
            AlertRule {
                id: "permission_denied_spike".to_string(),
                name: "Permission Denied Spike".to_string(),
                description: "Unusual number of permission denied events".to_string(),
                enabled: true,
                event_types: vec![SecurityEventType::PermissionDenied],
                min_severity: SecuritySeverity::Low,
                time_window_minutes: 15,
                threshold: 5,
                alert_severity: AlertSeverity::Warning,
                alert_category: AlertCategory::UnauthorizedAccess,
                conditions: vec![
                    AlertCondition::EventCount { threshold: 5 },
                    AlertCondition::TimeWindow { minutes: 15 },
                ],
            },
        ];

        let mut rules = self.rules.write().await;
        *rules = default_rules;
    }

    /// Add a security event to the monitoring system
    pub async fn add_security_event(&self, event: SecurityEvent) {
        // Store the event
        {
            let mut events = self.events.write().await;
            events.push(event.clone());
        }

        // Evaluate alert rules
        self.evaluate_alert_rules(&event).await;
    }

    /// Evaluate alert rules against a security event
    async fn evaluate_alert_rules(&self, event: &SecurityEvent) {
        let rules = self.rules.read().await;

        for rule in rules.iter().filter(|r| r.enabled) {
            if self.evaluate_rule(rule, event).await {
                self.create_alert_from_rule(rule, vec![event.id]).await;
            }
        }
    }

    /// Evaluate a single alert rule
    async fn evaluate_rule(&self, rule: &AlertRule, event: &SecurityEvent) -> bool {
        // Check event type filter
        if !rule.event_types.is_empty() && !rule.event_types.contains(&event.event_type) {
            return false;
        }

        // Check minimum severity
        if event.severity < rule.min_severity {
            return false;
        }

        // Evaluate conditions
        for condition in &rule.conditions {
            if !self.evaluate_condition(condition, event).await {
                return false;
            }
        }

        true
    }

    /// Evaluate a single condition
    async fn evaluate_condition(&self, condition: &AlertCondition, event: &SecurityEvent) -> bool {
        match condition {
            AlertCondition::EventCount { threshold } => {
                // This would need to be evaluated in context of multiple events
                // For now, return true if threshold is 1
                *threshold <= 1
            }
            AlertCondition::EventType { event_type } => &event.event_type == event_type,
            AlertCondition::Severity { min_severity } => event.severity >= *min_severity,
            AlertCondition::TimeWindow { minutes: _ } => {
                // Time window conditions need to be evaluated across multiple events
                true
            }
            AlertCondition::Metadata { key, value } => event.metadata.get(key) == Some(value),
            AlertCondition::Custom { expression: _ } => {
                // Custom expressions would need a proper expression engine
                false
            }
        }
    }

    /// Create an alert from a rule
    async fn create_alert_from_rule(&self, rule: &AlertRule, event_ids: Vec<Uuid>) {
        let alert = SecurityAlert {
            id: Uuid::new_v4(),
            title: rule.name.clone(),
            description: format!("Alert triggered: {}", rule.description),
            severity: rule.alert_severity.clone(),
            status: AlertStatus::Active,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            related_events: event_ids,
            source: "security_monitoring".to_string(),
            category: rule.alert_category.clone(),
            metadata: HashMap::from([
                ("rule_id".to_string(), rule.id.clone()),
                ("auto_generated".to_string(), "true".to_string()),
            ]),
            recommendations: self.generate_recommendations(&rule.alert_category),
        };

        self.add_alert(alert).await;
    }

    /// Add an alert to the system
    pub async fn add_alert(&self, alert: SecurityAlert) {
        {
            let mut alerts = self.alerts.write().await;
            alerts.insert(alert.id, alert.clone());
        }

        // Send notifications
        self.send_alert_notifications(&alert).await;

        log::warn!("🚨 SECURITY ALERT: {} - {}", alert.title, alert.description);
    }

    /// Send notifications for an alert
    async fn send_alert_notifications(&self, alert: &SecurityAlert) {
        let channels = self.notification_channels.read().await;

        for channel in channels.iter() {
            let message = self.create_notification_message(alert, channel).await;
            self.send_notification(message).await;
        }
    }

    /// Create notification message for alert and channel
    async fn create_notification_message(
        &self,
        alert: &SecurityAlert,
        channel: &NotificationChannel,
    ) -> NotificationMessage {
        let content = match channel {
            NotificationChannel::InApp => {
                format!("🚨 {}: {}", alert.title, alert.description)
            }
            NotificationChannel::Email { .. } => {
                format!(
                    "Security Alert: {}\n\n{}\n\nSeverity: {:?}\nTime: {}\n\nRecommendations:\n{}",
                    alert.title,
                    alert.description,
                    alert.severity,
                    alert.created_at.format("%Y-%m-%d %H:%M:%S UTC"),
                    alert.recommendations.join("\n")
                )
            }
            NotificationChannel::Webhook { .. } => serde_json::json!({
                "alert_id": alert.id,
                "title": alert.title,
                "description": alert.description,
                "severity": alert.severity,
                "category": alert.category,
                "created_at": alert.created_at,
                "recommendations": alert.recommendations
            })
            .to_string(),
            NotificationChannel::SystemLog => {
                format!(
                    "SECURITY_ALERT: {} - {} [Severity: {:?}]",
                    alert.title, alert.description, alert.severity
                )
            }
            NotificationChannel::External { system, .. } => {
                format!("External alert to {}: {}", system, alert.title)
            }
        };

        NotificationMessage {
            id: Uuid::new_v4(),
            alert_id: alert.id,
            channel: channel.clone(),
            content,
            sent_at: Utc::now(),
            status: NotificationStatus::Pending,
        }
    }

    /// Send notification message
    async fn send_notification(&self, message: NotificationMessage) {
        // Store notification
        {
            let mut notifications = self.notifications.write().await;
            notifications.push(message.clone());
        }

        // Actually send the notification based on channel
        match &message.channel {
            NotificationChannel::InApp => {
                // In-app notifications would be handled by the frontend
                log::info!("In-app notification: {}", message.content);
            }
            NotificationChannel::SystemLog => {
                log::warn!("{}", message.content);
            }
            NotificationChannel::Email { recipients } => {
                log::info!(
                    "Email notification to {:?}: {}",
                    recipients,
                    message.content
                );
                // Actual email sending would be implemented here
            }
            NotificationChannel::Webhook { url } => {
                log::info!("Webhook notification to {}: {}", url, message.content);
                // Actual webhook sending would be implemented here
            }
            NotificationChannel::External { system, endpoint } => {
                log::info!(
                    "External notification to {} at {}: {}",
                    system,
                    endpoint,
                    message.content
                );
                // Actual external notification would be implemented here
            }
        }
    }

    /// Generate recommendations based on alert category
    fn generate_recommendations(&self, category: &AlertCategory) -> Vec<String> {
        match category {
            AlertCategory::UnauthorizedAccess => vec![
                "Review access logs for unauthorized attempts".to_string(),
                "Consider implementing rate limiting".to_string(),
                "Verify authentication mechanisms are properly configured".to_string(),
            ],
            AlertCategory::SuspiciousActivity => vec![
                "Investigate the source of suspicious activity".to_string(),
                "Review user access patterns".to_string(),
                "Consider implementing additional monitoring".to_string(),
            ],
            AlertCategory::SystemSecurity => vec![
                "Immediate investigation required".to_string(),
                "Check system integrity".to_string(),
                "Review security configurations".to_string(),
            ],
            AlertCategory::DataProtection => vec![
                "Verify data encryption is properly configured".to_string(),
                "Review data access permissions".to_string(),
                "Check backup and recovery procedures".to_string(),
            ],
            AlertCategory::Configuration => vec![
                "Review security configuration settings".to_string(),
                "Validate configuration against security policies".to_string(),
                "Document configuration changes".to_string(),
            ],
            AlertCategory::Performance => vec![
                "Monitor system performance metrics".to_string(),
                "Check for resource exhaustion".to_string(),
                "Review system capacity planning".to_string(),
            ],
            AlertCategory::Compliance => vec![
                "Review compliance requirements".to_string(),
                "Document compliance violations".to_string(),
                "Implement corrective actions".to_string(),
            ],
        }
    }

    /// Get active alerts
    pub async fn get_active_alerts(&self) -> Vec<SecurityAlert> {
        let alerts = self.alerts.read().await;
        alerts
            .values()
            .filter(|alert| alert.status == AlertStatus::Active)
            .cloned()
            .collect()
    }

    /// Get alerts by severity
    pub async fn get_alerts_by_severity(&self, severity: AlertSeverity) -> Vec<SecurityAlert> {
        let alerts = self.alerts.read().await;
        alerts
            .values()
            .filter(|alert| alert.severity == severity)
            .cloned()
            .collect()
    }

    /// Get alerts by category
    pub async fn get_alerts_by_category(&self, category: AlertCategory) -> Vec<SecurityAlert> {
        let alerts = self.alerts.read().await;
        alerts
            .values()
            .filter(|alert| alert.category == category)
            .cloned()
            .collect()
    }

    /// Acknowledge an alert
    pub async fn acknowledge_alert(&self, alert_id: Uuid) -> Result<(), String> {
        let mut alerts = self.alerts.write().await;
        if let Some(alert) = alerts.get_mut(&alert_id) {
            alert.status = AlertStatus::Acknowledged;
            alert.updated_at = Utc::now();
            Ok(())
        } else {
            Err("Alert not found".to_string())
        }
    }

    /// Resolve an alert
    pub async fn resolve_alert(&self, alert_id: Uuid) -> Result<(), String> {
        let mut alerts = self.alerts.write().await;
        if let Some(alert) = alerts.get_mut(&alert_id) {
            alert.status = AlertStatus::Resolved;
            alert.updated_at = Utc::now();
            Ok(())
        } else {
            Err("Alert not found".to_string())
        }
    }

    /// Get alert statistics
    pub async fn get_alert_statistics(&self) -> AlertStatistics {
        let alerts = self.alerts.read().await;
        let now = Utc::now();
        let last_24h = now - chrono::Duration::hours(24);
        let last_7d = now - chrono::Duration::days(7);

        AlertStatistics {
            total_alerts: alerts.len(),
            active_alerts: alerts
                .values()
                .filter(|a| a.status == AlertStatus::Active)
                .count(),
            acknowledged_alerts: alerts
                .values()
                .filter(|a| a.status == AlertStatus::Acknowledged)
                .count(),
            resolved_alerts: alerts
                .values()
                .filter(|a| a.status == AlertStatus::Resolved)
                .count(),
            last_24h_alerts: alerts.values().filter(|a| a.created_at > last_24h).count(),
            last_7d_alerts: alerts.values().filter(|a| a.created_at > last_7d).count(),
            critical_alerts: alerts
                .values()
                .filter(|a| a.severity == AlertSeverity::Critical)
                .count(),
            high_alerts: alerts
                .values()
                .filter(|a| a.severity == AlertSeverity::Error)
                .count(),
        }
    }

    /// Add notification channel
    pub async fn add_notification_channel(&self, channel: NotificationChannel) {
        let mut channels = self.notification_channels.write().await;
        channels.push(channel);
    }

    /// Get notification channels
    pub async fn get_notification_channels(&self) -> Vec<NotificationChannel> {
        self.notification_channels.read().await.clone()
    }
}

impl Default for SecurityMonitoringSystem {
    fn default() -> Self {
        Self::new()
    }
}

/// Alert statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertStatistics {
    pub total_alerts: usize,
    pub active_alerts: usize,
    pub acknowledged_alerts: usize,
    pub resolved_alerts: usize,
    pub last_24h_alerts: usize,
    pub last_7d_alerts: usize,
    pub critical_alerts: usize,
    pub high_alerts: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_alert_creation() {
        let system = SecurityMonitoringSystem::new();

        // Wait a moment for default rules to initialize
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        // Add a critical security event to trigger immediate alert
        let critical_event = SecurityEvent {
            id: Uuid::new_v4(),
            event_type: SecurityEventType::AuthenticationFailure,
            timestamp: Utc::now(),
            reference: Some("op://test/vault/item".to_string()),
            actor: "test_user".to_string(),
            description: "Critical auth failure".to_string(),
            severity: SecuritySeverity::Critical,
            metadata: HashMap::new(),
        };

        system.add_security_event(critical_event).await;

        // Wait a moment for alert processing
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        let alerts = system.get_active_alerts().await;
        // Should have alerts based on default rules
        assert!(!alerts.is_empty());
    }

    #[tokio::test]
    async fn test_alert_acknowledgment() {
        let system = SecurityMonitoringSystem::new();

        // Create a test alert
        let alert = SecurityAlert {
            id: Uuid::new_v4(),
            title: "Test Alert".to_string(),
            description: "Test description".to_string(),
            severity: AlertSeverity::Warning,
            status: AlertStatus::Active,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            related_events: vec![],
            source: "test".to_string(),
            category: AlertCategory::SystemSecurity,
            metadata: HashMap::new(),
            recommendations: vec!["Test recommendation".to_string()],
        };

        system.add_alert(alert.clone()).await;

        // Acknowledge the alert
        let result = system.acknowledge_alert(alert.id).await;
        assert!(result.is_ok());

        let alerts = system.get_active_alerts().await;
        assert!(alerts.iter().all(|a| a.id != alert.id));
    }

    #[tokio::test]
    async fn test_alert_statistics() {
        let system = SecurityMonitoringSystem::new();
        let stats = system.get_alert_statistics().await;

        assert_eq!(stats.total_alerts, 0);
        assert_eq!(stats.active_alerts, 0);
        assert_eq!(stats.critical_alerts, 0);
    }
}
