//! Channel adapters for different chat platforms.

#[cfg(feature = "telegram")]
pub mod telegram;

#[cfg(feature = "discord")]
pub mod discord;

// Note: matrix module disabled due to sqlite dependency conflict
// Re-enable when matrix-sdk updates to compatible rusqlite version
// #[cfg(feature = "matrix")]
// pub mod matrix;

pub mod cli;
