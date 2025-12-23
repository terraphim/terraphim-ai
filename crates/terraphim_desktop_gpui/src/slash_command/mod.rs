//! Universal Slash Command System for GPUI
//!
//! This module provides a comprehensive slash command system that supports:
//! - View-scoped commands (Chat, Search, or Both)
//! - Knowledge Graph integration for contextual suggestions
//! - Trigger detection (/, ++) with debouncing
//! - GPUI-native inline popup UI
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                Universal Command System                     │
//! ├─────────────────────────────────────────────────────────────┤
//! │  CommandRegistry → SuggestionProviders → TriggerEngine     │
//! │                          ↓                                  │
//! │                  SlashCommandPopup (GPUI)                   │
//! └─────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Usage
//!
//! ```rust,ignore
//! use terraphim_desktop_gpui::slash_command::{CommandRegistry, ViewScope};
//!
//! // Create registry with built-in commands
//! let registry = CommandRegistry::with_builtin_commands();
//!
//! // Get commands for Chat view
//! let commands = registry.for_scope(ViewScope::Chat);
//!
//! // Search commands
//! let suggestions = registry.suggest("search", ViewScope::Chat, 10);
//!
//! // Execute a command
//! let context = CommandContext::new("rust", ViewScope::Chat);
//! let result = registry.execute("search", context);
//! ```

pub mod popup;
pub mod providers;
pub mod registry;
pub mod trigger;
pub mod types;

// Re-exports
pub use registry::CommandRegistry;
pub use types::{
    CommandCategory, CommandContext, CommandHandler, CommandIcon, CommandResult,
    SuggestionAction, SuggestionMetadata, TriggerInfo, TriggerType, UniversalCommand,
    UniversalSuggestion, ViewScope,
};
pub use providers::{
    CommandPaletteProvider, CompositeProvider, KGEnhancedCommandProvider,
    KnowledgeGraphProvider, SuggestionProvider,
};
pub use trigger::{
    CharTrigger, DebounceManager, TriggerConfig, TriggerDetectionResult, TriggerEngine,
};
pub use popup::{SlashCommandPopup, SlashCommandPopupEvent};
