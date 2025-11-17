//! Intelligent GitHub Self-Hosted Runner
//!
//! This crate provides an AI-powered GitHub Actions runner that:
//! - Registers as a standard self-hosted runner
//! - Uses knowledge graphs to understand CI/CD actions semantically
//! - Executes actions securely in Firecracker VMs
//! - Learns from successful runs to improve performance
//!
//! # Architecture
//!
//! The runner consists of several components:
//! - **Registration**: GitHub API integration for runner lifecycle
//! - **Parsers**: Extract knowledge from Earthfiles, Dockerfiles, and workflows
//! - **Knowledge Graph**: Semantic understanding of actions and dependencies
//! - **Interpreter**: LLM-powered action translation
//! - **Executor**: Firecracker VM-based secure execution
//! - **History**: Learning system for successful patterns

pub mod parsers;
pub mod knowledge_graph;
pub mod interpreter;
pub mod executor;
pub mod history;
pub mod registration;
pub mod event_handler;
pub mod agent;
pub mod error;
pub mod types;

pub use error::{RunnerError, RunnerResult};
pub use types::*;
pub use registration::RunnerRegistration;
pub use event_handler::WorkflowEventHandler;
pub use executor::ActionExecutor;
pub use agent::GitHubRunnerAgent;
