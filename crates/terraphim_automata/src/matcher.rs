use aho_corasick::{AhoCorasick, MatchKind};
use terraphim_types::{NormalizedTerm, NormalizedTermValue, Thesaurus};

use crate::{Result, TerraphimAutomataError};

#[derive(Debug, PartialEq, Clone)]
pub struct Matched {
    pub term: String,
    pub normalized_term: NormalizedTerm,
    pub pos: Option<(usize, usize)>,
}

pub fn find_matches(
    text: &str,
    thesaurus: Thesaurus,
    return_positions: bool,
) -> Result<Vec<Matched>> {
    let patterns: Vec<NormalizedTermValue> = thesaurus.keys().cloned().collect();

    let ac = AhoCorasick::builder()
        .match_kind(MatchKind::LeftmostLongest)
        .ascii_case_insensitive(true)
        .build(patterns.clone())?;

    let mut matches: Vec<Matched> = Vec::new();
    for mat in ac.find_iter(text) {
        let term = &patterns[mat.pattern()];
        let normalized_term = thesaurus
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

pub enum LinkType {
    WikiLinks,
    HTMLLinks,
    MarkdownLinks,
}

// // This function replacing instead of matching patterns
pub fn replace_matches(text: &str, thesaurus: Thesaurus, link_type: LinkType) -> Result<Vec<u8>> {
    let mut patterns: Vec<String> = Vec::new();
    let mut replace_with: Vec<String> = Vec::new();
    for (key, value) in thesaurus.into_iter() {
        match link_type {
            LinkType::WikiLinks => {
                patterns.push(key.to_string());
                replace_with.push(format!("[[{}]]", value.clone().value));
            }
            LinkType::HTMLLinks => {
                patterns.push(key.to_string());
                replace_with.push(format!(
                    "<a href=\"{}\">{}</a>",
                    value.clone().url.unwrap_or_default(),
                    value.clone().value
                ));
            }
            LinkType::MarkdownLinks => {
                patterns.push(key.to_string());
                replace_with.push(format!(
                    "[{}]({})",
                    value.clone().value,
                    value.clone().url.unwrap_or_default()
                ));
            }
        }
    }
    let ac = AhoCorasick::builder()
        .match_kind(MatchKind::LeftmostLongest)
        .ascii_case_insensitive(true)
        .build(patterns)?;

    let result = ac.replace_all_bytes(text.as_bytes(), &replace_with);
    Ok(result)
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
