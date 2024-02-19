pub mod matcher;

pub use matcher::{find_matches, replace_matches, Matched};
use std::fs::File;
use std::io::prelude::*;
use std::path::Path;

use terraphim_types::Thesaurus;

#[derive(thiserror::Error, Debug)]
pub enum TerraphimAutomataError {
    #[error("Reqwest error: {0}")]
    Reqwest(#[from] reqwest::Error),

    #[error("Serde deserialization error: {0}")]
    Serde(#[from] serde_json::Error),

    #[error("Dict error: {0}")]
    Dict(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Aho-Corasick build error: {0}")]
    AhoCorasick(#[from] aho_corasick::BuildError),
}

pub type Result<T> = std::result::Result<T, TerraphimAutomataError>;

pub async fn load_automata(url_or_file: &str) -> Result<Thesaurus> {
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

    let thesaurus = serde_json::from_str(&contents)?;
    Ok(thesaurus)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_load_automata_from_file() {
        let thesaurus = load_automata("tests/test_data.csv.gz").await.unwrap();
        assert_eq!(thesaurus.len(), 3);
        assert_eq!(thesaurus.get("foo").unwrap().id, 1);
        assert_eq!(thesaurus.get("bar").unwrap().id, 2);
        assert_eq!(thesaurus.get("baz").unwrap().id, 3);

        // TODO: No `parent` field in type `NormalizedTerm`
        // assert_eq!(thesaurus.get("foo").unwrap().parent, None);
        // assert_eq!(thesaurus.get("bar").unwrap().parent, Some("1".to_string()));
        // assert_eq!(thesaurus.get("baz").unwrap().parent, Some("2".to_string()));
    }

    #[tokio::test]
    async fn test_load_automata_from_url() {
        let thesaurus = load_automata(
            "https://raw.githubusercontent.com/github/copilot-sample-code/main/test_data.csv.gz",
        )
        .await
        .unwrap();
        assert_eq!(thesaurus.len(), 3);
        assert_eq!(thesaurus.get("foo").unwrap().id, 1);
        assert_eq!(thesaurus.get("bar").unwrap().id, 2);
        assert_eq!(thesaurus.get("baz").unwrap().id, 3);

        // TODO: No `parent` field in type `NormalizedTerm`
        // assert_eq!(thesaurus.get("foo").unwrap().parent, None);
        // assert_eq!(thesaurus.get("bar").unwrap().parent, Some("1".to_string()));
        // assert_eq!(thesaurus.get("baz").unwrap().parent, Some("2".to_string()));
    }
}
