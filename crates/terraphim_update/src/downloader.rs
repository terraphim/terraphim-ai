//! Download functionality with retry logic and exponential backoff
//!
//! This module provides robust download capabilities for update files,
//! including:
//! - Exponential backoff retry strategy
//! - Graceful network error handling
//! - Progress tracking
//! - Timeout handling

use anyhow::{anyhow, Result};
use std::io::{self, Write};
use std::time::{Duration, Instant};
use tracing::{debug, error, info, warn};

/// Default maximum number of retry attempts
pub const DEFAULT_MAX_RETRIES: u32 = 3;

/// Default initial delay between retries (in milliseconds)
pub const DEFAULT_INITIAL_DELAY_MS: u64 = 1000;

/// Default multiplier for exponential backoff
pub const DEFAULT_BACKOFF_MULTIPLIER: f64 = 2.0;

/// Maximum delay between retries (in milliseconds)
pub const MAX_DELAY_MS: u64 = 30000;

/// Configuration for download retry behavior
#[derive(Debug, Clone)]
pub struct DownloadConfig {
    /// Maximum number of retry attempts
    pub max_retries: u32,

    /// Initial delay between retries in milliseconds
    pub initial_delay_ms: u64,

    /// Multiplier for exponential backoff
    pub backoff_multiplier: f64,

    /// Request timeout
    pub timeout: Duration,

    /// Whether to show download progress
    pub show_progress: bool,
}

impl Default for DownloadConfig {
    fn default() -> Self {
        Self {
            max_retries: DEFAULT_MAX_RETRIES,
            initial_delay_ms: DEFAULT_INITIAL_DELAY_MS,
            backoff_multiplier: DEFAULT_BACKOFF_MULTIPLIER,
            timeout: Duration::from_secs(30),
            show_progress: false,
        }
    }
}

impl DownloadConfig {
    /// Create a new download config with default values
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the maximum number of retries
    pub fn with_max_retries(mut self, max_retries: u32) -> Self {
        self.max_retries = max_retries;
        self
    }

    /// Set the initial delay in milliseconds
    pub fn with_initial_delay_ms(mut self, delay_ms: u64) -> Self {
        self.initial_delay_ms = delay_ms;
        self
    }

    /// Set the backoff multiplier
    pub fn with_backoff_multiplier(mut self, multiplier: f64) -> Self {
        self.backoff_multiplier = multiplier;
        self
    }

    /// Set the timeout
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Enable or disable progress display
    pub fn with_progress(mut self, show: bool) -> Self {
        self.show_progress = show;
        self
    }
}

/// Result of a download operation
#[derive(Debug, Clone)]
pub struct DownloadResult {
    /// Whether the download was successful
    pub success: bool,

    /// Number of attempts made
    pub attempts: u32,

    /// Total duration of all attempts
    pub total_duration: Duration,

    /// Size of the downloaded file in bytes
    pub bytes_downloaded: u64,
}

/// Download a file with retry logic and exponential backoff
///
/// # Arguments
/// * `url` - URL to download from
/// * `output_path` - Path where to save the downloaded file
/// * `config` - Download configuration (optional, uses defaults if not provided)
///
/// # Returns
/// * `Ok(DownloadResult)` - Information about the download
/// * `Err(anyhow::Error)` - Error if download fails after all retries
///
/// # Example
/// ```no_run
/// use terraphim_update::downloader::download_with_retry;
/// use std::path::PathBuf;
///
/// let result = download_with_retry(
///     "https://example.com/binary.tar.gz",
///     &PathBuf::from("/tmp/binary.tar.gz"),
///     None
/// ).unwrap();
/// ```
pub fn download_with_retry(
    url: &str,
    output_path: &std::path::Path,
    config: Option<DownloadConfig>,
) -> Result<DownloadResult> {
    let config = config.unwrap_or_default();
    let start_time = Instant::now();
    let mut attempts = 0;
    let mut last_error: Option<String> = None;

    info!("Starting download from {}", url);

    for attempt in 1..=config.max_retries {
        attempts = attempt;

        let attempt_start = Instant::now();

        match perform_download(url, output_path, &config) {
            Ok(bytes) => {
                let duration = attempt_start.elapsed();

                info!(
                    "Download successful after {} attempt(s) ({:.2}s, {} bytes)",
                    attempt,
                    duration.as_secs_f64(),
                    bytes
                );

                return Ok(DownloadResult {
                    success: true,
                    attempts,
                    total_duration: start_time.elapsed(),
                    bytes_downloaded: bytes,
                });
            }
            Err(e) => {
                last_error = Some(e.to_string());
                let duration = attempt_start.elapsed();

                warn!(
                    "Download attempt {} failed after {:.2}s: {}",
                    attempt,
                    duration.as_secs_f64(),
                    e
                );

                if attempt < config.max_retries {
                    let delay = calculate_backoff_delay(attempt, &config);
                    info!("Waiting {:.2}s before retry...", delay.as_secs_f64());
                    std::thread::sleep(delay);
                }
            }
        }
    }

    let total_duration = start_time.elapsed();

    error!(
        "Download failed after {} attempt(s) ({:.2}s total)",
        attempts,
        total_duration.as_secs_f64()
    );

    Err(anyhow!(
        last_error.unwrap_or_else(|| "Download failed".to_string())
    ))
}

