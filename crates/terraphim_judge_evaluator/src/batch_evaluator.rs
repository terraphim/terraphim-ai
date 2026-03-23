//! Batch Evaluator - Parallel batch evaluation via ExecutionCoordinator
//!
//! Provides concurrent evaluation of multiple files with configurable
//! concurrency limits using tokio::sync::Semaphore.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use serde::{Deserialize, Serialize};
use tokio::sync::Semaphore;

use crate::judge_agent::{JudgeAgent, JudgeVerdict};

/// Result of a single file evaluation within a batch
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchResult {
    /// Path to the evaluated file
    pub file: PathBuf,
    /// The verdict if evaluation succeeded
    pub verdict: Option<JudgeVerdict>,
    /// Error message if evaluation failed
    pub error: Option<String>,
    /// Evaluation duration in milliseconds
    pub duration_ms: u64,
}

impl BatchResult {
    /// Create a new successful batch result
    pub fn success(file: PathBuf, verdict: JudgeVerdict, duration_ms: u64) -> Self {
        Self {
            file,
            verdict: Some(verdict),
            error: None,
            duration_ms,
        }
    }

    /// Create a new failed batch result
    pub fn error(file: PathBuf, error: String, duration_ms: u64) -> Self {
        Self {
            file,
            verdict: None,
            error: Some(error),
            duration_ms,
        }
    }

    /// Check if this result represents a successful evaluation
    pub fn is_success(&self) -> bool {
        self.verdict.is_some() && self.error.is_none()
    }

    /// Check if this result represents a failed evaluation
    pub fn is_error(&self) -> bool {
        self.error.is_some()
    }
}

/// Summary statistics for a batch evaluation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchSummary {
    /// Total number of files evaluated
    pub total: usize,
    /// Number of files that passed
    pub passed: usize,
    /// Number of files that failed
    pub failed: usize,
    /// Number of files with evaluation errors
    pub errors: usize,
    /// Average latency in milliseconds
    pub avg_latency_ms: u64,
    /// Total duration of the batch in milliseconds
    pub total_duration_ms: u64,
}

impl BatchSummary {
    /// Create a summary from a collection of batch results
    pub fn from_results(results: &[BatchResult], total_duration_ms: u64) -> Self {
        let total = results.len();
        let passed = results
            .iter()
            .filter(|r| r.verdict.as_ref().map(|v| v.is_pass()).unwrap_or(false))
            .count();
        let failed = results
            .iter()
            .filter(|r| r.verdict.as_ref().map(|v| v.is_fail()).unwrap_or(false))
            .count();
        let errors = results.iter().filter(|r| r.is_error()).count();

        let avg_latency_ms = if total > 0 {
            results.iter().map(|r| r.duration_ms).sum::<u64>() / total as u64
        } else {
            0
        };

        Self {
            total,
            passed,
            failed,
            errors,
            avg_latency_ms,
            total_duration_ms,
        }
    }
}

/// Batch evaluator for parallel evaluation of multiple files
///
/// Uses a semaphore to limit concurrent evaluations and collects
/// results as they complete.
pub struct BatchEvaluator {
    /// The judge agent used for evaluations (wrapped in Arc for sharing across tasks)
    judge: Arc<JudgeAgent>,
    /// Maximum number of concurrent evaluations
    max_concurrency: usize,
}

impl BatchEvaluator {
    /// Create a new batch evaluator
    ///
    /// # Arguments
    /// * `judge` - The JudgeAgent to use for evaluations
    /// * `max_concurrency` - Maximum number of concurrent evaluations
    ///
    /// # Example
    /// ```
    /// use terraphim_judge_evaluator::{JudgeAgent, BatchEvaluator};
    ///
    /// let judge = JudgeAgent::new();
    /// let evaluator = BatchEvaluator::new(judge, 4);
    /// ```
    pub fn new(judge: JudgeAgent, max_concurrency: usize) -> Self {
        Self {
            judge: Arc::new(judge),
            max_concurrency,
        }
    }

    /// Evaluate a batch of files
    ///
    /// Evaluates all files in parallel with the configured concurrency limit.
    /// Results are collected as evaluations complete (not in input order).
    ///
    /// # Arguments
    /// * `files` - Vector of file paths to evaluate
    /// * `profile` - The evaluation profile to use
    ///
    /// # Returns
    /// Vector of BatchResult, one per input file
    ///
    /// # Example
    /// ```rust,no_run
    /// use terraphim_judge_evaluator::{JudgeAgent, BatchEvaluator};
    /// use std::path::PathBuf;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let judge = JudgeAgent::new();
    /// let evaluator = BatchEvaluator::new(judge, 4);
    /// let files = vec![PathBuf::from("file1.rs"), PathBuf::from("file2.rs")];
    /// let results = evaluator.evaluate_batch(files, "default").await;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn evaluate_batch(&self, files: Vec<PathBuf>, profile: &str) -> Vec<BatchResult> {
        let start_time = Instant::now();
        let semaphore = Arc::new(Semaphore::new(self.max_concurrency));
        let mut handles = Vec::with_capacity(files.len());

