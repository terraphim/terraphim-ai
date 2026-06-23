//! Structured audit events for agent lifecycle tracking.
//!
//! Audit events are emitted via `tracing::info!` with target
//! `terraphim_spawner::audit`, making them easy to filter and forward
//! to external logging systems.

use std::fmt;

use terraphim_types::capability::ProcessId;

/// Audit events for agent lifecycle operations.
#[derive(Debug, Clone)]
pub enum AuditEvent {
    /// Agent process was spawned.
    AgentSpawned {
        process_id: ProcessId,
        provider_id: String,
    },
    /// Agent process was terminated (graceful or forced).
    AgentTerminated {
        process_id: ProcessId,
        graceful: bool,
    },
    /// Health check failed for an agent.
    HealthCheckFailed {
        process_id: ProcessId,
        reason: String,
    },
    /// Agent was automatically restarted.
    AgentRestarted { process_id: ProcessId, attempt: u32 },
    /// Resource limits were applied to a process.
    ResourceLimitApplied {
        process_id: ProcessId,
        limit_type: String,
        value: u64,
    },
}

impl fmt::Display for AuditEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AuditEvent::AgentSpawned {
                process_id,
                provider_id,
            } => {
                write!(
                    f,
                    "AgentSpawned(process={}, provider={})",
                    process_id, provider_id
                )
            }
            AuditEvent::AgentTerminated {
                process_id,
                graceful,
            } => {
                write!(
                    f,
                    "AgentTerminated(process={}, graceful={})",
                    process_id, graceful
                )
            }
            AuditEvent::HealthCheckFailed { process_id, reason } => {
                write!(
                    f,
                    "HealthCheckFailed(process={}, reason={})",
                    process_id, reason
                )
            }
            AuditEvent::AgentRestarted {
                process_id,
                attempt,
            } => {
                write!(
                    f,
                    "AgentRestarted(process={}, attempt={})",
                    process_id, attempt
                )
            }
            AuditEvent::ResourceLimitApplied {
                process_id,
                limit_type,
                value,
            } => {
                write!(
                    f,
                    "ResourceLimitApplied(process={}, type={}, value={})",
                    process_id, limit_type, value
                )
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audit_event_display() {
        let pid = ProcessId::new();

        let event = AuditEvent::AgentSpawned {
            process_id: pid,
            provider_id: "@codex".to_string(),
        };
        let s = format!("{}", event);
        assert!(s.contains("AgentSpawned"));
        assert!(s.contains("@codex"));

        let event = AuditEvent::AgentTerminated {
            process_id: pid,
            graceful: true,
        };
        let s = format!("{}", event);
        assert!(s.contains("AgentTerminated"));
        assert!(s.contains("graceful=true"));

        let event = AuditEvent::HealthCheckFailed {
            process_id: pid,
            reason: "timeout".to_string(),
        };
        let s = format!("{}", event);
        assert!(s.contains("HealthCheckFailed"));
        assert!(s.contains("timeout"));

        let event = AuditEvent::AgentRestarted {
            process_id: pid,
            attempt: 2,
        };
        let s = format!("{}", event);
        assert!(s.contains("AgentRestarted"));
        assert!(s.contains("attempt=2"));

        let event = AuditEvent::ResourceLimitApplied {
            process_id: pid,
            limit_type: "RLIMIT_AS".to_string(),
            value: 1073741824,
        };
        let s = format!("{}", event);
        assert!(s.contains("ResourceLimitApplied"));
        assert!(s.contains("RLIMIT_AS"));
    }
}
