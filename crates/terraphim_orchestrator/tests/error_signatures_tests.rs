//! Integration tests for per-provider error-signature classification
//! (Refs terraphim/adf-fleet#7).
//!
//! Each fixture under `tests/fixtures/stderr/` captures a realistic
//! stderr snippet from one of the subscription-only providers we support:
//! claude-code, opencode-go, zai-coding-plan, kimi-for-coding.
//!
//! These tests exercise the real classifier with the regex lists an
//! operator would ship in `orchestrator.toml`. No mocks.

use std::fs;
use std::path::{Path, PathBuf};

use terraphim_orchestrator::error_signatures::{
    self, CompiledSignatures, ErrorKind, ProviderErrorSignatures,
};
use terraphim_orchestrator::provider_budget::ProviderBudgetConfig;

fn fixture_path(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("stderr")
        .join(name)
}

fn read_fixture(name: &str) -> String {
    fs::read_to_string(fixture_path(name))
        .unwrap_or_else(|e| panic!("fixture `{}` must exist: {}", name, e))
}

/// The canonical set of regex lists we'd ship in `orchestrator.toml`.
/// Kept as a single source of truth so fixture expectations and the
/// realistic config stay in lock-step.
fn canonical_sigs(provider: &str) -> ProviderErrorSignatures {
    match provider {
        "claude-code" => ProviderErrorSignatures {
            throttle: vec![
                "rate.?limit".into(),
                "429".into(),
                "usage limit".into(),
                "rate_limit_error".into(),
            ],
            flake: vec![
                "timed out".into(),
                "timeout".into(),
                "connection reset".into(),
                "eof".into(),
            ],
        },
        "opencode-go" => ProviderErrorSignatures {
            throttle: vec!["rate limit".into(), "429".into(), "quota".into()],
            flake: vec![
                "i/o timeout".into(),
                "connection reset".into(),
                "unexpected eof".into(),
            ],
        },
        "zai-coding-plan" => ProviderErrorSignatures {
            throttle: vec![
                "insufficient.?balance".into(),
                "quota".into(),
                "rate.?limit".into(),
            ],
            flake: vec![
                "read timeout".into(),
                "connection reset".into(),
                "session aborted due to timeout".into(),
            ],
        },
        "kimi-for-coding" => ProviderErrorSignatures {
            throttle: vec!["quota.?exceeded".into(), "quota exceeded".into()],
            flake: vec!["eof".into(), "closed connection".into()],
        },
        other => panic!("unexpected provider in test fixture: {}", other),
    }
}

fn compile(provider: &str) -> CompiledSignatures {
    CompiledSignatures::compile(provider, &canonical_sigs(provider))
        .expect("canonical test signatures must compile")
}

#[test]
fn claude_429_classifies_as_throttle() {
    let sigs = compile("claude-code");
    assert_eq!(
        error_signatures::classify(&read_fixture("claude_429.txt"), Some(&sigs)),
        ErrorKind::Throttle
    );
}

#[test]
fn claude_usage_limit_classifies_as_throttle() {
    let sigs = compile("claude-code");
    assert_eq!(
        error_signatures::classify(&read_fixture("claude_usage_limit.txt"), Some(&sigs)),
        ErrorKind::Throttle
    );
}

#[test]
fn claude_timeout_classifies_as_flake() {
    let sigs = compile("claude-code");
    assert_eq!(
        error_signatures::classify(&read_fixture("claude_timeout.txt"), Some(&sigs)),
        ErrorKind::Flake
    );
}

#[test]
fn opencode_go_rate_limit_classifies_as_throttle() {
    let sigs = compile("opencode-go");
    assert_eq!(
        error_signatures::classify(&read_fixture("opencode_go_rate_limit.txt"), Some(&sigs)),
        ErrorKind::Throttle
    );
}

#[test]
fn opencode_go_timeout_classifies_as_flake() {
    let sigs = compile("opencode-go");
    assert_eq!(
        error_signatures::classify(&read_fixture("opencode_go_timeout.txt"), Some(&sigs)),
        ErrorKind::Flake
    );
}

#[test]
fn zai_insufficient_balance_classifies_as_throttle() {
    let sigs = compile("zai-coding-plan");
    assert_eq!(
        error_signatures::classify(&read_fixture("zai_insufficient_balance.txt"), Some(&sigs)),
        ErrorKind::Throttle
    );
}

#[test]
fn zai_glm5_timeout_classifies_as_flake() {
    let sigs = compile("zai-coding-plan");
    assert_eq!(
        error_signatures::classify(&read_fixture("zai_glm5_timeout.txt"), Some(&sigs)),
        ErrorKind::Flake
    );
}

#[test]
fn kimi_quota_classifies_as_throttle() {
    let sigs = compile("kimi-for-coding");
    assert_eq!(
        error_signatures::classify(&read_fixture("kimi_quota.txt"), Some(&sigs)),
        ErrorKind::Throttle
    );
}

