//! Static detection of Explicit Deferral Markers (EDMs) in Rust source files.

mod exclusion;
mod scanner;

pub use exclusion::is_non_production;
pub use scanner::NegativeContributionScanner;
