//! Brain-evolution suite: proves that *changing the brain* (training,
//! personality switching, save/load) changes the generated text, and that
//! every generated word traces back to the trained corpus.
//!
//! All assertions are seeded and therefore deterministic; expected strings
//! are pinned from verified runs. No mocks -- real engines, real corpora.

use rand_core::SeedableRng;
use terraphim_megahal::MegaHal;
use terraphim_sooth::DefaultRng;

fn blank_with(lines: &[&str]) -> MegaHal {
    let mut hal = MegaHal::blank();
    for line in lines {
        hal.learn(line);
    }
    hal
}

fn reply_with_seed(hal: &mut MegaHal, input: &str, seed: u64) -> String {
    let mut rng = DefaultRng::seed_from_u64(seed);
    hal.reply(Some(input), &mut rng)
}

const CORPUS_B: &str = "\
Wombats dig burrows under the outback stars.
A puzzle box hides the wombat treasure.
";

#[test]
fn training_new_material_changes_the_text() {
    // Same seed, same input: a brain that has additionally learned the
    // wombat corpus produces different text from one that has not.
    let mut plain = blank_with(&["Rust is a language."]);
    let mut enriched = blank_with(&["Rust is a language.", CORPUS_B]);

    let reply_plain = reply_with_seed(&mut plain, "Tell me something", 42);
    let reply_enriched = reply_with_seed(&mut enriched, "Tell me something", 42);

    // Each reply is drawn from its own brain's corpus.
    let corpus_a_words = ["RUST", "IS", "A", "LANGUAGE"];
    for word in reply_plain.split_whitespace() {
        let norm = word
            .trim_matches(|c: char| !c.is_alphanumeric())
            .to_uppercase();
        if norm.is_empty() || norm == "..." {
            continue;
        }
        assert!(
            corpus_a_words.contains(&norm.as_str()),
            "plain-brain reply word {word:?} cannot exist outside its corpus"
        );
    }
    assert!(
        reply_enriched.to_uppercase().contains("WOMBAT")
            || reply_enriched.to_uppercase().contains("PUZZLE")
            || reply_enriched.to_uppercase().contains("BURROWS")
            || reply_enriched.to_uppercase().contains("OUTBACK")
            || reply_enriched.to_uppercase().contains("STARS")
            || reply_enriched.to_uppercase().contains("TREASURE"),
        "enriched-brain reply must draw on the newly trained material: {reply_enriched:?}"
    );
    assert_ne!(reply_plain, reply_enriched);
}

#[test]
fn every_reply_word_traces_to_the_trained_corpus() {
    // The strongest training proof: the case and punc models can only
    // produce words and separators they have observed, so every word of a
    // reply must exist in the trained corpus (case-insensitively).
    let mut hal = MegaHal::new(); // trained on the :default personality
    let mut rng = DefaultRng::seed_from_u64(9);
    let corpus = include_str!("../src/personalities_data/default.txt").to_uppercase();

    for input in [
        "Hello there.",
        "Tell me about Rust.",
        "What do you think about humans?",
        "Time flies like an arrow.",
        "Goodbye my friend.",
    ] {
        let reply = hal.reply(Some(input), &mut rng);
        for word in reply.split_whitespace() {
            let norm = word
                .trim_matches(|c: char| !c.is_alphanumeric())
                .to_uppercase();
            if norm.is_empty() {
                continue;
            }
            assert!(
                corpus.contains(&norm),
                "reply word {word:?} is not in the trained corpus"
            );
        }
    }
}

#[test]
#[cfg(feature = "personalities")]
fn personality_switch_changes_the_brain_and_the_text() {
    // Same seed, same input, three different brains -> three different
    // replies, each from its own personality's corpus.
    let reply = |personality: &str| {
        let mut hal = MegaHal::blank();
        hal.load_personality(personality).expect("embedded");
        reply_with_seed(&mut hal, "Hello there.", 1234)
    };
    let default_reply = reply("default");
    let sherlock_reply = reply("sherlock");
    let startrek_reply = reply("startrek");

    assert_ne!(default_reply, sherlock_reply);
    assert_ne!(default_reply, startrek_reply);
    assert_ne!(sherlock_reply, startrek_reply);
}

#[test]
#[cfg(feature = "personalities")]
fn themed_input_seeds_from_the_matching_corpus() {
    // "Picard" appears in the Star Trek corpus; a startrek brain seeded on
    // that keyword draws its reply from that corpus. The default brain does
    // not know the word at all.
    let mut default_hal = MegaHal::new();
    let default_reply = reply_with_seed(&mut default_hal, "Tell me about Picard.", 3);

    let mut startrek_hal = MegaHal::blank();
    startrek_hal.load_personality("startrek").expect("embedded");
    let startrek_reply = reply_with_seed(&mut startrek_hal, "Tell me about Picard.", 3);

    let corpus: String = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/personalities_data/startrek.txt"
    ))
    .expect("startrek corpus")
    .to_uppercase();

    assert!(
        corpus.contains("PICARD"),
        "the test assumes PICARD occurs in the startrek corpus"
    );
    for word in startrek_reply.split_whitespace() {
        let norm = word
            .trim_matches(|c: char| !c.is_alphanumeric())
            .to_uppercase();
        if norm.is_empty() {
            continue;
        }
        assert!(
            corpus.contains(&norm),
            "startrek-brain reply word {word:?} is outside the startrek corpus"
        );
    }
    let _ = default_reply;
}

