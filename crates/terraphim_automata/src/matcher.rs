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

pub fn find_matches(
    text: &str,
    dict_hash: AHashMap<String, Dictionary>,
    return_positions: bool,
) -> Result<Vec<Matched>, Box<dyn Error>> {
    let patterns: Vec<String> = dict_hash.keys().cloned().collect();

    let ac = AhoCorasick::builder()
        .match_kind(MatchKind::LeftmostLongest)
        .ascii_case_insensitive(true)
        .build(patterns.clone())
        .unwrap();

    let mut matches: Vec<Matched> = Vec::new();
    for mat in ac.find_iter(text) {
        let term = &patterns[mat.pattern()];
        matches.push(Matched {
            term: term.clone(),
            id: dict_hash.get(term).unwrap().id.clone(),
            nterm: dict_hash.get(term).unwrap().nterm.clone(),
            pos: if return_positions {
                Some((mat.start(), mat.end()))
            } else {
                None
            },
        });
    }
    Ok(matches)
}

pub fn find_matches_ids(
    ac: &AhoCorasick,
    values: &Vec<u64>,
    text: &str,
) -> Result<Vec<u64>, Box<dyn Error>> {
    
    let mut matches = Vec::new();
    for mat in ac.find_iter(text) {
        // println!("mat: {:?}", mat);
        let id = values[mat.pattern()];
        matches.push(id);
    }
    Ok(matches)
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
