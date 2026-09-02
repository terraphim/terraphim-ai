//! Headless browser tests for the wasm API (`wasm-pack test --headless
//! --chrome`). They exercise the same surface the demo UI uses, including
//! the reply-latency measurement (target: < 50 ms per reply on a default
//! brain).

#![cfg(target_arch = "wasm32")]

use wasm_bindgen::JsValue;
use wasm_bindgen_test::*;

wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

use terraphim_megahal_wasm::MegahalBrain;

fn seed_brain() -> MegahalBrain {
    let mut brain = MegahalBrain::new("default");
    brain.learn("I love Rust and WebAssembly.");
    brain.learn("Rust is a systems programming language.");
    brain.learn("Markov chains are fun for chatting.");
    brain
}

#[wasm_bindgen_test]
fn personalities_are_listed_for_the_selector() {
    let personalities = MegahalBrain::list_personalities();
    assert!(personalities.len() >= 2);
    assert!(personalities.contains(&JsValue::from_str("default")));
    assert!(personalities.contains(&JsValue::from_str("sherlock")));
}

#[wasm_bindgen_test]
fn learn_and_reply_round_trip() {
    let mut brain = seed_brain();
    let reply = brain.reply("What about Rust?");
    assert!(!reply.is_empty(), "reply must not be empty");
    let greeting = brain.reply("");
    assert!(!greeting.is_empty(), "greeting must not be empty");
}

#[wasm_bindgen_test]
fn replies_are_deterministic_for_a_fixed_seed() {
    let draw = || {
        let mut brain = seed_brain();
        brain.set_seed(77);
        (
            brain.reply("Tell me about Rust."),
            brain.reply("And Markov chains?"),
        )
    };
    assert_eq!(draw(), draw());
}

#[wasm_bindgen_test]
fn reply_latency_on_a_default_brain_stays_under_budget() {
    let mut brain = MegahalBrain::new("default");
    // Warm the dictionaries with a few exchanges, then measure.
    let mut worst = 0.0_f64;
    for input in [
        "Hello there.",
        "What do you think about Rust?",
        "Tell me about Markov chains.",
        "Are you a human?",
        "Goodbye my friend.",
    ] {
        let _ = brain.reply(input);
        worst = worst.max(brain.last_reply_ms());
    }
    // Measured 2026-09-02 on macOS arm64 (Chrome headless): ~6 ms worst.
    // Documented in terraphim/terraphim-ai#3263.
    assert!(
        worst < 50.0,
        "worst reply latency {worst:.2} ms exceeded the 50 ms budget"
    );
}

#[wasm_bindgen_test]
fn save_load_round_trip_continues_the_conversation() {
    let mut original = seed_brain();
    let _ = original.reply("Tell me about Rust.");
    let saved = original.save();
    assert!(!saved.is_empty());

    let mut restored = MegahalBrain::new("default");
    restored.load(&saved).expect("own save format loads");
    assert_eq!(
        original.reply("And Markov chains?"),
        restored.reply("And Markov chains?"),
        "restored brain must continue the conversation identically"
    );
}

#[wasm_bindgen_test]
fn personality_switch_resets_the_brain() {
    let mut brain = MegahalBrain::new("default");
    let before = brain.save();
    brain.reset("sherlock", Some(7));
    let after = brain.save();
    assert_ne!(
        before, after,
        "switching personalities must change the brain"
    );
}
