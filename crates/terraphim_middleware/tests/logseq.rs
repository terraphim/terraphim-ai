#[cfg(test)]
mod tests {

    use terraphim_middleware::thesaurus::{Logseq, ThesaurusBuilder};

    use terraphim_middleware::Result;
    use terraphim_types::NormalizedTermValue;

    #[tokio::test]
    /// Test creating a thesaurus from a Logseq haystack (Markdown files)
    /// Uses `fixtures/logseq` as the haystack
    async fn test_logseq_thesaurus() -> Result<()> {
        let logseq = Logseq::default();
        let thesaurus = logseq
            .build("some_role".to_string(), "fixtures/logseq")
            .await?;
        let json = serde_json::to_string_pretty(&thesaurus)?;
        println!("{}", json);
        println!(
            "Value {:?}",
            thesaurus
                .get(&NormalizedTermValue::new("example bar".to_string()))
                .unwrap()
                .value
        );
        println!(
            "Key {:?}",
            thesaurus
                .get(&NormalizedTermValue::new("example bar".to_string()))
                .unwrap()
                .id
        );

        // Make sure `json` has the following structure:
        // {
        //     "example": {
        //       "id": "...",
        //       "nterm": "example bar"
        //     },
        //     "ai": {
        //       "id": "...",
        //       "nterm": "artificial intelligence"
        //     }
        // }

        assert_eq!(thesaurus.len(), 7);
        assert_eq!(
            thesaurus
                .get(&NormalizedTermValue::new("example bar".to_string()))
                .unwrap()
                .id,
            2
        );
        assert_eq!(
            thesaurus
                .get(&NormalizedTermValue::new("example bar".to_string()))
                .unwrap()
                .value,
            NormalizedTermValue::new("example".to_string())
        );
        assert_eq!(
            thesaurus
                .get(&NormalizedTermValue::new("example".to_string()))
                .unwrap()
                .value,
            NormalizedTermValue::new("example".to_string())
        );
        assert_eq!(
            thesaurus
                .get(&NormalizedTermValue::new("ai".to_string()))
                .unwrap()
                .value,
            NormalizedTermValue::new("ai".to_string())
        );

        Ok(())
    }
}
