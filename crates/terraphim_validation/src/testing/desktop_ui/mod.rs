//! Desktop UI testing framework for Terraphim AI
//!
//! This module provides comprehensive UI testing capabilities for the Terraphim
//! desktop application built with Tauri and Svelte, including:
//!
//! - Browser automation using Playwright
//! - Window management and lifecycle control
//! - Visual regression testing with screenshots
//! - Cross-platform UI validation (macOS, Windows, Linux)
//! - Auto-updater testing
//! - Performance and accessibility testing
//! - Integration testing with backend services

use serde::{Deserialize, Serialize};

pub mod accessibility;
pub mod auto_updater;
pub mod components;
pub mod cross_platform;
pub mod harness;
pub mod integration;
pub mod orchestrator;
pub mod performance;
pub mod utils;

// Simple result types for desktop UI testing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UITestResult {
    pub name: String,
    pub status: UITestStatus,
    pub message: Option<String>,
    pub details: Option<String>,
    pub duration_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UITestStatus {
    Pass,
    Fail,
    Skip,
    Error,
}

impl std::fmt::Display for UITestStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UITestStatus::Pass => write!(f, "Pass"),
            UITestStatus::Fail => write!(f, "Fail"),
            UITestStatus::Skip => write!(f, "Skip"),
            UITestStatus::Error => write!(f, "Error"),
        }
    }
}

// Re-export main types and functions
pub use accessibility::{AccessibilityTestConfig, AccessibilityTester};
pub use auto_updater::{AutoUpdaterTestConfig, AutoUpdaterTester};
pub use components::{ComponentTestConfig, UIComponentTester};
pub use cross_platform::{CrossPlatformTestConfig, CrossPlatformUITester};
pub use harness::{DesktopUITestConfig, DesktopUITestHarness};
pub use integration::{IntegrationTestConfig, IntegrationTester};
pub use orchestrator::{DesktopUITestOrchestrator, DesktopUITestSuiteConfig, TestSuiteResults};
pub use performance::{PerformanceResults, PerformanceTestConfig, PerformanceTester};
pub use utils::{
    ElementUtils, PlatformUtils, ResultUtils, ScreenshotComparison, ScreenshotUtils, TestDataUtils,
};
