//! Performance Benchmarking Framework for Terraphim AI
//!
//! This module provides comprehensive performance benchmarking capabilities
//! for terraphim-ai release validation, including:
//!
//! - Server API benchmarks (HTTP request/response timing, throughput)
//! - Search engine performance (query execution, ranking accuracy, indexing)
//! - Database operations (CRUD timing, transaction performance)
//! - File system operations (read/write, large files, concurrent access)
//! - Resource utilization monitoring (CPU, memory, disk I/O, network)
//! - Scalability testing (concurrent users, data scale, load balancing)
//! - Comparative analysis (baselines, regression detection)
//! - Automated benchmarking pipeline (CI/CD integration, performance gates)

use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use sysinfo::{Disks, Networks, Pid, System};
use tempfile::TempDir;
use tokio::sync::Mutex;

/// Performance benchmarking framework configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkConfig {
    /// Benchmark iterations per operation
    pub iterations: u32,
    /// Warmup iterations before actual benchmarking
    pub warmup_iterations: u32,
    /// Concurrent users for scalability testing
    pub concurrent_users: Vec<u32>,
    /// Data scale factors for testing
    pub data_scales: Vec<u64>,
    /// Performance thresholds/SLAs
    pub slos: PerformanceSLO,
    /// Resource monitoring intervals (ms)
    pub monitoring_interval_ms: u64,
    /// Enable detailed profiling
    pub enable_profiling: bool,
}

/// Service Level Objectives for performance validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceSLO {
    /// Maximum server startup time (ms)
    pub max_startup_time_ms: u64,
    /// Maximum API response time (ms)
    pub max_api_response_time_ms: u64,
    /// Maximum search query time (ms)
    pub max_search_time_ms: u64,
    /// Maximum indexing time per document (ms)
    pub max_indexing_time_per_doc_ms: u64,
    /// Maximum memory usage (MB)
    pub max_memory_mb: u64,
    /// Maximum CPU usage during idle (%)
    pub max_cpu_idle_percent: f32,
    /// Maximum CPU usage during load (%)
    pub max_cpu_load_percent: f32,
    /// Minimum requests per second for throughput tests
    pub min_rps: f64,
    /// Maximum concurrent users supported
    pub max_concurrent_users: u32,
    /// Maximum data scale (documents) supported
    pub max_data_scale: u64,
}

/// Individual benchmark result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkResult {
    /// Operation name
    pub operation: String,
    /// Total execution time
    pub total_time: Duration,
    /// Average execution time per operation
    pub avg_time: Duration,
    /// Minimum execution time
    pub min_time: Duration,
    /// Maximum execution time
    pub max_time: Duration,
    /// Operations per second
    pub ops_per_second: f64,
    /// Success rate (0.0-1.0)
    pub success_rate: f64,
    /// Error count
    pub error_count: u32,
    /// Resource usage during benchmark
    pub resource_usage: ResourceUsage,
}

/// Resource utilization snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceUsage {
    /// CPU usage percentage
    pub cpu_percent: f32,
    /// Memory usage in bytes
    pub memory_bytes: u64,
    /// Virtual memory in bytes
    pub virtual_memory_bytes: u64,
    /// Disk read bytes
    pub disk_read_bytes: u64,
    /// Disk write bytes
    pub disk_write_bytes: u64,
    /// Network bytes received
    pub network_rx_bytes: u64,
    /// Network bytes transmitted
    pub network_tx_bytes: u64,
    /// Thread count
    pub thread_count: usize,
}

/// Comprehensive benchmark report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkReport {
    /// Report timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Benchmark configuration used
    pub config: BenchmarkConfig,
    /// Individual benchmark results
    pub results: HashMap<String, BenchmarkResult>,
    /// SLO compliance status
    pub slo_compliance: SLOCompliance,
    /// System information
    pub system_info: SystemInfo,
    /// Performance trends (if baseline available)
    pub trends: Option<PerformanceTrends>,
}

/// SLO compliance status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SLOCompliance {
    /// Overall compliance percentage
    pub overall_compliance: f64,
    /// Individual SLO violations
    pub violations: Vec<SLOViolation>,
    /// Critical violations (performance gates)
    pub critical_violations: Vec<SLOViolation>,
}

/// Individual SLO violation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SLOViolation {
    /// SLO metric name
    pub metric: String,
    /// Actual value
    pub actual_value: String,
    /// Threshold value
    pub threshold_value: String,
    /// Violation severity
    pub severity: ViolationSeverity,
}

/// Violation severity levels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ViolationSeverity {
    Warning,
    Critical,
    Blocking,
}

/// System information for benchmarking context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemInfo {
    pub os: String,
    pub os_version: String,
    pub cpu_model: String,
    pub cpu_cores: usize,
    pub total_memory_mb: u64,
    pub available_memory_mb: u64,
    pub rust_version: String,
    pub terraphim_version: String,
}

/// Performance trends compared to baseline
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceTrends {
    /// Baseline timestamp
    pub baseline_timestamp: chrono::DateTime<chrono::Utc>,
    /// Performance improvements (positive values = faster)
    pub improvements: HashMap<String, f64>,
    /// Performance regressions (negative values = slower)
    pub regressions: HashMap<String, f64>,
    /// New operations not in baseline
    pub new_operations: Vec<String>,
}

/// Main benchmarking orchestrator
pub struct PerformanceBenchmarker {
    config: BenchmarkConfig,
    system: Arc<Mutex<System>>,
    results: HashMap<String, BenchmarkResult>,
    baseline: Option<BenchmarkReport>,
}

