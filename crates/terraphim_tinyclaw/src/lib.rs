//! TinyClaw - Multi-channel AI assistant for Terraphim
//!
//! This crate provides a multi-channel AI assistant that can interact
//! through Telegram, Discord, CLI, and other channels. It includes:
//!
//! - **Agent Loop**: Core conversation handling with tool calling
//! - **Channels**: Adapters for different messaging platforms
//! - **Tools**: Extensible tool system for filesystem, web, shell operations
//! - **Skills**: JSON-based reusable workflows
//! - **Session Management**: Persistent conversation history

pub mod agent;
pub mod bus;
pub mod channel;
pub mod channels;
pub mod config;
pub mod format;
pub mod session;
pub mod skills;
pub mod tools;
