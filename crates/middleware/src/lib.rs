use std::time;
use serde::Deserialize;
use serde_json as json;
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
struct Summary {
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
struct SubMatch {
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
        Data::Text { text: s.to_string() }
    }
    fn bytes(s: &str) -> Data {
        Data::Bytes { bytes: s.to_string() }
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


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}
