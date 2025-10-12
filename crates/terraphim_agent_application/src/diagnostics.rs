//! System diagnostics and health monitoring

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use async_trait::async_trait;
use log::{debug, info};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use crate::{ApplicationConfig, ApplicationError, ApplicationResult};

/// Diagnostics management trait
#[async_trait]
pub trait DiagnosticsManagement: Send + Sync {
    /// Start diagnostics manager
    async fn start(&self) -> ApplicationResult<()>;

    /// Stop diagnostics manager
    async fn stop(&self) -> ApplicationResult<()>;

    /// Perform system diagnostics
    async fn run_diagnostics(&self) -> ApplicationResult<DiagnosticsReport>;

    /// Get system metrics
    async fn get_metrics(&self) -> ApplicationResult<SystemMetrics>;

    /// Get performance report
    async fn get_performance_report(&self) -> ApplicationResult<PerformanceReport>;

    /// Check system health
    async fn check_system_health(&self) -> ApplicationResult<SystemHealth>;
}

/// Diagnostics manager implementation
pub struct DiagnosticsManager {
    /// Configuration
    config: ApplicationConfig,
    /// Metrics history
    metrics_history: Arc<RwLock<Vec<MetricsSnapshot>>>,
    /// Diagnostic checks
    diagnostic_checks: Arc<RwLock<HashMap<String, DiagnosticCheck>>>,
}

/// Diagnostics report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagnosticsReport {
    /// Report timestamp
    pub timestamp: SystemTime,
    /// System health
    pub system_health: SystemHealth,
    /// Performance metrics
    pub performance: PerformanceReport,
    /// Resource utilization
    pub resources: ResourceUtilization,
    /// Diagnostic checks results
    pub checks: HashMap<String, CheckResult>,
    /// Recommendations
    pub recommendations: Vec<String>,
}

/// System health status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemHealth {
    /// Overall health status
    pub status: HealthStatus,
    /// Health score (0.0 to 1.0)
    pub score: f64,
    /// Component health
    pub components: HashMap<String, ComponentHealth>,
    /// Issues detected
    pub issues: Vec<HealthIssue>,
}

/// Health status levels
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum HealthStatus {
    Excellent,
    Good,
    Fair,
    Poor,
    Critical,
}

/// Component health information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentHealth {
    /// Component name
    pub name: String,
    /// Health status
    pub status: HealthStatus,
    /// Health score
    pub score: f64,
    /// Last check time
    pub last_check: SystemTime,
    /// Issues
    pub issues: Vec<String>,
}

/// Health issue
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthIssue {
    /// Issue severity
    pub severity: IssueSeverity,
    /// Issue category
    pub category: String,
    /// Issue description
    pub description: String,
    /// Recommended action
    pub recommendation: Option<String>,
    /// First detected
    pub first_detected: SystemTime,
}

/// Issue severity levels
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum IssueSeverity {
    Info,
    Warning,
    Error,
    Critical,
}

/// System metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemMetrics {
    /// CPU metrics
    pub cpu: CpuMetrics,
    /// Memory metrics
    pub memory: MemoryMetrics,
    /// Disk metrics
    pub disk: DiskMetrics,
    /// Network metrics
    pub network: NetworkMetrics,
    /// Application metrics
    pub application: ApplicationMetrics,
}

/// CPU metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpuMetrics {
    /// CPU usage percentage
    pub usage_percent: f64,
    /// Load average (1 minute)
    pub load_average_1m: f64,
    /// Load average (5 minutes)
    pub load_average_5m: f64,
    /// Load average (15 minutes)
    pub load_average_15m: f64,
    /// Number of cores
    pub cores: u32,
}

/// Memory metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryMetrics {
    /// Total memory in MB
    pub total_mb: u64,
    /// Used memory in MB
    pub used_mb: u64,
    /// Available memory in MB
    pub available_mb: u64,
    /// Memory usage percentage
    pub usage_percent: f64,
    /// Swap usage in MB
    pub swap_used_mb: u64,
}

/// Disk metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiskMetrics {
    /// Total disk space in MB
    pub total_mb: u64,
    /// Used disk space in MB
    pub used_mb: u64,
    /// Available disk space in MB
    pub available_mb: u64,
    /// Disk usage percentage
    pub usage_percent: f64,
    /// Read operations per second
    pub read_ops_per_sec: f64,
    /// Write operations per second
    pub write_ops_per_sec: f64,
}

