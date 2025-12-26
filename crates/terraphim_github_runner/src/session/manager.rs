//! Session management for VM-based workflow execution
//!
//! Manages VM allocation lifecycle and session tracking for GitHub workflow execution.

use crate::error::{GitHubRunnerError, Result};
use crate::models::{SessionId, SnapshotId, WorkflowContext};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use dashmap::DashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

/// Active session state
#[derive(Debug, Clone)]
pub struct Session {
    /// Unique session identifier
    pub id: SessionId,
    /// Associated VM instance ID
    pub vm_id: String,
    /// VM type used
    pub vm_type: String,
    /// When the session started
    pub started_at: DateTime<Utc>,
    /// Current session state
    pub state: SessionState,
    /// Snapshots taken during this session
    pub snapshots: Vec<SnapshotId>,
    /// Last activity timestamp
    pub last_activity: DateTime<Utc>,
}

/// State of a session
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionState {
    /// Session is being initialized
    Initializing,
    /// Session is active and ready
    Active,
    /// Session is executing a command
    Executing,
    /// Session is paused (snapshot taken)
    Paused,
    /// Session completed successfully
    Completed,
    /// Session failed
    Failed,
    /// Session was rolled back
    RolledBack,
    /// Session is being terminated
    Terminating,
}

/// Trait for VM allocation providers
#[async_trait]
pub trait VmProvider: Send + Sync {
    /// Allocate a VM of the given type
    async fn allocate(&self, vm_type: &str) -> Result<(String, Duration)>;
    /// Release a VM by ID
    async fn release(&self, vm_id: &str) -> Result<()>;
}

/// Mock VM provider for testing
pub struct MockVmProvider;

#[async_trait]
impl VmProvider for MockVmProvider {
    async fn allocate(&self, _vm_type: &str) -> Result<(String, Duration)> {
        Ok((
            format!("mock-vm-{}", uuid::Uuid::new_v4()),
            Duration::from_millis(50),
        ))
    }

    async fn release(&self, _vm_id: &str) -> Result<()> {
        Ok(())
    }
}

/// Configuration for the session manager
#[derive(Debug, Clone)]
pub struct SessionManagerConfig {
    /// Default VM type to use
    pub default_vm_type: String,
    /// Session timeout (auto-release if no activity)
    pub session_timeout: Duration,
    /// Maximum concurrent sessions
    pub max_concurrent_sessions: usize,
    /// Enable automatic cleanup of stale sessions
    pub auto_cleanup: bool,
    /// Cleanup interval
    pub cleanup_interval: Duration,
}

impl Default for SessionManagerConfig {
    fn default() -> Self {
        Self {
            default_vm_type: "focal-optimized".to_string(),
            session_timeout: Duration::from_secs(3600), // 1 hour
            max_concurrent_sessions: 10,
            auto_cleanup: true,
            cleanup_interval: Duration::from_secs(60),
        }
    }
}

/// Manages VM sessions for workflow execution
pub struct SessionManager {
    /// VM provider for allocation
    vm_provider: Arc<dyn VmProvider>,
    /// Active sessions by session ID
    sessions: DashMap<SessionId, Session>,
    /// Configuration
    config: SessionManagerConfig,
    /// Whether the manager is initialized
    initialized: RwLock<bool>,
}

impl SessionManager {
    /// Create a new session manager with mock VM provider (for testing)
    pub fn new(config: SessionManagerConfig) -> Self {
        Self {
            vm_provider: Arc::new(MockVmProvider),
            sessions: DashMap::new(),
            config,
            initialized: RwLock::new(true),
        }
    }

    /// Create a new session manager with a custom VM provider
    pub fn with_provider(vm_provider: Arc<dyn VmProvider>, config: SessionManagerConfig) -> Self {
        Self {
            vm_provider,
            sessions: DashMap::new(),
            config,
            initialized: RwLock::new(true),
        }
    }

    /// Initialize the session manager
    pub async fn initialize(&self) -> Result<()> {
        *self.initialized.write().await = true;
        Ok(())
    }

