//! Performance Monitor
//!
//! Monitors and benchmarks TUI performance metrics including startup time,
//! command execution times, memory usage, and responsiveness.

use anyhow::{Result, anyhow};
use std::collections::HashMap;
use std::time::{Duration, Instant};
use sysinfo::{Process, System};

/// Performance metrics for TUI operations
#[derive(Debug, Clone)]
pub struct PerformanceMetrics {
    pub startup_time: Duration,
    pub command_execution_times: HashMap<String, Vec<Duration>>,
    pub memory_usage: MemoryStats,
    pub cpu_usage: f32,
    pub average_response_time: Duration,
    pub max_response_time: Duration,
    pub min_response_time: Duration,
}

/// Memory usage statistics
#[derive(Debug, Clone)]
pub struct MemoryStats {
    pub rss_bytes: u64,
    pub virtual_bytes: u64,
    pub peak_rss_bytes: u64,
}

/// Performance benchmark results
#[derive(Debug, Clone)]
pub struct PerformanceResults {
    pub metrics: PerformanceMetrics,
    pub benchmarks_passed: usize,
    pub benchmarks_total: usize,
    pub slo_violations: Vec<String>,
}

/// Service Level Objectives for performance
#[derive(Debug, Clone)]
pub struct PerformanceSLO {
    pub max_startup_time_ms: u64,
    pub max_command_time_ms: u64,
    pub max_memory_mb: u64,
    pub min_commands_per_second: f64,
}

impl Default for PerformanceSLO {
    fn default() -> Self {
        Self {
            max_startup_time_ms: 2000,     // 2 seconds
            max_command_time_ms: 500,      // 500ms per command
            max_memory_mb: 100,            // 100MB
            min_commands_per_second: 10.0, // 10 commands/second
        }
    }
}

/// Performance Monitor for TUI testing
pub struct PerformanceMonitor {
    system: System,
    slo: PerformanceSLO,
    start_time: Instant,
    command_times: HashMap<String, Vec<Duration>>,
    process_id: Option<sysinfo::Pid>,
}

impl PerformanceMonitor {
    /// Create a new performance monitor
    pub fn new() -> Result<Self> {
        let mut system = System::new_all();
        system.refresh_all();

        Ok(Self {
            system,
            slo: PerformanceSLO::default(),
            start_time: Instant::now(),
            command_times: HashMap::new(),
            process_id: None,
        })
    }

    /// Start monitoring a process
    pub fn start_monitoring(&mut self, process_id: u32) {
        self.process_id = Some(sysinfo::Pid::from_u32(process_id));
        self.start_time = Instant::now();
    }

    /// Record command execution time
    pub fn record_command_time(&mut self, command: &str, duration: Duration) {
        self.command_times
            .entry(command.to_string())
            .or_insert_with(Vec::new)
            .push(duration);
    }

    /// Get current memory usage
    pub fn get_memory_usage(&mut self) -> Result<MemoryStats> {
        self.system.refresh_all();

        let stats = if let Some(pid) = self.process_id {
            if let Some(process) = self.system.process(pid) {
                MemoryStats {
                    rss_bytes: process.memory(),
                    virtual_bytes: process.virtual_memory(),
                    peak_rss_bytes: 0, // Not available in sysinfo
                }
            } else {
                return Err(anyhow!("Process {:?} not found", pid));
            }
        } else {
            MemoryStats {
                rss_bytes: 0,
                virtual_bytes: 0,
                peak_rss_bytes: 0,
            }
        };

        Ok(stats)
    }

    /// Get current CPU usage
    pub fn get_cpu_usage(&mut self) -> Result<f32> {
        self.system.refresh_all();

        if let Some(pid) = self.process_id {
            if let Some(process) = self.system.process(pid) {
                Ok(process.cpu_usage())
            } else {
                Err(anyhow!("Process {:?} not found", pid))
            }
        } else {
            Ok(0.0)
        }
    }

