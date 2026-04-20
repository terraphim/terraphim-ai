//! Per-provider stderr classification into throttle / flake / unknown.
//!
//! Complements [`crate::agent_run_record::ExitClassifier`]: the classifier
//! here consumes *configurable* per-provider regex lists so operators can
//! tune detection per CLI (`claude-code`, `opencode-go`, `kimi-for-coding`,
//! ...) without code changes. Matches feed the existing
//! [`terraphim_spawner::health::CircuitBreaker`] (via
//! [`crate::provider_probe::ProviderHealthMap`]) and the
//! [`crate::provider_budget::ProviderBudgetTracker`] -- nothing new is
//! invented here.
//!
//! Classification outcomes:
//! * **Throttle** -- provider quota / rate-limit hit. Trip the breaker and
//!   force the hour+day budget windows past their caps so the routing
//!   filter drops the provider until the next window rolls.
//! * **Flake** -- transient failure (timeout, EOF, connection reset). Do
//!   NOT trip the breaker; the dispatch layer retries with the next entry
//!   in the pool.
//! * **Unknown** -- neither list matched. Escalate (fleet-meta issue) so a
//!   human can classify the pattern. Unknown is also counted as a soft
//!   failure so a pathological provider that repeatedly emits unclassified
//!   errors still eventually opens the breaker.

use std::collections::HashMap;
use std::fmt;

use regex::{Regex, RegexBuilder};
use serde::{Deserialize, Serialize};

/// Classifier verdict returned by [`classify`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorKind {
    /// Provider has hit its quota / rate limit -- back off for a window.
    Throttle,
    /// Transient failure (timeout, EOF). Retry next pool entry.
    Flake,
    /// No pattern matched. Escalate for human review.
    Unknown,
}

impl fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ErrorKind::Throttle => f.write_str("throttle"),
            ErrorKind::Flake => f.write_str("flake"),
            ErrorKind::Unknown => f.write_str("unknown"),
        }
    }
}

/// Serialised form of per-provider error signatures (matches the TOML
/// layout under `[[providers]].error_signatures`).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderErrorSignatures {
    /// Regex patterns matching rate-limit / quota errors.
    #[serde(default)]
    pub throttle: Vec<String>,
    /// Regex patterns matching transient errors (timeout, EOF, reset).
    #[serde(default)]
    pub flake: Vec<String>,
}

/// Compile error building per-provider regex patterns.
#[derive(Debug)]
pub struct CompileError {
    pub provider: String,
    pub pattern: String,
    pub source: regex::Error,
}

impl fmt::Display for CompileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "invalid error-signature regex for provider '{}': `{}`: {}",
            self.provider, self.pattern, self.source
        )
    }
}

impl std::error::Error for CompileError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.source)
    }
}

/// Runtime-compiled signatures for a single provider.
#[derive(Debug, Clone, Default)]
pub struct CompiledSignatures {
    /// Regexes that classify a stderr line as [`ErrorKind::Throttle`].
    pub throttle: Vec<Regex>,
    /// Regexes that classify a stderr line as [`ErrorKind::Flake`].
    pub flake: Vec<Regex>,
}

impl CompiledSignatures {
    /// Compile a [`ProviderErrorSignatures`] into runtime regexes.
    ///
    /// Patterns are compiled case-insensitively so the config can use the
    /// canonical spelling (`429`, `rate limit`, `timeout`) and still match
    /// mixed-case CLI output.
    pub fn compile(
        provider: &str,
        sigs: &ProviderErrorSignatures,
    ) -> Result<Self, CompileError> {
        let throttle = compile_list(provider, &sigs.throttle)?;
        let flake = compile_list(provider, &sigs.flake)?;
        Ok(Self { throttle, flake })
    }

    /// Whether this provider has any signatures configured at all.
    pub fn is_empty(&self) -> bool {
        self.throttle.is_empty() && self.flake.is_empty()
    }
}

fn compile_list(provider: &str, patterns: &[String]) -> Result<Vec<Regex>, CompileError> {
    patterns
        .iter()
        .map(|p| {
            RegexBuilder::new(p)
                .case_insensitive(true)
                .build()
                .map_err(|e| CompileError {
                    provider: provider.to_string(),
                    pattern: p.clone(),
                    source: e,
                })
        })
        .collect()
}

/// Map of compiled signatures keyed by provider id.
///
/// Missing providers (or providers with empty signature lists) are treated
/// as "no signatures configured" and their stderr classifies as
/// [`ErrorKind::Unknown`] -- fail-safe default.
pub type ProviderSignatureMap = HashMap<String, CompiledSignatures>;

/// Build a signature map from the raw config list. Invalid regexes surface
/// as `CompileError` so misconfiguration fails loud at startup rather than
/// silently disabling classification at runtime.
pub fn build_signature_map(
    configs: &[crate::provider_budget::ProviderBudgetConfig],
) -> Result<ProviderSignatureMap, CompileError> {
    let mut map = HashMap::new();
    for cfg in configs {
        if let Some(sigs) = cfg.error_signatures.as_ref() {
            let compiled = CompiledSignatures::compile(&cfg.id, sigs)?;
            if !compiled.is_empty() {
                map.insert(cfg.id.clone(), compiled);
            }
        }
    }
    Ok(map)
}

/// Classify a stderr snippet against the provider's compiled signatures.
///
/// Throttle is checked first so a message matching both lists (e.g.
/// "timeout waiting for rate limit reset") is treated as a throttle.
/// Returns [`ErrorKind::Unknown`] when no pattern matches or when the
/// provider has no signatures configured (`None`).
pub fn classify(stderr: &str, signatures: Option<&CompiledSignatures>) -> ErrorKind {
    let Some(sigs) = signatures else {
        return ErrorKind::Unknown;
    };
    if sigs.throttle.iter().any(|re| re.is_match(stderr)) {
        return ErrorKind::Throttle;
    }
    if sigs.flake.iter().any(|re| re.is_match(stderr)) {
        return ErrorKind::Flake;
    }
    ErrorKind::Unknown
}

