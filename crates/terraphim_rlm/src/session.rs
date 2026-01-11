//! Session management for RLM.
//!
//! The SessionManager handles:
//! - Session lifecycle (create, extend, destroy)
//! - VM affinity (mapping sessions to VMs)
//! - Context variables (get/set per-session state)
//! - Snapshot coordination with VMs

use dashmap::DashMap;
use jiff::{Timestamp, ToSpan};
use std::sync::atomic::{AtomicU32, Ordering};

use crate::config::RlmConfig;
use crate::error::{RlmError, RlmResult};
use crate::types::{BudgetStatus, SessionId, SessionInfo, SessionState};

/// Manages RLM sessions with VM affinity.
///
/// The SessionManager owns session state while VmManager owns VM lifecycle.
/// This separation ensures clear state ownership per the design specification.
pub struct SessionManager {
    /// Active sessions indexed by session ID.
    sessions: DashMap<SessionId, SessionInfo>,

    /// VM to session mapping for affinity.
    vm_to_session: DashMap<String, SessionId>,

    /// Session to VM mapping for reverse lookup.
    session_to_vm: DashMap<SessionId, String>,

    /// Configuration.
    config: RlmConfig,

    /// Total sessions created (for metrics).
    total_sessions_created: AtomicU32,

    /// Active session count.
    active_session_count: AtomicU32,
}

impl SessionManager {
    /// Create a new session manager.
    pub fn new(config: RlmConfig) -> Self {
        Self {
            sessions: DashMap::new(),
            vm_to_session: DashMap::new(),
            session_to_vm: DashMap::new(),
            config,
            total_sessions_created: AtomicU32::new(0),
            active_session_count: AtomicU32::new(0),
        }
    }

    /// Create a new session.
    pub fn create_session(&self) -> RlmResult<SessionInfo> {
        let session_id = SessionId::new();
        let mut session_info = SessionInfo::new(session_id, self.config.session_duration_secs);

        // Apply config-based budget limits
        session_info.budget_status.max_recursion_depth = self.config.max_recursion_depth;
        session_info.budget_status.token_budget = self.config.token_budget;
        session_info.budget_status.time_budget_ms = self.config.time_budget_ms;

        self.sessions.insert(session_id, session_info.clone());
        self.total_sessions_created.fetch_add(1, Ordering::Relaxed);
        self.active_session_count.fetch_add(1, Ordering::Relaxed);

        log::info!("Created session: {}", session_id);
        Ok(session_info)
    }

    /// Get a session by ID.
    pub fn get_session(&self, session_id: &SessionId) -> RlmResult<SessionInfo> {
        self.sessions
            .get(session_id)
            .map(|r| r.clone())
            .ok_or_else(|| RlmError::SessionNotFound {
                session_id: *session_id,
            })
    }

    /// Update session state.
    pub fn update_session_state(
        &self,
        session_id: &SessionId,
        state: SessionState,
    ) -> RlmResult<()> {
        let mut session =
            self.sessions
                .get_mut(session_id)
                .ok_or_else(|| RlmError::SessionNotFound {
                    session_id: *session_id,
                })?;

        session.state = state;
        Ok(())
    }

    /// Check if a session is valid (exists and not expired).
    pub fn is_session_valid(&self, session_id: &SessionId) -> bool {
        self.sessions
            .get(session_id)
            .map(|s| !s.is_expired() && s.state != SessionState::Terminated)
            .unwrap_or(false)
    }

    /// Validate a session token (used by LLM bridge).
    pub fn validate_session(&self, session_id: &SessionId) -> RlmResult<SessionInfo> {
        let session = self.get_session(session_id)?;

        if session.is_expired() {
            return Err(RlmError::SessionExpired {
                session_id: *session_id,
            });
        }

        if session.state == SessionState::Terminated {
            return Err(RlmError::InvalidSessionState {
                session_id: *session_id,
                state: "Terminated".to_string(),
                operation: "validate".to_string(),
            });
        }

        Ok(session)
    }

    /// Extend a session's lifetime.
    pub fn extend_session(&self, session_id: &SessionId) -> RlmResult<SessionInfo> {
        let mut session =
            self.sessions
                .get_mut(session_id)
                .ok_or_else(|| RlmError::SessionNotFound {
                    session_id: *session_id,
                })?;

        if !session.can_extend(self.config.max_extensions) {
            return Err(RlmError::MaxExtensionsReached {
                session_id: *session_id,
                max: self.config.max_extensions,
            });
        }

        if session.is_expired() {
            return Err(RlmError::SessionExpired {
                session_id: *session_id,
            });
        }

        // Extend the expiration time
        session.expires_at = session
            .expires_at
            .checked_add((self.config.extension_increment_secs as i64).seconds())
            .expect("adding seconds to timestamp should not fail");
        session.extension_count += 1;

        log::info!(
            "Extended session {} (extension {}/{})",
            session_id,
            session.extension_count,
            self.config.max_extensions
        );

        Ok(session.clone())
    }

