//! TUI Interface Testing Framework
//!
//! Comprehensive testing suite for terraphim-ai TUI interface validation.
//! Provides mock terminals, command simulation, and cross-platform testing.

pub mod command_simulator;
pub mod cross_platform;
pub mod harness;
pub mod integration;
pub mod mock_terminal;
pub mod output_validator;
pub mod performance_monitor;

pub use command_simulator::*;
pub use cross_platform::*;
pub use harness::*;
pub use integration::*;
pub use mock_terminal::*;
pub use output_validator::*;
pub use performance_monitor::*;