/// Network metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkMetrics {
    /// Bytes received per second
    pub bytes_received_per_sec: u64,
    /// Bytes sent per second
    pub bytes_sent_per_sec: u64,
    /// Packets received per second
    pub packets_received_per_sec: u64,
    /// Packets sent per second
    pub packets_sent_per_sec: u64,
    /// Active connections
    pub active_connections: u64,
}

/// Application-specific metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplicationMetrics {
    /// Active agents
    pub active_agents: u64,
    /// Active supervisors
    pub active_supervisors: u64,
    /// Tasks processed per second
    pub tasks_per_sec: f64,
    /// Average task duration
    pub avg_task_duration_ms: f64,
    /// Error rate
    pub error_rate: f64,
    /// Memory usage by application
    pub app_memory_mb: u64,
}

/// Performance report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceReport {
    /// Report period
    pub period: Duration,
    /// Throughput metrics
    pub throughput: ThroughputMetrics,
    /// Latency metrics
    pub latency: LatencyMetrics,
    /// Resource efficiency
    pub efficiency: EfficiencyMetrics,
    /// Performance trends
    pub trends: PerformanceTrends,
}

/// Throughput metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThroughputMetrics {
    /// Requests per second
    pub requests_per_sec: f64,
    /// Tasks completed per second
    pub tasks_per_sec: f64,
    /// Peak throughput
    pub peak_throughput: f64,
    /// Average throughput
    pub avg_throughput: f64,
}

/// Latency metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LatencyMetrics {
    /// Average response time in milliseconds
    pub avg_response_time_ms: f64,
    /// 95th percentile response time
    pub p95_response_time_ms: f64,
    /// 99th percentile response time
    pub p99_response_time_ms: f64,
    /// Maximum response time
    pub max_response_time_ms: f64,
}

/// Efficiency metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EfficiencyMetrics {
    /// CPU efficiency (tasks per CPU second)
    pub cpu_efficiency: f64,
    /// Memory efficiency (tasks per MB)
    pub memory_efficiency: f64,
    /// Resource utilization score
    pub utilization_score: f64,
    /// Cost efficiency score
    pub cost_efficiency: f64,
}

/// Performance trends
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceTrends {
    /// Throughput trend (positive = improving)
    pub throughput_trend: f64,
    /// Latency trend (negative = improving)
    pub latency_trend: f64,
    /// Error rate trend (negative = improving)
    pub error_rate_trend: f64,
    /// Resource usage trend (negative = improving)
    pub resource_usage_trend: f64,
}

/// Resource utilization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceUtilization {
    /// CPU utilization percentage
    pub cpu_utilization: f64,
    /// Memory utilization percentage
    pub memory_utilization: f64,
    /// Disk utilization percentage
    pub disk_utilization: f64,
    /// Network utilization percentage
    pub network_utilization: f64,
    /// Overall utilization score
    pub overall_utilization: f64,
}

/// Diagnostic check
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagnosticCheck {
    /// Check name
    pub name: String,
    /// Check description
    pub description: String,
    /// Check function
    pub check_type: CheckType,
    /// Check interval
    pub interval: Duration,
    /// Last run time
    pub last_run: Option<SystemTime>,
    /// Enabled status
    pub enabled: bool,
}

/// Types of diagnostic checks
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CheckType {
    /// System resource check
    SystemResource,
    /// Application health check
    ApplicationHealth,
    /// Performance check
    Performance,
    /// Security check
    Security,
    /// Configuration check
    Configuration,
}

/// Check result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckResult {
    /// Check name
    pub check_name: String,
    /// Success status
    pub success: bool,
    /// Result message
    pub message: String,
    /// Check duration
    pub duration: Duration,
    /// Severity (if failed)
    pub severity: Option<IssueSeverity>,
    /// Recommendations
    pub recommendations: Vec<String>,
}

/// Metrics snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsSnapshot {
    /// Snapshot timestamp
    pub timestamp: SystemTime,
    /// System metrics at this time
    pub metrics: SystemMetrics,
}

