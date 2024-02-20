use ahash::AHashMap;
use logseq::LogseqMiddleware;
use ripgrep::RipgrepMiddleware;
use serde::Deserialize;
use serde_json as json;
use std::collections::hash_map::DefaultHasher;
use cached::proc_macro::cached;
use serde::Deserialize;
use serde_json as json;
use std::collections::hash_map::DefaultHasher;
use std::collections::HashSet;
use std::fs::{self};
use std::hash::{Hash, Hasher};
use std::time;
use terraphim_config::{ConfigState, ServiceType};
use terraphim_types::{Article, SearchQuery};

mod logseq;
mod ripgrep;
use terraphim_types::{Article, ConfigState, SearchQuery};
use tokio::io::AsyncReadExt;
use tokio::process::Command;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Serde deserialization error: {0}")]
    Json(#[from] json::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Role not found: {0}")]
    RoleNotFound(String),

    #[error("Indexation error: {0}")]
    Indexation(String),
}

pub type Result<T> = std::result::Result<T, Error>;

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

/// The `Begin` message is sent at the beginning of each search.
/// It contains the path that is being searched, if one exists.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Hash)]
pub struct Begin {
    pub path: Option<Data>,
}

/// The `End` message is sent at the end of a search.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Hash)]
pub struct End {
    path: Option<Data>,
    binary_offset: Option<u64>,
    stats: Stats,
}

/// The `Summary` message is sent at the end of a search.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Hash)]
pub struct Summary {
    elapsed_total: Duration,
    stats: Stats,
}

/// The `Match` message is sent for each non-overlapping match of a search.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Hash)]
pub struct Match {
    pub path: Option<Data>,
    pub lines: Data,
    line_number: Option<u64>,
    absolute_offset: u64,
    pub submatches: Vec<SubMatch>,
}

/// The `Context` specifies the lines surrounding a match.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Hash)]
pub struct Context {
    pub path: Option<Data>,
    pub lines: Data,
    line_number: Option<u64>,
    absolute_offset: u64,
    submatches: Vec<SubMatch>,
}

/// A `SubMatch` is a match within a match.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Hash)]
pub struct SubMatch {
    #[serde(rename = "match")]
    m: Data,
    start: usize,
    end: usize,
}

/// The `Data` type is used for fields that may be either text or bytes.
/// In contains the raw data of the field.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Hash)]
#[serde(untagged)]
pub enum Data {
    Text { text: String },
    // This variant is used when the data isn't valid UTF-8. The bytes are
    // base64 encoded, so using a String here is OK.
    Bytes { bytes: String },
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

/// Decode JSON Lines into a Vec<Message>.
pub fn json_decode(jsonlines: &str) -> Result<Vec<Message>> {
    Ok(json::Deserializer::from_str(jsonlines)
        .into_iter()
        .collect::<std::result::Result<Vec<Message>, serde_json::Error>>()?)
}

fn calculate_hash<T: Hash>(t: &T) -> String {
    let mut s = DefaultHasher::new();
    t.hash(&mut s);
    format!("{:x}", s.finish())
}

/// A Middleware is a service that can be used to index and search through
/// haystacks. Every middleware receives a needle and a haystack and returns
/// a HashMap of Articles.
trait Middleware {
    /// Index the haystack and return a HashMap of Articles
    ///
    /// # Errors
    ///
    /// Returns an error if the middleware fails to index the haystack
    async fn index(
        &mut self,
        needle: String,
        haystack: String,
    ) -> Result<AHashMap<String, Article>>;
}

/// Use Middleware to search through haystacks
pub async fn search_haystacks(
    config_state: ConfigState,
    search_query: SearchQuery,
) -> Result<AHashMap<String, Article>> {
    let config = config_state.config.lock().await.clone();

    let search_query_role = search_query
        .role
        .unwrap_or(config.default_role)
        .to_lowercase();

    let role_config = config
        .roles
        .get(&search_query_role)
        .ok_or_else(|| Error::RoleNotFound(search_query_role.to_string()))?;

    // Define middleware to be used for searching.
    let mut ripgrep = RipgrepMiddleware::new(config_state.clone());
    let mut logseq = LogseqMiddleware::new(config_state.clone());

    let mut cached_articles: AHashMap<String, Article> = AHashMap::new();

    for haystack in &role_config.haystacks {
        println!("Handling haystack: {:#?}", haystack);

        let needle = search_query.search_term.clone();
        let haystack_inner = haystack.haystack.clone();

        cached_articles = match haystack.service {
            ServiceType::Ripgrep => {
                // Search through articles using ripgrep
                // This spins up ripgrep the service and indexes into the
                // `TerraphimGraph` and caches the articles
                ripgrep
                    .index(needle.clone(), haystack_inner.clone())
                    .await?
            }
            ServiceType::Logseq => {
                // Search through articles in logseq format
                logseq.index(needle.clone(), haystack_inner.clone()).await?
            }
        };
    }
    Ok(cached_articles)
}