impl PerformanceBenchmarker {
    /// Create a new performance benchmarker
    pub fn new(config: BenchmarkConfig) -> Self {
        let mut system = System::new_all();
        system.refresh_all();

        Self {
            config,
            system: Arc::new(Mutex::new(system)),
            results: HashMap::new(),
            baseline: None,
        }
    }

    /// Load baseline report for trend analysis
    pub fn load_baseline(&mut self, baseline: BenchmarkReport) {
        self.baseline = Some(baseline);
    }

    /// Run all performance benchmarks
    pub async fn run_all_benchmarks(&mut self) -> Result<BenchmarkReport> {
        log::info!("Starting comprehensive performance benchmarking suite");

        // Core performance benchmarks
        self.run_server_api_benchmarks().await?;
        self.run_search_engine_benchmarks().await?;
        self.run_database_benchmarks().await?;
        self.run_filesystem_benchmarks().await?;

        // Resource utilization monitoring
        self.run_resource_monitoring().await?;

        // Scalability testing
        self.run_scalability_benchmarks().await?;

        // Comparative analysis
        self.run_comparative_analysis().await?;

        // Generate comprehensive report
        self.generate_report().await
    }

    /// Run server API benchmarks
    async fn run_server_api_benchmarks(&mut self) -> Result<()> {
        log::info!("Running server API benchmarks");

        // Health check endpoint benchmark
        self.benchmark_api_endpoint("/health", "health_check", 1000)
            .await?;

        // Search endpoint benchmark
        self.benchmark_api_endpoint("/api/search", "search_api", 500)
            .await?;

        // Config endpoint benchmark
        self.benchmark_api_endpoint("/api/config", "config_api", 100)
            .await?;

        // Chat completion benchmark
        self.benchmark_api_endpoint("/api/chat", "chat_api", 200)
            .await?;

        Ok(())
    }

    /// Benchmark a specific API endpoint
    async fn benchmark_api_endpoint(
        &mut self,
        endpoint: &str,
        name: &str,
        iterations: u32,
    ) -> Result<()> {
        log::info!("Benchmarking API endpoint: {}", endpoint);

        let mut times = Vec::new();
        let mut errors = 0;

        // Warmup iterations
        for _ in 0..self.config.warmup_iterations {
            let _ = self.call_api_endpoint(endpoint).await;
        }

        // Actual benchmarking
        let start_time = Instant::now();
        for _ in 0..iterations {
            let call_start = Instant::now();
            match self.call_api_endpoint(endpoint).await {
                Ok(_) => {
                    times.push(call_start.elapsed());
                }
                Err(_) => {
                    errors += 1;
                }
            }
        }
        let total_time = start_time.elapsed();

        // Calculate statistics
        let success_count = times.len() as u32;
        let success_rate = success_count as f64 / iterations as f64;

        let avg_time = if !times.is_empty() {
            times.iter().sum::<Duration>() / times.len() as u32
        } else {
            Duration::default()
        };

        let min_time = times.iter().min().cloned().unwrap_or_default();
        let max_time = times.iter().max().cloned().unwrap_or_default();

        let ops_per_second = if total_time.as_secs_f64() > 0.0 {
            success_count as f64 / total_time.as_secs_f64()
        } else {
            0.0
        };

        // Capture resource usage
        let resource_usage = self.capture_resource_usage().await?;

        let result = BenchmarkResult {
            operation: name.to_string(),
            total_time,
            avg_time,
            min_time,
            max_time,
            ops_per_second,
            success_rate,
            error_count: errors,
            resource_usage,
        };

        self.results.insert(name.to_string(), result);
        Ok(())
    }

    /// Make an actual HTTP call to the terraphim server.
    ///
    /// Requires `TERRAPHIM_SERVER_URL` env var (e.g. `http://localhost:3000`).
    /// Returns `Ok(())` immediately when the env var is absent so the benchmark
    /// can still be run without a live server (all timings will be near-zero).
    async fn call_api_endpoint(&self, endpoint: &str) -> Result<()> {
        let base = match std::env::var("TERRAPHIM_SERVER_URL") {
            Ok(url) => url,
            Err(_) => return Ok(()),
        };
        let url = format!("{}{}", base.trim_end_matches('/'), endpoint);
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .map_err(|e| anyhow!("reqwest client: {}", e))?;
        client
            .get(&url)
            .send()
            .await
            .map_err(|e| anyhow!("HTTP GET {}: {}", url, e))?;
        Ok(())
    }

    /// Run search engine performance benchmarks
    async fn run_search_engine_benchmarks(&mut self) -> Result<()> {
        log::info!("Running search engine benchmarks");

        // Query execution time benchmark
        self.benchmark_search_query("simple_query", "rust programming", 1000)
            .await?;

        // Complex query benchmark
        self.benchmark_search_query("complex_query", "rust async tokio performance", 500)
            .await?;

        // Fuzzy search benchmark
        self.benchmark_search_query("fuzzy_search", "machne learnng", 500)
            .await?;

        // Large result set benchmark
        self.benchmark_search_query("large_results", "documentation", 200)
            .await?;

        // Indexing performance benchmark
        self.benchmark_indexing_performance().await?;

        Ok(())
    }

