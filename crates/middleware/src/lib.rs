use ripgrep::RipgrepMiddleware;
use serde::Deserialize;
use serde_json as json;
use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::time;
use terraphim_types::{Article, ConfigState, SearchQuery};

mod ripgrep;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Failed to load config state")]
    ConfigStateLoad(#[from] terraphim_types::Error),

    #[error("Serde deserialization error: {0}")]
    Json(#[from] json::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Role not found: {0}")]
    RoleNotFound(String),

    #[error("Indexation error: {0}")]
    IndexationError(String),
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
    async fn index(&mut self, needle: String, haystack: String)
        -> Result<HashMap<String, Article>>;
}

/// Use Middleware to search through haystacks
pub async fn search_haystacks(
    config_state: ConfigState,
    search_query: SearchQuery,
) -> Result<HashMap<String, Article>> {
    let current_config_state = config_state.config.lock().await.clone();
    let default_role = current_config_state.default_role.clone();
    // if role is not provided, use the default role in the config
    let role = match search_query.role {
        None => default_role.as_str(),
        Some(ref role) => role.as_str(),
    };
    // normalize roles to lowercase - same as term
    let role = role.to_lowercase();
    // if role have a ripgrep service, use it to spin index and search process and return cached articles
    // Role config
    let role_config = current_config_state
        .roles
        .get(&role)
        .ok_or_else(|| Error::RoleNotFound(role.to_string()))?;

    // Define all middleware to be used for searching.
    let mut ripgrep_middleware = RipgrepMiddleware::new(config_state.clone());

    let mut articles_cached: HashMap<String, Article> = HashMap::new();
    for each_haystack in &role_config.haystacks {
        println!(" each_haystack: {:#?}", each_haystack);

        // Spin ripgrep service and index output of ripgrep into Cached Articles and TerraphimGraph
        let needle = search_query.search_term.clone();
        let haystack = each_haystack.haystack.clone();

        articles_cached = match each_haystack.service.as_str() {
            "ripgrep" => {
                // Search through articles using ripgrep
                ripgrep_middleware
                    .index(needle.clone(), haystack.clone())
                    .await?
            }
            _ => {
                println!("Unknown middleware: {:#?}", each_haystack.service);
                HashMap::new()
            }
        };
    }
    Ok(articles_cached)
}
