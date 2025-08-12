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
