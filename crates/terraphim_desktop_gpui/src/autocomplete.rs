use anyhow::Result;
use terraphim_automata::{load_thesaurus, Autocomplete};
use terraphim_types::Thesaurus;

/// Autocomplete engine integrated with Terraphim automata
pub struct AutocompleteEngine {
    automata: Autocomplete,
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
    /// Create engine from thesaurus JSON file
    pub fn from_thesaurus_file(path: &str) -> Result<Self> {
        let thesaurus = load_thesaurus(path)?;
        let automata = Autocomplete::from_thesaurus(&thesaurus)?;

        log::info!(
            "Loaded autocomplete engine with {} terms",
            thesaurus.len()
        );

        Ok(Self {
            automata,
            thesaurus,
        })
    }

    /// Create engine from role configuration
    pub fn from_role(role_name: &str, config_path: Option<&str>) -> Result<Self> {
        use terraphim_config::Config;

        let config = if let Some(path) = config_path {
            Config::from_file(path)?
        } else {
            Config::load()?
        };

        let role = config
            .roles
            .get(role_name)
            .ok_or_else(|| anyhow::anyhow!("Role '{}' not found", role_name))?;

        // Get thesaurus path from role config
        let thesaurus_path = role
            .thesaurus_path
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("No thesaurus path configured for role"))?;

        Self::from_thesaurus_file(thesaurus_path)
    }

    /// Get autocomplete suggestions for a query
    pub fn autocomplete(&self, query: &str, limit: usize) -> Vec<AutocompleteSuggestion> {
        if query.is_empty() {
            return vec![];
        }

        // Get suggestions from automata
        let suggestions = self.automata.autocomplete(query, limit);

        suggestions
            .into_iter()
            .map(|term| {
                // Look up additional info from thesaurus
                let thesaurus_entry = self.thesaurus.iter().find(|t| t.nterm == term);

                AutocompleteSuggestion {
                    term: term.clone(),
                    nterm: term.clone(),
                    score: 1.0, // TODO: Calculate relevance score
                    from_kg: thesaurus_entry.is_some(),
                    definition: thesaurus_entry
                        .and_then(|t| t.definition.clone()),
                    url: thesaurus_entry.map(|t| t.url.clone()),
                }
            })
            .collect()
    }

    /// Check if a term exists in the knowledge graph
    pub fn is_kg_term(&self, term: &str) -> bool {
        self.thesaurus.iter().any(|t| {
            t.nterm.eq_ignore_ascii_case(term) || t.term.eq_ignore_ascii_case(term)
        })
    }

    /// Get all terms for a partial match (fuzzy search)
    pub fn fuzzy_search(&self, query: &str, limit: usize) -> Vec<AutocompleteSuggestion> {
        use terraphim_automata::fuzzy_autocomplete_search_jaro_winkler;

        let suggestions = fuzzy_autocomplete_search_jaro_winkler(&self.automata, query, limit);

        suggestions
            .into_iter()
            .map(|term| {
                let thesaurus_entry = self.thesaurus.iter().find(|t| t.nterm == term);

                AutocompleteSuggestion {
                    term: term.clone(),
                    nterm: term.clone(),
                    score: 0.8, // Fuzzy match score
                    from_kg: thesaurus_entry.is_some(),
                    definition: thesaurus_entry
                        .and_then(|t| t.definition.clone()),
                    url: thesaurus_entry.map(|t| t.url.clone()),
                }
            })
            .collect()
    }

    /// Get term count
    pub fn term_count(&self) -> usize {
        self.thesaurus.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_autocomplete_engine_creation() {
        // This would need a test thesaurus file
        // For now, just test the structure compiles
    }

    #[test]
    fn test_is_kg_term() {
        // Would test with actual thesaurus data
    }
}
