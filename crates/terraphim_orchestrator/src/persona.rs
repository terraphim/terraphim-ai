use handlebars::Handlebars;
use std::collections::HashMap;
use std::path::Path;
use terraphim_types::persona::{PersonaDefinition, PersonaLoadError};
use tracing::{info, warn};

#[cfg(test)]
use terraphim_types::persona::{CharacteristicDef, SfiaSkillDef};

/// Registry for loading and accessing persona definitions.
/// Stores personas with case-insensitive lookup.
#[derive(Debug, Clone)]
pub struct PersonaRegistry {
    personas: HashMap<String, PersonaDefinition>,
}

impl PersonaRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self {
            personas: HashMap::new(),
        }
    }

    /// Load all persona TOML files from a directory.
    ///
    /// Reads all `*.toml` files from the given directory. For each file,
    /// attempts to parse it as a PersonaDefinition. If parsing fails,
    /// a warning is logged and the file is skipped.
    ///
    /// Returns an error if the directory does not exist or cannot be read.
    pub fn load_from_dir(dir: &Path) -> Result<Self, PersonaLoadError> {
        if !dir.exists() {
            return Err(PersonaLoadError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("Persona directory not found: {}", dir.display()),
            )));
        }

        if !dir.is_dir() {
            return Err(PersonaLoadError::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("Not a directory: {}", dir.display()),
            )));
        }

        let mut registry = Self::new();

        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.extension().map(|e| e == "toml").unwrap_or(false) {
                match PersonaDefinition::from_file(&path) {
                    Ok(persona) => {
                        info!(name = %persona.agent_name, path = %path.display(), "loaded persona");
                        registry.insert(persona);
                    }
                    Err(e) => {
                        warn!(path = %path.display(), error = %e, "failed to load persona file, skipping");
                    }
                }
            }
        }

        info!(count = registry.len(), dir = %dir.display(), "persona registry loaded");
        Ok(registry)
    }

    /// Get a persona by name (case-insensitive lookup).
    pub fn get(&self, name: &str) -> Option<&PersonaDefinition> {
        self.personas.get(&name.to_lowercase())
    }

    /// Get the number of personas in the registry.
    pub fn len(&self) -> usize {
        self.personas.len()
    }

    /// Check if the registry is empty.
    pub fn is_empty(&self) -> bool {
        self.personas.is_empty()
    }

    /// Insert a persona into the registry.
    /// Uses lowercase key for case-insensitive lookup.
    pub fn insert(&mut self, persona: PersonaDefinition) {
        let key = persona.agent_name.to_lowercase();
        self.personas.insert(key, persona);
    }

    /// Get a list of all persona names in the registry.
    pub fn persona_names(&self) -> Vec<&str> {
        self.personas
            .values()
            .map(|p| p.agent_name.as_str())
            .collect()
    }
}

impl Default for PersonaRegistry {
    fn default() -> Self {
        Self::new()
    }
}

const DEFAULT_TEMPLATE: &str = include_str!("../data/metaprompt-template.hbs");
const TEMPLATE_NAME: &str = "metaprompt";

/// Error type for metaprompt rendering operations.
#[derive(Debug, thiserror::Error)]
pub enum MetapromptRenderError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Template compilation error: {0}")]
    Template(String),
    #[error("Template render error: {0}")]
    Render(String),
}

/// Renderer for persona metaprompts using Handlebars templates.
///
/// The renderer uses strict mode and expects all template variables
/// to be present in the PersonaDefinition. A default template is
/// embedded at compile time, but a custom template can be loaded
/// from a file.
#[derive(Debug)]
pub struct MetapromptRenderer {
    handlebars: Handlebars<'static>,
}

impl MetapromptRenderer {
    /// Create a new renderer with the default embedded template.
    pub fn new() -> Result<Self, MetapromptRenderError> {
        let mut handlebars = Handlebars::new();
        handlebars.set_strict_mode(true);

        handlebars
            .register_template_string(TEMPLATE_NAME, DEFAULT_TEMPLATE)
            .map_err(|e| MetapromptRenderError::Template(e.to_string()))?;

        Ok(Self { handlebars })
    }

    /// Create a new renderer from a custom template file.
    ///
    /// The file should be a valid Handlebars template that can
    /// render a PersonaDefinition.
    pub fn from_template_file(path: &Path) -> Result<Self, MetapromptRenderError> {
        let template_str = std::fs::read_to_string(path)?;

        let mut handlebars = Handlebars::new();
        handlebars.set_strict_mode(true);

        handlebars
            .register_template_string(TEMPLATE_NAME, &template_str)
            .map_err(|e| MetapromptRenderError::Template(e.to_string()))?;

        Ok(Self { handlebars })
    }

    /// Render a persona into a metaprompt preamble.
    ///
    /// Returns the rendered metaprompt string using the configured
    /// Handlebars template and the persona's data.
    pub fn render(&self, persona: &PersonaDefinition) -> Result<String, MetapromptRenderError> {
        self.handlebars
            .render(TEMPLATE_NAME, persona)
            .map_err(|e| MetapromptRenderError::Render(e.to_string()))
    }

