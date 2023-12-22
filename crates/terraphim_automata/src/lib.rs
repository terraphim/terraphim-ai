pub mod matcher;

pub use matcher::{find_matches, replace_matches, Dictionary, Matched};
// use std::collections::HashMap;
use ahash::AHashMap;
use std::fs::File;
use std::io::prelude::*;
use std::path::Path;
use thiserror::Error;
pub type ResponseJSON = AHashMap<String, Dictionary>;

#[derive(Error, Debug)]
pub enum TerraphimAutomataError {
    #[error("Reqwest error: {0}")]
    Reqwest(#[from] reqwest::Error),
    #[error("Serde deserialization error: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, TerraphimAutomataError>;

pub async fn load_automata(url_or_file: &str) -> Result<AHashMap<String, Dictionary>> {
    /// TODO: use async version of reqwest
    async fn read_url(url: &str) -> Result<String> {
        let response = reqwest::Client::new()
        .get(url)
        .header("Accept", "application/json")
        .send()
        .await?;

        let text = response.text().await?;
        
        Ok(text)
    }
    let contents = if url_or_file.starts_with("http") {
        read_url(url_or_file).await?
    } else {
        let mut file = File::open(Path::new(url_or_file))?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;
        contents
    };

    let dict_hash = serde_json::from_str(&contents)?;
    Ok(dict_hash)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_automata_from_file() {
        let dict_hash = load_automata("tests/test_data.csv.gz").unwrap();
        assert_eq!(dict_hash.len(), 3);
        assert_eq!(dict_hash.get("foo").unwrap().id, "1");
        assert_eq!(dict_hash.get("bar").unwrap().id, "2");
        assert_eq!(dict_hash.get("baz").unwrap().id, "3");
        assert_eq!(dict_hash.get("foo").unwrap().parent, None);
        assert_eq!(dict_hash.get("bar").unwrap().parent, Some("1".to_string()));
        assert_eq!(dict_hash.get("baz").unwrap().parent, Some("2".to_string()));
    }

    #[test]
    fn test_load_automata_from_url() {
        let dict_hash = load_automata(
            "https://raw.githubusercontent.com/github/copilot-sample-code/main/test_data.csv.gz",
        )
        .unwrap();
        assert_eq!(dict_hash.len(), 3);
        assert_eq!(dict_hash.get("foo").unwrap().id, "1");
        assert_eq!(dict_hash.get("bar").unwrap().id, "2");
        assert_eq!(dict_hash.get("baz").unwrap().id, "3");
        assert_eq!(dict_hash.get("foo").unwrap().parent, None);
        assert_eq!(dict_hash.get("bar").unwrap().parent, Some("1".to_string()));
        assert_eq!(dict_hash.get("baz").unwrap().parent, Some("2".to_string()));
    }
}
