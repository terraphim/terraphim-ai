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


