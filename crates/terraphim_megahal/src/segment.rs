//! Sentence decomposition and segmentation -- a faithful port of MegaHAL's
//! `_decompose` / `_segment` (upstream `lib/megahal/megahal.rb`, Unlicense).
//!
//! A line is split into two parallel sequences: word separators (whitespace
//! and punctuation) and words. Normalised (upper-case) forms of the words are
//! derived by the caller. Words are runs of `[[:word:]]` characters
//! (alphanumeric plus underscore); apostrophes and hyphens between two word
//! fragments are merged into the word ("don't", "hob-goblin").
//!
//! Divergence from upstream: the CLD language-detection fallback (character
//! segmentation for Japanese, Korean, Chinese, Thai, Lao, Burmese, Khmer) is
//! replaced by a Unicode-range heuristic. English and other alphabetic
//! languages -- including every conformance fixture -- are unaffected.

/// Maximum input line length in characters; longer lines are treated as empty.
pub const MAXIMUM_LENGTH: usize = 1024;

/// A sentence decomposed into the three parallel sequences the five models
/// learn from: separators, normalised words and original words.
///
/// `None` models upstream's `nil` (a missing input, e.g. `reply(nil)` asking
/// for a greeting). Empty vectors model the empty string.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Decomposed {
    pub puncs: Option<Vec<String>>,
    pub norms: Option<Vec<String>>,
    pub words: Option<Vec<String>>,
}

/// Decompose a (already stripped) line, mirroring `_decompose`.
pub fn decompose(line: Option<&str>) -> Decomposed {
    let Some(line) = line else {
        return Decomposed {
            puncs: None,
            norms: None,
            words: None,
        };
    };
    if line.chars().count() > MAXIMUM_LENGTH {
        return decompose(Some(""));
    }
    if line.is_empty() {
        return Decomposed {
            puncs: Some(Vec::new()),
            norms: Some(Vec::new()),
            words: Some(Vec::new()),
        };
    }
    let (puncs, words) = segment(line);
    let norms = words.iter().map(|w| w.to_uppercase()).collect();
    Decomposed {
        puncs: Some(puncs),
        norms: Some(norms),
        words: Some(words),
    }
}

/// Upstream's `[[:word:]]`: alphanumeric or underscore.
fn is_word_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// Heuristic replacement for CLD: character segmentation for scripts that do
/// not delimit words with whitespace (CJK, Hangul, Thai, Lao, Burmese, Khmer).
fn character_segmentation(line: &str) -> bool {
    line.chars().any(|c| {
        matches!(
            c as u32,
            0x1100..=0x11FF      // Hangul jamo
                | 0x2E80..=0x9FFF // CJK radicals, kana, unified ideographs
                | 0xAC00..=0xD7AF // Hangul syllables
                | 0x0E00..=0x0E7F // Thai
                | 0x0E80..=0x0EFF // Lao
                | 0x1000..=0x109F // Myanmar
                | 0x1780..=0x17FF // Khmer
                | 0xF900..=0xFAFF // CJK compatibility ideographs
                | 0x20000..=0x2FA1F // CJK extensions B-F
        )
    })
}

/// Mirror of Ruby's `line.split(/([[:word:]]+)/)` (or the single-character
/// variant): alternating separator and word tokens.
///
/// Ruby's split keeps the (possibly empty) gap before the first match,
/// includes every interior gap, and drops only *trailing empty* gaps:
/// `"hello"` -> `["", "hello"]`, `"  spaced  out! "` ->
/// `["  ", "spaced", "  ", "out", "! "]`, `"!!!"` -> `["!!!"]`.
fn split_with_capture(line: &str, single_chars: bool) -> Vec<String> {
    let mut pairs: Vec<(String, String)> = Vec::new(); // (gap before word, word)
    let mut gap = String::new();
    let mut word = String::new();

    fn flush_word(gap: &mut String, word: &mut String, pairs: &mut Vec<(String, String)>) {
        if !word.is_empty() {
            pairs.push((std::mem::take(gap), std::mem::take(word)));
        }
    }

    for c in line.chars() {
        if is_word_char(c) {
            if single_chars {
                flush_word(&mut gap, &mut word, &mut pairs);
                pairs.push((std::mem::take(&mut gap), c.to_string()));
            } else {
                word.push(c);
            }
        } else {
            flush_word(&mut gap, &mut word, &mut pairs);
            gap.push(c);
        }
    }

    // The trailing gap is kept only when non-empty (Ruby drops trailing
    // empty strings after the final match).
    let trailing_gap = if !word.is_empty() {
        pairs.push((std::mem::take(&mut gap), std::mem::take(&mut word)));
        String::new()
    } else {
        std::mem::take(&mut gap)
    };

    let mut tokens: Vec<String> = Vec::with_capacity(pairs.len() * 2 + 1);
    for (g, w) in &pairs {
        tokens.push(g.clone());
        tokens.push(w.clone());
    }
    if !trailing_gap.is_empty() {
        tokens.push(trailing_gap);
    }
    tokens
}

