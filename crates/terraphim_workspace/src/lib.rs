//! Workspace management for Terraphim
//!
//! This crate provides workspace lifecycle management including:
//! - Workspace initialization and teardown
//! - Git branch management
//! - Lifecycle hooks (async callbacks)
//! - State tracking

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

pub mod git;

pub use git::GitWorkspace;

/// Errors that can occur during workspace operations
#[derive(thiserror::Error, Debug)]
pub enum WorkspaceError {
    #[error("Workspace initialization failed: {0}")]
    InitializationFailed(String),

    #[error("Workspace not found: {0}")]
    NotFound(PathBuf),

    #[error("Invalid workspace configuration: {0}")]
    InvalidConfiguration(String),

    #[error("Workspace state error: {0}")]
    StateError(String),

    #[error("Git operation failed: {0}")]
    GitError(#[from] git::GitError),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Hook execution failed: {0}")]
    HookFailed(String),
}

/// Result type for workspace operations
pub type Result<T> = std::result::Result<T, WorkspaceError>;

/// Workspace lifecycle states
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum WorkspaceState {
    /// Workspace has been created but not initialized
    Created,
    /// Workspace is being initialized
    Initializing,
    /// Workspace is ready for use
    Ready,
    /// Workspace has active operations running
    Running,
    /// Workspace is being cleaned up
    Cleaning,
    /// Workspace has been torn down
    TornDown,
}

impl WorkspaceState {
    /// Check if the workspace can transition to the target state
    pub fn can_transition_to(&self, target: WorkspaceState) -> bool {
        use WorkspaceState::*;
        match (*self, target) {
            (Created, Initializing) => true,
            (Created, TornDown) => true, // Can tear down without initializing
            (Initializing, Ready) => true,
            (Initializing, Cleaning) => true, // Can abort initialization
            (Ready, Running) => true,
            (Ready, Cleaning) => true,
            (Running, Ready) => true,
            (Running, Cleaning) => true,
            (Cleaning, TornDown) => true,
            (Cleaning, Ready) => true, // Can recover from cleaning
            (TornDown, Created) => true, // Can re-create
            _ => false,
        }
    }

    /// Check if this is a terminal state
    pub fn is_terminal(&self) -> bool {
        matches!(self, WorkspaceState::TornDown)
    }

    /// Check if the workspace is active (Ready or Running)
    pub fn is_active(&self) -> bool {
        matches!(self, WorkspaceState::Ready | WorkspaceState::Running)
    }
}

impl std::fmt::Display for WorkspaceState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WorkspaceState::Created => write!(f, "Created"),
            WorkspaceState::Initializing => write!(f, "Initializing"),
            WorkspaceState::Ready => write!(f, "Ready"),
            WorkspaceState::Running => write!(f, "Running"),
            WorkspaceState::Cleaning => write!(f, "Cleaning"),
            WorkspaceState::TornDown => write!(f, "TornDown"),
        }
    }
}

/// Workspace configuration
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WorkspaceConfig {
    /// Working directory for the workspace
    pub working_dir: PathBuf,
    /// Git branch to use (if any)
    pub git_branch: Option<String>,
    /// Whether to clean up on exit
    pub cleanup_on_exit: bool,
    /// Additional environment variables
    #[serde(default)]
    pub env_vars: HashMap<String, String>,
    /// Workspace name
    pub name: Option<String>,
    /// Maximum cleanup attempts
    #[serde(default = "default_max_cleanup_attempts")]
    pub max_cleanup_attempts: u32,
    /// Cleanup timeout in seconds
    #[serde(default = "default_cleanup_timeout_secs")]
    pub cleanup_timeout_secs: u64,
}

fn default_max_cleanup_attempts() -> u32 {
    3
}

fn default_cleanup_timeout_secs() -> u64 {
    30
}

impl WorkspaceConfig {
    /// Create a new workspace configuration
    pub fn new(working_dir: impl Into<PathBuf>) -> Self {
        Self {
            working_dir: working_dir.into(),
            git_branch: None,
            cleanup_on_exit: true,
            env_vars: HashMap::new(),
            name: None,
            max_cleanup_attempts: default_max_cleanup_attempts(),
            cleanup_timeout_secs: default_cleanup_timeout_secs(),
        }
    }