/// Perform a single download attempt
fn perform_download(
    url: &str,
    output_path: &std::path::Path,
    config: &DownloadConfig,
) -> Result<u64> {
    let response = ureq::get(url)
        .timeout(config.timeout)
        .call()
        .map_err(|e| anyhow!("HTTP request failed: {}", e))?;

    if response.status() != 200 {
        return Err(anyhow!(
            "HTTP error: {} {}",
            response.status(),
            response.status_text()
        ));
    }

    let content_length = response
        .header("Content-Length")
        .and_then(|h| h.parse::<u64>().ok());

    let mut reader = response.into_reader();

    let mut file = std::fs::File::create(output_path)?;
    let mut total_bytes = 0u64;
    let mut buffer = [0u8; 8192];

    loop {
        let bytes_read = reader.read(&mut buffer)?;

        if bytes_read == 0 {
            break;
        }

        file.write_all(&buffer[..bytes_read])?;
        total_bytes += bytes_read as u64;

        if config.show_progress {
            if let Some(total) = content_length {
                let percent = (total_bytes as f64 / total as f64) * 100.0;
                print!(
                    "\rProgress: {:.0}% ({}/{} bytes)",
                    percent, total_bytes, total
                );
                io::stdout().flush()?;
            } else {
                print!("\rDownloaded: {} bytes", total_bytes);
                io::stdout().flush()?;
            }
        }
    }

    if config.show_progress {
        println!();
    }

    debug!("Downloaded {} bytes to {:?}", total_bytes, output_path);

    Ok(total_bytes)
}

/// Calculate the delay for exponential backoff
fn calculate_backoff_delay(attempt: u32, config: &DownloadConfig) -> Duration {
    let delay_ms =
        (config.initial_delay_ms as f64) * config.backoff_multiplier.powi(attempt as i32 - 1);

    let delay_ms = delay_ms.min(MAX_DELAY_MS as f64) as u64;

    Duration::from_millis(delay_ms)
}

