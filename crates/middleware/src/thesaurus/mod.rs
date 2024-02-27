//! Logseq is a knowledge graph that uses Markdown files to store notes. This
//! module provides a middleware for creating a Thesaurus from a Logseq
//! haystack.
//!
//! Example:
//!
//! If we parse a file named `path/to/concept.md` with the following content:
//!
//! ```
//! synonyms:: foo, bar, baz
//! ```
//!
//! Then the thesaurus will contain the following entries:
//!
//! ```rust
//! use terraphim_types::Thesaurus;
//!
//! let mut thesaurus = Thesaurus::new();
//! thesaurus.insert("concept".to_string(), "foo".to_string());
//! thesaurus.insert("concept".to_string(), "bar".to_string());
//! thesaurus.insert("concept".to_string(), "baz".to_string());
//! ```

use crate::Result;
use cached::proc_macro::cached;
use std::collections::HashSet;
use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::process::Stdio;
use terraphim_types::{Concept, NormalizedTerm, Thesaurus};
use tokio::io::AsyncReadExt;
use tokio::process::Command;

use crate::command::ripgrep::{json_decode, Data, Message};
use crate::Error;

use terraphim_config::{Config, ConfigState, ServiceType};

/// A ThesaurusBuilder receives a path containing
/// resources (e.g. files) with key-value pairs and returns a `Thesaurus`
/// (a dictionary with synonyms which map to higher-level concepts)
trait ThesaurusBuilder {
    /// `haystack` is the root directory for building the thesaurus
    /// (e.g. a directory of Logseq files)
    // This could be generalized (e.g. to take a `Read` trait object
    // or a `Resource` or a glob of inputs)
    async fn build(&self, haystack: PathBuf) -> Result<Thesaurus>;
}

/// In Logseq, `::` serves as a delimiter between the property name and its
/// value, e.g.
///
/// ```
/// title:: My Note
/// tags:: #idea #project
/// ```
const LOGSEQ_KEY_VALUE_DELIMITER: &str = "::";

/// The synonyms keyword used in Logseq documents
const LOGSEQ_SYNONYMS_KEYWORD: &str = "synonyms";

/// A builder for a knowledge graph, which knows how to handle Logseq input.
struct LogseqKnowledgeGraph {}

impl LogseqKnowledgeGraph {
    /// Build the knowledge graph from the data source
    /// and store it in each rolegraph.
    async fn build(&self, haystack: PathBuf) -> Result<()> {
        // Initialize a logseq service for parsing the data source
        let mut config = Config::new(ServiceType::Ripgrep);
        let config_state = ConfigState::new(&mut config).await?;

        let logseq = Logseq::default();
        let thesaurus = logseq.build(haystack).await?;
        println!("{:#?}", thesaurus);

        // Iterate over the roles and store the thesaurus in each rolegraph
        for role in config_state.config.lock().await.roles.values() {
            todo!()
        }

        Ok(())
    }
}

/// LogseqMiddleware is a Middleware that uses ripgrep to index and search
/// through haystacks.
pub struct Logseq {
    service: LogseqService,
}

impl Default for Logseq {
    fn default() -> Self {
        Self {
            service: LogseqService::default(),
        }
    }
}

impl ThesaurusBuilder for Logseq {
    /// Build a thesaurus from a Logseq haystack
    ///
    /// # Errors
    ///
    /// Returns an error if the middleware fails to create the thesaurus
    async fn build(&self, haystack: PathBuf) -> Result<Thesaurus> {
        let messages = self
            .service
            .get_raw_messages(LOGSEQ_KEY_VALUE_DELIMITER, &haystack)
            .await?;

        let articles = index_inner(messages);
        Ok(articles)
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
            default_args: ["--json", "--trim", "--ignore-case"]
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
        println!("Running logseq with needle: {needle} and haystack: {haystack}");

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

#[cached]
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
fn index_inner(messages: Vec<Message>) -> Thesaurus {
    let mut thesaurus = Thesaurus::new();
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

                // The path is the concept
                let concept = match concept_from_path(path) {
                    Ok(concept) => concept,
                    Err(e) => {
                        println!("Error: Failed to get concept from path: {:?}. Skipping", e);
                        continue;
                    }
                };
                println!("Found concept: {concept:?}");
                current_concept = Some(concept);
            }
            Message::Match(message) => {
                let Some(path) = message.path() else {
                    continue;
                };

                let body = match fs::read_to_string(path) {
                    Ok(body) => body,
                    Err(e) => {
                        println!("Error: Failed to read file: {:?}. Skipping", e);
                        continue;
                    }
                };

                let lines = match &message.lines {
                    Data::Text { text } => text,
                    _ => {
                        println!("Error: lines is not text: {:?}", message.lines);
                        continue;
                    }
                };

                // Split text by delimiter (`::`)
                // If the key is `synonyms`, then the value is a comma-separated
                // list of synonyms, which we can use to build the thesaurus
                let Some((synonym_keyword, synonym)) = lines.split_once(LOGSEQ_KEY_VALUE_DELIMITER)
                else {
                    println!("Error: Expected key-value pair, got {}. Skipping", lines);
                    continue;
                };

                if synonym_keyword != LOGSEQ_SYNONYMS_KEYWORD {
                    // Not a synonym, skip
                    continue;
                }

                let synonyms: Vec<String> =
                    synonym.split(',').map(|s| s.trim().to_string()).collect();

                let concept = match current_concept {
                    Some(ref concept) => concept,
                    None => {
                        println!("Error: No concept found. Skipping");
                        continue;
                    }
                };
                for synonym in synonyms {
                    let nterm = NormalizedTerm::new(concept.id.clone(), synonym.into());
                    thesaurus.insert(concept.id.clone(), nterm);
                }
            }
            Message::End(_) => {
                // The `End` message could be received before the `Begin`
                // message causing the concept to be empty
                let concept = match current_concept {
                    Some(ref concept) => concept,
                    None => {
                        println!("Error: End message received before Begin message. Skipping");
                        continue;
                    }
                };
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
