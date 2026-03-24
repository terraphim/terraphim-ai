//! Persona definition types for agent personas with SFIA skill framework support.
//!
//! This module provides types for defining agent personas with:
//! - Core characteristics and personality traits
//! - SFIA (Skills Framework for the Information Age) skill definitions
//! - TOML serialization/deserialization for persona configuration files
//!
//! # Example TOML
//!
//! ```toml
//! agent_name = "Terraphim Architect"
//! role_name = "Systems Architect"
//! name_origin = "Greek: Terra (Earth) + phainein (to show)"
//! vibe = "Thoughtful, grounded, precise, architectural"
//! symbol = "⚡"
//! speech_style = "Technical yet accessible"
//! terraphim_nature = "Earth spirit of knowledge architecture"
//! sfia_title = "Solution Architect"
//! primary_level = 5
//! guiding_phrase = "Structure precedes function"
//! level_essence = "Enables and ensures"
//!
//! [[core_characteristics]]
//! name = "Systems Thinking"
//! description = "Views problems holistically"
//!
//! [[core_characteristics]]
//! name = "Pattern Recognition"
//! description = "Identifies recurring structures"
//!
//! [[sfia_skills]]
//! code = "ARCH"
//! name = "Solution Architecture"
//! level = 5
//! description = "Designs and communicates solution architectures"
//! ```

use serde::{Deserialize, Serialize};
use std::path::Path;

/// A complete persona definition for an AI agent.
///
/// This struct captures both the personality characteristics and
/// professional skills (via SFIA framework) of an agent persona.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PersonaDefinition {
    /// The agent's display name
    pub agent_name: String,
    /// The role/title of the agent
    pub role_name: String,
    /// Explanation of the agent's name origin
    pub name_origin: String,
    /// The overall vibe/personality of the agent
    pub vibe: String,
    /// Symbol or emoji representing the agent
    pub symbol: String,
    /// Core personality characteristics
    #[serde(default)]
    pub core_characteristics: Vec<CharacteristicDef>,
    /// How the agent speaks (style description)
    pub speech_style: String,
    /// Description of the agent's nature/persona
    pub terraphim_nature: String,
    /// SFIA professional title
    pub sfia_title: String,
    /// Primary SFIA skill level (typically 1-7)
    pub primary_level: u8,
    /// A guiding phrase for the persona
    pub guiding_phrase: String,
    /// Description of what the level represents
    pub level_essence: String,
    /// SFIA skills possessed by this persona
    #[serde(default)]
    pub sfia_skills: Vec<SfiaSkillDef>,
}

/// Definition of a core personality characteristic.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CharacteristicDef {
    /// Name of the characteristic
    pub name: String,
    /// Description of how this characteristic manifests
    pub description: String,
}

/// SFIA skill definition.
///
/// SFIA (Skills Framework for the Information Age) provides a common
/// reference model for skills in the IT industry.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SfiaSkillDef {
    /// SFIA skill code (e.g., "ARCH", "DESN")
    pub code: String,
    /// Full name of the skill
    pub name: String,
    /// Skill level (typically 1-7 in SFIA framework)
    pub level: u8,
    /// Description of skill at this level
    pub description: String,
}

impl PersonaDefinition {
    /// Parse a PersonaDefinition from a TOML string.
    ///
    /// # Arguments
    ///
    /// * `toml_str` - The TOML string to parse
    ///
    /// # Returns
    ///
    /// Returns `Ok(PersonaDefinition)` on success, or `Err(toml::de::Error)`
    /// if parsing fails.
    ///
    /// # Example
    ///
    /// ```
    /// use terraphim_types::PersonaDefinition;
    ///
    /// let toml = r#"
    ///     agent_name = "Test Agent"
    ///     role_name = "Tester"
    ///     name_origin = "Test"
    ///     vibe = "Helpful"
    ///     symbol = "T"
    ///     speech_style = "Clear"
    ///     terraphim_nature = "Test nature"
    ///     sfia_title = "Test Engineer"
    ///     primary_level = 3
    ///     guiding_phrase = "Test everything"
    ///     level_essence = "Ensures quality"
    /// "#;
    ///
    /// let persona = PersonaDefinition::from_toml(toml).unwrap();
    /// assert_eq!(persona.agent_name, "Test Agent");
    /// ```
    pub fn from_toml(toml_str: &str) -> Result<Self, toml::de::Error> {
        toml::from_str(toml_str)
    }

    /// Load a PersonaDefinition from a file.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the TOML file
    ///
    /// # Returns
    ///
    /// Returns `Ok(PersonaDefinition)` on success, or `Err(PersonaLoadError)`
    /// if the file cannot be read or parsed.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use terraphim_types::PersonaDefinition;
    ///
    /// let persona = PersonaDefinition::from_file("/path/to/persona.toml").unwrap();
    /// ```
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self, PersonaLoadError> {
        let content = std::fs::read_to_string(path.as_ref()).map_err(PersonaLoadError::Io)?;
        Self::from_toml(&content).map_err(|e| PersonaLoadError::Parse(e.to_string()))
    }

