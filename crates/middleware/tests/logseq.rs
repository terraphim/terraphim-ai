#[cfg(test)]
mod tests {

    use terraphim_middleware::thesaurus::{Logseq, ThesaurusBuilder};

    use terraphim_middleware::Result;

    #[tokio::test]
    /// Test creating a thesaurus from a Logseq haystack (Markdown files)
    /// Uses `fixtures/logseq` as the haystack
    async fn test_logseq_thesaurus() -> Result<()> {
        let logseq = Logseq::default();
        let thesaurus = logseq.build("fixtures/logseq").await?;
        let json = serde_json::to_string_pretty(&thesaurus)?;
        println!("{}", json);

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

        assert_eq!(thesaurus.len(), 2);
        assert_eq!(thesaurus.get("example").unwrap().value, "example bar");
        assert_eq!(
            thesaurus.get("ai").unwrap().value,
            "artificial intelligence"
        );

        Ok(())
    }
}