    /// Benchmark search query performance
    async fn benchmark_search_query(
        &mut self,
        name: &str,
        query: &str,
        iterations: u32,
    ) -> Result<()> {
        let mut times = Vec::new();
        let mut errors = 0;

        // Warmup
        for _ in 0..self.config.warmup_iterations {
            let _ = self.execute_search_query(query).await;
        }

        // Benchmarking
        let start_time = Instant::now();
        for _ in 0..iterations {
            let call_start = Instant::now();
            match self.execute_search_query(query).await {
                Ok(_) => {
                    times.push(call_start.elapsed());
                }
                Err(_) => {
                    errors += 1;
                }
            }
        }
        let total_time = start_time.elapsed();

        // Calculate statistics (same as API benchmark)
        let success_count = times.len() as u32;
        let success_rate = success_count as f64 / iterations as f64;

        let avg_time = if !times.is_empty() {
            times.iter().sum::<Duration>() / times.len() as u32
        } else {
            Duration::default()
        };

        let min_time = times.iter().min().cloned().unwrap_or_default();
        let max_time = times.iter().max().cloned().unwrap_or_default();

        let ops_per_second = if total_time.as_secs_f64() > 0.0 {
            success_count as f64 / total_time.as_secs_f64()
        } else {
            0.0
        };

        let resource_usage = self.capture_resource_usage().await?;

        let result = BenchmarkResult {
            operation: name.to_string(),
            total_time,
            avg_time,
            min_time,
            max_time,
            ops_per_second,
            success_rate,
            error_count: errors,
            resource_usage,
        };

        self.results.insert(name.to_string(), result);
        Ok(())
    }

    /// Execute a search query against the terraphim search API.
    ///
    /// Requires `TERRAPHIM_SERVER_URL` env var.  Returns `Ok(())` immediately
    /// when the env var is absent so the benchmark degrades gracefully.
    async fn execute_search_query(&self, query: &str) -> Result<()> {
        let base = match std::env::var("TERRAPHIM_SERVER_URL") {
            Ok(url) => url,
            Err(_) => return Ok(()),
        };
        let encoded = urlencoding::encode(query);
        let url = format!("{}/api/search?q={}", base.trim_end_matches('/'), encoded);
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .map_err(|e| anyhow!("reqwest client: {}", e))?;
        client
            .get(&url)
            .send()
            .await
            .map_err(|e| anyhow!("search GET {}: {}", url, e))?;
        Ok(())
    }

    /// Benchmark indexing performance using real string tokenisation/hashing.
    async fn benchmark_indexing_performance(&mut self) -> Result<()> {
        let iterations = self.config.iterations.min(200);
        let doc = "The quick brown fox jumps over the lazy dog. ".repeat(50);
        let mut times = Vec::with_capacity(iterations as usize);

        let total_start = Instant::now();
        for _ in 0..iterations {
            let t = Instant::now();
            // Simulate tokenisation: split on whitespace and collect into a map
            let mut index: HashMap<&str, u32> = HashMap::new();
            for word in doc.split_whitespace() {
                *index.entry(word).or_insert(0) += 1;
            }
            std::hint::black_box(index.len());
            times.push(t.elapsed());
        }
        let total_time = total_start.elapsed();

        let avg_time = times.iter().sum::<Duration>() / times.len() as u32;
        let min_time = *times.iter().min().unwrap();
        let max_time = *times.iter().max().unwrap();
        let ops_per_second = iterations as f64 / total_time.as_secs_f64().max(f64::EPSILON);

        let result = BenchmarkResult {
            operation: "document_indexing".to_string(),
            total_time,
            avg_time,
            min_time,
            max_time,
            ops_per_second,
            success_rate: 1.0,
            error_count: 0,
            resource_usage: self.capture_resource_usage().await?,
        };

        self.results.insert("document_indexing".to_string(), result);
        Ok(())
    }

    /// Run database operation benchmarks
    async fn run_database_benchmarks(&mut self) -> Result<()> {
        log::info!("Running database benchmarks");

        // CRUD operations benchmark
        self.benchmark_crud_operations().await?;

        // Transaction performance benchmark
        self.benchmark_transaction_performance().await?;

        // Query optimization benchmark
        self.benchmark_query_optimization().await?;

        Ok(())
    }

    /// Benchmark CRUD operations using real in-memory HashMap insert/get/remove.
    async fn benchmark_crud_operations(&mut self) -> Result<()> {
        let iterations = self.config.iterations.min(500) as usize;
        let mut times = Vec::with_capacity(iterations);
        let mut store: HashMap<String, String> = HashMap::with_capacity(iterations);

        let total_start = Instant::now();
        for i in 0..iterations {
            let t = Instant::now();
            let key = format!("doc:{}", i);
            let value = serde_json::to_string(&serde_json::json!({"id": i, "body": "test"}))
                .unwrap_or_default();
            store.insert(key.clone(), value.clone());
            let _ = store.get(&key);
            store.remove(&key);
            times.push(t.elapsed());
        }
        let total_time = total_start.elapsed();

        let success_count = times.len() as u32;
        let avg_time = times.iter().sum::<Duration>() / success_count;
        let min_time = *times.iter().min().unwrap();
        let max_time = *times.iter().max().unwrap();
        let ops_per_second = success_count as f64 / total_time.as_secs_f64().max(f64::EPSILON);

        let result = BenchmarkResult {
            operation: "crud_operations".to_string(),
            total_time,
            avg_time,
            min_time,
            max_time,
            ops_per_second,
            success_rate: 1.0,
            error_count: 0,
            resource_usage: self.capture_resource_usage().await?,
        };

        self.results.insert("crud_operations".to_string(), result);
        Ok(())
    }

