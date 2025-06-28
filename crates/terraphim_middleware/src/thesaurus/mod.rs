//! Logseq is a knowledge graph that uses Markdown files to store notes. This
//! module provides a middleware for creating a Thesaurus from a Logseq
//! haystack.
//!
//! Example:
//!
//! If we parse a file named `path/to/concept.md` with the following content:
//!
//! ```markdown
//! synonyms:: foo, bar, baz
//! ```
//!
//! Then the thesaurus will contain the following entries:
//!
//! ```rust
//! use terraphim_types::{Thesaurus, Concept, NormalizedTerm};
//! let concept = Concept::new("concept".into());
//! let nterm = NormalizedTerm::new(concept.id, concept.value.clone());
//! let mut thesaurus = Thesaurus::new("Engineer".to_string());
//! thesaurus.insert(concept.value.clone(), nterm.clone());
//! thesaurus.insert("foo".to_string().into(),nterm.clone());
//! thesaurus.insert("bar".to_string().to_string().into(), nterm.clone());
//! thesaurus.insert("baz".to_string().into(), nterm.clone());
//! ```
//! The logic as follows: if you ask for concept by name you get concept, if you ask (get) for any of the synonyms you will get concept with id,
//! its pre-computed reverse tree traversal - any of the synonyms (leaf) maps into the concepts (root)

use terraphim_automata::AutomataPath;
use terraphim_config::ConfigState;
use terraphim_config::Role;
use terraphim_persistence::Persistable;
use terraphim_rolegraph::{Error as RoleGraphError, RoleGraph, RoleGraphSync};
use terraphim_types::SearchQuery;
use terraphim_types::{Concept, NormalizedTerm, RoleName, Thesaurus};

use crate::Result;
use cached::proc_macro::cached;
use std::collections::HashSet;
use std::path::Path;
use std::path::PathBuf;
use std::process::Stdio;
use tokio::io::AsyncReadExt;
use tokio::process::Command;

use crate::command::ripgrep::{json_decode, Data, Message};
use crate::Error;

pub async fn build_thesaurus_from_haystack(
    config_state: &mut ConfigState,
    search_query: &SearchQuery,
) -> Result<()> {
    // build thesaurus from haystack or load from remote
    // FIXME: introduce LRU cache for locally build thesaurus via persistance crate
    log::debug!("Building thesaurus from haystack");
    let config = config_state.config.lock().await.clone();
    let roles = config.roles.clone();
    let default_role = config.default_role.clone();
    let role_name = search_query.role.clone().unwrap_or_default();
    log::debug!("Role name: {}", role_name);
    let role: &mut Role = &mut roles
        .get(&role_name)
        .unwrap_or(&roles[&default_role])
        .to_owned();
    log::debug!("Role: {:?}", role);
    for haystack in &role.haystacks {
        log::debug!("Updating thesaurus for haystack: {:?}", haystack);

        let logseq = Logseq::default();
        let mut thesaurus: Thesaurus = logseq
            .build(role_name.as_lowercase().to_string(), PathBuf::from(&haystack.location))
            .await?;
        match thesaurus.save().await {
            Ok(_) => {
                log::info!("Thesaurus for role `{}` saved to persistence", role_name);
                // We reload the thesaurus from persistence to ensure we are using the
                // canonical, persisted version going forward.
                thesaurus = thesaurus.load().await?;
            }
            Err(e) => log::error!("Failed to save thesaurus: {:?}", e),
        }
        
        // The following code wrote the thesaurus to a local file for debugging
        // purposes. This is now removed to enforce using the persistence layer.
        // let mut haystack_path = haystack.path.clone();
        // haystack_path.pop();
        // let thesaurus_path = haystack
        //     .path
        //     .join(format!("{}_thesaurus.json", role_name.clone()));
        // let thesaurus_json = serde_json::to_string_pretty(&thesaurus)?;
        // tokio::fs::write(&thesaurus_path, thesaurus_json).await?;
        // log::debug!("Thesaurus written to {:#?}", thesaurus_path);
        // role.kg.as_mut().unwrap().automata_path = Some(AutomataPath::Local(thesaurus_path));
        
        log::debug!("Make sure thesaurus updated in a role {}", role_name);

        update_thesaurus(config_state, &role_name, thesaurus).await?;
    }
    Ok(())
}

