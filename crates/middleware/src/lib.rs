use serde::Deserialize;
use serde_json as json;
use std::collections::hash_map::DefaultHasher;
use std::collections::{HashSet, HashMap};
use std::fs::{self, File};
use std::hash::{Hash, Hasher};
use std::process::{ExitStatus, Stdio};
use std::time;
use terraphim_types::{ConfigState, SearchQuery, Article, merge_and_serialize};
use tokio::io::AsyncBufRead;
use tokio::process::{Child, Command};

use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader as TokioBufferedReader};

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

impl Message {
    fn unwrap_begin(&self) -> Begin {
        match *self {
            Message::Begin(ref x) => x.clone(),
            ref x => panic!("expected Message::Begin but got {:?}", x),
        }
    }

    fn unwrap_end(&self) -> End {
        match *self {
            Message::End(ref x) => x.clone(),
            ref x => panic!("expected Message::End but got {:?}", x),
        }
    }

    fn unwrap_match(&self) -> Match {
        match *self {
            Message::Match(ref x) => x.clone(),
            ref x => panic!("expected Message::Match but got {:?}", x),
        }
    }

    fn unwrap_context(&self) -> Context {
        match *self {
            Message::Context(ref x) => x.clone(),
            ref x => panic!("expected Message::Context but got {:?}", x),
        }
    }

    fn unwrap_summary(&self) -> Summary {
        match *self {
            Message::Summary(ref x) => x.clone(),
            ref x => panic!("expected Message::Summary but got {:?}", x),
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct Begin {
    pub path: Option<Data>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct End {
    path: Option<Data>,
    binary_offset: Option<u64>,
    stats: Stats,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct Summary {
    elapsed_total: Duration,
    stats: Stats,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct Match {
    pub path: Option<Data>,
    pub lines: Data,
    line_number: Option<u64>,
    absolute_offset: u64,
    pub submatches: Vec<SubMatch>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct Context {
    pub path: Option<Data>,
    pub lines: Data,
    line_number: Option<u64>,
    absolute_offset: u64,
    submatches: Vec<SubMatch>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
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
    // This variant is used when the data isn't valid UTF-8. The bytes are
    // base64 encoded, so using a String here is OK.
    Bytes { bytes: String },
}

impl Data {
    fn text(s: &str) -> Data {
        Data::Text {
            text: s.to_string(),
        }
    }
    fn bytes(s: &str) -> Data {
        Data::Bytes {
            bytes: s.to_string(),
        }
    }
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

/// Decode JSON Lines into a Vec<Message>. If there was an error decoding,
/// this function panics.
pub fn json_decode(jsonlines: &str) -> Vec<Message> {
    json::Deserializer::from_str(jsonlines)
        .into_iter()
        .collect::<Result<Vec<Message>, _>>()
        .unwrap()
}
fn calculate_hash<T: Hash>(t: &T) -> String {
    let mut s = DefaultHasher::new();
    t.hash(&mut s);
    format!("{:x}", s.finish())
}

pub struct RipgrepService {
    command: String,
    args: Vec<String>,
}
impl RipgrepService {
    pub fn new(command: String, args: Vec<String>) -> Self {
        Self { command, args }
    }
    pub async fn run(&self) -> Result<Vec<Message>, std::io::Error> {
        let mut child = Command::new(&self.command)
            .args(&self.args)
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();
        let mut stdout = child.stdout.take().expect("Stdout is not available");
        let read = async move {
            let mut data = String::new();
            stdout.read_to_string(&mut data).await.map(|_| data)
        };
        let output = read.await;
        let msgs = json_decode(&output.unwrap());
        Ok(msgs)
    }
}

/// Service to run and index output of ripgrep into TerraphimGraph

pub async fn run_ripgrep_service_and_index(
    mut config_state: ConfigState,
    needle: String,
    haystack: String,
) -> HashMap<String, Article>{
    let ripgrep_service = RipgrepService::new(
        "rg".to_string(),
        vec![
            format!("{}", needle.clone()),
            haystack.clone(),
            "--json".to_string(),
            "--trim".to_string(),
            "-C3".to_string(),
            "-i".to_string(),
        ],
    );
    let msgs = ripgrep_service.run().await.unwrap();

    let mut article = Article::default();

    /// Cache of the articles already processed by index service
    let mut cached_articles: HashMap<String, Article> = HashMap::new();
    let mut existing_paths: HashSet<String> = HashSet::new();

    for each_msg in msgs.iter() {
        match each_msg {
            Message::Begin(begin_msg) => {
                println!("stdout: {:#?}", each_msg);
                article = Article::default();

                // get path
                let path: Option<Data> = begin_msg.path.clone();
                let path_text = match path {
                    Some(Data::Text { text }) => text,
                    _ => {
                        println!("Error: path is not text");
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
                println!("stdout: {:#?}", article);
                let path = match_msg.path.clone();
                let path = path.unwrap();
                let path_text = match path {
                    Data::Text { text } => text,
                    _ => {
                        println!("Error: path is not text");
                        continue;
                    }
                };
                let body = fs::read_to_string(path_text).unwrap();
                article.body = body;

                let lines = match &match_msg.lines {
                    Data::Text { text } => text,
                    _ => {
                        println!("Error: lines is not text");
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
                // let article = Article::new(context_msg.clone());
                println!("stdout: {:#?}", article);

                let article_url = article.url.clone();
                let path = context_msg.path.clone();
                let path = path.unwrap();
                let path_text = match path {
                    Data::Text { text } => text,
                    _ => {
                        println!("Error: path is not text");
                        continue;
                    }
                };

                // We got a context for a different article
                if article_url != path_text {
                    continue;
                }

                let lines = match &context_msg.lines {
                    Data::Text { text } => text,
                    _ => {
                        println!("Error: lines is not text");
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
            Message::End(end_msg) => {
                println!("stdout: {:#?}", each_msg);
                // The `End` message could be received before the `Begin` message
                // causing the article to be empty
                let id = match article.id {
                    Some(ref id) => id,
                    None => continue,
                };
                config_state
                    .index_article(article.clone())
                    .await
                    .expect("Failed to index article");
                cached_articles.insert(id.clone().to_string(), article.clone());

            }
            _ => {}
        };

    }
cached_articles
}
/// Spin ripgrep service and index output of ripgrep into Cached Articles and TerraphimGraph
pub async fn search_haystacks(config_state:ConfigState, search_query:SearchQuery)->HashMap<String, Article>{
    
    let current_config_state= config_state.config.lock().await.clone();
    let default_role = current_config_state.default_role.clone();
    // if role is not provided, use the default role in the config
    let role = if search_query.role.is_none() {
        default_role.as_str()
    } else {
        search_query.role.as_ref().unwrap()
    };
            // if role have a ripgrep service, use it to spin index and search process and return cached articles
    println!(" role: {}", role);
    // Role config
    // FIXME: this fails when role name arrives in lowercase
    let role_config = current_config_state.roles.get(role).unwrap();
    println!(" role_config: {:#?}", role_config);
    let mut articles_cached:HashMap<String,Article> = HashMap::new();
    for each_haystack in &role_config.haystacks {
        println!(" each_haystack: {:#?}", each_haystack);
        articles_cached = match each_haystack.service.as_str() {
            "ripgrep" => {
                let needle = search_query.search_term.clone();
                let haystack = each_haystack.haystack.clone();
                // return cached articles
                run_ripgrep_service_and_index(config_state.clone(), needle, haystack).await
            }
            _ => {
                println!("Haystack service not supported, hence skipping");
                HashMap::new()
            }
        };
    };
    articles_cached
}