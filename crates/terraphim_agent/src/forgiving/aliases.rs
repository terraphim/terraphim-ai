//! Command alias management
//!
//! Maps short aliases to their canonical command forms.

use std::collections::HashMap;

/// Default command aliases
pub const DEFAULT_ALIASES: &[(&str, &str)] = &[
    // Search aliases
    ("q", "search"),
    ("query", "search"),
    ("find", "search"),
    ("s", "search"),
    // Help aliases
    ("h", "help"),
    ("?", "help"),
    // Config aliases
    ("c", "config"),
    ("cfg", "config"),
    // Role aliases
    ("r", "role"),
    // Graph aliases
    ("g", "graph"),
    ("kg", "graph"),
    // Quit aliases
    ("quit", "quit"),
    ("exit", "quit"),
    ("bye", "quit"),
    // MCP tool aliases
    ("ac", "autocomplete"),
    ("th", "thesaurus"),
];

/// Registry for command aliases
#[derive(Debug, Clone)]
pub struct AliasRegistry {
    aliases: HashMap<String, String>,
}

impl AliasRegistry {
    /// Create a new registry with default aliases
    pub fn new() -> Self {
        let mut aliases = HashMap::new();
        for (alias, canonical) in DEFAULT_ALIASES {
            aliases.insert(alias.to_string(), canonical.to_string());
        }
        Self { aliases }
    }

    /// Create an empty registry
    pub fn empty() -> Self {
        Self {
            aliases: HashMap::new(),
        }
    }

    /// Add an alias
    pub fn add(&mut self, alias: impl Into<String>, canonical: impl Into<String>) {
        self.aliases.insert(alias.into(), canonical.into());
    }

    /// Remove an alias
    pub fn remove(&mut self, alias: &str) -> Option<String> {
        self.aliases.remove(alias)
    }

    /// Expand an alias to its canonical form
    /// Returns None if the input is not an alias
    pub fn expand(&self, input: &str) -> Option<&str> {
        self.aliases.get(input).map(|s| s.as_str())
    }

    /// Check if a string is an alias
    pub fn is_alias(&self, input: &str) -> bool {
        self.aliases.contains_key(input)
    }

    /// Get all aliases for a canonical command
    pub fn aliases_for(&self, canonical: &str) -> Vec<&str> {
        self.aliases
            .iter()
            .filter(|(_, v)| v.as_str() == canonical)
            .map(|(k, _)| k.as_str())
            .collect()
    }

    /// Get all registered aliases
    pub fn all(&self) -> &HashMap<String, String> {
        &self.aliases
    }

    /// Merge another registry into this one
    /// Later values override earlier ones
    pub fn merge(&mut self, other: &AliasRegistry) {
        for (alias, canonical) in &other.aliases {
            self.aliases.insert(alias.clone(), canonical.clone());
        }
    }

    /// Load aliases from a TOML-style config string
    pub fn from_config(config: &str) -> Result<Self, String> {
        let mut registry = Self::empty();

        for line in config.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            if let Some((alias, canonical)) = line.split_once('=') {
                let alias = alias.trim().trim_matches('"');
                let canonical = canonical.trim().trim_matches('"');
                registry.add(alias, canonical);
            }
        }

        Ok(registry)
    }
}

impl Default for AliasRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_aliases() {
        let registry = AliasRegistry::new();

        assert_eq!(registry.expand("q"), Some("search"));
        assert_eq!(registry.expand("h"), Some("help"));
        assert_eq!(registry.expand("?"), Some("help"));
        assert_eq!(registry.expand("c"), Some("config"));
    }

    #[test]
    fn test_custom_alias() {
        let mut registry = AliasRegistry::new();
        registry.add("ss", "sessions search");

        assert_eq!(registry.expand("ss"), Some("sessions search"));
    }

    #[test]
    fn test_aliases_for() {
        let registry = AliasRegistry::new();
        let search_aliases = registry.aliases_for("search");

        assert!(search_aliases.contains(&"q"));
        assert!(search_aliases.contains(&"query"));
        assert!(search_aliases.contains(&"find"));
    }

    #[test]
    fn test_from_config() {
        let config = r#"
            # Custom aliases
            ss = "sessions search"
            si = "sessions import"
        "#;

        let registry = AliasRegistry::from_config(config).unwrap();
        assert_eq!(registry.expand("ss"), Some("sessions search"));
        assert_eq!(registry.expand("si"), Some("sessions import"));
    }

    #[test]
    fn test_merge() {
        let mut base = AliasRegistry::new();
        let mut custom = AliasRegistry::empty();
        custom.add("custom", "mycommand");

        base.merge(&custom);
        assert_eq!(base.expand("custom"), Some("mycommand"));
        // Original aliases preserved
        assert_eq!(base.expand("q"), Some("search"));
    }

    #[test]
    fn test_empty_registry() {
        let registry = AliasRegistry::empty();
        assert!(registry.expand("q").is_none());
        assert!(!registry.is_alias("q"));
        assert!(registry.all().is_empty());
    }

    #[test]
    fn test_remove_alias() {
        let mut registry = AliasRegistry::new();
        assert_eq!(registry.expand("q"), Some("search"));
        let removed = registry.remove("q");
        assert_eq!(removed, Some("search".to_string()));
        assert!(registry.expand("q").is_none());
    }

    #[test]
    fn test_remove_nonexistent() {
        let mut registry = AliasRegistry::new();
        assert!(registry.remove("nonexistent").is_none());
    }

    #[test]
    fn test_is_alias() {
        let registry = AliasRegistry::new();
        assert!(registry.is_alias("q"));
        assert!(registry.is_alias("?"));
        assert!(!registry.is_alias("search"));
        assert!(!registry.is_alias(""));
    }

    #[test]
    fn test_add_duplicate_overwrites() {
        let mut registry = AliasRegistry::empty();
        registry.add("x", "cmd_a");
        registry.add("x", "cmd_b");
        assert_eq!(registry.expand("x"), Some("cmd_b"));
    }

    #[test]
    fn test_from_config_whitespace_lines() {
        let config = "\n\n  \n# comment\n";
        let registry = AliasRegistry::from_config(config).unwrap();
        assert!(registry.all().is_empty());
    }

    #[test]
    fn test_aliases_for_no_matches() {
        let registry = AliasRegistry::new();
        let matches = registry.aliases_for("nonexistent_command");
        assert!(matches.is_empty());
    }

    #[test]
    fn test_expand_empty_string() {
        let registry = AliasRegistry::new();
        assert!(registry.expand("").is_none());
    }
}

#[cfg(test)]
mod proptests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn expand_never_panics(input: String) {
            let registry = AliasRegistry::new();
            let _ = registry.expand(&input);
        }

        #[test]
        fn is_alias_never_panics(input: String) {
            let registry = AliasRegistry::new();
            let _ = registry.is_alias(&input);
        }

        #[test]
        fn add_then_expand_roundtrip(alias: String, canonical: String) {
            let mut registry = AliasRegistry::empty();
            registry.add(&alias, &canonical);
            prop_assert_eq!(registry.expand(&alias), Some(canonical.as_str()));
        }

        #[test]
        fn remove_then_expand_absent(alias: String, canonical: String) {
            let mut registry = AliasRegistry::empty();
            registry.add(&alias, &canonical);
            registry.remove(&alias);
            prop_assert!(registry.expand(&alias).is_none());
        }
    }
}