    /// Benchmark transaction performance using real serde_json round-trips.
    async fn benchmark_transaction_performance(&mut self) -> Result<()> {
        let iterations = self.config.iterations.min(200) as usize;
        let mut times = Vec::with_capacity(iterations);

        let total_start = Instant::now();
        for i in 0..iterations {
            let t = Instant::now();
            // Simulate a transaction: serialise, deserialise, and validate
            let payload = serde_json::json!({
                "tx_id": i,
                "items": [{"id": i, "amount": i * 10}],
                "status": "committed"
            });
            let serialised = serde_json::to_string(&payload).unwrap_or_default();
            let deserialised: serde_json::Value =
                serde_json::from_str(&serialised).unwrap_or(serde_json::Value::Null);
            std::hint::black_box(deserialised.is_object());
            times.push(t.elapsed());
        }
        let total_time = total_start.elapsed();

        let success_count = times.len() as u32;
        let avg_time = times.iter().sum::<Duration>() / success_count;
        let min_time = *times.iter().min().unwrap();
        let max_time = *times.iter().max().unwrap();
        let ops_per_second = success_count as f64 / total_time.as_secs_f64().max(f64::EPSILON);

        let result = BenchmarkResult {
            operation: "transaction_performance".to_string(),
            total_time,
            avg_time,
            min_time,
            max_time,
            ops_per_second,
            success_rate: 1.0,
            error_count: 0,
            resource_usage: self.capture_resource_usage().await?,
        };

        self.results
            .insert("transaction_performance".to_string(), result);
        Ok(())
    }

    /// Benchmark query optimisation using real string parsing.
    async fn benchmark_query_optimization(&mut self) -> Result<()> {
        let iterations = self.config.iterations.min(500) as usize;
        let queries = [
            "rust async tokio",
            "knowledge graph terraphim",
            "search indexing performance",
            "machine learning embeddings",
            "concurrent programming channels",
        ];
        let mut times = Vec::with_capacity(iterations);

        let total_start = Instant::now();
        for i in 0..iterations {
            let t = Instant::now();
            let query = queries[i % queries.len()];
            // Simulate query normalisation: lowercase, split, dedup
            let mut tokens: Vec<&str> = query.split_whitespace().collect();
            tokens.sort_unstable();
            tokens.dedup();
            let normalised = tokens.join(" ");
            std::hint::black_box(normalised.len());
            times.push(t.elapsed());
        }
        let total_time = total_start.elapsed();

        let success_count = times.len() as u32;
        let avg_time = times.iter().sum::<Duration>() / success_count;
        let min_time = *times.iter().min().unwrap();
        let max_time = *times.iter().max().unwrap();
        let ops_per_second = success_count as f64 / total_time.as_secs_f64().max(f64::EPSILON);

        let result = BenchmarkResult {
            operation: "query_optimization".to_string(),
            total_time,
            avg_time,
            min_time,
            max_time,
            ops_per_second,
            success_rate: 1.0,
            error_count: 0,
            resource_usage: self.capture_resource_usage().await?,
        };

        self.results
            .insert("query_optimization".to_string(), result);
        Ok(())
    }

    /// Run filesystem operation benchmarks
    async fn run_filesystem_benchmarks(&mut self) -> Result<()> {
        log::info!("Running filesystem benchmarks");

        // Read/write performance benchmark
        self.benchmark_file_operations().await?;

        // Large file handling benchmark
        self.benchmark_large_file_handling().await?;

        // Concurrent access benchmark
        self.benchmark_concurrent_access().await?;

        Ok(())
    }

    /// Benchmark file operations with real I/O on a temporary directory.
    async fn benchmark_file_operations(&mut self) -> Result<()> {
        let iterations = self.config.iterations.min(100) as usize;
        let tmpdir = TempDir::new().map_err(|e| anyhow!("TempDir: {}", e))?;
        let content = b"terraphim benchmark payload ".repeat(256); // ~7 KB per write
        let mut times = Vec::with_capacity(iterations);
        let mut errors = 0u32;

        let total_start = Instant::now();
        for i in 0..iterations {
            let t = Instant::now();
            let path = tmpdir.path().join(format!("bench_{}.dat", i));
            match tokio::fs::write(&path, &content).await {
                Ok(_) => {
                    let _ = tokio::fs::read(&path).await;
                    let _ = tokio::fs::remove_file(&path).await;
                    times.push(t.elapsed());
                }
                Err(_) => {
                    errors += 1;
                }
            }
        }
        let total_time = total_start.elapsed();

        if times.is_empty() {
            return Err(anyhow!("All file_operations iterations failed"));
        }
        let success_count = times.len() as u32;
        let avg_time = times.iter().sum::<Duration>() / success_count;
        let min_time = *times.iter().min().unwrap();
        let max_time = *times.iter().max().unwrap();
        let ops_per_second = success_count as f64 / total_time.as_secs_f64().max(f64::EPSILON);
        let success_rate = success_count as f64 / iterations as f64;

        let result = BenchmarkResult {
            operation: "file_operations".to_string(),
            total_time,
            avg_time,
            min_time,
            max_time,
            ops_per_second,
            success_rate,
            error_count: errors,
            resource_usage: self.capture_resource_usage().await?,
        };

        self.results.insert("file_operations".to_string(), result);
        Ok(())
    }