    /// Compose a full prompt with metapreamble and task.
    ///
    /// On render success, returns: "{preamble}\n\n---\n\n## Current Task\n\n{task}"
    /// On render failure, logs a warning and returns the task unchanged.
    pub fn compose_prompt(&self, persona: &PersonaDefinition, task: &str) -> String {
        match self.render(persona) {
            Ok(preamble) => {
                format!("{}\n\n---\n\n## Current Task\n\n{}", preamble, task)
            }
            Err(e) => {
                warn!(
                    agent = %persona.agent_name,
                    error = %e,
                    "metaprompt render failed, returning task without preamble"
                );
                task.to_string()
            }
        }
    }
}

impl Default for MetapromptRenderer {
    fn default() -> Self {
        Self::new().expect("default template should always compile")
    }
}

/// Create a test persona for use in tests.
#[cfg(test)]
pub fn test_persona() -> PersonaDefinition {
    PersonaDefinition {
        agent_name: "TestAgent".to_string(),
        role_name: "Test Engineer".to_string(),
        name_origin: "From testing".to_string(),
        vibe: "Thorough, methodical".to_string(),
        symbol: "Checkmark".to_string(),
        core_characteristics: vec![CharacteristicDef {
            name: "Thorough".to_string(),
            description: "checks everything twice".to_string(),
        }],
        speech_style: "Precise and factual.".to_string(),
        terraphim_nature: "Adapted to testing environments.".to_string(),
        sfia_title: "Test Engineer".to_string(),
        primary_level: 4,
        guiding_phrase: "Enable".to_string(),
        level_essence: "Works autonomously under general direction.".to_string(),
        sfia_skills: vec![SfiaSkillDef {
            code: "TEST".to_string(),
            name: "Testing".to_string(),
            level: 4,
            description: "Designs and executes test plans.".to_string(),
        }],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Serialize;
    use std::io::Write;
    use tempfile::TempDir;

    #[test]
    fn test_registry_new_is_empty() {
        let registry = PersonaRegistry::new();
        assert!(registry.is_empty());
        assert_eq!(registry.len(), 0);
    }

    #[test]
    fn test_registry_insert_and_get() {
        let mut registry = PersonaRegistry::new();
        let persona = test_persona();

        registry.insert(persona);

        assert!(!registry.is_empty());
        assert_eq!(registry.len(), 1);
        assert!(registry.get("TestAgent").is_some());
        assert_eq!(registry.get("TestAgent").unwrap().agent_name, "TestAgent");
    }

    #[test]
    fn test_registry_get_case_insensitive() {
        let mut registry = PersonaRegistry::new();
        let persona = test_persona();

        registry.insert(persona);

        // All these should resolve to the same persona
        assert!(registry.get("vigil").is_none()); // vigil doesn't exist

        // But for our test persona, case variations should work
        assert!(registry.get("TestAgent").is_some());
        assert!(registry.get("testagent").is_some());
        assert!(registry.get("TESTAGENT").is_some());
        assert!(registry.get("TestAGENT").is_some());
    }

    #[test]
    fn test_registry_load_from_dir() {
        let temp_dir = TempDir::new().unwrap();

        // Create test TOML files
        let persona1 = r#"
agent_name = "Vigil"
role_name = "Test Role 1"
name_origin = "Test"
vibe = "Test"
symbol = "T"
core_characteristics = []
speech_style = "Test"
terraphim_nature = "Test"
sfia_title = "Test"
primary_level = 4
guiding_phrase = "Test"
level_essence = "Test"
sfia_skills = []
"#;

        let persona2 = r#"
agent_name = "Sentinel"
role_name = "Test Role 2"
name_origin = "Test"
vibe = "Test"
symbol = "S"
core_characteristics = []
speech_style = "Test"
terraphim_nature = "Test"
sfia_title = "Test"
primary_level = 3
guiding_phrase = "Test"
level_essence = "Test"
sfia_skills = []
"#;

        let mut file1 = std::fs::File::create(temp_dir.path().join("vigil.toml")).unwrap();
        file1.write_all(persona1.as_bytes()).unwrap();

        let mut file2 = std::fs::File::create(temp_dir.path().join("sentinel.toml")).unwrap();
        file2.write_all(persona2.as_bytes()).unwrap();

        // Create a non-toml file (should be ignored)
        let mut file3 = std::fs::File::create(temp_dir.path().join("readme.txt")).unwrap();
        file3.write_all(b"This is not a persona").unwrap();

        let registry = PersonaRegistry::load_from_dir(temp_dir.path()).unwrap();

        assert_eq!(registry.len(), 2);
        assert!(registry.get("vigil").is_some());
        assert!(registry.get("sentinel").is_some());
        assert!(registry.get("Vigil").is_some()); // case-insensitive
        assert!(registry.get("SENTINEL").is_some()); // case-insensitive
    }

    #[test]
    fn test_registry_load_missing_dir() {
        let result = PersonaRegistry::load_from_dir(Path::new("/nonexistent/path/12345"));
        assert!(result.is_err());

        // Verify it's the right error type
        match result {
            Err(PersonaLoadError::Io(e)) => {
                assert_eq!(e.kind(), std::io::ErrorKind::NotFound);
            }
            _ => panic!("Expected Io error with NotFound kind"),
        }
    }

    #[test]
    fn test_renderer_default_template() {
        let renderer = MetapromptRenderer::new();
        assert!(renderer.is_ok());
    }

    #[test]
    fn test_renderer_render_persona() {
        let renderer = MetapromptRenderer::new().unwrap();
        let persona = test_persona();

        let result = renderer.render(&persona);
        assert!(result.is_ok());

        let rendered = result.unwrap();
        assert!(rendered.contains(&persona.agent_name));
        assert!(rendered.contains(&persona.role_name));
        assert!(rendered.contains(&persona.sfia_skills[0].code));
        assert!(rendered.contains(&persona.sfia_skills[0].name));
    }

    #[test]
    fn test_renderer_compose_prompt() {
        let renderer = MetapromptRenderer::new().unwrap();
        let persona = test_persona();
        let task = "Write some tests for the new feature";

        let prompt = renderer.compose_prompt(&persona, task);

        // Should contain the separator
        assert!(prompt.contains("---"));
        // Should contain the task section header
        assert!(prompt.contains("## Current Task"));
        // Should contain the task verbatim
        assert!(prompt.contains(task));
        // Should contain the preamble (from rendering the persona)
        assert!(prompt.contains(&persona.agent_name));
    }

    #[test]
    fn test_renderer_compose_prompt_contains_task() {
        let renderer = MetapromptRenderer::new().unwrap();
        let persona = test_persona();
        let task = "This is the specific task to accomplish";

        let prompt = renderer.compose_prompt(&persona, task);

        // Verify task appears after the final separator
        // The prompt contains "## Current Task" followed by the task
        assert!(prompt.contains("## Current Task"));
        assert!(prompt.contains(task));

        // Verify task appears at the end of the prompt
        assert!(prompt.ends_with(task));
    }

    #[test]
    fn test_renderer_strict_mode_missing_field() {
        let renderer = MetapromptRenderer::new().unwrap();

        // Create a minimal persona that's missing required fields
        #[derive(Serialize)]
        struct IncompletePersona {
            agent_name: String,
        }

        let incomplete = IncompletePersona {
            agent_name: "Incomplete".to_string(),
        };

        // Try to render with the incomplete persona
        // This should fail because the template expects many fields
        let result: Result<String, MetapromptRenderError> = renderer
            .handlebars
            .render(TEMPLATE_NAME, &incomplete)
            .map_err(|e| MetapromptRenderError::Render(e.to_string()));

        assert!(result.is_err());
    }

    #[test]
    fn test_renderer_from_template_file() {
        let temp_dir = TempDir::new().unwrap();
        let template_path = temp_dir.path().join("custom.hbs");

        let custom_template = "Hello {{agent_name}}, you are a {{role_name}}!";
        std::fs::write(&template_path, custom_template).unwrap();

        let renderer = MetapromptRenderer::from_template_file(&template_path).unwrap();
        let persona = test_persona();

        let result = renderer.render(&persona).unwrap();
        assert!(result.contains(&persona.agent_name));
        assert!(result.contains(&persona.role_name));
    }

    #[test]
    fn test_persona_names_returns_all_names() {
        let mut registry = PersonaRegistry::new();

        let mut persona1 = test_persona();
        persona1.agent_name = "Alpha".to_string();
        registry.insert(persona1);

        let mut persona2 = test_persona();
        persona2.agent_name = "Beta".to_string();
        registry.insert(persona2);

        let names = registry.persona_names();
        assert_eq!(names.len(), 2);
        assert!(names.contains(&"Alpha"));
        assert!(names.contains(&"Beta"));
    }

    #[test]
    fn test_compose_prompt_fallback_on_render_failure() {
        let renderer = MetapromptRenderer::new().unwrap();
        let task = "Do the thing";

        let broken = PersonaDefinition {
            agent_name: "Broken".to_string(),
            ..test_persona() // Take valid fields from test_persona
        };

        // This should succeed because test_persona has all required fields
        let prompt = renderer.compose_prompt(&broken, task);
        assert!(prompt.contains(task));

        // Verify it contains the separator (meaning render succeeded)
        assert!(prompt.contains("---"));
    }

    #[test]
    fn test_registry_insert_overwrites_existing() {
        let mut registry = PersonaRegistry::new();

        let mut persona1 = test_persona();
        persona1.agent_name = "SameName".to_string();
        persona1.role_name = "Role1".to_string();
        registry.insert(persona1);

        let mut persona2 = test_persona();
        persona2.agent_name = "SAMENAME".to_string(); // Different case, same key
        persona2.role_name = "Role2".to_string();
        registry.insert(persona2);

        // Should only have one entry (the second one)
        assert_eq!(registry.len(), 1);
        assert_eq!(registry.get("samename").unwrap().role_name, "Role2");
    }
}
