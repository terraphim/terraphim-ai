//! Carrying the capitalisation of the original text over to a replacement.
//!
//! Thesaurus keys are lowercase, so a synonym arrives as `"judgment"` even
//! when the text said `"Choice"` at the start of a sentence. The concept name
//! may arrive in its display form (`"LLM"`), which must survive. The rule is
//! therefore one-directional: capitalisation is only ever *added* to match
//! the original, never removed.

use serde::{Deserialize, Serialize};

/// The capitalisation pattern of a piece of text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Capitalisation {
    /// Every cased letter is lowercase (`"choice"`). Replacements pass
    /// through unchanged.
    Lower,
    /// The first letter is uppercase (`"Choice"`, `"Paper clip"`). The
    /// replacement's first letter is uppercased.
    Sentence,
    /// Several words, each starting with an uppercase letter
    /// (`"Paper Clip"`). Each word of the replacement is capitalised.
    Title,
    /// Two or more letters, all uppercase (`"CHOICE"`). The whole replacement
    /// is uppercased.
    Upper,
    /// Anything else, including text with no cased letters (`"iPhone"`,
    /// `"42"`). Replacements pass through unchanged.
    Other,
}

impl Capitalisation {
    /// Classify `text`.
    ///
    /// ```
    /// use terraphim_lsp_core::Capitalisation;
    /// assert_eq!(Capitalisation::of("choice"), Capitalisation::Lower);
    /// assert_eq!(Capitalisation::of("Choice"), Capitalisation::Sentence);
    /// assert_eq!(Capitalisation::of("Paper Clip"), Capitalisation::Title);
    /// assert_eq!(Capitalisation::of("CHOICE"), Capitalisation::Upper);
    /// assert_eq!(Capitalisation::of("iPhone"), Capitalisation::Other);
    /// ```
    pub fn of(text: &str) -> Self {
        let mut cased = text
            .chars()
            .filter(|c| c.is_uppercase() || c.is_lowercase());
        let Some(first) = cased.next() else {
            return Self::Other;
        };
        let rest: Vec<char> = cased.collect();
        if first.is_lowercase() {
            return if rest.iter().all(|c| c.is_lowercase()) {
                Self::Lower
            } else {
                Self::Other
            };
        }
        if !rest.is_empty() && rest.iter().all(|c| c.is_uppercase()) {
            return Self::Upper;
        }
        let mut words = text.split_whitespace().peekable();
        let first_word = words.next();
        let is_title = words.peek().is_some()
            && first_word.into_iter().chain(words).all(|word| {
                word.chars()
                    .find(|c| c.is_alphabetic())
                    .is_none_or(char::is_uppercase)
            });
        if is_title {
            Self::Title
        } else {
            Self::Sentence
        }
    }

    /// Apply this capitalisation to `replacement`, only ever adding capitals.
    ///
    /// ```
    /// use terraphim_lsp_core::Capitalisation;
    /// assert_eq!(Capitalisation::Sentence.apply("judgment"), "Judgment");
    /// assert_eq!(Capitalisation::Title.apply("paper clip"), "Paper Clip");
    /// assert_eq!(Capitalisation::Upper.apply("trade-off"), "TRADE-OFF");
    /// assert_eq!(Capitalisation::Lower.apply("LLM"), "LLM");
    /// ```
    pub fn apply(self, replacement: &str) -> String {
        match self {
            Self::Lower | Self::Other => replacement.to_string(),
            Self::Upper => replacement.to_uppercase(),
            Self::Sentence => capitalise_words(replacement, false),
            Self::Title => capitalise_words(replacement, true),
        }
    }
}

/// Uppercase the first letter of the first word, or of every word when
/// `every_word` is set. Characters before the first letter of a word
/// (quotes, emphasis markers) are kept.
fn capitalise_words(text: &str, every_word: bool) -> String {
    let mut out = String::with_capacity(text.len());
    let mut awaiting_letter = true;
    for ch in text.chars() {
        if ch.is_whitespace() {
            awaiting_letter |= every_word;
            out.push(ch);
        } else if awaiting_letter && ch.is_alphabetic() {
            out.extend(ch.to_uppercase());
            awaiting_letter = false;
        } else {
            out.push(ch);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_common_patterns() {
        assert_eq!(Capitalisation::of("paper clip"), Capitalisation::Lower);
        assert_eq!(Capitalisation::of("Paper clip"), Capitalisation::Sentence);
        assert_eq!(Capitalisation::of("Paper Clip"), Capitalisation::Title);
        assert_eq!(Capitalisation::of("PAPER CLIP"), Capitalisation::Upper);
        assert_eq!(Capitalisation::of("Café"), Capitalisation::Sentence);
        assert_eq!(Capitalisation::of("ÉCOLE"), Capitalisation::Upper);
        assert_eq!(Capitalisation::of("8-bit"), Capitalisation::Lower);
        assert_eq!(Capitalisation::of("42"), Capitalisation::Other);
    }

    #[test]
    fn a_single_capital_letter_is_sentence_case() {
        assert_eq!(Capitalisation::of("A"), Capitalisation::Sentence);
        assert_eq!(Capitalisation::of("I"), Capitalisation::Sentence);
    }

    #[test]
    fn display_forms_are_never_lowered() {
        for case in [
            Capitalisation::Lower,
            Capitalisation::Sentence,
            Capitalisation::Title,
            Capitalisation::Other,
        ] {
            assert_eq!(case.apply("LLM"), "LLM", "{case:?}");
        }
    }

    #[test]
    fn capitalising_skips_leading_marks_and_handles_non_ascii() {
        assert_eq!(
            Capitalisation::Sentence.apply("\"élan\" vital"),
            "\"Élan\" vital"
        );
        assert_eq!(Capitalisation::Title.apply("coffee  shop"), "Coffee  Shop");
        assert_eq!(Capitalisation::Upper.apply("straße"), "STRASSE");
    }
}