    /// Assign a VM to a session (VM affinity).
    pub fn assign_vm(&self, session_id: &SessionId, vm_instance_id: String) -> RlmResult<()> {
        // Check if VM is already assigned to another session
        if let Some(existing_session) = self.vm_to_session.get(&vm_instance_id) {
            if *existing_session != *session_id {
                log::warn!(
                    "VM {} already assigned to session {}, reassigning to {}",
                    vm_instance_id,
                    *existing_session,
                    session_id
                );
                // Remove old mapping
                self.session_to_vm.remove(&existing_session);
            }
        }

        // Update session with VM ID
        let mut session =
            self.sessions
                .get_mut(session_id)
                .ok_or_else(|| RlmError::SessionNotFound {
                    session_id: *session_id,
                })?;

        session.vm_instance_id = Some(vm_instance_id.clone());
        session.state = SessionState::Ready;

        // Update mappings
        self.vm_to_session
            .insert(vm_instance_id.clone(), *session_id);
        self.session_to_vm.insert(*session_id, vm_instance_id);

        Ok(())
    }

    /// Get the VM assigned to a session.
    pub fn get_assigned_vm(&self, session_id: &SessionId) -> Option<String> {
        self.session_to_vm.get(session_id).map(|v| v.clone())
    }

    /// Get the session assigned to a VM.
    pub fn get_session_for_vm(&self, vm_instance_id: &str) -> Option<SessionId> {
        self.vm_to_session.get(vm_instance_id).map(|v| *v)
    }

    /// Set a context variable for a session.
    pub fn set_context_variable(
        &self,
        session_id: &SessionId,
        key: String,
        value: String,
    ) -> RlmResult<()> {
        let mut session =
            self.sessions
                .get_mut(session_id)
                .ok_or_else(|| RlmError::SessionNotFound {
                    session_id: *session_id,
                })?;

        session.context_variables.insert(key, value);
        Ok(())
    }

    /// Get a context variable from a session.
    pub fn get_context_variable(
        &self,
        session_id: &SessionId,
        key: &str,
    ) -> RlmResult<Option<String>> {
        let session = self.get_session(session_id)?;
        Ok(session.context_variables.get(key).cloned())
    }

    /// Get all context variables for a session.
    pub fn get_all_context_variables(
        &self,
        session_id: &SessionId,
    ) -> RlmResult<std::collections::HashMap<String, String>> {
        let session = self.get_session(session_id)?;
        Ok(session.context_variables.clone())
    }

    /// Update budget status for a session.
    pub fn update_budget(&self, session_id: &SessionId, budget: BudgetStatus) -> RlmResult<()> {
        let mut session =
            self.sessions
                .get_mut(session_id)
                .ok_or_else(|| RlmError::SessionNotFound {
                    session_id: *session_id,
                })?;

        session.budget_status = budget;
        Ok(())
    }

    /// Increment recursion depth for a session.
    pub fn increment_recursion_depth(&self, session_id: &SessionId) -> RlmResult<u32> {
        let mut session =
            self.sessions
                .get_mut(session_id)
                .ok_or_else(|| RlmError::SessionNotFound {
                    session_id: *session_id,
                })?;

        let new_depth = session.recursion_depth + 1;

        // Check before committing the increment
        if new_depth > session.budget_status.max_recursion_depth {
            return Err(RlmError::RecursionDepthExceeded {
                depth: new_depth,
                max_depth: session.budget_status.max_recursion_depth,
            });
        }

        session.recursion_depth = new_depth;
        session.budget_status.current_recursion_depth = session.recursion_depth;

        Ok(session.recursion_depth)
    }

    /// Decrement recursion depth for a session.
    pub fn decrement_recursion_depth(&self, session_id: &SessionId) -> RlmResult<u32> {
        let mut session =
            self.sessions
                .get_mut(session_id)
                .ok_or_else(|| RlmError::SessionNotFound {
                    session_id: *session_id,
                })?;

        session.recursion_depth = session.recursion_depth.saturating_sub(1);
        session.budget_status.current_recursion_depth = session.recursion_depth;

        Ok(session.recursion_depth)
    }

