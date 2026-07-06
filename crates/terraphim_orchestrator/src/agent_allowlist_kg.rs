//! Loads the `recognised_agent` KG concept
//! (`crates/terraphim_orchestrator/kg/recognised_agents.md`) into the set of
//! fleet automation logins consumed by [`crate::pr_review::author_is_agent`].
//!
//! Follows the project-wide KG markdown convention (see root `CLAUDE.md`
//! "Extending Knowledge Graph"): a `synonyms::` line lists the recognised
//! terms — here, exact-match Gitea PR author logins. Operators extend the
//! allowlist by editing that line; no rebuild is required because the
//! deployed copy under `/opt/ai-dark-factory/kg/recognised_agents.md` (or
//! the path in `ADF_RECOGNISED_AGENTS_KG`) is re-read from disk, falling
//! back to the binary's embedded copy if the file is missing.
//!
//! Parsing is a pure function ([`parse_recognised_agents`]) so it is testable
//! without I/O; only [`load_recognised_agents`] touches the filesystem.

use std::collections::BTreeSet;
use std::path::Path;

/// The `recognised_agents.md` shipped in the repo, embedded at compile time
/// as the fallback used when no on-disk KG file is found.
const DEFAULT_KG_MARKDOWN: &str = include_str!("../kg/recognised_agents.md");

/// Env var pointing at an on-disk override of `recognised_agents.md`
/// (e.g. `/opt/ai-dark-factory/kg/recognised_agents.md` in production, so
/// operators can extend the allowlist without redeploying the orchestrator
/// binary).
const KG_PATH_ENV_VAR: &str = "ADF_RECOGNISED_AGENTS_KG";

/// Default on-disk path checked when `ADF_RECOGNISED_AGENTS_KG` is unset,
/// relative to the orchestrator's working directory.
const DEFAULT_KG_RELATIVE_PATH: &str = "kg/recognised_agents.md";

/// Parse a `synonyms::` line (case-insensitive key, comma-separated values)
/// out of KG markdown, returning the recognised logins verbatim (trimmed,
/// case-preserved — Gitea logins are case-sensitive).
///
/// Multiple `synonyms::` lines are merged. Lines that don't match the
/// directive are ignored, matching the tolerant parsing style used
/// elsewhere for this KG format (see `kg_router.rs`).
pub fn parse_recognised_agents(markdown: &str) -> BTreeSet<String> {
    markdown
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            let lower = trimmed.to_ascii_lowercase();
            lower
                .starts_with("synonyms::")
                .then(|| trimmed["synonyms::".len()..].to_string())
        })
        .flat_map(|rest| {
            rest.split(',')
                .map(|s| s.trim().to_string())
                .collect::<Vec<_>>()
        })
        .filter(|s| !s.is_empty())
        .collect()
}

/// Load the recognised-agent allowlist from disk, falling back to the
/// embedded default when the file is absent or unreadable.
///
/// Path resolution: `ADF_RECOGNISED_AGENTS_KG` env var if set, else
/// `kg/recognised_agents.md` relative to the current working directory.
pub fn load_recognised_agents() -> BTreeSet<String> {
    let path =
        std::env::var(KG_PATH_ENV_VAR).unwrap_or_else(|_| DEFAULT_KG_RELATIVE_PATH.to_string());
    let markdown = std::fs::read_to_string(Path::new(&path))
        .unwrap_or_else(|_| DEFAULT_KG_MARKDOWN.to_string());
    parse_recognised_agents(&markdown)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_embedded_default() {
        let logins = parse_recognised_agents(DEFAULT_KG_MARKDOWN);
        assert!(logins.contains("claude-code"));
        assert!(logins.contains("root"));
        assert!(logins.contains("implementation-swarm"));
    }

    #[test]
    fn parses_multiple_synonyms_lines() {
        let markdown = "# Concept\n\nsynonyms:: alpha, beta\nsynonyms:: gamma\n";
        let logins = parse_recognised_agents(markdown);
        assert_eq!(
            logins,
            ["alpha", "beta", "gamma"]
                .into_iter()
                .map(String::from)
                .collect::<BTreeSet<_>>()
        );
    }

    #[test]
    fn ignores_unrelated_lines_and_trims_whitespace() {
        let markdown = "# Title\n\nSome prose.\n\nsynonyms::  claude-code ,  root  \n";
        let logins = parse_recognised_agents(markdown);
        assert_eq!(
            logins,
            ["claude-code", "root"]
                .into_iter()
                .map(String::from)
                .collect::<BTreeSet<_>>()
        );
    }

    #[test]
    fn returns_empty_set_when_no_synonyms_line_present() {
        assert!(parse_recognised_agents("# Title\n\nJust prose, no directive.\n").is_empty());
    }

    #[test]
    fn load_falls_back_to_embedded_default_when_env_path_missing() {
        // SAFETY: test-only env mutation, no concurrent readers of this var
        // in this crate's test suite.
        unsafe {
            std::env::set_var(KG_PATH_ENV_VAR, "/nonexistent/path/recognised_agents.md");
        }
        let logins = load_recognised_agents();
        assert!(logins.contains("implementation-swarm"));
        unsafe {
            std::env::remove_var(KG_PATH_ENV_VAR);
        }
    }
}