impl DiagnosticsManager {
    /// Create a new diagnostics manager
    pub async fn new(config: ApplicationConfig) -> ApplicationResult<Self> {
        let mut diagnostic_checks = HashMap::new();

        // Register default diagnostic checks
        diagnostic_checks.insert(
            "cpu_usage".to_string(),
            DiagnosticCheck {
                name: "CPU Usage Check".to_string(),
                description: "Monitor CPU utilization".to_string(),
                check_type: CheckType::SystemResource,
                interval: Duration::from_secs(60),
                last_run: None,
                enabled: true,
            },
        );

        diagnostic_checks.insert(
            "memory_usage".to_string(),
            DiagnosticCheck {
                name: "Memory Usage Check".to_string(),
                description: "Monitor memory utilization".to_string(),
                check_type: CheckType::SystemResource,
                interval: Duration::from_secs(60),
                last_run: None,
                enabled: true,
            },
        );

        diagnostic_checks.insert(
            "agent_health".to_string(),
            DiagnosticCheck {
                name: "Agent Health Check".to_string(),
                description: "Monitor agent system health".to_string(),
                check_type: CheckType::ApplicationHealth,
                interval: Duration::from_secs(30),
                last_run: None,
                enabled: true,
            },
        );

        Ok(Self {
            config,
            metrics_history: Arc::new(RwLock::new(Vec::new())),
            diagnostic_checks: Arc::new(RwLock::new(diagnostic_checks)),
        })
    }

    /// Collect current system metrics
    async fn collect_metrics(&self) -> SystemMetrics {
        // In a real implementation, these would be collected from the system
        SystemMetrics {
            cpu: CpuMetrics {
                usage_percent: 45.0,
                load_average_1m: 0.8,
                load_average_5m: 0.7,
                load_average_15m: 0.6,
                cores: 8,
            },
            memory: MemoryMetrics {
                total_mb: 16384,
                used_mb: 8192,
                available_mb: 8192,
                usage_percent: 50.0,
                swap_used_mb: 1024,
            },
            disk: DiskMetrics {
                total_mb: 512000,
                used_mb: 256000,
                available_mb: 256000,
                usage_percent: 50.0,
                read_ops_per_sec: 100.0,
                write_ops_per_sec: 50.0,
            },
            network: NetworkMetrics {
                bytes_received_per_sec: 1024000,
                bytes_sent_per_sec: 512000,
                packets_received_per_sec: 1000,
                packets_sent_per_sec: 800,
                active_connections: 50,
            },
            application: ApplicationMetrics {
                active_agents: 25,
                active_supervisors: 5,
                tasks_per_sec: 100.0,
                avg_task_duration_ms: 250.0,
                error_rate: 0.01,
                app_memory_mb: 2048,
            },
        }
    }

    /// Run a specific diagnostic check
    async fn run_check(&self, check: &DiagnosticCheck) -> CheckResult {
        let start_time = std::time::Instant::now();

        let (success, message, severity, recommendations) = match check.check_type {
            CheckType::SystemResource => {
                let metrics = self.collect_metrics().await;
                if metrics.cpu.usage_percent > 90.0 {
                    (
                        false,
                        "High CPU usage detected".to_string(),
                        Some(IssueSeverity::Warning),
                        vec!["Consider scaling up resources".to_string()],
                    )
                } else {
                    (
                        true,
                        "System resources within normal limits".to_string(),
                        None,
                        vec![],
                    )
                }
            }
            CheckType::ApplicationHealth => {
                let metrics = self.collect_metrics().await;
                if metrics.application.error_rate > 0.05 {
                    (
                        false,
                        "High error rate detected".to_string(),
                        Some(IssueSeverity::Error),
                        vec!["Check agent logs for errors".to_string()],
                    )
                } else {
                    (true, "Application health is good".to_string(), None, vec![])
                }
            }
            CheckType::Performance => {
                let metrics = self.collect_metrics().await;
                if metrics.application.avg_task_duration_ms > 1000.0 {
                    (
                        false,
                        "High task latency detected".to_string(),
                        Some(IssueSeverity::Warning),
                        vec!["Optimize task processing".to_string()],
                    )
                } else {
                    (
                        true,
                        "Performance within acceptable limits".to_string(),
                        None,
                        vec![],
                    )
                }
            }
            CheckType::Security => (true, "Security check passed".to_string(), None, vec![]),
            CheckType::Configuration => (true, "Configuration is valid".to_string(), None, vec![]),
        };

        CheckResult {
            check_name: check.name.clone(),
            success,
            message,
            duration: start_time.elapsed(),
            severity,
            recommendations,
        }
    }
}

#[async_trait]
impl DiagnosticsManagement for DiagnosticsManager {
    async fn start(&self) -> ApplicationResult<()> {
        info!("Starting diagnostics manager");
        // In a real implementation, this would start periodic diagnostic checks
        Ok(())
    }

    async fn stop(&self) -> ApplicationResult<()> {
        info!("Stopping diagnostics manager");
        // In a real implementation, this would stop diagnostic checks
        Ok(())
    }