    /// Record that a snapshot was created for a session.
    ///
    /// This updates the session's snapshot count and optionally sets the current snapshot.
    pub fn record_snapshot_created(
        &self,
        session_id: &SessionId,
        snapshot_id: String,
        set_as_current: bool,
    ) -> RlmResult<()> {
        let mut session =
            self.sessions
                .get_mut(session_id)
                .ok_or_else(|| RlmError::SessionNotFound {
                    session_id: *session_id,
                })?;

        session.snapshot_count += 1;
        if set_as_current {
            session.current_snapshot_id = Some(snapshot_id);
        }

        log::debug!(
            "Recorded snapshot for session {} (count: {})",
            session_id,
            session.snapshot_count
        );

        Ok(())
    }

    /// Record that a snapshot was restored for a session.
    ///
    /// This sets the current snapshot ID for rollback tracking.
    pub fn record_snapshot_restored(
        &self,
        session_id: &SessionId,
        snapshot_id: String,
    ) -> RlmResult<()> {
        let mut session =
            self.sessions
                .get_mut(session_id)
                .ok_or_else(|| RlmError::SessionNotFound {
                    session_id: *session_id,
                })?;

        session.current_snapshot_id = Some(snapshot_id.clone());

        log::debug!(
            "Recorded snapshot restore for session {}: {}",
            session_id,
            snapshot_id
        );

        Ok(())
    }

    /// Get the current snapshot ID for a session.
    pub fn get_current_snapshot(&self, session_id: &SessionId) -> RlmResult<Option<String>> {
        let session = self.get_session(session_id)?;
        Ok(session.current_snapshot_id)
    }

    /// Clear snapshot tracking for a session (used when all snapshots are deleted).
    pub fn clear_snapshot_tracking(&self, session_id: &SessionId) -> RlmResult<()> {
        let mut session =
            self.sessions
                .get_mut(session_id)
                .ok_or_else(|| RlmError::SessionNotFound {
                    session_id: *session_id,
                })?;

        session.current_snapshot_id = None;
        session.snapshot_count = 0;

        log::debug!("Cleared snapshot tracking for session {}", session_id);

        Ok(())
    }

    /// Destroy a session and release resources.
    pub fn destroy_session(&self, session_id: &SessionId) -> RlmResult<()> {
        // Remove from sessions
        let session = self.sessions.remove(session_id);

        if session.is_none() {
            return Err(RlmError::SessionNotFound {
                session_id: *session_id,
            });
        }

        // Remove VM mappings
        if let Some(vm_id) = self.session_to_vm.remove(session_id) {
            self.vm_to_session.remove(&vm_id.1);
        }

        self.active_session_count.fetch_sub(1, Ordering::Relaxed);

        log::info!("Destroyed session: {}", session_id);
        Ok(())
    }

    /// Clean up expired sessions.
    pub fn cleanup_expired_sessions(&self) -> Vec<SessionId> {
        let now = Timestamp::now();
        let mut expired = Vec::new();

        self.sessions.retain(|session_id, session| {
            if session.expires_at < now {
                expired.push(*session_id);

                // Clean up VM mappings
                if let Some(vm_id) = &session.vm_instance_id {
                    self.vm_to_session.remove(vm_id);
                    self.session_to_vm.remove(session_id);
                }

                self.active_session_count.fetch_sub(1, Ordering::Relaxed);
                false
            } else {
                true
            }
        });

        if !expired.is_empty() {
            log::info!("Cleaned up {} expired sessions", expired.len());
        }

        expired
    }

    /// Get session statistics.
    pub fn get_stats(&self) -> SessionStats {
        SessionStats {
            total_sessions_created: self.total_sessions_created.load(Ordering::Relaxed),
            active_sessions: self.active_session_count.load(Ordering::Relaxed),
            sessions_with_vm: self.session_to_vm.len() as u32,
        }
    }

    /// List all active sessions.
    pub fn list_sessions(&self) -> Vec<SessionInfo> {
        self.sessions.iter().map(|r| r.clone()).collect()
    }
}

/// Session statistics.
#[derive(Debug, Clone)]
pub struct SessionStats {
    pub total_sessions_created: u32,
    pub active_sessions: u32,
    pub sessions_with_vm: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> RlmConfig {
        RlmConfig {
            session_duration_secs: 3600,
            extension_increment_secs: 1800,
            max_extensions: 3,
            ..Default::default()
        }
    }

    #[test]
    fn test_session_create_and_get() {
        let manager = SessionManager::new(test_config());

        let session = manager.create_session().unwrap();
        assert_eq!(session.state, SessionState::Initializing);

        let retrieved = manager.get_session(&session.id).unwrap();
        assert_eq!(retrieved.id, session.id);
    }

    #[test]
    fn test_session_validation() {
        let manager = SessionManager::new(test_config());

        let session = manager.create_session().unwrap();
        assert!(manager.is_session_valid(&session.id));

        let validated = manager.validate_session(&session.id);
        assert!(validated.is_ok());
    }