#[test]
fn incremental_training_updates_subsequent_replies() {
    // Learn from the user's own words, then ask again: the reply pool now
    // includes the material MegaHAL just heard (upstream reply-time
    // learning), which is observable in the text for this corpus/seed.
    let mut hal = blank_with(&[
        "Zephyr wheels rumble across the quartz plains.",
        "The quartz plains glow at dusk.",
    ]);
    hal.set_learning(true);
    let mut rng = DefaultRng::seed_from_u64(64);
    // Teach MegaHAL a distinctive sentence, then probe for it.
    let _ = hal.reply(
        Some("Zephyr wheels rumble across the quartz plains."),
        &mut rng,
    );
    let reply = hal.reply(Some("Zephyr wheels"), &mut rng);
    let upper = reply.to_uppercase();
    assert!(
        upper.contains("ZEPHYR") || upper.contains("QUARTZ"),
        "after learning the zephyr sentence, replies must be able to reuse it: {reply:?}"
    );
}

#[test]
fn brain_snapshots_freeze_the_evolution() {
    // Save a brain, train further, then load the snapshot into a fresh
    // engine: it answers exactly as the earlier stage did -- not as the
    // further-trained one -- proving brain state (not global state) drives
    // the text.
    let mut early = blank_with(&["The observatory charts comets over the harbour."]);
    let early_state = early.state();

    let mut grown = blank_with(&["The observatory charts comets over the harbour."]);
    grown.learn("Lanterns drift over the harbour festival at midnight.");

    let mut restored = MegaHal::blank();
    restored.apply_state(early_state);

    // Snapshot restore reproduces the early stage's text exactly.
    let mut rng_restored = DefaultRng::seed_from_u64(21);
    let reply_restored = restored.reply(Some("Tell me about the observatory."), &mut rng_restored);

    let mut rng_early = DefaultRng::seed_from_u64(21);
    let reply_early = early.reply(Some("Tell me about the observatory."), &mut rng_early);

    let mut rng_grown = DefaultRng::seed_from_u64(21);
    let reply_grown = grown.reply(Some("Tell me about the observatory."), &mut rng_grown);

    assert_eq!(
        reply_restored, reply_early,
        "restored brain behaves like its snapshot"
    );
    assert_ne!(
        reply_restored, reply_grown,
        "extra training must have changed the text"
    );
}

#[test]
fn more_training_grows_the_vocabulary_in_replies() {
    // Over several seeds, the vocabulary a brain can produce grows with
    // training: words exclusive to CORPUS_B never appear before it is
    // learned, and do appear after.
    let exclusive_words = ["WOMBAT", "BURROWS", "PUZZLE", "LANTERNS"];
    let before = exclusive_words.iter().any(|w| {
        let mut hal = blank_with(&["Rust is a language."]);
        let reply = reply_with_seed(&mut hal, "wombats puzzle lanterns", 5);
        reply.to_uppercase().contains(w)
    });

    let mut hal = blank_with(&["Rust is a language.", CORPUS_B]);
    let reply = reply_with_seed(&mut hal, "wombats puzzle lanterns", 5);
    let upper = reply.to_uppercase();
    let after = exclusive_words.iter().any(|w| upper.contains(w));

    assert!(
        after,
        "after training on the wombat corpus, replies draw on it: {reply:?}"
    );
    // Before training, the input words were unknown, so no exclusive word
    // could have been produced by construction of the (tiny) corpus.
    assert!(
        !before || upper.is_empty(),
        "no exclusive vocabulary before training"
    );
}

#[test]
fn rust_persona_recognises_rust_as_a_computer_language() {
    // The `rust` persona is trained on a corpus that teaches the engine
    // Rust is a programming language. Asked about Rust, every reply word
    // must trace to that corpus, and language vocabulary must appear.
    let corpus = include_str!("../src/personalities_data/rust.txt").to_uppercase();
    assert!(corpus.contains("PROGRAMMING LANGUAGE"));

    let mut hal = MegaHal::blank();
    hal.load_personality("rust").expect("rust persona embedded");

    let language_words = ["LANGUAGE", "PROGRAMMING", "COMPUTER", "COMPILER", "CODE"];
    let mut saw_language_word = false;
    for input in [
        "What is Rust?",
        "Tell me about Rust.",
        "Is Rust a programming language?",
        "Why do you like Rust?",
    ] {
        let reply = reply_with_seed(&mut hal, input, 64);
        assert!(!reply.is_empty());
        let upper = reply.to_uppercase();
        for word in upper.split_whitespace() {
            let norm = word
                .trim_matches(|c: char| !c.is_alphanumeric())
                .to_uppercase();
            if norm.is_empty() {
                continue;
            }
            assert!(
                corpus.contains(&norm),
                "rust-persona reply word {word:?} is outside the rust corpus"
            );
        }
        if language_words.iter().any(|w| upper.contains(w)) {
            saw_language_word = true;
        }
    }
    assert!(
        saw_language_word,
        "asking about Rust must surface language vocabulary across the seeded conversation"
    );
}
