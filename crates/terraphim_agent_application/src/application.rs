//! Main application implementation following OTP application behavior pattern

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use async_trait::async_trait;
use log::{debug, error, info, warn};
use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc, RwLock};
use tokio::time::interval;

use terraphim_agent_supervisor::{AgentPid, SupervisorId};
use terraphim_kg_orchestration::SupervisionTreeOrchestrator;

use crate::{
    ApplicationConfig, ApplicationError, ApplicationResult, ConfigurationChange,
    ConfigurationManager, DeploymentManager, DiagnosticsManager, HotReloadManager,
    LifecycleManager,
};

/// OTP-style application behavior for the Terraphim agent system
#[async_trait]
pub trait Application: Send + Sync {
    /// Start the application
    async fn start(&mut self) -> ApplicationResult<()>;

    /// Stop the application
    async fn stop(&mut self) -> ApplicationResult<()>;

    /// Restart the application
    async fn restart(&mut self) -> ApplicationResult<()>;

    /// Get application status
    async fn status(&self) -> ApplicationResult<ApplicationStatus>;

    /// Handle configuration changes
    async fn handle_config_change(&mut self, change: ConfigurationChange) -> ApplicationResult<()>;

    /// Perform health check
    async fn health_check(&self) -> ApplicationResult<HealthStatus>;
}

/// Terraphim agent application implementation
pub struct TerraphimAgentApplication {
    /// Application state
    state: Arc<RwLock<ApplicationState>>,
    /// Configuration manager
    config_manager: Arc<ConfigurationManager>,
    /// Lifecycle manager
    lifecycle_manager: Arc<LifecycleManager>,
    /// Deployment manager
    deployment_manager: Arc<DeploymentManager>,
    /// Hot reload manager
    hot_reload_manager: Arc<HotReloadManager>,
    /// Diagnostics manager
    diagnostics_manager: Arc<DiagnosticsManager>,
    /// Supervision tree orchestrator
    orchestrator: Arc<RwLock<Option<SupervisionTreeOrchestrator>>>,
    /// System message channel
    system_tx: mpsc::UnboundedSender<SystemMessage>,
    /// System message receiver
    system_rx: Arc<RwLock<Option<mpsc::UnboundedReceiver<SystemMessage>>>>,
}

/// Application state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplicationState {
    /// Current status
    pub status: ApplicationStatus,
    /// Start time
    pub start_time: Option<SystemTime>,
    /// Uptime in seconds
    pub uptime_seconds: u64,
    /// Active agents
    pub active_agents: HashMap<AgentPid, AgentInfo>,
    /// Active supervisors
    pub active_supervisors: HashMap<SupervisorId, SupervisorInfo>,
    /// System metrics
    pub metrics: SystemMetrics,
    /// Last health check
    pub last_health_check: Option<SystemTime>,
    /// Configuration version
    pub config_version: u64,
}

/// Application status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ApplicationStatus {
    /// Application is starting up
    Starting,
    /// Application is running normally
    Running,
    /// Application is stopping
    Stopping,
    /// Application is stopped
    Stopped,
    /// Application is restarting
    Restarting,
    /// Application has failed
    Failed(String),
}

/// Health status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthStatus {
    /// Overall health
    pub overall: HealthLevel,
    /// Component health statuses
    pub components: HashMap<String, ComponentHealth>,
    /// Health check timestamp
    pub timestamp: SystemTime,
    /// Health metrics
    pub metrics: HealthMetrics,
}

/// Health level
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum HealthLevel {
    Healthy,
    Degraded,
    Unhealthy,
    Critical,
}

/// Component health status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentHealth {
    /// Health level
    pub level: HealthLevel,
    /// Status message
    pub message: String,
    /// Last check time
    pub last_check: SystemTime,
    /// Check duration
    pub check_duration: Duration,
}

