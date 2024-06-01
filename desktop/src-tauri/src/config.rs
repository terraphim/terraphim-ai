use std::path::PathBuf;

use ahash::AHashMap;
use tauri::Url;
use terraphim_automata::AutomataPath;
use terraphim_config::{
    Config, ConfigBuilder, Haystack, KnowledgeGraph, Role, ServiceType, TerraphimConfigError,
};
use terraphim_types::{KnowledgeGraphInputType, RelevanceFunction};

/// The path to the default haystack directory
// TODO: Replace this with a file-based config loader based on `twelf` in the
// future
const DEFAULT_HAYSTACK_PATH: &str = "../../docs/";

/// Load the default config
///
// TODO: Replace this with a file-based config loader based on `twelf` in the
// future
pub(crate) fn load_config() -> Result<Config, TerraphimConfigError> {
    let automata_path = AutomataPath::from_local("data/term_to_id.json");

    // Create the path to the default haystack directory
    // by concating the current directory with the default haystack path
    let docs_path = std::env::current_dir()?.join(DEFAULT_HAYSTACK_PATH);
    println!("Docs path: {:?}", docs_path);

    ConfigBuilder::new()
        .global_shortcut("Ctrl+X")
        .add_role(
            "Default",
            Role {
                shortname: Some("Default".to_string()),
                name: "Default".to_string(),
                relevance_function: RelevanceFunction::TitleScorer,
                theme: "spacelab".to_string(),
                kg: None,
                haystacks: vec![Haystack {
                    path: docs_path.clone(),
                    service: ServiceType::Ripgrep,
                }],
                extra: AHashMap::new(),
            },
        )
        .add_role(
            "Engineer",
            Role {
                shortname: Some("Engineer".to_string()),
                name: "Engineer".to_string(),
                relevance_function: RelevanceFunction::TitleScorer,
                theme: "lumen".to_string(),
                kg: None,
                haystacks: vec![Haystack {
                    path: docs_path.clone(),
                    service: ServiceType::Ripgrep,
                }],
                extra: AHashMap::new(),
            },
        )
        .add_role(
            "System Operator",
            Role {
                shortname: Some("operator".to_string()),
                name: "System Operator".to_string(),
                relevance_function: RelevanceFunction::TitleScorer,
                theme: "superhero".to_string(),
                kg: Some(KnowledgeGraph {
                    automata_path,
                    input_type: KnowledgeGraphInputType::Markdown,
                    path: PathBuf::from("~/pkm"),
                    public: true,
                    publish: true,
                }),
                haystacks: vec![Haystack {
                    path: docs_path,
                    service: ServiceType::Ripgrep,
                }],
                extra: AHashMap::new(),
            },
        )
        .default_role("Default")?
        .build()
}