/// Download a file silently (no retries, fail fast)
///
/// # Arguments
/// * `url` - URL to download from
/// * `output_path` - Path where to save the downloaded file
///
/// # Returns
/// * `Ok(())` - Success
/// * `Err(anyhow::Error)` - Error if download fails
///
/// # Example
/// ```no_run
/// use terraphim_update::downloader::download_silent;
/// use std::path::PathBuf;
///
/// download_silent(
///     "https://example.com/binary.tar.gz",
///     &PathBuf::from("/tmp/binary.tar.gz")
/// ).unwrap();
/// ```
pub fn download_silent(url: &str, output_path: &std::path::Path) -> Result<()> {
    let config = DownloadConfig {
        max_retries: 1,
        show_progress: false,
        ..Default::default()
    };

    download_with_retry(url, output_path, Some(config))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::net::ToSocketAddrs;

    fn can_connect(host: &str, port: u16) -> bool {
        let addr = (host, port)
            .to_socket_addrs()
            .ok()
            .and_then(|mut addrs| addrs.next());
        let Some(addr) = addr else {
            return false;
        };
        std::net::TcpStream::connect_timeout(&addr, Duration::from_millis(200)).is_ok()
    }

    #[test]
    fn test_download_config_default() {
        let config = DownloadConfig::default();

        assert_eq!(config.max_retries, DEFAULT_MAX_RETRIES);
        assert_eq!(config.initial_delay_ms, DEFAULT_INITIAL_DELAY_MS);
        assert_eq!(config.backoff_multiplier, DEFAULT_BACKOFF_MULTIPLIER);
        assert_eq!(config.timeout, Duration::from_secs(30));
        assert!(!config.show_progress);
    }

    #[test]
    fn test_download_config_builder() {
        let config = DownloadConfig::new()
            .with_max_retries(5)
            .with_initial_delay_ms(2000)
            .with_backoff_multiplier(3.0)
            .with_timeout(Duration::from_secs(60))
            .with_progress(true);

        assert_eq!(config.max_retries, 5);
        assert_eq!(config.initial_delay_ms, 2000);
        assert_eq!(config.backoff_multiplier, 3.0);
        assert_eq!(config.timeout, Duration::from_secs(60));
        assert!(config.show_progress);
    }

    #[test]
    fn test_calculate_backoff_delay() {
        let config = DownloadConfig::default();

        let delay1 = calculate_backoff_delay(1, &config);
        assert_eq!(delay1, Duration::from_millis(1000));

        let delay2 = calculate_backoff_delay(2, &config);
        assert_eq!(delay2, Duration::from_millis(2000));

        let delay3 = calculate_backoff_delay(3, &config);
        assert_eq!(delay3, Duration::from_millis(4000));

        let delay4 = calculate_backoff_delay(4, &config);
        assert_eq!(delay4, Duration::from_millis(8000));
    }

    #[test]
    fn test_calculate_backoff_delay_with_custom_multiplier() {
        let config = DownloadConfig {
            initial_delay_ms: 500,
            backoff_multiplier: 3.0,
            ..Default::default()
        };

        let delay1 = calculate_backoff_delay(1, &config);
        assert_eq!(delay1, Duration::from_millis(500));

        let delay2 = calculate_backoff_delay(2, &config);
        assert_eq!(delay2, Duration::from_millis(1500));

        let delay3 = calculate_backoff_delay(3, &config);
        assert_eq!(delay3, Duration::from_millis(4500));
    }

    #[test]
    fn test_calculate_backoff_delay_max_limit() {
        let config = DownloadConfig {
            initial_delay_ms: 10000,
            backoff_multiplier: 10.0,
            ..Default::default()
        };

        let delay = calculate_backoff_delay(5, &config);

        assert_eq!(delay, Duration::from_millis(MAX_DELAY_MS));
    }

    #[test]
    fn test_download_silent_local_file() {
        if !can_connect("httpbin.org", 443) {
            eprintln!("Skipping network test: cannot reach httpbin.org");
            return;
        }
        let temp_dir = tempfile::tempdir().unwrap();
        let output_file = temp_dir.path().join("output.txt");

        let result = download_silent("https://httpbin.org/bytes/100", &output_file);

        assert!(result.is_ok());
        assert!(output_file.exists());
    }

    #[test]
    fn test_download_invalid_url() {
        let temp_dir = tempfile::tempdir().unwrap();
        let output_file = temp_dir.path().join("output.txt");

        let result = download_with_retry(
            "http://localhost:9999/nonexistent",
            &output_file,
            Some(DownloadConfig {
                max_retries: 2,
                ..Default::default()
            }),
        );

        assert!(result.is_err());
        assert!(!output_file.exists());
    }

    #[test]
    fn test_download_with_timeout() {
        let temp_dir = tempfile::tempdir().unwrap();
        let output_file = temp_dir.path().join("output.txt");

        let start = std::time::Instant::now();

        let result = download_with_retry(
            "http://httpbin.org/delay/10",
            &output_file,
            Some(DownloadConfig {
                max_retries: 2,
                timeout: Duration::from_millis(100),
                initial_delay_ms: 100,
                ..Default::default()
            }),
        );

        let elapsed = start.elapsed();

        assert!(result.is_err());
        assert!(elapsed < Duration::from_secs(1));
        assert!(!output_file.exists());
    }

    #[test]
    fn test_download_max_retries() {
        let temp_dir = tempfile::tempdir().unwrap();
        let output_file = temp_dir.path().join("output.txt");

        let start = std::time::Instant::now();

        let result = download_with_retry(
            "http://localhost:9999/nonexistent",
            &output_file,
            Some(DownloadConfig {
                max_retries: 3,
                initial_delay_ms: 100,
                ..Default::default()
            }),
        );

        let elapsed = start.elapsed();

        assert!(result.is_err());

        let expected_min_delay = Duration::from_millis(100 + 200);
        assert!(elapsed >= expected_min_delay);
    }

    #[test]
    fn test_download_creates_output_file() {
        if !can_connect("httpbin.org", 443) {
            eprintln!("Skipping network test: cannot reach httpbin.org");
            return;
        }
        let temp_dir = tempfile::tempdir().unwrap();
        let output_file = temp_dir.path().join("output.txt");

        let result = download_with_retry("https://httpbin.org/bytes/100", &output_file, None);

        assert!(result.is_ok());
        assert!(output_file.exists());

        let content = fs::read(&output_file).unwrap();
        assert_eq!(content.len(), 100);
    }

    #[test]
    fn test_download_result_success() {
        if !can_connect("httpbin.org", 443) {
            eprintln!("Skipping network test: cannot reach httpbin.org");
            return;
        }
        let temp_dir = tempfile::tempdir().unwrap();
        let output_file = temp_dir.path().join("output.txt");

        let result =
            download_with_retry("https://httpbin.org/bytes/100", &output_file, None).unwrap();

        assert!(result.success);
        assert!(result.attempts >= 1);
        assert_eq!(result.bytes_downloaded, 100);
        assert!(result.total_duration.as_millis() > 0);
    }
}