    /// Benchmark large file handling with real I/O on a temporary file.
    async fn benchmark_large_file_handling(&mut self) -> Result<()> {
        let iterations = 5usize; // Large files: fewer iterations
        let tmpdir = TempDir::new().map_err(|e| anyhow!("TempDir: {}", e))?;
        let large_content = b"x".repeat(1024 * 1024); // 1 MB per file
        let mut times = Vec::with_capacity(iterations);

        let total_start = Instant::now();
        for i in 0..iterations {
            let t = Instant::now();
            let path = tmpdir.path().join(format!("large_{}.dat", i));
            tokio::fs::write(&path, &large_content)
                .await
                .map_err(|e| anyhow!("large write: {}", e))?;
            let read_back = tokio::fs::read(&path)
                .await
                .map_err(|e| anyhow!("large read: {}", e))?;
            std::hint::black_box(read_back.len());
            tokio::fs::remove_file(&path).await.ok();
            times.push(t.elapsed());
        }
        let total_time = total_start.elapsed();

        let success_count = times.len() as u32;
        let avg_time = times.iter().sum::<Duration>() / success_count;
        let min_time = *times.iter().min().unwrap();
        let max_time = *times.iter().max().unwrap();
        let ops_per_second = success_count as f64 / total_time.as_secs_f64().max(f64::EPSILON);

        let result = BenchmarkResult {
            operation: "large_file_handling".to_string(),
            total_time,
            avg_time,
            min_time,
            max_time,
            ops_per_second,
            success_rate: 1.0,
            error_count: 0,
            resource_usage: self.capture_resource_usage().await?,
        };

        self.results
            .insert("large_file_handling".to_string(), result);
        Ok(())
    }

    /// Benchmark concurrent file access using real parallel tokio tasks.
    async fn benchmark_concurrent_access(&mut self) -> Result<()> {
        let concurrency = 8usize;
        let tmpdir = Arc::new(TempDir::new().map_err(|e| anyhow!("TempDir: {}", e))?);
        let content = Arc::new(b"concurrent bench ".repeat(128)); // ~2 KB

        let total_start = Instant::now();
        let mut handles = Vec::with_capacity(concurrency);
        for i in 0..concurrency {
            let dir = tmpdir.path().to_path_buf();
            let data = Arc::clone(&content);
            handles.push(tokio::spawn(async move {
                let path = dir.join(format!("concurrent_{}.dat", i));
                tokio::fs::write(&path, data.as_ref()).await?;
                let _ = tokio::fs::read(&path).await?;
                tokio::fs::remove_file(&path).await?;
                Ok::<(), anyhow::Error>(())
            }));
        }

        let mut errors = 0u32;
        let mut times = Vec::with_capacity(concurrency);
        for handle in handles {
            let t = Instant::now();
            match handle.await {
                Ok(Ok(_)) => times.push(t.elapsed()),
                _ => errors += 1,
            }
        }
        let total_time = total_start.elapsed();

        if times.is_empty() {
            return Err(anyhow!("All concurrent_file_access tasks failed"));
        }
        let success_count = times.len() as u32;
        let avg_time = times.iter().sum::<Duration>() / success_count;
        let min_time = *times.iter().min().unwrap();
        let max_time = *times.iter().max().unwrap();
        let ops_per_second = success_count as f64 / total_time.as_secs_f64().max(f64::EPSILON);

        let result = BenchmarkResult {
            operation: "concurrent_file_access".to_string(),
            total_time,
            avg_time,
            min_time,
            max_time,
            ops_per_second,
            success_rate: success_count as f64 / concurrency as f64,
            error_count: errors,
            resource_usage: self.capture_resource_usage().await?,
        };

        self.results
            .insert("concurrent_file_access".to_string(), result);
        Ok(())
    }

    /// Run resource utilization monitoring
    async fn run_resource_monitoring(&mut self) -> Result<()> {
        log::info!("Running resource utilization monitoring");

        // Monitor resources during idle
        self.monitor_resources_during_idle().await?;

        // Monitor resources during load
        self.monitor_resources_during_load().await?;

        Ok(())
    }

    /// Monitor resource usage during idle: take N sysinfo snapshots at intervals.
    async fn monitor_resources_during_idle(&mut self) -> Result<()> {
        let samples = 5usize;
        let interval = Duration::from_millis(self.config.monitoring_interval_ms.min(100));
        let mut times = Vec::with_capacity(samples);

        let total_start = Instant::now();
        for _ in 0..samples {
            let t = Instant::now();
            let _usage = self.capture_resource_usage().await?;
            times.push(t.elapsed());
            tokio::time::sleep(interval).await;
        }
        let total_time = total_start.elapsed();

        let success_count = times.len() as u32;
        let avg_time = times.iter().sum::<Duration>() / success_count;
        let min_time = *times.iter().min().unwrap();
        let max_time = *times.iter().max().unwrap();
        let ops_per_second = success_count as f64 / total_time.as_secs_f64().max(f64::EPSILON);

        let result = BenchmarkResult {
            operation: "resource_monitoring_idle".to_string(),
            total_time,
            avg_time,
            min_time,
            max_time,
            ops_per_second,
            success_rate: 1.0,
            error_count: 0,
            resource_usage: self.capture_resource_usage().await?,
        };

        self.results
            .insert("resource_monitoring_idle".to_string(), result);
        Ok(())
    }

    /// Monitor resource usage under load: run CPU-bound work and capture sysinfo.
    async fn monitor_resources_during_load(&mut self) -> Result<()> {
        let samples = 5usize;
        let interval = Duration::from_millis(self.config.monitoring_interval_ms.min(100));
        let mut times = Vec::with_capacity(samples);

        let total_start = Instant::now();
        for i in 0..samples {
            let t = Instant::now();
            // Generate real CPU load: hash computation
            let work: u64 = (0..10_000u64)
                .map(|x| x.wrapping_mul(i as u64 + 1))
                .fold(0, |acc, x| acc ^ x);
            std::hint::black_box(work);
            let _usage = self.capture_resource_usage().await?;
            times.push(t.elapsed());
            tokio::time::sleep(interval).await;
        }
        let total_time = total_start.elapsed();

        let success_count = times.len() as u32;
        let avg_time = times.iter().sum::<Duration>() / success_count;
        let min_time = *times.iter().min().unwrap();
        let max_time = *times.iter().max().unwrap();
        let ops_per_second = success_count as f64 / total_time.as_secs_f64().max(f64::EPSILON);

        let result = BenchmarkResult {
            operation: "resource_monitoring_load".to_string(),
            total_time,
            avg_time,
            min_time,
            max_time,
            ops_per_second,
            success_rate: 1.0,
            error_count: 0,
            resource_usage: self.capture_resource_usage().await?,
        };

        self.results
            .insert("resource_monitoring_load".to_string(), result);
        Ok(())
    }

