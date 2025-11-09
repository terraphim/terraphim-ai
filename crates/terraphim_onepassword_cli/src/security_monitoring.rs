/*!
# Enhanced 1Password Security Monitoring

This module provides security monitoring capabilities for 1Password integration,
including access logging, anomaly detection, and security compliance checks.
*/

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::fs::OpenOptions;
use tokio::io::AsyncWriteExt;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::{OnePasswordError, OnePasswordLoader, SecretLoader};

/// Security event types for 1Password monitoring
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SecurityEventType {
    /// Secret access event
    SecretAccess,
    /// Authentication failure
    AuthenticationFailure,
    /// Permission denied
    PermissionDenied,
    /// Suspicious access pattern
    SuspiciousAccess,
    /// Configuration change
    ConfigurationChange,
    /// Security policy violation
    PolicyViolation,
}

impl std::fmt::Display for SecurityEventType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SecurityEventType::SecretAccess => write!(f, "SecretAccess"),
            SecurityEventType::AuthenticationFailure => write!(f, "AuthenticationFailure"),
            SecurityEventType::PermissionDenied => write!(f, "PermissionDenied"),
            SecurityEventType::SuspiciousAccess => write!(f, "SuspiciousAccess"),
            SecurityEventType::ConfigurationChange => write!(f, "ConfigurationChange"),
            SecurityEventType::PolicyViolation => write!(f, "PolicyViolation"),
        }
    }
}

/// Security event with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityEvent {
    /// Unique event identifier
    pub id: Uuid,
    /// Event type
    pub event_type: SecurityEventType,
    /// Timestamp when event occurred
    pub timestamp: DateTime<Utc>,
    /// 1Password reference (if applicable)
    pub reference: Option<String>,
    /// User or process that triggered the event
    pub actor: String,
    /// Event description
    pub description: String,
    /// Security severity level
    pub severity: SecuritySeverity,
    /// Additional metadata
    pub metadata: HashMap<String, String>,
}

/// Security severity levels
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum SecuritySeverity {
    Info,
    Low,
    Medium,
    High,
    Critical,
}

/// Security configuration for 1Password monitoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    /// Enable access logging
    pub enable_access_logging: bool,
    /// Enable anomaly detection
    pub enable_anomaly_detection: bool,
    /// Maximum allowed accesses per minute
    pub max_accesses_per_minute: u32,
    /// Require authentication for sensitive vaults
    pub require_auth_for_sensitive: bool,
    /// Sensitive vault patterns
    pub sensitive_vault_patterns: Vec<String>,
    /// Enable security alerts
    pub enable_security_alerts: bool,
    /// Log file path
    pub log_file_path: Option<PathBuf>,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            enable_access_logging: true,
            enable_anomaly_detection: true,
            max_accesses_per_minute: 10,
            require_auth_for_sensitive: true,
            sensitive_vault_patterns: vec![
                "production".to_string(),
                "prod".to_string(),
                "secret".to_string(),
                "credential".to_string(),
                "admin".to_string(),
            ],
            enable_security_alerts: true,
            log_file_path: Some(PathBuf::from("terraphim_security.log")),
        }
    }
}

/// Enhanced 1Password loader with security monitoring
#[derive(Debug, Clone)]
pub struct SecureOnePasswordLoader {
    /// Base 1Password loader
    base_loader: OnePasswordLoader,
    /// Security configuration
    config: SecurityConfig,
    /// Security events log
    events: Arc<RwLock<Vec<SecurityEvent>>>,
    /// Access tracking for anomaly detection
    access_tracker: Arc<RwLock<HashMap<String, Vec<DateTime<Utc>>>>>,
}