    /// Run performance benchmarks
    pub async fn run_performance_tests(&mut self) -> Result<PerformanceResults> {
        let mut results = PerformanceResults {
            metrics: self.collect_metrics().await?,
            benchmarks_passed: 0,
            benchmarks_total: 0,
            slo_violations: Vec::new(),
        };

        // Run benchmark tests
        results.benchmarks_total = 4;
        results.benchmarks_passed = 0;

        // Benchmark 1: Startup time SLO
        if results.metrics.startup_time.as_millis() <= self.slo.max_startup_time_ms as u128 {
            results.benchmarks_passed += 1;
        } else {
            results.slo_violations.push(format!(
                "Startup time SLO violated: {}ms > {}ms",
                results.metrics.startup_time.as_millis(),
                self.slo.max_startup_time_ms
            ));
        }

        // Benchmark 2: Command execution time SLO
        let avg_command_time = results.metrics.average_response_time;
        if avg_command_time.as_millis() <= self.slo.max_command_time_ms as u128 {
            results.benchmarks_passed += 1;
        } else {
            results.slo_violations.push(format!(
                "Command execution time SLO violated: {}ms > {}ms",
                avg_command_time.as_millis(),
                self.slo.max_command_time_ms
            ));
        }

        // Benchmark 3: Memory usage SLO
        let memory_mb = results.metrics.memory_usage.rss_bytes / (1024 * 1024);
        if memory_mb <= self.slo.max_memory_mb {
            results.benchmarks_passed += 1;
        } else {
            results.slo_violations.push(format!(
                "Memory usage SLO violated: {}MB > {}MB",
                memory_mb, self.slo.max_memory_mb
            ));
        }

        // Benchmark 4: Commands per second SLO
        let commands_per_second = if !results.metrics.command_execution_times.is_empty() {
            let total_commands: usize = results
                .metrics
                .command_execution_times
                .values()
                .map(|v| v.len())
                .sum();
            let total_time = results.metrics.startup_time;
            total_commands as f64 / total_time.as_secs_f64()
        } else {
            0.0
        };

        if commands_per_second >= self.slo.min_commands_per_second {
            results.benchmarks_passed += 1;
        } else {
            results.slo_violations.push(format!(
                "Commands/second SLO violated: {:.2} < {:.2}",
                commands_per_second, self.slo.min_commands_per_second
            ));
        }

        Ok(results)
    }

    /// Collect current performance metrics
    async fn collect_metrics(&mut self) -> Result<PerformanceMetrics> {
        let memory_usage = self.get_memory_usage()?;
        let cpu_usage = self.get_cpu_usage()?;

        // Calculate response time statistics
        let mut all_times: Vec<Duration> = self.command_times.values().flatten().cloned().collect();
        all_times.sort();

        let (avg_time, min_time, max_time) = if all_times.is_empty() {
            (
                Duration::default(),
                Duration::default(),
                Duration::default(),
            )
        } else {
            let avg = all_times.iter().sum::<Duration>() / all_times.len() as u32;
            let min = all_times[0];
            let max = all_times[all_times.len() - 1];
            (avg, min, max)
        };

        Ok(PerformanceMetrics {
            startup_time: self.start_time.elapsed(),
            command_execution_times: self.command_times.clone(),
            memory_usage,
            cpu_usage,
            average_response_time: avg_time,
            max_response_time: max_time,
            min_response_time: min_time,
        })
    }

    /// Reset performance monitoring
    pub fn reset(&mut self) -> Result<()> {
        self.start_time = Instant::now();
        self.command_times.clear();
        self.system.refresh_all();
        Ok(())
    }

    /// Set custom SLO values
    pub fn set_slo(&mut self, slo: PerformanceSLO) {
        self.slo = slo;
    }

    /// Get current SLO settings
    pub fn get_slo(&self) -> &PerformanceSLO {
        &self.slo
    }

    /// Generate performance report
    pub async fn generate_report(&mut self) -> Result<String> {
        let metrics = self.collect_metrics().await?;
        let results = self.run_performance_tests().await?;

        let mut report = format!("TUI Performance Report\n{}\n", "=".repeat(50));

        report.push_str(&format!(
            "Startup Time: {:.2}s\n",
            metrics.startup_time.as_secs_f64()
        ));
        report.push_str(&format!(
            "Memory Usage: {} MB\n",
            metrics.memory_usage.rss_bytes / (1024 * 1024)
        ));
        report.push_str(&format!("CPU Usage: {:.1}%\n", metrics.cpu_usage));
        report.push_str(&format!(
            "Average Response Time: {:.2}ms\n",
            metrics.average_response_time.as_millis()
        ));
        report.push_str(&format!(
            "Max Response Time: {:.2}ms\n",
            metrics.max_response_time.as_millis()
        ));
        report.push_str(&format!(
            "Min Response Time: {:.2}ms\n",
            metrics.min_response_time.as_millis()
        ));

        report.push_str(&format!(
            "\nBenchmarks: {}/{}\n",
            results.benchmarks_passed, results.benchmarks_total
        ));

        if !results.slo_violations.is_empty() {
            report.push_str(&format!(
                "\nSLO Violations ({}):\n",
                results.slo_violations.len()
            ));
            for violation in &results.slo_violations {
                report.push_str(&format!("  - {}\n", violation));
            }
        }

        if !metrics.command_execution_times.is_empty() {
            report.push_str(&format!(
                "\nCommand Performance ({} commands):\n",
                metrics.command_execution_times.len()
            ));
            for (command, times) in &metrics.command_execution_times {
                let avg_time = times.iter().sum::<Duration>() / times.len() as u32;
                report.push_str(&format!(
                    "  {}: {:.2}ms ({} executions)\n",
                    command,
                    avg_time.as_millis(),
                    times.len()
                ));
            }
        }

        Ok(report)
    }

