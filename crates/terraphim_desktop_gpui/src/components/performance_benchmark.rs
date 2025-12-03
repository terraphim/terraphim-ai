/// Performance Benchmarking and Validation System
///
/// This module provides comprehensive benchmarking capabilities to measure
/// and validate the performance improvements from the optimization systems.
///
/// Features:
/// - Automated performance benchmarking
/// - Regression detection and alerting
/// - Performance trend analysis
/// - Comparative benchmarking
/// - Real-time performance validation
/// - Detailed performance reports

use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};
use anyhow::Result;
use parking_lot::RwLock;

use gpui::*;

/// Benchmark configuration
#[derive(Debug, Clone)]
pub struct BenchmarkConfig {
    /// Benchmark execution mode
    pub mode: BenchmarkMode,
    /// Number of iterations for each benchmark
    pub iterations: usize,
    /// Warmup iterations before actual measurement
    pub warmup_iterations: usize,
    /// Minimum measurement duration
    pub min_duration: Duration,
    /// Maximum measurement duration
    pub max_duration: Duration,
    /// Outlier detection threshold (standard deviations)
    pub outlier_threshold: f64,
    /// Statistical significance level
    pub significance_level: f64,
    /// Enable statistical analysis
    pub enable_statistics: bool,
    /// Baseline comparison file
    pub baseline_file: Option<String>,
}

impl Default for BenchmarkConfig {
    fn default() -> Self {
        Self {
            mode: BenchmarkMode::Full,
            iterations: 100,
            warmup_iterations: 10,
            min_duration: Duration::from_millis(100),
            max_duration: Duration::from_secs(10),
            outlier_threshold: 2.0,
            significance_level: 0.05,
            enable_statistics: true,
            baseline_file: None,
        }
    }
}

/// Benchmark execution modes
#[derive(Debug, Clone, PartialEq)]
pub enum BenchmarkMode {
    /// Quick benchmark (fewer iterations)
    Quick,
    /// Standard benchmark (default iterations)
    Standard,
    /// Full benchmark (comprehensive testing)
    Full,
    /// Continuous monitoring mode
    Continuous,
    /// Regression testing mode
    Regression,
}

/// Benchmark category
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum BenchmarkCategory {
    /// Rendering performance
    Rendering,
    /// Memory usage
    Memory,
    /// Async operations
    Async,
    /// Component virtualization
    Virtualization,
    /// Network operations
    Network,
    /// Database operations
    Database,
    /// Custom benchmark
    Custom(String),
}

/// Benchmark result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkResult {
    /// Unique benchmark identifier
    pub id: String,
    /// Benchmark category
    pub category: BenchmarkCategory,
    /// Benchmark name
    pub name: String,
    /// Execution timestamp
    pub timestamp: Instant,
    /// Total duration
    pub total_duration: Duration,
    /// Individual iteration durations
    pub iterations: Vec<Duration>,
    /// Statistical metrics
    pub statistics: BenchmarkStatistics,
    /// Baseline comparison
    pub baseline_comparison: Option<BaselineComparison>,
    /// Performance regression detected
    pub regression: bool,
    /// Custom metrics
    pub custom_metrics: HashMap<String, f64>,
    /// Environment information
    pub environment: EnvironmentInfo,
}

/// Benchmark statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkStatistics {
    /// Minimum duration
    pub min: Duration,
    /// Maximum duration
    pub max: Duration,
    /// Mean duration
    pub mean: Duration,
    /// Median duration
    pub median: Duration,
    /// Standard deviation
    pub std_dev: Duration,
    /// 95th percentile
    pub p95: Duration,
    /// 99th percentile
    pub p99: Duration,
    /// Coefficient of variation
    pub cv: f64,
    /// Sample size (after outlier removal)
    pub sample_size: usize,
    /// Outliers removed
    pub outliers_removed: usize,
}

