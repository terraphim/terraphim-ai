use crate::indexer::IndexMiddleware;
use crate::Result;
use terraphim_config::Haystack;
use terraphim_types::Index;

/// Indexes email via JMAP as Terraphim documents.
#[derive(Debug, Clone, Default)]
pub struct JmapHaystackIndexer;

impl IndexMiddleware for JmapHaystackIndexer {
    fn index(
        &self,
        needle: &str,
        haystack: &Haystack,
    ) -> impl std::future::Future<Output = Result<Index>> + Send {
        let query = needle.to_string();
        let extras = haystack.get_extra_parameters().clone();
        let location = haystack.location.clone();
        async move {
            let session_url = std::env::var("JMAP_SESSION_URL")
                .ok()
                .or(if !location.is_empty() {
                    Some(location)
                } else {
                    None
                })
                .unwrap_or("https://api.fastmail.com/jmap/session".to_string());

            let access_token = std::env::var("JMAP_ACCESS_TOKEN")
                .ok()
                .or_else(|| extras.get("access_token").cloned())
                .unwrap_or_default();

            if access_token.is_empty() {
                log::warn!("JMAP_ACCESS_TOKEN not set; returning empty index");
                return Ok(Index::default());
            }

            let limit: u32 = extras
                .get("limit")
                .and_then(|s| s.parse().ok())
                .unwrap_or(50);

            let jmap_client = match haystack_jmap::JMAPClient::new(access_token, &session_url).await
            {
                Ok(c) => c,
                Err(e) => {
                    log::error!("Failed to create JMAP client: {}", e);
                    return Ok(Index::default());
                }
            };

            let emails = match jmap_client.search_emails(&query, limit).await {
                Ok(e) => e,
                Err(e) => {
                    log::warn!("JMAP search failed: {}", e);
                    return Ok(Index::default());
                }
            };

            let mut index = Index::new();
            for email in &emails {
                let doc = haystack_jmap::email_to_document(email);
                index.insert(doc.id.clone(), doc);
            }
            Ok(index)
        }
    }
}