#[test]
fn kimi_eof_classifies_as_flake() {
    let sigs = compile("kimi-for-coding");
    assert_eq!(
        error_signatures::classify(&read_fixture("kimi_eof.txt"), Some(&sigs)),
        ErrorKind::Flake
    );
}

#[test]
fn unknown_stderr_classifies_as_unknown_across_all_providers() {
    // A panic trace the upstream CLIs never emit in practice must fall
    // through both lists for every configured provider so the
    // orchestrator escalates it for human review.
    let stderr = read_fixture("unknown_error.txt");
    for provider in [
        "claude-code",
        "opencode-go",
        "zai-coding-plan",
        "kimi-for-coding",
    ] {
        let sigs = compile(provider);
        assert_eq!(
            error_signatures::classify(&stderr, Some(&sigs)),
            ErrorKind::Unknown,
            "provider {} should not match panic fixture",
            provider
        );
    }
}

#[test]
fn build_signature_map_round_trips_canonical_config() {
    // Construct the same [[providers]] block an operator would ship and
    // round-trip it through the public build_signature_map API to
    // confirm compilation + lookup wiring.
    let configs: Vec<ProviderBudgetConfig> = [
        "claude-code",
        "opencode-go",
        "zai-coding-plan",
        "kimi-for-coding",
    ]
    .into_iter()
    .map(|id| ProviderBudgetConfig {
        id: id.to_string(),
        error_signatures: Some(canonical_sigs(id)),
        ..Default::default()
    })
    .collect();

    let map =
        error_signatures::build_signature_map(&configs).expect("canonical config must compile");
    assert_eq!(map.len(), 4);

    // Spot-check: each provider's fixture still classifies correctly
    // through the map-lookup path (what the orchestrator actually does).
    let cases = [
        ("claude-code", "claude_429.txt", ErrorKind::Throttle),
        ("claude-code", "claude_timeout.txt", ErrorKind::Flake),
        (
            "opencode-go",
            "opencode_go_rate_limit.txt",
            ErrorKind::Throttle,
        ),
        (
            "zai-coding-plan",
            "zai_insufficient_balance.txt",
            ErrorKind::Throttle,
        ),
        ("kimi-for-coding", "kimi_eof.txt", ErrorKind::Flake),
    ];
    for (provider, fixture, want) in cases {
        let sigs = map.get(provider);
        let got = error_signatures::classify(&read_fixture(fixture), sigs);
        assert_eq!(got, want, "{}:{}", provider, fixture);
    }
}

#[test]
fn missing_provider_in_map_classifies_as_unknown() {
    // A provider that isn't in the map (e.g. signatures accidentally
    // omitted from the operator's config) must fall back to Unknown
    // rather than silently treat the run as a throttle or a flake.
    let configs = vec![ProviderBudgetConfig {
        id: "claude-code".to_string(),
        error_signatures: Some(canonical_sigs("claude-code")),
        ..Default::default()
    }];
    let map = error_signatures::build_signature_map(&configs).unwrap();

    let stderr = read_fixture("zai_insufficient_balance.txt");
    let sigs = map.get("zai-coding-plan"); // never configured
    assert_eq!(
        error_signatures::classify(&stderr, sigs),
        ErrorKind::Unknown
    );
}

#[test]
fn throttle_beats_flake_when_stderr_contains_both() {
    // Real captured scenario: a 429 message that also mentions timeout
    // ("timeout waiting for rate-limit reset"). Must classify as
    // Throttle so we trip the breaker instead of retrying blindly.
    let mixed = "timeout waiting for rate-limit reset; retry-after 30s";
    let sigs = compile("claude-code");
    assert_eq!(
        error_signatures::classify(mixed, Some(&sigs)),
        ErrorKind::Throttle
    );
}

#[test]
fn dedupe_key_collapses_minor_shape_variance() {
    // Two stderr shapes from the same underlying failure -- trailing
    // newline + case variance + extra detail -- must hash to one key
    // so we don't open duplicate fleet-meta issues.
    let a = "  UPSTREAM CLOSED CONNECTION, EOF BEFORE COMPLETION\n";
    let b = "upstream closed connection, eof before completion (sess 3)";
    let ka = error_signatures::unknown_dedupe_key("kimi-for-coding", a);
    let kb = error_signatures::unknown_dedupe_key("kimi-for-coding", b);
    assert_eq!(ka, kb);
    assert!(ka.starts_with("kimi-for-coding::"));
}

#[test]
fn classify_lines_matches_live_stderr_capture_pattern() {
    // The orchestrator captures stderr line-by-line via
    // ManagedAgent::output_rx and feeds classify_lines directly.
    // Mirror that exact call shape here.
    let sigs = compile("opencode-go");
    let lines: Vec<String> = read_fixture("opencode_go_rate_limit.txt")
        .lines()
        .map(|s| s.to_string())
        .collect();
    assert!(lines.len() >= 3, "fixture must exercise multi-line join");
    assert_eq!(
        error_signatures::classify_lines(&lines, Some(&sigs)),
        ErrorKind::Throttle
    );
}