    /// Create a new session for a workflow
    pub async fn create_session(&self, context: &WorkflowContext) -> Result<Session> {
        // Check concurrent session limit
        if self.sessions.len() >= self.config.max_concurrent_sessions {
            return Err(GitHubRunnerError::VmAllocation(format!(
                "Maximum concurrent sessions ({}) reached",
                self.config.max_concurrent_sessions
            )));
        }

        let session_id = context.session_id.clone();
        let vm_type = self.config.default_vm_type.clone();

        // Allocate a VM
        let (vm_id, allocation_time) = self.vm_provider.allocate(&vm_type).await?;

        log::info!(
            "Allocated VM {} in {:?} for session {}",
            vm_id,
            allocation_time,
            session_id
        );

        let now = Utc::now();
        let session = Session {
            id: session_id.clone(),
            vm_id,
            vm_type,
            started_at: now,
            state: SessionState::Active,
            snapshots: Vec::new(),
            last_activity: now,
        };

        self.sessions.insert(session_id, session.clone());

        Ok(session)
    }

    /// Get an existing session
    pub fn get_session(&self, session_id: &SessionId) -> Option<Session> {
        self.sessions.get(session_id).map(|s| s.clone())
    }

    /// Update session state
    pub fn update_session_state(&self, session_id: &SessionId, state: SessionState) -> Result<()> {
        let mut session = self.sessions.get_mut(session_id).ok_or_else(|| {
            GitHubRunnerError::SessionNotFound {
                session_id: session_id.to_string(),
            }
        })?;

        session.state = state;
        session.last_activity = Utc::now();

        Ok(())
    }

    /// Record a snapshot for a session
    pub fn add_snapshot(&self, session_id: &SessionId, snapshot_id: SnapshotId) -> Result<()> {
        let mut session = self.sessions.get_mut(session_id).ok_or_else(|| {
            GitHubRunnerError::SessionNotFound {
                session_id: session_id.to_string(),
            }
        })?;

        session.snapshots.push(snapshot_id);
        session.last_activity = Utc::now();

        Ok(())
    }

    /// Get the last snapshot for a session
    pub fn get_last_snapshot(&self, session_id: &SessionId) -> Option<SnapshotId> {
        self.sessions
            .get(session_id)
            .and_then(|s| s.snapshots.last().cloned())
    }

    /// Release a session and its VM
    pub async fn release_session(&self, session_id: &SessionId) -> Result<()> {
        let session = self
            .sessions
            .remove(session_id)
            .ok_or_else(|| GitHubRunnerError::SessionNotFound {
                session_id: session_id.to_string(),
            })?
            .1;

        // Release the VM
        self.vm_provider.release(&session.vm_id).await?;

        log::info!("Released session {} with VM {}", session_id, session.vm_id);

        Ok(())
    }

    /// Get the number of active sessions
    pub fn active_session_count(&self) -> usize {
        self.sessions.len()
    }

    /// Get all active session IDs
    pub fn active_session_ids(&self) -> Vec<SessionId> {
        self.sessions.iter().map(|s| s.key().clone()).collect()
    }

    /// Cleanup stale sessions (sessions that have been inactive too long)
    pub async fn cleanup_stale_sessions(&self) -> Result<usize> {
        let now = Utc::now();
        let timeout = chrono::Duration::from_std(self.config.session_timeout)
            .unwrap_or(chrono::Duration::hours(1));

        let stale_sessions: Vec<SessionId> = self
            .sessions
            .iter()
            .filter(|s| {
                let elapsed = now - s.last_activity;
                elapsed > timeout
            })
            .map(|s| s.key().clone())
            .collect();

        let count = stale_sessions.len();

        for session_id in stale_sessions {
            if let Err(e) = self.release_session(&session_id).await {
                log::warn!("Failed to release stale session {}: {}", session_id, e);
            }
        }

        if count > 0 {
            log::info!("Cleaned up {} stale sessions", count);
        }

        Ok(count)
    }

    /// Get session statistics
    pub fn get_stats(&self) -> SessionStats {
        let sessions: Vec<_> = self.sessions.iter().map(|s| s.value().clone()).collect();

        let mut stats = SessionStats {
            total_sessions: sessions.len(),
            active_sessions: 0,
            executing_sessions: 0,
            completed_sessions: 0,
            failed_sessions: 0,
            total_snapshots: 0,
        };

        for session in sessions {
            match session.state {
                SessionState::Active => stats.active_sessions += 1,
                SessionState::Executing => stats.executing_sessions += 1,
                SessionState::Completed => stats.completed_sessions += 1,
                SessionState::Failed | SessionState::RolledBack => stats.failed_sessions += 1,
                _ => {}
            }
            stats.total_snapshots += session.snapshots.len();
        }

        stats
    }
}

