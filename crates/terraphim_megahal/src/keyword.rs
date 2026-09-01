//! Keyword extraction -- a faithful port of `lib/megahal/keyword.rb`
//! (upstream MegaHAL gem, Unlicense).
//!
//! `extract` removes banned and digit-leading words, swaps antonyms ("I"
//! <-> "YOU"), and de-duplicates preserving first-occurrence order. It exists
//! purely to emulate the original MegaHAL's behaviour; upstream itself notes
//! keywords would be better learned from question-answer pairs.

use crate::keyword_data::{ANTONYMS, AUXILIARY, BANNED, GREETING};

/// Antonym lookup table built from [`crate::keyword_data::ANTONYMS`] (both
/// directions, mirroring `SWAP = Hash[ANTONYMS + ANTONYMS.map(&:reverse)]`).
///
/// Ruby semantics: building the hash from the concatenated list means the
/// *last* pair whose key is `word` wins (later assignments override earlier
/// ones). For example both ("YOU", "I") and ("YOU", "ME") occur, and "ME"
/// wins because it appears later.
fn swap(word: &str) -> Option<&'static str> {
    let mut result = None;
    for (a, b) in ANTONYMS.iter() {
        if *a == word {
            result = Some(*b);
        }
    }
    for (b, a) in ANTONYMS.iter() {
        if *a == word {
            result = Some(*b);
        }
    }
    result
}

/// Mirror of `MegaHAL.extract`: `None` input yields the greeting keywords;
/// otherwise banned/digit-leading words are removed, antonyms swapped and the
/// result de-duplicated in first-occurrence order.
pub fn extract(words: Option<&[String]>) -> Vec<String> {
    let Some(words) = words else {
        return GREETING.iter().map(|w| (*w).to_string()).collect();
    };
    let mut out: Vec<String> = Vec::with_capacity(words.len());
    for word in words {
        if word.starts_with(|c: char| c.is_ascii_digit()) {
            continue;
        }
        if BANNED.contains(&word.as_str()) {
            continue;
        }
        let effective = swap(word)
            .map(str::to_string)
            .unwrap_or_else(|| word.clone());
        if !out.contains(&effective) {
            out.push(effective);
        }
    }
    out
}

/// Whether `word` is an auxiliary word (`_select_keyword` shuffles these out
/// of the keyword list before choosing one at random).
pub fn is_auxiliary(word: &str) -> bool {
    AUXILIARY.contains(&word)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn words(items: &[&str]) -> Vec<String> {
        items.iter().map(|s| (*s).to_string()).collect()
    }

    #[test]
    fn extract_nil_yields_greetings() {
        assert_eq!(extract(None), words(&GREETING));
    }

    #[test]
    fn extract_removes_banned_and_digits() {
        let got = extract(Some(&words(&["THE", "HELLO", "123ABC", "WORLD"])));
        assert_eq!(got, words(&["HELLO", "WORLD"]));
    }

    #[test]
    fn extract_swaps_antonyms_both_ways() {
        assert_eq!(
            extract(Some(&words(&["I", "LOVE", "RUST"]))),
            words(&["YOU", "HATE", "RUST"])
        );
        // Duplicate antonym keys: Ruby's Hash construction makes the later
        // ("YOU", "ME") pair win over ("YOU", "I").
        assert_eq!(extract(Some(&words(&["YOU"]))), words(&["ME"]));
    }

    #[test]
    fn extract_dedupes_preserving_order() {
        let got = extract(Some(&words(&["RUST", "WASM", "RUST"])));
        assert_eq!(got, words(&["RUST", "WASM"]));
    }

    #[test]
    fn auxiliary_membership() {
        assert!(is_auxiliary("MYSELF"));
        assert!(is_auxiliary("YOU"));
        assert!(!is_auxiliary("RUST"));
    }
}
