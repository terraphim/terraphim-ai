use ahash::AHashSet;
use parking_lot::RwLock;
use terraphim_automata::find_matches;
use terraphim_types::Thesaurus;

use crate::config::KgScorerConfig;
use fff_search::external_scorer::ExternalScorer;
use fff_search::types::FileItem;

/// Scores files by counting knowledge-graph concept matches in their path.
///
/// Implements [`ExternalScorer`] so it can be plugged directly into a
/// `fff-search` [`ScoringContext`].  The scorer reads the file's
/// `relative_path`, runs it through the Aho-Corasick automata built from
/// the thesaurus, and returns `min(unique_matches * weight_per_term,
/// max_boost)`.
pub struct KgPathScorer {
    thesaurus: RwLock<Thesaurus>,
    config: KgScorerConfig,
}

impl KgPathScorer {
    /// Create a scorer with the given thesaurus and default config.
    pub fn new(thesaurus: Thesaurus) -> Self {
        Self {
            thesaurus: RwLock::new(thesaurus),
            config: KgScorerConfig::default(),
        }
    }

    /// Create a scorer with an explicit config.
    pub fn with_config(thesaurus: Thesaurus, config: KgScorerConfig) -> Self {
        Self {
            thesaurus: RwLock::new(thesaurus),
            config,
        }
    }

    /// Replace the internal thesaurus without restarting the scorer (hot-reload).
    pub fn update_thesaurus(&self, thesaurus: Thesaurus) {
        *self.thesaurus.write() = thesaurus;
    }
}

impl ExternalScorer for KgPathScorer {
    fn score(&self, file: &FileItem) -> i32 {
        let thesaurus = self.thesaurus.read().clone();

        if thesaurus.is_empty() {
            return 0;
        }

        let matches = match find_matches(&file.relative_path, thesaurus, false) {
            Ok(m) => m,
            Err(err) => {
                tracing::warn!(
                    path = %file.relative_path,
                    error = %err,
                    "KgPathScorer: find_matches failed"
                );
                return 0;
            }
        };

        let unique_ids: AHashSet<u64> = matches
            .iter()
            .map(|m| m.normalized_term.id)
            .collect();

        let unique_count = unique_ids.len() as i32;
        (unique_count * self.config.weight_per_term).min(self.config.max_boost)
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use terraphim_types::{NormalizedTerm, NormalizedTermValue};

    use super::*;

    fn make_file(relative_path: &str) -> FileItem {
        FileItem::new_raw(
            PathBuf::from(relative_path),
            relative_path.to_string(),
            relative_path
                .rsplit('/')
                .next()
                .unwrap_or(relative_path)
                .to_string(),
            0,
            0,
            None,
            false,
        )
    }

    fn make_term(id: &str, value: &str) -> (NormalizedTermValue, NormalizedTerm) {
        let key = NormalizedTermValue::from(value.to_string());
        let term = NormalizedTerm {
            id: id.parse::<u64>().unwrap_or(0),
            value: NormalizedTermValue::from(value.to_string()),
            display_value: None,
            url: None,
        };
        (key, term)
    }

    fn thesaurus_with(entries: &[(&str, &str)]) -> Thesaurus {
        let mut t = Thesaurus::new("test".to_string());
        for (id, val) in entries {
            let (k, v) = make_term(id, val);
            t.insert(k, v);
        }
        t
    }

    #[test]
    fn empty_thesaurus_returns_zero() {
        let scorer = KgPathScorer::new(Thesaurus::new("empty".to_string()));
        let file = make_file("src/main.rs");
        assert_eq!(scorer.score(&file), 0);
    }

    #[test]
    fn path_match_returns_weight() {
        let t = thesaurus_with(&[("1", "automata")]);
        let scorer = KgPathScorer::new(t);
        let file = make_file("crates/terraphim_automata/src/lib.rs");
        // "automata" appears once -> 1 * 5 = 5
        assert_eq!(scorer.score(&file), 5);
    }

    #[test]
    fn no_match_returns_zero() {
        let t = thesaurus_with(&[("1", "blockchain")]);
        let scorer = KgPathScorer::new(t);
        let file = make_file("src/main.rs");
        assert_eq!(scorer.score(&file), 0);
    }

    #[test]
    fn multiple_unique_terms_sum_weights() {
        let t = thesaurus_with(&[("1", "terraphim"), ("2", "automata")]);
        let scorer = KgPathScorer::new(t);
        // path contains both "terraphim" and "automata"
        let file = make_file("crates/terraphim_automata/src/lib.rs");
        // 2 unique terms * 5 = 10
        assert_eq!(scorer.score(&file), 10);
    }

    #[test]
    fn score_capped_at_max_boost() {
        // 7 unique terms * 5 = 35, capped at 30
        let entries: Vec<(String, String)> = (1..=7)
            .map(|i| (i.to_string(), format!("term{i}")))
            .collect();
        let mut t = Thesaurus::new("test".to_string());
        for (id, val) in &entries {
            let (k, v) = make_term(id, val);
            t.insert(k, v);
        }
        let path = entries
            .iter()
            .map(|(_, v)| v.as_str())
            .collect::<Vec<_>>()
            .join("/");
        let scorer = KgPathScorer::new(t);
        let file = make_file(&path);
        assert_eq!(scorer.score(&file), 30);
    }

    #[test]
    fn hot_reload_updates_thesaurus() {
        let old = thesaurus_with(&[("1", "oldterm")]);
        let scorer = KgPathScorer::new(old);
        let file = make_file("src/oldterm.rs");

        assert_eq!(scorer.score(&file), 5);

        // Replace with a thesaurus that does not match
        let new_t = thesaurus_with(&[("2", "newterm")]);
        scorer.update_thesaurus(new_t);

        // "oldterm" no longer in thesaurus
        assert_eq!(scorer.score(&file), 0);
        // "newterm" now matches
        let file2 = make_file("src/newterm.rs");
        assert_eq!(scorer.score(&file2), 5);
    }
}