    #[test]
    fn test_session_extension() {
        let manager = SessionManager::new(test_config());

        let session = manager.create_session().unwrap();
        let original_expiry = session.expires_at;

        let extended = manager.extend_session(&session.id).unwrap();
        assert!(extended.expires_at > original_expiry);
        assert_eq!(extended.extension_count, 1);

        // Test max extensions
        manager.extend_session(&session.id).unwrap();
        manager.extend_session(&session.id).unwrap();

        let result = manager.extend_session(&session.id);
        assert!(matches!(result, Err(RlmError::MaxExtensionsReached { .. })));
    }

    #[test]
    fn test_vm_affinity() {
        let manager = SessionManager::new(test_config());

        let session = manager.create_session().unwrap();
        manager
            .assign_vm(&session.id, "vm-123".to_string())
            .unwrap();

        assert_eq!(
            manager.get_assigned_vm(&session.id),
            Some("vm-123".to_string())
        );
        assert_eq!(manager.get_session_for_vm("vm-123"), Some(session.id));
    }

    #[test]
    fn test_context_variables() {
        let manager = SessionManager::new(test_config());

        let session = manager.create_session().unwrap();
        manager
            .set_context_variable(&session.id, "key1".to_string(), "value1".to_string())
            .unwrap();

        let value = manager.get_context_variable(&session.id, "key1").unwrap();
        assert_eq!(value, Some("value1".to_string()));

        let all_vars = manager.get_all_context_variables(&session.id).unwrap();
        assert_eq!(all_vars.len(), 1);
    }

    #[test]
    fn test_recursion_depth() {
        let manager = SessionManager::new(RlmConfig {
            max_recursion_depth: 3,
            ..test_config()
        });

        let session = manager.create_session().unwrap();

        assert_eq!(manager.increment_recursion_depth(&session.id).unwrap(), 1);
        assert_eq!(manager.increment_recursion_depth(&session.id).unwrap(), 2);
        assert_eq!(manager.increment_recursion_depth(&session.id).unwrap(), 3);

        // Should fail on exceeding max depth
        let result = manager.increment_recursion_depth(&session.id);
        assert!(matches!(
            result,
            Err(RlmError::RecursionDepthExceeded { .. })
        ));

        // Decrement should work
        assert_eq!(manager.decrement_recursion_depth(&session.id).unwrap(), 2);
    }

    #[test]
    fn test_session_destroy() {
        let manager = SessionManager::new(test_config());

        let session = manager.create_session().unwrap();
        manager
            .assign_vm(&session.id, "vm-456".to_string())
            .unwrap();

        let stats_before = manager.get_stats();
        assert_eq!(stats_before.active_sessions, 1);

        manager.destroy_session(&session.id).unwrap();

        let stats_after = manager.get_stats();
        assert_eq!(stats_after.active_sessions, 0);

        // VM mapping should be cleaned up
        assert!(manager.get_session_for_vm("vm-456").is_none());
    }

    #[test]
    fn test_session_stats() {
        let manager = SessionManager::new(test_config());

        manager.create_session().unwrap();
        manager.create_session().unwrap();

        let stats = manager.get_stats();
        assert_eq!(stats.total_sessions_created, 2);
        assert_eq!(stats.active_sessions, 2);
    }

    #[test]
    fn test_snapshot_tracking() {
        let manager = SessionManager::new(test_config());
        let session = manager.create_session().unwrap();

        // Initial state - no snapshots
        assert!(manager.get_current_snapshot(&session.id).unwrap().is_none());
        let s = manager.get_session(&session.id).unwrap();
        assert_eq!(s.snapshot_count, 0);

        // Record a snapshot creation without setting as current
        manager
            .record_snapshot_created(&session.id, "snap1".to_string(), false)
            .unwrap();
        let s = manager.get_session(&session.id).unwrap();
        assert_eq!(s.snapshot_count, 1);
        assert!(s.current_snapshot_id.is_none());

        // Record a snapshot creation and set as current
        manager
            .record_snapshot_created(&session.id, "snap2".to_string(), true)
            .unwrap();
        let s = manager.get_session(&session.id).unwrap();
        assert_eq!(s.snapshot_count, 2);
        assert_eq!(s.current_snapshot_id, Some("snap2".to_string()));

        // Record a snapshot restore
        manager
            .record_snapshot_restored(&session.id, "snap1".to_string())
            .unwrap();
        let current = manager.get_current_snapshot(&session.id).unwrap();
        assert_eq!(current, Some("snap1".to_string()));

        // Clear snapshot tracking
        manager.clear_snapshot_tracking(&session.id).unwrap();
        let s = manager.get_session(&session.id).unwrap();
        assert_eq!(s.snapshot_count, 0);
        assert!(s.current_snapshot_id.is_none());
    }
}
