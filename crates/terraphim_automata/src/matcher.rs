use serde::{Deserialize, Serialize};

use ahash::AHashMap;
use std::error::Error;

use aho_corasick::{AhoCorasick, MatchKind};

#[derive(Debug, PartialEq, Clone)]
pub struct Matched {
    pub term: String,
    pub id: u64,
    pub nterm: String,
    pub pos: Option<(usize, usize)>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Dictionary {
    pub id: u64,
    pub nterm: String,
}

// This function replacing instead of matching patterns
pub fn replace_matches(
    text: &str,
    dict_hash: AHashMap<String, Dictionary>,
) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut patterns: Vec<String> = Vec::new();
    let mut replace_with: Vec<String> = Vec::new();
    for (key, value) in dict_hash.iter() {
        patterns.push(key.clone());
        replace_with.push(value.clone().id.clone().to_string());
    }
    let ac = AhoCorasick::builder()
        .match_kind(MatchKind::LeftmostLongest)
        .ascii_case_insensitive(true)
        .build(patterns)
        .unwrap();

    let result = ac.replace_all_bytes(text.as_bytes(), &replace_with);
    Ok(result)
}