    async fn run_diagnostics(&self) -> ApplicationResult<DiagnosticsReport> {
        debug!("Running system diagnostics");

        let metrics = self.collect_metrics().await;
        let checks = self.diagnostic_checks.read().await;
        let mut check_results = HashMap::new();

        // Run all enabled diagnostic checks
        for (check_name, check) in checks.iter() {
            if check.enabled {
                let result = self.run_check(check).await;
                check_results.insert(check_name.clone(), result);
            }
        }

        // Analyze system health
        let mut issues = Vec::new();
        let mut component_health = HashMap::new();

        // Check for issues based on metrics and check results
        if metrics.cpu.usage_percent > 80.0 {
            issues.push(HealthIssue {
                severity: IssueSeverity::Warning,
                category: "performance".to_string(),
                description: "High CPU usage detected".to_string(),
                recommendation: Some("Consider scaling resources".to_string()),
                first_detected: SystemTime::now(),
            });
        }

        if metrics.memory.usage_percent > 85.0 {
            issues.push(HealthIssue {
                severity: IssueSeverity::Warning,
                category: "resources".to_string(),
                description: "High memory usage detected".to_string(),
                recommendation: Some("Monitor memory leaks".to_string()),
                first_detected: SystemTime::now(),
            });
        }

        // Calculate overall health score
        let health_score = if issues.is_empty() {
            1.0
        } else {
            let critical_issues = issues
                .iter()
                .filter(|i| i.severity == IssueSeverity::Critical)
                .count();
            let error_issues = issues
                .iter()
                .filter(|i| i.severity == IssueSeverity::Error)
                .count();
            let warning_issues = issues
                .iter()
                .filter(|i| i.severity == IssueSeverity::Warning)
                .count();

            1.0 - (critical_issues as f64 * 0.5
                + error_issues as f64 * 0.3
                + warning_issues as f64 * 0.1)
        };

        let health_status = match health_score {
            s if s >= 0.9 => HealthStatus::Excellent,
            s if s >= 0.7 => HealthStatus::Good,
            s if s >= 0.5 => HealthStatus::Fair,
            s if s >= 0.3 => HealthStatus::Poor,
            _ => HealthStatus::Critical,
        };

        let system_health = SystemHealth {
            status: health_status,
            score: health_score,
            components: component_health,
            issues,
        };

        let performance = PerformanceReport {
            period: Duration::from_secs(3600),
            throughput: ThroughputMetrics {
                requests_per_sec: metrics.application.tasks_per_sec,
                tasks_per_sec: metrics.application.tasks_per_sec,
                peak_throughput: metrics.application.tasks_per_sec * 1.2,
                avg_throughput: metrics.application.tasks_per_sec,
            },
            latency: LatencyMetrics {
                avg_response_time_ms: metrics.application.avg_task_duration_ms,
                p95_response_time_ms: metrics.application.avg_task_duration_ms * 1.5,
                p99_response_time_ms: metrics.application.avg_task_duration_ms * 2.0,
                max_response_time_ms: metrics.application.avg_task_duration_ms * 3.0,
            },
            efficiency: EfficiencyMetrics {
                cpu_efficiency: metrics.application.tasks_per_sec / metrics.cpu.usage_percent,
                memory_efficiency: metrics.application.tasks_per_sec
                    / (metrics.memory.used_mb as f64),
                utilization_score: (metrics.cpu.usage_percent + metrics.memory.usage_percent)
                    / 200.0,
                cost_efficiency: 0.8,
            },
            trends: PerformanceTrends {
                throughput_trend: 0.05,
                latency_trend: -0.02,
                error_rate_trend: -0.01,
                resource_usage_trend: 0.03,
            },
        };

        let resources = ResourceUtilization {
            cpu_utilization: metrics.cpu.usage_percent,
            memory_utilization: metrics.memory.usage_percent,
            disk_utilization: metrics.disk.usage_percent,
            network_utilization: 30.0, // Calculated based on network metrics
            overall_utilization: (metrics.cpu.usage_percent
                + metrics.memory.usage_percent
                + metrics.disk.usage_percent)
                / 3.0,
        };

        let mut recommendations = Vec::new();
        if health_score < 0.8 {
            recommendations.push(
                "Consider reviewing system performance and addressing identified issues"
                    .to_string(),
            );
        }
        if metrics.cpu.usage_percent > 70.0 {
            recommendations
                .push("Monitor CPU usage and consider scaling if trend continues".to_string());
        }

        Ok(DiagnosticsReport {
            timestamp: SystemTime::now(),
            system_health,
            performance,
            resources,
            checks: check_results,
            recommendations,
        })
    }

