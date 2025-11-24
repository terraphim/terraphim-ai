use anyhow::Result;
use terraphim_automata::{
    autocomplete_search, build_autocomplete_index, fuzzy_autocomplete_search_jaro_winkler,
    load_thesaurus, AutocompleteConfig, AutocompleteIndex, AutocompleteResult,
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
        let index = build_autocomplete_index(&thesaurus, &config)?;

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
    pub fn from_role(role_name: &str, config_path: Option<&str>) -> Result<Self> {
        use terraphim_config::Config;
        use terraphim_automata::AutomataPath;

        let config = if let Some(path) = config_path {
            Config::from_file(path)?
        } else {
            Config::load()?
        };

        let role = config
            .roles
            .get(role_name)
            .ok_or_else(|| anyhow::anyhow!("Role '{}' not found in config", role_name))?;

        // Load thesaurus from role configuration
        let thesaurus_path = role
            .extra
            .get("thesaurus_path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("No thesaurus_path found for role '{}'", role_name))?;

        let automata_path = AutomataPath::from_local(thesaurus_path);
        let thesaurus = load_thesaurus(&automata_path)?;

        Self::from_thesaurus(thesaurus)
    }

    /// Perform autocomplete search
    pub fn autocomplete(&self, query: &str, limit: usize) -> Vec<AutocompleteSuggestion> {
        // For short queries, use exact matching
        let results = if query.len() < 3 {
            autocomplete_search(&self.index, query, limit)
        } else {
            // For longer queries, use fuzzy matching
            fuzzy_autocomplete_search_jaro_winkler(&self.index, query, limit, 0.8)
        };

        results
            .into_iter()
            .map(|r| AutocompleteSuggestion {
                term: r.term.clone(),
                nterm: r.term.clone(),
                score: r.score,
                from_kg: true,
                definition: self.get_definition(&r.term),
                url: self.get_url(&r.term),
            })
            .collect()
    }

    /// Perform fuzzy search for autocomplete suggestions
    pub fn fuzzy_search(&self, query: &str, limit: usize) -> Vec<AutocompleteSuggestion> {
        let results = fuzzy_autocomplete_search_jaro_winkler(&self.index, query, limit, 0.7);

        results
            .into_iter()
            .map(|r| AutocompleteSuggestion {
                term: r.term.clone(),
                nterm: r.term.clone(),
                score: r.score,
                from_kg: true,
                definition: self.get_definition(&r.term),
                url: self.get_url(&r.term),
            })
            .collect()
    }

    /// Check if a term exists in the knowledge graph
    pub fn is_kg_term(&self, term: &str) -> bool {
        self.thesaurus
            .iter()
            .any(|t| t.nterm.eq_ignore_ascii_case(term) || t.term.eq_ignore_ascii_case(term))
    }

    /// Get term definition from thesaurus
    fn get_definition(&self, term: &str) -> Option<String> {
        self.thesaurus
            .iter()
            .find(|t| t.term.eq_ignore_ascii_case(term) || t.nterm.eq_ignore_ascii_case(term))
            .and_then(|t| t.definition.clone())
    }

    /// Get term URL from thesaurus
    fn get_url(&self, term: &str) -> Option<String> {
        self.thesaurus
            .iter()
            .find(|t| t.term.eq_ignore_ascii_case(term) || t.nterm.eq_ignore_ascii_case(term))
            .map(|t| t.url.clone())
    }

    /// Get all available terms
    pub fn get_terms(&self) -> Vec<String> {
        self.thesaurus
            .iter()
            .map(|t| t.nterm.clone())
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
    use terraphim_types::IndexedDocument;

    fn create_test_thesaurus() -> Thesaurus {
        vec![
            IndexedDocument {
                id: 1,
                nterm: "rust".to_string(),
                term: "Rust".to_string(),
                url: "https://rust-lang.org".to_string(),
                definition: Some("A systems programming language".to_string()),
            },
            IndexedDocument {
                id: 2,
                nterm: "tokio".to_string(),
                term: "Tokio".to_string(),
                url: "https://tokio.rs".to_string(),
                definition: Some("An async runtime for Rust".to_string()),
            },
        ]
    }

    #[test]
    fn test_autocomplete_engine_creation() {
        let thesaurus = create_test_thesaurus();
        let engine = AutocompleteEngine::from_thesaurus(thesaurus).unwrap();
        assert_eq!(engine.term_count(), 2);
    }

    #[test]
    fn test_is_kg_term() {
        let thesaurus = create_test_thesaurus();
        let engine = AutocompleteEngine::from_thesaurus(thesaurus).unwrap();

        assert!(engine.is_kg_term("rust"));
        assert!(engine.is_kg_term("Rust"));
        assert!(engine.is_kg_term("tokio"));
        assert!(!engine.is_kg_term("nonexistent"));
    }

    #[test]
    fn test_get_terms() {
        let thesaurus = create_test_thesaurus();
        let engine = AutocompleteEngine::from_thesaurus(thesaurus).unwrap();

        let terms = engine.get_terms();
        assert_eq!(terms.len(), 2);
        assert!(terms.contains(&"rust".to_string()));
        assert!(terms.contains(&"tokio".to_string()));
    }
}
