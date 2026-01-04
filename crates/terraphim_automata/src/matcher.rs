use aho_corasick::{AhoCorasick, MatchKind};
use terraphim_types::{NormalizedTerm, NormalizedTermValue, Thesaurus};

use crate::url_protector::UrlProtector;

use crate::{Result, TerraphimAutomataError};

#[derive(Debug, PartialEq, Clone)]
pub struct Matched {
    pub term: String,
    pub normalized_term: NormalizedTerm,
    pub pos: Option<(usize, usize)>,
}

/// Minimum pattern length for find_matches to prevent spurious matches.
const MIN_FIND_PATTERN_LENGTH: usize = 2;

pub fn find_matches(
    text: &str,
    thesaurus: Thesaurus,
    return_positions: bool,
) -> Result<Vec<Matched>> {
    // Filter out empty and too-short patterns
    let valid_patterns: Vec<(NormalizedTermValue, NormalizedTerm)> = thesaurus
        .into_iter()
        .filter_map(|(key, value)| {
            let pattern_str = key.to_string();
            if pattern_str.trim().is_empty() || pattern_str.len() < MIN_FIND_PATTERN_LENGTH {
                log::warn!(
                    "Skipping invalid pattern in find_matches: {:?} (length {} < {})",
                    pattern_str,
                    pattern_str.len(),
                    MIN_FIND_PATTERN_LENGTH
                );
                None
            } else {
                Some((key.clone(), value.clone()))
            }
        })
        .collect();

    if valid_patterns.is_empty() {
        log::debug!("No valid patterns for find_matches, returning empty");
        return Ok(Vec::new());
    }

    let patterns: Vec<NormalizedTermValue> =
        valid_patterns.iter().map(|(k, _)| k.clone()).collect();
    let pattern_map: std::collections::HashMap<NormalizedTermValue, NormalizedTerm> =
        valid_patterns.into_iter().collect();

    log::debug!(
        "Building find_matches automaton with {} valid patterns",
        patterns.len()
    );

    let ac = AhoCorasick::builder()
        .match_kind(MatchKind::LeftmostLongest)
        .ascii_case_insensitive(true)
        .build(patterns.clone())?;

    let mut matches: Vec<Matched> = Vec::new();
    for mat in ac.find_iter(text) {
        let term = &patterns[mat.pattern()];
        let normalized_term = pattern_map
            .get(term)
            .ok_or_else(|| TerraphimAutomataError::Dict(format!("Unknown term {term}")))?;

        matches.push(Matched {
            term: term.to_string(),
            normalized_term: normalized_term.clone(),
            pos: if return_positions {
                Some((mat.start(), mat.end()))
            } else {
                None
            },
        });
    }
    Ok(matches)
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LinkType {
    WikiLinks,
    HTMLLinks,
    MarkdownLinks,
    #[default]
    PlainText,
}

/// Minimum pattern length to prevent spurious matches.
/// Patterns shorter than this are filtered out to avoid:
/// - Empty patterns matching at every character position
/// - Single-character patterns causing excessive matches
const MIN_PATTERN_LENGTH: usize = 2;

/// Replace matches in text using the thesaurus.
///
/// Uses `display()` method on `NormalizedTerm` to get the case-preserved
/// display value for replacement output.
///
/// URLs (http, https, mailto, email addresses) are protected from replacement
/// to prevent corruption of links.
///
/// Patterns shorter than MIN_PATTERN_LENGTH (2) are filtered out to prevent
/// spurious matches at every character position.
pub fn replace_matches(text: &str, thesaurus: Thesaurus, link_type: LinkType) -> Result<Vec<u8>> {
    // Protect URLs from replacement
    let protector = UrlProtector::new();
    let (masked_text, protected_urls) = protector.mask_urls(text);

    let mut patterns: Vec<String> = Vec::new();
    let mut replace_with: Vec<String> = Vec::new();

    for (key, value) in thesaurus.into_iter() {
        let pattern_str = key.to_string();

        // Skip empty or too-short patterns to prevent spurious matches
        // Empty patterns match at every character position, causing text like
        // "exmatching_and_iterators_in_rustpmatching..." to appear
        if pattern_str.trim().is_empty() || pattern_str.len() < MIN_PATTERN_LENGTH {
            log::warn!(
                "Skipping invalid pattern: {:?} (length {} < {})",
                pattern_str,
                pattern_str.len(),
                MIN_PATTERN_LENGTH
            );
            continue;
        }

        // Use display() to get case-preserved value for output
        let display_text = value.display();
        let replacement = match link_type {
            LinkType::WikiLinks => format!("[[{}]]", display_text),
            LinkType::HTMLLinks => format!(
                "<a href=\"{}\">{}</a>",
                value.url.as_deref().unwrap_or_default(),
                display_text
            ),
            LinkType::MarkdownLinks => format!(
                "[{}]({})",
                display_text,
                value.url.as_deref().unwrap_or_default()
            ),
            LinkType::PlainText => display_text.to_string(),
        };

        patterns.push(pattern_str);
        replace_with.push(replacement);
    }

    // Validate alignment - patterns and replacements must be 1:1
    debug_assert_eq!(
        patterns.len(),
        replace_with.len(),
        "Pattern/replacement vector mismatch: {} patterns vs {} replacements",
        patterns.len(),
        replace_with.len()
    );

    // If no valid patterns, return original text unchanged
    if patterns.is_empty() {
        log::debug!("No valid patterns to replace, returning original text");
        return Ok(text.as_bytes().to_vec());
    }

    log::debug!("Building automaton with {} valid patterns", patterns.len());

    let ac = AhoCorasick::builder()
        .match_kind(MatchKind::LeftmostLongest)
        .ascii_case_insensitive(true)
        .build(&patterns)?;

    // Perform replacement on masked text
    let replaced = ac.replace_all(&masked_text, &replace_with);

    // Restore protected URLs
    let result = protector.restore_urls(&replaced, &protected_urls);

    Ok(result.into_bytes())
}

// tests

/// Extract the paragraph text starting at each automata term match.
///
/// For every matched term in `text`, returns the substring from the start of the term
/// until the end of the containing paragraph (first blank line or end-of-text).
pub fn extract_paragraphs_from_automata(
    text: &str,
    thesaurus: Thesaurus,
    include_term: bool,
) -> Result<Vec<(Matched, String)>> {
    let matches = find_matches(text, thesaurus, true)?;
    let mut results: Vec<(Matched, String)> = Vec::new();

    for m in matches.into_iter() {
        let (start, end) = m.pos.ok_or_else(|| {
            TerraphimAutomataError::Dict("Positions were not returned".to_string())
        })?;

        // Start at term start (or right after the term) depending on flag
        let paragraph_start = if include_term { start } else { end };
        let paragraph_end = find_paragraph_end(text, end);

        if paragraph_start <= paragraph_end && paragraph_start < text.len() {
            let slice = &text[paragraph_start..paragraph_end];
            results.push((m, slice.to_string()));
        }
    }

    Ok(results)
}

/// Find the end of the paragraph starting after `from_index`.
/// Paragraphs are separated by a blank line, matched by the earliest of
/// "\r\n\r\n", "\n\n", or "\r\r". If none found, returns end of text.
fn find_paragraph_end(text: &str, from_index: usize) -> usize {
    if from_index >= text.len() {
        return text.len();
    }
    let tail = &text[from_index..];
    let mut end_rel: Option<usize> = None;
    for sep in ["\r\n\r\n", "\n\n", "\r\r"] {
        if let Some(i) = tail.find(sep) {
            end_rel = Some(match end_rel {
                Some(cur) => cur.min(i),
                None => i,
            });
        }
    }
    match end_rel {
        Some(i) => from_index + i,
        None => text.len(),
    }
}

#[cfg(test)]
mod paragraph_tests {
    use super::*;
    use terraphim_types::{NormalizedTerm, NormalizedTermValue, Thesaurus};

    #[test]
    fn extracts_paragraph_from_term() {
        let mut thesaurus = Thesaurus::new("test".to_string());
        let norm = NormalizedTerm::new(1, NormalizedTermValue::from("lorem"));
        thesaurus.insert(NormalizedTermValue::from("lorem"), norm);

        let text = "Intro\n\nlorem ipsum dolor sit amet,\nconsectetur adipiscing elit.\n\nNext paragraph starts here.";

        let results = extract_paragraphs_from_automata(text, thesaurus, true).unwrap();
        assert_eq!(results.len(), 1);
        let (_m, para) = &results[0];
        assert!(para.starts_with("lorem ipsum"));
        assert!(para.contains("consectetur"));
        assert!(!para.contains("Next paragraph"));
    }
}

#[cfg(test)]
mod replacement_bug_tests {
    use super::*;
    use terraphim_types::{NormalizedTerm, NormalizedTermValue, Thesaurus};

    /// Regression test for the bug where empty patterns caused text to be
    /// inserted between every character.
    ///
    /// Bug manifestation: "npm install express" became
    /// "bun install exmatching_and_iterators_in_rustpmatching..."
    #[test]
    fn test_empty_pattern_does_not_cause_spurious_insertions() {
        let mut thesaurus = Thesaurus::new("test".to_string());

        // Simulate the bug: empty pattern with a display value
        let bad_nterm = NormalizedTerm::new(
            1,
            NormalizedTermValue::from("matching_and_iterators_in_rust"),
        )
        .with_display_value("matching_and_iterators_in_rust".to_string());
        thesaurus.insert(NormalizedTermValue::from(""), bad_nterm);

        // Add a valid pattern
        let bun_nterm = NormalizedTerm::new(2, NormalizedTermValue::from("bun"))
            .with_display_value("bun".to_string());
        thesaurus.insert(NormalizedTermValue::from("npm"), bun_nterm);

        let result =
            replace_matches("npm install express", thesaurus, LinkType::PlainText).unwrap();
        let result_str = String::from_utf8(result).unwrap();

        // Should NOT have spurious insertions between characters
        assert_eq!(result_str, "bun install express");
        assert!(!result_str.contains("matching_and_iterators_in_rust"));
    }

    #[test]
    fn test_single_char_pattern_is_filtered() {
        let mut thesaurus = Thesaurus::new("test".to_string());

        // Single character pattern - should be filtered
        let single_char_nterm = NormalizedTerm::new(1, NormalizedTermValue::from("expanded"))
            .with_display_value("expanded".to_string());
        thesaurus.insert(NormalizedTermValue::from("e"), single_char_nterm);

        // Valid pattern
        let bun_nterm = NormalizedTerm::new(2, NormalizedTermValue::from("bun"))
            .with_display_value("bun".to_string());
        thesaurus.insert(NormalizedTermValue::from("npm"), bun_nterm);

        let result =
            replace_matches("npm install express", thesaurus, LinkType::PlainText).unwrap();
        let result_str = String::from_utf8(result).unwrap();

        // Single-char pattern should be filtered, only npm->bun replacement should happen
        assert_eq!(result_str, "bun install express");
        assert!(!result_str.contains("expanded"));
    }

    #[test]
    fn test_whitespace_only_pattern_is_filtered() {
        let mut thesaurus = Thesaurus::new("test".to_string());

        // Whitespace-only pattern - should be filtered
        let ws_nterm = NormalizedTerm::new(1, NormalizedTermValue::from("space"))
            .with_display_value("space".to_string());
        thesaurus.insert(NormalizedTermValue::from("   "), ws_nterm);

        // Valid pattern
        let bun_nterm = NormalizedTerm::new(2, NormalizedTermValue::from("bun"))
            .with_display_value("bun".to_string());
        thesaurus.insert(NormalizedTermValue::from("npm"), bun_nterm);

        let result =
            replace_matches("npm install express", thesaurus, LinkType::PlainText).unwrap();
        let result_str = String::from_utf8(result).unwrap();

        assert_eq!(result_str, "bun install express");
        assert!(!result_str.contains("space"));
    }

    #[test]
    fn test_valid_replacement_still_works() {
        let mut thesaurus = Thesaurus::new("test".to_string());

        // Valid patterns
        let bun_nterm = NormalizedTerm::new(1, NormalizedTermValue::from("bun"))
            .with_display_value("bun".to_string());
        thesaurus.insert(NormalizedTermValue::from("npm"), bun_nterm);

        let yarn_nterm = NormalizedTerm::new(2, NormalizedTermValue::from("bun"))
            .with_display_value("bun".to_string());
        thesaurus.insert(NormalizedTermValue::from("yarn"), yarn_nterm);

        let result = replace_matches(
            "npm install && yarn add lodash",
            thesaurus,
            LinkType::PlainText,
        )
        .unwrap();
        let result_str = String::from_utf8(result).unwrap();

        assert_eq!(result_str, "bun install && bun add lodash");
    }

    #[test]
    fn test_empty_thesaurus_returns_original() {
        let thesaurus = Thesaurus::new("test".to_string());

        let result =
            replace_matches("npm install express", thesaurus, LinkType::PlainText).unwrap();
        let result_str = String::from_utf8(result).unwrap();

        assert_eq!(result_str, "npm install express");
    }

    #[test]
    fn test_find_matches_filters_empty_patterns() {
        let mut thesaurus = Thesaurus::new("test".to_string());

        // Empty pattern
        let empty_nterm = NormalizedTerm::new(1, NormalizedTermValue::from("empty"))
            .with_display_value("empty".to_string());
        thesaurus.insert(NormalizedTermValue::from(""), empty_nterm);

        // Valid pattern
        let test_nterm = NormalizedTerm::new(2, NormalizedTermValue::from("test"))
            .with_display_value("test".to_string());
        thesaurus.insert(NormalizedTermValue::from("hello"), test_nterm);

        let matches = find_matches("hello world", thesaurus, false).unwrap();

        // Should only find "hello", not empty pattern matches
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].term, "hello");
    }
}
