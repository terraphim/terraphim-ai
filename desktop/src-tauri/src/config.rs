use std::path::PathBuf;

use ahash::AHashMap;
use terraphim_automata::AutomataPath;
use terraphim_config::{
    Config, ConfigBuilder, Haystack, KnowledgeGraph, KnowledgeGraphLocal, Role, ServiceType,
    TerraphimConfigError,
};
use terraphim_types::{KnowledgeGraphInputType, RelevanceFunction};

/// The path to the default haystack directory
// TODO: Replace this with a file-based config loader based on `twelf` in the
// future
const DEFAULT_HAYSTACK_PATH: &str = "docs/src/";
// const DEFAULT_HAYSTACK_PATH: &str = "terraphim_server/fixtures";

/// Load the default config
///
pub(crate) fn load_config() -> Result<Config, TerraphimConfigError> {
    let automata_path = AutomataPath::from_local("data/term_to_id.json");

    // Create the path to the default haystack directory
    // by concating the current directory with the default haystack path
    let mut docs_path = std::env::current_dir().unwrap();
    docs_path.pop();
    docs_path.pop();
    docs_path = docs_path.join(DEFAULT_HAYSTACK_PATH);
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
            "Terraphim Engineer",
            Role {
                shortname: Some("Terraphim Engineer".to_string()),
                name: "Terraphim Engineer".to_string(),
                relevance_function: RelevanceFunction::TerraphimGraph,
                theme: "lumen".to_string(),
                kg: Some(KnowledgeGraph {
                    automata_path: Some(AutomataPath::from_local(docs_path.join("Terraphim Engineer_thesaurus.json".to_string()))),
                    knowledge_graph_local: Some(KnowledgeGraphLocal {
                        input_type: KnowledgeGraphInputType::Markdown,
                        path: docs_path.join("kg"),
                        public: true,
                        publish: true,
                    }),
                }),
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
                    automata_path: Some(automata_path.clone()),
                    knowledge_graph_local: Some(KnowledgeGraphLocal {
                        input_type: KnowledgeGraphInputType::Markdown,
                        path: PathBuf::from("/tmp/system_operator/pages/"),
                        public: true,
                        publish: true,
                    }),
                }),
                haystacks: vec![Haystack {
                    path: docs_path.clone(),
                    service: ServiceType::Ripgrep,
                }],
                extra: AHashMap::new(),
            },
        )
        .default_role("Default")?
        .build()
}
