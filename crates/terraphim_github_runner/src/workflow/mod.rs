//! Workflow parsing and execution
//!
//! This module provides:
//! - LLM-based workflow understanding (parser.rs)
//! - Step-by-step execution with snapshots (executor.rs)

pub mod parser;

// Will be implemented in Step 2.2
// pub mod executor;

pub use parser::{ParsedWorkflow, WorkflowParser, WorkflowStep};
