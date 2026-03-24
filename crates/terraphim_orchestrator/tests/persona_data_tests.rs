//! Integration tests for persona data files
//!
//! Tests that all persona TOML files can be loaded and parsed correctly,
//! and that the metaprompt template renders without errors.

use std::path::PathBuf;
use terraphim_types::PersonaDefinition;

/// Get the path to the data/personas directory from the crate root
fn personas_dir() -> PathBuf {
    // CARGO_MANIFEST_DIR is crates/terraphim_orchestrator/
    // We need to go up two levels to reach the repo root, then into data/personas
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("data/personas")
        .canonicalize()
        .expect("Failed to canonicalize personas directory path")
}

/// Get the path to the metaprompt template
fn metaprompt_template_path() -> PathBuf {
    personas_dir().join("metaprompt-template.hbs")
}

/// Ferrox TOML parses into valid PersonaDefinition
#[test]
fn test_ferrox_toml_parses() {
    let path = personas_dir().join("ferrox.toml");
    let persona = PersonaDefinition::from_file(&path).expect("Failed to parse ferrox.toml");

    assert_eq!(persona.agent_name, "Ferrox");
    assert_eq!(persona.role_name, "Rust Engineer");
    assert_eq!(persona.primary_level, 5);
    assert_eq!(persona.sfia_title, "Principal Software Engineer");
    assert_eq!(persona.core_characteristics.len(), 5);
    assert_eq!(persona.sfia_skills.len(), 4);
}

/// Vigil TOML parses into valid PersonaDefinition
#[test]
fn test_vigil_toml_parses() {
    let path = personas_dir().join("vigil.toml");
    let persona = PersonaDefinition::from_file(&path).expect("Failed to parse vigil.toml");

    assert_eq!(persona.agent_name, "Vigil");
    assert_eq!(persona.role_name, "Security Engineer");
    assert_eq!(persona.primary_level, 5);
    assert_eq!(persona.sfia_title, "Principal Security Engineer");
    assert_eq!(persona.core_characteristics.len(), 5);
    assert_eq!(persona.sfia_skills.len(), 4);
}

/// Carthos TOML parses into valid PersonaDefinition
#[test]
fn test_carthos_toml_parses() {
    let path = personas_dir().join("carthos.toml");
    let persona = PersonaDefinition::from_file(&path).expect("Failed to parse carthos.toml");

    assert_eq!(persona.agent_name, "Carthos");
    assert_eq!(persona.role_name, "Domain Architect");
    assert_eq!(persona.primary_level, 5);
    assert_eq!(persona.sfia_title, "Principal Solution Architect");
    assert_eq!(persona.core_characteristics.len(), 5);
    assert_eq!(persona.sfia_skills.len(), 3);
}

/// Lux TOML parses into valid PersonaDefinition
#[test]
fn test_lux_toml_parses() {
    let path = personas_dir().join("lux.toml");
    let persona = PersonaDefinition::from_file(&path).expect("Failed to parse lux.toml");

    assert_eq!(persona.agent_name, "Lux");
    assert_eq!(persona.role_name, "TypeScript Engineer");
    assert_eq!(persona.primary_level, 4);
    assert_eq!(persona.sfia_title, "Senior Frontend Engineer");
    assert_eq!(persona.core_characteristics.len(), 5);
    assert_eq!(persona.sfia_skills.len(), 4);
}

/// Conduit TOML parses into valid PersonaDefinition
#[test]
fn test_conduit_toml_parses() {
    let path = personas_dir().join("conduit.toml");
    let persona = PersonaDefinition::from_file(&path).expect("Failed to parse conduit.toml");

    assert_eq!(persona.agent_name, "Conduit");
    assert_eq!(persona.role_name, "DevOps Engineer");
    assert_eq!(persona.primary_level, 4);
    assert_eq!(persona.sfia_title, "Senior DevOps Engineer");
    assert_eq!(persona.core_characteristics.len(), 5);
    assert_eq!(persona.sfia_skills.len(), 3);
}

/// Meridian TOML parses into valid PersonaDefinition
#[test]
fn test_meridian_toml_parses() {
    let path = personas_dir().join("meridian.toml");
    let persona = PersonaDefinition::from_file(&path).expect("Failed to parse meridian.toml");

    assert_eq!(persona.agent_name, "Meridian");
    assert_eq!(persona.role_name, "Market Researcher");
    assert_eq!(persona.primary_level, 4);
    assert_eq!(persona.sfia_title, "Senior Research Analyst");
    assert_eq!(persona.core_characteristics.len(), 5);
    assert_eq!(persona.sfia_skills.len(), 2);
}