    /// Set the git branch
    pub fn with_git_branch(mut self, branch: impl Into<String>) -> Self {
        self.git_branch = Some(branch.into());
        self
    }

    /// Set cleanup on exit
    pub fn with_cleanup_on_exit(mut self, cleanup: bool) -> Self {
        self.cleanup_on_exit = cleanup;
        self
    }

    /// Set workspace name
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Add an environment variable
    pub fn with_env_var(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.env_vars.insert(key.into(), value.into());
        self
    }

    /// Validate the configuration
    pub fn validate(&self) -> Result<()> {
        // Check working directory exists or can be created
        if !self.working_dir.exists() && !self.working_dir.parent().map(|p| p.exists()).unwrap_or(false) {
            return Err(WorkspaceError::InvalidConfiguration(format!(
                "Working directory parent does not exist: {:?}",
                self.working_dir
            )));
        }

        // Validate name if provided
        if let Some(name) = &self.name {
            if name.is_empty() {
                return Err(WorkspaceError::InvalidConfiguration(
                    "Workspace name cannot be empty".to_string(),
                ));
            }
        }

        Ok(())
    }
}

/// Type alias for lifecycle hooks
pub type LifecycleHook = Arc<dyn Fn(WorkspaceContext) -> futures::future::BoxFuture<'static, Result<()>> + Send + Sync>;

/// Context passed to lifecycle hooks
#[derive(Debug, Clone)]
pub struct WorkspaceContext {
    /// Workspace ID
    pub id: uuid::Uuid,
    /// Workspace configuration
    pub config: WorkspaceConfig,
    /// Current state
    pub state: WorkspaceState,
    /// Working directory
    pub working_dir: PathBuf,
    /// Additional metadata
    pub metadata: HashMap<String, String>,
}

/// Workspace manager handles workspace lifecycle
pub struct WorkspaceManager {
    id: uuid::Uuid,
    config: WorkspaceConfig,
    state: Arc<RwLock<WorkspaceState>>,
    git: Option<Arc<RwLock<GitWorkspace>>>,
    /// Hook called on initialization
    on_init: Option<LifecycleHook>,
    /// Hook called when workspace becomes ready
    on_ready: Option<LifecycleHook>,
    /// Hook called on error
    on_error: Option<LifecycleHook>,
    /// Hook called on teardown
    on_teardown: Option<LifecycleHook>,
    /// Metadata storage
    metadata: Arc<RwLock<HashMap<String, String>>>,
}