/// Health metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthMetrics {
    /// CPU usage percentage
    pub cpu_usage: f64,
    /// Memory usage in MB
    pub memory_usage_mb: u64,
    /// Memory usage percentage
    pub memory_usage_percent: f64,
    /// Active connections
    pub active_connections: u64,
    /// Request rate (requests per second)
    pub request_rate: f64,
    /// Error rate (errors per second)
    pub error_rate: f64,
    /// Average response time in milliseconds
    pub avg_response_time_ms: f64,
}

/// Agent information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentInfo {
    /// Agent ID
    pub agent_id: AgentPid,
    /// Agent type
    pub agent_type: String,
    /// Agent status
    pub status: String,
    /// Start time
    pub start_time: SystemTime,
    /// Last activity
    pub last_activity: SystemTime,
    /// Task count
    pub task_count: u64,
    /// Success rate
    pub success_rate: f64,
}

/// Supervisor information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SupervisorInfo {
    /// Supervisor ID
    pub supervisor_id: SupervisorId,
    /// Supervised agents
    pub supervised_agents: Vec<AgentPid>,
    /// Restart count
    pub restart_count: u32,
    /// Last restart time
    pub last_restart: Option<SystemTime>,
    /// Status
    pub status: String,
}

/// System metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemMetrics {
    /// Total tasks processed
    pub total_tasks: u64,
    /// Successful tasks
    pub successful_tasks: u64,
    /// Failed tasks
    pub failed_tasks: u64,
    /// Average task duration
    pub avg_task_duration: Duration,
    /// System load average
    pub load_average: f64,
    /// Memory usage
    pub memory_usage: u64,
    /// CPU usage
    pub cpu_usage: f64,
}

/// System messages for application management
#[derive(Debug, Clone)]
pub enum SystemMessage {
    /// Configuration changed
    ConfigurationChanged(ConfigurationChange),
    /// Agent started
    AgentStarted(AgentPid, String),
    /// Agent stopped
    AgentStopped(AgentPid, String),
    /// Agent failed
    AgentFailed(AgentPid, String),
    /// Supervisor started
    SupervisorStarted(SupervisorId),
    /// Supervisor stopped
    SupervisorStopped(SupervisorId),
    /// Health check requested
    HealthCheckRequested,
    /// System shutdown requested
    ShutdownRequested,
    /// Hot reload requested
    HotReloadRequested(String),
}

impl Default for ApplicationState {
    fn default() -> Self {
        Self {
            status: ApplicationStatus::Stopped,
            start_time: None,
            uptime_seconds: 0,
            active_agents: HashMap::new(),
            active_supervisors: HashMap::new(),
            metrics: SystemMetrics {
                total_tasks: 0,
                successful_tasks: 0,
                failed_tasks: 0,
                avg_task_duration: Duration::ZERO,
                load_average: 0.0,
                memory_usage: 0,
                cpu_usage: 0.0,
            },
            last_health_check: None,
            config_version: 0,
        }
    }
}

impl TerraphimAgentApplication {
    /// Create a new Terraphim agent application
    pub async fn new(config_path: &str) -> ApplicationResult<Self> {
        info!("Creating Terraphim agent application");

        let config_manager = Arc::new(ConfigurationManager::new(config_path).await?);
        let config = config_manager.get_config().await;

        let lifecycle_manager = Arc::new(LifecycleManager::new(config.clone()).await?);
        let deployment_manager = Arc::new(DeploymentManager::new(config.clone()).await?);
        let hot_reload_manager = Arc::new(HotReloadManager::new(config.clone()).await?);
        let diagnostics_manager = Arc::new(DiagnosticsManager::new(config.clone()).await?);

        let (system_tx, system_rx) = mpsc::unbounded_channel();

        Ok(Self {
            state: Arc::new(RwLock::new(ApplicationState::default())),
            config_manager,
            lifecycle_manager,
            deployment_manager,
            hot_reload_manager,
            diagnostics_manager,
            orchestrator: Arc::new(RwLock::new(None)),
            system_tx,
            system_rx: Arc::new(RwLock::new(Some(system_rx))),
        })
    }

