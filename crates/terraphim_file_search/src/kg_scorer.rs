use ahash::AHashSet;
use parking_lot::RwLock;
use terraphim_automata::find_matches;
use terraphim_types::Thesaurus;

use crate::config::KgScorerConfig;

/// Scores files by counting knowledge-graph concept matches in their path.
///
/// The scorer reads a resolved relative path, runs it through the Aho-Corasick
/// automata built from the thesaurus, and returns
/// `min(unique_matches * weight_per_term, max_boost)`.
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

    /// Calculate the KG boost for a resolved relative file path.
    pub fn score_path(&self, relative_path: &str) -> i32 {
        let thesaurus = self.thesaurus.read().clone();

        if thesaurus.is_empty() {
            return 0;
        }

        let matches = match find_matches(relative_path, thesaurus, false) {
            Ok(m) => m,
            Err(err) => {
                tracing::warn!(
                    path = %relative_path,
                    error = %err,
                    "KgPathScorer: find_matches failed"
                );
                return 0;
            }
        };

        let unique_ids: AHashSet<u64> = matches.iter().map(|m| m.normalized_term.id).collect();

        let unique_count = unique_ids.len() as i32;
        (unique_count * self.config.weight_per_term).min(self.config.max_boost)
    }
}

#[cfg(test)]
mod tests {
    use terraphim_types::{NormalizedTerm, NormalizedTermValue};

    use super::*;

    fn make_term(id: u64, value: &str) -> (NormalizedTermValue, NormalizedTerm) {
        let key = NormalizedTermValue::from(value.to_string());
        let term = NormalizedTerm {
            id,
            value: NormalizedTermValue::from(value.to_string()),
            display_value: None,
            url: None,
            action: None,
            priority: None,
            trigger: None,
            pinned: false,
        };
        (key, term)
    }

    fn thesaurus_with(entries: &[(u64, &str)]) -> Thesaurus {
        let mut t = Thesaurus::new("test".to_string());
        for (id, val) in entries {
            let (k, v) = make_term(*id, val);
            t.insert(k, v);
        }
        t
    }

    #[test]
    fn empty_thesaurus_returns_zero() {
        let scorer = KgPathScorer::new(Thesaurus::new("empty".to_string()));
        assert_eq!(scorer.score_path("src/main.rs"), 0);
    }

    #[test]
    fn path_match_returns_weight() {
        let t = thesaurus_with(&[(1, "automata")]);
        let scorer = KgPathScorer::new(t);
        // "automata" appears once -> 1 * 5 = 5
        assert_eq!(scorer.score_path("crates/terraphim_automata/src/lib.rs"), 5);
    }

    #[test]
    fn no_match_returns_zero() {
        let t = thesaurus_with(&[(1, "blockchain")]);
        let scorer = KgPathScorer::new(t);
        assert_eq!(scorer.score_path("src/main.rs"), 0);
    }

    #[test]
    fn multiple_unique_terms_sum_weights() {
        let t = thesaurus_with(&[(1, "terraphim"), (2, "automata")]);
        let scorer = KgPathScorer::new(t);
        // path contains both "terraphim" and "automata"
        // 2 unique terms * 5 = 10
        assert_eq!(
            scorer.score_path("crates/terraphim_automata/src/lib.rs"),
            10
        );
    }

    #[test]
    fn score_capped_at_max_boost() {
        // 7 unique terms * 5 = 35, capped at 30
        let entries: Vec<(u64, String)> = (1..=7).map(|i| (i as u64, format!("term{i}"))).collect();
        let mut t = Thesaurus::new("test".to_string());
        for (id, val) in &entries {
            let (k, v) = make_term(*id, val);
            t.insert(k, v);
        }
        let path = entries
            .iter()
            .map(|(_, v)| v.as_str())
            .collect::<Vec<_>>()
            .join("/");
        let scorer = KgPathScorer::new(t);
        assert_eq!(scorer.score_path(&path), 30);
    }

    #[test]
    fn hot_reload_updates_thesaurus() {
        let old = thesaurus_with(&[(1, "oldterm")]);
        let scorer = KgPathScorer::new(old);

        assert_eq!(scorer.score_path("src/oldterm.rs"), 5);

        // Replace with a thesaurus that does not match
        let new_t = thesaurus_with(&[(2, "newterm")]);
        scorer.update_thesaurus(new_t);

        // "oldterm" no longer in thesaurus
        assert_eq!(scorer.score_path("src/oldterm.rs"), 0);
        // "newterm" now matches
        assert_eq!(scorer.score_path("src/newterm.rs"), 5);
    }
}
