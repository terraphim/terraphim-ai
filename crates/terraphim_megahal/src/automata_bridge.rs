//! Knowledge-graph keyword seeding via `terraphim_automata` (feature
//! `automata`).
//!
//! Runs Aho-Corasick thesaurus matching over the user input and injects the
//! matched role-thesaurus concepts (normalised to upper case) as additional
//! reply-seeding keywords, so replies gravitate towards domain concepts.
//!
//! The feature is additive: [`MegaHal::reply`] and the Ruby conformance
//! suite are untouched unless [`MegaHal::reply_with_thesaurus`] is called
//! explicitly.

use terraphim_automata::find_matches;
use terraphim_types::Thesaurus;

use crate::MegaHal;
use rand_core::Rng;

impl MegaHal {
    /// Compute the knowledge-graph keywords for `text`: the normalised
    /// canonical terms of every thesaurus concept matched in the input,
    /// upper-cased and de-duplicated in match order.
    pub fn kg_keywords(text: &str, thesaurus: &Thesaurus) -> Vec<String> {
        let mut keywords: Vec<String> = Vec::new();
        let Ok(matches) = find_matches(text, thesaurus, false) else {
            return keywords;
        };
        for matched in matches {
            let bytes = matched.normalized_term.value.as_ref();
            let value = String::from_utf8_lossy(bytes).to_uppercase();
            if !keywords.contains(&value) {
                keywords.push(value);
            }
        }
        keywords
    }

    /// Reply as [`MegaHal::reply_with_error`], first matching `text` against
    /// a role thesaurus and injecting the matched concepts into the keyword
    /// set (see [`MegaHal::reply_with_extra_keywords`]).
    pub fn reply_with_thesaurus(
        &mut self,
        input: Option<&str>,
        rng: &mut impl Rng,
        error_reply: &str,
        thesaurus: &Thesaurus,
    ) -> String {
        let extra = input
            .map(str::trim)
            .map(|text| Self::kg_keywords(text, thesaurus))
            .unwrap_or_default();
        self.reply_with_extra_keywords(input, rng, error_reply, &extra)
    }
}
