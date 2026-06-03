//! Multi-step flow orchestration for ADF agent pipelines.
/// Flow configuration types (step definitions, matrix expansion, strategies).
pub mod config;
/// Step result envelopes that carry output, timing, and cost data.
pub mod envelope;
/// Flow executor that drives step-by-step execution with timeout and error handling.
pub mod executor;
/// Persistent flow state and run-record storage.
pub mod state;
/// Template token parser for `{{steps.<name>.*}}` variable substitution.
pub mod token_parser;
