use ahash::AHashMap;
use serde::Deserialize;
use serde_json as json;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::Path;
use std::time;
use terraphim_config::{ConfigState, ServiceType};
use terraphim_types::{Index, SearchQuery};

use crate::{Error, Result};

mod logseq;
mod ripgrep;

pub use logseq::LogseqIndexer;
pub use ripgrep::RipgrepIndexer;

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

impl Begin {
    /// Gets the path of the file being searched (if set).
    pub(crate) fn path(&self) -> Option<String> {
        as_path(&self.path)
    }
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

impl Match {
    /// Gets the path of the file being searched (if set).
    pub(crate) fn path(&self) -> Option<String> {
        as_path(&self.path)
    }
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

impl Context {
    /// Gets the path of the file being searched (if set).
    pub(crate) fn path(&self) -> Option<String> {
        as_path(&self.path)
    }
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

/// Gets the path from a `Data` type.
fn as_path(data: &Option<Data>) -> Option<String> {
    // Return immediately if the data is None
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

/// A Middleware is a service that creates an index of articles from
/// a haystack.
///
/// Every middleware receives a needle and a haystack and returns
/// a HashMap of Articles.
pub trait IndexMiddleware {
    /// Index the haystack and return a HashMap of Articles
    ///
    /// # Errors
    ///
    /// Returns an error if the middleware fails to index the haystack
    // Note: use of `async fn` in public traits is discouraged as auto trait bounds cannot be specified
    fn index(
        &self,
        needle: &str,
        haystack: &Path,
    ) -> impl std::future::Future<Output = Result<Index>> + Send;
}

/// Use Middleware to search through haystacks and return an index of articles
/// that match the search query.
pub async fn search_haystacks(
    mut config_state: ConfigState,
    search_query: SearchQuery,
) -> Result<Index> {
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
    let ripgrep = RipgrepIndexer::default();
    let logseq = LogseqIndexer::default();

    let mut all_new_articles: Index = AHashMap::new();

    for haystack in &role_config.haystacks {
        log::info!("Finding articles in haystack: {:#?}", haystack);

        let needle = search_query.search_term.clone();

        let new_articles = match haystack.service {
            ServiceType::Ripgrep => {
                // Search through articles using ripgrep
                // This spins up ripgrep the service and indexes into the
                // `TerraphimGraph` and caches the articles
                ripgrep.index(&needle, &haystack.path).await?
            }
            ServiceType::Logseq => {
                // Search through articles in logseq format
                logseq.index(&needle, &haystack.path).await?
            }
        };

        for new_article in new_articles.values() {
            if let Err(e) = config_state.index_article(&new_article).await {
                log::warn!(
                    "Failed to index article `{}` ({}): {e:?}",
                    new_article.title,
                    new_article.url
                );
            }
        }

        all_new_articles.extend(new_articles);
    }
    Ok(all_new_articles)
}
