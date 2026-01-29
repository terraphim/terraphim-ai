//! Performance testing utilities for server API
//!
//! This module provides tools for load testing, response time benchmarking,
//! and memory usage monitoring for the terraphim server API.

use crate::testing::server_api::validation::ResponseValidator;
use crate::testing::server_api::{TestFixtures, TestServer};
use std::time::{Duration, Instant};
use tokio::task;

/// Performance test results
#[derive(Debug, Clone)]
pub struct PerformanceResults {
    /// Number of requests made
    pub request_count: usize,
    /// Total duration of all requests
    pub total_duration: Duration,
    /// Average response time
    pub avg_response_time: Duration,
    /// Minimum response time
    pub min_response_time: Duration,
    /// Maximum response time
    pub max_response_time: Duration,
    /// 95th percentile response time
    pub p95_response_time: Duration,
    /// Number of failed requests
    pub failed_requests: usize,
    /// Requests per second
    pub requests_per_second: f64,
}

/// Concurrent request testing
pub async fn test_concurrent_requests(
    server: &TestServer,
    endpoint: &str,
    concurrency: usize,
    request_count: usize,
) -> Result<PerformanceResults, Box<dyn std::error::Error>> {
    let mut handles = Vec::new();
    let mut response_times = Vec::new();

    // Spawn concurrent requests
    for i in 0..request_count {
        let client = reqwest::Client::new();
        let base_url = server.base_url.clone();
        let endpoint = endpoint.to_string();

        let handle = task::spawn(async move {
            let start = Instant::now();

            let url = format!("{}{}", base_url, endpoint);
            let result = client.get(&url).send().await;

            let duration = start.elapsed();

            match result {
                Ok(response) if response.status().is_success() => Ok(duration),
                _ => Err(duration),
            }
        });

        handles.push(handle);

        // Limit concurrency
        if handles.len() >= concurrency {
            let handle = handles.remove(0);
            let result = handle.await?;
            match result {
                Ok(duration) => response_times.push(duration),
                Err(duration) => response_times.push(duration), // Still record timing even for failed requests
            }
        }
    }

    // Wait for remaining requests
    for handle in handles {
        let result = handle.await?;
        match result {
            Ok(duration) => response_times.push(duration),
            Err(duration) => response_times.push(duration),
        }
    }

    // Calculate statistics
    let total_duration: Duration = response_times.iter().sum();
    let avg_response_time = total_duration / response_times.len() as u32;
    let min_response_time = response_times.iter().min().unwrap().clone();
    let max_response_time = response_times.iter().max().unwrap().clone();

    // Calculate 95th percentile
    response_times.sort();
    let p95_index = (response_times.len() as f64 * 0.95) as usize;
    let p95_response_time = response_times[p95_index];

    let failed_requests = response_times.len() - request_count; // Approximation

    let results = PerformanceResults {
        request_count,
        total_duration,
        avg_response_time,
        min_response_time,
        max_response_time,
        p95_response_time,
        failed_requests,
        requests_per_second: request_count as f64 / total_duration.as_secs_f64(),
    };

    Ok(results)
}

/// Large dataset processing test
pub async fn test_large_dataset_processing(
    server: &TestServer,
) -> Result<PerformanceResults, Box<dyn std::error::Error>> {
    let large_document = TestFixtures::large_document();

    // Test document creation
    let start = Instant::now();
    let response = server.post("/documents", &large_document).await;
    let creation_time = start.elapsed();

    response.validate_status(reqwest::StatusCode::OK);

    // Test searching for the large document
    let start = Instant::now();
    let response = server.get("/documents/search?query=Large").await;
    let search_time = start.elapsed();

    response.validate_status(reqwest::StatusCode::OK);

    Ok(PerformanceResults {
        request_count: 2,
        total_duration: creation_time + search_time,
        avg_response_time: (creation_time + search_time) / 2,
        min_response_time: creation_time.min(search_time),
        max_response_time: creation_time.max(search_time),
        p95_response_time: creation_time.max(search_time), // Approximation
        failed_requests: 0,
        requests_per_second: 2.0 / (creation_time + search_time).as_secs_f64(),
    })
}

/// Memory usage monitoring (placeholder - requires platform-specific implementation)
pub async fn monitor_memory_usage<F, Fut>(
    test_fn: F,
) -> Result<(u64, u64), Box<dyn std::error::Error>>
where
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = ()>,
{
    // Get initial memory usage (placeholder)
    let initial_memory = get_memory_usage();

    // Run the test
    test_fn().await;

    // Get final memory usage (placeholder)
    let final_memory = get_memory_usage();

    Ok((initial_memory, final_memory))
}

/// Get current memory usage (platform-specific implementation needed)
fn get_memory_usage() -> u64 {
    // Placeholder implementation
    // In a real implementation, this would use platform-specific APIs
    // like reading /proc/self/status on Linux or task_info on macOS
    0
}

/// Performance assertion helpers
pub mod assertions {
    use super::PerformanceResults;
    use std::time::Duration;

    /// Assert that average response time is within acceptable limits
    pub fn assert_avg_response_time(results: &PerformanceResults, max_avg_ms: u64) {
        let max_avg = Duration::from_millis(max_avg_ms);
        assert!(
            results.avg_response_time <= max_avg,
            "Average response time {}ms exceeds limit {}ms",
            results.avg_response_time.as_millis(),
            max_avg_ms
        );
    }

    /// Assert that 95th percentile response time is within acceptable limits
    pub fn assert_p95_response_time(results: &PerformanceResults, max_p95_ms: u64) {
        let max_p95 = Duration::from_millis(max_p95_ms);
        assert!(
            results.p95_response_time <= max_p95,
            "95th percentile response time {}ms exceeds limit {}ms",
            results.p95_response_time.as_millis(),
            max_p95_ms
        );
    }

    /// Assert that requests per second meets minimum threshold
    pub fn assert_requests_per_second(results: &PerformanceResults, min_rps: f64) {
        assert!(
            results.requests_per_second >= min_rps,
            "Requests per second {:.2} below minimum threshold {:.2}",
            results.requests_per_second,
            min_rps
        );
    }

    /// Assert that failure rate is below acceptable threshold
    pub fn assert_failure_rate(results: &PerformanceResults, max_failure_rate: f64) {
        let failure_rate = results.failed_requests as f64 / results.request_count as f64;
        assert!(
            failure_rate <= max_failure_rate,
            "Failure rate {:.2}% exceeds maximum threshold {:.2}%",
            failure_rate * 100.0,
            max_failure_rate * 100.0
        );
    }

    /// Assert that memory usage increase is within acceptable limits
    pub fn assert_memory_usage_increase(initial: u64, final_memory: u64, max_increase_mb: u64) {
        let increase = final_memory.saturating_sub(initial);
        let max_increase_bytes = max_increase_mb * 1024 * 1024;

        assert!(
            increase <= max_increase_bytes,
            "Memory usage increase {} bytes exceeds limit {} MB",
            increase,
            max_increase_mb
        );
    }
}