async fn update_thesaurus(
    config_state: &mut ConfigState,
    role_name: &RoleName,
    thesaurus: Thesaurus,
) -> Result<()> {
    log::debug!("Updating thesaurus for role: {}", role_name);
    let mut rolegraphs = config_state.roles.clone();
    let rolegraph = RoleGraph::new(role_name.clone(), thesaurus).await;
    match rolegraph {
        Ok(rolegraph) => {
            let rolegraph_value = RoleGraphSync::from(rolegraph);
            rolegraphs.insert(role_name.clone(), rolegraph_value);
        }
        Err(e) => log::error!("Failed to update role and thesaurus: {:?}", e),
    }

    Ok(())
}

/// A ThesaurusBuilder receives a path containing
/// resources (e.g. files) with key-value pairs and returns a `Thesaurus`
/// (a dictionary with synonyms which map to higher-level concepts)
pub trait ThesaurusBuilder {
    /// Build the thesaurus from the data source
    ///
    /// * `name` is the name of the thesaurus
    /// * `haystack` is the root directory for building the thesaurus
    ///   (e.g. a directory of Logseq files)
    ///
    // This could be generalized (e.g. to take a `Read` trait object
    // or a `Resource` or a glob of inputs)
    fn build<P: Into<PathBuf> + Send>(
        &self,
        name: String,
        haystack: P,
    ) -> impl std::future::Future<Output = Result<Thesaurus>> + Send;
}

/// In Logseq, `::` serves as a delimiter between the property name and its
/// value, e.g.
///
/// ```markdown
/// title:: My Note
/// tags:: #idea #project
/// ```
// FIXME: move to config item per role
const LOGSEQ_KEY_VALUE_DELIMITER: &str = "::";

/// The synonyms keyword used in Logseq documents
// FIXME: move to config item per role
const LOGSEQ_SYNONYMS_KEYWORD: &str = "synonyms";

/// A builder for a knowledge graph, which knows how to handle Logseq input.
#[derive(Default)]
pub struct Logseq {
    service: LogseqService,
}

impl ThesaurusBuilder for Logseq {
    /// Build the knowledge graph from the data source
    /// and store it in each rolegraph.
    async fn build<P: Into<PathBuf> + Send>(&self, name: String, haystack: P) -> Result<Thesaurus> {
        let haystack = haystack.into();
        let messages = self
            .service
            .get_raw_messages(LOGSEQ_KEY_VALUE_DELIMITER, &haystack)
            .await?;

        let thesaurus = index_inner(name, messages);
        Ok(thesaurus)
    }
}

pub struct LogseqService {
    command: String,
    default_args: Vec<String>,
}

/// Returns a new ripgrep service with default arguments
impl Default for LogseqService {
    fn default() -> Self {
        Self {
            command: "rg".to_string(),
            default_args: ["--json", "--trim", "--ignore-case", "-tmarkdown"]
                .into_iter()
                .map(String::from)
                .collect(),
        }
    }
}

impl LogseqService {
    /// Run ripgrep with the given needle and haystack
    ///
    /// Returns a Vec of Messages, which correspond to ripgrep's internal
    /// JSON output. Learn more about ripgrep's JSON output here:
    /// https://docs.rs/grep-printer/0.2.1/grep_printer/struct.JSON.html
    pub async fn get_raw_messages(&self, needle: &str, haystack: &Path) -> Result<Vec<Message>> {
        let haystack = haystack.to_string_lossy().to_string();
        log::debug!("Running logseq with needle `{needle}` and haystack `{haystack}`");

        // Merge the default arguments with the needle and haystack
        let args: Vec<String> = vec![needle.to_string(), haystack]
            .into_iter()
            .chain(self.default_args.clone())
            .collect();

        let mut child = Command::new(&self.command)
            .args(args)
            .stdout(Stdio::piped())
            .spawn()?;

        let mut stdout = child.stdout.take().expect("Stdout is not available");
        let read = async move {
            let mut data = String::new();
            stdout.read_to_string(&mut data).await.map(|_| data)
        };
        let output = read.await?;
        json_decode(&output)
    }
}

