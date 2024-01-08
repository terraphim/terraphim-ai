use serde::Deserialize;
use serde_json as json;
use std::collections::hash_map::DefaultHasher;
use std::collections::{HashMap, HashSet};
use std::fs::{self};
use std::hash::{Hash, Hasher};
use std::process::Stdio;
use std::time;
use terraphim_types::{Article, ConfigState, SearchQuery};
use tokio::io::AsyncReadExt;
use tokio::process::Command;

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

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
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
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct Begin {
    pub path: Option<Data>,
}

/// The `End` message is sent at the end of a search.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct End {
    path: Option<Data>,
    binary_offset: Option<u64>,
    stats: Stats,
}

/// The `Summary` message is sent at the end of a search.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct Summary {
    elapsed_total: Duration,
    stats: Stats,
}

/// The `Match` message is sent for each non-overlapping match of a search.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct Match {
    pub path: Option<Data>,
    pub lines: Data,
    line_number: Option<u64>,
    absolute_offset: u64,
    pub submatches: Vec<SubMatch>,
}

/// The `Context` specifies the lines surrounding a match.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct Context {
    pub path: Option<Data>,
    pub lines: Data,
    line_number: Option<u64>,
    absolute_offset: u64,
    submatches: Vec<SubMatch>,
}

/// A `SubMatch` is a match within a match.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
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

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
struct Stats {
    elapsed: Duration,
    searches: u64,
    searches_with_match: u64,
    bytes_searched: u64,
    bytes_printed: u64,
    matched_lines: u64,
    matches: u64,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
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

pub struct RipgrepService {
    command: String,
    default_args: Vec<String>,
}

/// Returns a new ripgrep service with default arguments
impl Default for RipgrepService {
    fn default() -> Self {
        Self {
            command: "rg".to_string(),
            default_args: ["--json", "--trim", "-C3", "-i"]
                .into_iter()
                .map(String::from)
                .collect(),
        }
    }
}

impl RipgrepService {
    /// Run ripgrep with the given needle and haystack
    pub async fn run(&self, needle: String, haystack: String) -> Result<Vec<Message>> {
        // Merge the default arguments with the needle and haystack
        let args: Vec<String> = vec![needle, haystack]
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

/// RipgrepMiddleware is a Middleware that uses ripgrep to index and search
/// through haystacks.
struct RipgrepMiddleware {
    service: RipgrepService,
    config_state: ConfigState,
}

impl RipgrepMiddleware {
    pub fn new(config_state: ConfigState) -> Self {
        Self {
            service: RipgrepService::default(),
            config_state,
        }
    }
}

impl Middleware for RipgrepMiddleware {
    async fn index(
        &mut self,
        needle: String,
        haystack: String,
    ) -> Result<HashMap<String, Article>> {
        let messages = self.service.run(needle, haystack).await?;
        let mut article = Article::default();

        // Cache of the articles already processed by index service
        let mut cached_articles: HashMap<String, Article> = HashMap::new();
        let mut existing_paths: HashSet<String> = HashSet::new();

        for message in messages.iter() {
            match message {
                Message::Begin(begin_msg) => {
                    article = Article::default();

                    // get path
                    let path: Option<Data> = begin_msg.path.clone();
                    let path_text = match path {
                        Some(Data::Text { text }) => text,
                        _ => {
                            continue;
                        }
                    };

                    if existing_paths.contains(&path_text) {
                        continue;
                    }
                    existing_paths.insert(path_text.clone());

                    let id = calculate_hash(&path_text);
                    article.id = Some(id.clone());
                    article.title = path_text.clone();
                    article.url = path_text.clone();
                }
                Message::Match(match_msg) => {
                    let path = match_msg.path.clone();
                    let cloned_path = path.clone();
                    let path = cloned_path.ok_or_else(|| {
                        Error::IndexationError(format!("Unknown path: {:?}", path.clone()))
                    })?;

                    let path_text = match path {
                        Data::Text { text } => text,
                        _ => {
                            println!("Error: path is not text: {path:?}");
                            continue;
                        }
                    };
                    let body = fs::read_to_string(path_text)?;
                    article.body = body;

                    let lines = match &match_msg.lines {
                        Data::Text { text } => text,
                        _ => {
                            println!("Error: lines is not text: {:?}", match_msg.lines);
                            continue;
                        }
                    };
                    match article.description {
                        Some(description) => {
                            article.description = Some(description + " " + &lines);
                        }
                        None => {
                            article.description = Some(lines.clone());
                        }
                    }
                }
                Message::Context(context_msg) => {
                    let article_url = article.url.clone();
                    let path = context_msg.path.clone();
                    let cloned_path = path.clone();
                    let path = cloned_path.ok_or_else(|| {
                        Error::IndexationError(format!("Unknown path: {:?}", path.clone()))
                    })?;

                    let path_text = match path {
                        Data::Text { text } => text,
                        _ => {
                            println!("Error: path is not text: {path:?}");
                            continue;
                        }
                    };

                    // We got a context for a different article
                    if article_url != path_text {
                        println!(
                            "Error: Context for differrent article. article_url != path_text: {article_url:?} != {path_text:?}"
                        );
                        continue;
                    }

                    let lines = match &context_msg.lines {
                        Data::Text { text } => text,
                        _ => {
                            println!("Error: lines is not text: {:?}", context_msg.lines);
                            continue;
                        }
                    };
                    match article.description {
                        Some(description) => {
                            article.description = Some(description + " " + &lines);
                        }
                        None => {
                            article.description = Some(lines.clone());
                        }
                    }
                }
                Message::End(_) => {
                    // The `End` message could be received before the `Begin`
                    // message causing the article to be empty
                    let id = match article.id {
                        Some(ref id) => id,
                        None => {
                            println!("Error: End message received before Begin message. Skipping.");
                            continue;
                        }
                    };
                    // We are done with this article, index it
                    self.config_state
                        .index_article(article.clone())
                        .await
                        .expect("Failed to index article");
                    cached_articles.insert(id.clone().to_string(), article.clone());
                }
                _ => {}
            };
        }
        Ok(cached_articles)
    }
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
    // if role have a ripgrep service, use it to spin index and search process and return cached articles
    println!(" role: {:?}", role);
    // Role config
    // FIXME: this fails when role name arrives in lowercase. Should we canonicalize role names to uppercase?
    let role_config = current_config_state
        .roles
        .get(role)
        .ok_or_else(|| Error::RoleNotFound(role.to_string()))?;
    println!("role_config: {:#?}", role_config);

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
                // return cached articles
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