/// Classify a list of stderr lines by joining them (newline-separated)
/// and running [`classify`]. Convenience wrapper for the spawn-exit path
/// where stderr is captured line-by-line.
pub fn classify_lines(lines: &[String], signatures: Option<&CompiledSignatures>) -> ErrorKind {
    if lines.is_empty() {
        return ErrorKind::Unknown;
    }
    let joined = lines.join("\n");
    classify(&joined, signatures)
}

/// Build a stable dedupe key for an unknown-error escalation: provider
/// plus the leading 20 chars of the stderr (both-ends trimmed + lowercased).
/// The short prefix + lowercase normalisation means minor suffix variance
/// (trailing newlines, mixed case, extra detail) dedupes to one key so we
/// don't spam fleet-meta with duplicate issues for the same stderr shape.
pub fn unknown_dedupe_key(provider: &str, stderr: &str) -> String {
    let head: String = stderr
        .trim()
        .chars()
        .take(20)
        .collect::<String>()
        .to_lowercase();
    format!("{}::{}", provider, head)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider_budget::ProviderBudgetConfig;

    fn sigs(throttle: &[&str], flake: &[&str]) -> ProviderErrorSignatures {
        ProviderErrorSignatures {
            throttle: throttle.iter().map(|s| s.to_string()).collect(),
            flake: flake.iter().map(|s| s.to_string()).collect(),
        }
    }

    #[test]
    fn unknown_when_no_signatures() {
        assert_eq!(classify("anything", None), ErrorKind::Unknown);
    }

    #[test]
    fn throttle_matches_case_insensitive() {
        let compiled =
            CompiledSignatures::compile("zai-coding-plan", &sigs(&["insufficient.?balance"], &[]))
                .unwrap();
        assert_eq!(
            classify("ERROR: Insufficient Balance on account", Some(&compiled)),
            ErrorKind::Throttle
        );
    }

    #[test]
    fn flake_matches_timeout() {
        let compiled =
            CompiledSignatures::compile("claude-code", &sigs(&[], &["timeout", "EOF"])).unwrap();
        assert_eq!(
            classify("stream timeout after 60s", Some(&compiled)),
            ErrorKind::Flake
        );
    }

    #[test]
    fn throttle_beats_flake_when_both_match() {
        let compiled = CompiledSignatures::compile(
            "claude-code",
            &sigs(&["rate.?limit"], &["rate.?limit.*timeout"]),
        )
        .unwrap();
        // Both lists match, throttle wins.
        assert_eq!(
            classify("rate limit timeout", Some(&compiled)),
            ErrorKind::Throttle
        );
    }

    #[test]
    fn unknown_when_no_pattern_matches() {
        let compiled =
            CompiledSignatures::compile("claude-code", &sigs(&["429"], &["timeout"])).unwrap();
        assert_eq!(
            classify("panic: internal assertion failed", Some(&compiled)),
            ErrorKind::Unknown
        );
    }

    #[test]
    fn classify_lines_joins_and_matches() {
        let compiled =
            CompiledSignatures::compile("kimi-for-coding", &sigs(&["quota"], &["EOF"])).unwrap();
        let lines = vec![
            "starting run".to_string(),
            "error: daily quota exceeded".to_string(),
        ];
        assert_eq!(classify_lines(&lines, Some(&compiled)), ErrorKind::Throttle);
    }

    #[test]
    fn classify_lines_empty_is_unknown() {
        let compiled =
            CompiledSignatures::compile("kimi-for-coding", &sigs(&["quota"], &[])).unwrap();
        assert_eq!(classify_lines(&[], Some(&compiled)), ErrorKind::Unknown);
    }

    #[test]
    fn compile_error_wraps_regex_error() {
        let err = CompiledSignatures::compile(
            "bad",
            &sigs(&["[unterminated"], &[]),
        )
        .unwrap_err();
        assert_eq!(err.provider, "bad");
        assert_eq!(err.pattern, "[unterminated");
    }

    #[test]
    fn build_signature_map_skips_providers_without_signatures() {
        let configs = vec![
            ProviderBudgetConfig {
                id: "opencode-go".to_string(),
                error_signatures: Some(sigs(&["429"], &["timeout"])),
                ..Default::default()
            },
            ProviderBudgetConfig {
                id: "no-sigs".to_string(),
                error_signatures: None,
                ..Default::default()
            },
            ProviderBudgetConfig {
                id: "empty-sigs".to_string(),
                error_signatures: Some(ProviderErrorSignatures::default()),
                ..Default::default()
            },
        ];
        let map = build_signature_map(&configs).unwrap();
        assert!(map.contains_key("opencode-go"));
        assert!(!map.contains_key("no-sigs"));
        assert!(!map.contains_key("empty-sigs"));
    }

    #[test]
    fn dedupe_key_is_stable_and_lowercase() {
        let k1 = unknown_dedupe_key("claude-code", "   Unexpected JSON token\n");
        let k2 = unknown_dedupe_key("claude-code", "UNEXPECTED JSON token with extra suffix");
        assert_eq!(k1, k2);
        assert!(k1.starts_with("claude-code::"));
    }

    #[test]
    fn display_matches_expected_tokens() {
        assert_eq!(ErrorKind::Throttle.to_string(), "throttle");
        assert_eq!(ErrorKind::Flake.to_string(), "flake");
        assert_eq!(ErrorKind::Unknown.to_string(), "unknown");
    }
}