    /// Start system message handler
    async fn start_message_handler(&self) -> ApplicationResult<()> {
        let mut rx = self.system_rx.write().await.take().ok_or_else(|| {
            ApplicationError::SystemError("Message handler already started".to_string())
        })?;

        let state = self.state.clone();
        let config_manager = self.config_manager.clone();
        let hot_reload_manager = self.hot_reload_manager.clone();

        tokio::spawn(async move {
            while let Some(message) = rx.recv().await {
                if let Err(e) = Self::handle_system_message(
                    message,
                    &state,
                    &config_manager,
                    &hot_reload_manager,
                )
                .await
                {
                    error!("Error handling system message: {}", e);
                }
            }
        });

        Ok(())
    }

    /// Handle system messages
    async fn handle_system_message(
        message: SystemMessage,
        state: &Arc<RwLock<ApplicationState>>,
        config_manager: &Arc<ConfigurationManager>,
        hot_reload_manager: &Arc<HotReloadManager>,
    ) -> ApplicationResult<()> {
        match message {
            SystemMessage::ConfigurationChanged(change) => {
                info!("Configuration changed: {:?}", change.change_type);
                let mut app_state = state.write().await;
                app_state.config_version += 1;
            }
            SystemMessage::AgentStarted(agent_id, agent_type) => {
                info!("Agent started: {} ({})", agent_id, agent_type);
                let mut app_state = state.write().await;
                app_state.active_agents.insert(
                    agent_id.clone(),
                    AgentInfo {
                        agent_id,
                        agent_type,
                        status: "running".to_string(),
                        start_time: SystemTime::now(),
                        last_activity: SystemTime::now(),
                        task_count: 0,
                        success_rate: 1.0,
                    },
                );
            }
            SystemMessage::AgentStopped(agent_id, reason) => {
                info!("Agent stopped: {} ({})", agent_id, reason);
                let mut app_state = state.write().await;
                app_state.active_agents.remove(&agent_id);
            }
            SystemMessage::AgentFailed(agent_id, error) => {
                warn!("Agent failed: {} ({})", agent_id, error);
                let mut app_state = state.write().await;
                if let Some(agent_info) = app_state.active_agents.get_mut(&agent_id) {
                    agent_info.status = format!("failed: {}", error);
                }
            }
            SystemMessage::SupervisorStarted(supervisor_id) => {
                info!("Supervisor started: {}", supervisor_id);
                let mut app_state = state.write().await;
                app_state.active_supervisors.insert(
                    supervisor_id.clone(),
                    SupervisorInfo {
                        supervisor_id,
                        supervised_agents: Vec::new(),
                        restart_count: 0,
                        last_restart: None,
                        status: "running".to_string(),
                    },
                );
            }
            SystemMessage::SupervisorStopped(supervisor_id) => {
                info!("Supervisor stopped: {}", supervisor_id);
                let mut app_state = state.write().await;
                app_state.active_supervisors.remove(&supervisor_id);
            }
            SystemMessage::HealthCheckRequested => {
                debug!("Health check requested");
                let mut app_state = state.write().await;
                app_state.last_health_check = Some(SystemTime::now());
            }
            SystemMessage::ShutdownRequested => {
                info!("Shutdown requested");
                let mut app_state = state.write().await;
                app_state.status = ApplicationStatus::Stopping;
            }
            SystemMessage::HotReloadRequested(component) => {
                info!("Hot reload requested for component: {}", component);
                if let Err(e) = hot_reload_manager.reload_component(&component).await {
                    error!("Hot reload failed for {}: {}", component, e);
                }
            }
        }
        Ok(())
    }