/// Mirror of `_segment`: split into tokens, guard the boundaries so the
/// sequence starts and ends with a separator, merge `'`/`-` trigrams, then
/// partition into (separators, words).
pub fn segment(line: &str) -> (Vec<String>, Vec<String>) {
    let mut sequence = split_with_capture(line, character_segmentation(line));

    // Ensure the sequence starts with and ends with a separator. Ruby's
    // `/[[:word:]]+/ =~ token` is an unanchored match: any word character
    // anywhere in the token counts.
    let looks_like_word = |token: &str| token.chars().any(is_word_char);
    if sequence.last().map(|t| looks_like_word(t)).unwrap_or(false) {
        sequence.push(String::new());
    }
    if sequence
        .first()
        .map(|t| looks_like_word(t))
        .unwrap_or(false)
    {
        sequence.insert(0, String::new());
    }

    // Merge trigrams around a lone `'` or `-` separator: "don't" and
    // "hob-goblin" become single words. Ruby rescans from the left after
    // each merge.
    loop {
        let merge_at = (1..sequence.len().saturating_sub(1))
            .find(|&i| matches!(sequence[i].as_str(), "'" | "-"));
        let Some(i) = merge_at else { break };
        let merged = format!("{}{}{}", sequence[i - 1], sequence[i], sequence[i + 1]);
        sequence[i] = merged;
        sequence.remove(i + 1);
        sequence.remove(i - 1);
    }

    // Even indices are separators, odd indices are words.
    let mut puncs = Vec::new();
    let mut words = Vec::new();
    for (index, token) in sequence.into_iter().enumerate() {
        if index % 2 == 0 {
            puncs.push(token);
        } else {
            words.push(token);
        }
    }
    (puncs, words)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn word_list(line: &str) -> Vec<String> {
        segment(line).1
    }

    #[test]
    fn segment_plain_sentence() {
        let (puncs, words) = segment("Hello there, world!");
        assert_eq!(words, ["Hello", "there", "world"]);
        assert_eq!(
            puncs,
            ["", " ", ", ", "!"]
                .iter()
                .map(|s| s.to_string())
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn segment_merges_apostrophes_and_hyphens() {
        assert_eq!(word_list("don't"), ["don't"]);
        assert_eq!(
            word_list("hob-goblin don't x"),
            ["hob-goblin", "don't", "x"]
        );
    }

    #[test]
    fn segment_handles_boundaries() {
        assert_eq!(word_list(""), Vec::<String>::new());
        assert_eq!(word_list("!!!"), Vec::<String>::new());
        assert_eq!(word_list("hello"), ["hello"]);
        let (puncs, _) = segment("!!!");
        assert_eq!(puncs, ["!!!"]);
    }

    #[test]
    fn segment_preserves_inner_spacing() {
        let (puncs, words) = segment("  spaced  out! ");
        assert_eq!(words, ["spaced", "out"]);
        assert_eq!(puncs, ["  ", "  ", "! "]);
    }

    #[test]
    fn decompose_nil_empty_and_long() {
        assert_eq!(
            decompose(None),
            Decomposed {
                puncs: None,
                norms: None,
                words: None
            }
        );
        let empty = decompose(Some(""));
        assert_eq!(empty.words, Some(Vec::new()));
        assert_eq!(empty.norms, Some(Vec::new()));

        let long = "x".repeat(MAXIMUM_LENGTH + 1);
        let decomposed = decompose(Some(&long));
        assert_eq!(decomposed.words, Some(Vec::new()));

        let exactly_max = "x".repeat(MAXIMUM_LENGTH);
        assert_eq!(
            decompose(Some(&exactly_max)).words.unwrap()[0].len(),
            MAXIMUM_LENGTH
        );
    }

    #[test]
    fn decompose_produces_uppercase_norms() {
        let decomposed = decompose(Some("Hello World"));
        assert_eq!(
            decomposed.words,
            Some(vec!["Hello".to_string(), "World".to_string()])
        );
        assert_eq!(
            decomposed.norms,
            Some(vec!["HELLO".to_string(), "WORLD".to_string()])
        );
    }

    #[test]
    fn cjk_input_uses_character_segmentation() {
        assert!(character_segmentation("日本語のテキスト"));
        assert!(!character_segmentation("plain english"));
        let (_, words) = segment("日本語");
        assert_eq!(words, ["日", "本", "語"]);
    }
}
