//! Keyword-based capability extraction for routing.
//!
//! This module provides keyword matching to extract capabilities from text,
//! enabling intelligent routing based on prompt content.
//!
//! Uses terraphim_automata::find_matches with Thesaurus for O(text + patterns) performance,
//! replacing the previous O(text * keywords) substring search approach.

use std::collections::HashMap;
use terraphim_automata::find_matches;
use terraphim_types::capability::Capability;
use terraphim_types::{NormalizedTerm, NormalizedTermValue, Thesaurus};

/// Internal keyword mapping structure.
#[derive(Debug, Clone)]
#[allow(dead_code)]
struct KeywordMapping {
    keywords: Vec<String>,
    capability: Capability,
    priority: u32,
}

/// Maps keywords to capabilities using terraphim_automata Thesaurus.
#[derive(Debug, Clone)]
pub struct KeywordRouter {
    mappings: Vec<KeywordMapping>,
    keyword_thesaurus: Thesaurus,
    keyword_to_mapping: HashMap<String, usize>,
}

impl KeywordRouter {
    /// Create a new KeywordRouter with default mappings.
    pub fn new() -> Self {
        Self::from_mappings(Self::default_mappings())
    }

    /// Create with custom mappings.
    pub fn with_mappings(mappings: Vec<(Vec<String>, Capability, u32)>) -> Self {
        let keyword_mappings: Vec<KeywordMapping> = mappings
            .into_iter()
            .map(|(keywords, capability, priority)| KeywordMapping {
                keywords,
                capability,
                priority,
            })
            .collect();

        Self::from_mappings(keyword_mappings)
    }

    /// Build router from internal KeywordMapping structs using Thesaurus.
    fn from_mappings(mappings: Vec<KeywordMapping>) -> Self {
        let mut thesaurus = Thesaurus::new("keyword-routing".to_string());
        let mut keyword_to_mapping: HashMap<String, usize> = HashMap::new();

        for (mapping_idx, mapping) in mappings.iter().enumerate() {
            for keyword in &mapping.keywords {
                let key_lower = keyword.to_lowercase();
                let term = NormalizedTerm::new(
                    format!("{:?}", mapping.capability),
                    NormalizedTermValue::from(key_lower.clone()),
                );
                thesaurus.insert(NormalizedTermValue::from(key_lower.clone()), term);
                keyword_to_mapping.insert(key_lower, mapping_idx);
            }
        }

        Self {
            mappings,
            keyword_thesaurus: thesaurus,
            keyword_to_mapping,
        }
    }

    /// Extract capabilities from text using terraphim_automata::find_matches.
    ///
    /// This method achieves O(text + patterns) performance by using the Thesaurus
    /// with Aho-Corasick automaton via terraphim_automata.
    pub fn extract_capabilities(&self, text: &str) -> Vec<Capability> {
        let text_lower = text.to_lowercase();
        let matches = match find_matches(&text_lower, self.keyword_thesaurus.clone(), false) {
            Ok(m) => m,
            Err(_) => return vec![],
        };

        let mut caps = std::collections::HashSet::new();
        let mut seen_mappings = std::collections::HashSet::new();

        for matched in &matches {
            let key = matched.term.to_lowercase();
            if let Some(&mapping_idx) = self.keyword_to_mapping.get(&key) {
                if seen_mappings.insert(mapping_idx) {
                    caps.insert(self.mappings[mapping_idx].capability);
                }
            }
        }

        caps.into_iter().collect()
    }

    /// Check if text contains any capability-indicating keywords.
    pub fn has_keywords(&self, text: &str) -> bool {
        !self.extract_capabilities(text).is_empty()
    }

    /// Get the default keyword mappings.
    fn default_mappings() -> Vec<KeywordMapping> {
        vec![
            // Deep thinking (high priority)
            KeywordMapping {
                keywords: vec![
                    "think".to_string(),
                    "thinking".to_string(),
                    "reason".to_string(),
                    "reasoning".to_string(),
                    "analyze deeply".to_string(),
                    "complex analysis".to_string(),
                    "deep dive".to_string(),
                    "carefully consider".to_string(),
                ],
                capability: Capability::DeepThinking,
                priority: 100,
            },
            // Fast thinking (lower priority)
            KeywordMapping {
                keywords: vec![
                    "quick".to_string(),
                    "fast".to_string(),
                    "simple".to_string(),
                    "brief".to_string(),
                    "short".to_string(),
                    "summary".to_string(),
                ],
                capability: Capability::FastThinking,
                priority: 50,
            },
            // Code generation
            KeywordMapping {
                keywords: vec![
                    "implement".to_string(),
                    "code".to_string(),
                    "write function".to_string(),
                    "create".to_string(),
                    "build".to_string(),
                    "develop".to_string(),
                    "program".to_string(),
                ],
                capability: Capability::CodeGeneration,
                priority: 90,
            },
            // Code review
            KeywordMapping {
                keywords: vec![
                    "review".to_string(),
                    "check".to_string(),
                    "audit".to_string(),
                    "inspect".to_string(),
                    "evaluate code".to_string(),
                ],
                capability: Capability::CodeReview,
                priority: 85,
            },
            // Architecture
            KeywordMapping {
                keywords: vec![
                    "design".to_string(),
                    "architecture".to_string(),
                    "structure".to_string(),
                    "system design".to_string(),
                    "pattern".to_string(),
                ],
                capability: Capability::Architecture,
                priority: 88,
            },
            // Testing
            KeywordMapping {
                keywords: vec![
                    "test".to_string(),
                    "testing".to_string(),
                    "unit test".to_string(),
                    "integration test".to_string(),
                    "spec".to_string(),
                ],
                capability: Capability::Testing,
                priority: 80,
            },
            // Refactoring
            KeywordMapping {
                keywords: vec![
                    "refactor".to_string(),
                    "restructure".to_string(),
                    "clean up".to_string(),
                    "improve".to_string(),
                    "optimize code".to_string(),
                ],
                capability: Capability::Refactoring,
                priority: 75,
            },
            // Documentation
            KeywordMapping {
                keywords: vec![
                    "document".to_string(),
                    "documentation".to_string(),
                    "readme".to_string(),
                    "explain how".to_string(),
                    "guide".to_string(),
                ],
                capability: Capability::Documentation,
                priority: 70,
            },
            // Explanation
            KeywordMapping {
                keywords: vec![
                    "explain".to_string(),
                    "clarify".to_string(),
                    "describe".to_string(),
                    "what is".to_string(),
                    "how does".to_string(),
                ],
                capability: Capability::Explanation,
                priority: 65,
            },
            // Security audit
            KeywordMapping {
                keywords: vec![
                    "security".to_string(),
                    "secure".to_string(),
                    "vulnerability".to_string(),
                    "audit".to_string(),
                    "threat".to_string(),
                    "sanitize".to_string(),
                ],
                capability: Capability::SecurityAudit,
                priority: 95,
            },
            // Performance
            KeywordMapping {
                keywords: vec![
                    "performance".to_string(),
                    "optimize".to_string(),
                    "speed".to_string(),
                    "fast".to_string(),
                    "efficient".to_string(),
                    "benchmark".to_string(),
                ],
                capability: Capability::Performance,
                priority: 78,
            },
        ]
    }
}