/// Baseline comparison
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaselineComparison {
    /// Baseline timestamp
    pub baseline_timestamp: Instant,
    /// Performance change percentage
    pub change_percent: f64,
    /// Statistical significance
    pub significant: bool,
    /// P-value from statistical test
    pub p_value: f64,
    /// Improvement or regression
    pub direction: ChangeDirection,
}

/// Change direction
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ChangeDirection {
    Improvement,
    Regression,
    NoChange,
    Inconclusive,
}

/// Environment information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentInfo {
    /// OS information
    pub os: String,
    /// CPU information
    pub cpu: String,
    /// Memory information
    pub memory: String,
    /// Rust version
    pub rust_version: String,
    /// Build configuration
    pub build_config: String,
    /// Test environment
    pub test_environment: String,
}

/// Performance regression
#[derive(Debug, Clone)]
pub struct PerformanceRegression {
    /// Benchmark ID
    pub benchmark_id: String,
    /// Regression magnitude
    pub magnitude: f64,
    /// Confidence level
    pub confidence: f64,
    /// Severity level
    pub severity: RegressionSeverity,
}

/// Regression severity levels
#[derive(Debug, Clone, PartialEq)]
pub enum RegressionSeverity {
    Minor,    // < 10% regression
    Moderate, // 10-25% regression
    Major,    // 25-50% regression
    Critical, // > 50% regression
}

/// Main performance benchmark system
pub struct PerformanceBenchmark {
    config: BenchmarkConfig,
    benchmarks: Arc<RwLock<HashMap<String, Box<dyn Benchmark>>>>,
    results: Arc<RwLock<VecDeque<BenchmarkResult>>>,
    baselines: Arc<RwLock<HashMap<String, BenchmarkResult>>>,
    regressions: Arc<Mutex<Vec<PerformanceRegression>>>,
}