/// Creates a `term_to_id` structure, which maps terms to their corresponding
/// concept IDs.
///
/// E.g. if a logseq document titled "validated system" contains
///
/// ```md
/// synonyms:: operation service module, something else
/// ```
///
/// Then the final`term_to_id.json` file will contain the following entries:
///
/// ```json
/// {
/// "validated system": {
///     "id": 1351,
///     "nterm": "validated system"
///   },
///   "operation service module": {
///     "id": 1351,
///     "nterm": "validated system"
///   },
///   "something else": {
///     "id": 1351,
///     "nterm": "validated system"
///   }
/// }
/// ```
///
// This is a free-standing function because it's a requirement for caching the
// results
#[cached]
fn index_inner(name: String, messages: Vec<Message>) -> Thesaurus {
    let mut thesaurus = Thesaurus::new(name);
    let mut current_concept: Option<Concept> = None;

    let mut existing_paths: HashSet<PathBuf> = HashSet::new();

    for message in messages {
        match message {
            Message::Begin(message) => {
                let Some(path_str) = message.path() else {
                    continue;
                };
                let path = PathBuf::from(&path_str);

                if existing_paths.contains(&path) {
                    // Already processed this input
                    continue;
                }
                existing_paths.insert(path.clone());

                // Use the path as the concept
                let concept = match concept_from_path(path) {
                    Ok(concept) => concept,
                    Err(e) => {
                        log::info!("Failed to get concept from path: {:?}. Skipping", e);
                        continue;
                    }
                };
                log::trace!("Found concept: {concept}");
                current_concept = Some(concept);
            }
            Message::Match(message) => {
                if message.path().is_none() {
                    continue;
                };

                // let body = match fs::read_to_string(path) {
                //     Ok(body) => body,
                //     Err(e) => {
                //         println!("Error: Failed to read file: {:?}. Skipping", e);
                //         continue;
                //     }
                // };

                let lines = match &message.lines {
                    Data::Text { text } => text,
                    _ => {
                        log::warn!("Error: lines is not text: {:?}", message.lines);
                        continue;
                    }
                };

                // Split text by delimiter (`::`)
                // If the key is `synonyms`, then the value is a comma-separated
                // list of synonyms, which we can use to build the thesaurus
                let Some((synonym_keyword, synonym)) = lines.split_once(LOGSEQ_KEY_VALUE_DELIMITER)
                else {
                    log::warn!("Error: Expected key-value pair, got {}. Skipping", lines);
                    continue;
                };

                if synonym_keyword != LOGSEQ_SYNONYMS_KEYWORD {
                    // Not a synonym, skip
                    continue;
                }

                let synonyms: Vec<String> =
                    synonym.split(',').map(|s| s.trim().to_string()).collect();

                let concept = match current_concept {
                    Some(ref concept) => {
                        let nterm = NormalizedTerm::new(concept.id, concept.value.clone());
                        thesaurus.insert(concept.value.clone(), nterm);
                        concept
                    }
                    None => {
                        log::warn!("Error: No concept found. Skipping");
                        continue;
                    }
                };
                for synonym in synonyms {
                    let nterm = NormalizedTerm::new(concept.id, concept.value.clone());
                    thesaurus.insert(synonym.into(), nterm);
                }
            }
            _ => {}
        };
    }
    thesaurus
}

/// Uses the file stem as the concept name
fn concept_from_path(path: PathBuf) -> Result<Concept> {
    let stem = path
        .file_stem()
        .ok_or(Error::Indexation(format!("No file stem in path {path:?}")))?;
    let concept_str = stem.to_string_lossy().to_string();
    Ok(Concept::from(concept_str))
}