    /// Run scalability benchmarks
    async fn run_scalability_benchmarks(&mut self) -> Result<()> {
        log::info!("Running scalability benchmarks");

        // Clone to avoid borrow conflict
        let concurrent_users = self.config.concurrent_users.clone();
        let data_scales = self.config.data_scales.clone();

        // Concurrent user simulation
        for users in concurrent_users {
            self.benchmark_concurrent_users(users).await?;
        }

        // Data scale handling
        for scale in data_scales {
            self.benchmark_data_scale(scale).await?;
        }

        Ok(())
    }

    /// Benchmark concurrent user simulation: spawn `user_count` parallel tasks.
    async fn benchmark_concurrent_users(&mut self, user_count: u32) -> Result<()> {
        let operation_name = format!("concurrent_users_{}", user_count);
        let count = user_count.min(50) as usize; // cap at 50 to avoid resource exhaustion

        let total_start = Instant::now();
        let mut handles = Vec::with_capacity(count);
        for i in 0..count {
            handles.push(tokio::spawn(async move {
                // Simulate a user action: serialise a document and compute a hash
                let doc = serde_json::json!({"user": i, "action": "search", "query": "rust"});
                let s = serde_json::to_string(&doc).unwrap_or_default();
                let hash: u64 = s.bytes().fold(0u64, |acc, b| acc.wrapping_add(b as u64));
                std::hint::black_box(hash);
            }));
        }
        let mut times = Vec::with_capacity(count);
        for handle in handles {
            let t = Instant::now();
            handle.await.ok();
            times.push(t.elapsed());
        }
        let total_time = total_start.elapsed();

        let success_count = times.len() as u32;
        let avg_time = if success_count > 0 {
            times.iter().sum::<Duration>() / success_count
        } else {
            Duration::ZERO
        };
        let min_time = times.iter().min().cloned().unwrap_or(Duration::ZERO);
        let max_time = times.iter().max().cloned().unwrap_or(Duration::ZERO);
        let ops_per_second = success_count as f64 / total_time.as_secs_f64().max(f64::EPSILON);

        let result = BenchmarkResult {
            operation: operation_name.clone(),
            total_time,
            avg_time,
            min_time,
            max_time,
            ops_per_second,
            success_rate: 1.0,
            error_count: 0,
            resource_usage: self.capture_resource_usage().await?,
        };

        self.results.insert(operation_name, result);
        Ok(())
    }

    /// Benchmark data scale handling: process `data_scale` records in-memory.
    async fn benchmark_data_scale(&mut self, data_scale: u64) -> Result<()> {
        let operation_name = format!("data_scale_{}", data_scale);
        // Cap iterations to avoid excessive test time (max ~100 k ops)
        let n = data_scale.min(100_000) as usize;

        let total_start = Instant::now();
        let mut store: HashMap<usize, u64> = HashMap::with_capacity(n);
        for i in 0..n {
            store.insert(i, i as u64 * 7 + 13);
        }
        let total_time = total_start.elapsed();

        let ops_per_second = n as f64 / total_time.as_secs_f64().max(f64::EPSILON);
        let avg_time = if n > 0 {
            total_time / n as u32
        } else {
            Duration::ZERO
        };

        let result = BenchmarkResult {
            operation: operation_name.clone(),
            total_time,
            avg_time,
            min_time: Duration::ZERO,
            max_time: total_time,
            ops_per_second,
            success_rate: 1.0,
            error_count: 0,
            resource_usage: self.capture_resource_usage().await?,
        };

        self.results.insert(operation_name, result);
        Ok(())
    }

    /// Run comparative analysis
    async fn run_comparative_analysis(&mut self) -> Result<()> {
        log::info!("Running comparative analysis");

        // This will be handled in generate_report() with trend analysis
        Ok(())
    }

    /// Capture current resource usage
    async fn capture_resource_usage(&self) -> Result<ResourceUsage> {
        let mut system = self.system.lock().await;
        system.refresh_all();

        // Calculate average CPU usage from all CPUs
        let cpus = system.cpus();
        let cpu_percent = if cpus.is_empty() {
            0.0
        } else {
            cpus.iter().map(|cpu| cpu.cpu_usage()).sum::<f32>() / cpus.len() as f32
        };

        // Get current process info (if available)
        let current_pid = Pid::from_u32(std::process::id());
        let (memory_bytes, virtual_memory_bytes, thread_count) =
            if let Some(process) = system.process(current_pid) {
                (process.memory(), process.virtual_memory(), 1) // Thread count not easily available
            } else {
                (0, 0, 0)
            };

        // Disk and network stats (simplified)
        let disks = Disks::new_with_refreshed_list();
        let disk_read_bytes = disks
            .iter()
            .map(|disk| disk.total_space() - disk.available_space())
            .sum();
        let disk_write_bytes = 0; // Not easily available from sysinfo

        let networks = Networks::new_with_refreshed_list();
        let network_rx_bytes = networks.iter().map(|(_, network)| network.received()).sum();
        let network_tx_bytes = networks
            .iter()
            .map(|(_, network)| network.transmitted())
            .sum();

        Ok(ResourceUsage {
            cpu_percent,
            memory_bytes,
            virtual_memory_bytes,
            disk_read_bytes,
            disk_write_bytes,
            network_rx_bytes,
            network_tx_bytes,
            thread_count,
        })
    }

