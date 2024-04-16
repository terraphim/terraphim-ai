pub mod matcher;

pub use matcher::{find_matches, Matched};
use std::fs::File;
use std::io::prelude::*;
use url::Url;

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

/// `load_automata` loads output of the knowledge graph builder
// pub async fn load_automata(url_or_file: &str) -> Result<matcher::Automata> {
//     let thesaurus = load_thesaurus(url_or_file).await?;
//     let automata = Automata::new(thesaurus);
//     Ok(automata)
// }

/// Load a thesaurus from a file or URL
pub async fn load_thesaurus(automata_url: Url) -> Result<Thesaurus> {
    async fn read_url(url: Url) -> Result<String> {
        let response = reqwest::Client::new()
            .get(url)
            .header("Accept", "application/json")
            .send()
            .await?;

        let text = response.text().await?;

        Ok(text)
    }

    log::debug!("Reading thesaurus from file {automata_url:?}");
    let contents = if automata_url.scheme() == "http" || automata_url.scheme() == "https" {
        read_url(automata_url).await?
    } else {
        let file_path = automata_url
            .to_file_path()
            .map_err(|_| TerraphimAutomataError::Dict("Invalid file path".to_string()))?;
        let mut file = File::open(file_path)?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;
        contents
    };

    let thesaurus = serde_json::from_str(&contents)?;
    Ok(thesaurus)
}

#[cfg(test)]
mod tests {
    use terraphim_types::{Id, NormalizedTermValue};

    use super::*;

    #[tokio::test]
    async fn test_load_thesaurus_from_file() {
        let thesaurus = load_thesaurus(Url::parse("data/term_to_id_simple.json").unwrap())
            .await
            .unwrap();
        assert_eq!(thesaurus.len(), 3);
        assert_eq!(
            thesaurus.get(&NormalizedTermValue::from("foo")).unwrap().id,
            Id::from(1)
        );
        assert_eq!(
            thesaurus.get(&NormalizedTermValue::from("bar")).unwrap().id,
            Id::from(2)
        );
        assert_eq!(
            thesaurus.get(&NormalizedTermValue::from("baz")).unwrap().id,
            Id::from(1)
        );
    }

    #[tokio::test]
    async fn test_load_thesaurus_from_url() {
        let thesaurus = load_thesaurus(
            Url::parse("https://system-operator.s3.eu-west-2.amazonaws.com/term_to_id.json")
                .unwrap(),
        )
        .await
        .unwrap();
        assert_eq!(thesaurus.len(), 1725);
        assert_eq!(
            thesaurus
                .get(&NormalizedTermValue::from("@risk a user guide"))
                .unwrap()
                .id,
            Id::from(661)
        );
    }
}
