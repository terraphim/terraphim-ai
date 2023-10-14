use serde::{Deserialize, Serialize};
use smol_str::SmolStr;
use ahash::AHashMap;
use std::error::Error;

use aho_corasick::{AhoCorasick, MatchKind};

#[derive(Debug, PartialEq, Clone,Deserialize)]
pub struct Matched {
    pub term: SmolStr,
    pub id: u64,
    pub nterm: SmolStr,
    pub pos: Option<(usize, usize)>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Dictionary {
    pub id: u64,
    pub nterm: SmolStr,
}

pub fn find_matches(
    text: &str,
    dict_hash: AHashMap<SmolStr, Dictionary>,
    return_positions: bool,
) -> Result<Vec<Matched>, Box<dyn Error>> {
    let patterns= dict_hash.keys().map(|x| x.as_str()).collect::<Vec<&str>>();
    


    let ac = AhoCorasick::builder()
        .match_kind(MatchKind::LeftmostLongest)
        .ascii_case_insensitive(true)
        .build(patterns.clone())
        .unwrap();

    let mut matches: Vec<Matched> = Vec::new();
    for mat in ac.find_iter(text) {
        let term = patterns[mat.pattern()];
        let ndict = dict_hash.get(term).unwrap();
        let id = ndict.id.clone();
        let nterm = ndict.nterm.clone();
        matches.push(Matched {
            term: term.clone().into(),
            id: id,
            nterm: nterm,
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
    text: &str,
    dict_hash: &AHashMap<SmolStr, Dictionary>,
) -> Result<Vec<u64>, Box<dyn Error>> {
    let patterns= dict_hash.keys().map(|x| x.as_str()).collect::<Vec<&str>>();

    let ac = AhoCorasick::builder()
        .match_kind(MatchKind::LeftmostLongest)
        .ascii_case_insensitive(true)
        .build(&patterns.clone())
        .unwrap();

    let mut matches = Vec::new();
    for mat in ac.find_iter(text) {
        let term = patterns[mat.pattern()];
        let ndict = dict_hash.get(term).unwrap();
        let id = ndict.id.clone();
        matches.push(id.clone());
    }
    Ok(matches)
}

// This function replacing instead of matching patterns
pub fn replace_matches(
    text: &str,
    dict_hash: AHashMap<SmolStr, Dictionary>,
) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut patterns= Vec::new();
    let mut replace_with: Vec<String> = Vec::new();
    for (key, value) in dict_hash.iter() {
        patterns.push(key.as_str());
        replace_with.push(value.clone().id.clone().to_string());
    }
    let ac = AhoCorasick::builder()
        .match_kind(MatchKind::LeftmostLongest)
        .ascii_case_insensitive(true)
        .build(patterns.clone())
        .unwrap();

    let result = ac.replace_all_bytes(text.as_bytes(), &replace_with);
    Ok(result)
}
