//! Indefinite-article (`a`/`an`) fix-up for a replacement.
//!
//! When a KG term is replaced by one of its synonyms, an `a`/`an` that
//! *immediately* precedes the term (only whitespace between them, at most one
//! line break) is switched to match the new text. The article is derived
//! purely from the new text, so cycling back restores the original article:
//! the rule is reversible by construction. Nothing else is touched.
//!
//! The vowel-sound test is a heuristic: a leading vowel letter means "an",
//! with small exception lists for words that start with a "you"/"w" sound
//! ("a unicorn", "a one-off") or a silent "h" ("an hour"). Text whose first
//! meaningful character is not a letter (digits, symbols) leaves the article
//! alone.
//!
//! The rules and word lists are copied verbatim from terraphim-editor's
//! `terraphim_alternatives::article` module (R-2.6), so the editor can
//! delegate to this crate without a change in behaviour. Keep the two in step.

use serde::{Deserialize, Serialize};

/// An English indefinite article.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Article {
    /// "a"
    A,
    /// "an"
    An,
}

/// The article the given text should take, or `None` when the first
/// meaningful character is not a letter.
///
/// Leading Markdown emphasis and opening quotes/brackets are skipped.
///
/// ```
/// use terraphim_lsp_core::{Article, article_for};
/// assert_eq!(article_for("eraser"), Some(Article::An));
/// assert_eq!(article_for("*unicorn*"), Some(Article::A));
/// assert_eq!(article_for("hour"), Some(Article::An));
/// assert_eq!(article_for("8-bit"), None);
/// ```
pub fn article_for(text: &str) -> Option<Article> {
    let trimmed = text.trim_start_matches(|c: char| {
        matches!(
            c,
            '*' | '_' | '"' | '\'' | '(' | '[' | '\u{201C}' | '\u{2018}'
        )
    });
    let first = trimmed.chars().next()?;
    if !first.is_alphabetic() {
        return None;
    }
    let word: String = trimmed
        .chars()
        .take_while(|c| c.is_alphabetic())
        .flat_map(char::to_lowercase)
        .collect();
    Some(if wants_an(&word) {
        Article::An
    } else {
        Article::A
    })
}

/// Words starting with a vowel letter that still take "a".
const A_PREFIXES: &[&str] = &[
    "eu", "ewe", "ubiq", "ufo", "unanim", "uni", "uran", "uri", "use", "usu", "uten", "uti", "utop",
];
/// Exact words starting with a vowel letter that take "a".
const A_WORDS: &[&str] = &["one", "once"];
/// Prefixes that override `A_PREFIXES` back to "an" ("an unidentified").
const AN_OVERRIDES: &[&str] = &["unid", "unim", "unin"];
/// Silent-h prefixes that take "an".
const SILENT_H: &[&str] = &["heir", "honest", "honor", "honour", "hour"];

fn wants_an(word: &str) -> bool {
    let starts = |list: &[&str]| list.iter().any(|prefix| word.starts_with(prefix));
    if starts(AN_OVERRIDES) || starts(SILENT_H) {
        return true;
    }
    if starts(A_PREFIXES) || A_WORDS.contains(&word) {
        return false;
    }
    matches!(word.chars().next(), Some('a' | 'e' | 'i' | 'o' | 'u'))
}

/// Byte range of an article immediately before `span_start` in `text`.
pub(crate) fn preceding_article(text: &str, span_start: usize) -> Option<(usize, usize)> {
    let before = &text[..span_start];
    let article_end = before.trim_end_matches(char::is_whitespace).len();
    let gap = &before[article_end..];
    if gap.is_empty() || gap.matches('\n').count() > 1 {
        return None;
    }
    let head = &before[..article_end];
    let article_start = head
        .char_indices()
        .rev()
        .find(|(_, c)| !c.is_alphanumeric())
        .map_or(0, |(index, c)| index + c.len_utf8());
    matches!(&head[article_start..], "a" | "an" | "A" | "An" | "AN")
        .then_some((article_start, article_end))
}

/// Spells `wanted` in the case style of `existing` ("A" -> "An", "AN" -> "A").
pub(crate) fn respell(existing: &str, wanted: Article) -> &'static str {
    let upper = existing.starts_with(|c: char| c.is_uppercase());
    let shouting = existing.len() > 1 && existing.chars().all(char::is_uppercase);
    match (wanted, upper, shouting) {
        (Article::A, true, _) => "A",
        (Article::A, false, _) => "a",
        (Article::An, true, true) => "AN",
        (Article::An, true, false) => "An",
        (Article::An, false, _) => "an",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vowel_letters_take_an() {
        for word in [
            "eraser",
            "apple",
            "idea",
            "orange",
            "umbrella",
            "unidentified",
        ] {
            assert_eq!(article_for(word), Some(Article::An), "{word}");
        }
        for word in ["rubber", "choice", "decision"] {
            assert_eq!(article_for(word), Some(Article::A), "{word}");
        }
    }

    #[test]
    fn exceptions_follow_the_sound() {
        for word in ["unicorn", "one-off", "euro", "user", "utility"] {
            assert_eq!(article_for(word), Some(Article::A), "{word}");
        }
        for word in ["hour", "honest", "heir"] {
            assert_eq!(article_for(word), Some(Article::An), "{word}");
        }
    }

    #[test]
    fn markup_is_skipped_and_non_letters_leave_the_article_alone() {
        assert_eq!(article_for("**apple**"), Some(Article::An));
        assert_eq!(article_for("\u{201C}idea\u{201D}"), Some(Article::An));
        assert_eq!(article_for("8-bit"), None);
        assert_eq!(article_for(""), None);
        assert_eq!(article_for("**"), None);
    }

    #[test]
    fn finds_only_an_immediately_preceding_article() {
        let text = "I want a  thing";
        assert_eq!(preceding_article(text, 10), Some((7, 8)));
        assert_eq!(preceding_article(text, 2), None, "no article before 'want'");
        assert_eq!(
            preceding_article("banana split", 7),
            None,
            "word ending in a"
        );
        assert_eq!(
            preceding_article("An\nidea", 3),
            Some((0, 2)),
            "one line break"
        );
        assert_eq!(
            preceding_article("a\n\nidea", 3),
            None,
            "blank line between"
        );
        assert_eq!(preceding_article("aidea", 1), None, "no whitespace");
        assert_eq!(
            preceding_article("(a idea", 3),
            Some((1, 2)),
            "after punctuation"
        );
    }

    #[test]
    fn respell_keeps_case_style() {
        assert_eq!(respell("a", Article::An), "an");
        assert_eq!(respell("A", Article::An), "An");
        assert_eq!(respell("An", Article::A), "A");
        assert_eq!(respell("an", Article::A), "a");
        assert_eq!(respell("AN", Article::An), "AN");
        assert_eq!(respell("AN", Article::A), "A");
    }
}