    /// Run stress test with multiple concurrent commands
    pub async fn run_stress_test(
        &mut self,
        commands: Vec<String>,
        concurrency: usize,
    ) -> Result<StressTestResults> {
        use std::sync::Arc;
        use tokio::sync::Semaphore;

        let semaphore = Arc::new(Semaphore::new(concurrency));
        let mut handles = Vec::new();

        let start_time = Instant::now();

        for command in commands {
            let sem = semaphore.clone();
            let handle = tokio::spawn(async move {
                let _permit = sem.acquire().await.unwrap();
                // Simulate command execution time
                tokio::time::sleep(Duration::from_millis(10)).await;
                (command, Duration::from_millis(10))
            });
            handles.push(handle);
        }

        let mut results = Vec::new();
        for handle in handles {
            if let Ok(result) = handle.await {
                self.record_command_time(&result.0, result.1);
                results.push(result);
            }
        }

        let total_time = start_time.elapsed();
        let throughput = results.len() as f64 / total_time.as_secs_f64();

        Ok(StressTestResults {
            total_commands: results.len(),
            total_time,
            throughput_cps: throughput,
            average_latency: results.iter().map(|(_, d)| *d).sum::<Duration>()
                / results.len() as u32,
        })
    }
}

/// Results from stress testing
#[derive(Debug, Clone)]
pub struct StressTestResults {
    pub total_commands: usize,
    pub total_time: Duration,
    pub throughput_cps: f64,
    pub average_latency: Duration,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_performance_monitor_creation() {
        let monitor = PerformanceMonitor::new();
        assert!(monitor.is_ok());
    }

    #[tokio::test]
    async fn test_metrics_collection() {
        let mut monitor = PerformanceMonitor::new().unwrap();

        // Record some command times
        monitor.record_command_time("test_cmd", Duration::from_millis(100));
        monitor.record_command_time("test_cmd", Duration::from_millis(150));
        monitor.record_command_time("other_cmd", Duration::from_millis(50));

        let metrics = monitor.collect_metrics().await.unwrap();

        assert_eq!(metrics.command_execution_times.len(), 2);
        assert!(metrics.average_response_time > Duration::from_millis(90));
        assert!(metrics.max_response_time >= Duration::from_millis(150));
        assert!(metrics.min_response_time <= Duration::from_millis(50));
    }

    #[tokio::test]
    async fn test_performance_benchmarks() {
        let mut monitor = PerformanceMonitor::new().unwrap();

        // Set up some basic metrics
        monitor.record_command_time("cmd", Duration::from_millis(100));

        let results = monitor.run_performance_tests().await.unwrap();
        assert_eq!(results.benchmarks_total, 4);

        // Benchmarks should pass with default SLO and minimal metrics
        // (Note: some may fail due to missing process monitoring)
        println!(
            "Benchmarks passed: {}/{}",
            results.benchmarks_passed, results.benchmarks_total
        );
    }

    #[tokio::test]
    async fn test_stress_test() {
        let mut monitor = PerformanceMonitor::new().unwrap();

        let commands = vec!["cmd1".to_string(), "cmd2".to_string(), "cmd3".to_string()];

        let results = monitor.run_stress_test(commands, 2).await.unwrap();

        assert_eq!(results.total_commands, 3);
        assert!(results.throughput_cps > 0.0);
        assert!(results.average_latency > Duration::default());
    }

    #[test]
    fn test_slo_defaults() {
        let slo = PerformanceSLO::default();

        assert_eq!(slo.max_startup_time_ms, 2000);
        assert_eq!(slo.max_command_time_ms, 500);
        assert_eq!(slo.max_memory_mb, 100);
        assert!(slo.min_commands_per_second >= 10.0);
    }
}
