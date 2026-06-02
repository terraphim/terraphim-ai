/// Provider adapter for Claude Code usage via the `ccusage` tool.
#[cfg(feature = "providers")]
pub mod ccusage;
/// Provider adapter for Claude (Anthropic) usage.
#[cfg(feature = "providers")]
pub mod claude;
/// Provider adapter for Kimi (Moonshot AI) usage.
#[cfg(feature = "providers")]
pub mod kimi;
/// Provider adapter for MiniMax usage.
#[cfg(feature = "providers")]
pub mod minimax;
/// Provider adapter for OpenCode Go local database usage.
#[cfg(feature = "providers")]
pub mod opencode_go;
/// Provider adapter for Z.ai (ZAI) usage.
#[cfg(feature = "providers")]
pub mod zai;
