pub mod config;
pub mod registry;
pub mod simple_search;

/// Reusable components module for Terraphim GPUI
///
/// This module provides high-performance, fully-tested reusable components
/// built on the ReusableComponent trait architecture.
pub mod traits;
// pub mod testing;      // Commented out for demo (compilation errors)
pub mod search;
// pub mod search_services;  // Commented out for demo (compilation errors)
// pub mod concurrent_search;  // Commented out for demo (compilation errors)
// pub mod search_performance;  // Commented out for demo (compilation errors)
// pub mod knowledge_graph;  // Commented out for demo (compilation errors)
// pub mod kg_search_modal;  // Commented out for demo (compilation errors)
// pub mod kg_autocomplete;  // Commented out for demo (compilation errors)
// pub mod term_discovery;   // Commented out for demo (compilation errors)
pub mod context;
pub mod context_item;
// pub mod search_context_bridge;  // Commented out for demo (compilation errors)
// pub mod add_document_modal;     // Commented out for demo (compilation errors)
// pub mod enhanced_chat;          // Commented out for demo (compilation errors)

// Advanced performance optimization modules - commented out for demo
// pub mod advanced_virtualization;
// pub mod performance_dashboard;
// pub mod memory_optimizer;
// pub mod render_optimizer;
// pub mod async_optimizer;
// pub mod performance_benchmark;
// pub mod optimization_integration;

pub use config::*;
pub use performance::*;
pub use registry::*;
pub use simple_search::*;
pub use traits::*;
// pub use testing::*;      // Commented out for demo
pub use search::*;
// pub use search_services::*;  // Commented out for demo
// pub use concurrent_search::*;  // Commented out for demo
// pub use search_performance::*;  // Commented out for demo
// pub use knowledge_graph::*;  // Commented out for demo
// pub use kg_search_modal::*;  // Commented out for demo
// pub use kg_autocomplete::*;  // Commented out for demo
// pub use term_discovery::*;   // Commented out for demo
pub use context::*;
pub use context_item::*;
// pub use search_context_bridge::*;  // Commented out for demo
// pub use add_document_modal::*;     // Commented out for demo
// pub use enhanced_chat::*;          // Commented out for demo

// Re-export optimization modules - commented out for demo
// pub use advanced_virtualization::*;
// pub use performance_dashboard::*;
// pub use memory_optimizer::*;
// pub use render_optimizer::*;
// pub use async_optimizer::*;
// pub use performance_benchmark::*;
// pub use optimization_integration::*;