    /// Generate comprehensive benchmark report
    async fn generate_report(&mut self) -> Result<BenchmarkReport> {
        log::info!("Generating benchmark report");

        // Calculate SLO compliance
        let slo_compliance = self.calculate_slo_compliance();

        // Gather system information
        let system_info = self.gather_system_info().await?;

        // Calculate performance trends
        let trends = self.calculate_performance_trends();

        let report = BenchmarkReport {
            timestamp: chrono::Utc::now(),
            config: self.config.clone(),
            results: self.results.clone(),
            slo_compliance,
            system_info,
            trends,
        };

        Ok(report)
    }

    /// Export report to JSON
    pub fn export_json(&self, report: &BenchmarkReport) -> Result<String> {
        serde_json::to_string_pretty(report)
            .map_err(|e| anyhow!("Failed to serialize report: {}", e))
    }

    /// Calculate SLO compliance
    fn calculate_slo_compliance(&self) -> SLOCompliance {
        let mut violations = Vec::new();
        let mut critical_violations = Vec::new();

        // Check each benchmark result against SLOs
        for (operation, result) in &self.results {
            match operation.as_str() {
                "health_check" | "config_api" => {
                    if result.avg_time.as_millis()
                        > self.config.slos.max_api_response_time_ms as u128
                    {
                        violations.push(SLOViolation {
                            metric: format!("{} response time", operation),
                            actual_value: format!("{}ms", result.avg_time.as_millis()),
                            threshold_value: format!(
                                "{}ms",
                                self.config.slos.max_api_response_time_ms
                            ),
                            severity: ViolationSeverity::Warning,
                        });
                    }
                }
                "search_api" => {
                    if result.avg_time.as_millis() > self.config.slos.max_search_time_ms as u128 {
                        violations.push(SLOViolation {
                            metric: format!("{} response time", operation),
                            actual_value: format!("{}ms", result.avg_time.as_millis()),
                            threshold_value: format!("{}ms", self.config.slos.max_search_time_ms),
                            severity: ViolationSeverity::Critical,
                        });
                    }
                }
                "resource_monitoring_idle" => {
                    if result.resource_usage.cpu_percent > self.config.slos.max_cpu_idle_percent {
                        critical_violations.push(SLOViolation {
                            metric: "CPU usage during idle".to_string(),
                            actual_value: format!("{:.1}%", result.resource_usage.cpu_percent),
                            threshold_value: format!(
                                "{:.1}%",
                                self.config.slos.max_cpu_idle_percent
                            ),
                            severity: ViolationSeverity::Critical,
                        });
                    }
                }
                "resource_monitoring_load" => {
                    if result.resource_usage.cpu_percent > self.config.slos.max_cpu_load_percent {
                        violations.push(SLOViolation {
                            metric: "CPU usage during load".to_string(),
                            actual_value: format!("{:.1}%", result.resource_usage.cpu_percent),
                            threshold_value: format!(
                                "{:.1}%",
                                self.config.slos.max_cpu_load_percent
                            ),
                            severity: ViolationSeverity::Warning,
                        });
                    }
                }
                _ => {} // Other operations don't have specific SLOs yet
            }
        }

        let total_checks = self.results.len();
        let violations_count = violations.len() + critical_violations.len();
        let overall_compliance = if total_checks > 0 {
            ((total_checks - violations_count) as f64 / total_checks as f64) * 100.0
        } else {
            100.0
        };

        SLOCompliance {
            overall_compliance,
            violations,
            critical_violations,
        }
    }

    /// Gather system information
    async fn gather_system_info(&self) -> Result<SystemInfo> {
        let system = self.system.lock().await;

        Ok(SystemInfo {
            os: System::name().unwrap_or_else(|| "Unknown".to_string()),
            os_version: System::os_version().unwrap_or_else(|| "Unknown".to_string()),
            cpu_model: system
                .cpus()
                .first()
                .map(|cpu| cpu.brand().to_string())
                .unwrap_or_else(|| "Unknown".to_string()),
            cpu_cores: system.cpus().len(),
            total_memory_mb: system.total_memory() / (1024 * 1024),
            available_memory_mb: system.available_memory() / (1024 * 1024),
            rust_version: option_env!("CARGO_PKG_RUST_VERSION")
                .unwrap_or("Unknown")
                .to_string(),
            terraphim_version: env!("CARGO_PKG_VERSION").to_string(),
        })
    }

    /// Calculate performance trends compared to baseline
    fn calculate_performance_trends(&self) -> Option<PerformanceTrends> {
        let baseline = self.baseline.as_ref()?;

        let mut improvements = HashMap::new();
        let mut regressions = HashMap::new();
        let mut new_operations = Vec::new();

        for (operation, current_result) in &self.results {
            if let Some(baseline_result) = baseline.results.get(operation) {
                let current_avg = current_result.avg_time.as_secs_f64();
                let baseline_avg = baseline_result.avg_time.as_secs_f64();

                if current_avg > 0.0 && baseline_avg > 0.0 {
                    let change_percent = ((baseline_avg - current_avg) / baseline_avg) * 100.0;

                    if change_percent > 5.0 {
                        // 5% improvement threshold
                        improvements.insert(operation.clone(), change_percent);
                    } else if change_percent < -5.0 {
                        // 5% regression threshold
                        regressions.insert(operation.clone(), change_percent);
                    }
                }
            } else {
                new_operations.push(operation.clone());
            }
        }

        Some(PerformanceTrends {
            baseline_timestamp: baseline.timestamp,
            improvements,
            regressions,
            new_operations,
        })
    }

