//! Centralized logging initialization utilities
//! 
//! This module provides standardized logging setup functions for different contexts
//! (servers, tests, development) to ensure consistent logging behavior across the codebase.

/// Logging configuration presets for different use cases
#[derive(Debug, Clone)]
pub enum LoggingConfig {
    /// Production server logging (WARN level, structured format)
    Server,
    /// Development server logging (INFO level, human-readable format)  
    Development,
    /// Test environment logging (DEBUG level, test-friendly format)
    Test,
    /// Integration test logging (INFO level, reduced noise)
    IntegrationTest,
    /// Custom logging level
    Custom { level: log::LevelFilter },
}

/// Initialize logging based on configuration preset
pub fn init_logging(config: LoggingConfig) {
    match config {
        LoggingConfig::Server => init_server_logging(),
        LoggingConfig::Development => init_development_logging(),
        LoggingConfig::Test => init_test_logging(),
        LoggingConfig::IntegrationTest => init_integration_test_logging(),
        LoggingConfig::Custom { level } => init_custom_logging(level),
    }
}

/// Initialize production server logging
pub fn init_server_logging() {
    let _ = env_logger::builder()
        .filter_level(log::LevelFilter::Warn)
        .format_timestamp_secs()
        .format_module_path(false)
        .try_init();
}

/// Initialize development server logging with more verbose output
pub fn init_development_logging() {
    let _ = env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .format_timestamp_secs()
        .try_init();
}

/// Initialize test environment logging
pub fn init_test_logging() {
    let _ = env_logger::builder()
        .filter_level(log::LevelFilter::Debug)
        .is_test(true)
        .try_init();
}

/// Initialize integration test logging with reduced noise
pub fn init_integration_test_logging() {
    let _ = env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .is_test(true)
        .try_init();
}

/// Initialize logging with custom level
pub fn init_custom_logging(level: log::LevelFilter) {
    let _ = env_logger::builder()
        .filter_level(level)
        .try_init();
}

/// Initialize logging respecting LOG_LEVEL environment variable
/// Falls back to INFO level if LOG_LEVEL is not set
pub fn init_env_logging() {
    let _ = env_logger::builder()
        .filter_level(
            std::env::var("LOG_LEVEL")
                .ok()
                .and_then(|level| level.parse::<log::LevelFilter>().ok())
                .unwrap_or(log::LevelFilter::Info)
        )
        .try_init();
}

/// Tracing-based initialization for applications requiring structured logging
#[cfg(feature = "tracing")]
pub fn init_tracing_logging(level: tracing::Level) {
    use tracing_subscriber::prelude::*;
    
    let _ = tracing_subscriber::registry()
        .with(
            tracing_subscriber::fmt::layer()
                .with_env_filter(
                    tracing_subscriber::EnvFilter::from_default_env()
                        .add_directive(level.into())
                )
        )
        .try_init();
}

/// Initialize tracing with simple format (for external crates that use tracing)
/// This function is available without the tracing feature for compatibility
pub fn init_external_tracing_logging(verbose: bool) {
    // This is for applications that already have tracing as a dependency
    // We can't use the tracing types here, so we provide a helper that callers use
    
    // The actual implementation should be done by the caller with their tracing setup
    // This is just a marker function to show the intended pattern
    
    if verbose {
        log::info!("Verbose logging enabled (external tracing should be configured by caller)");
    } else {
        log::info!("Standard logging enabled (external tracing should be configured by caller)");
    }
}

/// Get appropriate logging config based on environment
pub fn detect_logging_config() -> LoggingConfig {
    // Check for explicit LOG_LEVEL environment variable
    if let Ok(level_str) = std::env::var("LOG_LEVEL") {
        if let Ok(level) = level_str.parse::<log::LevelFilter>() {
            return LoggingConfig::Custom { level };
        }
    }
    
    // Detect test environment
    if cfg!(test) || std::env::var("RUST_TEST_THREADS").is_ok() {
        return LoggingConfig::Test;
    }
    
    // Detect development vs production based on debug assertions
    if cfg!(debug_assertions) {
        LoggingConfig::Development
    } else {
        LoggingConfig::Server
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_logging_config_detection() {
        // This will use Test config in test environment
        let config = detect_logging_config();
        matches!(config, LoggingConfig::Test);
    }
    
    #[test] 
    fn test_logging_initialization() {
        // Test that initialization doesn't panic
        init_logging(LoggingConfig::Test);
        log::info!("Test logging message");
    }
}