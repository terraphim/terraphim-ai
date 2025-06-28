//! Ripgrep command and message types.
//!
//! This module contains the `RipgrepCommand` struct, which is used to run
//! ripgrep and return the results as a Vec of `Message` types. The `Message`
//! type is used to represent ripgrep's internal JSON output.
//!
//! Learn more about ripgrep's JSON output here:
//! https://docs.rs/grep-printer/0.2.1/grep_printer/struct.JSON.html

use serde::Deserialize;
use std::path::Path;
use std::process::Stdio;
use std::time;
use tokio::io::AsyncReadExt;
use tokio::process::Command;

use crate::Result;

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
    Ok(serde_json::Deserializer::from_str(jsonlines)
        .into_iter()
        .collect::<std::result::Result<Vec<Message>, serde_json::Error>>()?)
}

pub struct RipgrepCommand {
    command: String,
    default_args: Vec<String>,
}

/// Returns a new ripgrep service with default arguments
impl Default for RipgrepCommand {
    fn default() -> Self {
        Self {
            command: "rg".to_string(),
            default_args: ["--json", "--trim", "-C3", "--ignore-case", "-tmarkdown"]
                .into_iter()
                .map(String::from)
                .collect(),
        }
    }
}

impl RipgrepCommand {
    /// Runs ripgrep to find `needle` in `haystack`
    ///
    /// Returns a Vec of Messages, which correspond to ripgrep's internal
    /// JSON output. Learn more about ripgrep's JSON output here:
    /// https://docs.rs/grep-printer/0.2.1/grep_printer/struct.JSON.html
    pub async fn run(&self, needle: &str, haystack: &Path) -> Result<Vec<Message>> {
        // Put options first, then needle, then haystack (correct ripgrep argument order)
        let args: Vec<String> = self.default_args.clone()
            .into_iter()
            .chain(vec![needle.to_string(), haystack.to_string_lossy().to_string()])
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
        log::debug!("Raw ripgrep output ({} bytes): {}", output.len(), &output[..std::cmp::min(500, output.len())]);
        let messages = json_decode(&output)?;
        log::debug!("JSON decode produced {} messages", messages.len());
        Ok(messages)
    }
}