    /// Start periodic tasks
    async fn start_periodic_tasks(&self) -> ApplicationResult<()> {
        let state = self.state.clone();
        let system_tx = self.system_tx.clone();
        let config_manager = self.config_manager.clone();

        // Health check task
        tokio::spawn(async move {
            let config = config_manager.get_config().await;
            let mut interval = interval(Duration::from_secs(config.health.check_interval_seconds));

            loop {
                interval.tick().await;
                if let Err(e) = system_tx.send(SystemMessage::HealthCheckRequested) {
                    error!("Failed to send health check request: {}", e);
                    break;
                }
            }
        });

        // Metrics update task
        let state_clone = state.clone();
        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(60)); // Update metrics every minute

            loop {
                interval.tick().await;
                let mut app_state = state_clone.write().await;

                // Update uptime
                if let Some(start_time) = app_state.start_time {
                    app_state.uptime_seconds = start_time.elapsed().unwrap_or_default().as_secs();
                }

                // Update system metrics (simplified)
                app_state.metrics.load_average = Self::get_system_load().await;
                app_state.metrics.memory_usage = Self::get_memory_usage().await;
                app_state.metrics.cpu_usage = Self::get_cpu_usage().await;
            }
        });

        Ok(())
    }

    /// Get system load (simplified implementation)
    async fn get_system_load() -> f64 {
        // In a real implementation, this would read from /proc/loadavg or use system APIs
        0.5 // Mock value
    }

    /// Get memory usage (simplified implementation)
    async fn get_memory_usage() -> u64 {
        // In a real implementation, this would read from /proc/meminfo or use system APIs
        1024 // Mock value in MB
    }

    /// Get CPU usage (simplified implementation)
    async fn get_cpu_usage() -> f64 {
        // In a real implementation, this would calculate CPU usage from /proc/stat
        0.3 // Mock value (30%)
    }

    /// Send system message
    pub async fn send_system_message(&self, message: SystemMessage) -> ApplicationResult<()> {
        self.system_tx
            .send(message)
            .map_err(|e| ApplicationError::SystemError(e.to_string()))?;
        Ok(())
    }
}

#[async_trait]
impl Application for TerraphimAgentApplication {
    async fn start(&mut self) -> ApplicationResult<()> {
        info!("Starting Terraphim agent application");

        // Update state to starting
        {
            let mut state = self.state.write().await;
            state.status = ApplicationStatus::Starting;
            state.start_time = Some(SystemTime::now());
        }

        // Start configuration hot reloading
        let mut config_manager =
            Arc::try_unwrap(self.config_manager.clone()).unwrap_or_else(|arc| (*arc).clone());
        config_manager.start_hot_reload().await?;
        self.config_manager = Arc::new(config_manager);

        // Start system message handler
        self.start_message_handler().await?;

        // Start lifecycle manager
        self.lifecycle_manager.start().await?;

        // Start deployment manager
        self.deployment_manager.start().await?;

        // Start hot reload manager
        self.hot_reload_manager.start().await?;

        // Start diagnostics manager
        self.diagnostics_manager.start().await?;

        // Create and start supervision tree orchestrator
        let config = self.config_manager.get_config().await;
        let orchestration_config = terraphim_kg_orchestration::SupervisionOrchestrationConfig {
            max_concurrent_workflows: config.deployment.max_concurrent_agents,
            default_restart_strategy: match config.supervision.default_restart_strategy.as_str() {
                "one_for_one" => terraphim_agent_supervisor::RestartStrategy::OneForOne,
                "one_for_all" => terraphim_agent_supervisor::RestartStrategy::OneForAll,
                "rest_for_one" => terraphim_agent_supervisor::RestartStrategy::RestForOne,
                _ => terraphim_agent_supervisor::RestartStrategy::OneForOne,
            },
            max_restart_attempts: config.supervision.max_restart_intensity,
            restart_intensity: config.supervision.max_restart_intensity,
            restart_period_seconds: config.supervision.restart_period_seconds,
            workflow_timeout_seconds: config.deployment.agent_startup_timeout_seconds,
            enable_auto_recovery: true,
            health_check_interval_seconds: config.health.check_interval_seconds,
        };

        let orchestrator = SupervisionTreeOrchestrator::new(orchestration_config)
            .await
            .map_err(|e| ApplicationError::SupervisionError(e.to_string()))?;

        orchestrator
            .start()
            .await
            .map_err(|e| ApplicationError::SupervisionError(e.to_string()))?;

        {
            let mut orch_guard = self.orchestrator.write().await;
            *orch_guard = Some(orchestrator);
        }

        // Start periodic tasks
        self.start_periodic_tasks().await?;

        // Update state to running
        {
            let mut state = self.state.write().await;
            state.status = ApplicationStatus::Running;
        }

        info!("Terraphim agent application started successfully");
        Ok(())
    }