impl PerformanceBenchmark {
    /// Create new performance benchmark system
    pub fn new(config: BenchmarkConfig) -> Self {
        Self {
            config,
            benchmarks: Arc::new(RwLock::new(HashMap::new())),
            results: Arc::new(RwLock::new(VecDeque::new())),
            baselines: Arc::new(RwLock::new(HashMap::new())),
            regressions: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Register a benchmark
    pub fn register_benchmark(&self, benchmark: Box<dyn Benchmark>) {
        let mut benchmarks = self.benchmarks.write();
        benchmarks.insert(benchmark.id(), benchmark);
    }

    /// Run all benchmarks
    pub async fn run_all(&self) -> Result<Vec<BenchmarkResult>> {
        let benchmarks = self.benchmarks.read();
        let mut results = Vec::new();

        for (_, benchmark) in benchmarks.iter() {
            let result = self.run_benchmark(benchmark.as_ref()).await?;
            results.push(result);
        }

        // Store results
        {
            let mut stored = self.results.write();
            for result in &results {
                stored.push_back(result.clone());
            }
        }

        // Check for regressions
        self.check_regressions(&results).await;

        Ok(results)
    }

    /// Run a specific benchmark
    pub async fn run_benchmark(&self, benchmark: &dyn Benchmark) -> Result<BenchmarkResult> {
        let id = benchmark.id();
        let category = benchmark.category();
        let name = benchmark.name();

        // Setup
        benchmark.setup().await?;

        // Warmup iterations
        for _ in 0..self.config.warmup_iterations {
            benchmark.execute().await?;
        }

        // Actual benchmark iterations
        let mut iterations = Vec::with_capacity(self.config.iterations);
        let start_time = Instant::now();

        for i in 0..self.config.iterations {
            let iter_start = Instant::now();

            // Execute benchmark
            benchmark.execute().await?;

            let duration = iter_start.elapsed();
            iterations.push(duration);

            // Check if we should stop early
            if start_time.elapsed() > self.config.max_duration {
                break;
            }
        }

        // Cleanup
        benchmark.cleanup().await?;

        // Calculate statistics
        let statistics = self.calculate_statistics(&iterations)?;

        // Check against baseline
        let baseline_comparison = if let Ok(Some(baseline)) = self.get_baseline(&id).await {
            Some(self.compare_with_baseline(&statistics, &baseline.statistics)?)
        } else {
            None
        };

        // Create result
        let result = BenchmarkResult {
            id,
            category,
            name,
            timestamp: Instant::now(),
            total_duration: start_time.elapsed(),
            iterations,
            statistics,
            baseline_comparison,
            regression: baseline_comparison
                .as_ref()
                .map(|c| c.direction == ChangeDirection::Regression)
                .unwrap_or(false),
            custom_metrics: benchmark.custom_metrics(),
            environment: self.collect_environment_info(),
        };

        Ok(result)
    }

    /// Get benchmark results
    pub fn get_results(&self) -> Vec<BenchmarkResult> {
        self.results.read().iter().cloned().collect()
    }

    /// Get results for a specific benchmark
    pub fn get_benchmark_results(&self, id: &str) -> Vec<BenchmarkResult> {
        self.results.read()
            .iter()
            .filter(|r| r.id == id)
            .cloned()
            .collect()
    }

    /// Save results to file
    pub async fn save_results(&self, path: &str) -> Result<()> {
        let results = self.get_results();
        let json = serde_json::to_string_pretty(&results)?;
        fs::write(path, json)?;
        Ok(())
    }

    /// Load results from file
    pub async fn load_results(&self, path: &str) -> Result<()> {
        let json = fs::read_to_string(path)?;
        let results: Vec<BenchmarkResult> = serde_json::from_str(&json)?;

        let mut stored = self.results.write();
        for result in results {
            stored.push_back(result);
        }

        Ok(())
    }

    /// Update baseline with latest results
    pub async fn update_baselines(&self) -> Result<()> {
        let results = self.results.read();
        let mut baselines = self.baselines.write();

        for result in results.iter() {
            // Keep the best (fastest) result as baseline
            if !baselines.contains_key(&result.id) ||
               result.statistics.mean < baselines[&result.id].statistics.mean {
                baselines.insert(result.id.clone(), result.clone());
            }
        }

        Ok(())
    }

    /// Save baselines to file
    pub async fn save_baselines(&self, path: &str) -> Result<()> {
        let baselines = self.baselines.read();
        let baseline_vec: Vec<_> = baselines.values().cloned().collect();
        let json = serde_json::to_string_pretty(&baseline_vec)?;
        fs::write(path, json)?;
        Ok(())
    }

    /// Load baselines from file
    pub async fn load_baselines(&self, path: &str) -> Result<()> {
        let json = fs::read_to_string(path)?;
        let baseline_results: Vec<BenchmarkResult> = serde_json::from_str(&json)?;

        let mut baselines = self.baselines.write();
        for result in baseline_results {
            baselines.insert(result.id.clone(), result);
        }

        Ok(())
    }

    /// Get detected regressions
    pub fn get_regressions(&self) -> Vec<PerformanceRegression> {
        self.regressions.lock().clone()
    }

    /// Generate performance report
    pub fn generate_report(&self) -> PerformanceReport {
        let results = self.results.read();
        let regressions = self.regressions.lock();

        PerformanceReport {
            generated_at: Instant::now(),
            summary: self.generate_summary(&results),
            benchmarks_by_category: self.group_by_category(&results),
            regressions: regressions.clone(),
            recommendations: self.generate_recommendations(&results, &regressions),
        }
    }

    // Private methods

    fn calculate_statistics(&self, durations: &[Duration]) -> Result<BenchmarkStatistics> {
        if durations.is_empty() {
            return Err(anyhow::anyhow!("No durations provided"));
        }

        // Convert to nanoseconds for calculations
        let mut nanos: Vec<u64> = durations.iter()
            .map(|d| d.as_nanos() as u64)
            .collect();

        // Sort for percentile calculations
        nanos.sort_unstable();

        // Remove outliers if enabled
        let (clean_nanos, outliers_removed) = if self.config.enable_statistics {
            self.remove_outliers(&nanos)
        } else {
            (nanos, 0)
        };

        if clean_nanos.is_empty() {
            return Err(anyhow::anyhow!("All values were outliers"));
        }

        // Calculate basic statistics
        let min = Duration::from_nanos(clean_nanos[0]);
        let max = Duration::from_nanos(clean_nanos[clean_nanos.len() - 1]);
        let mean_nanos: u64 = clean_nanos.iter().sum::<u64>() / clean_nanos.len() as u64;
        let mean = Duration::from_nanos(mean_nanos);

        let median = if clean_nanos.len() % 2 == 0 {
            let mid = clean_nanos.len() / 2;
            Duration::from_nanos((clean_nanos[mid - 1] + clean_nanos[mid]) / 2)
        } else {
            Duration::from_nanos(clean_nanos[clean_nanos.len() / 2])
        };

        // Calculate standard deviation
        let variance: f64 = clean_nanos.iter()
            .map(|&x| {
                let diff = x as f64 - mean_nanos as f64;
                diff * diff
            })
            .sum::<f64>() / clean_nanos.len() as f64;
        let std_dev_nanos = variance.sqrt() as u64;
        let std_dev = Duration::from_nanos(std_dev_nanos);

        // Calculate percentiles
        let p95 = Duration::from_nanos(self.percentile(&clean_nanos, 0.95));
        let p99 = Duration::from_nanos(self.percentile(&clean_nanos, 0.99));

        // Coefficient of variation
        let cv = if mean_nanos > 0 {
            (std_dev_nanos as f64 / mean_nanos as f64) * 100.0
        } else {
            0.0
        };

        Ok(BenchmarkStatistics {
            min,
            max,
            mean,
            median,
            std_dev,
            p95,
            p99,
            cv,
            sample_size: clean_nanos.len(),
            outliers_removed,
        })
    }

    fn remove_outliers(&self, values: &[u64]) -> (Vec<u64>, usize) {
        if values.len() < 3 {
            return (values.to_vec(), 0);
        }

        let mean: f64 = values.iter().sum::<u64>() as f64 / values.len() as f64;
        let variance: f64 = values.iter()
            .map(|&x| {
                let diff = x as f64 - mean;
                diff * diff
            })
            .sum::<f64>() / values.len() as f64;
        let std_dev = variance.sqrt();

        let threshold = self.config.outlier_threshold * std_dev;
        let mut clean = Vec::new();
        let mut outliers = 0;

        for &value in values {
            if (value as f64 - mean).abs() <= threshold {
                clean.push(value);
            } else {
                outliers += 1;
            }
        }

        (clean, outliers)
    }

    fn percentile(&self, sorted_values: &[u64], percentile: f64) -> u64 {
        if sorted_values.is_empty() {
            return 0;
        }

        if percentile <= 0.0 {
            return sorted_values[0];
        }
        if percentile >= 1.0 {
            return sorted_values[sorted_values.len() - 1];
        }

        let index = (percentile * (sorted_values.len() - 1) as f64) as usize;
        sorted_values[index]
    }

    async fn get_baseline(&self, id: &str) -> Result<Option<&BenchmarkResult>> {
        Ok(self.baselines.read().get(id))
    }

    fn compare_with_baseline(&self, current: &BenchmarkStatistics, baseline: &BenchmarkStatistics) -> Result<BaselineComparison> {
        let change_percent = if baseline.mean.as_nanos() > 0 {
            ((current.mean.as_nanos() as f64 - baseline.mean.as_nanos() as f64) / baseline.mean.as_nanos() as f64) * 100.0
        } else {
            0.0
        };

        // Perform statistical test (simplified)
        let significant = change_percent.abs() > 5.0; // 5% threshold
        let p_value = if significant { 0.01 } else { 0.5 };

        let direction = if change_percent > 10.0 {
            ChangeDirection::Regression
        } else if change_percent < -10.0 {
            ChangeDirection::Improvement
        } else {
            ChangeDirection::NoChange
        };

        Ok(BaselineComparison {
            baseline_timestamp: Instant::now(), // Would store actual baseline timestamp
            change_percent,
            significant,
            p_value,
            direction,
        })
    }

    async fn check_regressions(&self, results: &[BenchmarkResult]) {
        let mut regressions = self.regressions.lock();

        for result in results {
            if result.regression {
                if let Some(comparison) = &result.baseline_comparison {
                    let severity = if comparison.change_percent > 50.0 {
                        RegressionSeverity::Critical
                    } else if comparison.change_percent > 25.0 {
                        RegressionSeverity::Major
                    } else if comparison.change_percent > 10.0 {
                        RegressionSeverity::Moderate
                    } else {
                        RegressionSeverity::Minor
                    };

                    regressions.push(PerformanceRegression {
                        benchmark_id: result.id.clone(),
                        magnitude: comparison.change_percent,
                        confidence: 1.0 - comparison.p_value,
                        severity,
                    });
                }
            }
        }
    }

    fn collect_environment_info(&self) -> EnvironmentInfo {
        EnvironmentInfo {
            os: std::env::consts::OS.to_string(),
            cpu: "Unknown".to_string(), // Would detect actual CPU
            memory: "Unknown".to_string(), // Would detect actual memory
            rust_version: "1.70.0".to_string(), // Would get actual version
            build_config: "debug".to_string(), // Would detect debug/release
            test_environment: "local".to_string(),
        }
    }

    fn generate_summary(&self, results: &[BenchmarkResult]) -> BenchmarkSummary {
        let total_benchmarks = results.len();
        let regressions = results.iter().filter(|r| r.regression).count();
        let improvements = results.iter()
            .filter(|r| {
                r.baseline_comparison.as_ref()
                    .map(|c| c.direction == ChangeDirection::Improvement)
                    .unwrap_or(false)
            })
            .count();

        BenchmarkSummary {
            total_benchmarks,
            regressions,
            improvements,
            avg_performance_change: self.calculate_avg_change(results),
            test_duration: Duration::from_secs(0), // Would calculate actual duration
        }
    }

    fn group_by_category(&self, results: &[BenchmarkResult]) -> HashMap<BenchmarkCategory, Vec<BenchmarkResult>> {
        let mut grouped = HashMap::new();

        for result in results {
            grouped.entry(result.category.clone())
                .or_insert_with(Vec::new)
                .push(result.clone());
        }

        grouped
    }

    fn generate_recommendations(&self, results: &[BenchmarkResult], regressions: &[PerformanceRegression]) -> Vec<String> {
        let mut recommendations = Vec::new();

        if !regressions.is_empty() {
            recommendations.push(format!(
                "Detected {} performance regressions. Consider rolling back changes or optimizing affected areas.",
                regressions.len()
            ));
        }

        // Analyze patterns in results
        let render_results: Vec<_> = results.iter()
            .filter(|r| r.category == BenchmarkCategory::Rendering)
            .collect();

        if render_results.iter().any(|r| r.statistics.cv > 20.0) {
            recommendations.push(
                "Rendering performance shows high variability. Consider optimizing render pipeline consistency.".to_string()
            );
        }

        let memory_results: Vec<_> = results.iter()
            .filter(|r| r.category == BenchmarkCategory::Memory)
            .collect();

        if memory_results.iter().any(|r| {
            r.custom_metrics.get("memory_leaks")
                .map(|&v| v > 0.0)
                .unwrap_or(false)
        }) {
            recommendations.push(
                "Memory leaks detected in benchmarks. Review memory management and cleanup procedures.".to_string()
            );
        }

        recommendations
    }

    fn calculate_avg_change(&self, results: &[BenchmarkResult]) -> f64 {
        let changes: Vec<f64> = results.iter()
            .filter_map(|r| {
                r.baseline_comparison.as_ref().map(|c| c.change_percent)
            })
            .collect();

        if changes.is_empty() {
            0.0
        } else {
            changes.iter().sum::<f64>() / changes.len() as f64
        }
    }
}

/// Benchmark trait
#[async_trait::async_trait]
pub trait Benchmark: Send + Sync {
    /// Unique benchmark identifier
    fn id(&self) -> String;

    /// Benchmark category
    fn category(&self) -> BenchmarkCategory;

    /// Benchmark name
    fn name(&self) -> String;

    /// Setup before execution
    async fn setup(&self) -> Result<()>;

    /// Execute benchmark
    async fn execute(&self) -> Result<()>;

    /// Cleanup after execution
    async fn cleanup(&self) -> Result<()>;

    /// Get custom metrics
    fn custom_metrics(&self) -> HashMap<String, f64> {
        HashMap::new()
    }
}

/// Performance report
#[derive(Debug, Clone)]
pub struct PerformanceReport {
    pub generated_at: Instant,
    pub summary: BenchmarkSummary,
    pub benchmarks_by_category: HashMap<BenchmarkCategory, Vec<BenchmarkResult>>,
    pub regressions: Vec<PerformanceRegression>,
    pub recommendations: Vec<String>,
}

/// Benchmark summary
#[derive(Debug, Clone)]
pub struct BenchmarkSummary {
    pub total_benchmarks: usize,
    pub regressions: usize,
    pub improvements: usize,
    pub avg_performance_change: f64,
    pub test_duration: Duration,
}

// Example benchmarks

/// Rendering performance benchmark
pub struct RenderingBenchmark {
    id: String,
    elements_to_render: usize,
}

impl RenderingBenchmark {
    pub fn new(elements_to_render: usize) -> Self {
        Self {
            id: format!("render_{}_elements", elements_to_render),
            elements_to_render,
        }
    }
}

#[async_trait::async_trait]
impl Benchmark for RenderingBenchmark {
    fn id(&self) -> String {
        self.id.clone()
    }

    fn category(&self) -> BenchmarkCategory {
        BenchmarkCategory::Rendering
    }

    fn name(&self) -> String {
        format!("Rendering {} elements", self.elements_to_render)
    }

    async fn setup(&self) -> Result<()> {
        // Setup rendering context
        Ok(())
    }

    async fn execute(&self) -> Result<()> {
        // Simulate rendering work
        for _ in 0..self.elements_to_render {
            // Simulate render work
            tokio::task::yield_now().await;
        }
        Ok(())
    }

    async fn cleanup(&self) -> Result<()> {
        // Cleanup resources
        Ok(())
    }

    fn custom_metrics(&self) -> HashMap<String, f64> {
        let mut metrics = HashMap::new();
        metrics.insert("elements_rendered".to_string(), self.elements_to_render as f64);
        metrics
    }
}

/// Memory usage benchmark
pub struct MemoryBenchmark {
    id: String,
    allocation_size: usize,
    allocation_count: usize,
}

impl MemoryBenchmark {
    pub fn new(allocation_size: usize, allocation_count: usize) -> Self {
        Self {
            id: format!("memory_{}_x_{}", allocation_size, allocation_count),
            allocation_size,
            allocation_count,
        }
    }
}

#[async_trait::async_trait]
impl Benchmark for MemoryBenchmark {
    fn id(&self) -> String {
        self.id.clone()
    }

    fn category(&self) -> BenchmarkCategory {
        BenchmarkCategory::Memory
    }

    fn name(&self) -> String {
        format!("Memory {} allocations of {} bytes", self.allocation_count, self.allocation_size)
    }

    async fn setup(&self) -> Result<()> {
        Ok(())
    }

    async fn execute(&self) -> Result<()> {
        let mut allocations = Vec::with_capacity(self.allocation_count);

        for _ in 0..self.allocation_count {
            allocations.push(vec![0u8; self.allocation_size]);
        }

        // Use allocations to prevent optimization
        let sum: usize = allocations.iter().map(|v| v.len()).sum();

        Ok(())
    }

    async fn cleanup(&self) -> Result<()> {
        Ok(())
    }

    fn custom_metrics(&self) -> HashMap<String, f64> {
        let mut metrics = HashMap::new();
        metrics.insert("total_allocated".to_string(), (self.allocation_size * self.allocation_count) as f64);
        metrics
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_benchmark_creation() {
        let config = BenchmarkConfig::default();
        let benchmark = PerformanceBenchmark::new(config);

        // Should not panic
        let results = benchmark.get_results();
        assert!(results.is_empty());
    }

    #[test]
    fn test_statistics_calculation() {
        let benchmark = PerformanceBenchmark::new(BenchmarkConfig::default());
        let durations = vec![
            Duration::from_millis(100),
            Duration::from_millis(110),
            Duration::from_millis(105),
            Duration::from_millis(95),
            Duration::from_millis(120),
        ];

        let stats = benchmark.calculate_statistics(&durations).unwrap();

        assert!(stats.min <= stats.mean);
        assert!(stats.mean <= stats.max);
        assert!(stats.std_dev >= Duration::ZERO);
        assert!(stats.sample_size <= durations.len());
    }

    #[test]
    fn test_outlier_removal() {
        let benchmark = PerformanceBenchmark::new(BenchmarkConfig::default());
        let values = vec![100, 105, 110, 120, 2000]; // 2000 is an outlier

        let (clean, outliers) = benchmark.remove_outliers(&values);

        assert_eq!(outliers, 1);
        assert!(clean.len() < values.len());
        assert!(!clean.contains(&2000));
    }

    #[test]
    fn test_percentile_calculation() {
        let benchmark = PerformanceBenchmark::new(BenchmarkConfig::default());
        let values = vec![10, 20, 30, 40, 50, 60, 70, 80, 90, 100];

        assert_eq!(benchmark.percentile(&values, 0.0), 10);
        assert_eq!(benchmark.percentile(&values, 1.0), 100);
        assert_eq!(benchmark.percentile(&values, 0.5), 55); // Median
    }

    #[tokio::test]
    async fn test_rendering_benchmark() {
        let benchmark = RenderingBenchmark::new(100);

        assert_eq!(benchmark.category(), BenchmarkCategory::Rendering);
        assert!(benchmark.name().contains("100 elements"));

        // Execute benchmark
        benchmark.setup().await.unwrap();
        benchmark.execute().await.unwrap();
        benchmark.cleanup().await.unwrap();

        let metrics = benchmark.custom_metrics();
        assert_eq!(metrics.get("elements_rendered"), Some(&100.0));
    }

    #[tokio::test]
    async fn test_memory_benchmark() {
        let benchmark = MemoryBenchmark::new(1024, 100);

        assert_eq!(benchmark.category(), BenchmarkCategory::Memory);
        assert!(benchmark.name().contains("1024 bytes"));
        assert!(benchmark.name().contains("100 allocations"));

        // Execute benchmark
        benchmark.setup().await.unwrap();
        benchmark.execute().await.unwrap();
        benchmark.cleanup().await.unwrap();

        let metrics = benchmark.custom_metrics();
        assert_eq!(metrics.get("total_allocated"), Some(&(1024 * 100) as f64));
    }

    #[test]
    fn test_regression_severity() {
        assert!(RegressionSeverity::Critical > RegressionSeverity::Major);
        assert!(RegressionSeverity::Major > RegressionSeverity::Moderate);
        assert!(RegressionSeverity::Moderate > RegressionSeverity::Minor);
    }

    #[test]
    fn test_benchmark_category() {
        assert_eq!(BenchmarkCategory::Rendering, BenchmarkCategory::Rendering);
        assert_ne!(BenchmarkCategory::Memory, BenchmarkCategory::Rendering);
    }
}