impl WorkspaceManager {
    /// Create a new workspace manager
    pub fn new(config: WorkspaceConfig) -> Result<Self> {
        config.validate()?;

        let id = uuid::Uuid::new_v4();
        let git = if config.git_branch.is_some() || GitWorkspace::is_git_repo(&config.working_dir) {
            Some(Arc::new(RwLock::new(GitWorkspace::new(&config.working_dir)?)))
        } else {
            None
        };

        Ok(Self {
            id,
            config,
            state: Arc::new(RwLock::new(WorkspaceState::Created)),
            git,
            on_init: None,
            on_ready: None,
            on_error: None,
            on_teardown: None,
            metadata: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    /// Set the on_init hook
    pub fn on_init<F, Fut>(mut self, hook: F) -> Self
    where
        F: Fn(WorkspaceContext) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = Result<()>> + Send + 'static,
    {
        self.on_init = Some(Arc::new(move |ctx| {
            Box::pin(hook(ctx))
        }));
        self
    }

    /// Set the on_ready hook
    pub fn on_ready<F, Fut>(mut self, hook: F) -> Self
    where
        F: Fn(WorkspaceContext) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = Result<()>> + Send + 'static,
    {
        self.on_ready = Some(Arc::new(move |ctx| {
            Box::pin(hook(ctx))
        }));
        self
    }

    /// Set the on_error hook
    pub fn on_error<F, Fut>(mut self, hook: F) -> Self
    where
        F: Fn(WorkspaceContext) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = Result<()>> + Send + 'static,
    {
        self.on_error = Some(Arc::new(move |ctx| {
            Box::pin(hook(ctx))
        }));
        self
    }

    /// Set the on_teardown hook
    pub fn on_teardown<F, Fut>(mut self, hook: F) -> Self
    where
        F: Fn(WorkspaceContext) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = Result<()>> + Send + 'static,
    {
        self.on_teardown = Some(Arc::new(move |ctx| {
            Box::pin(hook(ctx))
        }));
        self
    }

    /// Get the workspace ID
    pub fn id(&self) -> uuid::Uuid {
        self.id
    }

    /// Get the current state
    pub async fn state(&self) -> WorkspaceState {
        *self.state.read().await
    }

    /// Get the workspace configuration
    pub fn config(&self) -> &WorkspaceConfig {
        &self.config
    }

    /// Get the working directory
    pub fn working_dir(&self) -> &Path {
        &self.config.working_dir
    }

    /// Get metadata value
    pub async fn get_metadata(&self, key: &str) -> Option<String> {
        self.metadata.read().await.get(key).cloned()
    }

    /// Set metadata value
    pub async fn set_metadata(&self, key: impl Into<String>, value: impl Into<String>) {
        self.metadata.write().await.insert(key.into(), value.into());
    }

    /// Create the workspace context for hooks
    fn create_context(&self, state: WorkspaceState) -> WorkspaceContext {
        WorkspaceContext {
            id: self.id,
            config: self.config.clone(),
            state,
            working_dir: self.config.working_dir.clone(),
            metadata: HashMap::new(), // Could be populated from self.metadata
        }
    }

    /// Initialize the workspace
    pub async fn initialize(&self) -> Result<()> {
        let mut state = self.state.write().await;
        
        if !state.can_transition_to(WorkspaceState::Initializing) {
            return Err(WorkspaceError::StateError(format!(
                "Cannot initialize workspace from state: {}",
                *state
            )));
        }

        info!(workspace_id = %self.id, "Initializing workspace");
        *state = WorkspaceState::Initializing;
        drop(state);

        // Ensure working directory exists
        if !self.config.working_dir.exists() {
            tokio::fs::create_dir_all(&self.config.working_dir).await?;
        }

        // Setup git branch if specified
        if let (Some(git), Some(branch)) = (&self.git, &self.config.git_branch) {
            info!(branch = %branch, "Setting up git branch");
            let git = git.read().await;
            git.checkout_or_create_branch(branch).await?;
        }

        // Call on_init hook
        if let Some(hook) = &self.on_init {
            let ctx = self.create_context(WorkspaceState::Initializing);
            if let Err(e) = hook(ctx).await {
                error!(error = %e, "on_init hook failed");
                self.handle_error().await?;
                return Err(e);
            }
        }

        // Transition to Ready
        let mut state = self.state.write().await;
        *state = WorkspaceState::Ready;
        drop(state);

        // Call on_ready hook
        if let Some(hook) = &self.on_ready {
            let ctx = self.create_context(WorkspaceState::Ready);
            if let Err(e) = hook(ctx).await {
                error!(error = %e, "on_ready hook failed");
                // Don't fail if on_ready fails, just log it
            }
        }

        info!(workspace_id = %self.id, "Workspace ready");
        Ok(())
    }

    /// Mark workspace as running
    pub async fn start_running(&self) -> Result<()> {
        let mut state = self.state.write().await;
        
        if !state.can_transition_to(WorkspaceState::Running) {
            return Err(WorkspaceError::StateError(format!(
                "Cannot start running from state: {}",
                *state
            )));
        }

        *state = WorkspaceState::Running;
        info!(workspace_id = %self.id, "Workspace is now running");
        Ok(())
    }

    /// Mark workspace as ready (done running)
    pub async fn stop_running(&self) -> Result<()> {
        let mut state = self.state.write().await;
        
        if !state.can_transition_to(WorkspaceState::Ready) {
            return Err(WorkspaceError::StateError(format!(
                "Cannot stop running from state: {}",
                *state
            )));
        }

        *state = WorkspaceState::Ready;
        info!(workspace_id = %self.id, "Workspace stopped running");
        Ok(())
    }

    /// Teardown the workspace
    pub async fn teardown(&self) -> Result<()> {
        let mut state = self.state.write().await;
        
        if !state.can_transition_to(WorkspaceState::Cleaning) {
            return Err(WorkspaceError::StateError(format!(
                "Cannot teardown from state: {}",
                *state
            )));
        }

        info!(workspace_id = %self.id, "Tearing down workspace");
        *state = WorkspaceState::Cleaning;
        drop(state);

        // Call on_teardown hook
        if let Some(hook) = &self.on_teardown {
            let ctx = self.create_context(WorkspaceState::Cleaning);
            if let Err(e) = hook(ctx).await {
                warn!(error = %e, "on_teardown hook failed");
                // Continue with teardown even if hook fails
            }
        }

        // Cleanup if enabled
        if self.config.cleanup_on_exit {
            self.cleanup().await?;
        }

        // Mark as torn down
        let mut state = self.state.write().await;
        *state = WorkspaceState::TornDown;

        info!(workspace_id = %self.id, "Workspace torn down");
        Ok(())
    }

    /// Handle error state
    async fn handle_error(&self) -> Result<()> {
        if let Some(hook) = &self.on_error {
            let ctx = self.create_context(WorkspaceState::Cleaning);
            let _ = hook(ctx).await;
        }
        Ok(())
    }

    /// Cleanup workspace resources
    async fn cleanup(&self) -> Result<()> {
        debug!(workspace_id = %self.id, "Cleaning up workspace resources");

        // Restore git state if needed
        if let Some(git) = &self.git {
            let mut git = git.write().await;
            if let Err(e) = git.restore_state().await {
                warn!(error = %e, "Failed to restore git state");
            }
        }

        // Additional cleanup could be added here
        // (e.g., removing temporary files, closing handles, etc.)

        Ok(())
    }

    /// Get the git workspace (if available)
    pub fn git(&self) -> Option<&Arc<RwLock<GitWorkspace>>> {
        self.git.as_ref()
    }
}

impl std::fmt::Debug for WorkspaceManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WorkspaceManager")
            .field("id", &self.id)
            .field("config", &self.config)
            .field("has_on_init", &self.on_init.is_some())
            .field("has_on_ready", &self.on_ready.is_some())
            .field("has_on_error", &self.on_error.is_some())
            .field("has_on_teardown", &self.on_teardown.is_some())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};

    #[test]
    fn test_workspace_state_transitions() {
        use WorkspaceState::*;

        // Valid transitions
        assert!(Created.can_transition_to(Initializing));
        assert!(Created.can_transition_to(TornDown));
        assert!(Initializing.can_transition_to(Ready));
        assert!(Initializing.can_transition_to(Cleaning));
        assert!(Ready.can_transition_to(Running));
        assert!(Ready.can_transition_to(Cleaning));
        assert!(Running.can_transition_to(Ready));
        assert!(Running.can_transition_to(Cleaning));
        assert!(Cleaning.can_transition_to(TornDown));
        assert!(TornDown.can_transition_to(Created));

        // Invalid transitions
        assert!(!Created.can_transition_to(Running));
        assert!(!Created.can_transition_to(Ready));
        assert!(!Initializing.can_transition_to(Running));
        assert!(!Ready.can_transition_to(Initializing));
        assert!(!Running.can_transition_to(Initializing));
        assert!(!TornDown.can_transition_to(Running));
        assert!(!TornDown.can_transition_to(Ready));
    }

    #[test]
    fn test_workspace_state_properties() {
        assert!(WorkspaceState::TornDown.is_terminal());
        assert!(!WorkspaceState::Ready.is_terminal());
        assert!(!WorkspaceState::Running.is_terminal());

        assert!(WorkspaceState::Ready.is_active());
        assert!(WorkspaceState::Running.is_active());
        assert!(!WorkspaceState::Created.is_active());
        assert!(!WorkspaceState::TornDown.is_active());
    }

    #[test]
    fn test_workspace_config_validation() {
        // Valid config with existing directory
        let temp_dir = std::env::temp_dir();
        let config = WorkspaceConfig::new(&temp_dir);
        assert!(config.validate().is_ok());

        // Valid config with non-existent but creatable directory
        let new_dir = temp_dir.join("terraphim_test_workspace_new");
        let config = WorkspaceConfig::new(&new_dir);
        assert!(config.validate().is_ok());

        // Invalid config with empty name
        let config = WorkspaceConfig::new(&temp_dir).with_name("");
        assert!(config.validate().is_err());

        // Config with valid name
        let config = WorkspaceConfig::new(&temp_dir)
            .with_name("test-workspace")
            .with_git_branch("main")
            .with_cleanup_on_exit(true);
        assert!(config.validate().is_ok());
        assert_eq!(config.name, Some("test-workspace".to_string()));
        assert_eq!(config.git_branch, Some("main".to_string()));
        assert!(config.cleanup_on_exit);
    }

    #[tokio::test]
    async fn test_workspace_lifecycle_transitions() {
        let temp_dir = tempfile::tempdir().unwrap();
        let config = WorkspaceConfig::new(temp_dir.path());
        let manager = WorkspaceManager::new(config).unwrap();

        assert_eq!(manager.state().await, WorkspaceState::Created);

        // Initialize
        manager.initialize().await.unwrap();
        assert_eq!(manager.state().await, WorkspaceState::Ready);

        // Start running
        manager.start_running().await.unwrap();
        assert_eq!(manager.state().await, WorkspaceState::Running);

        // Stop running
        manager.stop_running().await.unwrap();
        assert_eq!(manager.state().await, WorkspaceState::Ready);

        // Teardown
        manager.teardown().await.unwrap();
        assert_eq!(manager.state().await, WorkspaceState::TornDown);
    }

    #[tokio::test]
    async fn test_workspace_hooks() {
        let temp_dir = tempfile::tempdir().unwrap();
        let config = WorkspaceConfig::new(temp_dir.path());

        let init_called = Arc::new(AtomicBool::new(false));
        let ready_called = Arc::new(AtomicBool::new(false));
        let teardown_called = Arc::new(AtomicBool::new(false));

        let init_flag = init_called.clone();
        let ready_flag = ready_called.clone();
        let teardown_flag = teardown_called.clone();

        let manager = WorkspaceManager::new(config)
            .unwrap()
            .on_init(move |_ctx| {
                let flag = init_flag.clone();
                async move {
                    flag.store(true, Ordering::SeqCst);
                    Ok(())
                }
            })
            .on_ready(move |_ctx| {
                let flag = ready_flag.clone();
                async move {
                    flag.store(true, Ordering::SeqCst);
                    Ok(())
                }
            })
            .on_teardown(move |_ctx| {
                let flag = teardown_flag.clone();
                async move {
                    flag.store(true, Ordering::SeqCst);
                    Ok(())
                }
            });

        manager.initialize().await.unwrap();
        assert!(init_called.load(Ordering::SeqCst));
        assert!(ready_called.load(Ordering::SeqCst));

        manager.teardown().await.unwrap();
        assert!(teardown_called.load(Ordering::SeqCst));
    }

    #[tokio::test]
    async fn test_workspace_metadata() {
        let temp_dir = tempfile::tempdir().unwrap();
        let config = WorkspaceConfig::new(temp_dir.path());
        let manager = WorkspaceManager::new(config).unwrap();

        // Set and get metadata
        manager.set_metadata("key1", "value1").await;
        manager.set_metadata("key2", "value2").await;

        assert_eq!(manager.get_metadata("key1").await, Some("value1".to_string()));
        assert_eq!(manager.get_metadata("key2").await, Some("value2".to_string()));
        assert_eq!(manager.get_metadata("nonexistent").await, None);
    }

    #[tokio::test]
    async fn test_invalid_state_transitions() {
        let temp_dir = tempfile::tempdir().unwrap();
        let config = WorkspaceConfig::new(temp_dir.path());
        let manager = WorkspaceManager::new(config).unwrap();

        // Cannot go from Created to Running
        assert!(manager.start_running().await.is_err());

        // Initialize first
        manager.initialize().await.unwrap();

        // Cannot initialize twice
        assert!(manager.initialize().await.is_err());

        // Start and stop running
        manager.start_running().await.unwrap();
        manager.stop_running().await.unwrap();

        // Teardown
        manager.teardown().await.unwrap();

        // Cannot teardown twice
        assert!(manager.teardown().await.is_err());
    }

    #[tokio::test]
    async fn test_workspace_without_cleanup() {
        let temp_dir = tempfile::tempdir().unwrap();
        let config = WorkspaceConfig::new(temp_dir.path())
            .with_cleanup_on_exit(false);
        let manager = WorkspaceManager::new(config).unwrap();

        manager.initialize().await.unwrap();
        manager.teardown().await.unwrap();

        // Directory should still exist since cleanup_on_exit is false
        assert!(temp_dir.path().exists());
    }
}