impl SecureOnePasswordLoader {
    /// Create a new secure 1Password loader
    pub fn new(config: SecurityConfig) -> Self {
        Self {
            base_loader: OnePasswordLoader::new(),
            config,
            events: Arc::new(RwLock::new(Vec::new())),
            access_tracker: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Record a security event
    pub async fn record_security_event(&self, event: SecurityEvent) {
        let mut events = self.events.write().await;
        events.push(event.clone());

        // Log to file if configured
        if let Some(log_path) = &self.config.log_file_path {
            if let Err(e) = self.log_event_to_file(&event, log_path).await {
                log::error!("Failed to log security event to file: {}", e);
            }
        }

        // Trigger alerts for high-severity events
        if event.severity >= SecuritySeverity::High && self.config.enable_security_alerts {
            self.trigger_security_alert(&event).await;
        }

        log::info!("Security event recorded: {:?}", event);
    }

    /// Log event to file
    async fn log_event_to_file(
        &self,
        event: &SecurityEvent,
        log_path: &PathBuf,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let log_line = serde_json::to_string(event)?;
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(log_path)
            .await?;

        file.write_all(log_line.as_bytes()).await?;
        file.write_all(b"\n").await?;
        file.flush().await?;

        Ok(())
    }

    /// Trigger security alert
    async fn trigger_security_alert(&self, event: &SecurityEvent) {
        log::warn!(
            "🚨 SECURITY ALERT: {} - {}",
            event.event_type,
            event.description
        );

        // In a real implementation, this could send notifications, emails, etc.
        // For now, we'll just log prominently
        match event.severity {
            SecuritySeverity::Critical => {
                log::error!("🔴 CRITICAL SECURITY EVENT: Immediate attention required!");
            }
            SecuritySeverity::High => {
                log::error!("🟠 HIGH SECURITY EVENT: Investigate promptly!");
            }
            _ => {
                log::warn!("🟡 SECURITY EVENT: Review recommended");
            }
        }
    }

    /// Check for suspicious access patterns
    async fn detect_suspicious_access(&self, reference: &str) -> Option<SecurityEvent> {
        if !self.config.enable_anomaly_detection {
            return None;
        }

        let mut tracker = self.access_tracker.write().await;
        let now = Utc::now();
        let accesses = tracker
            .entry(reference.to_string())
            .or_insert_with(Vec::new);

        // Clean old accesses (older than 1 minute)
        accesses.retain(|&time| now.signed_duration_since(time).num_seconds() < 60);

        // Add current access
        accesses.push(now);

        // Check if exceeding threshold
        if accesses.len() > self.config.max_accesses_per_minute as usize {
            Some(SecurityEvent {
                id: Uuid::new_v4(),
                event_type: SecurityEventType::SuspiciousAccess,
                timestamp: now,
                reference: Some(reference.to_string()),
                actor: "system".to_string(),
                description: format!(
                    "High frequency access detected: {} accesses in 1 minute for {}",
                    accesses.len(),
                    reference
                ),
                severity: SecuritySeverity::Medium,
                metadata: HashMap::from([
                    ("access_count".to_string(), accesses.len().to_string()),
                    (
                        "threshold".to_string(),
                        self.config.max_accesses_per_minute.to_string(),
                    ),
                ]),
            })
        } else {
            None
        }
    }

    /// Check if vault is sensitive
    fn is_sensitive_vault(&self, vault: &str) -> bool {
        self.config
            .sensitive_vault_patterns
            .iter()
            .any(|pattern| vault.to_lowercase().contains(&pattern.to_lowercase()))
    }

    /// Get security events
    pub async fn get_security_events(&self) -> Vec<SecurityEvent> {
        self.events.read().await.clone()
    }

    /// Get security events by type
    pub async fn get_events_by_type(&self, event_type: SecurityEventType) -> Vec<SecurityEvent> {
        let events = self.events.read().await;
        events
            .iter()
            .filter(|event| event.event_type == event_type)
            .cloned()
            .collect()
    }

    /// Get security events by severity
    pub async fn get_events_by_severity(&self, severity: SecuritySeverity) -> Vec<SecurityEvent> {
        let events = self.events.read().await;
        events
            .iter()
            .filter(|event| event.severity == severity)
            .cloned()
            .collect()
    }

    /// Clear old security events
    pub async fn clear_old_events(&self, days: i64) {
        let mut events = self.events.write().await;
        let cutoff = Utc::now() - chrono::Duration::days(days);
        events.retain(|event| event.timestamp > cutoff);
    }

    /// Generate security report
    pub async fn generate_security_report(&self) -> SecurityReport {
        let events = self.events.read().await;
        let now = Utc::now();
        let last_24h = now - chrono::Duration::hours(24);
        let last_7d = now - chrono::Duration::days(7);

        let recent_events: Vec<_> = events
            .iter()
            .filter(|event| event.timestamp > last_24h)
            .collect();

        let weekly_events: Vec<_> = events
            .iter()
            .filter(|event| event.timestamp > last_7d)
            .collect();

        let critical_events = events
            .iter()
            .filter(|event| event.severity == SecuritySeverity::Critical)
            .count();

        let high_events = events
            .iter()
            .filter(|event| event.severity == SecuritySeverity::High)
            .count();

        SecurityReport {
            generated_at: now,
            total_events: events.len(),
            last_24h_events: recent_events.len(),
            last_7d_events: weekly_events.len(),
            critical_events_count: critical_events,
            high_events_count: high_events,
            security_score: self.calculate_security_score(&events),
            recommendations: self.generate_security_recommendations(&events),
        }
    }

    /// Calculate security score (0-100)
    fn calculate_security_score(&self, events: &[SecurityEvent]) -> f32 {
        if events.is_empty() {
            return 100.0;
        }

        let now = Utc::now();
        let last_30d = now - chrono::Duration::days(30);
        let recent_events: Vec<_> = events
            .iter()
            .filter(|event| event.timestamp > last_30d)
            .collect();

        if recent_events.is_empty() {
            return 100.0;
        }

        let critical_weight = 10.0;
        let high_weight = 5.0;
        let medium_weight = 2.0;
        let low_weight = 1.0;
        let info_weight = 0.1;

        let weighted_score: f32 = recent_events
            .iter()
            .map(|event| match event.severity {
                SecuritySeverity::Critical => critical_weight,
                SecuritySeverity::High => high_weight,
                SecuritySeverity::Medium => medium_weight,
                SecuritySeverity::Low => low_weight,
                SecuritySeverity::Info => info_weight,
            })
            .sum();

        // Score decreases with weighted events, max 100
        (100.0 - weighted_score).max(0.0)
    }

    /// Generate security recommendations
    fn generate_security_recommendations(&self, events: &[SecurityEvent]) -> Vec<String> {
        let mut recommendations = Vec::new();

        let auth_failures = events
            .iter()
            .filter(|event| event.event_type == SecurityEventType::AuthenticationFailure)
            .count();

        if auth_failures > 5 {
            recommendations.push(
                "High number of authentication failures detected. Consider reviewing access controls.".to_string(),
            );
        }

        let suspicious_access = events
            .iter()
            .filter(|event| event.event_type == SecurityEventType::SuspiciousAccess)
            .count();

        if suspicious_access > 3 {
            recommendations.push(
                "Multiple suspicious access patterns detected. Review access logs and consider rate limiting.".to_string(),
            );
        }

        let critical_events = events
            .iter()
            .filter(|event| event.severity == SecuritySeverity::Critical)
            .count();

        if critical_events > 0 {
            recommendations.push(
                "Critical security events detected. Immediate investigation required.".to_string(),
            );
        }

        if recommendations.is_empty() {
            recommendations.push("Security posture looks good. Continue monitoring.".to_string());
        }

        recommendations
    }
}

#[async_trait::async_trait]
impl SecretLoader for SecureOnePasswordLoader {
    async fn resolve_secret(&self, reference: &str) -> Result<String, OnePasswordError> {
        let actor = std::env::var("USER").unwrap_or_else(|_| "unknown".to_string());
        let now = Utc::now();

        // Check for sensitive vault access
        if self.config.require_auth_for_sensitive {
            if let Ok(op_ref) = self.base_loader.parse_reference(reference) {
                if self.is_sensitive_vault(&op_ref.vault) {
                    // Verify authentication status for sensitive vaults
                    if !self.base_loader.check_authenticated().await {
                        let event = SecurityEvent {
                            id: Uuid::new_v4(),
                            event_type: SecurityEventType::AuthenticationFailure,
                            timestamp: now,
                            reference: Some(reference.to_string()),
                            actor: actor.clone(),
                            description: format!(
                                "Attempted access to sensitive vault '{}' without authentication",
                                op_ref.vault
                            ),
                            severity: SecuritySeverity::High,
                            metadata: HashMap::from([
                                ("vault".to_string(), op_ref.vault),
                                ("sensitive".to_string(), "true".to_string()),
                            ]),
                        };
                        self.record_security_event(event).await;
                        return Err(OnePasswordError::NotAuthenticated);
                    }
                }
            }
        }

        // Detect suspicious access patterns
        if let Some(suspicious_event) = self.detect_suspicious_access(reference).await {
            self.record_security_event(suspicious_event).await;
        }

        // Log access if enabled
        if self.config.enable_access_logging {
            let event = SecurityEvent {
                id: Uuid::new_v4(),
                event_type: SecurityEventType::SecretAccess,
                timestamp: now,
                reference: Some(reference.to_string()),
                actor: actor.clone(),
                description: format!("Secret accessed: {}", reference),
                severity: SecuritySeverity::Info,
                metadata: HashMap::new(),
            };
            self.record_security_event(event).await;
        }

        // Resolve the secret
        match self.base_loader.resolve_secret(reference).await {
            Ok(value) => {
                log::debug!("Successfully resolved secret: {}", reference);
                Ok(value)
            }
            Err(e) => {
                let event = SecurityEvent {
                    id: Uuid::new_v4(),
                    event_type: match &e {
                        OnePasswordError::SecretNotFound { .. } => {
                            SecurityEventType::PermissionDenied
                        }
                        OnePasswordError::PermissionDenied { .. } => {
                            SecurityEventType::PermissionDenied
                        }
                        OnePasswordError::NotAuthenticated => {
                            SecurityEventType::AuthenticationFailure
                        }
                        _ => SecurityEventType::PolicyViolation,
                    },
                    timestamp: now,
                    reference: Some(reference.to_string()),
                    actor,
                    description: format!("Failed to resolve secret: {}", e),
                    severity: match &e {
                        OnePasswordError::NotAuthenticated => SecuritySeverity::High,
                        OnePasswordError::PermissionDenied { .. } => SecuritySeverity::Medium,
                        _ => SecuritySeverity::Low,
                    },
                    metadata: HashMap::from([("error".to_string(), e.to_string())]),
                };
                self.record_security_event(event).await;
                Err(e)
            }
        }
    }

    async fn process_config(&self, config: &str) -> Result<String, OnePasswordError> {
        self.base_loader.process_config(config).await
    }

    async fn is_available(&self) -> bool {
        self.base_loader.is_available().await
    }
}

/// Security report summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityReport {
    /// When the report was generated
    pub generated_at: DateTime<Utc>,
    /// Total number of security events
    pub total_events: usize,
    /// Events in last 24 hours
    pub last_24h_events: usize,
    /// Events in last 7 days
    pub last_7d_events: usize,
    /// Number of critical events
    pub critical_events_count: usize,
    /// Number of high severity events
    pub high_events_count: usize,
    /// Overall security score (0-100)
    pub security_score: f32,
    /// Security recommendations
    pub recommendations: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_security_event_recording() {
        let config = SecurityConfig::default();
        let loader = SecureOnePasswordLoader::new(config);

        let event = SecurityEvent {
            id: Uuid::new_v4(),
            event_type: SecurityEventType::SecretAccess,
            timestamp: Utc::now(),
            reference: Some("op://test/vault/item/field".to_string()),
            actor: "test_user".to_string(),
            description: "Test event".to_string(),
            severity: SecuritySeverity::Info,
            metadata: HashMap::new(),
        };

        loader.record_security_event(event).await;
        let events = loader.get_security_events().await;
        assert_eq!(events.len(), 1);
    }

    #[tokio::test]
    async fn test_suspicious_access_detection() {
        let mut config = SecurityConfig::default();
        config.max_accesses_per_minute = 2; // Low threshold for testing
        let loader = SecureOnePasswordLoader::new(config);

        let reference = "op://test/vault/item/field";

        // First access should be fine
        let event1 = loader.detect_suspicious_access(reference).await;
        assert!(event1.is_none());

        // Simulate multiple rapid accesses
        for _ in 0..3 {
            let _ = loader.detect_suspicious_access(reference).await;
        }

        // Third access should trigger suspicion
        let event = loader.detect_suspicious_access(reference).await;
        assert!(event.is_some());
        assert_eq!(
            event.unwrap().event_type,
            SecurityEventType::SuspiciousAccess
        );
    }

    #[tokio::test]
    async fn test_sensitive_vault_detection() {
        let config = SecurityConfig::default();
        let loader = SecureOnePasswordLoader::new(config);

        assert!(loader.is_sensitive_vault("production"));
        assert!(loader.is_sensitive_vault("prod-secrets"));
        assert!(loader.is_sensitive_vault("admin-credentials"));
        assert!(!loader.is_sensitive_vault("development"));
        assert!(!loader.is_sensitive_vault("test-data"));
    }

    #[tokio::test]
    async fn test_security_report_generation() {
        let config = SecurityConfig::default();
        let loader = SecureOnePasswordLoader::new(config);

        let report = loader.generate_security_report().await;
        assert!(report.security_score >= 0.0 && report.security_score <= 100.0);
        assert!(!report.recommendations.is_empty());
    }
}