    /// Serialize the persona to a TOML string.
    ///
    /// # Returns
    ///
    /// Returns `Ok(String)` containing the TOML representation, or
    /// `Err(toml::ser::Error)` if serialization fails.
    ///
    /// # Example
    ///
    /// ```
    /// use terraphim_types::PersonaDefinition;
    ///
    /// let toml = r#"
    ///     agent_name = "Test Agent"
    ///     role_name = "Tester"
    ///     name_origin = "Test"
    ///     vibe = "Helpful"
    ///     symbol = "T"
    ///     speech_style = "Clear"
    ///     terraphim_nature = "Test nature"
    ///     sfia_title = "Test Engineer"
    ///     primary_level = 3
    ///     guiding_phrase = "Test everything"
    ///     level_essence = "Ensures quality"
    /// "#;
    ///
    /// let persona = PersonaDefinition::from_toml(toml).unwrap();
    /// let output = persona.to_toml().unwrap();
    /// assert!(output.contains("agent_name = \"Test Agent\""));
    /// ```
    pub fn to_toml(&self) -> Result<String, toml::ser::Error> {
        toml::to_string_pretty(self)
    }
}

/// Errors that can occur when loading a persona definition.
#[derive(Debug, thiserror::Error)]
pub enum PersonaLoadError {
    /// IO error when reading the persona file.
    #[error("IO error reading persona file: {0}")]
    Io(#[from] std::io::Error),
    /// TOML parsing error.
    #[error("TOML parse error: {0}")]
    Parse(String),
    /// Persona not found at the specified path.
    #[error("Persona not found: {0}")]
    NotFound(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env::temp_dir;
    use std::fs;

    /// Minimal valid TOML parses into PersonaDefinition
    #[test]
    fn test_persona_from_toml_minimal() {
        let toml = r#"
            agent_name = "Test Agent"
            role_name = "Tester"
            name_origin = "Test"
            vibe = "Helpful"
            symbol = "T"
            speech_style = "Clear"
            terraphim_nature = "Test nature"
            sfia_title = "Test Engineer"
            primary_level = 3
            guiding_phrase = "Test everything"
            level_essence = "Ensures quality"
        "#;

        let persona = PersonaDefinition::from_toml(toml).unwrap();
        assert_eq!(persona.agent_name, "Test Agent");
        assert_eq!(persona.role_name, "Tester");
        assert_eq!(persona.primary_level, 3);
        assert!(persona.core_characteristics.is_empty());
        assert!(persona.sfia_skills.is_empty());
    }

    /// Full persona TOML with all fields parses correctly
    #[test]
    fn test_persona_from_toml_full() {
        let toml = r#"
            agent_name = "Terraphim Architect"
            role_name = "Systems Architect"
            name_origin = "Greek: Terra (Earth) + phainein (to show)"
            vibe = "Thoughtful, grounded, precise, architectural"
            symbol = "⚡"
            speech_style = "Technical yet accessible"
            terraphim_nature = "Earth spirit of knowledge architecture"
            sfia_title = "Solution Architect"
            primary_level = 5
            guiding_phrase = "Structure precedes function"
            level_essence = "Enables and ensures"

            [[core_characteristics]]
            name = "Systems Thinking"
            description = "Views problems holistically"

            [[core_characteristics]]
            name = "Pattern Recognition"
            description = "Identifies recurring structures"

            [[sfia_skills]]
            code = "ARCH"
            name = "Solution Architecture"
            level = 5
            description = "Designs and communicates solution architectures"

            [[sfia_skills]]
            code = "DESN"
            name = "Systems Design"
            level = 5
            description = "Specifies and designs large-scale systems"
        "#;

        let persona = PersonaDefinition::from_toml(toml).unwrap();
        assert_eq!(persona.agent_name, "Terraphim Architect");
        assert_eq!(persona.symbol, "⚡");
        assert_eq!(persona.primary_level, 5);
        assert_eq!(persona.core_characteristics.len(), 2);
        assert_eq!(persona.core_characteristics[0].name, "Systems Thinking");
        assert_eq!(persona.sfia_skills.len(), 2);
        assert_eq!(persona.sfia_skills[0].code, "ARCH");
        assert_eq!(persona.sfia_skills[1].name, "Systems Design");
    }

    /// from_toml(to_toml(def)) produces identical struct
    #[test]
    fn test_persona_roundtrip() {
        let toml = r#"
            agent_name = "Test Agent"
            role_name = "Tester"
            name_origin = "Test"
            vibe = "Helpful"
            symbol = "T"
            speech_style = "Clear"
            terraphim_nature = "Test nature"
            sfia_title = "Test Engineer"
            primary_level = 3
            guiding_phrase = "Test everything"
            level_essence = "Ensures quality"

            [[core_characteristics]]
            name = "Test Char"
            description = "A test characteristic"

            [[sfia_skills]]
            code = "TEST"
            name = "Testing"
            level = 3
            description = "Tests things"
        "#;

        let persona = PersonaDefinition::from_toml(toml).unwrap();
        let output = persona.to_toml().unwrap();
        let reparsed = PersonaDefinition::from_toml(&output).unwrap();

        assert_eq!(persona, reparsed);
    }

    /// Missing agent_name returns parse error
    #[test]
    fn test_persona_missing_required_field() {
        let toml = r#"
            role_name = "Tester"
            name_origin = "Test"
            vibe = "Helpful"
            symbol = "T"
            speech_style = "Clear"
            terraphim_nature = "Test nature"
            sfia_title = "Test Engineer"
            primary_level = 3
            guiding_phrase = "Test everything"
            level_essence = "Ensures quality"
        "#;

        let result = PersonaDefinition::from_toml(toml);
        assert!(result.is_err());
    }

    /// Array of {name, description} objects parses
    #[test]
    fn test_persona_characteristic_parsing() {
        let toml = r#"
            agent_name = "Test"
            role_name = "Tester"
            name_origin = "Test"
            vibe = "Helpful"
            symbol = "T"
            speech_style = "Clear"
            terraphim_nature = "Test"
            sfia_title = "Tester"
            primary_level = 3
            guiding_phrase = "Test"
            level_essence = "Test"

            [[core_characteristics]]
            name = "First"
            description = "First characteristic"

            [[core_characteristics]]
            name = "Second"
            description = "Second characteristic"

            [[core_characteristics]]
            name = "Third"
            description = "Third characteristic"
        "#;

        let persona = PersonaDefinition::from_toml(toml).unwrap();
        assert_eq!(persona.core_characteristics.len(), 3);
        assert_eq!(persona.core_characteristics[1].name, "Second");
        assert_eq!(
            persona.core_characteristics[1].description,
            "Second characteristic"
        );
    }

    /// Array of {code, name, level, description} objects parses
    #[test]
    fn test_persona_sfia_skill_parsing() {
        let toml = r#"
            agent_name = "Test"
            role_name = "Tester"
            name_origin = "Test"
            vibe = "Helpful"
            symbol = "T"
            speech_style = "Clear"
            terraphim_nature = "Test"
            sfia_title = "Tester"
            primary_level = 3
            guiding_phrase = "Test"
            level_essence = "Test"

            [[sfia_skills]]
            code = "CODE1"
            name = "Skill One"
            level = 2
            description = "First skill"

            [[sfia_skills]]
            code = "CODE2"
            name = "Skill Two"
            level = 4
            description = "Second skill"
        "#;

        let persona = PersonaDefinition::from_toml(toml).unwrap();
        assert_eq!(persona.sfia_skills.len(), 2);
        assert_eq!(persona.sfia_skills[0].code, "CODE1");
        assert_eq!(persona.sfia_skills[0].level, 2);
        assert_eq!(persona.sfia_skills[1].name, "Skill Two");
        assert_eq!(persona.sfia_skills[1].level, 4);
    }

    /// Level 0 and level 8 are accepted (no range enforcement at type level)
    #[test]
    fn test_persona_sfia_level_bounds() {
        let toml = r#"
            agent_name = "Test"
            role_name = "Tester"
            name_origin = "Test"
            vibe = "Helpful"
            symbol = "T"
            speech_style = "Clear"
            terraphim_nature = "Test"
            sfia_title = "Tester"
            primary_level = 0
            guiding_phrase = "Test"
            level_essence = "Test"

            [[sfia_skills]]
            code = "ZERO"
            name = "Zero Level"
            level = 0
            description = "Level zero"

            [[sfia_skills]]
            code = "EIGHT"
            name = "Eight Level"
            level = 8
            description = "Level eight"
        "#;

        let persona = PersonaDefinition::from_toml(toml).unwrap();
        assert_eq!(persona.primary_level, 0);
        assert_eq!(persona.sfia_skills[0].level, 0);
        assert_eq!(persona.sfia_skills[1].level, 8);
    }

    /// Missing file returns PersonaLoadError::Io
    #[test]
    fn test_persona_from_file_not_found() {
        let path = temp_dir().join("nonexistent_persona_12345.toml");
        let result = PersonaDefinition::from_file(&path);

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("IO error"));
    }

    /// Invalid TOML returns PersonaLoadError::Parse
    #[test]
    fn test_persona_from_file_invalid_toml() {
        let temp_file = temp_dir().join("invalid_persona_test.toml");
        fs::write(&temp_file, "this is not valid toml = [").unwrap();

        let result = PersonaDefinition::from_file(&temp_file);
        fs::remove_file(&temp_file).unwrap();

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("TOML parse error"));
    }

    /// Clone and PartialEq derive work correctly
    #[test]
    fn test_persona_definition_clone_eq() {
        let toml = r#"
            agent_name = "Test Agent"
            role_name = "Tester"
            name_origin = "Test"
            vibe = "Helpful"
            symbol = "T"
            speech_style = "Clear"
            terraphim_nature = "Test nature"
            sfia_title = "Test Engineer"
            primary_level = 3
            guiding_phrase = "Test everything"
            level_essence = "Ensures quality"
        "#;

        let persona = PersonaDefinition::from_toml(toml).unwrap();
        let cloned = persona.clone();

        assert_eq!(persona, cloned);
        assert!(persona.agent_name == cloned.agent_name);
    }
}