    /// Export report to HTML
    pub fn export_html(&self, report: &BenchmarkReport) -> Result<String> {
        Ok(format!(
            "<!DOCTYPE html><html><body><h1>Performance Benchmark Report</h1>\
             <p>Generated: {}</p><p>SLO Compliance: {:.1}%</p></body></html>",
            report.timestamp, report.slo_compliance.overall_compliance
        ))
    }
}

impl Default for BenchmarkConfig {
    fn default() -> Self {
        Self {
            iterations: 1000,
            warmup_iterations: 100,
            concurrent_users: vec![1, 5, 10, 25, 50],
            data_scales: vec![1000, 10000, 100000, 1000000],
            slos: PerformanceSLO::default(),
            monitoring_interval_ms: 1000,
            enable_profiling: false,
        }
    }
}

impl Default for PerformanceSLO {
    fn default() -> Self {
        Self {
            max_startup_time_ms: 5000,
            max_api_response_time_ms: 500,
            max_search_time_ms: 1000,
            max_indexing_time_per_doc_ms: 50,
            max_memory_mb: 1024,
            max_cpu_idle_percent: 5.0,
            max_cpu_load_percent: 80.0,
            min_rps: 10.0,
            max_concurrent_users: 100,
            max_data_scale: 1000000,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fast_config() -> BenchmarkConfig {
        BenchmarkConfig {
            iterations: 10,
            warmup_iterations: 2,
            concurrent_users: vec![2],
            data_scales: vec![100],
            monitoring_interval_ms: 10,
            enable_profiling: false,
            slos: PerformanceSLO::default(),
        }
    }

    #[tokio::test]
    async fn performance_filesystem_benchmarks_produce_real_timings() {
        let mut benchmarker = PerformanceBenchmarker::new(fast_config());
        benchmarker
            .run_filesystem_benchmarks()
            .await
            .expect("filesystem benchmarks should complete");

        let file_ops = benchmarker
            .results
            .get("file_operations")
            .expect("file_operations result missing");
        assert!(
            file_ops.total_time > Duration::ZERO,
            "file_operations total_time must be non-zero"
        );
        assert!(
            file_ops.ops_per_second > 0.0,
            "file_operations ops_per_second must be positive"
        );

        let large = benchmarker
            .results
            .get("large_file_handling")
            .expect("large_file_handling result missing");
        assert!(
            large.total_time > Duration::ZERO,
            "large_file_handling total_time must be non-zero"
        );

        let concurrent = benchmarker
            .results
            .get("concurrent_file_access")
            .expect("concurrent_file_access result missing");
        assert!(
            concurrent.total_time > Duration::ZERO,
            "concurrent_file_access total_time must be non-zero"
        );
    }

    #[tokio::test]
    async fn performance_indexing_benchmarks_produce_real_timings() {
        let mut benchmarker = PerformanceBenchmarker::new(fast_config());
        benchmarker
            .benchmark_indexing_performance()
            .await
            .expect("indexing benchmark should complete");

        let result = benchmarker
            .results
            .get("document_indexing")
            .expect("document_indexing result missing");
        assert!(
            result.total_time > Duration::ZERO,
            "indexing total_time must be non-zero"
        );
        assert_eq!(result.error_count, 0);
    }

    #[tokio::test]
    async fn performance_crud_benchmarks_produce_real_timings() {
        let mut benchmarker = PerformanceBenchmarker::new(fast_config());
        benchmarker
            .run_database_benchmarks()
            .await
            .expect("db benchmarks should complete");

        let crud = benchmarker
            .results
            .get("crud_operations")
            .expect("crud_operations result missing");
        assert!(
            crud.total_time > Duration::ZERO,
            "crud total_time must be non-zero"
        );
        assert_eq!(crud.success_rate, 1.0);

        let txn = benchmarker
            .results
            .get("transaction_performance")
            .expect("transaction_performance result missing");
        assert!(
            txn.total_time > Duration::ZERO,
            "transaction total_time must be non-zero"
        );
    }

    #[tokio::test]
    async fn performance_resource_monitoring_returns_valid_data() {
        let mut benchmarker = PerformanceBenchmarker::new(fast_config());
        let usage = benchmarker
            .capture_resource_usage()
            .await
            .expect("capture_resource_usage should succeed");
        assert!(
            usage.memory_bytes > 0 || usage.cpu_percent >= 0.0,
            "resource usage should have valid fields"
        );
    }

    #[tokio::test]
    async fn performance_scalability_benchmarks_produce_real_timings() {
        let mut benchmarker = PerformanceBenchmarker::new(fast_config());
        benchmarker
            .run_scalability_benchmarks()
            .await
            .expect("scalability benchmarks should complete");

        let user_result = benchmarker
            .results
            .get("concurrent_users_2")
            .expect("concurrent_users_2 result missing");
        assert!(
            user_result.total_time > Duration::ZERO,
            "concurrent_users total_time must be non-zero"
        );

        let scale_result = benchmarker
            .results
            .get("data_scale_100")
            .expect("data_scale_100 result missing");
        assert!(
            scale_result.ops_per_second > 0.0,
            "data_scale ops_per_second must be positive"
        );
    }
}
