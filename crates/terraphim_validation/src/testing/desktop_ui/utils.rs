//! Desktop UI Testing Utilities
//!
//! Utility functions and helpers for desktop UI testing including
//! screenshot comparison, element waiting, and test data management.

use crate::testing::{Result, ValidationResult, ValidationStatus};
use image::{DynamicImage, GenericImageView, ImageBuffer, Rgba};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

/// Screenshot comparison utilities
pub struct ScreenshotUtils;

impl ScreenshotUtils {
    /// Compare two screenshots and return difference metrics
    pub fn compare_screenshots(
        baseline_path: &Path,
        current_path: &Path,
        diff_path: Option<&Path>,
    ) -> Result<ScreenshotComparison> {
        let baseline = image::open(baseline_path)?;
        let current = image::open(current_path)?;

        if baseline.dimensions() != current.dimensions() {
            return Err(anyhow::anyhow!("Screenshot dimensions don't match"));
        }

        let (width, height) = baseline.dimensions();
        let mut diff_pixels = 0;
        let mut max_diff: f64 = 0.0;

        let mut diff_image = ImageBuffer::new(width, height);
        for y in 0..height {
            for x in 0..width {
                let baseline_pixel = baseline.get_pixel(x, y);
                let current_pixel = current.get_pixel(x, y);

                let diff = Self::pixel_difference(baseline_pixel, current_pixel);
                max_diff = max_diff.max(diff);

                if diff > 0.01 {
                    // Threshold for considering pixels different
                    diff_pixels += 1;
                    // Create red difference highlighting
                    diff_image.put_pixel(x, y, Rgba([255, 0, 0, 255]));
                } else {
                    diff_image.put_pixel(x, y, current_pixel);
                }
            }
        }

        let total_pixels = (width * height) as usize;
        let diff_percentage = (diff_pixels as f64 / total_pixels as f64) * 100.0;

        // Save diff image if path provided
        if let Some(diff_path) = diff_path {
            diff_image.save(diff_path)?;
        }

        Ok(ScreenshotComparison {
            total_pixels,
            different_pixels: diff_pixels,
            difference_percentage: diff_percentage,
            max_difference: max_diff,
            matches: diff_percentage < 0.1, // 0.1% threshold
        })
    }

    /// Calculate the difference between two pixels (0.0 to 1.0)
    fn pixel_difference(pixel1: Rgba<u8>, pixel2: Rgba<u8>) -> f64 {
        let r_diff = (pixel1[0] as f64 - pixel2[0] as f64).abs() / 255.0;
        let g_diff = (pixel1[1] as f64 - pixel2[1] as f64).abs() / 255.0;
        let b_diff = (pixel1[2] as f64 - pixel2[2] as f64).abs() / 255.0;
        let a_diff = (pixel1[3] as f64 - pixel2[3] as f64).abs() / 255.0;

        // Weighted difference (luminance approximation)
        (0.299 * r_diff + 0.587 * g_diff + 0.114 * b_diff + 0.5 * a_diff) / 2.0
    }

    /// Take a screenshot with timestamp
    pub fn take_timestamped_screenshot(name: &str, output_dir: &Path) -> Result<PathBuf> {
        use chrono::Utc;

        let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
        let filename = format!("{}_{}.png", name, timestamp);
        let path = output_dir.join(filename);

        // In real implementation, this would capture screenshot
        // For now, just return the path
        Ok(path)
    }
}

/// Screenshot comparison results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScreenshotComparison {
    pub total_pixels: usize,
    pub different_pixels: usize,
    pub difference_percentage: f64,
    pub max_difference: f64,
    pub matches: bool,
}

/// Element waiting utilities
pub struct ElementUtils;

impl ElementUtils {
    /// Wait for element to be visible with timeout
    pub async fn wait_for_element_visible(selector: &str, timeout: Duration) -> Result<bool> {
        // Implementation would use Playwright to wait for element
        tokio::time::sleep(Duration::from_millis(100)).await;
        Ok(true)
    }

    /// Wait for element to contain specific text
    pub async fn wait_for_text(
        selector: &str,
        expected_text: &str,
        timeout: Duration,
    ) -> Result<bool> {
        // Implementation would wait for text to appear
        tokio::time::sleep(Duration::from_millis(100)).await;
        Ok(true)
    }

    /// Wait for element to be clickable
    pub async fn wait_for_clickable(selector: &str, timeout: Duration) -> Result<bool> {
        // Implementation would wait for element to be clickable
        tokio::time::sleep(Duration::from_millis(100)).await;
        Ok(true)
    }
}

/// Test data management utilities
pub struct TestDataUtils;

impl TestDataUtils {
    /// Load test data from JSON file
    pub fn load_test_data<T: for<'de> serde::Deserialize<'de>>(path: &Path) -> Result<T> {
        let content = fs::read_to_string(path)?;
        let data: T = serde_json::from_str(&content)?;
        Ok(data)
    }

    /// Save test data to JSON file
    pub fn save_test_data<T: serde::Serialize>(path: &Path, data: &T) -> Result<()> {
        let content = serde_json::to_string_pretty(data)?;
        fs::write(path, content)?;
        Ok(())
    }

    /// Generate test data for UI testing
    pub fn generate_test_search_queries() -> Vec<String> {
        vec![
            "machine learning".to_string(),
            "artificial intelligence".to_string(),
            "neural networks".to_string(),
            "deep learning".to_string(),
            "computer vision".to_string(),
            "natural language processing".to_string(),
        ]
    }