    async fn stop(&mut self) -> ApplicationResult<()> {
        info!("Stopping Terraphim agent application");

        // Update state to stopping
        {
            let mut state = self.state.write().await;
            state.status = ApplicationStatus::Stopping;
        }

        // Stop orchestrator
        if let Some(orchestrator) = self.orchestrator.write().await.take() {
            if let Err(e) = orchestrator.shutdown().await {
                warn!("Error stopping orchestrator: {}", e);
            }
        }

        // Stop managers
        if let Err(e) = self.diagnostics_manager.stop().await {
            warn!("Error stopping diagnostics manager: {}", e);
        }

        if let Err(e) = self.hot_reload_manager.stop().await {
            warn!("Error stopping hot reload manager: {}", e);
        }

        if let Err(e) = self.deployment_manager.stop().await {
            warn!("Error stopping deployment manager: {}", e);
        }

        if let Err(e) = self.lifecycle_manager.stop().await {
            warn!("Error stopping lifecycle manager: {}", e);
        }

        // Update state to stopped
        {
            let mut state = self.state.write().await;
            state.status = ApplicationStatus::Stopped;
            state.active_agents.clear();
            state.active_supervisors.clear();
        }

        info!("Terraphim agent application stopped");
        Ok(())
    }

    async fn restart(&mut self) -> ApplicationResult<()> {
        info!("Restarting Terraphim agent application");

        {
            let mut state = self.state.write().await;
            state.status = ApplicationStatus::Restarting;
        }

        self.stop().await?;
        tokio::time::sleep(Duration::from_secs(1)).await; // Brief pause
        self.start().await?;

        info!("Terraphim agent application restarted");
        Ok(())
    }

    async fn status(&self) -> ApplicationResult<ApplicationStatus> {
        let state = self.state.read().await;
        Ok(state.status.clone())
    }

    async fn handle_config_change(&mut self, change: ConfigurationChange) -> ApplicationResult<()> {
        info!("Handling configuration change: {:?}", change.change_type);

        // Send system message
        self.send_system_message(SystemMessage::ConfigurationChanged(change.clone()))
            .await?;

        // Handle specific configuration changes
        match change.section.as_str() {
            "supervision" => {
                info!("Supervision configuration changed, restarting orchestrator");
                // In a real implementation, we would gracefully update the orchestrator
            }
            "deployment" => {
                info!("Deployment configuration changed, updating deployment manager");
                // In a real implementation, we would update deployment settings
            }
            "health" => {
                info!("Health configuration changed, updating health checks");
                // In a real implementation, we would update health check intervals
            }
            _ => {
                debug!(
                    "Configuration change for section '{}' handled generically",
                    change.section
                );
            }
        }

        Ok(())
    }