        // Spawn evaluation tasks for all files
        for file in files {
            let permit = semaphore.clone().acquire_owned().await;
            let judge = Arc::clone(&self.judge);
            let profile = profile.to_string();

            let handle = tokio::spawn(async move {
                let task_start = Instant::now();

                // Wait for semaphore permit (concurrency limit)
                let _permit = permit;

                // Evaluate the file
                let result = judge.evaluate(&file, &profile).await;

                let duration_ms = task_start.elapsed().as_millis() as u64;

                match result {
                    Ok(verdict) => BatchResult::success(file, verdict, duration_ms),
                    Err(e) => BatchResult::error(file, e.to_string(), duration_ms),
                }
            });

            handles.push(handle);
        }

        // Collect results as they complete
        let mut results = Vec::with_capacity(handles.len());
        for handle in handles {
            match handle.await {
                Ok(result) => results.push(result),
                Err(e) => {
                    // Task panicked - create an error result
                    results.push(BatchResult::error(
                        PathBuf::from("unknown"),
                        format!("Task panicked: {}", e),
                        0,
                    ));
                }
            }
        }

        log::info!(
            "Batch evaluation completed: {} files in {}ms",
            results.len(),
            start_time.elapsed().as_millis()
        );

        results
    }

    /// Evaluate a batch and return results with summary statistics
    ///
    /// Similar to `evaluate_batch` but also computes summary statistics.
    ///
    /// # Arguments
    /// * `files` - Vector of file paths to evaluate
    /// * `profile` - The evaluation profile to use
    ///
    /// # Returns
    /// Tuple of (results vector, summary statistics)
    pub async fn evaluate_batch_with_summary(
        &self,
        files: Vec<PathBuf>,
        profile: &str,
    ) -> (Vec<BatchResult>, BatchSummary) {
        let start_time = Instant::now();
        let results = self.evaluate_batch(files, profile).await;
        let total_duration_ms = start_time.elapsed().as_millis() as u64;

        let summary = BatchSummary::from_results(&results, total_duration_ms);

        (results, summary)
    }

    /// Get the maximum concurrency level
    pub fn max_concurrency(&self) -> usize {
        self.max_concurrency
    }

    /// Get a reference to the judge agent
    pub fn judge(&self) -> &JudgeAgent {
        &self.judge
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    fn create_test_file(dir: &TempDir, name: &str, content: &str) -> PathBuf {
        let path = dir.path().join(name);
        let mut file = std::fs::File::create(&path).unwrap();
        file.write_all(content.as_bytes()).unwrap();
        path
    }

    #[tokio::test]
    async fn test_batch_evaluator_new() {
        let judge = JudgeAgent::new();
        let evaluator = BatchEvaluator::new(judge, 4);

        assert_eq!(evaluator.max_concurrency(), 4);
    }

    #[tokio::test]
    async fn test_batch_evaluate_batch_of_three() {
        let temp_dir = TempDir::new().unwrap();

        // Create test files
        let file1 = create_test_file(&temp_dir, "file1.rs", "fn main() {}");
        let file2 = create_test_file(&temp_dir, "file2.rs", "fn test() {}");
        let file3 = create_test_file(&temp_dir, "file3.rs", "fn helper() {}");

        let judge = JudgeAgent::new();
        let evaluator = BatchEvaluator::new(judge, 4);

        let files = vec![file1.clone(), file2.clone(), file3.clone()];
        let results = evaluator.evaluate_batch(files, "default").await;

        assert_eq!(results.len(), 3);

        // Check all files were evaluated
        let result_files: Vec<_> = results.iter().map(|r| &r.file).collect();
        assert!(result_files.contains(&&file1));
        assert!(result_files.contains(&&file2));
        assert!(result_files.contains(&&file3));

        // All should succeed with the mock implementation
        for result in &results {
            assert!(result.is_success(), "File {:?} should succeed", result.file);
            assert!(result.verdict.is_some());
            assert!(result.error.is_none());
        }
    }

    #[tokio::test]
    async fn test_concurrency_limit_respected() {
        let temp_dir = TempDir::new().unwrap();

        // Create 5 test files
        let mut files = Vec::new();
        for i in 0..5 {
            files.push(create_test_file(
                &temp_dir,
                &format!("file{}.rs", i),
                "fn main() {}",
            ));
        }

        let judge = JudgeAgent::new();
        let max_concurrency = 2;
        let evaluator = BatchEvaluator::new(judge, max_concurrency);

        assert_eq!(evaluator.max_concurrency(), max_concurrency);

        let results = evaluator.evaluate_batch(files, "default").await;
        assert_eq!(results.len(), 5);

        // All should succeed
        for result in &results {
            assert!(result.is_success());
        }
    }

    #[tokio::test]
    async fn test_error_handling_for_bad_files() {
        let judge = JudgeAgent::new();
        let evaluator = BatchEvaluator::new(judge, 4);

        // Include a non-existent file
        let files = vec![PathBuf::from("/nonexistent/path/file.rs")];

        let results = evaluator.evaluate_batch(files, "default").await;

        assert_eq!(results.len(), 1);
        assert!(results[0].is_error());
        assert!(results[0].verdict.is_none());
        assert!(results[0].error.is_some());
        assert!(results[0].error.as_ref().unwrap().contains("No such file"));
    }

    #[tokio::test]
    async fn test_mixed_success_and_error() {
        let temp_dir = TempDir::new().unwrap();
        let good_file = create_test_file(&temp_dir, "good.rs", "fn main() {}");

        let judge = JudgeAgent::new();
        let evaluator = BatchEvaluator::new(judge, 4);

        let files = vec![good_file.clone(), PathBuf::from("/nonexistent/bad.rs")];

        let results = evaluator.evaluate_batch(files, "default").await;

        assert_eq!(results.len(), 2);

        // Find the good file result
        let good_result = results.iter().find(|r| r.file == good_file).unwrap();
        assert!(good_result.is_success());

        // Find the bad file result
        let bad_result = results
            .iter()
            .find(|r| r.file == PathBuf::from("/nonexistent/bad.rs"))
            .unwrap();
        assert!(bad_result.is_error());
    }

    #[tokio::test]
    async fn test_batch_summary_calculation() {
        let results = vec![
            BatchResult::success(
                PathBuf::from("pass.rs"),
                JudgeVerdict::new(
                    "PASS".to_string(),
                    std::collections::BTreeMap::new(),
                    "quick".to_string(),
                    "".to_string(),
                    100,
                ),
                100,
            ),
            BatchResult::success(
                PathBuf::from("fail.rs"),
                JudgeVerdict::new(
                    "FAIL".to_string(),
                    std::collections::BTreeMap::new(),
                    "quick".to_string(),
                    "".to_string(),
                    150,
                ),
                150,
            ),
            BatchResult::error(PathBuf::from("error.rs"), "IO error".to_string(), 50),
        ];

        let summary = BatchSummary::from_results(&results, 500);

        assert_eq!(summary.total, 3);
        assert_eq!(summary.passed, 1);
        assert_eq!(summary.failed, 1);
        assert_eq!(summary.errors, 1);
        assert_eq!(summary.avg_latency_ms, 100); // (100 + 150 + 50) / 3
        assert_eq!(summary.total_duration_ms, 500);
    }

    #[tokio::test]
    async fn test_evaluate_batch_with_summary() {
        let temp_dir = TempDir::new().unwrap();
        let file1 = create_test_file(&temp_dir, "file1.rs", "fn main() {}");
        let file2 = create_test_file(&temp_dir, "file2.rs", "fn test() {}");

        let judge = JudgeAgent::new();
        let evaluator = BatchEvaluator::new(judge, 4);

        let files = vec![file1, file2];
        let (results, summary) = evaluator
            .evaluate_batch_with_summary(files, "default")
            .await;

        assert_eq!(results.len(), 2);
        assert_eq!(summary.total, 2);
        assert_eq!(summary.passed, 2); // Mock returns PASS for default profile
        assert_eq!(summary.failed, 0);
        assert_eq!(summary.errors, 0);
        // avg_latency_ms may be 0 due to fast mock execution
    }

    #[tokio::test]
    async fn test_batch_result_helpers() {
        let verdict = JudgeVerdict::new(
            "PASS".to_string(),
            std::collections::BTreeMap::new(),
            "quick".to_string(),
            "".to_string(),
            100,
        );

        let success_result = BatchResult::success(PathBuf::from("test.rs"), verdict.clone(), 100);
        assert!(success_result.is_success());
        assert!(!success_result.is_error());

        let error_result = BatchResult::error(PathBuf::from("test.rs"), "error".to_string(), 50);
        assert!(!error_result.is_success());
        assert!(error_result.is_error());
    }

    #[tokio::test]
    async fn test_empty_batch() {
        let judge = JudgeAgent::new();
        let evaluator = BatchEvaluator::new(judge, 4);

        let results = evaluator.evaluate_batch(vec![], "default").await;

        assert!(results.is_empty());

        let summary = BatchSummary::from_results(&results, 0);
        assert_eq!(summary.total, 0);
        assert_eq!(summary.avg_latency_ms, 0);
    }
}
