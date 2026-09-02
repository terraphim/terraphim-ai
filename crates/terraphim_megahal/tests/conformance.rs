//! Ruby-oracle conformance suite: replays `fixtures/megahal_fixtures.json`
//! against the Rust port.
//!
//! The fixtures are captured by `scripts/generate_megahal_fixtures.rb`,
//! which drives the real upstream MegaHAL logic (vendored gem sources,
//! Unlicense) with the canonical RNG contract documented in the crate root:
//! PCG32 (pinned by `rng_self_check`), `rand(n) = next_u32 % n`, and a
//! descending Fisher-Yates `shuffle`. Any divergence between the Rust
//! engine and these golden replies is a bug in the port until the fixture
//! is regenerated with provenance.

use rand_core::{Rng, SeedableRng};
use serde::Deserialize;
use terraphim_megahal::MegaHal;
use terraphim_sooth::DefaultRng;

#[derive(Deserialize)]
struct Fixtures {
    #[allow(dead_code)]
    source: String,
    rng_self_check: Vec<RngSeed>,
    scenarios: Vec<Scenario>,
}

#[derive(Deserialize)]
struct RngSeed {
    seed: u64,
    first_values: Vec<u32>,
}

#[derive(Deserialize)]
struct Scenario {
    name: String,
    seed: u64,
    #[serde(default)]
    blank: bool,
    #[serde(default)]
    personality: Option<String>,
    learning: bool,
    #[serde(default)]
    train_lines: Vec<String>,
    conversation: Vec<Option<String>>,
    replies: Vec<String>,
}

fn fixtures() -> Fixtures {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/fixtures/megahal_fixtures.json"
    );
    let text = std::fs::read_to_string(path).expect("fixtures file readable");
    serde_json::from_str(&text).expect("fixtures JSON well-formed")
}

/// Pins the Ruby PCG32 mirror against `rand_pcg::Pcg32` exactly as seeded by
/// `rand_core::SeedableRng::seed_from_u64`. If this fails, the fixture
/// generator's RNG diverged from the Rust side and every reply fixture is
/// suspect -- fix the mirror and regenerate before touching the engine.
#[test]
fn rng_self_check_pins_the_canonical_rng() {
    let fixtures = fixtures();
    assert!(fixtures.rng_self_check.len() >= 3, "expected several seeds");
    for seed in &fixtures.rng_self_check {
        let mut rng = DefaultRng::seed_from_u64(seed.seed);
        for (index, expected) in seed.first_values.iter().enumerate() {
            assert_eq!(
                rng.next_u32(),
                *expected,
                "seed {} word {index}: Ruby PCG32 mirror diverged from rand_pcg",
                seed.seed
            );
        }
    }
}

#[test]
fn conformance_scenarios_replay_exactly() {
    let fixtures = fixtures();
    assert!(fixtures.scenarios.len() >= 5, "expected several scenarios");
    let mut skipped = 0_usize;
    for scenario in &fixtures.scenarios {
        let needs_extra_personality =
            matches!(scenario.personality.as_deref(), Some(p) if p != "default");
        if needs_extra_personality && cfg!(not(feature = "personalities")) {
            skipped += 1;
            continue;
        }
        let mut rng = DefaultRng::seed_from_u64(scenario.seed);
        let mut hal = if scenario.blank {
            MegaHal::blank()
        } else {
            MegaHal::new()
        };
        if let Some(personality) = &scenario.personality
            && personality != "default"
        {
            hal.load_personality(personality)
                .expect("personality embedded");
        }
        hal.set_learning(scenario.learning);
        for line in &scenario.train_lines {
            hal.learn(line);
        }

        for (input, expected) in scenario.conversation.iter().zip(&scenario.replies) {
            let reply = hal.reply(input.as_deref(), &mut rng);
            assert_eq!(
                reply, *expected,
                "scenario `{}` diverged on input {input:?}",
                scenario.name
            );
        }

        // Replay determinism: a fresh engine with the same seed produces the
        // identical conversation again.
        let mut replay_rng = DefaultRng::seed_from_u64(scenario.seed);
        let mut replay = if scenario.blank {
            MegaHal::blank()
        } else {
            MegaHal::new()
        };
        if let Some(personality) = &scenario.personality
            && personality != "default"
        {
            replay
                .load_personality(personality)
                .expect("personality embedded");
        }
        replay.set_learning(scenario.learning);
        for line in &scenario.train_lines {
            replay.learn(line);
        }
        for (input, expected) in scenario.conversation.iter().zip(&scenario.replies) {
            assert_eq!(replay.reply(input.as_deref(), &mut replay_rng), *expected);
        }
    }
    assert_eq!(
        skipped,
        if cfg!(feature = "personalities") {
            0
        } else {
            1
        },
        "expected exactly the personality-gated scenario to be skipped"
    );
}

/// The engine must never panic and never emit an empty reply, whatever the
/// input (adversarial fuzz-ish sweep with real personalities, no mocks).
#[test]
fn reply_is_robust_for_adversarial_inputs() {
    let long_input = "x".repeat(2000);
    let inputs: Vec<Option<&str>> = vec![
        None,
        Some(""),
        Some(" "),
        Some("!!!"),
        Some("a"),
        Some("1234567890"),
        Some("don't stop believing"),
        Some("supercalifragilisticexpialidocious"),
        Some("\t mixed \n case INPUT ??"),
        Some(long_input.as_str()),
        Some("日本語のテキスト"),
        Some("INSERT 'quotes' AND --dashes-- AND \"double quotes\""),
    ];
    let mut rng = DefaultRng::seed_from_u64(1);
    let mut hal = MegaHal::new();
    for input in &inputs {
        let reply = hal.reply(*input, &mut rng);
        assert!(!reply.is_empty(), "reply to {input:?} was empty");
    }
}
