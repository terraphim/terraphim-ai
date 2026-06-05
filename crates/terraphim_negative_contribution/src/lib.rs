//! Static detection of Explicit Deferral Markers (EDMs) in Rust source files.

mod exclusion;
mod scanner;

/// Returns `true` if the given file path should be excluded from negative-contribution scanning (e.g. test files, examples).
pub use exclusion::is_non_production;
/// Scanner that walks Rust source files and reports Explicit Deferral Markers (EDMs) such as `todo!`, `unimplemented!`, and `#[allow(dead_code)]`.
pub use scanner::NegativeContributionScanner;
