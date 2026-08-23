//! Asserts `terraphim_sooth::Predictor` against golden fixtures captured
//! from the real upstream `sooth` Ruby gem
//! (<https://github.com/kranzky/sooth>, v2.3.2, Unlicense).
//!
//! See `fixtures/sooth_fixtures.json`'s `source` field (Ruby version, gem
//! version, capture command) and `scripts/generate_fixtures.rb` for
//! provenance and regeneration.
//!
//! Two layers of checks live in this file:
//!  1. [`matches_upstream_sooth_formulas`] drives the captured fixture
//!     values through the Rust port and verifies they match.
//!  2. [`fixture_file_is_internally_consistent`] *ignores* the captured
//!     values and recomputes `surprise` / `uncertainty` / `count` directly
//!     from each scenario's `observe_sequence`, then asserts the recomputed
//!     values match the captured ones to within a tight floating-point
//!     tolerance. This is the defence-in-depth oracle: even if the Ruby
//!     gem becomes unobtainable for a future maintainer, anyone with a
//!     Rust toolchain can confirm the JSON is internally consistent with
//!     the algorithm it claims to encode.

use std::collections::HashMap;

use serde::Deserialize;
use terraphim_sooth::Predictor;

#[derive(Debug, Deserialize)]
struct FixtureFile {
    scenarios: Vec<Scenario>,
}

#[derive(Debug, Deserialize)]
struct Scenario {
    name: String,
    observe_sequence: Vec<u32>,
    expected_observe_counts: Vec<u32>,
    count: u32,
    surprise: HashMap<String, f64>,
    uncertainty: f64,
}

const FIXTURE_JSON: &str = include_str!("../fixtures/sooth_fixtures.json");

/// Every scenario uses the same fixed context; the fixture format is
/// context-agnostic (see `scripts/generate_fixtures.rb`), so any distinct
/// context per scenario is equivalent -- a fresh `Predictor` is used per
/// scenario, so a single context works for all of them.
const CONTEXT: (u32, u32) = (1, 1);

#[test]
fn matches_upstream_sooth_formulas() {
    let fixtures: FixtureFile = serde_json::from_str(FIXTURE_JSON).expect("fixture JSON parses");
    assert!(
        !fixtures.scenarios.is_empty(),
        "fixture file must contain at least one scenario"
    );

    for scenario in &fixtures.scenarios {
        let mut predictor = Predictor::new();

        assert_eq!(
            scenario.observe_sequence.len(),
            scenario.expected_observe_counts.len(),
            "scenario '{}': observe_sequence/expected_observe_counts length mismatch",
            scenario.name,
        );

        for (symbol, &expected_count) in scenario
            .observe_sequence
            .iter()
            .zip(&scenario.expected_observe_counts)
        {
            let got = predictor.observe(CONTEXT, *symbol);
            assert_eq!(
                got, expected_count,
                "scenario '{}': observe({symbol}) returned {got}, expected {expected_count}",
                scenario.name,
            );
        }

        assert_eq!(
            predictor.count(CONTEXT),
            scenario.count,
            "scenario '{}': count mismatch",
            scenario.name,
        );

        for (symbol_str, &expected_surprise) in &scenario.surprise {
            let symbol: u32 = symbol_str.parse().expect("fixture symbol key is a u32");
            let got = predictor.surprise(CONTEXT, symbol).unwrap_or_else(|| {
                panic!("scenario '{}': surprise({symbol}) was None", scenario.name)
            });
            assert!(
                (got - expected_surprise).abs() < 1e-9,
                "scenario '{}': surprise({symbol}) = {got}, expected {expected_surprise}",
                scenario.name,
            );
        }

        let got_uncertainty = predictor
            .uncertainty(CONTEXT)
            .unwrap_or_else(|| panic!("scenario '{}': uncertainty was None", scenario.name));
        assert!(
            (got_uncertainty - scenario.uncertainty).abs() < 1e-9,
            "scenario '{}': uncertainty = {got_uncertainty}, expected {}",
            scenario.name,
            scenario.uncertainty,
        );
    }
}