impl Default for KeywordRouter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_deep_thinking() {
        let router = KeywordRouter::new();

        let caps =
            router.extract_capabilities("I need you to think carefully about this complex problem");

        assert!(caps.contains(&Capability::DeepThinking));
    }

    #[test]
    fn test_extract_code_generation() {
        let router = KeywordRouter::new();

        let caps = router.extract_capabilities("Please implement a function to parse JSON");

        assert!(caps.contains(&Capability::CodeGeneration));
    }

    #[test]
    fn test_extract_security_audit() {
        let router = KeywordRouter::new();

        let caps = router.extract_capabilities("Audit this code for security vulnerabilities");

        assert!(caps.contains(&Capability::SecurityAudit));
    }

    #[test]
    fn test_multiple_capabilities() {
        let router = KeywordRouter::new();

        let caps = router.extract_capabilities(
            "Implement a secure authentication system and write tests for it",
        );

        assert!(caps.contains(&Capability::CodeGeneration));
        assert!(caps.contains(&Capability::SecurityAudit));
        assert!(caps.contains(&Capability::Testing));
    }

    #[test]
    fn test_no_capabilities() {
        let router = KeywordRouter::new();

        let caps = router.extract_capabilities("Hello, how are you today?");

        assert!(caps.is_empty());
    }

    #[test]
    fn test_case_insensitive() {
        let router = KeywordRouter::new();

        let caps1 = router.extract_capabilities("IMPLEMENT this feature");
        let caps2 = router.extract_capabilities("implement this feature");
        let caps3 = router.extract_capabilities("Implement this feature");

        assert_eq!(caps1, caps2);
        assert_eq!(caps2, caps3);
    }

    #[test]
    fn test_has_keywords() {
        let router = KeywordRouter::new();

        assert!(router.has_keywords("Think about this problem"));
        assert!(!router.has_keywords("Hello world"));
    }

    #[test]
    fn test_with_custom_mappings() {
        let router = KeywordRouter::with_mappings(vec![
            (
                vec!["custom1".to_string(), "custom2".to_string()],
                Capability::DeepThinking,
                100,
            ),
            (
                vec!["my_unique_keyword".to_string()],
                Capability::CodeGeneration,
                50,
            ),
        ]);

        let caps = router.extract_capabilities("This has custom1 in it");
        assert!(caps.contains(&Capability::DeepThinking));

        let caps2 = router.extract_capabilities("This has my_unique_keyword");
        assert!(caps2.contains(&Capability::CodeGeneration));
    }

    #[test]
    fn test_thesaurus_keyword_count() {
        let router = KeywordRouter::new();
        assert!(router.keyword_thesaurus.len() >= 60);
    }

    #[test]
    fn test_with_mappings_builds_thesaurus() {
        let router = KeywordRouter::with_mappings(vec![(
            vec!["foo".into(), "bar".into()],
            Capability::Testing,
            50,
        )]);
        let caps = router.extract_capabilities("this is a foo test");
        assert!(caps.contains(&Capability::Testing));
    }

    #[test]
    fn test_clone_preserves_behaviour() {
        let router = KeywordRouter::new();
        let cloned = router.clone();
        let caps1 = router.extract_capabilities("implement a function");
        let caps2 = cloned.extract_capabilities("implement a function");
        assert_eq!(caps1, caps2);
    }
}

#[cfg(test)]
mod proptest_tests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn extract_capabilities_never_panics(text in "\\PC{0,500}") {
            let router = KeywordRouter::new();
            // Should never panic, regardless of input
            let _ = router.extract_capabilities(&text);
        }

        #[test]
        fn extract_capabilities_returns_subset_of_all(text in "\\PC{0,200}") {
            let router = KeywordRouter::new();
            let all_caps = Capability::all();
            let extracted = router.extract_capabilities(&text);

            for cap in &extracted {
                prop_assert!(all_caps.contains(cap));
            }
        }

        #[test]
        fn has_keywords_consistent_with_extract(text in "\\PC{0,200}") {
            let router = KeywordRouter::new();
            let has = router.has_keywords(&text);
            let extracted = router.extract_capabilities(&text);

            prop_assert_eq!(has, !extracted.is_empty());
        }
    }
}