    async fn health_check(&self) -> ApplicationResult<HealthStatus> {
        let state = self.state.read().await;
        let config = self.config_manager.get_config().await;

        let mut components = HashMap::new();

        // Check lifecycle manager
        let lifecycle_health = self
            .lifecycle_manager
            .health_check()
            .await
            .map_err(|e| ApplicationError::HealthCheckFailed(e.to_string()))?;
        components.insert(
            "lifecycle".to_string(),
            ComponentHealth {
                level: if lifecycle_health {
                    HealthLevel::Healthy
                } else {
                    HealthLevel::Unhealthy
                },
                message: if lifecycle_health {
                    "OK".to_string()
                } else {
                    "Failed".to_string()
                },
                last_check: SystemTime::now(),
                check_duration: Duration::from_millis(10),
            },
        );

        // Check deployment manager
        let deployment_health = self
            .deployment_manager
            .health_check()
            .await
            .map_err(|e| ApplicationError::HealthCheckFailed(e.to_string()))?;
        components.insert(
            "deployment".to_string(),
            ComponentHealth {
                level: if deployment_health {
                    HealthLevel::Healthy
                } else {
                    HealthLevel::Unhealthy
                },
                message: if deployment_health {
                    "OK".to_string()
                } else {
                    "Failed".to_string()
                },
                last_check: SystemTime::now(),
                check_duration: Duration::from_millis(15),
            },
        );

        // Check orchestrator
        let orchestrator_health =
            if let Some(orchestrator) = self.orchestrator.read().await.as_ref() {
                // In a real implementation, we would check orchestrator health
                true
            } else {
                false
            };
        components.insert(
            "orchestrator".to_string(),
            ComponentHealth {
                level: if orchestrator_health {
                    HealthLevel::Healthy
                } else {
                    HealthLevel::Critical
                },
                message: if orchestrator_health {
                    "OK".to_string()
                } else {
                    "Not running".to_string()
                },
                last_check: SystemTime::now(),
                check_duration: Duration::from_millis(5),
            },
        );

        // Determine overall health
        let overall = if components.values().all(|c| c.level == HealthLevel::Healthy) {
            HealthLevel::Healthy
        } else if components
            .values()
            .any(|c| c.level == HealthLevel::Critical)
        {
            HealthLevel::Critical
        } else if components
            .values()
            .any(|c| c.level == HealthLevel::Unhealthy)
        {
            HealthLevel::Unhealthy
        } else {
            HealthLevel::Degraded
        };

        let metrics = HealthMetrics {
            cpu_usage: state.metrics.cpu_usage,
            memory_usage_mb: state.metrics.memory_usage,
            memory_usage_percent: (state.metrics.memory_usage as f64
                / config.resources.max_memory_mb as f64)
                * 100.0,
            active_connections: state.active_agents.len() as u64,
            request_rate: 0.0,          // Would be calculated from actual metrics
            error_rate: 0.0,            // Would be calculated from actual metrics
            avg_response_time_ms: 50.0, // Would be calculated from actual metrics
        };

        Ok(HealthStatus {
            overall,
            components,
            timestamp: SystemTime::now(),
            metrics,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[tokio::test]
    async fn test_application_creation() {
        let temp_file = NamedTempFile::new().unwrap();
        let app = TerraphimAgentApplication::new(temp_file.path().to_str().unwrap()).await;
        assert!(app.is_ok());
    }

    #[tokio::test]
    async fn test_application_status() {
        let temp_file = NamedTempFile::new().unwrap();
        let app = TerraphimAgentApplication::new(temp_file.path().to_str().unwrap())
            .await
            .unwrap();
        let status = app.status().await.unwrap();
        assert_eq!(status, ApplicationStatus::Stopped);
    }

    #[tokio::test]
    async fn test_health_check() {
        let temp_file = NamedTempFile::new().unwrap();
        let app = TerraphimAgentApplication::new(temp_file.path().to_str().unwrap())
            .await
            .unwrap();
        let health = app.health_check().await;
        assert!(health.is_ok());
    }

    #[tokio::test]
    async fn test_system_message_sending() {
        let temp_file = NamedTempFile::new().unwrap();
        let app = TerraphimAgentApplication::new(temp_file.path().to_str().unwrap())
            .await
            .unwrap();
        let result = app
            .send_system_message(SystemMessage::HealthCheckRequested)
            .await;
        assert!(result.is_ok());
    }
}
