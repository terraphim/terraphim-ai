use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time;
use thiserror::Error;
#[cfg(feature = "tokio-runtime")]
use tokio::io::AsyncReadExt;
#[cfg(feature = "tokio-runtime")]
use tokio::process::Command;

use cached::proc_macro::cached;
use serde::Deserialize;
use terraphim_types::{Concept, NormalizedTerm, NormalizedTermValue, Thesaurus};

#[derive(Error, Debug)]
pub enum BuilderError {
    #[error("IO error")]
    Io(#[from] std::io::Error),
    #[error("JSON error")]
    Json(#[from] serde_json::Error),
    #[error("Indexation error: {0}")]
    Indexation(String),
}

pub type Result<T> = std::result::Result<T, BuilderError>;

/// A ThesaurusBuilder receives a path containing
/// resources (e.g. files) with key-value pairs and returns a `Thesaurus`
/// (a dictionary with synonyms which map to higher-level concepts)
pub trait ThesaurusBuilder {
    /// Build the thesaurus from the data source
    fn build<P: Into<PathBuf> + Send>(
        &self,
        name: String,
        haystack: P,
    ) -> impl std::future::Future<Output = Result<Thesaurus>> + Send;
}

const LOGSEQ_KEY_VALUE_DELIMITER: &str = "::";
const LOGSEQ_SYNONYMS_KEYWORD: &str = "synonyms";

#[derive(Default)]
pub struct Logseq {
    service: LogseqService,
}

impl ThesaurusBuilder for Logseq {
    async fn build<P: Into<PathBuf> + Send>(&self, name: String, haystack: P) -> Result<Thesaurus> {
        let haystack = haystack.into();
        #[cfg(feature = "tokio-runtime")]
        let messages = self
            .service
            .get_raw_messages(LOGSEQ_KEY_VALUE_DELIMITER, &haystack)
            .await?;
        #[cfg(not(feature = "tokio-runtime"))]
        let messages: Vec<Message> = Vec::new();
        let thesaurus = index_inner(name, messages);
        Ok(thesaurus)
    }
}

pub struct LogseqService {
    command: String,
    default_args: Vec<String>,
}

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

#[cfg(feature = "tokio-runtime")]
impl LogseqService {
    pub async fn get_raw_messages(&self, needle: &str, haystack: &Path) -> Result<Vec<Message>> {
        let haystack = haystack.to_string_lossy().to_string();
        log::debug!("Running logseq with needle `{needle}` and haystack `{haystack}`");
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
                    continue;
                }
                existing_paths.insert(path.clone());
                let concept = match concept_from_path(path) {
                    Ok(concept) => concept,
                    Err(e) => {
                        log::info!("Failed to get concept from path: {:?}. Skipping", e);
                        continue;
                    }
                };
                current_concept = Some(concept);
            }
            Message::Match(message) => {
                if message.path.is_none() {
                    continue;
                };
                let lines = match &message.lines {
                    Data::Text { text } => text,
                    _ => {
                        log::warn!("Error: lines is not text: {:?}", message.lines);
                        continue;
                    }
                };
                let Some((synonym_keyword, synonym)) = lines.split_once(LOGSEQ_KEY_VALUE_DELIMITER)
                else {
                    log::warn!("Error: Expected key-value pair, got {}. Skipping", lines);
                    continue;
                };
                if synonym_keyword != LOGSEQ_SYNONYMS_KEYWORD {
                    continue;
                }
                let synonyms: Vec<String> =
                    synonym.split(',').map(|s| s.trim().to_string()).collect();
                let concept = match current_concept {
                    Some(ref concept) => {
                        let nterm = NormalizedTerm::new(concept.id, concept.value.clone());
                        thesaurus.insert(concept.value.clone(), nterm.clone());
                        concept
                    }
                    None => {
                        log::warn!("Error: No concept found. Skipping");
                        continue;
                    }
                };
                for synonym in synonyms {
                    let nterm = NormalizedTerm::new(concept.id, concept.value.clone());
                    thesaurus.insert(NormalizedTermValue::new(synonym), nterm.clone());
                }
            }
            _ => {}
        };
    }
    thesaurus
}

fn concept_from_path(path: PathBuf) -> Result<Concept> {
    let stem = path.file_stem().ok_or(BuilderError::Indexation(format!(
        "No file stem in path {path:?}"
    )))?;
    let concept_str = stem.to_string_lossy().to_string();
    Ok(Concept::from(concept_str))
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Hash)]
#[serde(tag = "type", content = "data")]
#[serde(rename_all = "snake_case")]
pub enum Message {
    Begin(Begin),
    End(End),
    Match(Match),
    Context(Context),
    Summary(Summary),
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Hash)]
pub struct Begin {
    pub path: Option<Data>,
}

impl Begin {
    pub(crate) fn path(&self) -> Option<String> {
        as_path(&self.path)
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Hash)]
pub struct End {
    path: Option<Data>,
    binary_offset: Option<u64>,
    stats: Stats,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Hash)]
pub struct Summary {
    elapsed_total: Duration,
    stats: Stats,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Hash)]
pub struct Match {
    pub path: Option<Data>,
    pub lines: Data,
    line_number: Option<u64>,
    absolute_offset: u64,
    pub submatches: Vec<SubMatch>,
}

impl Match {
    pub(crate) fn path(&self) -> Option<String> {
        as_path(&self.path)
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Hash)]
pub struct Context {
    pub path: Option<Data>,
    pub lines: Data,
    line_number: Option<u64>,
    absolute_offset: u64,
    submatches: Vec<SubMatch>,
}

impl Context {
    pub(crate) fn path(&self) -> Option<String> {
        as_path(&self.path)
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Hash)]
pub struct SubMatch {
    #[serde(rename = "match")]
    m: Data,
    start: usize,
    end: usize,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Hash)]
#[serde(untagged)]
pub enum Data {
    Text { text: String },
    Bytes { bytes: String },
}

fn as_path(data: &Option<Data>) -> Option<String> {
    let data = match data {
        Some(data) => data,
        None => return None,
    };
    match data {
        Data::Text { text } => Some(text.clone()),
        _ => None,
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Hash)]
struct Stats {
    elapsed: Duration,
    searches: u64,
    searches_with_match: u64,
    bytes_searched: u64,
    bytes_printed: u64,
    matched_lines: u64,
    matches: u64,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Hash)]
struct Duration {
    #[serde(flatten)]
    duration: time::Duration,
    human: String,
}

pub fn json_decode(jsonlines: &str) -> Result<Vec<Message>> {
    Ok(serde_json::Deserializer::from_str(jsonlines)
        .into_iter()
        .collect::<std::result::Result<Vec<Message>, serde_json::Error>>()?)
}