/// Mneme TOML parses into valid PersonaDefinition
#[test]
fn test_mneme_toml_parses() {
    let path = personas_dir().join("mneme.toml");
    let persona = PersonaDefinition::from_file(&path).expect("Failed to parse mneme.toml");

    assert_eq!(persona.agent_name, "Mneme");
    assert_eq!(persona.role_name, "Meta-Learning Agent");
    assert_eq!(persona.primary_level, 5);
    assert_eq!(persona.sfia_title, "Principal Knowledge Engineer");
    assert_eq!(persona.core_characteristics.len(), 5);
    assert_eq!(persona.sfia_skills.len(), 3);
}

/// Echo TOML parses into valid PersonaDefinition
#[test]
fn test_echo_toml_parses() {
    let path = personas_dir().join("echo.toml");
    let persona = PersonaDefinition::from_file(&path).expect("Failed to parse echo.toml");

    assert_eq!(persona.agent_name, "Echo");
    assert_eq!(persona.role_name, "Twin Maintainer");
    assert_eq!(persona.primary_level, 4);
    assert_eq!(persona.sfia_title, "Senior Integration Engineer");
    assert_eq!(persona.core_characteristics.len(), 5);
    assert_eq!(persona.sfia_skills.len(), 4);
}

/// All persona files can be loaded into a registry
#[test]
fn test_all_personas_load_into_registry() {
    let dir = personas_dir();
    let entries = std::fs::read_dir(&dir).expect("Failed to read personas directory");

    let mut personas: Vec<PersonaDefinition> = Vec::new();

    for entry in entries {
        let entry = entry.expect("Failed to read directory entry");
        let path = entry.path();

        // Skip non-TOML files (like the metaprompt template)
        if path.extension().is_some_and(|ext| ext == "toml") {
            let persona = PersonaDefinition::from_file(&path)
                .unwrap_or_else(|_| panic!("Failed to parse {:?}", path));
            personas.push(persona);
        }
    }

    // Should have exactly 8 personas
    assert_eq!(personas.len(), 8, "Expected 8 persona TOML files");

    // Verify all have unique agent names
    let names: Vec<_> = personas.iter().map(|p| &p.agent_name).collect();
    let unique_names: std::collections::HashSet<_> = names.iter().cloned().collect();
    assert_eq!(
        names.len(),
        unique_names.len(),
        "All agent names should be unique"
    );
}

/// All personas render through the metaprompt template without error
#[test]
fn test_all_personas_render_without_error() {
    use handlebars::Handlebars;
    use serde_json::json;

    let template_path = metaprompt_template_path();
    let template_content =
        std::fs::read_to_string(&template_path).expect("Failed to read metaprompt template");

    let mut handlebars = Handlebars::new();
    handlebars
        .register_template_string("metaprompt", &template_content)
        .expect("Failed to register template");

    let dir = personas_dir();
    let entries = std::fs::read_dir(&dir).expect("Failed to read personas directory");

    for entry in entries {
        let entry = entry.expect("Failed to read directory entry");
        let path = entry.path();

        // Skip non-TOML files
        if path.extension().is_some_and(|ext| ext == "toml") {
            let persona = PersonaDefinition::from_file(&path)
                .unwrap_or_else(|_| panic!("Failed to parse {:?}", path));

            // Convert persona to JSON for Handlebars rendering
            let persona_json = json!({
                "agent_name": persona.agent_name,
                "role_name": persona.role_name,
                "name_origin": persona.name_origin,
                "vibe": persona.vibe,
                "symbol": persona.symbol,
                "speech_style": persona.speech_style,
                "terraphim_nature": persona.terraphim_nature,
                "sfia_title": persona.sfia_title,
                "primary_level": persona.primary_level,
                "guiding_phrase": persona.guiding_phrase,
                "level_essence": persona.level_essence,
                "core_characteristics": persona.core_characteristics.iter().map(|c| {
                    json!({
                        "name": c.name,
                        "description": c.description
                    })
                }).collect::<Vec<_>>(),
                "sfia_skills": persona.sfia_skills.iter().map(|s| {
                    json!({
                        "code": s.code,
                        "name": s.name,
                        "level": s.level,
                        "description": s.description
                    })
                }).collect::<Vec<_>>()
            });

            let rendered = handlebars
                .render("metaprompt", &persona_json)
                .unwrap_or_else(|_| panic!("Failed to render template for {:?}", path));

            // Basic assertions on rendered content
            assert!(
                rendered.contains(&persona.agent_name),
                "Rendered output should contain agent name"
            );
            assert!(
                rendered.contains(&persona.role_name),
                "Rendered output should contain role name"
            );
            assert!(
                rendered.contains(&persona.sfia_title),
                "Rendered output should contain SFIA title"
            );

            // Verify core characteristics are rendered
            for char in &persona.core_characteristics {
                assert!(
                    rendered.contains(&char.name),
                    "Rendered output should contain characteristic: {}",
                    char.name
                );
            }

            // Verify SFIA skills are rendered
            for skill in &persona.sfia_skills {
                assert!(
                    rendered.contains(&skill.code),
                    "Rendered output should contain skill code: {}",
                    skill.code
                );
            }
        }
    }
}
