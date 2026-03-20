use std::collections::HashMap;
use std::time::{Duration, Instant};

use tracing::{info, warn};

/// Tracks session information for an agent.
#[derive(Debug, Clone)]
pub struct AgentSession {
    /// Unique session ID.
    pub session_id: String,
    /// When the session started.
    pub started_at: Instant,
    /// Number of completed sessions (rotations) for this agent.
    pub completed_sessions: u32,
    /// Number of completions since last rotation.
    pub completions_since_rotation: u32,
    /// Accumulated state (context) for this session.
    pub context: HashMap<String, String>,
}

impl AgentSession {
    /// Create a new session with a generated ID.
    pub fn new(completed_sessions: u32) -> Self {
        Self {
            session_id: format!("session-{}", uuid::Uuid::new_v4()),
            started_at: Instant::now(),
            completed_sessions,
            completions_since_rotation: 0,
            context: HashMap::new(),
        }
    }

    /// Record a completion and return whether rotation is needed.
    pub fn record_completion(&mut self, max_sessions: u32) -> bool {
        self.completions_since_rotation += 1;
        self.completions_since_rotation >= max_sessions
    }

    /// Rotate to a new session, clearing accumulated context.
    pub fn rotate(&mut self) {
        self.completed_sessions += 1;
        self.completions_since_rotation = 0;
        self.session_id = format!("session-{}", uuid::Uuid::new_v4());
        self.started_at = Instant::now();
        self.context.clear();
        info!(
            session_id = %self.session_id,
            completed = self.completed_sessions,
            "session rotated"
        );
    }

    /// Check if this session has exceeded the maximum lifetime.
    pub fn should_rotate(&self, max_sessions: u32, max_duration: Option<Duration>) -> bool {
        if self.completions_since_rotation >= max_sessions {
            return true;
        }

        if let Some(max_dur) = max_duration {
            if self.started_at.elapsed() >= max_dur {
                return true;
            }
        }

        false
    }

    /// Get the uptime of the current session.
    pub fn uptime(&self) -> Duration {
        self.started_at.elapsed()
    }

    /// Set a context value.
    pub fn set_context(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.context.insert(key.into(), value.into());
    }

    /// Get a context value.
    pub fn get_context(&self, key: &str) -> Option<&String> {
        self.context.get(key)
    }
}

/// Manages session rotation for all agents.
pub struct SessionRotationManager {
    /// Maximum number of sessions before rotation (0 = disabled).
    pub max_sessions_before_rotation: u32,
    /// Optional maximum duration for a session.
    pub max_session_duration: Option<Duration>,
    /// Current sessions for each agent.
    sessions: HashMap<String, AgentSession>,
}

impl SessionRotationManager {
    /// Create a new session rotation manager.
    pub fn new(max_sessions_before_rotation: u32) -> Self {
        info!(
            max_sessions = max_sessions_before_rotation,
            "session rotation manager initialized"
        );

        Self {
            max_sessions_before_rotation,
            max_session_duration: None,
            sessions: HashMap::new(),
        }
    }

    /// Create with a maximum session duration.
    pub fn with_duration(mut self, duration: Duration) -> Self {
        self.max_session_duration = Some(duration);
        self
    }

    /// Get or create a session for an agent.
    pub fn get_or_create_session(&mut self, agent_name: &str) -> &mut AgentSession {
        self.sessions
            .entry(agent_name.to_string())
            .or_insert_with(|| AgentSession::new(0))
    }

    /// Check if an agent needs session rotation and perform it if needed.
    /// Returns true if rotation was performed.
    pub fn check_and_rotate(&mut self, agent_name: &str) -> bool {
        if self.max_sessions_before_rotation == 0 {
            return false; // Rotation disabled
        }

        if let Some(session) = self.sessions.get_mut(agent_name) {
            if session.should_rotate(self.max_sessions_before_rotation, self.max_session_duration) {
                warn!(
                    agent = %agent_name,
                    completed = session.completed_sessions,
                    max = self.max_sessions_before_rotation,
                    "performing session rotation"
                );
                session.rotate();
                return true;
            }
        }

        false
    }

    /// Record an agent completion and check rotation.
    /// This should be called when an agent completes its task.
    /// Returns true if rotation was performed.
    pub fn on_agent_completion(&mut self, agent_name: &str) -> bool {
        // Store max value to avoid borrow issues
        let max_sessions = self.max_sessions_before_rotation;

        if max_sessions == 0 {
            // Get or create session but don't rotate
            self.get_or_create_session(agent_name);
            return false;
        }

        // Get or create the session
        let session = self.get_or_create_session(agent_name);

        // Record the completion
        let should_rotate = session.record_completion(max_sessions);

        if should_rotate {
            warn!(
                agent = %agent_name,
                max = max_sessions,
                "session rotation triggered after agent completion"
            );
            // Get mutable reference and rotate
            if let Some(session) = self.sessions.get_mut(agent_name) {
                session.rotate();
                return true;
            }
        }

        false
    }

    /// Get session info for an agent.
    pub fn get_session(&self, agent_name: &str) -> Option<&AgentSession> {
        self.sessions.get(agent_name)
    }

    /// Get all agent names with active sessions.
    pub fn active_agents(&self) -> Vec<&String> {
        self.sessions.keys().collect()
    }

