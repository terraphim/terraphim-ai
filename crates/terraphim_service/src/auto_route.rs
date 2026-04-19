//! Intent-based role auto-routing.
//!
//! Scores every configured role's in-memory `RoleGraph` against the query and
//! returns the role with the highest rank-weighted match. Used by both the CLI
//! (`terraphim-agent search` without `--role`) and the MCP server's `search`
//! tool when the `role` argument is unset.
//!
//! Design: `docs/research/design-intent-based-role-auto-routing.md`.
//!
//! # Locking
//!
//! Acquires every `RoleGraphSync` mutex sequentially. Lock-hold time on these
//! mutexes is now part of routing latency; long-running writers (re-indexing,
//! bulk ingest) will serialise routing behind them.
//!
//! # PA / JMAP downweight
//!
//! When a role has any `ServiceType::Jmap` haystack and `$JMAP_ACCESS_TOKEN`
//! is unset, its raw rank-sum is multiplied by [`JMAP_MISSING_TOKEN_DOWNWEIGHT`].
//! This is a per-haystack-type policy: future additions to `ServiceType` that
//! also need ambient credentials should consider applying the same downweight.

use terraphim_config::{Config, ConfigState};
use terraphim_types::RoleName;

/// Score downweight applied to a role total when it has any `ServiceType::Jmap`
/// haystack and `$JMAP_ACCESS_TOKEN` is not set.
///
/// Rationale: PA roughly has a two-haystack design (Obsidian + JMAP); with one
/// half disabled, effective coverage halves. Tunable without an API change.
pub const JMAP_MISSING_TOKEN_DOWNWEIGHT: f64 = 0.5;

/// Inputs the helper needs that are not part of `Config`/`ConfigState`.
#[derive(Debug, Clone)]
pub struct AutoRouteContext {
    /// The persisted `selected_role` if it is non-empty AND present in
    /// `config.roles`. Callers should normalise via [`AutoRouteContext::from_env`]
    /// or by hand before constructing this struct.
    pub selected_role: Option<RoleName>,
    /// Whether `$JMAP_ACCESS_TOKEN` is set and non-empty.
    pub jmap_token_present: bool,
}

impl AutoRouteContext {
    /// Build a context from the process environment.
    ///
    /// Reads `$JMAP_ACCESS_TOKEN` once. The caller is responsible for
    /// normalising `selected_role` against `config.roles` before calling.
    pub fn from_env(selected_role: Option<RoleName>) -> Self {
        let jmap_token_present = std::env::var("JMAP_ACCESS_TOKEN")
            .map(|v| !v.trim().is_empty())
            .unwrap_or(false);
        Self {
            selected_role,
            jmap_token_present,
        }
    }
}

/// Why the helper picked the role it did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AutoRouteReason {
    /// One role had the strictly highest score.
    ScoredWinner,
    /// Multiple roles tied at the top score and `selected_role` was among them.
    TieBrokenBySelectedRole,
    /// Multiple roles tied at the top score and the alphabetically first was picked.
    TieBrokenAlphabetically,
    /// All roles scored zero and `selected_role` was set; that role was returned.
    ZeroMatchSelectedRole,
    /// All roles scored zero and `selected_role` was unset; `Default` (or, if
    /// `Default` is absent, the alphabetically first role) was returned.
    ZeroMatchDefault,
}

/// Result of a routing decision.
#[derive(Debug, Clone)]
pub struct AutoRouteResult {
    /// The chosen role.
    pub role: RoleName,
    /// The chosen role's final score (post-downweight).
    pub score: i64,
    /// All scored candidates including zero-scored, sorted by `(-score, name)`.
    pub candidates: Vec<(RoleName, i64)>,
    /// Why this role was chosen.
    pub reason: AutoRouteReason,
}

/// Choose a role for `query` by scoring each in-memory rolegraph.
///
/// Skeleton: implementation lands in the next commit. For now this returns
/// the persisted `selected_role` (or `Default`) so dependent crates compile.
pub async fn auto_select_role(
    _query: &str,
    config: &Config,
    _state: &ConfigState,
    ctx: &AutoRouteContext,
) -> AutoRouteResult {
    let role = ctx
        .selected_role
        .clone()
        .filter(|r| config.roles.contains_key(r))
        .unwrap_or_else(|| RoleName::from("Default"));
    AutoRouteResult {
        role,
        score: 0,
        candidates: Vec::new(),
        reason: AutoRouteReason::ZeroMatchDefault,
    }
}