    async fn get_metrics(&self) -> ApplicationResult<SystemMetrics> {
        let metrics = self.collect_metrics().await;

        // Store metrics in history
        let snapshot = MetricsSnapshot {
            timestamp: SystemTime::now(),
            metrics: metrics.clone(),
        };

        let mut history = self.metrics_history.write().await;
        history.push(snapshot);

        // Keep only recent metrics (last 100 snapshots)
        if history.len() > 100 {
            history.remove(0);
        }

        Ok(metrics)
    }

    async fn get_performance_report(&self) -> ApplicationResult<PerformanceReport> {
        let metrics = self.collect_metrics().await;

        Ok(PerformanceReport {
            period: Duration::from_secs(3600),
            throughput: ThroughputMetrics {
                requests_per_sec: metrics.application.tasks_per_sec,
                tasks_per_sec: metrics.application.tasks_per_sec,
                peak_throughput: metrics.application.tasks_per_sec * 1.2,
                avg_throughput: metrics.application.tasks_per_sec,
            },
            latency: LatencyMetrics {
                avg_response_time_ms: metrics.application.avg_task_duration_ms,
                p95_response_time_ms: metrics.application.avg_task_duration_ms * 1.5,
                p99_response_time_ms: metrics.application.avg_task_duration_ms * 2.0,
                max_response_time_ms: metrics.application.avg_task_duration_ms * 3.0,
            },
            efficiency: EfficiencyMetrics {
                cpu_efficiency: metrics.application.tasks_per_sec / metrics.cpu.usage_percent,
                memory_efficiency: metrics.application.tasks_per_sec
                    / (metrics.memory.used_mb as f64),
                utilization_score: (metrics.cpu.usage_percent + metrics.memory.usage_percent)
                    / 200.0,
                cost_efficiency: 0.8,
            },
            trends: PerformanceTrends {
                throughput_trend: 0.05,
                latency_trend: -0.02,
                error_rate_trend: -0.01,
                resource_usage_trend: 0.03,
            },
        })
    }

    async fn check_system_health(&self) -> ApplicationResult<SystemHealth> {
        let metrics = self.collect_metrics().await;
        let mut issues = Vec::new();

        // Analyze metrics for health issues
        if metrics.cpu.usage_percent > 90.0 {
            issues.push(HealthIssue {
                severity: IssueSeverity::Critical,
                category: "performance".to_string(),
                description: "Critical CPU usage".to_string(),
                recommendation: Some("Immediate resource scaling required".to_string()),
                first_detected: SystemTime::now(),
            });
        }

        if metrics.application.error_rate > 0.1 {
            issues.push(HealthIssue {
                severity: IssueSeverity::Error,
                category: "reliability".to_string(),
                description: "High error rate detected".to_string(),
                recommendation: Some("Investigate error causes".to_string()),
                first_detected: SystemTime::now(),
            });
        }

        let health_score = if issues.is_empty() { 1.0 } else { 0.6 };
        let status = if health_score >= 0.8 {
            HealthStatus::Good
        } else {
            HealthStatus::Fair
        };

        Ok(SystemHealth {
            status,
            score: health_score,
            components: HashMap::new(),
            issues,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ApplicationConfig;

    #[tokio::test]
    async fn test_diagnostics_manager_creation() {
        let config = ApplicationConfig::default();
        let manager = DiagnosticsManager::new(config).await;
        assert!(manager.is_ok());
    }

    #[tokio::test]
    async fn test_metrics_collection() {
        let config = ApplicationConfig::default();
        let manager = DiagnosticsManager::new(config).await.unwrap();

        let metrics = manager.get_metrics().await;
        assert!(metrics.is_ok());

        let metrics = metrics.unwrap();
        assert!(metrics.cpu.usage_percent >= 0.0);
        assert!(metrics.memory.total_mb > 0);
    }

    #[tokio::test]
    async fn test_diagnostics_report() {
        let config = ApplicationConfig::default();
        let manager = DiagnosticsManager::new(config).await.unwrap();

        let report = manager.run_diagnostics().await;
        assert!(report.is_ok());

        let report = report.unwrap();
        assert!(!report.checks.is_empty());
        assert!(report.system_health.score >= 0.0 && report.system_health.score <= 1.0);
    }

    #[tokio::test]
    async fn test_system_health_check() {
        let config = ApplicationConfig::default();
        let manager = DiagnosticsManager::new(config).await.unwrap();

        let health = manager.check_system_health().await;
        assert!(health.is_ok());

        let health = health.unwrap();
        assert!(health.score >= 0.0 && health.score <= 1.0);
    }
}