    /// Force rotation for a specific agent.
    pub fn force_rotation(&mut self, agent_name: &str) {
        if let Some(session) = self.sessions.get_mut(agent_name) {
            info!(agent = %agent_name, "forcing session rotation");
            session.rotate();
        }
    }

    /// Get the total number of tracked sessions.
    pub fn session_count(&self) -> usize {
        self.sessions.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_creation() {
        let session = AgentSession::new(0);
        assert!(!session.session_id.is_empty());
        assert_eq!(session.completed_sessions, 0);
        assert!(session.context.is_empty());
    }

    #[test]
    fn test_session_rotation() {
        let mut session = AgentSession::new(0);
        let old_id = session.session_id.clone();

        session.rotate();

        assert_ne!(session.session_id, old_id);
        assert_eq!(session.completed_sessions, 1);
        assert!(session.context.is_empty());
    }

    #[test]
    fn test_should_rotate_by_count() {
        let mut session = AgentSession::new(0);
        assert!(!session.should_rotate(10, None));

        // Set completions to threshold
        session.completions_since_rotation = 10;

        assert!(session.should_rotate(10, None));
    }

    #[test]
    fn test_should_rotate_by_duration() {
        let session = AgentSession::new(0);

        // Should not rotate with long duration
        assert!(!session.should_rotate(100, Some(Duration::from_secs(3600))));

        // Should rotate with very short duration (0 seconds)
        // Note: this will be true because some time has elapsed
        assert!(session.should_rotate(100, Some(Duration::from_nanos(1))));
    }

    #[test]
    fn test_session_context() {
        let mut session = AgentSession::new(0);

        session.set_context("key1", "value1");
        session.set_context("key2", "value2");

        assert_eq!(session.get_context("key1"), Some(&"value1".to_string()));
        assert_eq!(session.get_context("key2"), Some(&"value2".to_string()));
        assert_eq!(session.get_context("key3"), None);

        // After rotation, context should be cleared
        session.rotate();
        assert_eq!(session.get_context("key1"), None);
    }

    #[test]
    fn test_rotation_manager_creation() {
        let manager = SessionRotationManager::new(10);
        assert_eq!(manager.max_sessions_before_rotation, 10);
        assert!(manager.max_session_duration.is_none());
        assert_eq!(manager.session_count(), 0);
    }

    #[test]
    fn test_rotation_manager_with_duration() {
        let manager = SessionRotationManager::new(10).with_duration(Duration::from_secs(300));
        assert_eq!(manager.max_session_duration, Some(Duration::from_secs(300)));
    }

    #[test]
    fn test_get_or_create_session() {
        let mut manager = SessionRotationManager::new(10);

        let session = manager.get_or_create_session("agent1");
        assert_eq!(session.completed_sessions, 0);
        let session_id = session.session_id.clone();

        // Get same session again
        let session2 = manager.get_or_create_session("agent1");
        assert_eq!(session2.session_id, session_id);

        assert_eq!(manager.session_count(), 1);
    }

    #[test]
    fn test_check_and_rotate_disabled() {
        let mut manager = SessionRotationManager::new(0); // Disabled
        manager.get_or_create_session("agent1");

        assert!(!manager.check_and_rotate("agent1"));
    }

    #[test]
    fn test_check_and_rotation_triggered() {
        let mut manager = SessionRotationManager::new(5);
        manager.get_or_create_session("agent1");

        // Manually set completions to 5 (threshold)
        if let Some(session) = manager.sessions.get_mut("agent1") {
            session.completions_since_rotation = 5;
        }

        // At 5 completions, should rotate
        assert!(manager.check_and_rotate("agent1"));
    }

    #[test]
    fn test_on_agent_completion() {
        let mut manager = SessionRotationManager::new(3);
        manager.get_or_create_session("agent1");

        // First completion - no rotation (completions = 1 < 3)
        assert!(!manager.on_agent_completion("agent1"));

        // Second completion - no rotation (completions = 2 < 3)
        assert!(!manager.on_agent_completion("agent1"));

        // Third completion - should trigger rotation (completions = 3 >= 3)
        assert!(manager.on_agent_completion("agent1"));

        // Fourth completion - no rotation (after rotate, completions = 1 < 3)
        assert!(!manager.on_agent_completion("agent1"));
    }

    #[test]
    fn test_on_agent_completion_disabled() {
        let mut manager = SessionRotationManager::new(0); // Disabled
        manager.get_or_create_session("agent1");

        // Should never rotate
        assert!(!manager.on_agent_completion("agent1"));
        assert!(!manager.on_agent_completion("agent1"));
        assert!(!manager.on_agent_completion("agent1"));
    }

    #[test]
    fn test_force_rotation() {
        let mut manager = SessionRotationManager::new(10);
        let session = manager.get_or_create_session("agent1");
        let old_id = session.session_id.clone();

        manager.force_rotation("agent1");

        let new_session = manager.get_session("agent1").unwrap();
        assert_ne!(new_session.session_id, old_id);
        assert_eq!(new_session.completed_sessions, 1);
    }

    #[test]
    fn test_active_agents() {
        let mut manager = SessionRotationManager::new(10);

        manager.get_or_create_session("agent1");
        manager.get_or_create_session("agent2");
        manager.get_or_create_session("agent3");

        let active = manager.active_agents();
        assert_eq!(active.len(), 3);
        assert!(active.contains(&&"agent1".to_string()));
        assert!(active.contains(&&"agent2".to_string()));
        assert!(active.contains(&&"agent3".to_string()));
    }
}