    /// Generate test configuration data
    pub fn generate_test_config() -> HashMap<String, serde_json::Value> {
        let mut config = HashMap::new();
        config.insert("theme".to_string(), serde_json::json!("dark"));
        config.insert("language".to_string(), serde_json::json!("en"));
        config.insert("auto_save".to_string(), serde_json::json!(true));
        config.insert("max_results".to_string(), serde_json::json!(50));
        config
    }
}

/// Platform detection utilities
pub struct PlatformUtils;

impl PlatformUtils {
    /// Detect current platform
    pub fn detect_platform() -> Platform {
        match std::env::consts::OS {
            "macos" => Platform::MacOS,
            "windows" => Platform::Windows,
            "linux" => Platform::Linux,
            _ => Platform::Unknown,
        }
    }

    /// Check if running on CI
    pub fn is_ci() -> bool {
        std::env::var("CI").is_ok()
            || std::env::var("CONTINUOUS_INTEGRATION").is_ok()
            || std::env::var("BUILD_NUMBER").is_ok()
    }

    /// Get platform-specific paths
    pub fn get_platform_paths() -> PlatformPaths {
        match Self::detect_platform() {
            Platform::MacOS => PlatformPaths {
                app_data: dirs::data_dir()
                    .unwrap_or_else(|| PathBuf::from("~/Library/Application Support")),
                temp: std::env::temp_dir(),
                screenshots: PathBuf::from("~/Desktop"),
            },
            Platform::Windows => PlatformPaths {
                app_data: dirs::data_dir().unwrap_or_else(|| PathBuf::from("%APPDATA%")),
                temp: std::env::temp_dir(),
                screenshots: PathBuf::from("%USERPROFILE%/Pictures/Screenshots"),
            },
            Platform::Linux => PlatformPaths {
                app_data: dirs::data_dir().unwrap_or_else(|| PathBuf::from("~/.local/share")),
                temp: std::env::temp_dir(),
                screenshots: PathBuf::from("~/Pictures"),
            },
            Platform::Unknown => PlatformPaths {
                app_data: std::env::temp_dir(),
                temp: std::env::temp_dir(),
                screenshots: std::env::temp_dir(),
            },
        }
    }
}

#[derive(Debug, Clone)]
pub enum Platform {
    MacOS,
    Windows,
    Linux,
    Unknown,
}

#[derive(Debug, Clone)]
pub struct PlatformPaths {
    pub app_data: PathBuf,
    pub temp: PathBuf,
    pub screenshots: PathBuf,
}

/// Test result aggregation utilities
pub struct ResultUtils;

impl ResultUtils {
    /// Aggregate multiple validation results
    pub fn aggregate_results(results: Vec<ValidationResult>) -> AggregatedResults {
        let total = results.len();
        let passed = results
            .iter()
            .filter(|r| matches!(r.status, ValidationStatus::Passed))
            .count();
        let failed = results
            .iter()
            .filter(|r| matches!(r.status, ValidationStatus::Failed))
            .count();
        let skipped = results
            .iter()
            .filter(|r| matches!(r.status, ValidationStatus::Skipped))
            .count();

        AggregatedResults {
            total,
            passed,
            failed,
            skipped,
            success_rate: if total > 0 {
                (passed as f64 / total as f64) * 100.0
            } else {
                0.0
            },
        }
    }

    /// Generate summary report
    pub fn generate_summary(results: &[ValidationResult]) -> String {
        let aggregated = Self::aggregate_results(results.to_vec());

        format!(
            "Test Summary:\nTotal: {}\nPassed: {}\nFailed: {}\nSkipped: {}\nSuccess Rate: {:.1}%",
            aggregated.total,
            aggregated.passed,
            aggregated.failed,
            aggregated.skipped,
            aggregated.success_rate
        )
    }

    /// Aggregate multiple UI test results
    pub fn aggregate_ui_results(results: Vec<super::UITestResult>) -> AggregatedResults {
        let total = results.len();
        let passed = results
            .iter()
            .filter(|r| matches!(r.status, super::UITestStatus::Pass))
            .count();
        let failed = results
            .iter()
            .filter(|r| {
                matches!(
                    r.status,
                    super::UITestStatus::Fail | super::UITestStatus::Error
                )
            })
            .count();
        let skipped = results
            .iter()
            .filter(|r| matches!(r.status, super::UITestStatus::Skip))
            .count();

        AggregatedResults {
            total,
            passed,
            failed,
            skipped,
            success_rate: if total > 0 {
                (passed as f64 / total as f64) * 100.0
            } else {
                0.0
            },
        }
    }

    /// Generate summary report for UI test results
    pub fn generate_ui_summary(results: &[super::UITestResult]) -> String {
        let aggregated = Self::aggregate_ui_results(results.to_vec());

        format!(
            "Test Summary:\nTotal: {}\nPassed: {}\nFailed: {}\nSkipped: {}\nSuccess Rate: {:.1}%",
            aggregated.total,
            aggregated.passed,
            aggregated.failed,
            aggregated.skipped,
            aggregated.success_rate
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregatedResults {
    pub total: usize,
    pub passed: usize,
    pub failed: usize,
    pub skipped: usize,
    pub success_rate: f64,
}