/// Session statistics
#[derive(Debug, Clone, Default)]
pub struct SessionStats {
    /// Total number of tracked sessions
    pub total_sessions: usize,
    /// Sessions in active state
    pub active_sessions: usize,
    /// Sessions currently executing
    pub executing_sessions: usize,
    /// Sessions that completed successfully
    pub completed_sessions: usize,
    /// Sessions that failed
    pub failed_sessions: usize,
    /// Total snapshots across all sessions
    pub total_snapshots: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{GitHubEvent, GitHubEventType, RepositoryInfo};
    use std::collections::HashMap;

    fn create_test_event() -> GitHubEvent {
        GitHubEvent {
            event_type: GitHubEventType::PullRequest,
            action: Some("opened".to_string()),
            repository: RepositoryInfo {
                full_name: "test/repo".to_string(),
                clone_url: None,
                default_branch: Some("main".to_string()),
            },
            pull_request: None,
            git_ref: None,
            sha: Some("abc123".to_string()),
            extra: HashMap::new(),
        }
    }

    #[tokio::test]
    async fn test_create_session() {
        let manager = SessionManager::new(SessionManagerConfig::default());
        let context = WorkflowContext::new(create_test_event());

        let session = manager.create_session(&context).await.unwrap();
        assert_eq!(session.id, context.session_id);
        assert_eq!(session.state, SessionState::Active);
        assert!(session.snapshots.is_empty());
    }

    #[tokio::test]
    async fn test_get_session() {
        let manager = SessionManager::new(SessionManagerConfig::default());
        let context = WorkflowContext::new(create_test_event());

        let created = manager.create_session(&context).await.unwrap();
        let retrieved = manager.get_session(&created.id);

        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().id, created.id);
    }

    #[tokio::test]
    async fn test_update_session_state() {
        let manager = SessionManager::new(SessionManagerConfig::default());
        let context = WorkflowContext::new(create_test_event());

        let session = manager.create_session(&context).await.unwrap();
        manager
            .update_session_state(&session.id, SessionState::Executing)
            .unwrap();

        let updated = manager.get_session(&session.id).unwrap();
        assert_eq!(updated.state, SessionState::Executing);
    }

    #[tokio::test]
    async fn test_add_snapshot() {
        let manager = SessionManager::new(SessionManagerConfig::default());
        let context = WorkflowContext::new(create_test_event());

        let session = manager.create_session(&context).await.unwrap();
        let snapshot_id = SnapshotId::new("snap-1".to_string());

        manager
            .add_snapshot(&session.id, snapshot_id.clone())
            .unwrap();

        let last = manager.get_last_snapshot(&session.id);
        assert_eq!(last, Some(snapshot_id));
    }

    #[tokio::test]
    async fn test_release_session() {
        let manager = SessionManager::new(SessionManagerConfig::default());
        let context = WorkflowContext::new(create_test_event());

        let session = manager.create_session(&context).await.unwrap();
        assert_eq!(manager.active_session_count(), 1);

        manager.release_session(&session.id).await.unwrap();
        assert_eq!(manager.active_session_count(), 0);
    }

    #[tokio::test]
    async fn test_max_concurrent_sessions() {
        let config = SessionManagerConfig {
            max_concurrent_sessions: 2,
            ..Default::default()
        };
        let manager = SessionManager::new(config);

        // Create 2 sessions
        let event1 = create_test_event();
        let event2 = create_test_event();
        let event3 = create_test_event();

        let ctx1 = WorkflowContext::new(event1);
        let ctx2 = WorkflowContext::new(event2);
        let ctx3 = WorkflowContext::new(event3);

        manager.create_session(&ctx1).await.unwrap();
        manager.create_session(&ctx2).await.unwrap();

        // Third should fail
        let result = manager.create_session(&ctx3).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_session_stats() {
        let manager = SessionManager::new(SessionManagerConfig::default());
        let context = WorkflowContext::new(create_test_event());

        let session = manager.create_session(&context).await.unwrap();
        manager
            .add_snapshot(&session.id, SnapshotId::new("snap-1".to_string()))
            .unwrap();
        manager
            .add_snapshot(&session.id, SnapshotId::new("snap-2".to_string()))
            .unwrap();

        let stats = manager.get_stats();
        assert_eq!(stats.total_sessions, 1);
        assert_eq!(stats.active_sessions, 1);
        assert_eq!(stats.total_snapshots, 2);
    }
}
