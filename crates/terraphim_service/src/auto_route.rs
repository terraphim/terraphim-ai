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
//! is unset, [`JMAP_MISSING_TOKEN_PENALTY`] is subtracted (saturating at zero)
//! from its raw distinct-concept score. This is a per-haystack-type policy:
//! future additions to `ServiceType` that also need ambient credentials should
//! consider applying the same penalty.

use ahash::AHashSet;
use terraphim_config::{Config, ConfigState, ServiceType};
use terraphim_rolegraph::RoleGraph;
use terraphim_types::RoleName;

/// Legacy multiplicative downweight retained for fixture link-compat only.
/// New code uses [`JMAP_MISSING_TOKEN_PENALTY`] (saturating subtraction).
#[deprecated(
    note = "use JMAP_MISSING_TOKEN_PENALTY (saturating subtraction); kept exported only for fixture link-compat"
)]
pub const JMAP_MISSING_TOKEN_DOWNWEIGHT: f64 = 0.5;

/// Penalty subtracted (saturating at zero) from a role's raw distinct-concept
/// score when the role has any `ServiceType::Jmap` haystack and
/// `$JMAP_ACCESS_TOKEN` is not set.
///
/// Distinct-concept scores typically fall in `0..=10`; multiplicative
/// downweights collapse at those magnitudes (see design section 2.3).
/// Subtraction keeps the policy monotonic without rounding pathology.
pub const JMAP_MISSING_TOKEN_PENALTY: i64 = 1;

/// Count distinct canonical concept IDs from `rg.thesaurus` that any substring
/// of `query` matches. Uses the rolegraph's pre-built Aho-Corasick automaton
/// (populated from the thesaurus at `RoleGraph::new_sync` time and requires
/// no document indexing). Returns 0 when the thesaurus is empty or no concept
/// matches.
fn score_distinct_concepts(rg: &RoleGraph, query: &str) -> usize {
    let ids = rg.find_matching_node_ids(query);
    let unique: AHashSet<u64> = ids.into_iter().collect();
    unique.len()
}

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
    /// The chosen role's final score (post-penalty). Semantically the count
    /// of distinct canonical concept IDs in the role's thesaurus that the
    /// query touched, optionally reduced by [`JMAP_MISSING_TOKEN_PENALTY`].
    pub score: i64,
    /// All scored candidates including zero-scored, sorted by `(-score, name)`.
    pub candidates: Vec<(RoleName, i64)>,
    /// Why this role was chosen.
    pub reason: AutoRouteReason,
}

/// Choose a role for `query` by scoring each in-memory rolegraph.
///
/// Returns a concrete role per the policies in section 3 of the design.
/// The function never errors; in degenerate cases (no roles configured)
/// returns a synthesised result with `RoleName::from("Default")` and reason
/// `ZeroMatchDefault`.
pub async fn auto_select_role(
    query: &str,
    config: &Config,
    state: &ConfigState,
    ctx: &AutoRouteContext,
) -> AutoRouteResult {
    // Score every role in state.roles. Lock each rolegraph sequentially.
    // Scoring is "distinct canonical concept count" against the role's
    // thesaurus-driven Aho-Corasick automaton; this works cold (no document
    // indexing required), unlike the prior Node.rank sum.
    let mut scored: Vec<(RoleName, i64)> = Vec::with_capacity(state.roles.len());
    for (role_name, rg_sync) in state.roles.iter() {
        let rg = rg_sync.lock().await;
        let raw_score: i64 = score_distinct_concepts(&rg, query) as i64;

        // PA / JMAP penalty: applied to role total, not per-term.
        let has_jmap = config
            .roles
            .get(role_name)
            .map(|r| r.haystacks.iter().any(|h| h.service == ServiceType::Jmap))
            .unwrap_or(false);

        let final_score: i64 = if has_jmap && !ctx.jmap_token_present {
            raw_score.saturating_sub(JMAP_MISSING_TOKEN_PENALTY)
        } else {
            raw_score
        };

        scored.push((role_name.clone(), final_score));
    }

    // Sort by (-score, name asc) so candidates[0] is the natural winner and
    // ties are broken alphabetically.
    scored.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.original.cmp(&b.0.original)));

    // Degenerate: no roles configured.
    if scored.is_empty() {
        return AutoRouteResult {
            role: RoleName::from("Default"),
            score: 0,
            candidates: Vec::new(),
            reason: AutoRouteReason::ZeroMatchDefault,
        };
    }

    let top_score = scored[0].1;

    // Zero-match path.
    if top_score == 0 {
        if let Some(sel) = ctx.selected_role.as_ref() {
            if config.roles.contains_key(sel) {
                return AutoRouteResult {
                    role: sel.clone(),
                    score: 0,
                    candidates: scored,
                    reason: AutoRouteReason::ZeroMatchSelectedRole,
                };
            }
        }
        // Fall back to Default if present, else the alphabetically first role.
        let default_role = RoleName::from("Default");
        let chosen = if config.roles.contains_key(&default_role) {
            default_role
        } else {
            scored[0].0.clone()
        };
        return AutoRouteResult {
            role: chosen,
            score: 0,
            candidates: scored,
            reason: AutoRouteReason::ZeroMatchDefault,
        };
    }

    // Collect the tied set (all roles sharing top_score).
    let tied: Vec<&RoleName> = scored
        .iter()
        .filter(|(_, s)| *s == top_score)
        .map(|(n, _)| n)
        .collect();

    if tied.len() == 1 {
        return AutoRouteResult {
            role: scored[0].0.clone(),
            score: top_score,
            candidates: scored,
            reason: AutoRouteReason::ScoredWinner,
        };
    }

    // Tie-break: prefer selected_role if it's in the tied set.
    if let Some(sel) = ctx.selected_role.as_ref() {
        if tied.iter().any(|n| *n == sel) {
            return AutoRouteResult {
                role: sel.clone(),
                score: top_score,
                candidates: scored,
                reason: AutoRouteReason::TieBrokenBySelectedRole,
            };
        }
    }

    AutoRouteResult {
        role: scored[0].0.clone(),
        score: top_score,
        candidates: scored,
        reason: AutoRouteReason::TieBrokenAlphabetically,
    }
}
