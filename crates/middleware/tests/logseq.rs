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

        Ok(())
    }
}