/// Recomputes every captured value from the observation history alone,
/// without trusting the captured numbers. If this test ever fails while
/// `matches_upstream_sooth_formulas` still passes, the fixture file's
/// `surprise` / `uncertainty` / `count` values have drifted from the
/// `observe_sequence` and need regenerating.
///
/// This is the second-line defence for the Ruby-oracle conformance harness:
/// it doesn't *replace* the captured fixtures (those still come from
/// `gem install sooth`), but it guarantees the committed JSON is at least
/// self-consistent with the algorithm.
#[test]
fn fixture_file_is_internally_consistent() {
    let fixtures: FixtureFile = serde_json::from_str(FIXTURE_JSON).expect("fixture JSON parses");

    for scenario in &fixtures.scenarios {
        // Recompute `expected_observe_counts` and per-symbol totals directly
        // from the observation sequence.
        let mut per_symbol_counts: HashMap<u32, u32> = HashMap::new();
        let mut recomputed_counts = Vec::with_capacity(scenario.observe_sequence.len());
        for &symbol in &scenario.observe_sequence {
            let count = per_symbol_counts.entry(symbol).or_insert(0);
            *count += 1;
            recomputed_counts.push(*count);
        }
        let total: u32 = per_symbol_counts.values().sum();

        assert_eq!(
            recomputed_counts, scenario.expected_observe_counts,
            "scenario '{}': recomputed per-call observe counts diverged from the fixture",
            scenario.name,
        );
        assert_eq!(
            total, scenario.count,
            "scenario '{}': recomputed total = {total}, fixture count = {}",
            scenario.name, scenario.count,
        );

        // Recompute surprise for every symbol the fixture mentions. The
        // Ruby capture only emits `surprise` for symbols that were actually
        // observed in that context (unobserved symbols return `nil` and are
        // deliberately omitted), so recomputing against the captured map is
        // a complete check.
        for (symbol_str, &captured_surprise) in &scenario.surprise {
            let symbol: u32 = symbol_str.parse().unwrap_or_else(|_| {
                panic!(
                    "scenario '{}': symbol key '{symbol_str}' is not a u32",
                    scenario.name
                )
            });
            let count = per_symbol_counts[&symbol];
            assert!(
                count > 0,
                "scenario '{}': symbol {symbol} appears in `surprise` but was never observed",
                scenario.name,
            );
            let probability = f64::from(count) / f64::from(total);
            let expected_surprise = -probability.log2();
            assert!(
                (expected_surprise - captured_surprise).abs() < 1e-12,
                "scenario '{}': recomputed surprise({symbol}) = {expected_surprise}, fixture = {captured_surprise}",
                scenario.name,
            );
        }

        // Recompute Shannon entropy from the same per-symbol histogram.
        let recomputed_uncertainty = per_symbol_counts
            .values()
            .filter(|&&c| c > 0)
            .map(|&c| {
                let p = f64::from(c) / f64::from(total);
                -p * p.log2()
            })
            .sum::<f64>();
        assert!(
            (recomputed_uncertainty - scenario.uncertainty).abs() < 1e-12,
            "scenario '{}': recomputed uncertainty = {recomputed_uncertainty}, fixture = {}",
            scenario.name,
            scenario.uncertainty,
        );
    }
}

/// Sanity guard: the capture must cover at least five distinct contexts
/// (one per scenario) and at least one of those contexts must observe three
/// or more distinct symbols so `uncertainty > 0` is exercised. This is the
/// minimum-coverage contract documented in
/// `crates/terraphim_sooth/README.md`.
#[test]
fn fixture_file_meets_coverage_contract() {
    let fixtures: FixtureFile = serde_json::from_str(FIXTURE_JSON).expect("fixture JSON parses");

    assert!(
        fixtures.scenarios.len() >= 5,
        "fixture file has {} scenarios, expected >=5",
        fixtures.scenarios.len(),
    );

    let any_three_symbols = fixtures.scenarios.iter().any(|s| {
        let mut uniq = std::collections::BTreeSet::new();
        for &sym in &s.observe_sequence {
            uniq.insert(sym);
        }
        uniq.len() >= 3
    });
    assert!(
        any_three_symbols,
        "at least one scenario must observe >=3 distinct symbols so uncertainty > 0",
    );
}
