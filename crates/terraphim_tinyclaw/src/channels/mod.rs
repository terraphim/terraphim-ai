//! Channel adapters for different chat platforms.

#[cfg(feature = "telegram")]
pub mod telegram;

#[cfg(feature = "discord")]
pub mod discord;

#[cfg(feature = "slack")]
pub mod slack;

// Note: matrix module disabled due to sqlite dependency conflict
// Re-enable when matrix-sdk updates to compatible rusqlite version
// #[cfg(feature = "matrix")]
// pub mod matrix;

pub mod cli;

// Wave 4 (Phase B) channels added for Hermes parity.
// These are unconditionally compiled (no feature gate) because they
// don't pull in heavy SDK dependencies.
pub mod email;
pub mod gitea;
pub mod github;
pub mod linear;
