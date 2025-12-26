//! # Terraphim Firecracker
//!
//! Sub-2 second VM boot optimization system for Terraphim AI coding assistant.
//!
//! This crate provides:
//! - VM pool management with prewarming for instant allocation
//! - Firecracker VM lifecycle management
//! - Performance optimization for sub-2 second boot times
//! - Snapshot-based state management
//!
//! ## Architecture
//!
//! ```text
//! VmPoolManager
//!     ├── VmAllocator (allocation strategy)
//!     ├── PrewarmingManager (maintain pool levels)
//!     ├── VmMaintenanceManager (health checks)
//!     └── Sub2SecondOptimizer (boot optimization)
//!
//! Sub2SecondVmManager
//!     ├── FirecrackerClient (Firecracker API)
//!     ├── VmStorage (state persistence)
//!     └── Performance optimization
//! ```
//!
//! ## Example
//!
//! ```rust,ignore
//! use terraphim_firecracker::{VmPoolManager, PoolConfig, Sub2SecondOptimizer};
//! use std::sync::Arc;
//!
//! // Create optimizer
//! let optimizer = Arc::new(Sub2SecondOptimizer::new());
//!
//! // Create pool manager with default config
//! let pool_manager = VmPoolManager::new(vm_manager, optimizer, PoolConfig::default());
//!
//! // Initialize pools
//! pool_manager.initialize_pools(vec!["focal-optimized".to_string()]).await?;
//!
//! // Allocate a VM (sub-500ms from pool)
//! let (vm_instance, allocation_time) = pool_manager.allocate_vm("focal-optimized").await?;
//! ```

// Core modules
pub mod config;
pub mod error;
pub mod manager;
pub mod performance;
pub mod pool;
pub mod storage;
pub mod vm;

// Re-exports for convenient access

// VM types
pub use vm::{
    FirecrackerClient, Sub2SecondVmManager, Vm, VmConfig, VmInstance, VmManager, VmMetrics,
    VmState, VmStorage,
};

// Pool management
pub use pool::{
    PoolConfig, PoolStats, PoolTypeStats, PrewarmedState, PrewarmedVm, VmAllocator,
    VmMaintenanceManager, VmPoolManager,
};

// Performance optimization
pub use performance::{
    BenchmarkResults, BootMetrics, OptimizationStrategy, PerformanceMetrics, PerformanceMonitor,
    PrewarmingManager, Sub2SecondOptimizer,
};

// Storage
pub use storage::InMemoryVmStorage;

// Configuration
pub use config::Config;

/// Crate version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Target allocation time for prewarmed VMs (500ms)
pub const PREWARMED_ALLOCATION_TARGET_MS: u64 = 500;

/// Target boot time for VMs (2 seconds)
pub const TARGET_BOOT_TIME_MS: u64 = 2000;
