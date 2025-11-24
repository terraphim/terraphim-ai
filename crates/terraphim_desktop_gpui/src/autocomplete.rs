use anyhow::Result;
use terraphim_automata::{
    autocomplete_search, build_autocomplete_index, fuzzy_autocomplete_search, AutocompleteConfig, AutocompleteIndex,
};
use terraphim_types::Thesaurus;

/// Autocomplete engine integrated with Terraphim automata
pub struct AutocompleteEngine {
    index: AutocompleteIndex,
    thesaurus: Thesaurus,
}

#[derive(Clone, Debug)]
pub struct AutocompleteSuggestion {
    pub term: String,
    pub nterm: String,
    pub score: f64,
    pub from_kg: bool,
    pub definition: Option<String>,
    pub url: Option<String>,
}

impl AutocompleteEngine {
    /// Create engine from thesaurus
    pub fn from_thesaurus(thesaurus: Thesaurus) -> Result<Self> {
        let config = AutocompleteConfig::default();
        // Pass by value since build_autocomplete_index takes ownership
        let index = build_autocomplete_index(thesaurus.clone(), Some(config))?;

        log::info!(
            "Loaded autocomplete engine with {} terms",
            thesaurus.len()
        );

        Ok(Self { index, thesaurus })
    }

    /// Create engine from thesaurus JSON string
    pub fn from_thesaurus_json(json: &str) -> Result<Self> {
        let thesaurus = terraphim_automata::load_thesaurus_from_json(json)?;
        Self::from_thesaurus(thesaurus)
    }

    /// Create engine from role configuration
    pub async fn from_role(role_name: &str, config_path: Option<&str>) -> Result<Self> {
        
        

        // Load config
        let mut config = if let Some(path) = config_path {
            // For simplicity, we'll create a config from ConfigState
            // In a real app, you'd load from file properly
            return Err(anyhow::anyhow!("Loading from file not yet implemented. Use from_thesaurus_json instead."));
        } else {
            return Err(anyhow::anyhow!("No config path provided"));
        };
    }

    /// Perform autocomplete search
    pub fn autocomplete(&self, query: &str, limit: usize) -> Vec<AutocompleteSuggestion> {
        // For short queries, use exact matching
        let results = if query.len() < 3 {
            autocomplete_search(&self.index, query, Some(limit)).unwrap_or_default()
        } else {
            // For longer queries, use fuzzy matching
            fuzzy_autocomplete_search(&self.index, query, 0.8, Some(limit)).unwrap_or_default()
        };

        results
            .into_iter()
            .map(|r| AutocompleteSuggestion {
                term: r.term.clone(),
                nterm: r.term.clone(),
                score: r.score,
                from_kg: true,
                definition: None,
                url: None,
            })
            .collect()
    }

    /// Perform fuzzy search for autocomplete suggestions
    pub fn fuzzy_search(&self, query: &str, limit: usize) -> Vec<AutocompleteSuggestion> {
        let results =
            fuzzy_autocomplete_search(&self.index, query, 0.7, Some(limit)).unwrap_or_default();

        results
            .into_iter()
            .map(|r| AutocompleteSuggestion {
                term: r.term.clone(),
                nterm: r.term.clone(),
                score: r.score,
                from_kg: true,
                definition: None,
                url: None,
            })
            .collect()
    }

    /// Check if a term exists in the knowledge graph
    pub fn is_kg_term(&self, term: &str) -> bool {
        self.index.metadata_get(term).is_some()
    }

    /// Get all available terms
    pub fn get_terms(&self) -> Vec<String> {
        self.index
            .metadata_iter()
            .map(|(k, _)| k.to_string())
            .collect()
    }

    /// Get thesaurus size
    pub fn term_count(&self) -> usize {
        self.thesaurus.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_autocomplete_suggestion_structure() {
        let suggestion = AutocompleteSuggestion {
            term: "rust".to_string(),
            nterm: "rust".to_string(),
            score: 1.0,
            from_kg: true,
            definition: Some("A programming language".to_string()),
            url: Some("https://rust-lang.org".to_string()),
        };

        assert_eq!(suggestion.term, "rust");
        assert_eq!(suggestion.score, 1.0);
        assert!(suggestion.from_kg);
    }

    #[test]
    fn test_autocomplete_from_json() {
        let json = r#"[
            {"id": 1, "nterm": "rust", "url": "https://rust-lang.org"}
        ]"#;

        let result = AutocompleteEngine::from_thesaurus_json(json);
        assert!(result.is_ok());

        let engine = result.unwrap();
        assert_eq!(engine.term_count(), 1);
    }

    #[test]
    fn test_is_kg_term() {
        let json = r#"[
            {"id": 1, "nterm": "rust", "url": "https://rust-lang.org"},
            {"id": 2, "nterm": "tokio", "url": "https://tokio.rs"}
        ]"#;

        let engine = AutocompleteEngine::from_thesaurus_json(json).unwrap();

        assert!(engine.is_kg_term("rust"));
        assert!(engine.is_kg_term("tokio"));
        assert!(!engine.is_kg_term("nonexistent"));
    }

    #[test]
    fn test_autocomplete_search() {
        let json = r#"[
            {"id": 1, "nterm": "rust", "url": "https://rust-lang.org"},
            {"id": 2, "nterm": "ruby", "url": "https://ruby-lang.org"}
        ]"#;

        let engine = AutocompleteEngine::from_thesaurus_json(json).unwrap();
        let suggestions = engine.autocomplete("ru", 10);

        assert!(!suggestions.is_empty());
        assert!(suggestions.iter().any(|s| s.term.starts_with("ru")));
    }

    #[test]
    fn test_fuzzy_search() {
        let json = r#"[
            {"id": 1, "nterm": "rust", "url": "https://rust-lang.org"},
            {"id": 2, "nterm": "ruby", "url": "https://ruby-lang.org"}
        ]"#;

        let engine = AutocompleteEngine::from_thesaurus_json(json).unwrap();
        let suggestions = engine.fuzzy_search("rst", 10);

        // Fuzzy search should find "rust" even with missing 'u'
        assert!(!suggestions.is_empty());
    }

    #[test]
    fn test_get_terms() {
        let json = r#"[
            {"id": 1, "nterm": "rust", "url": "https://rust-lang.org"},
            {"id": 2, "nterm": "tokio", "url": "https://tokio.rs"}
        ]"#;

        let engine = AutocompleteEngine::from_thesaurus_json(json).unwrap();
        let terms = engine.get_terms();

        assert_eq!(terms.len(), 2);
        assert!(terms.contains(&"rust".to_string()));
        assert!(terms.contains(&"tokio".to_string()));
    }
}
